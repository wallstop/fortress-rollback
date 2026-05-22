use std::collections::VecDeque;
use std::fmt;
use std::sync::Arc;

use crate::{
    frame_info::PlayerInput,
    network::{
        messages::ConnectionStatus,
        protocol::{Event, UdpProtocol},
    },
    report_violation, report_violation_to, safe_frame_add,
    sessions::session_trait::Session,
    telemetry::{ViolationKind, ViolationObserver, ViolationSeverity},
    Config, EventDrain, FortressError, FortressEvent, FortressRequest, FortressResult, Frame,
    GameStateCell, InputStatus, InputVec, InternalErrorKind, InvalidFrameReason,
    InvalidRequestKind, NetworkStats, NonBlockingSocket, PlayerHandle, RequestVec, SessionState,
};

/// The number of frames the spectator advances in a single step during normal operation.
///
/// When not catching up to the host, spectators advance one frame at a time to maintain
/// smooth playback. During catchup mode (when far behind), `catchup_speed` is used instead.
const NORMAL_SPEED: usize = 1;

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
    /// Whichever host delivers a frame first fills the buffer. A host that
    /// disconnects is removed; spectation continues while at least one host
    /// remains. See [`SpectatorSession::num_hosts`].
    hosts: Vec<UdpProtocol<T>>,
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
}

impl<T: Config> SpectatorSession<T> {
    /// Creates a new [`SpectatorSession`] for a spectator.
    /// The session will receive inputs from all players from the given host(s) directly.
    /// The session will use the provided socket.
    ///
    /// `hosts` may contain more than one endpoint for failover: confirmed inputs
    /// are accepted from whichever host delivers them first, and the session keeps
    /// advancing while at least one host remains connected.
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
    ) -> Self {
        // host connection status
        let host_connect_status = vec![ConnectionStatus::default(); num_players];

        // Use at least 1 for buffer size to prevent panics
        let actual_buffer_size = buffer_size.max(1);

        // When rewind is enabled, allocate one game-state cell per ring slot.
        let state_buffer = if enable_rewind {
            (0..actual_buffer_size)
                .map(|_| GameStateCell::default())
                .collect()
        } else {
            Vec::new()
        };

        Self {
            state: SessionState::Synchronizing,
            num_players,
            buffer_size: actual_buffer_size,
            inputs: vec![
                vec![PlayerInput::blank_input(Frame::NULL); num_players];
                actual_buffer_size
            ],
            host_connect_status,
            socket,
            hosts,
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
        }
    }

    /// Returns the number of hosts currently feeding this spectator.
    ///
    /// For a single-host spectator this is always `1`. For a failover spectator
    /// created via [`SessionBuilder::start_spectator_session_multi`], this starts
    /// at the number of supplied addresses and drops by one each time a host
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
    /// [`Self::stream_delay`] frames. It stays clamped to [`Frame::NULL`] (or
    /// below the first received frame) when no inputs have been received yet, so
    /// callers never try to grab a negative frame.
    fn viewable_frame(&self) -> Frame {
        let delay = i32::try_from(self.stream_delay).unwrap_or(i32::MAX);
        self.last_recv_frame.saturating_sub(delay)
    }

    /// Returns the current [`SessionState`] of a session.
    #[must_use]
    pub fn current_state(&self) -> SessionState {
        self.state
    }

    /// Returns the number of frames behind the host
    #[must_use]
    pub fn frames_behind_host(&self) -> usize {
        let diff = self.last_recv_frame - self.current_frame;
        // Gracefully handle the case where current_frame somehow exceeds last_recv_frame.
        // This shouldn't happen in normal operation, but we report it and return 0 rather than panic.
        if diff < 0 {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::FrameSync,
                "frames_behind_host: current_frame {} exceeds last_recv_frame {} - returning 0",
                self.current_frame,
                self.last_recv_frame
            );
            return 0;
        }
        diff as usize
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

    /// Returns all events that happened since last queried for events. If the number of stored events exceeds `MAX_EVENT_QUEUE_SIZE`, the oldest events will be discarded.
    #[must_use = "events should be handled to react to session state changes"]
    pub fn events(&mut self) -> EventDrain<'_, T> {
        EventDrain::from_drain(self.event_queue.drain(..))
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
        // receive info from host, trigger events and send messages
        self.poll_remote_clients();

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
        let effective_behind = if viewable > self.current_frame {
            // The `viewable > current_frame` guard guarantees the `Frame - Frame`
            // difference is strictly positive, so the `as usize` cast is sound and
            // never wraps (mirrors the non-negative reasoning in `frames_behind_host`).
            (viewable - self.current_frame) as usize
        } else {
            0
        };

        let frames_to_advance = if effective_behind > self.max_frames_behind {
            self.catchup_speed
        } else {
            NORMAL_SPEED
        };

        // Pre-allocate for the expected number of frames to advance.
        // In normal operation this is 1 (fits inline), in catchup mode it's catchup_speed
        // which may exceed the inline capacity of 4, so we keep with_capacity here.
        // With rewind enabled each advanced frame also emits a SaveGameState, so the
        // batch can hold up to twice as many requests.
        let capacity = if self.enable_rewind {
            frames_to_advance.saturating_mul(2)
        } else {
            frames_to_advance
        };
        let mut requests = RequestVec::<T>::with_capacity(capacity);

        for _ in 0..frames_to_advance {
            // get inputs for the next frame
            let frame_to_grab = safe_frame_add!(
                self.current_frame,
                1,
                "SpectatorSession::advance_frames next"
            );

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

        // Run each host's poll and collect events tagged with the originating host
        // index. We gather everything first to avoid a borrow conflict between the
        // mutable host iteration and the event-handling that also mutates `self`.
        // (`self.hosts` and `self.host_connect_status` are disjoint fields, so the
        // mutable-host + immutable-connect_status borrow below is allowed.)
        let mut events = Vec::new();
        for (host_index, host) in self.hosts.iter_mut().enumerate() {
            let addr = host.peer_addr();
            for event in host.poll(&self.host_connect_status) {
                events.push((host_index, event, addr.clone()));
            }
        }

        // handle all events locally, recording which hosts disconnected this poll
        let mut disconnected_hosts = Vec::new();
        for (host_index, event, addr) in std::mem::take(&mut events) {
            if let Some(disconnected_index) = self.handle_event(host_index, event, addr) {
                disconnected_hosts.push(disconnected_index);
            }
        }

        // Remove any hosts that disconnected during this poll. host_index is only
        // used during event handling above (before removal), so removing entries now
        // is safe. The shared `host_connect_status` is not per-host, so removal does
        // not disturb it.
        self.remove_disconnected_hosts(disconnected_hosts);

        // send out all pending UDP messages
        for host in &mut self.hosts {
            host.send_all_messages(&mut self.socket);
        }
    }

    fn remove_disconnected_hosts(&mut self, disconnected_hosts: Vec<usize>) {
        if disconnected_hosts.is_empty() {
            return;
        }

        let mut remove_host = vec![false; self.hosts.len()];
        for host_index in disconnected_hosts {
            if let Some(should_remove) = remove_host.get_mut(host_index) {
                *should_remove = true;
            } else {
                report_violation_to!(
                    &self.violation_observer,
                    ViolationSeverity::Error,
                    ViolationKind::InternalError,
                    "spectator: disconnected host index {} out of bounds (hosts.len()={})",
                    host_index,
                    self.hosts.len()
                );
            }
        }

        self.hosts = std::mem::take(&mut self.hosts)
            .into_iter()
            .zip(remove_host)
            .filter_map(
                |(host, should_remove)| {
                    if should_remove {
                        None
                    } else {
                        Some(host)
                    }
                },
            )
            .collect();
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
                        (player_input.input, InputStatus::Disconnected)
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
            Event::Input { input, player } => {
                // Validate frame before using as index - negative frames would wrap around
                if input.frame.is_null() || input.frame.as_i32() < 0 {
                    report_violation!(
                        ViolationSeverity::Warning,
                        ViolationKind::FrameSync,
                        "Received input with invalid frame {:?} for player {} - ignoring",
                        input.frame,
                        player
                    );
                    return None;
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
                    return None;
                }

                // Save the input. With redundant hosts, the first report for a
                // given player/frame wins: identical duplicates are ignored, while
                // divergent duplicates are reported and ignored. Newer frames may
                // replace older contents in the same ring slot.
                let frame_index = input.frame.as_i32() as usize % self.buffer_size;
                if let Some(frame_inputs) = self.inputs.get_mut(frame_index) {
                    if let Some(player_input) = frame_inputs.get_mut(player.as_usize()) {
                        if input.frame > player_input.frame {
                            *player_input = input;
                        } else if input.frame == player_input.frame
                            && !player_input.equal(&input, true)
                        {
                            report_violation_to!(
                                &self.violation_observer,
                                ViolationSeverity::Error,
                                ViolationKind::FrameSync,
                                "spectator: divergent duplicate input for player {} at frame {} ignored",
                                player,
                                input.frame
                            );
                        }
                    } else {
                        report_violation!(
                            ViolationSeverity::Warning,
                            ViolationKind::InternalError,
                            "Failed to store input for player {} at frame {} - player index out of bounds",
                            player,
                            input.frame
                        );
                        return None;
                    }
                } else {
                    report_violation!(
                        ViolationSeverity::Warning,
                        ViolationKind::InternalError,
                        "Failed to store input at frame {} - frame index {} out of bounds",
                        input.frame,
                        frame_index
                    );
                    return None;
                }

                // Whether this input is at or beyond the live-edge frontier. Only the
                // freshest host should refresh the shared connect-status and frame
                // advantage; a lagging redundant host must not overwrite them with
                // stale data.
                let is_frontier = input.frame >= self.last_recv_frame;

                // advance the live edge only on strictly newer frames
                if input.frame > self.last_recv_frame {
                    self.last_recv_frame = input.frame;
                }

                if let Some(host) = self.hosts.get_mut(host_index) {
                    if is_frontier {
                        host.update_local_frame_advantage(input.frame);
                    }

                    // Update host connection status from THIS host. Connected
                    // last-frame freshness only comes from frontier hosts, but
                    // disconnect cutoffs are monotonic knowledge and must still
                    // be accepted from lagging hosts.
                    for i in 0..self.num_players {
                        let status = host.peer_connect_status(PlayerHandle::new(i));
                        if let Some(slot) = self.host_connect_status.get_mut(i) {
                            merge_connection_status_if_relevant(slot, status, is_frontier);
                        } else {
                            report_violation!(
                                ViolationSeverity::Warning,
                                ViolationKind::InternalError,
                                "Failed to update connection status for player {} - index out of bounds",
                                i
                            );
                        }
                    }
                } else {
                    report_violation!(
                        ViolationSeverity::Warning,
                        ViolationKind::InternalError,
                        "Received input from unknown host index {} - ignoring frame advantage update",
                        host_index
                    );
                }
            },
        }

        // check event queue size and discard oldest events if too big
        while self.event_queue.len() > self.max_event_queue_size {
            self.event_queue.pop_front();
        }

        disconnected_host
    }
}

fn merge_connection_status(current: &mut ConnectionStatus, incoming: ConnectionStatus) {
    if current.disconnected {
        if incoming.disconnected {
            current.last_frame = std::cmp::min(current.last_frame, incoming.last_frame);
        }
        return;
    }

    if incoming.disconnected {
        current.disconnected = true;
        current.last_frame = incoming.last_frame;
    } else {
        current.last_frame = std::cmp::max(current.last_frame, incoming.last_frame);
    }
}

fn merge_connection_status_if_relevant(
    current: &mut ConnectionStatus,
    incoming: ConnectionStatus,
    is_frontier: bool,
) {
    if is_frontier || incoming.disconnected {
        merge_connection_status(current, incoming);
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
    fn spectator_duplicate_same_frame_divergent_input_keeps_first_and_reports_violation() {
        use crate::telemetry::CollectingObserver;

        let observer = Arc::new(CollectingObserver::new());
        let mut session: SpectatorSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .with_violation_observer(observer.clone())
            .start_spectator_session(test_addr(7301), DummySocket)
            .unwrap();
        session.last_recv_frame = Frame::new(10);
        let frame = Frame::new(2);
        let player = PlayerHandle::new(0);
        let addr = test_addr(7301);

        assert_eq!(
            session.handle_event(
                0,
                Event::Input {
                    input: PlayerInput::new(frame, 11),
                    player,
                },
                addr,
            ),
            None
        );
        assert_eq!(
            session.handle_event(
                0,
                Event::Input {
                    input: PlayerInput::new(frame, 99),
                    player,
                },
                addr,
            ),
            None
        );

        let buffer_index = frame.buffer_index(session.buffer_size).unwrap();
        assert_eq!(session.inputs[buffer_index][0].input, 11_u8);
        assert!(observer
            .violations()
            .iter()
            .any(|violation| violation.kind == ViolationKind::FrameSync));
    }

    #[test]
    fn spectator_connection_status_merge_is_monotonic() {
        let mut current = ConnectionStatus {
            disconnected: true,
            last_frame: Frame::new(5),
        };
        merge_connection_status(
            &mut current,
            ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(10),
            },
        );
        assert_eq!(
            current,
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(5),
            }
        );

        merge_connection_status(
            &mut current,
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(3),
            },
        );
        assert_eq!(current.last_frame, Frame::new(3));

        let mut connected = ConnectionStatus {
            disconnected: false,
            last_frame: Frame::new(4),
        };
        merge_connection_status(
            &mut connected,
            ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(9),
            },
        );
        assert_eq!(
            connected,
            ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(9),
            }
        );
    }

    #[test]
    fn spectator_connection_status_merge_accepts_lagging_disconnect_only() {
        let mut current = ConnectionStatus {
            disconnected: false,
            last_frame: Frame::new(20),
        };

        merge_connection_status_if_relevant(
            &mut current,
            ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(10),
            },
            false,
        );
        assert_eq!(
            current,
            ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(20),
            }
        );

        merge_connection_status_if_relevant(
            &mut current,
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(7),
            },
            false,
        );
        assert_eq!(
            current,
            ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(7),
            }
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
