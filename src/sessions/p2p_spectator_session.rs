use std::collections::VecDeque;
use std::fmt;
use std::sync::Arc;

use crate::error::{allocation_failed, try_reserve_hint};
use crate::{
    frame_info::PlayerInput,
    network::{
        messages::ConnectionStatus,
        protocol::{Event, UdpProtocol},
    },
    report_violation, report_violation_to,
    sessions::session_trait::Session,
    telemetry::{ViolationKind, ViolationObserver, ViolationSeverity},
    Config, EventDrain, FortressError, FortressEvent, FortressRequest, FortressResult, Frame,
    GameStateCell, InputStatus, InputVec, InternalErrorKind, InvalidFrameReason,
    InvalidRequestKind, NetworkStats, NonBlockingSocket, PlayerHandle, RequestVec, SessionMetrics,
    SessionState,
};

/// The number of frames the spectator advances in a single step during normal operation.
///
/// When not catching up to the host, spectators advance one frame at a time to maintain
/// smooth playback. During catchup mode (when far behind), `catchup_speed` is used instead.
const NORMAL_SPEED: usize = 1;

struct HostFrameSnapshot<I>
where
    I: Copy + Clone + PartialEq + Eq,
{
    frame: Frame,
    inputs: Vec<Option<PlayerInput<I>>>,
    status: Vec<ConnectionStatus>,
}

impl<I> HostFrameSnapshot<I>
where
    I: Copy + Clone + PartialEq + Eq,
{
    fn new(
        frame: Frame,
        num_players: usize,
        status: Vec<ConnectionStatus>,
    ) -> Result<Self, FortressError> {
        let mut inputs = Vec::new();
        inputs
            .try_reserve_exact(num_players)
            .map_err(|_err| allocation_failed("spectator.host_frame_snapshot", num_players))?;
        for _ in 0..num_players {
            inputs.push(None);
        }

        Ok(Self {
            frame,
            inputs,
            status,
        })
    }

    fn is_complete(&self) -> bool {
        self.inputs.iter().all(Option::is_some)
    }
}

struct HostEventBatch<T>
where
    T: Config,
{
    host_index: usize,
    addr: T::Address,
    events: Vec<Event<T>>,
}

#[derive(Clone)]
struct CanonicalFrameHost<A> {
    frame: Frame,
    addr: A,
}

#[derive(Clone)]
struct SpectatorDivergenceState<A> {
    frame: Frame,
    player: PlayerHandle,
    _marker: std::marker::PhantomData<A>,
}

/// [`SpectatorSession`] provides all functionality to connect to a remote host in a peer-to-peer fashion.
///
/// The host will broadcast all confirmed inputs to this session.
/// This session can be used to spectate a session without contributing to the game input.
///
/// This type implements the [`Session`] trait. Note that [`add_local_input`](Session::add_local_input)
/// and [`local_player_handle_required`](Session::local_player_handle_required) return
/// "not supported" errors, since spectators do not contribute input.
///
/// [`Session`]: crate::Session
pub struct SpectatorSession<T>
where
    T: Config,
{
    state: SessionState,
    num_players: usize,
    buffer_size: usize,
    inputs: Vec<Vec<PlayerInput<T::Input>>>,
    host_connect_status: Vec<ConnectionStatus>,
    socket: Box<dyn NonBlockingSocket<T::Address>>,
    /// One or more redundant hosts feeding confirmed inputs to this spectator.
    ///
    /// Unresolved frames use the highest-priority currently connected host by
    /// this vector's order as the canonical source. A host that disconnects is
    /// removed; spectation continues while at least one host remains. See
    /// [`SpectatorSession::num_hosts`].
    hosts: Vec<UdpProtocol<T>>,
    host_snapshots: Vec<Vec<Option<HostFrameSnapshot<T::Input>>>>,
    /// Per-host disconnect-witness provenance: `host_drop_witness[host][player]`
    /// records that host's own most-recent forwarded **drop** of the slot — its
    /// freeze `last_frame` and the [`ConnectionStatus::epoch`] generation it
    /// dropped at (`None` if it has not dropped the slot in the current,
    /// not-yet-consumed cycle). A drop report at a strictly NEWER epoch resets
    /// the witness to the new cycle; one at the SAME epoch max-merges the freeze
    /// (retransmits / pre-convergence-high views); one at a STRICTLY OLDER epoch
    /// than [`Self::host_status_epoch`] is a reordered earlier-cycle packet and
    /// is ignored. RE-ARMED at commit time for the committing host whenever its
    /// snapshot's drop is adopted into a connected slot
    /// ([`Self::witness_adopted_drop`] — a follow can consume the arrival-time
    /// witness between staging and commit, and a spectator-authored freeze must
    /// stay re-openable by its author), and CONSUMED — cleared for the player
    /// across all hosts — whenever a reactivation is followed
    /// ([`Self::consume_drop_witnesses`]).
    ///
    /// This gates the `disconnected -> connected` reactivation FOLLOW in
    /// [`merge_connection_status`]: a genuine hot-join re-open is gossiped by a
    /// host that itself previously gossiped the slot disconnected (every live
    /// host transitions its own `local_connect_status` through the drop before
    /// the rearm re-opens it, and gossip rides every packet, so a live witness
    /// re-establishes itself after consumption), whereas a stale lagging host
    /// that never observed the drop gossips connected without ever having
    /// reported the drop. The follow additionally requires the connected
    /// report's epoch to be `>=` the witnessed drop's epoch (a genuine
    /// reactivation strictly postdates the drop that bumped the generation; a
    /// reordered PRE-drop connected snapshot carries the older pre-drop epoch and
    /// is rejected) — see [`Self::reactivation_provenance`]. Kept index-parallel
    /// with [`Self::hosts`]/[`Self::host_snapshots`] (entries are removed
    /// together in [`Self::remove_disconnected_hosts`]).
    host_drop_witness: Vec<Vec<Option<DropWitness>>>,
    /// Per-host high-water [`ConnectionStatus::epoch`] generation:
    /// `host_status_epoch[host][player]` is the maximum epoch this spectator has
    /// observed in that host's forwarded connect-status stream for the slot
    /// (monotone non-decreasing, advanced by EVERY report — connected or
    /// disconnected). It is the cross-cycle discriminator the consumed witness
    /// table cannot provide: a reordered earlier-cycle drop report arriving after
    /// a follow consumed the witness re-arms it on a pre-epoch gate, but its
    /// epoch is strictly below this high-water (which the reactivation the host
    /// forwarded already advanced), so the re-arm is rejected. Unlike the witness
    /// it is NOT consumed (it is a monotone watermark, not per-cycle provenance).
    /// Index-parallel with [`Self::host_drop_witness`].
    host_status_epoch: Vec<Vec<u16>>,
    canonical_hosts: Vec<Option<CanonicalFrameHost<T::Address>>>,
    event_queue: VecDeque<FortressEvent<T>>,
    current_frame: Frame,
    last_recv_frame: Frame,
    max_frames_behind: usize,
    catchup_speed: usize,
    /// Number of frames to hold playback back from the live edge (anti-stream-sniping).
    stream_delay: usize,
    /// Whether the spectator records game state every frame to support rewind/seek.
    enable_rewind: bool,
    /// Per-frame saved game-state ring buffer used for rewind/seek.
    ///
    /// Empty when [`Self::enable_rewind`] is `false`. When rewind is enabled,
    /// its length equals [`Self::buffer_size`] and each slot is indexed by
    /// `frame.buffer_index(buffer_size)`.
    state_buffer: Vec<GameStateCell<T::State>>,
    /// Optional observer for specification violations.
    violation_observer: Option<Arc<dyn ViolationObserver>>,
    /// Maximum number of events to queue before oldest are dropped.
    max_event_queue_size: usize,
    /// Cumulative, always-on session metrics (see [`SpectatorSession::metrics`]).
    metrics: SessionMetrics,
    /// Whether an event-queue-overflow `Warning` has already been reported since
    /// the last [`events`](SpectatorSession::events) drain. Rate-limits the
    /// overflow violation to one per overflow episode; `metrics` keeps the full
    /// history regardless.
    event_discard_warned: bool,
    spectator_divergence: Option<SpectatorDivergenceState<T::Address>>,
    /// Host indices that will emit `Disconnected` during the current poll.
    /// Cross-host comparisons must ignore these hosts so same-poll failover
    /// cannot falsely latch divergence against a host that is no longer connected.
    disconnecting_hosts: Vec<usize>,
}

impl<T: Config> SpectatorSession<T> {
    /// Creates a new [`SpectatorSession`] for a spectator.
    /// The session will receive inputs from all players from the given host(s) directly.
    /// The session will use the provided socket.
    ///
    /// `hosts` may contain more than one endpoint for failover: unresolved frames
    /// use the highest-priority currently connected host by host order as their
    /// canonical source, and the session keeps advancing while at least one host
    /// remains connected.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        num_players: usize,
        socket: Box<dyn NonBlockingSocket<T::Address>>,
        hosts: Vec<UdpProtocol<T>>,
        buffer_size: usize,
        max_frames_behind: usize,
        catchup_speed: usize,
        stream_delay: usize,
        enable_rewind: bool,
        violation_observer: Option<Arc<dyn ViolationObserver>>,
        event_queue_size: usize,
    ) -> Result<Self, FortressError> {
        // host connection status
        let mut host_connect_status = Vec::new();
        host_connect_status
            .try_reserve_exact(num_players)
            .map_err(|_err| allocation_failed("spectator.host_connect_status", num_players))?;
        for _ in 0..num_players {
            host_connect_status.push(ConnectionStatus::default());
        }

        // Use at least 1 for buffer size to prevent panics
        let actual_buffer_size = buffer_size.max(1);

        // When rewind is enabled, allocate one game-state cell per ring slot.
        let mut state_buffer = Vec::new();
        if enable_rewind {
            state_buffer
                .try_reserve_exact(actual_buffer_size)
                .map_err(|_err| allocation_failed("spectator.state_buffer", actual_buffer_size))?;
            for _ in 0..actual_buffer_size {
                state_buffer.push(GameStateCell::default());
            }
        }

        let mut inputs = Vec::new();
        inputs
            .try_reserve_exact(actual_buffer_size)
            .map_err(|_err| allocation_failed("spectator.inputs", actual_buffer_size))?;
        for _ in 0..actual_buffer_size {
            let mut frame_inputs = Vec::new();
            // reserve-in-loop: one fresh per-player input buffer per frame slot, reserved once to its exact bounded size (`num_players`).
            let reserved = frame_inputs.try_reserve_exact(num_players);
            reserved.map_err(|_err| allocation_failed("spectator.frame_inputs", num_players))?;
            for _ in 0..num_players {
                frame_inputs.push(PlayerInput::blank_input(Frame::NULL));
            }
            inputs.push(frame_inputs);
        }

        let mut host_snapshots = Vec::new();
        host_snapshots
            .try_reserve_exact(hosts.len())
            .map_err(|_err| allocation_failed("spectator.host_snapshots", hosts.len()))?;
        for _ in 0..hosts.len() {
            let mut frames = Vec::new();
            // reserve-in-loop: one fresh snapshot-frame buffer per host, reserved once to its exact bounded size (`actual_buffer_size`).
            let reserved = frames.try_reserve_exact(actual_buffer_size);
            reserved.map_err(|_err| {
                allocation_failed("spectator.host_snapshot_frames", actual_buffer_size)
            })?;
            for _ in 0..actual_buffer_size {
                frames.push(None);
            }
            host_snapshots.push(frames);
        }

        let mut host_drop_witness = Vec::new();
        host_drop_witness
            .try_reserve_exact(hosts.len())
            .map_err(|_err| allocation_failed("spectator.host_drop_witness", hosts.len()))?;
        for _ in 0..hosts.len() {
            let mut witness = Vec::new();
            // reserve-in-loop: one fresh per-player drop-witness row per host, reserved once to its exact bounded size (`num_players`).
            let reserved = witness.try_reserve_exact(num_players);
            reserved.map_err(|_err| {
                allocation_failed("spectator.host_drop_witness_row", num_players)
            })?;
            for _ in 0..num_players {
                witness.push(None);
            }
            host_drop_witness.push(witness);
        }

        let mut host_status_epoch = Vec::new();
        host_status_epoch
            .try_reserve_exact(hosts.len())
            .map_err(|_err| allocation_failed("spectator.host_status_epoch", hosts.len()))?;
        for _ in 0..hosts.len() {
            let mut row = Vec::new();
            // reserve-in-loop: one fresh per-player epoch-watermark row per host, reserved once to its exact bounded size (`num_players`).
            row.try_reserve_exact(num_players).map_err(|_err| {
                allocation_failed("spectator.host_status_epoch_row", num_players)
            })?;
            // Fills the just-reserved exact capacity in place (no reallocation).
            // alloc-bound: `num_players` is the session-validated player count, bounded at construction (parallels the `host_drop_witness` row).
            row.resize(num_players, 0_u16);
            host_status_epoch.push(row);
        }

        let mut canonical_hosts = Vec::new();
        canonical_hosts
            .try_reserve_exact(actual_buffer_size)
            .map_err(|_err| allocation_failed("spectator.canonical_hosts", actual_buffer_size))?;
        for _ in 0..actual_buffer_size {
            canonical_hosts.push(None);
        }

        Ok(Self {
            state: SessionState::Synchronizing,
            num_players,
            buffer_size: actual_buffer_size,
            inputs,
            host_connect_status,
            socket,
            hosts,
            host_snapshots,
            host_drop_witness,
            host_status_epoch,
            canonical_hosts,
            event_queue: VecDeque::new(),
            current_frame: Frame::NULL,
            last_recv_frame: Frame::NULL,
            max_frames_behind,
            catchup_speed,
            stream_delay,
            enable_rewind,
            state_buffer,
            violation_observer,
            max_event_queue_size: event_queue_size,
            metrics: SessionMetrics::new(),
            event_discard_warned: false,
            spectator_divergence: None,
            disconnecting_hosts: Vec::new(),
        })
    }

    /// Returns the number of hosts currently feeding this spectator.
    ///
    /// For a single-host spectator this starts at `1` and may drop to `0` if
    /// the host disconnects. For a failover spectator created via
    /// [`SessionBuilder::start_spectator_session_multi`], this starts at the
    /// number of supplied addresses and drops by one each time a host
    /// disconnects, letting the application observe redundancy in real time.
    ///
    /// # Example
    ///
    /// ```
    /// # use fortress_rollback::prelude::*;
    /// # use fortress_rollback::Message;
    /// # use std::net::SocketAddr;
    /// # #[derive(Debug)]
    /// # struct TestConfig;
    /// # impl Config for TestConfig {
    /// #     type Input = u8;
    /// #     type State = u8;
    /// #     type Address = SocketAddr;
    /// # }
    /// # struct DummySocket;
    /// # impl NonBlockingSocket<SocketAddr> for DummySocket {
    /// #     fn send_to(&mut self, _msg: &Message, _addr: &SocketAddr) {}
    /// #     fn receive_all_messages(&mut self) -> Vec<(SocketAddr, Message)> { Vec::new() }
    /// # }
    /// let host_a: SocketAddr = "127.0.0.1:7000".parse()?;
    /// let host_b: SocketAddr = "127.0.0.1:7001".parse()?;
    /// let session = SessionBuilder::<TestConfig>::new()
    ///     .with_num_players(2)?
    ///     .start_spectator_session_multi(&[host_a, host_b], DummySocket)
    ///     .ok_or(FortressError::NotSynchronized)?;
    /// assert_eq!(session.num_hosts(), 2);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// [`SessionBuilder::start_spectator_session_multi`]: crate::SessionBuilder::start_spectator_session_multi
    #[must_use = "the host count should be inspected"]
    pub fn num_hosts(&self) -> usize {
        self.hosts.len()
    }

    /// Returns `true` if this spectator records state for rewind/seek.
    ///
    /// This reflects the [`SpectatorConfig::enable_rewind`] setting the session
    /// was built with. When `false`, [`Self::seek_to_frame`] returns a
    /// "not supported" error.
    ///
    /// # Example
    ///
    /// ```
    /// # use fortress_rollback::prelude::*;
    /// # use fortress_rollback::Message;
    /// # use std::net::SocketAddr;
    /// # #[derive(Debug)]
    /// # struct TestConfig;
    /// # impl Config for TestConfig {
    /// #     type Input = u8;
    /// #     type State = u8;
    /// #     type Address = SocketAddr;
    /// # }
    /// # struct DummySocket;
    /// # impl NonBlockingSocket<SocketAddr> for DummySocket {
    /// #     fn send_to(&mut self, _msg: &Message, _addr: &SocketAddr) {}
    /// #     fn receive_all_messages(&mut self) -> Vec<(SocketAddr, Message)> { Vec::new() }
    /// # }
    /// # use fortress_rollback::SpectatorConfig;
    /// let host: SocketAddr = "127.0.0.1:7000".parse()?;
    /// let config = SpectatorConfig { enable_rewind: true, ..SpectatorConfig::default() };
    /// let session = SessionBuilder::<TestConfig>::new()
    ///     .with_num_players(2)?
    ///     .with_spectator_config(config)
    ///     .start_spectator_session(host, DummySocket)
    ///     .ok_or(FortressError::NotSynchronized)?;
    /// assert!(session.is_rewind_enabled());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// [`SpectatorConfig::enable_rewind`]: crate::SpectatorConfig::enable_rewind
    #[must_use = "the rewind setting should be inspected"]
    pub fn is_rewind_enabled(&self) -> bool {
        self.enable_rewind
    }

    /// Returns the configured stream delay in frames.
    ///
    /// This reflects the [`SpectatorConfig::stream_delay`] setting the session
    /// was built with. The spectator never advances past
    /// `last_received_frame - stream_delay`.
    ///
    /// # Example
    ///
    /// ```
    /// # use fortress_rollback::prelude::*;
    /// # use fortress_rollback::Message;
    /// # use std::net::SocketAddr;
    /// # #[derive(Debug)]
    /// # struct TestConfig;
    /// # impl Config for TestConfig {
    /// #     type Input = u8;
    /// #     type State = u8;
    /// #     type Address = SocketAddr;
    /// # }
    /// # struct DummySocket;
    /// # impl NonBlockingSocket<SocketAddr> for DummySocket {
    /// #     fn send_to(&mut self, _msg: &Message, _addr: &SocketAddr) {}
    /// #     fn receive_all_messages(&mut self) -> Vec<(SocketAddr, Message)> { Vec::new() }
    /// # }
    /// # use fortress_rollback::SpectatorConfig;
    /// let host: SocketAddr = "127.0.0.1:7000".parse()?;
    /// let config = SpectatorConfig { stream_delay: 6, ..SpectatorConfig::default() };
    /// let session = SessionBuilder::<TestConfig>::new()
    ///     .with_num_players(2)?
    ///     .with_spectator_config(config)
    ///     .start_spectator_session(host, DummySocket)
    ///     .ok_or(FortressError::NotSynchronized)?;
    /// assert_eq!(session.stream_delay(), 6);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// [`SpectatorConfig::stream_delay`]: crate::SpectatorConfig::stream_delay
    #[must_use = "the stream delay should be inspected"]
    pub fn stream_delay(&self) -> usize {
        self.stream_delay
    }

    /// Computes the most recent frame the spectator is currently allowed to view.
    ///
    /// This is the live edge ([`Self::last_recv_frame`]) pulled back by
    /// [`Self::stream_delay`] frames. It stays clamped to [`Frame::NULL`] when
    /// no delayed frame is viewable yet, so callers never try to grab a negative
    /// frame.
    fn viewable_frame(&self) -> Frame {
        if self.hosts.is_empty() && self.spectator_divergence.is_none() {
            return self.last_recv_frame;
        }

        let Ok(delay) = i32::try_from(self.stream_delay) else {
            return Frame::NULL;
        };
        self.last_recv_frame
            .checked_sub(delay)
            .filter(|frame| *frame >= Frame::NULL)
            .unwrap_or(Frame::NULL)
    }

    fn spectator_divergence_error(&self) -> Option<FortressError> {
        self.spectator_divergence
            .as_ref()
            .map(|divergence| FortressError::SpectatorDivergence {
                frame: divergence.frame,
                player: divergence.player,
            })
    }

    /// Returns the current [`SessionState`] of a session.
    #[must_use]
    pub fn current_state(&self) -> SessionState {
        self.state
    }

    /// Returns the number of frames behind the host
    #[must_use]
    pub fn frames_behind_host(&self) -> usize {
        let _violation_scope = self.scoped_violation_observer();
        // Gracefully handle the case where current_frame somehow exceeds last_recv_frame.
        // This shouldn't happen in normal operation, but we report it and return 0 rather than panic.
        if self.current_frame > self.last_recv_frame {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::FrameSync,
                "frames_behind_host: current_frame {} exceeds last_recv_frame {} - returning 0",
                self.current_frame,
                self.last_recv_frame
            );
            return 0;
        }
        Self::positive_frame_distance(self.last_recv_frame, self.current_frame).unwrap_or(0)
    }

    fn positive_frame_distance(lead: Frame, base: Frame) -> Option<usize> {
        let diff = i64::from(lead.as_i32()) - i64::from(base.as_i32());
        (diff > 0).then(|| usize::try_from(diff).unwrap_or(usize::MAX))
    }

    /// Used to fetch some statistics about the quality of the network connection.
    ///
    /// For a multi-host (failover) spectator this reports stats for the first
    /// currently-connected host, so the reported peer can change after a failover
    /// removes the original first host.
    ///
    /// # Errors
    /// - Returns [`NotSynchronized`] if the session is not connected to other clients yet.
    ///
    /// [`NotSynchronized`]: FortressError::NotSynchronized
    #[must_use = "network stats should be inspected or logged"]
    pub fn network_stats(&self) -> Result<NetworkStats, FortressError> {
        self.hosts
            .first()
            .ok_or(FortressError::NotSynchronized)?
            .network_stats()
    }

    /// Returns the local player handle.
    ///
    /// Spectators do not have a local player, so this always returns a
    /// "not supported" error.
    ///
    /// # Errors
    ///
    /// Always returns [`InvalidRequestKind::NotSupported`].
    ///
    /// [`InvalidRequestKind::NotSupported`]: crate::InvalidRequestKind::NotSupported
    #[must_use = "returns the local player handle which should be used"]
    pub fn local_player_handle_required(&self) -> FortressResult<PlayerHandle> {
        Err(InvalidRequestKind::NotSupported {
            operation: "local_player_handle_required",
        }
        .into())
    }

    /// Adds local input for the given player.
    ///
    /// Spectators do not contribute input, so this always returns a
    /// "not supported" error.
    ///
    /// # Errors
    ///
    /// Always returns [`InvalidRequestKind::NotSupported`].
    ///
    /// [`InvalidRequestKind::NotSupported`]: crate::InvalidRequestKind::NotSupported
    #[must_use = "error should be handled"]
    pub fn add_local_input(
        &mut self,
        _player_handle: PlayerHandle,
        _input: T::Input,
    ) -> FortressResult<()> {
        Err(InvalidRequestKind::NotSupported {
            operation: "add_local_input",
        }
        .into())
    }

    /// Returns all events that happened since last queried for events. If the
    /// number of stored events exceeds the configured event queue size, the
    /// oldest events will be discarded.
    #[must_use = "events should be handled to react to session state changes"]
    pub fn events(&mut self) -> EventDrain<'_, T> {
        // Draining starts a new overflow episode: re-arm the rate-limited
        // event-queue-overflow warning (see `trim_event_queue`).
        self.event_discard_warned = false;
        EventDrain::from_drain(self.event_queue.drain(..))
    }

    /// Returns a snapshot of this spectator's cumulative [`SessionMetrics`].
    ///
    /// Counters are always-on, monotonic for the life of the session, and cheap
    /// to read (the returned value is `Copy`). A non-zero
    /// [`events_discarded_total`](SessionMetrics::events_discarded_total) means
    /// the application is draining [`events`](Self::events) slower than they
    /// arrive and has lost notifications.
    ///
    /// [`SessionMetrics`] is shared with [`P2PSession`](crate::P2PSession), so
    /// per-kind categories a spectator never emits (for example
    /// `wait_recommendation`) stay at zero here.
    pub fn metrics(&self) -> SessionMetrics {
        self.metrics
    }

    /// Returns a reference to the violation observer, if one was configured.
    ///
    /// This allows checking for violations that occurred during session operations
    /// when using a [`CollectingObserver`] or similar.
    ///
    /// [`CollectingObserver`]: crate::telemetry::CollectingObserver
    #[must_use]
    pub fn violation_observer(&self) -> Option<&Arc<dyn ViolationObserver>> {
        self.violation_observer.as_ref()
    }

    /// Installs the session's configured violation observer (if any) as the
    /// current thread's scoped observer for the duration of a public entry
    /// point, so every `report_violation!` emitted beneath it routes to the
    /// per-session observer. Returns `None` (a no-op — violations fall back to
    /// the default [`TracingObserver`]) when no observer was configured.
    ///
    /// Mirrors the scoping in [`P2PSession`] and [`SyncTestSession`]. Nested
    /// entry points (e.g. `advance_frame` calling `poll_remote_clients`) push
    /// the same observer twice and pop in LIFO order, which is harmless. Sites
    /// that already hold the observer in hand use `report_violation_to!` and
    /// route directly regardless of this scope.
    ///
    /// [`TracingObserver`]: crate::telemetry::TracingObserver
    /// [`P2PSession`]: crate::P2PSession
    /// [`SyncTestSession`]: crate::SyncTestSession
    #[inline]
    #[must_use]
    fn scoped_violation_observer(&self) -> Option<crate::telemetry::ScopedObserverGuard> {
        self.violation_observer
            .as_ref()
            .map(|observer| crate::telemetry::push_violation_observer(Arc::clone(observer)))
    }

    /// Computes the request-batch preallocation capacity for [`advance_frame`].
    ///
    /// `frames_to_advance` is clamped to `buffer_size` because the advance loop
    /// breaks once `frame_to_grab` passes the viewable boundary, and the number
    /// of buffered-but-unsimulated frames can never exceed `buffer_size`. This
    /// keeps the allocation bounded even when an unvalidated `catchup_speed`
    /// (e.g. from a directly constructed [`crate::SpectatorConfig`]) is pathologically
    /// large. When rewind is enabled each advanced frame also emits a
    /// `SaveGameState`, so the batch can hold up to twice as many requests.
    ///
    /// [`advance_frame`]: Self::advance_frame
    fn advance_capacity(
        frames_to_advance: usize,
        buffer_size: usize,
        enable_rewind: bool,
    ) -> usize {
        let bounded = frames_to_advance.min(buffer_size);
        if enable_rewind {
            bounded.saturating_mul(2)
        } else {
            bounded
        }
    }

    /// You should call this to notify Fortress Rollback that you are ready to advance your gamestate by a single frame.
    /// Returns an order-sensitive [`RequestVec`]. You should fulfill all requests in the exact order they are provided.
    /// Failure to do so will result in incorrect game state, potential desync, or errors returned from subsequent API calls.
    ///
    /// # Errors
    /// - Returns [`NotSynchronized`] if the session is not yet ready to accept input.
    ///   In this case, you either need to start the session or wait for synchronization between clients.
    ///
    /// [`RequestVec`]: crate::RequestVec
    /// [`NotSynchronized`]: FortressError::NotSynchronized
    #[must_use = "FortressRequests must be processed to advance the game state"]
    pub fn advance_frame(&mut self) -> FortressResult<RequestVec<T>> {
        let _violation_scope = self.scoped_violation_observer();
        if let Some(err) = self.spectator_divergence_error() {
            return Err(err);
        }

        // receive info from host, trigger events and send messages
        self.poll_remote_clients();

        if let Some(err) = self.spectator_divergence_error() {
            return Err(err);
        }

        if self.state != SessionState::Running {
            return Err(FortressError::NotSynchronized);
        }

        // The most recent frame the spectator may display. With stream_delay == 0
        // this is the live edge; otherwise it trails the live edge so playback is
        // held back from the host's most recent inputs.
        let viewable = self.viewable_frame();

        // How far behind the viewable edge we are. We use this (rather than the raw
        // distance to the live edge) so a configured stream_delay does not force the
        // spectator into perpetual catchup mode.
        let effective_behind =
            Self::positive_frame_distance(viewable, self.current_frame).unwrap_or(0);

        let frames_to_advance = if effective_behind > self.max_frames_behind {
            self.catchup_speed
        } else {
            NORMAL_SPEED
        };

        // Reserve fallibly for the expected catch-up batch. In normal operation
        // this stays inline; when users configure a very large catchup_speed, a
        // failed heap reservation becomes a structured error instead of an abort.
        let capacity =
            Self::advance_capacity(frames_to_advance, self.buffer_size, self.enable_rewind);
        let mut requests = RequestVec::<T>::new();
        requests
            .try_reserve(capacity)
            .map_err(|_err| allocation_failed("spectator.advance.requests", capacity))?;

        for _ in 0..frames_to_advance {
            // get inputs for the next frame
            let frame_to_grab = self.current_frame.try_add(1)?;

            // Respect the stream-delay boundary: never advance past the viewable
            // frame. If no earlier frame was gathered in this batch, the post-loop
            // guard reports PredictionThreshold so callers know playback is waiting
            // for newer host inputs to move the delayed boundary forward.
            if frame_to_grab > viewable {
                break;
            }

            match self.inputs_at_frame(frame_to_grab) {
                Ok(synced_inputs) => {
                    // SAVE/SEEK FRAME INVARIANT (the crux of rewind support):
                    //
                    // The GameStub/contract requires SaveGameState{frame: F} be emitted
                    // when the user's game is AT frame F. The spectator's `current_frame`
                    // is the LAST simulated frame (starts at NULL == -1); the user's game
                    // frame == current_frame + 1 (the next frame to simulate). So when we
                    // are about to simulate `frame_to_grab` (== current_frame + 1), the
                    // user's game is exactly at `frame_to_grab`. We therefore emit
                    // SaveGameState{frame: frame_to_grab} BEFORE AdvanceFrame. The saved
                    // cell labeled F holds the state at the START of frame F, stored in
                    // state_buffer[F.buffer_index(buffer_size)].
                    if self.enable_rewind {
                        if let Some(cell) = self.save_cell_for(frame_to_grab) {
                            requests.push(FortressRequest::SaveGameState {
                                cell,
                                frame: frame_to_grab,
                            });
                        }
                    }

                    requests.push(FortressRequest::AdvanceFrame {
                        inputs: synced_inputs,
                    });

                    // advance the frame, but only after grabbing the inputs succeeded
                    self.current_frame = frame_to_grab;
                },
                // Nothing more is available yet — stop and return whatever progress we
                // have already gathered (returning partial progress is correct; the old
                // code discarded gathered catchup requests on a mid-batch error here).
                Err(FortressError::PredictionThreshold) => break,
                // A genuine error (e.g. SpectatorTooFarBehind) must propagate.
                Err(other) => return Err(other),
            }
        }

        // Preserve the historical Ok(empty) result when no advance was even attempted
        // (e.g. catchup_speed == 0 while behind). Only surface "nothing available yet"
        // as PredictionThreshold when we actually tried to advance at least one frame.
        if frames_to_advance > 0 && requests.is_empty() {
            return Err(FortressError::PredictionThreshold);
        }

        Ok(requests)
    }

    /// Returns the rewind ring cell for `frame`, cloned so the user can save into it.
    ///
    /// Cloning shares the underlying storage (the cell is backed by an `Arc<Mutex<…>>`),
    /// so a save through the returned clone is visible via [`Self::state_buffer`]. Returns
    /// `None` only if `frame` cannot be mapped to a ring slot (negative frame or empty
    /// buffer), which should not happen for the valid `frame_to_grab` values used here.
    fn save_cell_for(&self, frame: Frame) -> Option<GameStateCell<T::State>> {
        let idx = frame.buffer_index(self.buffer_size)?;
        self.state_buffer.get(idx).cloned()
    }

    /// Seeks the spectator to `target_frame` within the buffered rewind window.
    /// After the returned requests are processed, [`current_frame()`](Self::current_frame)
    /// `== target_frame` and the game state reflects all frames up to and including
    /// `target_frame`.
    ///
    /// Because every in-window frame is saved (when rewind is enabled), a seek is a single
    /// [`LoadGameState`](crate::FortressRequest::LoadGameState) — no re-simulation is
    /// required. After seeking backwards, a normal [`advance_frame`](Self::advance_frame)
    /// resumes from `target_frame` (catchup may kick in to return to the live edge).
    /// Forward-seeking to a previously visited frame still held in the ring is also
    /// supported.
    ///
    /// The seekable upper bound is [`current_frame()`](Self::current_frame) `- 1`:
    /// seeking to `target` loads the saved cell labeled `target + 1`, and the cell
    /// labeled `current_frame + 1` is only saved on the *next* advance, so seeking
    /// to the exact current frame returns [`MissingState`](InvalidFrameReason::MissingState).
    ///
    /// # Errors
    ///
    /// - Returns [`InvalidRequestKind::NotSupported`] if rewind was not enabled via
    ///   [`SpectatorConfig::enable_rewind`].
    /// - Returns [`FortressError::InvalidFrameStructured`] with
    ///   [`InvalidFrameReason::MustBeNonNegative`] if `target_frame` is negative.
    /// - Returns [`FortressError::FrameArithmeticOverflow`] if `target_frame + 1`
    ///   cannot be represented.
    /// - Returns [`FortressError::InvalidFrameStructured`] with
    ///   [`InvalidFrameReason::MissingState`] if the requested frame's state has rolled
    ///   out of the ring or was never saved.
    ///
    /// # Example
    ///
    /// ```
    /// # use fortress_rollback::prelude::*;
    /// # use fortress_rollback::Message;
    /// # use std::net::SocketAddr;
    /// # #[derive(Debug)]
    /// # struct TestConfig;
    /// # impl Config for TestConfig {
    /// #     type Input = u8;
    /// #     type State = u8;
    /// #     type Address = SocketAddr;
    /// # }
    /// # struct DummySocket;
    /// # impl NonBlockingSocket<SocketAddr> for DummySocket {
    /// #     fn send_to(&mut self, _msg: &Message, _addr: &SocketAddr) {}
    /// #     fn receive_all_messages(&mut self) -> Vec<(SocketAddr, Message)> { Vec::new() }
    /// # }
    /// # use fortress_rollback::SpectatorConfig;
    /// let host: SocketAddr = "127.0.0.1:7000".parse()?;
    /// let config = SpectatorConfig { enable_rewind: true, ..SpectatorConfig::default() };
    /// let mut session = SessionBuilder::<TestConfig>::new()
    ///     .with_num_players(2)?
    ///     .with_spectator_config(config)
    ///     .start_spectator_session(host, DummySocket)
    ///     .ok_or(FortressError::NotSynchronized)?;
    /// // Seeking to a frame that was never saved reports MissingState.
    /// assert!(session.seek_to_frame(Frame::new(0)).is_err());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// [`InvalidRequestKind::NotSupported`]: crate::InvalidRequestKind::NotSupported
    /// [`InvalidFrameReason::MustBeNonNegative`]: crate::InvalidFrameReason::MustBeNonNegative
    /// [`InvalidFrameReason::MissingState`]: crate::InvalidFrameReason::MissingState
    /// [`SpectatorConfig::enable_rewind`]: crate::SpectatorConfig::enable_rewind
    #[must_use = "FortressRequests must be processed to seek the game state"]
    pub fn seek_to_frame(&mut self, target_frame: Frame) -> FortressResult<RequestVec<T>> {
        let _violation_scope = self.scoped_violation_observer();
        if !self.enable_rewind {
            return Err(InvalidRequestKind::NotSupported {
                operation: "seek_to_frame",
            }
            .into());
        }

        if !target_frame.is_valid() {
            return Err(FortressError::InvalidFrameStructured {
                frame: target_frame,
                reason: InvalidFrameReason::MustBeNonNegative,
            });
        }

        // The saved-state label we need is `target_frame + 1`: loading the cell
        // labeled `label` restores the user's game to frame `label`, which leaves
        // current_frame == label - 1 == target_frame.
        let label = target_frame.try_add(1)?;

        let idx =
            label
                .buffer_index(self.buffer_size)
                .ok_or(FortressError::InvalidFrameStructured {
                    frame: target_frame,
                    reason: InvalidFrameReason::MissingState,
                })?;
        let cell = self
            .state_buffer
            .get(idx)
            .ok_or(FortressError::InvalidFrameStructured {
                frame: target_frame,
                reason: InvalidFrameReason::MissingState,
            })?
            .clone();

        // The requested state has rolled out of the ring (a newer frame overwrote
        // the slot) or was never saved.
        if cell.frame() != label {
            return Err(FortressError::InvalidFrameStructured {
                frame: target_frame,
                reason: InvalidFrameReason::MissingState,
            });
        }

        let mut requests = RequestVec::<T>::with_capacity(1);
        requests.push(FortressRequest::LoadGameState { cell, frame: label });
        self.current_frame = target_frame;
        Ok(requests)
    }

    /// Receive UDP packages, distribute them to corresponding UDP endpoints, handle all occurring events and send all outgoing UDP packages.
    /// Should be called periodically by your application to give Fortress Rollback a chance to do internal work like packet transmissions.
    pub fn poll_remote_clients(&mut self) {
        let _violation_scope = self.scoped_violation_observer();
        // Get all udp packets and distribute them to associated endpoints.
        // The endpoints will handle their packets, which will trigger both events and UDP replies.
        // Route each message to the FIRST host that claims to handle it, then stop.
        for (from, msg) in &self.socket.receive_all_messages() {
            for host in &mut self.hosts {
                if host.is_handling_message(from) {
                    host.handle_message(msg);
                    break;
                }
            }
        }

        // Handle all events locally, recording which hosts disconnected this poll.
        // Host events are drained into a per-host temporary first to avoid a
        // borrow conflict between the mutable host poll and event handling that
        // mutates the wider spectator session. Every batch is collected before
        // handling begins so hosts that emit Disconnected later in host order are
        // already excluded from unresolved-frame canonical comparisons.
        //
        let hosts_len = self.hosts.len();

        // alloc-bound: disconnecting host indices are deduplicated on insertion,
        // so this vector is bounded by the number of hosts present at poll start.
        self.disconnecting_hosts.clear();
        if self
            .disconnecting_hosts
            .try_reserve_exact(hosts_len)
            .is_err()
        {
            report_violation_to!(
                &self.violation_observer,
                ViolationSeverity::Error,
                ViolationKind::InternalError,
                "spectator: failed to reserve disconnecting host collection for {} hosts",
                hosts_len
            );
            return;
        }

        // alloc-bound: disconnected host indices are deduplicated on insertion,
        // so this vector is bounded by the number of hosts present at poll start.
        let mut disconnected_hosts = Vec::new();
        if disconnected_hosts.try_reserve_exact(hosts_len).is_err() {
            report_violation_to!(
                &self.violation_observer,
                ViolationSeverity::Error,
                ViolationKind::InternalError,
                "spectator: failed to reserve disconnected host collection for {} hosts",
                hosts_len
            );
            self.disconnecting_hosts.clear();
            return;
        }

        // alloc-bound: one drained event batch is stored per host present at
        // poll start, so this collection is bounded by `hosts_len`.
        let mut host_event_batches = Vec::new();
        if host_event_batches.try_reserve_exact(hosts_len).is_err() {
            report_violation_to!(
                &self.violation_observer,
                ViolationSeverity::Error,
                ViolationKind::InternalError,
                "spectator: failed to reserve event batches for {} hosts",
                hosts_len
            );
            self.disconnecting_hosts.clear();
            return;
        }

        for host_index in 0..hosts_len {
            // alloc-bound: this temporary is scoped to one host's drained
            // protocol events for one poll. Growth is fallible, so an
            // unexpectedly large protocol queue reports an internal violation
            // instead of risking allocator abort.
            let mut host_events = Vec::new();
            let addr = {
                let Some(host) = self.hosts.get_mut(host_index) else {
                    continue;
                };
                let addr = host.peer_addr();
                let events = host.poll(&self.host_connect_status);
                // Best-effort single bulk reservation: prefer the (untrusted)
                // upper bound, falling back to the lower bound when the upper is
                // absent. `try_reserve_hint` reserves with saturating arithmetic
                // and silently ignores failure, so it never aborts and never
                // changes behavior; the per-event guard below is the load-bearing
                // panic-free growth path.
                let (lower_bound, upper_bound) = events.size_hint();
                try_reserve_hint(&mut host_events, upper_bound.or(Some(lower_bound)), 1);
                for event in events {
                    // The bulk pre-reservation above covers the common case in a
                    // single allocation; this fallible guard keeps growth
                    // panic-free when the untrusted size_hint under-reported.
                    // reserve-in-loop: guards an under-reporting/absent size_hint.
                    if host_events.try_reserve(1).is_err() {
                        report_violation_to!(
                            &self.violation_observer,
                            ViolationSeverity::Error,
                            ViolationKind::InternalError,
                            "spectator: failed to grow host event collection"
                        );
                        return;
                    }
                    host_events.push(event);
                }
                addr
            };

            host_event_batches.push(HostEventBatch {
                host_index,
                addr,
                events: host_events,
            });
        }

        for batch in &host_event_batches {
            if !batch
                .events
                .iter()
                .any(|event| matches!(event, Event::Disconnected))
            {
                continue;
            }
            if !self.disconnecting_hosts.contains(&batch.host_index) {
                self.disconnecting_hosts.push(batch.host_index);
            }
        }

        for batch in host_event_batches {
            for event in batch.events {
                if disconnected_hosts.contains(&batch.host_index) {
                    continue;
                }
                if let Some(host_index) =
                    self.handle_event(batch.host_index, event, batch.addr.clone())
                {
                    if !disconnected_hosts.contains(&host_index) {
                        disconnected_hosts.push(host_index);
                    }
                }
            }
        }

        // Remove any hosts that disconnected during this poll. host_index is only
        // used during event handling above (before removal), so removing entries now
        // is safe. The shared `host_connect_status` is not per-host, so removal does
        // not disturb it.
        self.remove_disconnected_hosts(disconnected_hosts);
        self.disconnecting_hosts.clear();
        self.try_commit_ready_frames();

        // send out all pending UDP messages
        for host in &mut self.hosts {
            host.send_all_messages(&mut self.socket);
        }
    }

    fn remove_disconnected_hosts(&mut self, mut disconnected_hosts: Vec<usize>) {
        if disconnected_hosts.is_empty() {
            return;
        }

        let hosts_len = self.hosts.len();
        disconnected_hosts.sort_unstable();
        disconnected_hosts.dedup();
        for &host_index in &disconnected_hosts {
            if host_index >= hosts_len {
                report_violation_to!(
                    &self.violation_observer,
                    ViolationSeverity::Error,
                    ViolationKind::InternalError,
                    "spectator: disconnected host index {} out of bounds (hosts.len()={})",
                    host_index,
                    hosts_len
                );
            }
        }

        retain_surviving_hosts(&mut self.hosts, &disconnected_hosts);
        retain_surviving_hosts(&mut self.host_snapshots, &disconnected_hosts);
        // Keep the disconnect-witness table index-parallel with `hosts`: a
        // surviving host's witness rows must follow it to its new index, or a
        // promoted host would inherit the removed host's drop provenance and a
        // stale-connected gossip could wrongly pass the reactivation gate.
        retain_surviving_hosts(&mut self.host_drop_witness, &disconnected_hosts);
        // The epoch watermark is likewise per-host and must follow its host to
        // the new index alongside the witness it discriminates.
        retain_surviving_hosts(&mut self.host_status_epoch, &disconnected_hosts);
    }

    /// Returns the current frame of a session.
    #[must_use]
    pub fn current_frame(&self) -> Frame {
        self.current_frame
    }

    /// Returns the number of players this session was constructed with.
    #[must_use]
    pub fn num_players(&self) -> usize {
        self.num_players
    }

    fn inputs_at_frame(&self, frame_to_grab: Frame) -> Result<InputVec<T::Input>, FortressError> {
        // Validate frame is valid before computing index
        if frame_to_grab.is_null() || frame_to_grab.as_i32() < 0 {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::FrameSync,
                "inputs_at_frame called with invalid frame {:?}",
                frame_to_grab
            );
            return Err(FortressError::InvalidFrameStructured {
                frame: frame_to_grab,
                reason: InvalidFrameReason::NullOrNegative,
            });
        }

        let buffer_index = frame_to_grab.as_i32() as usize % self.buffer_size;
        let player_inputs =
            self.inputs
                .get(buffer_index)
                .ok_or(FortressError::InternalErrorStructured {
                    kind: InternalErrorKind::BufferIndexOutOfBounds,
                })?;

        if player_inputs.is_empty() {
            return Err(FortressError::InternalErrorStructured {
                kind: InternalErrorKind::EmptyPlayerInputs,
            });
        }

        let mut saw_stale_slot = false;
        for (player_index, player_input) in player_inputs.iter().enumerate() {
            if player_input.frame > frame_to_grab {
                report_violation_to!(
                    &self.violation_observer,
                    ViolationSeverity::Error,
                    ViolationKind::FrameSync,
                    "spectator: player {} input frame {} is newer than requested frame {}",
                    player_index,
                    player_input.frame,
                    frame_to_grab
                );
                return Err(FortressError::SpectatorTooFarBehind);
            }

            if player_input.frame < frame_to_grab {
                saw_stale_slot = true;
            }
        }

        if saw_stale_slot {
            return Err(FortressError::PredictionThreshold);
        }

        let host_connect_status_len = self.host_connect_status.len();
        Ok(player_inputs
            .iter()
            .enumerate()
            .map(|(player_index, player_input)| {
                if let Some(status) = self.host_connect_status.get(player_index) {
                    if status.disconnected && status.last_frame < frame_to_grab {
                        // Frozen-slot value convergence (audit finding F9).
                        //
                        // The host's player-side stream surfaces a dropped peer's
                        // input at the AGREED FREEZE FRAME `F` (=
                        // `status.last_frame`), NOT the value it happened to have
                        // forwarded for `frame_to_grab`: `synchronized_inputs` /
                        // `confirmed_inputs` both return `last_confirmed_input()`,
                        // the queue value re-rolled to `F` by `set_frozen_value_at`
                        // every time 3rd-peer gossip mines `F` down (sync_layer
                        // mod.rs frozen-slot bypass). The spectator stream, however,
                        // is append-only: the host forwards each frame exactly once
                        // (`next_spectator_frame` is monotonic) and never re-sends a
                        // frame whose dropped-slot value a later gossip lowered. So
                        // if the host had directly detected the drop and frozen
                        // "high" — forwarding `frame_to_grab`'s pre-convergence
                        // value before the lower gossip arrived — the spectator's
                        // committed buffer value for `frame_to_grab` is stale and
                        // would silently diverge from the mesh.
                        //
                        // Fix: surface the value the spectator committed at `F`
                        // itself (when that slot was still Confirmed), mirroring the
                        // host's frozen-value semantics. `host_connect_status`
                        // converges `F` DOWN via `merge_connection_status` (S22), so
                        // this self-corrects as gossip lowers `F` — no host re-send
                        // path needed. If the buffered value at `F` is unavailable
                        // (evicted from the ring, or `F` predates this spectator's
                        // history), fall back to the forwarded value: that is the
                        // same `F`-evicted residual the host tolerates, and it
                        // preserves the prior behaviour for the common case where
                        // `F` equals the last forwarded frame.
                        let frozen = self
                            .frozen_input_at(player_index, status.last_frame)
                            .unwrap_or(player_input.input);
                        (frozen, InputStatus::Disconnected)
                    } else {
                        (player_input.input, InputStatus::Confirmed)
                    }
                } else {
                    // `host_connect_status` is sized by construction to
                    // cover every host player; reaching this branch means
                    // the spectator's snapshot of the host's connect-status
                    // table is shorter than the host's player list. Surface
                    // the mismatch rather than silently treating the input
                    // as Confirmed.
                    report_violation!(
                        ViolationSeverity::Error,
                        ViolationKind::InternalError,
                        "spectator: host_connect_status missing for player_index {} (host_connect_status.len()={})",
                        player_index,
                        host_connect_status_len
                    );
                    (player_input.input, InputStatus::Confirmed)
                }
            })
            .collect())
    }

    /// Returns the dropped slot's committed input value at the agreed freeze frame
    /// `freeze_frame` (= `host_connect_status[player_index].last_frame`), or `None`
    /// if that frame is not in the ring buffer (evicted or never committed).
    ///
    /// This is the spectator analog of the host's frozen-slot bypass (the dropped
    /// peer's input at the agreed freeze frame `F`). See the call site in
    /// [`Self::inputs_at_frame`] for the F9 convergence rationale. The lookup
    /// validates the buffered entry's frame so a ring-buffer slot reused for a
    /// later frame is rejected rather than misread.
    fn frozen_input_at(&self, player_index: usize, freeze_frame: Frame) -> Option<T::Input> {
        // `buffer_index` already returns `None` for null/negative frames
        // (`Frame::NULL == -1`), so no separate guard is needed.
        let buffer_index = freeze_frame.buffer_index(self.buffer_size)?;
        let entry = self.inputs.get(buffer_index)?.get(player_index)?;
        if entry.frame != freeze_frame {
            return None;
        }
        Some(entry.input)
    }

    fn snapshot_input(
        &self,
        host_index: usize,
        frame: Frame,
        player_index: usize,
    ) -> Option<PlayerInput<T::Input>> {
        let buffer_index = frame.buffer_index(self.buffer_size)?;
        let snapshot = self
            .host_snapshots
            .get(host_index)?
            .get(buffer_index)?
            .as_ref()?;
        if snapshot.frame != frame {
            return None;
        }
        snapshot.inputs.get(player_index).copied().flatten()
    }

    fn snapshot_is_complete(&self, host_index: usize, frame: Frame) -> bool {
        let Some(buffer_index) = frame.buffer_index(self.buffer_size) else {
            return false;
        };
        self.host_snapshots
            .get(host_index)
            .and_then(|host| host.get(buffer_index))
            .and_then(Option::as_ref)
            .is_some_and(|snapshot| snapshot.frame == frame && snapshot.is_complete())
    }

    fn host_is_disconnect_pending(&self, host_index: usize) -> bool {
        self.disconnecting_hosts.contains(&host_index)
    }

    fn has_surviving_host(&self) -> bool {
        (0..self.hosts.len()).any(|host_index| !self.host_is_disconnect_pending(host_index))
    }

    fn latch_spectator_divergence(
        &mut self,
        frame: Frame,
        player: PlayerHandle,
        primary_addr: T::Address,
        conflicting_addr: T::Address,
    ) {
        if self.spectator_divergence.is_some() {
            return;
        }

        report_violation_to!(
            &self.violation_observer,
            ViolationSeverity::Error,
            ViolationKind::FrameSync,
            "spectator: divergent host input for player {} at frame {}; failing closed",
            player,
            frame
        );
        self.event_queue
            .push_back(FortressEvent::SpectatorDivergence {
                frame,
                player,
                primary_addr,
                conflicting_addr,
            });
        self.spectator_divergence = Some(SpectatorDivergenceState {
            frame,
            player,
            _marker: std::marker::PhantomData,
        });
        self.trim_event_queue();
    }

    /// Bounds the event queue to `max_event_queue_size`, dropping the oldest
    /// events first.
    ///
    /// Overflow means the application is draining events slower than they
    /// arrive; the dropped events are lost. This used to happen silently
    /// (defect D9). Every drop is now recorded in [`SessionMetrics`] (total +
    /// per-[`EventKind`](crate::metrics::EventKind)), and one rate-limited
    /// `Warning` violation is reported per overflow episode (the flag is cleared
    /// on each [`events`](Self::events) drain) so the loss is observable via
    /// [`metrics`](Self::metrics) and any registered violation observer without
    /// flooding on a churn burst.
    fn trim_event_queue(&mut self) {
        let mut discarded = 0u64;
        while self.event_queue.len() > self.max_event_queue_size {
            if let Some(dropped) = self.event_queue.pop_front() {
                self.metrics.record_event_discard(dropped.kind());
                discarded += 1;
            } else {
                break;
            }
        }
        if discarded > 0 && !self.event_discard_warned {
            self.event_discard_warned = true;
            report_violation_to!(
                &self.violation_observer,
                ViolationSeverity::Warning,
                ViolationKind::NetworkProtocol,
                "spectator event queue overflow: discarding undrained event(s) (queue cap {}); \
                 drain events every poll or raise the event-queue size to avoid losing \
                 notifications. See SessionMetrics::events_discarded_total for the running count",
                self.max_event_queue_size
            );
        }
    }

    fn detect_staged_input_disagreement(
        &mut self,
        host_index: usize,
        input: PlayerInput<T::Input>,
        player: PlayerHandle,
        addr: T::Address,
        compare_disconnect_pending: bool,
    ) -> bool {
        let player_index = player.as_usize();
        for other_host_index in 0..self.hosts.len() {
            if other_host_index == host_index {
                continue;
            }
            if self.host_is_disconnect_pending(other_host_index) && !compare_disconnect_pending {
                continue;
            }
            let Some(other_input) =
                self.snapshot_input(other_host_index, input.frame, player_index)
            else {
                continue;
            };
            if input.equal(&other_input, true) {
                continue;
            }

            let Some(other_addr) = self.hosts.get(other_host_index).map(UdpProtocol::peer_addr)
            else {
                continue;
            };
            let (primary_addr, conflicting_addr) = if other_host_index < host_index {
                (other_addr, addr)
            } else {
                (addr, other_addr)
            };
            self.latch_spectator_divergence(input.frame, player, primary_addr, conflicting_addr);
            return true;
        }

        if input.frame <= self.last_recv_frame {
            let Some(buffer_index) = input.frame.buffer_index(self.buffer_size) else {
                return false;
            };
            let Some(Some(canonical_host)) = self.canonical_hosts.get(buffer_index) else {
                return false;
            };
            if canonical_host.frame != input.frame {
                return false;
            }
            let Some(committed_input) = self
                .inputs
                .get(buffer_index)
                .and_then(|frame_inputs| frame_inputs.get(player_index))
                .copied()
            else {
                return false;
            };
            if !committed_input.equal(&input, true) {
                self.latch_spectator_divergence(
                    input.frame,
                    player,
                    canonical_host.addr.clone(),
                    addr,
                );
                return true;
            }
        }

        false
    }

    /// Returns the dropped-slot freeze frame (`ConnectionStatus.last_frame`) that
    /// `host_index` reported for `player_index` in its staged snapshot at `frame`,
    /// or `None` if there is no matching snapshot or the host does not report that
    /// slot as disconnected there.
    fn snapshot_freeze_frame(
        &self,
        host_index: usize,
        frame: Frame,
        player_index: usize,
    ) -> Option<Frame> {
        let buffer_index = frame.buffer_index(self.buffer_size)?;
        let snapshot = self
            .host_snapshots
            .get(host_index)?
            .get(buffer_index)?
            .as_ref()?;
        if snapshot.frame != frame {
            return None;
        }
        let status = snapshot.status.get(player_index)?;
        if !status.disconnected {
            return None;
        }
        Some(status.last_frame)
    }

    /// Folds a dropped slot's freeze frame down to the global minimum reported by
    /// ANY connected host with a staged snapshot at `frame`, returning the
    /// converged status.
    ///
    /// This is the spectator analog of the mesh's asymmetric-loss convergence
    /// (c25fc1f): under packet loss two hosts can freeze the same dropped peer at
    /// DIFFERENT frames. The spectator replays a dropped slot from `self.inputs[F]`
    /// where `F` is the adopted freeze frame, so it must adopt the GLOBAL-MIN `F`
    /// across all hosts — that is the only frame every survivor confirmed the same
    /// value for. The existing [`merge_connection_status`] converges DOWN only when
    /// the host reporting the lower `F` becomes the canonical committer; if a host
    /// with the higher `F` is permanently canonical (e.g. always host index 0),
    /// the lower-`F` host's value is never folded in and the spectator replays a
    /// frozen value from the non-overlapping region that no other host vouched for.
    /// Pre-folding the canonical status against every connected host's staged
    /// freeze frame closes that gap regardless of canonical-host selection.
    fn converged_drop_status(
        &self,
        canonical_host_index: usize,
        frame: Frame,
        player_index: usize,
        canonical_status: ConnectionStatus,
    ) -> ConnectionStatus {
        if !canonical_status.disconnected {
            return canonical_status;
        }
        let mut converged = canonical_status;
        for other_host_index in 0..self.hosts.len() {
            if other_host_index == canonical_host_index {
                continue;
            }
            if self.host_is_disconnect_pending(other_host_index) {
                continue;
            }
            if let Some(other_freeze_frame) =
                self.snapshot_freeze_frame(other_host_index, frame, player_index)
            {
                converged.last_frame = std::cmp::min(converged.last_frame, other_freeze_frame);
            }
        }
        converged
    }

    /// Folds `host_index`'s raw forwarded connect-status report into the per-host
    /// epoch watermark ([`Self::host_status_epoch`]) and disconnect-witness table
    /// ([`Self::host_drop_witness`]).
    ///
    /// For every slot: advance the host's epoch high-water by the report's
    /// [`ConnectionStatus::epoch`] (monotone max, ALL reports — connected or
    /// disconnected). Then, for a slot the host reports **disconnected** whose
    /// epoch is NOT strictly below that high-water (i.e. it is the current cycle,
    /// not a reordered earlier one), record/refresh the witness: a strictly newer
    /// epoch resets it to the new drop cycle; the same epoch max-merges the freeze
    /// (retransmits / pre-convergence-high views). A drop report at a strictly
    /// older epoch is a reordered earlier-cycle packet and is ignored — closing
    /// the cross-cycle stale-re-arm fail-open that a freeze-only witness could not
    /// discriminate.
    fn witness_host_status_reports(
        &mut self,
        host_index: usize,
        status_snapshot: &[ConnectionStatus],
    ) {
        let (Some(witness_row), Some(epoch_row)) = (
            self.host_drop_witness.get_mut(host_index),
            self.host_status_epoch.get_mut(host_index),
        ) else {
            return;
        };
        for (player_index, status) in status_snapshot.iter().enumerate() {
            let Some(seen) = epoch_row.get_mut(player_index) else {
                continue;
            };
            // A report whose epoch is strictly below the high-water is a
            // reordered earlier-cycle packet.
            let is_stale = status.epoch < *seen;
            *seen = (*seen).max(status.epoch);
            if !status.disconnected || is_stale {
                continue;
            }
            let Some(slot) = witness_row.get_mut(player_index) else {
                continue;
            };
            *slot = Some(match *slot {
                // Strictly newer cycle: reset to this drop.
                Some(witness) if status.epoch > witness.epoch => DropWitness {
                    freeze: status.last_frame,
                    epoch: status.epoch,
                },
                // Same cycle: max-merge the freeze.
                Some(witness) if status.epoch == witness.epoch => DropWitness {
                    freeze: witness.freeze.max(status.last_frame),
                    epoch: witness.epoch,
                },
                // Older than the witness — unreachable given `!is_stale`
                // (high-water >= witness epoch), but never regress the witness.
                Some(witness) => witness,
                None => DropWitness {
                    freeze: status.last_frame,
                    epoch: status.epoch,
                },
            });
        }
    }

    /// Classifies whether a `disconnected -> connected` report from
    /// `host_index` for `player_index` may be followed as a genuine
    /// reactivation.
    ///
    /// `Witnessed` iff the spectator's slot is currently latched disconnected,
    /// AND `host_index`'s own gossip has reported that slot disconnected at a
    /// freeze frame `>=` the latched freeze (since the last followed reactivation
    /// consumed the witness for this player — [`Self::consume_drop_witnesses`]),
    /// AND the `incoming_epoch` of the connected report being merged is `>=` that
    /// witnessed drop's [`ConnectionStatus::epoch`].
    ///
    /// The freeze comparison accepts every genuine current-drop witness once the
    /// latch has converged at or below that host's view (the host reported the
    /// drop at its own possibly-higher view; the latch converges DOWN). The
    /// EPOCH comparison is what discriminates drop CYCLES, which the freeze
    /// cannot: an earlier cycle's pre-convergence-high freeze can numerically
    /// cover a later drop's converged (global-min) freeze (a rejoin re-bases
    /// `last_frame` to `activation_frame - 1`). A genuine reactivation bumps the
    /// generation strictly past the drop it re-opens, so its connected report
    /// carries `epoch > witnessed.epoch`; a reordered PRE-drop connected snapshot
    /// carries the older pre-drop epoch (`< witnessed.epoch`) and is rejected
    /// (the within-cycle fail-open). Paired with the witness's own stale-drop
    /// rejection in [`Self::witness_host_status_reports`], a reordered earlier
    /// drop can no longer re-arm consumed provenance either (the cross-cycle
    /// fail-open). `>=` (not `>`) is deliberate: in the legacy uniform-epoch
    /// world every report shares epoch `0`, so the comparison is inert and the
    /// gate reduces to the freeze-only behavior — no regression for peers that
    /// do not arm the epoch.
    ///
    /// One transient micro-window still fails closed (the safe direction,
    /// self-correcting): a genuine witness whose own freeze sits BELOW a
    /// still-unconverged latch — possible only when all its drop-era arrivals
    /// are retransmits masked by first-writer staging — is blocked until its next
    /// fresh drop-bearing packet converges the latch down. When the slot is not
    /// latched disconnected the returned value is inert (only the
    /// `(disconnected, connected)` merge arm consults it).
    fn reactivation_provenance(
        &self,
        host_index: usize,
        player_index: usize,
        incoming_epoch: u16,
    ) -> ReactivationProvenance {
        let witness = self
            .host_drop_witness
            .get(host_index)
            .and_then(|row| row.get(player_index))
            .copied()
            .flatten();
        let Some(witness) = witness else {
            return ReactivationProvenance::Unwitnessed;
        };
        match self.host_connect_status.get(player_index) {
            Some(current)
                if current.disconnected
                    && witness.freeze >= current.last_frame
                    && incoming_epoch >= witness.epoch =>
            {
                ReactivationProvenance::Witnessed
            },
            Some(_) | None => ReactivationProvenance::Unwitnessed,
        }
    }

    /// Consumes every host's disconnect witness for `player_index` after a
    /// reactivation FOLLOW: provenance for the drop that was just re-opened
    /// must not survive into the next drop cycle, because a host's own
    /// (possibly pre-convergence-high) view of THIS drop can numerically cover
    /// the NEXT drop's converged freeze and would otherwise authorize
    /// resurrecting it. A live host that witnesses the next drop
    /// re-establishes its witness on its next drop-bearing packet (gossip
    /// rides every packet); a host whose drop reports all arrived before the
    /// follow fails closed at the frozen label, the safe direction — except
    /// when the spectator itself authors the next freeze by committing that
    /// host's stale staged drop snapshot, in which case the commit re-arms the
    /// committing host ([`Self::witness_adopted_drop`]) so the freeze stays
    /// re-openable.
    fn consume_drop_witnesses(&mut self, player_index: usize) {
        for witness in &mut self.host_drop_witness {
            if let Some(slot) = witness.get_mut(player_index) {
                *slot = None;
            }
        }
    }

    /// Re-arms the COMMITTING host's disconnect witness at the adopted freeze
    /// frame and generation (mirroring [`Self::witness_host_status_reports`]'s
    /// newer-resets / same-max-merges policy) when the `(connected,
    /// disconnected)` ADOPT arm of [`merge_connection_status`] fires at commit
    /// time.
    ///
    /// Needed because witnessing is otherwise arrival-time only
    /// ([`Self::handle_host_input`]) while adoption happens at COMMIT time: a
    /// followed reactivation can consume the witness table between the two, so
    /// a stale STAGED drop snapshot that then commits would re-freeze the slot
    /// with the committing host's witness left `None` — a spectator-authored
    /// freeze that the host's own later connected gossip could never re-open
    /// (permanent freeze). Re-arming the committing host at the freeze it just
    /// latched keeps the spectator's adopt self-consistent: per-host in-order
    /// delivery guarantees that host's later connected report postdates this
    /// drop in its own stream, so following it merely undoes the spectator's
    /// own adopt. See the adopt-arm interaction notes on
    /// [`merge_connection_status`] for the analyzed soundness cost.
    fn witness_adopted_drop(
        &mut self,
        host_index: usize,
        player_index: usize,
        freeze: Frame,
        epoch: u16,
    ) {
        if let Some(slot) = self
            .host_drop_witness
            .get_mut(host_index)
            .and_then(|witness| witness.get_mut(player_index))
        {
            *slot = Some(match *slot {
                Some(witness) if epoch > witness.epoch => DropWitness { freeze, epoch },
                Some(witness) if epoch == witness.epoch => DropWitness {
                    freeze: witness.freeze.max(freeze),
                    epoch: witness.epoch,
                },
                Some(witness) => witness,
                None => DropWitness { freeze, epoch },
            });
        }
    }

    fn try_commit_ready_frames(&mut self) {
        self.try_commit_ready_frames_with_pending_host(None);
    }

    fn try_commit_ready_frames_with_pending_host(
        &mut self,
        pending_host_to_include: Option<usize>,
    ) {
        loop {
            if self.spectator_divergence.is_some() {
                return;
            }

            let Some(next_frame) = self.last_recv_frame.checked_add(1) else {
                return;
            };
            let canonical_host_index = (0..self.hosts.len())
                .find(|&index| !self.host_is_disconnect_pending(index))
                .or(pending_host_to_include);
            let Some(canonical_host_index) = canonical_host_index else {
                return;
            };
            if !self.snapshot_is_complete(canonical_host_index, next_frame) {
                return;
            }

            if self.detect_snapshot_disagreement(
                canonical_host_index,
                next_frame,
                !self.has_surviving_host(),
            ) {
                return;
            }

            self.commit_canonical_snapshot(canonical_host_index, next_frame);
        }
    }

    fn detect_snapshot_disagreement(
        &mut self,
        canonical_host_index: usize,
        frame: Frame,
        compare_disconnect_pending: bool,
    ) -> bool {
        let primary_addr = match self.hosts.get(canonical_host_index) {
            Some(host) => host.peer_addr(),
            None => return false,
        };

        for host_index in 0..self.hosts.len() {
            if host_index == canonical_host_index {
                continue;
            }
            if self.host_is_disconnect_pending(host_index) && !compare_disconnect_pending {
                continue;
            }
            let conflicting_addr = match self.hosts.get(host_index) {
                Some(host) => host.peer_addr(),
                None => continue,
            };

            for player_index in 0..self.num_players {
                let Some(primary_input) =
                    self.snapshot_input(canonical_host_index, frame, player_index)
                else {
                    continue;
                };
                let Some(conflicting_input) = self.snapshot_input(host_index, frame, player_index)
                else {
                    continue;
                };
                if !primary_input.equal(&conflicting_input, true) {
                    self.latch_spectator_divergence(
                        frame,
                        PlayerHandle::new(player_index),
                        primary_addr,
                        conflicting_addr,
                    );
                    return true;
                }
            }
        }

        false
    }

    fn commit_canonical_snapshot(&mut self, host_index: usize, frame: Frame) {
        let Some(buffer_index) = frame.buffer_index(self.buffer_size) else {
            return;
        };

        for player_index in 0..self.num_players {
            let Some(input) = self.snapshot_input(host_index, frame, player_index) else {
                return;
            };
            if let Some(slot) = self
                .inputs
                .get_mut(buffer_index)
                .and_then(|frame_inputs| frame_inputs.get_mut(player_index))
            {
                *slot = input;
            } else {
                report_violation_to!(
                    &self.violation_observer,
                    ViolationSeverity::Error,
                    ViolationKind::InternalError,
                    "spectator: canonical input slot missing for player {} at frame {}",
                    player_index,
                    frame
                );
                return;
            }
        }

        for player_index in 0..self.num_players {
            // Re-borrow per player so `converged_drop_status` (which borrows
            // `&self.host_snapshots`/`&self.hosts`) does not conflict with the
            // mutable `host_connect_status` borrow below.
            let Some(canonical_status) = self
                .host_snapshots
                .get(host_index)
                .and_then(|host| host.get(buffer_index))
                .and_then(Option::as_ref)
                .and_then(|snapshot| snapshot.status.get(player_index).copied())
            else {
                report_violation_to!(
                    &self.violation_observer,
                    ViolationSeverity::Error,
                    ViolationKind::InternalError,
                    "spectator: canonical status missing for player {} at frame {}",
                    player_index,
                    frame
                );
                return;
            };
            // Converge a dropped slot's freeze frame DOWN to the global minimum
            // across every connected host with a staged snapshot at this frame —
            // not just the canonical host. Without this, a host that permanently
            // holds the canonical role while reporting a HIGHER freeze frame makes
            // the spectator replay `self.inputs[F_high]`, a frozen value from the
            // non-overlapping region that the lower-`F` host never vouched for
            // (the c25fc1f asymmetric-loss desync, spectator-side).
            let status =
                self.converged_drop_status(host_index, frame, player_index, canonical_status);
            let provenance = self.reactivation_provenance(host_index, player_index, status.epoch);
            let outcome = self.host_connect_status.get_mut(player_index).map_or(
                MergeOutcome::NoTransition,
                |slot| {
                    // Reactivation-safe merge rather than raw overwrite: the canonical
                    // host can oscillate across frames (redundant hosts failing over
                    // under asymmetric loss), so a later canonical host that reports a
                    // dropped slot disconnected at a HIGHER last_frame would otherwise
                    // raise this spectator's already-frozen freeze frame and push the
                    // input-status path back to `Confirmed` for frames the mesh already
                    // froze. The merge converges a dropped slot's freeze `last_frame`
                    // DOWN to the mesh global-min and never raises it — the spectator
                    // analog of the protocol `on_input` convergence (audit F4,
                    // Session 20). It still FOLLOWS a genuine disconnected->connected
                    // reactivation (so hot-join re-opens are tracked), but only from a
                    // canonical host whose own gossip has witnessed the latched drop
                    // (`reactivation_provenance`): a stale lagging host that never
                    // observed the drop and becomes canonical can no longer resurrect
                    // a permanently-dropped slot's label (the critic-#1 reactivation
                    // residual). A fresh drop is still adopted. The only consumer of
                    // `host_connect_status` that this affects is the `inputs_at_frame`
                    // input-status path: the spectator never fills `pending_output`
                    // for these host endpoints, so `host.poll` reads the slice but
                    // never transmits it.
                    merge_connection_status(slot, status, provenance)
                },
            );
            match outcome {
                MergeOutcome::FollowedReactivation => {
                    // A follow consumes every host's witness for this player:
                    // the re-opened drop's provenance (possibly a
                    // pre-convergence-high view) must not be allowed to
                    // authorize re-opening the NEXT drop, whose converged
                    // freeze it can numerically cover.
                    self.consume_drop_witnesses(player_index);
                },
                MergeOutcome::AdoptedDrop => {
                    // An adopt re-arms the COMMITTING host's witness at the
                    // adopted freeze: the spectator just made this host's
                    // report the latch, so the host's own later connected
                    // report (which postdates the drop in its in-order stream)
                    // must stay able to undo the spectator's own adopt — even
                    // when a follow consumed the arrival-time witness between
                    // staging and commit (`witness_adopted_drop`).
                    self.witness_adopted_drop(
                        host_index,
                        player_index,
                        status.last_frame,
                        status.epoch,
                    );
                },
                MergeOutcome::NoTransition => {},
            }
        }

        if let Some(host) = self.hosts.get_mut(host_index) {
            host.update_local_frame_advantage(frame);
            if let Some(slot) = self.canonical_hosts.get_mut(buffer_index) {
                *slot = Some(CanonicalFrameHost {
                    frame,
                    addr: host.peer_addr(),
                });
            }
        }

        self.last_recv_frame = frame;
    }

    fn handle_host_input(
        &mut self,
        host_index: usize,
        input: PlayerInput<T::Input>,
        player: PlayerHandle,
        status_snapshot: Vec<ConnectionStatus>,
        addr: T::Address,
    ) {
        // Validate frame before using as index - negative frames would wrap around
        if input.frame.is_null() || input.frame.as_i32() < 0 {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::FrameSync,
                "Received input with invalid frame {:?} for player {} - ignoring",
                input.frame,
                player
            );
            return;
        }

        // Validate player handle is in bounds
        if player.as_usize() >= self.num_players {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::InternalError,
                "Received input for player {} but only {} players configured - ignoring",
                player,
                self.num_players
            );
            return;
        }

        // Record this host's own gossip into the per-host disconnect-witness
        // table BEFORE staging: every connect-status observation from a host
        // flows through this single chokepoint (staged snapshots are derived
        // from `status_snapshot`), so the table sees each host's stream
        // regardless of which host is canonical, of arrival order, and of the
        // first-writer-wins staged-snapshot policy below (which would mask a
        // retransmitted frame's newer status).
        self.witness_host_status_reports(host_index, &status_snapshot);

        let Some(frame_index) = input.frame.buffer_index(self.buffer_size) else {
            return;
        };

        let mut same_host_conflict = false;
        {
            let Some(host_ring) = self.host_snapshots.get_mut(host_index) else {
                report_violation!(
                    ViolationSeverity::Warning,
                    ViolationKind::InternalError,
                    "Received input from unknown host index {} - ignoring",
                    host_index
                );
                return;
            };
            let Some(slot) = host_ring.get_mut(frame_index) else {
                report_violation!(
                    ViolationSeverity::Warning,
                    ViolationKind::InternalError,
                    "Failed to stage input at frame {} - frame index {} out of bounds",
                    input.frame,
                    frame_index
                );
                return;
            };
            if !matches!(slot, Some(snapshot) if snapshot.frame == input.frame) {
                let Ok(snapshot) =
                    HostFrameSnapshot::new(input.frame, self.num_players, status_snapshot)
                else {
                    return;
                };
                *slot = Some(snapshot);
            }

            let Some(snapshot) = slot.as_mut() else {
                return;
            };
            let Some(player_slot) = snapshot.inputs.get_mut(player.as_usize()) else {
                return;
            };
            if let Some(existing_input) = player_slot {
                same_host_conflict = !existing_input.equal(&input, true);
            } else {
                *player_slot = Some(input);
            }
        }

        if same_host_conflict {
            self.latch_spectator_divergence(input.frame, player, addr.clone(), addr);
            return;
        }

        let host_disconnect_pending = self.host_is_disconnect_pending(host_index);
        let compare_disconnect_pending = host_disconnect_pending && !self.has_surviving_host();
        if (!host_disconnect_pending || compare_disconnect_pending)
            && self.detect_staged_input_disagreement(
                host_index,
                input,
                player,
                addr,
                compare_disconnect_pending,
            )
        {
            return;
        }

        // Late-arrival convergence: if this host's dropped-slot freeze frame is
        // LOWER than the freeze frame the spectator already latched (e.g. the
        // higher-`F` host raced ahead and committed first), converge DOWN so the
        // dropped slot replays the global-min frozen value. Commit-time
        // `converged_drop_status` only sees hosts already staged when the frame
        // commits; this closes the non-overlapping ordering where the lower-`F`
        // host arrives after the commit (the c25fc1f asymmetric-loss desync).
        if !host_disconnect_pending {
            self.converge_latched_drop_status(host_index, input.frame);
        }

        if input.frame > self.last_recv_frame {
            self.try_commit_ready_frames_with_pending_host(
                host_disconnect_pending.then_some(host_index),
            );
        }
    }

    /// Converges the spectator's latched per-player freeze frame DOWN to a
    /// connected host's lower reported freeze frame for an already-observed frame.
    ///
    /// Mirrors [`Self::converged_drop_status`] for the late-arrival ordering: a
    /// host whose dropped-slot freeze frame is lower than the latched
    /// `host_connect_status` value lowers it (never raises), so the dropped slot
    /// replays the global-min frozen value regardless of canonical-host selection
    /// or arrival order. Only lowers an already-disconnected slot; it never
    /// reactivates, re-drops, or raises a freeze frame.
    fn converge_latched_drop_status(&mut self, host_index: usize, frame: Frame) {
        for player_index in 0..self.num_players {
            let Some(host_freeze_frame) =
                self.snapshot_freeze_frame(host_index, frame, player_index)
            else {
                continue;
            };
            if let Some(slot) = self.host_connect_status.get_mut(player_index) {
                if slot.disconnected && host_freeze_frame < slot.last_frame {
                    slot.last_frame = host_freeze_frame;
                }
            }
        }
    }

    /// Handles a single protocol event originating from `host_index`.
    ///
    /// Returns `Some(host_index)` if the event was an [`Event::Disconnected`],
    /// signalling that this host should be removed from [`Self::hosts`] after all
    /// events for this poll have been processed. Returns `None` otherwise.
    fn handle_event(
        &mut self,
        host_index: usize,
        event: Event<T>,
        addr: T::Address,
    ) -> Option<usize> {
        let mut disconnected_host = None;
        match event {
            // forward to user
            Event::Synchronizing {
                total,
                count,
                total_requests_sent,
                elapsed_ms,
            } => {
                self.event_queue.push_back(FortressEvent::Synchronizing {
                    addr,
                    total,
                    count,
                    total_requests_sent,
                    elapsed_ms,
                });
            },
            // forward to user
            Event::NetworkInterrupted { disconnect_timeout } => {
                self.event_queue
                    .push_back(FortressEvent::NetworkInterrupted {
                        addr,
                        disconnect_timeout,
                    });
            },
            // forward to user
            Event::NetworkResumed => {
                self.event_queue
                    .push_back(FortressEvent::NetworkResumed { addr });
            },
            // synced with a host, then forward to user. The first host to sync flips
            // the session to Running; subsequent hosts are idempotent.
            Event::Synchronized => {
                self.state = SessionState::Running;
                self.event_queue
                    .push_back(FortressEvent::Synchronized { addr });
            },
            // disconnect the host, then forward to user. The host is removed by the
            // caller after all events have been handled.
            Event::Disconnected => {
                disconnected_host = Some(host_index);
                self.event_queue
                    .push_back(FortressEvent::Disconnected { addr });
            },
            // forward sync timeout to user
            Event::SyncTimeout { elapsed_ms } => {
                self.event_queue
                    .push_back(FortressEvent::SyncTimeout { addr, elapsed_ms });
            },
            // add the input and all associated information
            Event::Input {
                input,
                player,
                peer_connect_status,
            } => {
                self.handle_host_input(host_index, input, player, peer_connect_status, addr);
            },
        }

        // check event queue size and discard oldest events if too big
        self.trim_event_queue();

        disconnected_host
    }
}

/// Removes the entries at `disconnected_hosts` (sorted, deduplicated host
/// indices) from a host-index-parallel table, keeping every surviving entry in
/// its original relative order so the table stays aligned with
/// [`SpectatorSession::hosts`] after the removal re-indexes survivors.
fn retain_surviving_hosts<E>(entries: &mut Vec<E>, disconnected_hosts: &[usize]) {
    let mut host_index = 0;
    entries.retain(|_entry| {
        let should_remove = disconnected_hosts.binary_search(&host_index).is_ok();
        host_index += 1;
        !should_remove
    });
}

/// A host's most-recent forwarded drop of a slot, recorded in
/// [`SpectatorSession::host_drop_witness`]: the freeze frame it dropped at and
/// the [`ConnectionStatus::epoch`] generation of that drop. The epoch lets
/// [`SpectatorSession::reactivation_provenance`] discriminate drop cycles a bare
/// freeze frame cannot (an earlier cycle's pre-convergence-high freeze can
/// numerically cover a later cycle's converged freeze).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DropWitness {
    /// Highest freeze `last_frame` reported for this drop cycle.
    freeze: Frame,
    /// The [`ConnectionStatus::epoch`] generation of this drop.
    epoch: u16,
}

/// Whether a host's `disconnected -> connected` report may be followed as a
/// genuine reactivation. Computed per (canonical host, player) by
/// [`SpectatorSession::reactivation_provenance`] from the per-host
/// disconnect-witness table; only the `(disconnected, connected)` arm of
/// [`merge_connection_status`] consults it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReactivationProvenance {
    /// The reporting host's own gossip previously reported this slot
    /// disconnected at (or above) the spectator's latched freeze frame, so its
    /// connected report is a genuine re-open and must be followed.
    Witnessed,
    /// The reporting host never reported this slot disconnected for the
    /// latched drop; its connected report is indistinguishable from a stale
    /// pre-drop view and must not resurrect the latched drop.
    Unwitnessed,
}

/// What [`merge_connection_status`] did to the spectator's persistent slot,
/// reported so the commit path can maintain the disconnect-witness table:
/// a follow must consume every host's witness for the player
/// ([`SpectatorSession::consume_drop_witnesses`]) and an adopt must re-arm the
/// COMMITTING host's witness at the adopted freeze
/// ([`SpectatorSession::witness_adopted_drop`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MergeOutcome {
    /// The `(disconnected, connected)` arm followed a witnessed reactivation.
    FollowedReactivation,
    /// The `(connected, disconnected)` arm latched a fresh drop at the
    /// incoming freeze frame.
    AdoptedDrop,
    /// No state transition: the freeze frame converged down, the connected
    /// `last_frame` advanced, or an unwitnessed connected report was blocked.
    NoTransition,
}

/// Fold a canonical host's per-player connect-status into the spectator's
/// persistent view. This is NOT a pure monotonic latch — it must follow a
/// genuine reactivation so a hot-join spectator does not freeze a slot that the
/// live mesh has brought back. The contract, by `(current, incoming)` state:
///
/// - both disconnected: converge the freeze `last_frame` DOWN to the global
///   minimum. Never raise an already-frozen slot's freeze frame — a later
///   canonical host (redundant hosts failing over under asymmetric loss) must
///   not push the spectator's input-status path back to `Confirmed` for frames
///   the mesh already froze. This is the always-safe part of the fix and the
///   spectator analog of the protocol `on_input` convergence (audit F4,
///   Session 20).
/// - `current` disconnected, `incoming` connected: FOLLOW the reactivation
///   (`*current = incoming`) — but only when `provenance` is `Witnessed`, i.e.
///   the reporting host's own gossip previously reported this slot
///   disconnected for the latched drop. A hot-join host that re-opens a
///   dropped slot transitions its own `local_connect_status` through the drop
///   and gossips both states to spectators (`p2p_session.rs`), so a genuine
///   re-open always comes from a disconnect witness; a stale lagging host that
///   never observed the drop is `Unwitnessed` and must NOT resurrect a
///   permanently-dropped slot's label when it becomes the canonical source
///   (the critic-#1 reactivation residual, closed via per-host provenance —
///   no "usually-right" heuristic). A followed re-open CONSUMES every host's
///   witness for the player ([`SpectatorSession::consume_drop_witnesses`]) so
///   the re-opened cycle's provenance — including a pre-convergence-high view
///   that numerically covers a later drop's converged freeze — cannot
///   authorize re-opening the next drop.
/// - `current` connected, `incoming` disconnected: adopt the drop at the
///   incoming freeze frame.
/// - both connected: advance the `last_frame` via max.
///
/// Returns the [`MergeOutcome`] so the caller can maintain the witness table:
/// after a [`MergeOutcome::FollowedReactivation`] it must consume the witness
/// table for this player ([`SpectatorSession::consume_drop_witnesses`]) so the
/// re-opened drop's provenance cannot leak into the next drop cycle, and after
/// a [`MergeOutcome::AdoptedDrop`] it must re-arm the committing host's
/// witness at the adopted freeze ([`SpectatorSession::witness_adopted_drop`])
/// so a spectator-authored freeze remains re-openable by its own author (see
/// the adopt-arm interaction below).
///
/// Cross-cycle / within-cycle discrimination is by the per-slot
/// [`ConnectionStatus::epoch`] generation the owning host arms on every
/// `connected <-> disconnected` transition (`P2PSession::arm_status_epoch`),
/// tracked per (host, player) as a high-water in
/// [`SpectatorSession::host_status_epoch`] and recorded with each drop in
/// [`DropWitness::epoch`]. The two formerly fail-open corners (Session 31's
/// documented residuals) are CLOSED by it:
///
/// - **Cross-cycle (CLOSED):** a drop-era report from an EARLIER cycle that
///   ARRIVES only after a follow consumed the witnesses can no longer re-arm
///   stale provenance — [`SpectatorSession::witness_host_status_reports`] only
///   records a drop whose epoch is NOT strictly below the host's high-water, and
///   a host that forwarded the intervening reactivation has already advanced its
///   high-water past the stale drop's generation, so the re-arm is rejected.
/// - **Within-cycle (CLOSED):** a host's own REORDERED PRE-drop connected
///   snapshot carries the older pre-drop generation, so the
///   `incoming_epoch >= witness.epoch` clause in
///   [`SpectatorSession::reactivation_provenance`] rejects it (a genuine
///   reactivation strictly postdates the drop that bumped the generation).
///
/// Residual (documented, NARROW): one fail-closed micro-window remains (the safe
/// direction, self-correcting) — a genuine reactivation gossiped only by hosts
/// whose drop-era packets never reached this spectator, or only before the
/// previous follow consumed the witness with no drop-bearing packet after it, is
/// not followed; the spectator keeps the frozen label until the host's next
/// drop-bearing packet re-establishes provenance. (In the legacy uniform-epoch
/// world — peers that never arm the epoch, all reports at generation `0` — the
/// `>=` gate and the not-strictly-below witness test are inert, so the gate
/// reduces exactly to the pre-epoch freeze-only behavior, with the old
/// transient corners; no regression for un-upgraded peers.)
///
/// Adopt-arm interaction (commit-time re-arm): the `(connected, disconnected)`
/// ADOPT arm is unconditional, so a stale STAGED drop snapshot that commits
/// after a follow consumed the witness table re-freezes the just-reactivated
/// slot — a freeze the SPECTATOR authored, leaving the committing host's own
/// witness `None`. The mitigation re-arms the COMMITTING host's witness at the
/// adopted freeze and generation (`witness_adopted_drop`, mirroring the
/// newer-resets / same-max-merges policy) whenever the adopt arm fires: the
/// spectator just made that host's report the latch, and per-host in-order
/// delivery means that host's later connected report postdates this drop in its
/// own stream (carrying a strictly newer generation), so following it merely
/// undoes the spectator's own adopt. The late-arrival path
/// ([`SpectatorSession::converge_latched_drop_status`]) needs no analogous
/// re-arm: it only LOWERS an already-disconnected slot's freeze frame (never
/// adopts from connected), and lowering the latch only widens the set of
/// witnesses that cover it.
fn merge_connection_status(
    current: &mut ConnectionStatus,
    incoming: ConnectionStatus,
    provenance: ReactivationProvenance,
) -> MergeOutcome {
    match (current.disconnected, incoming.disconnected) {
        (true, true) => {
            current.last_frame = std::cmp::min(current.last_frame, incoming.last_frame);
            MergeOutcome::NoTransition
        },
        (true, false) => match provenance {
            ReactivationProvenance::Witnessed => {
                // Follow the genuine reactivation; preserves hot-join re-open
                // (no regression vs the old raw overwrite).
                *current = incoming;
                MergeOutcome::FollowedReactivation
            },
            ReactivationProvenance::Unwitnessed => {
                // Stale-connected gossip from a host that never witnessed the
                // latched drop: keep the frozen label. The mesh froze every
                // frame past `current.last_frame`, so resurrecting here would
                // silently replay unfrozen values for frozen frames.
                MergeOutcome::NoTransition
            },
        },
        (false, true) => {
            current.disconnected = true;
            current.last_frame = incoming.last_frame;
            // Carry the adopting host's drop generation onto the latch. The latch
            // epoch is informational (the reactivation gate reads the per-host
            // witness/incoming epochs, never the latch); this just keeps it
            // faithful to the drop the spectator latched.
            current.epoch = incoming.epoch;
            MergeOutcome::AdoptedDrop
        },
        (false, false) => {
            current.last_frame = std::cmp::max(current.last_frame, incoming.last_frame);
            MergeOutcome::NoTransition
        },
    }
}

impl<T: Config> fmt::Debug for SpectatorSession<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SpectatorSession")
            .field("state", &self.state)
            .field("num_players", &self.num_players)
            .field("num_hosts", &self.hosts.len())
            .field("current_frame", &self.current_frame)
            .field("last_recv_frame", &self.last_recv_frame)
            .field("buffer_size", &self.buffer_size)
            .field("max_frames_behind", &self.max_frames_behind)
            .field("catchup_speed", &self.catchup_speed)
            .field("stream_delay", &self.stream_delay)
            .field("enable_rewind", &self.enable_rewind)
            .finish_non_exhaustive()
    }
}

impl<T: Config> Session<T> for SpectatorSession<T> {
    fn advance_frame(&mut self) -> FortressResult<RequestVec<T>> {
        Self::advance_frame(self)
    }

    fn local_player_handle_required(&self) -> FortressResult<PlayerHandle> {
        Self::local_player_handle_required(self)
    }

    fn add_local_input(
        &mut self,
        player_handle: PlayerHandle,
        input: T::Input,
    ) -> FortressResult<()> {
        Self::add_local_input(self, player_handle, input)
    }

    fn events(&mut self) -> EventDrain<'_, T> {
        Self::events(self)
    }

    fn current_state(&self) -> SessionState {
        Self::current_state(self)
    }

    fn poll_remote_clients(&mut self) {
        Self::poll_remote_clients(self)
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::needless_collect
)]
mod tests {
    use super::*;
    use crate::network::{
        compression,
        messages::{Input, MessageBody, MessageHeader},
    };
    use crate::{Config, Message, NonBlockingSocket, SessionBuilder};
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    /// A minimal test configuration for unit testing.
    struct TestConfig;

    impl Config for TestConfig {
        type Input = u8;
        type State = u8;
        type Address = SocketAddr;
    }

    fn test_addr(port: u16) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)
    }

    /// A dummy socket that doesn't actually send or receive messages.
    /// Used for unit testing without network dependencies.
    struct DummySocket;

    impl NonBlockingSocket<SocketAddr> for DummySocket {
        fn send_to(&mut self, _msg: &Message, _addr: &SocketAddr) {}
        fn receive_all_messages(&mut self) -> Vec<(SocketAddr, Message)> {
            Vec::new()
        }
    }

    fn spectator_input_message(
        frame: Frame,
        player_inputs: [u8; 2],
        peer_connect_status: Vec<ConnectionStatus>,
    ) -> Message {
        spectator_input_message_with_disconnect(frame, player_inputs, peer_connect_status, false)
    }

    fn spectator_input_message_with_disconnect(
        frame: Frame,
        player_inputs: [u8; 2],
        peer_connect_status: Vec<ConnectionStatus>,
        disconnect_requested: bool,
    ) -> Message {
        let input_bytes = vec![player_inputs[0], player_inputs[1]];
        let encoded = compression::encode(&[0_u8; 2], std::iter::once(&input_bytes));

        Message {
            header: MessageHeader { magic: 1 },
            body: MessageBody::Input(Input {
                peer_connect_status,
                disconnect_requested,
                start_frame: frame,
                ack_frame: Frame::NULL,
                bytes: encoded,
            }),
        }
    }

    fn queue_host_input(
        session: &mut SpectatorSession<TestConfig>,
        host_index: usize,
        frame: Frame,
        player_inputs: [u8; 2],
        peer_connect_status: Vec<ConnectionStatus>,
    ) {
        let msg = spectator_input_message(frame, player_inputs, peer_connect_status);
        session.hosts[host_index].force_running_for_tests();
        session.hosts[host_index].handle_message(&msg);
    }

    fn queue_host_disconnect_input(
        session: &mut SpectatorSession<TestConfig>,
        host_index: usize,
        frame: Frame,
        player_inputs: [u8; 2],
        peer_connect_status: Vec<ConnectionStatus>,
    ) {
        let msg = spectator_input_message_with_disconnect(
            frame,
            player_inputs,
            peer_connect_status,
            true,
        );
        session.hosts[host_index].force_running_for_tests();
        session.hosts[host_index].handle_message(&msg);
    }

    // Helper function to create a spectator session for testing
    fn create_test_spectator_session() -> Option<SpectatorSession<TestConfig>> {
        SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .start_spectator_session(test_addr(7000), DummySocket)
    }

    fn create_test_spectator_session_with_config(
        num_players: usize,
        buffer_size: usize,
        max_frames_behind: usize,
        catchup_speed: usize,
    ) -> Option<SpectatorSession<TestConfig>> {
        use crate::SpectatorConfig;
        SessionBuilder::new()
            .with_num_players(num_players)
            .unwrap()
            .with_spectator_config(SpectatorConfig {
                buffer_size,
                catchup_speed,
                max_frames_behind,
                ..SpectatorConfig::default()
            })
            .start_spectator_session(test_addr(7001), DummySocket)
    }

    // ==========================================
    // Constructor Tests
    // ==========================================

    #[test]
    fn spectator_session_creates_successfully() {
        let session = create_test_spectator_session();
        assert!(session.is_some());
    }

    /// Regression for defect D9 (PLAN.md §2) on the **spectator** session: its
    /// event-queue trim used to discard undrained events silently, just like
    /// the P2P session. The overflow now increments [`SessionMetrics`] (total +
    /// per-[`EventKind`](crate::metrics::EventKind)) and reports a single
    /// rate-limited `Warning` to the configured violation observer.
    #[test]
    fn spectator_event_queue_overflow_records_discard_telemetry() {
        use crate::metrics::EventKind;
        use crate::telemetry::CollectingObserver;

        let observer = Arc::new(CollectingObserver::new());
        let mut session = SessionBuilder::<TestConfig>::new()
            .with_num_players(2)
            .unwrap()
            .with_event_queue_size(10)
            .unwrap()
            .with_violation_observer(Arc::clone(&observer) as Arc<dyn ViolationObserver>)
            .start_spectator_session(test_addr(7100), DummySocket)
            .expect("spectator session builds");

        assert_eq!(session.metrics().events_discarded_total, 0);

        let addr = test_addr(9000);
        // Canary at the front: a safety-critical Disconnected.
        session
            .event_queue
            .push_back(FortressEvent::Disconnected { addr });
        // Push past the cap with benign events so the canary overflows out.
        for _ in 0..session.max_event_queue_size {
            session
                .event_queue
                .push_back(FortressEvent::NetworkResumed { addr });
        }
        session.trim_event_queue();

        assert_eq!(
            session.event_queue.len(),
            session.max_event_queue_size,
            "queue must be trimmed to its cap"
        );
        let metrics = session.metrics();
        assert!(
            metrics.events_discarded_total >= 1,
            "overflow must count discarded events; got {}",
            metrics.events_discarded_total
        );
        assert_eq!(
            metrics
                .events_discarded_by_kind
                .get(EventKind::Disconnected),
            1,
            "the safety-critical Disconnected canary must be attributed to its kind"
        );
        let violations = observer.violations();
        assert!(
            violations
                .iter()
                .any(|v| v.severity == ViolationSeverity::Warning
                    && v.kind == ViolationKind::NetworkProtocol
                    && v.message.contains("spectator event queue overflow")),
            "expected a rate-limited spectator overflow Warning; observed: {violations:?}"
        );
    }

    /// The spectator's overflow `Warning` is rate-limited per overflow episode
    /// (mirror of the P2P contract): several trim passes within one drain gap
    /// yield a single `Warning`, and `events()` re-arms it. The per-kind
    /// counters keep incrementing regardless.
    #[test]
    fn spectator_event_queue_overflow_warning_is_rate_limited_per_drain_gap() {
        use crate::telemetry::CollectingObserver;

        let observer = Arc::new(CollectingObserver::new());
        let mut session = SessionBuilder::<TestConfig>::new()
            .with_num_players(2)
            .unwrap()
            .with_event_queue_size(10)
            .unwrap()
            .with_violation_observer(Arc::clone(&observer) as Arc<dyn ViolationObserver>)
            .start_spectator_session(test_addr(7101), DummySocket)
            .expect("spectator session builds");

        let addr = test_addr(9001);
        let overflow_warnings = |observer: &CollectingObserver| {
            observer
                .violations()
                .iter()
                .filter(|v| {
                    v.severity == ViolationSeverity::Warning
                        && v.kind == ViolationKind::NetworkProtocol
                        && v.message.contains("spectator event queue overflow")
                })
                .count()
        };

        // First episode: several overflowing trim passes, no drain between them.
        for _ in 0..5 {
            for _ in 0..(session.max_event_queue_size + 1) {
                session
                    .event_queue
                    .push_back(FortressEvent::NetworkResumed { addr });
            }
            session.trim_event_queue();
        }
        assert!(
            session.metrics().events_discarded_total >= 5,
            "the passes must discard repeatedly; got {}",
            session.metrics().events_discarded_total
        );
        assert_eq!(
            overflow_warnings(&observer),
            1,
            "many discards within one drain gap must yield a single Warning"
        );

        // Draining re-arms the rate limiter.
        let _ = session.events();
        for _ in 0..(session.max_event_queue_size + 1) {
            session
                .event_queue
                .push_back(FortressEvent::NetworkResumed { addr });
        }
        session.trim_event_queue();
        assert_eq!(
            overflow_warnings(&observer),
            2,
            "a fresh drain must re-arm the overflow Warning"
        );
    }

    #[test]
    fn advance_capacity_clamps_pathological_frames_to_advance() {
        // A pathological catchup_speed (usize::MAX) must not blow up the
        // preallocation: the result is clamped to buffer_size, then doubled
        // for rewind.
        let capacity = SpectatorSession::<TestConfig>::advance_capacity(usize::MAX, 60, true);
        assert_eq!(capacity, 120);

        let capacity_no_rewind =
            SpectatorSession::<TestConfig>::advance_capacity(usize::MAX, 60, false);
        assert_eq!(capacity_no_rewind, 60);
    }

    #[test]
    fn advance_capacity_normal_cases_return_expected_small_values() {
        // Normal operation: frames_to_advance == 1.
        assert_eq!(
            SpectatorSession::<TestConfig>::advance_capacity(1, 60, false),
            1
        );
        assert_eq!(
            SpectatorSession::<TestConfig>::advance_capacity(1, 60, true),
            2
        );
        // Catchup below the buffer bound passes through (doubled with rewind).
        assert_eq!(
            SpectatorSession::<TestConfig>::advance_capacity(4, 60, false),
            4
        );
        assert_eq!(
            SpectatorSession::<TestConfig>::advance_capacity(4, 60, true),
            8
        );
    }

    #[test]
    fn advance_capacity_does_not_overflow_with_huge_buffer_size() {
        // Even when buffer_size itself is huge, the *2 rewind doubling uses
        // saturating arithmetic and must not panic.
        let capacity =
            SpectatorSession::<TestConfig>::advance_capacity(usize::MAX, usize::MAX, true);
        assert_eq!(capacity, usize::MAX);
    }

    #[test]
    fn spectator_session_with_custom_config() {
        let session = create_test_spectator_session_with_config(4, 120, 20, 3);
        assert!(session.is_some());
        let session = session.unwrap();
        assert_eq!(session.num_players(), 4);
    }

    #[test]
    fn spectator_session_single_player() {
        let session = create_test_spectator_session_with_config(1, 60, 10, 1);
        assert!(session.is_some());
        let session = session.unwrap();
        assert_eq!(session.num_players(), 1);
    }

    #[test]
    fn spectator_session_many_players() {
        let session = create_test_spectator_session_with_config(8, 60, 10, 1);
        assert!(session.is_some());
        let session = session.unwrap();
        assert_eq!(session.num_players(), 8);
    }

    // ==========================================
    // State and Getter Tests
    // ==========================================

    #[test]
    fn spectator_session_initial_state_is_synchronizing() {
        let session = create_test_spectator_session().unwrap();
        assert_eq!(session.current_state(), SessionState::Synchronizing);
    }

    #[test]
    fn spectator_session_initial_frame_is_null() {
        let session = create_test_spectator_session().unwrap();
        assert_eq!(session.current_frame(), Frame::NULL);
    }

    #[test]
    fn spectator_session_num_players_returns_correct_count() {
        let session2 = create_test_spectator_session_with_config(2, 60, 10, 1).unwrap();
        assert_eq!(session2.num_players(), 2);

        let session4 = create_test_spectator_session_with_config(4, 60, 10, 1).unwrap();
        assert_eq!(session4.num_players(), 4);
    }

    #[test]
    fn spectator_session_frames_behind_host_initially_zero() {
        let session = create_test_spectator_session().unwrap();
        // Both last_recv_frame and current_frame start at NULL (Frame(-1))
        // so frames_behind_host should be 0
        assert_eq!(session.frames_behind_host(), 0);
    }

    #[test]
    fn spectator_session_frames_behind_host_uses_wide_distance_math() {
        let mut session = create_test_spectator_session().unwrap();
        session.current_frame = Frame::NULL;
        session.last_recv_frame = Frame::new(i32::MAX);

        assert_eq!(
            session.frames_behind_host(),
            2_147_483_648,
            "distance from NULL to i32::MAX should not overflow i32 math"
        );
    }

    #[test]
    fn spectator_session_viewable_frame_clamps_to_null_until_delay_is_available() {
        let mut session = create_test_spectator_session().unwrap();
        session.stream_delay = 5;

        session.last_recv_frame = Frame::NULL;
        assert_eq!(session.viewable_frame(), Frame::NULL);

        session.last_recv_frame = Frame::new(3);
        assert_eq!(session.viewable_frame(), Frame::NULL);

        session.last_recv_frame = Frame::new(5);
        assert_eq!(session.viewable_frame(), Frame::new(0));
    }

    #[test]
    fn spectator_session_single_host_count_can_drop_to_zero() {
        let mut session = create_test_spectator_session().unwrap();
        assert_eq!(session.num_hosts(), 1);

        session.remove_disconnected_hosts(vec![0]);

        assert_eq!(session.num_hosts(), 0);
    }

    // ==========================================
    // advance_frame Tests
    // ==========================================

    #[test]
    fn spectator_session_advance_frame_returns_not_synchronized_when_not_running() {
        let mut session = create_test_spectator_session().unwrap();

        // Session starts in Synchronizing state
        let result = session.advance_frame();
        assert!(result.is_err());
        assert!(matches!(result, Err(FortressError::NotSynchronized)));
    }

    // ==========================================
    // network_stats Tests
    // ==========================================

    #[test]
    fn spectator_session_network_stats_returns_not_synchronized_initially() {
        let session = create_test_spectator_session().unwrap();

        // Network stats should fail when not synchronized
        let result = session.network_stats();
        assert!(result.is_err());
    }

    // ==========================================
    // events Tests
    // ==========================================

    #[test]
    fn spectator_session_events_initially_empty() {
        let mut session = create_test_spectator_session().unwrap();
        let events: Vec<_> = session.events().collect();
        assert!(events.is_empty());
    }

    #[test]
    fn spectator_session_events_drains_queue() {
        let mut session = create_test_spectator_session().unwrap();

        // First call to events
        let events1: Vec<_> = session.events().collect();
        assert!(events1.is_empty());

        // Second call should also be empty (queue was drained)
        let events2: Vec<_> = session.events().collect();
        assert!(events2.is_empty());
    }

    // ==========================================
    // violation_observer Tests
    // ==========================================

    #[test]
    fn spectator_session_violation_observer_none_by_default() {
        let session = create_test_spectator_session().unwrap();
        assert!(session.violation_observer().is_none());
    }

    #[test]
    fn spectator_session_with_violation_observer() {
        use crate::telemetry::CollectingObserver;

        let observer = Arc::new(CollectingObserver::new());
        let session: Option<SpectatorSession<TestConfig>> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .with_violation_observer(observer)
            .start_spectator_session(test_addr(7002), DummySocket);

        let session = session.unwrap();
        assert!(session.violation_observer().is_some());
    }

    // ==========================================
    // poll_remote_clients Tests
    // ==========================================

    #[test]
    fn spectator_session_poll_remote_clients_does_not_panic() {
        let mut session = create_test_spectator_session().unwrap();

        // Polling should not panic even with no messages
        session.poll_remote_clients();

        // State should still be synchronizing (no sync messages received)
        assert_eq!(session.current_state(), SessionState::Synchronizing);
    }

    #[test]
    fn spectator_session_poll_remote_clients_multiple_times() {
        let mut session = create_test_spectator_session().unwrap();

        // Multiple polls should not cause issues
        for _ in 0..10 {
            session.poll_remote_clients();
        }

        assert_eq!(session.current_state(), SessionState::Synchronizing);
    }

    // ==========================================
    // SpectatorConfig Tests
    // ==========================================

    #[test]
    fn spectator_config_default_values() {
        use crate::SpectatorConfig;

        let config = SpectatorConfig::default();
        assert_eq!(config.buffer_size, 60);
        assert_eq!(config.catchup_speed, 1);
        assert_eq!(config.max_frames_behind, 10);
    }

    #[test]
    fn spectator_config_new_equals_default() {
        use crate::SpectatorConfig;

        let new_config = SpectatorConfig::new();
        let default_config = SpectatorConfig::default();
        assert_eq!(new_config, default_config);
    }

    #[test]
    fn spectator_config_fast_paced_preset() {
        use crate::SpectatorConfig;

        let config = SpectatorConfig::fast_paced();
        assert_eq!(config.buffer_size, 90);
        assert_eq!(config.catchup_speed, 2);
        assert_eq!(config.max_frames_behind, 15);
    }

    #[test]
    fn spectator_config_slow_connection_preset() {
        use crate::SpectatorConfig;

        let config = SpectatorConfig::slow_connection();
        assert_eq!(config.buffer_size, 120);
        assert_eq!(config.catchup_speed, 1);
        assert_eq!(config.max_frames_behind, 20);
    }

    #[test]
    fn spectator_config_local_preset() {
        use crate::SpectatorConfig;

        let config = SpectatorConfig::local();
        assert_eq!(config.buffer_size, 30);
        assert_eq!(config.catchup_speed, 2);
        assert_eq!(config.max_frames_behind, 5);
    }

    #[test]
    fn spectator_config_equality() {
        use crate::SpectatorConfig;

        let a = SpectatorConfig {
            buffer_size: 100,
            catchup_speed: 2,
            max_frames_behind: 15,
            ..SpectatorConfig::default()
        };
        let b = SpectatorConfig {
            buffer_size: 100,
            catchup_speed: 2,
            max_frames_behind: 15,
            ..SpectatorConfig::default()
        };
        assert_eq!(a, b);
    }

    #[test]
    fn spectator_config_inequality() {
        use crate::SpectatorConfig;

        let a = SpectatorConfig::default();
        let b = SpectatorConfig::fast_paced();
        assert_ne!(a, b);
    }

    #[test]
    fn spectator_config_clone() {
        use crate::SpectatorConfig;

        let original = SpectatorConfig::fast_paced();
        let cloned = original;
        assert_eq!(original, cloned);
    }

    #[test]
    fn spectator_config_debug_format() {
        use crate::SpectatorConfig;

        let config = SpectatorConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("SpectatorConfig"));
        assert!(debug_str.contains("buffer_size"));
        assert!(debug_str.contains("60"));
    }

    #[test]
    fn spectator_config_all_presets_are_distinct() {
        use crate::SpectatorConfig;

        let default = SpectatorConfig::default();
        let fast_paced = SpectatorConfig::fast_paced();
        let slow_connection = SpectatorConfig::slow_connection();
        let local = SpectatorConfig::local();

        // All presets should be different
        assert_ne!(default, fast_paced);
        assert_ne!(default, slow_connection);
        assert_ne!(default, local);
        assert_ne!(fast_paced, slow_connection);
        assert_ne!(fast_paced, local);
        assert_ne!(slow_connection, local);
    }

    // ==========================================
    // Edge Case Tests
    // ==========================================

    #[test]
    fn spectator_session_with_minimum_buffer_size() {
        // Buffer size of 1 should work (edge case)
        let session = create_test_spectator_session_with_config(2, 1, 10, 1);
        assert!(session.is_some());
    }

    #[test]
    fn spectator_session_with_zero_buffer_size_is_rejected() {
        let session = create_test_spectator_session_with_config(2, 0, 10, 1);
        assert!(session.is_none());
    }

    #[test]
    fn spectator_session_with_large_buffer_size() {
        let session = create_test_spectator_session_with_config(2, 1000, 10, 1);
        assert!(session.is_some());
    }

    #[test]
    fn spectator_session_with_high_catchup_speed() {
        let session = create_test_spectator_session_with_config(2, 60, 10, 10);
        assert!(session.is_some());
    }

    #[test]
    fn spectator_session_with_zero_max_frames_behind() {
        // Zero max_frames_behind means always in catchup mode
        let session = create_test_spectator_session_with_config(2, 60, 0, 2);
        assert!(session.is_some());
    }

    // ==========================================
    // Internal State Tests
    // ==========================================

    #[test]
    fn spectator_config_stream_delay_boundary_is_validated() {
        use crate::SpectatorConfig;

        let valid = SessionBuilder::<TestConfig>::new()
            .with_num_players(2)
            .unwrap()
            .with_spectator_config(SpectatorConfig {
                buffer_size: 8,
                stream_delay: 7,
                ..SpectatorConfig::default()
            })
            .start_spectator_session(test_addr(7400), DummySocket);
        assert!(valid.is_some());

        let invalid_single = SessionBuilder::<TestConfig>::new()
            .with_num_players(2)
            .unwrap()
            .with_spectator_config(SpectatorConfig {
                buffer_size: 8,
                stream_delay: 8,
                ..SpectatorConfig::default()
            })
            .start_spectator_session(test_addr(7401), DummySocket);
        assert!(invalid_single.is_none());

        let invalid_multi = SessionBuilder::<TestConfig>::new()
            .with_num_players(2)
            .unwrap()
            .with_spectator_config(SpectatorConfig {
                buffer_size: 8,
                stream_delay: 8,
                ..SpectatorConfig::default()
            })
            .start_spectator_session_multi(&[test_addr(7402), test_addr(7403)], DummySocket);
        assert!(invalid_multi.is_none());
    }

    #[test]
    fn spectator_config_builders_do_not_impose_arbitrary_buffer_caps() {
        use crate::SpectatorConfig;

        let large_buffer = SessionBuilder::<TestConfig>::new()
            .with_num_players(2)
            .unwrap()
            .with_spectator_config(SpectatorConfig {
                buffer_size: 4_097,
                ..SpectatorConfig::default()
            })
            .start_spectator_session(test_addr(7404), DummySocket);
        assert!(large_buffer.is_some());

        let invalid_delay = SessionBuilder::<TestConfig>::new()
            .with_num_players(2)
            .unwrap()
            .with_spectator_config(SpectatorConfig {
                buffer_size: 4_096,
                stream_delay: 4_096,
                ..SpectatorConfig::default()
            })
            .start_spectator_session_multi(&[test_addr(7405), test_addr(7406)], DummySocket);
        assert!(invalid_delay.is_none());
    }

    #[test]
    fn spectator_session_host_connect_status_initialized() {
        // Verify that host_connect_status is initialized for all players
        let session = create_test_spectator_session_with_config(4, 60, 10, 1).unwrap();
        // We can't directly check host_connect_status, but we can verify
        // the session was created with the correct number of players
        assert_eq!(session.num_players(), 4);
    }

    #[test]
    fn spectator_session_last_recv_frame_initially_null() {
        let session = create_test_spectator_session().unwrap();
        // last_recv_frame starts at NULL (Frame(-1)), which means
        // frames_behind_host should be 0 (since current_frame is also NULL)
        assert_eq!(session.frames_behind_host(), 0);
    }

    // ==========================================
    // NORMAL_SPEED Constant Test
    // ==========================================

    #[test]
    fn normal_speed_is_one() {
        // NORMAL_SPEED constant should be 1 for smooth playback
        assert_eq!(NORMAL_SPEED, 1);
    }

    // ==========================================
    // Current Frame Tests
    // ==========================================

    #[test]
    fn spectator_session_current_frame_is_null_initially() {
        let session = create_test_spectator_session().unwrap();
        assert!(session.current_frame().is_null());
        assert_eq!(session.current_frame(), Frame::NULL);
    }

    // ==========================================
    // Session State Tests
    // ==========================================

    #[test]
    fn spectator_session_state_transitions() {
        // Session starts in Synchronizing state
        let session = create_test_spectator_session().unwrap();
        assert_eq!(session.current_state(), SessionState::Synchronizing);

        // We can't easily transition to Running without a real network connection,
        // but we verify the initial state is correct
    }

    // ==========================================
    // SpectatorConfig Builder Tests
    // ==========================================

    #[test]
    fn spectator_config_with_zero_catchup_speed() {
        use crate::SpectatorConfig;

        // Catchup speed of 0 is technically valid (no frames advanced in catchup)
        let config = SpectatorConfig {
            buffer_size: 60,
            catchup_speed: 0,
            max_frames_behind: 10,
            ..SpectatorConfig::default()
        };
        assert_eq!(config.catchup_speed, 0);
    }

    #[test]
    fn spectator_config_extreme_values() {
        use crate::SpectatorConfig;

        // Test with extreme values
        let config = SpectatorConfig {
            buffer_size: usize::MAX,
            catchup_speed: usize::MAX,
            max_frames_behind: usize::MAX,
            ..SpectatorConfig::default()
        };
        assert_eq!(config.buffer_size, usize::MAX);
        assert_eq!(config.catchup_speed, usize::MAX);
        assert_eq!(config.max_frames_behind, usize::MAX);
    }

    // ==========================================
    // Multiple Poll Tests
    // ==========================================

    #[test]
    fn spectator_session_poll_preserves_state() {
        let mut session = create_test_spectator_session().unwrap();

        // Record initial state
        let initial_state = session.current_state();
        let initial_frame = session.current_frame();

        // Poll multiple times
        for _ in 0..5 {
            session.poll_remote_clients();
        }

        // State should not change without actual network events
        assert_eq!(session.current_state(), initial_state);
        assert_eq!(session.current_frame(), initial_frame);
    }

    #[test]
    fn spectator_session_events_empty_after_drain() {
        let mut session = create_test_spectator_session().unwrap();

        // Drain events
        let events: Vec<_> = session.events().collect();
        assert!(events.is_empty());

        // Poll and drain again
        session.poll_remote_clients();
        let events: Vec<_> = session.events().collect();
        assert!(events.is_empty());
    }

    // ==========================================
    // Network Stats Edge Cases
    // ==========================================

    #[test]
    fn spectator_session_network_stats_before_sync() {
        let session = create_test_spectator_session().unwrap();

        // Should fail when not synchronized
        let result = session.network_stats();
        assert!(result.is_err());
        assert!(matches!(result, Err(FortressError::NotSynchronized)));
    }

    // ==========================================
    // Violation Observer Tests
    // ==========================================

    #[test]
    fn spectator_session_violation_observer_is_arc() {
        use crate::telemetry::CollectingObserver;

        let observer = Arc::new(CollectingObserver::new());
        let observer_clone = Arc::clone(&observer);

        let session: Option<SpectatorSession<TestConfig>> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .with_violation_observer(observer)
            .start_spectator_session(test_addr(7003), DummySocket);

        let session = session.unwrap();

        // Observer should be accessible
        assert!(session.violation_observer().is_some());

        // The clone should still be usable (Arc reference counting)
        assert_eq!(observer_clone.violations().len(), 0);
    }

    #[test]
    fn spectator_session_without_violation_observer() {
        let session = create_test_spectator_session().unwrap();
        assert!(session.violation_observer().is_none());
    }

    // ==========================================
    // Frames Behind Host Edge Cases
    // ==========================================

    #[test]
    fn spectator_session_frames_behind_with_both_null() {
        let session = create_test_spectator_session().unwrap();
        // Both last_recv_frame and current_frame are NULL
        // NULL - NULL = 0, so frames_behind should be 0
        assert_eq!(session.frames_behind_host(), 0);
    }

    #[test]
    fn spectator_inputs_at_frame_stale_player_slot_returns_prediction_threshold() {
        let mut session = create_test_spectator_session().unwrap();
        let frame = Frame::new(5);
        let buffer_index = frame.buffer_index(session.buffer_size).unwrap();

        session.inputs[buffer_index][0] = PlayerInput::new(frame, 10);
        session.inputs[buffer_index][1] = PlayerInput::new(Frame::new(4), 20);

        let result = session.inputs_at_frame(frame);

        assert!(matches!(result, Err(FortressError::PredictionThreshold)));
    }

    #[test]
    fn spectator_inputs_at_frame_newer_wrapped_player_slot_returns_too_far_behind() {
        use crate::telemetry::CollectingObserver;

        let observer = Arc::new(CollectingObserver::new());
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .with_violation_observer(observer.clone())
            .start_spectator_session(test_addr(7300), DummySocket)
            .unwrap();
        let frame = Frame::new(5);
        let newer_wrapped_frame = Frame::new(frame.as_i32() + session.buffer_size as i32);
        let buffer_index = frame.buffer_index(session.buffer_size).unwrap();

        session.inputs[buffer_index][0] = PlayerInput::new(frame, 10);
        session.inputs[buffer_index][1] = PlayerInput::new(newer_wrapped_frame, 20);

        let result = session.inputs_at_frame(frame);

        assert!(matches!(result, Err(FortressError::SpectatorTooFarBehind)));
        assert!(observer
            .violations()
            .iter()
            .any(|violation| violation.kind == ViolationKind::FrameSync));
    }

    #[test]
    fn spectator_frames_behind_host_routes_report_violation_to_session_observer() {
        use crate::telemetry::CollectingObserver;

        let observer = Arc::new(CollectingObserver::new());
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .with_violation_observer(observer.clone())
            .start_spectator_session(test_addr(7350), DummySocket)
            .unwrap();

        // Force the defensive guard `current_frame > last_recv_frame`. That site
        // uses the plain `report_violation!` macro (which routes through the
        // thread-local scoped observer), unlike the session's `report_violation_to!`
        // sites which carry the observer explicitly. Without a scope installed at
        // this entry point the violation reaches only the default `TracingObserver`
        // and never the per-session observer configured via
        // `with_violation_observer` — the gap this test pins.
        session.current_frame = Frame::new(10);
        session.last_recv_frame = Frame::new(5);

        let behind = session.frames_behind_host();
        assert_eq!(
            behind, 0,
            "frames_behind_host returns 0 when current_frame exceeds last_recv_frame"
        );

        // NON-VACUITY: deleting the `let _violation_scope = self.scoped_violation_observer();`
        // line from `frames_behind_host` makes this assertion fail.
        assert!(
            observer
                .violations()
                .iter()
                .any(|violation| violation.kind == ViolationKind::FrameSync),
            "frames_behind_host violation must route to the per-session observer"
        );
    }

    #[test]
    fn spectator_inputs_at_frame_stale_then_newer_slot_returns_too_far_behind() {
        use crate::telemetry::CollectingObserver;

        let observer = Arc::new(CollectingObserver::new());
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(3)
            .unwrap()
            .with_violation_observer(observer.clone())
            .start_spectator_session(test_addr(7304), DummySocket)
            .unwrap();
        let frame = Frame::new(5);
        let newer_wrapped_frame = Frame::new(frame.as_i32() + session.buffer_size as i32);
        let buffer_index = frame.buffer_index(session.buffer_size).unwrap();

        session.inputs[buffer_index][0] = PlayerInput::new(Frame::new(4), 10);
        session.inputs[buffer_index][1] = PlayerInput::new(newer_wrapped_frame, 20);
        session.inputs[buffer_index][2] = PlayerInput::new(frame, 30);

        let result = session.inputs_at_frame(frame);

        assert!(matches!(result, Err(FortressError::SpectatorTooFarBehind)));
        assert!(observer
            .violations()
            .iter()
            .any(|violation| violation.kind == ViolationKind::FrameSync));
    }

    #[test]
    fn spectator_redundant_host_divergence_latches_error_and_event() {
        use crate::telemetry::CollectingObserver;

        let observer = Arc::new(CollectingObserver::new());
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .with_violation_observer(observer.clone())
            .start_spectator_session_multi(&[test_addr(7301), test_addr(7302)], DummySocket)
            .unwrap();
        session.state = SessionState::Running;
        let frame = Frame::new(0);

        queue_host_input(
            &mut session,
            0,
            frame,
            [11, 22],
            vec![ConnectionStatus::default(); 2],
        );
        session.poll_remote_clients();
        let buffer_index = frame.buffer_index(session.buffer_size).unwrap();
        assert_eq!(session.inputs[buffer_index][0].input, 11_u8);

        queue_host_input(
            &mut session,
            1,
            frame,
            [99, 22],
            vec![ConnectionStatus::default(); 2],
        );
        session.poll_remote_clients();

        assert!(observer
            .violations()
            .iter()
            .any(|violation| violation.kind == ViolationKind::FrameSync));
        assert!(session.events().any(|event| {
            matches!(
                event,
                FortressEvent::SpectatorDivergence {
                    frame: event_frame,
                    player,
                    ..
                } if event_frame == frame && player == PlayerHandle::new(0)
            )
        }));
        assert!(matches!(
            session.advance_frame(),
            Err(FortressError::SpectatorDivergence {
                frame: event_frame,
                player,
            }) if event_frame == frame && player == PlayerHandle::new(0)
        ));
    }

    #[test]
    fn spectator_partial_host_input_conflict_latches_after_canonical_commit() {
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .start_spectator_session_multi(&[test_addr(7305), test_addr(7306)], DummySocket)
            .unwrap();
        let frame = Frame::new(0);

        session.handle_host_input(
            0,
            PlayerInput::new(frame, 11),
            PlayerHandle::new(0),
            vec![ConnectionStatus::default(); 2],
            test_addr(7305),
        );
        session.handle_host_input(
            0,
            PlayerInput::new(frame, 22),
            PlayerHandle::new(1),
            vec![ConnectionStatus::default(); 2],
            test_addr(7305),
        );
        assert_eq!(session.last_recv_frame, frame);

        session.handle_host_input(
            1,
            PlayerInput::new(frame, 99),
            PlayerHandle::new(0),
            vec![ConnectionStatus::default(); 2],
            test_addr(7306),
        );

        assert!(matches!(
            session.advance_frame(),
            Err(FortressError::SpectatorDivergence {
                frame: event_frame,
                player,
            }) if event_frame == frame && player == PlayerHandle::new(0)
        ));
    }

    // Completeness-Critic #2 arbitration (A): divergent dropped-slot streams that
    // land on DIFFERENT frames per host (non-overlapping at commit time). Host 0
    // commits frames 0..=2 ahead of any other host; then host 1 forwards a
    // DIVERGENT player-0 value for one of those already-committed frames. The
    // claim is that the staged-disagreement first loop returns None (no overlap)
    // and divergence is silently missed. This test pins the ACTUAL behavior of the
    // SECOND block (committed-frame comparison) in detect_staged_input_disagreement
    // and is NON-VACUOUS: mutating the `committed_input.equal(&input, true)` guard
    // to a no-op makes it fail (confirmed during arbitration).
    #[test]
    fn spectator_nonoverlapping_divergent_late_stream_latches_divergence() {
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .start_spectator_session_multi(
                &[test_addr(7401), test_addr(7402), test_addr(7403)],
                DummySocket,
            )
            .unwrap();
        let status = vec![ConnectionStatus::default(); 2];

        // Host 0 races ahead and the spectator commits frames 0,1,2 from it alone.
        // Hosts 1 and 2 have NOT delivered these frames yet, so the staged
        // first-loop comparison finds None and the canonical-commit comparison
        // (detect_snapshot_disagreement) also finds None for the other hosts.
        for f in 0..=2 {
            let frame = Frame::new(f);
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(frame, 11),
                PlayerHandle::new(0),
                status.clone(),
                test_addr(7401),
            );
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(frame, 22),
                PlayerHandle::new(1),
                status.clone(),
                test_addr(7401),
            );
        }
        assert_eq!(session.last_recv_frame, Frame::new(2));

        // Now host 1 forwards a DIVERGENT player-0 value (99) for frame 1, which is
        // already committed (1 <= last_recv_frame == 2). Non-overlapping at commit.
        // test:
        session.handle_host_input(
            1,
            PlayerInput::new(Frame::new(1), 99),
            PlayerHandle::new(0),
            status,
            test_addr(7402),
        );

        assert!(
            session.spectator_divergence.is_some(),
            "non-overlapping late divergent stream must be caught by the committed-frame block"
        );
        assert!(matches!(
            session.advance_frame(),
            Err(FortressError::SpectatorDivergence {
                frame: event_frame,
                player,
            }) if event_frame == Frame::new(1) && player == PlayerHandle::new(0)
        ));
    }

    // Completeness-Critic #2 arbitration (B): the c25fc1f playback analog. Player 1
    // is the dropped slot. Two hosts froze it at DIFFERENT freeze frames and the
    // committed value at the AGREED freeze frame F is what the spectator's
    // freeze-bypass (`frozen_input_at`) plays back for every frame > F. This test
    // proves the freeze-bypass introduces no NEW silent divergence: the value
    // played back at frames > F is exactly `self.inputs[F]`, whose correctness is
    // guarded by the same disagreement detection at frame F. Host 1 commits the
    // agreed freeze frame F=1 (canonical, value 50). Host 0 then forwards a
    // DIVERGENT value (60) for the SAME frame F=1; the late committed-frame block
    // catches it before any frame > F can play back a divergent frozen value.
    #[test]
    fn spectator_frozen_slot_divergent_value_at_freeze_frame_latches_before_playback() {
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .start_spectator_session_multi(&[test_addr(7411), test_addr(7412)], DummySocket)
            .unwrap();
        // Player 1 dropped at freeze frame F = 1 on both hosts.
        let dropped = ConnectionStatus {
            disconnected: true,
            last_frame: Frame::new(1),
            epoch: 0,
        };
        let status = vec![ConnectionStatus::default(), dropped];

        // Frame 0: agreed by both (player 1 still confirmed for setup).
        // Host 0 is canonical and commits frames 0 and 1 first (value 50 at F=1).
        for (frame, p1) in [(Frame::new(0), 40_u8), (Frame::new(1), 50_u8)] {
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(frame, 11),
                PlayerHandle::new(0),
                status.clone(),
                test_addr(7411),
            );
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(frame, p1),
                PlayerHandle::new(1),
                status.clone(),
                test_addr(7411),
            );
        }
        assert_eq!(session.last_recv_frame, Frame::new(1));
        let f1_index = Frame::new(1).buffer_index(session.buffer_size).unwrap();
        // Committed value at freeze frame F=1 is host 0's value (50). The freeze
        // playback would surface THIS value for every frame > 1.
        assert_eq!(session.inputs[f1_index][1].input, 50_u8);

        // Host 0 advances to frame 2 so F=1 < live edge (freeze playback active).
        // test:
        session.handle_host_input(
            0,
            PlayerInput::new(Frame::new(2), 12),
            PlayerHandle::new(0),
            status.clone(),
            test_addr(7411),
        );
        // (player 1 is dropped; host stops forwarding new player-1 inputs)
        assert_eq!(session.last_recv_frame, Frame::new(1));

        // Host 1 NOW forwards its DIVERGENT frozen value (60) for the freeze frame
        // F=1. This is the value the mesh actually agreed; if it is silently
        // dropped, the spectator plays back host 0's stale 50 for every frame > 1.
        // test:
        session.handle_host_input(
            1,
            PlayerInput::new(Frame::new(1), 60),
            PlayerHandle::new(1),
            status,
            test_addr(7412),
        );

        assert!(
            session.spectator_divergence.is_some(),
            "divergent value at the freeze frame must latch before freeze playback can serve it"
        );
        assert!(matches!(
            session.advance_frame(),
            Err(FortressError::SpectatorDivergence {
                frame: event_frame,
                player,
            }) if event_frame == Frame::new(1) && player == PlayerHandle::new(1)
        ));
    }

    // Completeness-Critic #2 arbitration (C): the TRUE non-overlapping gap. The
    // dropped slot (player 1) is frozen at DIFFERENT freeze frames per host under
    // asymmetric loss. Host 0 (ALWAYS canonical, index 0, connected) froze HIGH at
    // F_A = 2 with value 50. Host 1 received the dropped peer only through F_B = 1
    // and froze at the mesh-agreed value 60 (the global-min freeze frame). Because
    // host 0 is canonical, the spectator's host_connect_status freezes at F_A = 2,
    // and the freeze-bypass plays back self.inputs[2] (= 50) for every frame >= 2.
    // Host 1 never forwarded player 1 at frame 2 (its last received was frame 1),
    // so there is NO host-1 snapshot at frame 2 to compare against -- the
    // non-overlapping region. The mesh value 60 (host 1's frozen value at its lower
    // freeze frame) is never reconciled. If divergence is silently missed, this is
    // the spectator analog of the c25fc1f asymmetric-loss desync.
    #[test]
    fn spectator_asymmetric_freeze_frame_nonoverlapping_region_converges_to_global_min() {
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .start_spectator_session_multi(&[test_addr(7421), test_addr(7422)], DummySocket)
            .unwrap();

        // Frame 0: both hosts agree, player 1 still connected.
        let connected = vec![ConnectionStatus::default(); 2];
        // test:
        session.handle_host_input(
            0,
            PlayerInput::new(Frame::new(0), 11),
            PlayerHandle::new(0),
            connected.clone(),
            test_addr(7421),
        );
        // test:
        session.handle_host_input(
            0,
            PlayerInput::new(Frame::new(0), 40),
            PlayerHandle::new(1),
            connected.clone(),
            test_addr(7421),
        );
        assert_eq!(session.last_recv_frame, Frame::new(0));

        // Host 0 froze player 1 HIGH at F_A = 2 (drop detected late). It forwards
        // player 1 through frame 2 with value 50, reporting disconnected@last_frame=2.
        let dropped_at_2 = vec![
            ConnectionStatus::default(),
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(2),
                epoch: 0,
            },
        ];
        // CRITICAL: at the OVERLAPPING frame 1 host 0 forwards the SAME value the
        // mesh agreed (60) so the overlap region is clean. Only at frame 2 (the
        // NON-overlapping region, where host 1 never forwarded player 1) does host
        // 0's later-frozen value 50 appear. This isolates the divergence to the
        // non-overlapping frozen region exactly as the finding describes.
        for (frame, p1) in [(Frame::new(1), 60_u8), (Frame::new(2), 50_u8)] {
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(frame, 12),
                PlayerHandle::new(0),
                dropped_at_2.clone(),
                test_addr(7421),
            );
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(frame, p1),
                PlayerHandle::new(1),
                dropped_at_2.clone(),
                test_addr(7421),
            );
        }
        assert_eq!(session.last_recv_frame, Frame::new(2));

        // Host 1 received player 1 only through F_B = 1 and froze at the mesh-agreed
        // value 60 (global-min freeze). It forwards player 0 through frame 2 (to
        // overlap host 0 on player 0) and player 1 ONLY through frame 1 with 60.
        let dropped_at_1 = vec![
            ConnectionStatus::default(),
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(1),
                epoch: 0,
            },
        ];
        // test:
        session.handle_host_input(
            1,
            PlayerInput::new(Frame::new(0), 11),
            PlayerHandle::new(0),
            connected.clone(),
            test_addr(7422),
        );
        // test:
        session.handle_host_input(
            1,
            PlayerInput::new(Frame::new(0), 40),
            PlayerHandle::new(1),
            connected,
            test_addr(7422),
        );
        // Host 1's DIVERGENT frozen player-1 value 60 at its freeze frame F_B = 1.
        // test:
        session.handle_host_input(
            1,
            PlayerInput::new(Frame::new(1), 12),
            PlayerHandle::new(0),
            dropped_at_1.clone(),
            test_addr(7422),
        );
        // test:
        session.handle_host_input(
            1,
            PlayerInput::new(Frame::new(1), 60),
            PlayerHandle::new(1),
            dropped_at_1.clone(),
            test_addr(7422),
        );
        // Host 1 forwards player 0 at frame 2 (overlap on the live slot) but NOT
        // player 1 (frozen below frame 2).
        // test:
        session.handle_host_input(
            1,
            PlayerInput::new(Frame::new(2), 12),
            PlayerHandle::new(0),
            dropped_at_1,
            test_addr(7422),
        );

        // Without the global-min freeze-frame convergence, the spectator silently
        // freezes player 1 at host 0's HIGH frame F_A = 2 and replays
        // self.inputs[2][1] = 50 for every frame >= 2 while the mesh agreed on 60
        // -- a silent cross-host desync in the non-overlapping region. With the fix,
        // the freeze frame converges DOWN to the global-min F_B = 1 across all
        // connected hosts and the spectator replays the mesh-agreed value 60.
        assert!(
            session.spectator_divergence.is_none(),
            "asymmetric freeze frames must converge, not fail closed (matches the \
             existing convergence design)"
        );
        assert_eq!(
            session.host_connect_status[1].last_frame,
            Frame::new(1),
            "freeze frame must converge DOWN to the global-min F_B = 1"
        );
        let played_frame2 = session.inputs_at_frame(Frame::new(2)).unwrap();
        assert_eq!(
            played_frame2[1].0, 60_u8,
            "dropped slot must replay the mesh-agreed frozen value (60), not host 0's stale 50"
        );
        assert_eq!(played_frame2[1].1, InputStatus::Disconnected);
    }

    // Completeness-Critic #2 coverage (commit-time path). Same asymmetric-loss
    // mesh as `spectator_asymmetric_freeze_frame_nonoverlapping_region_converges_to_global_min`,
    // but the distinguishing ORDERING exercises `converged_drop_status` (the
    // commit-time fold), NOT `converge_latched_drop_status` (the late-arrival fold).
    // Here the non-canonical lower-`F` host (host 1, F_B = 1) has its snapshots for
    // EVERY frame STAGED BEFORE the canonical higher-`F` host (host 0, F_A = 2)
    // commits them. When host 0 finally arrives and the spectator commits frame 2,
    // host 1's frame-2 snapshot (player 1 disconnected@last_frame=1) is already
    // staged, so the commit-time fold lowers host 0's freeze frame DOWN to the
    // global-min F_B = 1 at COMMIT time -- before any late-arrival fold could run.
    // (`host_connect_status[1]` is never disconnected while only host 1 is staged,
    // so the late-arrival path is inert throughout.) This pins the commit-time path
    // that the late-arrival test leaves unverified.
    #[test]
    fn spectator_asymmetric_freeze_frame_converges_at_commit_time_when_lower_host_staged_first() {
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .start_spectator_session_multi(&[test_addr(7431), test_addr(7432)], DummySocket)
            .unwrap();

        let connected = vec![ConnectionStatus::default(); 2];
        // Host 1 received player 1 only through F_B = 1 and froze at the mesh-agreed
        // value 60 (the global-min freeze frame).
        let dropped_at_1 = vec![
            ConnectionStatus::default(),
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(1),
                epoch: 0,
            },
        ];

        // ORDERING: deliver ALL of host 1's snapshots FIRST so they are STAGED
        // before host 0 (the canonical committer) arrives. Nothing commits yet --
        // host 0 (index 0, always connected) is canonical and not yet staged.
        // Frame 0: player 1 still connected (value 40).
        // test:
        session.handle_host_input(
            1,
            PlayerInput::new(Frame::new(0), 11),
            PlayerHandle::new(0),
            connected.clone(),
            test_addr(7432),
        );
        // test:
        session.handle_host_input(
            1,
            PlayerInput::new(Frame::new(0), 40),
            PlayerHandle::new(1),
            connected.clone(),
            test_addr(7432),
        );
        // Frame 1: host 1 freezes player 1 at F_B = 1 with the mesh-agreed value 60.
        // test:
        session.handle_host_input(
            1,
            PlayerInput::new(Frame::new(1), 12),
            PlayerHandle::new(0),
            dropped_at_1.clone(),
            test_addr(7432),
        );
        // test:
        session.handle_host_input(
            1,
            PlayerInput::new(Frame::new(1), 60),
            PlayerHandle::new(1),
            dropped_at_1.clone(),
            test_addr(7432),
        );
        // Frame 2: host 1 forwards player 0 (overlap on the live slot) but NOT
        // player 1 (frozen below frame 2). Its frame-2 snapshot still reports player
        // 1 disconnected@last_frame=1 -- the lower freeze frame the commit-time fold
        // must read when host 0 commits frame 2.
        // test:
        session.handle_host_input(
            1,
            PlayerInput::new(Frame::new(2), 12),
            PlayerHandle::new(0),
            dropped_at_1,
            test_addr(7432),
        );
        // Nothing has committed yet: the canonical host (0) is not staged.
        assert_eq!(session.last_recv_frame, Frame::NULL);
        // The late-arrival path is inert: player 1 is not yet disconnected in
        // host_connect_status because no frame has committed.
        assert!(!session.host_connect_status[1].disconnected);

        // NOW host 0 (canonical, froze player 1 HIGH at F_A = 2) arrives. Frame 0:
        // player 1 connected, value 40.
        // test:
        session.handle_host_input(
            0,
            PlayerInput::new(Frame::new(0), 11),
            PlayerHandle::new(0),
            connected.clone(),
            test_addr(7431),
        );
        // test:
        session.handle_host_input(
            0,
            PlayerInput::new(Frame::new(0), 40),
            PlayerHandle::new(1),
            connected,
            test_addr(7431),
        );
        // Host 0 froze player 1 HIGH at F_A = 2. At the OVERLAPPING frame 1 it
        // forwards the SAME mesh-agreed value (60) so the overlap region is clean;
        // only at frame 2 (the non-overlapping region) does its later-frozen value
        // 50 appear.
        let dropped_at_2 = vec![
            ConnectionStatus::default(),
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(2),
                epoch: 0,
            },
        ];
        for (frame, p1) in [(Frame::new(1), 60_u8), (Frame::new(2), 50_u8)] {
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(frame, 12),
                PlayerHandle::new(0),
                dropped_at_2.clone(),
                test_addr(7431),
            );
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(frame, p1),
                PlayerHandle::new(1),
                dropped_at_2.clone(),
                test_addr(7431),
            );
        }
        // Host 0's frame-2 arrival cascades the commit of frames 0,1,2. At the
        // commit of frame 2, host 1's frame-2 snapshot (disconnected@1) is already
        // staged, so `converged_drop_status` folds host 0's freeze frame (2) DOWN to
        // the global-min F_B = 1 at COMMIT time.
        assert_eq!(session.last_recv_frame, Frame::new(2));

        assert!(
            session.spectator_divergence.is_none(),
            "asymmetric freeze frames must converge at commit time, not fail closed"
        );
        assert_eq!(
            session.host_connect_status[1].last_frame,
            Frame::new(1),
            "commit-time fold must converge the freeze frame DOWN to the global-min F_B = 1"
        );
        let played_frame2 = session.inputs_at_frame(Frame::new(2)).unwrap();
        assert_eq!(
            played_frame2[1].0, 60_u8,
            "dropped slot must replay the mesh-agreed frozen value (60), not host 0's stale 50"
        );
        assert_eq!(played_frame2[1].1, InputStatus::Disconnected);
    }

    #[test]
    fn spectator_partial_host_input_conflict_latches_before_canonical_commit() {
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .start_spectator_session_multi(&[test_addr(7307), test_addr(7308)], DummySocket)
            .unwrap();
        let frame = Frame::new(0);

        session.handle_host_input(
            1,
            PlayerInput::new(frame, 99),
            PlayerHandle::new(0),
            vec![ConnectionStatus::default(); 2],
            test_addr(7308),
        );
        assert_eq!(session.last_recv_frame, Frame::NULL);

        session.handle_host_input(
            0,
            PlayerInput::new(frame, 11),
            PlayerHandle::new(0),
            vec![ConnectionStatus::default(); 2],
            test_addr(7307),
        );

        assert!(session.events().any(|event| {
            matches!(
                event,
                FortressEvent::SpectatorDivergence {
                    frame: event_frame,
                    player,
                    primary_addr,
                    conflicting_addr,
                } if event_frame == frame
                    && player == PlayerHandle::new(0)
                    && primary_addr == test_addr(7307)
                    && conflicting_addr == test_addr(7308)
            )
        }));
        assert!(matches!(
            session.advance_frame(),
            Err(FortressError::SpectatorDivergence {
                frame: event_frame,
                player,
            }) if event_frame == frame && player == PlayerHandle::new(0)
        ));
    }

    #[test]
    fn spectator_pending_primary_disconnect_allows_unresolved_failover_without_divergence() {
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .start_spectator_session_multi(&[test_addr(7311), test_addr(7312)], DummySocket)
            .unwrap();
        let frame = Frame::new(0);
        let status = vec![ConnectionStatus::default(); 2];

        session.handle_host_input(
            0,
            PlayerInput::new(frame, 99),
            PlayerHandle::new(0),
            status.clone(),
            test_addr(7311),
        );
        assert_eq!(session.last_recv_frame, Frame::NULL);

        session.disconnecting_hosts.push(0);
        session.handle_host_input(
            1,
            PlayerInput::new(frame, 11),
            PlayerHandle::new(0),
            status.clone(),
            test_addr(7312),
        );
        session.handle_host_input(
            1,
            PlayerInput::new(frame, 22),
            PlayerHandle::new(1),
            status,
            test_addr(7312),
        );

        assert!(session.spectator_divergence.is_none());
        assert!(!session
            .events()
            .any(|event| { matches!(event, FortressEvent::SpectatorDivergence { .. }) }));
        assert_eq!(session.last_recv_frame, frame);
        let buffer_index = frame.buffer_index(session.buffer_size).unwrap();
        assert_eq!(session.inputs[buffer_index][0].input, 11_u8);
        assert_eq!(session.inputs[buffer_index][1].input, 22_u8);
    }

    #[test]
    fn spectator_same_poll_later_disconnect_is_excluded_before_earlier_input() {
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .start_spectator_session_multi(&[test_addr(7313), test_addr(7314)], DummySocket)
            .unwrap();
        let frame = Frame::new(0);
        let status = vec![ConnectionStatus::default(); 2];

        session.handle_host_input(
            1,
            PlayerInput::new(frame, 99),
            PlayerHandle::new(0),
            status.clone(),
            test_addr(7314),
        );
        assert_eq!(session.last_recv_frame, Frame::NULL);

        queue_host_disconnect_input(&mut session, 1, frame, [99, 22], status.clone());
        queue_host_input(&mut session, 0, frame, [11, 22], status);
        session.poll_remote_clients();

        assert_eq!(session.num_hosts(), 1);
        assert!(session.spectator_divergence.is_none());
        assert!(!session
            .events()
            .any(|event| { matches!(event, FortressEvent::SpectatorDivergence { .. }) }));
        assert_eq!(session.last_recv_frame, frame);
        let buffer_index = frame.buffer_index(session.buffer_size).unwrap();
        assert_eq!(session.inputs[buffer_index][0].input, 11_u8);
        assert_eq!(session.inputs[buffer_index][1].input, 22_u8);
    }

    #[test]
    fn spectator_same_host_input_before_disconnect_is_preserved() {
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .start_spectator_session(test_addr(7315), DummySocket)
            .unwrap();
        let frame = Frame::new(0);
        let status = vec![ConnectionStatus::default(); 2];

        queue_host_input(&mut session, 0, frame, [11, 22], status.clone());
        queue_host_disconnect_input(&mut session, 0, frame, [11, 22], status);
        session.poll_remote_clients();

        assert_eq!(session.num_hosts(), 0);
        assert!(session.spectator_divergence.is_none());
        assert_eq!(session.last_recv_frame, frame);
        let buffer_index = frame.buffer_index(session.buffer_size).unwrap();
        assert_eq!(session.inputs[buffer_index][0].input, 11_u8);
        assert_eq!(session.inputs[buffer_index][1].input, 22_u8);
    }

    #[test]
    fn spectator_disconnect_packet_preserves_final_inputs() {
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .start_spectator_session(test_addr(7316), DummySocket)
            .unwrap();
        let frame = Frame::new(0);
        let status = vec![ConnectionStatus::default(); 2];

        queue_host_disconnect_input(&mut session, 0, frame, [11, 22], status);
        session.poll_remote_clients();

        assert_eq!(session.num_hosts(), 0);
        assert!(session.spectator_divergence.is_none());
        assert_eq!(session.last_recv_frame, frame);
        let buffer_index = frame.buffer_index(session.buffer_size).unwrap();
        assert_eq!(session.inputs[buffer_index][0].input, 11_u8);
        assert_eq!(session.inputs[buffer_index][1].input, 22_u8);
    }

    #[test]
    fn spectator_all_hosts_disconnect_with_conflict_latches_divergence() {
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .start_spectator_session_multi(&[test_addr(7317), test_addr(7318)], DummySocket)
            .unwrap();
        let frame = Frame::new(0);
        let status = vec![ConnectionStatus::default(); 2];

        queue_host_disconnect_input(&mut session, 0, frame, [11, 22], status.clone());
        queue_host_disconnect_input(&mut session, 1, frame, [99, 22], status);
        session.poll_remote_clients();

        assert_eq!(session.num_hosts(), 0);
        assert!(matches!(
            session.spectator_divergence,
            Some(SpectatorDivergenceState {
                frame: event_frame,
                player,
                ..
            }) if event_frame == frame && player == PlayerHandle::new(0)
        ));
        assert!(session.events().any(|event| {
            matches!(
                event,
                FortressEvent::SpectatorDivergence {
                    frame: event_frame,
                    player,
                    ..
                } if event_frame == frame && player == PlayerHandle::new(0)
            )
        }));
        assert!(matches!(
            session.advance_frame(),
            Err(FortressError::SpectatorDivergence {
                frame: event_frame,
                player,
            }) if event_frame == frame && player == PlayerHandle::new(0)
        ));
    }

    #[test]
    fn spectator_host_frame_snapshot_keeps_packet_status_per_frame() {
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .start_spectator_session_multi(&[test_addr(7309)], DummySocket)
            .unwrap();
        let frame0 = Frame::new(0);
        let frame1 = Frame::new(1);
        let status0 = vec![
            ConnectionStatus {
                disconnected: false,
                last_frame: frame0,
                epoch: 0,
            },
            ConnectionStatus {
                disconnected: false,
                last_frame: frame0,
                epoch: 0,
            },
        ];
        let status1 = vec![
            ConnectionStatus {
                disconnected: false,
                last_frame: frame1,
                epoch: 0,
            },
            ConnectionStatus {
                disconnected: true,
                last_frame: frame1,
                epoch: 0,
            },
        ];

        queue_host_input(&mut session, 0, frame0, [0, 0], status0.clone());
        queue_host_input(&mut session, 0, frame1, [1, 2], status1.clone());
        session.poll_remote_clients();

        let frame0_index = frame0.buffer_index(session.buffer_size).unwrap();
        let frame1_index = frame1.buffer_index(session.buffer_size).unwrap();
        assert_eq!(
            session.host_snapshots[0][frame0_index]
                .as_ref()
                .unwrap()
                .status,
            status0
        );
        assert_eq!(
            session.host_snapshots[0][frame1_index]
                .as_ref()
                .unwrap()
                .status,
            status1
        );
    }

    #[test]
    fn spectator_connection_status_merge_converges_freeze_and_follows_reactivation() {
        // (disconnected, disconnected) -> converge freeze frame DOWN to the min;
        // never raise an already-frozen slot's freeze frame.
        let mut both_disc = ConnectionStatus {
            disconnected: true,
            last_frame: Frame::new(5),
            epoch: 0,
        };
        let outcome = merge_connection_status(
            &mut both_disc,
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(8),
                epoch: 0,
            },
            ReactivationProvenance::Witnessed,
        );
        assert_eq!(outcome, MergeOutcome::NoTransition);
        assert_eq!(
            both_disc,
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(5),
                epoch: 0,
            },
            "both-disconnected must take the min freeze frame, never raise it"
        );
        let outcome = merge_connection_status(
            &mut both_disc,
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(3),
                epoch: 0,
            },
            ReactivationProvenance::Unwitnessed,
        );
        assert_eq!(outcome, MergeOutcome::NoTransition);
        assert_eq!(
            both_disc,
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(3),
                epoch: 0,
            },
            "both-disconnected converges further down to a lower incoming freeze"
        );

        // (disconnected, connected) + Witnessed -> FOLLOW the reactivation:
        // become connected at the incoming last_frame (preserves hot-join
        // re-open; no regression vs the old raw overwrite).
        let mut reactivating = ConnectionStatus {
            disconnected: true,
            last_frame: Frame::new(2),
            epoch: 0,
        };
        let outcome = merge_connection_status(
            &mut reactivating,
            ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(10),
                epoch: 0,
            },
            ReactivationProvenance::Witnessed,
        );
        assert_eq!(
            outcome,
            MergeOutcome::FollowedReactivation,
            "a witnessed follow must signal the caller to consume the witnesses"
        );
        assert_eq!(
            reactivating,
            ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(10),
                epoch: 0,
            },
            "disconnected+incoming-connected must follow a witnessed reactivation"
        );

        // (disconnected, connected) + Unwitnessed -> keep the frozen label: a
        // stale-connected gossip from a host that never witnessed the latched
        // drop must not resurrect it (critic #1 reactivation residual).
        let mut stale_resurrect = ConnectionStatus {
            disconnected: true,
            last_frame: Frame::new(2),
            epoch: 0,
        };
        let outcome = merge_connection_status(
            &mut stale_resurrect,
            ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(10),
                epoch: 0,
            },
            ReactivationProvenance::Unwitnessed,
        );
        assert_eq!(outcome, MergeOutcome::NoTransition);
        assert_eq!(
            stale_resurrect,
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(2),
                epoch: 0,
            },
            "disconnected+incoming-connected must NOT be followed without witness provenance"
        );

        // (connected, disconnected) -> adopt the drop at the incoming freeze frame.
        let mut newly_dropped = ConnectionStatus {
            disconnected: false,
            last_frame: Frame::new(7),
            epoch: 0,
        };
        let outcome = merge_connection_status(
            &mut newly_dropped,
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(4),
                epoch: 0,
            },
            ReactivationProvenance::Unwitnessed,
        );
        assert_eq!(
            outcome,
            MergeOutcome::AdoptedDrop,
            "an adopt must signal the caller to re-arm the committing host's witness"
        );
        assert_eq!(
            newly_dropped,
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(4),
                epoch: 0,
            },
            "connected+incoming-disconnected must adopt the new drop"
        );

        // (connected, connected) -> advance via max.
        let mut both_conn = ConnectionStatus {
            disconnected: false,
            last_frame: Frame::new(4),
            epoch: 0,
        };
        let outcome = merge_connection_status(
            &mut both_conn,
            ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(9),
                epoch: 0,
            },
            ReactivationProvenance::Unwitnessed,
        );
        assert_eq!(outcome, MergeOutcome::NoTransition);
        assert_eq!(
            both_conn,
            ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(9),
                epoch: 0,
            },
            "both-connected must advance the last_frame via max"
        );
    }

    #[test]
    fn spectator_same_frame_redundant_host_cannot_refresh_or_disconnect_shared_status() {
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .start_spectator_session_multi(&[test_addr(7310), test_addr(7311)], DummySocket)
            .unwrap();
        let frame = Frame::new(0);
        let fresh_status = vec![
            ConnectionStatus {
                disconnected: false,
                last_frame: frame,
                epoch: 0,
            },
            ConnectionStatus {
                disconnected: false,
                last_frame: frame,
                epoch: 0,
            },
        ];

        queue_host_input(&mut session, 0, frame, [11, 22], fresh_status);
        session.poll_remote_clients();
        assert_eq!(session.last_recv_frame, frame);
        assert_eq!(
            session.host_connect_status,
            vec![
                ConnectionStatus {
                    disconnected: false,
                    last_frame: frame,
                    epoch: 0,
                },
                ConnectionStatus {
                    disconnected: false,
                    last_frame: frame,
                    epoch: 0,
                },
            ]
        );

        let stale_same_frame_status = vec![
            ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(99),
                epoch: 0,
            },
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(3),
                epoch: 0,
            },
        ];
        queue_host_input(&mut session, 1, frame, [11, 22], stale_same_frame_status);
        session.poll_remote_clients();

        assert_eq!(
            session.host_connect_status,
            vec![
                ConnectionStatus {
                    disconnected: false,
                    last_frame: frame,
                    epoch: 0,
                },
                ConnectionStatus {
                    disconnected: false,
                    last_frame: frame,
                    epoch: 0,
                },
            ]
        );
    }

    #[test]
    fn spectator_lower_priority_snapshot_is_provisional_until_primary_arrives() {
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .start_spectator_session_multi(&[test_addr(7320), test_addr(7321)], DummySocket)
            .unwrap();
        let frame = Frame::new(0);
        let lower_status = vec![
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(4),
                epoch: 0,
            },
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(4),
                epoch: 0,
            },
        ];
        queue_host_input(&mut session, 1, frame, [11, 22], lower_status);
        session.poll_remote_clients();
        assert_eq!(session.last_recv_frame, Frame::NULL);

        let primary_status = vec![
            ConnectionStatus {
                disconnected: false,
                last_frame: frame,
                epoch: 0,
            },
            ConnectionStatus {
                disconnected: false,
                last_frame: frame,
                epoch: 0,
            },
        ];
        queue_host_input(&mut session, 0, frame, [11, 22], primary_status);
        session.poll_remote_clients();

        assert_eq!(session.last_recv_frame, frame);
        assert_eq!(
            session.host_connect_status,
            vec![
                ConnectionStatus {
                    disconnected: false,
                    last_frame: frame,
                    epoch: 0,
                },
                ConnectionStatus {
                    disconnected: false,
                    last_frame: frame,
                    epoch: 0,
                },
            ]
        );
        let buffer_index = frame.buffer_index(session.buffer_size).unwrap();
        assert_eq!(session.inputs[buffer_index][0].input, 11_u8);
        assert_eq!(session.inputs[buffer_index][1].input, 22_u8);
    }

    #[test]
    fn spectator_disconnected_primary_promotes_next_host_for_unresolved_frame() {
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .start_spectator_session_multi(&[test_addr(7330), test_addr(7331)], DummySocket)
            .unwrap();
        let frame = Frame::new(0);
        let promoted_status = vec![
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(4),
                epoch: 0,
            },
            ConnectionStatus {
                disconnected: false,
                last_frame: frame,
                epoch: 0,
            },
        ];
        queue_host_input(&mut session, 1, frame, [31, 32], promoted_status);
        session.poll_remote_clients();
        assert_eq!(session.last_recv_frame, Frame::NULL);

        session.remove_disconnected_hosts(vec![0]);
        session.try_commit_ready_frames();

        assert_eq!(session.last_recv_frame, frame);
        let buffer_index = frame.buffer_index(session.buffer_size).unwrap();
        assert_eq!(session.inputs[buffer_index][0].input, 31_u8);
        assert_eq!(session.inputs[buffer_index][1].input, 32_u8);
        assert_eq!(
            session.host_connect_status,
            vec![
                ConnectionStatus {
                    disconnected: true,
                    last_frame: Frame::new(4),
                    epoch: 0,
                },
                ConnectionStatus {
                    disconnected: false,
                    last_frame: frame,
                    epoch: 0,
                },
            ]
        );
    }

    #[test]
    fn multi_host_later_canonical_snapshot_does_not_raise_disconnect_freeze_frame() {
        // Regression for the spectator freeze-frame-RAISE bug (N-PLAYER-DESYNC-AUDIT.md,
        // completeness critic #1): with >=2 redundant hosts under asymmetric loss the
        // per-frame canonical host oscillates. A later canonical host that reports a
        // dropped slot disconnected at a HIGHER freeze last_frame used to RAW-OVERWRITE
        // the spectator's already-frozen label, raising its freeze frame so the public
        // input-status path then returned Confirmed for frames the mesh had already
        // frozen -> silent spectator divergence. The reactivation-safe merge converges
        // a dropped slot's freeze frame DOWN to the global minimum and never raises it.
        //
        // RED check: temporarily restore `*slot = status;` (raw overwrite) in
        // `commit_canonical_snapshot` (and add `#[allow(dead_code)]` to
        // `merge_connection_status`, which `#![deny(warnings)]` would otherwise
        // flag as unused) and this test fails — `host_connect_status[1]` becomes
        // the raised freeze 8 and `inputs_at_frame(6)` reads Confirmed.
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .start_spectator_session_multi(&[test_addr(7340), test_addr(7341)], DummySocket)
            .unwrap();

        // Host 0 (canonical) reports player 1 dropped, frozen LOW at frame 4. Host 1
        // forwards byte-identical inputs (no input divergence — the silent-status
        // case) but reports the same drop frozen HIGHER at frame 8. The mesh-correct
        // freeze frame is the global minimum (4).
        let dropped_low = vec![
            ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(0),
                epoch: 0,
            },
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(4),
                epoch: 0,
            },
        ];
        let dropped_high = vec![
            ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(0),
                epoch: 0,
            },
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(8),
                epoch: 0,
            },
        ];

        // Frame 0 commits from the canonical host 0, establishing the low freeze (4).
        queue_host_input(&mut session, 0, Frame::new(0), [10, 20], dropped_low);
        queue_host_input(&mut session, 1, Frame::new(0), [10, 20], dropped_high);
        session.poll_remote_clients();
        assert_eq!(session.last_recv_frame, Frame::new(0));
        assert_eq!(
            session.host_connect_status,
            vec![
                ConnectionStatus {
                    disconnected: false,
                    last_frame: Frame::new(0),
                    epoch: 0,
                },
                ConnectionStatus {
                    disconnected: true,
                    last_frame: Frame::new(4),
                    epoch: 0,
                },
            ]
        );

        // Stage frames 1..=6 on host 1: player 1 still dropped at the HIGHER freeze
        // (8), player 0 connected and advancing its last_frame with each frame (so
        // the connected `max` branch is exercised end-to-end). Host 0 stays canonical
        // and has no snapshot for these frames, so nothing commits yet.
        for frame in 1..=6_i32 {
            let status = vec![
                ConnectionStatus {
                    disconnected: false,
                    last_frame: Frame::new(frame),
                    epoch: 0,
                },
                ConnectionStatus {
                    disconnected: true,
                    last_frame: Frame::new(8),
                    epoch: 0,
                },
            ];
            queue_host_input(
                &mut session,
                1,
                Frame::new(frame),
                [10 + frame as u8, 20 + frame as u8],
                status,
            );
        }
        session.poll_remote_clients();
        assert_eq!(session.last_recv_frame, Frame::new(0));

        // Host 0 drops; host 1 (re-indexed to 0) becomes canonical and commits the
        // later frames. The shared host_connect_status is preserved across removal.
        session.remove_disconnected_hosts(vec![0]);
        session.try_commit_ready_frames();
        assert_eq!(session.last_recv_frame, Frame::new(6));

        // Observable behavior FIRST (review nit #3): through the public input-status
        // path the dropped slot must read Disconnected for a frame between the two
        // freeze frames (4 < 6 < 8). Under the raw-overwrite bug the freeze was raised
        // to 8, so 8 < 6 is false and this returned Confirmed -> silent divergence.
        let synced = session.inputs_at_frame(Frame::new(6)).unwrap();
        assert_eq!(synced[0].1, InputStatus::Confirmed);
        assert_eq!(synced[1].1, InputStatus::Disconnected);

        // The later canonical host must NOT raise the latched freeze frame: player 1
        // stays disconnected at the converged minimum (4), NOT host 1's higher 8.
        assert_eq!(
            session.host_connect_status,
            vec![
                ConnectionStatus {
                    disconnected: false,
                    last_frame: Frame::new(6),
                    epoch: 0,
                },
                ConnectionStatus {
                    disconnected: true,
                    last_frame: Frame::new(4),
                    epoch: 0,
                },
            ]
        );
    }

    #[test]
    fn spectator_follows_host_reactivation_after_disconnect() {
        // No-regression guard for the reviewer's Major finding: a hot-join host that
        // re-opens a dropped slot sets it back to connected in its local_connect_status
        // (p2p_session.rs reopen) and gossips that disconnected->connected transition
        // to its spectators (send_confirmed_inputs_to_spectators). The spectator MUST
        // follow that reactivation; the old raw overwrite did. A pure monotonic latch
        // would freeze the reactivated slot as Disconnected forever, diverging from the
        // live game even for a single host + hot-join (a supported config).
        //
        // RED check: reintroduce the latch in `merge_connection_status` by making the
        // (true, false) arm `return;` (leave disconnected, ignore the reactivation) —
        // this test fails: player 1 reads Disconnected and host_connect_status[1]
        // stays disconnected.
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .start_spectator_session_multi(&[test_addr(7350)], DummySocket)
            .unwrap();

        // Frames 0..=2: the single host reports player 1 dropped, frozen low at 2.
        let dropped = vec![
            ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(0),
                epoch: 0,
            },
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(2),
                epoch: 0,
            },
        ];
        for frame in 0..=2_i32 {
            queue_host_input(
                &mut session,
                0,
                Frame::new(frame),
                [10 + frame as u8, 20 + frame as u8],
                dropped.clone(),
            );
        }
        session.poll_remote_clients();
        assert_eq!(session.last_recv_frame, Frame::new(2));
        assert_eq!(
            session.host_connect_status,
            vec![
                ConnectionStatus {
                    disconnected: false,
                    last_frame: Frame::new(0),
                    epoch: 0,
                },
                ConnectionStatus {
                    disconnected: true,
                    last_frame: Frame::new(2),
                    epoch: 0,
                },
            ]
        );

        // Frames 3..=6: the SAME host reactivates player 1 (disconnected -> connected),
        // mirroring a hot-join re-open which gossips connected at a HIGH last_frame,
        // and forwards real inputs for the rejoined slot.
        let reactivated = vec![
            ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(10),
                epoch: 0,
            },
            ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(10),
                epoch: 0,
            },
        ];
        for frame in 3..=6_i32 {
            queue_host_input(
                &mut session,
                0,
                Frame::new(frame),
                [10 + frame as u8, 20 + frame as u8],
                reactivated.clone(),
            );
        }
        session.poll_remote_clients();
        assert_eq!(session.last_recv_frame, Frame::new(6));

        // The spectator must have FOLLOWED the reactivation: player 1 is connected
        // again. host_connect_status[1] is connected at the reactivation last_frame.
        assert!(!session.host_connect_status[1].disconnected);
        assert_eq!(
            session.host_connect_status,
            vec![
                ConnectionStatus {
                    disconnected: false,
                    last_frame: Frame::new(10),
                    epoch: 0,
                },
                ConnectionStatus {
                    disconnected: false,
                    last_frame: Frame::new(10),
                    epoch: 0,
                },
            ]
        );

        // Observable: through the public input-status path, a frame past the old
        // freeze frame (5 > 2) now reads Confirmed for player 1 — the spectator
        // tracks the live game. Under a reintroduced latch it would read Disconnected.
        let synced = session.inputs_at_frame(Frame::new(5)).unwrap();
        assert_eq!(synced[0].1, InputStatus::Confirmed);
        assert_eq!(synced[1].1, InputStatus::Confirmed);
    }

    // Critic #1 reactivation residual (N-PLAYER-DESYNC-AUDIT.md, "Additional
    // Problems"): a stale lagging host that becomes canonical for a PERMANENTLY
    // dropped slot must not resurrect the slot's label. Host 0 (canonical)
    // commits the drop of player 1 frozen at F = 2. Host 1 lags: it never
    // reported the drop (under asymmetric loss it received MORE of player 1's
    // pre-drop inputs, so its gossip still shows the slot connected at a HIGHER
    // last_frame) and stages later frames with player-1 values from the region
    // the mesh froze. When host 0 disconnects and host 1 becomes canonical, the
    // unconditional reactivation FOLLOW resurrected the label: the spectator
    // read Confirmed for frames the mesh froze and played host 1's unfrozen
    // values -- a silent spectator desync. With per-host disconnect-witness
    // provenance, host 1 never reported the slot disconnected, so its
    // stale-connected gossip must NOT be followed.
    #[test]
    fn stale_lagging_canonical_host_cannot_resurrect_permanently_dropped_slot() {
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .start_spectator_session_multi(&[test_addr(7501), test_addr(7502)], DummySocket)
            .unwrap();

        // Host 0 (canonical): player 1 dropped, frozen at F = 2. Frames 0..=2
        // still carry player 1's real inputs (its last confirmed input is the
        // freeze frame itself).
        let dropped_at_2 = vec![
            ConnectionStatus::default(),
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(2),
                epoch: 0,
            },
        ];
        for (frame, p1) in [(0_i32, 20_u8), (1, 21), (2, 22)] {
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), 10 + frame as u8),
                PlayerHandle::new(0),
                dropped_at_2.clone(),
                test_addr(7501),
            );
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), p1),
                PlayerHandle::new(1),
                dropped_at_2.clone(),
                test_addr(7501),
            );
        }
        assert_eq!(session.last_recv_frame, Frame::new(2));
        assert_eq!(
            session.host_connect_status[1],
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(2),
                epoch: 0,
            }
        );

        // Host 1 lags: it NEVER reported the drop. Its gossip still shows
        // player 1 connected through its higher pre-drop view (frame 6), and it
        // stages frames 3..=6 with player-1 values from the region the mesh
        // froze. Nothing commits yet: host 0 (canonical) has no snapshots past
        // frame 2.
        for frame in 3..=6_i32 {
            let stale_connected = vec![
                ConnectionStatus {
                    disconnected: false,
                    last_frame: Frame::new(frame),
                    epoch: 0,
                },
                ConnectionStatus {
                    disconnected: false,
                    last_frame: Frame::new(6),
                    epoch: 0,
                },
            ];
            // test:
            session.handle_host_input(
                1,
                PlayerInput::new(Frame::new(frame), 10 + frame as u8),
                PlayerHandle::new(0),
                stale_connected.clone(),
                test_addr(7502),
            );
            // test:
            session.handle_host_input(
                1,
                PlayerInput::new(Frame::new(frame), 90 + frame as u8),
                PlayerHandle::new(1),
                stale_connected,
                test_addr(7502),
            );
        }
        assert_eq!(session.last_recv_frame, Frame::new(2));

        // Host 0 disconnects; host 1 (re-indexed to 0) becomes canonical and
        // commits frames 3..=6 from its stale-connected snapshots.
        session.remove_disconnected_hosts(vec![0]);
        session.try_commit_ready_frames();
        assert_eq!(session.last_recv_frame, Frame::new(6));
        assert!(session.spectator_divergence.is_none());

        // Observable behavior FIRST: through the public input-status path the
        // dropped slot must still read Disconnected past the freeze frame and
        // replay the mesh-agreed frozen value committed at F = 2 (22), not host
        // 1's unfrozen value from the frozen region (94).
        let synced = session.inputs_at_frame(Frame::new(4)).unwrap();
        assert_eq!(synced[0].1, InputStatus::Confirmed);
        assert_eq!(
            synced[1].1,
            InputStatus::Disconnected,
            "stale-connected gossip from a host that never witnessed the drop \
             must not resurrect the dropped slot"
        );
        assert_eq!(
            synced[1].0, 22_u8,
            "dropped slot must keep replaying the frozen value at F = 2"
        );

        // The label itself stays latched at the converged freeze frame.
        assert_eq!(
            session.host_connect_status[1],
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(2),
                epoch: 0,
            }
        );
    }

    // Second drop->rejoin cycle: provenance from an EARLIER drop cycle must not
    // resurrect a LATER drop. Host 1 witnessed cycle-1's drop (frozen at F1 = 1)
    // but lagged through the reactivation and never observed cycle-2's drop
    // (frozen at F2 = 5 > F1: the rejoined slot confirmed real inputs between
    // the cycles). When host 0 dies and stale host 1 becomes canonical, its
    // connected gossip must not resurrect the cycle-2 drop -- its witnessed
    // freeze (1) belongs to the consumed first cycle.
    #[test]
    fn second_drop_after_reactivation_not_resurrected_by_cycle_one_witness() {
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .start_spectator_session_multi(&[test_addr(7503), test_addr(7504)], DummySocket)
            .unwrap();

        // Cycle 1: host 0 (canonical) commits frames 0..=1 with player 1
        // dropped, frozen at F1 = 1.
        let dropped_at_1 = vec![
            ConnectionStatus::default(),
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(1),
                epoch: 0,
            },
        ];
        for (frame, p1) in [(0_i32, 20_u8), (1, 21)] {
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), 10 + frame as u8),
                PlayerHandle::new(0),
                dropped_at_1.clone(),
                test_addr(7503),
            );
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), p1),
                PlayerHandle::new(1),
                dropped_at_1.clone(),
                test_addr(7503),
            );
        }
        assert_eq!(session.last_recv_frame, Frame::new(1));

        // Host 1 ALSO witnessed cycle-1's drop: it delivers the already
        // committed frame 1 with byte-identical inputs and the same disc@1
        // report (this is the only drop host 1 ever observes).
        // test:
        session.handle_host_input(
            1,
            PlayerInput::new(Frame::new(1), 11),
            PlayerHandle::new(0),
            dropped_at_1.clone(),
            test_addr(7504),
        );
        // test:
        session.handle_host_input(
            1,
            PlayerInput::new(Frame::new(1), 21),
            PlayerHandle::new(1),
            dropped_at_1,
            test_addr(7504),
        );
        assert!(session.spectator_divergence.is_none());

        // Reactivation (hot-join re-open): host 0 gossips connected again and
        // forwards real player-1 inputs for frames 2..=3. Host 0 witnessed
        // cycle-1's drop, so the spectator follows the genuine reactivation.
        let reactivated = vec![
            ConnectionStatus::default(),
            ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(10),
                epoch: 0,
            },
        ];
        for (frame, p1) in [(2_i32, 32_u8), (3, 33)] {
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), 10 + frame as u8),
                PlayerHandle::new(0),
                reactivated.clone(),
                test_addr(7503),
            );
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), p1),
                PlayerHandle::new(1),
                reactivated.clone(),
                test_addr(7503),
            );
        }
        assert_eq!(session.last_recv_frame, Frame::new(3));
        assert!(
            !session.host_connect_status[1].disconnected,
            "genuine reactivation from a drop-witness host must be followed"
        );

        // Cycle 2: host 0 commits frames 4..=5 with player 1 dropped AGAIN,
        // frozen at F2 = 5 (its last confirmed cycle-2 input).
        let dropped_at_5 = vec![
            ConnectionStatus::default(),
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(5),
                epoch: 0,
            },
        ];
        for (frame, p1) in [(4_i32, 34_u8), (5, 35)] {
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), 10 + frame as u8),
                PlayerHandle::new(0),
                dropped_at_5.clone(),
                test_addr(7503),
            );
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), p1),
                PlayerHandle::new(1),
                dropped_at_5.clone(),
                test_addr(7503),
            );
        }
        assert_eq!(session.last_recv_frame, Frame::new(5));
        assert_eq!(
            session.host_connect_status[1],
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(5),
                epoch: 0,
            }
        );

        // Host 1 is stale across cycle 2: it saw the reactivation but never the
        // second drop, so its gossip still reports player 1 CONNECTED. It
        // stages frames 6..=9 with player-1 values from the frozen region.
        for frame in 6..=9_i32 {
            let stale_connected = vec![
                ConnectionStatus {
                    disconnected: false,
                    last_frame: Frame::new(frame),
                    epoch: 0,
                },
                ConnectionStatus {
                    disconnected: false,
                    last_frame: Frame::new(12),
                    epoch: 0,
                },
            ];
            // test:
            session.handle_host_input(
                1,
                PlayerInput::new(Frame::new(frame), 10 + frame as u8),
                PlayerHandle::new(0),
                stale_connected.clone(),
                test_addr(7504),
            );
            // test:
            session.handle_host_input(
                1,
                PlayerInput::new(Frame::new(frame), 90 + frame as u8),
                PlayerHandle::new(1),
                stale_connected,
                test_addr(7504),
            );
        }
        assert_eq!(session.last_recv_frame, Frame::new(5));

        // Host 0 dies; stale host 1 becomes canonical and commits 6..=9. Its
        // cycle-1 witness (freeze 1 < latched freeze 5) must not authorize a
        // resurrect of the cycle-2 drop.
        session.remove_disconnected_hosts(vec![0]);
        session.try_commit_ready_frames();
        assert_eq!(session.last_recv_frame, Frame::new(9));
        assert!(session.spectator_divergence.is_none());

        let synced = session.inputs_at_frame(Frame::new(7)).unwrap();
        assert_eq!(synced[0].1, InputStatus::Confirmed);
        assert_eq!(
            synced[1].1,
            InputStatus::Disconnected,
            "a witness of an earlier, consumed drop cycle must not resurrect a \
             later drop"
        );
        assert_eq!(
            synced[1].0, 35_u8,
            "dropped slot must keep replaying the cycle-2 frozen value at F2 = 5"
        );
        assert_eq!(
            session.host_connect_status[1],
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(5),
                epoch: 0,
            }
        );
    }

    // Cross-cycle discrimination via freeze-frame comparison alone is UNSOUND:
    // the witness stores a host's OWN (possibly pre-convergence HIGH) view of
    // an earlier drop, while the latch stores the converged global-min freeze
    // of the later drop, and the two ranges overlap numerically. Host 1
    // witnessed cycle-1's drop at its high view disc@6 (the mesh converged the
    // latch down to F1 = 1); a genuine reactivation is followed; cycle 2 drops
    // again, converging at F2 = 5 <= 6 (realizable: the rejoin sets
    // `last_frame = activation_frame - 1` from the serving host's own saved
    // frame, `p2p_session.rs`, so the second freeze need not exceed an earlier
    // cycle's high view). When host 0 dies and host 1 — stale across cycle 2 —
    // becomes canonical, its retained cycle-1 witness (6 >= 5) must NOT
    // authorize resurrecting the cycle-2 drop: the witness is CONSUMED when the
    // cycle-1 reactivation is followed, so only post-follow drop reports count.
    #[test]
    fn second_drop_after_reactivation_not_resurrected_by_high_view_cycle_one_witness() {
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .start_spectator_session_multi(&[test_addr(7509), test_addr(7510)], DummySocket)
            .unwrap();

        // Cycle 1: host 0 (canonical) commits frames 0..=1 with player 1
        // dropped, frozen at the converged F1 = 1.
        let dropped_at_1 = vec![
            ConnectionStatus::default(),
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(1),
                epoch: 0,
            },
        ];
        for (frame, p1) in [(0_i32, 20_u8), (1, 21)] {
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), 10 + frame as u8),
                PlayerHandle::new(0),
                dropped_at_1.clone(),
                test_addr(7509),
            );
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), p1),
                PlayerHandle::new(1),
                dropped_at_1.clone(),
                test_addr(7509),
            );
        }
        assert_eq!(session.last_recv_frame, Frame::new(1));

        // Host 1 witnessed cycle-1's drop at its own pre-convergence HIGH view
        // (disc@6): under asymmetric loss it received more of player 1's
        // pre-drop inputs than the mesh min. It delivers the already-committed
        // frame 1 with byte-identical inputs; the latched freeze (1) is NOT
        // raised (the merge only converges DOWN).
        let dropped_at_6 = vec![
            ConnectionStatus::default(),
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(6),
                epoch: 0,
            },
        ];
        // test:
        session.handle_host_input(
            1,
            PlayerInput::new(Frame::new(1), 11),
            PlayerHandle::new(0),
            dropped_at_6.clone(),
            test_addr(7510),
        );
        // test:
        session.handle_host_input(
            1,
            PlayerInput::new(Frame::new(1), 21),
            PlayerHandle::new(1),
            dropped_at_6,
            test_addr(7510),
        );
        assert!(session.spectator_divergence.is_none());
        assert_eq!(
            session.host_connect_status[1],
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(1),
                epoch: 0,
            }
        );

        // Genuine reactivation: host 0 gossips connected again and forwards
        // real player-1 inputs for frames 2..=3. Followed (host 0 witnessed
        // the latched drop); following CONSUMES every host's witness for
        // player 1.
        let reactivated = vec![
            ConnectionStatus::default(),
            ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(10),
                epoch: 0,
            },
        ];
        for (frame, p1) in [(2_i32, 32_u8), (3, 33)] {
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), 10 + frame as u8),
                PlayerHandle::new(0),
                reactivated.clone(),
                test_addr(7509),
            );
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), p1),
                PlayerHandle::new(1),
                reactivated.clone(),
                test_addr(7509),
            );
        }
        assert_eq!(session.last_recv_frame, Frame::new(3));
        assert!(
            !session.host_connect_status[1].disconnected,
            "genuine reactivation from a drop-witness host must be followed"
        );

        // Cycle 2: host 0 commits frames 4..=5 with player 1 dropped AGAIN,
        // converged at F2 = 5 — NUMERICALLY BELOW host 1's retained cycle-1
        // high view of 6.
        let dropped_at_5 = vec![
            ConnectionStatus::default(),
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(5),
                epoch: 0,
            },
        ];
        for (frame, p1) in [(4_i32, 34_u8), (5, 35)] {
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), 10 + frame as u8),
                PlayerHandle::new(0),
                dropped_at_5.clone(),
                test_addr(7509),
            );
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), p1),
                PlayerHandle::new(1),
                dropped_at_5.clone(),
                test_addr(7509),
            );
        }
        assert_eq!(session.last_recv_frame, Frame::new(5));
        assert_eq!(
            session.host_connect_status[1],
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(5),
                epoch: 0,
            }
        );

        // Host 1 is stale across cycle 2: it saw the reactivation but never the
        // second drop, so its gossip still reports player 1 CONNECTED. It
        // stages frames 6..=9 with player-1 values from the frozen region.
        for frame in 6..=9_i32 {
            let stale_connected = vec![
                ConnectionStatus {
                    disconnected: false,
                    last_frame: Frame::new(frame),
                    epoch: 0,
                },
                ConnectionStatus {
                    disconnected: false,
                    last_frame: Frame::new(12),
                    epoch: 0,
                },
            ];
            // test:
            session.handle_host_input(
                1,
                PlayerInput::new(Frame::new(frame), 10 + frame as u8),
                PlayerHandle::new(0),
                stale_connected.clone(),
                test_addr(7510),
            );
            // test:
            session.handle_host_input(
                1,
                PlayerInput::new(Frame::new(frame), 90 + frame as u8),
                PlayerHandle::new(1),
                stale_connected,
                test_addr(7510),
            );
        }
        assert_eq!(session.last_recv_frame, Frame::new(5));

        // Host 0 dies; stale host 1 becomes canonical and commits 6..=9. Its
        // cycle-1 high-view witness (6 >= latched 5) was consumed by the
        // followed cycle-1 reactivation, so it must not authorize a resurrect
        // of the cycle-2 drop.
        session.remove_disconnected_hosts(vec![0]);
        session.try_commit_ready_frames();
        assert_eq!(session.last_recv_frame, Frame::new(9));
        assert!(session.spectator_divergence.is_none());

        let synced = session.inputs_at_frame(Frame::new(7)).unwrap();
        assert_eq!(synced[0].1, InputStatus::Confirmed);
        assert_eq!(
            synced[1].1,
            InputStatus::Disconnected,
            "an earlier cycle's pre-convergence-high witness must not resurrect \
             a later drop whose converged freeze it numerically covers"
        );
        assert_eq!(
            synced[1].0, 35_u8,
            "dropped slot must keep replaying the cycle-2 frozen value at F2 = 5"
        );
        assert_eq!(
            session.host_connect_status[1],
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(5),
                epoch: 0,
            }
        );
    }

    // S46 — CROSS-CYCLE fail-open corner, CLOSED by the per-slot connect-status
    // `epoch`. The pre-epoch gate discriminated drop cycles only by consumption:
    // a followed reactivation consumes the witness table, but a host's reordered
    // EARLIER-cycle drop report that ARRIVES after the consume RE-ARMS that
    // host's witness with stale provenance — and a `disc@v` report carried no
    // cycle identity, so the gate could not tell it from a current-cycle drop.
    // If `v` numerically covers the NEXT drop's converged freeze and that host
    // then becomes canonical with a stale-connected view (it forwarded the
    // reactivation but missed the next drop), the slot wrongly RESURRECTS.
    //
    // The epoch closes it: host 1's reordered cycle-1 drop carries `epoch = 1`,
    // strictly below host 1's latest-seen generation `2` (advanced by the
    // reactivation host 1 forwarded), so the witness re-arm is rejected and the
    // resurrect cannot fire. RED on an epoch-blind gate (player 1 → Confirmed);
    // GREEN with the epoch gate (player 1 stays Disconnected at the cycle-2
    // frozen value). Sibling of
    // `second_drop_after_reactivation_not_resurrected_by_high_view_cycle_one_witness`
    // (which is the fail-CLOSED variant: no post-consume re-arm).
    #[test]
    fn cross_cycle_stale_drop_epoch_blocks_resurrect_after_consume() {
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .start_spectator_session_multi(&[test_addr(7515), test_addr(7516)], DummySocket)
            .unwrap();
        let cs = |disconnected: bool, last_frame: Frame, epoch: u16| ConnectionStatus {
            disconnected,
            last_frame,
            epoch,
        };

        // Cycle 1: host 0 (canonical) commits frames 0..=1 with player 1 dropped
        // at the converged F1 = 1 (host 0 drop generation = epoch 1).
        let h0_drop1 = vec![cs(false, Frame::NULL, 0), cs(true, Frame::new(1), 1)];
        for (frame, p1) in [(0_i32, 20_u8), (1, 21)] {
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), 10 + frame as u8),
                PlayerHandle::new(0),
                h0_drop1.clone(),
                test_addr(7515),
            );
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), p1),
                PlayerHandle::new(1),
                h0_drop1.clone(),
                test_addr(7515),
            );
        }
        assert_eq!(session.last_recv_frame, Frame::new(1));

        // Host 1 witnessed cycle-1's drop at its own pre-convergence HIGH view
        // (disc@6, host 1 drop generation = epoch 1); byte-identical at frame 1.
        let h1_drop6 = vec![cs(false, Frame::NULL, 0), cs(true, Frame::new(6), 1)];
        // test:
        session.handle_host_input(
            1,
            PlayerInput::new(Frame::new(1), 11),
            PlayerHandle::new(0),
            h1_drop6.clone(),
            test_addr(7516),
        );
        // test:
        session.handle_host_input(
            1,
            PlayerInput::new(Frame::new(1), 21),
            PlayerHandle::new(1),
            h1_drop6.clone(),
            test_addr(7516),
        );

        // Genuine reactivation: host 0 re-opens player 1 (epoch 2), frames 2..=3.
        // Followed (host 0 witnessed the latch); the follow CONSUMES witnesses.
        // Host 1 ALSO forwards the reactivation (epoch 2), advancing its
        // latest-seen generation for player 1 to 2.
        let react = vec![cs(false, Frame::NULL, 0), cs(false, Frame::new(10), 2)];
        for (frame, p1) in [(2_i32, 32_u8), (3, 33)] {
            for host in [0_usize, 1] {
                let addr = if host == 0 {
                    test_addr(7515)
                } else {
                    test_addr(7516)
                };
                // test:
                session.handle_host_input(
                    host,
                    PlayerInput::new(Frame::new(frame), 10 + frame as u8),
                    PlayerHandle::new(0),
                    react.clone(),
                    addr,
                );
                // test:
                session.handle_host_input(
                    host,
                    PlayerInput::new(Frame::new(frame), p1),
                    PlayerHandle::new(1),
                    react.clone(),
                    addr,
                );
            }
        }
        assert_eq!(session.last_recv_frame, Frame::new(3));
        assert!(
            !session.host_connect_status[1].disconnected,
            "genuine reactivation from a drop-witness host must be followed"
        );

        // Cycle 2: host 0 drops player 1 AGAIN at F2 = 5 (epoch 3) — numerically
        // BELOW host 1's retained cycle-1 high view of 6.
        let h0_drop5 = vec![cs(false, Frame::NULL, 0), cs(true, Frame::new(5), 3)];
        for (frame, p1) in [(4_i32, 34_u8), (5, 35)] {
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), 10 + frame as u8),
                PlayerHandle::new(0),
                h0_drop5.clone(),
                test_addr(7515),
            );
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), p1),
                PlayerHandle::new(1),
                h0_drop5.clone(),
                test_addr(7515),
            );
        }
        assert_eq!(session.last_recv_frame, Frame::new(5));

        // THE FAIL-OPEN TRIGGER: host 1's REORDERED cycle-1 drop report
        // (disc@6, epoch 1) arrives LATE, after the consume — re-delivered on an
        // already-staged old frame so only the witness table is touched. On an
        // epoch-blind gate it re-arms host 1's witness (6 >= latched 5); the
        // epoch gate rejects it (epoch 1 < host 1's latest-seen generation 2).
        // test:
        session.handle_host_input(
            1,
            PlayerInput::new(Frame::new(1), 21),
            PlayerHandle::new(1),
            h1_drop6,
            test_addr(7516),
        );

        // Host 1 is stale across cycle 2: it forwarded the reactivation but never
        // the second drop, so its gossip still reports player 1 CONNECTED
        // (epoch 2). It stages frames 6..=9 with player-1 values from the frozen
        // region.
        for frame in 6..=9_i32 {
            let stale_conn = vec![
                cs(false, Frame::new(frame), 0),
                cs(false, Frame::new(12), 2),
            ];
            // test:
            session.handle_host_input(
                1,
                PlayerInput::new(Frame::new(frame), 10 + frame as u8),
                PlayerHandle::new(0),
                stale_conn.clone(),
                test_addr(7516),
            );
            // test:
            session.handle_host_input(
                1,
                PlayerInput::new(Frame::new(frame), 90 + frame as u8),
                PlayerHandle::new(1),
                stale_conn,
                test_addr(7516),
            );
        }
        assert_eq!(session.last_recv_frame, Frame::new(5));

        // Host 0 dies; stale host 1 becomes canonical and commits 6..=9. Its
        // re-armed cycle-1 witness must NOT authorize a resurrect of cycle 2.
        session.remove_disconnected_hosts(vec![0]);
        session.try_commit_ready_frames();
        assert_eq!(session.last_recv_frame, Frame::new(9));
        assert!(session.spectator_divergence.is_none());

        let synced = session.inputs_at_frame(Frame::new(7)).unwrap();
        assert_eq!(synced[0].1, InputStatus::Confirmed);
        assert_eq!(
            synced[1].1,
            InputStatus::Disconnected,
            "a reordered EARLIER-cycle drop report (epoch 1 < latest-seen 2) must \
             not re-arm provenance and resurrect the cycle-2 drop"
        );
        assert_eq!(
            synced[1].0, 35_u8,
            "dropped slot must keep replaying the cycle-2 frozen value at F2 = 5"
        );
        assert!(session.host_connect_status[1].disconnected);
        assert_eq!(session.host_connect_status[1].last_frame, Frame::new(5));
    }

    // Positive provenance guard: a failover host that DID witness the drop
    // (its own gossip reported the slot disconnected before the canonical host
    // died) must still have its disconnected->connected report followed as a
    // genuine reactivation. Ensures the disconnect-witness gate closes only the
    // stale-resurrect residual and does not regress hot-join spectation across
    // host failover.
    #[test]
    fn witnessed_failover_host_reactivation_is_followed() {
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .start_spectator_session_multi(&[test_addr(7505), test_addr(7506)], DummySocket)
            .unwrap();

        // Host 0 (canonical) commits frames 0..=2 with player 1 dropped,
        // frozen at F = 2.
        let dropped_at_2 = vec![
            ConnectionStatus::default(),
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(2),
                epoch: 0,
            },
        ];
        for (frame, p1) in [(0_i32, 20_u8), (1, 21), (2, 22)] {
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), 10 + frame as u8),
                PlayerHandle::new(0),
                dropped_at_2.clone(),
                test_addr(7505),
            );
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), p1),
                PlayerHandle::new(1),
                dropped_at_2.clone(),
                test_addr(7505),
            );
        }
        assert_eq!(session.last_recv_frame, Frame::new(2));

        // Host 1 ALSO witnessed the drop: it delivers the already-committed
        // frame 2 with byte-identical inputs and the same disc@2 report.
        // test:
        session.handle_host_input(
            1,
            PlayerInput::new(Frame::new(2), 12),
            PlayerHandle::new(0),
            dropped_at_2.clone(),
            test_addr(7506),
        );
        // test:
        session.handle_host_input(
            1,
            PlayerInput::new(Frame::new(2), 22),
            PlayerHandle::new(1),
            dropped_at_2,
            test_addr(7506),
        );
        assert!(session.spectator_divergence.is_none());

        // The mesh reactivates player 1 (hot-join re-open). Host 1 gossips the
        // disconnected->connected transition and forwards real player-1 inputs
        // for frames 3..=6; host 0 dies before committing the reactivation.
        let reactivated = vec![
            ConnectionStatus::default(),
            ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(10),
                epoch: 0,
            },
        ];
        for frame in 3..=6_i32 {
            // test:
            session.handle_host_input(
                1,
                PlayerInput::new(Frame::new(frame), 10 + frame as u8),
                PlayerHandle::new(0),
                reactivated.clone(),
                test_addr(7506),
            );
            // test:
            session.handle_host_input(
                1,
                PlayerInput::new(Frame::new(frame), 50 + frame as u8),
                PlayerHandle::new(1),
                reactivated.clone(),
                test_addr(7506),
            );
        }
        assert_eq!(session.last_recv_frame, Frame::new(2));

        session.remove_disconnected_hosts(vec![0]);
        session.try_commit_ready_frames();
        assert_eq!(session.last_recv_frame, Frame::new(6));

        // The witnessed failover host's reactivation is followed: player 1 is
        // connected again and reads Confirmed with its real forwarded input.
        assert!(
            !session.host_connect_status[1].disconnected,
            "a drop-witness failover host's reactivation must be followed"
        );
        let synced = session.inputs_at_frame(Frame::new(5)).unwrap();
        assert_eq!(synced[0].1, InputStatus::Confirmed);
        assert_eq!(synced[1].1, InputStatus::Confirmed);
        assert_eq!(synced[1].0, 55_u8);
    }

    // NULL-freeze witness semantics: a host can report a drop frozen at
    // `Frame::NULL` (the slot never confirmed any input). Such a witness
    // authorizes re-opening a drop latched at `Frame::NULL` (the gate's
    // `witness >= latch` holds at `NULL >= NULL`) but never a drop latched at
    // a real frame (`NULL < 0 <= latch`).
    #[test]
    fn null_freeze_witness_follows_null_latch_but_not_real_latch() {
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .start_spectator_session_multi(&[test_addr(7511)], DummySocket)
            .unwrap();

        // The host reports player 1 dropped before it ever confirmed an input.
        session.witness_host_status_reports(
            0,
            &[
                ConnectionStatus::default(),
                ConnectionStatus {
                    disconnected: true,
                    last_frame: Frame::NULL,
                    epoch: 0,
                },
            ],
        );
        assert_eq!(
            session.host_drop_witness[0][1],
            Some(DropWitness {
                freeze: Frame::NULL,
                epoch: 0
            })
        );

        // Latch also frozen at NULL: the witness covers it -> follow.
        session.host_connect_status[1] = ConnectionStatus {
            disconnected: true,
            last_frame: Frame::NULL,
            epoch: 0,
        };
        assert_eq!(
            session.reactivation_provenance(0, 1, 0),
            ReactivationProvenance::Witnessed,
            "witness Some(NULL) must cover a latch frozen at NULL"
        );

        // Latch frozen at a real frame: a NULL witness does not cover it ->
        // block.
        session.host_connect_status[1] = ConnectionStatus {
            disconnected: true,
            last_frame: Frame::new(0),
            epoch: 0,
        };
        assert_eq!(
            session.reactivation_provenance(0, 1, 0),
            ReactivationProvenance::Unwitnessed,
            "witness Some(NULL) must not cover a latch frozen at a real frame"
        );
    }

    // S46 — WITHIN-CYCLE fail-open corner, CLOSED by the connect-status `epoch`.
    // A host that genuinely witnessed the current drop holds follow authority for
    // the whole cycle, so on a freeze-only gate its OWN reordered PRE-drop
    // connected snapshot (first-writer staged at a later frame) passed the gate
    // while it was canonical and transiently RESURRECTED the slot. The epoch
    // discriminates: the pre-drop connected report carries the older pre-drop
    // generation (`< witnessed.epoch`, which the drop bumped past), so it is
    // rejected; a genuine reactivation carries `> witnessed.epoch` and is
    // followed. `>=` keeps the legacy uniform-epoch follow intact. Direct
    // provenance probe (the discriminator the corner turns on).
    #[test]
    fn within_cycle_pre_drop_epoch_blocks_follow_post_drop_epoch_follows() {
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .start_spectator_session_multi(&[test_addr(7517)], DummySocket)
            .unwrap();

        // Host 0 witnessed its own drop of player 1 at freeze 2, generation 1
        // (connected at gen 0, then the drop bumped it to gen 1).
        session.witness_host_status_reports(
            0,
            &[
                ConnectionStatus::default(),
                ConnectionStatus {
                    disconnected: true,
                    last_frame: Frame::new(2),
                    epoch: 1,
                },
            ],
        );
        assert_eq!(
            session.host_drop_witness[0][1],
            Some(DropWitness {
                freeze: Frame::new(2),
                epoch: 1
            })
        );
        // Spectator latched the drop at freeze 2.
        session.host_connect_status[1] = ConnectionStatus {
            disconnected: true,
            last_frame: Frame::new(2),
            epoch: 1,
        };

        // A reordered PRE-drop connected snapshot carries generation 0 (< the
        // witnessed drop's generation 1): must NOT be followed (within-cycle).
        assert_eq!(
            session.reactivation_provenance(0, 1, 0),
            ReactivationProvenance::Unwitnessed,
            "a reordered pre-drop connected report (epoch 0 < drop epoch 1) must \
             not resurrect the dropped slot"
        );
        // The `>=` boundary (generation equal to the witnessed drop) follows —
        // this is exactly the legacy uniform-epoch world (all reports gen 0), so
        // there is no regression for peers that never arm the epoch.
        assert_eq!(
            session.reactivation_provenance(0, 1, 1),
            ReactivationProvenance::Witnessed,
            "epoch == witnessed drop generation follows (legacy uniform-epoch parity)"
        );
        // A genuine reactivation carries a strictly newer generation: followed.
        assert_eq!(
            session.reactivation_provenance(0, 1, 2),
            ReactivationProvenance::Witnessed,
            "a genuine reactivation (epoch 2 > drop epoch 1) is followed"
        );
    }

    // Consumption must not cost liveness for later genuine cycles: after a
    // followed reactivation consumes the witness table, a host that witnesses
    // the NEXT drop re-establishes its witness on its next drop-bearing packet
    // (gossip rides every packet), so the next genuine reactivation is
    // followed too. Single host, two full drop -> rejoin cycles.
    #[test]
    fn second_reactivation_followed_after_witness_consumption() {
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .start_spectator_session_multi(&[test_addr(7512)], DummySocket)
            .unwrap();

        // Cycle 1: frames 0..=1 with player 1 dropped, frozen at F1 = 1.
        let dropped_at_1 = vec![
            ConnectionStatus::default(),
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(1),
                epoch: 0,
            },
        ];
        for (frame, p1) in [(0_i32, 20_u8), (1, 21)] {
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), 10 + frame as u8),
                PlayerHandle::new(0),
                dropped_at_1.clone(),
                test_addr(7512),
            );
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), p1),
                PlayerHandle::new(1),
                dropped_at_1.clone(),
                test_addr(7512),
            );
        }
        assert_eq!(
            session.host_drop_witness[0][1],
            Some(DropWitness {
                freeze: Frame::new(1),
                epoch: 0
            })
        );

        // Reactivation #1 (frames 2..=3): followed, and the follow CONSUMES
        // the witness.
        let reactivated_1 = vec![
            ConnectionStatus::default(),
            ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(10),
                epoch: 0,
            },
        ];
        for (frame, p1) in [(2_i32, 32_u8), (3, 33)] {
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), 10 + frame as u8),
                PlayerHandle::new(0),
                reactivated_1.clone(),
                test_addr(7512),
            );
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), p1),
                PlayerHandle::new(1),
                reactivated_1.clone(),
                test_addr(7512),
            );
        }
        assert!(!session.host_connect_status[1].disconnected);
        assert_eq!(
            session.host_drop_witness[0][1], None,
            "a followed reactivation must consume the witness"
        );

        // Cycle 2: frames 4..=5 with player 1 dropped again, frozen at F2 = 5.
        // The drop-bearing gossip re-establishes the witness post-consume.
        let dropped_at_5 = vec![
            ConnectionStatus::default(),
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(5),
                epoch: 0,
            },
        ];
        for (frame, p1) in [(4_i32, 34_u8), (5, 35)] {
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), 10 + frame as u8),
                PlayerHandle::new(0),
                dropped_at_5.clone(),
                test_addr(7512),
            );
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), p1),
                PlayerHandle::new(1),
                dropped_at_5.clone(),
                test_addr(7512),
            );
        }
        assert_eq!(
            session.host_connect_status[1],
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(5),
                epoch: 0,
            }
        );
        assert_eq!(
            session.host_drop_witness[0][1],
            Some(DropWitness {
                freeze: Frame::new(5),
                epoch: 0
            })
        );

        // Reactivation #2 (frames 6..=7): the re-established witness covers
        // the cycle-2 latch (5 >= 5), so the second genuine reactivation is
        // followed too — consumption costs no liveness across cycles.
        let reactivated_2 = vec![
            ConnectionStatus::default(),
            ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(12),
                epoch: 0,
            },
        ];
        for (frame, p1) in [(6_i32, 36_u8), (7, 37)] {
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), 10 + frame as u8),
                PlayerHandle::new(0),
                reactivated_2.clone(),
                test_addr(7512),
            );
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), p1),
                PlayerHandle::new(1),
                reactivated_2.clone(),
                test_addr(7512),
            );
        }
        assert_eq!(session.last_recv_frame, Frame::new(7));
        assert!(
            !session.host_connect_status[1].disconnected,
            "the second genuine reactivation must be followed after the first \
             follow consumed the witness"
        );
        let synced = session.inputs_at_frame(Frame::new(7)).unwrap();
        assert_eq!(synced[1].1, InputStatus::Confirmed);
        assert_eq!(synced[1].0, 37_u8);
    }

    // Late-arrival guard: the late-arrival convergence path
    // (`converge_latched_drop_status`) must not be a resurrect vector either. A
    // never-witness host delivering a stale-connected snapshot for an
    // already-committed frame leaves the latched drop untouched (the path only
    // LOWERS an already-disconnected slot's freeze frame).
    #[test]
    fn late_arriving_connected_snapshot_does_not_resurrect_dropped_slot() {
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .start_spectator_session_multi(&[test_addr(7507), test_addr(7508)], DummySocket)
            .unwrap();

        // Host 0 (canonical) commits frames 0..=2 with player 1 dropped,
        // frozen at F = 2.
        let dropped_at_2 = vec![
            ConnectionStatus::default(),
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(2),
                epoch: 0,
            },
        ];
        for (frame, p1) in [(0_i32, 20_u8), (1, 21), (2, 22)] {
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), 10 + frame as u8),
                PlayerHandle::new(0),
                dropped_at_2.clone(),
                test_addr(7507),
            );
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), p1),
                PlayerHandle::new(1),
                dropped_at_2.clone(),
                test_addr(7507),
            );
        }
        assert_eq!(session.last_recv_frame, Frame::new(2));

        // Host 1 (lagging, never reported the drop) late-delivers the already
        // committed frame 1 with byte-identical inputs but a stale-connected
        // view of player 1.
        let stale_connected = vec![
            ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(1),
                epoch: 0,
            },
            ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(6),
                epoch: 0,
            },
        ];
        // test:
        session.handle_host_input(
            1,
            PlayerInput::new(Frame::new(1), 11),
            PlayerHandle::new(0),
            stale_connected.clone(),
            test_addr(7508),
        );
        // test:
        session.handle_host_input(
            1,
            PlayerInput::new(Frame::new(1), 21),
            PlayerHandle::new(1),
            stale_connected,
            test_addr(7508),
        );

        assert!(session.spectator_divergence.is_none());
        assert_eq!(
            session.host_connect_status[1],
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(2),
                epoch: 0,
            },
            "a late-arriving stale-connected snapshot must not resurrect the latched drop"
        );
    }

    // Adopt-arm x consume interaction: a stale STAGED drop snapshot that
    // commits AFTER a followed reactivation consumed the witness table
    // re-freezes the just-reactivated slot via the unconditional
    // `(connected, disconnected)` ADOPT arm — a spectator-authored freeze
    // fabricated post-follow from a stale staged snapshot. The committing
    // host's own witness was wiped by the consume (all its drop-era arrivals
    // predate the follow), so without the commit-time re-arm
    // (`witness_adopted_drop`) NOTHING could ever re-open the slot: the host's
    // later genuine connected gossip would be `Unwitnessed` forever, a
    // permanent freeze the pre-provenance code recovered from. The spectator
    // must instead RECOVER once the host catches up and gossips connected.
    //
    // Timeline: host 0 latches the drop (disc@1) and host 1's witness is
    // armed; host 1, lagging on rearm processing, forwards frames 3..=4 early
    // still carrying its stale disc@1 view (staged only — nothing commits);
    // host 0's frame 2 (genuine rearm) commits the follow, consuming every
    // witness; host 0 dies; host 1 becomes canonical and its stale staged
    // frames commit, re-freezing the slot at disc@1; host 1 then catches up
    // and gossips connected with real player-1 inputs.
    #[test]
    fn stale_staged_adopt_after_witness_consumption_recovers_when_host_catches_up() {
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .start_spectator_session_multi(&[test_addr(7513), test_addr(7514)], DummySocket)
            .unwrap();

        // Host 0 (canonical) commits frames 0..=1 with player 1 dropped,
        // frozen at F = 1.
        let dropped_at_1 = vec![
            ConnectionStatus::default(),
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(1),
                epoch: 0,
            },
        ];
        for (frame, p1) in [(0_i32, 20_u8), (1, 21)] {
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), 10 + frame as u8),
                PlayerHandle::new(0),
                dropped_at_1.clone(),
                test_addr(7513),
            );
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), p1),
                PlayerHandle::new(1),
                dropped_at_1.clone(),
                test_addr(7513),
            );
        }
        assert_eq!(session.last_recv_frame, Frame::new(1));
        assert_eq!(
            session.host_connect_status[1],
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(1),
                epoch: 0,
            }
        );

        // Host 1 lags on rearm processing: it forwards frames 3..=4 EARLY,
        // still carrying its stale disc@1 view (for the frozen slot it
        // forwards the frozen value, 21). These STAGE only — host 0 (the
        // canonical source) has no frame 2 yet, so nothing commits. The
        // arrivals arm host 1's witness at the stale freeze.
        for frame in 3..=4_i32 {
            // test:
            session.handle_host_input(
                1,
                PlayerInput::new(Frame::new(frame), 10 + frame as u8),
                PlayerHandle::new(0),
                dropped_at_1.clone(),
                test_addr(7514),
            );
            // test:
            session.handle_host_input(
                1,
                PlayerInput::new(Frame::new(frame), 21),
                PlayerHandle::new(1),
                dropped_at_1.clone(),
                test_addr(7514),
            );
        }
        assert_eq!(session.last_recv_frame, Frame::new(1));
        assert_eq!(
            session.host_drop_witness[1][1],
            Some(DropWitness {
                freeze: Frame::new(1),
                epoch: 0
            })
        );

        // Host 0's frame 2 (genuine rearm: player 1 connected again with a
        // real input) arrives and COMMITS: the reactivation is followed and
        // the follow consumes EVERY host's witness for player 1 — including
        // host 1's, whose drop-era arrivals all predate the consume.
        let reactivated = vec![
            ConnectionStatus::default(),
            ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(10),
                epoch: 0,
            },
        ];
        // test:
        session.handle_host_input(
            0,
            PlayerInput::new(Frame::new(2), 12),
            PlayerHandle::new(0),
            reactivated.clone(),
            test_addr(7513),
        );
        // test:
        session.handle_host_input(
            0,
            PlayerInput::new(Frame::new(2), 32),
            PlayerHandle::new(1),
            reactivated,
            test_addr(7513),
        );
        assert_eq!(session.last_recv_frame, Frame::new(2));
        assert!(
            !session.host_connect_status[1].disconnected,
            "the genuine rearm from the drop-witness host must be followed"
        );
        assert_eq!(
            session.host_drop_witness[0][1], None,
            "the follow must consume host 0's witness"
        );
        assert_eq!(
            session.host_drop_witness[1][1], None,
            "the follow must consume host 1's witness too"
        );

        // Host 0 dies; host 1 (re-indexed to 0) becomes canonical and its
        // STALE staged frames 3..=4 commit: the unconditional adopt arm
        // re-freezes the just-reactivated slot at disc@1.
        session.remove_disconnected_hosts(vec![0]);
        session.try_commit_ready_frames();
        assert_eq!(session.last_recv_frame, Frame::new(4));
        assert!(session.spectator_divergence.is_none());
        assert_eq!(
            session.host_connect_status[1],
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(1),
                epoch: 0,
            },
            "the stale staged snapshots re-freeze the slot via the adopt arm"
        );
        let refrozen = session.inputs_at_frame(Frame::new(4)).unwrap();
        assert_eq!(
            refrozen[1].1,
            InputStatus::Disconnected,
            "the adopt re-freeze is visible at the frames the stale snapshots cover"
        );
        assert_eq!(
            refrozen[1].0, 21_u8,
            "the re-frozen slot replays the value committed at the adopted freeze"
        );
        assert_eq!(
            session.host_drop_witness[0][1],
            Some(DropWitness {
                freeze: Frame::new(1),
                epoch: 0
            }),
            "the commit-time adopt must re-arm the COMMITTING host's witness at \
             the adopted freeze (the consume wiped its arrival-time witness)"
        );

        // Host 1 catches up: it processes the rearm and gossips player 1
        // connected with real inputs for frames 5..=6. The spectator just made
        // host 1's own report the latch, so host 1's later connected report —
        // which postdates the drop in its own in-order stream — must be
        // followed: anything else freezes the slot PERMANENTLY on a label the
        // spectator itself fabricated from a stale staged snapshot.
        let caught_up = vec![
            ConnectionStatus::default(),
            ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(12),
                epoch: 0,
            },
        ];
        for (frame, p1) in [(5_i32, 55_u8), (6, 56)] {
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), 10 + frame as u8),
                PlayerHandle::new(0),
                caught_up.clone(),
                test_addr(7514),
            );
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), p1),
                PlayerHandle::new(1),
                caught_up.clone(),
                test_addr(7514),
            );
        }
        assert_eq!(session.last_recv_frame, Frame::new(6));
        assert!(
            !session.host_connect_status[1].disconnected,
            "the spectator must RECOVER from its own stale-staged adopt re-freeze \
             once the committing host gossips connected, not freeze forever"
        );
        let recovered = session.inputs_at_frame(Frame::new(6)).unwrap();
        assert_eq!(recovered[0].1, InputStatus::Confirmed);
        assert_eq!(recovered[1].1, InputStatus::Confirmed);
        assert_eq!(recovered[1].0, 56_u8);
        // The recovery follow consumes the witnesses again, as every follow
        // does.
        assert_eq!(session.host_drop_witness[0][1], None);
    }

    // PINS A DOCUMENTED RESIDUAL, NOT DESIRED BEHAVIOR: the fail-closed window
    // of arrival-time witness provenance (see the residual notes on
    // `merge_connection_status`). A GENUINE reactivation gossiped only by a
    // host whose drop-era reports for the CURRENT drop never reached this
    // spectator — its only witness arrivals predate the previous follow's
    // consume, with no drop-bearing packet after it — is not followed: the
    // spectator keeps the frozen label rather than risking a resurrect it
    // cannot distinguish from stale-connected gossip. If a future change adds
    // a dedicated host -> spectator reactivation signal, this test SHOULD
    // start failing and be replaced by a recovery assertion.
    #[test]
    fn genuine_reactivation_with_only_preconsume_witnesses_fails_closed_at_frozen_label() {
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .start_spectator_session_multi(&[test_addr(7515), test_addr(7516)], DummySocket)
            .unwrap();

        // Cycle 1: host 0 (canonical) commits frames 0..=1 with player 1
        // dropped, frozen at F1 = 1; host 1 also witnessed it (delivers the
        // already-committed frame 1 with byte-identical inputs).
        let dropped_at_1 = vec![
            ConnectionStatus::default(),
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(1),
                epoch: 0,
            },
        ];
        for (frame, p1) in [(0_i32, 20_u8), (1, 21)] {
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), 10 + frame as u8),
                PlayerHandle::new(0),
                dropped_at_1.clone(),
                test_addr(7515),
            );
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), p1),
                PlayerHandle::new(1),
                dropped_at_1.clone(),
                test_addr(7515),
            );
        }
        // test:
        session.handle_host_input(
            1,
            PlayerInput::new(Frame::new(1), 11),
            PlayerHandle::new(0),
            dropped_at_1.clone(),
            test_addr(7516),
        );
        // test:
        session.handle_host_input(
            1,
            PlayerInput::new(Frame::new(1), 21),
            PlayerHandle::new(1),
            dropped_at_1,
            test_addr(7516),
        );
        assert_eq!(
            session.host_drop_witness[1][1],
            Some(DropWitness {
                freeze: Frame::new(1),
                epoch: 0
            })
        );

        // Rearm #1: host 0 forwards frames 2..=3 with player 1 connected and
        // real inputs; the follow CONSUMES every witness for player 1. Host
        // 1's only witness arrivals are now strictly pre-consume.
        let reactivated = vec![
            ConnectionStatus::default(),
            ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(10),
                epoch: 0,
            },
        ];
        for (frame, p1) in [(2_i32, 32_u8), (3, 33)] {
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), 10 + frame as u8),
                PlayerHandle::new(0),
                reactivated.clone(),
                test_addr(7515),
            );
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), p1),
                PlayerHandle::new(1),
                reactivated.clone(),
                test_addr(7515),
            );
        }
        assert!(!session.host_connect_status[1].disconnected);
        assert_eq!(session.host_drop_witness[1][1], None);

        // Cycle 2: host 0 commits frames 4..=5 with player 1 dropped AGAIN,
        // frozen at F2 = 5. Host 1's drop-2-era packets are ALL lost: no
        // drop-bearing packet from host 1 arrives after the consume, so its
        // witness stays None.
        let dropped_at_5 = vec![
            ConnectionStatus::default(),
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(5),
                epoch: 0,
            },
        ];
        for (frame, p1) in [(4_i32, 34_u8), (5, 35)] {
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), 10 + frame as u8),
                PlayerHandle::new(0),
                dropped_at_5.clone(),
                test_addr(7515),
            );
            // test:
            session.handle_host_input(
                0,
                PlayerInput::new(Frame::new(frame), p1),
                PlayerHandle::new(1),
                dropped_at_5.clone(),
                test_addr(7515),
            );
        }
        assert_eq!(session.last_recv_frame, Frame::new(5));
        assert_eq!(session.host_drop_witness[1][1], None);

        // The mesh GENUINELY rearms player 1, but host 0 dies before
        // forwarding the rearm. Host 1 — whose drop-2-era packets never
        // arrived — forwards frames 6..=7 with player 1 connected and real
        // inputs. This is a genuine reactivation the spectator cannot
        // distinguish from stale-connected gossip.
        let rearmed = vec![
            ConnectionStatus::default(),
            ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(12),
                epoch: 0,
            },
        ];
        for (frame, p1) in [(6_i32, 66_u8), (7, 67)] {
            // test:
            session.handle_host_input(
                1,
                PlayerInput::new(Frame::new(frame), 10 + frame as u8),
                PlayerHandle::new(0),
                rearmed.clone(),
                test_addr(7516),
            );
            // test:
            session.handle_host_input(
                1,
                PlayerInput::new(Frame::new(frame), p1),
                PlayerHandle::new(1),
                rearmed.clone(),
                test_addr(7516),
            );
        }
        session.remove_disconnected_hosts(vec![0]);
        session.try_commit_ready_frames();
        assert_eq!(session.last_recv_frame, Frame::new(7));
        assert!(session.spectator_divergence.is_none());

        // Documented fail-closed degradation: the genuine reactivation is NOT
        // followed (host 1 has no post-consume witness), so the frozen label
        // persists and the slot keeps replaying the cycle-2 frozen value.
        assert_eq!(
            session.host_connect_status[1],
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(5),
                epoch: 0,
            },
            "a witness-less genuine reactivation fails closed at the frozen \
             label (documented residual; safe direction)"
        );
        let synced = session.inputs_at_frame(Frame::new(7)).unwrap();
        assert_eq!(synced[0].1, InputStatus::Confirmed);
        assert_eq!(synced[1].1, InputStatus::Disconnected);
        assert_eq!(
            synced[1].0, 35_u8,
            "the frozen slot keeps replaying the cycle-2 freeze value"
        );
    }

    #[test]
    fn spectator_all_hosts_dropped_drains_buffered_frames_then_prediction_threshold() {
        let mut session = create_test_spectator_session().unwrap();
        session.state = SessionState::Running;
        session.hosts.clear();
        session.current_frame = Frame::NULL;
        session.last_recv_frame = Frame::new(1);

        for frame in [Frame::new(0), Frame::new(1)] {
            let buffer_index = frame.buffer_index(session.buffer_size).unwrap();
            session.inputs[buffer_index][0] = PlayerInput::new(frame, frame.as_i32() as u8);
            session.inputs[buffer_index][1] =
                PlayerInput::new(frame, frame.as_i32().saturating_add(10) as u8);
        }

        let first = session.advance_frame().unwrap();
        assert_eq!(session.current_frame(), Frame::new(0));
        assert_eq!(
            first
                .iter()
                .filter(|request| matches!(request, FortressRequest::AdvanceFrame { .. }))
                .count(),
            1
        );

        let second = session.advance_frame().unwrap();
        assert_eq!(session.current_frame(), Frame::new(1));
        assert_eq!(
            second
                .iter()
                .filter(|request| matches!(request, FortressRequest::AdvanceFrame { .. }))
                .count(),
            1
        );

        assert!(matches!(
            session.advance_frame(),
            Err(FortressError::PredictionThreshold)
        ));
    }

    #[test]
    fn spectator_stream_delay_releases_after_clean_all_hosts_disconnect() {
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .with_spectator_config(crate::SpectatorConfig {
                stream_delay: 2,
                ..crate::SpectatorConfig::default()
            })
            .start_spectator_session(test_addr(7061), DummySocket)
            .unwrap();
        session.state = SessionState::Running;
        session.hosts.clear();
        session.current_frame = Frame::NULL;
        session.last_recv_frame = Frame::new(3);

        for frame in [Frame::new(0), Frame::new(1), Frame::new(2), Frame::new(3)] {
            let buffer_index = frame.buffer_index(session.buffer_size).unwrap();
            session.inputs[buffer_index][0] = PlayerInput::new(frame, frame.as_i32() as u8);
            session.inputs[buffer_index][1] =
                PlayerInput::new(frame, frame.as_i32().saturating_add(10) as u8);
        }

        for expected in 0..=3 {
            let requests = session.advance_frame().unwrap();
            assert_eq!(session.current_frame(), Frame::new(expected));
            assert_eq!(
                requests
                    .iter()
                    .filter(|request| matches!(request, FortressRequest::AdvanceFrame { .. }))
                    .count(),
                1
            );
        }
        assert!(matches!(
            session.advance_frame(),
            Err(FortressError::PredictionThreshold)
        ));
    }

    #[test]
    fn spectator_stream_delay_boundary_returns_prediction_threshold() {
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .with_spectator_config(crate::SpectatorConfig {
                stream_delay: 2,
                ..crate::SpectatorConfig::default()
            })
            .start_spectator_session(test_addr(7060), DummySocket)
            .unwrap();
        session.state = SessionState::Running;
        session.current_frame = Frame::new(3);
        session.last_recv_frame = Frame::new(5);
        let blocked_frame = Frame::new(4);
        let buffer_index = blocked_frame.buffer_index(session.buffer_size).unwrap();
        session.inputs[buffer_index][0] = PlayerInput::new(blocked_frame, 40);
        session.inputs[buffer_index][1] = PlayerInput::new(blocked_frame, 41);

        let result = session.advance_frame();

        assert!(matches!(result, Err(FortressError::PredictionThreshold)));
        assert_eq!(session.current_frame(), Frame::new(3));
    }

    // ==========================================
    // Failover / Multi-host Tests
    // ==========================================

    #[test]
    fn spectator_session_single_host_reports_one_host() {
        let session = create_test_spectator_session().unwrap();
        assert_eq!(session.num_hosts(), 1);
    }

    #[test]
    fn spectator_session_multi_host_reports_host_count() {
        let session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .start_spectator_session_multi(
                &[test_addr(7100), test_addr(7101), test_addr(7102)],
                DummySocket,
            )
            .unwrap();
        assert_eq!(session.num_hosts(), 3);
    }

    #[test]
    fn spectator_remove_disconnected_hosts_uses_original_indices() {
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .start_spectator_session_multi(
                &[
                    test_addr(7110),
                    test_addr(7111),
                    test_addr(7112),
                    test_addr(7113),
                ],
                DummySocket,
            )
            .unwrap();

        session.remove_disconnected_hosts(vec![2, 0, 2]);

        let remaining_ports: Vec<_> = session
            .hosts
            .iter()
            .map(|host| host.peer_addr().port())
            .collect();
        assert_eq!(remaining_ports, vec![7111, 7113]);
    }

    #[test]
    fn spectator_remove_disconnected_hosts_ignores_invalid_indices() {
        use crate::telemetry::CollectingObserver;

        let observer = Arc::new(CollectingObserver::new());
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .with_violation_observer(observer.clone())
            .start_spectator_session_multi(
                &[test_addr(7120), test_addr(7121), test_addr(7122)],
                DummySocket,
            )
            .unwrap();

        session.remove_disconnected_hosts(vec![usize::MAX, 1]);

        let remaining_ports: Vec<_> = session
            .hosts
            .iter()
            .map(|host| host.peer_addr().port())
            .collect();
        assert_eq!(remaining_ports, vec![7120, 7122]);
        assert!(observer
            .violations()
            .iter()
            .any(|violation| violation.kind == ViolationKind::InternalError));
    }

    #[test]
    fn spectator_session_multi_host_empty_returns_none() {
        let session: Option<SpectatorSession<TestConfig>> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .start_spectator_session_multi(&[], DummySocket);
        assert!(session.is_none());
    }

    // ==========================================
    // Rewind / Stream-delay Accessor Tests
    // ==========================================

    fn create_spectator_with_stream_and_rewind(
        stream_delay: usize,
        enable_rewind: bool,
    ) -> SpectatorSession<TestConfig> {
        use crate::SpectatorConfig;
        SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .with_spectator_config(SpectatorConfig {
                stream_delay,
                enable_rewind,
                ..SpectatorConfig::default()
            })
            .start_spectator_session(test_addr(7200), DummySocket)
            .unwrap()
    }

    #[test]
    fn spectator_session_rewind_disabled_by_default() {
        let session = create_test_spectator_session().unwrap();
        assert!(!session.is_rewind_enabled());
        assert_eq!(session.stream_delay(), 0);
    }

    #[test]
    fn spectator_session_rewind_and_stream_delay_accessors() {
        let session = create_spectator_with_stream_and_rewind(5, true);
        assert!(session.is_rewind_enabled());
        assert_eq!(session.stream_delay(), 5);
    }

    #[test]
    fn spectator_session_seek_when_rewind_disabled_returns_not_supported() {
        let mut session = create_spectator_with_stream_and_rewind(0, false);
        let result = session.seek_to_frame(Frame::new(0));
        assert!(matches!(
            result,
            Err(FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::NotSupported {
                    operation: "seek_to_frame"
                }
            })
        ));
    }

    #[test]
    fn spectator_session_seek_negative_frame_returns_must_be_non_negative() {
        let mut session = create_spectator_with_stream_and_rewind(0, true);
        let result = session.seek_to_frame(Frame::new(-3));
        assert!(matches!(
            result,
            Err(FortressError::InvalidFrameStructured {
                reason: InvalidFrameReason::MustBeNonNegative,
                ..
            })
        ));
    }

    #[test]
    fn spectator_session_seek_unsaved_frame_returns_missing_state() {
        let mut session = create_spectator_with_stream_and_rewind(0, true);
        // No frames have been saved yet, so seeking to any frame is MissingState.
        let result = session.seek_to_frame(Frame::new(0));
        assert!(matches!(
            result,
            Err(FortressError::InvalidFrameStructured {
                reason: InvalidFrameReason::MissingState,
                ..
            })
        ));
    }

    #[test]
    fn spectator_session_seek_max_frame_returns_arithmetic_overflow() {
        let mut session = create_spectator_with_stream_and_rewind(0, true);
        let result = session.seek_to_frame(Frame::new(i32::MAX));
        assert!(matches!(
            result,
            Err(FortressError::FrameArithmeticOverflow {
                frame,
                operand: 1,
                operation: "add"
            }) if frame == Frame::new(i32::MAX)
        ));
    }
}
