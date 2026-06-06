use std::collections::BTreeMap;
use std::sync::Arc;

use web_time::Duration;

use crate::{
    error::InvalidRequestKind,
    network::protocol::UdpProtocol,
    replay::Replay,
    sessions::player_registry::PlayerRegistry,
    sessions::replay_session::ReplaySession,
    telemetry::{SessionTelemetry, ViolationObserver},
    time_sync::TimeSyncConfig,
    Config, DesyncDetection, FortressError, NonBlockingSocket, P2PSession, PlayerHandle,
    PlayerType, SpectatorSession, SyncTestSession,
};

// Re-export config types for backwards compatibility with code that imports from builder
pub use crate::sessions::config::{
    DisconnectBehavior, InputQueueConfig, ProtocolConfig, SaveMode, SpectatorConfig, SyncConfig,
};

const DEFAULT_PLAYERS: usize = 2;
/// Default desync detection mode.
///
/// Defaults to `On { interval: 60 }` to catch state divergence early (once per second at 60fps).
/// This aligns with Fortress Rollback's correctness-first philosophy. Users who want to disable
/// desync detection for performance reasons can explicitly set `DesyncDetection::Off`.
///
/// # Breaking Change from GGRS
///
/// GGRS defaulted to `DesyncDetection::Off`. Fortress Rollback enables it by default because:
/// - Silent desync is a correctness bug that's hard to debug
/// - The overhead is minimal (one checksum comparison per second)
/// - Early detection prevents subtle multiplayer issues from reaching production
const DEFAULT_DETECTION_MODE: DesyncDetection = DesyncDetection::On { interval: 60 };

const DEFAULT_INPUT_DELAY: usize = 0;
/// Default peer disconnect timeout.
///
/// # Formal Specification Alignment
/// - **formal-spec.md**: `DEFAULT_DISCONNECT_TIMEOUT = 2000ms`
const DEFAULT_DISCONNECT_TIMEOUT: Duration = Duration::from_secs(2);
const DEFAULT_DISCONNECT_NOTIFY_START: Duration = Duration::from_millis(500);
/// Default frames per second for session timing.
///
/// # Formal Specification Alignment
/// - **formal-spec.md**: `DEFAULT_FPS = 60`
const DEFAULT_FPS: usize = 60;
/// Default maximum prediction window in frames.
///
/// # Formal Specification Alignment
/// - **TLA+**: `MAX_PREDICTION` in `specs/tla/Rollback.tla` (set to 1-3 for model checking)
/// - **Z3**: `MAX_PREDICTION = 8` in `tests/test_z3_verification.rs`
/// - **formal-spec.md**: `DEFAULT_MAX_PREDICTION = 8`, INV-2 bounds rollback depth
/// - **Kani**: Various proofs verify rollback bounds with configurable max_prediction
const DEFAULT_MAX_PREDICTION_FRAMES: usize = 8;
const DEFAULT_CHECK_DISTANCE: usize = 2;
// If the spectator is more than this amount of frames behind, it will advance the game two steps at a time to catch up
const DEFAULT_MAX_FRAMES_BEHIND: usize = 10;
// The amount of frames the spectator advances in a single step if too far behind
const DEFAULT_CATCHUP_SPEED: usize = 1;
/// Default event queue size.
/// Events older than this threshold may be dropped if not polled.
const DEFAULT_EVENT_QUEUE_SIZE: usize = 100;

/// The [`SessionBuilder`] builds all Fortress Rollback Sessions.
///
/// After setting all appropriate values, use `SessionBuilder::start_yxz_session(...)`
/// to consume the builder and create a Session of desired type.
#[must_use = "SessionBuilder must be consumed by calling a start_*_session method"]
pub struct SessionBuilder<T>
where
    T: Config,
{
    num_players: usize,
    local_players: usize,
    max_prediction: usize,
    /// FPS defines the expected update frequency of this session.
    fps: usize,
    save_mode: SaveMode,
    desync_detection: DesyncDetection,
    /// The time until a remote player gets disconnected.
    disconnect_timeout: Duration,
    /// The time until the client will get a notification that a remote player is about to be disconnected.
    disconnect_notify_start: Duration,
    player_reg: PlayerRegistry<T>,
    input_delay: usize,
    check_dist: usize,
    max_frames_behind: usize,
    catchup_speed: usize,
    /// Optional observer for specification violations.
    violation_observer: Option<Arc<dyn ViolationObserver>>,
    /// Configuration for the synchronization protocol.
    sync_config: SyncConfig,
    /// Configuration for the network protocol behavior.
    protocol_config: ProtocolConfig,
    /// Configuration for spectator sessions.
    spectator_config: SpectatorConfig,
    /// Configuration for time synchronization.
    time_sync_config: TimeSyncConfig,
    /// Configuration for input queue sizing.
    input_queue_config: InputQueueConfig,
    /// Maximum number of events to queue before oldest are dropped.
    event_queue_size: usize,
    /// Whether to enable replay recording during P2P sessions.
    recording: bool,
    /// Optional telemetry observer for session performance events.
    telemetry: Option<Arc<dyn SessionTelemetry>>,
    /// Controls how a [`P2PSession`] reacts when a remote peer's
    /// disconnect-timeout fires. See [`DisconnectBehavior`] for options.
    /// Defaults to [`DisconnectBehavior::Halt`] for back-compat with legacy
    /// GGRS-style behavior.
    disconnect_behavior: DisconnectBehavior,
    /// Whether this session serves hot-joins (host role). Set via
    /// [`with_hot_join`](Self::with_hot_join) or implied by
    /// [`add_reserved_player`](Self::add_reserved_player).
    #[cfg(feature = "hot-join")]
    accept_hot_join: bool,
    /// Remote handles reserved for future hot-joiners.
    #[cfg(feature = "hot-join")]
    reserved_slots: std::collections::BTreeSet<PlayerHandle>,
    /// Host-side hot-join serve timeout, in `poll_remote_clients` calls. Defaults
    /// to `DEFAULT_HOT_JOIN_SERVE_TIMEOUT_POLLS`; set via
    /// [`with_hot_join_serve_timeout_polls`](Self::with_hot_join_serve_timeout_polls).
    #[cfg(feature = "hot-join")]
    hot_join_serve_timeout_polls: usize,
    /// Maximum complete encoded `StateSnapshot` wire-message size the host will
    /// serve. Defaults to `DEFAULT_HOT_JOIN_MAX_SNAPSHOT_WIRE_BYTES`; set via
    /// [`with_hot_join_max_snapshot_wire_bytes`](Self::with_hot_join_max_snapshot_wire_bytes).
    #[cfg(feature = "hot-join")]
    hot_join_max_snapshot_wire_bytes: usize,
    /// Joiner-side hot-join ack-resend budget, in `poll_remote_clients` calls.
    /// Defaults to `DEFAULT_HOT_JOIN_ACK_RESENDS`; set via
    /// [`with_hot_join_ack_resends`](Self::with_hot_join_ack_resends).
    #[cfg(feature = "hot-join")]
    hot_join_ack_resends: usize,
}

impl<T: Config> std::fmt::Debug for SessionBuilder<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Destructure to ensure all fields are included when new fields are added.
        // The compiler will error if a new field is added but not handled here.
        let Self {
            num_players,
            local_players,
            max_prediction,
            fps,
            save_mode,
            desync_detection,
            disconnect_timeout,
            disconnect_notify_start,
            player_reg,
            input_delay,
            check_dist,
            max_frames_behind,
            catchup_speed,
            violation_observer,
            sync_config,
            protocol_config,
            spectator_config,
            time_sync_config,
            input_queue_config,
            event_queue_size,
            recording,
            telemetry,
            disconnect_behavior,
            #[cfg(feature = "hot-join")]
            accept_hot_join,
            #[cfg(feature = "hot-join")]
            reserved_slots,
            #[cfg(feature = "hot-join")]
            hot_join_serve_timeout_polls,
            #[cfg(feature = "hot-join")]
            hot_join_max_snapshot_wire_bytes,
            #[cfg(feature = "hot-join")]
            hot_join_ack_resends,
        } = self;

        let mut debug = f.debug_struct("SessionBuilder");
        debug
            .field("num_players", num_players)
            .field("local_players", local_players)
            .field("max_prediction", max_prediction)
            .field("fps", fps)
            .field("save_mode", save_mode)
            .field("desync_detection", desync_detection)
            .field("disconnect_timeout", disconnect_timeout)
            .field("disconnect_notify_start", disconnect_notify_start)
            .field("player_reg", player_reg)
            .field("input_delay", input_delay)
            .field("check_dist", check_dist)
            .field("max_frames_behind", max_frames_behind)
            .field("catchup_speed", catchup_speed)
            .field("has_violation_observer", &violation_observer.is_some())
            .field("has_telemetry", &telemetry.is_some())
            .field("sync_config", sync_config)
            .field("protocol_config", protocol_config)
            .field("spectator_config", spectator_config)
            .field("time_sync_config", time_sync_config)
            .field("input_queue_config", input_queue_config)
            .field("event_queue_size", event_queue_size)
            .field("recording", recording)
            .field("disconnect_behavior", disconnect_behavior);
        #[cfg(feature = "hot-join")]
        debug
            .field("accept_hot_join", accept_hot_join)
            .field("reserved_slots", reserved_slots)
            .field("hot_join_serve_timeout_polls", hot_join_serve_timeout_polls)
            .field(
                "hot_join_max_snapshot_wire_bytes",
                hot_join_max_snapshot_wire_bytes,
            )
            .field("hot_join_ack_resends", hot_join_ack_resends);
        debug.finish()
    }
}

impl<T: Config> Default for SessionBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Config> SessionBuilder<T> {
    /// Construct a new builder with all values set to their defaults.
    pub fn new() -> Self {
        Self {
            player_reg: PlayerRegistry::new(),
            local_players: 0,
            num_players: DEFAULT_PLAYERS,
            max_prediction: DEFAULT_MAX_PREDICTION_FRAMES,
            fps: DEFAULT_FPS,
            save_mode: SaveMode::default(),
            desync_detection: DEFAULT_DETECTION_MODE,
            disconnect_timeout: DEFAULT_DISCONNECT_TIMEOUT,
            disconnect_notify_start: DEFAULT_DISCONNECT_NOTIFY_START,
            input_delay: DEFAULT_INPUT_DELAY,
            check_dist: DEFAULT_CHECK_DISTANCE,
            max_frames_behind: DEFAULT_MAX_FRAMES_BEHIND,
            catchup_speed: DEFAULT_CATCHUP_SPEED,
            violation_observer: None,
            sync_config: SyncConfig::default(),
            protocol_config: ProtocolConfig::default(),
            spectator_config: SpectatorConfig::default(),
            time_sync_config: TimeSyncConfig::default(),
            input_queue_config: InputQueueConfig::default(),
            event_queue_size: DEFAULT_EVENT_QUEUE_SIZE,
            recording: false,
            telemetry: None,
            disconnect_behavior: DisconnectBehavior::default(),
            #[cfg(feature = "hot-join")]
            accept_hot_join: false,
            #[cfg(feature = "hot-join")]
            reserved_slots: std::collections::BTreeSet::new(),
            #[cfg(feature = "hot-join")]
            hot_join_serve_timeout_polls:
                crate::sessions::p2p_session::DEFAULT_HOT_JOIN_SERVE_TIMEOUT_POLLS,
            #[cfg(feature = "hot-join")]
            hot_join_max_snapshot_wire_bytes:
                crate::sessions::hot_join::DEFAULT_HOT_JOIN_MAX_SNAPSHOT_WIRE_BYTES,
            #[cfg(feature = "hot-join")]
            hot_join_ack_resends: crate::sessions::p2p_session::DEFAULT_HOT_JOIN_ACK_RESENDS,
        }
    }

    /// Must be called for each player in the session (e.g. in a 3 player session, must be called 3 times) before starting the session.
    /// Player handles for players should be between 0 and `num_players`, spectator handles should be higher than `num_players`.
    /// Later, you will need the player handle to add input, change parameters or disconnect the player or spectator.
    ///
    /// # Errors
    /// - Returns a [`FortressError`] if a player with that handle has been added before
    /// - Returns a [`FortressError`] if the handle is invalid for the given [`PlayerType`]
    ///
    pub fn add_player(
        mut self,
        player_type: PlayerType<T::Address>,
        player_handle: PlayerHandle,
    ) -> Result<Self, FortressError> {
        // check if the player handle is already in use
        if self.player_reg.handles.contains_key(&player_handle) {
            return Err(InvalidRequestKind::PlayerHandleInUse {
                handle: player_handle,
            }
            .into());
        }
        // check if the player handle is valid for the given player type
        match player_type {
            PlayerType::Local => {
                self.local_players += 1;
                if !player_handle.is_valid_player_for(self.num_players) {
                    return Err(InvalidRequestKind::InvalidLocalPlayerHandle {
                        handle: player_handle,
                        num_players: self.num_players,
                    }
                    .into());
                }
            },
            PlayerType::Remote(_) => {
                if !player_handle.is_valid_player_for(self.num_players) {
                    return Err(InvalidRequestKind::InvalidRemotePlayerHandle {
                        handle: player_handle,
                        num_players: self.num_players,
                    }
                    .into());
                }
            },
            PlayerType::Spectator(_) => {
                if !player_handle.is_spectator_for(self.num_players) {
                    return Err(InvalidRequestKind::InvalidSpectatorHandle {
                        handle: player_handle,
                        num_players: self.num_players,
                    }
                    .into());
                }
            },
        }
        self.player_reg.handles.insert(player_handle, player_type);
        Ok(self)
    }

    /// Adds a local player at the specified handle index.
    ///
    /// This is a convenience wrapper around [`Self::add_player`] with [`PlayerType::Local`].
    ///
    /// # Arguments
    ///
    /// * `handle` - The player handle index (0-based)
    ///
    /// # Errors
    ///
    /// Returns an error if the handle is invalid or a player already exists at this handle.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fortress_rollback::prelude::*;
    /// # use std::net::SocketAddr;
    /// # #[derive(Debug)]
    /// # struct TestConfig;
    /// # impl Config for TestConfig {
    /// #     type Input = u8;
    /// #     type State = u8;
    /// #     type Address = SocketAddr;
    /// # }
    /// let builder = SessionBuilder::<TestConfig>::new()
    ///     .with_num_players(2)?
    ///     .add_local_player(0)?;
    /// # Ok::<(), FortressError>(())
    /// ```
    pub fn add_local_player(self, handle: usize) -> Result<Self, FortressError> {
        self.add_player(PlayerType::Local, PlayerHandle::new(handle))
    }

    /// Adds a remote player at the specified handle index.
    ///
    /// This is a convenience wrapper around [`Self::add_player`] with [`PlayerType::Remote`].
    ///
    /// # Arguments
    ///
    /// * `handle` - The player handle index (0-based)
    /// * `addr` - The network address of the remote player
    ///
    /// # Errors
    ///
    /// Returns an error if the handle is invalid or a player already exists at this handle.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fortress_rollback::prelude::*;
    /// # use std::net::SocketAddr;
    /// # #[derive(Debug)]
    /// # struct TestConfig;
    /// # impl Config for TestConfig {
    /// #     type Input = u8;
    /// #     type State = u8;
    /// #     type Address = SocketAddr;
    /// # }
    /// let addr: SocketAddr = "127.0.0.1:7000".parse()?;
    /// let builder = SessionBuilder::<TestConfig>::new()
    ///     .with_num_players(2)?
    ///     .add_remote_player(0, addr)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn add_remote_player(self, handle: usize, addr: T::Address) -> Result<Self, FortressError> {
        self.add_player(PlayerType::Remote(addr), PlayerHandle::new(handle))
    }

    /// Enables (or disables) serving hot-joins for this session (host role).
    ///
    /// When enabled, a host [`P2PSession`] responds to a hot-joiner's snapshot
    /// request for a reserved/dropped slot by capturing and serving its saved
    /// state as an **ack-gated transaction**: it caches and re-sends the
    /// snapshot until the joiner acks, and only then reactivates the slot. See
    /// [`add_reserved_player`](Self::add_reserved_player) and
    /// [`start_hot_join_session`](Self::start_hot_join_session).
    ///
    /// While a join is in flight the solo host **pauses** (its
    /// [`advance_frame`](P2PSession::advance_frame) returns an empty request set
    /// and the simulation does not advance) for the duration of the ~1–2 RTT
    /// handshake; it resumes automatically once the join completes or the serve
    /// times out. This is safe in the 2-peer reserved-slot scope and bounds the
    /// handshake. Keep polling/advancing during the pause.
    ///
    /// # Recovery from an abandoned join
    ///
    /// If a serve is abandoned mid-handshake (the joiner stops acking but its
    /// endpoint stays alive), the host aborts it after a bounded number of polls
    /// and **resumes advancing solo** with the slot left reserved/frozen — it
    /// never falls back to `Synchronizing` and emits no user-facing
    /// `Disconnected`. Because the slot stays reserved, the **same still-alive
    /// joiner** can retry **in-session**: it keeps re-sending its `JoinRequest`
    /// while `HotJoining`, so once the snapshot reaches it the host re-opens a
    /// serve and the join completes with no further action from your code. A
    /// brand-new joiner connection to that reserved address is likewise served.
    ///
    /// # Re-joining a gracefully-dropped slot
    ///
    /// Hot-join serving is not limited to slots reserved at build time. When this
    /// host serves hot-joins, a player slot that is **cleanly gracefully dropped**
    /// — via [`remove_player`](P2PSession::remove_player), or automatically on the
    /// disconnect timeout when [`DisconnectBehavior::ContinueWithout`] is
    /// configured — is automatically returned to the reserved/frozen state, so a
    /// returning peer can re-fill it exactly like a build-time reserved slot. The
    /// returning peer connects with
    /// [`start_hot_join_session`](Self::start_hot_join_session) from the **same
    /// address** the dropped peer used (the host keys the slot's endpoint by that
    /// address). Slots dropped via the legacy
    /// [`disconnect_player`](P2PSession::disconnect_player) (a `Halt`-style
    /// disconnect), or dropped while hot-join serving is disabled, are **not** made
    /// re-joinable.
    ///
    /// Hot-join requires `max_prediction >= 1`: in lockstep mode
    /// (`max_prediction == 0`) the host never saves state and so can never serve
    /// a snapshot, so the start methods reject that configuration.
    ///
    /// This is feature-gated behind the `hot-join` feature.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fortress_rollback::prelude::*;
    /// # use std::net::SocketAddr;
    /// # #[derive(Clone)]
    /// # struct State;
    /// # #[derive(Debug)]
    /// # struct TestConfig;
    /// # impl Config for TestConfig {
    /// #     type Input = u8;
    /// #     type State = u8;
    /// #     type Address = SocketAddr;
    /// # }
    /// let builder = SessionBuilder::<TestConfig>::new().with_hot_join(true);
    /// ```
    #[cfg(feature = "hot-join")]
    pub fn with_hot_join(mut self, enabled: bool) -> Self {
        self.accept_hot_join = enabled;
        self
    }

    /// Registers a **remote** slot reserved for a future hot-joiner (host side).
    ///
    /// This creates a remote endpoint at `addr` exactly like
    /// [`add_remote_player`](Self::add_remote_player) (so a joiner can later
    /// synchronize to that address), but additionally records `handle` as
    /// *reserved*. A reserved slot:
    ///
    /// - is frozen and marked disconnected from frame 0 (behaving like a
    ///   gracefully-dropped Feature-5 slot), so the host reaches
    ///   [`SessionState::Running`](crate::SessionState::Running) and advances
    ///   solo without waiting for the absent joiner;
    /// - does not block synchronization or trigger a sync-timeout disconnect
    ///   while waiting for a joiner;
    /// - is reactivated when a peer hot-joins and loads the host's snapshot.
    ///
    /// Calling this implies hot-join serving (it sets the same flag as
    /// [`with_hot_join(true)`](Self::with_hot_join)) so the session cannot be
    /// misconfigured with a reserved slot but no serving.
    ///
    /// This is feature-gated behind the `hot-join` feature.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`add_player`](Self::add_player) (e.g. the
    /// handle is already in use or invalid for a remote player).
    #[cfg(feature = "hot-join")]
    pub fn add_reserved_player(
        mut self,
        addr: T::Address,
        handle: PlayerHandle,
    ) -> Result<Self, FortressError> {
        self = self.add_player(PlayerType::Remote(addr), handle)?;
        self.reserved_slots.insert(handle);
        self.accept_hot_join = true;
        Ok(self)
    }

    /// Overrides the host-side hot-join **serve timeout**, in
    /// [`poll_remote_clients`](P2PSession::poll_remote_clients) calls.
    ///
    /// This is the maximum number of polls a host keeps a single in-flight serve
    /// open (re-sending the cached snapshot each poll) before aborting it and
    /// resuming solo with the slot still reserved. Defaults to
    /// `DEFAULT_HOT_JOIN_SERVE_TIMEOUT_POLLS` (600 polls). Larger values tolerate
    /// slower or lossier joiners; smaller values free the paused host sooner.
    ///
    /// Only meaningful on the host side (a session built with
    /// [`add_reserved_player`](Self::add_reserved_player) /
    /// [`with_hot_join`](Self::with_hot_join)); it is inert on a joiner session.
    ///
    /// This is feature-gated behind the `hot-join` feature.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidRequestKind::NotSupported`] if `polls` is less than `2`:
    /// the host sends a snapshot once per poll and checks the timeout after that
    /// send, so a one-poll timeout would open and abort the serve in the same
    /// call.
    #[cfg(feature = "hot-join")]
    pub fn with_hot_join_serve_timeout_polls(
        mut self,
        polls: usize,
    ) -> Result<Self, FortressError> {
        if polls < 2 {
            return Err(InvalidRequestKind::NotSupported {
                operation: "with_hot_join_serve_timeout_polls(<2) (the serve timeout must be >= 2)",
            }
            .into());
        }
        self.hot_join_serve_timeout_polls = polls;
        Ok(self)
    }

    /// Overrides the maximum complete encoded hot-join `StateSnapshot` wire
    /// message the host will serve, in bytes.
    ///
    /// The default is
    /// `DEFAULT_HOT_JOIN_MAX_SNAPSHOT_WIRE_BYTES` (4 KiB), matching the built-in
    /// UDP sockets' receive buffers. Raising this is useful only when every
    /// peer's transport can receive larger packets. Oversized snapshots are
    /// rejected before the host allocates `state_bytes` or opens a paused serve,
    /// so a too-large state cannot repeatedly wedge the host's hot-join loop.
    ///
    /// Only meaningful on the host side (a session built with
    /// [`add_reserved_player`](Self::add_reserved_player) /
    /// [`with_hot_join`](Self::with_hot_join)); it is inert on a joiner session.
    ///
    /// This is feature-gated behind the `hot-join` feature.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidRequestKind::ConfigValueOutOfRange`] if `bytes` is `0`.
    #[cfg(feature = "hot-join")]
    pub fn with_hot_join_max_snapshot_wire_bytes(
        mut self,
        bytes: usize,
    ) -> Result<Self, FortressError> {
        if bytes == 0 {
            return Err(InvalidRequestKind::ConfigValueOutOfRange {
                field: "hot_join_max_snapshot_wire_bytes",
                min: 1,
                max: u64::MAX,
                actual: 0,
            }
            .into());
        }
        self.hot_join_max_snapshot_wire_bytes = bytes;
        Ok(self)
    }

    /// Overrides the joiner-side hot-join **ack-resend budget**, in
    /// [`poll_remote_clients`](P2PSession::poll_remote_clients) calls.
    ///
    /// After applying the host's snapshot the joiner re-sends its
    /// `StateSnapshotAck` for up to this many polls to tolerate a lost ack,
    /// stopping early once it observes the host has progressed past the
    /// activation frame. Defaults to `DEFAULT_HOT_JOIN_ACK_RESENDS` (30 polls).
    /// `0` is allowed and means the joiner acks exactly once with no loss
    /// tolerance.
    ///
    /// Only meaningful on the joiner side (a session built with
    /// [`start_hot_join_session`](Self::start_hot_join_session)); it is inert on a
    /// host session.
    ///
    /// This is feature-gated behind the `hot-join` feature.
    #[cfg(feature = "hot-join")]
    pub fn with_hot_join_ack_resends(mut self, resends: usize) -> Self {
        self.hot_join_ack_resends = resends;
        self
    }

    /// Change the maximum prediction window. Default is 8.
    ///
    /// ## Lockstep mode
    ///
    /// As a special case, if you set this to 0, Fortress Rollback will run in lockstep mode:
    /// * Fortress Rollback will only request that you advance the gamestate if the current frame has inputs
    ///   confirmed from all other clients.
    /// * Fortress Rollback will never request you to save or roll back the gamestate.
    ///
    /// Lockstep mode can significantly reduce the (Fortress Rollback) framerate of your game, but may be
    /// appropriate for games where a Fortress Rollback frame does not correspond to a rendered frame, such as a
    /// game where Fortress Rollback frames are only advanced once a second; with input delay set to zero, the
    /// framerate impact is approximately equivalent to taking the highest latency client and adding
    /// its latency to the current time to tick a frame.
    pub fn with_max_prediction_window(mut self, window: usize) -> Self {
        self.max_prediction = window;
        self
    }

    /// Change the amount of frames Fortress Rollback will delay the inputs for local players.
    ///
    /// # Errors
    ///
    /// Returns a [`FortressError`] if `delay` exceeds the maximum allowed value.
    /// The maximum delay is `queue_length - 1` (default 127, configurable via
    /// [`with_input_queue_config`](Self::with_input_queue_config)).
    ///
    /// This limit ensures the circular input buffer doesn't overflow.
    /// At 60fps with default settings, max delay is 127 frames (~2.1 seconds),
    /// far exceeding any practical input delay (typically 0-8 frames).
    ///
    /// This constraint was discovered through Kani formal verification.
    ///
    /// # Example
    ///
    /// ```
    /// use fortress_rollback::{SessionBuilder, Config, InputQueueConfig, FortressError};
    ///
    /// # #[derive(Debug)]
    /// # struct TestConfig;
    /// # impl Config for TestConfig {
    /// #     type Input = u8;
    /// #     type State = ();
    /// #     type Address = std::net::SocketAddr;
    /// # }
    /// // Default queue allows delays up to 127
    /// let builder = SessionBuilder::<TestConfig>::new()
    ///     .with_input_delay(8)?;
    ///
    /// // With custom queue size, max delay changes
    /// let builder = SessionBuilder::<TestConfig>::new()
    ///     .with_input_queue_config(InputQueueConfig::minimal()) // queue_length = 32
    ///     .with_input_delay(30)?; // max is now 31
    ///
    /// // Exceeding the limit returns an error
    /// let result = SessionBuilder::<TestConfig>::new()
    ///     .with_input_delay(200);
    /// assert!(result.is_err());
    /// # Ok::<(), FortressError>(())
    /// ```
    pub fn with_input_delay(mut self, delay: usize) -> Result<Self, FortressError> {
        let max_delay = self.input_queue_config.max_frame_delay();
        if delay > max_delay {
            return Err(InvalidRequestKind::FrameDelayTooLarge { delay, max_delay }.into());
        }
        self.input_delay = delay;
        Ok(self)
    }

    /// Change number of total players. Default is 2.
    ///
    /// # Errors
    ///
    /// Returns a [`FortressError`] if `num_players` is 0.
    pub fn with_num_players(mut self, num_players: usize) -> Result<Self, FortressError> {
        if num_players == 0 {
            return Err(InvalidRequestKind::ZeroPlayers.into());
        }
        self.num_players = num_players;
        Ok(self)
    }

    /// Sets the save mode for game state management.
    ///
    /// Controls how frequently the session requests state saves for rollback.
    /// See [`SaveMode`] for detailed documentation on each option.
    ///
    /// # Example
    ///
    /// ```
    /// use fortress_rollback::{SessionBuilder, SaveMode, Config};
    ///
    /// # struct MyConfig;
    /// # impl Config for MyConfig {
    /// #     type Input = u32;
    /// #     type State = ();
    /// #     type Address = std::net::SocketAddr;
    /// # }
    /// // For games with expensive state serialization
    /// let builder = SessionBuilder::<MyConfig>::new()
    ///     .with_save_mode(SaveMode::Sparse);
    /// ```
    pub fn with_save_mode(mut self, save_mode: SaveMode) -> Self {
        self.save_mode = save_mode;
        self
    }

    /// Sets the sparse saving mode (deprecated: use `with_save_mode` instead).
    ///
    /// With sparse saving turned on, only the minimum confirmed frame
    /// (for which all inputs from all players are confirmed correct) will be saved.
    /// This leads to much less save requests at the cost of potentially longer rollbacks
    /// and thus more advance frame requests.
    ///
    /// Recommended if saving your gamestate takes much more time than advancing
    /// the game state.
    #[deprecated(
        since = "0.2.0",
        note = "Use `with_save_mode(SaveMode::Sparse)` instead"
    )]
    pub fn with_sparse_saving_mode(mut self, sparse_saving: bool) -> Self {
        self.save_mode = if sparse_saving {
            SaveMode::Sparse
        } else {
            SaveMode::EveryFrame
        };
        self
    }

    /// Sets the desync detection mode. With desync detection, the session will compare checksums for all peers to detect discrepancies / desyncs between peers
    /// If a desync is found the session will send a DesyncDetected event.
    pub fn with_desync_detection_mode(mut self, desync_detection: DesyncDetection) -> Self {
        self.desync_detection = desync_detection;
        self
    }

    /// Sets the disconnect timeout. The session will automatically disconnect from a remote peer if it has not received a packet in the timeout window.
    pub fn with_disconnect_timeout(mut self, timeout: Duration) -> Self {
        self.disconnect_timeout = timeout;
        self
    }

    /// Sets the time before the first notification will be sent in case of a prolonged period of no received packages.
    pub fn with_disconnect_notify_delay(mut self, notify_delay: Duration) -> Self {
        self.disconnect_notify_start = notify_delay;
        self
    }

    /// Controls what happens when a peer disconnects mid-session.
    ///
    /// Defaults to [`DisconnectBehavior::Halt`] for back-compat with the
    /// legacy GGRS-style behavior. Set to [`DisconnectBehavior::ContinueWithout`]
    /// to enable graceful peer drop: the dropped peer's input queue is frozen
    /// (it repeats their last confirmed input forever), a
    /// [`crate::FortressEvent::PeerDropped`] event is emitted, and the
    /// session keeps advancing for the remaining peers.
    ///
    /// # Example
    ///
    /// ```
    /// use fortress_rollback::{Config, DisconnectBehavior, SessionBuilder};
    ///
    /// # struct MyConfig;
    /// # impl Config for MyConfig {
    /// #     type Input = u8;
    /// #     type State = ();
    /// #     type Address = std::net::SocketAddr;
    /// # }
    /// let builder = SessionBuilder::<MyConfig>::new()
    ///     .with_disconnect_behavior(DisconnectBehavior::ContinueWithout);
    /// ```
    pub fn with_disconnect_behavior(mut self, behavior: DisconnectBehavior) -> Self {
        self.disconnect_behavior = behavior;
        self
    }

    /// Sets the synchronization protocol configuration.
    ///
    /// This allows fine-tuning the sync handshake behavior for different network
    /// conditions. See [`SyncConfig`] for available options and presets.
    ///
    /// # Example
    ///
    /// ```
    /// use fortress_rollback::{SessionBuilder, Config, SyncConfig};
    ///
    /// # struct MyConfig;
    /// # impl Config for MyConfig {
    /// #     type Input = u8;
    /// #     type State = ();
    /// #     type Address = std::net::SocketAddr;
    /// # }
    /// // Use the high-latency preset
    /// let builder = SessionBuilder::<MyConfig>::new()
    ///     .with_sync_config(SyncConfig::high_latency());
    ///
    /// // Or customize individual settings
    /// let custom_config = SyncConfig {
    ///     num_sync_packets: 8,
    ///     ..SyncConfig::default()
    /// };
    /// let builder = SessionBuilder::<MyConfig>::new()
    ///     .with_sync_config(custom_config);
    /// ```
    pub fn with_sync_config(mut self, sync_config: SyncConfig) -> Self {
        self.sync_config = sync_config;
        self
    }

    /// Sets the network protocol configuration.
    ///
    /// This allows fine-tuning network timing, buffering, and telemetry thresholds.
    /// See [`ProtocolConfig`] for available options and presets.
    ///
    /// # Example
    ///
    /// ```
    /// use fortress_rollback::{SessionBuilder, Config, ProtocolConfig};
    ///
    /// # struct MyConfig;
    /// # impl Config for MyConfig {
    /// #     type Input = u8;
    /// #     type State = ();
    /// #     type Address = std::net::SocketAddr;
    /// # }
    /// // Use the competitive preset for LAN play
    /// let builder = SessionBuilder::<MyConfig>::new()
    ///     .with_protocol_config(ProtocolConfig::competitive());
    ///
    /// // Or customize individual settings
    /// let custom_config = ProtocolConfig {
    ///     quality_report_interval: web_time::Duration::from_millis(100),
    ///     ..ProtocolConfig::default()
    /// };
    /// let builder = SessionBuilder::<MyConfig>::new()
    ///     .with_protocol_config(custom_config);
    /// ```
    pub fn with_protocol_config(mut self, protocol_config: ProtocolConfig) -> Self {
        self.protocol_config = protocol_config;
        self
    }

    /// Sets the spectator session configuration.
    ///
    /// This allows fine-tuning spectator behavior including buffer sizes,
    /// catch-up speed, and frame lag tolerance.
    /// See [`SpectatorConfig`] for available options and presets.
    ///
    /// # Example
    ///
    /// ```
    /// use fortress_rollback::{SessionBuilder, Config, SpectatorConfig};
    ///
    /// # struct MyConfig;
    /// # impl Config for MyConfig {
    /// #     type Input = u8;
    /// #     type State = ();
    /// #     type Address = std::net::SocketAddr;
    /// # }
    /// // Use the fast-paced preset for action games
    /// let builder = SessionBuilder::<MyConfig>::new()
    ///     .with_spectator_config(SpectatorConfig::fast_paced());
    ///
    /// // Or customize individual settings
    /// let custom_config = SpectatorConfig {
    ///     buffer_size: 90,
    ///     max_frames_behind: 15,
    ///     ..SpectatorConfig::default()
    /// };
    /// let builder = SessionBuilder::<MyConfig>::new()
    ///     .with_spectator_config(custom_config);
    /// ```
    pub fn with_spectator_config(mut self, spectator_config: SpectatorConfig) -> Self {
        self.spectator_config = spectator_config;
        // Also update the legacy fields for backwards compatibility
        self.max_frames_behind = spectator_config.max_frames_behind;
        self.catchup_speed = spectator_config.catchup_speed;
        self
    }

    /// Sets the time synchronization configuration.
    ///
    /// This allows fine-tuning the frame advantage averaging window size,
    /// which affects how responsive vs stable the synchronization is.
    /// See [`TimeSyncConfig`] for available options and presets.
    ///
    /// # Example
    ///
    /// ```
    /// use fortress_rollback::{SessionBuilder, Config, TimeSyncConfig};
    ///
    /// # struct MyConfig;
    /// # impl Config for MyConfig {
    /// #     type Input = u8;
    /// #     type State = ();
    /// #     type Address = std::net::SocketAddr;
    /// # }
    /// // Use the responsive preset for competitive play
    /// let builder = SessionBuilder::<MyConfig>::new()
    ///     .with_time_sync_config(TimeSyncConfig::responsive());
    ///
    /// // Or customize the window size
    /// let custom_config = TimeSyncConfig {
    ///     window_size: 45,
    /// };
    /// let builder = SessionBuilder::<MyConfig>::new()
    ///     .with_time_sync_config(custom_config);
    /// ```
    pub fn with_time_sync_config(mut self, time_sync_config: TimeSyncConfig) -> Self {
        self.time_sync_config = time_sync_config;
        self
    }

    /// Sets the input queue configuration.
    ///
    /// This allows configuring the size of the input queue (circular buffer) that stores
    /// player inputs. A larger queue allows for longer input history and higher frame delays,
    /// but uses more memory.
    ///
    /// See [`InputQueueConfig`] for available options and presets.
    ///
    /// # Important
    ///
    /// If you plan to use [`with_input_delay`](Self::with_input_delay), call this method first
    /// to ensure the delay is validated against the correct queue size.
    ///
    /// # Example
    ///
    /// ```
    /// use fortress_rollback::{SessionBuilder, Config, InputQueueConfig};
    ///
    /// # struct MyConfig;
    /// # impl Config for MyConfig {
    /// #     type Input = u8;
    /// #     type State = ();
    /// #     type Address = std::net::SocketAddr;
    /// # }
    /// // For high-latency networks, use a larger queue
    /// let builder = SessionBuilder::<MyConfig>::new()
    ///     .with_input_queue_config(InputQueueConfig::high_latency());
    ///
    /// // For memory-constrained environments, use a smaller queue
    /// let builder = SessionBuilder::<MyConfig>::new()
    ///     .with_input_queue_config(InputQueueConfig::minimal());
    ///
    /// // Or customize the queue length
    /// let custom_config = InputQueueConfig {
    ///     queue_length: 64,
    /// };
    /// let builder = SessionBuilder::<MyConfig>::new()
    ///     .with_input_queue_config(custom_config);
    /// ```
    pub fn with_input_queue_config(mut self, input_queue_config: InputQueueConfig) -> Self {
        self.input_queue_config = input_queue_config;
        self
    }

    /// Sets the maximum number of events to queue before oldest are dropped.
    ///
    /// When the event queue exceeds this size, the oldest events are discarded.
    /// This provides backpressure if the application isn't consuming events quickly enough.
    ///
    /// # Arguments
    ///
    /// * `size` - Maximum number of events to buffer. Must be at least 10.
    ///
    /// # Errors
    ///
    /// Returns a [`FortressError`] if `size` is less than 10.
    ///
    /// # Example
    ///
    /// ```
    /// use fortress_rollback::{SessionBuilder, Config, FortressError};
    ///
    /// # struct MyConfig;
    /// # impl Config for MyConfig {
    /// #     type Input = u8;
    /// #     type State = ();
    /// #     type Address = std::net::SocketAddr;
    /// # }
    /// // Increase event queue for high-frequency event scenarios
    /// let builder = SessionBuilder::<MyConfig>::new()
    ///     .with_event_queue_size(200)?;
    /// # Ok::<(), FortressError>(())
    /// ```
    pub fn with_event_queue_size(mut self, size: usize) -> Result<Self, FortressError> {
        if size < 10 {
            return Err(InvalidRequestKind::EventQueueSizeTooSmall { size }.into());
        }
        self.event_queue_size = size;
        Ok(self)
    }

    /// Enables or disables replay recording during a P2P session.
    ///
    /// When recording is enabled, the [`P2PSession`] will capture all confirmed
    /// inputs as they are processed. After the session ends, call
    /// [`P2PSession::into_replay`] to extract the recorded [`Replay`].
    ///
    /// Recording is disabled by default.
    ///
    /// # Example
    ///
    /// ```
    /// # use fortress_rollback::prelude::*;
    /// # use std::net::SocketAddr;
    /// # #[derive(Debug)]
    /// # struct TestConfig;
    /// # impl Config for TestConfig {
    /// #     type Input = u8;
    /// #     type State = u8;
    /// #     type Address = SocketAddr;
    /// # }
    /// let builder = SessionBuilder::<TestConfig>::new()
    ///     .with_recording(true);
    /// ```
    ///
    /// [`Replay`]: crate::replay::Replay
    pub fn with_recording(mut self, enabled: bool) -> Self {
        self.recording = enabled;
        self
    }

    /// Sets the FPS this session is used with. This influences estimations for frame synchronization between sessions.
    /// # Errors
    /// - Returns a [`FortressError`] if the fps is 0
    pub fn with_fps(mut self, fps: usize) -> Result<Self, FortressError> {
        if fps == 0 {
            return Err(InvalidRequestKind::ZeroFps.into());
        }
        self.fps = fps;
        Ok(self)
    }

    /// Change the check distance. Default is 2.
    pub fn with_check_distance(mut self, check_distance: usize) -> Self {
        self.check_dist = check_distance;
        self
    }

    /// Sets the maximum frames behind. If the spectator is more than this amount of frames behind the received inputs,
    /// it will catch up with `catchup_speed` amount of frames per step.
    ///
    /// Note: Prefer using [`Self::with_spectator_config`] for configuring spectator behavior.
    pub fn with_max_frames_behind(
        mut self,
        max_frames_behind: usize,
    ) -> Result<Self, FortressError> {
        self.max_frames_behind = max_frames_behind;
        self.spectator_config.max_frames_behind = max_frames_behind;
        Ok(self)
    }

    /// Sets the catchup speed. Per default, this is set to 1, so the spectator never catches up.
    /// If you want the spectator to catch up to the host if `max_frames_behind` is surpassed, set this to a value higher than 1.
    ///
    /// Note: Prefer using [`Self::with_spectator_config`] for configuring spectator behavior.
    pub fn with_catchup_speed(mut self, catchup_speed: usize) -> Result<Self, FortressError> {
        self.catchup_speed = catchup_speed;
        self.spectator_config.catchup_speed = catchup_speed;
        Ok(self)
    }

    /// Sets a custom observer for specification violations.
    ///
    /// When a violation occurs during session operation (e.g., frame sync issues,
    /// input queue anomalies, checksum mismatches), it will be reported to this observer.
    /// This enables programmatic monitoring, custom logging, or test assertions.
    ///
    /// If no observer is set, violations are logged via the `tracing` crate by default.
    ///
    /// # Example
    ///
    /// ```
    /// use fortress_rollback::{SessionBuilder, Config, telemetry::CollectingObserver};
    /// use std::sync::Arc;
    ///
    /// # struct MyConfig;
    /// # impl Config for MyConfig {
    /// #     type Input = u8;
    /// #     type State = ();
    /// #     type Address = std::net::SocketAddr;
    /// # }
    /// let observer = Arc::new(CollectingObserver::new());
    /// let builder = SessionBuilder::<MyConfig>::new()
    ///     .with_violation_observer(observer.clone());
    ///
    /// // After session operations, check for violations
    /// // assert!(observer.violations().is_empty());
    /// ```
    pub fn with_violation_observer(mut self, observer: Arc<dyn ViolationObserver>) -> Self {
        self.violation_observer = Some(observer);
        self
    }

    /// Attaches a telemetry observer to receive session performance events.
    ///
    /// The telemetry observer will receive callbacks for rollbacks, prediction
    /// misses, frame advances, and network statistics during P2P sessions.
    ///
    /// # Example
    ///
    /// ```
    /// use fortress_rollback::prelude::*;
    /// use fortress_rollback::telemetry::{CollectingTelemetry, SessionTelemetry};
    /// use std::sync::Arc;
    ///
    /// # struct MyConfig;
    /// # impl Config for MyConfig {
    /// #     type Input = u8;
    /// #     type State = ();
    /// #     type Address = std::net::SocketAddr;
    /// # }
    /// let telemetry = Arc::new(CollectingTelemetry::new());
    /// let builder = SessionBuilder::<MyConfig>::new()
    ///     .with_telemetry(telemetry.clone());
    ///
    /// // After session operations, inspect telemetry events
    /// // assert!(telemetry.events().is_empty());
    /// ```
    pub fn with_telemetry(mut self, telemetry: Arc<dyn SessionTelemetry>) -> Self {
        self.telemetry = Some(telemetry);
        self
    }

    // =========================================================================
    // Session Presets
    // =========================================================================

    /// Applies LAN-optimized defaults for low-latency local network play.
    ///
    /// This preset configures the session for minimal latency scenarios typical
    /// of local area networks, where RTT is typically <10ms and packet loss is rare.
    ///
    /// # Configuration Applied
    ///
    /// - **Sync**: Fast handshake with fewer required roundtrips ([`SyncConfig::lan()`])
    /// - **Protocol**: Competitive settings with quick detection ([`ProtocolConfig::competitive()`])
    /// - **Time Sync**: Small window for responsive sync ([`TimeSyncConfig::lan()`])
    /// - **Input Delay**: 0 frames (can be adjusted with [`with_input_delay()`])
    ///
    /// # Example
    ///
    /// ```
    /// use fortress_rollback::{SessionBuilder, Config};
    ///
    /// # struct MyConfig;
    /// # impl Config for MyConfig {
    /// #     type Input = u8;
    /// #     type State = ();
    /// #     type Address = std::net::SocketAddr;
    /// # }
    /// let builder = SessionBuilder::<MyConfig>::new()
    ///     .with_lan_defaults();
    /// ```
    ///
    /// [`SyncConfig::lan()`]: crate::SyncConfig::lan
    /// [`ProtocolConfig::competitive()`]: crate::ProtocolConfig::competitive
    /// [`TimeSyncConfig::lan()`]: crate::TimeSyncConfig::lan
    /// [`with_input_delay()`]: Self::with_input_delay
    pub fn with_lan_defaults(self) -> Self {
        self.with_sync_config(SyncConfig::lan())
            .with_protocol_config(ProtocolConfig::competitive())
            .with_time_sync_config(TimeSyncConfig::lan())
    }

    /// Applies Internet-optimized defaults for typical online play.
    ///
    /// This preset configures the session for typical internet connections with
    /// moderate latency (30-100ms RTT) and occasional packet loss (<5%).
    ///
    /// # Configuration Applied
    ///
    /// - **Sync**: Default handshake settings ([`SyncConfig::default()`])
    /// - **Protocol**: Default network timing ([`ProtocolConfig::default()`])
    /// - **Time Sync**: Default averaging window ([`TimeSyncConfig::default()`])
    /// - **Input Delay**: 2 frames (recommended for hiding network jitter)
    ///
    /// # Errors
    ///
    /// Returns a [`FortressError`] if the input delay cannot be set
    /// (e.g., if the input queue is too small).
    ///
    /// # Example
    ///
    /// ```
    /// use fortress_rollback::{SessionBuilder, Config, FortressError};
    ///
    /// # struct MyConfig;
    /// # impl Config for MyConfig {
    /// #     type Input = u8;
    /// #     type State = ();
    /// #     type Address = std::net::SocketAddr;
    /// # }
    /// let builder = SessionBuilder::<MyConfig>::new()
    ///     .with_internet_defaults()?;
    /// # Ok::<(), FortressError>(())
    /// ```
    ///
    /// [`SyncConfig::default()`]: crate::SyncConfig::default
    /// [`ProtocolConfig::default()`]: crate::ProtocolConfig::default
    /// [`TimeSyncConfig::default()`]: crate::TimeSyncConfig::default
    pub fn with_internet_defaults(self) -> Result<Self, FortressError> {
        self.with_sync_config(SyncConfig::default())
            .with_protocol_config(ProtocolConfig::default())
            .with_time_sync_config(TimeSyncConfig::default())
            .with_input_delay(2)
    }

    /// Applies mobile/high-latency defaults for unstable connections.
    ///
    /// This preset configures the session for challenging network conditions
    /// typical of mobile/cellular networks or high-latency connections:
    /// - High RTT (100-300ms)
    /// - Variable latency (high jitter)
    /// - Intermittent packet loss (5-20%)
    /// - Connection handoffs (WiFi/cellular switches)
    ///
    /// # Configuration Applied
    ///
    /// - **Sync**: Robust handshake with more retries ([`SyncConfig::mobile()`])
    /// - **Protocol**: Tolerant settings with larger buffers ([`ProtocolConfig::mobile()`])
    /// - **Time Sync**: Large window for stable sync ([`TimeSyncConfig::mobile()`])
    /// - **Input Queue**: Larger queue for longer history ([`InputQueueConfig::high_latency()`])
    /// - **Input Delay**: 4 frames (helps absorb jitter spikes)
    ///
    /// # Errors
    ///
    /// Returns a [`FortressError`] if the input delay cannot be set.
    ///
    /// # Example
    ///
    /// ```
    /// use fortress_rollback::{SessionBuilder, Config, FortressError};
    ///
    /// # struct MyConfig;
    /// # impl Config for MyConfig {
    /// #     type Input = u8;
    /// #     type State = ();
    /// #     type Address = std::net::SocketAddr;
    /// # }
    /// let builder = SessionBuilder::<MyConfig>::new()
    ///     .with_high_latency_defaults()?;
    /// # Ok::<(), FortressError>(())
    /// ```
    ///
    /// [`SyncConfig::mobile()`]: crate::SyncConfig::mobile
    /// [`ProtocolConfig::mobile()`]: crate::ProtocolConfig::mobile
    /// [`TimeSyncConfig::mobile()`]: crate::TimeSyncConfig::mobile
    /// [`InputQueueConfig::high_latency()`]: crate::InputQueueConfig::high_latency
    pub fn with_high_latency_defaults(self) -> Result<Self, FortressError> {
        self.with_sync_config(SyncConfig::mobile())
            .with_protocol_config(ProtocolConfig::mobile())
            .with_time_sync_config(TimeSyncConfig::mobile())
            .with_input_queue_config(InputQueueConfig::high_latency())
            .with_input_delay(4)
    }

    fn validate_rollback_config(&self) -> Result<(), FortressError> {
        self.input_queue_config.validate()?;
        self.input_queue_config
            .validate_frame_delay(self.input_delay)?;
        self.protocol_config.validate()?;
        Ok(())
    }

    fn validate_spectator_config(&self) -> Result<(), FortressError> {
        self.protocol_config.validate()?;
        self.spectator_config.validate()
    }

    fn validate_synctest_config(&self) -> Result<(), FortressError> {
        self.input_queue_config.validate()?;
        self.input_queue_config
            .validate_frame_delay(self.input_delay)
    }

    /// Consumes the builder to construct a [`P2PSession`] and starts synchronization of endpoints.
    /// # Errors
    /// - Returns a [`FortressError`] if insufficient players have been registered.
    pub fn start_p2p_session(
        mut self,
        socket: impl NonBlockingSocket<T::Address> + 'static,
    ) -> Result<P2PSession<T>, FortressError> {
        self.validate_rollback_config()?;

        // Hot-join requires a non-zero prediction window. In lockstep mode
        // (`max_prediction == 0`) the host never saves state, so it can never
        // capture a snapshot to serve a joiner — the join could never complete.
        // Reject at build time (only when this host actually serves hot-joins)
        // rather than hang a joiner forever. See `with_hot_join` /
        // `start_hot_join_session`.
        #[cfg(feature = "hot-join")]
        if (self.accept_hot_join || !self.reserved_slots.is_empty()) && self.max_prediction == 0 {
            return Err(InvalidRequestKind::NotSupported {
                operation:
                    "hot-join host with max_prediction == 0 (lockstep); hot-join requires max_prediction >= 1",
            }
            .into());
        }

        // check if all players are added without iterating over the configured
        // player count, which may be intentionally huge.
        let registered_count = self
            .player_reg
            .handles
            .keys()
            .filter(|handle| handle.is_valid_player_for(self.num_players))
            .count();
        if registered_count < self.num_players {
            return Err(InvalidRequestKind::NotEnoughPlayers {
                expected: self.num_players,
                actual: registered_count,
            }
            .into());
        }

        // count the number of players per address
        let mut addr_count = BTreeMap::<PlayerType<T::Address>, Vec<PlayerHandle>>::new();
        for (handle, player_type) in self.player_reg.handles.iter() {
            match player_type {
                PlayerType::Remote(_) | PlayerType::Spectator(_) => addr_count
                    .entry(player_type.clone())
                    .or_insert_with(Vec::new)
                    .push(*handle),
                PlayerType::Local => (),
            }
        }

        // for each unique address, create an endpoint
        for (player_type, handles) in addr_count.into_iter() {
            match player_type {
                PlayerType::Remote(peer_addr) => {
                    // Propagate the original `create_endpoint` error verbatim so
                    // callers can distinguish IO (socket), protocol, and config
                    // failures (and `AllocationFailed`) instead of forcing every
                    // cause to a single opaque endpoint-creation error.
                    let endpoint =
                        self.create_endpoint(handles, peer_addr.clone(), self.local_players)?;
                    self.player_reg.remotes.insert(peer_addr, endpoint);
                },
                PlayerType::Spectator(peer_addr) => {
                    // the host of the spectator sends inputs for all players;
                    // propagate the original error verbatim (see above).
                    let endpoint =
                        self.create_endpoint(handles, peer_addr.clone(), self.num_players)?;
                    self.player_reg.spectators.insert(peer_addr, endpoint);
                },
                PlayerType::Local => (),
            }
        }

        #[cfg(feature = "hot-join")]
        let hot_join = crate::sessions::p2p_session::HotJoinConfig {
            reserved_slots: self.reserved_slots,
            accept_hot_join: self.accept_hot_join,
            joiner: None,
            serve_timeout_polls: self.hot_join_serve_timeout_polls,
            max_snapshot_wire_bytes: self.hot_join_max_snapshot_wire_bytes,
            ack_resends: self.hot_join_ack_resends,
        };

        P2PSession::<T>::new(
            self.num_players,
            self.max_prediction,
            Box::new(socket),
            self.player_reg,
            self.save_mode,
            self.desync_detection,
            self.input_delay,
            self.violation_observer,
            self.protocol_config,
            self.input_queue_config.queue_length,
            self.event_queue_size,
            self.recording,
            self.telemetry,
            self.disconnect_behavior,
            #[cfg(feature = "hot-join")]
            hot_join,
        )
    }

    /// Consumes the builder to construct a hot-joiner [`P2PSession`] that joins a
    /// running host's reserved slot.
    ///
    /// The joiner is built as a 2-peer-style session: the calling side is the
    /// **local** player and the `host` is its single **remote** at `host_addr`.
    /// Register exactly one local player (the slot being filled) and the host as
    /// a remote player before calling this. The session starts in
    /// [`SessionState::HotJoining`](crate::SessionState::HotJoining); it
    /// synchronizes with the host, requests a state snapshot, loads it (emitting a
    /// [`LoadGameState`](crate::FortressRequest::LoadGameState) on the first
    /// [`advance_frame`](P2PSession::advance_frame)), and only then transitions to
    /// [`Running`](crate::SessionState::Running).
    ///
    /// Hot-join uses **input delay 0** for the joining slot (the activation-frame
    /// model relies on the joiner contributing inputs for frames `>= F` with no
    /// extra delay). This method enforces that requirement and returns an error
    /// otherwise.
    ///
    /// This is feature-gated behind the `hot-join` feature.
    ///
    /// # Errors
    ///
    /// - Returns [`InvalidRequestKind::NotSupported`] if the configured input
    ///   delay is non-zero.
    /// - Returns the same player-count / configuration errors as
    ///   [`start_p2p_session`](Self::start_p2p_session).
    /// - Returns an error if exactly one local player is not registered, or if
    ///   the host endpoint cannot be created/synchronized.
    #[cfg(feature = "hot-join")]
    pub fn start_hot_join_session(
        mut self,
        socket: impl NonBlockingSocket<T::Address> + 'static,
        host_addr: T::Address,
    ) -> Result<P2PSession<T>, FortressError> {
        self.validate_rollback_config()?;

        // Hot-join requires input delay 0 (see method docs / activation-frame model).
        if self.input_delay != 0 {
            return Err(InvalidRequestKind::NotSupported {
                operation: "start_hot_join_session with non-zero input delay (hot-join requires input delay 0)",
            }
            .into());
        }

        // Hot-join requires a non-zero prediction window. In lockstep mode
        // (`max_prediction == 0`) the host never saves state and so can never
        // serve a snapshot; a joiner would hang in `HotJoining` forever. Reject
        // at build time on the joiner side too, mirroring the host-side guard in
        // `start_p2p_session`.
        if self.max_prediction == 0 {
            return Err(InvalidRequestKind::NotSupported {
                operation:
                    "start_hot_join_session with max_prediction == 0 (lockstep); hot-join requires max_prediction >= 1",
            }
            .into());
        }

        // The joiner must have exactly one local player (the slot it fills).
        let local_handle = self.player_reg.local_player_handle_required()?;

        // All slots must be registered, same as a normal P2P session.
        let registered_count = self
            .player_reg
            .handles
            .keys()
            .filter(|handle| handle.is_valid_player_for(self.num_players))
            .count();
        if registered_count < self.num_players {
            return Err(InvalidRequestKind::NotEnoughPlayers {
                expected: self.num_players,
                actual: registered_count,
            }
            .into());
        }

        // Build remote endpoints exactly like start_p2p_session: one per unique
        // remote address (the host plus any other remote slots in scope).
        let mut addr_count = BTreeMap::<PlayerType<T::Address>, Vec<PlayerHandle>>::new();
        for (handle, player_type) in self.player_reg.handles.iter() {
            match player_type {
                PlayerType::Remote(_) | PlayerType::Spectator(_) => addr_count
                    .entry(player_type.clone())
                    .or_insert_with(Vec::new)
                    .push(*handle),
                PlayerType::Local => (),
            }
        }
        for (player_type, handles) in addr_count.into_iter() {
            match player_type {
                PlayerType::Remote(peer_addr) => {
                    let mut endpoint =
                        self.create_endpoint(handles, peer_addr.clone(), self.local_players)?;
                    // Defer input processing until the snapshot is applied: the
                    // joiner must not ack the host's inputs before the activation
                    // frame is known (acking would let the host trim its
                    // pending_output below that frame). Cleared by the session
                    // once the snapshot is applied.
                    endpoint.set_defer_input_processing(true);
                    self.player_reg.remotes.insert(peer_addr, endpoint);
                },
                PlayerType::Spectator(peer_addr) => {
                    let endpoint =
                        self.create_endpoint(handles, peer_addr.clone(), self.num_players)?;
                    self.player_reg.spectators.insert(peer_addr, endpoint);
                },
                PlayerType::Local => (),
            }
        }

        let hot_join = crate::sessions::p2p_session::HotJoinConfig {
            reserved_slots: self.reserved_slots,
            // A joiner does not serve hot-joins.
            accept_hot_join: false,
            joiner: Some(crate::sessions::p2p_session::JoinerStateInit {
                local_handle,
                host_addr,
            }),
            serve_timeout_polls: self.hot_join_serve_timeout_polls,
            max_snapshot_wire_bytes: self.hot_join_max_snapshot_wire_bytes,
            ack_resends: self.hot_join_ack_resends,
        };

        P2PSession::<T>::new(
            self.num_players,
            self.max_prediction,
            Box::new(socket),
            self.player_reg,
            self.save_mode,
            self.desync_detection,
            self.input_delay,
            self.violation_observer,
            self.protocol_config,
            self.input_queue_config.queue_length,
            self.event_queue_size,
            self.recording,
            self.telemetry,
            self.disconnect_behavior,
            hot_join,
        )
    }

    /// Consumes the builder to create a new [`SpectatorSession`].
    /// A [`SpectatorSession`] provides all functionality to connect to a remote host in a peer-to-peer fashion.
    /// The host will broadcast all confirmed inputs to this session.
    /// This session can be used to spectate a session without contributing to the game input.
    ///
    /// For redundancy across multiple game peers, see
    /// [`start_spectator_session_multi`](Self::start_spectator_session_multi).
    ///
    /// # Returns
    /// Returns `None` if the protocol or spectator configuration is invalid,
    /// or if protocol initialization fails (e.g., due to serialization issues
    /// with the Input type).
    pub fn start_spectator_session(
        self,
        host_addr: T::Address,
        socket: impl NonBlockingSocket<T::Address> + 'static,
    ) -> Option<SpectatorSession<T>> {
        self.validate_spectator_config().ok()?;

        // create the single host endpoint and synchronize it
        let host = self.build_spectator_host(host_addr)?;

        SpectatorSession::new(
            self.num_players,
            Box::new(socket),
            vec![host],
            self.spectator_config.buffer_size,
            self.spectator_config.max_frames_behind,
            self.spectator_config.catchup_speed,
            self.spectator_config.stream_delay,
            self.spectator_config.enable_rewind,
            self.violation_observer,
            self.event_queue_size,
        )
        .ok()
    }

    /// Consumes the builder to create a redundant (failover) [`SpectatorSession`].
    ///
    /// Unlike [`start_spectator_session`](Self::start_spectator_session), this connects
    /// to **multiple** game peers ("hosts") at once. Unresolved frames use the
    /// highest-priority currently connected host by the order in `host_addrs` as
    /// the canonical source. Lower-priority host snapshots remain provisional
    /// while a higher-priority host is connected. If duplicate host addresses are
    /// provided, inbound packets are routed to the first matching host endpoint;
    /// later duplicates do not receive that packet.
    ///
    /// Connected redundant hosts that disagree for the same player/frame report a
    /// frame-sync violation, emit [`FortressEvent::SpectatorDivergence`], and
    /// make future [`advance_frame`](SpectatorSession::advance_frame) calls return
    /// [`FortressError::SpectatorDivergence`].
    ///
    /// The supplied addresses should be distinct game peers that are all spectating the
    /// same match (e.g. several players in the same P2P session who each registered this
    /// spectator). Spectation continues while at least one host remains connected. When a
    /// host disconnects it is removed and a [`FortressEvent::Disconnected`] event is
    /// emitted for it; observe [`SpectatorSession::num_hosts`] to track redundancy. When
    /// all hosts have dropped, already-buffered frames may still drain normally. Once no
    /// buffered frame is viewable, [`advance_frame`](SpectatorSession::advance_frame)
    /// returns [`FortressError::PredictionThreshold`].
    ///
    /// A [`SpectatorConfig::catchup_speed`] of `0` remains accepted for compatibility:
    /// if catch-up mode is triggered, no frame is attempted and
    /// [`advance_frame`](SpectatorSession::advance_frame) returns `Ok(<empty>)`.
    ///
    /// # Returns
    ///
    /// Returns `None` if `host_addrs` is empty, if the protocol or spectator
    /// configuration is invalid, or if protocol initialization fails for any address
    /// (e.g. due to serialization issues with the Input type).
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
    ///
    /// // An empty address list yields no session.
    /// let none = SessionBuilder::<TestConfig>::new()
    ///     .with_num_players(2)?
    ///     .start_spectator_session_multi(&[], DummySocket);
    /// assert!(none.is_none());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// [`FortressEvent::Disconnected`]: crate::FortressEvent::Disconnected
    /// [`FortressEvent::SpectatorDivergence`]: crate::FortressEvent::SpectatorDivergence
    /// [`FortressError::PredictionThreshold`]: crate::FortressError::PredictionThreshold
    /// [`SpectatorSession::num_hosts`]: crate::SpectatorSession::num_hosts
    pub fn start_spectator_session_multi(
        self,
        host_addrs: &[T::Address],
        socket: impl NonBlockingSocket<T::Address> + 'static,
    ) -> Option<SpectatorSession<T>> {
        // A failover spectator needs at least one host.
        if host_addrs.is_empty() {
            return None;
        }

        self.validate_spectator_config().ok()?;

        // Build and synchronize one host endpoint per address.
        let mut hosts = Vec::new();
        hosts.try_reserve_exact(host_addrs.len()).ok()?;
        for host_addr in host_addrs {
            hosts.push(self.build_spectator_host(host_addr.clone())?);
        }

        SpectatorSession::new(
            self.num_players,
            Box::new(socket),
            hosts,
            self.spectator_config.buffer_size,
            self.spectator_config.max_frames_behind,
            self.spectator_config.catchup_speed,
            self.spectator_config.stream_delay,
            self.spectator_config.enable_rewind,
            self.violation_observer,
            self.event_queue_size,
        )
        .ok()
    }

    /// Builds and synchronizes a single spectator host endpoint for `host_addr`.
    ///
    /// Returns `None` if protocol creation or synchronization fails. Shared by
    /// [`start_spectator_session`](Self::start_spectator_session) and
    /// [`start_spectator_session_multi`](Self::start_spectator_session_multi) to avoid
    /// duplicating the single-host construction logic.
    fn build_spectator_host(&self, host_addr: T::Address) -> Option<UdpProtocol<T>> {
        let mut handles = Vec::new();
        handles.try_reserve_exact(self.num_players).ok()?;
        for handle in (0..self.num_players).map(PlayerHandle::new) {
            handles.push(handle);
        }

        let mut host = UdpProtocol::new(
            handles,
            host_addr,
            self.num_players,
            1, //should not matter since the spectator is never sending
            self.max_prediction,
            self.disconnect_timeout,
            self.disconnect_notify_start,
            self.fps,
            DesyncDetection::Off,
            self.sync_config,
            self.protocol_config.clone(),
            self.time_sync_config,
        )
        .ok()?;
        host.synchronize().ok()?;
        Some(host)
    }

    /// Consumes the builder to construct a new [`SyncTestSession`]. During a [`SyncTestSession`], Fortress Rollback will simulate a rollback every frame
    /// and resimulate the last n states, where n is the given `check_distance`.
    /// The resimulated checksums will be compared with the original checksums and report if there was a mismatch.
    /// Due to the decentralized nature of saving and loading gamestates, checksum comparisons can only be made if `check_distance` is 2 or higher.
    /// This is a great way to test if your system runs deterministically.
    /// After creating the session, add a local player, set input delay for them and then start the session.
    pub fn start_synctest_session(self) -> Result<SyncTestSession<T>, FortressError> {
        self.validate_synctest_config()?;

        if self.check_dist >= self.max_prediction {
            return Err(InvalidRequestKind::CheckDistanceTooLarge {
                check_dist: self.check_dist,
                max_prediction: self.max_prediction,
            }
            .into());
        }

        SyncTestSession::try_with_queue_length(
            self.num_players,
            self.max_prediction,
            self.check_dist,
            self.input_delay,
            self.violation_observer,
            self.input_queue_config.queue_length,
        )
    }

    /// Creates a replay playback session from a recorded [`Replay`].
    ///
    /// The returned [`ReplaySession`] will play back the recorded inputs
    /// frame by frame when [`advance_frame`](crate::Session::advance_frame)
    /// is called. No network, save/load, or local input is needed.
    ///
    /// The builder is consumed but most configuration is ignored since
    /// replay playback does not require networking or synchronization.
    ///
    /// # Example
    ///
    /// ```
    /// # use fortress_rollback::prelude::*;
    /// # use fortress_rollback::replay::{Replay, ReplayMetadata};
    /// # use std::net::SocketAddr;
    /// # #[derive(Debug)]
    /// # struct TestConfig;
    /// # impl Config for TestConfig {
    /// #     type Input = u8;
    /// #     type State = u8;
    /// #     type Address = SocketAddr;
    /// # }
    /// let replay = Replay::<u8> {
    ///     num_players: 2,
    ///     frames: vec![vec![0, 0]; 10],
    ///     checksums: vec![None; 10],
    ///     metadata: ReplayMetadata {
    ///         library_version: env!("CARGO_PKG_VERSION").to_string(),
    ///         num_players: 2,
    ///         total_frames: 10,
    ///         skipped_frames: 0,
    ///     },
    /// };
    /// let session = SessionBuilder::<TestConfig>::new()
    ///     .start_replay_session(replay)?;
    /// assert!(!session.is_complete());
    /// # Ok::<(), fortress_rollback::FortressError>(())
    /// ```
    ///
    /// [`Replay`]: crate::replay::Replay
    /// [`ReplaySession`]: crate::sessions::replay_session::ReplaySession
    ///
    /// # Errors
    ///
    /// Returns an error if the replay fails internal consistency validation
    /// (see [`Replay::validate`]).
    pub fn start_replay_session(
        self,
        replay: Replay<T::Input>,
    ) -> crate::FortressResult<ReplaySession<T>> {
        ReplaySession::new(replay)
    }

    /// Creates a replay playback session with checksum validation enabled.
    ///
    /// When validation is enabled, the session emits [`FortressRequest::SaveGameState`]
    /// requests before each [`FortressRequest::AdvanceFrame`], allowing the application
    /// to compute checksums. These checksums are compared against the checksums stored
    /// in the replay to detect non-determinism.
    ///
    /// If a mismatch is detected, a [`FortressEvent::ReplayDesync`] event is emitted
    /// with the frame number and both checksums.
    ///
    /// # Example
    ///
    /// ```
    /// # use fortress_rollback::prelude::*;
    /// # use fortress_rollback::replay::{Replay, ReplayMetadata};
    /// # use std::net::SocketAddr;
    /// # #[derive(Debug)]
    /// # struct TestConfig;
    /// # impl Config for TestConfig {
    /// #     type Input = u8;
    /// #     type State = u8;
    /// #     type Address = SocketAddr;
    /// # }
    /// let replay = Replay::<u8> {
    ///     num_players: 2,
    ///     frames: vec![vec![0, 0]; 10],
    ///     checksums: vec![Some(0x1234); 10],
    ///     metadata: ReplayMetadata {
    ///         library_version: env!("CARGO_PKG_VERSION").to_string(),
    ///         num_players: 2,
    ///         total_frames: 10,
    ///         skipped_frames: 0,
    ///     },
    /// };
    /// let session = SessionBuilder::<TestConfig>::new()
    ///     .start_replay_session_with_validation(replay)?;
    /// assert!(!session.is_complete());
    /// # Ok::<(), fortress_rollback::FortressError>(())
    /// ```
    ///
    /// [`Replay`]: crate::replay::Replay
    /// [`ReplaySession`]: crate::sessions::replay_session::ReplaySession
    /// [`FortressRequest::SaveGameState`]: crate::FortressRequest::SaveGameState
    /// [`FortressRequest::AdvanceFrame`]: crate::FortressRequest::AdvanceFrame
    /// [`FortressEvent::ReplayDesync`]: crate::FortressEvent::ReplayDesync
    ///
    /// # Errors
    ///
    /// Returns an error if the replay fails internal consistency validation
    /// (see [`Replay::validate`]).
    pub fn start_replay_session_with_validation(
        self,
        replay: Replay<T::Input>,
    ) -> crate::FortressResult<ReplaySession<T>> {
        ReplaySession::new_with_validation(replay)
    }

    fn create_endpoint(
        &self,
        handles: Vec<PlayerHandle>,
        peer_addr: T::Address,
        local_players: usize,
    ) -> Result<UdpProtocol<T>, FortressError> {
        // create the endpoint, set parameters
        let mut endpoint = UdpProtocol::new(
            handles,
            peer_addr,
            self.num_players,
            local_players,
            self.max_prediction,
            self.disconnect_timeout,
            self.disconnect_notify_start,
            self.fps,
            self.desync_detection,
            self.sync_config,
            self.protocol_config.clone(),
            self.time_sync_config,
        )?;
        // start the synchronization
        endpoint.synchronize()?;
        Ok(endpoint)
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::net::SocketAddr;

    #[repr(C)]
    #[derive(Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
    struct TestInput {
        inp: u8,
    }

    struct TestConfig;

    impl Config for TestConfig {
        type Input = TestInput;
        type State = Vec<u8>;
        type Address = SocketAddr;
    }

    // ========================================================================
    // SessionBuilder SaveMode Integration Tests
    // ========================================================================

    #[test]
    fn test_with_save_mode_every_frame() {
        let builder = SessionBuilder::<TestConfig>::new().with_save_mode(SaveMode::EveryFrame);
        assert_eq!(builder.save_mode, SaveMode::EveryFrame);
    }

    #[test]
    fn test_with_save_mode_sparse() {
        let builder = SessionBuilder::<TestConfig>::new().with_save_mode(SaveMode::Sparse);
        assert_eq!(builder.save_mode, SaveMode::Sparse);
    }

    #[test]
    #[allow(deprecated)]
    fn test_deprecated_with_sparse_saving_mode_true() {
        let builder = SessionBuilder::<TestConfig>::new().with_sparse_saving_mode(true);
        assert_eq!(builder.save_mode, SaveMode::Sparse);
    }

    #[test]
    #[allow(deprecated)]
    fn test_deprecated_with_sparse_saving_mode_false() {
        let builder = SessionBuilder::<TestConfig>::new().with_sparse_saving_mode(false);
        assert_eq!(builder.save_mode, SaveMode::EveryFrame);
    }

    #[test]
    fn test_builder_default_save_mode() {
        let builder = SessionBuilder::<TestConfig>::new();
        assert_eq!(builder.save_mode, SaveMode::EveryFrame);
    }

    // ========================================================================
    // Input Delay Bounds Tests
    // These tests verify the fix for a Kani-discovered edge case where
    // frame_delay >= INPUT_QUEUE_LENGTH could cause circular buffer overflow.
    // ========================================================================

    #[test]
    fn test_with_input_delay_accepts_zero() {
        let builder = SessionBuilder::<TestConfig>::new()
            .with_input_delay(0)
            .expect("Zero delay should be valid");
        assert_eq!(builder.input_delay, 0);
    }

    #[test]
    fn test_with_input_delay_accepts_typical_values() {
        for delay in 1..=8 {
            let builder = SessionBuilder::<TestConfig>::new()
                .with_input_delay(delay)
                .expect("Typical delay values should be valid");
            assert_eq!(builder.input_delay, delay);
        }
    }

    #[test]
    fn test_with_input_delay_accepts_max_valid() {
        use crate::input_queue::INPUT_QUEUE_LENGTH;
        let max_delay = INPUT_QUEUE_LENGTH - 1;
        let builder = SessionBuilder::<TestConfig>::new()
            .with_input_delay(max_delay)
            .expect("Max delay should be valid");
        assert_eq!(builder.input_delay, max_delay);
    }

    #[test]
    fn test_with_input_delay_rejects_excessive_delay() {
        use crate::input_queue::INPUT_QUEUE_LENGTH;
        let result = SessionBuilder::<TestConfig>::new().with_input_delay(INPUT_QUEUE_LENGTH);
        // Excessive delay should return an error
        assert!(result.is_err());
    }

    #[test]
    fn test_with_input_delay_rejects_very_large_delay() {
        use crate::input_queue::INPUT_QUEUE_LENGTH;
        let result = SessionBuilder::<TestConfig>::new().with_input_delay(INPUT_QUEUE_LENGTH * 2);
        // Excessive delay should return an error
        assert!(result.is_err());
    }

    // ========================================================================
    // SessionBuilder Config Integration Tests
    // ========================================================================

    #[test]
    fn test_with_input_queue_config() {
        let builder = SessionBuilder::<TestConfig>::new()
            .with_input_queue_config(InputQueueConfig::minimal());
        assert_eq!(builder.input_queue_config.queue_length, 32);
    }

    #[test]
    fn test_input_queue_config_affects_max_delay() {
        // With minimal config (queue_length=32), max delay is 31
        let builder = SessionBuilder::<TestConfig>::new()
            .with_input_queue_config(InputQueueConfig::minimal())
            .with_input_delay(31)
            .expect("Delay of 31 should be valid with minimal config"); // Should succeed
        assert_eq!(builder.input_delay, 31);
    }

    #[test]
    fn test_input_queue_config_custom_queue_rejects_excessive_delay() {
        // With minimal config (queue_length=32), max delay is 31
        // Trying to set delay=32 should return an error
        let result = SessionBuilder::<TestConfig>::new()
            .with_input_queue_config(InputQueueConfig::minimal())
            .with_input_delay(32);
        assert!(result.is_err());
    }

    #[test]
    fn with_sync_config_applies_to_builder() {
        let builder =
            SessionBuilder::<TestConfig>::new().with_sync_config(SyncConfig::high_latency());
        assert_eq!(builder.sync_config, SyncConfig::high_latency());
    }

    #[test]
    fn with_protocol_config_applies_to_builder() {
        let builder =
            SessionBuilder::<TestConfig>::new().with_protocol_config(ProtocolConfig::competitive());
        assert_eq!(builder.protocol_config, ProtocolConfig::competitive());
    }

    // ========================================================================
    // Event Queue Size Tests
    // ========================================================================

    #[test]
    fn test_with_event_queue_size_rejects_too_small() {
        // Values less than 10 should be rejected
        let result = SessionBuilder::<TestConfig>::new().with_event_queue_size(9);
        assert!(result.is_err());
    }

    #[test]
    fn test_with_event_queue_size_rejects_zero() {
        let result = SessionBuilder::<TestConfig>::new().with_event_queue_size(0);
        assert!(result.is_err());
    }

    #[test]
    fn test_with_event_queue_size_accepts_minimum() {
        // Minimum value of 10 should be accepted
        let builder = SessionBuilder::<TestConfig>::new()
            .with_event_queue_size(10)
            .expect("Event queue size of 10 should be valid");
        assert_eq!(builder.event_queue_size, 10);
    }

    #[test]
    fn test_with_event_queue_size_accepts_valid_values() {
        // Test various valid values
        for size in [10, 50, 100, 200, 500, usize::MAX] {
            let builder = SessionBuilder::<TestConfig>::new()
                .with_event_queue_size(size)
                .expect("Valid event queue size should be accepted");
            assert_eq!(builder.event_queue_size, size);
        }
    }

    #[test]
    fn test_builder_default_event_queue_size() {
        // Default should be 100
        let builder = SessionBuilder::<TestConfig>::new();
        assert_eq!(builder.event_queue_size, DEFAULT_EVENT_QUEUE_SIZE);
        assert_eq!(builder.event_queue_size, 100);
    }

    #[cfg(feature = "hot-join")]
    #[test]
    fn with_hot_join_serve_timeout_polls_rejects_values_below_two() {
        for polls in [0, 1] {
            let result =
                SessionBuilder::<TestConfig>::new().with_hot_join_serve_timeout_polls(polls);
            assert!(matches!(
                result,
                Err(FortressError::InvalidRequestStructured {
                    kind: InvalidRequestKind::NotSupported { .. }
                })
            ));
        }
    }

    #[cfg(feature = "hot-join")]
    #[test]
    fn with_hot_join_serve_timeout_polls_accepts_two() {
        let builder = SessionBuilder::<TestConfig>::new()
            .with_hot_join_serve_timeout_polls(2)
            .expect("two polls leaves a serve open across calls");
        assert_eq!(builder.hot_join_serve_timeout_polls, 2);
    }

    #[cfg(feature = "hot-join")]
    #[test]
    fn with_hot_join_max_snapshot_wire_bytes_rejects_zero() {
        let result = SessionBuilder::<TestConfig>::new().with_hot_join_max_snapshot_wire_bytes(0);
        assert!(matches!(
            result,
            Err(FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::ConfigValueOutOfRange {
                    field: "hot_join_max_snapshot_wire_bytes",
                    min: 1,
                    actual: 0,
                    ..
                }
            })
        ));
    }

    #[cfg(feature = "hot-join")]
    #[test]
    fn with_hot_join_max_snapshot_wire_bytes_accepts_positive_value() {
        let builder = SessionBuilder::<TestConfig>::new()
            .with_hot_join_max_snapshot_wire_bytes(8192)
            .expect("positive snapshot wire cap is valid");
        assert_eq!(builder.hot_join_max_snapshot_wire_bytes, 8192);
    }

    // ========================================================================
    // Convenience Method Tests (add_local_player, add_remote_player)
    // ========================================================================

    fn test_addr(port: u16) -> SocketAddr {
        SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port)
    }

    struct DummySocket;

    impl NonBlockingSocket<SocketAddr> for DummySocket {
        fn send_to(&mut self, _msg: &crate::Message, _addr: &SocketAddr) {}

        fn receive_all_messages(&mut self) -> Vec<(SocketAddr, crate::Message)> {
            Vec::new()
        }
    }

    fn single_local_builder() -> SessionBuilder<TestConfig> {
        SessionBuilder::<TestConfig>::new()
            .with_num_players(1)
            .unwrap()
            .add_local_player(0)
            .unwrap()
    }

    fn assert_allocation_failed(err: FortressError, expected_context: &'static str) {
        match err {
            FortressError::InvalidRequestStructured {
                kind:
                    InvalidRequestKind::AllocationFailed {
                        context,
                        requested_elements,
                    },
            } => {
                assert_eq!(context, expected_context);
                assert!(requested_elements > 0);
            },
            other => panic!("expected allocation failure, got {other:?}"),
        }
    }

    #[test]
    fn with_num_players_accepts_large_user_configured_values() {
        let builder = SessionBuilder::<TestConfig>::new()
            .with_num_players(usize::MAX)
            .unwrap();

        assert_eq!(builder.num_players, usize::MAX);
    }

    #[test]
    fn start_p2p_session_reports_not_enough_players_for_huge_player_count_quickly() {
        let err = SessionBuilder::<TestConfig>::new()
            .with_num_players(usize::MAX)
            .unwrap()
            .add_local_player(0)
            .unwrap()
            .start_p2p_session(DummySocket)
            .unwrap_err();

        assert!(matches!(
            err,
            FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::NotEnoughPlayers {
                    expected: usize::MAX,
                    actual: 1,
                }
            }
        ));
    }

    #[test]
    fn legacy_spectator_setters_accept_large_user_configured_values() {
        let builder = SessionBuilder::<TestConfig>::new()
            .with_max_frames_behind(usize::MAX)
            .unwrap()
            .with_catchup_speed(usize::MAX)
            .unwrap();

        assert_eq!(builder.max_frames_behind, usize::MAX);
        assert_eq!(builder.catchup_speed, usize::MAX);
        assert_eq!(builder.spectator_config.max_frames_behind, usize::MAX);
        assert_eq!(builder.spectator_config.catchup_speed, usize::MAX);
    }

    #[test]
    fn legacy_spectator_setters_accept_zero_catchup_speed() {
        let builder = SessionBuilder::<TestConfig>::new()
            .with_catchup_speed(0)
            .unwrap();

        assert_eq!(builder.catchup_speed, 0);
        assert_eq!(builder.spectator_config.catchup_speed, 0);
    }

    #[test]
    fn start_p2p_session_reports_max_prediction_allocation_failure() {
        let err = single_local_builder()
            .with_max_prediction_window(usize::MAX)
            .start_p2p_session(DummySocket)
            .unwrap_err();

        assert_allocation_failed(err, "saved_states.states");
    }

    #[test]
    fn start_p2p_session_reports_time_sync_allocation_failure() {
        let err = SessionBuilder::<TestConfig>::new()
            .with_num_players(2)
            .unwrap()
            .with_time_sync_config(TimeSyncConfig {
                window_size: usize::MAX,
            })
            .add_local_player(0)
            .unwrap()
            .add_remote_player(1, test_addr(7_000))
            .unwrap()
            .start_p2p_session(DummySocket)
            .unwrap_err();

        assert_allocation_failed(err, "time_sync.local");
    }

    #[test]
    fn start_p2p_session_reports_input_queue_allocation_failure() {
        let err = single_local_builder()
            .with_input_queue_config(InputQueueConfig {
                queue_length: usize::MAX,
            })
            .start_p2p_session(DummySocket)
            .unwrap_err();

        assert_allocation_failed(err, "input_queue.inputs");
    }

    #[test]
    fn start_spectator_session_returns_none_when_buffer_reservation_fails() {
        let session = SessionBuilder::<TestConfig>::new()
            .with_num_players(2)
            .unwrap()
            .with_spectator_config(SpectatorConfig {
                buffer_size: usize::MAX,
                stream_delay: 0,
                ..SpectatorConfig::default()
            })
            .start_spectator_session(test_addr(7_500), DummySocket);

        assert!(session.is_none());
    }

    #[test]
    fn test_add_local_player_behaves_like_add_player_local() {
        // Arrange
        let builder_convenience = SessionBuilder::<TestConfig>::new()
            .with_num_players(2)
            .unwrap()
            .add_local_player(0)
            .expect("add_local_player should succeed");

        let builder_explicit = SessionBuilder::<TestConfig>::new()
            .with_num_players(2)
            .unwrap()
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .expect("add_player should succeed");

        // Assert - both builders should have the same player configuration
        assert_eq!(
            builder_convenience.local_players,
            builder_explicit.local_players
        );
        assert_eq!(
            builder_convenience.player_reg.handles.len(),
            builder_explicit.player_reg.handles.len()
        );
        assert!(builder_convenience
            .player_reg
            .handles
            .contains_key(&PlayerHandle::new(0)));
    }

    #[test]
    fn test_add_remote_player_behaves_like_add_player_remote() {
        // Arrange
        let addr = test_addr(7000);

        let builder_convenience = SessionBuilder::<TestConfig>::new()
            .with_num_players(2)
            .unwrap()
            .add_remote_player(0, addr)
            .expect("add_remote_player should succeed");

        let builder_explicit = SessionBuilder::<TestConfig>::new()
            .with_num_players(2)
            .unwrap()
            .add_player(PlayerType::Remote(addr), PlayerHandle::new(0))
            .expect("add_player should succeed");

        // Assert - both builders should have the same player configuration
        assert_eq!(
            builder_convenience.player_reg.handles.len(),
            builder_explicit.player_reg.handles.len()
        );
        assert!(builder_convenience
            .player_reg
            .handles
            .contains_key(&PlayerHandle::new(0)));
    }

    #[test]
    fn test_add_local_player_propagates_handle_in_use_error() {
        // Arrange - add a player at handle 0
        let builder = SessionBuilder::<TestConfig>::new()
            .with_num_players(2)
            .unwrap()
            .add_local_player(0)
            .expect("First add should succeed");

        // Act - try to add another player at the same handle
        let result = builder.add_local_player(0);

        // Assert - should return an error
        assert!(result.is_err());
    }

    #[test]
    fn test_add_local_player_propagates_invalid_handle_error() {
        // Arrange - create a builder with 2 players
        let builder = SessionBuilder::<TestConfig>::new()
            .with_num_players(2)
            .unwrap();

        // Act - try to add a player at an invalid handle (out of range)
        let result = builder.add_local_player(5);

        // Assert - should return an error
        assert!(result.is_err());
    }

    #[test]
    fn test_add_remote_player_propagates_handle_in_use_error() {
        // Arrange - add a player at handle 0
        let addr = test_addr(7000);
        let builder = SessionBuilder::<TestConfig>::new()
            .with_num_players(2)
            .unwrap()
            .add_remote_player(0, addr)
            .expect("First add should succeed");

        // Act - try to add another player at the same handle
        let result = builder.add_remote_player(0, addr);

        // Assert - should return an error
        assert!(result.is_err());
    }

    #[test]
    fn test_add_remote_player_propagates_invalid_handle_error() {
        // Arrange - create a builder with 2 players
        let addr = test_addr(7000);
        let builder = SessionBuilder::<TestConfig>::new()
            .with_num_players(2)
            .unwrap();

        // Act - try to add a player at an invalid handle (out of range)
        let result = builder.add_remote_player(5, addr);

        // Assert - should return an error
        assert!(result.is_err());
    }

    #[test]
    fn test_add_local_player_and_add_remote_player_can_be_combined() {
        // Arrange & Act - add local player then remote player
        let addr = test_addr(7000);
        let builder = SessionBuilder::<TestConfig>::new()
            .with_num_players(2)
            .unwrap()
            .add_local_player(0)
            .expect("add_local_player should succeed")
            .add_remote_player(1, addr)
            .expect("add_remote_player should succeed");

        // Assert
        assert_eq!(builder.local_players, 1);
        assert_eq!(builder.player_reg.handles.len(), 2);
        assert!(builder
            .player_reg
            .handles
            .contains_key(&PlayerHandle::new(0)));
        assert!(builder
            .player_reg
            .handles
            .contains_key(&PlayerHandle::new(1)));
    }

    // ========================================================================
    // Session Preset Tests
    // ========================================================================

    #[test]
    fn with_lan_defaults_returns_valid_builder() {
        // Arrange & Act: Create builder with LAN preset
        let builder = SessionBuilder::<TestConfig>::new()
            .with_num_players(2)
            .unwrap()
            .with_lan_defaults();

        // Assert: Builder is in a valid state and can accept players
        let result = builder.add_local_player(0);
        assert!(result.is_ok());
    }

    #[test]
    fn with_internet_defaults_returns_valid_builder() {
        // Arrange & Act: Create builder with internet preset
        let builder = SessionBuilder::<TestConfig>::new()
            .with_num_players(2)
            .unwrap()
            .with_internet_defaults()
            .expect("with_internet_defaults should succeed");

        // Assert: Builder is in a valid state and can accept players
        let result = builder.add_local_player(0);
        assert!(result.is_ok());
    }

    #[test]
    fn with_high_latency_defaults_returns_valid_builder() {
        // Arrange & Act: Create builder with high-latency preset
        let builder = SessionBuilder::<TestConfig>::new()
            .with_num_players(2)
            .unwrap()
            .with_high_latency_defaults()
            .expect("with_high_latency_defaults should succeed");

        // Assert: Builder is in a valid state and can accept players
        let result = builder.add_local_player(0);
        assert!(result.is_ok());
    }

    #[test]
    fn with_lan_defaults_applies_expected_configs() {
        // Arrange & Act
        let builder = SessionBuilder::<TestConfig>::new().with_lan_defaults();

        // Assert: Verify the preset applied the expected configuration
        assert_eq!(builder.sync_config, SyncConfig::lan());
        assert_eq!(builder.protocol_config, ProtocolConfig::competitive());
        assert_eq!(builder.time_sync_config, TimeSyncConfig::lan());
    }

    #[test]
    fn with_internet_defaults_applies_expected_configs() {
        // Arrange & Act
        let builder = SessionBuilder::<TestConfig>::new()
            .with_internet_defaults()
            .expect("with_internet_defaults should succeed");

        // Assert: Verify the preset applied the expected configuration
        assert_eq!(builder.sync_config, SyncConfig::default());
        assert_eq!(builder.protocol_config, ProtocolConfig::default());
        assert_eq!(builder.time_sync_config, TimeSyncConfig::default());
        assert_eq!(builder.input_delay, 2);
    }

    #[test]
    fn with_high_latency_defaults_applies_expected_configs() {
        // Arrange & Act
        let builder = SessionBuilder::<TestConfig>::new()
            .with_high_latency_defaults()
            .expect("with_high_latency_defaults should succeed");

        // Assert: Verify the preset applied the expected configuration
        assert_eq!(builder.sync_config, SyncConfig::mobile());
        assert_eq!(builder.protocol_config, ProtocolConfig::mobile());
        assert_eq!(builder.time_sync_config, TimeSyncConfig::mobile());
        assert_eq!(builder.input_queue_config, InputQueueConfig::high_latency());
        assert_eq!(builder.input_delay, 4);
    }

    #[test]
    fn presets_are_chainable_with_other_methods() {
        // Arrange & Act: Chain preset with additional configuration
        let builder = SessionBuilder::<TestConfig>::new()
            .with_num_players(4)
            .unwrap()
            .with_lan_defaults()
            .with_max_prediction_window(6)
            .with_desync_detection_mode(DesyncDetection::Off);

        // Assert: Both preset and subsequent configs applied
        assert_eq!(builder.sync_config, SyncConfig::lan());
        assert_eq!(builder.max_prediction, 6);
        assert_eq!(builder.desync_detection, DesyncDetection::Off);
    }

    #[test]
    fn builder_start_replay_session_with_validation() {
        use crate::replay::{Replay, ReplayMetadata};

        let replay = Replay::<TestInput> {
            num_players: 2,
            frames: vec![
                vec![TestInput { inp: 0 }, TestInput { inp: 1 }],
                vec![TestInput { inp: 2 }, TestInput { inp: 3 }],
            ],
            checksums: vec![Some(0x1234), Some(0x5678)],
            metadata: ReplayMetadata {
                library_version: "test".to_string(),
                num_players: 2,
                total_frames: 2,
                skipped_frames: 0,
            },
        };

        let mut session = SessionBuilder::<TestConfig>::new()
            .start_replay_session_with_validation(replay)
            .unwrap();

        assert!(!session.is_complete());

        // Validation mode should emit SaveGameState before AdvanceFrame
        let requests = session.advance_frame().unwrap();
        assert_eq!(requests.len(), 2);
        assert!(matches!(
            &requests[0],
            crate::FortressRequest::SaveGameState { .. }
        ));
        assert!(matches!(
            &requests[1],
            crate::FortressRequest::AdvanceFrame { .. }
        ));
    }
}
