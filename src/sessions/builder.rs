use std::collections::BTreeMap;
use std::sync::Arc;

use instant::Duration;

use crate::{
    input_queue::MAX_FRAME_DELAY, network::protocol::UdpProtocol,
    sessions::p2p_session::PlayerRegistry, telemetry::ViolationObserver, time_sync::TimeSyncConfig,
    Config, DesyncDetection, FortressError, NonBlockingSocket, P2PSession, PlayerHandle,
    PlayerType, SpectatorSession, SyncTestSession,
};

/// Configuration for the synchronization protocol.
///
/// This struct allows fine-tuning the sync handshake behavior for different
/// network conditions. The defaults work well for typical networks with <15%
/// packet loss and <100ms RTT.
///
/// # Example
///
/// ```
/// use fortress_rollback::SyncConfig;
/// use instant::Duration;
///
/// // For high-latency networks, increase retry intervals
/// let high_latency_config = SyncConfig {
///     sync_retry_interval: Duration::from_millis(500),
///     running_retry_interval: Duration::from_millis(500),
///     keepalive_interval: Duration::from_millis(500),
///     ..SyncConfig::default()
/// };
///
/// // For lossy networks, increase required roundtrips
/// let lossy_config = SyncConfig {
///     num_sync_packets: 8,
///     ..SyncConfig::default()
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyncConfig {
    /// Number of successful sync roundtrips required before considering
    /// the connection synchronized. Higher values provide more confidence
    /// but take longer to synchronize.
    ///
    /// Default: 5
    pub num_sync_packets: u32,

    /// Time between sync request retries during the synchronization phase.
    /// If a sync request doesn't receive a reply within this interval,
    /// another request is sent.
    ///
    /// Default: 200ms
    pub sync_retry_interval: Duration,

    /// Maximum time to wait for synchronization to complete. If sync takes
    /// longer than this, a `SyncTimeout` event is emitted.
    ///
    /// Default: `None` (no timeout)
    pub sync_timeout: Option<Duration>,

    /// Time between input retries during the running phase. If we haven't
    /// received an ack for our inputs within this interval, resend them.
    ///
    /// Default: 200ms
    pub running_retry_interval: Duration,

    /// Time between keepalive packets when idle. Keepalives prevent
    /// disconnect timeouts during periods of no input.
    ///
    /// Default: 200ms
    pub keepalive_interval: Duration,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            num_sync_packets: 5,
            sync_retry_interval: Duration::from_millis(200),
            sync_timeout: None,
            running_retry_interval: Duration::from_millis(200),
            keepalive_interval: Duration::from_millis(200),
        }
    }
}

impl SyncConfig {
    /// Creates a new `SyncConfig` with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Configuration preset for high-latency networks (100-200ms RTT).
    ///
    /// Uses longer intervals to avoid flooding the network with retries.
    pub fn high_latency() -> Self {
        Self {
            num_sync_packets: 5,
            sync_retry_interval: Duration::from_millis(400),
            sync_timeout: Some(Duration::from_secs(10)),
            running_retry_interval: Duration::from_millis(400),
            keepalive_interval: Duration::from_millis(400),
        }
    }

    /// Configuration preset for lossy networks (5-15% packet loss).
    ///
    /// Uses more sync packets for higher confidence and a sync timeout.
    pub fn lossy() -> Self {
        Self {
            num_sync_packets: 8,
            sync_retry_interval: Duration::from_millis(200),
            sync_timeout: Some(Duration::from_secs(10)),
            running_retry_interval: Duration::from_millis(200),
            keepalive_interval: Duration::from_millis(200),
        }
    }

    /// Configuration preset for local network / LAN play.
    ///
    /// Uses shorter intervals and fewer sync packets for faster connection.
    pub fn lan() -> Self {
        Self {
            num_sync_packets: 3,
            sync_retry_interval: Duration::from_millis(100),
            sync_timeout: Some(Duration::from_secs(5)),
            running_retry_interval: Duration::from_millis(100),
            keepalive_interval: Duration::from_millis(100),
        }
    }
}

/// Configuration for network protocol behavior.
///
/// These settings control network timing, buffering, and telemetry thresholds.
/// The defaults work well for most scenarios; adjust for specific requirements.
///
/// # Example
///
/// ```
/// use fortress_rollback::ProtocolConfig;
/// use instant::Duration;
///
/// // For competitive/LAN play, use faster quality reports
/// let competitive_config = ProtocolConfig {
///     quality_report_interval: Duration::from_millis(100),
///     shutdown_delay: Duration::from_millis(3000),
///     ..ProtocolConfig::default()
/// };
///
/// // For debugging, use longer timeouts and lower thresholds
/// let debug_config = ProtocolConfig {
///     shutdown_delay: Duration::from_millis(10000),
///     sync_retry_warning_threshold: 5,
///     sync_duration_warning_ms: 1000,
///     ..ProtocolConfig::default()
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtocolConfig {
    /// Interval between network quality reports.
    ///
    /// Lower values provide more responsive network stats but increase bandwidth
    /// usage slightly. The quality report is a small packet that measures RTT.
    ///
    /// Default: 200ms
    pub quality_report_interval: Duration,

    /// Time to wait in Disconnected state before transitioning to Shutdown.
    ///
    /// This delay allows for graceful cleanup and final message delivery.
    /// After this timeout, the protocol will no longer process messages.
    ///
    /// Default: 5000ms
    pub shutdown_delay: Duration,

    /// Number of checksums to retain for desync detection history.
    ///
    /// Higher values can detect older desyncs but use more memory.
    /// Only relevant when desync detection is enabled.
    ///
    /// Default: 32
    pub max_checksum_history: usize,

    /// Maximum pending output messages before warning.
    ///
    /// When pending outputs exceed this limit, it indicates the peer
    /// isn't acknowledging inputs quickly enough. This may suggest
    /// network congestion or peer disconnection.
    ///
    /// Default: 128
    pub pending_output_limit: usize,

    /// Threshold for emitting sync retry warnings.
    ///
    /// Emits a telemetry warning when sync requests exceed this number.
    /// With 5 required roundtrips and 200ms retry interval, this threshold
    /// represents roughly 50% sustained packet loss over multiple retries.
    ///
    /// Default: 10
    pub sync_retry_warning_threshold: u32,

    /// Threshold for emitting sync duration warnings in milliseconds.
    ///
    /// Emits a telemetry warning when synchronization takes longer than this.
    /// Typical sync should complete in ~1 second for good connections.
    ///
    /// Default: 3000ms
    pub sync_duration_warning_ms: u128,
}

impl Default for ProtocolConfig {
    fn default() -> Self {
        Self {
            quality_report_interval: Duration::from_millis(200),
            shutdown_delay: Duration::from_millis(5000),
            max_checksum_history: 32,
            pending_output_limit: 128,
            sync_retry_warning_threshold: 10,
            sync_duration_warning_ms: 3000,
        }
    }
}

impl ProtocolConfig {
    /// Creates a new `ProtocolConfig` with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Configuration preset for competitive/LAN play.
    ///
    /// Uses faster quality reports and shorter shutdown delay for
    /// more responsive network stats and quicker cleanup.
    pub fn competitive() -> Self {
        Self {
            quality_report_interval: Duration::from_millis(100),
            shutdown_delay: Duration::from_millis(3000),
            max_checksum_history: 32,
            pending_output_limit: 128,
            sync_retry_warning_threshold: 10,
            sync_duration_warning_ms: 2000,
        }
    }

    /// Configuration preset for high-latency WAN connections.
    ///
    /// Uses longer intervals and more tolerant thresholds to reduce
    /// unnecessary warnings on slower connections.
    pub fn high_latency() -> Self {
        Self {
            quality_report_interval: Duration::from_millis(400),
            shutdown_delay: Duration::from_millis(10000),
            max_checksum_history: 64,
            pending_output_limit: 256,
            sync_retry_warning_threshold: 20,
            sync_duration_warning_ms: 10000,
        }
    }

    /// Configuration preset for debugging.
    ///
    /// Uses longer timeouts and lower warning thresholds to make
    /// it easier to observe telemetry events during development.
    pub fn debug() -> Self {
        Self {
            quality_report_interval: Duration::from_millis(500),
            shutdown_delay: Duration::from_millis(30000),
            max_checksum_history: 128,
            pending_output_limit: 64,
            sync_retry_warning_threshold: 5,
            sync_duration_warning_ms: 1000,
        }
    }
}

/// Configuration for spectator sessions.
///
/// These settings control spectator behavior including buffer sizes,
/// catch-up speed, and frame lag tolerance.
///
/// # Example
///
/// ```
/// use fortress_rollback::SpectatorConfig;
///
/// // For watching a fast-paced game, use larger buffer and faster catchup
/// let fast_game_config = SpectatorConfig {
///     buffer_size: 90,
///     catchup_speed: 2,
///     max_frames_behind: 15,
/// };
///
/// // For spectators on slower connections
/// let slow_connection_config = SpectatorConfig {
///     buffer_size: 120,
///     max_frames_behind: 20,
///     ..SpectatorConfig::default()
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpectatorConfig {
    /// The number of frames of input that the spectator can buffer.
    /// This defines how many frames of inputs from the host the spectator
    /// can store before older inputs are overwritten.
    ///
    /// A larger buffer allows the spectator to tolerate more latency
    /// or jitter, but uses more memory.
    ///
    /// Default: 60 (1 second at 60 FPS)
    pub buffer_size: usize,

    /// How many frames to advance per step when the spectator is behind.
    /// When the spectator falls more than `max_frames_behind` frames behind
    /// the host, it will advance this many frames per step to catch up.
    ///
    /// Higher values catch up faster but may cause visual stuttering.
    ///
    /// Default: 1
    pub catchup_speed: usize,

    /// The maximum number of frames the spectator can fall behind before
    /// triggering catch-up mode. When the spectator is more than this many
    /// frames behind the host's current frame, it will use `catchup_speed`
    /// to advance faster.
    ///
    /// Default: 10
    pub max_frames_behind: usize,
}

impl Default for SpectatorConfig {
    fn default() -> Self {
        Self {
            buffer_size: 60,
            catchup_speed: 1,
            max_frames_behind: 10,
        }
    }
}

impl SpectatorConfig {
    /// Creates a new `SpectatorConfig` with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Configuration preset for fast-paced games.
    ///
    /// Uses a larger buffer and faster catch-up for games where
    /// falling behind is more noticeable.
    pub fn fast_paced() -> Self {
        Self {
            buffer_size: 90,
            catchup_speed: 2,
            max_frames_behind: 15,
        }
    }

    /// Configuration preset for spectators on slower connections.
    ///
    /// Uses a larger buffer and more tolerance for falling behind.
    pub fn slow_connection() -> Self {
        Self {
            buffer_size: 120,
            catchup_speed: 1,
            max_frames_behind: 20,
        }
    }

    /// Configuration preset for local viewing with minimal latency.
    ///
    /// Uses smaller buffer and stricter catch-up for responsive viewing.
    pub fn local() -> Self {
        Self {
            buffer_size: 30,
            catchup_speed: 2,
            max_frames_behind: 5,
        }
    }
}

const DEFAULT_PLAYERS: usize = 2;
const DEFAULT_SAVE_MODE: bool = false;
const DEFAULT_DETECTION_MODE: DesyncDetection = DesyncDetection::Off;
const DEFAULT_INPUT_DELAY: usize = 0;
const DEFAULT_DISCONNECT_TIMEOUT: Duration = Duration::from_millis(2000);
const DEFAULT_DISCONNECT_NOTIFY_START: Duration = Duration::from_millis(500);
const DEFAULT_FPS: usize = 60;
const DEFAULT_MAX_PREDICTION_FRAMES: usize = 8;
const DEFAULT_CHECK_DISTANCE: usize = 2;
// If the spectator is more than this amount of frames behind, it will advance the game two steps at a time to catch up
const DEFAULT_MAX_FRAMES_BEHIND: usize = 10;
// The amount of frames the spectator advances in a single step if too far behind
const DEFAULT_CATCHUP_SPEED: usize = 1;
// The amount of events a spectator can buffer; should never be an issue if the user polls the events at every step
pub(crate) const MAX_EVENT_QUEUE_SIZE: usize = 100;

/// The [`SessionBuilder`] builds all Fortress Rollback Sessions. After setting all appropriate values, use `SessionBuilder::start_yxz_session(...)`
/// to consume the builder and create a Session of desired type.
pub struct SessionBuilder<T>
where
    T: Config,
{
    num_players: usize,
    local_players: usize,
    max_prediction: usize,
    /// FPS defines the expected update frequency of this session.
    fps: usize,
    sparse_saving: bool,
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
}

impl<T: Config> std::fmt::Debug for SessionBuilder<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionBuilder")
            .field("num_players", &self.num_players)
            .field("local_players", &self.local_players)
            .field("max_prediction", &self.max_prediction)
            .field("fps", &self.fps)
            .field("sparse_saving", &self.sparse_saving)
            .field("desync_detection", &self.desync_detection)
            .field("disconnect_timeout", &self.disconnect_timeout)
            .field("disconnect_notify_start", &self.disconnect_notify_start)
            .field("player_reg", &self.player_reg)
            .field("input_delay", &self.input_delay)
            .field("check_dist", &self.check_dist)
            .field("max_frames_behind", &self.max_frames_behind)
            .field("catchup_speed", &self.catchup_speed)
            .field("has_violation_observer", &self.violation_observer.is_some())
            .field("sync_config", &self.sync_config)
            .field("protocol_config", &self.protocol_config)
            .field("spectator_config", &self.spectator_config)
            .field("time_sync_config", &self.time_sync_config)
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
            sparse_saving: DEFAULT_SAVE_MODE,
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
            return Err(FortressError::InvalidRequest {
                info: "Player handle already in use.".to_owned(),
            });
        }
        // check if the player handle is valid for the given player type
        match player_type {
            PlayerType::Local => {
                self.local_players += 1;
                if !player_handle.is_valid_player_for(self.num_players) {
                    return Err(FortressError::InvalidRequest {
                        info: "The player handle you provided is invalid. For a local player, the handle should be between 0 and num_players".to_owned(),
                    });
                }
            }
            PlayerType::Remote(_) => {
                if !player_handle.is_valid_player_for(self.num_players) {
                    return Err(FortressError::InvalidRequest {
                        info: "The player handle you provided is invalid. For a remote player, the handle should be between 0 and num_players".to_owned(),
                    });
                }
            }
            PlayerType::Spectator(_) => {
                if !player_handle.is_spectator_for(self.num_players) {
                    return Err(FortressError::InvalidRequest {
                        info: "The player handle you provided is invalid. For a spectator, the handle should be num_players or higher".to_owned(),
                    });
                }
            }
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
    /// # Panics
    /// Panics if `delay` exceeds `MAX_FRAME_DELAY` (127 in production). This limit ensures
    /// the circular input buffer doesn't overflow. At 60fps, 127 frames = ~2.1 seconds,
    /// far exceeding any practical input delay (typically 0-8 frames).
    ///
    /// This constraint was discovered through Kani formal verification.
    pub fn with_input_delay(mut self, delay: usize) -> Self {
        assert!(
            delay <= MAX_FRAME_DELAY,
            "Input delay {} exceeds maximum allowed value of {}. \
             At 60fps, this would be {:.1}+ seconds of delay, which is impractical for gameplay.",
            delay,
            MAX_FRAME_DELAY,
            delay as f64 / 60.0
        );
        self.input_delay = delay;
        self
    }

    /// Change number of total players. Default is 2.
    pub fn with_num_players(mut self, num_players: usize) -> Self {
        self.num_players = num_players;
        self
    }

    /// Sets the sparse saving mode. With sparse saving turned on, only the minimum confirmed frame
    /// (for which all inputs from all players are confirmed correct) will be saved. This leads to
    /// much less save requests at the cost of potentially longer rollbacks and thus more advance
    /// frame requests. Recommended, if saving your gamestate takes much more time than advancing
    /// the game state.
    pub fn with_sparse_saving_mode(mut self, sparse_saving: bool) -> Self {
        self.sparse_saving = sparse_saving;
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
    ///     quality_report_interval: instant::Duration::from_millis(100),
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

    /// Sets the FPS this session is used with. This influences estimations for frame synchronization between sessions.
    /// # Errors
    /// - Returns [`InvalidRequest`] if the fps is 0
    ///
    /// [`InvalidRequest`]: FortressError::InvalidRequest
    pub fn with_fps(mut self, fps: usize) -> Result<Self, FortressError> {
        if fps == 0 {
            return Err(FortressError::InvalidRequest {
                info: "FPS should be higher than 0.".to_owned(),
            });
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
        if max_frames_behind < 1 {
            return Err(FortressError::InvalidRequest {
                info: "Max frames behind cannot be smaller than 1.".to_owned(),
            });
        }

        if max_frames_behind >= self.spectator_config.buffer_size {
            return Err(FortressError::InvalidRequest {
                info: format!(
                    "Max frames behind cannot be larger or equal than the Spectator buffer size ({})",
                    self.spectator_config.buffer_size
                ),
            });
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
        if catchup_speed < 1 {
            return Err(FortressError::InvalidRequest {
                info: "Catchup speed cannot be smaller than 1.".to_owned(),
            });
        }

        if catchup_speed >= self.spectator_config.max_frames_behind {
            return Err(FortressError::InvalidRequest {
                info: "Catchup speed cannot be larger or equal than the allowed maximum frames behind host"
                    .to_owned(),
            });
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
        for player_handle in 0..self.num_players {
            let handle = PlayerHandle::new(player_handle);
            if !self.player_reg.handles.contains_key(&handle) {
                return Err(FortressError::InvalidRequest{
                    info: "Not enough players have been added. Keep registering players up to the defined player number.".to_owned(),
                });
            }
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
                    self.player_reg.remotes.insert(
                        peer_addr.clone(),
                        self.create_endpoint(handles, peer_addr.clone(), self.local_players),
                    );
                }
                PlayerType::Spectator(peer_addr) => {
                    self.player_reg.spectators.insert(
                        peer_addr.clone(),
                        self.create_endpoint(handles, peer_addr.clone(), self.num_players), // the host of the spectator sends inputs for all players
                    );
                }
                PlayerType::Local => (),
            }
        }

        Ok(P2PSession::<T>::new(
            self.num_players,
            self.max_prediction,
            Box::new(socket),
            self.player_reg,
            self.sparse_saving,
            self.desync_detection,
            self.input_delay,
            self.violation_observer,
            self.protocol_config,
        ))
    }

    /// Consumes the builder to create a new [`SpectatorSession`].
    /// A [`SpectatorSession`] provides all functionality to connect to a remote host in a peer-to-peer fashion.
    /// The host will broadcast all confirmed inputs to this session.
    /// This session can be used to spectate a session without contributing to the game input.
    pub fn start_spectator_session(
        self,
        host_addr: T::Address,
        socket: impl NonBlockingSocket<T::Address> + 'static,
    ) -> SpectatorSession<T> {
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
        );
        host.synchronize();
        SpectatorSession::new(
            self.num_players,
            Box::new(socket),
            host,
            self.spectator_config.buffer_size,
            self.spectator_config.max_frames_behind,
            self.spectator_config.catchup_speed,
            self.violation_observer,
        )
    }

    /// Consumes the builder to construct a new [`SyncTestSession`]. During a [`SyncTestSession`], Fortress Rollback will simulate a rollback every frame
    /// and resimulate the last n states, where n is the given `check_distance`.
    /// The resimulated checksums will be compared with the original checksums and report if there was a mismatch.
    /// Due to the decentralized nature of saving and loading gamestates, checksum comparisons can only be made if `check_distance` is 2 or higher.
    /// This is a great way to test if your system runs deterministically.
    /// After creating the session, add a local player, set input delay for them and then start the session.
    pub fn start_synctest_session(self) -> Result<SyncTestSession<T>, FortressError> {
        if self.check_dist >= self.max_prediction {
            return Err(FortressError::InvalidRequest {
                info: "Check distance too big.".to_owned(),
            });
        }
        Ok(SyncTestSession::new(
            self.num_players,
            self.max_prediction,
            self.check_dist,
            self.input_delay,
            self.violation_observer,
        ))
    }

    fn create_endpoint(
        &self,
        handles: Vec<PlayerHandle>,
        peer_addr: T::Address,
        local_players: usize,
    ) -> UdpProtocol<T> {
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
        );
        // start the synchronization
        endpoint.synchronize();
        endpoint
    }
}

#[cfg(test)]
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
    // Input Delay Bounds Tests
    // These tests verify the fix for a Kani-discovered edge case where
    // frame_delay >= INPUT_QUEUE_LENGTH could cause circular buffer overflow.
    // ========================================================================

    #[test]
    fn test_with_input_delay_accepts_zero() {
        let builder = SessionBuilder::<TestConfig>::new().with_input_delay(0);
        assert_eq!(builder.input_delay, 0);
    }

    #[test]
    fn test_with_input_delay_accepts_typical_values() {
        for delay in 1..=8 {
            let builder = SessionBuilder::<TestConfig>::new().with_input_delay(delay);
            assert_eq!(builder.input_delay, delay);
        }
    }

    #[test]
    fn test_with_input_delay_accepts_max_valid() {
        let builder = SessionBuilder::<TestConfig>::new().with_input_delay(MAX_FRAME_DELAY);
        assert_eq!(builder.input_delay, MAX_FRAME_DELAY);
    }

    #[test]
    #[should_panic(expected = "exceeds maximum allowed value")]
    fn test_with_input_delay_panics_on_excessive_delay() {
        let _builder = SessionBuilder::<TestConfig>::new().with_input_delay(MAX_FRAME_DELAY + 1);
    }

    #[test]
    #[should_panic(expected = "exceeds maximum allowed value")]
    fn test_with_input_delay_panics_on_queue_length() {
        use crate::input_queue::INPUT_QUEUE_LENGTH;
        let _builder = SessionBuilder::<TestConfig>::new().with_input_delay(INPUT_QUEUE_LENGTH);
    }
}
