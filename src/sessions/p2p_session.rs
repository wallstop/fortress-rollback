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

/// Number of [`poll_remote_clients`](P2PSession::poll_remote_clients) calls over
/// which an N-peer coordinator re-sends the `JoinCommitted`/`JoinAborted`
/// lifecycle message to the joiner and every survivor after a serve concludes.
///
/// The budget only paces the *proactive* fan-out; delivery to every reopened
/// participant is reliable-until-converged regardless: a reopened survivor
/// keeps re-acking `ReactivateSlotAck{h, F}` each poll until it hears the
/// outcome, and the joiner re-sends its `StateSnapshotAck` likewise — the
/// coordinator's post-serve responder answers each straggler with one more
/// lifecycle resend. A survivor must never *guess* the outcome (observed input
/// progress is not proof of a commit: a live joiner legally feeds a reopened
/// queue between the reopen and an eventual abort), so the explicit message is
/// the only close. Small and deterministic for tests.
#[cfg(feature = "hot-join")]
pub(crate) const NPEER_JOIN_LIFECYCLE_RESENDS: usize = 10;

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
    /// The single in-flight **N-peer** coordinator serve (one join at a time,
    /// mesh-wide). `Some` from the moment a `JoinRequest` opens an N-peer serve
    /// (survivor set non-empty) until commit or abort. While `Some` the
    /// coordinator is **paused** like the 2-peer serve, but with
    /// rollback-while-paused semantics (see
    /// [`advance_frame`](P2PSession::advance_frame)).
    npeer: Option<NPeerServe<T>>,
    /// Post-serve `JoinCommitted`/`JoinAborted` announcer + commit responder.
    /// Lives past the serve so lifecycle delivery tolerates loss; replaced when
    /// the next N-peer serve concludes, cleared when one opens.
    npeer_post: Option<NPeerPostServe<T>>,
    /// One-frame next-serve guard (R3): after an N-peer abort, a new N-peer
    /// serve may only open once the coordinator's last-saved frame has advanced
    /// PAST this value, so every attempt's `(handle, F)` pair is unique and
    /// stale lifecycle messages are discriminable. `Frame::NULL` = no guard.
    npeer_next_serve_min_frame: Frame,
    /// Survivor-side pending reactivation for the single in-flight mesh join
    /// (one join at a time, mesh-wide). Driven by
    /// [`poll_npeer_survivor`](P2PSession::poll_npeer_survivor) on every
    /// hot-join session — survivors do not opt in.
    pending_reactivation: Option<PendingReactivation<T>>,
    /// Survivor-side per-handle high-water of CLOSED reactivation attempts:
    /// the highest activation frame `F` whose attempt for the handle this
    /// session concluded (lifecycle close, implied close, or local
    /// joiner-death close). A directive with `frame <=` this value is a stale
    /// straggler of a closed attempt and is rejected: its lifecycle messages
    /// no longer exist anywhere once the coordinator's next serve supersedes
    /// the post-serve responder memo, so accepting it would wedge this
    /// survivor in a never-closeable reopened attempt (and, transitively,
    /// stall every future join of the slot mesh-wide). Sound because R3 makes
    /// activation frames strictly monotone across a coordinator's attempts,
    /// so every genuine new attempt carries a strictly higher frame.
    /// Bounded: at most one entry per player handle.
    npeer_closed_attempt_frames: BTreeMap<PlayerHandle, Frame>,
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
            npeer: None,
            npeer_post: None,
            npeer_next_serve_min_frame: Frame::NULL,
            pending_reactivation: None,
            npeer_closed_attempt_frames: BTreeMap::new(),
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

/// Coordinator-side state for a single in-flight **N-peer** join serve
/// (chunk N2 of the Session-18 design).
///
/// Unlike the 2-peer [`JoinServe`], the snapshot is **not** captured at open:
/// the activation frame must clear the survivor cap (`F = S + 1` where
/// `S = L =` the coordinator's last-sent frame, frozen by the pause), but the
/// saved state at `S` may still embed *predicted* survivor inputs when the
/// request arrives. The serve therefore pauses first (fixing `L`), then waits —
/// still paused, with rollback repair running each `advance_frame` — until the
/// coordinator's own confirmed frame reaches `S` with no misprediction pending,
/// and only then captures (wait-then-capture). `ReactivateSlot{h, F}`
/// directives broadcast to survivors from the first poll (F is fixed at open),
/// shortening the barrier.
#[cfg(feature = "hot-join")]
struct NPeerServe<T>
where
    T: Config,
{
    /// The reserved handle being filled.
    handle: PlayerHandle,
    /// The joiner endpoint's address.
    joiner_addr: T::Address,
    /// `S = L =` the coordinator's last-sent (= last-saved, gated `EveryFrame`/
    /// zero-delay) frame at open. The snapshot is captured at exactly `S`.
    snapshot_frame: Frame,
    /// `F = S + 1`, the mesh-wide activation frame carried verbatim in every
    /// `ReactivateSlot` directive and lifecycle message of this attempt.
    activation_frame: Frame,
    /// The snapshot, captured once the wait-then-capture gate passes; re-sent
    /// to the joiner each poll thereafter (reliable retransmit).
    snapshot: Option<StateSnapshot>,
    /// Whether the joiner has acked the snapshot (`StateSnapshotAck(S)`).
    joiner_acked: bool,
    /// Survivor addresses snapshotted at open: running, non-reserved remote
    /// endpoints excluding the joiner's address. Fixed for the attempt (used
    /// for lifecycle fan-out).
    survivors: std::collections::BTreeSet<T::Address>,
    /// Survivors that have not yet acked `ReactivateSlot{h, F}`. Directives are
    /// re-sent to these each poll; a survivor that drops mid-join is pruned
    /// (its slot freezes via the normal machinery; the join continues).
    pending_acks: std::collections::BTreeSet<T::Address>,
    /// Number of polls since the serve opened; aborts at the session's
    /// configured serve timeout (shared with the 2-peer path).
    polls_since_serve: usize,
}

/// Post-serve lifecycle announcer (and committed-case responder) for the most
/// recently concluded N-peer serve.
///
/// Carries `JoinCommitted`/`JoinAborted{h, F}` to the joiner and every survivor
/// for [`NPEER_JOIN_LIFECYCLE_RESENDS`] polls (best-effort; see the constant's
/// rationale). The committed memo additionally outlives its resend budget as a
/// responder: a duplicate `StateSnapshotAck` from the joiner (which re-acks
/// until it observes the commit) re-arms one more `JoinCommitted` send,
/// making joiner-side commit delivery effectively reliable for chunk N4.
#[cfg(feature = "hot-join")]
struct NPeerPostServe<T>
where
    T: Config,
{
    /// `true` = `JoinCommitted`, `false` = `JoinAborted`.
    committed: bool,
    /// The handle of the concluded attempt.
    handle: PlayerHandle,
    /// The activation frame `F` of the concluded attempt.
    frame: Frame,
    /// The snapshot frame `S = F - 1` (committed-case responder matching for
    /// duplicate `StateSnapshotAck`s).
    snapshot_frame: Frame,
    /// The joiner's address.
    joiner_addr: T::Address,
    /// The survivor fan-out set of the concluded attempt.
    survivors: std::collections::BTreeSet<T::Address>,
    /// Remaining announcer resends.
    resends_left: usize,
}

/// Survivor-side state for the single in-flight mesh join (chunk N3).
///
/// Created when a valid `ReactivateSlot{h, F}` directive arrives; cleared ONLY
/// by a matching `JoinCommitted` or `JoinAborted` from the directive's
/// coordinator (whose delivery the re-ack convergence loop makes reliable; see
/// [`NPEER_JOIN_LIFECYCLE_RESENDS`]). After the survivor acks
/// (`reopened == true`) it never self-reverts on a timeout guess — a
/// self-revert can race a commit it has not yet heard about, and the joiner's
/// protocol-level input acks would then let it prune `pending_output` for
/// inputs a re-frozen queue silently dropped (permanent silent desync). Nor
/// does it self-CLEAR on observed input progress — a live joiner legally feeds
/// the reopened queue before an eventual abort, so progress is not proof of a
/// commit. The only re-freeze paths are an explicit matching `JoinAborted`
/// (coordinator authority) and the normal graceful-drop machinery if the
/// joiner's endpoint later dies post-reopen.
#[cfg(feature = "hot-join")]
struct PendingReactivation<T>
where
    T: Config,
{
    /// The slot to reopen.
    handle: PlayerHandle,
    /// The activation frame `F` carried verbatim from the directive.
    frame: Frame,
    /// The coordinator that issued the directive. Lifecycle messages
    /// (`JoinCommitted`/`JoinAborted`) and duplicate directives must come from
    /// this address to match the attempt.
    coordinator_addr: T::Address,
    /// The joiner's address (the registry owner of `handle`).
    joiner_addr: T::Address,
    /// The slot's connection status captured immediately before any mutation
    /// (`disconnected == true`, `last_frame ==` the pre-reopen freeze frame).
    /// Restored verbatim on a matching `JoinAborted` after reopen.
    pre_freeze_status: ConnectionStatus,
    /// The slot's frozen value (queue `last_confirmed_input`) captured
    /// immediately before any mutation. Restored on a matching `JoinAborted`
    /// after reopen: the reopened queue's own tracked value is NOT a valid
    /// restore source, because any joiner input it confirmed before the abort
    /// overwrote it (see [`InputQueue::refreeze_with_value`]).
    ///
    /// [`InputQueue::refreeze_with_value`]: crate::__internal::InputQueue::refreeze_with_value
    pre_freeze_input: Option<T::Input>,
    /// Whether the slot has been reopened (and the ack sent). Set once the
    /// joiner endpoint reaches `Running`.
    reopened: bool,
}

/// Whether a reactivated-slot gossip re-seed also arms the per-slot merge
/// reactivation floor (session-33 round-2 review Finding 1).
///
/// The floor's `>= F - 1` re-drop theorem holds only in COMMITTED worlds, so
/// only commit-evidence callers (coordinator commit, survivor `JoinCommitted`
/// receipt, commit-evidence implied/local close) may arm it; the pre-commit
/// survivor reopen seeds only. An aborted attempt therefore never leaves a
/// floor behind, and the mesh's genuine `{disconnected, f0}` gossip
/// re-converges the slot — see [`UdpProtocol::arm_reactivation_floor`] for
/// the full argument and the stall this prevents.
#[cfg(feature = "hot-join")]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum FloorArming {
    /// Commit evidence in hand: seed the caches AND arm the floor at `F - 1`.
    CommitEvidence,
    /// Pre-commit reopen: seed the caches only (the pending-reactivation
    /// shield owns the attempt window; the floor stays unarmed so an abort
    /// converges).
    SeedOnly,
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
                npeer: None,
                npeer_post: None,
                npeer_next_serve_min_frame: Frame::NULL,
                pending_reactivation: None,
                npeer_closed_attempt_frames: BTreeMap::new(),
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

        // N-peer hot-join coordinator PAUSE (chunk N2): like the 2-peer pause
        // below, the simulation must not advance to a new frame while an N-peer
        // serve is open — the pause is what freezes the coordinator's last-sent
        // frame `L` and thereby caps every survivor's confirmed frame at `L`.
        // UNLIKE the 2-peer pause, rollback repair MUST still run: the serve's
        // wait-then-capture gate holds the snapshot until the coordinator's own
        // confirmed frame reaches `S = L` with no misprediction pending, and a
        // misprediction discovered DURING the wait (a survivor's late input for
        // a frame <= L differing from the prediction baked into saved state)
        // would otherwise never be repaired — the wait would deadlock into the
        // Phase-4 abort or, worse, a gate without the misprediction check would
        // serve a speculative state. The 2-peer arm below keeps its exact empty
        // return: its sole remote slot is frozen for the entire serve (no
        // prediction episodes), so no rollback can pend there.
        #[cfg(feature = "hot-join")]
        if self.hot_join.npeer.is_some() {
            return self.advance_frame_npeer_paused();
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

        // Connect-status nudge: while any remote slot is locally disconnected
        // but its drop is not yet mesh-agreed, every running remote endpoint
        // must keep gossiping our view even when input-idle — otherwise a
        // survivor capped at its prediction window with a fully-acked send
        // queue can never deliver the `disconnected` gossip and mesh agreement
        // (the release condition in `remote_slot_confirmed_bound`) becomes
        // unreachable. Computed before the endpoint poll below so the flag is
        // live on the poll that follows a detection; cleared the poll after
        // the mesh agrees. Allocation-free.
        let nudge_needed = self.connect_status_nudge_needed();
        for remote_endpoint in self.player_reg.remotes.values_mut() {
            remote_endpoint.set_connect_status_nudge(nudge_needed);
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
        // N-peer survivor drain (chunk N3): runs for EVERY hot-join-feature
        // session, not gated on `accept_hot_join` — survivors are plain P2P
        // sessions that never opted into serving. The handler itself rejects
        // directives on a serving coordinator (it owns its slots' lifecycle),
        // and drains are cheap no-ops on sessions that never receive N-peer
        // traffic (2-peer hosts/joiners, the coordinator itself).
        self.poll_npeer_survivor();
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
            // Already serving this handle on the N-peer arm: ignore the
            // duplicate request (the N-peer serve loop drives retransmission).
            if self
                .hot_join
                .npeer
                .as_ref()
                .is_some_and(|serve| serve.handle == handle)
            {
                continue;
            }

            // Arm selection (N2): the N-peer arm is taken iff at least one
            // running, non-reserved remote endpoint exists besides the joiner's
            // address (a survivor that must agree on the activation frame). An
            // empty survivor set falls through to the 2-peer flow below,
            // byte-identical to today.
            let survivors = self.npeer_survivor_addrs(&addr);
            if !survivors.is_empty() {
                self.try_open_npeer_serve(addr, handle, survivors);
                continue;
            }
            // One join at a time, mesh-wide: while an N-peer serve is open, no
            // 2-peer serve may open either. Unreachable for a 2-machine mesh
            // (an N-peer serve requires a survivor, and a 2-machine mesh has
            // none), so the 2-peer path is byte-identical; this only protects
            // the mixed case where every survivor died mid-serve and another
            // reserved handle's request races the open N-peer serve.
            if self.hot_join.npeer.is_some() {
                trace!(
                    "Ignoring 2-peer hot-join request for slot {} while an N-peer serve is open",
                    handle
                );
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
            // (last_frame = F-1). `confirmed_frame` reports NULL from here until
            // the joiner's first input lands: the freeze barrier folds the
            // rebuilt endpoint's default `{connected, NULL}` status cache into
            // the reactivated slot's bound (min(F-1, NULL) = NULL) — strictly
            // conservative, see `remote_slot_confirmed_bound`'s N==2 transient
            // windows. The next advance predicts handle h = RepeatLastConfirmed
            // (== the frozen default == same result as before), and the existing
            // misprediction -> rollback path corrects when the joiner's real
            // inputs arrive.
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
            // is handled. `confirmed_frame` reports NULL here (the freeze barrier
            // folds the rebuilt endpoint's default `{connected, NULL}` cache —
            // NULL < F-1 < F, see the reactivation comment above), so no frame
            // >= F is checksum-compared until reconciliation completes.
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

        // N-peer serve + post-serve lifecycle (chunk N2). No-ops unless an
        // N-peer serve (or its post-serve announcer) is live, so the 2-peer
        // phases above are unaffected.
        self.poll_npeer_host_serve();
        self.poll_npeer_post_serve();
    }

    /// Returns the **survivor set** for a prospective N-peer serve: the
    /// addresses of every running, non-reserved remote endpoint other than the
    /// joiner's. Spectators live in a separate registry and are never
    /// consulted. `BTreeSet` (and the `BTreeMap` walk) keeps iteration
    /// deterministic.
    #[cfg(feature = "hot-join")]
    fn npeer_survivor_addrs(
        &self,
        joiner_addr: &T::Address,
    ) -> std::collections::BTreeSet<T::Address> {
        let mut survivors = std::collections::BTreeSet::new();
        for (addr, endpoint) in self.player_reg.remotes.iter() {
            if addr == joiner_addr {
                continue;
            }
            if !endpoint.is_running() {
                continue;
            }
            if self.hot_join.endpoint_is_reserved(endpoint) {
                continue;
            }
            survivors.insert(addr.clone());
        }
        survivors
    }

    /// Opens an N-peer serve for `handle` requested by the joiner at `addr`,
    /// applying the fail-closed gates that make `S = L` (snapshot frame =
    /// survivor cap) provable. On any gate failure the request is dropped (the
    /// joiner re-requests every poll, so a transient failure self-heals; a
    /// configuration failure repeats its violation and the join can never
    /// complete — honest, never wrong).
    #[cfg(feature = "hot-join")]
    fn try_open_npeer_serve(
        &mut self,
        addr: T::Address,
        handle: PlayerHandle,
        survivors: std::collections::BTreeSet<T::Address>,
    ) {
        // One join at a time, mesh-wide (R6): a single coordinator-side N-peer
        // serve, and never concurrently with a 2-peer serve.
        if self.hot_join.npeer.is_some() || !self.hot_join.joining.is_empty() {
            trace!(
                "Ignoring N-peer hot-join request for slot {} while another serve is open",
                handle
            );
            return;
        }

        // Gate (R2a): `S = L = last_saved_frame` requires saved state at
        // exactly the last-sent frame, which only `SaveMode::EveryFrame`
        // guarantees. Under Sparse saving `last_saved` can trail `L` by up to
        // `max_prediction`, and serving the older frame would pick
        // `F = saved + 1 <= L` — a frame survivors may already have committed
        // frozen (an unrecoverable confirmed-history rewrite). Fail closed.
        if self.save_mode != SaveMode::EveryFrame {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::Configuration,
                "N-peer hot-join serve for slot {} requires SaveMode::EveryFrame (configured {:?}); ignoring the join request",
                handle,
                self.save_mode
            );
            return;
        }

        // Gate (R2b): the survivor cap is the minimum of the coordinator's
        // *gossiped* local last-input frames (`local_connect_status[local]
        // .last_frame`, stamped with input delay folded in), while the snapshot
        // can only be captured at `last_saved_frame`. The two coincide exactly
        // when every local slot's last-sent frame equals `last_saved_frame`
        // (zero input delay, normal advance cadence). With input delay `d > 0`
        // the gossiped frame runs `d` ahead of the simulation, the cap sits at
        // a frame the coordinator has not simulated, and no correct snapshot
        // exists — fail closed (checked dynamically so runtime delay changes
        // are caught too). A coordinator with no local players gossips no cap
        // at all (nothing pins survivors during the pause) — also fail closed.
        let snapshot_frame = self.sync_layer.last_saved_frame();
        if snapshot_frame.is_null() {
            // Nothing saved yet; the joiner will re-send. Skip this poll.
            return;
        }
        let mut local_handles = self.player_reg.local_player_handles_iter().peekable();
        if local_handles.peek().is_none() {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::Configuration,
                "N-peer hot-join serve for slot {} requires at least one local player on the coordinator (the pause caps survivors via the local slots' gossiped last-input frames); ignoring the join request",
                handle
            );
            return;
        }
        for local in local_handles {
            let last_sent = self
                .local_connect_status
                .get(local.as_usize())
                .map_or(Frame::NULL, |status| status.last_frame);
            if last_sent != snapshot_frame {
                report_violation!(
                    ViolationSeverity::Warning,
                    ViolationKind::Configuration,
                    "N-peer hot-join serve for slot {} requires the local slot {}'s last-sent frame ({}) to equal last_saved_frame ({}) — non-zero input delay (or a stalled save cadence) breaks the S = L identity; ignoring the join request",
                    handle,
                    local,
                    last_sent,
                    snapshot_frame
                );
                return;
            }
        }

        // Gate (R3): one-frame next-serve guard. After an abort, `(handle, F)`
        // must be unique per attempt so stale lifecycle messages from the
        // previous attempt are discriminable; require the coordinator to have
        // advanced past the aborted attempt's snapshot frame first. The joiner
        // keeps re-requesting, so this is a bounded delay of one frame.
        if !self.hot_join.npeer_next_serve_min_frame.is_null()
            && snapshot_frame <= self.hot_join.npeer_next_serve_min_frame
        {
            trace!(
                "Deferring N-peer hot-join serve for slot {}: last_saved {} has not advanced past the previous attempt's frame {}",
                handle,
                snapshot_frame,
                self.hot_join.npeer_next_serve_min_frame
            );
            return;
        }

        // The joiner endpoint must still exist to receive the serve traffic.
        if !self.player_reg.remotes.contains_key(&addr) {
            return;
        }

        // Gate (round-5 Finding 1): the served slot's pre-attempt freeze must
        // be mesh-converged AT THIS COORDINATOR before a serve may open.
        // Everything downstream — the snapshot's byte-truth at `S`, the bound
        // clamp's `(f0, v0)` uniformity, the survivors' floors and restores —
        // quantifies over a single agreed freeze, and the convergence
        // re-adjust is what creates it. Two fail-closed defers (the joiner
        // re-requests every poll, so both self-heal):
        // - a survivor still claims the slot CONNECTED: its freeze frame is
        //   unknown here, and once that survivor's slot reopens its gossip
        //   flips connected forever — the correction signal would be lost for
        //   the entire attempt era (a committed attempt then bakes the
        //   divergence in permanently);
        // - a survivor's DISCONNECTED claim sits BELOW this coordinator's own
        //   freeze: this coordinator's mine-down + gap re-simulation is owed,
        //   and a snapshot captured first would embed history no survivor
        //   serves. The very next `advance_frame` applies it (the claim has
        //   already merged), so this defers by about one tick.
        // A claim ABOVE the local freeze is fine: that survivor owes its own
        // re-adjust, which the directive acceptance gate on its side defers
        // until ITS fold converges. Deferral is trace-level (transient
        // network condition, like the R3 defer above), not a violation.
        //
        // NULL-freeze skip: a NULL local freeze (a build-time reserved slot
        // that was never occupied, or a drop with zero receipts) IS the
        // global minimum by definition — no peer's freeze can undercut it,
        // this coordinator's history for the slot is already the convergence
        // target, and for a never-occupied slot the survivors' claims stay
        // `{connected, NULL}` forever (there was no drop to gossip), so
        // waiting on them would wedge the build-time-reserved serve path.
        {
            let Some(local_freeze) = self
                .local_connect_status
                .get(handle.as_usize())
                .filter(|status| status.disconnected)
                .map(|status| status.last_frame)
            else {
                trace!(
                    "Deferring N-peer hot-join serve for slot {}: the slot is not locally frozen/disconnected",
                    handle
                );
                return;
            };
            if !local_freeze.is_null() {
                for survivor in &survivors {
                    let Some(endpoint) = self.player_reg.remotes.get(survivor) else {
                        continue;
                    };
                    let claim = endpoint.peer_connect_status(handle);
                    if !claim.disconnected || claim.last_frame < local_freeze {
                        trace!(
                            "Deferring N-peer hot-join serve for slot {}: survivor {:?}'s freeze claim {:?} has not converged with the local freeze {} (re-requested next poll)",
                            handle,
                            survivor,
                            claim,
                            local_freeze
                        );
                        return;
                    }
                }
            }
        }

        let activation_frame = safe_frame_add!(snapshot_frame, 1, "try_open_npeer_serve");

        // A concluded-serve memo for an older attempt is superseded the moment
        // a new serve opens (one join at a time mesh-wide makes them
        // unambiguous, and the joiner of the previous attempt has had its
        // announcer window).
        self.hot_join.npeer_post = None;

        let pending_acks = survivors.clone();
        self.hot_join.npeer = Some(NPeerServe {
            handle,
            joiner_addr: addr.clone(),
            snapshot_frame,
            activation_frame,
            snapshot: None,
            joiner_acked: false,
            survivors,
            pending_acks,
            polls_since_serve: 0,
        });

        self.event_queue
            .push_back(FortressEvent::JoinRequested { handle, addr });
    }

    /// Returns `true` while a freeze-frame convergence re-adjust whose forced
    /// re-simulation would start at or below `snapshot_frame` is OWED but not
    /// yet applied — i.e. the endpoint claim merge has outrun
    /// [`update_player_disconnects`](Self::update_player_disconnects), which
    /// runs only inside `advance_frame` (session-33 round-5 review Finding 1,
    /// coordinator sibling).
    ///
    /// This is a read-only dry-run of the re-adjust trigger: for each remote
    /// slot, fold the running non-reserved endpoints' claims exactly like the
    /// real fold; the re-adjust fires when no folded endpoint still reports
    /// the slot connected AND the slot is either locally connected (a
    /// propagated first freeze) or locally frozen ABOVE the folded minimum (a
    /// mine-down is owed). The `<= snapshot_frame` reach term scopes it to
    /// rewrites that would invalidate a snapshot at `snapshot_frame`; the
    /// serve poll consumes it to defer a capture (pre-capture) or abort the
    /// serve (post-capture). Allocation-free; called once per serve poll.
    #[cfg(feature = "hot-join")]
    fn npeer_owed_freeze_readjust_at_or_below(&self, snapshot_frame: Frame) -> bool {
        for handle_idx in 0..self.num_players {
            let handle = PlayerHandle::new(handle_idx);
            if !matches!(
                self.player_reg.handles.get(&handle),
                Some(PlayerType::Remote(_))
            ) {
                continue;
            }
            let Some(status) = self.local_connect_status.get(handle_idx) else {
                continue;
            };
            let mut queue_connected = true;
            let mut queue_min_confirmed = Frame::new(i32::MAX);
            let mut any_folded = false;
            for endpoint in self.player_reg.remotes.values() {
                if !endpoint.is_running() {
                    continue;
                }
                if self.hot_join.endpoint_is_reserved(endpoint) {
                    continue;
                }
                let claim = endpoint.peer_connect_status(handle);
                any_folded = true;
                queue_connected = queue_connected && !claim.disconnected;
                queue_min_confirmed = std::cmp::min(queue_min_confirmed, claim.last_frame);
            }
            if !any_folded || queue_connected {
                continue;
            }
            let local_connected = !status.disconnected;
            if local_connected {
                queue_min_confirmed = std::cmp::min(queue_min_confirmed, status.last_frame);
            }
            let readjust_owed = local_connected || status.last_frame > queue_min_confirmed;
            if readjust_owed
                && safe_frame_add!(
                    queue_min_confirmed,
                    1,
                    "P2PSession::npeer_owed_freeze_readjust_at_or_below"
                ) <= snapshot_frame
            {
                return true;
            }
        }
        false
    }

    /// Drives the open N-peer serve one poll: directive fan-out + ack
    /// collection, the wait-then-capture snapshot gate, joiner retransmit, the
    /// commit barrier, and the Phase-4 timeout.
    #[cfg(feature = "hot-join")]
    fn poll_npeer_host_serve(&mut self) {
        // Take the serve out so `self` stays borrowable; put it back unless the
        // serve concluded this poll.
        let Some(mut serve) = self.hot_join.npeer.take() else {
            return;
        };
        serve.polls_since_serve = serve.polls_since_serve.saturating_add(1);

        // Prune survivors that died mid-join (their endpoints left Running via
        // the normal disconnect machinery, which froze their slots): the join
        // must not wait on a dead peer's ack. They stay in `survivors` so the
        // lifecycle fan-out still attempts delivery (a no-op on a dead
        // endpoint).
        serve.pending_acks.retain(|addr| {
            self.player_reg
                .remotes
                .get(addr)
                .is_some_and(UdpProtocol::is_running)
        });

        // Directive fan-out: re-send `ReactivateSlot{h, F}` to every survivor
        // that has not acked (the serve-cadence retransmit; loss tolerant).
        for addr in &serve.pending_acks {
            if let Some(endpoint) = self.player_reg.remotes.get_mut(addr) {
                endpoint.send_reactivate_slot(serve.handle.as_usize(), serve.activation_frame);
            }
        }

        // Collect survivor acks. Drained from every survivor of the attempt
        // (not only pending ones) so duplicate acks cannot rot in the
        // single-slot protocol state.
        for addr in &serve.survivors {
            let Some(endpoint) = self.player_reg.remotes.get_mut(addr) else {
                continue;
            };
            let Some(ack) = endpoint.take_received_reactivate_slot_ack() else {
                continue;
            };
            if ack.handle == serve.handle.as_usize() && ack.frame == serve.activation_frame {
                serve.pending_acks.remove(addr);
            } else {
                report_violation!(
                    ViolationSeverity::Warning,
                    ViolationKind::NetworkProtocol,
                    "Ignoring stale ReactivateSlotAck for slot {} frame {} from {:?} (serving slot {} frame {})",
                    ack.handle,
                    ack.frame,
                    addr,
                    serve.handle,
                    serve.activation_frame
                );
            }
        }

        // Freeze-convergence invalidation (session-33 round-5 review
        // Finding 1, coordinator sibling): endpoint claims merge during the
        // poll, the capture gate / joiner ack / commit barrier all run HERE
        // (also during the poll), but the convergence re-adjust those claims
        // demand runs only inside `advance_frame` — so a lowered freeze
        // claim's arrival poll can satisfy the capture gate (the fold sees
        // mesh agreement instantly) while the state rewrite it owes is still
        // pending. Dry-run the re-adjust trigger every poll:
        // - owed and NOT yet captured: simply defer the capture this poll (a
        //   delay, not a protocol change — the next `advance_frame` applies
        //   the re-adjust, the paused arm repairs, and the gate's
        //   misprediction term holds the capture until the repair is done);
        // - owed and already captured (an N>=4 relay lowering landing
        //   mid-serve): ABORT fail-closed, before the joiner ack is consumed
        //   and before the commit barrier. Never recapture at the same `S`:
        //   the joiner applies the FIRST snapshot it receives (duplicates
        //   are idempotent) and acks by FRAME, so stale bytes already in
        //   flight are indistinguishable from a recapture. The joiner
        //   retries; the R3 guard forces a strictly later `(handle, F)`.
        let owed_readjust = self.npeer_owed_freeze_readjust_at_or_below(serve.snapshot_frame);
        if owed_readjust && serve.snapshot.is_some() {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::FrameSync,
                "N-peer hot-join serve for slot {} aborted: a freeze convergence re-adjust at or below the captured snapshot frame {} is owed (a same-frame recapture would be indistinguishable from the stale bytes already in flight)",
                serve.handle,
                serve.snapshot_frame
            );
            self.abort_npeer_serve(serve);
            return;
        }

        // Wait-then-capture (R1): the snapshot is captured only once the
        // coordinator's own confirmed frame has reached `S` AND no misprediction
        // is pending — i.e. saved state at `S` is a fully-confirmed,
        // non-speculative state. Repair of a pending misprediction happens in
        // `advance_frame`'s N-peer paused arm; this gate simply waits for it.
        //
        // Achievability bound (honest, fail-closed — review minor m3): the
        // gate transitively needs every survivor's gossiped claims to cover
        // `S`, and a survivor that hits its prediction cap goes gossip-idle
        // with whatever claims it last sent. If cross-peer delivery lag
        // exceeds the prediction window, those pre-cap claims never reach
        // `S`, the gate starves, and EVERY attempt ends in the Phase-4 abort
        // below (which fires a Warning violation per attempt, so operators
        // can see why joins never land). That is the honest outcome — abort
        // un-pauses and the R3 retry refreshes the claims by about one frame
        // per attempt — never a wrong `F`.
        if serve.snapshot.is_none()
            && !owed_readjust
            && self.confirmed_frame() >= serve.snapshot_frame
            && self
                .sync_layer
                .check_simulation_consistency(self.disconnect_frame)
                .is_null()
        {
            if self.sync_layer.last_saved_frame() == serve.snapshot_frame {
                match crate::sessions::hot_join::capture_snapshot_with_max_wire_bytes(
                    &self.sync_layer,
                    serve.snapshot_frame,
                    self.num_players,
                    self.hot_join.max_snapshot_wire_bytes,
                ) {
                    Ok(Some(snapshot)) => serve.snapshot = Some(snapshot),
                    Ok(None) => {
                        // No valid saved state at S (should not happen under the
                        // EveryFrame gate); retry next poll, bounded by Phase 4.
                    },
                    Err(e) => {
                        report_violation!(
                            ViolationSeverity::Error,
                            ViolationKind::InternalError,
                            "Failed to capture N-peer hot-join snapshot at frame {}: {}",
                            serve.snapshot_frame,
                            e
                        );
                    },
                }
            } else {
                // The pause holds last_saved fixed, so this indicates the serve
                // opened against a moving frame counter — fail loudly and let
                // Phase 4 abort rather than serve a wrong F.
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::FrameSync,
                    "N-peer hot-join serve for slot {}: last_saved_frame {} moved off the pinned snapshot frame {} while paused",
                    serve.handle,
                    self.sync_layer.last_saved_frame(),
                    serve.snapshot_frame
                );
            }
        }

        // Joiner snapshot retransmit + ack collection.
        if let Some(endpoint) = self.player_reg.remotes.get_mut(&serve.joiner_addr) {
            if let Some(snapshot) = &serve.snapshot {
                endpoint.send_state_snapshot(snapshot.clone());
            }
            if let Some(acked) = endpoint.take_received_snapshot_ack() {
                if serve.snapshot.is_some() && acked == serve.snapshot_frame {
                    serve.joiner_acked = true;
                } else {
                    report_violation!(
                        ViolationSeverity::Warning,
                        ViolationKind::NetworkProtocol,
                        "Ignoring N-peer hot-join ack for frame {} (serving frame {}) on slot {}",
                        acked,
                        serve.snapshot_frame,
                        serve.handle
                    );
                }
            }
        }

        // Commit barrier: the joiner has the snapshot AND every (live) survivor
        // has reopened-and-acked. Un-pausing earlier would lift the cap and let
        // an un-reopened survivor commit `F` frozen (unrecoverable).
        if serve.joiner_acked && serve.pending_acks.is_empty() {
            self.commit_npeer_serve(serve);
            return;
        }

        // Phase-4 timeout: liveness bound only (never a safety lever) — abort
        // and let the joiner retry.
        if serve.polls_since_serve >= self.hot_join.serve_timeout_polls {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::NetworkProtocol,
                "N-peer hot-join serve for slot {} timed out after {} polls (joiner_acked={}, pending survivor acks={}); aborting (slot stays reserved/frozen, coordinator resumes)",
                serve.handle,
                self.hot_join.serve_timeout_polls,
                serve.joiner_acked,
                serve.pending_acks.len()
            );
            self.abort_npeer_serve(serve);
            return;
        }

        self.hot_join.npeer = Some(serve);
    }

    /// Concludes an N-peer serve successfully: reactivates the local slot at
    /// `F`, announces `JoinCommitted` to the joiner and every survivor, and
    /// un-pauses (the serve is dropped, not restored).
    #[cfg(feature = "hot-join")]
    fn commit_npeer_serve(&mut self, serve: NPeerServe<T>) {
        let NPeerServe {
            handle,
            joiner_addr,
            snapshot_frame,
            activation_frame,
            survivors,
            ..
        } = serve;

        // Pre-commit frozen bound, read before the status flip below: needed
        // to arm the pre-activation serving floor.
        let frozen_bound = self
            .local_connect_status
            .get(handle.as_usize())
            .map_or(Frame::NULL, |status| status.last_frame);

        // Reactivate the local slot exactly like the 2-peer Phase 3: unfreeze +
        // reposition the queue at F and mark it connected with
        // last_frame = F - 1.
        if let Err(e) = self
            .sync_layer
            .reactivate_player_at_frame(handle, activation_frame)
        {
            // Should-never-happen (the handle was validated at open). Abort the
            // attempt rather than half-commit.
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::InternalError,
                "Failed to reactivate N-peer hot-join slot {} at frame {}: {}; aborting the attempt",
                handle,
                activation_frame,
                e
            );
            self.abort_npeer_serve(NPeerServe {
                handle,
                joiner_addr,
                snapshot_frame,
                activation_frame,
                snapshot: None,
                joiner_acked: true,
                pending_acks: std::collections::BTreeSet::new(),
                survivors,
                polls_since_serve: 0,
            });
            return;
        }
        // Arm the pre-activation serving floor (defense-in-depth on the
        // coordinator: the wait gate + the paused-arm spectator flush make
        // sub-F requests structurally unreachable here, but the floor keeps
        // the contract uniform across every reopen site).
        if let Err(e) =
            self.sync_layer
                .set_reactivation_floor(handle, activation_frame, frozen_bound)
        {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::InternalError,
                "Failed to arm the reactivation floor for N-peer slot {}: {}",
                handle,
                e
            );
        }
        if let Some(status) = self.local_connect_status.get_mut(handle.as_usize()) {
            status.disconnected = false;
            status.last_frame =
                safe_frame_sub!(activation_frame, 1, "P2PSession::commit_npeer_serve");
        } else {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::InternalError,
                "N-peer hot-join slot {} has no connection status entry at commit",
                handle
            );
        }
        // Anchor the reactivated slot's prediction at F, mirroring the 2-peer
        // Phase 3. Here `F == current_frame` (the pause pinned
        // current = S + 1 = F), so the next advance's `adjust_gamestate`
        // early-returns into `reset_prediction()` — no frames are re-simulated
        // (none were simulated at or past F; the design's invariant-7 caveat),
        // but every queue's prediction state is re-anchored so the joiner's
        // first real input at F is accepted and compared.
        self.disconnect_frame = if self.disconnect_frame.is_null() {
            activation_frame
        } else {
            std::cmp::min(self.disconnect_frame, activation_frame)
        };

        // Un-stick every endpoint's cached (sticky-disconnected) view of the
        // reactivated slot — see `reset_reactivated_slot_gossip`. This IS the
        // commit, so the merge reactivation floor arms here.
        self.reset_reactivated_slot_gossip(
            handle,
            activation_frame,
            &joiner_addr,
            FloorArming::CommitEvidence,
        );

        // Join complete: the slot is no longer reserved; the coordinator
        // un-pauses (npeer stays None).
        self.hot_join.reserved_slots.remove(&handle);
        self.hot_join.npeer_post = Some(NPeerPostServe {
            committed: true,
            handle,
            frame: activation_frame,
            snapshot_frame,
            joiner_addr: joiner_addr.clone(),
            survivors,
            resends_left: NPEER_JOIN_LIFECYCLE_RESENDS,
        });

        self.event_queue.push_back(FortressEvent::PeerJoined {
            handle,
            addr: joiner_addr,
        });
    }

    /// Concludes an N-peer serve unsuccessfully: announces `JoinAborted` to the
    /// joiner and every survivor, keeps the slot reserved/frozen, clears the
    /// joiner endpoint's stale `pending_output` (see
    /// [`abort_hot_join_serve`](Self::abort_hot_join_serve) for why), arms the
    /// one-frame next-serve guard, and un-pauses.
    #[cfg(feature = "hot-join")]
    fn abort_npeer_serve(&mut self, serve: NPeerServe<T>) {
        let NPeerServe {
            handle,
            joiner_addr,
            snapshot_frame,
            activation_frame,
            survivors,
            ..
        } = serve;

        if let Some(endpoint) = self.player_reg.remotes.get_mut(&joiner_addr) {
            endpoint.clear_pending_output();
        }

        // R3: `(handle, F)` must be unique per attempt — the next serve must
        // pick a strictly later frame.
        self.hot_join.npeer_next_serve_min_frame =
            std::cmp::max(self.hot_join.npeer_next_serve_min_frame, snapshot_frame);

        self.hot_join.npeer_post = Some(NPeerPostServe {
            committed: false,
            handle,
            frame: activation_frame,
            snapshot_frame,
            joiner_addr,
            survivors,
            resends_left: NPEER_JOIN_LIFECYCLE_RESENDS,
        });
        // npeer stays None: the coordinator un-pauses; the slot stays in
        // `reserved_slots` so the joiner can retry.
    }

    /// Aborts the open N-peer serve if it is for `handle` (the joiner-endpoint
    /// `Event::Disconnected` fast path, mirroring the 2-peer
    /// [`abort_hot_join_serve`](Self::abort_hot_join_serve)). Survivors that
    /// already reopened still receive `JoinAborted` via the announcer.
    #[cfg(feature = "hot-join")]
    fn abort_npeer_serve_for_handle(&mut self, handle: PlayerHandle) {
        if self
            .hot_join
            .npeer
            .as_ref()
            .is_some_and(|serve| serve.handle == handle)
        {
            if let Some(serve) = self.hot_join.npeer.take() {
                report_violation!(
                    ViolationSeverity::Warning,
                    ViolationKind::NetworkProtocol,
                    "N-peer hot-join serve for slot {} aborted: the joiner endpoint disconnected mid-serve",
                    handle
                );
                self.abort_npeer_serve(serve);
            }
        }
    }

    /// Drives the post-serve lifecycle announcer one poll: re-sends the
    /// concluded attempt's `JoinCommitted`/`JoinAborted` while the resend
    /// budget lasts, and responds to stragglers past the budget — a duplicate
    /// joiner `StateSnapshotAck` (committed case) or a survivor's
    /// `ReactivateSlotAck` convergence ping (either case) proves the sender
    /// has not yet observed the lifecycle outcome and re-arms one resend.
    /// Together with the survivor-side re-ack loop this makes lifecycle
    /// delivery reliable-until-converged for every reopened participant; the
    /// memo lives until the next N-peer serve supersedes it.
    #[cfg(feature = "hot-join")]
    fn poll_npeer_post_serve(&mut self) {
        let Some(mut post) = self.hot_join.npeer_post.take() else {
            return;
        };

        // Committed-case responder: a duplicate StateSnapshotAck from the
        // joiner proves it has not yet observed the commit. Only drained while
        // no 2-peer serve is open for the same endpoint (the 2-peer Phase 3
        // owns that drain during its own serves).
        if post.committed
            && self
                .hot_join
                .joining
                .values()
                .all(|serve| serve.addr != post.joiner_addr)
        {
            if let Some(endpoint) = self.player_reg.remotes.get_mut(&post.joiner_addr) {
                if let Some(acked) = endpoint.take_received_snapshot_ack() {
                    if acked == post.snapshot_frame {
                        post.resends_left = post.resends_left.max(1);
                    }
                }
            }
        }

        // Survivor responder: a reopened survivor keeps re-acking until it
        // hears the lifecycle outcome; answer a matching straggler ack with
        // one more resend.
        for addr in &post.survivors {
            let Some(endpoint) = self.player_reg.remotes.get_mut(addr) else {
                continue;
            };
            let Some(ack) = endpoint.take_received_reactivate_slot_ack() else {
                continue;
            };
            if ack.handle == post.handle.as_usize() && ack.frame == post.frame {
                post.resends_left = post.resends_left.max(1);
            }
        }

        if post.resends_left > 0 {
            post.resends_left -= 1;
            let handle = post.handle.as_usize();
            let frame = post.frame;
            let committed = post.committed;
            // Joiner first, then survivors in deterministic order.
            let mut targets: Vec<&T::Address> = Vec::with_capacity(post.survivors.len() + 1); // alloc-bound: bounded by the registry-sized survivor set + 1.
            targets.push(&post.joiner_addr);
            targets.extend(post.survivors.iter());
            for addr in targets {
                if let Some(endpoint) = self.player_reg.remotes.get_mut(addr) {
                    if committed {
                        endpoint.send_join_committed(handle, frame);
                    } else {
                        endpoint.send_join_aborted(handle, frame);
                    }
                }
            }
        }

        self.hot_join.npeer_post = Some(post);
    }

    /// Re-seeds every remote endpoint's cached connect-status view of a
    /// reactivated slot to `{connected, F - 1}` — optionally arming the
    /// per-slot reactivation floor against stale in-flight `disconnected`
    /// gossip — and bootstraps the JOINER endpoint's caches for every OTHER
    /// slot. See
    /// [`UdpProtocol::seed_peer_connect_status_for_reactivation`] for why the
    /// out-of-band un-stick is required (the gossip merge is deliberately
    /// sticky-disconnected, so a committed reactivation can never resurrect
    /// the cached views through gossip alone — `update_player_disconnects`
    /// would otherwise re-apply the drop on the freshly reopened slot
    /// forever), and
    /// [`UdpProtocol::seed_peer_connect_status_for_joiner_bootstrap`] for why
    /// the rebuilt joiner endpoint's default `{connected, NULL}` caches must
    /// not enter the folds raw (they pin this session's confirmed frame at
    /// `NULL` until the joiner's first own gossip — the post-commit
    /// mesh-wide dip).
    ///
    /// `arming` selects whether the floor arms too — commit-evidence callers
    /// only; see [`FloorArming`] and [`UdpProtocol::arm_reactivation_floor`].
    #[cfg(feature = "hot-join")]
    fn reset_reactivated_slot_gossip(
        &mut self,
        handle: PlayerHandle,
        activation_frame: Frame,
        joiner_addr: &T::Address,
        arming: FloorArming,
    ) {
        let seeded_last_frame = safe_frame_sub!(
            activation_frame,
            1,
            "P2PSession::reset_reactivated_slot_gossip"
        );
        for (addr, endpoint) in self.player_reg.remotes.iter_mut() {
            endpoint.seed_peer_connect_status_for_reactivation(handle, seeded_last_frame);
            if arming == FloorArming::CommitEvidence {
                endpoint.arm_reactivation_floor(handle, seeded_last_frame);
            }
            if addr != joiner_addr {
                continue;
            }
            // Joiner-endpoint bootstrap for the other slots: live slots are
            // claimed through at most `S = F - 1` (what the acked snapshot
            // provably covers), dropped slots keep this session's agreed
            // frozen view (claiming them connected would block their
            // mesh-agreed exclusion from the confirmed fold and pin this
            // session at the old freeze frame).
            for (idx, local) in self.local_connect_status.iter().enumerate() {
                let bootstrap_handle = PlayerHandle::new(idx);
                if bootstrap_handle == handle {
                    continue;
                }
                let status = if local.disconnected {
                    *local
                } else {
                    ConnectionStatus {
                        disconnected: false,
                        last_frame: std::cmp::min(local.last_frame, seeded_last_frame),
                    }
                };
                endpoint.seed_peer_connect_status_for_joiner_bootstrap(bootstrap_handle, status);
            }
        }
    }

    /// The N-peer coordinator's paused `advance_frame` body: never advances to
    /// a new frame (the pause is the survivor cap), but **does** run rollback
    /// repair so the serve's wait-then-capture gate can be satisfied — a
    /// misprediction for a frame `<= S` discovered during the wait is repaired
    /// here (the returned requests carry the rollback's
    /// `LoadGameState`/`AdvanceFrame`/`SaveGameState` sequence), after which
    /// saved state at `S` is fully confirmed and servable.
    ///
    /// Confirmed inputs are still streamed to spectators (and the replay
    /// recorder) while paused: the commit reactivates the slot at `F` with its
    /// pre-`F` ring history blanked, so any spectator frame still owed below
    /// `F - 1` must be flushed *before* the commit — while the slot's frozen
    /// branch can still serve it.
    #[cfg(feature = "hot-join")]
    fn advance_frame_npeer_paused(&mut self) -> FortressResult<RequestVec<T>> {
        let mut requests = RequestVec::<T>::new();

        // Inputs arriving for frames the paused coordinator never requested
        // cannot create mispredictions (prediction episodes only cover
        // requested frames), so this repairs strictly pre-pause speculation.
        let confirmed_frame = self.confirmed_frame();
        let first_incorrect = self
            .sync_layer
            .check_simulation_consistency(self.disconnect_frame);
        if !first_incorrect.is_null() {
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

        // Spectator/replay flush (no input discard: `set_last_confirmed_frame`
        // is deliberately NOT called while paused, so the rollback window and
        // the snapshot's source data stay intact).
        self.send_confirmed_inputs_to_spectators(confirmed_frame)?;
        self.record_confirmed_inputs(confirmed_frame);

        Ok(requests)
    }

    /// Survivor side of N-peer hot-join (chunk N3): drains and validates
    /// `ReactivateSlot` directives, progresses the pending reopen (rearm the
    /// joiner endpoint → wait `Running` → reopen at `F` → ack), and applies the
    /// coordinator's `JoinCommitted`/`JoinAborted` lifecycle.
    #[cfg(feature = "hot-join")]
    fn poll_npeer_survivor(&mut self) {
        // Drain everything first (deterministic BTreeMap order), releasing the
        // endpoint borrows before any session mutation. Draining unconditionally
        // (even messages this session will reject) keeps the single-slot
        // protocol state from rotting.
        let mut directives: Vec<(T::Address, crate::network::messages::ReactivateSlot)> =
            Vec::new(); // alloc-bound: at most one buffered directive per remote endpoint (registry-sized).
        let mut committed: Vec<(T::Address, crate::network::messages::JoinCommitted)> = Vec::new(); // alloc-bound: at most one per remote endpoint (registry-sized).
        let mut aborted: Vec<(T::Address, crate::network::messages::JoinAborted)> = Vec::new(); // alloc-bound: at most one per remote endpoint (registry-sized).
        for (addr, endpoint) in self.player_reg.remotes.iter_mut() {
            if let Some(directive) = endpoint.take_received_reactivate_slot() {
                directives.push((addr.clone(), directive));
            }
            if let Some(body) = endpoint.take_received_join_committed() {
                committed.push((addr.clone(), body));
            }
            if let Some(body) = endpoint.take_received_join_aborted() {
                aborted.push((addr.clone(), body));
            }
        }

        for (sender, directive) in directives {
            self.handle_reactivate_directive(&sender, &directive);
        }
        self.progress_pending_reactivation();
        for (sender, body) in committed {
            self.handle_join_committed_directive(&sender, &body);
        }
        for (sender, body) in aborted {
            self.handle_join_aborted_directive(&sender, &body);
        }

        // Lifecycle convergence ping: while reopened-and-pending, keep re-acking
        // the coordinator every poll. The coordinator's post-serve responder
        // answers a stray (post-conclusion) ack with one more
        // `JoinCommitted`/`JoinAborted`, so lifecycle delivery to a reopened
        // survivor is reliable-until-converged — the survivor itself never
        // guesses the outcome (a guess could race the true lifecycle message
        // into a permanent silent desync; observed input progress is NOT proof
        // of a commit, because a live joiner legally feeds the reopened queue
        // between the reopen and an eventual abort).
        if let Some(pending) = &self.hot_join.pending_reactivation {
            if pending.reopened {
                let handle = pending.handle.as_usize();
                let frame = pending.frame;
                let coordinator = pending.coordinator_addr.clone();
                if let Some(endpoint) = self.player_reg.remotes.get_mut(&coordinator) {
                    endpoint.send_reactivate_slot_ack(handle, frame);
                }
            }
        }
    }

    /// Records that an N-peer reactivation attempt `(handle, frame)` was
    /// CLOSED on this survivor (lifecycle close, implied close, or local
    /// joiner-death close), arming the per-handle stale-directive guard. See
    /// [`HotJoinState::npeer_closed_attempt_frames`].
    #[cfg(feature = "hot-join")]
    fn record_closed_npeer_attempt(&mut self, handle: PlayerHandle, frame: Frame) {
        self.hot_join
            .npeer_closed_attempt_frames
            .entry(handle)
            .and_modify(|closed| *closed = std::cmp::max(*closed, frame))
            .or_insert(frame);
    }

    /// Restores a REOPENED slot to its captured pre-reopen reserved state —
    /// the `JoinAborted` restore, shared by the lifecycle handler, the
    /// implied-abort close (a strictly-newer same-coordinator directive with
    /// no commit evidence), and the local joiner-endpoint-death close.
    ///
    /// The frozen value is restored from the directive-time capture — the
    /// reopened queue's own tracked value may already hold a leaked joiner
    /// input (see [`PendingReactivation::pre_freeze_input`]) — the connection
    /// status is restored verbatim to the pre-reopen freeze point, the slot
    /// re-enters `reserved_slots` (the rearmed joiner endpoint must stay
    /// excluded from the confirmed/disconnect folds), and a forced
    /// re-simulation from `F` is armed so any speculative frame that embedded
    /// the aborted attempt's real inputs is recomputed with the restored
    /// frozen value.
    #[cfg(feature = "hot-join")]
    fn restore_pre_reopen_frozen_state(
        &mut self,
        handle: PlayerHandle,
        frame: Frame,
        pre_freeze_status: ConnectionStatus,
        pre_freeze_input: Option<T::Input>,
    ) {
        if let Err(e) = self
            .sync_layer
            .refreeze_player_with_value(handle, pre_freeze_input)
        {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::InternalError,
                "Failed to re-freeze N-peer slot {} on attempt close: {}",
                handle,
                e
            );
        }
        if let Some(status) = self.local_connect_status.get_mut(handle.as_usize()) {
            *status = pre_freeze_status;
        } else {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::InternalError,
                "N-peer slot {} has no connection status entry at the pre-reopen restore",
                handle
            );
        }
        self.hot_join.reserved_slots.insert(handle);

        // Speculative frames at/past F may have been simulated with the
        // joiner's REAL inputs (accepted by the reopened queue before the
        // close) — values no other survivor will confirm. Force a
        // re-simulation from F so every such frame is recomputed with the
        // restored frozen value. (If nothing at/past F was simulated, the next
        // advance's `adjust_gamestate` early-returns into a prediction reset —
        // harmless.)
        self.disconnect_frame = if self.disconnect_frame.is_null() {
            frame
        } else {
            std::cmp::min(self.disconnect_frame, frame)
        };
    }

    /// Commit-evidence read for discriminating an unheard reopened attempt's
    /// outcome (the cap theorem): returns `true` iff this session holds
    /// evidence that attempt `(handle, frame)` COMMITTED. Two legs, each
    /// individually sound:
    ///
    /// 1. **Confirmed history:**
    ///    `max(confirmed_frame, last_confirmed_frame) >= F`. While this
    ///    survivor holds the reopened pending, the bound clamp in
    ///    [`remote_slot_confirmed_bound`](Self::remote_slot_confirmed_bound)
    ///    folds every disconnected gossip claim for the slot at the seeded
    ///    `F - 1`, so in an ABORTED world the confirmed fold cannot cross
    ///    `F` while the attempt is unresolved: raising any live peer's slot
    ///    bound to `>= F` requires receiving that peer's inputs at `>= F`,
    ///    and every such validated `Input` packet first merges that peer's
    ///    claim for this slot — `{disconnected, f0}` from every participant
    ///    that has CONCLUDED the attempt (never-reopened peers kept it,
    ///    reopened peers restore it once the abort reaches them, the
    ///    coordinator never left it pre-commit) — engaging the clamp. A
    ///    pre-commit joiner input leak therefore cannot satisfy this leg
    ///    while any live peer's DISCONNECTED claim is folded, or any folded
    ///    connected claim is stale at `<= F - 1` (a starved still-pending
    ///    survivor's seed or a contract-honoring joiner's self-claim, which
    ///    pins the min raw) — the leg fires only when the world committed
    ///    (live CONNECTED gossip `>= F` corroborates the leak: the blackout
    ///    shape where leg 2 has nothing and this leg is load-bearing), or
    ///    in the coordinator-dead(-or-pruned) shape where EVERY folded live
    ///    peer is itself a still-pending reopened survivor whose
    ///    leak-raised connected claim folds raw (the everyone-else-dead
    ///    corner is its degenerate sub-shape, not the whole region). In the
    ///    coordinator-DEAD variant the leaked closers commit-arm
    ///    CONSISTENTLY with each other — living counterparts exist but
    ///    agree (any starved/restored peer's folded claim would instead
    ///    pin/clamp everyone below `F` and all closes abort-arm); only the
    ///    asymmetric coordinator-PRUNED variant (alive and folded in
    ///    another survivor, link-dead toward this one) has a living
    ///    divergence counterpart, and it additionally needs a full
    ///    disconnect-timeout link death plus sustained suppression of the
    ///    re-ack-driven abort re-delivery. All variants are bounded by the
    ///    chunk-N4 joiner contract (no self-claim past `F - 1` pre-commit;
    ///    teardown on coordinator loss). The `max` guards the documented
    ///    confirmed-frame dip (which can only UNDERSTATE evidence — leg 2
    ///    is the robustness backstop); F-sanity (`F > max(frozen bound,
    ///    confirmed, last_confirmed)`, enforced at directive acceptance AND
    ///    re-checked at both reopen sites — a stale-`F` reopen is cancelled
    ///    fail-closed, see
    ///    [`npeer_activation_frame_is_sane`](Self::npeer_activation_frame_is_sane))
    ///    keeps pre-attempt and pre-reopen history from pre-satisfying it:
    ///    evidence is only ever consulted for a REOPENED pending attempt,
    ///    and every reopened pending began its window with
    ///    `max(confirmed, last_confirmed) < F`.
    /// 2. **Gossip freeze-frame (session-33 round-2 review Finding 2): any
    ///    remote endpoint's cached DISCONNECTED claim for `handle` with
    ///    `last_frame >= F`** — some peer FROZE the slot at or past the
    ///    activation frame, which only the committed era's re-drop can
    ///    produce. Soundness (induction over every freeze source in an
    ///    uncommitted world): an abort restore freezes at the pre-attempt
    ///    `f0 <= S = F - 1`; a peer that never reopened keeps `f0`; a
    ///    user-initiated `disconnect_player`/`remove_player` against the
    ///    in-flight joiner cannot freeze a reopened slot mid-attempt (the
    ///    public entry points close the attempt first — see
    ///    [`close_reopened_pending_before_user_disconnect`](Self::close_reopened_pending_before_user_disconnect),
    ///    the session-33 round-3 induction-gap closure); the only other way
    ///    a reopened slot re-freezes is a commit-evidence close, which
    ///    itself requires this evidence — so no FIRST
    ///    `{disconnected, >= F}` claim can ever arise in an aborted world,
    ///    leaks included. This leg is what saves an input-starved survivor
    ///    (zero joiner inputs AND zero `JoinCommitted` received) from
    ///    misclassifying a committed attempt as aborted once any committed
    ///    peer's re-drop gossip reaches it — a restore to `(f0, v0)` there
    ///    would diverge its frozen history from the committed peers' real
    ///    re-drop and leave its `f0` claims permanently floor-filtered.
    ///
    /// **Deliberately NOT evidence — input-receipt terms** (the local
    /// `last_frame` for the slot, or another peer's CONNECTED claim
    /// `>= F`): a live joiner legally feeds reopened queues between the
    /// reopen and an eventual abort (see `poll_npeer_survivor`; the leak
    /// tests pin the value-safe restore against exactly this), so a received
    /// input at `>= F` — directly or relayed as a connected claim — exists
    /// in aborted worlds too. Treating it as commit evidence would convert a
    /// clean abort into a re-drop that freezes the LEAKED value no other
    /// peer holds: a silent value divergence, strictly worse than the
    /// stall this read is balancing against.
    ///
    /// `Frame::NULL` terms (-1) are below any valid `F >= 0`, so a rearmed
    /// endpoint's default caches can never fake evidence.
    ///
    /// **Honest residuals (both fail-toward-stall, never desync; both rooted
    /// in epoch-less gossip — the tracked session-31/32 wire item):**
    /// - *Starved survivor with no visible re-drop:* a committed attempt in
    ///   which no `{disconnected, >= F}` claim has reached this survivor by
    ///   close time — either the symmetric corner (no peer anywhere received
    ///   a joiner input, every claim sits at `F - 1`) or the blackout
    ///   variant (the joiner is still alive toward other peers, so nobody
    ///   has dropped it yet) — takes the abort arm. Serving stays
    ///   byte-identical in the symmetric corner (every committed peer's
    ///   re-drop rolls its frozen value to the same `F - 1` phantom = `v0`),
    ///   but this survivor's `f0` claims are filtered by the committed
    ///   peers' armed floors, which can pin their confirmed fold at `F - 1`
    ///   — a stall, not a desync. The abort arm reports a violation when it
    ///   sees conflicting CONNECTED `>= F` gossip (the blackout fingerprint)
    ///   so operators get the breadcrumb.
    /// - *Cross-era straggler (multi-attempt):* a `{disconnected, >= F2}`
    ///   claim from a previous COMMITTED attempt's era, reordered past the
    ///   next attempt's reopen seed, can false-positive this read for an
    ///   ABORTED later attempt. The consequence is the commit arm: an
    ///   ordinary re-drop at the local receipt whose serving is
    ///   byte-identical to the restore for a leak-free closer (sync-layer
    ///   floor + the `F - 1` phantom both serve `(v0', Disconnected)`), with
    ///   this session's own floor possibly filtering the mesh's lower freeze
    ///   claims — a stall in the worst case. The confirmed-history leg
    ///   cannot be poisoned this way: F-sanity requires
    ///   `F > max(confirmed, last_confirmed)` both at directive acceptance
    ///   and again at reopen time (a stale-`F` reopen is cancelled
    ///   fail-closed — see
    ///   [`npeer_activation_frame_is_sane`](Self::npeer_activation_frame_is_sane)),
    ///   so every consulted attempt — evidence is only ever read for a
    ///   REOPENED pending attempt — began its reopened window below `F`,
    ///   and only post-reopen growth (clamp-capped below `F` while an
    ///   unresolved attempt is held in an aborted world; real era-`F`
    ///   confirmations in a committed one) can satisfy leg 1.
    #[cfg(feature = "hot-join")]
    fn npeer_attempt_commit_evidence(&self, handle: PlayerHandle, frame: Frame) -> bool {
        if std::cmp::max(
            self.confirmed_frame(),
            self.sync_layer.last_confirmed_frame(),
        ) >= frame
        {
            return true;
        }
        self.player_reg.remotes.values().any(|endpoint| {
            let claim = endpoint.peer_connect_status(handle);
            claim.disconnected && claim.last_frame >= frame
        })
    }

    /// Closes a wedged REOPENED pending attempt whose conclusion this
    /// survivor provably missed, discriminating the unheard outcome by the
    /// cap theorem (see [`Self::npeer_attempt_commit_evidence`]): commit
    /// evidence means the slot committed live mesh-wide — clear the pending,
    /// re-seed + arm the gossip floor, and leave the slot untouched; no
    /// commit evidence means the attempt aborted — apply the full pre-reopen
    /// restore. Either way the close is recorded in the stale-directive
    /// guard.
    #[cfg(feature = "hot-join")]
    fn close_unheard_reopened_attempt(&mut self, committed: bool) {
        let Some(pending) = self.hot_join.pending_reactivation.take() else {
            return;
        };
        self.record_closed_npeer_attempt(pending.handle, pending.frame);
        if committed {
            // The slot committed live; re-seed the cached views exactly like
            // the lifecycle commit close would have (between the reopen and
            // the commit, a not-yet-reopened survivor's stale disconnected
            // gossip may have re-stuck them). This close IS this survivor's
            // commit evidence, so the merge reactivation floor arms here —
            // leaving it unarmed would let a stale pre-attempt carrier
            // re-drop the committed slot below F - 1 afterwards (the round-1
            // Finding-3 hole).
            self.reset_reactivated_slot_gossip(
                pending.handle,
                pending.frame,
                &pending.joiner_addr,
                FloorArming::CommitEvidence,
            );
        } else {
            // Blackout fingerprint (see `npeer_attempt_commit_evidence`):
            // CONNECTED gossip at/past F is NOT commit evidence (a legal
            // pre-commit joiner leak produces it in aborted worlds too), but
            // its presence on an abort-arm close means the attempt MAY have
            // committed while this survivor was starved of every
            // discriminating signal — epoch-less gossip cannot tell the two
            // apart, and a wrong restore in a committed world can stall the
            // mesh (the committed peers' floors filter this survivor's f0
            // claims). Surface the breadcrumb for operators.
            if self.player_reg.remotes.values().any(|endpoint| {
                let claim = endpoint.peer_connect_status(pending.handle);
                !claim.disconnected && claim.last_frame >= pending.frame
            }) {
                report_violation!(
                    ViolationSeverity::Warning,
                    ViolationKind::NetworkProtocol,
                    "Closing the unheard reopened attempt for slot {} frame {} as ABORTED despite conflicting connected gossip at/past the activation frame: if the attempt actually committed (input-starved survivor), this restore can stall the mesh until the slot is re-served",
                    pending.handle,
                    pending.frame
                );
            }
            self.restore_pre_reopen_frozen_state(
                pending.handle,
                pending.frame,
                pending.pre_freeze_status,
                pending.pre_freeze_input,
            );
        }
    }

    /// Closes a REOPENED pending reactivation when its JOINER endpoint dies
    /// (`Event::Disconnected` for the joiner's address) — the survivor-side
    /// local re-freeze path.
    ///
    /// Without this close the pending entry would survive the drop forever:
    /// its shield permanently exempts the slot from the disconnect-
    /// convergence fold, and `reopened`-attempts-are-never-superseded blocks
    /// every future directive for the handle — once the coordinator's
    /// lifecycle messages are gone (e.g. the coordinator died mid-attempt),
    /// that is a permanent mesh-wide join wedge for the slot.
    ///
    /// The unheard outcome is discriminated by the same cap-theorem evidence
    /// read as the implied-close in
    /// [`handle_reactivate_directive`](Self::handle_reactivate_directive) —
    /// see [`npeer_attempt_commit_evidence`](Self::npeer_attempt_commit_evidence)
    /// for the two legs (confirmed history, gossip freeze-frame), their
    /// soundness, and why input-receipt terms (the local `last_frame`, a
    /// CONNECTED claim `>= F`) are deliberately NOT a third leg — a legal
    /// pre-commit joiner leak produces them in aborted worlds too, and
    /// treating them as evidence would freeze the leaked value no other
    /// peer holds. Commit evidence means the slot
    /// committed live and this death is the committed era's ordinary
    /// re-drop — clear the pending, re-seed + arm the gossip floor, and let
    /// the normal disconnect machinery (the caller's fall-through) freeze
    /// the slot at the genuinely received frames, convergent with every
    /// other committed peer. No commit evidence means the attempt aborted or
    /// can never commit (the coordinator observes the same joiner death and
    /// aborts) — apply the pre-reopen restore, the same deterministic
    /// value-safe restore every other reopened survivor applies, converging
    /// the mesh on the agreed pre-attempt freeze. The gossip leg is what
    /// keeps an INPUT-STARVED survivor (zero joiner inputs and zero
    /// `JoinCommitted` received — a dual one-way blackout) from misclassifying
    /// a committed attempt as aborted whenever any visible peer's claim
    /// carries the committed era (session-33 round-2 review Finding 2): the
    /// commit arm's re-drop at the seeded `F - 1` then `min`s the mesh's
    /// converged freeze down to `F - 1` with the `F - 1` phantom value, byte-
    /// identical everywhere. The one remaining ambiguous corner — a
    /// committed attempt in which NO visible claim ever exceeded `F - 1`
    /// (instant joiner death before any peer this survivor can see received
    /// an input) — takes the abort arm: serving is still byte-identical
    /// (every committed peer's re-drop rolls its frozen value to the same
    /// `F - 1` phantom = the pre-reopen value, and the freeze frames
    /// converge by the ordinary gossip min), but this survivor's `f0` claims
    /// are filtered by the committed peers' armed floors, which can pin
    /// their confirmed fold at `F - 1` — fail-toward-stall, never desync;
    /// closing it needs per-slot reactivation epochs (the tracked
    /// session-31/32 wire item).
    ///
    /// A PRE-reopen pending is deliberately left untouched: the slot is
    /// still in the reserved shape and the coordinator's abort timeline owns
    /// the attempt (the `Event::Disconnected` reserved-slot branch swallows
    /// the event); if the coordinator is also gone, the pre-reopen pending is
    /// supersedable by any future directive, so nothing wedges.
    ///
    /// The window between a coordinator death and the joiner endpoint's
    /// death — during which the pending is held and directives for the
    /// handle stay blocked — is inherent to R4 (a survivor never guesses a
    /// pending attempt's outcome). Bounding it is the chunk-N4
    /// joiner-teardown contract: a joiner that loses its coordinator must
    /// tear down its survivor channels (or simply stop sending) so this
    /// close fires via the ordinary endpoint timeout.
    #[cfg(feature = "hot-join")]
    fn close_reopened_pending_on_joiner_endpoint_death(&mut self, addr: &T::Address) {
        let Some(pending) = &self.hot_join.pending_reactivation else {
            return;
        };
        if !pending.reopened || pending.joiner_addr != *addr {
            return;
        }
        let committed = self.npeer_attempt_commit_evidence(pending.handle, pending.frame);
        report_violation!(
            ViolationSeverity::Warning,
            ViolationKind::NetworkProtocol,
            "Joiner endpoint at {:?} died while the reopened reactivation for slot {} frame {} was pending; closing the attempt locally as {}",
            addr,
            pending.handle,
            pending.frame,
            if committed { "committed" } else { "aborted" }
        );
        self.close_unheard_reopened_attempt(committed);
    }

    /// Closes a REOPENED pending reactivation before a user-initiated
    /// disconnect/removal touches the attempt's joiner (session-33 round-3
    /// review Finding 2) — the public [`disconnect_player`](Self::disconnect_player)
    /// and [`remove_player`](Self::remove_player) entry points call this
    /// first whenever the kicked handle's address is the pending attempt's
    /// joiner address.
    ///
    /// Without the close-first, the kick would freeze the reopened slot at
    /// its LOCAL RECEIPT — which a legal pre-commit joiner input leak can
    /// have raised to `>= F` — while `pending_reactivation` stayed held: an
    /// unmodeled freeze source minting a `{disconnected, >= F}` claim in a
    /// possibly-ABORTED world (the exact claim the
    /// [`npeer_attempt_commit_evidence`](Self::npeer_attempt_commit_evidence)
    /// induction proves cannot otherwise exist), poisoning other survivors'
    /// close discrimination, and leaving a frozen slot under a held pending
    /// (a later implied-commit close would re-seed a slot the user
    /// explicitly removed).
    ///
    /// The close discriminates the unheard outcome with the same evidence
    /// read as the joiner-endpoint-death close; the caller then applies the
    /// user's request to the post-close state. After an abort-arm close the
    /// slot is back in the reserved/dropped pre-attempt shape, so the kick
    /// reports it already disconnected/removed — the user's intent (this
    /// player is gone) already holds. After a commit-arm close the slot is
    /// live and the kick proceeds as the committed era's ordinary
    /// user-driven drop (whose freeze frame is `>= F - 1`, the genuine
    /// committed-era class the merge floor theorem covers). Every other
    /// shape is byte-identical to the unguarded behavior: no pending, a
    /// pending for a different joiner address, or a PRE-reopen pending
    /// (the slot is still frozen/disconnected, so the existing
    /// already-disconnected guards fail the call closed before any state
    /// changes).
    #[cfg(feature = "hot-join")]
    fn close_reopened_pending_before_user_disconnect(&mut self, addr: &T::Address) {
        let Some(pending) = &self.hot_join.pending_reactivation else {
            return;
        };
        if !pending.reopened || pending.joiner_addr != *addr {
            return;
        }
        let committed = self.npeer_attempt_commit_evidence(pending.handle, pending.frame);
        report_violation!(
            ViolationSeverity::Warning,
            ViolationKind::NetworkProtocol,
            "User-initiated disconnect/removal targets the joiner at {:?} while the reopened reactivation for slot {} frame {} is pending; closing the attempt locally as {} first",
            addr,
            pending.handle,
            pending.frame,
            if committed { "committed" } else { "aborted" }
        );
        self.close_unheard_reopened_attempt(committed);
    }

    /// N-peer activation-frame sanity predicate, shared by directive
    /// ACCEPTANCE and both REOPEN sites (session-33 round-4 review
    /// Finding 1): returns `true` iff `frame` is a real frame strictly past
    /// the slot's frozen bound, this session's instantaneous confirmed fold,
    /// and the sync layer's discard high-water — i.e. reopening the slot at
    /// `frame` cannot reposition the queue into history this session already
    /// confirmed or discarded.
    ///
    /// Why re-checking at REOPEN time is load-bearing and not redundant: the
    /// acceptance-time check is protected by the coordinator's pause (no
    /// inputs at/past `F` until the commit, and the commit requires this
    /// survivor's ack) — but the pause also ends at the ABORT, and abort
    /// delivery to a PRE-reopen survivor is best-effort only (the announcer's
    /// bounded resend burst; the survivor re-ack convergence loop arms only
    /// after the reopen). A pre-reopen survivor that misses `JoinAborted`
    /// keeps the pending held while its confirmed fold legitimately crosses
    /// `F` (the frozen slot folds `None`: locally disconnected, joiner
    /// endpoint reserved-excluded, every live claim disconnected — byte-safe
    /// by itself). A LATE reopen (the joiner<->survivor handshake completing
    /// after the abort) would then reposition the queue below confirmed
    /// history and arm `disconnect_frame = F`, after which `adjust_gamestate`
    /// re-targets a discarded state every advance — a probe-confirmed
    /// permanent wedge (`SynchronizedInputsFailed`, then `WrongSavedFrame`
    /// forever, unhealed by the eventual restore). The reopen-time re-check
    /// turns that into a fail-closed cancel.
    #[cfg(feature = "hot-join")]
    fn npeer_activation_frame_is_sane(&self, frame: Frame, frozen_bound: Frame) -> bool {
        !frame.is_null()
            && frame.as_i32() >= 0
            && frame > frozen_bound
            && frame > self.confirmed_frame()
            && frame > self.sync_layer.last_confirmed_frame()
    }

    /// Validates and applies a `ReactivateSlot{h, F}` directive (fail-closed:
    /// anything suspicious is ignored with a violation and no state change).
    #[cfg(feature = "hot-join")]
    fn handle_reactivate_directive(
        &mut self,
        sender: &T::Address,
        directive: &crate::network::messages::ReactivateSlot,
    ) {
        let handle = PlayerHandle::new(directive.handle);
        let frame = directive.frame;

        // A serving coordinator owns its reserved slots' lifecycle; it never
        // takes reopen directives from the mesh.
        if self.hot_join.accept_hot_join {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::NetworkProtocol,
                "Ignoring ReactivateSlot for slot {} from {:?}: this session serves hot-joins (coordinator role)",
                handle,
                sender
            );
            return;
        }
        if !handle.is_valid_player_for(self.num_players) {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::NetworkProtocol,
                "Ignoring ReactivateSlot for out-of-range slot {} from {:?}",
                handle,
                sender
            );
            return;
        }
        // The slot must be a remote player (its owner is the joiner address).
        let joiner_addr = match self.player_reg.handles.get(&handle) {
            Some(PlayerType::Remote(addr)) => addr.clone(),
            Some(PlayerType::Local) | Some(PlayerType::Spectator(_)) | None => {
                report_violation!(
                    ViolationSeverity::Warning,
                    ViolationKind::NetworkProtocol,
                    "Ignoring ReactivateSlot for non-remote slot {} from {:?}",
                    handle,
                    sender
                );
                return;
            },
        };
        // The joiner cannot direct its own slot's reactivation — only a
        // coordinator (a different peer) carries that authority.
        if *sender == joiner_addr {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::NetworkProtocol,
                "Ignoring ReactivateSlot for slot {} sent by the slot owner {:?}",
                handle,
                sender
            );
            return;
        }

        // Duplicate / conflicting attempt handling.
        if let Some(pending) = &self.hot_join.pending_reactivation {
            let matches = pending.handle == handle
                && pending.frame == frame
                && pending.coordinator_addr == *sender;
            if matches {
                // Duplicate directive: re-ack if already reopened (ack-loss
                // tolerance); otherwise the rearm/reopen is already in motion.
                if pending.reopened {
                    let coordinator = pending.coordinator_addr.clone();
                    if let Some(endpoint) = self.player_reg.remotes.get_mut(&coordinator) {
                        endpoint.send_reactivate_slot_ack(handle.as_usize(), frame);
                    }
                }
                return;
            }
            // A different attempt while one is pending. A NOT-yet-reopened
            // pending attempt may be superseded by (a) a STRICTLY NEWER
            // directive from the SAME coordinator (it aborted the previous
            // attempt and opened a retry — its abort may have been lost; the
            // coordinator is the attempt's authority, and R3 makes its
            // activation frames strictly monotone across attempts, so
            // `frame > pending.frame` is exactly the genuine-retry test — a
            // delayed duplicate of an OLDER attempt re-validating here would
            // reopen a frame whose lifecycle messages no longer exist) or
            // (b) a DIFFERENT sender's directive once the pending
            // coordinator's endpoint is gone (takeover). A stale
            // same-coordinator duplicate stays frame-ordered even when that
            // coordinator has since died — R3's monotonicity is a property of
            // the sender, not of its liveness.
            //
            // A REOPENED attempt is normally closed only by its own lifecycle
            // messages (whose delivery the re-ack loop makes reliable) — with
            // ONE exception: a strictly-newer directive from the same
            // coordinator PROVES the pending attempt already concluded
            // (one-join-at-a-time means the coordinator cannot be serving a
            // second attempt while the first is open, and R3 orders them), so
            // the survivor missed the lifecycle close (e.g. it was lost and
            // the new serve destroyed the post-serve responder). The unheard
            // outcome is discriminated by the CAP THEOREM — see
            // `npeer_attempt_commit_evidence` for the two evidence legs
            // (confirmed history, gossip freeze-frame), their soundness, the
            // honest residuals, and why input-receipt terms (local
            // `last_frame` / CONNECTED claims `>= F`) are deliberately NOT
            // evidence (a legal pre-commit joiner leak produces them in
            // aborted worlds too). No cache-resetting rearm can be in flight
            // here (the single pending slot blocks every other directive
            // while it is held).
            let coordinator_alive = self
                .player_reg
                .remotes
                .get(&pending.coordinator_addr)
                .is_some_and(UdpProtocol::is_running);
            let same_coordinator = pending.coordinator_addr == *sender;
            let newer_same_coordinator = same_coordinator && frame > pending.frame;
            if pending.reopened {
                if !newer_same_coordinator {
                    report_violation!(
                        ViolationSeverity::Warning,
                        ViolationKind::NetworkProtocol,
                        "Ignoring ReactivateSlot for slot {} frame {} from {:?}: reopened attempt for slot {} frame {} from {:?} is still pending",
                        handle,
                        frame,
                        sender,
                        pending.handle,
                        pending.frame,
                        pending.coordinator_addr
                    );
                    return;
                }
                let committed = self.npeer_attempt_commit_evidence(pending.handle, pending.frame);
                report_violation!(
                    ViolationSeverity::Warning,
                    ViolationKind::NetworkProtocol,
                    "ReactivateSlot for slot {} frame {} from {:?} implies the reopened pending attempt for slot {} frame {} concluded (lifecycle close was missed); closing it locally as {}",
                    handle,
                    frame,
                    sender,
                    pending.handle,
                    pending.frame,
                    if committed { "committed" } else { "aborted" }
                );
                self.close_unheard_reopened_attempt(committed);
                // Fall through: the new directive is validated freshly below
                // (after an implied COMMIT the slot is live, so the
                // frozen/disconnected gate fail-closed-rejects it until the
                // mesh's genuine re-drop lands and a retransmit re-arrives).
            } else if newer_same_coordinator || (!coordinator_alive && !same_coordinator) {
                report_violation!(
                    ViolationSeverity::Warning,
                    ViolationKind::NetworkProtocol,
                    "Replacing stale pre-reopen pending reactivation for slot {} frame {} from {:?} with directive for slot {} frame {} from {:?}",
                    pending.handle,
                    pending.frame,
                    pending.coordinator_addr,
                    handle,
                    frame,
                    sender
                );
                self.hot_join.pending_reactivation = None;
            } else {
                report_violation!(
                    ViolationSeverity::Warning,
                    ViolationKind::NetworkProtocol,
                    "Ignoring ReactivateSlot for slot {} frame {} from {:?}: attempt for slot {} frame {} from {:?} is still pending",
                    handle,
                    frame,
                    sender,
                    pending.handle,
                    pending.frame,
                    pending.coordinator_addr
                );
                return;
            }
        }

        // Stale-straggler guard: a directive at or below the highest CLOSED
        // attempt frame for this handle is a delayed duplicate of a concluded
        // attempt (the coordinator re-sends directives every poll while a
        // serve is open). Accepting it would reopen an attempt whose
        // lifecycle messages no longer exist anywhere. R3 monotonicity makes
        // every genuine new attempt strictly newer.
        if let Some(&closed) = self.hot_join.npeer_closed_attempt_frames.get(&handle) {
            if frame <= closed {
                report_violation!(
                    ViolationSeverity::Warning,
                    ViolationKind::NetworkProtocol,
                    "Ignoring ReactivateSlot for slot {} from {:?}: frame {} is at or below the closed-attempt high-water {}",
                    handle,
                    sender,
                    frame,
                    closed
                );
                return;
            }
        }

        // The slot must currently be frozen + disconnected (a dropped/reserved
        // slot) — reopening a live slot would corrupt confirmed history.
        let Some(status) = self.local_connect_status.get(handle.as_usize()).copied() else {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::InternalError,
                "Ignoring ReactivateSlot for slot {}: no connection status entry",
                handle
            );
            return;
        };
        if !status.disconnected || !self.sync_layer.player_is_frozen(handle) {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::NetworkProtocol,
                "Ignoring ReactivateSlot for slot {} from {:?}: slot is not frozen/disconnected (disconnected={}, frozen={})",
                handle,
                sender,
                status.disconnected,
                self.sync_layer.player_is_frozen(handle)
            );
            return;
        }
        // F sanity: must be a real frame strictly past the slot's frozen
        // bound and this survivor's confirmed frame (the cap guarantees the
        // latter for an honest coordinator; violating either would rewrite
        // committed history). `confirmed_frame()` is instantaneous and can
        // transiently DIP below the discard high-water during endpoint-cache
        // churn, so the sync layer's assigned `last_confirmed_frame` — the
        // frame whose history may already be discarded — is floored too
        // (defense-in-depth against a buggy/hostile coordinator timing a
        // directive into a dip; an honest coordinator's F clears both by the
        // cap argument). Honest limit of the defense (session-33 round-2
        // review): `last_confirmed_frame` is itself ASSIGNED from the dipped
        // instantaneous read on every advance (`set_last_confirmed_frame`),
        // not max-held, so a coordinator that times a directive into a
        // multi-advance dip can shrink this floor too — the residue is
        // byzantine-coordinator-only (an honest coordinator's F clears any
        // true high-water by the cap argument regardless). The shared
        // predicate (`npeer_activation_frame_is_sane`) is RE-CHECKED at both
        // reopen sites: this acceptance-time pass holds only while the
        // coordinator's pause does, and the pause also ends at the abort
        // (session-33 round-4 review Finding 1).
        if !self.npeer_activation_frame_is_sane(frame, status.last_frame) {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::NetworkProtocol,
                "Ignoring ReactivateSlot for slot {} from {:?}: activation frame {} is not past the frozen bound {}, confirmed frame {}, and discard high-water {}",
                handle,
                sender,
                frame,
                status.last_frame,
                self.confirmed_frame(),
                self.sync_layer.last_confirmed_frame()
            );
            return;
        }

        // Pre-attempt freeze-convergence gate (session-33 round-5 review
        // Finding 1): everything in the attempt machinery quantifies over a
        // single mesh-agreed pre-attempt freeze `(f0, v0)` — the bound clamp's
        // byte-safety proof, the abort restore, the reactivation floor, the
        // evidence induction — but survivors routinely freeze a dying peer's
        // slot at DIFFERENT frames, and the freeze-frame convergence re-adjust
        // (`update_player_disconnects` -> mine-down + frozen-value re-roll +
        // gap re-simulation) is what creates the agreement. Accepting a
        // directive before this survivor's freeze has converged would let the
        // held pending suspend that very mechanism (the fold shield) while the
        // slot goes mesh-agreed-excluded — after which this survivor's
        // confirmed fold crosses the un-converged gap carrying receipts no
        // other peer serves (silent confirmed-state divergence), and the
        // deferred mine-down later targets already-confirmed history (the
        // `WrongSavedFrame` wedge). Fail closed instead, with no pending and
        // no closed-attempt high-water: the coordinator re-sends the directive
        // every poll, and the generic (pending-free) re-adjust runs in the
        // very next `advance_frame` once the lagging claim lands — the gate
        // self-heals in about one round-trip. Convergence here means: no
        // running fold-visible endpoint still claims the slot CONNECTED (its
        // freeze would be unknown), and no folded DISCONNECTED claim sits
        // BELOW the local freeze frame (this survivor's own re-adjust is
        // owed). A folded claim ABOVE the local freeze is fine — that peer
        // owes its own re-adjust, which the directive/serve gates on ITS side
        // cover. An empty fold passes vacuously (the N == 2 post-drop shape,
        // mirroring the mesh-agreed arm of the confirmed fold), and so does a
        // NULL local freeze: NULL is the global minimum by definition — no
        // hidden freeze can undercut it, so this survivor's history for the
        // slot is already the convergence target.
        if !status.last_frame.is_null() {
            let mut any_connected_claim = false;
            let mut claim_min: Option<Frame> = None;
            for endpoint in self.player_reg.remotes.values() {
                if !endpoint.is_running() {
                    continue;
                }
                if self.hot_join.endpoint_is_reserved(endpoint) {
                    continue;
                }
                let claim = endpoint.peer_connect_status(handle);
                if !claim.disconnected {
                    any_connected_claim = true;
                }
                claim_min = Some(match claim_min {
                    Some(min) => std::cmp::min(min, claim.last_frame),
                    None => claim.last_frame,
                });
            }
            let readjust_owed = claim_min.is_some_and(|min| min < status.last_frame);
            if any_connected_claim || readjust_owed {
                report_violation!(
                    ViolationSeverity::Warning,
                    ViolationKind::NetworkProtocol,
                    "Ignoring ReactivateSlot for slot {} from {:?}: the slot's pre-attempt freeze is not yet mesh-converged (a running peer still claims it connected: {}, lowest folded freeze claim {:?} vs local freeze {}); the per-poll directive retransmit self-heals once convergence lands",
                    handle,
                    sender,
                    any_connected_claim,
                    claim_min,
                    status.last_frame
                );
                return;
            }
        }

        // Re-arm the joiner endpoint when it is terminal (same-address rejoin,
        // exactly like the coordinator's `rearm_dropped_slot_for_rejoin`); a
        // Running or still-synchronizing endpoint is left alone (rearming a
        // live channel would reset a working handshake).
        match self.player_reg.remotes.get_mut(&joiner_addr) {
            Some(endpoint) => {
                if endpoint.is_synchronized() && !endpoint.is_running() {
                    if let Err(e) = endpoint.rearm_for_rejoin() {
                        report_violation!(
                            ViolationSeverity::Error,
                            ViolationKind::InternalError,
                            "Failed to re-arm joiner endpoint at {:?} for N-peer reactivation of slot {}: {}",
                            joiner_addr,
                            handle,
                            e
                        );
                        return;
                    }
                }
            },
            None => {
                report_violation!(
                    ViolationSeverity::Warning,
                    ViolationKind::InternalError,
                    "Ignoring ReactivateSlot for slot {}: no remote endpoint at owner address {:?}",
                    handle,
                    joiner_addr
                );
                return;
            },
        }

        // While the attempt is pending (slot not yet live), treat the joiner
        // endpoint as reserved: it is excluded from the confirmed-frame and
        // disconnect-convergence folds (its freshly reset `{connected, NULL}`
        // status cache would otherwise pin this survivor's confirmed frame at
        // NULL), its `Event::Disconnected` is swallowed (the coordinator's
        // abort timeline owns a dying joiner pre-reopen), and its sync-timeout
        // event is suppressed — exactly the build-time reserved-slot shape.
        self.hot_join.reserved_slots.insert(handle);

        self.hot_join.pending_reactivation = Some(PendingReactivation {
            handle,
            frame,
            coordinator_addr: sender.clone(),
            joiner_addr,
            pre_freeze_status: status,
            pre_freeze_input: self.sync_layer.player_last_confirmed_input(handle),
            reopened: false,
        });
    }

    /// Progresses the pending reactivation: once the joiner endpoint is
    /// `Running`, reopen the slot at `F` and ack the coordinator. Reopening is
    /// gated on the live channel so the survivor never goes "real" on a slot it
    /// cannot receive inputs for (Agreement B).
    #[cfg(feature = "hot-join")]
    fn progress_pending_reactivation(&mut self) {
        let Some(pending) = &self.hot_join.pending_reactivation else {
            return;
        };
        if pending.reopened {
            return;
        }
        let joiner_running = self
            .player_reg
            .remotes
            .get(&pending.joiner_addr)
            .is_some_and(UdpProtocol::is_running);
        if !joiner_running {
            return;
        }

        let handle = pending.handle;
        let frame = pending.frame;
        let coordinator = pending.coordinator_addr.clone();
        let joiner_addr = pending.joiner_addr.clone();
        let frozen_bound = pending.pre_freeze_status.last_frame;

        // Re-validate F against CURRENT confirmed history before any
        // mutation (session-33 round-4 review Finding 1): the acceptance-time
        // F-sanity was protected by the coordinator's pause, but the pause
        // also ends at the ABORT, and a pre-reopen survivor that missed
        // `JoinAborted` (bounded resend burst; the re-ack loop arms only
        // after the reopen) keeps this pending held while its confirmed fold
        // legitimately crosses `F` (the frozen slot is excluded from the
        // fold). Reopening now would reposition the queue below
        // already-confirmed/discarded history and arm a forced re-simulation
        // from `F` that `adjust_gamestate` can never satisfy — a permanent
        // wedge (probe-confirmed: `SynchronizedInputsFailed`, then
        // `WrongSavedFrame` forever). Fail closed instead: cancel the attempt
        // exactly like the reactivate-failure arm below (slot stays frozen
        // AND reserved; no closed-attempt high-water — a live retry directive
        // at a genuinely newer F re-validates from this reserved shape, and a
        // stale-(h, F) duplicate re-fails acceptance F-sanity).
        //
        // Cancelling here is provably safe in BOTH worlds: this path runs
        // only while `!pending.reopened`, i.e. BEFORE this survivor's
        // reopen-ack, and the coordinator's commit barrier requires every
        // live survivor's ack — so the attempt cannot have committed
        // anywhere. The world is either still open (the serve then aborts on
        // its own Phase-4 timeout without our ack) or already aborted; a
        // cancelled pre-ack attempt strands nothing. The only other shape is
        // the pre-existing documented commit-after-prune divergence (the
        // coordinator pruned this survivor's pending ack mid-serve), where
        // this survivor's history has already diverged and staying frozen is
        // strictly safer than reopening into its own confirmed history.
        if !self.npeer_activation_frame_is_sane(frame, frozen_bound) {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::NetworkProtocol,
                "Cancelling the pending N-peer reactivation for slot {} at frame {}: the activation frame is no longer past the frozen bound {}, confirmed frame {}, and discard high-water {} (the attempt's conclusion was never heard); the slot stays frozen/reserved",
                handle,
                frame,
                frozen_bound,
                self.confirmed_frame(),
                self.sync_layer.last_confirmed_frame()
            );
            self.hot_join.pending_reactivation = None;
            return;
        }

        if let Err(e) = self.sync_layer.reactivate_player_at_frame(handle, frame) {
            // Should-never-happen (validated at directive time): fail closed by
            // cancelling the attempt — the slot stays frozen/reserved (and
            // deliberately RESERVED: the rearmed-Running joiner endpoint must
            // remain excluded from the confirmed/disconnect folds, exactly as
            // after a pre-reopen abort), and the coordinator's serve aborts on
            // its own timeout. No closed-attempt high-water is recorded here:
            // the attempt is still OPEN on the coordinator, whose per-poll
            // directive retransmit may legitimately retry the same `(h, F)`
            // from this reserved shape — recording would block that retry.
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::InternalError,
                "Failed to reactivate N-peer slot {} at frame {} on survivor: {}; cancelling the pending reactivation",
                handle,
                frame,
                e
            );
            self.hot_join.pending_reactivation = None;
            return;
        }
        // Arm the pre-activation serving floor: unlike the wait-gated
        // coordinator, nothing gates a survivor's receipt at reopen — a
        // late-arriving misprediction below F (or a lagging spectator/replay
        // flush) legitimately asks for pre-activation frames afterwards, and
        // they must present exactly as the pre-reopen simulation presented
        // them. See `SyncLayer::set_reactivation_floor`.
        if let Err(e) = self
            .sync_layer
            .set_reactivation_floor(handle, frame, frozen_bound)
        {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::InternalError,
                "Failed to arm the reactivation floor for N-peer slot {}: {}",
                handle,
                e
            );
        }
        if let Some(status) = self.local_connect_status.get_mut(handle.as_usize()) {
            status.disconnected = false;
            status.last_frame =
                safe_frame_sub!(frame, 1, "P2PSession::progress_pending_reactivation");
        } else {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::InternalError,
                "N-peer slot {} has no connection status entry at survivor reopen",
                handle
            );
        }
        // The slot is live again: hand it back to the normal machinery (the
        // joiner endpoint re-enters the folds, and a joiner endpoint dying
        // post-reopen is handled by the ordinary graceful-drop path).
        self.hot_join.reserved_slots.remove(&handle);
        // Force a re-simulation from F (session-33 review Finding 1): unlike
        // the paused coordinator, this survivor kept advancing under the cap
        // and has typically SIMULATED frames >= F with the frozen value. The
        // reset blanked the queue's prediction episode, so without an armed
        // rollback the joiner's real inputs from F are stored but never
        // COMPARED (`add_input` only compares against an open episode) — the
        // speculation would be kept forever, silently diverging from peers
        // that simulate F.. with the real inputs. The armed rollback
        // re-simulates F..current with the episode-anchored frozen prediction
        // (byte-identical values — `last_confirmed_input` is preserved) and
        // leaves an episode at F, so the real inputs are compared and
        // reconciled. Mirrors the coordinator commit and the abort restore.
        self.disconnect_frame = if self.disconnect_frame.is_null() {
            frame
        } else {
            std::cmp::min(self.disconnect_frame, frame)
        };
        // Un-stick this survivor's cached (sticky-disconnected) views of the
        // reopened slot — see `reset_reactivated_slot_gossip`. (Between this
        // reopen and the commit, a not-yet-reopened survivor's gossip can
        // re-stick them; the commit-receipt re-seeds.) SeedOnly: this reopen
        // is PRE-commit — the merge reactivation floor must not arm before
        // commit evidence, or an aborted attempt would leave the floor
        // filtering the mesh's genuine f0 drop gossip forever and pin this
        // survivor's confirmed frame at F - 1 (session-33 round-2 review
        // Finding 1). The pending shield covers this window instead.
        self.reset_reactivated_slot_gossip(handle, frame, &joiner_addr, FloorArming::SeedOnly);

        if let Some(p) = self.hot_join.pending_reactivation.as_mut() {
            p.reopened = true;
        }
        if let Some(endpoint) = self.player_reg.remotes.get_mut(&coordinator) {
            endpoint.send_reactivate_slot_ack(handle.as_usize(), frame);
        }
    }

    /// Applies a `JoinCommitted{h, F}`: a matching one completes the pending
    /// attempt (the slot stays live). Stale/mismatched ones are ignored.
    #[cfg(feature = "hot-join")]
    fn handle_join_committed_directive(
        &mut self,
        sender: &T::Address,
        body: &crate::network::messages::JoinCommitted,
    ) {
        let Some(pending) = &self.hot_join.pending_reactivation else {
            // Normal: the coordinator re-sends the lifecycle message several
            // times; every resend after the first lands here.
            trace!(
                "Ignoring JoinCommitted for slot {} frame {} from {:?} with no pending reactivation",
                body.handle,
                body.frame,
                sender
            );
            return;
        };
        let matches = pending.handle.as_usize() == body.handle
            && pending.frame == body.frame
            && pending.coordinator_addr == *sender;
        if !matches {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::NetworkProtocol,
                "Ignoring stale JoinCommitted for slot {} frame {} from {:?} (pending attempt: slot {} frame {} from {:?})",
                body.handle,
                body.frame,
                sender,
                pending.handle,
                pending.frame,
                pending.coordinator_addr
            );
            return;
        }
        let pending_handle = pending.handle;
        let pending_frame = pending.frame;
        let pending_joiner_addr = pending.joiner_addr.clone();
        if !pending.reopened {
            // The commit barrier requires our ack, so a matching commit
            // without a local reopen means the coordinator (or the channel)
            // misbehaved. Reopen now anyway: staying frozen while the mesh
            // commits the slot live would let our confirmed history diverge
            // (the gossip-min barrier would confirm frozen values for frames
            // other peers confirm as real); reopening at worst stalls us until
            // the joiner's inputs arrive. Stall over desync.
            let handle = pending.handle;
            let frame = pending.frame;
            let frozen_bound = pending.pre_freeze_status.last_frame;
            // ... UNLESS the activation frame is no longer past this
            // session's confirmed history (session-33 round-4 review
            // Finding 1: an aborted-unheard pre-reopen survivor legitimately
            // confirms past F). A defensive reopen at a stale F would
            // reposition the queue below confirmed/discarded history and
            // permanently wedge the session (the probe's `WrongSavedFrame`
            // loop) — strictly worse than any stall. The claimed commit is
            // untrusted here by construction (a genuine commit is impossible
            // without our ack; the only honest-world shape is the documented
            // commit-after-prune divergence, where the divergence already
            // happened and staying frozen is strictly safer). Cancel
            // fail-closed: slot stays frozen AND reserved, no closed-attempt
            // high-water (any stale-(h, F) straggler re-fails acceptance
            // F-sanity; a genuinely newer directive re-validates fresh).
            if !self.npeer_activation_frame_is_sane(frame, frozen_bound) {
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::NetworkProtocol,
                    "JoinCommitted for slot {} frame {} from {:?} arrived before this survivor reopened/acked AND the activation frame is no longer past the frozen bound {}, confirmed frame {}, and discard high-water {}; cancelling the pending reactivation fail-closed (the slot stays frozen/reserved)",
                    body.handle,
                    body.frame,
                    sender,
                    frozen_bound,
                    self.confirmed_frame(),
                    self.sync_layer.last_confirmed_frame()
                );
                self.hot_join.pending_reactivation = None;
                return;
            }
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::NetworkProtocol,
                "JoinCommitted for slot {} frame {} from {:?} arrived before this survivor reopened/acked; reopening defensively",
                body.handle,
                body.frame,
                sender
            );
            if let Err(e) = self.sync_layer.reactivate_player_at_frame(handle, frame) {
                // Should-never-happen (handle pre-validated). The slot stays
                // frozen AND deliberately reserved — the rearmed-Running
                // joiner endpoint must remain excluded from the
                // confirmed/disconnect folds (un-reserving would let its
                // caches pin this survivor's confirmed frame), and a future
                // directive re-validates from this exact reserved shape. No
                // closed-attempt high-water is recorded here either: the
                // serve may still be open coordinator-side, and its per-poll
                // directive retransmit may legitimately retry the same
                // `(h, F)` from this reserved shape.
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::InternalError,
                    "Failed to defensively reactivate N-peer slot {} at frame {}: {}",
                    handle,
                    frame,
                    e
                );
                self.hot_join.pending_reactivation = None;
                return;
            }
            if let Err(e) = self
                .sync_layer
                .set_reactivation_floor(handle, frame, frozen_bound)
            {
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::InternalError,
                    "Failed to arm the reactivation floor for N-peer slot {}: {}",
                    handle,
                    e
                );
            }
            if let Some(status) = self.local_connect_status.get_mut(handle.as_usize()) {
                status.disconnected = false;
                status.last_frame = safe_frame_sub!(
                    frame,
                    1,
                    "P2PSession::handle_join_committed_directive reopen"
                );
            } else {
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::InternalError,
                    "N-peer slot {} has no connection status entry at defensive reopen",
                    handle
                );
            }
            self.hot_join.reserved_slots.remove(&handle);
            // Same forced re-simulation from F as `progress_pending_reactivation`
            // (Finding 1): this survivor too may hold frozen-value speculation
            // for frames >= F that must be compared against the joiner's real
            // inputs.
            self.disconnect_frame = if self.disconnect_frame.is_null() {
                frame
            } else {
                std::cmp::min(self.disconnect_frame, frame)
            };
        }
        // Re-seed the cached views at the commit: between this survivor's
        // reopen and the commit, a not-yet-reopened survivor's disconnected
        // gossip may have re-stuck them (the merge is sticky-disconnected).
        // The receipt is this survivor's commit evidence: the merge
        // reactivation floor arms here (and not at the reopen above).
        let handle = pending_handle;
        let frame = pending_frame;
        self.reset_reactivated_slot_gossip(
            handle,
            frame,
            &pending_joiner_addr,
            FloorArming::CommitEvidence,
        );
        self.hot_join.pending_reactivation = None;
        // The attempt is closed: arm the stale-straggler guard against the
        // directive duplicates still in flight.
        self.record_closed_npeer_attempt(handle, frame);
    }

    /// Applies a `JoinAborted{h, F}`: a matching one cancels the pending
    /// attempt — restoring the pre-reopen frozen state if the slot was already
    /// reopened. Stale/mismatched ones are ignored.
    #[cfg(feature = "hot-join")]
    fn handle_join_aborted_directive(
        &mut self,
        sender: &T::Address,
        body: &crate::network::messages::JoinAborted,
    ) {
        let Some(pending) = &self.hot_join.pending_reactivation else {
            trace!(
                "Ignoring JoinAborted for slot {} frame {} from {:?} with no pending reactivation",
                body.handle,
                body.frame,
                sender
            );
            return;
        };
        let matches = pending.handle.as_usize() == body.handle
            && pending.frame == body.frame
            && pending.coordinator_addr == *sender;
        if !matches {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::NetworkProtocol,
                "Ignoring stale JoinAborted for slot {} frame {} from {:?} (pending attempt: slot {} frame {} from {:?})",
                body.handle,
                body.frame,
                sender,
                pending.handle,
                pending.frame,
                pending.coordinator_addr
            );
            return;
        }

        let handle = pending.handle;
        let frame = pending.frame;
        let pre_freeze_status = pending.pre_freeze_status;
        let pre_freeze_input = pending.pre_freeze_input;
        let reopened = pending.reopened;
        self.hot_join.pending_reactivation = None;
        // The attempt is closed: arm the stale-straggler guard against the
        // directive duplicates still in flight.
        self.record_closed_npeer_attempt(handle, frame);

        if !reopened {
            // Pre-reopen abort: nothing was mutated besides the rearm and the
            // reserved-slot membership. Keep both — the slot remains in the
            // exact build-time reserved shape (frozen queue, disconnected
            // status, re-synchronizable endpoint, reserved membership), ready
            // for the joiner's retry.
            return;
        }

        // Post-reopen abort: restore the slot to its pre-reopen reserved
        // state (captured frozen value, verbatim status, reserved membership,
        // forced re-simulation from F) — see the shared helper.
        self.restore_pre_reopen_frozen_state(handle, frame, pre_freeze_status, pre_freeze_input);
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
            PlayerType::Remote(addr) => {
                // N-peer survivor: a REOPENED pending reactivation for this
                // endpoint is closed FIRST, so the removal applies to the
                // post-close state instead of freezing the slot mid-attempt
                // (session-33 round-3 review Finding 2; see the method docs).
                #[cfg(feature = "hot-join")]
                {
                    let addr = addr.clone();
                    self.close_reopened_pending_before_user_disconnect(&addr);
                }
                #[cfg(not(feature = "hot-join"))]
                let _ = addr;
            },
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
            Some(PlayerType::Remote(addr)) => {
                // N-peer survivor: a REOPENED pending reactivation for this
                // endpoint is closed FIRST, so the kick applies to the
                // post-close state instead of freezing the slot mid-attempt
                // (session-33 round-3 review Finding 2; see the method docs).
                #[cfg(feature = "hot-join")]
                {
                    let addr = addr.clone();
                    self.close_reopened_pending_before_user_disconnect(&addr);
                }
                #[cfg(not(feature = "hot-join"))]
                let _ = addr;
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
    ///
    /// # Mesh-gossip bound at `N >= 3` (the "freeze barrier")
    ///
    /// Mirroring upstream GGPO's N-player confirmed frame (`PollNPlayers`),
    /// each **remote** slot contributes not its locally-received frame alone,
    /// but a fold over every running remote endpoint's **gossiped** view of
    /// that slot (the internal `remote_slot_confirmed_bound`): while the slot
    /// is connected, the minimum of the local receipt and the gossiped views;
    /// once the slot is **locally disconnected** but its drop is not yet
    /// mesh-agreed, the gossiped views ONLY (the local detection value is
    /// dropped from the fold, exactly as GGPO skips
    /// `local_connect_status[i]` when disconnected — required for liveness,
    /// see `remote_slot_confirmed_bound`). At `N == 2` this collapses to
    /// the local receipt (a peer's self-claim always covers the inputs it
    /// sent), so the value is unchanged in normal operation (two named
    /// conservative transient windows are documented in
    /// `remote_slot_confirmed_bound`); at `N >= 3` the bound IS the
    /// mesh-gossip minimum at all times — even a healthy steady state
    /// permanently paces roughly one gossip delivery behind the local
    /// receipt (GGPO `PollNPlayers` parity), and asymmetric loss widens that
    /// gap until gossip catches up. At `N == 3` this guarantees the
    /// confirmed frame can never
    /// run past a freeze frame the mesh may later agree on for a dropping
    /// peer — so the dropped slot's input at the agreed frame is never lost
    /// and the post-agreement rollback target always stays inside the
    /// prediction window. At `N >= 4` the same bound strictly NARROWS the
    /// race but does not fully close it: two corners (the stale-echo freeze
    /// and the double-failure relay) are documented as residuals in
    /// `remote_slot_confirmed_bound`.
    ///
    /// A slot whose disconnect is **mesh-agreed** (locally disconnected and no
    /// running endpoint still reports it connected) is excluded from the
    /// minimum entirely — its frozen input value carries it from then on.
    ///
    /// # Non-monotonicity
    ///
    /// The reported value is no longer guaranteed monotonic across calls: a
    /// gossiped adopt/min can lower a folded term, and fold membership changes
    /// (an endpoint starting, stopping, or a slot re-entering the minimum on
    /// hot-join reactivation) can transiently lower the result relative to an
    /// earlier call. Callers must not assume `confirmed_frame()` never
    /// decreases.
    #[must_use]
    pub fn confirmed_frame(&self) -> Frame {
        let mut confirmed_frame = Frame::new(i32::MAX);

        for (idx, con_stat) in self.local_connect_status.iter().enumerate() {
            let handle = PlayerHandle::new(idx);
            if self.player_reg.is_local_player(handle) {
                // Local players can never be disconnected (`remove_player` and
                // `disconnect_player` both reject local handles), so the local
                // slot always contributes its own last added frame.
                confirmed_frame = std::cmp::min(confirmed_frame, con_stat.last_frame);
            } else if let Some(bound) = self.remote_slot_confirmed_bound(handle, con_stat) {
                confirmed_frame = std::cmp::min(confirmed_frame, bound);
            }
            // `None`: the slot's disconnect is mesh-agreed — excluded from the
            // minimum; the frozen input value carries the slot from here on.
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

                    // Round-5 Finding 1 (coordinator sibling) backstop: this
                    // rewrite invalidates an already-captured N-peer snapshot
                    // at or above `disconnect_frame` — abort that serve
                    // fail-closed (see the helper for the discrimination
                    // argument and why the serve-poll's owed check dominates
                    // this in every derived ordering).
                    #[cfg(feature = "hot-join")]
                    self.abort_npeer_serve_if_snapshot_invalidated(disconnect_frame);
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

    /// Returns the [`Self::confirmed_frame`] contribution for a **remote** slot,
    /// or `None` when the slot's disconnect is mesh-agreed (the slot is then
    /// excluded from the confirmed-frame minimum; its frozen input value
    /// carries it).
    ///
    /// # Why (GGPO `PollNPlayers` gossip-min semantics — the N0 freeze barrier)
    ///
    /// Upstream GGPO's N-player confirmed frame (`PollNPlayers`) folds, for
    /// every slot, the minimum over every endpoint's gossiped
    /// `peer_connect_status[slot].last_frame`, **including the local view only
    /// while `local_connect_status[slot]` is not disconnected** — once the
    /// slot is locally disconnected the local term is skipped and only the
    /// gossip terms remain. This helper mirrors that exactly:
    ///
    /// - **Connected slot:** `min(local receipt, gossiped views)`. With no
    ///   contributing endpoints the local receipt alone (pre-barrier value).
    /// - **Locally disconnected, NOT yet mesh-agreed** (some running endpoint
    ///   still reports the slot connected): the gossiped views ONLY — the
    ///   local detection/freeze value is dropped from the fold.
    /// - **Mesh-agreed** (locally disconnected and no running endpoint still
    ///   reports it connected): `None`, the slot leaves the minimum.
    ///
    /// Using local receipt alone (the pre-barrier behavior) lets a survivor's
    /// confirmed frame race past the mesh-agreed freeze frame `F` of a
    /// dropping peer under asymmetric loss, with two damage mechanisms:
    ///
    /// - **Window floor (the mechanism the red repros actually hit):** the
    ///   race lets `current_frame` run so far that the `F + 1` rollback
    ///   target falls below the prediction-window floor; the S20 clamp then
    ///   re-simulates only from the floor, leaving every frame in
    ///   `(F, floor)` permanently uncorrected (the constant post-floor offset
    ///   in the red-test output shows the frozen-value re-roll itself had
    ///   SUCCEEDED — only the resimulation was clamped short).
    /// - **Ring eviction (extreme stagger only):** the dropped slot's input
    ///   at `F` is genuinely lost only once the input ring wraps over it
    ///   (receipt stagger >= the queue capacity, `INPUT_QUEUE_LENGTH`), at
    ///   which point the convergence re-roll `set_frozen_value_at(F)` /
    ///   first-freeze `freeze_at(F)` fail-safes into a stale value. Note that
    ///   `set_last_confirmed_frame`'s logical discard alone does NOT defeat
    ///   the re-roll: `discard_confirmed_frames` only moves the ring's
    ///   tail/length, and `confirmed_input` indexes the ring modulo its
    ///   capacity without a tail check, so a "discarded" frame's bytes remain
    ///   readable until physically overwritten.
    ///
    /// Folding the gossiped views caps the confirmed frame itself, which
    /// closes both mechanisms at `N == 3`; at `N >= 4` it strictly narrows
    /// them (see the documented residuals below).
    ///
    /// # Soundness of dropping the local term once locally disconnected
    ///
    /// The bound may then EXCEED the local freeze value `L_local` — that is
    /// safe:
    ///
    /// - Frames above `L_local` are confirmed via the frozen-value branch (the
    ///   sync layer serves the frozen `last_confirmed_input` for a
    ///   disconnected slot past its `last_frame`), which is self-consistent
    ///   regardless of confirmation pacing — no real input above `L_local` is
    ///   ever needed for them.
    /// - Any future converged freeze `F'` is the min over the convergence
    ///   fold's terms ({the possibly mined-down local view} ∪ {running
    ///   endpoints' reported values}; see
    ///   [`Self::update_player_disconnects`]). Case 1: `F'` equals the local
    ///   term. The mining-down of that term only ever comes from
    ///   endpoint-term overrides (`update_player_disconnects` passes a
    ///   `queue_min_confirmed` folded over endpoint terms into
    ///   [`Self::disconnect_player_at_frames`]), and the gossip-only bound is
    ///   <= every endpoint term, hence <= any such override — so
    ///   `confirmed <= override`, the re-roll input at the override frame is
    ///   retained (discard runs through `confirmed - 1`), and the
    ///   `override + 1` rollback target stays inside the prediction window
    ///   (the throttle caps `current <= confirmed + max_prediction`). If `F'`
    ///   is the ORIGINAL local detection value (never mined), no re-roll below
    ///   it is ever needed — the frozen value was captured at detection time,
    ///   when the pre-detection bound (which still folded the local term) had
    ///   held `confirmed <= L_local`. Case 2: `F'` equals some endpoint `j`'s
    ///   term: the gossip-only bound <= our cache of `j`'s term <= `F'`
    ///   directly. Either way confirmation never outruns data needed for any
    ///   future convergence.
    /// - **Liveness (why the local term is dropped):** connect-status
    ///   gossip travels only in Input messages, a session capped at
    ///   `confirmed + max_prediction` with a fully-acked send queue sends no
    ///   Input messages, and `update_player_disconnects` runs in
    ///   `advance_frame` BEFORE the throttle. Under asymmetric loss both
    ///   survivors can burn their entire window against the lagging receipt
    ///   `F` before either detects the drop; folding the local detection value
    ///   would then keep BOTH bounds at `F` — both capped, both gossip-mute
    ///   (KeepAlives carry no connect status). Before the connect-status
    ///   nudge existed this was a permanent, silent deadlock; with the nudge
    ///   it would still hold every staggered release hostage to the nudge
    ///   cadence. Gossip-only folding (GGPO parity) instead lifts the low
    ///   survivor's bound to its peer's stale-HIGHER gossiped view
    ///   immediately, restoring headroom so its ordinary Input packets deliver
    ///   the `disconnected@F` gossip and the mesh converges without waiting on
    ///   any timer.
    ///
    /// # Liveness closure: the connect-status nudge
    ///
    /// Gossip-only folding alone is NOT enough for liveness. The common clean
    /// drop (zero stagger: every survivor's receipt of the dying peer is
    /// EXACTLY equal, e.g. a process kill on a quiet link) leaves every
    /// post-detection gossip-only bound equal to the other survivors' stale
    /// `connected@F` caches = `F`; with every survivor capped and fully
    /// acked, no Input message ever carries the disconnected gossip and mesh
    /// agreement is unreachable — a permanent pin. At `N >= 4` the staggered
    /// variant deadlocks too: only the min-receipt survivor regains headroom,
    /// while the others each wait forever for another MUTE peer's gossip.
    /// Both pins are closed at the protocol level: while
    /// [`Self::connect_status_nudge_needed`] reports a locally-disconnected,
    /// not-yet-mesh-agreed slot, every running remote endpoint that is
    /// **input-idle** (no real Input message sent for a keepalive interval
    /// and an empty send queue — active input traffic already carries the
    /// status, so the nudge never alters an advancing session's packet
    /// stream) re-sends a status-bearing duplicate Input message built from
    /// its retained `last_acked_input` on the keepalive cadence
    /// (`UdpProtocol::send_connect_status_nudge`) — so a drop's hold is now
    /// bounded by ~`disconnect_timeout` + the nudge cadence + delivery,
    /// even when every survivor is gossip-mute. Regression-pinned by the
    /// clean-drop and N=4 mutual-mute repros in `tests/sessions/peer_drop.rs`.
    ///
    /// **Asymmetric cutoff (the nudge stops before everyone has agreed):**
    /// [`Self::connect_status_nudge_needed`] clears on LOCAL mesh agreement,
    /// but release is a GLOBAL condition — a peer that has not yet agreed
    /// still needs this session's view. From the cutoff on, that view rides
    /// only this session's real Input traffic: its next fresh send and the
    /// `running_retry_interval` retransmission of any still-unacked pending
    /// Input. The retransmission timer is therefore load-bearing for global
    /// release, which is why `on_input` refreshes its pacer
    /// (`running_last_input_recv`) only for packets that stage at least one
    /// NEW frame — a progress-free packet stream (e.g. a still-nudging peer
    /// on the keepalive cadence) must never suppress it (see the gate in
    /// `UdpProtocol::on_input` and the blackout regression in
    /// `tests/sessions/peer_drop.rs`).
    ///
    /// # Documented residuals (`N >= 4`; not closed by the barrier or nudge)
    ///
    /// The barrier closes the desync at `N == 3` and strictly narrows it at
    /// `N >= 4`. Two `N >= 4` corners remain open (both require at least
    /// three survivors; neither is a regression — the pre-barrier code had no
    /// bound at all):
    ///
    /// - **Stale-echo freeze:** a third survivor `X` can freeze the dropped
    ///   slot using its stale cache of OUR old (lower) claim — the moment
    ///   ANOTHER peer's disconnect report flips `X`'s `queue_connected`,
    ///   `X`'s endpoint-terms override mins that stale-low cached term and
    ///   converges a freeze frame BELOW our current bound, which our
    ///   confirmation may already have passed (re-exposing the window-floor
    ///   mechanism). Impossible at `N == 3`: there the only packet that can
    ///   flip `X`'s `queue_connected` comes from the endpoint whose term it
    ///   would min, and the connect-status merge refreshes that same term
    ///   BEFORE the fold runs. Not a mute problem, so the nudge does not
    ///   close it (it is a stale-cache race).
    /// - **Double-failure relay:** an origin survivor that dies AFTER
    ///   relaying its low freeze value to a third peer but BEFORE delivering
    ///   it to us leaves a window where our bound (no longer folding the dead
    ///   origin's endpoint) exceeds the override later relayed through the
    ///   third peer.
    ///
    /// Future work for both: a stale-aware override fold (ignore cached terms
    /// older than the slot's disconnect epoch) or full
    /// mesh-agreement-before-freeze convergence. Byzantine peers are out of
    /// scope entirely.
    ///
    /// # `N == 2` identity (normal operation)
    ///
    /// The only remote slot's gossip term is that peer's SELF-claim, which is
    /// always >= the direct receipt term (a packet carrying inputs through
    /// frame `k` carries a self-claim `>= k`), so the connected-slot min
    /// collapses to the local receipt. After the `N == 2` remote drops, its
    /// endpoint is disconnected (not running), the fold is empty and the slot
    /// is excluded exactly as before. Two named transient windows fall
    /// outside this identity, both in the strictly CONSERVATIVE direction
    /// (the bound dips below the pure local receipt; nothing is confirmed
    /// early):
    ///
    /// - **Peer-initiated disconnect:** `disconnect_requested` packets skip
    ///   the connect-status merge while their inputs still process, so the
    ///   local receipt can briefly exceed the cached self-claim and the bound
    ///   holds at the stale cache until the endpoint leaves the fold.
    /// - **Hot-join activation:** the host reopens the joined slot with a
    ///   synthetic `last_frame = F - 1` while the rebuilt endpoint's status
    ///   cache is still the default `{connected, NULL}`, so
    ///   [`Self::confirmed_frame`] reports `Frame::NULL` (not `F - 1`) until
    ///   the joiner's first input packet delivers real gossip.
    ///
    /// # Release/liveness of the connected-slot hold
    ///
    /// The bound rises as gossip lands (every input packet carries
    /// connect-status; the merge runs even for undecodable packets), a
    /// never-detecting survivor follows via the propagated disconnect path in
    /// [`Self::update_player_disconnects`], and a dead survivor's endpoint
    /// times out (not running) and leaves the fold — so a hold is bounded by
    /// ~`disconnect_timeout` plus gossip delivery (plus the nudge cadence
    /// when survivors are input-idle, see above). One bounded delay worth
    /// naming: a `DisconnectBehavior::Halt` peer inside an otherwise
    /// `ContinueWithout` mesh stops advancing (and therefore stops gossiping
    /// fresh views) the moment it halts, so it holds the other survivors'
    /// bounds until ITS endpoints time out and leave their folds — bounded by
    /// the same `disconnect_timeout`, but a real added delay. While held,
    /// `advance_frame` returns `Ok` with no `AdvanceFrame` request (the
    /// normal prediction-window throttle), never an error.
    ///
    /// # Hot-join reserved endpoints
    ///
    /// Reserved/rearmed hot-join endpoints can sit `Running` with a freshly
    /// reset default `{connected, NULL}` status cache before their joiner
    /// activates; folding them would pin the bound to `Frame::NULL` forever
    /// (e.g. a host whose joiner abandons the join mid-handshake), so they are
    /// skipped — matching how `remote_is_connected` and `check_initial_sync`
    /// gate them out. **Fold alignment (cross-reference):**
    /// [`Self::update_player_disconnects`]' endpoint fold carries the same
    /// reserved-endpoint guard (aligned when the N-peer survivor machinery made
    /// reserved endpoints coexist with multi-survivor folds): without it, a
    /// reserved endpoint's default `{connected, NULL}` cache would both block
    /// mesh agreement and mine convergence overrides down to `NULL` there.
    fn remote_slot_confirmed_bound(
        &self,
        handle: PlayerHandle,
        local_status: &ConnectionStatus,
    ) -> Option<Frame> {
        // N-peer pending-reactivation shield, bound leg (companion to the
        // fold shield in `update_player_disconnects`): while this survivor
        // holds the REOPENED attempt `(handle, F)`, the attempt owns the
        // slot's status, but the paused coordinator (and any not-yet-reopened
        // survivor) keeps gossiping the pre-attempt `{disconnected, f0}`
        // truth, re-sticking this session's caches (the merge floor is
        // deliberately NOT armed pre-commit — see
        // `UdpProtocol::arm_reactivation_floor`). Folding those claims raw
        // would dip this survivor's confirmed bound to `f0` for the whole
        // attempt, freezing its own advance and stalling spectator/replay
        // flushes mid-join. They are therefore folded CLAMPED to the seeded
        // `F - 1` — the value the reopen seed stamped and the commit-receipt
        // re-seed would restore — never skipped (session-33 round-3 review
        // Finding 1: a skip removes the cap with the dip, and a post-reopen
        // ABORTED world then lets a leaked survivor confirm past `F`).
        //
        // Soundness, quantified over BOTH worlds:
        // - **Aborted (or still-open) world:** the clamp guarantees
        //   `confirmed < F` while the pending is held, so the eventual
        //   restore (`disconnect_frame = min(.., F)`) never re-simulates a
        //   frame this session already confirmed. The cap is engaged before
        //   `confirmed` could cross `F`: crossing requires every live peer's
        //   slot bound `>= F`, hence receiving each live peer's inputs at
        //   `>= F` — and every validated `Input` packet merges that peer's
        //   full connect-status vector first (`UdpProtocol::on_input` hoists
        //   the merge), delivering its `{disconnected, f0}` claim for
        //   `handle` (every participant that has CONCLUDED the attempt holds
        //   `{disconnected, f0}` in an aborted world: never-reopened peers
        //   kept it, reopened peers restore it once the abort reaches them,
        //   the coordinator never left it pre-commit — but a STILL-PENDING
        //   reopened peer that has not heard the abort keeps gossiping a
        //   CONNECTED claim the legal leak may have raised past `F`, which
        //   folds raw; see the residual below). The lone
        //   exception — `disconnect_requested` packets skip the merge — still
        //   caps: the cache then retains the reopen seed `{connected, F-1}`
        //   (nothing else writes it), which folds at `F - 1` anyway. Frames
        //   in `(f0, F)` confirmed under the clamp are frozen-served
        //   byte-identically on every peer (the sync-layer reactivation
        //   floor serves `(v0, Disconnected)` there), so nothing leaks —
        //   `(f0, v0)` is mesh-uniform here as a real precondition, not an
        //   assumption: directive acceptance and the serve open both
        //   fail-closed-defer until the slot's freeze convergence has landed
        //   (session-33 round-5 review Finding 1), with the N>=4
        //   fold-pruning relay lowering the documented residual.
        //   Residual (the true boundary): the cap requires at least one
        //   folded DISCONNECTED claim — or a folded STALE connected claim
        //   `<= F - 1` (a starved still-pending survivor's reopen seed, or
        //   a contract-honoring joiner's self-claim), which pins the min
        //   raw. A still-pending reopened survivor folds a CONNECTED claim
        //   the legal leak may have raised past `F`, and connected claims
        //   never clamp — so the uncapped region is: coordinator dead (or
        //   pruned from this fold) AND every folded live peer itself a
        //   still-pending reopened leak-fed survivor; only-the-joiner-left
        //   (everyone-else-dead) is its degenerate sub-shape, not the whole
        //   region. Consequence: in the coordinator-DEAD shape every
        //   survivor that crosses `F` holds the same leg-1 evidence and the
        //   eventual closes COMMIT-ARM CONSISTENTLY with each other (if any
        //   folded peer were starved/restored, its claim would instead
        //   pin/clamp everyone below `F` and all closes abort-arm) — no
        //   disagreeing live counterpart. The asymmetric coordinator-PRUNED
        //   variant (alive and folded in another survivor's fold, link-dead
        //   toward this one) does have a living divergence counterpart, but
        //   additionally requires a full disconnect-timeout link death plus
        //   sustained suppression of the per-poll re-ack-driven abort
        //   re-delivery. All variants are bounded by the chunk-N4 joiner
        //   contract (no self-claim past `F - 1` pre-commit; teardown on
        //   coordinator loss), and resolution defers to the
        //   joiner-endpoint-death close.
        // - **Committed world:** the clamp is transiently conservative — the
        //   bound holds at `F - 1` until the `JoinCommitted` receipt clears
        //   the pending (~1 RTT, the same conservative-lag class as the rest
        //   of the design), after which the re-seed + armed floor take over
        //   and the bound follows the normal rules. A committed-era re-drop
        //   claim `{disconnected, >= F}` arriving early is also clamped —
        //   conservative, and the evidence read
        //   (`npeer_attempt_commit_evidence`) reads the raw caches, so close
        //   discrimination is unaffected.
        // - **Mid-attempt (the dip the shield was built for):** the clamp is
        //   `F - 1`, not `f0`, so the spectator/replay flush keeps draining
        //   through `F - 1` (pinned by the spectator-flush test).
        #[cfg(feature = "hot-join")]
        let attempt_clamp = self
            .hot_join
            .pending_reactivation
            .as_ref()
            .filter(|pending| pending.reopened && pending.handle == handle)
            .map(|pending| {
                safe_frame_sub!(
                    pending.frame,
                    1,
                    "P2PSession::remote_slot_confirmed_bound pending clamp"
                )
            });
        let mut any_reports_connected = false;
        let mut gossip_min: Option<Frame> = None;
        for endpoint in self.player_reg.remotes.values() {
            if !endpoint.is_running() {
                continue;
            }
            #[cfg(feature = "hot-join")]
            if self.hot_join.endpoint_is_reserved(endpoint) {
                continue;
            }
            let status = endpoint.peer_connect_status(handle);
            #[cfg(not(feature = "hot-join"))]
            let folded_frame = status.last_frame;
            #[cfg(feature = "hot-join")]
            let folded_frame = if status.disconnected {
                attempt_clamp.unwrap_or(status.last_frame)
            } else {
                status.last_frame
            };
            if !status.disconnected {
                any_reports_connected = true;
            }
            gossip_min = Some(match gossip_min {
                Some(gossip) => std::cmp::min(gossip, folded_frame),
                None => folded_frame,
            });
        }
        match (local_status.disconnected, any_reports_connected, gossip_min) {
            // Connected slot, no contributing endpoints: the local receipt
            // alone (pre-barrier behavior).
            (false, _, None) => Some(local_status.last_frame),
            // Connected slot: barrier bound = min(local receipt, gossip).
            (false, _, Some(gossip)) => Some(std::cmp::min(local_status.last_frame, gossip)),
            // Locally disconnected but NOT yet mesh-agreed: gossip terms ONLY
            // (GGPO `PollNPlayers` parity — the liveness-critical arm; folding
            // the local term here pins capped survivors against their own
            // detection value, see the rustdoc).
            (true, true, Some(gossip)) => Some(gossip),
            // Unreachable: `any_reports_connected` implies at least one folded
            // endpoint, so `gossip_min` is `Some`. Handle the impossible
            // combination conservatively as mesh-agreed (exclude the slot)
            // rather than panicking or inventing a value.
            (true, true, None) => None,
            // Mesh-agreed disconnect (no running endpoint still reports the
            // slot connected; also covers the empty-fold `N == 2` post-drop
            // case): exclude the slot — the frozen value carries it.
            (true, false, _) => None,
        }
    }

    /// Returns `true` while any remote slot is **locally disconnected but not
    /// yet mesh-agreed** — exactly the window in which
    /// [`Self::remote_slot_confirmed_bound`] still returns `Some` for a
    /// disconnected slot. While true, every running remote endpoint must keep
    /// gossiping our connect status even when input-idle (the protocol-level
    /// connect-status nudge, `UdpProtocol::set_connect_status_nudge`):
    /// otherwise capped, fully-acked survivors are gossip-mute and mesh
    /// agreement can never be reached. Allocation-free; called once per
    /// [`Self::poll_remote_clients`].
    fn connect_status_nudge_needed(&self) -> bool {
        self.local_connect_status
            .iter()
            .enumerate()
            .any(|(idx, status)| {
                let handle = PlayerHandle::new(idx);
                // Local players can never be disconnected; the guard keeps the
                // helper's contract honest if that ever changes.
                status.disconnected
                    && !self.player_reg.is_local_player(handle)
                    && self.remote_slot_confirmed_bound(handle, status).is_some()
            })
    }

    /// Applies a freeze-frame convergence re-adjust to a RESERVED (or
    /// pre-reopen-pending) hot-join slot WITHOUT the generic disconnect
    /// path's endpoint teardown (session-33 round-5 review Finding 1).
    ///
    /// The generic re-adjust route (`disconnect_player_with_policy` ->
    /// `disconnect_player_at_frames`) calls `endpoint.disconnect()` on the
    /// slot's registry endpoint — for a reserved/rearmed slot that endpoint
    /// is the (re-)armed JOINER channel, and `Disconnected` is a terminal
    /// protocol state with no reconnect edge, so the teardown silently bricks
    /// the slot's rejoinability (the `Suppress` re-adjust path never re-arms)
    /// and kills any in-flight attempt handshake. The convergence itself is
    /// exactly four effects, applied here verbatim from the generic path:
    ///
    /// 1. mine `local_connect_status[handle].last_frame` DOWN to the folded
    ///    global-minimum freeze frame;
    /// 2. re-roll the frozen value to the dropped peer's input confirmed at
    ///    that frame (`SyncLayer::set_frozen_value_at` — fail-safe when the
    ///    target was already discarded, identical to the generic path);
    /// 3. arm `disconnect_frame` so the gap re-simulates with the agreed
    ///    value (and prune the now-stale local checksum history — F11);
    /// 4. abort an open N-peer serve whose CAPTURED snapshot the rewrite
    ///    invalidates (see
    ///    [`abort_npeer_serve_if_snapshot_invalidated`](Self::abort_npeer_serve_if_snapshot_invalidated)).
    ///
    /// If the slot is held by a PRE-reopen pending reactivation, the
    /// pending's captured `pre_freeze_status`/`pre_freeze_input` are
    /// REFRESHED to the converged values: the reopen's floor arming, the
    /// reopen-time F re-checks, and (decisively) the post-reopen abort
    /// RESTORE all read the capture, and restoring a stale-high freeze would
    /// resurrect exactly the divergence the convergence just healed.
    ///
    /// The defensive `!disconnected` arm is unreachable by construction
    /// (reserved and pre-reopen-pending slots are frozen + disconnected); it
    /// fails closed with no state change, and the recompute-per-call fold
    /// retries on the next advance.
    #[cfg(feature = "hot-join")]
    fn converge_reserved_slot_freeze(&mut self, handle: PlayerHandle, agreed_last_frame: Frame) {
        let converged_status = {
            let Some(status) = self.local_connect_status.get_mut(handle.as_usize()) else {
                report_violation!(
                    ViolationSeverity::Warning,
                    ViolationKind::InternalError,
                    "Invalid player handle {} in converge_reserved_slot_freeze - ignoring",
                    handle
                );
                return;
            };
            if !status.disconnected {
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::InternalError,
                    "Reserved/pending hot-join slot {} is unexpectedly connected during freeze convergence; skipping the re-adjust",
                    handle
                );
                return;
            }
            status.last_frame = std::cmp::min(status.last_frame, agreed_last_frame);
            *status
        };
        let converged_last_frame = converged_status.last_frame;
        self.sync_layer
            .set_frozen_value_at(handle, converged_last_frame);
        if self.sync_layer.current_frame() > converged_last_frame {
            let disconnect_frame = safe_frame_add!(
                converged_last_frame,
                1,
                "P2PSession::converge_reserved_slot_freeze"
            );
            self.disconnect_frame = if self.disconnect_frame.is_null() {
                disconnect_frame
            } else {
                std::cmp::min(self.disconnect_frame, disconnect_frame)
            };
            // F11 (mirrors `disconnect_player_at_frames`): the frozen-value
            // re-roll retroactively changed the slot's confirmed input at
            // every frame >= disconnect_frame, so locally stored checksums
            // for those frames are stale.
            self.local_checksum_history
                .retain(|&frame, _| frame < disconnect_frame);
            self.abort_npeer_serve_if_snapshot_invalidated(disconnect_frame);
        }
        let refreshed_input = self.sync_layer.player_last_confirmed_input(handle);
        if let Some(pending) = self
            .hot_join
            .pending_reactivation
            .as_mut()
            .filter(|pending| !pending.reopened && pending.handle == handle)
        {
            pending.pre_freeze_status = converged_status;
            pending.pre_freeze_input = refreshed_input;
        }
    }

    /// Aborts the open N-peer serve if a confirmed-history rewrite at
    /// `disconnect_frame` invalidates its already-CAPTURED snapshot
    /// (session-33 round-5 review Finding 1, coordinator sibling).
    ///
    /// A re-adjust (or propagated freeze) whose forced re-simulation starts
    /// at or below the snapshot frame `S` rewrites state the captured bytes
    /// embed. Re-capturing at the same `S` is NOT discriminable: the joiner
    /// applies the FIRST snapshot it receives (duplicates are idempotent) and
    /// acks by FRAME, so once the stale bytes may be in flight, a fresh ack
    /// can vouch for either byte stream. The only sound move is the
    /// fail-closed ABORT — the joiner retries, and the R3 next-serve guard
    /// forces the retry onto a strictly later `(handle, F)`.
    ///
    /// Pre-capture rewrites need no abort: the wait-then-capture gate's
    /// misprediction term (`check_simulation_consistency(disconnect_frame)`)
    /// holds the capture until the paused-arm repair has re-simulated the
    /// gap, and the owed-re-adjust deferral in
    /// [`poll_npeer_host_serve`](Self::poll_npeer_host_serve) keeps the gate
    /// from passing in the merge-to-apply window. This hook is the backstop
    /// for rewrites applied AFTER a capture — every ordering derived so far
    /// is caught earlier by the serve-poll's owed check (claims merge before
    /// the serve poll, which runs before `update_player_disconnects`), so
    /// this is defense-in-depth at the single chokepoint every
    /// `disconnect_frame` arming passes through.
    #[cfg(feature = "hot-join")]
    fn abort_npeer_serve_if_snapshot_invalidated(&mut self, disconnect_frame: Frame) {
        let invalidated = self.hot_join.npeer.as_ref().is_some_and(|serve| {
            serve.snapshot.is_some() && disconnect_frame <= serve.snapshot_frame
        });
        if !invalidated {
            return;
        }
        if let Some(serve) = self.hot_join.npeer.take() {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::FrameSync,
                "N-peer hot-join serve for slot {} aborted: a freeze convergence re-adjust rewrote confirmed history from frame {} <= the captured snapshot frame {} (a same-frame recapture would be indistinguishable from the stale bytes already in flight)",
                serve.handle,
                disconnect_frame,
                serve.snapshot_frame
            );
            self.abort_npeer_serve(serve);
        }
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

            // N-peer reactivation shield — REOPENED attempts only (session-33
            // round-5 review Finding 1). While this survivor holds the
            // REOPENED attempt for `handle`, the attempt owns the slot's
            // status: the paused coordinator (and any not-yet-reopened
            // survivor) keeps gossiping the slot's pre-attempt `disconnected`
            // state until the attempt concludes, and folding that stale
            // gossip here would re-apply the drop and re-freeze the
            // just-reopened LIVE slot, wedging the attempt.
            //
            // Both windows, both worlds:
            // - PRE-reopen (NOT skipped): the slot is still frozen +
            //   disconnected, and the freeze-frame convergence re-adjust IS
            //   the correctness mechanism — survivors freeze a dying peer's
            //   slot at different frames, and the global-min mine-down +
            //   frozen-value re-roll + gap re-simulation are what make the
            //   mesh's `(f0, v0)` uniform. Skipping it here (the pre-round-5
            //   filter) deferred the re-adjust while the slot's
            //   confirmed-fold exclusion stood, letting this survivor confirm
            //   the un-converged gap with receipts no other peer serves
            //   (silent confirmed divergence in committed worlds) and arming
            //   the deferred mine-down against already-confirmed history (the
            //   `WrongSavedFrame` wedge in aborted ones). The acceptance-time
            //   convergence gate (`handle_reactivate_directive`) makes a
            //   pending-held re-adjust unreachable on a full mesh; if one
            //   lands anyway (the documented N>=4 fold-pruning relay class —
            //   a lowering relayed through a peer this session cannot fold),
            //   the dedicated arm below applies the convergence WITHOUT the
            //   generic endpoint teardown and refreshes the pending's
            //   captured pre-freeze snapshot, so a later abort restores the
            //   CONVERGED freeze (`converge_reserved_slot_freeze`).
            // - REOPENED (skipped): in a committed world the commit re-seeds
            //   `{connected, F - 1}` mesh-wide and the freeze era is over; in
            //   an aborted world the restore re-asserts the (refreshed)
            //   captured freeze and the next pending-free call of this fold
            //   re-derives any convergence the reopened window deferred —
            //   recompute-per-call, nothing latched. Residual (documented): a
            //   lowering that first arrives DURING the reopened window
            //   requires the same N>=4 relay double-failure (acceptance was
            //   convergence-gated and full-mesh claims are monotone-down),
            //   and its post-restore re-adjust then targets confirmed history
            //   — the identical wedge the relay residual produces through the
            //   generic disconnect path with no attempt involved.
            #[cfg(feature = "hot-join")]
            if self
                .hot_join
                .pending_reactivation
                .as_ref()
                .is_some_and(|pending| pending.reopened && pending.handle == handle)
            {
                continue;
            }

            let mut queue_connected = true;
            let mut queue_min_confirmed = Frame::new(i32::MAX);

            // check all player connection status for every remote player
            //
            // Fold alignment (N-peer hot-join): like
            // `remote_slot_confirmed_bound`, this fold skips reserved hot-join
            // endpoints. A reserved (or N-peer rearmed) endpoint that reaches
            // `Running` before its joiner activates holds a freshly reset
            // `{connected, NULL}` status cache for EVERY slot; folding it would
            // block `queue_connected` from ever flipping (vetoing mesh
            // disconnect agreement) AND mine `queue_min_confirmed` down to
            // `NULL` (corrupting the converged freeze frame). For 2-peer
            // topologies the skip is outcome-identical: the only remote
            // endpoint being reserved leaves the fold empty, so
            // `queue_connected` keeps its `true` initializer — exactly what
            // folding the reserved endpoint's all-connected default produced.
            for endpoint in self.player_reg.remotes.values() {
                if !endpoint.is_running() {
                    continue;
                }
                #[cfg(feature = "hot-join")]
                if self.hot_join.endpoint_is_reserved(endpoint) {
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
                    // A reserved hot-join slot (the coordinator's rearmed
                    // slot, a survivor's accepted-or-cancelled attempt slot)
                    // converges WITHOUT the generic endpoint teardown: the
                    // slot's registry endpoint is the (re-)armed JOINER
                    // channel, and `disconnect()`ing it would brick the
                    // slot's rejoinability (terminal state, and the Suppress
                    // re-adjust path never re-arms). The pre-reopen-pending
                    // case is the same route (acceptance reserves the slot).
                    // See `converge_reserved_slot_freeze`.
                    #[cfg(feature = "hot-join")]
                    if self.hot_join.reserved_slots.contains(&handle)
                        || self
                            .hot_join
                            .pending_reactivation
                            .as_ref()
                            .is_some_and(|pending| !pending.reopened && pending.handle == handle)
                    {
                        self.converge_reserved_slot_freeze(handle, queue_min_confirmed);
                        continue;
                    }
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
                // N-peer survivor: a REOPENED pending reactivation whose JOINER
                // endpoint died is closed locally FIRST — the abort-evidence
                // arm re-freezes + re-reserves the slot (the reserved-slot
                // branch below then swallows the event), while the
                // commit-evidence arm clears the pending and falls through to
                // the ordinary drop of the live slot. See the method docs.
                #[cfg(feature = "hot-join")]
                self.close_reopened_pending_on_joiner_endpoint_death(&addr);
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
                    // `abort_hot_join_serve`. The N-peer counterpart additionally
                    // fans `JoinAborted` out to survivors that may already have
                    // reopened. On a survivor, a pre-reopen pending reactivation
                    // for these handles stays pending: the slot is already
                    // frozen/reserved and the coordinator's abort timeline owns
                    // the attempt.
                    for handle in player_handles.iter() {
                        self.abort_hot_join_serve(*handle);
                        self.abort_npeer_serve_for_handle(*handle);
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
    // N0 freeze barrier: `remote_slot_confirmed_bound` unit tests. The helper
    // is the gossip-min bound `confirmed_frame()` folds for every remote slot
    // (GGPO PollNPlayers semantics): `Some(min(local, gossiped views))` while
    // the slot is connected, `Some(min over gossiped views ONLY)` while it is
    // locally disconnected but not yet mesh-agreed (the local term is dropped
    // — the liveness-critical amendment, test (h)), `None` (excluded) once
    // the disconnect is mesh-agreed.
    // ======================================================================

    /// (a) Mesh-agreed exclusion: the slot is locally disconnected AND every
    /// running endpoint also reports it disconnected -> `None` (the slot leaves
    /// the confirmed-frame minimum; its frozen value carries it).
    #[test]
    fn remote_slot_confirmed_bound_mesh_agreed_disconnect_excludes_slot() {
        let (mut session, addr_b, addr_c, addr_d) = build_abcd_live_session();
        let c = PlayerHandle::new(2);

        // C dropped the production way: endpoint disconnected + local status
        // disconnected at the agreed frame.
        session
            .player_reg
            .remotes
            .get_mut(&addr_c)
            .expect("C endpoint")
            .disconnect();
        session.local_connect_status[c.as_usize()] = ConnectionStatus {
            disconnected: true,
            last_frame: Frame::new(5),
        };
        // Both running survivors gossip the slot as disconnected.
        for addr in [addr_b, addr_d] {
            session
                .player_reg
                .remotes
                .get_mut(&addr)
                .expect("survivor endpoint")
                .set_peer_connect_status_for_tests(
                    c,
                    ConnectionStatus {
                        disconnected: true,
                        last_frame: Frame::new(5),
                    },
                );
        }

        let status = session.local_connect_status[c.as_usize()];
        assert_eq!(
            session.remote_slot_confirmed_bound(c, &status),
            None,
            "a mesh-agreed disconnected slot must be excluded from the confirmed minimum"
        );
    }

    /// (b) Pre-agreement hold: the slot is locally disconnected (frozen high on
    /// direct detection) but one running endpoint still reports it connected ->
    /// `Some(min over gossiped views ONLY)` (the local term is dropped — GGPO
    /// `PollNPlayers` parity). Driving `update_player_disconnects` afterwards
    /// proves the bound never exceeds the converged freeze frame — the
    /// convergence override is folded over exactly the same endpoint terms, so
    /// `bound <= converged`. (Mutating the helper to skip the gossip fold —
    /// e.g. returning the bare local term 9 — flips the first `<=` assert;
    /// mutating it to exclude any locally-disconnected slot pre-agreement —
    /// pre-fix semantics — makes the bound `None` and flips the `Some`
    /// asserts. The complementary mutation — folding `min(local, gossip)`
    /// instead of gossip-only — is flipped by test (h) below.)
    #[test]
    fn remote_slot_confirmed_bound_pre_agreement_never_exceeds_convergence() {
        let (mut session, addr_b, addr_c, addr_d) = build_abcd_live_session();
        let c = PlayerHandle::new(2);

        // C locally dropped at our own HIGH receipt (9); its endpoint is down.
        session
            .player_reg
            .remotes
            .get_mut(&addr_c)
            .expect("C endpoint")
            .disconnect();
        session.local_connect_status[c.as_usize()] = ConnectionStatus {
            disconnected: true,
            last_frame: Frame::new(9),
        };
        // B has NOT detected yet: stale connected view at its low receipt 4.
        session
            .player_reg
            .remotes
            .get_mut(&addr_b)
            .expect("B endpoint")
            .set_peer_connect_status_for_tests(
                c,
                ConnectionStatus {
                    disconnected: false,
                    last_frame: Frame::new(4),
                },
            );
        // D already froze C at 6.
        session
            .player_reg
            .remotes
            .get_mut(&addr_d)
            .expect("D endpoint")
            .set_peer_connect_status_for_tests(
                c,
                ConnectionStatus {
                    disconnected: true,
                    last_frame: Frame::new(6),
                },
            );

        // Step 1: pre-agreement bound = gossip-only min(B 4, D 6) = 4 (the
        // local detection value 9 is dropped from the fold; it would not have
        // changed the value here anyway — test (h) pins the case where it would).
        let status = session.local_connect_status[c.as_usize()];
        let bound = session.remote_slot_confirmed_bound(c, &status);
        assert_eq!(
            bound,
            Some(Frame::new(4)),
            "pre-agreement bound must fold the still-connected survivor's lagging gossip"
        );

        // Convergence step 1: update_player_disconnects mines the local view to
        // the same fold minimum; the bound must not have exceeded it.
        session.update_player_disconnects();
        let converged = session.local_connect_status[c.as_usize()].last_frame;
        assert_eq!(converged, Frame::new(4), "convergence must agree at 4");
        assert!(
            bound.expect("bound is Some pre-agreement") <= converged,
            "the confirmed bound must never exceed the converged freeze frame"
        );

        // Step 2: B later detects at an even lower receipt (2) and gossips it.
        session
            .player_reg
            .remotes
            .get_mut(&addr_b)
            .expect("B endpoint")
            .set_peer_connect_status_for_tests(
                c,
                ConnectionStatus {
                    disconnected: true,
                    last_frame: Frame::new(2),
                },
            );
        let status = session.local_connect_status[c.as_usize()];
        let bound = session.remote_slot_confirmed_bound(c, &status);
        // All running endpoints now report disconnected -> mesh-agreed; but the
        // bound contract still holds through the final convergence step.
        assert_eq!(
            bound, None,
            "once every running endpoint reports the slot disconnected, it is mesh-agreed"
        );
        session.update_player_disconnects();
        assert_eq!(
            session.local_connect_status[c.as_usize()].last_frame,
            Frame::new(2),
            "the relayed lowering must still converge the local view down to 2"
        );
    }

    /// (c) Conservative hold: running endpoints with the default
    /// `{connected, NULL}` status cache (no gossip received yet) pin the bound
    /// at `Frame::NULL` — confirmation holds until real gossip arrives.
    /// (Mutating the helper to skip NULL cache entries would return `Some(7)`.)
    #[test]
    fn remote_slot_confirmed_bound_default_null_cache_holds_conservatively() {
        let (mut session, _addr_b, _addr_c, _addr_d) = build_abcd_live_session();
        let c = PlayerHandle::new(2);

        // We received C through frame 7, but no endpoint has gossiped any view
        // yet (all caches are the default `{connected, NULL}`).
        session.local_connect_status[c.as_usize()] = ConnectionStatus {
            disconnected: false,
            last_frame: Frame::new(7),
        };

        let status = session.local_connect_status[c.as_usize()];
        assert_eq!(
            session.remote_slot_confirmed_bound(c, &status),
            Some(Frame::NULL),
            "default (NULL) gossip caches must hold the bound at NULL until gossip lands"
        );
    }

    /// (d) Non-running endpoints leave the fold: a survivor endpoint that is
    /// Synchronizing (e.g. mid-rearm) contributes nothing. Toggling ONLY its
    /// running state flips the bound between the third party's view and its
    /// own lower view — the `is_running()` filter is the load-bearing line.
    #[test]
    fn remote_slot_confirmed_bound_ignores_non_running_endpoints() {
        fn bound_with_b_running(b_running: bool) -> Option<Frame> {
            let (mut session, addr_b, addr_c, addr_d) = build_abcd_live_session();
            let c = PlayerHandle::new(2);

            // Local receipt of C: 7. C's own endpoint self-claims 9.
            session.local_connect_status[c.as_usize()] = ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(7),
            };
            session
                .player_reg
                .remotes
                .get_mut(&addr_c)
                .expect("C endpoint")
                .set_peer_connect_status_for_tests(
                    c,
                    ConnectionStatus {
                        disconnected: false,
                        last_frame: Frame::new(9),
                    },
                );
            // D gossips C through 5; B holds a LOWER view (2).
            session
                .player_reg
                .remotes
                .get_mut(&addr_d)
                .expect("D endpoint")
                .set_peer_connect_status_for_tests(
                    c,
                    ConnectionStatus {
                        disconnected: false,
                        last_frame: Frame::new(5),
                    },
                );
            {
                let b_endpoint = session
                    .player_reg
                    .remotes
                    .get_mut(&addr_b)
                    .expect("B endpoint");
                b_endpoint.set_peer_connect_status_for_tests(
                    c,
                    ConnectionStatus {
                        disconnected: false,
                        last_frame: Frame::new(2),
                    },
                );
                if b_running {
                    b_endpoint.force_running_for_tests();
                } else {
                    b_endpoint.force_synchronizing_for_tests();
                }
            }

            let status = session.local_connect_status[c.as_usize()];
            session.remote_slot_confirmed_bound(c, &status)
        }

        assert_eq!(
            bound_with_b_running(false),
            Some(Frame::new(5)),
            "a non-running endpoint's view must be excluded from the bound"
        );
        assert_eq!(
            bound_with_b_running(true),
            Some(Frame::new(2)),
            "the same endpoint's lower view must be folded once it is running"
        );
    }

    /// (e) Hot-join reserved endpoints are skipped: a reserved endpoint can sit
    /// `Running` with a freshly reset default `{connected, NULL}` cache before
    /// its joiner activates. Folding it would pin the bound at NULL forever
    /// (an abandoned join would kill the host). With the guard: a locally
    /// disconnected reserved slot is mesh-agreed-excluded (`None`, the host
    /// runs solo on the frozen value), and a connected slot falls back to the
    /// local receipt.
    #[test]
    #[cfg(feature = "hot-join")]
    fn remote_slot_confirmed_bound_skips_reserved_hot_join_endpoint() {
        let addr = test_addr(9501);
        let mut host = build_hot_join_serving_host(addr);
        let dropped = PlayerHandle::new(1);
        host.player_reg
            .remotes
            .get_mut(&addr)
            .expect("remote endpoint must exist at build time")
            .force_running_for_tests();
        host.state = SessionState::Running;

        // Cleanly drop the remote: ContinueWithout on a serving host re-arms
        // the slot (reserved + endpoint rebuilt with a default cache).
        host.remove_player(dropped)
            .expect("remove_player should succeed");
        assert!(
            host.hot_join.reserved_slots.contains(&dropped),
            "the dropped slot must be re-reserved on a serving host"
        );
        // Manufacture the post-handshake window: the reserved endpoint is
        // Running (joiner synchronized, not yet activated) with the rebuilt
        // default `{connected, NULL}` cache.
        {
            let endpoint = host
                .player_reg
                .remotes
                .get_mut(&addr)
                .expect("re-armed endpoint must exist");
            endpoint.force_running_for_tests();
            assert!(endpoint.is_running(), "endpoint must be Running");
            assert!(
                host.hot_join.endpoint_is_reserved(endpoint),
                "the endpoint must read as reserved"
            );
        }

        // Locally-disconnected reserved slot: the reserved endpoint is guarded
        // out, the fold is empty -> mesh-agreed exclusion (None). Without the
        // guard the default `{connected, NULL}` cache would force Some(NULL),
        // pinning the host's confirmed frame at NULL forever.
        let status = host.local_connect_status[dropped.as_usize()];
        assert!(
            status.disconnected,
            "the dropped slot must be locally disconnected"
        );
        assert_eq!(
            host.remote_slot_confirmed_bound(dropped, &status),
            None,
            "a reserved endpoint must not resurrect a dropped slot's confirmed contribution"
        );
        // And the host's confirmed frame falls back to its own (local) slot.
        host.local_connect_status[0].last_frame = Frame::new(12);
        assert_eq!(
            host.confirmed_frame(),
            Frame::new(12),
            "a solo host with a reserved slot must confirm on its local slot alone"
        );

        // Connected variant: if the slot were still connected, the guarded-out
        // reserved endpoint leaves the fold empty and the bound collapses to
        // the local receipt.
        let connected_status = ConnectionStatus {
            disconnected: false,
            last_frame: Frame::new(5),
        };
        assert_eq!(
            host.remote_slot_confirmed_bound(dropped, &connected_status),
            Some(Frame::new(5)),
            "with only a reserved endpoint the bound must equal the local receipt"
        );
    }

    /// (f) `N == 2` identity (the byte-parity requirement): the only remote
    /// slot's gossip term is the peer's SELF-claim, which always covers the
    /// inputs it sent (self-claim >= our receipt), so the min collapses to
    /// today's local-receipt value. After the remote drops, its endpoint is
    /// not running, the fold is empty and the slot is excluded exactly as
    /// before the barrier.
    #[test]
    fn remote_slot_confirmed_bound_n2_identity() {
        let mut session = create_two_player_session();
        let remote = PlayerHandle::new(1);
        let addr = test_addr(8080);
        {
            let endpoint = session
                .player_reg
                .remotes
                .get_mut(&addr)
                .expect("remote endpoint must exist");
            endpoint.force_running_for_tests();
            // Self-claim 9 >= our receipt 7 (a packet carrying inputs through
            // frame k always carries a self-claim >= k).
            endpoint.set_peer_connect_status_for_tests(
                remote,
                ConnectionStatus {
                    disconnected: false,
                    last_frame: Frame::new(9),
                },
            );
        }
        session.local_connect_status[remote.as_usize()] = ConnectionStatus {
            disconnected: false,
            last_frame: Frame::new(7),
        };

        let status = session.local_connect_status[remote.as_usize()];
        assert_eq!(
            session.remote_slot_confirmed_bound(remote, &status),
            Some(Frame::new(7)),
            "at N=2 the bound must collapse to the local receipt (byte-parity with pre-barrier)"
        );

        // Post-drop: endpoint disconnected -> fold empty -> excluded (None).
        session
            .player_reg
            .remotes
            .get_mut(&addr)
            .expect("remote endpoint must exist")
            .disconnect();
        session.local_connect_status[remote.as_usize()] = ConnectionStatus {
            disconnected: true,
            last_frame: Frame::new(7),
        };
        let status = session.local_connect_status[remote.as_usize()];
        assert_eq!(
            session.remote_slot_confirmed_bound(remote, &status),
            None,
            "after the N=2 remote drops, the slot must be excluded exactly as pre-barrier"
        );
    }

    /// (g) Flavor-X closure: a CONNECTED slot at N>=3 is bounded by a lagging
    /// third party's gossiped view — even though we received the slot through a
    /// much higher frame, the bound equals the lagging gossip, so our confirmed
    /// frame can never discard the input at the freeze frame the mesh may later
    /// agree on. (Pre-fix `confirmed_frame()` used only the local 8.)
    #[test]
    fn remote_slot_confirmed_bound_lagging_third_party_gossip_bounds_connected_slot() {
        let (mut session, addr_b, addr_c, addr_d) = build_abcd_live_session();
        let c = PlayerHandle::new(2);

        // We received C through 8; C self-claims 8; D agrees at 8; B lags at 4
        // (the C->B link is lossy). Everyone still believes C is connected.
        session.local_connect_status[c.as_usize()] = ConnectionStatus {
            disconnected: false,
            last_frame: Frame::new(8),
        };
        for (addr, frame) in [(addr_c, 8), (addr_d, 8), (addr_b, 4)] {
            session
                .player_reg
                .remotes
                .get_mut(&addr)
                .expect("endpoint must exist")
                .set_peer_connect_status_for_tests(
                    c,
                    ConnectionStatus {
                        disconnected: false,
                        last_frame: Frame::new(frame),
                    },
                );
        }

        let status = session.local_connect_status[c.as_usize()];
        assert_eq!(
            session.remote_slot_confirmed_bound(c, &status),
            Some(Frame::new(4)),
            "a connected slot's bound must equal the lagging third-party gossip (flavor-X closure)"
        );
    }

    /// (h) Gossip-only fold for a locally-disconnected slot (GGPO
    /// `PollNPlayers` parity) — THE liveness-critical arm: with the slot
    /// locally disconnected at our LOW receipt 3 and the survivors still
    /// reporting it connected at 7, the bound is `Some(7)` (gossip-only), NOT
    /// `Some(3)`. A `min`-with-local mutation flips this assert and re-pins a
    /// capped survivor against its own detection value: connect-status gossip
    /// travels only in Input messages, so a survivor capped at
    /// `bound + max_prediction` with a fully-acked send queue cannot send the
    /// `disconnected` gossip through ordinary traffic — before the
    /// connect-status nudge existed that was a permanent deadlock, and even
    /// with the nudge it would hold every staggered release hostage to the
    /// nudge cadence instead of releasing immediately.
    ///
    /// Exceeding the local freeze value 3 is sound (Case 1 of the helper's
    /// rustdoc): frames above 3 are served by the frozen-value branch, and if
    /// the mesh later converges AT 3 (our own detection value, mined into the
    /// others via our gossip), no re-roll below 3 is ever needed — the frozen
    /// value was captured at detection. The second leg drives a real
    /// convergence step after D detects at 5: the convergence override is
    /// folded over endpoint terms only (B 7, D 5 -> 5), which is NOT below our
    /// local 3, so the local view stays 3 — the bound exceeding an
    /// eventually-agreed `F == L_local` never requires discarded data.
    #[test]
    fn remote_slot_confirmed_bound_locally_disconnected_folds_gossip_only() {
        let (mut session, addr_b, addr_c, addr_d) = build_abcd_live_session();
        let c = PlayerHandle::new(2);

        // C dropped the production way at our own LOW receipt (3); its
        // endpoint is down.
        session
            .player_reg
            .remotes
            .get_mut(&addr_c)
            .expect("C endpoint")
            .disconnect();
        session.local_connect_status[c.as_usize()] = ConnectionStatus {
            disconnected: true,
            last_frame: Frame::new(3),
        };
        // Both survivors received C through 7 and still believe it connected.
        for addr in [addr_b, addr_d] {
            session
                .player_reg
                .remotes
                .get_mut(&addr)
                .expect("survivor endpoint")
                .set_peer_connect_status_for_tests(
                    c,
                    ConnectionStatus {
                        disconnected: false,
                        last_frame: Frame::new(7),
                    },
                );
        }

        let status = session.local_connect_status[c.as_usize()];
        assert_eq!(
            session.remote_slot_confirmed_bound(c, &status),
            Some(Frame::new(7)),
            "a locally-disconnected (pre-agreement) slot must fold the gossiped views ONLY \
             — folding the local detection value 3 re-pins capped survivors"
        );

        // Second leg: D detects at 5 while B still reports connected at 7. The
        // gossip-only bound follows the endpoint fold down to 5, and a real
        // convergence step applies an endpoint-terms-only override (5), which
        // never mines our local view (3) below the bound's floor.
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
        let status = session.local_connect_status[c.as_usize()];
        assert_eq!(
            session.remote_slot_confirmed_bound(c, &status),
            Some(Frame::new(5)),
            "the gossip-only bound must track the endpoint fold (min(B 7, D 5) = 5)"
        );
        session.update_player_disconnects();
        assert_eq!(
            session.local_connect_status[c.as_usize()].last_frame,
            Frame::new(3),
            "an endpoint-terms override (5) above our own detection value (3) must not \
             re-raise the local view — Case 1 convergence lands at our original 3"
        );
    }

    /// (i, session wiring) `poll_remote_clients` arms the protocol-level
    /// connect-status nudge on every remote endpoint exactly while some remote
    /// slot is locally disconnected but NOT yet mesh-agreed
    /// (`connect_status_nudge_needed`), and disarms it the poll after the mesh
    /// agrees. Without this wiring, capped gossip-mute survivors can never
    /// deliver their `disconnected` view and mesh agreement is unreachable —
    /// the clean-drop liveness pin (see the integration repros in
    /// `tests/sessions/peer_drop.rs`).
    #[test]
    fn poll_remote_clients_sets_connect_status_nudge_until_mesh_agreement() {
        let (mut session, addr_b, addr_c, addr_d) = build_abcd_live_session();
        let c = PlayerHandle::new(2);

        // No disconnect anywhere: polling must keep the nudge disarmed.
        session.poll_remote_clients();
        for addr in [addr_b, addr_d] {
            assert!(
                !session
                    .player_reg
                    .remotes
                    .get(&addr)
                    .expect("survivor endpoint")
                    .connect_status_nudge_for_tests(),
                "no nudge may be armed while every slot is connected"
            );
        }

        // C dropped the production way (endpoint down + local status
        // disconnected) while both survivors still gossip it connected: the
        // drop is NOT mesh-agreed, so polling must arm the nudge.
        session
            .player_reg
            .remotes
            .get_mut(&addr_c)
            .expect("C endpoint")
            .disconnect();
        session.local_connect_status[c.as_usize()] = ConnectionStatus {
            disconnected: true,
            last_frame: Frame::new(5),
        };
        for addr in [addr_b, addr_d] {
            session
                .player_reg
                .remotes
                .get_mut(&addr)
                .expect("survivor endpoint")
                .set_peer_connect_status_for_tests(
                    c,
                    ConnectionStatus {
                        disconnected: false,
                        last_frame: Frame::new(5),
                    },
                );
        }
        session.poll_remote_clients();
        for addr in [addr_b, addr_d] {
            assert!(
                session
                    .player_reg
                    .remotes
                    .get(&addr)
                    .expect("survivor endpoint")
                    .connect_status_nudge_for_tests(),
                "the nudge must be armed while the drop awaits mesh agreement"
            );
        }

        // Both survivors now gossip the slot disconnected: mesh-agreed — the
        // next poll must disarm the nudge.
        for addr in [addr_b, addr_d] {
            session
                .player_reg
                .remotes
                .get_mut(&addr)
                .expect("survivor endpoint")
                .set_peer_connect_status_for_tests(
                    c,
                    ConnectionStatus {
                        disconnected: true,
                        last_frame: Frame::new(5),
                    },
                );
        }
        session.poll_remote_clients();
        for addr in [addr_b, addr_d] {
            assert!(
                !session
                    .player_reg
                    .remotes
                    .get(&addr)
                    .expect("survivor endpoint")
                    .connect_status_nudge_for_tests(),
                "the nudge must be disarmed once the drop is mesh-agreed"
            );
        }
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

    // ==========================================
    // N-peer mesh coordination tests (chunks N2 + N3)
    // ==========================================
    //
    // These exercise the coordinator-side N-peer serve orchestration and the
    // survivor-side reactivation response with a real 3-peer mesh: coordinator
    // A (local 0), survivor B (local 1), and joiner C (local 2). Sessions are
    // wired through an in-src deterministic routing bus (instant, loss-free
    // delivery with selective per-message-kind blocking) and a manually
    // advanced clock injected via `ProtocolConfig::clock`.
    //
    // The N>=3 build guards STAY in force for the public API; the coordinator
    // is built through the `#[cfg(test)]`-only
    // `start_p2p_session_skip_hot_join_build_guards_for_test` bypass.
    //
    // The joiner role is driven MANUALLY through raw `UdpProtocol` endpoints
    // (sync handshake, `JoinRequest`, `StateSnapshotAck`, real inputs): the
    // real joiner-session apply path (buffer-then-apply on `JoinCommitted`,
    // bridge-frame simulation, un-defer-all) is chunk N4 and intentionally
    // absent. Manual driving also lets tests withhold exactly one protocol
    // step (e.g. never ack the snapshot) to force abort paths
    // deterministically.
    #[cfg(feature = "hot-join")]
    mod npeer_mesh {
        use super::*;
        use crate::network::messages::{JoinAborted, JoinCommitted, MessageBody, ReactivateSlot};
        use crate::sessions::config::SyncConfig;
        use crate::time_sync::TimeSyncConfig;
        use crate::InputStatus;
        use std::collections::{BTreeSet, VecDeque};
        use std::sync::Mutex;
        use web_time::{Duration, Instant};

        const POLL_INTERVAL: Duration = Duration::from_millis(50);

        fn addr_a() -> SocketAddr {
            test_addr(9301)
        }
        fn addr_b() -> SocketAddr {
            test_addr(9302)
        }
        fn addr_c() -> SocketAddr {
            test_addr(9303)
        }
        fn addr_d() -> SocketAddr {
            test_addr(9304)
        }

        /// C's constant pre-drop input: the agreed frozen value for its slot is
        /// therefore exactly this, independent of the precise freeze frame.
        const C_FROZEN_INPUT: u8 = 37;
        /// C's post-rejoin input — distinct from the frozen value so every
        /// "real inputs flow" assertion is non-vacuous.
        const C_REJOIN_INPUT: u8 = 99;

        // ------------------------------------------------------------------
        // Deterministic in-src infrastructure (clock + routing bus)
        // ------------------------------------------------------------------

        /// Manually advanced clock shared by every protocol in a test via
        /// `ProtocolConfig::clock` (in-src analog of the integration tests'
        /// `TestClock`).
        #[derive(Clone)]
        struct MeshClock {
            now: Arc<Mutex<Instant>>,
            /// Deterministic [`ProtocolConfig::protocol_rng_seed`] dispenser
            /// (see [`MeshClock::protocol_config`]).
            next_protocol_seed: Arc<std::sync::atomic::AtomicU64>,
        }

        impl MeshClock {
            fn new() -> Self {
                Self {
                    now: Arc::new(Mutex::new(Instant::now())),
                    next_protocol_seed: Arc::new(std::sync::atomic::AtomicU64::new(1)),
                }
            }

            fn advance(&self, duration: Duration) {
                *self.now.lock().expect("MeshClock mutex poisoned") += duration;
            }

            /// Returns a protocol config carrying the injected mock clock AND
            /// a distinct, deterministic `protocol_rng_seed` per call
            /// (single-threaded fixtures construct sessions/endpoints in a
            /// fixed order, so seed assignment is reproducible). Seeding
            /// removes the harness's last entropy-fed input: without it,
            /// protocol magic numbers and sync-request randoms come from the
            /// thread-local RNG, which seeds itself from wall-clock timing
            /// entropy — every other input here is already virtualized by the
            /// injected clock and the in-memory bus (session-33 round-6 test
            /// hermeticity).
            fn protocol_config(&self) -> ProtocolConfig {
                let now = Arc::clone(&self.now);
                ProtocolConfig {
                    clock: Some(Arc::new(move || {
                        *now.lock().expect("MeshClock mutex poisoned")
                    })),
                    protocol_rng_seed: Some(
                        self.next_protocol_seed
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                    ),
                    ..ProtocolConfig::default()
                }
            }
        }

        /// Returns a stable name for a message body, used by the bus's
        /// selective drop filter (in-src code can match `MessageBody`
        /// directly).
        fn body_kind(body: &MessageBody) -> &'static str {
            match body {
                MessageBody::SyncRequest(_) => "SyncRequest",
                MessageBody::SyncReply(_) => "SyncReply",
                MessageBody::Input(_) => "Input",
                MessageBody::InputAck(_) => "InputAck",
                MessageBody::QualityReport(_) => "QualityReport",
                MessageBody::QualityReply(_) => "QualityReply",
                MessageBody::ChecksumReport(_) => "ChecksumReport",
                MessageBody::KeepAlive => "KeepAlive",
                MessageBody::JoinRequest(_) => "JoinRequest",
                MessageBody::StateSnapshot(_) => "StateSnapshot",
                MessageBody::StateSnapshotAck(_) => "StateSnapshotAck",
                MessageBody::ReactivateSlot(_) => "ReactivateSlot",
                MessageBody::ReactivateSlotAck(_) => "ReactivateSlotAck",
                MessageBody::JoinCommitted(_) => "JoinCommitted",
                MessageBody::JoinAborted(_) => "JoinAborted",
            }
        }

        type Inboxes = BTreeMap<SocketAddr, VecDeque<(SocketAddr, Message)>>;
        type BlockSet = BTreeSet<(SocketAddr, SocketAddr, &'static str)>;

        /// Shared in-memory routing bus: any number of [`MeshSocket`]s attach
        /// at addresses over time (a vacated address can be re-attached by a
        /// returning joiner), delivery is instant and deterministic, and
        /// `(from, to, kind)` triples can be selectively blocked.
        #[derive(Clone, Default)]
        struct MeshBus {
            inboxes: Arc<Mutex<Inboxes>>,
            blocked: Arc<Mutex<BlockSet>>,
        }

        impl MeshBus {
            fn new() -> Self {
                Self::default()
            }

            fn socket(&self, addr: SocketAddr) -> MeshSocket {
                MeshSocket {
                    addr,
                    bus: self.clone(),
                }
            }

            fn block(&self, from: SocketAddr, to: SocketAddr, kind: &'static str) {
                self.blocked
                    .lock()
                    .expect("MeshBus mutex poisoned")
                    .insert((from, to, kind));
            }

            fn unblock(&self, from: SocketAddr, to: SocketAddr, kind: &'static str) {
                self.blocked
                    .lock()
                    .expect("MeshBus mutex poisoned")
                    .remove(&(from, to, kind));
            }
        }

        struct MeshSocket {
            addr: SocketAddr,
            bus: MeshBus,
        }

        impl NonBlockingSocket<SocketAddr> for MeshSocket {
            fn send_to(&mut self, msg: &Message, addr: &SocketAddr) {
                if self
                    .bus
                    .blocked
                    .lock()
                    .expect("MeshBus mutex poisoned")
                    .contains(&(self.addr, *addr, body_kind(&msg.body)))
                {
                    return;
                }
                self.bus
                    .inboxes
                    .lock()
                    .expect("MeshBus mutex poisoned")
                    .entry(*addr)
                    .or_default()
                    .push_back((self.addr, msg.clone()));
            }

            fn receive_all_messages(&mut self) -> Vec<(SocketAddr, Message)> {
                self.bus
                    .inboxes
                    .lock()
                    .expect("MeshBus mutex poisoned")
                    .get_mut(&self.addr)
                    .map(|queue| queue.drain(..).collect())
                    .unwrap_or_default()
            }
        }

        // ------------------------------------------------------------------
        // Shadow game state (the "user side" of the request contract)
        // ------------------------------------------------------------------

        /// Minimal deterministic game: the state folds every advanced frame's
        /// inputs, and every save/load goes through the request cells — so two
        /// peers' shadow states at a frame are byte-equal iff they simulated
        /// identical input streams.
        #[derive(Default)]
        struct Shadow {
            state: u8,
            /// Frame -> state captured by that frame's `SaveGameState`
            /// (rollback re-saves overwrite — the repaired truth).
            states: BTreeMap<i32, u8>,
        }

        fn next_state(state: u8, inputs: &[(u8, InputStatus)]) -> u8 {
            inputs
                .iter()
                .fold(state.wrapping_mul(31).wrapping_add(7), |acc, (inp, _)| {
                    acc.wrapping_add(*inp)
                })
        }

        fn apply_requests(requests: &RequestVec<TestConfig>, shadow: &mut Shadow) {
            for request in requests.iter() {
                match request {
                    FortressRequest::SaveGameState { cell, frame } => {
                        cell.save(*frame, Some(shadow.state), Some(u128::from(shadow.state)));
                        shadow.states.insert(frame.as_i32(), shadow.state);
                    },
                    FortressRequest::LoadGameState { cell, .. } => {
                        shadow.state = cell.load().expect("loaded cell must hold a state");
                    },
                    FortressRequest::AdvanceFrame { inputs } => {
                        shadow.state = next_state(shadow.state, inputs);
                    },
                }
            }
        }

        // ------------------------------------------------------------------
        // Manual joiner (chunk N4 stands in for the real joiner session)
        // ------------------------------------------------------------------

        /// Drives the joiner role at the protocol level: sync handshakes,
        /// `JoinRequest`, snapshot receipt + ack, lifecycle drains, and real
        /// inputs for slot 2. See the module docs for why this is manual.
        struct ManualJoiner {
            // Boxed because `UdpProtocol::send_all_messages` takes
            // `&mut Box<dyn NonBlockingSocket<_>>`.
            socket: Box<dyn NonBlockingSocket<SocketAddr>>,
            protos: BTreeMap<SocketAddr, UdpProtocol<TestConfig>>,
            status: Vec<ConnectionStatus>,
        }

        impl ManualJoiner {
            fn new(bus: &MeshBus, addr: SocketAddr) -> Self {
                Self {
                    socket: Box::new(bus.socket(addr)),
                    protos: BTreeMap::new(),
                    status: vec![ConnectionStatus::default(); 3],
                }
            }

            /// Creates + starts synchronizing a protocol endpoint toward
            /// `peer` (which owns player `peer_handle`).
            fn connect(&mut self, peer: SocketAddr, peer_handle: usize, clock: &MeshClock) {
                let mut proto = UdpProtocol::<TestConfig>::new(
                    vec![PlayerHandle::new(peer_handle)],
                    peer,
                    3, // num_players
                    1, // local players (the joiner's own slot)
                    8, // max_prediction
                    Duration::from_secs(2),
                    Duration::from_millis(500),
                    60,
                    DesyncDetection::Off,
                    SyncConfig::default(),
                    clock.protocol_config(),
                    TimeSyncConfig::default(),
                )
                .expect("manual joiner protocol should construct");
                proto.synchronize().expect("fresh protocol synchronizes");
                self.protos.insert(peer, proto);
            }

            fn pump(&mut self) {
                for (from, msg) in self.socket.receive_all_messages() {
                    if let Some(proto) = self.protos.get_mut(&from) {
                        proto.handle_message(&msg);
                    }
                }
                for proto in self.protos.values_mut() {
                    // Drain (and drop) protocol events; the manual joiner only
                    // needs the state machine driven.
                    let _ = proto.poll(&self.status).count();
                    proto.send_all_messages(&mut self.socket);
                }
            }

            fn proto_mut(&mut self, peer: SocketAddr) -> &mut UdpProtocol<TestConfig> {
                self.protos
                    .get_mut(&peer)
                    .expect("manual joiner protocol exists for peer")
            }

            fn is_running(&self, peer: SocketAddr) -> bool {
                self.protos.get(&peer).is_some_and(UdpProtocol::is_running)
            }

            /// Sends slot 2's real input for `frame` to every connected peer,
            /// gossiping `last_frame = frame` for all slots (a live joiner's
            /// connect-status claims; the gossip-min folds keep the claim
            /// bounded by each receiver's own receipt, so an optimistic claim
            /// is safe).
            fn send_input(&mut self, frame: Frame, value: u8) {
                for status in &mut self.status {
                    status.last_frame = frame;
                }
                let mut inputs = BTreeMap::new();
                inputs.insert(PlayerHandle::new(2), PlayerInput::new(frame, value));
                let status = self.status.clone();
                for proto in self.protos.values_mut() {
                    proto.send_input(&inputs, &status);
                    proto.send_all_messages(&mut self.socket);
                }
            }
        }

        // ------------------------------------------------------------------
        // Mesh fixture
        // ------------------------------------------------------------------

        /// Coordinator + survivor pair (C's lifecycle differs per test).
        struct Duo {
            bus: MeshBus,
            clock: MeshClock,
            a: P2PSession<TestConfig>,
            b: P2PSession<TestConfig>,
            a_shadow: Shadow,
            b_shadow: Shadow,
            a_events: Vec<FortressEvent<TestConfig>>,
            b_events: Vec<FortressEvent<TestConfig>>,
        }

        impl Duo {
            fn poll_round(&mut self, joiner: Option<&mut ManualJoiner>) {
                self.a.poll_remote_clients();
                self.a_events.extend(self.a.events());
                self.b.poll_remote_clients();
                self.b_events.extend(self.b.events());
                if let Some(joiner) = joiner {
                    joiner.pump();
                }
                self.clock.advance(POLL_INTERVAL);
            }

            /// Adds inputs + advances both sessions once (either may be
            /// throttled or paused — requests are applied regardless).
            fn advance_both(&mut self, a_input: u8, b_input: u8) {
                self.a
                    .add_local_input(PlayerHandle::new(0), a_input)
                    .expect("A local input");
                let requests = self.a.advance_frame().expect("A advance");
                apply_requests(&requests, &mut self.a_shadow);
                self.b
                    .add_local_input(PlayerHandle::new(1), b_input)
                    .expect("B local input");
                let requests = self.b.advance_frame().expect("B advance");
                apply_requests(&requests, &mut self.b_shadow);
            }
        }

        /// Builds the 3-peer mesh (A coordinator, B survivor, C full session),
        /// synchronizes it, advances `pre_drop_rounds` lockstep rounds, then
        /// gracefully drops C on both A and B and advances a few more rounds.
        /// Returns the duo, ready for a rejoin attempt.
        fn mesh_with_dropped_slot(serve_timeout_polls: usize, pre_drop_rounds: u32) -> Duo {
            mesh_with_dropped_slot_opts(serve_timeout_polls, pre_drop_rounds, false, false).0
        }

        /// [`mesh_with_dropped_slot`] with VALUE-VARYING C inputs — the
        /// round-5 de-blind: with varying inputs, any freeze-frame
        /// disagreement the staging or the machinery produces becomes
        /// byte-visible in every downstream shadow/snapshot comparison
        /// instead of being masked by the constant `C_FROZEN_INPUT`. (The
        /// staging's drop itself is one frame ASYMMETRIC — A's receipt runs
        /// one ahead of B's at the removal — so a varying-input mesh
        /// genuinely exercises the generic convergence re-adjust during the
        /// post-drop rounds.) Tests that assert the served frozen VALUE
        /// (`C_FROZEN_INPUT`) keep the constant staging, where the value is
        /// freeze-frame-independent by construction.
        fn mesh_with_dropped_slot_varying(serve_timeout_polls: usize, pre_drop_rounds: u32) -> Duo {
            mesh_with_dropped_slot_opts(serve_timeout_polls, pre_drop_rounds, false, true).0
        }

        /// [`mesh_with_dropped_slot`] with a spectator registered on the
        /// survivor B (handle 3 at `addr_d`), driven at the protocol level by
        /// the returned manual endpoint (spectator endpoints gate B's initial
        /// sync, so it must complete the handshake; the caller keeps pumping
        /// it so the endpoint stays alive for the flush under test).
        fn mesh_with_dropped_slot_with_spectator(
            serve_timeout_polls: usize,
            pre_drop_rounds: u32,
        ) -> (Duo, ManualJoiner) {
            let (duo, spectator) =
                mesh_with_dropped_slot_opts(serve_timeout_polls, pre_drop_rounds, true, false);
            (
                duo,
                spectator.expect("spectator driver exists when requested"),
            )
        }

        fn mesh_with_dropped_slot_opts(
            serve_timeout_polls: usize,
            pre_drop_rounds: u32,
            spectator_on_b: bool,
            varying_c_input: bool,
        ) -> (Duo, Option<ManualJoiner>) {
            let bus = MeshBus::new();
            let clock = MeshClock::new();

            let a = SessionBuilder::<TestConfig>::new()
                .with_num_players(3)
                .expect("num players")
                .with_protocol_config(clock.protocol_config())
                .with_hot_join(true)
                .with_hot_join_serve_timeout_polls(serve_timeout_polls)
                .expect("serve timeout")
                .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
                .add_player(PlayerType::Local, PlayerHandle::new(0))
                .expect("A local")
                .add_player(PlayerType::Remote(addr_b()), PlayerHandle::new(1))
                .expect("A remote B")
                .add_player(PlayerType::Remote(addr_c()), PlayerHandle::new(2))
                .expect("A remote C")
                // N>=3 hot-join construction is publicly build-rejected (the
                // S20 guards stay); the test-only bypass reaches the N2/N3
                // machinery under test.
                .start_p2p_session_skip_hot_join_build_guards_for_test(bus.socket(addr_a()))
                .expect("A builds");

            let mut b_builder = SessionBuilder::<TestConfig>::new()
                .with_num_players(3)
                .expect("num players")
                .with_protocol_config(clock.protocol_config())
                .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
                .add_player(PlayerType::Remote(addr_a()), PlayerHandle::new(0))
                .expect("B remote A")
                .add_player(PlayerType::Local, PlayerHandle::new(1))
                .expect("B local")
                .add_player(PlayerType::Remote(addr_c()), PlayerHandle::new(2))
                .expect("B remote C");
            if spectator_on_b {
                b_builder = b_builder
                    .add_player(PlayerType::Spectator(addr_d()), PlayerHandle::new(3))
                    .expect("B spectator");
            }
            let b = b_builder
                .start_p2p_session(bus.socket(addr_b()))
                .expect("B builds (a plain survivor needs no bypass)");

            let mut c = SessionBuilder::<TestConfig>::new()
                .with_num_players(3)
                .expect("num players")
                .with_protocol_config(clock.protocol_config())
                .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
                .add_player(PlayerType::Remote(addr_a()), PlayerHandle::new(0))
                .expect("C remote A")
                .add_player(PlayerType::Remote(addr_b()), PlayerHandle::new(1))
                .expect("C remote B")
                .add_player(PlayerType::Local, PlayerHandle::new(2))
                .expect("C local")
                .start_p2p_session(bus.socket(addr_c()))
                .expect("C builds");

            let mut duo = Duo {
                bus,
                clock,
                a,
                b,
                a_shadow: Shadow::default(),
                b_shadow: Shadow::default(),
                a_events: Vec::new(),
                b_events: Vec::new(),
            };
            let mut c_shadow = Shadow::default();

            // The spectator endpoint gates B's initial sync, so it must be
            // driven at the protocol level (the `ManualJoiner` harness speaks
            // the same `UdpProtocol` handshake a real spectator session would).
            let mut spectator = spectator_on_b.then(|| {
                let mut spectator = ManualJoiner::new(&duo.bus.clone(), addr_d());
                spectator.connect(addr_b(), 1, &duo.clock.clone());
                spectator
            });

            // Initial synchronization of all three sessions (condition-driven
            // with a generous cap — session-33 round-6).
            for _ in 0..300 {
                duo.poll_round(None);
                c.poll_remote_clients();
                let _ = c.events().count();
                if let Some(spectator) = spectator.as_mut() {
                    spectator.pump();
                }
                if duo.a.current_state() == SessionState::Running
                    && duo.b.current_state() == SessionState::Running
                    && c.current_state() == SessionState::Running
                {
                    break;
                }
            }
            assert_eq!(duo.a.current_state(), SessionState::Running, "A syncs");
            assert_eq!(duo.b.current_state(), SessionState::Running, "B syncs");
            assert_eq!(c.current_state(), SessionState::Running, "C syncs");

            // Advance the full mesh in lockstep. By default C's input is
            // CONSTANT so the slot's agreed frozen value is C_FROZEN_INPUT
            // regardless of the precise freeze frame — NOTE (round-5
            // de-blind disclosure): this constant deliberately MASKS
            // freeze-frame asymmetry (the staged drop below is in fact one
            // frame asymmetric, healed by the generic convergence re-adjust
            // during the post-drop rounds). Tests that assert the served
            // frozen VALUE need the mask; value-sensitivity is exercised by
            // `varying_c_input` (the flagship happy path) and by the
            // dedicated asymmetric staging
            // (`mesh_with_asymmetric_dropped_slot`).
            for i in 0..pre_drop_rounds {
                for _ in 0..3 {
                    duo.poll_round(None);
                    c.poll_remote_clients();
                    let _ = c.events().count();
                    if let Some(spectator) = spectator.as_mut() {
                        spectator.pump();
                    }
                }
                duo.advance_both(10 + (i as u8), 20 + (i as u8));
                let c_input = if varying_c_input {
                    c_varying_input(c.current_frame().as_i32())
                } else {
                    C_FROZEN_INPUT
                };
                c.add_local_input(PlayerHandle::new(2), c_input)
                    .expect("C local input");
                let requests = c.advance_frame().expect("C advance");
                apply_requests(&requests, &mut c_shadow);
            }
            assert!(
                duo.a.current_frame().as_i32() >= 3,
                "mesh advanced pre-drop (A at {})",
                duo.a.current_frame()
            );

            // Graceful drop of C on both survivors; C's session goes away.
            duo.a
                .remove_player(PlayerHandle::new(2))
                .expect("A removes C");
            duo.b
                .remove_player(PlayerHandle::new(2))
                .expect("B removes C");
            drop(c);
            duo.a_events.extend(duo.a.events());
            duo.b_events.extend(duo.b.events());

            // A (the hot-join host) re-armed + re-reserved the slot; B (a plain
            // survivor) left its endpoint terminal.
            assert!(
                duo.a
                    .hot_join
                    .reserved_slots
                    .contains(&PlayerHandle::new(2)),
                "A re-reserves the dropped slot"
            );
            assert!(
                !duo.b
                    .hot_join
                    .reserved_slots
                    .contains(&PlayerHandle::new(2)),
                "B does not reserve (plain survivor)"
            );

            // Both keep advancing with the slot frozen.
            for i in 0..4_u8 {
                for _ in 0..3 {
                    duo.poll_round(None);
                    if let Some(spectator) = spectator.as_mut() {
                        spectator.pump();
                    }
                }
                duo.advance_both(50 + i, 60 + i);
            }

            (duo, spectator)
        }

        /// Drives the joiner's sync handshake toward `peer` until both sides
        /// are Running (bounded; asserts on exhaustion).
        fn sync_joiner_with(duo: &mut Duo, joiner: &mut ManualJoiner, peer: SocketAddr) {
            // Condition-driven with a generous cap (session-33 round-6).
            for _ in 0..300 {
                duo.poll_round(Some(joiner));
                let session_side_running = if peer == addr_a() {
                    duo.a
                        .player_reg
                        .remotes
                        .get(&addr_c())
                        .is_some_and(UdpProtocol::is_running)
                } else {
                    duo.b
                        .player_reg
                        .remotes
                        .get(&addr_c())
                        .is_some_and(UdpProtocol::is_running)
                };
                if joiner.is_running(peer) && session_side_running {
                    return;
                }
            }
            let session_side = if peer == addr_a() { &duo.a } else { &duo.b };
            panic!(
                "manual joiner failed to synchronize with {peer:?} — \
                 joiner endpoint (state, roundtrips left, outstanding randoms, magic, \
                 remote magic): {:?}; session-side endpoint for the joiner: {:?}",
                joiner
                    .protos
                    .get(&peer)
                    .map(UdpProtocol::sync_debug_snapshot),
                session_side
                    .player_reg
                    .remotes
                    .get(&addr_c())
                    .map(UdpProtocol::sync_debug_snapshot),
            );
        }

        // ------------------------------------------------------------------
        // Tests
        // ------------------------------------------------------------------

        /// Happy path: C drops gracefully, rejoins; A pauses + waits for its
        /// confirmed frame to reach S, serves; B re-arms + re-syncs the joiner
        /// endpoint, reopens at F, acks; the joiner acks the snapshot; A
        /// commits, un-pauses, and both A's and B's slot-2 queues accept C's
        /// real inputs from F onward with byte-identical confirmed values.
        ///
        /// Runs on the VALUE-VARYING staging (round-5 de-blind): the staged
        /// drop is one frame asymmetric, so this flagship test now also pins
        /// the generic freeze convergence — any frame/value disagreement
        /// surfaces in the snapshot byte-equality assert instead of being
        /// masked by a constant input.
        #[test]
        fn npeer_happy_path_rejoin_reactivates_all_survivors_at_one_frame() {
            let mut duo = mesh_with_dropped_slot_varying(600, 6);

            // --- Rejoin: manual joiner attaches at C's address, syncs to A.
            let mut c2 = ManualJoiner::new(&duo.bus.clone(), addr_c());
            c2.connect(addr_a(), 0, &duo.clock.clone());
            sync_joiner_with(&mut duo, &mut c2, addr_a());

            // --- JoinRequest opens an N-peer serve (survivor set = {B}).
            c2.proto_mut(addr_a()).send_join_request(2);
            c2.pump();
            duo.poll_round(Some(&mut c2));
            let (serve_s, serve_f) = {
                let serve = duo
                    .a
                    .hot_join
                    .npeer
                    .as_ref()
                    .expect("N-peer serve opens on the JoinRequest");
                assert_eq!(
                    serve.survivors,
                    BTreeSet::from([addr_b()]),
                    "survivor set is exactly B"
                );
                (serve.snapshot_frame, serve.activation_frame)
            };
            assert_eq!(
                serve_f,
                Frame::new(serve_s.as_i32() + 1),
                "F = S + 1 (the activation frame clears the survivor cap)"
            );
            assert_eq!(
                serve_s,
                duo.a.sync_layer.last_saved_frame(),
                "S = the coordinator's last-saved (= last-sent) frame"
            );
            assert!(
                duo.a_events
                    .iter()
                    .any(|e| matches!(e, FortressEvent::JoinRequested { handle, .. } if *handle == PlayerHandle::new(2))),
                "A emits JoinRequested"
            );

            // --- The pause: A's frame counter is pinned for the entire serve.
            let paused_at = duo.a.current_frame();
            duo.a
                .add_local_input(PlayerHandle::new(0), 70)
                .expect("A local input");
            let requests = duo.a.advance_frame().expect("A paused advance");
            apply_requests(&requests, &mut duo.a_shadow);
            assert_eq!(
                duo.a.current_frame(),
                paused_at,
                "the N-peer serve pauses the coordinator"
            );

            // --- B receives the directive: re-arms the joiner endpoint
            // (terminal -> Synchronizing) and goes pending (slot reserved).
            for _ in 0..6 {
                duo.poll_round(Some(&mut c2));
                duo.advance_both(71, 81);
                if duo.b.hot_join.pending_reactivation.is_some() {
                    break;
                }
            }
            {
                let pending = duo
                    .b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .expect("B holds a pending reactivation");
                assert_eq!(pending.handle, PlayerHandle::new(2));
                assert_eq!(pending.frame, serve_f, "the directive carries F verbatim");
                assert_eq!(pending.coordinator_addr, addr_a());
                assert!(
                    !pending.reopened,
                    "no reopen before the joiner channel is up"
                );
            }
            assert!(
                duo.b
                    .hot_join
                    .reserved_slots
                    .contains(&PlayerHandle::new(2)),
                "the pending slot is reserved on B (fold-skip + event-swallow)"
            );
            {
                let endpoint = duo
                    .b
                    .player_reg
                    .remotes
                    .get(&addr_c())
                    .expect("B's joiner endpoint exists");
                assert!(
                    !endpoint.is_running() && !endpoint.is_synchronized(),
                    "B re-armed the joiner endpoint (Synchronizing, not terminal)"
                );
            }

            // --- The joiner now syncs to B (deliberately created AFTER B's
            // re-arm: the production joiner-side survivor-endpoint sequencing
            // is chunk N4); B reopens at F and acks once Running-with-joiner.
            c2.connect(addr_b(), 1, &duo.clock.clone());
            for _ in 0..60 {
                duo.poll_round(Some(&mut c2));
                duo.advance_both(72, 82);
                if duo
                    .b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened)
                    && c2.is_running(addr_b())
                {
                    break;
                }
            }
            assert!(
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened),
                "B reopened after its joiner endpoint reached Running (pending: {:?}, b-side endpoint running/synced: {:?}, c2 proto_b running: {})",
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .map(|p| (p.handle, p.frame, p.reopened)),
                duo.b
                    .player_reg
                    .remotes
                    .get(&addr_c())
                    .map(|e| (e.is_running(), e.is_synchronized())),
                c2.is_running(addr_b())
            );
            assert!(
                !duo.b.sync_layer.player_is_frozen(PlayerHandle::new(2)),
                "B's slot-2 queue is live"
            );
            assert_eq!(
                duo.b.local_connect_status[2],
                ConnectionStatus {
                    disconnected: false,
                    last_frame: Frame::new(serve_f.as_i32() - 1),
                },
                "B reopened the slot at F (connected, last_frame = F - 1)"
            );
            assert!(
                !duo.b
                    .hot_join
                    .reserved_slots
                    .contains(&PlayerHandle::new(2)),
                "the reopened slot is no longer reserved on B"
            );

            // --- Wait-then-capture: A serves the snapshot at exactly S once
            // its confirmed frame caught up; the joiner receives it.
            let mut snapshot = None;
            for _ in 0..30 {
                duo.poll_round(Some(&mut c2));
                duo.advance_both(73, 83);
                if let Some(snap) = c2.proto_mut(addr_a()).take_received_snapshot() {
                    snapshot = Some(snap);
                    break;
                }
            }
            let snapshot = snapshot.expect("the joiner receives the snapshot");
            assert_eq!(snapshot.frame, serve_s, "the snapshot is at S = F - 1");
            let (served_state, _) = crate::network::codec::decode::<u8>(&snapshot.state_bytes)
                .expect("snapshot state decodes");
            assert_eq!(
                Some(&served_state),
                duo.b_shadow.states.get(&serve_s.as_i32()),
                "the served state at S byte-equals the survivor's state at S (fully confirmed, never speculative)"
            );

            // --- The joiner acks; A commits only once the joiner acked AND
            // every survivor acked; A un-pauses and announces JoinCommitted.
            c2.proto_mut(addr_a()).send_state_snapshot_ack(serve_s);
            c2.pump();
            for _ in 0..6 {
                duo.poll_round(Some(&mut c2));
                if duo.a.hot_join.npeer.is_none() {
                    break;
                }
            }
            assert!(duo.a.hot_join.npeer.is_none(), "the serve committed");
            assert!(
                duo.a_events
                    .iter()
                    .any(|e| matches!(e, FortressEvent::PeerJoined { handle, .. } if *handle == PlayerHandle::new(2))),
                "A emits PeerJoined at commit"
            );
            assert_eq!(
                duo.a.local_connect_status[2],
                ConnectionStatus {
                    disconnected: false,
                    last_frame: Frame::new(serve_f.as_i32() - 1),
                },
                "A reactivated its own slot at F"
            );
            assert!(
                !duo.a.sync_layer.player_is_frozen(PlayerHandle::new(2)),
                "A's slot-2 queue is live"
            );

            // --- The joiner contributes real inputs from F onward; both
            // survivors' queues accept them and the commit lifecycle reaches
            // both the joiner and B.
            let mut commit_seen = false;
            for k in 0..14_i32 {
                c2.send_input(Frame::new(serve_f.as_i32() + k), C_REJOIN_INPUT);
                for _ in 0..3 {
                    duo.poll_round(Some(&mut c2));
                }
                duo.advance_both(74, 84);
                if c2
                    .proto_mut(addr_a())
                    .take_received_join_committed()
                    .is_some_and(|body| body.handle == 2 && body.frame == serve_f)
                {
                    commit_seen = true;
                }
                if duo.a.confirmed_frame() >= serve_f && duo.b.confirmed_frame() >= serve_f {
                    break;
                }
            }
            assert!(commit_seen, "the joiner observes JoinCommitted{{2, F}}");
            assert!(
                duo.b.hot_join.pending_reactivation.is_none(),
                "B's pending reactivation cleared on JoinCommitted"
            );
            assert!(
                duo.a.current_frame() > paused_at,
                "A un-paused and advanced after the commit"
            );
            assert!(
                duo.a.confirmed_frame() >= serve_f && duo.b.confirmed_frame() >= serve_f,
                "both survivors confirm past F (A {}, B {})",
                duo.a.confirmed_frame(),
                duo.b.confirmed_frame()
            );

            // The desync pin: both survivors confirm C's REAL input at F (not
            // the frozen value) — the byte-identical activation agreement.
            let a_input = duo
                .a
                .sync_layer
                .confirmed_input(PlayerHandle::new(2), serve_f)
                .expect("A confirmed slot-2 input at F")
                .input;
            let b_input = duo
                .b
                .sync_layer
                .confirmed_input(PlayerHandle::new(2), serve_f)
                .expect("B confirmed slot-2 input at F")
                .input;
            assert_eq!(a_input, C_REJOIN_INPUT, "A committed C's real input at F");
            assert_eq!(b_input, C_REJOIN_INPUT, "B committed C's real input at F");

            // And the confirmed STATES agree (the shadow checksum equivalent).
            let probe = serve_f.as_i32() + 1;
            assert_eq!(
                duo.a_shadow.states.get(&probe),
                duo.b_shadow.states.get(&probe),
                "A and B byte-agree on the state at F + 1"
            );
        }

        /// Survivor speculation repair on the commit path (session-33 review
        /// Finding 1, probe-confirmed): with CONSTANT local inputs (held
        /// buttons / idle — the most common real input pattern) a survivor
        /// that speculated past `F` with the frozen value has no coincidental
        /// misprediction on any OTHER slot to drag it into a rollback, so the
        /// reopen itself must arm the forced re-simulation from `F`
        /// (`disconnect_frame = min(.., F)`) exactly like the coordinator
        /// commit and the abort restore do. Without the arming, the joiner's
        /// real inputs land in a queue with no open prediction episode
        /// (`add_input` only compares against an episode), `first_incorrect`
        /// is never set, and the survivor permanently keeps its frozen-value
        /// speculation for frames >= F — a silent byte divergence at F + 1 on
        /// the HAPPY path. The varying-input happy-path test masks this: its
        /// coordinator input changes at exactly F, and that unrelated
        /// misprediction-rollback coincidentally re-simulates the joiner slot.
        #[test]
        fn npeer_commit_with_constant_inputs_resimulates_survivor_speculation_from_f() {
            let mut duo = mesh_with_dropped_slot(600, 6);

            // From here on every local input is CONSTANT, so repeat-last
            // prediction is correct for both live slots at every frame >= the
            // serve window — no coincidental rollback can mask a missing
            // forced re-simulation of the joiner slot.
            const A_HELD: u8 = 70;
            const B_HELD: u8 = 80;
            for _ in 0..3_u8 {
                for _ in 0..3 {
                    duo.poll_round(None);
                }
                duo.advance_both(A_HELD, B_HELD);
            }

            // --- Rejoin: manual joiner attaches at C's address, syncs to A.
            let mut c2 = ManualJoiner::new(&duo.bus.clone(), addr_c());
            c2.connect(addr_a(), 0, &duo.clock.clone());
            sync_joiner_with(&mut duo, &mut c2, addr_a());
            c2.proto_mut(addr_a()).send_join_request(2);
            c2.pump();
            duo.poll_round(Some(&mut c2));
            let serve_f = duo
                .a
                .hot_join
                .npeer
                .as_ref()
                .expect("N-peer serve opens")
                .activation_frame;

            // --- Directive reaches B; B keeps advancing under the cap with
            // the held inputs (speculating frames >= F with the frozen value).
            for _ in 0..6 {
                duo.poll_round(Some(&mut c2));
                duo.advance_both(A_HELD, B_HELD);
                if duo.b.hot_join.pending_reactivation.is_some() {
                    break;
                }
            }
            assert!(
                duo.b.hot_join.pending_reactivation.is_some(),
                "B holds a pending reactivation"
            );

            // --- Joiner syncs to B; B reopens at F and acks.
            c2.connect(addr_b(), 1, &duo.clock.clone());
            for _ in 0..60 {
                duo.poll_round(Some(&mut c2));
                duo.advance_both(A_HELD, B_HELD);
                if duo
                    .b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened)
                    && c2.is_running(addr_b())
                {
                    break;
                }
            }
            assert!(
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened),
                "B reopened after its joiner endpoint reached Running"
            );
            // Precondition pin: B genuinely speculated past F before the
            // reopen (the finding's reachability condition — production-always
            // whenever max_prediction >= 2 and the serve outlasts ~1 frame).
            assert!(
                duo.b.current_frame() > serve_f,
                "B speculated past F before the reopen (current {}, F {})",
                duo.b.current_frame(),
                serve_f
            );

            // --- Joiner acks the snapshot; A commits and un-pauses.
            let mut snapshot = None;
            for _ in 0..30 {
                duo.poll_round(Some(&mut c2));
                duo.advance_both(A_HELD, B_HELD);
                if let Some(snap) = c2.proto_mut(addr_a()).take_received_snapshot() {
                    snapshot = Some(snap);
                    break;
                }
            }
            let snapshot = snapshot.expect("the joiner receives the snapshot");
            c2.proto_mut(addr_a())
                .send_state_snapshot_ack(snapshot.frame);
            c2.pump();
            for _ in 0..6 {
                duo.poll_round(Some(&mut c2));
                if duo.a.hot_join.npeer.is_none() {
                    break;
                }
            }
            assert!(duo.a.hot_join.npeer.is_none(), "the serve committed");

            // --- The joiner feeds real inputs from F; both survivors keep
            // advancing with the held inputs until both confirm past F.
            for k in 0..14_i32 {
                c2.send_input(Frame::new(serve_f.as_i32() + k), C_REJOIN_INPUT);
                for _ in 0..3 {
                    duo.poll_round(Some(&mut c2));
                }
                duo.advance_both(A_HELD, B_HELD);
                if duo.a.confirmed_frame() >= serve_f && duo.b.confirmed_frame() >= serve_f {
                    break;
                }
            }
            assert!(
                duo.a.confirmed_frame() >= serve_f && duo.b.confirmed_frame() >= serve_f,
                "both survivors confirm past F (A {}, B {})",
                duo.a.confirmed_frame(),
                duo.b.confirmed_frame()
            );

            // Both committed C's REAL input at F...
            let a_input = duo
                .a
                .sync_layer
                .confirmed_input(PlayerHandle::new(2), serve_f)
                .expect("A confirmed slot-2 input at F")
                .input;
            let b_input = duo
                .b
                .sync_layer
                .confirmed_input(PlayerHandle::new(2), serve_f)
                .expect("B confirmed slot-2 input at F")
                .input;
            assert_eq!(a_input, C_REJOIN_INPUT, "A committed C's real input at F");
            assert_eq!(b_input, C_REJOIN_INPUT, "B committed C's real input at F");

            // ...and the STATES at F + 1 byte-agree: B's frozen-value
            // speculation of frame F was re-simulated with the real input
            // (the reopen armed `disconnect_frame = F`), not silently kept.
            let probe = serve_f.as_i32() + 1;
            let a_state = duo.a_shadow.states.get(&probe);
            let b_state = duo.b_shadow.states.get(&probe);
            assert!(
                a_state.is_some() && b_state.is_some(),
                "both survivors simulated past F + 1"
            );
            assert_eq!(
                a_state, b_state,
                "A and B byte-agree on the state at F + 1 (constant-input commit must force the survivor's re-simulation from F)"
            );
        }

        /// Wait-then-capture (R1): a coordinator with a pending misprediction
        /// at serve-open repairs it WHILE PAUSED (the paused `advance_frame`
        /// surfaces the rollback's `LoadGameState`) and only then captures —
        /// the serve never embeds speculative survivor inputs.
        ///
        /// RED-provability (verified by hand, see the session report): with
        /// the wait gate neutralized (capture at open), the snapshot-state
        /// assertion fails (stale state served); with the paused-rollback arm
        /// neutralized (2-peer-style empty return), the LoadGameState
        /// assertion fails and the capture deadlocks into the Phase-4 abort.
        #[test]
        fn npeer_serve_waits_for_confirmed_and_repairs_misprediction_while_paused() {
            let mut duo = mesh_with_dropped_slot(600, 6);

            // Block B -> A inputs, then let B advance with a NEW input value:
            // A's prediction for B (repeat-last) is now wrong for that frame.
            duo.bus.block(addr_b(), addr_a(), "Input");
            for _ in 0..3 {
                duo.poll_round(None);
            }
            // B advances one frame with a changed input; A advances two frames
            // on stale predictions of B.
            duo.advance_both(90, 200);
            for _ in 0..3 {
                duo.poll_round(None);
            }
            duo.advance_both(91, 201);

            // Rejoin while the misprediction is outstanding.
            let mut c2 = ManualJoiner::new(&duo.bus.clone(), addr_c());
            c2.connect(addr_a(), 0, &duo.clock.clone());
            sync_joiner_with(&mut duo, &mut c2, addr_a());
            c2.proto_mut(addr_a()).send_join_request(2);
            c2.pump();
            duo.poll_round(Some(&mut c2));
            let serve_s = duo
                .a
                .hot_join
                .npeer
                .as_ref()
                .expect("N-peer serve opens")
                .snapshot_frame;

            // The wait gate holds: B's inputs through S have not arrived, so
            // A's confirmed frame trails S and nothing may be captured.
            for _ in 0..5 {
                duo.poll_round(Some(&mut c2));
                let serve = duo.a.hot_join.npeer.as_ref().expect("serve stays open");
                assert!(
                    serve.snapshot.is_none(),
                    "the serve must NOT capture while confirmed < S (confirmed {}, S {})",
                    duo.a.confirmed_frame(),
                    serve_s
                );
            }
            assert!(duo.a.confirmed_frame() < serve_s);

            // Unblock: B's held inputs (including the mispredicted frame)
            // re-deliver via its pending-output retransmit. The PAUSED advance
            // must then surface the repair rollback — without ever advancing
            // to a new frame.
            duo.bus.unblock(addr_b(), addr_a(), "Input");
            let paused_frame = duo.a.current_frame();
            let mut repair_seen = false;
            for _ in 0..20 {
                duo.poll_round(Some(&mut c2));
                duo.a
                    .add_local_input(PlayerHandle::new(0), 92)
                    .expect("A local input");
                let requests = duo.a.advance_frame().expect("A paused advance");
                if requests
                    .iter()
                    .any(|request| matches!(request, FortressRequest::LoadGameState { .. }))
                {
                    repair_seen = true;
                }
                apply_requests(&requests, &mut duo.a_shadow);
                assert_eq!(
                    duo.a.current_frame(),
                    paused_frame,
                    "rollback-while-paused never advances to a new frame"
                );
                if repair_seen {
                    break;
                }
            }
            assert!(
                repair_seen,
                "the paused coordinator surfaces the misprediction-repair rollback"
            );

            // The capture now fires — at exactly S, with the REPAIRED state
            // (byte-equal to B's ground truth at S, which B never mispredicted).
            let mut captured = None;
            for _ in 0..6 {
                duo.poll_round(Some(&mut c2));
                if let Some(serve) = duo.a.hot_join.npeer.as_ref() {
                    if let Some(snapshot) = &serve.snapshot {
                        captured = Some(snapshot.clone());
                        break;
                    }
                } else {
                    break;
                }
            }
            let snapshot = captured.expect("the serve captures after the repair");
            assert_eq!(snapshot.frame, serve_s, "captured at exactly S");
            let (served_state, _) = crate::network::codec::decode::<u8>(&snapshot.state_bytes)
                .expect("snapshot state decodes");
            assert_eq!(
                Some(&served_state),
                duo.b_shadow.states.get(&serve_s.as_i32()),
                "the served state embeds B's REAL inputs at S, not A's stale prediction"
            );
        }

        /// Pre-ack abort: B received the directive (re-armed + pending) but
        /// its joiner channel never comes up, so it never acks; A's Phase-4
        /// timeout aborts; `JoinAborted` clears B's pending attempt with the
        /// slot's frozen state untouched, and A's next-serve guard forces the
        /// retry onto a strictly later activation frame.
        #[test]
        fn npeer_pre_ack_abort_leaves_survivor_reserved_and_guards_next_serve() {
            // Serve budget: outlasts the pre-abort assertions (~7 polls) and
            // expires inside the exhaust loop below.
            let mut duo = mesh_with_dropped_slot(14, 6);

            let f_pre = duo.b.local_connect_status[2];
            assert!(f_pre.disconnected, "slot 2 is dropped on B");

            let mut c2 = ManualJoiner::new(&duo.bus.clone(), addr_c());
            c2.connect(addr_a(), 0, &duo.clock.clone());
            sync_joiner_with(&mut duo, &mut c2, addr_a());
            c2.proto_mut(addr_a()).send_join_request(2);
            c2.pump();
            duo.poll_round(Some(&mut c2));
            let serve_f = duo
                .a
                .hot_join
                .npeer
                .as_ref()
                .expect("N-peer serve opens")
                .activation_frame;

            // Stale-ack discrimination on the coordinator: a wrong-frame ack
            // must not count as the survivor's reopen ack.
            duo.b
                .player_reg
                .remotes
                .get_mut(&addr_a())
                .expect("B's coordinator endpoint")
                .send_reactivate_slot_ack(2, Frame::new(serve_f.as_i32() + 7));
            for _ in 0..2 {
                duo.poll_round(Some(&mut c2));
            }
            assert!(
                duo.a
                    .hot_join
                    .npeer
                    .as_ref()
                    .is_some_and(|serve| serve.pending_acks.contains(&addr_b())),
                "a mismatched ReactivateSlotAck frame is ignored (B still pending)"
            );

            // B is pending but never reopens (the joiner never syncs to B).
            for _ in 0..4 {
                duo.poll_round(Some(&mut c2));
                duo.advance_both(95, 96);
            }
            assert!(
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| !pending.reopened),
                "B is pending and unreopened"
            );
            assert!(
                duo.b.sync_layer.player_is_frozen(PlayerHandle::new(2)),
                "B's slot stays frozen pre-reopen"
            );

            // Commit barrier pin: the JOINER acks the snapshot, but B's reopen
            // ack is still missing — the serve must NOT commit (un-pausing
            // here would lift the survivor cap and let B commit F frozen).
            let mut joiner_acked = false;
            for _ in 0..4 {
                if let Some(snapshot) = c2.proto_mut(addr_a()).take_received_snapshot() {
                    c2.proto_mut(addr_a())
                        .send_state_snapshot_ack(snapshot.frame);
                    c2.pump();
                    joiner_acked = true;
                }
                duo.poll_round(Some(&mut c2));
                if joiner_acked {
                    break;
                }
            }
            assert!(joiner_acked, "the joiner received and acked the snapshot");
            duo.poll_round(Some(&mut c2));
            assert!(
                duo.a.hot_join.npeer.as_ref().is_some_and(
                    |serve| serve.joiner_acked && serve.pending_acks.contains(&addr_b())
                ),
                "the joiner ack alone must NOT commit while a survivor ack is pending"
            );
            assert!(
                duo.a
                    .hot_join
                    .reserved_slots
                    .contains(&PlayerHandle::new(2)),
                "A's slot stays reserved pre-commit"
            );

            // Exhaust the serve budget -> abort.
            for _ in 0..20 {
                duo.poll_round(Some(&mut c2));
                if duo.a.hot_join.npeer.is_none() {
                    break;
                }
            }
            assert!(duo.a.hot_join.npeer.is_none(), "Phase-4 aborts the serve");
            assert!(
                duo.a
                    .hot_join
                    .reserved_slots
                    .contains(&PlayerHandle::new(2)),
                "the slot stays reserved on A after the abort"
            );
            assert!(
                duo.a
                    .hot_join
                    .npeer_post
                    .as_ref()
                    .is_some_and(|post| !post.committed && post.frame == serve_f),
                "A announces JoinAborted{{2, F}}"
            );

            // The JoinAborted reaches B: pending cleared, slot byte-identical
            // to its pre-attempt reserved/frozen shape.
            for _ in 0..6 {
                duo.poll_round(Some(&mut c2));
                if duo.b.hot_join.pending_reactivation.is_none() {
                    break;
                }
            }
            assert!(
                duo.b.hot_join.pending_reactivation.is_none(),
                "JoinAborted clears B's pre-ack pending attempt"
            );
            assert!(
                duo.b.sync_layer.player_is_frozen(PlayerHandle::new(2)),
                "B's slot is still frozen"
            );
            assert_eq!(
                duo.b.local_connect_status[2], f_pre,
                "B's slot status is untouched by the aborted attempt"
            );
            assert!(
                duo.b
                    .hot_join
                    .reserved_slots
                    .contains(&PlayerHandle::new(2)),
                "the slot stays reserved on B (rejoinable)"
            );

            // R3 next-serve guard, part 1: an IMMEDIATE retry (before the
            // coordinator advances a single frame) must be deferred — opening
            // it would reuse the aborted attempt's exact (handle, F) pair and
            // make its stale lifecycle messages indistinguishable.
            c2.proto_mut(addr_a()).send_join_request(2);
            c2.pump();
            for _ in 0..3 {
                duo.poll_round(Some(&mut c2));
            }
            assert!(
                duo.a.hot_join.npeer.is_none(),
                "a retry before the coordinator advances past the aborted frame is deferred"
            );

            // R3 next-serve guard, part 2: once A advances, the retry opens on
            // a strictly later activation frame than the aborted attempt.
            for i in 0..3_u8 {
                for _ in 0..3 {
                    duo.poll_round(Some(&mut c2));
                }
                duo.advance_both(97 + i, 98 + i);
            }
            c2.proto_mut(addr_a()).send_join_request(2);
            c2.pump();
            for _ in 0..4 {
                duo.poll_round(Some(&mut c2));
                if duo.a.hot_join.npeer.is_some() {
                    break;
                }
            }
            let retry = duo
                .a
                .hot_join
                .npeer
                .as_ref()
                .expect("the retry opens a fresh serve");
            assert!(
                retry.activation_frame > serve_f,
                "the retry's F ({}) must be strictly later than the aborted attempt's ({})",
                retry.activation_frame,
                serve_f
            );
        }

        /// Post-ack abort: B reopened + acked, but the JOINER never acks the
        /// snapshot, so A's Phase-4 timeout aborts after B's reopen. The
        /// matching `JoinAborted` must restore B's slot byte-identically
        /// (frozen queue with the preserved agreed value, pre-reopen
        /// connection status, reserved membership) and repair any speculative
        /// frame that embedded the joiner's real input.
        #[test]
        fn npeer_post_ack_abort_refreezes_survivor_byte_identically() {
            // Serve budget: long enough for the directive + the joiner's
            // B-channel sync + the reopen + the leaked input (~25 polls), yet
            // short enough to expire inside the abort-wait loop below.
            let mut duo = mesh_with_dropped_slot(40, 6);

            let pre_status = duo.b.local_connect_status[2];

            let mut c2 = ManualJoiner::new(&duo.bus.clone(), addr_c());
            c2.connect(addr_a(), 0, &duo.clock.clone());
            sync_joiner_with(&mut duo, &mut c2, addr_a());
            c2.proto_mut(addr_a()).send_join_request(2);
            c2.pump();
            duo.poll_round(Some(&mut c2));
            let serve_f = duo
                .a
                .hot_join
                .npeer
                .as_ref()
                .expect("N-peer serve opens")
                .activation_frame;

            // Let B receive the directive and re-arm, then bring the joiner's
            // B-channel up so B reopens and acks. B does NOT advance in this
            // window (only A, whose paused advance is a repair-only no-op): B
            // must still have prediction-window headroom at the leak below so
            // it genuinely SIMULATES the leaked input — the divergence the
            // abort must repair.
            for _ in 0..4 {
                duo.poll_round(Some(&mut c2));
                duo.a
                    .add_local_input(PlayerHandle::new(0), 53)
                    .expect("A local input");
                let requests = duo.a.advance_frame().expect("A paused advance");
                apply_requests(&requests, &mut duo.a_shadow);
            }
            c2.connect(addr_b(), 1, &duo.clock.clone());
            for _ in 0..60 {
                duo.poll_round(Some(&mut c2));
                if duo
                    .b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened)
                    && c2.is_running(addr_b())
                {
                    break;
                }
            }
            assert!(
                c2.is_running(addr_b()),
                "the joiner's B-channel reached Running"
            );
            assert!(
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened),
                "B reopened + acked"
            );

            // The joiner leaks a REAL input at F to the reopened B queue, and B
            // then simulates F (and beyond) with the leaked value — exactly the
            // speculative divergence the abort must undo.
            c2.send_input(serve_f, C_REJOIN_INPUT);
            for _ in 0..3 {
                duo.poll_round(Some(&mut c2));
            }
            duo.advance_both(53, 112);
            duo.advance_both(53, 113);
            assert_eq!(
                duo.b.local_connect_status[2].last_frame, serve_f,
                "B's reopened queue accepted the joiner's real input at F"
            );
            assert!(
                duo.b_shadow.states.contains_key(&(serve_f.as_i32() + 1)),
                "B simulated past F with the leaked input (current {})",
                duo.b.current_frame()
            );

            // The joiner NEVER acks the snapshot -> Phase-4 abort.
            for _ in 0..60 {
                duo.poll_round(Some(&mut c2));
                if duo.a.hot_join.npeer.is_none() {
                    break;
                }
            }
            assert!(duo.a.hot_join.npeer.is_none(), "Phase-4 aborts the serve");

            // JoinAborted reaches B: the reopened slot is restored to its
            // pre-reopen reserved shape, byte-identically.
            for _ in 0..6 {
                duo.poll_round(Some(&mut c2));
                if duo.b.hot_join.pending_reactivation.is_none() {
                    break;
                }
            }
            assert!(
                duo.b.hot_join.pending_reactivation.is_none(),
                "JoinAborted clears B's attempt"
            );
            assert!(
                duo.b.sync_layer.player_is_frozen(PlayerHandle::new(2)),
                "B re-froze the slot"
            );
            assert_eq!(
                duo.b.local_connect_status[2], pre_status,
                "B restored the pre-reopen connection status verbatim"
            );
            assert!(
                duo.b
                    .hot_join
                    .reserved_slots
                    .contains(&PlayerHandle::new(2)),
                "the slot is reserved again on B"
            );

            // The restored frozen VALUE is the pre-reopen agreed value: B's
            // next advances must feed slot 2 with C_FROZEN_INPUT (Disconnected),
            // not the leaked real input.
            let mut frozen_checked = false;
            for i in 0..8_u8 {
                for _ in 0..3 {
                    duo.poll_round(Some(&mut c2));
                }
                duo.a
                    .add_local_input(PlayerHandle::new(0), 53)
                    .expect("A local input");
                let requests = duo.a.advance_frame().expect("A advance");
                apply_requests(&requests, &mut duo.a_shadow);

                duo.b
                    .add_local_input(PlayerHandle::new(1), 113 + i)
                    .expect("B local input");
                let requests = duo.b.advance_frame().expect("B advance");
                for request in requests.iter() {
                    if let FortressRequest::AdvanceFrame { inputs } = request {
                        let (value, status) = inputs[2];
                        assert_eq!(
                            value, C_FROZEN_INPUT,
                            "B's slot-2 value reverted to the agreed frozen input"
                        );
                        assert_eq!(status, InputStatus::Disconnected);
                        frozen_checked = true;
                    }
                }
                apply_requests(&requests, &mut duo.b_shadow);
            }
            assert!(frozen_checked, "B advanced at least one frame post-abort");

            // No-desync pin: the speculative frame that embedded the joiner's
            // real input was re-simulated with the frozen value (the abort
            // armed a forced rollback at F), so A's and B's states at F + 1
            // byte-agree.
            let probe = serve_f.as_i32() + 1;
            for _ in 0..20 {
                for _ in 0..3 {
                    duo.poll_round(Some(&mut c2));
                }
                duo.advance_both(53, 121);
                if duo.a_shadow.states.contains_key(&probe)
                    && duo.b_shadow.states.contains_key(&probe)
                    && duo.a.confirmed_frame().as_i32() > probe
                    && duo.b.confirmed_frame().as_i32() > probe
                {
                    break;
                }
            }
            // Liveness pin (session-33 round-2 review Finding 1 / Nit 2): the
            // recovery wait above must be an ASSERTION, not just a loop-break —
            // a survivor whose confirmed frame stays pinned at F - 1 after the
            // abort (the reopen-armed gossip floor filtering the genuine
            // pre-attempt drop gossip forever) otherwise slips through, because
            // the byte-agreement below can pass on speculative shadow states.
            assert!(
                duo.b.confirmed_frame().as_i32() > probe,
                "B's confirmed frame must recover past F after the abort (got {}, F = {})",
                duo.b.confirmed_frame(),
                serve_f
            );
            assert!(
                duo.a.confirmed_frame().as_i32() > probe,
                "A's confirmed frame must recover past F after the abort (got {}, F = {})",
                duo.a.confirmed_frame(),
                serve_f
            );
            let a_state = duo.a_shadow.states.get(&probe);
            let b_state = duo.b_shadow.states.get(&probe);
            assert!(
                a_state.is_some() && b_state.is_some(),
                "both survivors simulated past F + 1 after the abort"
            );
            assert_eq!(
                a_state, b_state,
                "post-abort states at F + 1 byte-agree (the leaked real input was rolled back)"
            );
        }

        /// Post-reopen abort LIVENESS (session-33 round-2 review Finding 1,
        /// probe-confirmed): after a reopened survivor's attempt aborts, the
        /// mesh's genuine pre-attempt drop gossip `{disconnected, f0}` (with
        /// `f0 < F - 1` — the production-default shape) must be re-adopted so
        /// the slot re-converges to mesh-agreed exclusion and BOTH peers'
        /// confirmed frames recover past `F`. A reactivation floor armed at
        /// the (pre-commit) reopen and never disarmed on the abort filters
        /// that gossip forever: the survivor's cached views stay
        /// `{connected, F - 1}`, the slot is never mesh-agreed-excluded, its
        /// confirmed frame pins at `F - 1`, and the whole mesh stalls behind
        /// it over healthy links. The floor must therefore arm only at
        /// commit-evidence points (an aborted world never arms one), which
        /// this test pins both ways: no floor survives the abort, and the
        /// mesh fully re-converges and resumes after it.
        #[test]
        fn npeer_post_reopen_abort_reconverges_genuine_drop_gossip_and_mesh_resumes() {
            let mut duo = mesh_with_dropped_slot(40, 6);
            let pre_status = duo.b.local_connect_status[2];
            let f0 = pre_status.last_frame;

            // --- Reach a REOPENED pending on B (the byte-identity abort
            // test's choreography).
            let mut c2 = ManualJoiner::new(&duo.bus.clone(), addr_c());
            c2.connect(addr_a(), 0, &duo.clock.clone());
            sync_joiner_with(&mut duo, &mut c2, addr_a());
            c2.proto_mut(addr_a()).send_join_request(2);
            c2.pump();
            duo.poll_round(Some(&mut c2));
            let serve_f = duo
                .a
                .hot_join
                .npeer
                .as_ref()
                .expect("N-peer serve opens")
                .activation_frame;
            assert!(
                f0.as_i32() < serve_f.as_i32() - 1,
                "precondition: the pre-attempt freeze frame ({}) is strictly below F - 1 ({}) — the production-default abort shape",
                f0,
                serve_f.as_i32() - 1
            );
            for _ in 0..4 {
                duo.poll_round(Some(&mut c2));
                duo.a
                    .add_local_input(PlayerHandle::new(0), 53)
                    .expect("A local input");
                let requests = duo.a.advance_frame().expect("A paused advance");
                apply_requests(&requests, &mut duo.a_shadow);
            }
            c2.connect(addr_b(), 1, &duo.clock.clone());
            for _ in 0..60 {
                duo.poll_round(Some(&mut c2));
                if duo
                    .b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened)
                    && c2.is_running(addr_b())
                {
                    break;
                }
            }
            assert!(
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened),
                "B reopened + acked"
            );

            // --- The joiner never acks the snapshot -> Phase-4 abort; the
            // JoinAborted restore re-freezes B.
            for _ in 0..60 {
                duo.poll_round(Some(&mut c2));
                if duo.a.hot_join.npeer.is_none() {
                    break;
                }
            }
            assert!(duo.a.hot_join.npeer.is_none(), "Phase-4 aborts the serve");
            for _ in 0..6 {
                duo.poll_round(Some(&mut c2));
                if duo.b.hot_join.pending_reactivation.is_none() {
                    break;
                }
            }
            assert!(
                duo.b.hot_join.pending_reactivation.is_none(),
                "JoinAborted clears B's attempt"
            );
            assert_eq!(
                duo.b.local_connect_status[2], pre_status,
                "B restored the pre-reopen connection status verbatim"
            );

            // --- Mechanism pin: an ABORTED world leaves NO reactivation
            // floor armed on B (the floor's `>= F - 1` threshold theorem is
            // valid only in committed worlds; armed here it would filter the
            // genuine f0 convergence below forever).
            for addr in [addr_a(), addr_c()] {
                let floor = duo
                    .b
                    .player_reg
                    .remotes
                    .get(&addr)
                    .expect("B endpoint exists")
                    .reactivation_floor_for_test(PlayerHandle::new(2));
                assert!(
                    floor.is_null(),
                    "no reactivation floor may outlive an aborted attempt on B's endpoint {:?} (got {})",
                    addr,
                    floor
                );
            }

            // --- Liveness: the slot re-converges to the genuine pre-attempt
            // drop state and BOTH confirmed frames recover past F.
            let probe = serve_f.as_i32() + 1;
            for _ in 0..120 {
                for _ in 0..3 {
                    duo.poll_round(Some(&mut c2));
                }
                duo.advance_both(53, 131);
                if duo.a.confirmed_frame().as_i32() > probe
                    && duo.b.confirmed_frame().as_i32() > probe
                {
                    break;
                }
            }
            let b_view_of_a = duo
                .b
                .player_reg
                .remotes
                .get(&addr_a())
                .expect("B's A endpoint")
                .peer_connect_status(PlayerHandle::new(2));
            assert!(
                b_view_of_a.disconnected,
                "B re-adopted A's genuine {{disconnected, f0}} gossip for the slot (got connected at {})",
                b_view_of_a.last_frame
            );
            assert_eq!(
                b_view_of_a.last_frame, f0,
                "the re-adopted freeze frame is the pre-attempt f0"
            );
            assert!(
                duo.b.confirmed_frame().as_i32() > probe,
                "B's confirmed frame must recover past F after the abort (got {}, F = {})",
                duo.b.confirmed_frame(),
                serve_f
            );
            assert!(
                duo.a.confirmed_frame().as_i32() > probe,
                "A's confirmed frame must recover past F after the abort (got {}, F = {})",
                duo.a.confirmed_frame(),
                serve_f
            );
            assert_eq!(
                duo.a_shadow.states.get(&probe),
                duo.b_shadow.states.get(&probe),
                "A and B byte-agree at F + 1 after the aborted attempt"
            );
        }

        /// Bound-clamp regression (session-33 round-3 review Finding 1): a
        /// reopened survivor whose attempt ABORTED but whose `JoinAborted`
        /// delivery is delayed (a short selective-loss burst — the responder
        /// re-sends per re-ack, so delivery is only deferred, never lost)
        /// must keep its confirmed frame strictly BELOW `F` while the pending
        /// is held — even when the un-paused coordinator streams real inputs
        /// at/past `F` and the joiner (whose abort teardown is chunk-N4
        /// scope) keeps legally leaking inputs `>= F` into the reopened
        /// queue.
        ///
        /// The round-2 bound shield SKIPPED the coordinator's re-stuck
        /// `{disconnected, f0}` claim wholesale, so the only folded gossip
        /// term for the slot was the joiner's own connected self-claim
        /// `>= F`: B confirmed (discarded history for, spectator-flushed,
        /// checksummed) frames built from leaked inputs no other peer will
        /// ever confirm — silent confirmed-state byte divergence — and the
        /// late `JoinAborted` restore then re-simulated frames B had already
        /// confirmed (an S1 violation). The clamp folds shielded
        /// disconnected claims at the seeded `F - 1` instead of skipping
        /// them, restoring the cap while keeping the f0-dip fix (pinned by
        /// the spectator-flush test).
        #[test]
        fn npeer_survivor_with_unresolved_abort_and_leaked_inputs_never_confirms_past_f() {
            let mut duo = mesh_with_dropped_slot(40, 6);
            let pre_status = duo.b.local_connect_status[2];
            let f0 = pre_status.last_frame;

            // The delayed lifecycle close: JoinAborted toward B is blocked
            // until the test delivers it.
            duo.bus.block(addr_a(), addr_b(), "JoinAborted");

            // --- Reach a REOPENED pending on B (the abort-probe
            // choreography).
            let mut c2 = ManualJoiner::new(&duo.bus.clone(), addr_c());
            c2.connect(addr_a(), 0, &duo.clock.clone());
            sync_joiner_with(&mut duo, &mut c2, addr_a());
            c2.proto_mut(addr_a()).send_join_request(2);
            c2.pump();
            duo.poll_round(Some(&mut c2));
            let serve_f = duo
                .a
                .hot_join
                .npeer
                .as_ref()
                .expect("N-peer serve opens")
                .activation_frame;
            assert!(
                f0.as_i32() < serve_f.as_i32() - 1,
                "precondition: the pre-attempt freeze frame ({}) is strictly below F - 1 ({})",
                f0,
                serve_f.as_i32() - 1
            );
            for _ in 0..4 {
                duo.poll_round(Some(&mut c2));
                duo.a
                    .add_local_input(PlayerHandle::new(0), 54)
                    .expect("A local input");
                let requests = duo.a.advance_frame().expect("A paused advance");
                apply_requests(&requests, &mut duo.a_shadow);
            }
            c2.connect(addr_b(), 1, &duo.clock.clone());
            for _ in 0..60 {
                duo.poll_round(Some(&mut c2));
                if duo
                    .b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened)
                    && c2.is_running(addr_b())
                {
                    break;
                }
            }
            assert!(
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened),
                "B reopened + acked"
            );

            // --- The joiner never acks the snapshot -> Phase-4 abort on A.
            // The blocked JoinAborted keeps B's pending held.
            for _ in 0..60 {
                duo.poll_round(Some(&mut c2));
                if duo.a.hot_join.npeer.is_none() {
                    break;
                }
            }
            assert!(duo.a.hot_join.npeer.is_none(), "Phase-4 aborts the serve");

            // --- The unresolved-abort window: A (un-paused) streams real
            // inputs at/past F, the joiner keeps leaking inputs >= F into
            // B's reopened queue, and B's confirmed frame must stay capped
            // below F the whole time.
            for k in 0..8_i32 {
                c2.send_input(Frame::new(serve_f.as_i32() + k), C_REJOIN_INPUT);
                for _ in 0..3 {
                    duo.poll_round(Some(&mut c2));
                }
                duo.advance_both(55, 132);
                assert!(
                    duo.b.hot_join.pending_reactivation.is_some(),
                    "precondition: the blocked JoinAborted keeps B's attempt pending"
                );
                assert!(
                    duo.b.confirmed_frame() < serve_f,
                    "B must not confirm at/past F while the aborted attempt is unresolved (got {}, F = {})",
                    duo.b.confirmed_frame(),
                    serve_f
                );
            }

            // --- Deliver the delayed JoinAborted: the restore applies to a
            // confirmed history that never crossed F (no confirmed-state
            // rewrite), and the slot re-enters the pre-attempt shape.
            duo.bus.unblock(addr_a(), addr_b(), "JoinAborted");
            for _ in 0..30 {
                duo.poll_round(Some(&mut c2));
                if duo.b.hot_join.pending_reactivation.is_none() {
                    break;
                }
            }
            assert!(
                duo.b.hot_join.pending_reactivation.is_none(),
                "the delivered JoinAborted closes B's attempt"
            );
            assert_eq!(
                duo.b.local_connect_status[2], pre_status,
                "B restored the pre-reopen connection status verbatim"
            );

            // --- Liveness + byte-identity: the abort world re-converges,
            // both confirmed frames recover past F, and the shadows agree at
            // F + 1 (the frames the leak would have poisoned).
            let probe = serve_f.as_i32() + 1;
            for _ in 0..120 {
                for _ in 0..3 {
                    duo.poll_round(Some(&mut c2));
                }
                duo.advance_both(56, 133);
                if duo.a.confirmed_frame().as_i32() > probe
                    && duo.b.confirmed_frame().as_i32() > probe
                {
                    break;
                }
            }
            assert!(
                duo.b.confirmed_frame().as_i32() > probe,
                "B's confirmed frame recovers past F after the delivered abort (got {}, F = {})",
                duo.b.confirmed_frame(),
                serve_f
            );
            assert!(
                duo.a.confirmed_frame().as_i32() > probe,
                "A's confirmed frame recovers past F after the abort (got {}, F = {})",
                duo.a.confirmed_frame(),
                serve_f
            );
            assert_eq!(
                duo.a_shadow.states.get(&probe),
                duo.b_shadow.states.get(&probe),
                "A and B byte-agree at F + 1 after the delayed abort delivery"
            );
        }

        /// Stages the session-33 round-4 review Finding 1 choreography on B:
        /// a PRE-reopen pending whose abort conclusion B never hears, held
        /// while B's confirmed frame legitimately crosses the attempt's
        /// activation frame `F`.
        ///
        /// Choreography: `JoinAborted` A->B is blocked; the joiner handshakes
        /// with A ONLY, so B accepts the directive but its joiner endpoint
        /// never goes `Running` and the pending stays pre-reopen (a
        /// pre-reopen survivor has no re-ack convergence loop, so nothing
        /// ever re-delivers the lost abort); the joiner never acks the
        /// snapshot, so the serve Phase-4 aborts and A un-pauses; A's stream
        /// then drives B's confirmed frame past `F` — byte-safe by itself,
        /// because the frozen slot folds `None` (locally disconnected, the
        /// rearmed joiner endpoint reserved-excluded, A's claim
        /// disconnected). Returns the joiner driver (connected to A only),
        /// the activation frame `F`, and B's captured pre-attempt slot-2
        /// status.
        fn reach_pre_reopen_pending_with_confirmed_past_f(
            duo: &mut Duo,
        ) -> (ManualJoiner, Frame, ConnectionStatus) {
            let pre_status = duo.b.local_connect_status[2];

            duo.bus.block(addr_a(), addr_b(), "JoinAborted");

            let mut c2 = ManualJoiner::new(&duo.bus.clone(), addr_c());
            c2.connect(addr_a(), 0, &duo.clock.clone());
            sync_joiner_with(duo, &mut c2, addr_a());
            c2.proto_mut(addr_a()).send_join_request(2);
            c2.pump();
            duo.poll_round(Some(&mut c2));
            let serve_f = duo
                .a
                .hot_join
                .npeer
                .as_ref()
                .expect("N-peer serve opens")
                .activation_frame;

            // B accepts the directive; the joiner<->B handshake never
            // starts, so the pending stays PRE-reopen.
            for _ in 0..60 {
                duo.poll_round(Some(&mut c2));
                if duo.b.hot_join.pending_reactivation.is_some() {
                    break;
                }
            }
            assert!(
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| !pending.reopened),
                "precondition: B holds the PRE-reopen pending"
            );

            // The joiner never acks the snapshot -> Phase-4 abort on A. B
            // has not acked either (the ack is sent at reopen), so the
            // attempt provably never committed anywhere.
            for _ in 0..60 {
                duo.poll_round(Some(&mut c2));
                if duo.a.hot_join.npeer.is_none() {
                    break;
                }
            }
            assert!(duo.a.hot_join.npeer.is_none(), "Phase-4 aborts the serve");

            // A un-pauses and streams real inputs; the frozen slot is
            // excluded from B's confirmed fold, so B's confirmed frame
            // crosses F while the blocked JoinAborted keeps the pre-reopen
            // pending held.
            for _ in 0..30 {
                for _ in 0..3 {
                    duo.poll_round(Some(&mut c2));
                }
                duo.advance_both(55, 132);
                if duo.b.confirmed_frame().as_i32() > serve_f.as_i32() + 1 {
                    break;
                }
            }
            assert!(
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| !pending.reopened),
                "precondition: the unheard abort keeps the pre-reopen pending held"
            );
            assert!(
                duo.b.confirmed_frame() > serve_f,
                "precondition: B's confirmed frame crossed F while the abort stayed unheard (got {}, F = {})",
                duo.b.confirmed_frame(),
                serve_f
            );

            (c2, serve_f, pre_status)
        }

        /// Session-33 round-4 review Finding 1 (Critical, probe-confirmed):
        /// the LATE reopen after an unheard abort must be rejected by the
        /// reopen-time F re-validation. Pre-fix,
        /// `progress_pending_reactivation` reopened the slot at the stale `F`
        /// the moment the delayed joiner<->B handshake completed —
        /// repositioning the queue below B's confirmed history and arming
        /// `disconnect_frame = F`, after which every `advance_frame` failed
        /// forever (`SynchronizedInputsFailed`, then `WrongSavedFrame` after
        /// the restore — a permanent wedge on a healthy session). With the
        /// re-check the reopen is REJECTED and the attempt is cancelled
        /// fail-closed: pending cleared, slot verbatim frozen + reserved, no
        /// closed-attempt high-water, and the session keeps advancing with
        /// its confirmed stream intact and byte-identical to the
        /// coordinator's.
        #[test]
        fn npeer_late_reopen_after_unheard_abort_is_cancelled_fail_closed() {
            let mut duo = mesh_with_dropped_slot(40, 6);
            let (mut c2, serve_f, pre_status) =
                reach_pre_reopen_pending_with_confirmed_past_f(&mut duo);

            // --- The LATE joiner<->B handshake completes: the reopen gate
            // (joiner endpoint Running) opens at the stale F.
            c2.connect(addr_b(), 1, &duo.clock.clone());
            for _ in 0..60 {
                duo.poll_round(Some(&mut c2));
                let concluded = match duo.b.hot_join.pending_reactivation.as_ref() {
                    None => true,
                    Some(pending) => pending.reopened,
                };
                if concluded {
                    break;
                }
            }
            assert!(
                !duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened),
                "B must NOT reopen slot 2 at the stale F (F = {}, confirmed = {})",
                serve_f,
                duo.b.confirmed_frame()
            );
            assert!(
                duo.b.hot_join.pending_reactivation.is_none(),
                "the stale-F attempt is cancelled fail-closed"
            );

            // --- Fail-closed shape: verbatim frozen status, frozen queue,
            // reserved membership (the rearmed joiner endpoint stays
            // excluded from the folds), and NO closed-attempt high-water
            // (the attempt was never heard closing; a genuine retry
            // directive at a newer F re-validates from this exact reserved
            // shape).
            assert_eq!(
                duo.b.local_connect_status[2], pre_status,
                "slot 2 keeps its pre-attempt frozen status verbatim"
            );
            assert!(
                duo.b.sync_layer.player_is_frozen(PlayerHandle::new(2)),
                "slot 2's queue stays frozen"
            );
            assert!(
                duo.b
                    .hot_join
                    .reserved_slots
                    .contains(&PlayerHandle::new(2)),
                "slot 2 stays reserved"
            );
            assert!(
                !duo.b
                    .hot_join
                    .npeer_closed_attempt_frames
                    .contains_key(&PlayerHandle::new(2)),
                "the fail-closed cancel records no closed-attempt high-water"
            );

            // --- The joiner leaks real inputs at/past F: the frozen queue
            // ignores them, nothing resurrects the cancelled attempt, and
            // B's session keeps advancing.
            for k in 0..8_i32 {
                c2.send_input(Frame::new(serve_f.as_i32() + k), C_REJOIN_INPUT);
                for _ in 0..3 {
                    duo.poll_round(Some(&mut c2));
                }
                duo.advance_both(56, 133);
                assert!(
                    duo.b.hot_join.pending_reactivation.is_none(),
                    "no leaked input resurrects the cancelled attempt"
                );
            }
            assert_eq!(
                duo.b.local_connect_status[2], pre_status,
                "the leak cannot touch the cancelled slot (receipts are gated on connected)"
            );

            // --- The delayed JoinAborted finally lands: with the pending
            // gone it is the no-pending no-op.
            duo.bus.unblock(addr_a(), addr_b(), "JoinAborted");
            for _ in 0..6 {
                duo.poll_round(Some(&mut c2));
            }
            assert!(
                duo.b.hot_join.pending_reactivation.is_none(),
                "the late JoinAborted does not resurrect the attempt"
            );
            assert_eq!(
                duo.b.local_connect_status[2], pre_status,
                "the late JoinAborted is a harmless no-op on the cancelled attempt"
            );

            // --- Liveness + mesh-wide consistency: both sessions keep
            // advancing (advance_both fails the test on any Err), both
            // confirmed streams keep growing, the slot stays frozen on BOTH
            // peers, and the shadows byte-agree across the window the wedge
            // used to occupy.
            let a_confirmed_before = duo.a.confirmed_frame();
            let b_confirmed_before = duo.b.confirmed_frame();
            for _ in 0..12 {
                for _ in 0..3 {
                    duo.poll_round(Some(&mut c2));
                }
                duo.advance_both(57, 134);
            }
            assert!(
                duo.b.confirmed_frame() > b_confirmed_before,
                "B's confirmed stream keeps growing after the cancel (was {}, now {})",
                b_confirmed_before,
                duo.b.confirmed_frame()
            );
            assert!(
                duo.a.confirmed_frame() > a_confirmed_before,
                "A's confirmed stream keeps growing (was {}, now {})",
                a_confirmed_before,
                duo.a.confirmed_frame()
            );
            assert!(
                duo.a.sync_layer.player_is_frozen(PlayerHandle::new(2))
                    && duo.a.local_connect_status[2].disconnected,
                "the slot is frozen on the coordinator too"
            );
            assert_eq!(
                duo.a.local_connect_status[2], duo.b.local_connect_status[2],
                "the slot's frozen view is consistent mesh-wide"
            );
            let probe = serve_f.as_i32() + 1;
            assert!(
                duo.a_shadow.states.contains_key(&probe)
                    && duo.b_shadow.states.contains_key(&probe),
                "both shadows saved frame F + 1"
            );
            assert_eq!(
                duo.a_shadow.states.get(&probe),
                duo.b_shadow.states.get(&probe),
                "A and B byte-agree at F + 1 (the frames the stale reopen would have corrupted)"
            );
        }

        /// Session-33 round-4 review Finding 1, the second reopen site: the
        /// DEFENSIVE reopen in `handle_join_committed_directive` must apply
        /// the same F re-validation. A `JoinCommitted` matching a PRE-reopen
        /// pending is already a protocol violation (the commit barrier
        /// requires this survivor's reopen-ack, which was never sent), so
        /// the message is untrusted; if the activation frame is ALSO no
        /// longer past B's confirmed history, reopening "defensively" would
        /// reposition the queue below confirmed history and permanently
        /// wedge the session (the probe's `WrongSavedFrame` loop) — strictly
        /// worse than any stall the defensive reopen exists to avoid. The
        /// fabricated-commit shape is staged through the lifecycle test seam
        /// exactly like `npeer_survivor_ignores_mismatched_lifecycle_messages`.
        #[test]
        fn npeer_defensive_reopen_below_confirmed_history_is_cancelled_fail_closed() {
            let mut duo = mesh_with_dropped_slot(40, 6);
            let (mut c2, serve_f, pre_status) =
                reach_pre_reopen_pending_with_confirmed_past_f(&mut duo);

            // --- A JoinCommitted for the held attempt arrives (fabricated:
            // the real serve aborted, and B never acked — a commit without
            // this survivor's ack is exactly the coordinator/channel
            // misbehavior the defensive arm guards).
            duo.b
                .player_reg
                .remotes
                .get_mut(&addr_a())
                .expect("B's coordinator endpoint")
                .set_received_join_committed_for_test(JoinCommitted {
                    handle: 2,
                    frame: serve_f,
                });
            duo.b.poll_remote_clients();

            // --- The stale-F defensive reopen is rejected: cancelled
            // fail-closed instead (pre-fix this reopened the slot below
            // confirmed history: status {connected, F - 1}, queue unfrozen,
            // reserved membership dropped, commit-arm re-seed +
            // closed-attempt high-water).
            assert!(
                duo.b.sync_layer.player_is_frozen(PlayerHandle::new(2)),
                "B must NOT defensively reopen slot 2 below its confirmed history (F = {}, confirmed = {})",
                serve_f,
                duo.b.confirmed_frame()
            );
            assert!(
                duo.b.hot_join.pending_reactivation.is_none(),
                "the stale-F attempt is cancelled fail-closed"
            );
            assert_eq!(
                duo.b.local_connect_status[2], pre_status,
                "slot 2 keeps its pre-attempt frozen status verbatim"
            );
            assert!(
                duo.b
                    .hot_join
                    .reserved_slots
                    .contains(&PlayerHandle::new(2)),
                "slot 2 stays reserved"
            );
            assert!(
                !duo.b
                    .hot_join
                    .npeer_closed_attempt_frames
                    .contains_key(&PlayerHandle::new(2)),
                "the fail-closed cancel records no closed-attempt high-water"
            );

            // --- Liveness: the session keeps advancing; the late
            // JoinAborted is a no-op; the mesh stays byte-consistent.
            duo.bus.unblock(addr_a(), addr_b(), "JoinAborted");
            let b_confirmed_before = duo.b.confirmed_frame();
            for _ in 0..12 {
                for _ in 0..3 {
                    duo.poll_round(Some(&mut c2));
                }
                duo.advance_both(58, 135);
            }
            assert!(
                duo.b.confirmed_frame() > b_confirmed_before,
                "B keeps advancing after the fail-closed cancel (was {}, now {})",
                b_confirmed_before,
                duo.b.confirmed_frame()
            );
            assert!(
                duo.b.hot_join.pending_reactivation.is_none(),
                "nothing resurrects the cancelled attempt"
            );
            assert_eq!(
                duo.b.local_connect_status[2], pre_status,
                "slot 2 stays in the pre-attempt frozen shape"
            );
            let probe = serve_f.as_i32() + 1;
            assert!(
                duo.a_shadow.states.contains_key(&probe),
                "the byte-agreement probe is non-vacuous"
            );
            assert_eq!(
                duo.a_shadow.states.get(&probe),
                duo.b_shadow.states.get(&probe),
                "A and B byte-agree at F + 1"
            );
        }

        /// Stages a reopened pending reactivation on B with a legal
        /// pre-commit joiner input leak at `F` (the serve stays open — the
        /// generous timeout keeps the attempt mid-flight), returning the
        /// joiner driver and the activation frame. Shared by the
        /// user-initiated-kick tests (session-33 round-3 review Finding 2).
        fn reach_reopened_pending_with_leaked_input(duo: &mut Duo) -> (ManualJoiner, Frame) {
            let mut c2 = ManualJoiner::new(&duo.bus.clone(), addr_c());
            c2.connect(addr_a(), 0, &duo.clock.clone());
            sync_joiner_with(duo, &mut c2, addr_a());
            c2.proto_mut(addr_a()).send_join_request(2);
            c2.pump();
            duo.poll_round(Some(&mut c2));
            let serve_f = duo
                .a
                .hot_join
                .npeer
                .as_ref()
                .expect("N-peer serve opens")
                .activation_frame;
            for _ in 0..4 {
                duo.poll_round(Some(&mut c2));
                duo.advance_both(57, 134);
            }
            c2.connect(addr_b(), 1, &duo.clock.clone());
            for _ in 0..60 {
                duo.poll_round(Some(&mut c2));
                duo.advance_both(58, 135);
                if duo
                    .b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened)
                    && c2.is_running(addr_b())
                {
                    break;
                }
            }
            assert!(
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened),
                "B reopened + acked"
            );

            // The legal pre-commit leak: the joiner streams its real input at
            // F into B's reopened queue (the poisonous receipt a mid-attempt
            // kick would otherwise freeze).
            for _ in 0..10 {
                c2.send_input(serve_f, C_REJOIN_INPUT);
                duo.poll_round(Some(&mut c2));
                if duo.b.local_connect_status[2].last_frame >= serve_f {
                    break;
                }
            }
            assert!(
                duo.b.local_connect_status[2].last_frame >= serve_f,
                "precondition: B's local receipt for the reopened slot reached F via the leak (got {}, F = {})",
                duo.b.local_connect_status[2].last_frame,
                serve_f
            );
            (c2, serve_f)
        }

        /// User-initiated freeze guard (session-33 round-3 review Finding 2):
        /// `disconnect_player` on a slot held by a REOPENED pending
        /// reactivation must close the attempt first through the same
        /// evidence-discriminated close the joiner-death path uses, then
        /// apply the user's request to the post-close state. Mid-attempt
        /// there is no commit evidence, so the close takes the abort arm —
        /// the verbatim pre-reopen restore — and the kick then reports the
        /// slot as already disconnected. Without the guard the kick froze
        /// the slot at the LEAKED receipt `>= F` while the pending stayed
        /// held: a `{disconnected, >= F}` claim in an aborted world — the
        /// exact shape the commit-evidence induction proves cannot otherwise
        /// exist — which another survivor's close would read as commit
        /// evidence (the mistaken-commit residual class).
        #[test]
        fn npeer_disconnect_player_on_reopened_pending_slot_closes_attempt_first() {
            let mut duo = mesh_with_dropped_slot(600, 6);
            let slot = PlayerHandle::new(2);
            let pre_status = duo.b.local_connect_status[2];

            // Non-pending control: with NO attempt in flight the dropped
            // slot's public-API semantics are untouched (fail-closed at the
            // existing already-disconnected guards).
            assert!(
                matches!(
                    duo.b.disconnect_player(slot),
                    Err(FortressError::InvalidRequestStructured {
                        kind: InvalidRequestKind::AlreadyDisconnected { .. }
                    })
                ),
                "control: disconnect_player on the dropped non-pending slot is already-disconnected"
            );

            let (mut c2, serve_f) = reach_reopened_pending_with_leaked_input(&mut duo);

            // --- The mid-attempt kick: the attempt must close FIRST (abort
            // arm — no commit evidence exists), then the kick applies to the
            // post-close state.
            let result = duo.b.disconnect_player(slot);
            assert!(
                matches!(
                    result,
                    Err(FortressError::InvalidRequestStructured {
                        kind: InvalidRequestKind::AlreadyDisconnected { .. }
                    })
                ),
                "a mid-attempt disconnect_player closes the attempt first (abort arm restores the drop), then reports already-disconnected (got {result:?})"
            );
            assert!(
                duo.b.hot_join.pending_reactivation.is_none(),
                "the user kick closed the reopened attempt"
            );
            assert_eq!(
                duo.b.local_connect_status[2], pre_status,
                "the close restored the pre-reopen connection status verbatim (NOT the leaked >= F receipt; F = {serve_f})"
            );
            assert!(
                duo.b.hot_join.reserved_slots.contains(&slot),
                "the abort arm re-reserved the slot"
            );
            assert_eq!(
                duo.b.current_state(),
                SessionState::Running,
                "the rejected kick must not halt the session (the Halt policy never ran)"
            );
            drop(c2.protos.remove(&addr_b()));
        }

        /// [`npeer_disconnect_player_on_reopened_pending_slot_closes_attempt_first`]
        /// for the second public entry point: `remove_player` on a slot held
        /// by a REOPENED pending reactivation closes the attempt first
        /// (abort arm mid-attempt) and then reports the slot as already
        /// removed — never a mid-attempt freeze at the leaked receipt.
        #[test]
        fn npeer_remove_player_on_reopened_pending_slot_closes_attempt_first() {
            let mut duo = mesh_with_dropped_slot(600, 6);
            let slot = PlayerHandle::new(2);
            let pre_status = duo.b.local_connect_status[2];

            // Non-pending control: existing semantics untouched.
            assert!(
                matches!(
                    duo.b.remove_player(slot),
                    Err(FortressError::InvalidRequestStructured {
                        kind: InvalidRequestKind::PlayerAlreadyRemoved { .. }
                    })
                ),
                "control: remove_player on the dropped non-pending slot is already-removed"
            );

            let (mut c2, serve_f) = reach_reopened_pending_with_leaked_input(&mut duo);

            let result = duo.b.remove_player(slot);
            assert!(
                matches!(
                    result,
                    Err(FortressError::InvalidRequestStructured {
                        kind: InvalidRequestKind::PlayerAlreadyRemoved { .. }
                    })
                ),
                "a mid-attempt remove_player closes the attempt first (abort arm restores the drop), then reports already-removed (got {result:?})"
            );
            assert!(
                duo.b.hot_join.pending_reactivation.is_none(),
                "the user removal closed the reopened attempt"
            );
            assert_eq!(
                duo.b.local_connect_status[2], pre_status,
                "the close restored the pre-reopen connection status verbatim (NOT the leaked >= F receipt; F = {serve_f})"
            );
            assert!(
                duo.b.hot_join.reserved_slots.contains(&slot),
                "the abort arm re-reserved the slot"
            );
            assert_eq!(
                duo.b.current_state(),
                SessionState::Running,
                "the rejected removal must not halt the session"
            );
            drop(c2.protos.remove(&addr_b()));
        }

        /// Floor lifecycle (session-33 round-2 review Finding 1): the
        /// per-slot gossip reactivation floor arms ONLY at commit-evidence
        /// points. While a survivor's attempt is reopened-but-unconcluded the
        /// floor stays NULL (the pending shield owns that window, and an
        /// abort must leave the mesh free to re-converge on f0); the
        /// `JoinCommitted` receipt — the survivor's commit evidence — arms it
        /// at `F - 1` on every endpoint, exactly like the coordinator's own
        /// commit does.
        #[test]
        fn npeer_reactivation_floor_arms_at_commit_receipt_not_at_reopen() {
            let mut duo = mesh_with_dropped_slot(600, 6);
            let slot = PlayerHandle::new(2);

            // B must not hear the commit until the test says so.
            duo.bus.block(addr_a(), addr_b(), "JoinCommitted");

            let mut c2 = ManualJoiner::new(&duo.bus.clone(), addr_c());
            c2.connect(addr_a(), 0, &duo.clock.clone());
            sync_joiner_with(&mut duo, &mut c2, addr_a());
            c2.proto_mut(addr_a()).send_join_request(2);
            c2.pump();
            duo.poll_round(Some(&mut c2));
            let serve_f = duo
                .a
                .hot_join
                .npeer
                .as_ref()
                .expect("N-peer serve opens")
                .activation_frame;
            for _ in 0..4 {
                duo.poll_round(Some(&mut c2));
                duo.advance_both(71, 81);
            }
            c2.connect(addr_b(), 1, &duo.clock.clone());
            for _ in 0..60 {
                duo.poll_round(Some(&mut c2));
                duo.advance_both(72, 82);
                if duo
                    .b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened)
                    && c2.is_running(addr_b())
                {
                    break;
                }
            }
            assert!(
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened),
                "B reopened + acked"
            );

            // --- Reopened but unconcluded: floors NULL on every B endpoint.
            for addr in [addr_a(), addr_c()] {
                let floor = duo
                    .b
                    .player_reg
                    .remotes
                    .get(&addr)
                    .expect("B endpoint exists")
                    .reactivation_floor_for_test(slot);
                assert!(
                    floor.is_null(),
                    "B's reopen is pre-commit: no reactivation floor may be armed yet on endpoint {:?} (got {})",
                    addr,
                    floor
                );
            }

            // --- The joiner acks the snapshot; the barrier commits on A,
            // arming A's floors.
            let mut snapshot = None;
            for _ in 0..30 {
                duo.poll_round(Some(&mut c2));
                duo.advance_both(73, 83);
                if let Some(snap) = c2.proto_mut(addr_a()).take_received_snapshot() {
                    snapshot = Some(snap);
                    break;
                }
            }
            let snapshot = snapshot.expect("the joiner receives the snapshot");
            c2.proto_mut(addr_a())
                .send_state_snapshot_ack(snapshot.frame);
            c2.pump();
            for _ in 0..6 {
                duo.poll_round(Some(&mut c2));
                if duo.a.hot_join.npeer.is_none() {
                    break;
                }
            }
            assert!(duo.a.hot_join.npeer.is_none(), "the serve committed on A");
            let expected_floor = Frame::new(serve_f.as_i32() - 1);
            for addr in [addr_b(), addr_c()] {
                let floor = duo
                    .a
                    .player_reg
                    .remotes
                    .get(&addr)
                    .expect("A endpoint exists")
                    .reactivation_floor_for_test(slot);
                assert_eq!(
                    floor, expected_floor,
                    "A's commit armed the reactivation floor at F - 1 on endpoint {addr:?}"
                );
            }
            // B still has not heard the commit: pending held, floors NULL.
            assert!(
                duo.b.hot_join.pending_reactivation.is_some(),
                "B's pending is held while JoinCommitted is blocked"
            );
            for addr in [addr_a(), addr_c()] {
                let floor = duo
                    .b
                    .player_reg
                    .remotes
                    .get(&addr)
                    .expect("B endpoint exists")
                    .reactivation_floor_for_test(slot);
                assert!(
                    floor.is_null(),
                    "still no commit evidence on B: floor on endpoint {:?} must stay NULL (got {})",
                    addr,
                    floor
                );
            }

            // --- Deliver the commit receipt: B's floors arm at F - 1.
            duo.bus.unblock(addr_a(), addr_b(), "JoinCommitted");
            for _ in 0..10 {
                duo.poll_round(Some(&mut c2));
                if duo.b.hot_join.pending_reactivation.is_none() {
                    break;
                }
            }
            assert!(
                duo.b.hot_join.pending_reactivation.is_none(),
                "the JoinCommitted receipt closes B's attempt"
            );
            for addr in [addr_a(), addr_c()] {
                let floor = duo
                    .b
                    .player_reg
                    .remotes
                    .get(&addr)
                    .expect("B endpoint exists")
                    .reactivation_floor_for_test(slot);
                assert_eq!(
                    floor, expected_floor,
                    "B's commit receipt armed the reactivation floor at F - 1 on endpoint {addr:?}"
                );
            }
        }

        /// Commit-evidence discrimination under input starvation (session-33
        /// round-2 review Finding 2): an attempt COMMITS but survivor B
        /// receives zero joiner inputs (joiner->B loss) and zero
        /// `JoinCommitted` copies (coordinator->B selective loss) before the
        /// joiner endpoint dies. B's local evidence (confirmed frame /
        /// discard high-water) is then capped at `F - 1`, but once the joiner
        /// dies toward the coordinator first (the committed era's natural
        /// detection order under a joiner crash), A's ordinary re-drop
        /// freezes the slot at its real receipt `>= F` and gossips
        /// `{disconnected, >= F}` to B — a claim NO uncommitted world can
        /// produce (an abort restore freezes at `f0 <= F - 1`; no first
        /// `>= F` freeze exists absent a commit, leaks included). The
        /// joiner-death close must read that gossip leg and take the COMMIT
        /// arm — the ordinary re-drop at the local receipt (`F - 1`), NOT the
        /// pre-attempt restore to `(f0, v0)` — after which the ordinary
        /// gossip min converges the whole mesh to `(F - 1, frozen value)` and
        /// both peers resume past `F` byte-identically. Restoring `f0`
        /// instead diverges B's frozen history from the committed peers'
        /// (whose armed floors then filter B's stale `f0` claims forever — a
        /// permanent stall): the repo principle is stall over desync, but
        /// here the commit is provable from the caches, so neither is
        /// acceptable.
        #[test]
        fn npeer_starved_survivor_joiner_death_uses_gossip_commit_evidence() {
            let mut duo = mesh_with_dropped_slot(600, 6);
            let slot = PlayerHandle::new(2);
            let pre_status = duo.b.local_connect_status[2];
            let f0 = pre_status.last_frame;

            // The dual blackout: B never hears the commit and never receives
            // a joiner input. (The joiner's B-channel still handshakes — sync
            // messages are not blocked — so B reopens and the barrier
            // commits.)
            duo.bus.block(addr_a(), addr_b(), "JoinCommitted");
            duo.bus.block(addr_c(), addr_b(), "Input");

            // --- Standard rejoin through B's reopen + ack.
            let mut c2 = ManualJoiner::new(&duo.bus.clone(), addr_c());
            c2.connect(addr_a(), 0, &duo.clock.clone());
            sync_joiner_with(&mut duo, &mut c2, addr_a());
            c2.proto_mut(addr_a()).send_join_request(2);
            c2.pump();
            duo.poll_round(Some(&mut c2));
            let serve_f = duo
                .a
                .hot_join
                .npeer
                .as_ref()
                .expect("N-peer serve opens")
                .activation_frame;
            let seeded = Frame::new(serve_f.as_i32() - 1);
            assert!(
                f0 < seeded,
                "precondition: f0 ({}) < F - 1 ({}) so the two close arms are distinguishable",
                f0,
                seeded
            );
            for _ in 0..4 {
                duo.poll_round(Some(&mut c2));
                duo.advance_both(71, 81);
            }
            c2.connect(addr_b(), 1, &duo.clock.clone());
            for _ in 0..60 {
                duo.poll_round(Some(&mut c2));
                duo.advance_both(72, 82);
                if duo
                    .b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened)
                    && c2.is_running(addr_b())
                {
                    break;
                }
            }
            assert!(
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened),
                "B reopened + acked"
            );

            // --- The joiner acks the snapshot; the barrier commits on A.
            let mut snapshot = None;
            for _ in 0..30 {
                duo.poll_round(Some(&mut c2));
                duo.advance_both(73, 83);
                if let Some(snap) = c2.proto_mut(addr_a()).take_received_snapshot() {
                    snapshot = Some(snap);
                    break;
                }
            }
            let snapshot = snapshot.expect("the joiner receives the snapshot");
            c2.proto_mut(addr_a())
                .send_state_snapshot_ack(snapshot.frame);
            c2.pump();
            for _ in 0..6 {
                duo.poll_round(Some(&mut c2));
                if duo.a.hot_join.npeer.is_none() {
                    break;
                }
            }
            assert!(duo.a.hot_join.npeer.is_none(), "the serve committed on A");

            // --- The joiner streams real inputs from F: they reach A only.
            for k in 0..3_i32 {
                c2.send_input(Frame::new(serve_f.as_i32() + k), C_REJOIN_INPUT);
                for _ in 0..3 {
                    duo.poll_round(Some(&mut c2));
                }
                duo.advance_both(74, 84);
            }

            // Starvation preconditions, pinned explicitly.
            assert!(
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened),
                "B still holds the reopened attempt (no JoinCommitted got through)"
            );
            assert_eq!(
                duo.b.local_connect_status[2].last_frame, seeded,
                "B received ZERO joiner inputs (local receipt is still the seeded F - 1)"
            );
            assert!(
                duo.b.confirmed_frame() < serve_f,
                "B's confirmed frame is capped below F (no local commit evidence; got {})",
                duo.b.confirmed_frame()
            );

            // --- The joiner dies toward A FIRST (the committed era's natural
            // crash-detection order): A's endpoint times out, A re-drops the
            // slot at its real receipt >= F, and its gossip carries the
            // committed era's {disconnected, >= F} freeze claim to B.
            c2.protos.remove(&addr_a());
            for _ in 0..120 {
                duo.poll_round(Some(&mut c2));
                duo.advance_both(75, 85);
                if duo.a.local_connect_status[2].disconnected {
                    break;
                }
            }
            assert!(
                duo.a.local_connect_status[2].disconnected,
                "A re-dropped the slot when the joiner died"
            );
            assert!(
                duo.a.local_connect_status[2].last_frame >= serve_f,
                "A's re-drop froze at its real receipt (got {}, F = {})",
                duo.a.local_connect_status[2].last_frame,
                serve_f
            );
            for _ in 0..10 {
                duo.poll_round(Some(&mut c2));
                duo.advance_both(75, 85);
                let claim = duo
                    .b
                    .player_reg
                    .remotes
                    .get(&addr_a())
                    .expect("B's A endpoint")
                    .peer_connect_status(slot);
                if claim.disconnected && claim.last_frame >= serve_f {
                    break;
                }
            }
            let b_view_of_a = duo
                .b
                .player_reg
                .remotes
                .get(&addr_a())
                .expect("B's A endpoint")
                .peer_connect_status(slot);
            assert!(
                b_view_of_a.disconnected && b_view_of_a.last_frame >= serve_f,
                "the committed era's re-drop gossip reached B (claim disconnected={} at {}, F = {})",
                b_view_of_a.disconnected,
                b_view_of_a.last_frame,
                serve_f
            );
            assert!(
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened),
                "B's attempt is still pending (its joiner channel is alive)"
            );

            // --- Now the joiner dies toward B too: B's joiner-endpoint close
            // must discriminate COMMITTED from the gossip freeze-frame leg.
            for _ in 0..120 {
                duo.poll_round(None);
                duo.advance_both(76, 86);
                if duo.b.hot_join.pending_reactivation.is_none()
                    && duo.b.local_connect_status[2].disconnected
                {
                    break;
                }
            }
            assert!(
                duo.b.hot_join.pending_reactivation.is_none(),
                "the joiner endpoint's death closed B's attempt"
            );
            assert!(
                duo.b.local_connect_status[2].disconnected,
                "B re-dropped the slot"
            );
            assert_eq!(
                duo.b.local_connect_status[2].last_frame, seeded,
                "B closed the attempt as COMMITTED: the committed era's re-drop freezes at the local receipt F - 1, not the pre-attempt f0 ({})",
                f0
            );
            assert!(
                !duo.b.hot_join.reserved_slots.contains(&slot),
                "the commit arm does not re-reserve the slot (this was the committed era's ordinary drop)"
            );

            // --- Convergence: the ordinary gossip min settles the WHOLE mesh
            // at (F - 1, frozen value); both peers resume and byte-agree
            // past F.
            let probe = serve_f.as_i32() + 1;
            let mut b_frozen_value_checked = false;
            for _ in 0..120 {
                for _ in 0..3 {
                    duo.poll_round(None);
                }
                duo.a
                    .add_local_input(PlayerHandle::new(0), 76)
                    .expect("A local input");
                let requests = duo.a.advance_frame().expect("A advance");
                apply_requests(&requests, &mut duo.a_shadow);
                duo.b
                    .add_local_input(PlayerHandle::new(1), 86)
                    .expect("B local input");
                let requests = duo.b.advance_frame().expect("B advance");
                for request in requests.iter() {
                    if let FortressRequest::AdvanceFrame { inputs } = request {
                        let (value, status) = inputs[2];
                        assert_eq!(
                            value, C_FROZEN_INPUT,
                            "B serves the converged frozen value for the re-dropped slot"
                        );
                        assert_eq!(status, InputStatus::Disconnected);
                        b_frozen_value_checked = true;
                    }
                }
                apply_requests(&requests, &mut duo.b_shadow);
                if duo.a.confirmed_frame().as_i32() > probe
                    && duo.b.confirmed_frame().as_i32() > probe
                    && duo.a_shadow.states.contains_key(&probe)
                    && duo.b_shadow.states.contains_key(&probe)
                {
                    break;
                }
            }
            assert!(
                b_frozen_value_checked,
                "B advanced at least one frame post-close"
            );
            assert_eq!(
                duo.a.local_connect_status[2].last_frame, seeded,
                "A's freeze frame converged down to B's F - 1 (the committed mesh's re-drop min)"
            );
            assert!(
                duo.b.confirmed_frame().as_i32() > probe
                    && duo.a.confirmed_frame().as_i32() > probe,
                "both peers resumed past F (A {}, B {})",
                duo.a.confirmed_frame(),
                duo.b.confirmed_frame()
            );
            assert_eq!(
                duo.a_shadow.states.get(&probe),
                duo.b_shadow.states.get(&probe),
                "A and B byte-agree at F + 1 (the starved survivor joined the committed era's converged re-drop — no silent divergence)"
            );
        }

        /// Stale-attempt discrimination (R3): lifecycle messages whose
        /// `(handle, frame)` or sender do not match the pending attempt are
        /// ignored with no state change.
        #[test]
        fn npeer_survivor_ignores_mismatched_lifecycle_messages() {
            let mut duo = mesh_with_dropped_slot(600, 6);

            let mut c2 = ManualJoiner::new(&duo.bus.clone(), addr_c());
            c2.connect(addr_a(), 0, &duo.clock.clone());
            sync_joiner_with(&mut duo, &mut c2, addr_a());
            c2.proto_mut(addr_a()).send_join_request(2);
            c2.pump();
            duo.poll_round(Some(&mut c2));
            let serve_f = duo
                .a
                .hot_join
                .npeer
                .as_ref()
                .expect("N-peer serve opens")
                .activation_frame;

            // Reach the reopened state on B.
            for _ in 0..4 {
                duo.poll_round(Some(&mut c2));
                duo.advance_both(130, 140);
            }
            c2.connect(addr_b(), 1, &duo.clock.clone());
            for _ in 0..60 {
                duo.poll_round(Some(&mut c2));
                duo.advance_both(131, 141);
                if duo
                    .b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened)
                    && c2.is_running(addr_b())
                {
                    break;
                }
            }
            assert!(
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened),
                "B reopened"
            );

            // Pending-attempt shield: re-stick B's cached view of A's claim
            // for slot 2 to the pre-attempt disconnected state (exactly what a
            // not-yet-reopened survivor's gossip, the paused coordinator's
            // multi-slot connect-status nudge, or a reordered stale packet
            // would deliver mid-attempt). The disconnect-propagation fold must
            // NOT re-apply the drop to the reopened slot while the attempt is
            // pending — without the `update_player_disconnects` shield this
            // re-freezes the slot and wedges the attempt.
            let restick = ConnectionStatus {
                disconnected: true,
                last_frame: Frame::new(serve_f.as_i32() - 3),
            };
            duo.b
                .player_reg
                .remotes
                .get_mut(&addr_a())
                .expect("B's coordinator endpoint")
                .set_peer_connect_status_for_tests(PlayerHandle::new(2), restick);
            duo.b
                .add_local_input(PlayerHandle::new(1), 142)
                .expect("B local input");
            let requests = duo.b.advance_frame().expect("B advance");
            apply_requests(&requests, &mut duo.b_shadow);
            assert!(
                !duo.b.local_connect_status[2].disconnected,
                "the pending-attempt shield keeps the reopened slot connected against stale disconnect gossip"
            );
            assert!(
                !duo.b.sync_layer.player_is_frozen(PlayerHandle::new(2)),
                "the reopened slot stays unfrozen under stale disconnect gossip"
            );

            let wrong_frame = Frame::new(serve_f.as_i32() + 5);

            // A JoinAborted with a mismatched F (staged via the test seam on
            // the coordinator's endpoint — the genuine wire path cannot forge
            // one) must be ignored: still pending, still reopened, still live.
            duo.b
                .player_reg
                .remotes
                .get_mut(&addr_a())
                .expect("B's coordinator endpoint")
                .set_received_join_aborted_for_test(JoinAborted {
                    handle: 2,
                    frame: wrong_frame,
                });
            duo.b.poll_remote_clients();
            assert!(
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened && pending.frame == serve_f),
                "a mismatched-frame JoinAborted is ignored"
            );
            assert!(
                !duo.b.sync_layer.player_is_frozen(PlayerHandle::new(2)),
                "the slot stays live after the stale JoinAborted"
            );

            // A JoinCommitted with a mismatched F is likewise ignored.
            duo.b
                .player_reg
                .remotes
                .get_mut(&addr_a())
                .expect("B's coordinator endpoint")
                .set_received_join_committed_for_test(JoinCommitted {
                    handle: 2,
                    frame: wrong_frame,
                });
            duo.b.poll_remote_clients();
            assert!(
                duo.b.hot_join.pending_reactivation.is_some(),
                "a mismatched-frame JoinCommitted is ignored"
            );

            // A correctly-framed JoinAborted from the WRONG SENDER (the joiner
            // itself) is ignored: lifecycle authority is attempt-scoped to the
            // coordinator address.
            duo.b
                .player_reg
                .remotes
                .get_mut(&addr_c())
                .expect("B's joiner endpoint")
                .set_received_join_aborted_for_test(JoinAborted {
                    handle: 2,
                    frame: serve_f,
                });
            duo.b.poll_remote_clients();
            assert!(
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened),
                "a JoinAborted from a non-coordinator sender is ignored"
            );
            assert!(
                !duo.b.sync_layer.player_is_frozen(PlayerHandle::new(2)),
                "the slot stays live after the forged JoinAborted"
            );
        }

        /// Local joiner-endpoint-death close (session-33 review Finding 6):
        /// a survivor whose REOPENED pending attempt loses BOTH its
        /// coordinator (death — no lifecycle message will ever arrive) and
        /// then its joiner endpoint must close the attempt locally off the
        /// joiner endpoint's `Event::Disconnected`: re-freeze the slot
        /// byte-identically to the agreed pre-attempt state (the same restore
        /// every other reopened survivor applies), clear the pending entry
        /// (un-wedging the shield and every future directive for the handle),
        /// record the closed-attempt high-water, and resume. Without the
        /// close the pending survives forever: the shield permanently exempts
        /// the slot from the disconnect fold and `reopened`-never-superseded
        /// blocks every future rejoin of the slot mesh-wide.
        ///
        /// The remaining window — between the coordinator's death and the
        /// joiner endpoint's death — is inherent to R4 (a survivor never
        /// guesses a pending attempt's outcome); bounding it is the chunk-N4
        /// joiner-teardown contract (a joiner that loses its coordinator must
        /// tear down its survivor channels so this close fires).
        #[test]
        fn npeer_survivor_refreezes_and_unwedges_when_joiner_dies_after_coordinator_death() {
            let mut duo = mesh_with_dropped_slot(600, 6);
            let pre_status = duo.b.local_connect_status[2];
            assert!(pre_status.disconnected, "slot 2 starts dropped on B");

            // --- Reach a REOPENED pending on B.
            let mut c2 = ManualJoiner::new(&duo.bus.clone(), addr_c());
            c2.connect(addr_a(), 0, &duo.clock.clone());
            sync_joiner_with(&mut duo, &mut c2, addr_a());
            c2.proto_mut(addr_a()).send_join_request(2);
            c2.pump();
            duo.poll_round(Some(&mut c2));
            let serve_f = duo
                .a
                .hot_join
                .npeer
                .as_ref()
                .expect("N-peer serve opens")
                .activation_frame;
            for _ in 0..4 {
                duo.poll_round(Some(&mut c2));
            }
            c2.connect(addr_b(), 1, &duo.clock.clone());
            for _ in 0..60 {
                duo.poll_round(Some(&mut c2));
                if duo
                    .b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened)
                    && c2.is_running(addr_b())
                {
                    break;
                }
            }
            assert!(
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened && pending.frame == serve_f),
                "B reopened for the attempt"
            );

            // The joiner leaks a real input at F into B's reopened queue, so
            // the re-freeze below must demonstrably restore the AGREED value,
            // not the leaked one.
            c2.send_input(serve_f, C_REJOIN_INPUT);
            for _ in 0..3 {
                duo.poll_round(Some(&mut c2));
            }

            // --- The coordinator DIES before any lifecycle message: B polls
            // alone (with the joiner still alive) until A's endpoint times
            // out and slot 0 drops via the ordinary machinery. The pending
            // attempt is now unfinishable-by-lifecycle.
            let poll_b_solo = |duo: &mut Duo, joiner: Option<&mut ManualJoiner>| {
                duo.b.poll_remote_clients();
                duo.b_events.extend(duo.b.events());
                if let Some(joiner) = joiner {
                    joiner.pump();
                }
                duo.clock.advance(POLL_INTERVAL);
            };
            for _ in 0..60 {
                poll_b_solo(&mut duo, Some(&mut c2));
                if duo.b.local_connect_status[0].disconnected {
                    break;
                }
            }
            assert!(
                duo.b.local_connect_status[0].disconnected,
                "B dropped the dead coordinator's slot via the normal machinery"
            );
            assert!(
                duo.b.hot_join.pending_reactivation.is_some(),
                "the pending attempt is still held (no lifecycle message can arrive)"
            );

            // --- The joiner endpoint then dies too: B must close the attempt
            // locally — byte-identical pre-attempt re-freeze + cleared
            // pending + recorded high-water.
            for _ in 0..60 {
                poll_b_solo(&mut duo, None);
                if duo.b.hot_join.pending_reactivation.is_none() {
                    break;
                }
            }
            assert!(
                duo.b.hot_join.pending_reactivation.is_none(),
                "the joiner endpoint's death closes the wedged reopened attempt"
            );
            assert!(
                duo.b.sync_layer.player_is_frozen(PlayerHandle::new(2)),
                "B re-froze the slot"
            );
            assert_eq!(
                duo.b.local_connect_status[2], pre_status,
                "B restored the pre-attempt connection status verbatim (not the joiner-era receipt)"
            );
            assert!(
                duo.b
                    .hot_join
                    .reserved_slots
                    .contains(&PlayerHandle::new(2)),
                "the slot is reserved again on B (rejoinable)"
            );

            // --- B RESUMES solo: both remote slots are excluded (mesh-agreed
            // drops), and the re-frozen slot feeds the AGREED value, not the
            // leaked one.
            let mut frozen_checked = false;
            let start = duo.b.current_frame();
            for i in 0..8_u8 {
                poll_b_solo(&mut duo, None);
                duo.b
                    .add_local_input(PlayerHandle::new(1), 210 + i)
                    .expect("B local input");
                let requests = duo.b.advance_frame().expect("B advance");
                for request in requests.iter() {
                    if let FortressRequest::AdvanceFrame { inputs } = request {
                        let (value, status) = inputs[2];
                        assert_eq!(
                            value, C_FROZEN_INPUT,
                            "the re-frozen slot feeds the agreed value, not the leaked joiner input"
                        );
                        assert_eq!(status, InputStatus::Disconnected);
                        frozen_checked = true;
                    }
                }
                apply_requests(&requests, &mut duo.b_shadow);
            }
            assert!(frozen_checked, "B advanced at least one frame post-close");
            assert!(
                duo.b.current_frame() > start,
                "B resumed advancing after both deaths"
            );

            // --- Un-wedge pin: a future directive for the slot (a takeover
            // coordinator's retry at a strictly newer frame) is ACCEPTED —
            // the closed attempt no longer blocks the handle.
            let f_retry = Frame::new(
                duo.b
                    .confirmed_frame()
                    .as_i32()
                    .max(serve_f.as_i32())
                    .saturating_add(10),
            );
            duo.b
                .player_reg
                .remotes
                .get_mut(&addr_a())
                .expect("B's coordinator endpoint")
                .set_received_reactivate_slot_for_test(ReactivateSlot {
                    handle: 2,
                    frame: f_retry,
                });
            duo.b.poll_remote_clients();
            assert!(
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.frame == f_retry),
                "a strictly-newer future directive is accepted — the slot is no longer wedged"
            );
        }

        /// Drives the standard rejoin up to B's reopened pending: blocks
        /// `A -> B` inputs first, lets A advance `blocked_frames` frames with
        /// CHANGED inputs (B's repeat-last predictions of A are then wrong for
        /// exactly those sub-F frames), opens the serve, and brings B to the
        /// reopened state. Returns `(joiner, F)`. The caller unblocks to
        /// trigger the sub-F misprediction rollback under test.
        fn reopened_survivor_with_pending_sub_f_misprediction(
            duo: &mut Duo,
            blocked_frames: u8,
        ) -> (ManualJoiner, Frame) {
            duo.bus.block(addr_a(), addr_b(), "Input");
            for i in 0..blocked_frames {
                for _ in 0..3 {
                    duo.poll_round(None);
                }
                // A's inputs CHANGE during the blocked window; B's own stay
                // constant (no reverse misprediction).
                duo.advance_both(240 + i, 80);
            }

            let mut c2 = ManualJoiner::new(&duo.bus.clone(), addr_c());
            c2.connect(addr_a(), 0, &duo.clock.clone());
            sync_joiner_with(duo, &mut c2, addr_a());
            c2.proto_mut(addr_a()).send_join_request(2);
            c2.pump();
            duo.poll_round(Some(&mut c2));
            let serve_f = duo
                .a
                .hot_join
                .npeer
                .as_ref()
                .expect("N-peer serve opens")
                .activation_frame;
            for _ in 0..4 {
                duo.poll_round(Some(&mut c2));
            }
            c2.connect(addr_b(), 1, &duo.clock.clone());
            for _ in 0..60 {
                duo.poll_round(Some(&mut c2));
                duo.advance_both(250, 80);
                if duo
                    .b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened)
                    && c2.is_running(addr_b())
                {
                    break;
                }
            }
            assert!(
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened),
                "B reopened"
            );
            assert!(
                duo.b.current_frame() > serve_f,
                "B speculated past F before the reopen (current {}, F {})",
                duo.b.current_frame(),
                serve_f
            );
            (c2, serve_f)
        }

        /// Unblocks `A -> B`, drives B until the sub-F misprediction rollback
        /// fires, and asserts every re-simulated pre-activation frame of the
        /// reopened slot presents EXACTLY as the pre-reopen simulation did:
        /// the agreed frozen value with `Disconnected` status. Returns the
        /// rollback's load frame.
        fn assert_sub_f_rollback_serves_frozen_disconnected(
            duo: &mut Duo,
            c2: &mut ManualJoiner,
            serve_f: Frame,
            expected_at_or_past_f: Option<u8>,
        ) -> Frame {
            duo.bus.unblock(addr_a(), addr_b(), "Input");
            let mut load_frame = Frame::NULL;
            for _ in 0..12 {
                duo.poll_round(Some(c2));
                duo.b
                    .add_local_input(PlayerHandle::new(1), 80)
                    .expect("B local input");
                let requests = duo
                    .b
                    .advance_frame()
                    .expect("B's sub-F rollback must be servable (reactivated slots present pre-activation frames with the frozen value)");
                let mut cursor = Frame::NULL;
                for request in requests.iter() {
                    match request {
                        FortressRequest::LoadGameState { frame, .. } => {
                            load_frame = *frame;
                            cursor = *frame;
                        },
                        FortressRequest::AdvanceFrame { inputs } => {
                            if !cursor.is_null() {
                                let (value, status) = inputs[2];
                                if cursor < serve_f {
                                    assert_eq!(
                                        value, C_FROZEN_INPUT,
                                        "re-simulated pre-activation frame {} must use the agreed frozen value",
                                        cursor
                                    );
                                    assert_eq!(
                                        status,
                                        InputStatus::Disconnected,
                                        "re-simulated pre-activation frame {} must present Disconnected (as the original simulation did), not a prediction",
                                        cursor
                                    );
                                } else if let Some(expected) = expected_at_or_past_f {
                                    assert_eq!(
                                        value, expected,
                                        "re-simulated frame {} at/past F must use the joiner's real input",
                                        cursor
                                    );
                                }
                                cursor = Frame::new(cursor.as_i32() + 1);
                            }
                        },
                        FortressRequest::SaveGameState { .. } => {},
                    }
                }
                apply_requests(&requests, &mut duo.b_shadow);
                if !load_frame.is_null() {
                    break;
                }
            }
            assert!(
                !load_frame.is_null(),
                "the late-arriving sub-F misprediction triggered a rollback on B"
            );
            assert!(
                load_frame < serve_f,
                "the rollback crossed the activation frame (loaded {}, F {})",
                load_frame,
                serve_f
            );
            load_frame
        }

        /// Sub-F rollback on a reopened slot, status flavor (session-33
        /// review Finding 5): BEFORE any joiner input is confirmed, a
        /// late-arriving misprediction below F rolls B back across
        /// pre-activation frames. The reopened (blanked) queue would serve
        /// them as `(frozen value, Predicted)` — value-correct but
        /// status-divergent from every peer that never rolled back (the
        /// original simulation presented `Disconnected`; status-sensitive
        /// games silently diverge). The reactivation floor must present them
        /// as `(frozen value, Disconnected)`, byte-identical to the
        /// pre-reopen simulation.
        #[test]
        fn npeer_sub_f_rollback_before_joiner_inputs_presents_disconnected_status() {
            let mut duo = mesh_with_dropped_slot(600, 6);
            let (mut c2, serve_f) = reopened_survivor_with_pending_sub_f_misprediction(&mut duo, 2);

            // No joiner inputs yet: the rollback's pre-activation frames hit
            // the empty reopened queue.
            assert_sub_f_rollback_serves_frozen_disconnected(&mut duo, &mut c2, serve_f, None);

            // The repaired sub-F history byte-agrees with A's (which never
            // rolled back): the floor served exactly the values A simulated.
            let probe = serve_f.as_i32() - 1;
            assert_eq!(
                duo.a_shadow.states.get(&probe),
                duo.b_shadow.states.get(&probe),
                "A and B byte-agree on the repaired state at F - 1"
            );
        }

        /// Sub-F rollback on a reopened slot, hard-error flavor (session-33
        /// review Finding 5): once the joiner's real inputs occupy the
        /// reopened ring (oldest frame = F), a sub-F request hits the
        /// queue's "requested frame is before oldest" guard, fails
        /// `synchronized_inputs`, and every `advance_frame` returns
        /// `SynchronizedInputsFailed` until the rollback window moves — a
        /// hard session error on a healthy mesh. The reactivation floor must
        /// serve the pre-activation frames instead.
        #[test]
        fn npeer_sub_f_rollback_after_joiner_inputs_is_servable() {
            let mut duo = mesh_with_dropped_slot(600, 6);
            let (mut c2, serve_f) = reopened_survivor_with_pending_sub_f_misprediction(&mut duo, 2);

            // The joiner's real input at F lands in B's reopened queue
            // BEFORE the rollback (the ring's oldest frame is now F).
            c2.send_input(serve_f, C_REJOIN_INPUT);
            for _ in 0..3 {
                duo.poll_round(Some(&mut c2));
            }
            assert_eq!(
                duo.b.local_connect_status[2].last_frame, serve_f,
                "B's reopened queue confirmed the joiner's real input at F"
            );
            // Drain the AT-F reconciliation rollback first (the reopen armed a
            // forced re-simulation from F — the Finding-1 fix — and the real
            // input at F just mispredicted against the frozen episode); the
            // helper below must catch the SUB-F rollback specifically.
            for _ in 0..2 {
                duo.b
                    .add_local_input(PlayerHandle::new(1), 80)
                    .expect("B local input");
                let requests = duo
                    .b
                    .advance_frame()
                    .expect("B advance (at-F reconciliation)");
                apply_requests(&requests, &mut duo.b_shadow);
            }

            assert_sub_f_rollback_serves_frozen_disconnected(
                &mut duo,
                &mut c2,
                serve_f,
                Some(C_REJOIN_INPUT),
            );
        }

        /// Spectator flush across a reopen (session-33 review Finding 4): a
        /// reopened survivor whose spectator stream lags behind the
        /// activation frame must still be able to drain the owed
        /// pre-activation frames — `confirmed_inputs` on the blanked ring
        /// would return `NoConfirmedInput`, the error propagates out of
        /// `advance_frame`, `next_spectator_frame` never advances, and every
        /// subsequent `advance_frame` fails forever. The reactivation floor
        /// serves the owed frames with the captured frozen value (exactly
        /// what the pre-reopen frozen branch streamed).
        #[test]
        fn npeer_reopened_survivor_with_spectator_flushes_pre_activation_frames() {
            let (mut duo, mut spectator) = mesh_with_dropped_slot_with_spectator(600, 6);

            // Stall B's confirmed frame (and with it the spectator stream)
            // below S: block A -> B inputs while A advances with CONSTANT
            // inputs — no misprediction (the late arrivals match B's
            // predictions), purely a confirmed-frame lag.
            duo.bus.block(addr_a(), addr_b(), "Input");
            for _ in 0..3_u8 {
                for _ in 0..3 {
                    duo.poll_round(None);
                    spectator.pump();
                }
                duo.advance_both(53, 80);
            }

            let mut c2 = ManualJoiner::new(&duo.bus.clone(), addr_c());
            c2.connect(addr_a(), 0, &duo.clock.clone());
            sync_joiner_with(&mut duo, &mut c2, addr_a());
            c2.proto_mut(addr_a()).send_join_request(2);
            c2.pump();
            duo.poll_round(Some(&mut c2));
            let (serve_s, serve_f) = {
                let serve = duo.a.hot_join.npeer.as_ref().expect("N-peer serve opens");
                (serve.snapshot_frame, serve.activation_frame)
            };
            for _ in 0..4 {
                duo.poll_round(Some(&mut c2));
                spectator.pump();
            }
            c2.connect(addr_b(), 1, &duo.clock.clone());
            for _ in 0..60 {
                duo.poll_round(Some(&mut c2));
                spectator.pump();
                duo.advance_both(53, 80);
                if duo
                    .b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened)
                    && c2.is_running(addr_b())
                {
                    break;
                }
            }
            assert!(
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened),
                "B reopened"
            );
            // The Finding-4 precondition: the spectator stream is owed frames
            // strictly below F - 1 at the reopen.
            assert!(
                duo.b.next_spectator_frame < Frame::new(serve_s.as_i32()),
                "B's spectator stream lags below S at the reopen (next {}, S {})",
                duo.b.next_spectator_frame,
                serve_s
            );

            // Unblock: A's retransmits recover B's confirmed frame past the
            // owed window (no rollback — the inputs match B's predictions);
            // the joiner contributes real inputs from F. Every B advance must
            // stay servable while the flush drains the owed pre-activation
            // frames.
            duo.bus.unblock(addr_a(), addr_b(), "Input");
            for k in 0..14_i32 {
                c2.send_input(Frame::new(serve_f.as_i32() + k), C_REJOIN_INPUT);
                for _ in 0..3 {
                    duo.poll_round(Some(&mut c2));
                    spectator.pump();
                }
                duo.b
                    .add_local_input(PlayerHandle::new(1), 80)
                    .expect("B local input");
                let requests = duo
                    .b
                    .advance_frame()
                    .expect("B's advance must stay servable while the spectator flush drains pre-activation frames");
                apply_requests(&requests, &mut duo.b_shadow);
                duo.a
                    .add_local_input(PlayerHandle::new(0), 53)
                    .expect("A local input");
                let requests = duo.a.advance_frame().expect("A advance");
                apply_requests(&requests, &mut duo.a_shadow);
                if duo.b.next_spectator_frame >= serve_f {
                    break;
                }
            }
            // `next >= F` means every owed pre-activation frame (<= F - 1 =
            // S) was drained — exactly the window the blanked ring could not
            // serve.
            assert!(
                duo.b.next_spectator_frame >= serve_f,
                "B's spectator stream drained the pre-activation window (next {}, F {})",
                duo.b.next_spectator_frame,
                serve_f
            );
        }

        /// One join at a time (R6): while an N-peer serve is open, a
        /// JoinRequest for a DIFFERENT reserved handle is ignored (no second
        /// serve, neither N-peer nor 2-peer).
        #[test]
        fn npeer_second_join_request_is_ignored_while_serve_open() {
            let bus = MeshBus::new();
            let clock = MeshClock::new();

            // A: local 0, survivor B (remote 1), and TWO reserved slots on two
            // distinct machines (C: 2, D: 3).
            let mut a = SessionBuilder::<TestConfig>::new()
                .with_num_players(4)
                .expect("num players")
                .with_protocol_config(clock.protocol_config())
                .with_hot_join(true)
                .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
                .add_player(PlayerType::Local, PlayerHandle::new(0))
                .expect("A local")
                .add_player(PlayerType::Remote(addr_b()), PlayerHandle::new(1))
                .expect("A remote B")
                .add_reserved_player(addr_c(), PlayerHandle::new(2))
                .expect("A reserved C")
                .add_reserved_player(addr_d(), PlayerHandle::new(3))
                .expect("A reserved D")
                .start_p2p_session_skip_hot_join_build_guards_for_test(bus.socket(addr_a()))
                .expect("A builds");

            let mut b = SessionBuilder::<TestConfig>::new()
                .with_num_players(4)
                .expect("num players")
                .with_protocol_config(clock.protocol_config())
                .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
                .add_player(PlayerType::Remote(addr_a()), PlayerHandle::new(0))
                .expect("B remote A")
                .add_player(PlayerType::Local, PlayerHandle::new(1))
                .expect("B local")
                .add_player(PlayerType::Remote(addr_c()), PlayerHandle::new(2))
                .expect("B remote C")
                .add_player(PlayerType::Remote(addr_d()), PlayerHandle::new(3))
                .expect("B remote D")
                .start_p2p_session(bus.socket(addr_b()))
                .expect("B builds");

            // Drive A <-> B sync (B itself stays Synchronizing forever — its C
            // and D endpoints have no peers — but A's B-endpoint reaches
            // Running, which is all the survivor set needs).
            // Condition-driven with a generous cap (session-33 round-6).
            for _ in 0..300 {
                a.poll_remote_clients();
                let _ = a.events().count();
                b.poll_remote_clients();
                let _ = b.events().count();
                clock.advance(POLL_INTERVAL);
                if a.current_state() == SessionState::Running {
                    break;
                }
            }
            assert_eq!(
                a.current_state(),
                SessionState::Running,
                "A reaches Running"
            );

            // A advances a few frames so a serve is openable.
            let mut a_shadow = Shadow::default();
            for i in 0..3_u8 {
                a.add_local_input(PlayerHandle::new(0), i).expect("A input");
                let requests = a.advance_frame().expect("A advance");
                apply_requests(&requests, &mut a_shadow);
                a.poll_remote_clients();
                let _ = a.events().count();
                clock.advance(POLL_INTERVAL);
            }

            // Open the N-peer serve for slot 2 (staged via the test seam; the
            // wire path is exercised by the full-mesh tests above).
            a.player_reg
                .remotes
                .get_mut(&addr_c())
                .expect("A's C endpoint")
                .set_pending_join_request_for_test(2);
            a.poll_remote_clients();
            assert!(
                a.hot_join
                    .npeer
                    .as_ref()
                    .is_some_and(|serve| serve.handle == PlayerHandle::new(2)),
                "the first request opens an N-peer serve for slot 2"
            );

            // A second request for a DIFFERENT reserved handle is ignored
            // while the serve is open.
            a.player_reg
                .remotes
                .get_mut(&addr_d())
                .expect("A's D endpoint")
                .set_pending_join_request_for_test(3);
            a.poll_remote_clients();
            assert!(
                a.hot_join
                    .npeer
                    .as_ref()
                    .is_some_and(|serve| serve.handle == PlayerHandle::new(2)),
                "the open serve is undisturbed"
            );
            assert!(
                a.hot_join.joining.is_empty(),
                "no 2-peer serve opened for the second request"
            );

            // A duplicate request for the SAME handle is also a no-op: the
            // serve neither re-opens nor RESETS. `polls_since_serve` must
            // strictly increase across the duplicate (a hypothetically reset
            // serve would re-enter this poll at exactly 1, so a bare `> 0`
            // could not detect the reset half).
            let polls_before_duplicate = a
                .hot_join
                .npeer
                .as_ref()
                .map(|serve| serve.polls_since_serve)
                .expect("serve open before the duplicate");
            a.player_reg
                .remotes
                .get_mut(&addr_c())
                .expect("A's C endpoint")
                .set_pending_join_request_for_test(2);
            a.poll_remote_clients();
            assert!(
                a.hot_join.npeer.as_ref().is_some_and(|serve| {
                    serve.handle == PlayerHandle::new(2)
                        && serve.polls_since_serve > polls_before_duplicate
                }),
                "a duplicate request neither re-opens nor resets the serve"
            );
        }

        /// Stale same-coordinator directive discrimination (session-33 review
        /// Finding 2, ordering (a)): the coordinator re-sends a directive
        /// every poll, so duplicates of an ABORTED attempt's `(h, F1)` remain
        /// in flight when the retry `(h, F2 > F1)` opens. A delayed `(h, F1)`
        /// arriving while `(h, F2)` is pending pre-reopen must NOT supersede
        /// it — R3 makes frames strictly monotone across a coordinator's
        /// attempts, so only a STRICTLY NEWER frame is a genuine retry. A
        /// stale supersede would reopen at F1 and ack into a void (the
        /// attempt-1 abort responder was destroyed when attempt 2 opened),
        /// wedging the survivor and stalling the mesh permanently.
        #[test]
        fn npeer_stale_same_coordinator_directive_cannot_supersede_newer_pending() {
            let mut duo = mesh_with_dropped_slot(600, 6);

            let stage = |duo: &mut Duo, frame: Frame| {
                duo.b
                    .player_reg
                    .remotes
                    .get_mut(&addr_a())
                    .expect("B's coordinator endpoint")
                    .set_received_reactivate_slot_for_test(ReactivateSlot { handle: 2, frame });
                duo.b.poll_remote_clients();
            };

            let f_newer = Frame::new(duo.b.confirmed_frame().as_i32() + 10);
            let f_stale = Frame::new(duo.b.confirmed_frame().as_i32() + 5);

            // The (newer) retry attempt is pending pre-reopen.
            stage(&mut duo, f_newer);
            assert!(
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.frame == f_newer && !pending.reopened),
                "the newer directive is pending pre-reopen"
            );

            // A delayed duplicate of the OLDER attempt arrives from the same
            // coordinator: it must be ignored as stale, not supersede.
            stage(&mut duo, f_stale);
            assert!(
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.frame == f_newer),
                "a stale same-coordinator directive (frame {} <= pending {}) must not supersede the pending attempt",
                f_stale,
                f_newer
            );

            // Positive control: a strictly newer same-coordinator directive IS
            // a genuine retry and supersedes the pre-reopen pending.
            let f_retry = Frame::new(f_newer.as_i32() + 3);
            stage(&mut duo, f_retry);
            assert!(
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.frame == f_retry),
                "a strictly newer same-coordinator directive supersedes the pre-reopen pending"
            );

            // The frame order binds the same coordinator even once its
            // endpoint has died (R3 monotonicity is a property of the sender,
            // not of its liveness): only a DIFFERENT sender may take over a
            // dead coordinator's pre-reopen pending. A stale same-coordinator
            // duplicate delivered after the death must still be ignored.
            duo.b
                .player_reg
                .remotes
                .get_mut(&addr_a())
                .expect("B's coordinator endpoint")
                .force_synchronizing_for_tests();
            stage(&mut duo, f_stale);
            assert!(
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.frame == f_retry),
                "a stale same-coordinator duplicate must not supersede via the dead-coordinator takeover arm"
            );
        }

        /// Closed-attempt high-water guard (session-33 review Finding 2,
        /// ordering (b)): after a survivor consumes an attempt's lifecycle
        /// close (here `JoinAborted{h, F1}`), a straggling duplicate directive
        /// `(h, F1)` landing in the post-close window must be rejected — its
        /// lifecycle messages no longer exist anywhere once the next serve
        /// supersedes the coordinator's responder memo, so accepting it would
        /// wedge the survivor in a never-closeable reopened attempt. R3
        /// monotonicity makes every genuine new attempt strictly newer than
        /// any closed one.
        #[test]
        fn npeer_directive_at_or_below_closed_attempt_frame_is_rejected() {
            let mut duo = mesh_with_dropped_slot(600, 6);

            let stage = |duo: &mut Duo, frame: Frame| {
                duo.b
                    .player_reg
                    .remotes
                    .get_mut(&addr_a())
                    .expect("B's coordinator endpoint")
                    .set_received_reactivate_slot_for_test(ReactivateSlot { handle: 2, frame });
                duo.b.poll_remote_clients();
            };

            // Attempt 1 goes pending, then closes via a matching JoinAborted.
            let f1 = Frame::new(duo.b.confirmed_frame().as_i32() + 5);
            stage(&mut duo, f1);
            assert!(
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.frame == f1),
                "attempt 1 is pending"
            );
            duo.b
                .player_reg
                .remotes
                .get_mut(&addr_a())
                .expect("B's coordinator endpoint")
                .set_received_join_aborted_for_test(JoinAborted {
                    handle: 2,
                    frame: f1,
                });
            duo.b.poll_remote_clients();
            assert!(
                duo.b.hot_join.pending_reactivation.is_none(),
                "the matching JoinAborted closes attempt 1"
            );

            // A straggling duplicate of the CLOSED attempt's directive arrives
            // (the coordinator re-sent it every poll while the attempt was
            // open): it must be rejected, not freshly accepted.
            stage(&mut duo, f1);
            assert!(
                duo.b.hot_join.pending_reactivation.is_none(),
                "a directive at the closed attempt's frame ({}) is a stale straggler and must be rejected",
                f1
            );

            // Positive control: the genuine retry (strictly newer frame) is
            // accepted.
            let f2 = Frame::new(f1.as_i32() + 1);
            stage(&mut duo, f2);
            assert!(
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.frame == f2),
                "the genuine retry (frame {} > closed {}) is accepted",
                f2,
                f1
            );
        }

        /// Implied close of a wedged REOPENED attempt (session-33 review
        /// Finding 2, ordering (c)): B reopened for attempt 1 but the
        /// `JoinAborted{h, F1}` is lost (here: blocked), and attempt 2 then
        /// destroys the abort responder. Attempt 2's directive `(h, F2 > F1)`
        /// from the same coordinator PROVES attempt 1 concluded
        /// (one-join-at-a-time + R3), and the survivor's confirmed frame
        /// discriminates the outcome (the pause caps confirmed <= S1 < F1
        /// while attempt 1 is open, and a post-abort slot's gossip bound
        /// re-sticks at the old freeze frame < F1; only a commit lets
        /// confirmed reach F1). Here confirmed < F1, so B applies the implied
        /// ABORT restore and then accepts attempt 2 — the retry must commit
        /// end to end instead of wedging the mesh forever.
        #[test]
        fn npeer_reopened_survivor_heals_from_lost_abort_via_newer_directive() {
            // Serve budget 40: attempt 1 must Phase-4 abort (the joiner never
            // acks its snapshot), and attempt 2 must commit well within it.
            let mut duo = mesh_with_dropped_slot(40, 6);

            // The lifecycle loss under test: A's JoinAborted never reaches B.
            duo.bus.block(addr_a(), addr_b(), "JoinAborted");

            // --- Attempt 1: B reopens + acks; the joiner never acks the
            // snapshot, so A Phase-4 aborts; B never hears it.
            let mut c2 = ManualJoiner::new(&duo.bus.clone(), addr_c());
            c2.connect(addr_a(), 0, &duo.clock.clone());
            sync_joiner_with(&mut duo, &mut c2, addr_a());
            c2.proto_mut(addr_a()).send_join_request(2);
            c2.pump();
            duo.poll_round(Some(&mut c2));
            let serve_f1 = duo
                .a
                .hot_join
                .npeer
                .as_ref()
                .expect("attempt 1 opens")
                .activation_frame;
            for _ in 0..4 {
                duo.poll_round(Some(&mut c2));
            }
            c2.connect(addr_b(), 1, &duo.clock.clone());
            for _ in 0..60 {
                duo.poll_round(Some(&mut c2));
                if duo
                    .b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened)
                    && c2.is_running(addr_b())
                {
                    break;
                }
            }
            assert!(
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened && pending.frame == serve_f1),
                "B reopened for attempt 1"
            );
            // Joiner-endpoint bootstrap pin (review minor m2): without the
            // all-slot cache bootstrap at the reopen, the rearmed joiner
            // endpoint's default `{connected, NULL}` caches enter B's folds
            // here and collapse B's confirmed frame to NULL — gossip-silencing
            // B and starving the coordinator's capture gate mesh-wide.
            assert!(
                !duo.b.confirmed_frame().is_null(),
                "B's confirmed frame stays real after the reopen (the joiner-endpoint caches were bootstrapped)"
            );
            for _ in 0..60 {
                duo.poll_round(Some(&mut c2));
                if duo.a.hot_join.npeer.is_none() {
                    break;
                }
            }
            assert!(duo.a.hot_join.npeer.is_none(), "attempt 1 Phase-4 aborts");
            assert!(
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened && pending.frame == serve_f1),
                "B still pends attempt 1 (the JoinAborted was lost)"
            );

            // --- A alone advances past the R3 guard (B's confirmed frame is
            // pinned at the old freeze frame by its wedged reopened slot —
            // the finding's f0-pin — so B must keep its remaining prediction
            // headroom for the healed retry below); the joiner retries;
            // attempt 2 opens at a strictly later frame and destroys the
            // attempt-1 abort responder.
            for i in 0..3_u8 {
                for _ in 0..3 {
                    duo.poll_round(Some(&mut c2));
                }
                duo.a
                    .add_local_input(PlayerHandle::new(0), 150 + i)
                    .expect("A local input");
                let requests = duo.a.advance_frame().expect("A advance");
                apply_requests(&requests, &mut duo.a_shadow);
            }
            c2.proto_mut(addr_a()).send_join_request(2);
            c2.pump();
            for _ in 0..4 {
                duo.poll_round(Some(&mut c2));
                if duo.a.hot_join.npeer.is_some() {
                    break;
                }
            }
            let serve_f2 = duo
                .a
                .hot_join
                .npeer
                .as_ref()
                .expect("attempt 2 opens")
                .activation_frame;
            assert!(serve_f2 > serve_f1, "attempt 2 is strictly newer (R3)");

            // --- Attempt 2's directive must close the wedged attempt-1
            // pending (implied abort: B's confirmed is pinned below F1) and
            // be accepted; B reopens at F2 and acks; the joiner acks its new
            // snapshot; attempt 2 COMMITS.
            let mut snapshot = None;
            for _ in 0..30 {
                duo.poll_round(Some(&mut c2));
                duo.advance_both(170, 180);
                if let Some(snap) = c2.proto_mut(addr_a()).take_received_snapshot() {
                    snapshot = Some(snap);
                    break;
                }
            }
            let snapshot = snapshot.expect("the joiner receives attempt 2's snapshot");
            c2.proto_mut(addr_a())
                .send_state_snapshot_ack(snapshot.frame);
            c2.pump();
            for _ in 0..20 {
                duo.poll_round(Some(&mut c2));
                if duo.a.hot_join.npeer.is_none() {
                    break;
                }
            }
            assert!(
                duo.a.hot_join.npeer.is_none(),
                "attempt 2 concluded on the coordinator"
            );
            assert!(
                duo.a
                    .hot_join
                    .npeer_post
                    .as_ref()
                    .is_some_and(|post| post.committed && post.frame == serve_f2),
                "attempt 2 COMMITTED (B's ack arrived — the wedge is healed)"
            );

            // --- The joiner feeds real inputs from F2; the commit lifecycle
            // clears B's pending; both survivors confirm C's real input at F2
            // and byte-agree past it.
            for k in 0..14_i32 {
                c2.send_input(Frame::new(serve_f2.as_i32() + k), C_REJOIN_INPUT);
                for _ in 0..3 {
                    duo.poll_round(Some(&mut c2));
                }
                duo.advance_both(190, 191);
                if duo.a.confirmed_frame() >= serve_f2 && duo.b.confirmed_frame() >= serve_f2 {
                    break;
                }
            }
            assert!(
                duo.b.hot_join.pending_reactivation.is_none(),
                "B's pending cleared by attempt 2's JoinCommitted"
            );
            let b_input = duo
                .b
                .sync_layer
                .confirmed_input(PlayerHandle::new(2), serve_f2)
                .expect("B confirmed slot-2 input at F2")
                .input;
            assert_eq!(b_input, C_REJOIN_INPUT, "B committed C's real input at F2");
            let probe = serve_f2.as_i32() + 1;
            assert_eq!(
                duo.a_shadow.states.get(&probe),
                duo.b_shadow.states.get(&probe),
                "A and B byte-agree on the state at F2 + 1 after the healed retry"
            );
        }

        /// Implied COMMIT close (the other arm of the Finding-2 ordering (c)
        /// discrimination): when a strictly-newer same-coordinator directive
        /// arrives at a survivor whose wedged reopened pending has COMMIT
        /// evidence (confirmed >= F1 — impossible for an aborted attempt: the
        /// pause caps confirmed below F while the attempt is open, and a
        /// post-abort slot's bound re-sticks at the old freeze frame), the
        /// survivor must close the pending WITHOUT re-freezing (the slot
        /// committed live mesh-wide; restoring the pre-reopen frozen value
        /// would silently diverge) and fail-closed-reject the directive while
        /// the slot is live.
        #[test]
        fn npeer_newer_directive_with_commit_evidence_closes_stale_reopened_pending() {
            let mut duo = mesh_with_dropped_slot(40, 6);

            // Reach a REOPENED pending on B for attempt 1 (block A -> B
            // lifecycle so the pending stays held past the conclusion).
            duo.bus.block(addr_a(), addr_b(), "JoinAborted");
            duo.bus.block(addr_a(), addr_b(), "JoinCommitted");
            let mut c2 = ManualJoiner::new(&duo.bus.clone(), addr_c());
            c2.connect(addr_a(), 0, &duo.clock.clone());
            sync_joiner_with(&mut duo, &mut c2, addr_a());
            c2.proto_mut(addr_a()).send_join_request(2);
            c2.pump();
            duo.poll_round(Some(&mut c2));
            let serve_f1 = duo
                .a
                .hot_join
                .npeer
                .as_ref()
                .expect("attempt 1 opens")
                .activation_frame;
            for _ in 0..4 {
                duo.poll_round(Some(&mut c2));
            }
            c2.connect(addr_b(), 1, &duo.clock.clone());
            for _ in 0..60 {
                duo.poll_round(Some(&mut c2));
                if duo
                    .b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened)
                    && c2.is_running(addr_b())
                {
                    break;
                }
            }
            assert!(
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened && pending.frame == serve_f1),
                "B reopened for attempt 1"
            );

            // Manufacture the commit-evidence state: every folded view of
            // every slot (B's own receipts and its cached views of A's and
            // C's claims) confirms through F1 + 2 — exactly the state a
            // committed era reaches once the joiner's inputs flow. (The wire
            // path cannot produce this for an aborted attempt: the pause caps
            // confirmed <= S1 < F1 until conclusion, and after an abort the
            // re-stuck disconnected gossip pins the slot's bound at the old
            // freeze frame.)
            let evidence = ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(serve_f1.as_i32() + 2),
            };
            for handle in 0..3 {
                duo.b.local_connect_status[handle].disconnected = false;
                duo.b.local_connect_status[handle].last_frame = evidence.last_frame;
                for endpoint in duo.b.player_reg.remotes.values_mut() {
                    endpoint.set_peer_connect_status_for_tests(PlayerHandle::new(handle), evidence);
                }
            }
            assert!(
                duo.b.confirmed_frame() >= serve_f1,
                "commit evidence in place (confirmed {} >= F1 {})",
                duo.b.confirmed_frame(),
                serve_f1
            );

            // A strictly-newer directive from the same coordinator arrives
            // (staged: the real attempt-2 serve shape is pinned end-to-end by
            // the lost-abort heal test). B must close the stale pending as
            // COMMITTED — slot stays live, no re-freeze — and fail-closed
            // reject the directive itself (the slot is not frozen).
            duo.b
                .player_reg
                .remotes
                .get_mut(&addr_a())
                .expect("B's coordinator endpoint")
                .set_received_reactivate_slot_for_test(ReactivateSlot {
                    handle: 2,
                    frame: Frame::new(serve_f1.as_i32() + 4),
                });
            duo.b.poll_remote_clients();
            assert!(
                duo.b.hot_join.pending_reactivation.is_none(),
                "the newer directive closes the stale reopened pending (implied commit)"
            );
            assert!(
                !duo.b.sync_layer.player_is_frozen(PlayerHandle::new(2)),
                "the committed slot stays LIVE — an implied-commit close must not re-freeze it"
            );
            assert!(
                !duo.b.local_connect_status[2].disconnected,
                "the committed slot's status stays connected"
            );
        }

        /// Survivor fail-closed validation: directives for non-frozen slots,
        /// non-remote slots, out-of-range handles, or with insane activation
        /// frames are ignored with no state change — and a serving coordinator
        /// never takes directives at all.
        #[test]
        fn npeer_survivor_rejects_invalid_reactivate_directives() {
            let mut duo = mesh_with_dropped_slot(600, 6);

            let stage = |duo: &mut Duo, handle: usize, frame: Frame| {
                duo.b
                    .player_reg
                    .remotes
                    .get_mut(&addr_a())
                    .expect("B's coordinator endpoint")
                    .set_received_reactivate_slot_for_test(ReactivateSlot { handle, frame });
                duo.b.poll_remote_clients();
            };

            let sane_frame = Frame::new(duo.b.confirmed_frame().as_i32() + 10);

            // (a) A directive for a CONNECTED slot (A's slot 0) is ignored.
            // Staged via the OTHER endpoint (addr_c) so the sender is not the
            // slot owner — this must reach (and be rejected by) the
            // frozen/disconnected validation specifically, not the
            // owner-self-direct check.
            duo.b
                .player_reg
                .remotes
                .get_mut(&addr_c())
                .expect("B's C endpoint")
                .set_received_reactivate_slot_for_test(ReactivateSlot {
                    handle: 0,
                    frame: sane_frame,
                });
            duo.b.poll_remote_clients();
            assert!(
                duo.b.hot_join.pending_reactivation.is_none(),
                "a directive for a connected slot is ignored"
            );
            assert!(
                !duo.b.local_connect_status[0].disconnected,
                "slot 0 is untouched"
            );

            // (b) A directive for B's LOCAL slot is ignored.
            stage(&mut duo, 1, sane_frame);
            assert!(
                duo.b.hot_join.pending_reactivation.is_none(),
                "a directive for a local slot is ignored"
            );

            // (c) An out-of-range handle is ignored.
            stage(&mut duo, 7, sane_frame);
            assert!(duo.b.hot_join.pending_reactivation.is_none());

            // (d) An activation frame at/below the slot's frozen bound is
            // ignored (would rewrite committed history).
            let frozen_bound = duo.b.local_connect_status[2].last_frame;
            stage(&mut duo, 2, frozen_bound);
            assert!(
                duo.b.hot_join.pending_reactivation.is_none(),
                "F <= the frozen bound is rejected"
            );
            assert!(duo.b.sync_layer.player_is_frozen(PlayerHandle::new(2)));

            // (e) A NULL activation frame is ignored.
            stage(&mut duo, 2, Frame::NULL);
            assert!(duo.b.hot_join.pending_reactivation.is_none());

            // (f) An activation frame at/below B's confirmed frame is ignored.
            let confirmed = duo.b.confirmed_frame();
            assert!(confirmed.as_i32() > 0, "B has confirmed frames");
            stage(&mut duo, 2, confirmed);
            assert!(
                duo.b.hot_join.pending_reactivation.is_none(),
                "F <= the survivor's confirmed frame is rejected"
            );

            // (f2) The F-sanity floor is monotone (review minor m1):
            // `confirmed_frame()` can transiently DIP during endpoint-cache
            // churn — manufacture a dip (one cached view collapses to NULL)
            // and stage F at the sync layer's discard high-water, which the
            // dipped instantaneous read no longer covers. It must still be
            // rejected: history at/below the high-water may already be
            // discarded.
            let high_water = duo.b.sync_layer.last_confirmed_frame();
            assert!(high_water.as_i32() > 0, "B has a discard high-water");
            let real_view = duo
                .b
                .player_reg
                .remotes
                .get(&addr_a())
                .expect("B's coordinator endpoint")
                .peer_connect_status(PlayerHandle::new(0));
            duo.b
                .player_reg
                .remotes
                .get_mut(&addr_a())
                .expect("B's coordinator endpoint")
                .set_peer_connect_status_for_tests(
                    PlayerHandle::new(0),
                    ConnectionStatus::default(),
                );
            assert!(
                duo.b.confirmed_frame() < high_water,
                "the manufactured dip is in place (confirmed {}, high-water {})",
                duo.b.confirmed_frame(),
                high_water
            );
            stage(&mut duo, 2, high_water);
            assert!(
                duo.b.hot_join.pending_reactivation.is_none(),
                "F <= the discard high-water is rejected even during a confirmed-frame dip"
            );
            duo.b
                .player_reg
                .remotes
                .get_mut(&addr_a())
                .expect("B's coordinator endpoint")
                .set_peer_connect_status_for_tests(PlayerHandle::new(0), real_view);

            // (g) Positive control (non-vacuity): a sane directive through the
            // same seam DOES create the pending attempt.
            stage(&mut duo, 2, sane_frame);
            assert!(
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.handle == PlayerHandle::new(2)
                        && pending.frame == sane_frame),
                "a valid directive is accepted (the rejections above are non-vacuous)"
            );

            // (h) A serving coordinator never takes directives: stage one on
            // A for its own reserved slot — it must be rejected even though
            // the slot IS frozen + disconnected there.
            duo.a
                .player_reg
                .remotes
                .get_mut(&addr_b())
                .expect("A's B endpoint")
                .set_received_reactivate_slot_for_test(ReactivateSlot {
                    handle: 2,
                    frame: sane_frame,
                });
            duo.a.poll_remote_clients();
            assert!(
                duo.a.hot_join.pending_reactivation.is_none(),
                "a coordinator (accept_hot_join) rejects reopen directives"
            );
            assert!(
                duo.a.sync_layer.player_is_frozen(PlayerHandle::new(2)),
                "A's reserved slot is untouched"
            );
        }

        // ------------------------------------------------------------------
        // Session-33 round-5 review Finding 1: pre-attempt freeze-frame
        // convergence (asymmetric, value-varying drop staging + tests)
        // ------------------------------------------------------------------

        /// C's frame-dependent input for the asymmetric staging: strictly
        /// varying so any freeze-frame disagreement between survivors is
        /// VALUE-visible. The symmetric staging's constant input
        /// (`C_FROZEN_INPUT`) deliberately masks exactly this class — the
        /// round-5 review's structural-blindness finding.
        fn c_varying_input(frame: i32) -> u8 {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            // test: deterministic small frame numbers; wrapping is fine.
            130_u8.wrapping_add(frame as u8)
        }

        /// Builds the 3-peer mesh like [`mesh_with_dropped_slot`], but stages
        /// the drop ASYMMETRICALLY with VALUE-VARYING C inputs plus a one-way
        /// `Input` burst — the shape the round-5 review proved the
        /// symmetric/constant staging is structurally blind to:
        ///
        /// - the LOW side removes C first, freezing slot 2 at its own receipt
        ///   `f0_low`;
        /// - C keeps advancing inside its prediction window, feeding the HIGH
        ///   side three more varying inputs;
        /// - the HIGH side then removes C, freezing at `f0_high = f0_low + 3`;
        /// - `low -> high` `Input` traffic (the connect-status claim carrier)
        ///   is blocked from BEFORE the first removal, so the low freezer's
        ///   `{disconnected, f0_low}` claim — the trigger of the high
        ///   freezer's convergence re-adjust (status mine-down + frozen-value
        ///   re-roll + gap re-simulation) — stays withheld until the caller
        ///   unblocks it.
        ///
        /// `survivor_freezes_high = true` puts the un-converged HIGH freeze on
        /// the survivor B (the coordinator A holds the agreed minimum);
        /// `false` mirrors it onto the coordinator A. Both A and B have
        /// advanced past `f0_high`, so the divergent gap frames exist in both
        /// shadows (asserted, with a non-vacuity divergence assert at the
        /// gap's top frame).
        fn mesh_with_asymmetric_dropped_slot(
            serve_timeout_polls: usize,
            survivor_freezes_high: bool,
        ) -> (Duo, Frame, Frame) {
            let bus = MeshBus::new();
            let clock = MeshClock::new();

            let a = SessionBuilder::<TestConfig>::new()
                .with_num_players(3)
                .expect("num players")
                .with_protocol_config(clock.protocol_config())
                .with_hot_join(true)
                .with_hot_join_serve_timeout_polls(serve_timeout_polls)
                .expect("serve timeout")
                .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
                .add_player(PlayerType::Local, PlayerHandle::new(0))
                .expect("A local")
                .add_player(PlayerType::Remote(addr_b()), PlayerHandle::new(1))
                .expect("A remote B")
                .add_player(PlayerType::Remote(addr_c()), PlayerHandle::new(2))
                .expect("A remote C")
                .start_p2p_session_skip_hot_join_build_guards_for_test(bus.socket(addr_a()))
                .expect("A builds");

            let b = SessionBuilder::<TestConfig>::new()
                .with_num_players(3)
                .expect("num players")
                .with_protocol_config(clock.protocol_config())
                .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
                .add_player(PlayerType::Remote(addr_a()), PlayerHandle::new(0))
                .expect("B remote A")
                .add_player(PlayerType::Local, PlayerHandle::new(1))
                .expect("B local")
                .add_player(PlayerType::Remote(addr_c()), PlayerHandle::new(2))
                .expect("B remote C")
                .start_p2p_session(bus.socket(addr_b()))
                .expect("B builds (a plain survivor needs no bypass)");

            let mut c = SessionBuilder::<TestConfig>::new()
                .with_num_players(3)
                .expect("num players")
                .with_protocol_config(clock.protocol_config())
                .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
                .add_player(PlayerType::Remote(addr_a()), PlayerHandle::new(0))
                .expect("C remote A")
                .add_player(PlayerType::Remote(addr_b()), PlayerHandle::new(1))
                .expect("C remote B")
                .add_player(PlayerType::Local, PlayerHandle::new(2))
                .expect("C local")
                .start_p2p_session(bus.socket(addr_c()))
                .expect("C builds");

            let mut duo = Duo {
                bus,
                clock,
                a,
                b,
                a_shadow: Shadow::default(),
                b_shadow: Shadow::default(),
                a_events: Vec::new(),
                b_events: Vec::new(),
            };
            let mut c_shadow = Shadow::default();

            // Condition-driven with a generous cap (session-33 round-6).
            for _ in 0..300 {
                duo.poll_round(None);
                c.poll_remote_clients();
                let _ = c.events().count();
                if duo.a.current_state() == SessionState::Running
                    && duo.b.current_state() == SessionState::Running
                    && c.current_state() == SessionState::Running
                {
                    break;
                }
            }
            assert_eq!(duo.a.current_state(), SessionState::Running, "A syncs");
            assert_eq!(duo.b.current_state(), SessionState::Running, "B syncs");
            assert_eq!(c.current_state(), SessionState::Running, "C syncs");

            // Lockstep pre-drop rounds with VARYING C input.
            for i in 0..6_u32 {
                for _ in 0..3 {
                    duo.poll_round(None);
                    c.poll_remote_clients();
                    let _ = c.events().count();
                }
                #[allow(clippy::cast_possible_truncation)]
                // test: bounded loop counter.
                duo.advance_both(10 + (i as u8), 20 + (i as u8));
                let c_frame = c.current_frame().as_i32();
                c.add_local_input(PlayerHandle::new(2), c_varying_input(c_frame))
                    .expect("C local input");
                let requests = c.advance_frame().expect("C advance");
                apply_requests(&requests, &mut c_shadow);
            }
            for _ in 0..3 {
                duo.poll_round(None);
                c.poll_remote_clients();
                let _ = c.events().count();
            }

            // One-way burst: the LOW freezer's claim/input traffic toward the
            // HIGH freezer is withheld from before the first removal.
            let (low_addr, high_addr) = if survivor_freezes_high {
                (addr_a(), addr_b())
            } else {
                (addr_b(), addr_a())
            };
            duo.bus.block(low_addr, high_addr, "Input");

            // The LOW side removes C at its own receipt.
            if survivor_freezes_high {
                duo.a
                    .remove_player(PlayerHandle::new(2))
                    .expect("A removes C");
            } else {
                duo.b
                    .remove_player(PlayerHandle::new(2))
                    .expect("B removes C");
            }
            let f0_low = if survivor_freezes_high {
                duo.a.local_connect_status[2].last_frame
            } else {
                duo.b.local_connect_status[2].last_frame
            };

            // C keeps advancing inside its prediction window: three more
            // VARYING inputs reach only the HIGH freezer.
            for _ in 0..3 {
                let c_frame = c.current_frame().as_i32();
                c.add_local_input(PlayerHandle::new(2), c_varying_input(c_frame))
                    .expect("C local input (post-low-removal)");
                let requests = c
                    .advance_frame()
                    .expect("C advances inside its prediction window");
                apply_requests(&requests, &mut c_shadow);
                for _ in 0..2 {
                    c.poll_remote_clients();
                    let _ = c.events().count();
                    duo.poll_round(None);
                }
            }

            // The HIGH side removes C at its (now higher) receipt.
            if survivor_freezes_high {
                duo.b
                    .remove_player(PlayerHandle::new(2))
                    .expect("B removes C");
            } else {
                duo.a
                    .remove_player(PlayerHandle::new(2))
                    .expect("A removes C");
            }
            let f0_high = if survivor_freezes_high {
                duo.b.local_connect_status[2].last_frame
            } else {
                duo.a.local_connect_status[2].last_frame
            };
            drop(c);
            duo.a_events.extend(duo.a.events());
            duo.b_events.extend(duo.b.events());

            assert!(
                f0_low < f0_high,
                "staging precondition: asymmetric freeze (low {f0_low}, high {f0_high})"
            );
            assert!(
                duo.a
                    .hot_join
                    .reserved_slots
                    .contains(&PlayerHandle::new(2)),
                "A re-reserves the dropped slot (coordinator rearm)"
            );

            // Both keep advancing past the divergent gap. Each stays inside
            // its prediction window: the burst pins the HIGH side's confirmed
            // fold near f0_low (the withheld claim is still CONNECTED there).
            for i in 0..4_u8 {
                for _ in 0..3 {
                    duo.poll_round(None);
                }
                duo.advance_both(50 + i, 60 + i);
            }

            // Staging non-vacuity: both shadows simulated the whole gap, and
            // the value-varying asymmetric freeze produces REAL divergence
            // (frozen-value vs real-input simulation) pending the heal under
            // test.
            for g in (f0_low.as_i32() + 1)..=f0_high.as_i32() {
                assert!(
                    duo.a_shadow.states.contains_key(&g),
                    "A simulated gap frame {g}"
                );
                assert!(
                    duo.b_shadow.states.contains_key(&g),
                    "B simulated gap frame {g}"
                );
            }
            assert_ne!(
                duo.a_shadow.states.get(&f0_high.as_i32()),
                duo.b_shadow.states.get(&f0_high.as_i32()),
                "staging precondition: the asymmetric freeze diverges the shadows at the gap's top frame"
            );

            (duo, f0_low, f0_high)
        }

        /// Session-33 round-5 review Finding 1 (Critical), survivor side: a
        /// held PRE-reopen pending must not suspend the slot's freeze-frame
        /// convergence. Staged with the REAL machinery: an asymmetric,
        /// value-varying drop (B froze slot 2 HIGH at its own receipt, A at
        /// the agreed minimum `f0_low`) plus a one-way A->B `Input` burst that
        /// withholds A's `{disconnected, f0_low}` claim until after the
        /// directive lands. Pre-fix, the any-pending fold shield deferred B's
        /// re-adjust indefinitely: once A's claim arrived, the slot went
        /// mesh-agreed-excluded and B's confirmed fold crossed the gap
        /// `(f0_low, f0_high]` carrying B's own receipts — inputs no other
        /// peer serves — silent confirmed-state divergence. Post-fix, the
        /// acceptance-time convergence gate fail-closed-rejects the directive
        /// while the freeze is un-converged (the per-poll retransmit
        /// self-heals), the generic re-adjust heals B the moment the claim
        /// arrives, and the SAME serve then completes from the converged
        /// shape — byte-agreement restored mesh-wide.
        #[test]
        fn npeer_asymmetric_freeze_unconverged_directive_defers_and_mesh_reconverges() {
            let (mut duo, f0_low, f0_high) = mesh_with_asymmetric_dropped_slot(600, true);

            // The joiner syncs to A and requests slot 2; the directive fan-out
            // begins (only `Input` is blocked A->B, not `ReactivateSlot`).
            let mut c2 = ManualJoiner::new(&duo.bus.clone(), addr_c());
            c2.connect(addr_a(), 0, &duo.clock.clone());
            sync_joiner_with(&mut duo, &mut c2, addr_a());
            c2.proto_mut(addr_a()).send_join_request(2);
            c2.pump();
            duo.poll_round(Some(&mut c2));
            let serve_f = duo
                .a
                .hot_join
                .npeer
                .as_ref()
                .expect(
                    "N-peer serve opens (A holds the agreed minimum, so the open is not deferred)",
                )
                .activation_frame;

            // Give the directive several polls to land on B. Pre-fix it is
            // accepted UN-converged (the pending then shields the slot);
            // post-fix the convergence gate rejects every retransmit until
            // A's claim arrives. No assert here — the symptom below decides.
            for _ in 0..8 {
                duo.poll_round(Some(&mut c2));
            }

            // The burst ends: A's {disconnected, f0_low} claim reaches B.
            duo.bus.unblock(addr_a(), addr_b(), "Input");
            for i in 0..6_u8 {
                for _ in 0..3 {
                    duo.poll_round(Some(&mut c2));
                }
                duo.advance_both(70 + i, 80 + i);
            }

            // B's confirmed fold crossed the gap (the frozen slot is excluded
            // once mesh-agreed), so any gap divergence is CONFIRMED state.
            assert!(
                duo.b.confirmed_frame() > f0_high,
                "B's confirmed frame crossed the gap (got {}, gap top {})",
                duo.b.confirmed_frame(),
                f0_high
            );

            // THE SYMPTOM PIN (red pre-fix): the gap frames must byte-agree —
            // B re-simulates them with the agreed frozen value v(f0_low) the
            // moment convergence lands, exactly like a pending-free survivor.
            for g in (f0_low.as_i32() + 1)..=f0_high.as_i32() {
                assert_eq!(
                    duo.a_shadow.states.get(&g),
                    duo.b_shadow.states.get(&g),
                    "A and B must byte-agree at gap frame {g} after the freeze convergence (a held pre-reopen pending must not suspend the re-adjust)"
                );
            }
            assert_eq!(
                duo.b.local_connect_status[2],
                ConnectionStatus {
                    disconnected: true,
                    last_frame: f0_low,
                },
                "B's slot-2 freeze converges down to the mesh minimum f0_low = {f0_low}"
            );

            // Liveness: the SAME serve completes once convergence landed. The
            // joiner syncs to B; the retransmitted directive validates from
            // the converged shape; B reopens and acks; the joiner acks the
            // snapshot; A commits. All reopen/commit steps are poll-driven.
            c2.connect(addr_b(), 1, &duo.clock.clone());
            // Condition-driven completion loop (session-33 round-6): generous
            // cap + stall diagnostics, same per-iteration drive as before.
            for _ in 0..600 {
                duo.poll_round(Some(&mut c2));
                if let Some(snap) = c2.proto_mut(addr_a()).take_received_snapshot() {
                    c2.proto_mut(addr_a()).send_state_snapshot_ack(snap.frame);
                    c2.pump();
                }
                if duo.a.hot_join.npeer.is_none() {
                    break;
                }
            }
            duo.a_events.extend(duo.a.events());
            assert!(
                duo.a.hot_join.npeer.is_none(),
                "the serve concludes once acceptance was deferred to the converged shape — \
                 stalled stages: a_snapshot_captured={}, b_pending_reopened={:?}, \
                 c2_running_to_b={}",
                duo.a
                    .hot_join
                    .npeer
                    .as_ref()
                    .is_some_and(|serve| serve.snapshot.is_some()),
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .map(|pending| pending.reopened),
                c2.is_running(addr_b()),
            );
            assert!(
                duo.a_events.iter().any(|e| matches!(
                    e,
                    FortressEvent::PeerJoined { handle, .. } if *handle == PlayerHandle::new(2)
                )),
                "the attempt COMMITS (deferral is a delay, not a denial) — post-serve memo: {:?}",
                duo.a
                    .hot_join
                    .npeer_post
                    .as_ref()
                    .map(|post| (post.committed, post.frame)),
            );
            assert!(
                !duo.b.sync_layer.player_is_frozen(PlayerHandle::new(2)),
                "B's slot is live after the commit"
            );

            // Real inputs flow post-commit; the mesh stays byte-consistent.
            for k in 0..8_i32 {
                c2.send_input(Frame::new(serve_f.as_i32() + k), C_REJOIN_INPUT);
                for _ in 0..3 {
                    duo.poll_round(Some(&mut c2));
                }
                duo.advance_both(110, 111);
            }
            let probe = serve_f.as_i32() + 1;
            assert!(
                duo.a_shadow.states.contains_key(&probe)
                    && duo.b_shadow.states.contains_key(&probe),
                "both shadows saved F + 1"
            );
            assert_eq!(
                duo.a_shadow.states.get(&probe),
                duo.b_shadow.states.get(&probe),
                "A and B byte-agree at F + 1"
            );
        }

        /// Session-33 round-5 review Finding 1, the acceptance-time
        /// convergence gate's own pin: a directive arriving while the slot's
        /// pre-attempt freeze is un-converged must be rejected WITHOUT
        /// creating a pending. The gate (not the pre-reopen convergence arm)
        /// is what closes the early-reopen ordering: an un-converged pending
        /// whose joiner channel comes up fast reopens BEFORE the lagging
        /// claim arrives, the slot flips to the attempt's shield + clamp, and
        /// the deferred re-adjust then survives the whole attempt era (in a
        /// committed world the re-seed erases its trigger permanently). With
        /// the gate, a pending can only ever begin converged, so the reopened
        /// window starts from mesh-uniform `(f0, v0)` at N = 3 by
        /// construction.
        #[test]
        fn npeer_unconverged_freeze_rejects_directive_until_claims_land() {
            let (mut duo, f0_low, f0_high) = mesh_with_asymmetric_dropped_slot(600, true);

            // Joiner -> A; the serve opens (A holds the agreed minimum) and
            // the directive retransmits to B every poll.
            let mut c2 = ManualJoiner::new(&duo.bus.clone(), addr_c());
            c2.connect(addr_a(), 0, &duo.clock.clone());
            sync_joiner_with(&mut duo, &mut c2, addr_a());
            c2.proto_mut(addr_a()).send_join_request(2);
            c2.pump();
            for _ in 0..10 {
                duo.poll_round(Some(&mut c2));
            }
            assert!(
                duo.a.hot_join.npeer.is_some(),
                "precondition: the serve is open and fanning out directives"
            );

            // THE GATE PIN: while A's {disconnected, f0_low} claim is
            // withheld (B still holds A's stale CONNECTED view of the slot),
            // every directive retransmit must be rejected — no pending, slot
            // untouched.
            assert!(
                duo.b.hot_join.pending_reactivation.is_none(),
                "the directive must be rejected while the slot's freeze is un-converged (no pending may exist)"
            );
            assert_eq!(
                duo.b.local_connect_status[2],
                ConnectionStatus {
                    disconnected: true,
                    last_frame: f0_high,
                },
                "the rejected directive leaves the slot untouched"
            );

            // The burst ends: the claim lands, the (pending-free) generic
            // re-adjust converges B, and the next retransmit is accepted —
            // with the pending's capture taken from the CONVERGED freeze.
            duo.bus.unblock(addr_a(), addr_b(), "Input");
            // Condition-driven acceptance loop (session-33 round-6): generous
            // cap + stall diagnostics, same per-iteration drive as before.
            for i in 0..60_u8 {
                for _ in 0..3 {
                    duo.poll_round(Some(&mut c2));
                }
                duo.advance_both(70 + i, 80 + i);
                if duo.b.hot_join.pending_reactivation.is_some() {
                    break;
                }
            }
            assert!(
                duo.b.hot_join.pending_reactivation.is_some(),
                "the retransmitted directive is accepted once convergence lands — \
                 stalled stages: b_slot_status={:?}, a_serve_open={}, b_confirmed={}",
                duo.b.local_connect_status[2],
                duo.a.hot_join.npeer.is_some(),
                duo.b.confirmed_frame(),
            );
            let pending = duo
                .b
                .hot_join
                .pending_reactivation
                .as_ref()
                .expect("the retransmitted directive is accepted once convergence lands");
            assert_eq!(
                pending.pre_freeze_status,
                ConnectionStatus {
                    disconnected: true,
                    last_frame: f0_low,
                },
                "the pending's captured pre-freeze snapshot is the CONVERGED freeze"
            );
            assert_eq!(
                pending.pre_freeze_input,
                Some(c_varying_input(f0_low.as_i32())),
                "the captured frozen value is the converged v(f0_low)"
            );
        }

        /// Session-33 round-5 review Finding 1, coordinator sibling: the
        /// coordinator must not capture (or commit) a snapshot embedding
        /// un-converged freeze history. Mirror staging: A froze slot 2 HIGH,
        /// B holds the agreed minimum, and the B->A `Input` burst withholds
        /// B's `{disconnected, f0_low}` claim. Pre-fix, the claim's arrival
        /// poll both lifts the wait-then-capture gate (the fold sees
        /// mesh-agreement instantly) and leaves the convergence re-adjust
        /// owed until the next `advance_frame` — so a poll-paced mesh
        /// captures the STALE state at `S` and commits it before A ever
        /// heals, while the commit's gossip re-seed erases the claim and the
        /// re-adjust never runs: the joiner bases on (and A keeps) history no
        /// other survivor serves. Post-fix, the serve never opens against a
        /// still-CONNECTED survivor claim (open-time convergence gate), the
        /// owed re-adjust defers the capture (delay, not a protocol change),
        /// and the serve that completes serves the HEALED state.
        #[test]
        fn npeer_coordinator_heals_freeze_convergence_before_capturing_snapshot() {
            let (mut duo, f0_low, f0_high) = mesh_with_asymmetric_dropped_slot(600, false);

            // The joiner syncs to A and starts requesting slot 2. Pre-fix the
            // serve opens immediately (S covers the divergent gap); post-fix
            // the open is deferred while B's claim is still CONNECTED at A.
            let mut c2 = ManualJoiner::new(&duo.bus.clone(), addr_c());
            c2.connect(addr_a(), 0, &duo.clock.clone());
            sync_joiner_with(&mut duo, &mut c2, addr_a());
            c2.proto_mut(addr_a()).send_join_request(2);
            c2.pump();
            for _ in 0..8 {
                duo.poll_round(Some(&mut c2));
            }
            if let Some(serve) = duo.a.hot_join.npeer.as_ref() {
                // Pre-fix path: the serve opened against the withheld claim.
                // The capture must still be starved (B's stale-connected
                // claim pins A's confirmed fold below S).
                assert!(
                    serve.snapshot.is_none(),
                    "precondition: no snapshot is captured while B's claim is withheld"
                );
                assert!(
                    serve.snapshot_frame >= f0_high,
                    "staging: the snapshot frame S = {} covers the divergent gap (f0_high = {})",
                    serve.snapshot_frame,
                    f0_high
                );
            }

            // Pre-fix path continued: B accepts the early directive (B IS the
            // agreed minimum — its own gate passes) and, with the joiner's
            // B-channel up, REOPENS before the burst ends — the slot's gossip
            // then flips `{connected, F - 1}` and B's `{disconnected, f0_low}`
            // claim is never delivered: the coordinator's convergence trigger
            // is erased for the whole attempt era. Post-fix the serve never
            // opened, so no pending exists and this phase is a bounded no-op.
            let mut connected_to_b = false;
            for _ in 0..30 {
                duo.poll_round(Some(&mut c2));
                if !connected_to_b && duo.b.hot_join.pending_reactivation.is_some() {
                    c2.connect(addr_b(), 1, &duo.clock.clone());
                    connected_to_b = true;
                }
                if duo
                    .b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened)
                {
                    break;
                }
                if !connected_to_b {
                    break;
                }
            }

            // The burst ends: whatever B's claims now say (the erased-trigger
            // pre-fix shape, or the still-frozen post-fix shape) reaches A
            // together with B's buffered inputs. Keep re-requesting (the
            // joiner's normal retry), sync the joiner to B once a directive
            // creates the pending (post-fix path; after B's endpoint re-arm,
            // like the production joiner sequencing), take + ack each
            // snapshot as it arrives, and advance only every third iteration
            // — polls alone drive the capture gate, the joiner ack, and the
            // commit barrier, while the convergence re-adjust runs only
            // inside `advance_frame` (the pre-fix hole this test pins).
            duo.bus.unblock(addr_b(), addr_a(), "Input");
            let mut first_snapshot: Option<crate::network::messages::StateSnapshot> = None;
            let mut joined = false;
            // Condition-driven drive loop (session-33 round-6): the
            // deterministic path joins at iteration 21 — the cap is ~28x
            // headroom, not pacing. It exists so a liveness stall fails
            // attributably (stage diagnostics in the assert below) instead of
            // riding a tight budget.
            for i in 0..600_u32 {
                duo.poll_round(Some(&mut c2));
                if !connected_to_b && duo.b.hot_join.pending_reactivation.is_some() {
                    c2.connect(addr_b(), 1, &duo.clock.clone());
                    connected_to_b = true;
                }
                if let Some(snap) = c2.proto_mut(addr_a()).take_received_snapshot() {
                    c2.proto_mut(addr_a()).send_state_snapshot_ack(snap.frame);
                    c2.pump();
                    if first_snapshot.is_none() {
                        first_snapshot = Some(snap);
                    }
                }
                duo.a_events.extend(duo.a.events());
                if duo.a_events.iter().any(|e| {
                    matches!(
                        e,
                        FortressEvent::PeerJoined { handle, .. } if *handle == PlayerHandle::new(2)
                    )
                }) {
                    joined = true;
                    break;
                }
                if i % 6 == 5 {
                    c2.proto_mut(addr_a()).send_join_request(2);
                    c2.pump();
                }
                if i % 3 == 2 {
                    #[allow(clippy::cast_possible_truncation)]
                    // test: bounded loop counter.
                    duo.advance_both(70 + (i % 8) as u8, 80 + (i % 8) as u8);
                }
            }
            assert!(
                joined,
                "the join completes (deferral is a delay, not a denial) — stalled stages: \
                 connected_to_b={}, b_pending_reopened={:?}, a_serve_open={}, \
                 a_snapshot_captured={}, joiner_received_snapshot={}, a_frame={}, b_frame={}",
                connected_to_b,
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .map(|pending| pending.reopened),
                duo.a.hot_join.npeer.is_some(),
                duo.a
                    .hot_join
                    .npeer
                    .as_ref()
                    .is_some_and(|serve| serve.snapshot.is_some()),
                first_snapshot.is_some(),
                duo.a.current_frame(),
                duo.b.current_frame(),
            );
            assert!(
                c2.is_running(addr_b()),
                "the joiner's survivor channel is up (slot-2 inputs can flow to B)"
            );
            let first_snapshot = first_snapshot.expect("the joiner received a snapshot");

            // THE SYMPTOM PINS (red pre-fix):
            // 1. The FIRST snapshot the joiner received (the one it applies —
            //    duplicates are idempotent) must byte-equal the survivors'
            //    agreed state at its frame. Pre-fix it is A's un-healed
            //    real-receipt history for the gap — state no other peer
            //    serves.
            let (served_state, _) =
                crate::network::codec::decode::<u8>(&first_snapshot.state_bytes)
                    .expect("snapshot state decodes");
            assert_eq!(
                Some(&served_state),
                duo.b_shadow.states.get(&first_snapshot.frame.as_i32()),
                "the served snapshot at S = {} must byte-equal the survivor truth (the coordinator heals its freeze convergence BEFORE capturing)",
                first_snapshot.frame
            );
            // 2. The coordinator's own confirmed gap history must converge to
            //    the mesh minimum (pre-fix the commit's gossip re-seed erases
            //    the trigger and A keeps the divergent gap forever).
            for g in (f0_low.as_i32() + 1)..=f0_high.as_i32() {
                assert_eq!(
                    duo.a_shadow.states.get(&g),
                    duo.b_shadow.states.get(&g),
                    "A and B must byte-agree at gap frame {g} (the owed re-adjust must run before the slot's freeze era closes)"
                );
            }

            // Liveness + post-join consistency: the slot is live both sides;
            // real inputs flow; a fresh shared frame byte-agrees.
            assert!(
                !duo.b.sync_layer.player_is_frozen(PlayerHandle::new(2)),
                "B's slot is live after the commit"
            );
            assert!(
                !duo.a.sync_layer.player_is_frozen(PlayerHandle::new(2)),
                "A's slot is live after the commit"
            );
            let activation = Frame::new(duo.b.local_connect_status[2].last_frame.as_i32() + 1);
            for k in 0..8_i32 {
                c2.send_input(Frame::new(activation.as_i32() + k), C_REJOIN_INPUT);
                for _ in 0..3 {
                    duo.poll_round(Some(&mut c2));
                }
                duo.advance_both(90, 91);
            }
            let probe = activation.as_i32() + 1;
            assert!(
                duo.a_shadow.states.contains_key(&probe)
                    && duo.b_shadow.states.contains_key(&probe),
                "both shadows saved F + 1"
            );
            assert_eq!(
                duo.a_shadow.states.get(&probe),
                duo.b_shadow.states.get(&probe),
                "A and B byte-agree at F + 1"
            );
        }

        /// Builds the symmetric converged drop in a TIGHT shape: varying C
        /// inputs, symmetric removal at the same receipt `f0`, NO post-drop
        /// advances (so the survivors' confirmed frames stay at ~`f0` and a
        /// one-frame convergence re-adjust stays inside the rollback window),
        /// and a poll-only loop until B's cached view of A's claim for slot 2
        /// is the converged `{disconnected, f0}` (delivered by the
        /// connect-status nudge).
        fn mesh_with_converged_drop_tight() -> (Duo, Frame) {
            let (mut duo, f0_low, f0_high) = {
                // Reuse the asymmetric builder's mesh construction by staging
                // symmetrically here instead: build the same trio inline.
                let bus = MeshBus::new();
                let clock = MeshClock::new();

                let a = SessionBuilder::<TestConfig>::new()
                    .with_num_players(3)
                    .expect("num players")
                    .with_protocol_config(clock.protocol_config())
                    .with_hot_join(true)
                    .with_hot_join_serve_timeout_polls(600)
                    .expect("serve timeout")
                    .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
                    .add_player(PlayerType::Local, PlayerHandle::new(0))
                    .expect("A local")
                    .add_player(PlayerType::Remote(addr_b()), PlayerHandle::new(1))
                    .expect("A remote B")
                    .add_player(PlayerType::Remote(addr_c()), PlayerHandle::new(2))
                    .expect("A remote C")
                    .start_p2p_session_skip_hot_join_build_guards_for_test(bus.socket(addr_a()))
                    .expect("A builds");

                let b = SessionBuilder::<TestConfig>::new()
                    .with_num_players(3)
                    .expect("num players")
                    .with_protocol_config(clock.protocol_config())
                    .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
                    .add_player(PlayerType::Remote(addr_a()), PlayerHandle::new(0))
                    .expect("B remote A")
                    .add_player(PlayerType::Local, PlayerHandle::new(1))
                    .expect("B local")
                    .add_player(PlayerType::Remote(addr_c()), PlayerHandle::new(2))
                    .expect("B remote C")
                    .start_p2p_session(bus.socket(addr_b()))
                    .expect("B builds");

                let mut c = SessionBuilder::<TestConfig>::new()
                    .with_num_players(3)
                    .expect("num players")
                    .with_protocol_config(clock.protocol_config())
                    .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
                    .add_player(PlayerType::Remote(addr_a()), PlayerHandle::new(0))
                    .expect("C remote A")
                    .add_player(PlayerType::Remote(addr_b()), PlayerHandle::new(1))
                    .expect("C remote B")
                    .add_player(PlayerType::Local, PlayerHandle::new(2))
                    .expect("C local")
                    .start_p2p_session(bus.socket(addr_c()))
                    .expect("C builds");

                let mut duo = Duo {
                    bus,
                    clock,
                    a,
                    b,
                    a_shadow: Shadow::default(),
                    b_shadow: Shadow::default(),
                    a_events: Vec::new(),
                    b_events: Vec::new(),
                };
                let mut c_shadow = Shadow::default();

                // Condition-driven with a generous cap (session-33 round-6).
                for _ in 0..300 {
                    duo.poll_round(None);
                    c.poll_remote_clients();
                    let _ = c.events().count();
                    if duo.a.current_state() == SessionState::Running
                        && duo.b.current_state() == SessionState::Running
                        && c.current_state() == SessionState::Running
                    {
                        break;
                    }
                }
                assert_eq!(duo.a.current_state(), SessionState::Running, "A syncs");
                assert_eq!(duo.b.current_state(), SessionState::Running, "B syncs");
                assert_eq!(c.current_state(), SessionState::Running, "C syncs");

                for i in 0..6_u32 {
                    for _ in 0..3 {
                        duo.poll_round(None);
                        c.poll_remote_clients();
                        let _ = c.events().count();
                    }
                    #[allow(clippy::cast_possible_truncation)]
                    // test: bounded loop counter.
                    duo.advance_both(10 + (i as u8), 20 + (i as u8));
                    let c_frame = c.current_frame().as_i32();
                    c.add_local_input(PlayerHandle::new(2), c_varying_input(c_frame))
                        .expect("C local input");
                    let requests = c.advance_frame().expect("C advance");
                    apply_requests(&requests, &mut c_shadow);
                }
                for _ in 0..3 {
                    duo.poll_round(None);
                    c.poll_remote_clients();
                    let _ = c.events().count();
                }

                // Symmetric removal at the same fully-delivered receipt.
                duo.a
                    .remove_player(PlayerHandle::new(2))
                    .expect("A removes C");
                duo.b
                    .remove_player(PlayerHandle::new(2))
                    .expect("B removes C");
                drop(c);
                duo.a_events.extend(duo.a.events());
                duo.b_events.extend(duo.b.events());

                let f0_a = duo.a.local_connect_status[2].last_frame;
                let f0_b = duo.b.local_connect_status[2].last_frame;
                (duo, f0_a, f0_b)
            };
            assert_eq!(
                f0_low, f0_high,
                "staging precondition: the symmetric removal froze both sides at the same frame"
            );
            let f0 = f0_low;

            // Poll-only convergence: the nudge delivers both disconnected
            // claims; NO advances, so both confirmed frames stay at ~f0.
            // Condition-driven with a generous cap (session-33 round-6).
            for _ in 0..120 {
                duo.poll_round(None);
                let a_view = duo
                    .b
                    .player_reg
                    .remotes
                    .get(&addr_a())
                    .expect("B's coordinator endpoint")
                    .peer_connect_status(PlayerHandle::new(2));
                if a_view.disconnected && a_view.last_frame == f0 {
                    break;
                }
            }
            let a_view = duo
                .b
                .player_reg
                .remotes
                .get(&addr_a())
                .expect("B's coordinator endpoint")
                .peer_connect_status(PlayerHandle::new(2));
            assert_eq!(
                a_view,
                ConnectionStatus {
                    disconnected: true,
                    last_frame: f0,
                },
                "staging precondition: B folded A's converged {{disconnected, f0}} claim"
            );

            (duo, f0)
        }

        /// Session-33 round-5 review Finding 1, the pre-reopen convergence
        /// arm: a freeze-frame lowering that lands while a PRE-reopen pending
        /// is held (the N>=4 relay image — at N=3 the acceptance-time
        /// convergence gate makes this unreachable, so the lowered claim is
        /// staged through the endpoint test seam) must be APPLIED, not
        /// deferred: status mine-down, frozen-value re-roll, forced
        /// re-simulation — all WITHOUT the generic path's endpoint teardown
        /// (which would kill the re-armed joiner endpoint and brick the
        /// attempt), and the pending's captured pre-freeze snapshot must be
        /// REFRESHED so a post-reopen abort restores the CONVERGED freeze
        /// (the captured-vs-live staleness called out by the round-5 brief).
        #[test]
        fn npeer_pre_reopen_pending_applies_late_freeze_convergence_without_teardown() {
            let (mut duo, f0) = mesh_with_converged_drop_tight();

            // The tight staging never advances post-drop, so B's own
            // `{disconnected, f0}` claim has no input carrier toward A (and
            // B's nudge is off: B's OWN fold is already mesh-agreed). Seed
            // A's cached view directly — the production carrier is B's
            // ordinary input gossip (any advancing mesh delivers it, and the
            // pre-existing stale-connected confirmed-fold pin blocks the
            // serve's capture until it has).
            duo.a
                .player_reg
                .remotes
                .get_mut(&addr_b())
                .expect("A's survivor endpoint")
                .set_peer_connect_status_for_tests(
                    PlayerHandle::new(2),
                    ConnectionStatus {
                        disconnected: true,
                        last_frame: f0,
                    },
                );

            // Open a serve + deliver the directive so B holds a PRE-reopen
            // pending (the joiner never syncs to B).
            let mut c2 = ManualJoiner::new(&duo.bus.clone(), addr_c());
            c2.connect(addr_a(), 0, &duo.clock.clone());
            sync_joiner_with(&mut duo, &mut c2, addr_a());
            c2.proto_mut(addr_a()).send_join_request(2);
            c2.pump();
            // Condition-driven staging loop (session-33 round-6): generous
            // cap + stall diagnostics, same per-iteration drive as before.
            for _ in 0..100 {
                duo.poll_round(Some(&mut c2));
                if duo.b.hot_join.pending_reactivation.is_some() {
                    break;
                }
            }
            assert!(
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| !pending.reopened),
                "precondition: B holds the PRE-reopen pending — stalled stages: \
                 b_pending_reopened={:?}, a_serve_open={}, b_slot_status={:?}",
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .map(|pending| pending.reopened),
                duo.a.hot_join.npeer.is_some(),
                duo.b.local_connect_status[2],
            );

            // The relay image: a peer B cannot fold re-gossips the agreed
            // freeze one frame LOWER through A's claim. One frame keeps the
            // owed re-simulation inside B's rollback window (B's confirmed
            // is still ~f0 — the tight staging never advanced post-drop).
            let lowered = Frame::new(f0.as_i32() - 1);
            duo.b
                .player_reg
                .remotes
                .get_mut(&addr_a())
                .expect("B's coordinator endpoint")
                .set_peer_connect_status_for_tests(
                    PlayerHandle::new(2),
                    ConnectionStatus {
                        disconnected: true,
                        last_frame: lowered,
                    },
                );

            // One survivor advance applies the convergence (the fold runs
            // inside `advance_frame`); the re-adjust's rollback is part of
            // the same call's requests.
            duo.b
                .add_local_input(PlayerHandle::new(1), 77)
                .expect("B local input");
            let requests = duo
                .b
                .advance_frame()
                .expect("B advances (the one-frame re-adjust stays inside the rollback window)");
            apply_requests(&requests, &mut duo.b_shadow);

            // THE MECHANISM PINS (red pre-fix: the any-pending shield skipped
            // all of this):
            // 1. Status mined down to the relayed minimum.
            assert_eq!(
                duo.b.local_connect_status[2],
                ConnectionStatus {
                    disconnected: true,
                    last_frame: lowered,
                },
                "the held pre-reopen pending must not suspend the status mine-down"
            );
            // 2. Frozen value re-rolled to the converged frame's input
            //    (varying staging => the values genuinely differ).
            assert_eq!(
                duo.b
                    .sync_layer
                    .player_last_confirmed_input(PlayerHandle::new(2)),
                Some(c_varying_input(lowered.as_i32())),
                "the frozen value re-rolls to v(f0 - 1)"
            );
            // 3. The pending's captured pre-freeze snapshot is REFRESHED, so
            //    a later abort restores the CONVERGED freeze, not the stale
            //    capture.
            let pending = duo
                .b
                .hot_join
                .pending_reactivation
                .as_ref()
                .expect("the pending survives the convergence");
            assert!(!pending.reopened, "the pending is still pre-reopen");
            assert_eq!(
                pending.pre_freeze_status,
                ConnectionStatus {
                    disconnected: true,
                    last_frame: lowered,
                },
                "pre_freeze_status is refreshed to the converged freeze"
            );
            assert_eq!(
                pending.pre_freeze_input,
                Some(c_varying_input(lowered.as_i32())),
                "pre_freeze_input is refreshed to the converged frozen value"
            );
            // 4. NO endpoint teardown: the re-armed joiner endpoint survives
            //    (the generic re-adjust path would `disconnect()` it — a
            //    terminal state with no reconnect edge).
            let joiner_endpoint = duo
                .b
                .player_reg
                .remotes
                .get(&addr_c())
                .expect("B's joiner endpoint exists");
            assert!(
                !joiner_endpoint.is_synchronized() || joiner_endpoint.is_running(),
                "the convergence must not tear down the re-armed joiner endpoint (it stays synchronizing/running, never terminal)"
            );
            // 5. The slot stays frozen + reserved (the convergence is a
            //    re-adjust of the freeze, not an attempt-state change).
            assert!(
                duo.b.sync_layer.player_is_frozen(PlayerHandle::new(2)),
                "the slot stays frozen"
            );
            assert!(
                duo.b
                    .hot_join
                    .reserved_slots
                    .contains(&PlayerHandle::new(2)),
                "the slot stays reserved"
            );
        }

        /// Session-33 round-5 review Finding 1, coordinator sibling's abort
        /// arm: a freeze-frame lowering whose re-adjust touches frames at or
        /// below the snapshot frame `S` while the serve already CAPTURED must
        /// abort the serve fail-closed — never recapture at the same `S`: the
        /// joiner applies the FIRST snapshot it receives (duplicates are
        /// idempotent) and acks by FRAME, so once stale bytes may be in
        /// flight, a same-`S` recapture is indistinguishable from the stale
        /// serve. Post-capture lowerings need N>=4 (relay) — at N=3 the
        /// open-time gate plus the pre-capture deferral close the window — so
        /// the lowered claim is staged through the endpoint test seam.
        #[test]
        fn npeer_post_capture_freeze_readjust_aborts_serve_fail_closed() {
            let mut duo = mesh_with_dropped_slot(600, 6);
            let f0 = duo.a.local_connect_status[2].last_frame;

            // Open the serve and drive it to the CAPTURED state; B reopens
            // and acks; the joiner receives the snapshot but does NOT ack
            // yet (the commit barrier stays open).
            let mut c2 = ManualJoiner::new(&duo.bus.clone(), addr_c());
            c2.connect(addr_a(), 0, &duo.clock.clone());
            sync_joiner_with(&mut duo, &mut c2, addr_a());
            c2.proto_mut(addr_a()).send_join_request(2);
            c2.pump();
            duo.poll_round(Some(&mut c2));
            let (serve_s, serve_f) = {
                let serve = duo.a.hot_join.npeer.as_ref().expect("N-peer serve opens");
                (serve.snapshot_frame, serve.activation_frame)
            };
            c2.connect(addr_b(), 1, &duo.clock.clone());
            let mut received_snapshot = false;
            // Condition-driven staging loop (session-33 round-6): generous
            // cap + stall diagnostics, same per-iteration drive as before.
            for i in 0..600_u32 {
                duo.poll_round(Some(&mut c2));
                // The capture gate also requires no pending misprediction;
                // the repair runs only inside `advance_frame` (the paused
                // arm), so interleave advances while B has prediction
                // headroom (A's paused advance never moves its frame).
                if i % 3 == 2
                    && duo.b.current_frame().as_i32() - duo.b.confirmed_frame().as_i32() < 6
                {
                    #[allow(clippy::cast_possible_truncation)]
                    // test: bounded loop counter.
                    duo.advance_both(40 + (i % 8) as u8, 41 + (i % 8) as u8);
                }
                if c2.proto_mut(addr_a()).take_received_snapshot().is_some() {
                    received_snapshot = true;
                }
                let b_reopened = duo
                    .b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .is_some_and(|pending| pending.reopened);
                let a_captured = duo
                    .a
                    .hot_join
                    .npeer
                    .as_ref()
                    .is_some_and(|serve| serve.snapshot.is_some());
                if received_snapshot && b_reopened && a_captured {
                    break;
                }
            }
            assert!(
                duo.a
                    .hot_join
                    .npeer
                    .as_ref()
                    .is_some_and(|serve| serve.snapshot.is_some()),
                "precondition: the snapshot is captured (and possibly in flight) — stalled \
                 stages: joiner_received_snapshot={}, b_pending_reopened={:?}, a_serve_open={}",
                received_snapshot,
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .map(|pending| pending.reopened),
                duo.a.hot_join.npeer.is_some(),
            );
            assert!(
                received_snapshot,
                "precondition: the joiner holds the (about to be stale) snapshot bytes"
            );

            // The relay image lands POST-capture: B's claim for slot 2
            // re-gossips one frame lower than the agreed freeze.
            duo.a
                .player_reg
                .remotes
                .get_mut(&addr_b())
                .expect("A's survivor endpoint")
                .set_peer_connect_status_for_tests(
                    PlayerHandle::new(2),
                    ConnectionStatus {
                        disconnected: true,
                        last_frame: Frame::new(f0.as_i32() - 1),
                    },
                );

            // The joiner's ack races in — pre-fix the next poll takes it and
            // COMMITS the stale snapshot; post-fix the serve-poll detects the
            // owed re-adjust BEFORE the ack is consumed and aborts.
            c2.proto_mut(addr_a()).send_state_snapshot_ack(serve_s);
            c2.pump();
            // Condition-driven conclusion loop (session-33 round-6): generous
            // cap + stall diagnostics, same per-iteration drive as before.
            for _ in 0..60 {
                duo.poll_round(Some(&mut c2));
                if duo.a.hot_join.npeer.is_none() {
                    break;
                }
            }
            duo.a_events.extend(duo.a.events());
            assert!(
                duo.a.hot_join.npeer.is_none(),
                "the serve concludes one way or the other — stalled stages: \
                 a_snapshot_captured={}, post_serve_memo={:?}",
                duo.a
                    .hot_join
                    .npeer
                    .as_ref()
                    .is_some_and(|serve| serve.snapshot.is_some()),
                duo.a
                    .hot_join
                    .npeer_post
                    .as_ref()
                    .map(|post| (post.committed, post.frame)),
            );
            assert!(
                duo.a
                    .hot_join
                    .npeer_post
                    .as_ref()
                    .is_some_and(|post| !post.committed && post.frame == serve_f),
                "the serve must ABORT fail-closed (never commit, never recapture at the same S) when a post-capture re-adjust invalidates the snapshot"
            );
            assert!(
                !duo.a_events.iter().any(|e| matches!(
                    e,
                    FortressEvent::PeerJoined { handle, .. } if *handle == PlayerHandle::new(2)
                )),
                "no PeerJoined: the stale snapshot must not commit"
            );
            assert!(
                duo.a
                    .hot_join
                    .reserved_slots
                    .contains(&PlayerHandle::new(2)),
                "the slot stays reserved on A (the joiner can retry)"
            );
            assert!(
                duo.a.sync_layer.player_is_frozen(PlayerHandle::new(2)),
                "A's slot stays frozen"
            );

            // The abort announcement reaches B: the reopened pending closes
            // and the slot restores to its pre-reopen frozen shape.
            // Condition-driven close loop (session-33 round-6): generous cap
            // + stall diagnostics, same per-iteration drive as before.
            for _ in 0..100 {
                duo.poll_round(Some(&mut c2));
                if duo.b.hot_join.pending_reactivation.is_none() {
                    break;
                }
            }
            assert!(
                duo.b.hot_join.pending_reactivation.is_none(),
                "JoinAborted closes B's reopened pending — stalled stages: \
                 b_pending_reopened={:?}, b_slot_status={:?}",
                duo.b
                    .hot_join
                    .pending_reactivation
                    .as_ref()
                    .map(|pending| pending.reopened),
                duo.b.local_connect_status[2],
            );
            assert!(
                duo.b.sync_layer.player_is_frozen(PlayerHandle::new(2)),
                "B's slot re-freezes on the abort restore"
            );
        }
    }
}
