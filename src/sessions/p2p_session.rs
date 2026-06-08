use crate::error::{allocation_failed, FortressError, InternalErrorKind, InvalidRequestKind};
use crate::frame_info::PlayerInput;
use crate::network::messages::ConnectionStatus;
#[cfg(feature = "hot-join")]
use crate::network::messages::StateSnapshot;
use crate::network::network_stats::NetworkStats;
use crate::network::protocol::UdpProtocol;
use crate::replay::{Replay, ReplayRecorder};
use crate::safe_frame_sub;
use crate::sessions::config::{DisconnectBehavior, ProtocolConfig, SaveMode};
use crate::sessions::player_registry::PlayerRegistry;
use crate::sessions::session_trait::Session;
use crate::sessions::sync_health::SyncHealth;
use crate::sync_layer::SyncLayer;
use crate::telemetry::{
    InvariantChecker, InvariantViolation, SessionTelemetry, ViolationKind, ViolationObserver,
    ViolationSeverity,
};
use crate::DesyncDetection;
use crate::HandleVec;
use crate::{
    network::protocol::Event, Config, EventDrain, FortressEvent, FortressRequest, FortressResult,
    Frame, InvalidFrameReason, NonBlockingSocket, PlayerHandle, PlayerType, RequestVec,
    SessionState,
};
use crate::{report_violation, safe_frame_add};
use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::convert::TryInto;
use std::fmt;
use std::sync::Arc;
use tracing::{debug, trace};

/// Minimum frames between [`FortressEvent::WaitRecommendation`] events.
///
/// Set to 60 (1 second at 60fps) to avoid spamming the user with frequent
/// wait suggestions. This prevents the event queue from being overwhelmed
/// with wait recommendations during network instability.
const RECOMMENDATION_INTERVAL: Frame = Frame::new(60);

/// Minimum recommended frames to wait when behind.
///
/// When the session calculates that the local player should wait for
/// remote players to catch up, this ensures the recommendation is at
/// least 3 frames. This avoids micro-stuttering from very small waits
/// and provides enough time for network conditions to improve.
const MIN_RECOMMENDATION: u32 = 3;

/// Default maximum number of events to queue before oldest are dropped.
///
/// This prevents unbounded memory growth if events aren't being consumed.
/// At 100 events, there's ample buffer for typical network jitter while
/// providing backpressure if the application isn't processing events.
///
/// Note: This constant documents the default; the actual value is now
/// configurable via SessionBuilder::with_event_queue_size().
#[cfg(test)]
const DEFAULT_MAX_EVENT_QUEUE_SIZE: usize = 100;

/// Default for the hot-join serve timeout: the maximum number of
/// [`poll_remote_clients`](P2PSession::poll_remote_clients) calls a host will
/// keep a single hot-join serve open (re-sending the cached snapshot each poll)
/// before aborting it. Override per session with
/// [`SessionBuilder::with_hot_join_serve_timeout_polls`](crate::SessionBuilder::with_hot_join_serve_timeout_polls).
///
/// While a serve is open the **solo host is paused** (see
/// [`advance_frame`](P2PSession::advance_frame)), so this bounds how long an
/// abandoned join can stall the host. A generous value (~600 polls; at 60fps
/// that is ~10 seconds of poll calls) tolerates a slow/lossy handshake yet
/// guarantees the host eventually resumes solo if the joiner never acks.
///
/// On timeout the slot stays in `reserved_slots` (frozen/disconnected) so the
/// host resumes solo. The abort also clears the joiner endpoint's accumulated
/// `pending_output` (the abandoned joiner never needs those pre-snapshot host
/// inputs — a retry loads a snapshot), which both stops a `send_input` overflow
/// `Disconnected` storm and leaves the endpoint able to re-serve. Because the
/// slot stays reserved, a still-alive joiner that keeps sending `JoinRequest`s
/// (which it does automatically while `HotJoining`) — or a brand-new joiner
/// connection — re-opens a serve and completes the join **in-session**.
#[cfg(feature = "hot-join")]
pub(crate) const DEFAULT_HOT_JOIN_SERVE_TIMEOUT_POLLS: usize = 600;

/// Default maximum encoded hot-join snapshot wire-message size.
#[cfg(feature = "hot-join")]
pub(crate) const DEFAULT_HOT_JOIN_MAX_SNAPSHOT_WIRE_BYTES: usize =
    crate::sessions::hot_join::DEFAULT_HOT_JOIN_MAX_SNAPSHOT_WIRE_BYTES;

/// Default for the hot-join ack-resend budget: the number of
/// [`poll_remote_clients`](P2PSession::poll_remote_clients) calls the joiner
/// keeps re-sending its `StateSnapshotAck` after applying the snapshot. Override
/// per session with
/// [`SessionBuilder::with_hot_join_ack_resends`](crate::SessionBuilder::with_hot_join_ack_resends).
///
/// The joiner cannot directly observe that the host received its ack and
/// reactivated the slot, so it re-acks for a bounded number of polls to
/// tolerate ack loss. Acking stops early once the joiner starts receiving host
/// inputs for frames `>= F` (proof the host is past the activation frame and
/// the join completed). Kept small and deterministic for tests.
#[cfg(feature = "hot-join")]
pub(crate) const DEFAULT_HOT_JOIN_ACK_RESENDS: usize = 30;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum DisconnectEventPolicy {
    Suppress,
    Emit,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum GracefulDropFailurePolicy {
    Abort,
    DisconnectAndHalt,
}

/// A [`P2PSession`] provides all functionality to connect to remote clients in a peer-to-peer fashion, exchange inputs and handle the gamestate by saving, loading and advancing.
///
/// This type implements the [`Session`] trait, enabling it to be used in generic
/// code that works with any session type.
///
/// [`Session`]: crate::Session
pub struct P2PSession<T>
where
    T: Config,
{
    /// The number of players of the session.
    num_players: usize,
    /// The maximum number of frames Fortress Rollback will roll back. Every gamestate older than this is guaranteed to be correct.
    max_prediction: usize,
    /// The sync layer handles player input queues and provides predictions.
    sync_layer: SyncLayer<T>,
    /// Controls how game states are saved for rollback.
    save_mode: SaveMode,

    /// If we receive a disconnect from another client, we have to rollback from that frame on in order to prevent wrong predictions
    disconnect_frame: Frame,

    /// Internal State of the Session.
    state: SessionState,

    /// The [`P2PSession`] uses this socket to send and receive all messages for remote players.
    socket: Box<dyn NonBlockingSocket<T::Address>>,
    /// Handles players and their endpoints
    player_reg: PlayerRegistry<T>,
    /// This struct contains information about remote players, like connection status and the frame of last received input.
    local_connect_status: Vec<ConnectionStatus>,

    /// notes which inputs have already been sent to the spectators
    next_spectator_frame: Frame,
    /// The soonest frame on which the session can send a [`FortressEvent::WaitRecommendation`] again.
    next_recommended_sleep: Frame,
    /// How many frames we estimate we are ahead of every remote client
    frames_ahead: i32,

    /// Contains all events to be forwarded to the user.
    event_queue: VecDeque<FortressEvent<T>>,
    /// Contains all local inputs not yet sent into the system. This should have inputs for every local player before calling advance_frame
    local_inputs: BTreeMap<PlayerHandle, PlayerInput<T::Input>>,

    /// With desync detection, the session will compare checksums for all peers to detect discrepancies / desyncs between peers
    desync_detection: DesyncDetection,
    /// Desync detection over the network
    local_checksum_history: BTreeMap<Frame, u128>,
    /// The last frame we sent a checksum for
    last_sent_checksum_frame: Frame,
    /// Optional observer for specification violations.
    violation_observer: Option<Arc<dyn ViolationObserver>>,
    /// Optional telemetry observer for session performance events.
    telemetry: Option<Arc<dyn SessionTelemetry>>,
    /// Protocol configuration for network behavior.
    protocol_config: ProtocolConfig,
    /// Maximum number of events to queue before oldest are dropped.
    max_event_queue_size: usize,
    /// Optional replay recorder for capturing confirmed inputs.
    recording: Option<ReplayRecorder<T::Input>>,
    /// The last frame recorded to the replay recorder.
    last_recorded_frame: Frame,
    /// Controls how the session reacts when a peer disconnects.
    /// See [`DisconnectBehavior`] for options.
    disconnect_behavior: DisconnectBehavior,

    /// Hot-join state (host and joiner orchestration).
    ///
    /// Feature-gated behind `hot-join`; absent (and zero-cost) otherwise.
    #[cfg(feature = "hot-join")]
    hot_join: HotJoinState<T>,
}

/// Per-session hot-join orchestration state.
///
/// A single struct keeps the (numerous) feature-gated `P2PSession` fields
/// together so the non-`hot-join` build stays byte-identical.
#[cfg(feature = "hot-join")]
struct HotJoinState<T>
where
    T: Config,
{
    /// Reserved slots not yet filled by a joiner. While a handle is in this set
    /// the host treats it as a Feature-5 frozen/disconnected slot, and
    /// [`check_initial_sync`](P2PSession::check_initial_sync) skips its endpoint
    /// so the host reaches `Running` solo.
    ///
    /// A handle stays here for its *entire* serving lifetime (it is also added
    /// to [`joining`](Self::joining) while a serve is open) and is only removed
    /// once the join completes (ack received). On a serve timeout it remains
    /// here so the host can resume solo and the joiner can retry.
    ///
    /// Populated from [`add_reserved_player`](crate::SessionBuilder::add_reserved_player)
    /// at build time, **and** re-populated at runtime when a slot is cleanly
    /// gracefully dropped on a hot-join-serving host (see
    /// [`rearm_dropped_slot_for_rejoin`](P2PSession::rearm_dropped_slot_for_rejoin)),
    /// which is what makes a dropped slot re-joinable.
    reserved_slots: std::collections::BTreeSet<PlayerHandle>,
    /// Whether this session serves hot-joins (host role).
    accept_hot_join: bool,
    /// In-flight host-side serves, keyed by the reserved handle being filled.
    ///
    /// A handle is present here only while the host is actively serving its
    /// snapshot and waiting for the joiner's ack. The slot remains
    /// FROZEN + `disconnected = true` for this entire window (it is **not**
    /// reactivated until the ack arrives — see [`JoinServe`]). While this map
    /// is non-empty the host is **paused** (see
    /// [`advance_frame`](P2PSession::advance_frame)), which bounds the whole
    /// handshake and keeps the cached snapshot frame in-window.
    joining: BTreeMap<PlayerHandle, JoinServe<T>>,
    /// Joiner-side state, present only on a session built via
    /// [`SessionBuilder::start_hot_join_session`](crate::SessionBuilder::start_hot_join_session).
    joiner: Option<JoinerState<T>>,
    /// Host-side serve timeout (in polls). Defaults to
    /// [`DEFAULT_HOT_JOIN_SERVE_TIMEOUT_POLLS`]; configurable via the builder.
    /// Always `>= 2` (validated at the builder boundary).
    serve_timeout_polls: usize,
    /// Maximum complete encoded `StateSnapshot` wire-message size the host will
    /// serve. Defaults to [`DEFAULT_HOT_JOIN_MAX_SNAPSHOT_WIRE_BYTES`].
    max_snapshot_wire_bytes: usize,
    /// Joiner-side ack-resend budget (in polls). Defaults to
    /// [`DEFAULT_HOT_JOIN_ACK_RESENDS`]; configurable via the builder. May be 0
    /// (the joiner acks exactly once, with no loss tolerance).
    ack_resends: usize,
}

#[cfg(feature = "hot-join")]
impl<T: Config> Default for HotJoinState<T> {
    fn default() -> Self {
        Self {
            reserved_slots: std::collections::BTreeSet::new(),
            accept_hot_join: false,
            joining: BTreeMap::new(),
            joiner: None,
            serve_timeout_polls: DEFAULT_HOT_JOIN_SERVE_TIMEOUT_POLLS,
            max_snapshot_wire_bytes: DEFAULT_HOT_JOIN_MAX_SNAPSHOT_WIRE_BYTES,
            ack_resends: DEFAULT_HOT_JOIN_ACK_RESENDS,
        }
    }
}

/// Host-side per-join serving state, held in
/// [`HotJoinState::joining`](HotJoinState::joining) for the duration of a single
/// ack-gated join transaction.
///
/// The cached `snapshot` is captured **once** at serve time and re-sent on every
/// poll until the joiner acks (the reliable retransmit). Because the host is
/// paused while any serve is open, `frame` stays in the prediction window and
/// the cached state never goes stale, so the ack always matches an in-window
/// frame and no re-capture is needed.
#[cfg(feature = "hot-join")]
struct JoinServe<T>
where
    T: Config,
{
    /// The joiner endpoint's address (where the snapshot is re-sent).
    addr: T::Address,
    /// The activation frame `F` the snapshot was captured at. The join
    /// completes only when the joiner acks exactly this frame.
    frame: Frame,
    /// The snapshot captured once at `frame`, re-sent each poll (reliable
    /// retransmit). Cloned per send.
    snapshot: StateSnapshot,
    /// Number of polls since the serve began; aborts at the session's configured
    /// serve timeout (default [`DEFAULT_HOT_JOIN_SERVE_TIMEOUT_POLLS`]).
    polls_since_serve: usize,
}

#[cfg(feature = "hot-join")]
impl<T: Config> HotJoinState<T> {
    /// Returns `true` if the endpoint exclusively owns reserved-but-unjoined
    /// handle(s) — such an endpoint must not gate the host's sync transition and
    /// must not be auto-disconnected by the sync-timeout path while it waits for
    /// a joiner.
    fn endpoint_is_reserved(&self, endpoint: &UdpProtocol<T>) -> bool {
        if self.reserved_slots.is_empty() {
            return false;
        }
        let handles = endpoint.handles();
        !handles.is_empty()
            && handles
                .iter()
                .all(|handle| self.reserved_slots.contains(handle))
    }
}

/// Joiner-side hot-join state machine.
#[cfg(feature = "hot-join")]
struct JoinerState<T>
where
    T: Config,
{
    /// The local handle this joiner is filling.
    local_handle: PlayerHandle,
    /// The host address this joiner connects to.
    host_addr: T::Address,
    /// The `LoadGameState` request the user must process before the first
    /// `AdvanceFrame`. Taken on the next `advance_frame` call.
    pending_load: Option<FortressRequest<T>>,
    /// The activation frame the joiner applied the snapshot at, set once the
    /// snapshot is applied. `Some(F)` drives the bounded ack-resend below.
    applied_frame: Option<Frame>,
    /// Remaining polls for which the joiner will re-send its
    /// `StateSnapshotAck(F)` (ack-loss tolerance). Counts down each
    /// [`poll_remote_clients`](crate::P2PSession::poll_remote_clients); resends stop
    /// when it reaches zero or the joiner starts receiving host inputs for
    /// frames `>= F`. Re-armed to the session's configured budget (default
    /// [`DEFAULT_HOT_JOIN_ACK_RESENDS`]).
    ack_resends_remaining: usize,
}

/// Construction-time hot-join configuration handed from [`crate::SessionBuilder`] to
/// [`P2PSession::new`]. Folded into [`HotJoinState`] inside the constructor.
#[cfg(feature = "hot-join")]
pub(crate) struct HotJoinConfig<T>
where
    T: Config,
{
    /// Handles reserved for future joiners (host role).
    pub(crate) reserved_slots: std::collections::BTreeSet<PlayerHandle>,
    /// Whether this session serves hot-joins.
    pub(crate) accept_hot_join: bool,
    /// Joiner-side state, present only for sessions built via
    /// `start_hot_join_session`.
    pub(crate) joiner: Option<JoinerStateInit<T>>,
    /// Host-side serve timeout in polls (defaults to
    /// [`DEFAULT_HOT_JOIN_SERVE_TIMEOUT_POLLS`]). Guaranteed `>= 2` by the builder.
    pub(crate) serve_timeout_polls: usize,
    /// Maximum complete encoded `StateSnapshot` wire-message size the host will
    /// serve (defaults to [`DEFAULT_HOT_JOIN_MAX_SNAPSHOT_WIRE_BYTES`]).
    pub(crate) max_snapshot_wire_bytes: usize,
    /// Joiner-side ack-resend budget in polls (defaults to
    /// [`DEFAULT_HOT_JOIN_ACK_RESENDS`]).
    pub(crate) ack_resends: usize,
}

/// Construction-time joiner configuration. Mirrors [`JoinerState`] but holds
/// only the inputs known at build time.
#[cfg(feature = "hot-join")]
pub(crate) struct JoinerStateInit<T>
where
    T: Config,
{
    /// The local handle this joiner fills.
    pub(crate) local_handle: PlayerHandle,
    /// The host address this joiner connects to.
    pub(crate) host_addr: T::Address,
}

#[cfg(feature = "hot-join")]
impl<T: Config> Default for HotJoinConfig<T> {
    fn default() -> Self {
        Self {
            reserved_slots: std::collections::BTreeSet::new(),
            accept_hot_join: false,
            joiner: None,
            serve_timeout_polls: DEFAULT_HOT_JOIN_SERVE_TIMEOUT_POLLS,
            max_snapshot_wire_bytes: DEFAULT_HOT_JOIN_MAX_SNAPSHOT_WIRE_BYTES,
            ack_resends: DEFAULT_HOT_JOIN_ACK_RESENDS,
        }
    }
}

impl<T: Config> P2PSession<T> {
    /// Creates a new [`P2PSession`] for players who participate on the game input. After creating the session, add local and remote players,
    /// set input delay for local players and then start the session. The session will use the provided socket.
    ///
    /// Note: This is an internal constructor called via SessionBuilder. The many parameters are
    /// acceptable here because users interact through the builder pattern, not this method directly.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        num_players: usize,
        max_prediction: usize,
        socket: Box<dyn NonBlockingSocket<T::Address>>,
        players: PlayerRegistry<T>,
        save_mode: SaveMode,
        desync_detection: DesyncDetection,
        input_delay: usize,
        violation_observer: Option<Arc<dyn ViolationObserver>>,
        protocol_config: ProtocolConfig,
        queue_length: usize,
        event_queue_size: usize,
        recording: bool,
        telemetry: Option<Arc<dyn SessionTelemetry>>,
        disconnect_behavior: DisconnectBehavior,
        #[cfg(feature = "hot-join")] hot_join: HotJoinConfig<T>,
    ) -> Result<Self, FortressError> {
        // local connection status
        let mut local_connect_status = Vec::new();
        local_connect_status
            .try_reserve_exact(num_players)
            .map_err(|_err| allocation_failed("p2p.local_connect_status", num_players))?;
        for _ in 0..num_players {
            local_connect_status.push(ConnectionStatus::default());
        }

        // sync layer & set input delay
        let mut sync_layer =
            SyncLayer::try_with_queue_length(num_players, max_prediction, queue_length)?;
        for (player_handle, player_type) in players.handles.iter() {
            if matches!(player_type, PlayerType::Local) {
                // This should never fail during construction as player handles are validated
                if let Err(e) = sync_layer.set_frame_delay(*player_handle, input_delay) {
                    report_violation!(
                        ViolationSeverity::Critical,
                        ViolationKind::InternalError,
                        "Failed to set frame delay for player {:?} during session construction: {}",
                        player_handle,
                        e
                    );
                }
            }
        }

        // initial session state - if there are no endpoints, we don't need a synchronization phase
        let state = if players.remotes.len() + players.spectators.len() == 0 {
            SessionState::Running
        } else {
            SessionState::Synchronizing
        };

        // A hot-joiner starts in HotJoining and only transitions to Running once
        // it has synchronized with the host AND applied a state snapshot.
        #[cfg(feature = "hot-join")]
        let state = if hot_join.joiner.is_some() {
            SessionState::HotJoining
        } else {
            state
        };

        // For each reserved (but not-yet-joined) slot, freeze its input queue and
        // mark it disconnected so it behaves exactly like a Feature-5 dropped slot
        // from frame 0: frozen default input, ignored by `confirmed_frame`. The
        // host then runs solo until a joiner fills the slot.
        #[cfg(feature = "hot-join")]
        for &handle in &hot_join.reserved_slots {
            // Reserved slots have no confirmed inputs yet (frozen from frame 0),
            // so there is no agreed freeze frame to roll back to. Passing
            // `Frame::NULL` leaves `last_confirmed_input` untouched (`None`),
            // preserving the existing default-input-from-frame-0 behavior.
            if let Err(e) = sync_layer.freeze_player(handle, Frame::NULL) {
                report_violation!(
                    ViolationSeverity::Critical,
                    ViolationKind::InternalError,
                    "Failed to freeze reserved hot-join slot {} during construction: {}",
                    handle,
                    e
                );
            }
            if let Some(status) = local_connect_status.get_mut(handle.as_usize()) {
                status.disconnected = true;
            } else {
                report_violation!(
                    ViolationSeverity::Critical,
                    ViolationKind::InternalError,
                    "Reserved hot-join slot {} has no connection status entry during construction",
                    handle
                );
            }
        }

        let save_mode = if max_prediction == 0 && save_mode == SaveMode::Sparse {
            // in lockstep mode, saving will never happen, but we use the last saved frame to mark
            // control marking frames confirmed, so we need to turn off sparse saving to ensure that
            // frames are marked as confirmed - otherwise we will never advance the game state.
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::Configuration,
                "Sparse saving setting is ignored because lockstep mode is on (max_prediction set to 0), so no saving will take place"
            );
            SaveMode::EveryFrame
        } else {
            save_mode
        };

        Ok(Self {
            state,
            num_players,
            max_prediction,
            save_mode,
            socket,
            local_connect_status,
            next_recommended_sleep: Frame::new(0),
            next_spectator_frame: Frame::new(0),
            frames_ahead: 0,
            sync_layer,
            disconnect_frame: Frame::NULL,
            player_reg: players,
            event_queue: VecDeque::new(),
            local_inputs: BTreeMap::new(),
            desync_detection,
            local_checksum_history: BTreeMap::new(),
            last_sent_checksum_frame: Frame::NULL,
            violation_observer,
            telemetry,
            protocol_config,
            max_event_queue_size: event_queue_size,
            recording: recording.then(|| ReplayRecorder::new(num_players)),
            last_recorded_frame: Frame::NULL,
            disconnect_behavior,
            #[cfg(feature = "hot-join")]
            hot_join: HotJoinState {
                reserved_slots: hot_join.reserved_slots,
                accept_hot_join: hot_join.accept_hot_join,
                joining: BTreeMap::new(),
                joiner: hot_join.joiner.map(|init| JoinerState {
                    local_handle: init.local_handle,
                    host_addr: init.host_addr,
                    pending_load: None,
                    applied_frame: None,
                    ack_resends_remaining: 0,
                }),
                serve_timeout_polls: hot_join.serve_timeout_polls,
                max_snapshot_wire_bytes: hot_join.max_snapshot_wire_bytes,
                ack_resends: hot_join.ack_resends,
            },
        })
    }

    /// Registers local input for a player for the current frame. This should be successfully called for every local player before calling [`advance_frame()`](Self::advance_frame).
    /// If this is called multiple times for the same player before advancing the frame, older given inputs will be overwritten.
    ///
    /// # Errors
    /// - Returns a [`FortressError`] when the given handle does not refer to a local player.
    ///
    pub fn add_local_input(
        &mut self,
        player_handle: PlayerHandle,
        input: T::Input,
    ) -> Result<(), FortressError> {
        // make sure the input is for a registered local player (zero-allocation check)
        if !self.player_reg.is_local_player(player_handle) {
            return Err(InvalidRequestKind::NotLocalPlayer {
                handle: player_handle,
            }
            .into());
        }
        let player_input = PlayerInput::<T::Input>::new(self.sync_layer.current_frame(), input);
        self.local_inputs.insert(player_handle, player_input);
        Ok(())
    }

    /// You should call this to notify Fortress Rollback that you are ready to advance your gamestate by a single frame.
    /// Returns an order-sensitive [`RequestVec`]. You should fulfill all requests in the exact order they are provided.
    /// Failure to do so will result in incorrect game state, potential desync, or errors returned from subsequent API calls.
    ///
    /// # Hot-join host pause
    ///
    /// With the `hot-join` feature, while a host is actively serving a join
    /// (from the moment it captures the snapshot until the joiner acks or the
    /// serve times out), this method returns an **empty** request set without
    /// advancing the simulation. In the 2-peer reserved-slot scope a serving
    /// host has no other active player, so pausing for the ~1–2 RTT handshake is
    /// safe and bounds the whole transaction: `current_frame` stays fixed (so
    /// the cached snapshot frame remains in the prediction window and the ack
    /// always matches an in-window frame) and the joiner-endpoint send queue
    /// cannot grow. The pause triggers **only** while a join is in flight; a
    /// normal session (or an idle host with an unfilled reserved slot) advances
    /// exactly as usual. Keep calling `advance_frame` (and/or
    /// [`poll_remote_clients`](Self::poll_remote_clients)) during the pause to
    /// drive the handshake to completion.
    ///
    /// # Errors
    /// - Returns a [`FortressError`] if the provided player handle refers to a remote player.
    /// - Returns a [`FortressError`] if the session is not yet ready to accept input. In this case, you either need to start the session or wait for synchronization between clients.
    ///
    /// [`RequestVec`]: crate::RequestVec
    #[must_use = "FortressRequests must be processed to advance the game state"]
    pub fn advance_frame(&mut self) -> FortressResult<RequestVec<T>> {
        // receive info from remote players, trigger events and send messages
        self.poll_remote_clients();

        // Apply propagated disconnect knowledge before the state gate. Under
        // Halt, a remote-reported drop must fail closed on the detecting call
        // itself instead of advancing one extra frame.
        self.update_player_disconnects();

        // session is not running and synchronized
        if self.state != SessionState::Running {
            trace!("Session not synchronized; returning error");
            return Err(FortressError::NotSynchronized);
        }

        // Hot-join: if a snapshot was just applied, the joiner must restore the
        // received state BEFORE any AdvanceFrame. Return exactly that LoadGameState
        // as the sole request for this call; subsequent calls run the normal path
        // from the activation frame.
        #[cfg(feature = "hot-join")]
        if let Some(joiner) = self.hot_join.joiner.as_mut() {
            if let Some(load) = joiner.pending_load.take() {
                let mut requests = RequestVec::<T>::new();
                requests.push(load);
                return Ok(requests);
            }
        }

        // Hot-join host PAUSE: while a join is being served (ack-gated), the solo
        // host must NOT advance the simulation. `poll_remote_clients` above
        // already drove the handshake this call; returning an empty request set
        // here holds `current_frame`/`last_saved_frame` stable so (a) the cached
        // serve snapshot frame stays in the prediction window and the ack always
        // matches an in-window frame, and (b) the joiner-endpoint `pending_output`
        // cannot grow (no `send_input` happens), structurally preventing the
        // pending-output overflow disconnect. Strictly gated on a non-empty
        // `joining` map, so a normal (non-hot-join, or idle-host) session is never
        // paused. Resumes automatically once the join completes or times out.
        #[cfg(feature = "hot-join")]
        if !self.hot_join.joining.is_empty() {
            return Ok(RequestVec::<T>::new());
        }

        // check if input for all local players is queued (zero-allocation via iterator)
        for handle in self.player_reg.local_player_handles_iter() {
            if !self.local_inputs.contains_key(&handle) {
                return Err(InvalidRequestKind::MissingLocalInput.into());
            }
        }

        /*
         *  DESYNC DETECTION
         */
        // Collect, send, compare and check the last checksums against the other peers. The timing
        // of this is important: since the checksum comparison looks at the current confirmed frame,
        // (and the sync layer will happily mark a frame as confirmed after requesting rollback and
        // resimulation of it, at which point that frame's new checksum will not be stored yet), we
        // must examine our checksum state *before* the sync layer is able to mark any frames as
        // confirmed.
        if self.desync_detection != DesyncDetection::Off {
            self.check_checksum_send_interval();
            self.compare_local_checksums_against_peers();
        }

        // This list of requests will be returned to the user.
        // SmallVec inline capacity of 4 covers the typical case (save + advance)
        // without heap allocation. During rollback, it spills to the heap as needed.
        let mut requests = RequestVec::<T>::new();

        /*
         * ROLLBACKS AND GAME STATE MANAGEMENT
         */

        // if in lockstep mode, we will only ever request to advance the frame when all inputs for
        // the current frame have been confirmed; therefore there's no need to roll back, and hence
        // no need to ever save the game state either.
        let lockstep = self.in_lockstep_mode();

        // if we are in the first frame, we have to save the state
        if self.sync_layer.current_frame() == 0 && !lockstep {
            trace!("Saving state of first frame");
            requests.push(self.sync_layer.save_current_state());
        }

        // find the confirmed frame for which we received all inputs
        let confirmed_frame = self.confirmed_frame();

        // check game consistency and roll back, if necessary
        if !lockstep {
            // the disconnect frame indicates if a rollback is necessary due to a previously
            // disconnected player (whose input would have been incorrectly predicted).
            let first_incorrect = self
                .sync_layer
                .check_simulation_consistency(self.disconnect_frame);
            // if we have an incorrect frame, then we need to rollback
            if first_incorrect != Frame::NULL {
                if let Some(telemetry) = &self.telemetry {
                    for (player, frame) in self
                        .sync_layer
                        .players_with_incorrect_predictions(self.disconnect_frame)
                    {
                        telemetry.on_prediction_miss(player, frame);
                    }
                }
                self.adjust_gamestate(first_incorrect, confirmed_frame, &mut requests)?;
                self.disconnect_frame = Frame::NULL;
            }

            // request gamestate save of current frame
            let last_saved = self.sync_layer.last_saved_frame();
            if self.save_mode == SaveMode::Sparse {
                self.check_last_saved_state(last_saved, confirmed_frame, &mut requests)?;
            } else {
                // without sparse saving, always save the current frame after correcting and rollbacking
                requests.push(self.sync_layer.save_current_state());
            }
        }

        /*
         *  SEND OFF AND THROW AWAY INPUTS BEFORE THE CONFIRMED FRAME
         */

        // send confirmed inputs to spectators before throwing them away
        self.send_confirmed_inputs_to_spectators(confirmed_frame)?;

        // record confirmed inputs to the replay recorder before they are discarded
        self.record_confirmed_inputs(confirmed_frame);

        // set the last confirmed frame and discard all saved inputs before that frame
        self.sync_layer
            .set_last_confirmed_frame(confirmed_frame, self.save_mode);

        /*
         *  WAIT RECOMMENDATION
         */

        // check time sync between clients and send wait recommendation, if appropriate
        self.check_wait_recommendation();

        /*
         *  INPUTS
         */

        // register local inputs in the system and send them (zero-allocation via iterator)
        for handle in self.player_reg.local_player_handles_iter() {
            // we have checked that these all exist above, but return error for safety
            let player_input =
                self.local_inputs
                    .get_mut(&handle)
                    .ok_or_else(|| FortressError::MissingInput {
                        player_handle: handle,
                        frame: self.sync_layer.current_frame(),
                    })?;
            // send the input into the sync layer
            let actual_frame = self.sync_layer.add_local_input(handle, *player_input);
            player_input.frame = actual_frame;
            // if the input has not been dropped
            if actual_frame != Frame::NULL {
                self.local_connect_status
                    .get_mut(handle.as_usize())
                    .ok_or(FortressError::InternalErrorStructured {
                        kind: InternalErrorKind::ConnectionStatusIndexOutOfBounds {
                            player_handle: handle,
                        },
                    })?
                    .last_frame = actual_frame;
            }
        }

        // if the local inputs have not been dropped by the sync layer, send to all remote clients
        if !self.local_inputs.values().any(|&i| i.frame == Frame::NULL) {
            for endpoint in self.player_reg.remotes.values_mut() {
                endpoint.send_input(&self.local_inputs, &self.local_connect_status);
                endpoint.send_all_messages(&mut self.socket);
            }
        }

        /*
         * ADVANCE THE STATE
         */

        let can_advance = if lockstep {
            // lockstep mode: only advance if the current frame has inputs confirmed from all other
            // players.
            self.sync_layer.last_confirmed_frame() == self.sync_layer.current_frame()
        } else {
            // rollback mode: advance as long as we aren't past our prediction window
            let frames_ahead = if self.sync_layer.last_confirmed_frame().is_null() {
                // we haven't had any frames confirmed, so all frames we've advanced are "ahead"
                self.sync_layer.current_frame().as_i32()
            } else {
                // we're not at the first frame, so we have to subtract the last confirmed frame
                self.sync_layer.current_frame() - self.sync_layer.last_confirmed_frame()
            };
            frames_ahead < self.max_prediction as i32
        };
        if can_advance {
            // get correct inputs for the current frame
            let inputs = match self
                .sync_layer
                .synchronized_inputs(&self.local_connect_status)
            {
                Some(inputs) => inputs,
                None => {
                    report_violation!(
                        ViolationSeverity::Critical,
                        ViolationKind::InternalError,
                        "Failed to get synchronized inputs for frame {}",
                        self.sync_layer.current_frame()
                    );
                    return Err(FortressError::InternalErrorStructured {
                        kind: InternalErrorKind::SynchronizedInputsFailed {
                            frame: self.sync_layer.current_frame(),
                        },
                    });
                },
            };
            // advance the frame count
            self.sync_layer.advance_frame();
            // clear the local inputs after advancing the frame to allow new inputs to be ingested
            self.local_inputs.clear();
            requests.push(FortressRequest::AdvanceFrame { inputs });

            if let Some(telemetry) = &self.telemetry {
                telemetry.on_frame_advance(self.sync_layer.current_frame());
            }
        } else {
            debug!(
                "Prediction Threshold reached. Skipping on frame {}",
                self.sync_layer.current_frame()
            );
        }

        Ok(requests)
    }

    /// Should be called periodically by your application to give Fortress Rollback a chance to do internal work.
    /// Fortress Rollback will receive packets, distribute them to corresponding endpoints, handle all occurring events and send all outgoing packets.
    pub fn poll_remote_clients(&mut self) {
        // Get all packets and distribute them to associated endpoints.
        // The endpoints will handle their packets, which will trigger both events and UPD replies.
        for (from_addr, msg) in &self.socket.receive_all_messages() {
            if let Some(endpoint) = self.player_reg.remotes.get_mut(from_addr) {
                endpoint.handle_message(msg);
            }
            if let Some(endpoint) = self.player_reg.spectators.get_mut(from_addr) {
                endpoint.handle_message(msg);
            }
        }

        // update frame information between remote players
        for remote_endpoint in self.player_reg.remotes.values_mut() {
            if remote_endpoint.is_running() {
                remote_endpoint.update_local_frame_advantage(self.sync_layer.current_frame());
            }
        }

        // run endpoint poll and get events from players and spectators. This will trigger additional packets to be sent.
        let mut events = VecDeque::new();
        for endpoint in self.player_reg.remotes.values_mut() {
            let handles = endpoint.handles(); // Returns Arc<[PlayerHandle]>, cheap to clone
            let addr = endpoint.peer_addr();
            for event in endpoint.poll(&self.local_connect_status) {
                events.push_back((event, handles.clone(), addr.clone()))
            }
        }
        for endpoint in self.player_reg.spectators.values_mut() {
            let handles = endpoint.handles(); // Returns Arc<[PlayerHandle]>, cheap to clone
            let addr = endpoint.peer_addr();
            for event in endpoint.poll(&self.local_connect_status) {
                events.push_back((event, handles.clone(), addr.clone()))
            }
        }

        // handle all events locally
        for (event, handles, addr) in std::mem::take(&mut events) {
            self.handle_event(event, handles, addr);
        }

        // emit network stats telemetry for each running remote endpoint
        if let Some(telemetry) = &self.telemetry {
            for endpoint in self.player_reg.remotes.values() {
                if endpoint.is_running() {
                    if let Ok(stats) = endpoint.network_stats() {
                        for &handle in endpoint.handles().iter() {
                            telemetry.on_network_stats(handle, &stats);
                        }
                    }
                }
            }
        }

        // drive hot-join orchestration (host serve + joiner request/apply) so any
        // resulting JoinRequest/StateSnapshot/StateSnapshotAck is flushed below.
        #[cfg(feature = "hot-join")]
        self.poll_hot_join();

        // send all queued packets
        for endpoint in self.player_reg.remotes.values_mut() {
            endpoint.send_all_messages(&mut self.socket);
        }
        for endpoint in self.player_reg.spectators.values_mut() {
            endpoint.send_all_messages(&mut self.socket);
        }
    }

    /// Drives hot-join orchestration once per [`poll_remote_clients`](Self::poll_remote_clients) call:
    /// the host side serves snapshots for reserved slots, and the joiner side
    /// requests + applies a snapshot. Called after endpoint message handling and
    /// before the outgoing packet flush so any queued hot-join message is sent in
    /// the same poll.
    #[cfg(feature = "hot-join")]
    fn poll_hot_join(&mut self) {
        // A host whose only remaining unsynchronized endpoints are reserved slots
        // will never receive an `Event::Synchronized` to trigger the transition,
        // so drive it here. `check_initial_sync` skips reserved-unjoined endpoints
        // and early-returns unless we are Synchronizing, so this is a cheap no-op
        // once Running. Done whenever reserved slots exist, independent of the
        // serve flag, so a misconfigured host (reserved slot but serving disabled)
        // still reaches Running solo rather than hanging in Synchronizing.
        if !self.hot_join.reserved_slots.is_empty() {
            self.check_initial_sync();
        }
        if self.hot_join.accept_hot_join {
            self.poll_hot_join_host();
        }
        if self.hot_join.joiner.is_some() {
            self.poll_hot_join_joiner();
        }
    }

    /// Aborts an in-flight hot-join serve for `handle`, performing **all**
    /// teardown in one place so every abort path stays symmetric.
    ///
    /// Removing the serve from [`HotJoinState::joining`] is not enough: the
    /// joiner endpoint's `pending_output` has been accumulating the host's inputs
    /// since the pause began (the abandoned joiner never acked them, and a paused
    /// host never trims them). Left behind, those stale inputs make the next
    /// `send_input` see a full queue and raise an internal disconnect event on
    /// *every* subsequent frame. Clearing them also leaves the endpoint able to
    /// serve a fresh join cleanly (in-session retry). The joiner never needs these
    /// pre-snapshot inputs — a retry loads a snapshot.
    ///
    /// The slot is **kept** in `reserved_slots` (it stays frozen/disconnected), so
    /// the host resumes solo and a still-alive (or returning) joiner can re-open a
    /// serve. This is the *abort* teardown only; the successful-join path
    /// ([`poll_hot_join_host`](Self::poll_hot_join_host) Phase 3) deliberately does
    /// **not** clear `pending_output` (the joiner needs those frames `>= F`).
    ///
    /// Returns `true` if a serve was actually open for `handle`.
    #[cfg(feature = "hot-join")]
    fn abort_hot_join_serve(&mut self, handle: PlayerHandle) -> bool {
        let Some(serve) = self.hot_join.joining.remove(&handle) else {
            return false;
        };
        if let Some(endpoint) = self.player_reg.remotes.get_mut(&serve.addr) {
            endpoint.clear_pending_output();
        }
        true
    }

    /// Host side of hot-join: an **ack-gated, pause-based** join transaction.
    ///
    /// Each call performs four phases (in order). While any serve is open
    /// ([`HotJoinState::joining`] is non-empty) the solo host is paused by
    /// [`advance_frame`](Self::advance_frame), which bounds the whole handshake
    /// and keeps every cached serve frame in the prediction window.
    ///
    /// 1. **Open new serves.** Drain each endpoint's pending join request; for a
    ///    reserved slot not already serving, capture the snapshot ONCE at
    ///    `F = last_saved_frame`, cache it in [`JoinServe`], send it, and emit
    ///    [`FortressEvent::JoinRequested`] once. The slot stays FROZEN +
    ///    `disconnected = true` (it is **not** reactivated here).
    /// 2. **Re-send (reliable retransmit).** For every open serve, re-send the
    ///    cached snapshot and bump `polls_since_serve`. Fixes snapshot loss.
    /// 3. **Ack-gated reactivate.** Drain each serving endpoint's snapshot ack;
    ///    when it matches the cached `F`, NOW reactivate the slot, mark it
    ///    connected with `last_frame = F - 1`, drop it from both `joining` and
    ///    `reserved_slots`, and emit [`FortressEvent::PeerJoined`].
    /// 4. **Timeout.** A serve open longer than the configured serve timeout
    ///    (default [`DEFAULT_HOT_JOIN_SERVE_TIMEOUT_POLLS`])
    ///    is aborted: removed from `joining` but KEPT in `reserved_slots` (slot
    ///    stays frozen/disconnected, host resumes solo, joiner may retry).
    #[cfg(feature = "hot-join")]
    fn poll_hot_join_host(&mut self) {
        // Phase 1: drain pending join requests (addr, requested handle). Draining
        // releases the per-endpoint borrow so we can touch the sync layer below.
        let mut join_requests: Vec<(T::Address, usize)> = Vec::new();
        for endpoint in self.player_reg.remotes.values_mut() {
            let Some(requested) = endpoint.take_pending_join_request() else {
                continue;
            };
            // Authorization: only the endpoint that OWNS the requested handle may
            // solicit its snapshot. `requested` is peer-controlled, so without this
            // any connected endpoint could request another endpoint's reserved
            // slot and be served its snapshot (and have the wrong address paired to
            // that slot on reactivation). `handles()` is the immutable build-time
            // binding established by `add_reserved_player(addr, handle)`, so the
            // owning endpoint's `handles()` contains the reserved handle and no
            // other endpoint's does. (The reserved-and-unjoined check below is a
            // separate gate; this one is ownership.)
            if endpoint.handles().contains(&PlayerHandle::new(requested)) {
                join_requests.push((endpoint.peer_addr(), requested));
            } else {
                report_violation!(
                    ViolationSeverity::Warning,
                    ViolationKind::NetworkProtocol,
                    "Ignoring hot-join request for handle {} from endpoint {:?} that does not own it",
                    requested,
                    endpoint.peer_addr()
                );
            }
        }

        for (addr, requested) in join_requests {
            let handle = PlayerHandle::new(requested);
            // Only serve a slot that is genuinely reserved-and-unjoined.
            if !self.hot_join.reserved_slots.contains(&handle) {
                // A request for a non-reserved handle is ignored (it may be a
                // duplicate after the slot was already filled, or a bogus request).
                continue;
            }
            // Already serving this handle: ignore the duplicate request. The
            // re-send phase below drives retransmission; we must NOT re-capture
            // (the cached snapshot frame must stay stable while paused).
            if self.hot_join.joining.contains_key(&handle) {
                continue;
            }

            // Activation frame F = last_saved_frame (current_frame - 1). Because
            // the host pauses for the entire serve, F stays in-window.
            let activation_frame = self.sync_layer.last_saved_frame();
            if activation_frame.is_null() {
                // Nothing saved yet; the joiner will re-send. Skip this poll.
                continue;
            }
            // Defensive clamp: never serve a frame more than max_prediction
            // behind current_frame (outside the rollback window).
            let behind = self.sync_layer.current_frame() - activation_frame;
            if behind < 0 || behind > self.max_prediction as i32 {
                report_violation!(
                    ViolationSeverity::Warning,
                    ViolationKind::FrameSync,
                    "Skipping hot-join serve: activation frame {} is {} frames behind current {} (max_prediction {})",
                    activation_frame,
                    behind,
                    self.sync_layer.current_frame(),
                    self.max_prediction
                );
                continue;
            }

            let snapshot = match crate::sessions::hot_join::capture_snapshot_with_max_wire_bytes(
                &self.sync_layer,
                activation_frame,
                self.num_players,
                self.hot_join.max_snapshot_wire_bytes,
            ) {
                Ok(Some(snapshot)) => snapshot,
                Ok(None) => continue, // no valid saved state at F; retry next poll
                Err(e) => {
                    report_violation!(
                        ViolationSeverity::Error,
                        ViolationKind::InternalError,
                        "Failed to capture hot-join snapshot at frame {}: {}",
                        activation_frame,
                        e
                    );
                    continue;
                },
            };

            // The joiner endpoint must still exist to receive the snapshot that
            // Phase 2 sends below this same poll.
            if !self.player_reg.remotes.contains_key(&addr) {
                continue;
            }
            // Cache the serve. The actual send is Phase 2's job — it is the SOLE
            // snapshot-send site, so the serve we open here is transmitted exactly
            // once this poll (Phase 2 runs immediately below) and once per poll
            // thereafter. Sending here too would double the first poll's traffic
            // and desync the `polls_since_serve`/timeout accounting. The slot is
            // NOT reactivated yet: it stays frozen/disconnected until the ack
            // arrives (Phase 3). `handle` stays in `reserved_slots` AND is added to
            // `joining`, which pauses the host until the join resolves.
            self.hot_join.joining.insert(
                handle,
                JoinServe {
                    addr: addr.clone(),
                    frame: activation_frame,
                    snapshot,
                    polls_since_serve: 0,
                },
            );

            self.event_queue
                .push_back(FortressEvent::JoinRequested { handle, addr });
        }

        // Phase 2: the SOLE snapshot-send site (reliable retransmit). Send the
        // cached snapshot for every open serve — including one just opened in
        // Phase 1 — exactly once, and advance its poll counter. Because this is the
        // only place a snapshot is sent, each open serve emits exactly one snapshot
        // per poll, so `polls_since_serve` counts sends 1:1 and the Phase-4 timeout
        // accounting is exact. The host is paused, so the cached frame and state
        // are stable — we deliberately do NOT re-capture.
        for serve in self.hot_join.joining.values_mut() {
            serve.polls_since_serve = serve.polls_since_serve.saturating_add(1);
            if let Some(endpoint) = self.player_reg.remotes.get_mut(&serve.addr) {
                endpoint.send_state_snapshot(serve.snapshot.clone());
            }
        }

        // Phase 3: ack-gated reactivation. Collect (handle, acked_frame) for every
        // serving endpoint that produced a snapshot ack this poll. Draining the
        // ack releases the endpoint borrow before we mutate the sync layer.
        let mut acks: Vec<(PlayerHandle, Frame)> = Vec::new();
        for (&handle, serve) in &self.hot_join.joining {
            if let Some(endpoint) = self.player_reg.remotes.get_mut(&serve.addr) {
                if let Some(acked) = endpoint.take_received_snapshot_ack() {
                    acks.push((handle, acked));
                }
            }
        }
        for (handle, acked) in acks {
            let Some(serve) = self.hot_join.joining.get(&handle) else {
                continue;
            };
            // The ack must match the cached activation frame. Because the host is
            // paused, that frame is still in-window; a non-matching ack is stale
            // (e.g. an ack for an earlier aborted serve) and is ignored — the
            // joiner's bounded ack-resend will deliver the matching one.
            if acked != serve.frame {
                report_violation!(
                    ViolationSeverity::Warning,
                    ViolationKind::NetworkProtocol,
                    "Ignoring hot-join ack for frame {} (serving frame {}) on slot {}",
                    acked,
                    serve.frame,
                    handle
                );
                continue;
            }
            let activation_frame = serve.frame;
            let addr = serve.addr.clone();

            // NOW reactivate the slot: unfreeze + reposition the queue at F, mark
            // it connected, and reopen frames >= F as unconfirmed
            // (last_frame = F-1). `confirmed_frame` drops to F-1; the next advance
            // predicts handle h = RepeatLastConfirmed (== the frozen default ==
            // same result as before), and the existing misprediction -> rollback
            // path corrects when the joiner's real inputs arrive.
            if let Err(e) = self
                .sync_layer
                .reactivate_player_at_frame(handle, activation_frame)
            {
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::InternalError,
                    "Failed to reactivate hot-join slot {} at frame {}: {}",
                    handle,
                    activation_frame,
                    e
                );
                continue;
            }
            if let Some(status) = self.local_connect_status.get_mut(handle.as_usize()) {
                status.disconnected = false;
                status.last_frame =
                    safe_frame_sub!(activation_frame, 1, "P2PSession::poll_hot_join_host reopen");
            } else {
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::InternalError,
                    "Hot-join slot {} has no connection status entry during serve",
                    handle
                );
                continue;
            }
            // Force the host to re-simulate from the activation frame F so its
            // prediction for the reactivated handle anchors at F.
            //
            // The host is paused at `current_frame = F + 1` (the snapshot is the
            // saved state at `F = last_saved_frame`, which is `current_frame - 1`).
            // Without this, the host's first input request for the reactivated
            // handle is for `current_frame` (F+1), which creates a prediction
            // anchored at F+1 and SKIPS frame F entirely. The joiner, however,
            // loads the snapshot at F and contributes its first real input for
            // frame F. That input arrives "late" (after the host predicted past
            // it) and is silently dropped by the input queue's prediction-frame
            // check (`add_input_by_frame` rejects an input whose frame != the
            // advanced `prediction.frame`). The host then permanently simulates
            // frame F with the *previous* occupant's stale frozen input instead
            // of the joiner's real one, diverging at F+1 onward — a desync that is
            // only masked when the stale value happens to reduce to the same
            // game-state as the joiner's real input.
            //
            // Re-rooting the rollback at F (exactly the AI-takeover resimulation
            // mechanism) makes the host re-request `input(F)` first, anchoring the
            // prediction at F. `prediction.frame` then stays at F (the oldest
            // unconfirmed frame) until the joiner's real frame-F input arrives, so
            // that input is accepted and the standard misprediction -> rollback
            // path reconciles it — identical to how every other late remote input
            // is handled. `confirmed_frame` is still F-1 here, so no frame >= F is
            // checksum-compared until reconciliation completes.
            self.disconnect_frame = if self.disconnect_frame.is_null() {
                activation_frame
            } else {
                std::cmp::min(self.disconnect_frame, activation_frame)
            };
            // Join complete: retire the serve and the reservation, resume solo+peer
            // advancing (the host unpauses once `joining` is empty).
            self.hot_join.joining.remove(&handle);
            self.hot_join.reserved_slots.remove(&handle);

            self.event_queue
                .push_back(FortressEvent::PeerJoined { handle, addr });
        }

        // Phase 4: abort any serve that has been open too long. The slot stays in
        // `reserved_slots` (frozen/disconnected), so the host resumes solo once
        // `joining` empties. Because the slot stays reserved, a still-alive joiner
        // that later sends a fresh `JoinRequest` re-opens a serve (in-session
        // retry); see the joiner-endpoint cleanup below for why that works.
        // `polls_since_serve` is incremented once per poll in Phase 2 and starts at
        // 0, so after the Phase-2 send of poll number N it equals N. Aborting at
        // `>= serve_timeout_polls` therefore keeps the serve open for *exactly*
        // `serve_timeout_polls` polls (the documented maximum) before tearing it
        // down, rather than one poll longer.
        let serve_timeout_polls = self.hot_join.serve_timeout_polls;
        let timed_out: Vec<PlayerHandle> = self
            .hot_join
            .joining
            .iter()
            .filter(|(_, serve)| serve.polls_since_serve >= serve_timeout_polls)
            .map(|(&handle, _)| handle)
            .collect();
        for handle in timed_out {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::NetworkProtocol,
                "Hot-join serve for slot {} timed out after {} polls; aborting (slot stays reserved/frozen, host resumes solo)",
                handle,
                serve_timeout_polls
            );
            // Single teardown path — removes the serve AND clears the joiner
            // endpoint's stale pending_output. See `abort_hot_join_serve`.
            self.abort_hot_join_serve(handle);
        }
    }

    /// Joiner side of hot-join: once synchronized with the host, request a
    /// snapshot; when one arrives, apply it **idempotently** (buffering the
    /// `LoadGameState`), ack it, fix up connection status, and transition to
    /// `Running`. After applying, it re-sends the ack for a bounded number of
    /// polls so an ack lost in flight does not wedge the host's serve.
    ///
    /// Idempotent apply: a duplicate snapshot arriving after the transition to
    /// `Running` (the host re-sends until it sees an ack — see
    /// [`poll_hot_join_host`](Self::poll_hot_join_host)) is **not** re-applied;
    /// the joiner simply re-acks the already-applied frame and ignores the body.
    #[cfg(feature = "hot-join")]
    fn poll_hot_join_joiner(&mut self) {
        let Some(joiner) = self.hot_join.joiner.as_ref() else {
            return;
        };
        let host_addr = joiner.host_addr.clone();
        let local_handle = joiner.local_handle;
        let applied_frame = joiner.applied_frame;

        // While still HotJoining, request a snapshot once the host endpoint is
        // synchronized (protocol Running). Re-send each poll until a snapshot
        // arrives to tolerate loss; `send_join_request` is a no-op unless Running.
        if self.state == SessionState::HotJoining {
            if let Some(endpoint) = self.player_reg.remotes.get_mut(&host_addr) {
                if endpoint.is_running() {
                    endpoint.send_join_request(local_handle.as_usize());
                }
            }
        }

        // Drain a received snapshot, if any.
        let snapshot = self
            .player_reg
            .remotes
            .get_mut(&host_addr)
            .and_then(UdpProtocol::take_received_snapshot);

        // Once the snapshot is applied (`applied_frame` is `Some`, set in lockstep
        // with the transition to `Running`), this block is the SOLE ack-send site
        // and the function returns here without ever re-applying — a duplicate
        // snapshot is idempotently dropped. Keeping a single send site makes a
        // same-poll double-ack structurally impossible:
        //
        // - A (duplicate) snapshot arriving here proves the host has not yet seen
        //   our ack, so it re-arms the bounded resend budget.
        // - The bounded resend then sends at most ONE ack this poll, stopping once
        //   the host clearly received it (we observe host inputs for frames >= F,
        //   i.e. confirmed_frame >= F) or the resend budget is spent.
        if let Some(frame) = applied_frame {
            if snapshot.is_some() {
                let ack_resends = self.hot_join.ack_resends;
                if let Some(joiner) = self.hot_join.joiner.as_mut() {
                    joiner.ack_resends_remaining = ack_resends;
                }
            }
            let host_progressed = self.confirmed_frame() >= frame;
            let mut stop = host_progressed;
            if let Some(joiner) = self.hot_join.joiner.as_mut() {
                if joiner.ack_resends_remaining == 0 {
                    stop = true;
                } else {
                    joiner.ack_resends_remaining -= 1;
                }
            }
            if !stop {
                if let Some(endpoint) = self.player_reg.remotes.get_mut(&host_addr) {
                    endpoint.send_state_snapshot_ack(frame);
                }
            }
            return;
        }

        // Not yet applied (still `HotJoining`): a snapshot is required to proceed.
        let Some(snapshot) = snapshot else {
            return;
        };

        let activation_frame = snapshot.frame;
        let load = match crate::sessions::hot_join::apply_snapshot(
            &mut self.sync_layer,
            &snapshot,
            self.num_players,
        ) {
            Ok(load) => load,
            Err(e) => {
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::NetworkProtocol,
                    "Failed to apply hot-join snapshot at frame {}: {}",
                    activation_frame,
                    e
                );
                return;
            },
        };

        // Ack the loaded frame so the host reactivates + emits PeerJoined, and
        // resume input processing now that the activation frame is known: the
        // host's pending_output still holds frames >= F (the joiner never acked
        // while HotJoining, and the host was paused so it did not trim them), so
        // the joiner will receive and accept the host's inputs from F onward via
        // the normal protocol path.
        //
        // MINOR-1: `defer_input_processing` was set on every joiner remote at
        // build time but is cleared here only on the host endpoint. That is
        // correct *only* in the 2-peer reserved-slot scope this feature targets
        // (the joiner has exactly one remote — the host). Do NOT generalize this
        // to N-peer without revisiting which remotes must be un-deferred.
        if let Some(endpoint) = self.player_reg.remotes.get_mut(&host_addr) {
            endpoint.set_defer_input_processing(false);
            endpoint.send_state_snapshot_ack(activation_frame);
        }

        // Set every slot's connection status consistent with the seek: all players
        // are connected with last_frame = F - 1 (matching the joiner's sought
        // last_confirmed_frame). The joiner now contributes real inputs for its
        // own slot from F onward; the host slot is confirmed up to F-1 already.
        let baseline = safe_frame_sub!(
            activation_frame,
            1,
            "P2PSession::poll_hot_join_joiner baseline"
        );
        for status in &mut self.local_connect_status {
            status.disconnected = false;
            status.last_frame = baseline;
        }

        // Re-root desync-detection checksum sending onto the HOST's global
        // interval grid. Checksum comparison is by exact frame-number match
        // (`compare_local_checksums_against_peers` looks up `remote_frame` verbatim
        // in `local_checksum_history`), and the host (running from frame 0) only
        // ever sends/stores checksums at multiples of `interval`
        // (`check_checksum_send_interval`: first send at `interval`, then
        // `last_sent_checksum_frame + interval`). The joiner MUST land on that same
        // grid or the two never share a frame and desync detection is silently
        // disabled on the hot-join path.
        //
        // `check_checksum_send_interval` computes the next send as
        // `last_sent_checksum_frame + interval`, so to make the joiner's first
        // post-join send land on the first grid boundary >= F we set
        // `last_sent_checksum_frame = first_send - interval`:
        //   next_boundary = smallest multiple of interval >= F
        //   first_send    = max(next_boundary, interval)  // host never sends < interval
        //   anchor        = first_send - interval         // >= 0, never NULL/negative
        // F == 0 (or any F <= interval) clamps to `first_send = interval`, matching
        // the host (which never sends a checksum for frame 0). Only applied when
        // desync detection is On with interval >= 1.
        if let DesyncDetection::On { interval } = self.desync_detection {
            if interval >= 1 {
                let f = activation_frame.as_i32().max(0);
                let iv = (interval as i32).max(1);
                // Smallest multiple of `iv` that is >= `f` (ceil division;
                // i32::div_ceil is unstable, so compute it directly). All terms are
                // non-negative and `saturating_*` keeps it overflow-safe even for an
                // extreme `interval`/`f`.
                let next_boundary = f
                    .saturating_add(iv)
                    .saturating_sub(1)
                    .saturating_div(iv)
                    .saturating_mul(iv);
                let first_send = next_boundary.max(iv);
                self.last_sent_checksum_frame = Frame::new(first_send.saturating_sub(iv));
            }
        }

        // Buffer the LoadGameState so it is returned as the sole request on the
        // next advance_frame (the user restores the received state BEFORE any
        // AdvanceFrame), record the applied frame + arm the bounded ack-resend,
        // then go Running.
        let ack_resends = self.hot_join.ack_resends;
        if let Some(joiner) = self.hot_join.joiner.as_mut() {
            joiner.pending_load = Some(load);
            joiner.applied_frame = Some(activation_frame);
            joiner.ack_resends_remaining = ack_resends;
        }
        self.state = SessionState::Running;
    }

    /// Returns the configured [`DisconnectBehavior`] for this session.
    ///
    /// Set at construction time via
    /// [`crate::SessionBuilder::with_disconnect_behavior`]. Defaults to
    /// [`DisconnectBehavior::Halt`] for back-compat with the legacy
    /// GGRS-style "session halts on any peer drop" behavior.
    #[must_use]
    pub fn disconnect_behavior(&self) -> DisconnectBehavior {
        self.disconnect_behavior
    }

    /// Removes a remote player from the session and continues with the
    /// remaining peers (graceful drop), regardless of the session's
    /// configured [`DisconnectBehavior`].
    ///
    /// This is the **explicit** form of graceful drop. The configured
    /// [`DisconnectBehavior`] only governs **automatic** removal on the
    /// disconnect-timeout path; calling this method always opts in to the
    /// graceful-drop flow regardless of that setting. The mental model is:
    ///
    /// - `DisconnectBehavior::Halt` (default) + auto-timeout → session halts.
    /// - `DisconnectBehavior::ContinueWithout` + auto-timeout → graceful drop.
    /// - `remove_player(...)` (any `DisconnectBehavior`) → graceful drop.
    ///
    /// On invocation, the input queue is frozen (it repeats the last
    /// confirmed input forever) for **every** non-spectator player handle
    /// owned by the dropped endpoint — not just the targeted handle. A single
    /// remote address can host more than one player handle (e.g. couch co-op
    /// behind one socket); the graceful-drop contract applies to all of them.
    /// Each affected handle is marked disconnected on this session's
    /// connection-status table, the corresponding network endpoint is
    /// disconnected, and one [`FortressEvent::PeerDropped`] event per
    /// non-spectator handle is queued, followed by exactly one address-level
    /// [`FortressEvent::Disconnected`] event in the same batch for back-compat
    /// with code that consumes the legacy event. Remaining peers continue
    /// advancing the session — the game layer decides how to handle the
    /// gameplay impact (AI takeover, pause, end the match, etc.).
    ///
    /// For automatic graceful drop on disconnect timeout, configure
    /// [`crate::SessionBuilder::with_disconnect_behavior`] with
    /// [`DisconnectBehavior::ContinueWithout`].
    ///
    /// # Re-joining the dropped slot (hot-join)
    ///
    /// With the `hot-join` feature enabled and serving turned on (via
    /// `SessionBuilder::with_hot_join`), a cleanly removed slot is automatically
    /// returned to the reserved/frozen state, so a peer can re-fill it later via
    /// `SessionBuilder::start_hot_join_session` from the same address. Without
    /// hot-join serving the slot stays dropped for the remainder of the session.
    /// (The method names are unlinked here because they only exist under the
    /// `hot-join` feature, which the default doc build does not enable.)
    ///
    /// # Spectator endpoints at the same address
    ///
    /// `remove_player` operates on the **Remote** endpoint at the address
    /// only. A `Spectator` endpoint registered at the same `T::Address` is
    /// an independent endpoint and is **not** affected — it remains running
    /// and continues receiving forwarded inputs until it disconnects on its
    /// own. Co-locating a `Remote` and a `Spectator` at the same address is
    /// unusual; this note documents the behavior for that edge case.
    ///
    /// # Difference from [`Self::disconnect_player`]
    ///
    /// `disconnect_player` performs the legacy GGRS-style disconnect:
    /// it marks the player disconnected and disconnects the endpoint, but
    /// does *not* freeze the input queue or emit `PeerDropped`. With the
    /// default [`DisconnectBehavior::Halt`], that is enough to halt
    /// further frame advance because `confirmed_frame` stops progressing.
    ///
    /// `remove_player` extends that with the freeze + `PeerDropped` event
    /// emission required for graceful peer drop.
    ///
    /// # Errors
    /// - Returns [`InvalidRequestKind::DisconnectInvalidHandle`] if
    ///   `player_handle` is unregistered, or refers to a spectator (graceful
    ///   removal applies to remote player handles only — spectator endpoints
    ///   disconnect via their own lifecycle).
    /// - Returns [`InvalidRequestKind::DisconnectLocalPlayer`] if
    ///   `player_handle` refers to a local player.
    /// - Returns [`InvalidRequestKind::PlayerAlreadyRemoved`] if `player_handle`
    ///   is already marked disconnected — either by a previous `remove_player`
    ///   call, by auto-removal under [`DisconnectBehavior::ContinueWithout`],
    ///   or by a previous explicit [`Self::disconnect_player`] call.
    /// - Returns a [`FortressError::InternalErrorStructured`] (e.g.
    ///   [`InternalErrorKind::DisconnectStatusNotFound`] or
    ///   [`InternalErrorKind::IndexOutOfBounds`]) if an internal invariant is
    ///   violated (a registered handle has no corresponding input queue or
    ///   connection-status entry). These should not occur in correct code;
    ///   treat them as a library bug and report.
    ///
    /// [`InvalidRequestKind::DisconnectInvalidHandle`]: crate::error::InvalidRequestKind::DisconnectInvalidHandle
    /// [`InvalidRequestKind::DisconnectLocalPlayer`]: crate::error::InvalidRequestKind::DisconnectLocalPlayer
    /// [`InvalidRequestKind::PlayerAlreadyRemoved`]: crate::error::InvalidRequestKind::PlayerAlreadyRemoved
    /// [`InternalErrorKind::DisconnectStatusNotFound`]: crate::error::InternalErrorKind::DisconnectStatusNotFound
    /// [`InternalErrorKind::IndexOutOfBounds`]: crate::error::InternalErrorKind::IndexOutOfBounds
    #[must_use = "remove_player errors should be handled"]
    pub fn remove_player(&mut self, player_handle: PlayerHandle) -> Result<(), FortressError> {
        let player_type = self.player_reg.handles.get(&player_handle).ok_or(
            InvalidRequestKind::DisconnectInvalidHandle {
                handle: player_handle,
            },
        )?;
        match player_type {
            PlayerType::Local => {
                return Err(InvalidRequestKind::DisconnectLocalPlayer {
                    handle: player_handle,
                }
                .into());
            },
            PlayerType::Spectator(_) => {
                return Err(InvalidRequestKind::DisconnectInvalidHandle {
                    handle: player_handle,
                }
                .into());
            },
            PlayerType::Remote(_) => {},
        }

        // Verify the player isn't already removed/disconnected. Using
        // PlayerAlreadyRemoved (not AlreadyDisconnected) so applications can
        // distinguish "double remove_player call" from the legacy double
        // disconnect_player error.
        let status = self
            .local_connect_status
            .get(player_handle.as_usize())
            .ok_or(FortressError::InternalErrorStructured {
                kind: InternalErrorKind::DisconnectStatusNotFound { player_handle },
            })?;
        if status.disconnected {
            return Err(InvalidRequestKind::PlayerAlreadyRemoved {
                handle: player_handle,
            }
            .into());
        }
        self.disconnect_player_with_policy(
            player_handle,
            None,
            DisconnectBehavior::ContinueWithout,
            DisconnectEventPolicy::Emit,
            GracefulDropFailurePolicy::Abort,
        )
    }

    /// Disconnects a remote player and all other remote players with the same address from the session.
    ///
    /// # Spectator endpoints at the same address
    ///
    /// When `player_handle` refers to a **Remote** player, `disconnect_player`
    /// disconnects the Remote endpoint at the address only. A `Spectator`
    /// endpoint registered at the same `T::Address` is an independent
    /// endpoint and is **not** affected — it remains running and continues
    /// receiving forwarded inputs until it disconnects on its own. When
    /// `player_handle` refers to a **Spectator**, only that specific
    /// spectator endpoint is disconnected; any Remote endpoint at the same
    /// address is left running. Co-locating a `Remote` and a `Spectator` at
    /// the same address is unusual; this note documents the behavior for
    /// that edge case.
    ///
    /// # Errors
    /// - Returns [`InvalidRequestKind::DisconnectInvalidHandle`] if
    ///   `player_handle` is not a registered handle.
    /// - Returns [`InvalidRequestKind::DisconnectLocalPlayer`] if
    ///   `player_handle` refers to a local player.
    /// - Returns [`InvalidRequestKind::AlreadyDisconnected`] if
    ///   `player_handle` was already disconnected.
    /// - Returns a [`FortressError::InternalErrorStructured`] (e.g.
    ///   [`InternalErrorKind::DisconnectStatusNotFound`]) if an internal
    ///   invariant is violated (a registered remote handle has no
    ///   corresponding connection-status entry). This should not occur in
    ///   correct code; treat it as a library bug and report.
    ///
    /// [`InvalidRequestKind::DisconnectInvalidHandle`]: crate::error::InvalidRequestKind::DisconnectInvalidHandle
    /// [`InvalidRequestKind::DisconnectLocalPlayer`]: crate::error::InvalidRequestKind::DisconnectLocalPlayer
    /// [`InvalidRequestKind::AlreadyDisconnected`]: crate::error::InvalidRequestKind::AlreadyDisconnected
    /// [`InternalErrorKind::DisconnectStatusNotFound`]: crate::error::InternalErrorKind::DisconnectStatusNotFound
    #[must_use = "disconnect errors should be handled"]
    pub fn disconnect_player(&mut self, player_handle: PlayerHandle) -> Result<(), FortressError> {
        match self.player_reg.handles.get(&player_handle) {
            // the local player cannot be disconnected
            None => Err(InvalidRequestKind::DisconnectInvalidHandle {
                handle: player_handle,
            }
            .into()),
            Some(PlayerType::Local) => Err(InvalidRequestKind::DisconnectLocalPlayer {
                handle: player_handle,
            }
            .into()),
            // a remote player can only be disconnected if not already disconnected, since there is some additional logic attached
            Some(PlayerType::Remote(_)) => {
                let status = self
                    .local_connect_status
                    .get(player_handle.as_usize())
                    .ok_or(FortressError::InternalErrorStructured {
                        kind: InternalErrorKind::DisconnectStatusNotFound { player_handle },
                    })?;
                if !status.disconnected {
                    return self.disconnect_player_with_policy(
                        player_handle,
                        None,
                        DisconnectBehavior::Halt,
                        DisconnectEventPolicy::Suppress,
                        GracefulDropFailurePolicy::DisconnectAndHalt,
                    );
                }
                Err(InvalidRequestKind::AlreadyDisconnected {
                    handle: player_handle,
                }
                .into())
            },
            // disconnecting spectators is simpler
            Some(PlayerType::Spectator(_)) => {
                self.disconnect_player_at_frame(player_handle, Frame::NULL);
                Ok(())
            },
        }
    }

    /// Returns a [`NetworkStats`] struct that gives information about the quality of the network connection.
    ///
    /// The returned struct includes:
    /// - Network quality metrics (ping, send queue length, bandwidth)
    /// - Frame advantage/disadvantage relative to the peer
    /// - **Checksum comparison data** for desync detection
    ///
    /// # Checksum Fields
    ///
    /// The checksum fields (`last_compared_frame`, `local_checksum`, `remote_checksum`,
    /// `checksums_match`) are populated when desync detection is enabled and at least
    /// one checksum comparison has occurred. Use these to detect game state divergence.
    ///
    /// # Errors
    /// - Returns a [`FortressError`] if the handle does not refer to a remote player or spectator.
    /// - Returns a [`FortressError`] if the session is not connected to other clients yet.
    pub fn network_stats(
        &self,
        player_handle: PlayerHandle,
    ) -> Result<NetworkStats, FortressError> {
        let mut stats = match self.player_reg.handles.get(&player_handle) {
            Some(PlayerType::Remote(addr)) => match self.player_reg.remotes.get(addr) {
                Some(endpoint) => endpoint.network_stats()?,
                None => {
                    return Err(FortressError::InternalErrorStructured {
                        kind: InternalErrorKind::EndpointNotFoundForRemote { player_handle },
                    });
                },
            },
            Some(PlayerType::Spectator(addr)) => match self.player_reg.spectators.get(addr) {
                Some(endpoint) => endpoint.network_stats()?,
                None => {
                    return Err(FortressError::InternalErrorStructured {
                        kind: InternalErrorKind::EndpointNotFoundForSpectator { player_handle },
                    });
                },
            },
            _ => {
                return Err(InvalidRequestKind::NotRemotePlayerOrSpectator {
                    handle: player_handle,
                }
                .into());
            },
        };

        // Populate checksum fields from local history and remote pending checksums
        self.populate_checksum_stats(&mut stats, player_handle);

        Ok(stats)
    }

    /// Populates the checksum-related fields in NetworkStats.
    fn populate_checksum_stats(&self, stats: &mut NetworkStats, player_handle: PlayerHandle) {
        // Get the remote endpoint's pending checksums
        let Some(player_type) = self.player_reg.handles.get(&player_handle) else {
            return;
        };
        let addr = match player_type {
            PlayerType::Remote(addr) => addr,
            _ => return,
        };
        let Some(remote) = self.player_reg.remotes.get(addr) else {
            return;
        };

        // Find the most recent frame where we have both local and remote checksums
        let mut latest_compared_frame: Option<Frame> = None;
        let mut latest_local: Option<u128> = None;
        let mut latest_remote: Option<u128> = None;

        for (&frame, &local_cs) in &self.local_checksum_history {
            if let Some(&remote_cs) = remote.pending_checksums.get(&frame) {
                if latest_compared_frame.is_none_or(|f| frame > f) {
                    latest_compared_frame = Some(frame);
                    latest_local = Some(local_cs);
                    latest_remote = Some(remote_cs);
                }
            }
        }

        stats.last_compared_frame = latest_compared_frame;
        stats.local_checksum = latest_local;
        stats.remote_checksum = latest_remote;
        stats.checksums_match = match (latest_local, latest_remote) {
            (Some(local), Some(remote)) => Some(local == remote),
            _ => None,
        };
    }

    /// Returns the highest confirmed frame where all inputs have been received.
    ///
    /// A "confirmed frame" means all players' inputs for that frame have been received
    /// and will not be rolled back. The game state at this frame is **locally correct**
    /// based on the inputs received.
    ///
    /// # Important: This Does NOT Guarantee Synchronization
    ///
    /// **Do NOT use this method alone to determine when to terminate a session.**
    ///
    /// This method tells you "inputs are confirmed locally" but does **not** mean:
    /// - Both peers have simulated to the same frame
    /// - Game state matches between peers
    /// - The session is safe to terminate
    ///
    /// Peers run asynchronously. When peer A's `confirmed_frame()` returns 100, peer B
    /// might still be at frame 80. If peer A terminates, peer B will continue processing
    /// more frames, leading to different final states.
    ///
    /// # Correct Termination Pattern
    ///
    /// Use [`sync_health`](Self::sync_health) in addition to `confirmed_frame()`:
    ///
    /// ```ignore
    /// // WRONG: Terminating based on confirmed_frame alone
    /// if session.confirmed_frame() >= target_frames {
    ///     break; // Dangerous! Peers may be at different frames!
    /// }
    ///
    /// // CORRECT: Use sync_health to verify peer synchronization
    /// if session.confirmed_frame() >= target_frames {
    ///     match session.sync_health(peer_handle) {
    ///         Some(SyncHealth::InSync) => break, // Safe to exit
    ///         Some(SyncHealth::DesyncDetected { .. }) => panic!("Desync!"),
    ///         _ => continue, // Keep polling until sync status is known
    ///     }
    /// }
    /// ```
    ///
    /// # Valid Uses
    ///
    /// - Knowing which frames are safe to discard from rollback history
    /// - Computing checksums over confirmed game state
    /// - Delta compression (older confirmed state won't change)
    /// - Progress reporting to users
    #[must_use]
    pub fn confirmed_frame(&self) -> Frame {
        let mut confirmed_frame = Frame::new(i32::MAX);

        for con_stat in &self.local_connect_status {
            if !con_stat.disconnected {
                confirmed_frame = std::cmp::min(confirmed_frame, con_stat.last_frame);
            }
        }

        // If all players are disconnected, this should not happen in a running session
        if confirmed_frame.as_i32() == i32::MAX {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::FrameSync,
                "No connected players found when computing confirmed_frame - returning 0 as fallback"
            );
            return Frame::new(0);
        }
        confirmed_frame
    }

    /// Returns the current frame of a session.
    #[must_use]
    pub fn current_frame(&self) -> Frame {
        self.sync_layer.current_frame()
    }

    /// Returns the maximum prediction window of a session.
    #[must_use]
    pub fn max_prediction(&self) -> usize {
        self.max_prediction
    }

    /// Returns true if the session is running in lockstep mode.
    ///
    /// In lockstep mode, a session will only advance if the current frame has inputs confirmed from
    /// all other players.
    #[must_use]
    pub fn in_lockstep_mode(&mut self) -> bool {
        self.max_prediction == 0
    }

    /// Returns the current [`SessionState`] of a session.
    #[must_use]
    pub fn current_state(&self) -> SessionState {
        self.state
    }

    /// Returns all events that happened since last queried for events. If the
    /// number of stored events exceeds the configured event queue size, the
    /// oldest events will be discarded.
    #[must_use = "events should be handled to react to session state changes"]
    pub fn events(&mut self) -> EventDrain<'_, T> {
        EventDrain::from_drain(self.event_queue.drain(..))
    }

    /// Returns the confirmed inputs for all players at a specific frame.
    ///
    /// This is useful for computing deterministic checksums over confirmed game state.
    /// The returned inputs are guaranteed to be the same across all peers for the same frame,
    /// making them suitable for desync detection and verification.
    ///
    /// # Arguments
    ///
    /// * `frame` - The frame to get confirmed inputs for. Must be <= `confirmed_frame()`.
    ///
    /// # Returns
    ///
    /// A vector of inputs for each player, in player handle order (0, 1, 2, ...).
    /// Returns an error if the frame is not confirmed yet or has been discarded.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let confirmed = session.confirmed_frame();
    /// if confirmed.as_i32() >= 100 {
    ///     let inputs = session.confirmed_inputs_for_frame(Frame::new(100))?;
    ///     // These inputs are deterministic across all peers
    ///     let checksum = compute_checksum(&inputs);
    /// }
    /// ```
    #[must_use = "confirmed inputs should be used for game state computation"]
    pub fn confirmed_inputs_for_frame(&self, frame: Frame) -> Result<Vec<T::Input>, FortressError> {
        if frame > self.confirmed_frame() {
            return Err(FortressError::InvalidFrameStructured {
                frame,
                reason: InvalidFrameReason::NotConfirmed {
                    confirmed_frame: self.confirmed_frame(),
                },
            });
        }
        self.sync_layer
            .confirmed_inputs(frame, &self.local_connect_status)
            .map(|inputs| inputs.into_iter().map(|pi| pi.input).collect())
    }

    /// Returns the number of players added to this session
    #[must_use]
    pub fn num_players(&self) -> usize {
        self.player_reg.num_players()
    }

    /// Return the number of spectators currently registered
    #[must_use]
    pub fn num_spectators(&self) -> usize {
        self.player_reg.num_spectators()
    }

    /// Returns `true` if replay recording is enabled for this session.
    ///
    /// Recording is enabled via [`SessionBuilder::with_recording`].
    ///
    /// # Example
    ///
    /// ```ignore
    /// if session.is_recording() {
    ///     println!("Recording inputs for replay");
    /// }
    /// ```
    ///
    /// [`SessionBuilder::with_recording`]: crate::SessionBuilder::with_recording
    #[must_use]
    pub fn is_recording(&self) -> bool {
        self.recording.is_some()
    }

    /// Consumes this session and returns the recorded [`Replay`], if recording
    /// was enabled.
    ///
    /// Returns `Ok(Replay)` if recording was enabled via
    /// [`SessionBuilder::with_recording`], or an error if recording was not
    /// enabled.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidRequestKind::NotSupported`] if recording was not enabled.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let replay = session.into_replay()?;
    /// let bytes = replay.to_bytes()?;
    /// std::fs::write("match.replay", bytes)?;
    /// ```
    ///
    /// [`SessionBuilder::with_recording`]: crate::SessionBuilder::with_recording
    pub fn into_replay(self) -> FortressResult<Replay<T::Input>> {
        self.recording
            .map(ReplayRecorder::into_replay)
            .ok_or_else(|| {
                InvalidRequestKind::NotSupported {
                    operation: "into_replay (recording not enabled)",
                }
                .into()
            })
    }

    /// Extracts the recorded [`Replay`] without consuming the session.
    ///
    /// After calling this, the session continues but recording is disabled
    /// (the recorder has been taken).
    ///
    /// # Errors
    ///
    /// Returns [`InvalidRequestKind::NotSupported`] if recording was not enabled
    /// or has already been taken.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let replay = session.take_replay()?;
    /// let bytes = replay.to_bytes()?;
    /// // Session continues without recording
    /// ```
    pub fn take_replay(&mut self) -> FortressResult<Replay<T::Input>> {
        self.recording
            .take()
            .map(ReplayRecorder::into_replay)
            .ok_or_else(|| {
                InvalidRequestKind::NotSupported {
                    operation: "take_replay (recording not enabled or already taken)",
                }
                .into()
            })
    }

    /// Records confirmed inputs up to the given frame into the replay recorder.
    ///
    /// When a frame's inputs cannot be retrieved (e.g., because the frame was
    /// already discarded from the input queue), default placeholder inputs are
    /// recorded to maintain frame index alignment, and the recorder's
    /// `skipped_frames` counter is incremented. Recording continues with
    /// subsequent frames rather than stopping at the first failure.
    fn record_confirmed_inputs(&mut self, confirmed_frame: Frame) {
        if self.recording.is_none() {
            return;
        }

        // Collect inputs first, then record them, to avoid overlapping borrows.
        // Entries are tagged as either real inputs or skipped placeholders.
        let mut frames_to_record: Vec<(Frame, Option<Vec<T::Input>>)> = Vec::new();
        let mut frame_to_record = self.last_recorded_frame.saturating_next();
        while frame_to_record <= confirmed_frame {
            match self.confirmed_inputs_for_frame(frame_to_record) {
                Ok(inputs) => {
                    frames_to_record.push((frame_to_record, Some(inputs)));
                },
                Err(err) => {
                    // If we can't get inputs for this frame, record a placeholder
                    // and continue collecting subsequent frames. This maintains
                    // frame index alignment in the replay.
                    report_violation!(
                        ViolationSeverity::Warning,
                        ViolationKind::InputQueue,
                        "record_confirmed_inputs: failed to get inputs for frame {} (skipping): {}",
                        frame_to_record,
                        err
                    );
                    frames_to_record.push((frame_to_record, None));
                },
            }
            frame_to_record = frame_to_record.saturating_next();
        }

        // Now record all collected frames
        if let Some(recorder) = self.recording.as_mut() {
            for (frame, maybe_inputs) in frames_to_record {
                match maybe_inputs {
                    Some(inputs) => {
                        let checksum = self
                            .sync_layer
                            .saved_state_by_frame(frame)
                            .and_then(|cell| cell.checksum());
                        recorder.record_frame(inputs, checksum);
                    },
                    None => {
                        if let Err(error) = recorder.record_skipped_frame() {
                            report_violation!(
                                ViolationSeverity::Error,
                                ViolationKind::InternalError,
                                "Failed to record skipped replay frame {}: {}",
                                frame,
                                error
                            );
                        }
                    },
                }
                self.last_recorded_frame = frame;
            }
        }
    }

    /// Returns an iterator over local player handles.
    ///
    /// This is a zero-allocation alternative to [`local_player_handles`].
    /// Use this when you only need to iterate once or want to avoid allocation.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// for handle in session.local_player_handles_iter() {
    ///     session.add_local_input(handle, get_local_input())?;
    /// }
    /// ```
    ///
    /// [`local_player_handles`]: Self::local_player_handles
    #[must_use = "iterators are lazy and do nothing unless consumed"]
    pub fn local_player_handles_iter(&self) -> impl Iterator<Item = PlayerHandle> + '_ {
        self.player_reg.local_player_handles_iter()
    }

    /// Returns the handles of local players that have been added.
    ///
    /// For a zero-allocation alternative, see [`local_player_handles_iter`].
    ///
    /// [`local_player_handles_iter`]: Self::local_player_handles_iter
    #[must_use]
    pub fn local_player_handles(&self) -> HandleVec {
        self.player_reg.local_player_handles()
    }

    /// Returns the handle for the first local player, if any.
    ///
    /// This is a zero-allocation convenience method for games with a single local player.
    /// For games with multiple local players, use [`Self::local_player_handles`].
    ///
    /// # Returns
    ///
    /// The first local player's handle, or `None` if there are no local players.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Typical usage: get the local player's handle for input submission
    /// if let Some(handle) = session.local_player_handle() {
    ///     session.add_local_input(handle, local_input)?;
    /// }
    /// ```
    #[must_use]
    pub fn local_player_handle(&self) -> Option<PlayerHandle> {
        self.player_reg.local_player_handles_iter().next()
    }

    /// Returns the single local player's handle, or an error if there isn't exactly one.
    ///
    /// This is a zero-allocation convenience method for the common case of games with exactly one
    /// local player (typical client in a networked game). It returns an error if:
    /// - No local players are registered (`NoLocalPlayers`)
    /// - More than one local player is registered (`MultipleLocalPlayers`)
    ///
    /// For games with multiple local players (e.g., local co-op), use
    /// [`Self::local_player_handles`] instead.
    ///
    /// # Errors
    ///
    /// - [`InvalidRequestKind::NoLocalPlayers`] if no local players are registered.
    /// - [`InvalidRequestKind::MultipleLocalPlayers`] if more than one local player is registered.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Typical usage for single-local-player games
    /// let handle = session.local_player_handle_required()?;
    /// session.add_local_input(handle, local_input)?;
    /// ```
    ///
    /// [`InvalidRequestKind::NoLocalPlayers`]: crate::InvalidRequestKind::NoLocalPlayers
    /// [`InvalidRequestKind::MultipleLocalPlayers`]: crate::InvalidRequestKind::MultipleLocalPlayers
    #[must_use = "returns the local player handle which should be used"]
    pub fn local_player_handle_required(&self) -> Result<PlayerHandle, FortressError> {
        self.player_reg.local_player_handle_required()
    }

    /// Returns an iterator over remote player handles.
    ///
    /// This is a zero-allocation alternative to [`remote_player_handles`].
    /// Use this when you only need to iterate once or want to avoid allocation.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// for handle in session.remote_player_handles_iter() {
    ///     let stats = session.network_stats(handle)?;
    ///     println!("Remote player {:?}: ping={}ms", handle, stats.ping);
    /// }
    /// ```
    ///
    /// [`remote_player_handles`]: Self::remote_player_handles
    #[must_use = "iterators are lazy and do nothing unless consumed"]
    pub fn remote_player_handles_iter(&self) -> impl Iterator<Item = PlayerHandle> + '_ {
        self.player_reg.remote_player_handles_iter()
    }

    /// Returns the first remote player's handle, if any.
    ///
    /// This is a zero-allocation convenience method for games with a single remote player
    /// (typical 1v1 networked game). For games with multiple remote players,
    /// use [`Self::remote_player_handles`].
    ///
    /// # Examples
    ///
    /// ```ignore
    /// if let Some(remote) = session.remote_player_handle() {
    ///     println!("Connected to remote player: {:?}", remote);
    /// }
    /// ```
    #[must_use]
    pub fn remote_player_handle(&self) -> Option<PlayerHandle> {
        self.player_reg.remote_player_handles_iter().next()
    }

    /// Returns the single remote player's handle, or an error if there isn't exactly one.
    ///
    /// This is the preferred zero-allocation method for 1v1 games where you expect exactly one
    /// remote opponent. For games with multiple remote players, use [`Self::remote_player_handles`] instead.
    ///
    /// # Errors
    ///
    /// * [`FortressError::InvalidRequestStructured`] with [`InvalidRequestKind::NoRemotePlayers`]
    ///   if no remote players are registered.
    /// * [`FortressError::InvalidRequestStructured`] with [`InvalidRequestKind::MultipleRemotePlayers`]
    ///   if more than one remote player is registered.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Typical 1v1 game: get the opponent's handle
    /// let opponent = session.remote_player_handle_required()?;
    /// let stats = session.network_stats(opponent)?;
    /// println!("Ping to opponent: {}ms", stats.ping);
    /// ```
    ///
    /// [`FortressError::InvalidRequestStructured`]: crate::FortressError::InvalidRequestStructured
    /// [`InvalidRequestKind::NoRemotePlayers`]: crate::InvalidRequestKind::NoRemotePlayers
    /// [`InvalidRequestKind::MultipleRemotePlayers`]: crate::InvalidRequestKind::MultipleRemotePlayers
    #[must_use = "returns the remote player handle which should be used"]
    pub fn remote_player_handle_required(&self) -> Result<PlayerHandle, FortressError> {
        self.player_reg.remote_player_handle_required()
    }

    /// Returns `true` if the given handle refers to a local player.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// for handle in session.all_player_handles() {
    ///     if session.is_local_player(handle) {
    ///         session.add_local_input(handle, get_local_input())?;
    ///     }
    /// }
    /// ```
    #[must_use]
    pub fn is_local_player(&self, handle: PlayerHandle) -> bool {
        self.player_reg.is_local_player(handle)
    }

    /// Returns `true` if the given handle refers to a remote player.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// for handle in session.all_player_handles() {
    ///     if session.is_remote_player(handle) {
    ///         println!("Remote player: {:?}", handle);
    ///     }
    /// }
    /// ```
    #[must_use]
    pub fn is_remote_player(&self, handle: PlayerHandle) -> bool {
        self.player_reg.is_remote_player(handle)
    }

    /// Returns `true` if the given handle refers to a spectator.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// for handle in session.spectator_handles() {
    ///     assert!(session.is_spectator_handle(handle));
    /// }
    /// ```
    #[must_use]
    pub fn is_spectator_handle(&self, handle: PlayerHandle) -> bool {
        self.player_reg.is_spectator_handle(handle)
    }

    /// Returns the player type for the given handle, if registered.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// match session.player_type(handle) {
    ///     Some(PlayerType::Local) => println!("Local player"),
    ///     Some(PlayerType::Remote(addr)) => println!("Remote at {}", addr),
    ///     Some(PlayerType::Spectator(addr)) => println!("Spectator at {}", addr),
    ///     None => println!("Unknown handle"),
    /// }
    /// ```
    #[must_use]
    pub fn player_type(&self, handle: PlayerHandle) -> Option<PlayerType<T::Address>> {
        self.player_reg.player_type(handle)
    }

    /// Returns the number of local players in the session.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// if session.num_local_players() > 1 {
    ///     println!("This is a local co-op session");
    /// }
    /// ```
    #[must_use]
    pub fn num_local_players(&self) -> usize {
        self.player_reg.num_local_players()
    }

    /// Returns the number of remote players in the session (excluding spectators).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let remote_count = session.num_remote_players();
    /// println!("Connected to {} remote players", remote_count);
    /// ```
    #[must_use]
    pub fn num_remote_players(&self) -> usize {
        self.player_reg.num_remote_players()
    }

    /// Returns an iterator over all registered player handles.
    ///
    /// This is a zero-allocation alternative to [`all_player_handles`].
    /// Handles are returned in sorted order (BTreeMap iteration order).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// for handle in session.all_player_handles_iter() {
    ///     println!("Player handle: {:?}", handle);
    /// }
    /// ```
    ///
    /// [`all_player_handles`]: Self::all_player_handles
    #[must_use = "iterators are lazy and do nothing unless consumed"]
    pub fn all_player_handles_iter(&self) -> impl Iterator<Item = PlayerHandle> + '_ {
        self.player_reg.all_player_handles_iter()
    }

    /// Returns all registered player handles in sorted order.
    ///
    /// This includes local players, remote players, and spectators.
    /// For a zero-allocation alternative, see [`all_player_handles_iter`].
    ///
    /// # Examples
    ///
    /// ```ignore
    /// for handle in session.all_player_handles() {
    ///     println!("Player handle: {:?}", handle);
    /// }
    /// ```
    ///
    /// [`all_player_handles_iter`]: Self::all_player_handles_iter
    #[must_use]
    pub fn all_player_handles(&self) -> HandleVec {
        self.player_reg.all_player_handles()
    }

    /// Returns the handles of remote players that have been added.
    ///
    /// For a zero-allocation alternative, see [`remote_player_handles_iter`].
    ///
    /// [`remote_player_handles_iter`]: Self::remote_player_handles_iter
    #[must_use]
    pub fn remote_player_handles(&self) -> HandleVec {
        self.player_reg.remote_player_handles()
    }

    /// Returns an iterator over spectator handles.
    ///
    /// This is a zero-allocation alternative to [`spectator_handles`].
    /// Use this when you only need to iterate once or want to avoid allocation.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// for handle in session.spectator_handles_iter() {
    ///     println!("Spectator: {:?}", handle);
    /// }
    /// ```
    ///
    /// [`spectator_handles`]: Self::spectator_handles
    #[must_use = "iterators are lazy and do nothing unless consumed"]
    pub fn spectator_handles_iter(&self) -> impl Iterator<Item = PlayerHandle> + '_ {
        self.player_reg.spectator_handles_iter()
    }

    /// Returns the handles of spectators that have been added.
    ///
    /// For a zero-allocation alternative, see [`spectator_handles_iter`].
    ///
    /// [`spectator_handles_iter`]: Self::spectator_handles_iter
    #[must_use]
    pub fn spectator_handles(&self) -> HandleVec {
        self.player_reg.spectator_handles()
    }

    /// Returns an iterator over handles associated with a given address.
    ///
    /// This is a zero-allocation alternative to [`handles_by_address`].
    /// Use this when you only need to iterate once or want to avoid allocation.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// for handle in session.handles_by_address_iter(&peer_addr) {
    ///     println!("Handle at {}: {:?}", peer_addr, handle);
    /// }
    /// ```
    ///
    /// [`handles_by_address`]: Self::handles_by_address
    #[must_use = "iterators are lazy and do nothing unless consumed"]
    pub fn handles_by_address_iter<'a>(
        &'a self,
        addr: &'a T::Address,
    ) -> impl Iterator<Item = PlayerHandle> + 'a {
        self.player_reg.handles_by_address_iter(addr)
    }

    /// Returns all handles associated to a certain address.
    ///
    /// For a zero-allocation alternative, see [`handles_by_address_iter`].
    ///
    /// [`handles_by_address_iter`]: Self::handles_by_address_iter
    #[must_use]
    pub fn handles_by_address(&self, addr: &T::Address) -> HandleVec {
        self.player_reg.handles_by_address(addr)
    }

    /// Returns the number of frames this session is estimated to be ahead of other sessions
    #[must_use]
    pub fn frames_ahead(&self) -> i32 {
        self.frames_ahead
    }

    /// Adjusts the input delay for a local player at runtime.
    ///
    /// This enables hybrid delay+rollback: a small fixed delay (1-3 frames)
    /// reduces misprediction frequency. Call this in response to
    /// [`FortressEvent::InputDelayRecommendation`] events or your own
    /// heuristics.
    ///
    /// # Mid-session behavior
    ///
    /// - **Increasing** the delay mid-session is supported. The input queue
    ///   replicates the most recently added input across the new gap so
    ///   subsequent sequential inputs continue to be accepted, and the same
    ///   replicated frames are pushed onto every remote endpoint's
    ///   pending-output buffer so the remote peer's input sequence remains
    ///   strictly monotonic.
    /// - **Decreasing** the delay mid-session is **not** supported; doing so
    ///   would require dropping inputs that may already have been sent to
    ///   remote peers. An error is returned in that case.
    /// - Mid-session increases require **exactly one local player on this
    ///   peer**. The protocol bundles all local players' inputs into a single
    ///   packet per frame; with multiple local players, synthesizing
    ///   replicated bytes for the unaffected players' gap frames would
    ///   require knowing inputs they have not yet produced. Set the delay
    ///   before adding any inputs (typically via
    ///   [`SessionBuilder::with_input_delay`]) when running with multiple
    ///   local players.
    ///
    /// # Errors
    /// - Returns [`FortressError`] if `player_handle` is not a registered
    ///   local player.
    /// - Returns [`FortressError`] (`FrameDelayTooLarge`) if `delay` exceeds
    ///   `queue_length - 1`.
    /// - Returns [`FortressError`] (`InputDelayDecreaseUnsupported`) if
    ///   `delay` is less than the current delay and inputs have already been
    ///   added.
    /// - Returns [`FortressError`]
    ///   (`InputDelayMidSessionMultiLocalUnsupported`) if attempting to
    ///   increase delay mid-session with more than one local player.
    /// - Returns [`FortressError`]
    ///   (`InputDelayMidSessionPendingOutputFull`) if the requested increase
    ///   would push more gap-fill frames into a remote's pending-output
    ///   buffer than the configured `pending_output_limit` allows.
    /// - Returns [`FortressError::InternalErrorStructured`] with
    ///   [`InternalErrorKind::ConnectionStatusIndexOutOfBounds`] if the
    ///   local connect-status entry for `player_handle` is missing during
    ///   the mid-session gap-fill mirror step. This indicates an internal
    ///   library bug; reaching this branch should not occur in correct code.
    /// - Returns [`FortressError::InternalErrorStructured`] with
    ///   [`InternalErrorKind::IndexOutOfBounds`] (name `"input_queues"`) if
    ///   any sync-layer input-queue lookup performed while reading the
    ///   current/last-added/confirmed-input state, or while applying the
    ///   new frame delay, fails to resolve `player_handle`. Reaching this
    ///   branch indicates an internal-invariant violation and should not
    ///   occur in correct code.
    /// - Internal errors from the input queue or sync layer are surfaced
    ///   unchanged. In particular, the bubbled-up
    ///   [`SyncLayer::set_frame_delay`](crate::__internal::SyncLayer::set_frame_delay)
    ///   call may surface
    ///   [`InvalidRequestKind::FrameDelayTooLarge`],
    ///   [`InvalidRequestKind::InputDelayDecreaseUnsupported`],
    ///   [`InternalErrorKind::IndexOutOfBounds`] (name `"inputs"`), or
    ///   [`InternalErrorKind::InputQueueGapFillFailed`] from the underlying
    ///   input queue.
    /// - Returns [`FortressError::InvalidPlayerHandle`] if any of the
    ///   bubbled-up sync-layer calls
    ///   ([`SyncLayer::frame_delay`](crate::__internal::SyncLayer::frame_delay),
    ///   `SyncLayer::last_added_frame`, `SyncLayer::confirmed_input`, or
    ///   [`SyncLayer::set_frame_delay`](crate::__internal::SyncLayer::set_frame_delay))
    ///   reject `player_handle`. This is unreachable in practice because the
    ///   `is_local_player(player_handle)` guard above pre-validates the
    ///   handle, but is documented for completeness so callers can match it
    ///   without surprise on a coding-error path.
    /// - Returns [`InvalidRequestKind::NoConfirmedInput`] (surfaced through
    ///   `SyncLayer::confirmed_input` from the underlying
    ///   [`InputQueue::confirmed_input`](crate::__internal::InputQueue::confirmed_input))
    ///   if the gap-fill loop requests a frame for which the input queue has
    ///   no confirmed input. This is internally unreachable because the loop
    ///   only iterates over frames `prev_last_added + 1 ..= new_last_added`,
    ///   which the input queue has just produced as part of the same
    ///   `set_frame_delay` call; documented for completeness.
    ///
    /// [`InvalidRequestKind::FrameDelayTooLarge`]: crate::error::InvalidRequestKind::FrameDelayTooLarge
    /// [`InvalidRequestKind::InputDelayDecreaseUnsupported`]: crate::error::InvalidRequestKind::InputDelayDecreaseUnsupported
    /// [`InvalidRequestKind::NoConfirmedInput`]: crate::error::InvalidRequestKind::NoConfirmedInput
    /// [`InternalErrorKind::InputQueueGapFillFailed`]: crate::error::InternalErrorKind::InputQueueGapFillFailed
    ///
    /// # Example
    ///
    /// ```no_run
    /// use fortress_rollback::{
    ///     Config, FortressError, FortressEvent, P2PSession, PlayerHandle,
    /// };
    ///
    /// fn apply_recommendations<C: Config>(
    ///     session: &mut P2PSession<C>,
    /// ) -> Result<(), FortressError> {
    ///     // Collect recommendations first to avoid simultaneous mutable borrows
    ///     // of the session via events() and set_input_delay().
    ///     let recommendations: Vec<(PlayerHandle, usize)> = session
    ///         .events()
    ///         .filter_map(|event| match event {
    ///             FortressEvent::InputDelayRecommendation {
    ///                 player_handle,
    ///                 suggested_delay,
    ///                 ..
    ///             } => Some((player_handle, suggested_delay)),
    ///             _ => None,
    ///         })
    ///         .collect();
    ///     for (handle, delay) in recommendations {
    ///         session.set_input_delay(handle, delay)?;
    ///     }
    ///     Ok(())
    /// }
    /// ```
    ///
    /// [`FortressEvent::InputDelayRecommendation`]: crate::FortressEvent::InputDelayRecommendation
    /// [`SessionBuilder::with_input_delay`]: crate::SessionBuilder::with_input_delay
    pub fn set_input_delay(
        &mut self,
        player_handle: PlayerHandle,
        delay: usize,
    ) -> Result<(), FortressError> {
        if !self.player_reg.is_local_player(player_handle) {
            return Err(InvalidRequestKind::NotLocalPlayer {
                handle: player_handle,
            }
            .into());
        }

        let current_delay = self.sync_layer.frame_delay(player_handle)?;
        let prev_last_added = self.sync_layer.last_added_frame(player_handle)?;

        // Detect mid-session increase: there are inputs in the queue and the
        // requested delay is strictly greater than the current delay. Only in
        // this case do we need to coordinate gap-fill on the protocol layer;
        // the no-op, initial-setup, and decrease cases are handled entirely
        // by the input queue.
        let mid_session_increase = !prev_last_added.is_null()
            && delay > current_delay
            && delay <= self.sync_layer.max_frame_delay();

        if mid_session_increase {
            // Multi-local + mid-session increase is unsupported: see rustdoc.
            let local_players = self.player_reg.num_local_players();
            if local_players > 1 {
                return Err(
                    InvalidRequestKind::InputDelayMidSessionMultiLocalUnsupported { local_players }
                        .into(),
                );
            }
            // Verify every running remote endpoint has enough room for the
            // gap-fill entries up front, before we mutate the input queue.
            // Spectators receive *confirmed* inputs via a separate stream
            // (`send_confirmed_inputs`) and are therefore unaffected by this
            // gap-fill.
            let delta = delay - current_delay;
            let mut min_capacity = usize::MAX;
            for endpoint in self.player_reg.remotes.values() {
                min_capacity =
                    std::cmp::min(min_capacity, endpoint.pending_output_capacity_remaining());
            }
            if delta > min_capacity {
                return Err(InvalidRequestKind::InputDelayMidSessionPendingOutputFull {
                    delta,
                    capacity: min_capacity,
                }
                .into());
            }
        }

        // Mutate the input queue. After this returns Ok, last_added_frame has
        // advanced by `delta` if a mid-session gap-fill happened.
        self.sync_layer.set_frame_delay(player_handle, delay)?;

        if !mid_session_increase {
            return Ok(());
        }

        let new_last_added = self.sync_layer.last_added_frame(player_handle)?;
        let mut frame = safe_frame_add!(prev_last_added, 1, "P2PSession::set_input_delay gap fill");
        // Push one InputBytes per replicated gap frame onto every remote
        // endpoint's pending_output. We pre-validated capacity above.
        while frame <= new_last_added {
            let player_input = self.sync_layer.confirmed_input(player_handle, frame)?;
            let mut inputs = std::collections::BTreeMap::new();
            inputs.insert(player_handle, player_input);
            for endpoint in self.player_reg.remotes.values_mut() {
                endpoint.enqueue_replicated_input(&inputs);
            }
            frame = safe_frame_add!(frame, 1, "P2PSession::set_input_delay gap fill loop");
        }

        // Mirror the queue's advanced last_added_frame on the connect-status
        // record before flushing, so each Input message stamps the remote's
        // `peer_connect_status[player_handle].last_frame` to match the
        // gap-fill bytes payload. The `?` on the lookup surfaces an
        // internal-invariant break rather than silently skipping (see the
        // `ConnectionStatusIndexOutOfBounds` case in this method's
        // rustdoc).
        let status = self
            .local_connect_status
            .get_mut(player_handle.as_usize())
            .ok_or(FortressError::InternalErrorStructured {
                kind: InternalErrorKind::ConnectionStatusIndexOutOfBounds { player_handle },
            })?;
        status.last_frame = new_last_added;

        // Flush each remote's pending output now so the gap-fill frames travel
        // on the wire promptly, just as they would have if produced by a
        // regular advance_frame.
        for endpoint in self.player_reg.remotes.values_mut() {
            endpoint.flush_pending_output(&self.local_connect_status);
            endpoint.send_all_messages(&mut self.socket);
        }

        Ok(())
    }

    /// Returns the current input delay (in frames) for a local player.
    ///
    /// # Errors
    /// - Returns [`FortressError`] if `player_handle` is not a registered local player.
    ///
    /// # Example
    ///
    /// Inspect the current delay before applying a recommendation, so that
    /// only strict increases are forwarded to [`set_input_delay`]:
    ///
    /// ```no_run
    /// use fortress_rollback::{Config, FortressError, P2PSession, PlayerHandle};
    ///
    /// fn maybe_apply_recommendation<C: Config>(
    ///     session: &mut P2PSession<C>,
    ///     local_handle: PlayerHandle,
    ///     recommended_delay: usize,
    /// ) -> Result<(), FortressError> {
    ///     let current = session.input_delay(local_handle)?;
    ///     if recommended_delay > current {
    ///         session.set_input_delay(local_handle, recommended_delay)?;
    ///     }
    ///     Ok(())
    /// }
    /// ```
    ///
    /// [`set_input_delay`]: Self::set_input_delay
    pub fn input_delay(&self, player_handle: PlayerHandle) -> Result<usize, FortressError> {
        if !self.player_reg.is_local_player(player_handle) {
            return Err(InvalidRequestKind::NotLocalPlayer {
                handle: player_handle,
            }
            .into());
        }
        self.sync_layer.frame_delay(player_handle)
    }

    /// Returns the [`DesyncDetection`] mode set for this session at creation time.
    #[must_use]
    pub fn desync_detection(&self) -> DesyncDetection {
        self.desync_detection
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

    /// Returns a reference to the telemetry observer, if one is attached.
    ///
    /// In tests, the typical pattern is to keep a separate
    /// <code>Arc<[CollectingTelemetry]></code> clone and call methods on it directly,
    /// rather than going through this accessor. This method is primarily useful
    /// when you need to confirm that a telemetry observer is attached or to pass
    /// the trait object to other code.
    ///
    /// [CollectingTelemetry]: crate::telemetry::CollectingTelemetry
    #[must_use]
    pub fn telemetry(&self) -> Option<&Arc<dyn SessionTelemetry>> {
        self.telemetry.as_ref()
    }

    /// Returns the synchronization health status for a specific remote peer.
    ///
    /// This is the **primary API for checking if a session is synchronized** before
    /// termination or for ongoing desync detection. Unlike [`confirmed_frame`], which
    /// only indicates input confirmation, this method verifies that game state checksums
    /// match between peers.
    ///
    /// # Arguments
    ///
    /// * `player_handle` - The handle of the remote player to check.
    ///
    /// # Returns
    ///
    /// * `Some(SyncHealth::InSync)` - Checksums match at the last compared frame.
    /// * `Some(SyncHealth::Pending)` - No checksum comparison available yet.
    /// * `Some(SyncHealth::DesyncDetected { .. })` - Checksums differ, indicating desync.
    /// * `None` - The handle doesn't refer to a remote player.
    ///
    /// # Important Anti-Pattern Warning
    ///
    /// **Do NOT use [`confirmed_frame`](Self::confirmed_frame) alone to determine
    /// when to terminate a session.** That method only indicates when rollbacks won't affect
    /// a frame - it does NOT mean both peers have simulated to the same frame.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // WRONG: Using confirmed_frame for termination
    /// if session.confirmed_frame() >= target_frames {
    ///     break; // Dangerous! Peers may be at different frames!
    /// }
    ///
    /// // CORRECT: Use sync_health for safe termination
    /// if session.confirmed_frame() >= target_frames {
    ///     match session.sync_health(peer_handle) {
    ///         Some(SyncHealth::InSync) => break, // Safe to exit
    ///         Some(SyncHealth::DesyncDetected { .. }) => panic!("Desync!"),
    ///         _ => continue, // Keep polling
    ///     }
    /// }
    /// ```
    ///
    /// [`confirmed_frame`]: Self::confirmed_frame
    #[must_use]
    pub fn sync_health(&self, player_handle: PlayerHandle) -> Option<SyncHealth> {
        // Only remote players have sync health
        let player_type = self.player_reg.handles.get(&player_handle)?;
        let addr = match player_type {
            PlayerType::Remote(addr) => addr,
            _ => return None,
        };

        // Get the remote endpoint
        let remote = self.player_reg.remotes.get(addr)?;

        // If desync detection is off, we can't determine sync health
        if self.desync_detection == DesyncDetection::Off {
            return Some(SyncHealth::Pending);
        }

        // Check for any pending checksums that don't match our local history
        // This catches desyncs before they're processed by compare_local_checksums_against_peers
        for (&remote_frame, &remote_checksum) in &remote.pending_checksums {
            // Only compare frames that have been confirmed locally
            if remote_frame >= self.sync_layer.last_confirmed_frame() {
                continue;
            }
            if let Some(&local_checksum) = self.local_checksum_history.get(&remote_frame) {
                if local_checksum != remote_checksum {
                    return Some(SyncHealth::DesyncDetected {
                        frame: remote_frame,
                        local_checksum,
                        remote_checksum,
                    });
                }
            }
        }

        // If THIS peer has a verified frame (a checksum it sent matched our local
        // history), we're in sync with it. Per-peer so verification against another
        // remote does not leak into this peer's verdict (an N>=3 logical error).
        if remote.last_verified_frame.is_some() {
            return Some(SyncHealth::InSync);
        }

        // No successful comparison yet - still pending
        Some(SyncHealth::Pending)
    }

    /// Returns `true` if every currently-**connected** remote peer shows
    /// [`SyncHealth::InSync`].
    ///
    /// This is a convenience method that checks all connected remote peers at
    /// once. Returns `false` if any connected peer is pending or has detected a
    /// desync. Returns `true` if there are no remote peers, or if every remote
    /// peer is disconnected.
    ///
    /// A gracefully-dropped remote (its slot frozen under
    /// [`DisconnectBehavior::ContinueWithout`]) and a reserved-but-unjoined
    /// hot-join endpoint do **not** block synchronization: they are excluded
    /// using the same connection predicate as [`last_verified_frame`]
    /// (the private `remote_is_connected` helper). A peer that is still
    /// connected but not yet verified keeps this `false` until it verifies.
    ///
    /// [`last_verified_frame`]: Self::last_verified_frame
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Wait until all peers are synchronized before exiting
    /// if session.confirmed_frame() >= target_frames && session.is_synchronized() {
    ///     break;
    /// }
    /// ```
    #[must_use]
    pub fn is_synchronized(&self) -> bool {
        // Only currently-connected remotes gate synchronization. A
        // gracefully-dropped or reserved hot-join endpoint must not block this
        // forever, mirroring `last_verified_frame()`'s connection-aware filter.
        self.player_reg
            .remote_player_handles_iter()
            .filter(|&handle| {
                self.player_reg
                    .handles
                    .get(&handle)
                    .and_then(|player_type| match player_type {
                        PlayerType::Remote(addr) => self.player_reg.remotes.get(addr),
                        _ => None,
                    })
                    .is_some_and(|remote| self.remote_is_connected(remote))
            })
            .all(|handle| matches!(self.sync_health(handle), Some(SyncHealth::InSync)))
    }

    /// Returns the highest frame for which checksums have been verified to match.
    ///
    /// This is useful for ensuring synchronization has been verified up to a
    /// specific point before terminating a session.
    ///
    /// # Returns
    ///
    /// * `Some(frame)` - The highest frame where checksums matched with **every**
    ///   currently-connected remote peer (the `min` of each connected peer's
    ///   individually-verified frame).
    /// * `None` - No checksum comparison has successfully completed with every
    ///   connected peer yet, or there are no connected remote peers.
    ///
    /// Disconnected slots and reserved-but-unjoined hot-join endpoints are
    /// excluded from the aggregation: a frozen slot that will never send a
    /// matching checksum must not force this to `None`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Ensure we've verified sync up to the target before exiting
    /// let target = Frame::new(100);
    /// if let Some(verified) = session.last_verified_frame() {
    ///     if verified >= target {
    ///         // Safe to terminate - verified sync at target frame
    ///     }
    /// }
    /// ```
    #[must_use]
    pub fn last_verified_frame(&self) -> Option<Frame> {
        let mut min_verified: Option<Frame> = None;
        let mut any_connected = false;
        for remote in self.player_reg.remotes.values() {
            if !self.remote_is_connected(remote) {
                continue;
            }
            any_connected = true;
            match remote.last_verified_frame {
                // An unverified connected peer makes the whole-mesh verified
                // frame undefined.
                None => return None,
                Some(frame) => {
                    min_verified = Some(match min_verified {
                        Some(current) => std::cmp::min(current, frame),
                        None => frame,
                    });
                },
            }
        }
        if any_connected {
            min_verified
        } else {
            None
        }
    }

    /// Returns `true` if a remote endpoint counts toward mesh-wide verification.
    ///
    /// A remote is connected for this purpose when it is not a reserved
    /// (hot-join) endpoint and at least one handle it owns is still marked
    /// connected in `local_connect_status`. This matches the notion of
    /// "connected" used by the session's disconnect/sync machinery.
    fn remote_is_connected(&self, remote: &UdpProtocol<T>) -> bool {
        #[cfg(feature = "hot-join")]
        if self.hot_join.endpoint_is_reserved(remote) {
            return false;
        }
        remote.handles().iter().any(|handle| {
            self.local_connect_status
                .get(handle.as_usize())
                .is_none_or(|status| !status.disconnected)
        })
    }

    /// Returns detailed synchronization information for all remote peers.
    ///
    /// This method provides a comprehensive view of the synchronization status
    /// with all connected remote players. Useful for debugging and diagnostics.
    ///
    /// # Returns
    ///
    /// A vector of tuples containing `(PlayerHandle, SyncHealth)` for each
    /// remote player. The vector is empty if there are no remote players.
    #[must_use]
    pub fn all_sync_health(&self) -> Vec<(PlayerHandle, SyncHealth)> {
        self.player_reg
            .remote_player_handles()
            .into_iter()
            .filter_map(|handle| self.sync_health(handle).map(|health| (handle, health)))
            .collect()
    }

    /// Apply graceful-drop event emission to every non-spectator handle in
    /// `handles` belonging to the endpoint at `addr`: freeze each handle's
    /// input queue and enqueue a [`FortressEvent::PeerDropped`] for it.
    ///
    /// Returns `Err` on the first [`SyncLayer::freeze_player`] failure (an
    /// internal-invariant violation, since handles in an endpoint are
    /// validated at session creation). On error, any handles successfully
    /// frozen up to that point remain frozen and their `PeerDropped` events
    /// remain enqueued — this keeps back-compat with the legacy event stream,
    /// but callers should surface the error so applications know the
    /// graceful-drop contract was partially broken.
    ///
    /// The address-level [`FortressEvent::Disconnected`] is **not** emitted by
    /// this helper; the caller is responsible for emitting it. This lets
    /// callers always emit `Disconnected` for legacy back-compat even when
    /// graceful-drop emission errors.
    ///
    /// Spectator handles in `handles` are skipped (they have no input queue
    /// to freeze and never receive `PeerDropped`).
    ///
    /// `freeze_frames` maps each dropped handle to its **agreed freeze frame**
    /// `F` — the global minimum across all peers of that slot's received frame.
    /// The caller must capture these frames *before* `disconnect_player_at_frames`
    /// overwrites `local_connect_status[handle].last_frame`. Each frame is passed
    /// through to [`SyncLayer::freeze_player`], which rolls the dropped slot's
    /// `last_confirmed_input` back to the value at `F`. Because every survivor
    /// computes the same global-min `F`, all survivors freeze the slot at the
    /// identical value — closing (for the common case) the under-loss desync where
    /// survivors otherwise freeze at their own (differing) last-received value. The
    /// frozen value is kept converged as `F` is mined down by the re-roll in
    /// `disconnect_player_at_frames`; a staggered-detection discard-before-convergence
    /// residual remains (see `CHANGELOG.md` / the N0 design notes). A handle
    /// missing from the map (or mapped to [`Frame::NULL`]) freezes without rolling
    /// back (fail-safe — the `freeze_at` rollback on [`InputQueue`] is a no-op).
    ///
    /// [`InputQueue`]: crate::__internal::InputQueue
    fn emit_peer_dropped_for_endpoint(
        &mut self,
        addr: &T::Address,
        handles: &[PlayerHandle],
        freeze_frames: &BTreeMap<PlayerHandle, Frame>,
    ) -> Result<(), FortressError> {
        for &handle in handles {
            if !handle.is_valid_player_for(self.num_players) {
                // Spectator handle (or otherwise out-of-range): no input queue
                // to freeze and no PeerDropped event for this handle.
                continue;
            }
            let freeze_frame = freeze_frames.get(&handle).copied().unwrap_or(Frame::NULL);
            self.sync_layer.freeze_player(handle, freeze_frame)?;
            self.event_queue.push_back(FortressEvent::PeerDropped {
                handle,
                addr: addr.clone(),
            });
        }
        Ok(())
    }

    /// Computes the per-handle **agreed freeze frame** `F` for each handle being
    /// dropped, used to roll each dropped slot's frozen value back to a value
    /// every survivor shares.
    ///
    /// For each handle the frame is
    /// `last_frame_overrides.get(handle).unwrap_or(local_connect_status[handle].last_frame)`
    /// — the same global-minimum frame `remote_disconnect_snapshot` mins into the
    /// session-wide `earliest_last_frame`. This MUST be read before
    /// `disconnect_player_at_frames` overwrites
    /// `local_connect_status[handle].last_frame`.
    fn agreed_freeze_frames(
        &self,
        handles: &[PlayerHandle],
        last_frame_overrides: Option<&BTreeMap<PlayerHandle, Frame>>,
    ) -> BTreeMap<PlayerHandle, Frame> {
        let mut frames = BTreeMap::new();
        for &handle in handles {
            if !handle.is_valid_player_for(self.num_players) {
                continue;
            }
            let local_last_frame = self
                .local_connect_status
                .get(handle.as_usize())
                .map_or(Frame::NULL, |status| status.last_frame);
            let freeze_frame = last_frame_overrides
                .and_then(|overrides| overrides.get(&handle).copied())
                .unwrap_or(local_last_frame);
            frames.insert(handle, freeze_frame);
        }
        frames
    }

    fn remote_disconnect_snapshot(
        &self,
        player_handle: PlayerHandle,
        last_frame_overrides: Option<&BTreeMap<PlayerHandle, Frame>>,
    ) -> Result<(T::Address, Vec<PlayerHandle>, Frame), FortressError> {
        let addr = match self.player_reg.handles.get(&player_handle) {
            Some(PlayerType::Remote(addr)) => addr.clone(),
            Some(PlayerType::Local) => {
                return Err(InvalidRequestKind::DisconnectLocalPlayer {
                    handle: player_handle,
                }
                .into());
            },
            Some(PlayerType::Spectator(_)) | None => {
                return Err(InvalidRequestKind::DisconnectInvalidHandle {
                    handle: player_handle,
                }
                .into());
            },
        };
        let endpoint =
            self.player_reg
                .remotes
                .get(&addr)
                .ok_or(FortressError::InternalErrorStructured {
                    kind: InternalErrorKind::EndpointNotFoundForRemote { player_handle },
                })?;
        let mut handles: Vec<PlayerHandle> = endpoint.handles().iter().copied().collect();
        if handles.is_empty() {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::InternalError,
                "Remote endpoint at {:?} had no handles during disconnect; using requested handle {}",
                addr,
                player_handle
            );
            handles.push(player_handle);
        }

        let mut earliest_last_frame = Frame::new(i32::MAX);
        for &handle in &handles {
            let status = self.local_connect_status.get(handle.as_usize()).ok_or(
                FortressError::InternalErrorStructured {
                    kind: InternalErrorKind::DisconnectStatusNotFound {
                        player_handle: handle,
                    },
                },
            )?;
            let last_frame = last_frame_overrides
                .and_then(|overrides| overrides.get(&handle).copied())
                .unwrap_or(status.last_frame);
            earliest_last_frame = std::cmp::min(earliest_last_frame, last_frame);
        }

        if earliest_last_frame.as_i32() == i32::MAX {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::InternalError,
                "Remote endpoint at {:?} had no valid handle statuses during disconnect; using NULL frame",
                addr
            );
            earliest_last_frame = Frame::NULL;
        }

        Ok((addr, handles, earliest_last_frame))
    }

    fn validate_graceful_drop_handles(
        &self,
        handles: &[PlayerHandle],
    ) -> Result<(), FortressError> {
        for &handle in handles {
            if !handle.is_valid_player_for(self.num_players) {
                continue;
            }
            self.sync_layer.validate_freeze_player(handle)?;
        }
        Ok(())
    }

    /// Force the session into a non-advancing state after an internal error
    /// prevented a disconnect observation from being applied.
    ///
    /// When the network layer reports a remote peer drop (either via a direct
    /// `Event::Disconnected` from the endpoint or via cross-peer propagation in
    /// [`Self::update_player_disconnects`]) and the bookkeeping needed to apply
    /// it fails partway, leaving the session in `SessionState::Running` would
    /// allow `advance_frame()` to continue producing inputs as if every peer
    /// were still healthy. That is the worst possible outcome: the simulation
    /// silently desyncs without surfacing the disconnect to the caller.
    ///
    /// Fail-closed semantics: we transition to `SessionState::Synchronizing`,
    /// which causes subsequent `advance_frame()` calls to return
    /// `NotSynchronized` until the caller takes action. The transition is
    /// idempotent and never re-enters `Running` automatically — only
    /// `check_initial_sync` can do that, and only after every remote endpoint
    /// has reported a fresh synchronization.
    ///
    /// Callers are responsible for emitting their own `report_violation!` with
    /// the contextual details that triggered the fail-closed; this helper only
    /// mutates state so it can be called from any error branch without forcing
    /// each call site to construct a synthetic `FortressError`.
    fn enter_fail_closed_disconnect_state(&mut self) {
        if self.state != SessionState::Synchronizing {
            self.state = SessionState::Synchronizing;
        }
    }

    fn disconnect_player_with_policy(
        &mut self,
        player_handle: PlayerHandle,
        last_frame_overrides: Option<&BTreeMap<PlayerHandle, Frame>>,
        behavior: DisconnectBehavior,
        event_policy: DisconnectEventPolicy,
        graceful_failure_policy: GracefulDropFailurePolicy,
    ) -> Result<(), FortressError> {
        let (addr, handles, earliest_last_frame) =
            self.remote_disconnect_snapshot(player_handle, last_frame_overrides)?;

        let mut graceful_drop_error = None;
        if behavior == DisconnectBehavior::ContinueWithout
            && event_policy == DisconnectEventPolicy::Emit
        {
            // Capture the per-handle agreed freeze frame BEFORE
            // `disconnect_player_at_frames` overwrites
            // `local_connect_status[handle].last_frame`. The agreed frame is the
            // same global-min value `update_player_disconnects` passes in via
            // `last_frame_overrides` (falling back to the locally received
            // `last_frame`). Threading these into the freeze makes every survivor
            // roll the dropped slot back to the identical value at `F`.
            let freeze_frames = self.agreed_freeze_frames(&handles, last_frame_overrides);
            match self.validate_graceful_drop_handles(&handles) {
                Ok(()) => {
                    if let Err(e) =
                        self.emit_peer_dropped_for_endpoint(&addr, &handles, &freeze_frames)
                    {
                        graceful_drop_error = Some(e);
                    }
                },
                Err(e) => graceful_drop_error = Some(e),
            }
        }

        if graceful_drop_error.is_some()
            && graceful_failure_policy == GracefulDropFailurePolicy::Abort
        {
            if let Some(e) = graceful_drop_error {
                return Err(e);
            }
        }

        self.disconnect_player_at_frames(player_handle, earliest_last_frame, last_frame_overrides);

        // Hot-join: a *cleanly* gracefully-dropped slot is documented to be
        // re-joinable (see `SessionBuilder::with_hot_join`). The steps above left
        // the slot in a dropped state — queue frozen, status disconnected, and its
        // endpoint `Disconnected` (a terminal protocol state with no reconnect
        // edge). Re-arm the endpoint and re-reserve its handle(s) so the slot
        // returns to the exact shape a build-time `add_reserved_player` slot has,
        // and the existing reserved-slot serve machinery handles a returning
        // joiner with no new netcode. Strictly gated to the clean graceful path so
        // a `Halt` disconnect, a suppressed legacy `disconnect_player`, a failed
        // graceful drop, or a session that does not serve hot-joins is untouched.
        #[cfg(feature = "hot-join")]
        if behavior == DisconnectBehavior::ContinueWithout
            && event_policy == DisconnectEventPolicy::Emit
            && graceful_drop_error.is_none()
            && self.hot_join.accept_hot_join
        {
            self.rearm_dropped_slot_for_rejoin(&addr, &handles);
        }

        if behavior == DisconnectBehavior::Halt || graceful_drop_error.is_some() {
            self.state = SessionState::Synchronizing;
        }

        if event_policy == DisconnectEventPolicy::Emit {
            self.event_queue
                .push_back(FortressEvent::Disconnected { addr });
        }

        if let Some(e) = graceful_drop_error {
            return Err(e);
        }

        Ok(())
    }

    /// Re-arms a cleanly gracefully-dropped slot so a returning peer can hot-join
    /// it, restoring the exact state a build-time reserved slot has.
    ///
    /// The caller ([`disconnect_player_with_policy`](Self::disconnect_player_with_policy))
    /// has already frozen the input queue and marked the connection status
    /// disconnected — the two invariants a reserved slot shares with a dropped one.
    /// This restores the remaining two: a re-synchronizable endpoint (the protocol
    /// has no reconnect edge, so the endpoint is rebuilt via
    /// [`UdpProtocol::rearm_for_rejoin`]) and `reserved_slots` membership for every
    /// non-spectator handle the endpoint owns. After this the slot is
    /// indistinguishable from one created by
    /// [`add_reserved_player`](crate::SessionBuilder::add_reserved_player), so the
    /// existing host serve path ([`poll_hot_join_host`](Self::poll_hot_join_host))
    /// fills it on rejoin with no special-casing.
    ///
    /// If the endpoint cannot be re-armed (a should-never-happen reconstruction
    /// failure) the slot is left dropped and **not** re-reserved: a reserved handle
    /// backed by a dead endpoint could never be served yet would wrongly suppress
    /// its disconnect/sync-timeout events.
    #[cfg(feature = "hot-join")]
    fn rearm_dropped_slot_for_rejoin(&mut self, addr: &T::Address, handles: &[PlayerHandle]) {
        let Some(endpoint) = self.player_reg.remotes.get_mut(addr) else {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::InternalError,
                "Cannot re-arm dropped slot for rejoin: no remote endpoint at {:?}",
                addr
            );
            return;
        };
        if let Err(e) = endpoint.rearm_for_rejoin() {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::InternalError,
                "Failed to re-arm dropped slot endpoint at {:?} for rejoin; leaving slot dropped (not re-reserved): {}",
                addr,
                e
            );
            return;
        }
        // Re-reserve every non-spectator handle the endpoint owns. `endpoint_is_reserved`
        // requires ALL of an endpoint's handles to be reserved, so a multi-handle
        // (couch co-op) endpoint must have each handle re-added or it would not be
        // treated as reserved. Spectator/out-of-range handles carry no reserved-slot
        // semantics and are skipped.
        for &handle in handles {
            if handle.is_valid_player_for(self.num_players) {
                self.hot_join.reserved_slots.insert(handle);
            }
        }
    }

    fn disconnect_player_at_frame(&mut self, player_handle: PlayerHandle, last_frame: Frame) {
        self.disconnect_player_at_frames(player_handle, last_frame, None);
    }

    fn disconnect_player_at_frames(
        &mut self,
        player_handle: PlayerHandle,
        earliest_last_frame: Frame,
        last_frame_overrides: Option<&BTreeMap<PlayerHandle, Frame>>,
    ) {
        // disconnect the remote player
        let Some(player_type) = self.player_reg.handles.get(&player_handle) else {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::InternalError,
                "Invalid player handle {} in disconnect_player_at_frame - ignoring",
                player_handle
            );
            return;
        };

        match player_type {
            PlayerType::Remote(addr) => {
                let Some(endpoint) = self.player_reg.remotes.get_mut(addr) else {
                    report_violation!(
                        ViolationSeverity::Warning,
                        ViolationKind::InternalError,
                        "No endpoint found for remote address in disconnect_player_at_frame - ignoring"
                    );
                    return;
                };

                // Collect the affected handles first so the per-handle status
                // mutation below does not hold a borrow of `endpoint` across the
                // `self.sync_layer` borrow needed for the frozen-value re-roll.
                let affected_handles: Vec<PlayerHandle> =
                    endpoint.handles().iter().copied().collect();
                endpoint.disconnect();

                // mark the affected players as disconnected
                for &handle in &affected_handles {
                    // Set/lower the agreed frame, then capture the resulting
                    // `status.last_frame` and END the `&mut status` borrow before
                    // touching `self.sync_layer`.
                    let converged_last_frame = {
                        let Some(status) = self.local_connect_status.get_mut(handle.as_usize())
                        else {
                            report_violation!(
                                ViolationSeverity::Warning,
                                ViolationKind::InternalError,
                                "Invalid player handle {} when marking as disconnected - skipping",
                                handle
                            );
                            continue;
                        };
                        let existing_last_frame = status.last_frame;
                        let handle_last_frame = last_frame_overrides
                            .and_then(|overrides| overrides.get(&handle).copied())
                            .unwrap_or(existing_last_frame);
                        if status.disconnected {
                            status.last_frame = std::cmp::min(status.last_frame, handle_last_frame);
                        } else {
                            status.last_frame = handle_last_frame;
                        }
                        status.disconnected = true;
                        status.last_frame
                    };

                    // Convergence chokepoint. `status.last_frame` has just been
                    // set (initial drop) or mined DOWN (re-adjust) to the
                    // global-min agreed freeze frame `F`. Re-roll the dropped
                    // slot's frozen value to the dropped peer's input confirmed
                    // at this `F`. This runs on EVERY path — Emit first-freeze,
                    // Suppress re-adjust, and `remove_player` — so a survivor
                    // that initially froze "high" (at its own locally-received
                    // frame on the direct-detection path) is corrected down to
                    // the same value every survivor shares once `F` converges.
                    // `set_frozen_value_at` is a no-op if the queue is not frozen
                    // and fail-safe if `F` is NULL or evicted, so calling it
                    // unconditionally here is safe.
                    self.sync_layer
                        .set_frozen_value_at(handle, converged_last_frame);
                }

                if self.sync_layer.current_frame() > earliest_last_frame {
                    // remember to adjust simulation to account for the fact that the player disconnected a few frames ago,
                    // resimulating with correct disconnect flags (to account for user having some AI kick in).
                    let disconnect_frame = safe_frame_add!(
                        earliest_last_frame,
                        1,
                        "P2PSession::disconnect_player_at_frame"
                    );
                    self.disconnect_frame = if self.disconnect_frame.is_null() {
                        disconnect_frame
                    } else {
                        std::cmp::min(self.disconnect_frame, disconnect_frame)
                    };

                    // F11: the freeze re-roll above (`set_frozen_value_at`)
                    // retroactively changed the dropped slot's confirmed input at
                    // every frame >= disconnect_frame (= F+1), so any checksum we
                    // already stored — and may already have sent — for those frames
                    // is now stale. Drop the stale local entries so a survivor's
                    // correct post-convergence checksum is not compared against our
                    // pre-convergence value, which would fire a false-positive
                    // DesyncDetected even though both peers' converged state is
                    // byte-identical. The entry at F itself is unchanged (the slot
                    // is frozen AT F, equal to its real input at F) and is
                    // intentionally kept (F < F+1). Uses the LOCAL `disconnect_frame`
                    // (= earliest_last_frame + 1) for THIS drop, not the
                    // min-across-drops `self.disconnect_frame`, to avoid
                    // over-removing on a later, lower-framed drop. With
                    // DesyncDetection::Off the map is empty, so this is a no-op.
                    self.local_checksum_history
                        .retain(|&frame, _| frame < disconnect_frame);
                }
            },
            PlayerType::Spectator(addr) => {
                let Some(endpoint) = self.player_reg.spectators.get_mut(addr) else {
                    report_violation!(
                        ViolationSeverity::Warning,
                        ViolationKind::InternalError,
                        "No endpoint found for spectator address in disconnect_player_at_frame - ignoring"
                    );
                    return;
                };
                endpoint.disconnect();
            },
            PlayerType::Local => (),
        }

        // check if all remotes are synchronized now
        self.check_initial_sync();
    }

    /// Change the session state to [`SessionState::Running`] if all UDP endpoints are synchronized.
    fn check_initial_sync(&mut self) {
        // if we are not synchronizing, we don't need to do anything
        if self.state != SessionState::Synchronizing {
            return;
        }

        // if any endpoint is not synchronized, we continue synchronizing
        for endpoint in self.player_reg.remotes.values_mut() {
            // A reserved-but-not-yet-joined endpoint must not block the host's
            // transition to Running: the slot is frozen/disconnected and the
            // host runs solo until a peer hot-joins. Skip such endpoints here.
            #[cfg(feature = "hot-join")]
            if self.hot_join.endpoint_is_reserved(endpoint) {
                continue;
            }
            if !endpoint.is_synchronized() {
                return;
            }
        }
        for endpoint in self.player_reg.spectators.values_mut() {
            if !endpoint.is_synchronized() {
                return;
            }
        }

        // everyone is synchronized, so we can change state and accept input
        self.state = SessionState::Running;
    }

    /// Roll back to `min_confirmed` frame and resimulate the game with most up-to-date input data.
    ///
    /// # Errors
    /// Returns `FortressError::InvalidFrame` if the frame to load is invalid.
    fn adjust_gamestate(
        &mut self,
        first_incorrect: Frame,
        min_confirmed: Frame,
        requests: &mut RequestVec<T>,
    ) -> Result<(), FortressError> {
        let current_frame = self.sync_layer.current_frame();
        // Floor of the live prediction window; computed up front because the
        // sparse earlier-checkpoint search below is bounded by it.
        let window_floor = safe_frame_sub!(
            current_frame,
            self.max_prediction as i32,
            "adjust_gamestate"
        )
        .max(Frame::new(0));

        // determine the frame to load
        let frame_to_load = if self.save_mode == SaveMode::Sparse {
            // With sparse saving we normally roll back to the sole tracked saved
            // state. But when a gossip-lowered disconnect frame drives
            // `first_incorrect` BELOW `last_saved_frame`, that single saved state
            // is CONTAMINATED: it embeds the dropped peer's pre-convergence
            // (predicted/high-frame) inputs for the `[first_incorrect,
            // last_saved_frame)` window, so re-simulating forward from it would
            // keep this survivor's confirmed history out of sync with peers that
            // re-simulated those frames with the dropped slot frozen at the
            // agreed frame `F` (audit finding F7). Loading the contaminated state
            // cannot fix its own embedded history. The circular buffer, however,
            // usually still holds an EARLIER sparse checkpoint taken at or below
            // `first_incorrect` (sparse saves at confirmed frames roughly every
            // `max_prediction` frames, and the previous one commonly lands at the
            // freeze frame, where the dropped slot's value is identical on every
            // survivor). Prefer that earlier checkpoint so re-simulation restarts
            // from an uncontaminated base and converges. If no such state is
            // buffered, fall back to `last_saved_frame` (the gap is then a genuine
            // unrecoverable residual, still flagged below).
            let last_saved = self.sync_layer.last_saved_frame();
            if last_saved > first_incorrect {
                let earlier = self
                    .sync_layer
                    .newest_saved_frame_in_range(window_floor, first_incorrect);
                if earlier.is_null() {
                    last_saved
                } else {
                    earlier
                }
            } else {
                last_saved
            }
        } else {
            // otherwise, we will rollback to first_incorrect
            first_incorrect
        };

        // we should always load a frame that is before or exactly the first incorrect frame.
        // This check runs on the UN-clamped `frame_to_load` (sparse mode's resolved load frame,
        // or `first_incorrect` otherwise) so it still catches the genuine sparse-mode case where
        // no buffered saved state at or below the first incorrect frame remains, leaving the gap
        // unrecoverable.
        if frame_to_load > first_incorrect {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::FrameSync,
                "frame_to_load {} > first_incorrect {} - this indicates a bug",
                frame_to_load,
                first_incorrect
            );
        }

        // Clamp the rollback target UP to the prediction-window floor `current_frame -
        // max_prediction` (floored at 0). In `EveryFrame` save mode this equals the oldest saved
        // frame, because saves are contiguous, so `load_frame` is guaranteed to find a valid saved
        // state at this frame. `load_frame` rejects anything older than this floor with
        // `OutsidePredictionWindow`, so without this clamp a disconnect frame gossiped DOWN below
        // the live window (a survivor that advanced far ahead while a peer's drop converged to a
        // much older global-min `F`) would make `load_frame` error every frame and, because
        // `disconnect_frame` is cleared only AFTER a successful `adjust_gamestate`, permanently
        // stall `advance_frame`. Clamping keeps the session live and re-simulates as many in-window
        // frames as possible with the corrected disconnect flags; frames below the floor are
        // unrecoverable (the documented discard-before-convergence residual). For any in-window
        // target this `.max(..)` is a no-op, so the normal prediction-miss rollback path is
        // unchanged. (`window_floor` is computed once at the top of this function.)
        let load_target = frame_to_load.max(window_floor);
        if load_target > first_incorrect {
            // Legitimate: the rollback target the disconnect convergence asked for fell below the
            // live prediction window, so we re-simulate from the window floor instead. This is the
            // gossip-lowered-disconnect-frame-outside-window residual, NOT the genuine sparse bug
            // the Error above guards — log at Warning so it is not mistaken for a real defect.
            // Fires once per gossip-lowering event, not per frame: `disconnect_frame` is cleared
            // after this adjust (and `reset_prediction` runs below), so we are not re-entered for
            // the same lowered target.
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::FrameSync,
                "rollback target {} is below the prediction-window floor {} (current {}, \
                 max_prediction {}); re-simulating from the floor. Frames below it cannot be \
                 corrected for the gossip-lowered disconnect frame.",
                frame_to_load,
                window_floor,
                current_frame,
                self.max_prediction
            );
        }

        // If load_target >= current_frame, there's nothing to roll back to.
        // This can happen when a misprediction is detected at the current frame
        // (e.g., at frame 0 when we haven't advanced yet). In this case, we just
        // need to reset predictions - the next frame advance will use the correct inputs.
        if load_target >= current_frame {
            debug!(
                "Skipping rollback: load_target {} >= current_frame {} - resetting predictions only",
                load_target, current_frame
            );
            self.sync_layer.reset_prediction();
            return Ok(());
        }

        let count = current_frame - load_target;

        if let Ok(depth) = usize::try_from(count) {
            if let Some(telemetry) = &self.telemetry {
                telemetry.on_rollback(depth, load_target);
            }
        }

        // request to load that frame
        debug!(
            "Pushing request to load frame {} (current frame {})",
            load_target, current_frame
        );
        requests.push(self.sync_layer.load_frame(load_target)?);

        // we are now at the desired frame
        let actual_frame = self.sync_layer.current_frame();
        if actual_frame != load_target {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::FrameSync,
                "current frame mismatch after load: expected={}, actual={}",
                load_target,
                actual_frame
            );
        }
        self.sync_layer.reset_prediction();

        // step forward to the previous current state, but with updated inputs
        for i in 0..count {
            let inputs = match self
                .sync_layer
                .synchronized_inputs(&self.local_connect_status)
            {
                Some(inputs) => inputs,
                None => {
                    report_violation!(
                        ViolationSeverity::Critical,
                        ViolationKind::InternalError,
                        "Failed to get synchronized inputs during resimulation at frame {}",
                        self.sync_layer.current_frame()
                    );
                    return Err(FortressError::InternalErrorStructured {
                        kind: InternalErrorKind::SynchronizedInputsFailed {
                            frame: self.sync_layer.current_frame(),
                        },
                    });
                },
            };

            // decide whether to request a state save
            if self.save_mode == SaveMode::Sparse {
                // with sparse saving, we only save exactly the min_confirmed frame
                if self.sync_layer.current_frame() == min_confirmed {
                    requests.push(self.sync_layer.save_current_state());
                }
            } else {
                // without sparse saving, we save every state except the very first (just loaded that))
                if i > 0 {
                    requests.push(self.sync_layer.save_current_state());
                }
            }

            // advance the frame
            self.sync_layer.advance_frame();
            requests.push(FortressRequest::AdvanceFrame { inputs });
        }
        // after all this, we should have arrived at the same frame where we started
        let final_frame = self.sync_layer.current_frame();
        if final_frame != current_frame {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::FrameSync,
                "current frame mismatch after resimulation: expected={}, actual={}",
                current_frame,
                final_frame
            );
        }
        Ok(())
    }

    /// For each spectator, send all confirmed input up until the minimum confirmed frame.
    fn send_confirmed_inputs_to_spectators(
        &mut self,
        confirmed_frame: Frame,
    ) -> Result<(), FortressError> {
        if self.num_spectators() == 0 {
            return Ok(());
        }

        while self.next_spectator_frame <= confirmed_frame {
            let mut inputs = self
                .sync_layer
                .confirmed_inputs(self.next_spectator_frame, &self.local_connect_status)?;

            // Validate input count matches num_players - this should always hold due to construction
            // but we recover gracefully rather than panic if somehow violated
            if inputs.len() != self.num_players {
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::InternalError,
                    "confirmed_inputs returned {} inputs but expected {} - skipping spectator send for frame {}",
                    inputs.len(),
                    self.num_players,
                    self.next_spectator_frame
                );
                self.next_spectator_frame = self.next_spectator_frame.try_add(1)?;
                continue;
            }

            let mut input_map = BTreeMap::new();
            for (handle, input) in inputs.iter_mut().enumerate() {
                // Validate frame consistency - should be NULL or match expected frame
                if input.frame != Frame::NULL && input.frame != self.next_spectator_frame {
                    report_violation!(
                        ViolationSeverity::Warning,
                        ViolationKind::FrameSync,
                        "Input frame {} doesn't match expected spectator frame {} for handle {}",
                        input.frame,
                        self.next_spectator_frame,
                        handle
                    );
                }
                input_map.insert(PlayerHandle::new(handle), *input);
            }

            // send it to all spectators
            for endpoint in self.player_reg.spectators.values_mut() {
                if endpoint.is_running() {
                    endpoint.send_input(&input_map, &self.local_connect_status);
                }
            }

            // onto the next frame
            self.next_spectator_frame = self.next_spectator_frame.try_add(1)?;
        }

        Ok(())
    }

    /// Check if players are registered as disconnected for earlier frames on other remote players in comparison to our local assumption.
    /// Disconnect players that are disconnected for other players and update the frame they disconnected
    fn update_player_disconnects(&mut self) {
        let mut propagated_by_addr: BTreeMap<T::Address, BTreeMap<PlayerHandle, Frame>> =
            BTreeMap::new();
        let mut representative_by_addr: BTreeMap<T::Address, PlayerHandle> = BTreeMap::new();
        let mut newly_disconnected_by_addr: BTreeMap<T::Address, bool> = BTreeMap::new();

        for handle_idx in 0..self.num_players {
            let handle = PlayerHandle::new(handle_idx);
            let mut queue_connected = true;
            let mut queue_min_confirmed = Frame::new(i32::MAX);

            // check all player connection status for every remote player
            for endpoint in self.player_reg.remotes.values() {
                if !endpoint.is_running() {
                    continue;
                }
                let con_status = endpoint.peer_connect_status(handle);
                let connected = !con_status.disconnected;
                let min_confirmed = con_status.last_frame;

                queue_connected = queue_connected && connected;
                queue_min_confirmed = std::cmp::min(queue_min_confirmed, min_confirmed);
            }

            // check our local info for that player
            let Some(status) = self.local_connect_status.get(handle_idx) else {
                report_violation!(
                    ViolationSeverity::Warning,
                    ViolationKind::InternalError,
                    "Invalid player index {} when checking connection status - skipping",
                    handle_idx
                );
                continue;
            };
            let local_connected = !status.disconnected;
            let local_min_confirmed = status.last_frame;

            if local_connected {
                queue_min_confirmed = std::cmp::min(queue_min_confirmed, local_min_confirmed);
            }

            if !queue_connected {
                // check to see if the remote disconnect is further back than we have disconnected that player.
                // If so, we need to re-adjust. This can happen when we e.g. detect our own disconnect at frame n
                // and later receive a disconnect notification for frame n-1.
                if local_connected || local_min_confirmed > queue_min_confirmed {
                    let Some(PlayerType::Remote(addr)) = self.player_reg.handles.get(&handle)
                    else {
                        continue;
                    };
                    propagated_by_addr
                        .entry(addr.clone())
                        .or_default()
                        .insert(handle, queue_min_confirmed);
                    representative_by_addr.entry(addr.clone()).or_insert(handle);
                    newly_disconnected_by_addr
                        .entry(addr.clone())
                        .and_modify(|newly_disconnected| {
                            *newly_disconnected = *newly_disconnected || local_connected;
                        })
                        .or_insert(local_connected);
                }
            }
        }

        for (addr, overrides) in &propagated_by_addr {
            let Some(&representative) = representative_by_addr.get(addr) else {
                // The two maps are populated together earlier in this function,
                // so reaching this branch indicates an internal invariant
                // violation — the safe response is the same as any other
                // "disconnect observation we could not apply": fail closed
                // rather than silently dropping the disconnect.
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::InternalError,
                    "Missing representative handle for propagated disconnect at {:?}",
                    addr
                );
                self.enter_fail_closed_disconnect_state();
                continue;
            };
            let event_policy = if newly_disconnected_by_addr
                .get(addr)
                .copied()
                .unwrap_or(false)
            {
                DisconnectEventPolicy::Emit
            } else {
                DisconnectEventPolicy::Suppress
            };
            if let Err(e) = self.disconnect_player_with_policy(
                representative,
                Some(overrides),
                self.disconnect_behavior,
                event_policy,
                GracefulDropFailurePolicy::DisconnectAndHalt,
            ) {
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::InternalError,
                    "Failed to apply propagated disconnect for endpoint at {:?}: {}",
                    addr,
                    e
                );
                // Fail closed: disconnect knowledge has been observed but could
                // not be fully applied. Returning to Synchronizing prevents
                // subsequent `advance_frame()` calls from continuing the
                // simulation as if every peer were still healthy.
                self.enter_fail_closed_disconnect_state();
            }
        }
    }

    /// Gather average frame advantage from each remote player endpoint and return the maximum.
    fn max_frame_advantage(&self) -> i32 {
        let mut interval = i32::MIN;
        for endpoint in self.player_reg.remotes.values() {
            for &handle in endpoint.handles().iter() {
                let Some(status) = self.local_connect_status.get(handle.as_usize()) else {
                    report_violation!(
                        ViolationSeverity::Warning,
                        ViolationKind::InternalError,
                        "Invalid player handle {} when checking frame advantage - skipping",
                        handle
                    );
                    continue;
                };
                if !status.disconnected {
                    interval = std::cmp::max(interval, endpoint.average_frame_advantage());
                }
            }
        }

        // if no remote player is connected
        if interval == i32::MIN {
            interval = 0;
        }

        interval
    }

    fn check_wait_recommendation(&mut self) {
        self.frames_ahead = self.max_frame_advantage();
        if self.sync_layer.current_frame() > self.next_recommended_sleep
            && self.frames_ahead >= MIN_RECOMMENDATION as i32
        {
            self.next_recommended_sleep = self.sync_layer.current_frame() + RECOMMENDATION_INTERVAL;
            // frames_ahead is guaranteed to be >= MIN_RECOMMENDATION (positive), so try_into should succeed.
            // Using unwrap_or(0) as defense-in-depth; 0 effectively skips the recommendation.
            let skip_frames = self.frames_ahead.try_into().unwrap_or(0);
            self.event_queue
                .push_back(FortressEvent::WaitRecommendation { skip_frames });
        }
    }

    fn check_last_saved_state(
        &mut self,
        last_saved: Frame,
        confirmed_frame: Frame,
        requests: &mut RequestVec<T>,
    ) -> Result<(), FortressError> {
        // in sparse saving mode, we need to make sure not to lose the last saved frame
        if self.sync_layer.current_frame() - last_saved >= self.max_prediction as i32 {
            // check if the current frame is confirmed, otherwise we need to roll back
            if confirmed_frame >= self.sync_layer.current_frame() {
                // the current frame is confirmed, save it
                requests.push(self.sync_layer.save_current_state());
            } else {
                // roll back to the last saved state, resimulate and save on the way
                self.adjust_gamestate(last_saved, confirmed_frame, requests)?;
            }

            // after all this, we should have saved the confirmed state
            let expected_saved_frame =
                std::cmp::min(confirmed_frame, self.sync_layer.current_frame());
            if confirmed_frame != Frame::NULL
                && self.sync_layer.last_saved_frame() != expected_saved_frame
            {
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::StateManagement,
                    "last_saved_frame mismatch after check_last_saved_state: expected={}, actual={}",
                    expected_saved_frame,
                    self.sync_layer.last_saved_frame()
                );
            }
        }
        Ok(())
    }

    fn resolve_disconnect_handle(
        &self,
        player_handles: &[PlayerHandle],
        addr: &T::Address,
    ) -> Option<PlayerHandle> {
        // Prefer explicit remote handles from the event payload to preserve
        // rollback semantics for remote disconnects.
        player_handles
            .iter()
            .copied()
            .rfind(|handle| {
                matches!(
                    self.player_reg.handles.get(handle),
                    Some(PlayerType::Remote(_))
                )
            })
            // Then accept any non-local handle carried by the event payload.
            .or_else(|| {
                player_handles.iter().rev().copied().find(|handle| {
                    matches!(
                        self.player_reg.handles.get(handle),
                        Some(PlayerType::Remote(_) | PlayerType::Spectator(_))
                    )
                })
            })
            // Finally, fall back to registry lookup by endpoint address.
            // If both a remote and spectator share the same address, avoid
            // guessing the target when payload handles are missing.
            .or_else(|| {
                let mut first_remote = None;
                let mut first_spectator = None;

                for handle in self.player_reg.handles_by_address_iter(addr) {
                    match self.player_reg.handles.get(&handle) {
                        Some(PlayerType::Remote(_)) if first_remote.is_none() => {
                            first_remote = Some(handle);
                        },
                        Some(PlayerType::Spectator(_)) if first_spectator.is_none() => {
                            first_spectator = Some(handle);
                        },
                        _ => {},
                    }
                }

                match (first_remote, first_spectator) {
                    (Some(remote), None) => Some(remote),
                    (None, Some(spectator)) => Some(spectator),
                    (Some(_), Some(_)) | (None, None) => None,
                }
            })
    }

    /// Handle events received from the UDP endpoints. Most events are being forwarded to the user for notification, but some require action.
    fn handle_event(
        &mut self,
        event: Event<T>,
        player_handles: Arc<[PlayerHandle]>,
        addr: T::Address,
    ) {
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
            // check if all remotes are synced, then forward to user
            Event::Synchronized => {
                self.check_initial_sync();
                self.event_queue
                    .push_back(FortressEvent::Synchronized { addr });
            },
            // disconnect the player, then forward to user
            Event::Disconnected => {
                // Hot-join: a reserved-but-unfilled slot's endpoint dropping is
                // EXPECTED (the joiner is absent or abandoned the join). The slot
                // is already frozen/disconnected, so treat this as a no-op: abort
                // any in-flight serve for those handles (host resumes solo) and
                // do NOT halt the session or emit a user-facing disconnect. This
                // is what keeps an abandoned join from killing the host (the slot
                // stays reserved so a peer can still retry).
                #[cfg(feature = "hot-join")]
                if !player_handles.is_empty()
                    && player_handles
                        .iter()
                        .all(|handle| self.hot_join.reserved_slots.contains(handle))
                {
                    // Abort any in-flight serve for these handles via the single
                    // teardown path, which also clears the endpoint's accumulated
                    // `pending_output`. Without that clear, a host that resumes
                    // serving this slot would see a full `pending_output` on every
                    // `send_input` and raise an internal disconnect on every frame
                    // (the same storm the Phase-4 timeout guards against). See
                    // `abort_hot_join_serve`.
                    for handle in player_handles.iter() {
                        self.abort_hot_join_serve(*handle);
                    }
                    return;
                }
                let Some(target_handle) = self.resolve_disconnect_handle(&player_handles, &addr)
                else {
                    report_violation!(
                        ViolationSeverity::Warning,
                        ViolationKind::NetworkProtocol,
                        "Received disconnect event for endpoint {:?} with no resolvable handles (handles={:?})",
                        addr,
                        player_handles
                    );
                    self.event_queue
                        .push_back(FortressEvent::Disconnected { addr });
                    return;
                };
                let event_count_before_disconnect = self.event_queue.len();
                // `resolve_disconnect_handle` only returns Remote or Spectator handles, never
                // Local; the registry-missing case is also filtered upstream. The catch-all
                // arm is therefore an invariant guard rather than a regular branch.
                match self.player_reg.handles.get(&target_handle) {
                    Some(PlayerType::Remote(_)) => {
                        if let Err(e) = self.disconnect_player_with_policy(
                            target_handle,
                            None,
                            self.disconnect_behavior,
                            DisconnectEventPolicy::Emit,
                            GracefulDropFailurePolicy::DisconnectAndHalt,
                        ) {
                            report_violation!(
                                ViolationSeverity::Error,
                                ViolationKind::InternalError,
                                "Failed to apply remote disconnect event for endpoint at {:?} (target={}, handles={:?}): {}",
                                addr,
                                target_handle,
                                player_handles,
                                e
                            );
                            if self.event_queue.len() == event_count_before_disconnect {
                                self.event_queue
                                    .push_back(FortressEvent::Disconnected { addr });
                            }
                            // Fail closed: a remote endpoint reported a
                            // disconnect that we could not fully apply.
                            // Leaving the session Running would let the
                            // simulation advance with stale connection state.
                            self.enter_fail_closed_disconnect_state();
                        }
                    },
                    Some(PlayerType::Spectator(_)) => {
                        self.disconnect_player_at_frame(target_handle, Frame::NULL);
                        self.event_queue
                            .push_back(FortressEvent::Disconnected { addr });
                    },
                    Some(PlayerType::Local) | None => {
                        report_violation!(
                            ViolationSeverity::Error,
                            ViolationKind::InternalError,
                            "resolve_disconnect_handle returned non-disconnectable target={} for endpoint {:?} (handles={:?}); registry state may be corrupt",
                            target_handle,
                            addr,
                            player_handles
                        );
                        if self.event_queue.len() == event_count_before_disconnect {
                            self.event_queue
                                .push_back(FortressEvent::Disconnected { addr });
                        }
                        // Registry state is reported as potentially corrupt;
                        // fail closed rather than continue advancing on a
                        // disconnect observation we could not apply.
                        self.enter_fail_closed_disconnect_state();
                    },
                }
            },
            // forward sync timeout to user
            Event::SyncTimeout { elapsed_ms } => {
                // Suppress sync-timeout for a reserved-but-unjoined slot: it is
                // expected to stay Synchronizing until a peer hot-joins, so a
                // timeout there is not actionable. (The protocol never
                // auto-disconnects a Synchronizing endpoint regardless; this only
                // avoids a misleading user-facing event.)
                #[cfg(feature = "hot-join")]
                if !player_handles.is_empty()
                    && player_handles
                        .iter()
                        .all(|handle| self.hot_join.reserved_slots.contains(handle))
                {
                    return;
                }
                self.event_queue
                    .push_back(FortressEvent::SyncTimeout { addr, elapsed_ms });
            },
            // add the input and all associated information
            Event::Input { input, player, .. } => {
                // input only comes from remote players, not spectators
                if !player.is_valid_player_for(self.num_players) {
                    report_violation!(
                        ViolationSeverity::Error,
                        ViolationKind::NetworkProtocol,
                        "Received input from invalid player handle {} (max={})",
                        player,
                        self.num_players - 1
                    );
                    return;
                }
                let Some(status) = self.local_connect_status.get_mut(player.as_usize()) else {
                    report_violation!(
                        ViolationSeverity::Warning,
                        ViolationKind::InternalError,
                        "Invalid player handle {} when handling input event - ignoring",
                        player
                    );
                    return;
                };
                if !status.disconnected {
                    // check if the input comes in the correct sequence
                    let current_remote_frame = status.last_frame;
                    let expected_frame = safe_frame_add!(
                        current_remote_frame,
                        1,
                        "P2PSession::handle_event input sequence"
                    );
                    if current_remote_frame != Frame::NULL && expected_frame != input.frame {
                        report_violation!(
                            ViolationSeverity::Error,
                            ViolationKind::NetworkProtocol,
                            "Input sequence violation: expected frame {}, got {}",
                            expected_frame,
                            input.frame
                        );
                        return;
                    }
                    // update our info
                    status.last_frame = input.frame;
                    // add the remote input
                    self.sync_layer.add_remote_input(player, input);
                }
            },
        }

        // check event queue size and discard oldest events if too big
        while self.event_queue.len() > self.max_event_queue_size {
            self.event_queue.pop_front();
        }
    }

    fn compare_local_checksums_against_peers(&mut self) {
        match self.desync_detection {
            DesyncDetection::On { .. } => {
                for remote in self.player_reg.remotes.values_mut() {
                    let mut checked_frames = Vec::new();

                    for (&remote_frame, &remote_checksum) in &remote.pending_checksums {
                        if remote_frame >= self.sync_layer.last_confirmed_frame() {
                            // we're still waiting for inputs for this frame
                            continue;
                        }
                        if let Some(&local_checksum) =
                            self.local_checksum_history.get(&remote_frame)
                        {
                            if local_checksum != remote_checksum {
                                self.event_queue.push_back(FortressEvent::DesyncDetected {
                                    frame: remote_frame,
                                    local_checksum,
                                    remote_checksum,
                                    addr: remote.peer_addr(),
                                });
                            } else {
                                // Checksums match - update this peer's verified frame.
                                // Per-peer (not session-global) so verification against one
                                // remote never leaks into another remote's sync verdict.
                                remote.last_verified_frame = match remote.last_verified_frame {
                                    Some(current) if current >= remote_frame => Some(current),
                                    _ => Some(remote_frame),
                                };
                            }
                            checked_frames.push(remote_frame);
                        }
                    }

                    for frame in checked_frames {
                        remote.pending_checksums.remove_entry(&frame);
                    }
                }
            },
            DesyncDetection::Off => (),
        }
    }

    fn check_checksum_send_interval(&mut self) {
        match self.desync_detection {
            DesyncDetection::On { interval } => {
                let frame_to_send = if self.last_sent_checksum_frame.is_null() {
                    Frame::new(interval as i32)
                } else {
                    self.last_sent_checksum_frame + interval as i32
                };

                if frame_to_send <= self.sync_layer.last_confirmed_frame()
                    && frame_to_send <= self.sync_layer.last_saved_frame()
                {
                    // M1: A disconnect-induced rollback is armed for this
                    // `advance_frame` (set in `update_player_disconnects`, cleared
                    // only after `adjust_gamestate`, which runs AFTER this
                    // function). Its re-simulation has not happened yet, so the
                    // saved cell at `frame_to_send >= disconnect_frame` still holds
                    // the dropped peer's PREDICTED input. Sending or storing a
                    // checksum from that stale cell would (a) re-pollute
                    // `local_checksum_history` right after the F11 retain cleared it
                    // — causing a false DesyncDetected when a survivor's correct
                    // post-convergence checksum arrives — and (b) gossip a stale
                    // checksum to peers. Defer: `last_sent_checksum_frame` is not
                    // advanced (the advance happens only inside the
                    // `if let Some(checksum)` block below), so the same frame is
                    // re-attempted next `advance_frame` once the cell has been
                    // re-simulated with the dropped slot frozen at the agreed frame.
                    // Bounded: `disconnect_frame` is non-null only on a frame
                    // actively processing a drop (F8's clamp makes `adjust_gamestate`
                    // infallible, so the clear at the end of `advance_frame` is
                    // reliable), so this never permanently stalls checksum sending.
                    if !self.disconnect_frame.is_null() && frame_to_send >= self.disconnect_frame {
                        return;
                    }

                    let Some(cell) = self.sync_layer.saved_state_by_frame(frame_to_send) else {
                        // This shouldn't happen if frame is within confirmed and saved range
                        report_violation!(
                            ViolationSeverity::Warning,
                            ViolationKind::StateManagement,
                            "Cell not found for frame {} in check_checksum_send_interval (confirmed={}, saved={}) - skipping checksum",
                            frame_to_send,
                            self.sync_layer.last_confirmed_frame(),
                            self.sync_layer.last_saved_frame()
                        );
                        return;
                    };

                    if let Some(checksum) = cell.checksum() {
                        for remote in self.player_reg.remotes.values_mut() {
                            remote.send_checksum_report(frame_to_send, checksum);
                        }
                        self.last_sent_checksum_frame = frame_to_send;
                        // collect locally for later comparison
                        self.local_checksum_history.insert(frame_to_send, checksum);
                    }

                    let max_history = self.protocol_config.max_checksum_history;
                    if self.local_checksum_history.len() > max_history {
                        let oldest_frame_to_keep =
                            frame_to_send - (max_history as i32 - 1) * interval as i32;
                        self.local_checksum_history
                            .retain(|&frame, _| frame >= oldest_frame_to_keep);
                    }
                }
            },
            DesyncDetection::Off => (),
        }
    }
}

impl<T: Config> fmt::Debug for P2PSession<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("P2PSession")
            .field("num_players", &self.num_players)
            .field("max_prediction", &self.max_prediction)
            .field("state", &self.state)
            .field("disconnect_frame", &self.disconnect_frame)
            .field("disconnect_behavior", &self.disconnect_behavior)
            .field("current_frame", &self.sync_layer.current_frame())
            .field("frames_ahead", &self.frames_ahead)
            .field("desync_detection", &self.desync_detection)
            .field("is_recording", &self.recording.is_some())
            .field("has_telemetry", &self.telemetry.is_some())
            .finish_non_exhaustive()
    }
}

impl<T: Config> InvariantChecker for P2PSession<T> {
    /// Checks that the session's invariants are satisfied.
    ///
    /// This method verifies:
    /// 1. No desync has been detected with any remote peer
    /// 2. The session state is consistent
    ///
    /// # Returns
    ///
    /// * `Ok(())` - All invariants hold
    /// * `Err(InvariantViolation)` - A desync or other invariant violation was detected
    ///
    /// # Example
    ///
    /// ```ignore
    /// use fortress_rollback::telemetry::InvariantChecker;
    ///
    /// // In tests, assert invariants after operations
    /// session.advance_frame()?;
    /// assert!(session.check_invariants().is_ok(), "Session invariants violated");
    ///
    /// // Or use the macro for debug-only checks
    /// fortress_rollback::debug_check_invariants!(session);
    /// ```
    fn check_invariants(&self) -> Result<(), InvariantViolation> {
        // Check for any desync with remote peers
        for (handle, health) in self.all_sync_health() {
            if let SyncHealth::DesyncDetected {
                frame,
                local_checksum,
                remote_checksum,
            } = health
            {
                return Err(InvariantViolation::new(
                    "P2PSession",
                    "checksum mismatch detected with remote peer",
                )
                .with_checksum_mismatch(
                    frame,
                    handle,
                    local_checksum,
                    remote_checksum,
                ));
            }
        }

        Ok(())
    }
}

impl<T: Config> Session<T> for P2PSession<T> {
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
    use crate::network::messages::Message;
    use crate::sessions::builder::SessionBuilder;
    use crate::{Config, NonBlockingSocket};
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

    // Helper function to create a local-only P2P session for testing (no network)
    fn create_local_only_session() -> P2PSession<TestConfig> {
        SessionBuilder::new()
            .with_num_players(1)
            .unwrap()
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .expect("Failed to add player")
            .start_p2p_session(DummySocket)
            .expect("Failed to create session")
    }

    // Helper function to create a 2-player P2P session with one remote
    fn create_two_player_session() -> P2PSession<TestConfig> {
        SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .expect("Failed to add local player")
            .add_player(PlayerType::Remote(test_addr(8080)), PlayerHandle::new(1))
            .expect("Failed to add remote player")
            .start_p2p_session(DummySocket)
            .expect("Failed to create session")
    }

    fn create_multi_handle_remote_session() -> P2PSession<TestConfig> {
        SessionBuilder::new()
            .with_num_players(3)
            .unwrap()
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .expect("Failed to add local player")
            .add_player(PlayerType::Remote(test_addr(8080)), PlayerHandle::new(1))
            .expect("Failed to add remote player 1")
            .add_player(PlayerType::Remote(test_addr(8080)), PlayerHandle::new(2))
            .expect("Failed to add remote player 2")
            .start_p2p_session(DummySocket)
            .expect("Failed to create session")
    }

    // Helper function to create a 2-player local-only session
    fn create_two_local_players_session() -> P2PSession<TestConfig> {
        SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .expect("Failed to add player 0")
            .add_player(PlayerType::Local, PlayerHandle::new(1))
            .expect("Failed to add player 1")
            .start_p2p_session(DummySocket)
            .expect("Failed to create session")
    }

    // ==========================================
    // Constant Tests
    // ==========================================

    #[test]
    fn recommendation_interval_is_reasonable() {
        // 60 frames at 60fps = 1 second
        assert_eq!(RECOMMENDATION_INTERVAL, Frame::new(60));
    }

    #[test]
    fn min_recommendation_is_reasonable() {
        // At least 2 frames to avoid micro-stuttering, but not more than 10
        // Use const_assert pattern to satisfy clippy about const assertions
        const _: () = assert!(MIN_RECOMMENDATION >= 2);
        const _: () = assert!(MIN_RECOMMENDATION <= 10);
        // Verify at runtime the constant is what we expect
        assert_eq!(MIN_RECOMMENDATION, 3);
    }

    #[test]
    fn default_max_event_queue_size_is_reasonable() {
        // Should be large enough to buffer network events (at least 50)
        // but not so large as to consume excessive memory (at most 1000)
        const _: () = assert!(DEFAULT_MAX_EVENT_QUEUE_SIZE >= 50);
        const _: () = assert!(DEFAULT_MAX_EVENT_QUEUE_SIZE <= 1000);
        // Verify at runtime the constant is what we expect
        assert_eq!(DEFAULT_MAX_EVENT_QUEUE_SIZE, 100);
    }

    // ==========================================
    // P2PSession Constructor and Initial State Tests
    // ==========================================

    #[test]
    fn p2p_session_local_only_starts_running() {
        let session = create_local_only_session();
        // With no remote players, session starts running immediately
        assert_eq!(session.current_state(), SessionState::Running);
    }

    #[test]
    fn p2p_session_with_remote_starts_synchronizing() {
        let session = create_two_player_session();
        // With remote players, session starts in synchronizing state
        assert_eq!(session.current_state(), SessionState::Synchronizing);
    }

    #[test]
    fn p2p_session_initial_frame_is_zero() {
        let session = create_local_only_session();
        assert_eq!(session.current_frame(), Frame::new(0));
    }

    #[test]
    fn p2p_session_initial_confirmed_frame_is_null() {
        let session = create_local_only_session();
        // Initially, confirmed frame is NULL (no frames confirmed yet)
        assert_eq!(session.confirmed_frame(), Frame::NULL);
    }

    #[test]
    fn p2p_session_num_players_returns_correct_count() {
        let session = create_two_local_players_session();
        assert_eq!(session.num_players(), 2);
    }

    #[test]
    fn p2p_session_num_spectators_initially_zero() {
        let session = create_local_only_session();
        assert_eq!(session.num_spectators(), 0);
    }

    #[test]
    fn p2p_session_local_player_handles_returns_correct_handles() {
        let session = create_two_local_players_session();
        let handles = session.local_player_handles();
        assert_eq!(handles.len(), 2);
        assert!(handles.contains(&PlayerHandle::new(0)));
        assert!(handles.contains(&PlayerHandle::new(1)));
    }

    // ==========================================
    // local_player_handle Tests
    // ==========================================

    // Helper function to create a remote-only P2P session (no local players)
    fn create_remote_only_session() -> P2PSession<TestConfig> {
        SessionBuilder::new()
            .with_num_players(2)
            .unwrap()
            .add_player(PlayerType::Remote(test_addr(8080)), PlayerHandle::new(0))
            .expect("Failed to add remote player 0")
            .add_player(PlayerType::Remote(test_addr(8081)), PlayerHandle::new(1))
            .expect("Failed to add remote player 1")
            .start_p2p_session(DummySocket)
            .expect("Failed to create session")
    }

    #[test]
    fn p2p_session_local_player_handle_returns_some_with_one_local() {
        // Arrange - session with one local player
        let session = create_local_only_session();

        // Act
        let result = session.local_player_handle();

        // Assert
        assert!(result.is_some());
        assert_eq!(result.unwrap(), PlayerHandle::new(0));
    }

    #[test]
    fn p2p_session_local_player_handle_returns_first_with_multiple_locals() {
        // Arrange - session with two local players
        let session = create_two_local_players_session();

        // Act
        let result = session.local_player_handle();

        // Assert - should return the first local player handle
        let all_handles = session.local_player_handles();
        assert_eq!(result, all_handles.first().copied());
    }

    #[test]
    fn p2p_session_local_player_handle_returns_none_with_no_locals() {
        // Arrange - session with only remote players
        let session = create_remote_only_session();

        // Act
        let result = session.local_player_handle();

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn p2p_session_remote_player_handles_with_remotes() {
        let session = create_two_player_session();
        let handles = session.remote_player_handles();
        assert_eq!(handles.len(), 1);
        assert!(handles.contains(&PlayerHandle::new(1)));
    }

    #[test]
    fn p2p_session_spectator_handles_initially_empty() {
        let session = create_two_player_session();
        assert!(session.spectator_handles().is_empty());
    }

    #[test]
    fn p2p_session_max_prediction_returns_configured_value() {
        let session = SessionBuilder::<TestConfig>::new()
            .with_num_players(1)
            .unwrap()
            .with_max_prediction_window(4)
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .expect("Failed to add player")
            .start_p2p_session(DummySocket)
            .expect("Failed to create session");
        assert_eq!(session.max_prediction(), 4);
    }

    #[test]
    fn p2p_session_frames_ahead_initially_zero() {
        let session = create_local_only_session();
        assert_eq!(session.frames_ahead(), 0);
    }

    #[test]
    fn p2p_session_desync_detection_returns_configured_value() {
        let session = SessionBuilder::<TestConfig>::new()
            .with_num_players(1)
            .unwrap()
            .with_desync_detection_mode(DesyncDetection::Off)
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .expect("Failed to add player")
            .start_p2p_session(DummySocket)
            .expect("Failed to create session");
        assert_eq!(session.desync_detection(), DesyncDetection::Off);
    }

    #[test]
    fn p2p_session_violation_observer_none_by_default() {
        let session = create_local_only_session();
        assert!(session.violation_observer().is_none());
    }

    // ==========================================
    // add_local_input Tests
    // ==========================================

    #[test]
    fn add_local_input_for_valid_handle_succeeds() {
        let mut session = create_local_only_session();
        session.add_local_input(PlayerHandle::new(0), 42u8).unwrap();
    }

    #[test]
    fn add_local_input_for_remote_handle_fails() {
        let mut session = create_two_player_session();
        // Handle 1 is remote
        let result = session.add_local_input(PlayerHandle::new(1), 42u8);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::NotLocalPlayer { .. }
            })
        ));
    }

    #[test]
    fn add_local_input_for_invalid_handle_fails() {
        let mut session = create_local_only_session();
        // Handle 99 doesn't exist
        let result = session.add_local_input(PlayerHandle::new(99), 42u8);
        assert!(result.is_err());
    }

    #[test]
    fn add_local_input_multiple_times_overwrites() {
        let mut session = create_local_only_session();
        session
            .add_local_input(PlayerHandle::new(0), 10u8)
            .expect("First input failed");
        session
            .add_local_input(PlayerHandle::new(0), 20u8)
            .expect("Second input failed");
        // Should succeed without error - second input overwrites first
    }

    // ==========================================
    // advance_frame Tests
    // ==========================================

    #[test]
    fn advance_frame_without_input_fails() {
        let mut session = create_local_only_session();
        let result = session.advance_frame();
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::MissingLocalInput
            })
        ));
    }

    #[test]
    fn advance_frame_when_not_synchronized_fails() {
        let mut session = create_two_player_session();
        // Session is in Synchronizing state
        assert_eq!(session.current_state(), SessionState::Synchronizing);
        session
            .add_local_input(PlayerHandle::new(0), 42u8)
            .expect("Input failed");
        let result = session.advance_frame();
        assert!(result.is_err());
        match result {
            Err(FortressError::NotSynchronized) => {},
            _ => panic!("Expected NotSynchronized error"),
        }
    }

    #[test]
    fn advance_frame_with_input_succeeds() {
        let mut session = create_local_only_session();
        session
            .add_local_input(PlayerHandle::new(0), 42u8)
            .expect("Input failed");
        let requests = session.advance_frame().expect("Advance failed");
        // Should have at least one request (likely SaveGameState and AdvanceFrame)
        assert!(!requests.is_empty());
    }

    #[test]
    fn advance_frame_increments_current_frame() {
        let mut session = create_local_only_session();
        assert_eq!(session.current_frame(), Frame::new(0));
        session
            .add_local_input(PlayerHandle::new(0), 42u8)
            .expect("Input failed");
        let _requests = session.advance_frame().expect("Advance failed");
        assert_eq!(session.current_frame(), Frame::new(1));
    }

    #[test]
    fn advance_frame_clears_local_inputs() {
        let mut session = create_local_only_session();
        session
            .add_local_input(PlayerHandle::new(0), 42u8)
            .expect("Input failed");
        let _requests = session.advance_frame().expect("Advance failed");
        // Now trying to advance again without input should fail
        let result = session.advance_frame();
        assert!(result.is_err());
    }

    #[test]
    fn advance_frame_multiple_local_players_requires_all_inputs() {
        let mut session = create_two_local_players_session();
        // Add input for player 0 only
        session
            .add_local_input(PlayerHandle::new(0), 42u8)
            .expect("Input failed");
        let result = session.advance_frame();
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::MissingLocalInput
            })
        ));
    }

    #[test]
    fn advance_frame_multiple_local_players_succeeds_with_all_inputs() {
        let mut session = create_two_local_players_session();
        session
            .add_local_input(PlayerHandle::new(0), 1u8)
            .expect("Input 0 failed");
        session
            .add_local_input(PlayerHandle::new(1), 2u8)
            .expect("Input 1 failed");
        let requests = session.advance_frame().expect("Advance failed");
        assert!(!requests.is_empty());
    }

    // ==========================================
    // poll_remote_clients Tests
    // ==========================================

    #[test]
    fn poll_remote_clients_does_not_panic() {
        let mut session = create_two_player_session();
        // Should not panic even with no messages
        session.poll_remote_clients();
    }

    #[test]
    fn poll_remote_clients_multiple_times() {
        let mut session = create_local_only_session();
        for _ in 0..10 {
            session.poll_remote_clients();
        }
        // Should complete without issues
    }

    // ==========================================
    // disconnect_player Tests
    // ==========================================

    #[test]
    fn disconnect_player_local_fails() {
        let mut session = create_local_only_session();
        let result = session.disconnect_player(PlayerHandle::new(0));
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::DisconnectLocalPlayer { .. }
            })
        ));
    }

    #[test]
    fn disconnect_player_invalid_handle_fails() {
        let mut session = create_local_only_session();
        let result = session.disconnect_player(PlayerHandle::new(99));
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::DisconnectInvalidHandle { .. }
            })
        ));
    }

    #[test]
    fn disconnect_player_remote_succeeds() {
        let mut session = create_two_player_session();
        // Disconnect remote player (handle 1)
        session.disconnect_player(PlayerHandle::new(1)).unwrap();
    }

    #[test]
    fn disconnect_player_already_disconnected_fails() {
        let mut session = create_two_player_session();
        session
            .disconnect_player(PlayerHandle::new(1))
            .expect("First disconnect failed");
        let result = session.disconnect_player(PlayerHandle::new(1));
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::AlreadyDisconnected { .. }
            })
        ));
    }

    #[test]
    fn disconnect_player_multi_handle_uses_earliest_last_frame_for_rollback() {
        let mut session = create_multi_handle_remote_session();
        session.state = SessionState::Running;
        session.local_connect_status[1].last_frame = Frame::new(5);
        session.local_connect_status[2].last_frame = Frame::new(2);
        for _ in 0..10 {
            session.sync_layer.advance_frame();
        }

        session
            .disconnect_player(PlayerHandle::new(1))
            .expect("multi-handle remote disconnect should succeed");

        assert_eq!(
            session.disconnect_frame,
            Frame::new(3),
            "rollback must start at earliest affected last_frame + 1"
        );
        assert!(session.local_connect_status[1].disconnected);
        assert!(session.local_connect_status[2].disconnected);
        assert_eq!(session.local_connect_status[1].last_frame, Frame::new(5));
        assert_eq!(session.local_connect_status[2].last_frame, Frame::new(2));
        assert_eq!(session.current_state(), SessionState::Synchronizing);
    }

    #[test]
    fn spectator_disconnect_event_disconnects_spectator_endpoint_data_driven() {
        #[derive(Debug)]
        struct Scenario {
            name: &'static str,
            handles: Vec<PlayerHandle>,
        }

        let scenarios = [
            Scenario {
                name: "spectator_handle_only",
                handles: vec![PlayerHandle::new(1)],
            },
            Scenario {
                name: "spectator_then_unknown",
                handles: vec![PlayerHandle::new(1), PlayerHandle::new(99)],
            },
            Scenario {
                name: "unknown_then_spectator",
                handles: vec![PlayerHandle::new(99), PlayerHandle::new(1)],
            },
            Scenario {
                name: "empty_payload_falls_back_to_address",
                handles: vec![],
            },
        ];

        for scenario in scenarios {
            let spectator_addr = test_addr(9090);
            let mut session = SessionBuilder::<TestConfig>::new()
                .with_num_players(1)
                .unwrap()
                .add_player(PlayerType::Local, PlayerHandle::new(0))
                .expect("Failed to add local player")
                .add_player(PlayerType::Spectator(spectator_addr), PlayerHandle::new(1))
                .expect("Failed to add spectator")
                .start_p2p_session(DummySocket)
                .expect("Failed to create session");

            let endpoint_before = session
                .player_reg
                .spectators
                .get(&spectator_addr)
                .expect("spectator endpoint should exist before disconnect");
            let before_is_synchronized = endpoint_before.is_synchronized();
            let before_is_running = endpoint_before.is_running();
            assert!(
                !before_is_synchronized,
                "scenario {}: spectator endpoint unexpectedly synchronized before disconnect; is_synchronized={}, is_running={}",
                scenario.name,
                before_is_synchronized,
                before_is_running,
            );

            let handles: Arc<[PlayerHandle]> = Arc::from(scenario.handles.clone());
            session.handle_event(Event::Disconnected, handles, spectator_addr);

            let endpoint_after = session
                .player_reg
                .spectators
                .get(&spectator_addr)
                .expect("spectator endpoint should still exist after disconnect event");
            let after_is_synchronized = endpoint_after.is_synchronized();
            let after_is_running = endpoint_after.is_running();
            assert!(
                after_is_synchronized,
                "scenario {}: spectator disconnect must move endpoint to a non-blocking state; is_synchronized={}, is_running={}",
                scenario.name,
                after_is_synchronized,
                after_is_running,
            );
            assert!(
                !after_is_running,
                "scenario {}: spectator endpoint should no longer be running after disconnect; is_synchronized={}, is_running={}",
                scenario.name,
                after_is_synchronized,
                after_is_running,
            );

            let events: Vec<_> = session.events().collect();
            assert_eq!(
                events
                    .iter()
                    .filter(|event| matches!(event, FortressEvent::Disconnected { .. }))
                    .count(),
                1,
                "scenario {}: spectator disconnect must emit exactly one Disconnected event",
                scenario.name,
            );
        }
    }

    #[test]
    fn disconnect_event_empty_payload_remote_only_falls_back_to_address() {
        let remote_addr = test_addr(9091);
        let mut session = SessionBuilder::<TestConfig>::new()
            .with_num_players(2)
            .unwrap()
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .expect("Failed to add local player")
            .add_player(PlayerType::Remote(remote_addr), PlayerHandle::new(1))
            .expect("Failed to add remote player")
            .start_p2p_session(DummySocket)
            .expect("Failed to create session");

        let endpoint_before = session
            .player_reg
            .remotes
            .get(&remote_addr)
            .expect("remote endpoint should exist before disconnect");
        assert!(
            !endpoint_before.is_synchronized(),
            "remote endpoint unexpectedly synchronized before disconnect"
        );

        let empty_handles: Arc<[PlayerHandle]> = Arc::from(Vec::new());
        session.handle_event(Event::Disconnected, empty_handles, remote_addr);

        let endpoint_after = session
            .player_reg
            .remotes
            .get(&remote_addr)
            .expect("remote endpoint should still exist after disconnect");
        assert!(
            endpoint_after.is_synchronized(),
            "remote-only address fallback should disconnect the remote endpoint"
        );
        assert!(
            session.local_connect_status[1].disconnected,
            "remote-only address fallback should mark the remote connect status disconnected"
        );

        let events: Vec<_> = session.events().collect();
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, FortressEvent::Disconnected { .. }))
                .count(),
            1,
            "remote-only address fallback must emit exactly one Disconnected event"
        );
    }

    #[test]
    fn disconnect_event_empty_payload_mixed_address_does_not_guess_target() {
        let shared_addr = test_addr(9092);
        let mut session = SessionBuilder::<TestConfig>::new()
            .with_num_players(2)
            .unwrap()
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .expect("Failed to add local player")
            .add_player(PlayerType::Remote(shared_addr), PlayerHandle::new(1))
            .expect("Failed to add remote player")
            .add_player(PlayerType::Spectator(shared_addr), PlayerHandle::new(2))
            .expect("Failed to add spectator")
            .start_p2p_session(DummySocket)
            .expect("Failed to create session");

        let remote_before = session
            .player_reg
            .remotes
            .get(&shared_addr)
            .expect("remote endpoint should exist before disconnect")
            .is_synchronized();
        let spectator_before = session
            .player_reg
            .spectators
            .get(&shared_addr)
            .expect("spectator endpoint should exist before disconnect")
            .is_synchronized();
        assert!(
            !remote_before && !spectator_before,
            "mixed-address endpoints should start unsynchronized"
        );

        let empty_handles: Arc<[PlayerHandle]> = Arc::from(Vec::new());
        session.handle_event(Event::Disconnected, empty_handles, shared_addr);

        let remote_after = session
            .player_reg
            .remotes
            .get(&shared_addr)
            .expect("remote endpoint should still exist after disconnect")
            .is_synchronized();
        let spectator_after = session
            .player_reg
            .spectators
            .get(&shared_addr)
            .expect("spectator endpoint should still exist after disconnect")
            .is_synchronized();

        assert!(
            !remote_after,
            "mixed-address empty-payload disconnect must not guess and disconnect remote"
        );
        assert!(
            !spectator_after,
            "mixed-address empty-payload disconnect must not guess and disconnect spectator"
        );

        let events: Vec<_> = session.events().collect();
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, FortressEvent::Disconnected { .. }))
                .count(),
            1,
            "mixed-address empty-payload disconnect must emit exactly one Disconnected event"
        );
    }

    // ==========================================
    // network_stats Tests
    // ==========================================

    #[test]
    fn network_stats_local_player_fails() {
        let session = create_local_only_session();
        let result = session.network_stats(PlayerHandle::new(0));
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::NotRemotePlayerOrSpectator { .. }
            })
        ));
    }

    #[test]
    fn network_stats_invalid_handle_fails() {
        let session = create_local_only_session();
        let result = session.network_stats(PlayerHandle::new(99));
        assert!(result.is_err());
    }

    #[test]
    fn network_stats_remote_not_synchronized_fails() {
        let session = create_two_player_session();
        // Session is not yet synchronized
        let result = session.network_stats(PlayerHandle::new(1));
        assert!(result.is_err());
        match result {
            Err(FortressError::NotSynchronized) => {},
            _ => panic!("Expected NotSynchronized error"),
        }
    }

    #[test]
    fn network_stats_spectator_uses_spectator_endpoint() {
        let session = SessionBuilder::<TestConfig>::new()
            .with_num_players(1)
            .unwrap()
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .expect("Failed to add local player")
            .add_player(
                PlayerType::Spectator(test_addr(9090)),
                PlayerHandle::new(10),
            )
            .expect("Failed to add spectator")
            .start_p2p_session(DummySocket)
            .expect("Failed to create session");

        let result = session.network_stats(PlayerHandle::new(10));

        assert!(
            matches!(result, Err(FortressError::NotSynchronized)),
            "spectator endpoint should be found and report its protocol state; got {result:?}"
        );
    }

    // ==========================================
    // confirmed_inputs_for_frame Tests
    // ==========================================

    #[test]
    fn confirmed_inputs_for_frame_future_frame_fails() {
        let session = create_local_only_session();
        let result = session.confirmed_inputs_for_frame(Frame::new(100));
        assert!(result.is_err());
        match result {
            Err(FortressError::InvalidFrameStructured { frame, reason }) => {
                assert_eq!(frame, Frame::new(100));
                assert!(matches!(reason, InvalidFrameReason::NotConfirmed { .. }));
            },
            _ => panic!("Expected InvalidFrameStructured error"),
        }
    }

    #[test]
    fn confirmed_inputs_for_frame_returns_correct_inputs() {
        let mut session = create_local_only_session();

        // Advance a few frames
        for i in 0..5 {
            session
                .add_local_input(PlayerHandle::new(0), (i * 10) as u8)
                .expect("Input failed");
            let _requests = session.advance_frame().expect("Advance failed");
        }

        // After advancing, confirmed_frame should be at least 0
        let confirmed = session.confirmed_frame();
        assert!(
            confirmed >= Frame::new(0),
            "Should have some confirmed frames"
        );

        // If we have a confirmed frame, we should be able to get inputs for it
        if confirmed >= Frame::new(0) && !confirmed.is_null() {
            let inputs = session
                .confirmed_inputs_for_frame(Frame::new(0))
                .expect("Should get inputs for confirmed frame");
            assert_eq!(inputs.len(), 1, "Should have 1 player's inputs");
            assert_eq!(inputs[0], 0u8, "Frame 0 should have input value 0");
        }
    }

    #[test]
    fn confirmed_inputs_for_frame_frame_at_confirmed_boundary() {
        let mut session = create_local_only_session();

        // Advance several frames
        for i in 0..10 {
            session
                .add_local_input(PlayerHandle::new(0), i as u8)
                .expect("Input failed");
            let _requests = session.advance_frame().expect("Advance failed");
        }

        let confirmed = session.confirmed_frame();

        // Getting inputs at the confirmed frame should succeed
        if !confirmed.is_null() {
            let result = session.confirmed_inputs_for_frame(confirmed);
            assert!(result.is_ok(), "Should succeed at confirmed frame boundary");
        }

        // Getting inputs one frame past confirmed should fail
        let result = session.confirmed_inputs_for_frame(confirmed + 1);
        assert!(result.is_err(), "Should fail past confirmed frame");
    }

    #[test]
    fn confirmed_inputs_for_frame_null_frame_handling() {
        let session = create_local_only_session();

        // NULL frame (which is -1) - the behavior depends on whether it's treated as
        // "before confirmed" or as an invalid frame. The function checks if frame > confirmed_frame,
        // and since NULL (-1) <= any confirmed frame, it will try to fetch from the input queue.
        // The input queue will then fail because frame -1 doesn't exist.
        let result = session.confirmed_inputs_for_frame(Frame::NULL);
        // This may succeed or fail depending on implementation details
        // The key is that it doesn't panic and handles the edge case
        match result {
            Ok(_) => {
                // If it somehow succeeds, that's fine - just verify behavior
            },
            Err(_) => {
                // Expected - frame -1 is not in the queue
            },
        }
    }

    #[test]
    fn confirmed_inputs_for_frame_discarded_frame_fails() {
        let mut session = create_local_only_session();

        // Advance many frames to trigger frame discard
        // INPUT_QUEUE_LENGTH is 128, so after 128+ frames, early frames are discarded
        for i in 0..150 {
            session
                .add_local_input(PlayerHandle::new(0), (i % 256) as u8)
                .expect("Input failed");
            let _requests = session.advance_frame().expect("Advance failed");
        }

        // Frame 0 should have been discarded by now (we're past INPUT_QUEUE_LENGTH)
        let result = session.confirmed_inputs_for_frame(Frame::new(0));
        // This might succeed or fail depending on how many frames were actually discarded
        // The key point is that it handles the edge case gracefully
        if result.is_err() {
            match result {
                Err(FortressError::InvalidRequestStructured { .. }) => {
                    // Expected - frame was discarded
                },
                Err(FortressError::InvalidFrame { .. })
                | Err(FortressError::InvalidFrameStructured { .. }) => {
                    // Also acceptable if frame is considered not confirmed
                },
                _ => panic!("Unexpected error type"),
            }
        }
        // If it succeeds, that's also fine - the frame might still be in the queue
    }

    /// Data-driven test cases for confirmed_inputs_for_frame edge cases
    #[test]
    fn confirmed_inputs_for_frame_edge_cases() {
        struct TestCase {
            name: &'static str,
            frames_to_advance: i32,
            frame_to_query: i32,
            expect_success: bool,
        }

        let test_cases = [
            TestCase {
                name: "frame 0 after advancing 5 frames",
                frames_to_advance: 5,
                frame_to_query: 0,
                expect_success: true,
            },
            TestCase {
                name: "frame beyond confirmed",
                frames_to_advance: 5,
                frame_to_query: 100,
                expect_success: false,
            },
            TestCase {
                name: "frame at current - 1",
                frames_to_advance: 10,
                frame_to_query: 9,
                // For local-only sessions, confirmed_frame tracks current_frame,
                // so frame 9 is confirmed after advancing 10 frames
                expect_success: true,
            },
            TestCase {
                name: "large negative frame",
                frames_to_advance: 5,
                frame_to_query: -100,
                // Negative frames fail in the input queue (no such frame exists)
                expect_success: false,
            },
        ];

        for tc in &test_cases {
            let mut session = create_local_only_session();

            // Advance the specified number of frames
            for i in 0..tc.frames_to_advance {
                session
                    .add_local_input(PlayerHandle::new(0), (i % 256) as u8)
                    .expect("Input failed");
                let _requests = session.advance_frame().expect("Advance failed");
            }

            let result = session.confirmed_inputs_for_frame(Frame::new(tc.frame_to_query));

            if tc.expect_success {
                assert!(
                    result.is_ok(),
                    "Test '{}' expected success but got {:?}",
                    tc.name,
                    result
                );
            } else {
                assert!(
                    result.is_err(),
                    "Test '{}' expected failure but got {:?}",
                    tc.name,
                    result
                );
            }
        }
    }

    // ==========================================
    // handles_by_address Tests
    // ==========================================

    #[test]
    fn handles_by_address_returns_correct_handles() {
        let session = create_two_player_session();
        let addr = test_addr(8080);
        let handles = session.handles_by_address(&addr);
        assert_eq!(handles.len(), 1);
        assert!(handles.contains(&PlayerHandle::new(1)));
    }

    #[test]
    fn handles_by_address_unknown_returns_empty() {
        let session = create_two_player_session();
        let unknown_addr = test_addr(9999);
        let handles = session.handles_by_address(&unknown_addr);
        assert!(handles.is_empty());
    }

    // ==========================================
    // events Tests
    // ==========================================

    #[test]
    fn events_initially_empty() {
        let mut session = create_local_only_session();
        let events: Vec<_> = session.events().collect();
        assert!(events.is_empty());
    }

    #[test]
    fn events_drains_queue() {
        let mut session = create_local_only_session();
        // First drain
        let _: Vec<_> = session.events().collect();
        // Second drain should also be empty
        let events: Vec<_> = session.events().collect();
        assert!(events.is_empty());
    }

    // ==========================================
    // in_lockstep_mode Tests
    // ==========================================

    #[test]
    fn in_lockstep_mode_false_with_default_prediction() {
        let mut session = create_local_only_session();
        assert!(!session.in_lockstep_mode());
    }

    #[test]
    fn in_lockstep_mode_true_with_zero_prediction() {
        let mut session = SessionBuilder::<TestConfig>::new()
            .with_num_players(1)
            .unwrap()
            .with_max_prediction_window(0)
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .expect("Failed to add player")
            .start_p2p_session(DummySocket)
            .expect("Failed to create session");
        assert!(session.in_lockstep_mode());
    }

    // ==========================================
    // sync_health Tests (with session)
    // ==========================================

    #[test]
    fn sync_health_local_player_returns_none() {
        let session = create_local_only_session();
        // Local player doesn't have sync health
        assert!(session.sync_health(PlayerHandle::new(0)).is_none());
    }

    #[test]
    fn sync_health_invalid_handle_returns_none() {
        let session = create_local_only_session();
        assert!(session.sync_health(PlayerHandle::new(99)).is_none());
    }

    #[test]
    fn sync_health_remote_with_desync_off_returns_pending() {
        let session = SessionBuilder::<TestConfig>::new()
            .with_num_players(2)
            .unwrap()
            .with_desync_detection_mode(DesyncDetection::Off)
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .expect("Failed to add local player")
            .add_player(PlayerType::Remote(test_addr(8080)), PlayerHandle::new(1))
            .expect("Failed to add remote player")
            .start_p2p_session(DummySocket)
            .expect("Failed to create session");
        // With desync detection off, sync_health returns Pending
        match session.sync_health(PlayerHandle::new(1)) {
            Some(SyncHealth::Pending) => {},
            other => panic!("Expected Pending, got {:?}", other),
        }
    }

    #[test]
    fn sync_health_remote_initially_pending() {
        let session = create_two_player_session();
        // Initially, no checksums compared, so should be Pending
        match session.sync_health(PlayerHandle::new(1)) {
            Some(SyncHealth::Pending) => {},
            other => panic!("Expected Pending, got {:?}", other),
        }
    }

    // ==========================================
    // is_synchronized Tests
    // ==========================================

    #[test]
    fn is_synchronized_no_remotes_returns_true() {
        let session = create_local_only_session();
        assert!(session.is_synchronized());
    }

    #[test]
    fn is_synchronized_with_pending_returns_false() {
        let session = create_two_player_session();
        // Remote player is pending, so not synchronized
        assert!(!session.is_synchronized());
    }

    // ==========================================
    // last_verified_frame Tests
    // ==========================================

    #[test]
    fn last_verified_frame_initially_none() {
        let session = create_local_only_session();
        assert!(session.last_verified_frame().is_none());
    }

    #[test]
    fn last_verified_frame_with_remote_initially_none() {
        let session = create_two_player_session();
        assert!(session.last_verified_frame().is_none());
    }

    /// Builds a 3-player session (local + two DISTINCT remote machines B and D)
    /// with desync detection enabled, used by the F12 per-peer verification tests.
    fn create_three_player_desync_session() -> P2PSession<TestConfig> {
        SessionBuilder::new()
            .with_num_players(3)
            .unwrap()
            .with_desync_detection_mode(DesyncDetection::On { interval: 1 })
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .expect("Failed to add local player")
            .add_player(PlayerType::Remote(test_addr(8080)), PlayerHandle::new(1))
            .expect("Failed to add remote player B")
            .add_player(PlayerType::Remote(test_addr(8081)), PlayerHandle::new(2))
            .expect("Failed to add remote player D")
            .start_p2p_session(DummySocket)
            .expect("Failed to create session")
    }

    /// F12 regression: with N>=3, a matching checksum from ONE remote (B) must not
    /// make `sync_health`/`is_synchronized`/`last_verified_frame` report sync for
    /// another remote (D) that never sent a matching checksum. Pre-fix the
    /// session-global `last_verified_frame` flag leaked B's verification into D's
    /// verdict, returning `InSync`/`true` for D.
    #[test]
    fn sync_health_verified_peer_does_not_leak_into_unverified_peer() {
        let mut session = create_three_player_desync_session();
        let handle_b = PlayerHandle::new(1);
        let handle_d = PlayerHandle::new(2);
        let addr_b = test_addr(8080);

        // Advance so a frame-0 checksum can be compared (the comparison skips
        // frames >= last_confirmed_frame, and last_confirmed_frame starts at NULL).
        session.sync_layer.advance_frame();
        session.sync_layer.advance_frame();
        session
            .sync_layer
            .set_last_confirmed_frame(Frame::new(2), session.save_mode);

        // Local checksum at frame 0; only B sends a MATCHING report for it.
        let frame = Frame::new(0);
        let checksum: u128 = 0xABCD_1234;
        session.local_checksum_history.insert(frame, checksum);
        session
            .player_reg
            .remotes
            .get_mut(&addr_b)
            .expect("B endpoint exists")
            .pending_checksums
            .insert(frame, checksum);

        // Run the production comparison: it should verify B (and only B).
        session.compare_local_checksums_against_peers();

        assert_eq!(
            session.sync_health(handle_b),
            Some(SyncHealth::InSync),
            "B sent a matching checksum and must report InSync"
        );
        assert_eq!(
            session.sync_health(handle_d),
            Some(SyncHealth::Pending),
            "D sent no matching checksum and must stay Pending (no cross-peer leak)"
        );
        assert!(
            !session.is_synchronized(),
            "is_synchronized() must be false while D is unverified"
        );
        assert!(
            session.last_verified_frame().is_none(),
            "last_verified_frame() must be None: D (a connected peer) is unverified"
        );
    }

    /// F12 follow-up: a remote that gracefully dropped (its slot marked
    /// disconnected) and never sent a matching checksum must NOT block
    /// `is_synchronized()`. Only currently-connected remotes gate it. Pre-fix,
    /// `is_synchronized()` iterated every `PlayerType::Remote` handle regardless
    /// of connection state, so an unverified-then-dropped peer left it `false`
    /// forever — breaking the `confirmed_frame() >= target && is_synchronized()`
    /// exit gate after any graceful drop. The per-peer `sync_health(handle)`
    /// query keeps reporting each peer's truthful state and is unaffected.
    #[test]
    fn dropped_unverified_remote_does_not_block_is_synchronized() {
        let mut session = create_three_player_desync_session();
        let handle_b = PlayerHandle::new(1); // stays connected, gets verified
        let handle_d = PlayerHandle::new(2); // drops, never verified
        let addr_b = test_addr(8080);

        // Advance so a frame-0 checksum can be compared.
        session.sync_layer.advance_frame();
        session.sync_layer.advance_frame();
        session
            .sync_layer
            .set_last_confirmed_frame(Frame::new(2), session.save_mode);

        // B sends a matching checksum at frame 0; D sends nothing.
        let frame = Frame::new(0);
        let checksum: u128 = 0xABCD_1234;
        session.local_checksum_history.insert(frame, checksum);
        session
            .player_reg
            .remotes
            .get_mut(&addr_b)
            .expect("B endpoint exists")
            .pending_checksums
            .insert(frame, checksum);
        session.compare_local_checksums_against_peers();

        // D gracefully drops: its slot is marked disconnected. It was never
        // verified, so its per-peer state is Pending.
        session.local_connect_status[handle_d.as_usize()].disconnected = true;

        // The dropped, never-verified D must not block synchronization: only
        // the still-connected, verified B counts.
        assert!(
            session.is_synchronized(),
            "is_synchronized() must be true: the only connected remote (B) is verified \
             and the dropped never-verified D is excluded"
        );

        // Per-peer truthful state is preserved: B is InSync, D is not.
        assert_eq!(
            session.sync_health(handle_b),
            Some(SyncHealth::InSync),
            "B (connected, verified) must report InSync"
        );
        assert_ne!(
            session.sync_health(handle_d),
            Some(SyncHealth::InSync),
            "D (dropped, never verified) must NOT report InSync; \
             sync_health stays per-peer truthful and is not connection-filtered"
        );
    }

    /// F11 regression: a graceful drop re-rolls the dropped slot's frozen value to
    /// the agreed freeze frame `F`, retroactively changing the correct checksum at
    /// every confirmed frame `> F`. The local checksum history stored (and possibly
    /// sent) before the drop is now stale for those frames, so it must be
    /// invalidated for frames `>= disconnect_frame` (= F+1). Otherwise a survivor's
    /// correct post-convergence checksum at `G > F` is compared against our stale
    /// pre-convergence entry, firing a false-positive `DesyncDetected`.
    #[test]
    fn graceful_drop_invalidates_stale_local_checksums_above_freeze_frame() {
        let mut session = create_three_player_desync_session();
        let addr_b = test_addr(8080);
        let handle_c = PlayerHandle::new(2); // the peer that drops
        let freeze_frame = Frame::new(5); // agreed freeze frame F
        let g = Frame::new(8); // a confirmed frame G > F we already checksummed

        // The session ran ahead before the drop converged.
        for _ in 0..10 {
            session.sync_layer.advance_frame();
        }

        // We stored (and may have sent) checksums while C was still connected,
        // including entries at and above the eventual freeze frame F.
        for f in 0..10 {
            session
                .local_checksum_history
                .insert(Frame::new(f), 0x1000 + u128::try_from(f).unwrap());
        }
        assert!(
            session.local_checksum_history.contains_key(&g),
            "precondition: a pre-drop checksum exists at G > F"
        );

        // C drops; convergence freezes C at F and arms disconnect_frame = F + 1.
        session.disconnect_player_at_frame(handle_c, freeze_frame);

        // F11: entries strictly above F (>= F+1 = disconnect_frame) are stale and
        // must be dropped; the entry AT F (frozen value == real input at F) and all
        // entries below F are kept.
        assert!(
            !session.local_checksum_history.contains_key(&g),
            "stale checksum at G > F must be invalidated after the freeze re-roll"
        );
        assert!(
            session.local_checksum_history.contains_key(&freeze_frame),
            "checksum at the freeze frame F itself must be kept (unchanged)"
        );
        assert!(
            session.local_checksum_history.contains_key(&Frame::new(4)),
            "checksums below F must be kept"
        );
        assert!(
            session
                .local_checksum_history
                .keys()
                .all(|&f| f <= freeze_frame),
            "no checksum entry may remain at or above disconnect_frame (F+1)"
        );

        // Now a survivor (B) sends its CORRECT post-convergence checksum at G.
        // With the stale local entry gone, the comparison finds no local checksum
        // at G and must NOT emit a DesyncDetected.
        session
            .sync_layer
            .set_last_confirmed_frame(Frame::new(10), session.save_mode);
        session
            .player_reg
            .remotes
            .get_mut(&addr_b)
            .expect("B endpoint exists")
            .pending_checksums
            .insert(g, 0xDEAD_BEEF); // differs from the now-removed stale entry
        session.compare_local_checksums_against_peers();

        assert!(
            !session
                .event_queue
                .iter()
                .any(|e| matches!(e, FortressEvent::DesyncDetected { .. })),
            "no false-positive DesyncDetected after stale checksum invalidation"
        );
    }

    /// M1 regression: on the exact `advance_frame` that processes a graceful
    /// drop, `check_checksum_send_interval` runs BEFORE `adjust_gamestate`'s
    /// disconnect rollback re-simulates the affected cells. Without the M1 skip
    /// guard, it would read the still-stale saved cell at
    /// `frame_to_send >= disconnect_frame` (holding the dropped peer's PREDICTED
    /// input), then BOTH store that stale checksum into `local_checksum_history`
    /// (re-polluting it right after the F11 retain cleared it) AND gossip it to
    /// every remote. The guard must defer (skip) that frame while a disconnect
    /// rollback is armed, advancing neither `last_sent_checksum_frame` nor the
    /// history.
    ///
    /// Observables (jointly prove the `if let Some(checksum)` send/store block
    /// did not run, since the send loop, the `last_sent_checksum_frame` advance,
    /// and the `local_checksum_history.insert` all live inside it):
    /// - `local_checksum_history` gains NO entry at `frame_to_send`.
    /// - `last_sent_checksum_frame` is NOT advanced (stays at its pre-call value),
    ///   so no `ChecksumReport` was queued to any remote for that frame and the
    ///   frame is re-attempted on the next `advance_frame` after re-simulation.
    #[test]
    fn checksum_send_deferred_while_disconnect_rollback_is_armed() {
        let mut session = create_three_player_desync_session();

        // Build a saved-state ring with a checksum in every cell, mirroring how a
        // live session populates cells (save_current_state -> cell.save -> advance).
        // After the loop: last_saved_frame = 8, current_frame = 9.
        for f in 0..=8 {
            let request = session.sync_layer.save_current_state();
            if let FortressRequest::SaveGameState { cell, frame } = request {
                assert_eq!(frame, Frame::new(f));
                // A deliberately STALE checksum: the value the pre-rollback
                // (predicted) cell would carry. The guard must prevent this from
                // being sent/stored for the deferred frame.
                cell.save(frame, Some(0u8), Some(0x5742_4C45_u128));
            }
            session.sync_layer.advance_frame();
        }
        session
            .sync_layer
            .set_last_confirmed_frame(Frame::new(8), session.save_mode);

        // interval = 1, so frame_to_send = last_sent_checksum_frame + 1 = 7.
        let last_sent_before = Frame::new(6);
        session.last_sent_checksum_frame = last_sent_before;
        let frame_to_send = Frame::new(7);

        // A disconnect rollback is armed for this advance_frame: disconnect_frame
        // is set (and NOT yet cleared, since adjust_gamestate runs AFTER
        // check_checksum_send_interval). frame_to_send (7) >= disconnect_frame (6),
        // so the cell at 7 is about to be re-simulated and is currently stale.
        session.disconnect_frame = Frame::new(6);

        // Sanity: the send/store gate (frame_to_send <= confirmed && <= saved) is
        // satisfied, so ONLY the M1 guard can be responsible for the skip.
        assert!(frame_to_send <= session.sync_layer.last_confirmed_frame());
        assert!(frame_to_send <= session.sync_layer.last_saved_frame());
        assert!(
            !session.local_checksum_history.contains_key(&frame_to_send),
            "precondition: history empty at frame_to_send before the call"
        );

        session.check_checksum_send_interval();

        // M1: the frame was deferred — nothing stored, nothing sent, the send
        // cursor did not advance (so the frame is retried next advance_frame).
        assert!(
            !session.local_checksum_history.contains_key(&frame_to_send),
            "no stale checksum may be stored for a frame a pending disconnect \
             rollback will re-simulate (frame_to_send >= disconnect_frame)"
        );
        assert_eq!(
            session.last_sent_checksum_frame, last_sent_before,
            "last_sent_checksum_frame must NOT advance: the frame is deferred (not \
             sent to any remote), so it is re-attempted after re-simulation"
        );
    }

    /// Session-26 desync-harvest lead (NORMAL prediction path) — invariant lock.
    ///
    /// `check_checksum_send_interval` harvests the checksum of the saved cell at
    /// `frame_to_send` via `saved_state_by_frame`, which returns the cell ONLY
    /// when its stored frame EXACTLY matches `frame_to_send`. This test pins that
    /// the harvested/stored/sent checksum is the one belonging to `frame_to_send`
    /// itself — never a different ring slot's value. Combined with the production
    /// invariant that `last_confirmed_frame` never exceeds `first_incorrect` (so
    /// a confirmed frame's cell has been re-saved with confirmed inputs after any
    /// rollback), this is the structural reason a faithful per-frame checksum can
    /// never be harvested from a speculative cell on the normal path.
    ///
    /// Construction writes a DISTINCT, recognizable checksum into the cell at
    /// `frame_to_send` and a DIFFERENT value into the (later) `current_frame`
    /// cell. If the harvest ever read the wrong (later/speculative) cell, the
    /// stored history value would be the later cell's checksum — the assertions
    /// below would catch it.
    #[test]
    fn checksum_harvest_uses_exact_frame_to_send_cell_not_a_later_cell() {
        let mut session = create_three_player_desync_session(); // interval = 1

        // Populate a contiguous ring: frames 0..=5 each get a UNIQUE checksum.
        // After the loop: last_saved_frame = 5, current_frame = 6.
        for f in 0..=5u128 {
            let request = session.sync_layer.save_current_state();
            if let FortressRequest::SaveGameState { cell, frame } = request {
                assert_eq!(frame, Frame::new(f as i32));
                // Unique, frame-derived checksum so a wrong-cell read is visible.
                cell.save(frame, Some(0u8), Some(0xC0DE_0000 + f));
            }
            session.sync_layer.advance_frame();
        }

        // Confirm through frame 3. No disconnect armed (normal path).
        session
            .sync_layer
            .set_last_confirmed_frame(Frame::new(3), session.save_mode);
        session.disconnect_frame = Frame::NULL;

        // interval = 1, last_sent_checksum_frame NULL => frame_to_send = 1.
        let frame_to_send = Frame::new(1);
        assert!(frame_to_send <= session.sync_layer.last_confirmed_frame());
        assert!(frame_to_send <= session.sync_layer.last_saved_frame());

        session.check_checksum_send_interval();

        // The stored checksum must be EXACTLY the cell-at-frame_to_send value,
        // not the current/last-saved (speculative) cell's value.
        let expected = 0xC0DE_0000 + 1u128;
        assert_eq!(
            session.local_checksum_history.get(&frame_to_send).copied(),
            Some(expected),
            "harvest must take the checksum of the cell at frame_to_send, not a later cell"
        );
        assert_eq!(
            session.last_sent_checksum_frame, frame_to_send,
            "harvest of a confirmed, exact-match cell advances the send cursor"
        );
        // Sanity: the later (current_frame) cell carried a DIFFERENT checksum, so
        // the equality above genuinely discriminates wrong-cell reads.
        assert_ne!(expected, 0xC0DE_0000 + 5u128);
    }

    /// Session-26 desync-harvest lead (NORMAL prediction path) — the OTHER half of
    /// the invariant: the harvest is gated on `frame_to_send <= last_confirmed_frame`
    /// (and `<= last_saved_frame`). A cell that is SAVED but NOT yet CONFIRMED must
    /// NEVER be harvested, even though its `saved_state_by_frame` lookup would
    /// succeed (the cell exists and its stored frame matches). This is what stops a
    /// speculative cell above `last_confirmed_frame` — which may still hold
    /// predicted inputs that a pending rollback will overwrite — from being gossiped.
    ///
    /// Construction: a fully-populated ring (frames 0..=5, each with a unique
    /// checksum) so `saved_state_by_frame(1)` WOULD return a cell, but
    /// `last_confirmed_frame` is held at 0. With interval = 1 and
    /// `last_sent_checksum_frame` NULL, `frame_to_send = 1 > last_confirmed_frame`,
    /// so the gate must skip the harvest entirely.
    ///
    /// Non-vacuity (verified by probe): dropping the `frame_to_send <=
    /// last_confirmed_frame` conjunct in `check_checksum_send_interval`
    /// (`src/sessions/p2p_session.rs` ~4145) turns this test RED — the speculative
    /// cell at frame 1 would then be harvested and stored. The cell genuinely
    /// exists (so the skip is attributable to the confirmed-frame gate, not a
    /// `saved_state_by_frame` miss), which is exactly the discriminating property.
    #[test]
    fn checksum_harvest_skips_speculative_cell_above_last_confirmed_frame() {
        let mut session = create_three_player_desync_session(); // interval = 1

        // Populate a contiguous ring: frames 0..=5 each get a UNIQUE checksum.
        // After the loop: last_saved_frame = 5, current_frame = 6.
        for f in 0..=5u128 {
            let request = session.sync_layer.save_current_state();
            if let FortressRequest::SaveGameState { cell, frame } = request {
                assert_eq!(frame, Frame::new(f as i32));
                cell.save(frame, Some(0u8), Some(0xBEEF_0000 + f));
            }
            session.sync_layer.advance_frame();
        }

        // Confirm ONLY through frame 0, so frame 1 is SAVED but NOT confirmed.
        session
            .sync_layer
            .set_last_confirmed_frame(Frame::new(0), session.save_mode);
        session.disconnect_frame = Frame::NULL;

        // interval = 1, last_sent_checksum_frame NULL => frame_to_send = 1.
        let frame_to_send = Frame::new(1);
        // The cell EXISTS and its stored frame matches (an exact-match hit), so a
        // skip here can ONLY be the confirmed-frame gate, not a lookup miss.
        assert!(
            session
                .sync_layer
                .saved_state_by_frame(frame_to_send)
                .is_some(),
            "the speculative cell at frame_to_send must exist (exact-match hit), so \
             the skip is attributable to the <= last_confirmed_frame gate"
        );
        // The gate's precondition: frame_to_send is ABOVE last_confirmed_frame.
        assert!(frame_to_send > session.sync_layer.last_confirmed_frame());
        // ...but within the saved range (so ONLY the confirmed gate can skip it).
        assert!(frame_to_send <= session.sync_layer.last_saved_frame());

        session.check_checksum_send_interval();

        // The speculative cell must NOT be harvested: no history entry, send cursor
        // unmoved. (Removing the `<= last_confirmed_frame` conjunct makes both of
        // these fail — the cell-at-1 checksum 0xBEEF_0001 would be stored.)
        assert!(
            !session.local_checksum_history.contains_key(&frame_to_send),
            "a SAVED-but-UNCONFIRMED speculative cell must not be harvested"
        );
        assert!(
            session.last_sent_checksum_frame.is_null(),
            "send cursor must not advance when frame_to_send exceeds last_confirmed_frame"
        );
    }

    /// Session-26 desync-harvest lead — sparse `SaveMode` coverage for the
    /// harvest's EXACT-MATCH `saved_state_by_frame` behavior.
    ///
    /// In sparse mode only a single checkpoint is kept (at the confirmed frame);
    /// the other ring slots hold stale frames from earlier saves. The harvest must
    /// rely on `saved_state_by_frame` returning a cell ONLY on an EXACT frame
    /// match: when `frame_to_send` has no matching checkpoint, the lookup returns
    /// `None` and the harvest is SKIPPED — it must never read a stale/gap cell that
    /// happens to occupy `frame_to_send`'s ring slot.
    ///
    /// Construction: a sparse session whose ring has `MAX_PREDICTION + 1 = 9`
    /// cells, so frame 10 maps to the SAME ring slot as `frame_to_send = 1`. We
    /// stamp a single checksummed checkpoint at frame 10 (a "stale" wrapped cell,
    /// from frame 1's perspective) and leave the rest of the ring at defaults.
    /// `saved_state_by_frame(1)` then finds a cell whose stored frame is 10, not 1
    /// — an EXACT-MATCH MISS — so it returns `None` and the harvest is skipped.
    /// Crucially the wrapped cell CARRIES a checksum, so a guard-less lookup would
    /// genuinely harvest a wrong value (not a no-op default cell).
    ///
    /// Non-vacuity (verified by probe): replacing the `saved_state_by_frame`
    /// exact-match guard with a raw `Some(cell)` (i.e. dropping the
    /// `cell_frame == frame` check in `src/sync_layer/mod.rs` ~1305) turns this RED
    /// — the harvest would then read the stale wrapped cell at slot 1 and store its
    /// checksum (`0x5A5A_000A`) at frame 1. With the real exact-match guard, no
    /// entry is stored.
    #[test]
    fn checksum_harvest_sparse_exact_match_miss_skips_no_stale_read() {
        const MAX_PREDICTION: usize = 8; // ring size = MAX_PREDICTION + 1 = 9
        let mut session: P2PSession<TestConfig> = SessionBuilder::new()
            .with_num_players(3)
            .expect("num_players")
            .with_desync_detection_mode(DesyncDetection::On { interval: 1 })
            .with_save_mode(SaveMode::Sparse)
            .with_max_prediction_window(MAX_PREDICTION)
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .expect("local player")
            .add_player(PlayerType::Remote(test_addr(8080)), PlayerHandle::new(1))
            .expect("remote player B")
            .add_player(PlayerType::Remote(test_addr(8081)), PlayerHandle::new(2))
            .expect("remote player D")
            .start_p2p_session(DummySocket)
            .expect("session");
        assert_eq!(session.save_mode, SaveMode::Sparse);

        // Advance to frame 10 WITHOUT saving anything in between, then stamp the
        // SINGLE sparse checkpoint at frame 10. Frame 10 maps to ring slot
        // 10 % 9 == 1 — the SAME slot as frame_to_send = 1. So slot 1 ends up
        // holding a cell whose stored frame is 10 (with a checksum), while every
        // other slot is still at its default (frame NULL). This is the stale
        // wrapped cell that an exact-match lookup at frame 1 must reject.
        for _ in 0..10 {
            session.sync_layer.advance_frame();
        }
        let request = session.sync_layer.save_current_state();
        if let FortressRequest::SaveGameState { cell, frame } = request {
            assert_eq!(frame, Frame::new(10));
            cell.save(frame, Some(0u8), Some(0x5A5A_000A));
        }
        session.sync_layer.advance_frame(); // current_frame = 11

        // Confirm through frame 10 (== last_saved_frame; sparse clamps to it).
        session
            .sync_layer
            .set_last_confirmed_frame(Frame::new(10), session.save_mode);
        session.disconnect_frame = Frame::NULL;
        assert_eq!(
            session.sync_layer.last_confirmed_frame(),
            Frame::new(10),
            "sparse confirm should land at the frame-10 checkpoint"
        );

        // interval = 1, last_sent_checksum_frame NULL => frame_to_send = 1.
        let frame_to_send = Frame::new(1);
        // The gate conjuncts pass: frame 1 <= last_confirmed (10) and <= last_saved
        // (10). The ONLY thing that stops the harvest is the EXACT-MATCH miss.
        assert!(frame_to_send <= session.sync_layer.last_confirmed_frame());
        assert!(frame_to_send <= session.sync_layer.last_saved_frame());
        assert!(
            session
                .sync_layer
                .saved_state_by_frame(frame_to_send)
                .is_none(),
            "sparse mode keeps only the frame-10 checkpoint, which occupies frame \
             1's ring slot; the exact-match lookup at frame 1 must MISS (it must \
             not return the wrapped frame-10 cell)"
        );

        session.check_checksum_send_interval();

        // Exact-match miss => harvest skipped: no history entry at frame 1. (The
        // production `report_violation!` Warning on the `None` branch is expected
        // here and is NOT a panic.) The cursor stays put so frame 1 is retried.
        assert!(
            !session.local_checksum_history.contains_key(&frame_to_send),
            "an exact-match miss must skip the harvest, never read a stale ring cell"
        );
        assert!(
            session.last_sent_checksum_frame.is_null(),
            "send cursor must not advance on an exact-match miss"
        );
    }

    // ==========================================
    // F7: last_saved_frame consistency after a deep sparse disconnect rollback
    // ==========================================

    /// F7 gap-closing regression: after a sparse-mode disconnect rollback loads an
    /// EARLIER buffered checkpoint `E < last_saved_frame` (the F7 fix), and the
    /// disconnect adjust runs with `confirmed_frame >= current_frame`, the in-loop
    /// sparse save (gated on `current_frame == min_confirmed`) never fires — so
    /// `adjust_gamestate` leaves `last_saved_frame` pinned to the loaded earlier
    /// frame `E`. This test pins that intermediate state, then drives the very next
    /// production step (`check_last_saved_state`, exactly as `advance_frame` does)
    /// and asserts it re-establishes `last_saved_frame` to `min(confirmed_frame,
    /// current_frame)` via a fresh save at the confirmed frame.
    ///
    /// This closes the one corner that could not be proven by inspection of the F7
    /// fix: that `last_saved_frame` is correctly re-established even when the
    /// confirmed frame is at/above the current frame at the disconnect adjust (the
    /// branch where no save happens during re-simulation).
    #[test]
    fn sparse_disconnect_rollback_loading_earlier_checkpoint_reestablishes_last_saved_frame() {
        const MAX_PREDICTION: usize = 4;
        let mut session: P2PSession<TestConfig> = SessionBuilder::new()
            .with_num_players(2)
            .expect("num_players")
            .with_save_mode(SaveMode::Sparse)
            .with_max_prediction_window(MAX_PREDICTION)
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .expect("local player")
            .add_player(PlayerType::Remote(test_addr(8080)), PlayerHandle::new(1))
            .expect("remote player")
            .start_p2p_session(DummySocket)
            .expect("session");

        let handle0 = PlayerHandle::new(0);
        let handle1 = PlayerHandle::new(1);

        // Build a frame-by-frame history that populates BOTH input queues for every
        // frame in [0, 8) (so re-simulation can fetch real inputs, not a prediction
        // error) and stamps two sparse checkpoints in the saved-states ring:
        //   - E = 4  (window floor for current_frame 8, the EARLIER checkpoint)
        //   - 6      (the contaminated `last_saved_frame`)
        // After the loop current_frame = 8, last_saved_frame = 6.
        for f in 0..8i32 {
            let frame = Frame::new(f);
            let _ = session
                .sync_layer
                .add_local_input(handle0, PlayerInput::new(frame, f as u8));
            session
                .sync_layer
                .add_remote_input(handle1, PlayerInput::new(frame, f as u8));
            if f == 4 || f == 6 {
                // Stamp the cell (as a live consumer would on a SaveGameState
                // request) so `load_frame` accepts it as a valid rollback target.
                if let FortressRequest::SaveGameState { cell, frame: saved } =
                    session.sync_layer.save_current_state()
                {
                    cell.save(saved, Some(f as u8), Some(u128::from(f as u8)));
                }
            }
            session.sync_layer.advance_frame();
        }
        assert_eq!(session.sync_layer.current_frame(), Frame::new(8));
        assert_eq!(session.sync_layer.last_saved_frame(), Frame::new(6));

        // Disconnect handle 1 at agreed freeze frame F = 4, lowering the rollback
        // target BELOW `last_saved_frame`. The frozen queue lets `synchronized_inputs`
        // surface the agreed-frame value during re-simulation instead of a prediction.
        let freeze_frame = Frame::new(4);
        session
            .sync_layer
            .freeze_player(handle1, freeze_frame)
            .expect("freeze");
        if let Some(status) = session.local_connect_status.get_mut(handle1.as_usize()) {
            status.disconnected = true;
            status.last_frame = freeze_frame;
        }
        // Surviving connected player confirms THROUGH the current frame, so
        // `confirmed_frame() == current_frame` — the exact corner under test.
        if let Some(status) = session.local_connect_status.get_mut(handle0.as_usize()) {
            status.disconnected = false;
            status.last_frame = Frame::new(8);
        }
        let confirmed_frame = session.confirmed_frame();
        assert_eq!(
            confirmed_frame,
            session.sync_layer.current_frame(),
            "scenario requires confirmed_frame >= current_frame at the disconnect adjust"
        );

        // The disconnect convergence drives first_incorrect = F + 1 = 5, which is in
        // [window_floor (4), last_saved_frame (6)). The F7 fix must load the earlier
        // checkpoint E = 4 (newest stamped frame in [4, 5]) instead of the
        // contaminated last_saved_frame = 6.
        let first_incorrect = Frame::new(5);
        let mut requests = RequestVec::<TestConfig>::new();
        session
            .adjust_gamestate(first_incorrect, confirmed_frame, &mut requests)
            .expect("adjust_gamestate");

        // After the deep rollback, re-simulation returned to current_frame = 8, but
        // the sparse save (gated on current_frame == min_confirmed == 8) never fired
        // during the loop, so `last_saved_frame` is pinned to the loaded earlier
        // checkpoint E = 4. This is the intermediate state the corner is about.
        assert_eq!(
            session.sync_layer.current_frame(),
            Frame::new(8),
            "re-simulation must return to the original current frame"
        );
        assert_eq!(
            session.sync_layer.last_saved_frame(),
            Frame::new(4),
            "deep rollback loaded the earlier checkpoint E=4 and the in-loop sparse \
             save never fired (min_confirmed == current_frame), so last_saved_frame \
             is pinned to E"
        );

        // Now the production follow-up exactly as advance_frame runs it: with
        // last_saved_frame = 4 and current_frame = 8, the gap is >= max_prediction,
        // and confirmed_frame (8) >= current_frame (8), so check_last_saved_state
        // saves the confirmed frame and re-establishes last_saved_frame.
        let last_saved = session.sync_layer.last_saved_frame();
        session
            .check_last_saved_state(last_saved, confirmed_frame, &mut requests)
            .expect("check_last_saved_state");

        let expected = std::cmp::min(confirmed_frame, session.sync_layer.current_frame());
        assert_eq!(
            session.sync_layer.last_saved_frame(),
            expected,
            "check_last_saved_state must re-establish last_saved_frame to \
             min(confirmed_frame, current_frame) after the deep sparse rollback"
        );
        assert_eq!(
            session.sync_layer.last_saved_frame(),
            Frame::new(8),
            "last_saved_frame must be re-established to the confirmed current frame"
        );
        assert!(
            session.sync_layer.last_saved_frame() <= session.sync_layer.current_frame(),
            "INV-8: last_saved_frame must not exceed current_frame"
        );
    }

    // ==========================================
    // all_sync_health Tests
    // ==========================================

    #[test]
    fn all_sync_health_no_remotes_returns_empty() {
        let session = create_local_only_session();
        let health = session.all_sync_health();
        assert!(health.is_empty());
    }

    #[test]
    fn all_sync_health_with_remote_returns_entry() {
        let session = create_two_player_session();
        let health = session.all_sync_health();
        assert_eq!(health.len(), 1);
        let first = health.first().expect("Expected at least one entry");
        assert_eq!(first.0, PlayerHandle::new(1));
        assert_eq!(first.1, SyncHealth::Pending);
    }

    // ==========================================
    // InvariantChecker Tests
    // ==========================================

    #[test]
    fn check_invariants_no_desync_passes() {
        let session = create_local_only_session();
        session.check_invariants().unwrap();
    }

    #[test]
    fn check_invariants_with_remote_no_desync_passes() {
        let session = create_two_player_session();
        // No desync detected yet
        session.check_invariants().unwrap();
    }

    // ==========================================
    // Full Session Lifecycle Tests
    // ==========================================

    #[test]
    fn full_session_lifecycle_local_only() {
        let mut session = create_local_only_session();

        // Initial state
        assert_eq!(session.current_state(), SessionState::Running);
        assert_eq!(session.current_frame(), Frame::new(0));

        // Advance a few frames
        for i in 0..5 {
            session
                .add_local_input(PlayerHandle::new(0), i as u8)
                .expect("Input failed");
            let requests = session.advance_frame().expect("Advance failed");
            assert!(!requests.is_empty());
        }

        // Verify frame advancement
        assert_eq!(session.current_frame(), Frame::new(5));
    }

    #[test]
    fn session_with_spectator() {
        let session = SessionBuilder::<TestConfig>::new()
            .with_num_players(1)
            .unwrap()
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .expect("Failed to add local player")
            .add_player(
                PlayerType::Spectator(test_addr(9090)),
                PlayerHandle::new(10),
            )
            .expect("Failed to add spectator")
            .start_p2p_session(DummySocket)
            .expect("Failed to create session");

        assert_eq!(session.num_players(), 1);
        assert_eq!(session.num_spectators(), 1);
        assert!(!session.spectator_handles().is_empty());
    }

    // ==========================================
    // Recording Tests
    // ==========================================

    fn create_local_only_session_with_recording() -> P2PSession<TestConfig> {
        SessionBuilder::new()
            .with_num_players(1)
            .unwrap()
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .expect("Failed to add player")
            .with_recording(true)
            .start_p2p_session(DummySocket)
            .expect("Failed to create session")
    }

    #[test]
    fn is_recording_true_when_enabled() {
        let session = create_local_only_session_with_recording();
        assert!(session.is_recording());
    }

    #[test]
    fn is_recording_false_by_default() {
        let session = create_local_only_session();
        assert!(!session.is_recording());
    }

    #[test]
    fn into_replay_without_recording_returns_error() {
        let session = create_local_only_session();
        let result = session.into_replay();
        assert!(result.is_err());
    }

    #[test]
    fn take_replay_without_recording_returns_error() {
        let mut session = create_local_only_session();
        let result = session.take_replay();
        assert!(result.is_err());
    }

    #[test]
    fn record_confirmed_inputs_advances_past_failed_frame() {
        // A fresh session with recording enabled: confirmed_frame() is Frame::NULL,
        // so requesting inputs for Frame(0) will fail. The bug was that
        // last_recorded_frame was never advanced on error, causing infinite retries.
        let mut session = create_local_only_session_with_recording();
        assert_eq!(session.last_recorded_frame, Frame::NULL);

        // Attempt to record up to Frame(5). All frames should fail since no inputs
        // have been confirmed, but last_recorded_frame must advance past ALL
        // failed frames (continue, not break).
        let target_frame = Frame::new(5);
        session.record_confirmed_inputs(target_frame);

        // last_recorded_frame should have advanced to the target frame,
        // because continue (not break) processes every frame in the range.
        assert_eq!(
            session.last_recorded_frame, target_frame,
            "last_recorded_frame should advance to the target frame, was {:?}",
            session.last_recorded_frame
        );

        // The recorder should have placeholder entries for all 6 frames
        // (Frame(0) through Frame(5)), maintaining frame index alignment.
        let recorder = session.recording.as_ref().unwrap();
        assert_eq!(
            recorder.recorded_frames(),
            6,
            "recorder should have placeholder entries for all attempted frames"
        );

        // All frames should be marked as skipped since none had real inputs.
        assert_eq!(
            recorder.skipped_frames(),
            6,
            "all 6 frames should be counted as skipped"
        );

        // Calling again should not re-attempt the same frames (no infinite loop).
        let previous = session.last_recorded_frame;
        session.record_confirmed_inputs(target_frame);
        assert_eq!(
            session.last_recorded_frame, previous,
            "last_recorded_frame should not change when called again with the same target"
        );

        // Verify the replay produced has correct metadata and frame alignment.
        let replay = session.into_replay().unwrap();
        assert_eq!(replay.frames.len(), 6);
        assert_eq!(replay.checksums.len(), 6);
        assert_eq!(replay.metadata.skipped_frames, 6);
        // All frames should have default (0) inputs since they were placeholders.
        for frame_inputs in &replay.frames {
            assert_eq!(frame_inputs.len(), 1); // 1 player
            assert_eq!(frame_inputs[0], u8::default());
        }
        // All checksums should be None for skipped frames.
        for checksum in &replay.checksums {
            assert_eq!(*checksum, None);
        }
        // Replay should pass validation since frame alignment is maintained.
        replay.validate().unwrap();
    }

    // ==========================================
    // Fail-closed disconnect helper tests
    // ==========================================
    //
    // These regression tests pin the contract that
    // `enter_fail_closed_disconnect_state` provides to the two call sites in
    // `update_player_disconnects` and `handle_event(Event::Disconnected)`:
    // when applying a disconnect observation fails partway, the session must
    // transition out of `Running` so subsequent `advance_frame()` calls fail
    // with `NotSynchronized` rather than silently desyncing.

    #[test]
    fn enter_fail_closed_transitions_running_to_synchronizing() {
        let mut session = create_two_player_session();
        // Force Running so we can observe the transition; in a real run this
        // would have happened via `check_initial_sync`.
        session.state = SessionState::Running;

        session.enter_fail_closed_disconnect_state();

        assert_eq!(
            session.current_state(),
            SessionState::Synchronizing,
            "fail-closed must move Running -> Synchronizing",
        );
    }

    #[test]
    fn enter_fail_closed_is_idempotent_when_already_synchronizing() {
        let mut session = create_two_player_session();
        // Default for a session with remotes is already Synchronizing.
        assert_eq!(session.current_state(), SessionState::Synchronizing);

        // Repeated calls must be a no-op; we should never accidentally re-enter
        // any "transition" side effects (the helper only mutates `state`).
        session.enter_fail_closed_disconnect_state();
        session.enter_fail_closed_disconnect_state();

        assert_eq!(session.current_state(), SessionState::Synchronizing);
    }

    #[test]
    fn fail_closed_blocks_advance_frame() {
        // Contract: after a fail-closed transition, `advance_frame()` must
        // refuse to run regardless of what else the session might think.
        let mut session = create_two_player_session();
        session.state = SessionState::Running;

        session.enter_fail_closed_disconnect_state();

        let result = session.advance_frame();
        assert!(
            matches!(result, Err(FortressError::NotSynchronized)),
            "expected NotSynchronized after fail-closed",
        );
    }

    /// Advances the host one frame and persists the emitted save cell so
    /// `last_saved_frame` becomes valid (a prerequisite for serving a snapshot).
    #[cfg(feature = "hot-join")]
    fn advance_host_and_save(host: &mut P2PSession<TestConfig>) {
        host.add_local_input(PlayerHandle::new(0), 0u8).unwrap();
        let requests = host.advance_frame().unwrap();
        for request in &*requests {
            if let FortressRequest::SaveGameState { cell, frame } = request {
                cell.save(*frame, Some(0u8), Some(0u128));
            }
        }
    }

    /// The host-side join-request authorization gate must serve a reserved slot
    /// only to the endpoint that actually owns the requested handle. A
    /// `JoinRequest` is peer-controlled, so an endpoint requesting a reserved
    /// handle bound to a *different* endpoint must be rejected, never served the
    /// other slot's snapshot — while the legitimate owner is unaffected.
    ///
    /// Pinned by serve-opening: with a saved state available, the owning endpoint
    /// requesting its own reserved handle DOES open a serve, but the SAME endpoint
    /// requesting a handle it does not own does NOT. Without the gate the spoofed
    /// request would open a serve too (the handle is reserved), so this is
    /// non-vacuous.
    #[test]
    #[cfg(feature = "hot-join")]
    fn poll_hot_join_host_rejects_join_request_from_non_owning_endpoint() {
        // local 0; reserved 1 @ addr_a; reserved 2 @ addr_b. Two distinct remote
        // endpoints, each the sole owner of one reserved handle.
        let addr_a = test_addr(9101);
        let addr_b = test_addr(9102);
        // This test deliberately assembles an N>=3 hot-join host (two reserved
        // slots on two distinct machines) to exercise `poll_hot_join`'s
        // cross-endpoint join-request ownership gate. The public
        // `start_p2p_session` now rejects N>=3 hot-join meshes (per the build-time
        // guard mitigating the unimplemented N-peer reactivation), so this uses
        // the `#[cfg(test)]`-only bypass to reach the internal state under test.
        let mut host = SessionBuilder::<TestConfig>::new()
            .with_num_players(3)
            .unwrap()
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .unwrap()
            .add_reserved_player(addr_a, PlayerHandle::new(1))
            .unwrap()
            .add_reserved_player(addr_b, PlayerHandle::new(2))
            .unwrap()
            .start_p2p_session_skip_hot_join_build_guards_for_test(DummySocket)
            .unwrap();

        // Reach Running (reserved endpoints are skipped by sync) and advance a
        // couple of frames so the host has a saved state it could serve.
        host.poll_remote_clients();
        assert_eq!(host.current_state(), SessionState::Running);
        advance_host_and_save(&mut host);
        advance_host_and_save(&mut host);
        assert!(!host.sync_layer.last_saved_frame().is_null());

        // addr_b owns handle 2, NOT handle 1. Stage a request from it for handle 1
        // — the cross-endpoint request the gate must reject.
        host.player_reg
            .remotes
            .get_mut(&addr_b)
            .expect("addr_b endpoint exists")
            .set_pending_join_request_for_test(1);
        host.poll_hot_join();
        assert!(
            !host.hot_join.joining.contains_key(&PlayerHandle::new(1)),
            "a join request from a non-owning endpoint must not open a serve"
        );

        // Positive control: the SAME endpoint requesting the slot it DOES own
        // (handle 2) passes the gate and opens a serve — proving the saved state
        // is servable and only the ownership mismatch blocked handle 1.
        host.player_reg
            .remotes
            .get_mut(&addr_b)
            .expect("addr_b endpoint exists")
            .set_pending_join_request_for_test(2);
        host.poll_hot_join();
        assert!(
            host.hot_join.joining.contains_key(&PlayerHandle::new(2)),
            "an endpoint requesting the reserved slot it owns must open a serve"
        );
        assert!(
            !host.hot_join.joining.contains_key(&PlayerHandle::new(1)),
            "the spoofed handle 1 must never be served"
        );
    }

    // ==========================================
    // Graceful-drop hot-join rejoin (rearm) tests
    // ==========================================
    //
    // These pin the "graceful-drop hot-join rejoin" contract: a *cleanly*
    // gracefully-dropped slot on a hot-join-serving host is returned to the same
    // reserved/frozen shape a build-time `add_reserved_player` slot has, so a
    // returning peer can re-join it via the existing serve path. The four shapes a
    // re-armed slot shares with a build-time reserved slot are: queue frozen,
    // connect-status disconnected, endpoint re-synchronizable (`Synchronizing`,
    // NOT the terminal `Disconnected`/`Shutdown`), and handle(s) present in
    // `reserved_slots`.
    //
    // Endpoint-state assertions reuse the existing `is_synchronized()` /
    // `is_running()` accessors instead of adding a test-only state getter. The
    // protocol lifecycle is one-directional
    // (`Initializing → Synchronizing → Running → Disconnected → Shutdown`), and
    // `is_synchronized()` is true ONLY for `Running`/`Disconnected`/`Shutdown`.
    // Therefore `!is_running() && !is_synchronized()` uniquely identifies a
    // re-synchronizing endpoint (`Synchronizing`) and positively rules out the
    // terminal `Disconnected`/`Shutdown` a plain (non-rearmed) drop would leave —
    // exactly the distinction these tests need.

    /// Builds a 2-player host (local 0, remote 1) that serves hot-joins. The
    /// remote slot is a *normal* connected remote (NOT a build-time reserved
    /// slot), so it can later be cleanly dropped and re-armed.
    #[cfg(feature = "hot-join")]
    fn build_hot_join_serving_host(remote_addr: SocketAddr) -> P2PSession<TestConfig> {
        SessionBuilder::<TestConfig>::new()
            .with_num_players(2)
            .unwrap()
            .with_hot_join(true)
            .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .unwrap()
            .add_player(PlayerType::Remote(remote_addr), PlayerHandle::new(1))
            .unwrap()
            .start_p2p_session(DummySocket)
            .expect("hot-join-serving host should build")
    }

    /// `remove_player` on a hot-join-serving host (with the rearm-eligible
    /// `ContinueWithout` behavior) must re-reserve the dropped handle AND leave its
    /// endpoint re-synchronizable (`Synchronizing`), never the terminal
    /// `Disconnected`/`Shutdown`.
    #[test]
    #[cfg(feature = "hot-join")]
    fn remove_player_on_hot_join_serving_host_rearms_dropped_slot() {
        // Arrange: a hot-join-serving host with a normal connected remote slot.
        let addr = test_addr(9201);
        let mut host = build_hot_join_serving_host(addr);
        // Pre-condition non-vacuity: handle 1 is NOT reserved before the drop.
        assert!(
            !host.hot_join.reserved_slots.contains(&PlayerHandle::new(1)),
            "handle 1 must not be reserved before being dropped"
        );

        // Act: cleanly remove the remote player (graceful drop on a serving host).
        host.remove_player(PlayerHandle::new(1))
            .expect("remove_player on a serving host should succeed");

        // Assert: the handle is re-reserved (slot returned to reserved state).
        assert!(
            host.hot_join.reserved_slots.contains(&PlayerHandle::new(1)),
            "a cleanly dropped slot on a serving host must be re-reserved for rejoin"
        );
        // Assert: the connect-status is disconnected (shared with a reserved slot).
        assert!(
            host.local_connect_status[1].disconnected,
            "the dropped slot's connect-status must be marked disconnected"
        );
        // Assert: the endpoint is re-synchronizable (Synchronizing), NOT terminal.
        let endpoint = host
            .player_reg
            .remotes
            .get(&addr)
            .expect("remote endpoint must still exist after rearm");
        assert!(
            !endpoint.is_running() && !endpoint.is_synchronized(),
            "re-armed endpoint must be Synchronizing (not Running/Disconnected/Shutdown)"
        );
        // And the whole endpoint now reads as reserved (its only handle is reserved).
        assert!(
            host.hot_join.endpoint_is_reserved(endpoint),
            "the re-armed endpoint must read as reserved"
        );
    }

    /// `remove_player` on a host that does NOT serve hot-joins must NOT re-reserve
    /// the dropped handle: the slot stays dropped (endpoint terminal
    /// `Disconnected`), exactly as before the rejoin feature.
    #[test]
    #[cfg(feature = "hot-join")]
    fn remove_player_on_non_serving_host_does_not_rearm_dropped_slot() {
        // Arrange: a host that does NOT accept hot-joins (default), ContinueWithout
        // so remove_player still takes the graceful-drop path.
        let addr = test_addr(9202);
        let mut host = SessionBuilder::<TestConfig>::new()
            .with_num_players(2)
            .unwrap()
            .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .unwrap()
            .add_player(PlayerType::Remote(addr), PlayerHandle::new(1))
            .unwrap()
            .start_p2p_session(DummySocket)
            .expect("non-serving host should build");
        assert!(
            !host.hot_join.accept_hot_join,
            "this host must not serve hot-joins"
        );

        // Act: cleanly remove the remote player.
        host.remove_player(PlayerHandle::new(1))
            .expect("remove_player should succeed");

        // Assert: the handle is NOT re-reserved; the slot stays dropped.
        assert!(
            !host.hot_join.reserved_slots.contains(&PlayerHandle::new(1)),
            "a host that does not serve hot-joins must not re-reserve a dropped slot"
        );
        // Assert: the endpoint is in the terminal Disconnected state (is_synchronized
        // is true ONLY for Running/Disconnected/Shutdown; not Running here).
        let endpoint = host
            .player_reg
            .remotes
            .get(&addr)
            .expect("remote endpoint must still exist after a plain drop");
        assert!(
            !endpoint.is_running() && endpoint.is_synchronized(),
            "a non-rearmed dropped endpoint must be terminal (Disconnected/Shutdown)"
        );
    }

    /// The legacy `disconnect_player` (which takes the `Halt`/`Suppress` path) must
    /// NOT re-reserve the slot, even on a hot-join-serving host: the rearm is
    /// strictly scoped to the clean `ContinueWithout` + `Emit` graceful drop.
    #[test]
    #[cfg(feature = "hot-join")]
    fn legacy_disconnect_player_on_serving_host_does_not_rearm_dropped_slot() {
        // Arrange: a hot-join-serving host with a normal connected remote slot.
        let addr = test_addr(9203);
        let mut host = build_hot_join_serving_host(addr);

        // Act: legacy disconnect (Halt behavior, suppressed events).
        host.disconnect_player(PlayerHandle::new(1))
            .expect("legacy disconnect_player should succeed");

        // Assert: the handle is NOT re-reserved (legacy path never re-arms).
        assert!(
            !host.hot_join.reserved_slots.contains(&PlayerHandle::new(1)),
            "legacy disconnect_player (Halt) must not re-reserve the dropped slot"
        );
        // The Halt path also moves the session out of Running into Synchronizing.
        assert_eq!(
            host.current_state(),
            SessionState::Synchronizing,
            "legacy disconnect_player (Halt) must put the session in Synchronizing"
        );
    }

    /// A multi-handle endpoint (one address owning two player handles — couch
    /// co-op) cleanly dropped on a serving host must re-reserve BOTH handles, so
    /// the whole endpoint reads as reserved (`endpoint_is_reserved` requires every
    /// handle reserved).
    #[test]
    #[cfg(feature = "hot-join")]
    fn remove_player_on_multi_handle_endpoint_rearms_all_handles() {
        // Arrange: local 0; one remote address owning handles 1 and 2.
        let addr = test_addr(9204);
        let mut host = SessionBuilder::<TestConfig>::new()
            .with_num_players(3)
            .unwrap()
            .with_hot_join(true)
            .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .unwrap()
            .add_player(PlayerType::Remote(addr), PlayerHandle::new(1))
            .unwrap()
            .add_player(PlayerType::Remote(addr), PlayerHandle::new(2))
            .unwrap()
            .start_p2p_session(DummySocket)
            .expect("multi-handle hot-join-serving host should build");

        // Act: removing either handle drops the whole endpoint (same address).
        host.remove_player(PlayerHandle::new(1))
            .expect("remove_player on a multi-handle endpoint should succeed");

        // Assert: BOTH handles are re-reserved.
        assert!(
            host.hot_join.reserved_slots.contains(&PlayerHandle::new(1))
                && host.hot_join.reserved_slots.contains(&PlayerHandle::new(2)),
            "both handles of a couch-co-op endpoint must be re-reserved; got {:?}",
            host.hot_join.reserved_slots
        );
        // Assert: the endpoint reads as fully reserved (requires ALL handles reserved).
        let endpoint = host
            .player_reg
            .remotes
            .get(&addr)
            .expect("multi-handle endpoint must still exist after rearm");
        assert!(
            host.hot_join.endpoint_is_reserved(endpoint),
            "a multi-handle endpoint must read as reserved only if ALL its handles are reserved"
        );
        // And the endpoint is re-synchronizable, not terminal.
        assert!(
            !endpoint.is_running() && !endpoint.is_synchronized(),
            "the re-armed multi-handle endpoint must be Synchronizing"
        );
    }

    /// The auto disconnect-timeout graceful path (session
    /// `disconnect_behavior == ContinueWithout`) drives the same rearm as the
    /// explicit `remove_player`. This exercises the `handle_event` /
    /// `update_player_disconnects` trigger by invoking the shared
    /// `disconnect_player_with_policy` entry point with the exact arguments those
    /// auto-timeout sites use (`ContinueWithout` + `Emit`), which is what the
    /// disconnect-timeout handler funnels into.
    ///
    /// Driving a real timeout deterministically end-to-end (clock past the
    /// disconnect timeout with no acks) is covered by the integration test
    /// `auto_timeout_dropped_slot_is_rejoinable_without_desync`; this unit test
    /// pins the same rearm at the policy boundary.
    #[test]
    #[cfg(feature = "hot-join")]
    fn auto_timeout_graceful_drop_on_serving_host_rearms_dropped_slot() {
        // Arrange.
        let addr = test_addr(9205);
        let mut host = build_hot_join_serving_host(addr);

        // Act: the auto-timeout path emits a graceful drop via the shared policy
        // entry point with ContinueWithout + Emit (see the Event::Disconnected
        // handler in handle_event / update_player_disconnects).
        let behavior = host.disconnect_behavior;
        host.disconnect_player_with_policy(
            PlayerHandle::new(1),
            None,
            behavior,
            DisconnectEventPolicy::Emit,
            GracefulDropFailurePolicy::DisconnectAndHalt,
        )
        .expect("auto-timeout graceful drop should succeed");

        // Assert: the slot is re-reserved and the endpoint re-synchronizable.
        assert!(
            host.hot_join.reserved_slots.contains(&PlayerHandle::new(1)),
            "an auto-timeout graceful drop on a serving host must re-reserve the slot"
        );
        let endpoint = host
            .player_reg
            .remotes
            .get(&addr)
            .expect("remote endpoint must still exist after rearm");
        assert!(
            !endpoint.is_running() && !endpoint.is_synchronized(),
            "the auto-timeout re-armed endpoint must be Synchronizing"
        );
    }

    // ======================================================================
    // ARBITRATION (audit finding F13): heterogeneous survivor disconnect
    // policies (graceful ContinueWithout rearm on some survivors vs Halt on
    // another) and whether they leave LIVE survivors disagreeing on a
    // dropped slot's rejoin eligibility.
    // ======================================================================
    //
    // F13 claim: in an N=4 mesh A,B,C,D where the app wires survivors to
    // DIFFERENT disconnect entry points after D drops — A `remove_player(D)`
    // (ContinueWithout, rearm), C auto-timeout under ContinueWithout (rearm),
    // B auto-timeout under Halt (no rearm, → Synchronizing) — the live
    // set / reactivation disagrees across ALL survivors, breaking invariant
    // (6) (live-player set must agree across survivors).
    //
    // The decisive question (crux): under Halt, B transitions to
    // `SessionState::Synchronizing` and `advance_frame()` returns
    // `NotSynchronized` (see `enter_fail_closed_disconnect_state` and the
    // state gate at the top of `advance_frame`). A Halted survivor therefore
    // produces NO further confirmed frames and is NOT a live participant — so
    // it cannot be in a *confirmed-stream* desync with the still-live A/C.
    // "Survivors disagree" conflates a live survivor with one that has
    // correctly, by the application's explicit choice of the Halt API, left
    // the simulation. The only genuine-desync question is whether the
    // survivors that REMAIN LIVE (A via `remove_player`, C via auto-timeout,
    // both ContinueWithout) agree — that is asserted below.

    /// Builds a hot-join-serving host with the given disconnect behavior, a
    /// local player at handle 0 and a normal connected remote (the to-be-dropped
    /// peer "D") at handle 1.
    #[cfg(feature = "hot-join")]
    fn build_serving_host_with_behavior(
        remote_addr: SocketAddr,
        behavior: DisconnectBehavior,
    ) -> P2PSession<TestConfig> {
        SessionBuilder::<TestConfig>::new()
            .with_num_players(2)
            .expect("2 players is valid")
            .with_hot_join(true)
            .with_disconnect_behavior(behavior)
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .expect("local player 0 is valid")
            .add_player(PlayerType::Remote(remote_addr), PlayerHandle::new(1))
            .expect("remote player 1 is valid")
            .start_p2p_session(DummySocket)
            .expect("serving host should build")
    }

    /// F13 (faithful repro of the contested scenario): three survivors with
    /// HETEROGENEOUS disconnect entry points for the same dropped peer D
    /// (handle 1):
    /// - A: `remove_player(D)` — ContinueWithout graceful drop (rearm).
    /// - C: auto-timeout under ContinueWithout — same graceful policy entry
    ///   point the timeout/event handler funnels into (rearm).
    /// - B: auto-timeout under Halt — the SAME `Event::Disconnected` /
    ///   auto-timeout funnel (behavior + `Emit`), but with Halt behavior (no
    ///   rearm, fail-closed). Using the Emit funnel rather than the Suppress
    ///   legacy `disconnect_player` keeps the rearm gate's
    ///   `behavior == ContinueWithout` clause load-bearing for B.
    ///
    /// The finding asserts these "leave a dropped slot rejoinable on some
    /// survivors and permanently rejected on others … survivors now disagree".
    ///
    /// To make the transitions and endpoint deltas REAL (not vacuous), each
    /// survivor is first driven genuinely LIVE before the drop: its D endpoint
    /// is forced to `Running` via the blessed test helper
    /// [`UdpProtocol::force_running_for_tests`] and the session itself is set to
    /// `SessionState::Running` (the same pattern used by the fail-closed and
    /// multi-handle disconnect tests in this module). A 1-remote session is
    /// otherwise born `Synchronizing` over `DummySocket`, so without this Arrange
    /// no survivor would ever leave `Synchronizing` and the state/endpoint
    /// assertions would be true-from-construction.
    ///
    /// This test then pins what ACTUALLY happens to genuinely-live survivors and
    /// shows the divergence is NOT a desync among live participants: B (Halt)
    /// genuinely transitions `Running → Synchronizing` and stops producing
    /// confirmed frames (no longer a live participant), while the two survivors
    /// that STAY LIVE (A and C) keep `Running` AND AGREE — both rearm the slot,
    /// re-reserve handle D, and flip its endpoint from `Running` to
    /// re-synchronizable. If F13 were a real cross-live-survivor desync, the
    /// `assert_eq!`s comparing A's and C's rejoin eligibility would fail.
    #[test]
    #[cfg(feature = "hot-join")]
    fn heterogeneous_survivor_drop_policies_live_survivors_agree_halted_steps_out() {
        // Arrange: three independent survivors, each holding the same dropped
        // peer D at handle 1. (Modeling each survivor as its own 2-player
        // serving host isolates exactly the per-survivor rearm/halt decision the
        // finding contests, with no cross-talk to confound it.)
        let addr_a = test_addr(9301);
        let addr_b = test_addr(9302);
        let addr_c = test_addr(9303);
        let mut survivor_a =
            build_serving_host_with_behavior(addr_a, DisconnectBehavior::ContinueWithout);
        let mut survivor_b = build_serving_host_with_behavior(addr_b, DisconnectBehavior::Halt);
        let mut survivor_c =
            build_serving_host_with_behavior(addr_c, DisconnectBehavior::ContinueWithout);

        let d = PlayerHandle::new(1);

        // Drive every survivor genuinely LIVE before the drop. A 1-remote
        // session is born `Synchronizing` and `DummySocket` delivers no traffic,
        // so we force the D endpoint (handle 1, keyed by the survivor's own
        // remote address) to `Running` and set the session live. Without this,
        // the post-drop state/endpoint deltas below would be vacuous (true from
        // construction). The forced state only touches the endpoint's protocol
        // `state`/`remote_magic`, not the sync layer, so the graceful-drop
        // freeze path behaves identically.
        for (name, survivor, addr) in [
            ("A", &mut survivor_a, addr_a),
            ("B", &mut survivor_b, addr_b),
            ("C", &mut survivor_c, addr_c),
        ] {
            survivor
                .player_reg
                .remotes
                .get_mut(&addr)
                .unwrap_or_else(|| panic!("survivor {name}: D endpoint must exist at build time"))
                .force_running_for_tests();
            survivor.state = SessionState::Running;
        }

        // Pre-condition baselines (now all non-vacuous): every survivor is
        // genuinely live with a live D endpoint, and D is not yet reserved.
        for (name, survivor, addr) in [
            ("A", &survivor_a, addr_a),
            ("B", &survivor_b, addr_b),
            ("C", &survivor_c, addr_c),
        ] {
            assert_eq!(
                survivor.current_state(),
                SessionState::Running,
                "survivor {name}: must be live (Running) before the drop"
            );
            let endpoint = survivor.player_reg.remotes.get(&addr).unwrap_or_else(|| {
                panic!("survivor {name}: D endpoint must exist before the drop")
            });
            assert!(
                endpoint.is_running(),
                "survivor {name}: D endpoint must be live (running) before the drop"
            );
            assert!(
                !survivor.hot_join.reserved_slots.contains(&d),
                "survivor {name}: D must not be reserved before the drop"
            );
        }

        // Act:
        // A — explicit graceful removal (ContinueWithout → rearm path).
        survivor_a
            .remove_player(d)
            .expect("A: remove_player should succeed");
        // C — auto-timeout under ContinueWithout. The disconnect-timeout / remote
        // disconnect-event handler funnels into `disconnect_player_with_policy`
        // with the session behavior + Emit (see the `Event::Disconnected`
        // handler), which is what drives the auto-rearm. Invoke that exact entry
        // point with C's configured behavior.
        let c_behavior = survivor_c.disconnect_behavior;
        survivor_c
            .disconnect_player_with_policy(
                d,
                None,
                c_behavior,
                DisconnectEventPolicy::Emit,
                GracefulDropFailurePolicy::DisconnectAndHalt,
            )
            .expect("C: auto-timeout graceful drop should succeed");
        // B — auto-timeout under Halt. We use the SAME faithful funnel as C (the
        // `Event::Disconnected` handler at the auto-timeout call site passes
        // `self.disconnect_behavior` + `Emit`), but with B's configured Halt
        // behavior. Going through the Emit path (rather than the Suppress legacy
        // `disconnect_player`) is what makes the rearm gate's
        // `behavior == ContinueWithout` clause load-bearing: under Halt + Emit
        // the gate must still NOT rearm. (`disconnect_player` would short-circuit
        // the rearm via `Suppress` regardless of behavior, hiding that clause.)
        let b_behavior = survivor_b.disconnect_behavior;
        assert_eq!(
            b_behavior,
            DisconnectBehavior::Halt,
            "B must be configured for Halt so this exercises the Halt + Emit funnel"
        );
        survivor_b
            .disconnect_player_with_policy(
                d,
                None,
                b_behavior,
                DisconnectEventPolicy::Emit,
                GracefulDropFailurePolicy::DisconnectAndHalt,
            )
            .expect("B: auto-timeout Halt drop should succeed");

        // Assert (crux): the Halted survivor B GENUINELY transitions out of the
        // live (Running) state it held in the baseline. It was `Running` before
        // the drop; the Halt funnel moves it to `Synchronizing`, after which
        // `advance_frame()` fails closed — so B produces no further confirmed
        // frames and cannot be in a confirmed-stream desync with the live
        // survivors.
        assert_eq!(
            survivor_b.current_state(),
            SessionState::Synchronizing,
            "B (Halt) must transition Running -> Synchronizing (was Running in the baseline) — \
             it is no longer a live participant"
        );
        // Match the error EXACTLY (no `if let Ok(..)` swallow): a Synchronizing
        // session must reject frame advance with `NotSynchronized` and nothing
        // else. (`TestConfig` is not `Debug`, so the success payload cannot be
        // formatted; `matches!` checks the exact expected error without it.)
        assert!(
            matches!(
                survivor_b.advance_frame(),
                Err(FortressError::NotSynchronized)
            ),
            "B (Halt) must reject frame advance with NotSynchronized — it serves no rejoin and \
             produces no confirmed frames"
        );
        assert!(
            !survivor_b.hot_join.reserved_slots.contains(&d),
            "B (Halt) must NOT re-reserve D — and, being Synchronizing, will not serve a rejoin"
        );

        // Assert (the genuine cross-survivor question): the survivors reached via
        // DIFFERENT graceful entry points but the SAME ContinueWithout policy
        // both STAY LIVE. They were `Running` in the baseline and a clean
        // ContinueWithout drop never flips the session out of `Running`, so this
        // proves they remain live participants while the Halt survivor B left.
        assert_eq!(
            survivor_a.current_state(),
            SessionState::Running,
            "A (ContinueWithout via remove_player) must STAY Running — it remains a live \
             participant after the drop"
        );
        assert_eq!(
            survivor_c.current_state(),
            SessionState::Running,
            "C (ContinueWithout via auto-timeout) must STAY Running — it remains a live \
             participant after the drop"
        );

        // Reserved-slot delta: both live survivors re-reserve D (false -> true
        // vs the baseline), and they AGREE; B (Halt) does not.
        let a_reserved = survivor_a.hot_join.reserved_slots.contains(&d);
        let c_reserved = survivor_c.hot_join.reserved_slots.contains(&d);
        assert_eq!(
            a_reserved, c_reserved,
            "live survivors A and C must AGREE on whether D's slot is reserved/rejoinable \
             (A via remove_player, C via auto-timeout, both ContinueWithout)"
        );
        assert!(
            a_reserved,
            "both live survivors must re-reserve D for rejoin under ContinueWithout \
             (was not reserved in the baseline)"
        );

        // Endpoint rejoinable delta: D's endpoint was forced `Running` in the
        // baseline; after the rearm both live survivors flip it to
        // re-synchronizable (`!is_running() && !is_synchronized()`, i.e.
        // Synchronizing) — a genuine running -> re-synchronizable transition, and
        // A and C AGREE. (A non-rearmed drop would instead leave it terminal
        // Disconnected, where `is_synchronized()` is true.)
        let endpoint_a = survivor_a
            .player_reg
            .remotes
            .get(&addr_a)
            .expect("A: D endpoint must still exist after rearm");
        let endpoint_c = survivor_c
            .player_reg
            .remotes
            .get(&addr_c)
            .expect("C: D endpoint must still exist after rearm");
        let a_rejoinable = !endpoint_a.is_running() && !endpoint_a.is_synchronized();
        let c_rejoinable = !endpoint_c.is_running() && !endpoint_c.is_synchronized();
        assert_eq!(
            a_rejoinable, c_rejoinable,
            "live survivors A and C must agree on D's endpoint rejoin-readiness"
        );
        assert!(
            a_rejoinable,
            "both live survivors must flip D's endpoint from running to re-synchronizable \
             (Synchronizing) after rearm"
        );
    }

    // ======================================================================
    // Finding A (Completeness-Critic #3): update_player_disconnects folds the
    // agreed min over RUNNING endpoints only. Arbitration verdict on HEAD:
    // NOTABUG. These tests pin WHY the is_running() filter is safe, prove the
    // filter is the load-bearing line (non-vacuity), and prove a survivor's
    // genuine lower view is already permanently mined into our OWN local view
    // before it could ever go non-running (recoverability).
    // ======================================================================

    /// Builds an `A(local) + B,C,D(remote)` session with three DISTINCT remote
    /// endpoints, so each survivor's `peer_connect_status` can be driven
    /// independently. (`num_players = 4`: handle 0 = local A, 1 = B, 2 = C,
    /// 3 = D.) The session is forced live with every remote endpoint `Running`,
    /// the same pattern the F13 / multi-handle disconnect tests use.
    fn build_abcd_live_session() -> (P2PSession<TestConfig>, SocketAddr, SocketAddr, SocketAddr) {
        let addr_b = test_addr(9401);
        let addr_c = test_addr(9402);
        let addr_d = test_addr(9403);
        let mut session = SessionBuilder::<TestConfig>::new()
            .with_num_players(4)
            .expect("4 players is valid")
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .expect("local A is valid")
            .add_player(PlayerType::Remote(addr_b), PlayerHandle::new(1))
            .expect("remote B is valid")
            .add_player(PlayerType::Remote(addr_c), PlayerHandle::new(2))
            .expect("remote C is valid")
            .add_player(PlayerType::Remote(addr_d), PlayerHandle::new(3))
            .expect("remote D is valid")
            .start_p2p_session(DummySocket)
            .expect("session should build");
        for addr in [addr_b, addr_c, addr_d] {
            session
                .player_reg
                .remotes
                .get_mut(&addr)
                .expect("remote endpoint must exist at build time")
                .force_running_for_tests();
        }
        session.state = SessionState::Running;
        (session, addr_b, addr_c, addr_d)
    }

    /// Faithful model: dropped peer C is non-running the production way (its
    /// endpoint already disconnected); survivor B is non-running the ONLY
    /// production Running->non-running survivor way — a hot-join rearm, which
    /// rebuilds the endpoint via `new()` and RESETS `peer_connect_status` to
    /// defaults `{disconnected:false, last_frame:NULL}`. D stays running with a
    /// real low view of C. With B holding only a default (NULL) view, excluding
    /// it from the min loses nothing: the converged freeze frame for C is the
    /// correct global min over the REAL (running) views.
    #[test]
    fn update_player_disconnects_nonrunning_survivor_holds_default_view_converges_to_real_min() {
        // Arrange.
        let (mut session, addr_b, addr_c, addr_d) = build_abcd_live_session();
        let c = PlayerHandle::new(2);

        // C is the dropped peer: its endpoint is disconnected (non-running) AND
        // our own local view of C is marked disconnected — the synchronous pair
        // the disconnect path always sets together. Local view of C starts high
        // (frame 100) so the propagation re-adjust path is exercised.
        session
            .player_reg
            .remotes
            .get_mut(&addr_c)
            .expect("C endpoint")
            .disconnect();
        session.local_connect_status[c.as_usize()] = ConnectionStatus {
            disconnected: false,
            last_frame: Frame::new(100),
        };

        // Survivor D stays RUNNING and reports a genuine low view of C: C's
        // input is only confirmed through frame 5 from D's perspective.
        session
            .player_reg
            .remotes
            .get_mut(&addr_d)
            .expect("D endpoint")
            .set_peer_connect_status_for_tests(
                c,
                ConnectionStatus {
                    disconnected: true,
                    last_frame: Frame::new(5),
                },
            );

        // Survivor B goes non-running the production way: a hot-join rearm
        // rebuilds it via new(), so its peer_connect_status is reset to default
        // (disconnected=false, last_frame=NULL). It holds NO lower view of C.
        // (Done by hand here so the test compiles without the hot-join feature;
        // `rearm_for_rejoin` is the production caller and is asserted to produce
        // exactly this state by the protocol-layer tests.)
        {
            let b_endpoint = session
                .player_reg
                .remotes
                .get_mut(&addr_b)
                .expect("B endpoint");
            b_endpoint.set_peer_connect_status_for_tests(c, ConnectionStatus::default());
            b_endpoint.force_synchronizing_for_tests();
            assert!(!b_endpoint.is_running(), "B must be non-running (rearmed)");
        }

        // Act.
        session.update_player_disconnects();

        // Assert: C converged to the real global min over running views. D said
        // 5; our local view was 100. min(5, 100) = 5. B's reset default view is
        // correctly irrelevant — it never held a lower number to lose.
        assert!(
            session.local_connect_status[c.as_usize()].disconnected,
            "C must be marked disconnected after propagation"
        );
        assert_eq!(
            session.local_connect_status[c.as_usize()].last_frame,
            Frame::new(5),
            "C must converge to the real global-min freeze frame from the running survivor D"
        );
    }

    /// NON-VACUITY: manufacture the (production-unreachable) state where B is
    /// non-running BUT still holds a LOWER view of C, then run the exact same
    /// scenario with B RUNNING. The ONLY difference between the two runs is
    /// `endpoint.is_running()` for B (toggled via the blessed test helper). The
    /// non-running run EXCLUDES B's lower view (min = D's 7); the running run
    /// INCLUDES it (min = B's 3). This proves the `is_running()` filter is the
    /// load-bearing line driving the behavior — not some incidental default.
    #[test]
    fn update_player_disconnects_is_running_filter_drives_min_inclusion() {
        // Helper that builds the identical scenario and toggles ONLY B's running
        // state, returning C's converged freeze frame.
        fn converged_c_frame(b_running: bool) -> Frame {
            let (mut session, addr_b, addr_c, addr_d) = build_abcd_live_session();
            let c = PlayerHandle::new(2);

            // C dropped; our local view of C is high so re-adjust fires.
            session
                .player_reg
                .remotes
                .get_mut(&addr_c)
                .expect("C endpoint")
                .disconnect();
            session.local_connect_status[c.as_usize()] = ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(100),
            };

            // D (always running) reports C confirmed through frame 7.
            session
                .player_reg
                .remotes
                .get_mut(&addr_d)
                .expect("D endpoint")
                .set_peer_connect_status_for_tests(
                    c,
                    ConnectionStatus {
                        disconnected: true,
                        last_frame: Frame::new(7),
                    },
                );

            // B holds a STRICTLY LOWER view of C (frame 3). The ONLY thing we
            // vary across the two runs is whether B is running.
            {
                let b_endpoint = session
                    .player_reg
                    .remotes
                    .get_mut(&addr_b)
                    .expect("B endpoint");
                b_endpoint.set_peer_connect_status_for_tests(
                    c,
                    ConnectionStatus {
                        disconnected: true,
                        last_frame: Frame::new(3),
                    },
                );
                if b_running {
                    b_endpoint.force_running_for_tests();
                } else {
                    b_endpoint.force_synchronizing_for_tests();
                }
                assert_eq!(
                    b_endpoint.is_running(),
                    b_running,
                    "B's running state must match the parameter under test"
                );
            }

            session.update_player_disconnects();
            session.local_connect_status[c.as_usize()].last_frame
        }

        // Act + Assert.
        let frame_b_nonrunning = converged_c_frame(false);
        let frame_b_running = converged_c_frame(true);

        // B non-running: its lower view (3) is EXCLUDED; min is D's 7.
        assert_eq!(
            frame_b_nonrunning,
            Frame::new(7),
            "non-running B's lower view (3) must be excluded; min is the running survivor D's 7"
        );
        // B running: its lower view (3) is INCLUDED; min drops to 3.
        assert_eq!(
            frame_b_running,
            Frame::new(3),
            "running B's lower view (3) must be included, lowering the converged min to 3"
        );
        // The delta between the two outcomes is attributable SOLELY to the
        // is_running() filter — the only line that changed behavior between runs.
        assert_ne!(
            frame_b_nonrunning, frame_b_running,
            "the is_running() filter must be the sole cause of the differing converged min"
        );
    }

    /// RECOVERABILITY: a survivor B's genuine lower view of C, observed WHILE B
    /// was running, is folded into our OWN local view of C during
    /// `update_player_disconnects`. Once B subsequently goes non-running (its
    /// view reset by a rearm), a fresh cycle CANNOT un-converge our local view:
    /// it stays at the mined-down value permanently. So nothing a non-running
    /// survivor "knew" is lost — it was already permanently captured.
    #[test]
    fn update_player_disconnects_mined_local_view_is_permanent_after_survivor_goes_nonrunning() {
        // Arrange: C dropped; our local view of C starts high (100).
        let (mut session, addr_b, addr_c, addr_d) = build_abcd_live_session();
        let c = PlayerHandle::new(2);
        session
            .player_reg
            .remotes
            .get_mut(&addr_c)
            .expect("C endpoint")
            .disconnect();
        session.local_connect_status[c.as_usize()] = ConnectionStatus {
            disconnected: false,
            last_frame: Frame::new(100),
        };

        // B is RUNNING and reports a low view of C (frame 4). D running, higher (9).
        session
            .player_reg
            .remotes
            .get_mut(&addr_b)
            .expect("B endpoint")
            .set_peer_connect_status_for_tests(
                c,
                ConnectionStatus {
                    disconnected: true,
                    last_frame: Frame::new(4),
                },
            );
        session
            .player_reg
            .remotes
            .get_mut(&addr_d)
            .expect("D endpoint")
            .set_peer_connect_status_for_tests(
                c,
                ConnectionStatus {
                    disconnected: true,
                    last_frame: Frame::new(9),
                },
            );

        // Act 1: first cycle with B running mines our local view of C down to 4.
        session.update_player_disconnects();
        assert_eq!(
            session.local_connect_status[c.as_usize()].last_frame,
            Frame::new(4),
            "first cycle (B running) must mine our local view of C down to B's low 4"
        );

        // B now goes non-running the production way: its view is RESET to default
        // (NULL) by the rearm, and it stops contributing to the min.
        {
            let b_endpoint = session
                .player_reg
                .remotes
                .get_mut(&addr_b)
                .expect("B endpoint");
            b_endpoint.set_peer_connect_status_for_tests(c, ConnectionStatus::default());
            b_endpoint.force_synchronizing_for_tests();
        }

        // Act 2: a fresh cycle now sees only D (9) among running survivors plus
        // our own (already-mined) local view of 4.
        session.update_player_disconnects();

        // Assert: our local view STAYS at 4 — the mined-down value is permanent.
        // `local_min_confirmed` (4) is folded UNCONDITIONALLY and min(4, 9) = 4,
        // so B going non-running can NOT un-converge what it already taught us.
        assert_eq!(
            session.local_connect_status[c.as_usize()].last_frame,
            Frame::new(4),
            "the mined-down local view (4) must persist after B goes non-running — \
             nothing a non-running survivor knew is lost"
        );
    }

    // ======================================================================
    // Finding B (Completeness-Critic #5): max_frame_advantage folds a
    // multi-handle endpoint's average once per handle and gates on
    // local_connect_status.disconnected rather than endpoint.is_running().
    // Arbitration verdict on HEAD: NOTABUG. These tests pin idempotence of the
    // per-handle max() fold and that the disconnected gate excludes a draining
    // multi-handle endpoint while counting a connected+running one.
    // ======================================================================

    /// Idempotence: a 2-handle endpoint with a seeded per-endpoint average X is
    /// folded once per handle via `max(interval, avg)`. Because
    /// `average_frame_advantage()` is per-ENDPOINT (independent of handle),
    /// `max(X, X) == X` — NOT 2X. Non-vacuous: an additive fold would read 2X.
    #[test]
    fn max_frame_advantage_multi_handle_endpoint_is_idempotent_not_additive() {
        // Arrange: 3-player session with a SINGLE remote endpoint owning BOTH
        // remote handles (1 and 2) — couch co-op behind one address.
        let mut session = create_multi_handle_remote_session();
        let addr = test_addr(8080);
        let seeded = 11;
        {
            let endpoint = session
                .player_reg
                .remotes
                .get_mut(&addr)
                .expect("multi-handle endpoint must exist");
            endpoint.force_running_for_tests();
            endpoint.seed_frame_advantage_for_tests(seeded);
            // Precondition: the endpoint owns exactly the two remote handles, so
            // the inner per-handle loop folds the same average twice.
            assert_eq!(
                endpoint.handles().len(),
                2,
                "endpoint must own both remote handles for the per-handle fold to run twice"
            );
            assert_eq!(
                endpoint.average_frame_advantage(),
                seeded,
                "seed helper must make the per-endpoint average exactly the target"
            );
        }
        // Both remote handles are connected in our local view.
        session.local_connect_status[1] = ConnectionStatus {
            disconnected: false,
            last_frame: Frame::new(0),
        };
        session.local_connect_status[2] = ConnectionStatus {
            disconnected: false,
            last_frame: Frame::new(0),
        };

        // Act.
        let advantage = session.max_frame_advantage();

        // Assert: the per-handle max() fold is idempotent — X, not 2X.
        assert_eq!(
            advantage,
            seeded,
            "max() fold over both handles must yield the per-endpoint average X ({seeded}), \
             not the additive 2X ({})",
            seeded * 2
        );
        assert_ne!(
            advantage,
            seeded * 2,
            "an additive (per-handle accumulating) fold would have produced 2X — it must not"
        );
    }

    /// Exclusion gate: a draining multi-handle endpoint whose handles are marked
    /// `disconnected` in `local_connect_status` is EXCLUDED (contributes
    /// nothing), while a connected+running endpoint with a seeded advantage IS
    /// counted. Confirms `!status.disconnected` excludes the same set an
    /// `is_running()` guard would (the disconnect path sets endpoint non-running
    /// and status.disconnected together, synchronously).
    #[test]
    fn max_frame_advantage_excludes_disconnected_multi_handle_counts_connected_running() {
        // --- Case 1: a disconnected multi-handle endpoint is excluded. ---
        let mut session = create_multi_handle_remote_session();
        let addr = test_addr(8080);
        {
            let endpoint = session
                .player_reg
                .remotes
                .get_mut(&addr)
                .expect("multi-handle endpoint must exist");
            endpoint.force_running_for_tests();
            endpoint.seed_frame_advantage_for_tests(42);
        }
        // Both of the endpoint's handles are disconnected in our local view —
        // the production-faithful state (the disconnect path marks every
        // affected handle's status.disconnected = true).
        session.local_connect_status[1] = ConnectionStatus {
            disconnected: true,
            last_frame: Frame::new(10),
        };
        session.local_connect_status[2] = ConnectionStatus {
            disconnected: true,
            last_frame: Frame::new(10),
        };

        let advantage_when_disconnected = session.max_frame_advantage();
        assert_eq!(
            advantage_when_disconnected, 0,
            "a fully-disconnected multi-handle endpoint must contribute nothing \
             (max_frame_advantage falls back to 0 when no connected remote is folded)"
        );

        // --- Case 2: a connected+running endpoint with a seeded advantage is
        // counted. ---
        let mut session = create_multi_handle_remote_session();
        {
            let endpoint = session
                .player_reg
                .remotes
                .get_mut(&addr)
                .expect("multi-handle endpoint must exist");
            endpoint.force_running_for_tests();
            endpoint.seed_frame_advantage_for_tests(42);
        }
        session.local_connect_status[1] = ConnectionStatus {
            disconnected: false,
            last_frame: Frame::new(10),
        };
        session.local_connect_status[2] = ConnectionStatus {
            disconnected: false,
            last_frame: Frame::new(10),
        };

        let advantage_when_connected = session.max_frame_advantage();
        assert_eq!(
            advantage_when_connected, 42,
            "a connected+running endpoint's seeded average must be counted"
        );
    }
}
