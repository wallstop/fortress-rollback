use std::collections::BTreeMap;
use std::sync::Arc;

use web_time::Duration;

use crate::{
    error::{InvalidRequestKind, SerializationErrorKind},
    network::protocol::UdpProtocol,
    sessions::player_registry::PlayerRegistry,
    telemetry::ViolationObserver,
    time_sync::TimeSyncConfig,
    Config, DesyncDetection, FortressError, NonBlockingSocket, P2PSession, PlayerHandle,
    PlayerType, SpectatorSession, SyncTestSession,
};

// Re-export config types for backwards compatibility with code that imports from builder
pub use crate::sessions::config::{
    InputQueueConfig, ProtocolConfig, SaveMode, SpectatorConfig, SyncConfig,
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
const DEFAULT_DISCONNECT_TIMEOUT: Duration = Duration::from_millis(2000);
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
        } = self;

        f.debug_struct("SessionBuilder")
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
            .field("sync_config", sync_config)
            .field("protocol_config", protocol_config)
            .field("spectator_config", spectator_config)
            .field("time_sync_config", time_sync_config)
            .field("input_queue_config", input_queue_config)
            .field("event_queue_size", event_queue_size)
            .finish()
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
        }
    }

    /// Must be called for each player in the session (e.g. in a 3 player session, must be called 3 times) before starting the session.
    /// Player handles for players should be between 0 and `num_players`, spectator handles should be higher than `num_players`.
    /// Later, you will need the player handle to add input, change parameters or disconnect the player or spectator.
    ///
    /// # Errors
    /// - Returns [`InvalidRequest`] if a player with that handle has been added before
    /// - Returns [`InvalidRequest`] if the handle is invalid for the given [`PlayerType`]
    ///
    /// [`InvalidRequest`]: FortressError::InvalidRequest
    /// [`num_players`]: Self#structfield.num_players
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
    /// Returns [`FortressError::InvalidRequest`] if `delay` exceeds the maximum allowed value.
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
    /// Returns [`FortressError::InvalidRequest`] if `num_players` is 0.
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
        since = "0.12.0",
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
    /// Returns [`FortressError::InvalidRequest`] if `size` is less than 10.
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

    /// Sets the FPS this session is used with. This influences estimations for frame synchronization between sessions.
    /// # Errors
    /// - Returns [`InvalidRequest`] if the fps is 0
    ///
    /// [`InvalidRequest`]: FortressError::InvalidRequest
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
        if max_frames_behind < 1 || max_frames_behind >= self.spectator_config.buffer_size {
            return Err(InvalidRequestKind::MaxFramesBehindInvalid {
                value: max_frames_behind,
                buffer_size: self.spectator_config.buffer_size,
            }
            .into());
        }
        self.max_frames_behind = max_frames_behind;
        self.spectator_config.max_frames_behind = max_frames_behind;
        Ok(self)
    }

    /// Sets the catchup speed. Per default, this is set to 1, so the spectator never catches up.
    /// If you want the spectator to catch up to the host if `max_frames_behind` is surpassed, set this to a value higher than 1.
    ///
    /// Note: Prefer using [`Self::with_spectator_config`] for configuring spectator behavior.
    pub fn with_catchup_speed(mut self, catchup_speed: usize) -> Result<Self, FortressError> {
        if catchup_speed < 1 || catchup_speed >= self.spectator_config.max_frames_behind {
            return Err(InvalidRequestKind::CatchupSpeedInvalid {
                speed: catchup_speed,
                max_frames_behind: self.spectator_config.max_frames_behind,
            }
            .into());
        }
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

    /// Consumes the builder to construct a [`P2PSession`] and starts synchronization of endpoints.
    /// # Errors
    /// - Returns [`InvalidRequest`] if insufficient players have been registered.
    ///
    /// [`InvalidRequest`]: FortressError::InvalidRequest
    pub fn start_p2p_session(
        mut self,
        socket: impl NonBlockingSocket<T::Address> + 'static,
    ) -> Result<P2PSession<T>, FortressError> {
        // check if all players are added
        let registered_count = (0..self.num_players)
            .filter(|&i| self.player_reg.handles.contains_key(&PlayerHandle::new(i)))
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
                    let endpoint = self
                        .create_endpoint(handles, peer_addr.clone(), self.local_players)
                        .ok_or(SerializationErrorKind::EndpointCreationFailed)?;
                    self.player_reg.remotes.insert(peer_addr, endpoint);
                },
                PlayerType::Spectator(peer_addr) => {
                    let endpoint = self
                        .create_endpoint(handles, peer_addr.clone(), self.num_players) // the host of the spectator sends inputs for all players
                        .ok_or(SerializationErrorKind::SpectatorEndpointCreationFailed)?;
                    self.player_reg.spectators.insert(peer_addr, endpoint);
                },
                PlayerType::Local => (),
            }
        }

        // Validate configurations
        self.protocol_config.validate()?;
        self.input_queue_config.validate()?;
        self.input_queue_config
            .validate_frame_delay(self.input_delay)?;

        Ok(P2PSession::<T>::new(
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
        ))
    }

    /// Consumes the builder to create a new [`SpectatorSession`].
    /// A [`SpectatorSession`] provides all functionality to connect to a remote host in a peer-to-peer fashion.
    /// The host will broadcast all confirmed inputs to this session.
    /// This session can be used to spectate a session without contributing to the game input.
    ///
    /// # Returns
    /// Returns `None` if the protocol initialization fails (e.g., due to serialization issues with the Input type).
    pub fn start_spectator_session(
        self,
        host_addr: T::Address,
        socket: impl NonBlockingSocket<T::Address> + 'static,
    ) -> Option<SpectatorSession<T>> {
        // Validate the protocol configuration
        self.protocol_config.validate().ok()?;

        // create host endpoint
        let mut host = UdpProtocol::new(
            (0..self.num_players).map(PlayerHandle::new).collect(),
            host_addr,
            self.num_players,
            1, //should not matter since the spectator is never sending
            self.max_prediction,
            self.disconnect_timeout,
            self.disconnect_notify_start,
            self.fps,
            DesyncDetection::Off,
            self.sync_config,
            self.protocol_config,
        )?;
        host.synchronize().ok()?;
        Some(SpectatorSession::new(
            self.num_players,
            Box::new(socket),
            host,
            self.spectator_config.buffer_size,
            self.spectator_config.max_frames_behind,
            self.spectator_config.catchup_speed,
            self.violation_observer,
            self.event_queue_size,
        ))
    }

    /// Consumes the builder to construct a new [`SyncTestSession`]. During a [`SyncTestSession`], Fortress Rollback will simulate a rollback every frame
    /// and resimulate the last n states, where n is the given `check_distance`.
    /// The resimulated checksums will be compared with the original checksums and report if there was a mismatch.
    /// Due to the decentralized nature of saving and loading gamestates, checksum comparisons can only be made if `check_distance` is 2 or higher.
    /// This is a great way to test if your system runs deterministically.
    /// After creating the session, add a local player, set input delay for them and then start the session.
    pub fn start_synctest_session(self) -> Result<SyncTestSession<T>, FortressError> {
        if self.check_dist >= self.max_prediction {
            return Err(InvalidRequestKind::CheckDistanceTooLarge {
                check_dist: self.check_dist,
                max_prediction: self.max_prediction,
            }
            .into());
        }

        // Validate the input queue configuration
        self.input_queue_config.validate()?;
        self.input_queue_config
            .validate_frame_delay(self.input_delay)?;

        Ok(SyncTestSession::with_queue_length(
            self.num_players,
            self.max_prediction,
            self.check_dist,
            self.input_delay,
            self.violation_observer,
            self.input_queue_config.queue_length,
        ))
    }

    fn create_endpoint(
        &self,
        handles: Vec<PlayerHandle>,
        peer_addr: T::Address,
        local_players: usize,
    ) -> Option<UdpProtocol<T>> {
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
            self.protocol_config,
        )?;
        // start the synchronization
        endpoint.synchronize().ok()?;
        Some(endpoint)
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
    #[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
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
        for size in [10, 50, 100, 200, 500] {
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
}
