use std::collections::BTreeMap;
use std::sync::Arc;

use web_time::Duration;

use crate::{
    input_queue::INPUT_QUEUE_LENGTH,
    network::protocol::UdpProtocol,
    report_violation,
    sessions::p2p_session::PlayerRegistry,
    telemetry::{ViolationKind, ViolationObserver, ViolationSeverity},
    time_sync::TimeSyncConfig,
    Config, DesyncDetection, FortressError, NonBlockingSocket, P2PSession, PlayerHandle,
    PlayerType, SpectatorSession, SyncTestSession,
};

/// Configuration for the synchronization protocol.
///
/// This struct allows fine-tuning the sync handshake behavior for different
/// network conditions. The defaults work well for typical networks with <15%
/// packet loss and <100ms RTT.
///
/// # Forward Compatibility
///
/// New fields may be added to this struct in future versions. To ensure your
/// code continues to compile, always use the `..Default::default()` or
/// `..SyncConfig::default()` pattern when constructing instances.
///
/// # Example
///
/// ```
/// use fortress_rollback::SyncConfig;
/// use web_time::Duration;
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
#[must_use = "SyncConfig has no effect unless passed to SessionBuilder::with_sync_config()"]
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

    /// Configuration preset for mobile/cellular networks.
    ///
    /// Mobile networks have high variability, intermittent connectivity,
    /// and often switch between WiFi and cellular. This preset combines
    /// aspects of high_latency and lossy with additional tolerance.
    ///
    /// Characteristics addressed:
    /// - High jitter (50-150ms variation)
    /// - Intermittent packet loss (5-20%)
    /// - Connection handoff during WiFi/cellular switches
    /// - Variable RTT (60-200ms)
    pub fn mobile() -> Self {
        Self {
            // More sync packets to handle intermittent loss
            num_sync_packets: 10,
            // Longer retry interval to avoid flooding during handoffs
            sync_retry_interval: Duration::from_millis(350),
            // Generous timeout for connection establishment
            sync_timeout: Some(Duration::from_secs(15)),
            // Longer retry interval during gameplay
            running_retry_interval: Duration::from_millis(350),
            // More frequent keepalives to detect connection issues
            keepalive_interval: Duration::from_millis(300),
        }
    }

    /// Configuration preset for competitive/esports scenarios.
    ///
    /// Prioritizes quick detection of network issues over tolerance.
    /// Assumes good network conditions and fails fast on problems.
    ///
    /// Characteristics:
    /// - Fast sync handshake
    /// - Quick failure detection
    /// - Strict timeout for connection
    pub fn competitive() -> Self {
        Self {
            // Fewer sync packets for faster connection
            num_sync_packets: 4,
            // Fast retry for quick connection
            sync_retry_interval: Duration::from_millis(100),
            // Strict timeout - fail fast if network is bad
            sync_timeout: Some(Duration::from_secs(3)),
            // Fast retries during gameplay
            running_retry_interval: Duration::from_millis(100),
            // Frequent keepalives for quick disconnect detection
            keepalive_interval: Duration::from_millis(100),
        }
    }
}

/// Configuration for network protocol behavior.
///
/// These settings control network timing, buffering, and telemetry thresholds.
/// The defaults work well for most scenarios; adjust for specific requirements.
///
/// # Forward Compatibility
///
/// New fields may be added to this struct in future versions. To ensure your
/// code continues to compile, always use the `..Default::default()` or
/// `..ProtocolConfig::default()` pattern when constructing instances.
///
/// # Example
///
/// ```
/// use fortress_rollback::ProtocolConfig;
/// use web_time::Duration;
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
#[must_use = "ProtocolConfig has no effect unless passed to SessionBuilder::with_protocol_config()"]
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

    /// Configuration preset for mobile/cellular networks.
    ///
    /// Mobile networks have high variability and frequent temporary
    /// disconnections during handoffs. This preset is more tolerant
    /// of sync delays and allows for larger output buffers.
    ///
    /// Characteristics addressed:
    /// - High jitter requiring more buffering
    /// - Connection handoffs during WiFi/cellular switches
    /// - Higher than normal retry expectations
    pub fn mobile() -> Self {
        Self {
            // Slower quality reports to reduce bandwidth on metered connections
            quality_report_interval: Duration::from_millis(350),
            // Very long shutdown delay to handle reconnection attempts
            shutdown_delay: Duration::from_millis(15000),
            // Larger checksum history for delayed desync detection
            max_checksum_history: 64,
            // Higher pending output limit for buffering during jitter
            pending_output_limit: 256,
            // Much higher threshold before warning - mobile is expected to retry often
            sync_retry_warning_threshold: 25,
            // Longer sync expected on mobile
            sync_duration_warning_ms: 12000,
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
///     ..SpectatorConfig::default()
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
#[must_use = "SpectatorConfig has no effect unless passed to SessionBuilder::with_spectator_config()"]
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

    /// Configuration preset for streaming/broadcast scenarios.
    ///
    /// Optimized for live event streaming, tournament broadcasts, and
    /// replay viewers. Uses a very large buffer and conservative catch-up
    /// to avoid visual stuttering on stream.
    ///
    /// Characteristics:
    /// - Large buffer (3 seconds at 60 FPS)
    /// - Slow, smooth catch-up to avoid jarring speed changes
    /// - High tolerance for falling behind
    pub fn broadcast() -> Self {
        Self {
            // 3 seconds of buffer at 60 FPS for smooth streaming
            buffer_size: 180,
            // Very slow catch-up to avoid visual stuttering on stream
            catchup_speed: 1,
            // Can fall far behind before catching up - prioritize smooth playback
            max_frames_behind: 30,
        }
    }

    /// Configuration preset for mobile/cellular spectators.
    ///
    /// Uses larger buffers and tolerant catch-up for variable
    /// mobile network conditions.
    pub fn mobile() -> Self {
        Self {
            // 2 seconds of buffer at 60 FPS
            buffer_size: 120,
            // Moderate catch-up speed
            catchup_speed: 1,
            // High tolerance for network variability
            max_frames_behind: 25,
        }
    }
}

/// Configuration for input queue sizing.
///
/// These settings control the size of the input queue (circular buffer) that stores
/// player inputs. A larger queue allows for longer input history and higher frame delays,
/// but uses more memory.
///
/// # Forward Compatibility
///
/// New fields may be added to this struct in future versions. To ensure your
/// code continues to compile, always use the `..Default::default()` or
/// `..InputQueueConfig::default()` pattern when constructing instances.
///
/// # Memory Usage
///
/// Each input queue stores `queue_length` inputs per player. With 2 players and
/// 128-frame queue (default), this is 256 input slots total.
///
/// # Constraints
///
/// - `queue_length` must be at least 2 (minimum for circular buffer operation)
/// - `queue_length` should be a power of 2 for optimal modulo performance (not enforced)
/// - `frame_delay` must be less than `queue_length` (enforced at session creation)
///
/// # Example
///
/// ```
/// use fortress_rollback::InputQueueConfig;
///
/// // Default configuration (128 frames = ~2.1 seconds at 60 FPS)
/// let default = InputQueueConfig::default();
/// assert_eq!(default.queue_length, 128);
///
/// // For games needing longer input history
/// let high_latency = InputQueueConfig::high_latency();
/// assert_eq!(high_latency.queue_length, 256);
///
/// // For memory-constrained environments
/// let minimal = InputQueueConfig::minimal();
/// assert_eq!(minimal.queue_length, 32);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use = "InputQueueConfig has no effect unless passed to SessionBuilder::with_input_queue_config()"]
pub struct InputQueueConfig {
    /// The length of the input queue (circular buffer) per player.
    ///
    /// This determines:
    /// - How many frames of input history are stored
    /// - The maximum allowed frame delay (`queue_length - 1`)
    /// - Memory usage per player
    ///
    /// At 60 FPS:
    /// - 32 frames = ~0.5 seconds
    /// - 64 frames = ~1.1 seconds
    /// - 128 frames (default) = ~2.1 seconds
    /// - 256 frames = ~4.3 seconds
    ///
    /// # Formal Specification Alignment
    /// - **TLA+**: `QUEUE_LENGTH` in `specs/tla/InputQueue.tla` (uses 3 for model checking)
    /// - **Kani**: `INPUT_QUEUE_LENGTH` in `src/input_queue.rs` (uses 8 for tractable verification)
    /// - **Z3**: `INPUT_QUEUE_LENGTH` in `tests/test_z3_verification.rs` (uses 128)
    /// - **formal-spec.md**: INV-4 (queue length bounds), INV-5 (index validity)
    /// - **spec-divergences.md**: Documents why different values are used
    ///
    /// Default: 128
    pub queue_length: usize,
}

impl Default for InputQueueConfig {
    fn default() -> Self {
        Self {
            queue_length: INPUT_QUEUE_LENGTH,
        }
    }
}

impl InputQueueConfig {
    /// Creates a new `InputQueueConfig` with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Configuration for high-latency networks.
    ///
    /// Uses a larger queue (256 frames = ~4.3 seconds at 60 FPS) to allow
    /// for higher frame delays and longer rollback windows.
    pub fn high_latency() -> Self {
        Self { queue_length: 256 }
    }

    /// Configuration for minimal memory usage.
    ///
    /// Uses a smaller queue (32 frames = ~0.5 seconds at 60 FPS).
    /// Suitable for games with low latency requirements.
    pub fn minimal() -> Self {
        Self { queue_length: 32 }
    }

    /// Configuration for standard networks.
    ///
    /// Uses the default queue size (128 frames = ~2.1 seconds at 60 FPS).
    pub fn standard() -> Self {
        Self::default()
    }

    /// Returns the maximum allowed frame delay for this configuration.
    ///
    /// This is always `queue_length - 1` to ensure the circular buffer
    /// doesn't overflow when advancing the queue head.
    #[must_use]
    pub fn max_frame_delay(&self) -> usize {
        self.queue_length.saturating_sub(1)
    }

    /// Validates that the given frame delay is valid for this configuration.
    ///
    /// # Errors
    ///
    /// Returns `FortressError::InvalidRequest` if `frame_delay >= queue_length`.
    pub fn validate_frame_delay(&self, frame_delay: usize) -> Result<(), FortressError> {
        if frame_delay >= self.queue_length {
            return Err(FortressError::InvalidRequest {
                info: format!(
                    "Frame delay {} is too large for queue length {}. Maximum allowed: {}",
                    frame_delay,
                    self.queue_length,
                    self.max_frame_delay()
                ),
            });
        }
        Ok(())
    }

    /// Validates the configuration itself.
    ///
    /// # Errors
    ///
    /// Returns `FortressError::InvalidRequest` if `queue_length < 2`.
    pub fn validate(&self) -> Result<(), FortressError> {
        if self.queue_length < 2 {
            return Err(FortressError::InvalidRequest {
                info: format!(
                    "Input queue length {} is too small. Minimum is 2.",
                    self.queue_length
                ),
            });
        }
        Ok(())
    }
}

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

/// Controls how game states are saved for rollback.
///
/// This enum replaces the boolean `sparse_saving` parameter for improved API clarity.
/// Using an enum makes the code self-documenting and prevents accidentally passing
/// the wrong boolean value.
///
/// # Choosing a Save Mode
///
/// - **`SaveMode::EveryFrame`** (default): Saves state every frame. Best when:
///   - State serialization is fast
///   - You want minimal rollback distance
///   - You have sufficient memory for frame history
///
/// - **`SaveMode::Sparse`**: Only saves the minimum confirmed frame. Best when:
///   - State serialization is expensive (complex game state)
///   - You want to minimize save overhead
///   - You can tolerate potentially longer rollbacks
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
///
/// // For games with fast state serialization (default)
/// let builder = SessionBuilder::<MyConfig>::new()
///     .with_save_mode(SaveMode::EveryFrame);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum SaveMode {
    /// Save game state every frame.
    ///
    /// This is the default mode. It provides the shortest possible rollback distance
    /// since the most recent confirmed state is always available. However, it requires
    /// a save operation every frame, which may be expensive for complex game states.
    ///
    /// Use this mode when:
    /// - Your game state is small or fast to serialize
    /// - You want minimal rollback distance
    /// - You have sufficient memory for the frame history
    #[default]
    EveryFrame,

    /// Only save the minimum confirmed frame.
    ///
    /// In this mode, only the frame for which all inputs from all players are confirmed
    /// correct will be saved. This dramatically reduces the number of save operations
    /// but may result in longer rollbacks when predictions are incorrect.
    ///
    /// Use this mode when:
    /// - Saving your game state is expensive (large or complex state)
    /// - Advancing the game state is relatively cheap
    /// - You can tolerate longer rollbacks in exchange for fewer saves
    Sparse,
}
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
// The amount of events a spectator can buffer; should never be an issue if the user polls the events at every step
pub(crate) const MAX_EVENT_QUEUE_SIZE: usize = 100;

/// The [`SessionBuilder`] builds all Fortress Rollback Sessions. After setting all appropriate values, use `SessionBuilder::start_yxz_session(...)`
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
            },
            PlayerType::Remote(_) => {
                if !player_handle.is_valid_player_for(self.num_players) {
                    return Err(FortressError::InvalidRequest {
                        info: "The player handle you provided is invalid. For a remote player, the handle should be between 0 and num_players".to_owned(),
                    });
                }
            },
            PlayerType::Spectator(_) => {
                if !player_handle.is_spectator_for(self.num_players) {
                    return Err(FortressError::InvalidRequest {
                        info: "The player handle you provided is invalid. For a spectator, the handle should be num_players or higher".to_owned(),
                    });
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
    /// # Note on Invalid Values
    ///
    /// If `delay` is greater than or equal to the configured `queue_length`
    /// (default 128, configurable via [`with_input_queue_config`](Self::with_input_queue_config)),
    /// a violation is reported and the delay is clamped to the maximum allowed value.
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
    /// use fortress_rollback::{SessionBuilder, Config, InputQueueConfig};
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
    ///     .with_input_delay(8);
    ///
    /// // With custom queue size, max delay changes
    /// let builder = SessionBuilder::<TestConfig>::new()
    ///     .with_input_queue_config(InputQueueConfig::minimal()) // queue_length = 32
    ///     .with_input_delay(30); // max is now 31
    /// ```
    pub fn with_input_delay(mut self, delay: usize) -> Self {
        let max_delay = self.input_queue_config.max_frame_delay();
        if delay > max_delay {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::Configuration,
                "Input delay {} exceeds maximum allowed value of {} (queue_length - 1). \
                 At 60fps, this would be {:.1}+ seconds of delay. Clamping to max.",
                delay,
                max_delay,
                delay as f64 / 60.0
            );
            self.input_delay = max_delay;
        } else {
            self.input_delay = delay;
        }
        self
    }

    /// Change number of total players. Default is 2.
    pub fn with_num_players(mut self, num_players: usize) -> Self {
        self.num_players = num_players;
        self
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
                    let endpoint = self
                        .create_endpoint(handles, peer_addr.clone(), self.local_players)
                        .ok_or_else(|| FortressError::SerializationError {
                            context:
                                "Failed to create protocol endpoint - input serialization error"
                                    .to_owned(),
                        })?;
                    self.player_reg.remotes.insert(peer_addr, endpoint);
                },
                PlayerType::Spectator(peer_addr) => {
                    let endpoint = self
                        .create_endpoint(handles, peer_addr.clone(), self.num_players) // the host of the spectator sends inputs for all players
                        .ok_or_else(|| FortressError::SerializationError {
                            context:
                                "Failed to create spectator endpoint - input serialization error"
                                    .to_owned(),
                        })?;
                    self.player_reg.spectators.insert(peer_addr, endpoint);
                },
                PlayerType::Local => (),
            }
        }

        // Validate the input queue configuration
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
        host.synchronize();
        Some(SpectatorSession::new(
            self.num_players,
            Box::new(socket),
            host,
            self.spectator_config.buffer_size,
            self.spectator_config.max_frames_behind,
            self.spectator_config.catchup_speed,
            self.violation_observer,
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
            return Err(FortressError::InvalidRequest {
                info: "Check distance too big.".to_owned(),
            });
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
        endpoint.synchronize();
        Some(endpoint)
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
    // SaveMode Tests
    // ========================================================================

    #[test]
    fn test_save_mode_default_is_every_frame() {
        let mode = SaveMode::default();
        assert_eq!(mode, SaveMode::EveryFrame);
    }

    #[test]
    fn test_save_mode_equality() {
        assert_eq!(SaveMode::EveryFrame, SaveMode::EveryFrame);
        assert_eq!(SaveMode::Sparse, SaveMode::Sparse);
        assert_ne!(SaveMode::EveryFrame, SaveMode::Sparse);
    }

    #[test]
    fn test_save_mode_debug_format() {
        let every_frame = SaveMode::EveryFrame;
        let sparse = SaveMode::Sparse;
        assert_eq!(format!("{:?}", every_frame), "EveryFrame");
        assert_eq!(format!("{:?}", sparse), "Sparse");
    }

    #[test]
    fn test_save_mode_clone() {
        let mode = SaveMode::Sparse;
        // Intentionally using clone to verify Clone trait works
        let cloned = Clone::clone(&mode);
        assert_eq!(mode, cloned);
    }

    #[test]
    fn test_save_mode_copy() {
        let mode = SaveMode::EveryFrame;
        let copied: SaveMode = mode; // Copy
        assert_eq!(mode, copied);
    }

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

    #[test]
    fn test_save_mode_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(SaveMode::EveryFrame);
        set.insert(SaveMode::Sparse);
        assert_eq!(set.len(), 2);

        // Inserting duplicates doesn't increase size
        set.insert(SaveMode::EveryFrame);
        assert_eq!(set.len(), 2);
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
        use crate::input_queue::INPUT_QUEUE_LENGTH;
        let max_delay = INPUT_QUEUE_LENGTH - 1;
        let builder = SessionBuilder::<TestConfig>::new().with_input_delay(max_delay);
        assert_eq!(builder.input_delay, max_delay);
    }

    #[test]
    fn test_with_input_delay_clamps_excessive_delay() {
        use crate::input_queue::INPUT_QUEUE_LENGTH;
        let builder = SessionBuilder::<TestConfig>::new().with_input_delay(INPUT_QUEUE_LENGTH);
        // Excessive delay should be clamped to max allowed value (queue_length - 1)
        assert_eq!(builder.input_delay, INPUT_QUEUE_LENGTH - 1);
    }

    #[test]
    fn test_with_input_delay_clamps_to_queue_length() {
        use crate::input_queue::INPUT_QUEUE_LENGTH;
        let builder = SessionBuilder::<TestConfig>::new().with_input_delay(INPUT_QUEUE_LENGTH * 2);
        // Excessive delay should be clamped to max allowed value (queue_length - 1)
        assert_eq!(builder.input_delay, INPUT_QUEUE_LENGTH - 1);
    }

    // ========================================================================
    // InputQueueConfig Tests
    // ========================================================================

    #[test]
    fn test_input_queue_config_default() {
        use crate::input_queue::INPUT_QUEUE_LENGTH;
        let config = InputQueueConfig::default();
        assert_eq!(config.queue_length, INPUT_QUEUE_LENGTH);
    }

    #[test]
    fn test_input_queue_config_presets() {
        let high_latency = InputQueueConfig::high_latency();
        assert_eq!(high_latency.queue_length, 256);

        let minimal = InputQueueConfig::minimal();
        assert_eq!(minimal.queue_length, 32);

        let standard = InputQueueConfig::standard();
        assert_eq!(standard, InputQueueConfig::default());
    }

    /// Test that standard() explicitly equals INPUT_QUEUE_LENGTH.
    ///
    /// This test catches any accidental hardcoding of the queue length value.
    /// Under Kani, INPUT_QUEUE_LENGTH is 8; in production it's 128.
    /// The test should pass in both environments.
    #[test]
    fn test_standard_preset_uses_input_queue_length_constant() {
        use crate::input_queue::INPUT_QUEUE_LENGTH;

        let standard = InputQueueConfig::standard();
        assert_eq!(
            standard.queue_length, INPUT_QUEUE_LENGTH,
            "standard() should return INPUT_QUEUE_LENGTH ({}), but got {}. \
             This may indicate a hardcoded value that doesn't account for \
             different build configurations (e.g., Kani vs production).",
            INPUT_QUEUE_LENGTH, standard.queue_length
        );
    }

    /// Data-driven test: all presets should pass validation.
    ///
    /// Uses a table-driven approach to test all presets consistently.
    #[test]
    fn test_all_presets_are_valid_configurations() {
        let presets: &[(&str, InputQueueConfig)] = &[
            ("standard", InputQueueConfig::standard()),
            ("high_latency", InputQueueConfig::high_latency()),
            ("minimal", InputQueueConfig::minimal()),
        ];

        for (name, config) in presets {
            assert!(
                config.validate().is_ok(),
                "Preset '{}' with queue_length={} should be valid, but validation failed: {:?}",
                name,
                config.queue_length,
                config.validate()
            );
        }
    }

    /// Data-driven test: all presets should have valid max_frame_delay.
    #[test]
    fn test_all_presets_max_frame_delay_is_valid() {
        let presets: &[(&str, InputQueueConfig)] = &[
            ("standard", InputQueueConfig::standard()),
            ("high_latency", InputQueueConfig::high_latency()),
            ("minimal", InputQueueConfig::minimal()),
        ];

        for (name, config) in presets {
            let max_delay = config.max_frame_delay();
            assert!(
                config.validate_frame_delay(max_delay).is_ok(),
                "Preset '{}': max_frame_delay() returned {}, but this is not a valid frame delay \
                 for queue_length={}",
                name,
                max_delay,
                config.queue_length
            );
        }
    }

    /// Test that hardcoded preset values match their documented values.
    ///
    /// Note: This test uses hardcoded values intentionally for high_latency and minimal
    /// because those presets ARE hardcoded in the implementation. This test documents
    /// and verifies that contract.
    #[test]
    fn test_hardcoded_preset_values() {
        // high_latency() is hardcoded to 256 (documented: ~4.3 seconds at 60 FPS)
        assert_eq!(
            InputQueueConfig::high_latency().queue_length,
            256,
            "high_latency() is documented to return queue_length=256"
        );

        // minimal() is hardcoded to 32 (documented: ~0.5 seconds at 60 FPS)
        assert_eq!(
            InputQueueConfig::minimal().queue_length,
            32,
            "minimal() is documented to return queue_length=32"
        );
    }

    #[test]
    fn test_input_queue_config_max_frame_delay() {
        let config = InputQueueConfig { queue_length: 64 };
        assert_eq!(config.max_frame_delay(), 63);

        let config = InputQueueConfig { queue_length: 128 };
        assert_eq!(config.max_frame_delay(), 127);
    }

    #[test]
    fn test_input_queue_config_validate() {
        // Valid configs
        assert!(InputQueueConfig { queue_length: 2 }.validate().is_ok());
        assert!(InputQueueConfig { queue_length: 128 }.validate().is_ok());

        // Invalid configs
        assert!(InputQueueConfig { queue_length: 0 }.validate().is_err());
        assert!(InputQueueConfig { queue_length: 1 }.validate().is_err());
    }

    #[test]
    fn test_input_queue_config_validate_frame_delay() {
        let config = InputQueueConfig { queue_length: 32 };

        // Valid delays
        assert!(config.validate_frame_delay(0).is_ok());
        assert!(config.validate_frame_delay(31).is_ok());

        // Invalid delays
        assert!(config.validate_frame_delay(32).is_err());
        assert!(config.validate_frame_delay(100).is_err());
    }

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
            .with_input_delay(31); // Should succeed
        assert_eq!(builder.input_delay, 31);
    }

    #[test]
    fn test_input_queue_config_custom_queue_clamps_delay() {
        // With minimal config (queue_length=32), max delay is 31
        // Trying to set delay=32 should clamp to 31
        let builder = SessionBuilder::<TestConfig>::new()
            .with_input_queue_config(InputQueueConfig::minimal())
            .with_input_delay(32);
        assert_eq!(builder.input_delay, 31);
    }

    // ========================================================================
    // SyncConfig Tests
    // ========================================================================

    #[test]
    fn sync_config_default_values() {
        let config = SyncConfig::default();
        assert_eq!(config.num_sync_packets, 5);
        assert_eq!(config.sync_retry_interval, Duration::from_millis(200));
        assert!(config.sync_timeout.is_none());
        assert_eq!(config.running_retry_interval, Duration::from_millis(200));
        assert_eq!(config.keepalive_interval, Duration::from_millis(200));
    }

    #[test]
    fn sync_config_new_equals_default() {
        let new_config = SyncConfig::new();
        let default_config = SyncConfig::default();
        assert_eq!(new_config, default_config);
    }

    #[test]
    fn sync_config_high_latency_preset() {
        let config = SyncConfig::high_latency();
        assert_eq!(config.num_sync_packets, 5);
        assert_eq!(config.sync_retry_interval, Duration::from_millis(400));
        assert_eq!(config.sync_timeout, Some(Duration::from_secs(10)));
        assert_eq!(config.running_retry_interval, Duration::from_millis(400));
        assert_eq!(config.keepalive_interval, Duration::from_millis(400));
    }

    #[test]
    fn sync_config_lossy_preset() {
        let config = SyncConfig::lossy();
        assert_eq!(config.num_sync_packets, 8);
        assert_eq!(config.sync_retry_interval, Duration::from_millis(200));
        assert_eq!(config.sync_timeout, Some(Duration::from_secs(10)));
        assert_eq!(config.running_retry_interval, Duration::from_millis(200));
        assert_eq!(config.keepalive_interval, Duration::from_millis(200));
    }

    #[test]
    fn sync_config_lan_preset() {
        let config = SyncConfig::lan();
        assert_eq!(config.num_sync_packets, 3);
        assert_eq!(config.sync_retry_interval, Duration::from_millis(100));
        assert_eq!(config.sync_timeout, Some(Duration::from_secs(5)));
        assert_eq!(config.running_retry_interval, Duration::from_millis(100));
        assert_eq!(config.keepalive_interval, Duration::from_millis(100));
    }

    #[test]
    fn sync_config_mobile_preset() {
        let config = SyncConfig::mobile();
        assert_eq!(config.num_sync_packets, 10);
        assert_eq!(config.sync_retry_interval, Duration::from_millis(350));
        assert_eq!(config.sync_timeout, Some(Duration::from_secs(15)));
        assert_eq!(config.running_retry_interval, Duration::from_millis(350));
        assert_eq!(config.keepalive_interval, Duration::from_millis(300));
    }

    #[test]
    fn sync_config_competitive_preset() {
        let config = SyncConfig::competitive();
        assert_eq!(config.num_sync_packets, 4);
        assert_eq!(config.sync_retry_interval, Duration::from_millis(100));
        assert_eq!(config.sync_timeout, Some(Duration::from_secs(3)));
        assert_eq!(config.running_retry_interval, Duration::from_millis(100));
        assert_eq!(config.keepalive_interval, Duration::from_millis(100));
    }

    #[test]
    fn sync_config_equality() {
        let config1 = SyncConfig::default();
        let config2 = SyncConfig::default();
        let config3 = SyncConfig::lan();
        assert_eq!(config1, config2);
        assert_ne!(config1, config3);
    }

    #[test]
    #[allow(clippy::clone_on_copy)] // Testing Clone trait implementation explicitly
    fn sync_config_clone() {
        let config = SyncConfig::high_latency();
        let cloned = config.clone();
        assert_eq!(config, cloned);
    }

    #[test]
    fn sync_config_copy() {
        let config = SyncConfig::lossy();
        let copied: SyncConfig = config; // Copy trait
        assert_eq!(config, copied);
    }

    #[test]
    fn sync_config_debug_format() {
        let config = SyncConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("SyncConfig"));
        assert!(debug_str.contains("num_sync_packets"));
        assert!(debug_str.contains("sync_retry_interval"));
    }

    #[test]
    fn sync_config_presets_differ() {
        // Ensure all presets are distinct configurations
        let presets = [
            SyncConfig::default(),
            SyncConfig::high_latency(),
            SyncConfig::lossy(),
            SyncConfig::lan(),
            SyncConfig::mobile(),
            SyncConfig::competitive(),
        ];

        // Check that no two presets are equal (except default and new)
        for (i, preset_a) in presets.iter().enumerate() {
            for (j, preset_b) in presets.iter().enumerate() {
                if i != j {
                    assert_ne!(
                        preset_a, preset_b,
                        "Presets at index {} and {} should differ",
                        i, j
                    );
                }
            }
        }
    }

    #[test]
    fn with_sync_config_applies_to_builder() {
        let builder =
            SessionBuilder::<TestConfig>::new().with_sync_config(SyncConfig::high_latency());
        assert_eq!(builder.sync_config, SyncConfig::high_latency());
    }

    // ========================================================================
    // ProtocolConfig Tests
    // ========================================================================

    #[test]
    fn protocol_config_default_values() {
        let config = ProtocolConfig::default();
        assert_eq!(config.quality_report_interval, Duration::from_millis(200));
        assert_eq!(config.shutdown_delay, Duration::from_millis(5000));
        assert_eq!(config.max_checksum_history, 32);
        assert_eq!(config.pending_output_limit, 128);
        assert_eq!(config.sync_retry_warning_threshold, 10);
        assert_eq!(config.sync_duration_warning_ms, 3000);
    }

    #[test]
    fn protocol_config_new_equals_default() {
        let new_config = ProtocolConfig::new();
        let default_config = ProtocolConfig::default();
        assert_eq!(new_config, default_config);
    }

    #[test]
    fn protocol_config_competitive_preset() {
        let config = ProtocolConfig::competitive();
        assert_eq!(config.quality_report_interval, Duration::from_millis(100));
        assert_eq!(config.shutdown_delay, Duration::from_millis(3000));
        assert_eq!(config.max_checksum_history, 32);
        assert_eq!(config.pending_output_limit, 128);
        assert_eq!(config.sync_retry_warning_threshold, 10);
        assert_eq!(config.sync_duration_warning_ms, 2000);
    }

    #[test]
    fn protocol_config_high_latency_preset() {
        let config = ProtocolConfig::high_latency();
        assert_eq!(config.quality_report_interval, Duration::from_millis(400));
        assert_eq!(config.shutdown_delay, Duration::from_millis(10000));
        assert_eq!(config.max_checksum_history, 64);
        assert_eq!(config.pending_output_limit, 256);
        assert_eq!(config.sync_retry_warning_threshold, 20);
        assert_eq!(config.sync_duration_warning_ms, 10000);
    }

    #[test]
    fn protocol_config_debug_preset() {
        let config = ProtocolConfig::debug();
        assert_eq!(config.quality_report_interval, Duration::from_millis(500));
        assert_eq!(config.shutdown_delay, Duration::from_millis(30000));
        assert_eq!(config.max_checksum_history, 128);
        assert_eq!(config.pending_output_limit, 64);
        assert_eq!(config.sync_retry_warning_threshold, 5);
        assert_eq!(config.sync_duration_warning_ms, 1000);
    }

    #[test]
    fn protocol_config_mobile_preset() {
        let config = ProtocolConfig::mobile();
        assert_eq!(config.quality_report_interval, Duration::from_millis(350));
        assert_eq!(config.shutdown_delay, Duration::from_millis(15000));
        assert_eq!(config.max_checksum_history, 64);
        assert_eq!(config.pending_output_limit, 256);
        assert_eq!(config.sync_retry_warning_threshold, 25);
        assert_eq!(config.sync_duration_warning_ms, 12000);
    }

    #[test]
    fn protocol_config_equality() {
        let config1 = ProtocolConfig::default();
        let config2 = ProtocolConfig::default();
        let config3 = ProtocolConfig::competitive();
        assert_eq!(config1, config2);
        assert_ne!(config1, config3);
    }

    #[test]
    #[allow(clippy::clone_on_copy)] // Testing Clone trait implementation explicitly
    fn protocol_config_clone() {
        let config = ProtocolConfig::high_latency();
        let cloned = config.clone();
        assert_eq!(config, cloned);
    }

    #[test]
    fn protocol_config_copy() {
        let config = ProtocolConfig::mobile();
        let copied: ProtocolConfig = config; // Copy trait
        assert_eq!(config, copied);
    }

    #[test]
    fn protocol_config_debug_format() {
        let config = ProtocolConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("ProtocolConfig"));
        assert!(debug_str.contains("quality_report_interval"));
        assert!(debug_str.contains("shutdown_delay"));
    }

    #[test]
    fn protocol_config_presets_differ() {
        // Ensure all presets are distinct configurations
        let presets = [
            ProtocolConfig::default(),
            ProtocolConfig::competitive(),
            ProtocolConfig::high_latency(),
            ProtocolConfig::debug(),
            ProtocolConfig::mobile(),
        ];

        for (i, preset_a) in presets.iter().enumerate() {
            for (j, preset_b) in presets.iter().enumerate() {
                if i != j {
                    assert_ne!(
                        preset_a, preset_b,
                        "ProtocolConfig presets at index {} and {} should differ",
                        i, j
                    );
                }
            }
        }
    }

    #[test]
    fn with_protocol_config_applies_to_builder() {
        let builder =
            SessionBuilder::<TestConfig>::new().with_protocol_config(ProtocolConfig::competitive());
        assert_eq!(builder.protocol_config, ProtocolConfig::competitive());
    }
}

// =============================================================================
// Kani Formal Verification Proofs for InputQueueConfig
// =============================================================================
//
// These proofs formally verify the validation constraints for configurable constants.
// This completes the Phase 11 gap analysis by adding formal verification for:
// - InputQueueConfig.validate() - queue_length >= 2
// - InputQueueConfig.validate_frame_delay() - frame_delay < queue_length
// - InputQueueConfig.max_frame_delay() - derivation is correct
//
// The proofs verify these constraints hold for ANY valid configuration within
// Kani's symbolic execution bounds.
#[cfg(kani)]
mod kani_config_proofs {
    use super::*;

    /// Proof: validate() accepts all queue_length >= 2
    ///
    /// Verifies that InputQueueConfig.validate() returns Ok for any queue_length >= 2
    /// and Err for queue_length < 2.
    #[kani::proof]
    #[kani::unwind(2)]
    fn proof_validate_accepts_valid_queue_lengths() {
        let queue_length: usize = kani::any();
        // Focus on boundary region for tractability
        kani::assume(queue_length <= 512);

        let config = InputQueueConfig { queue_length };
        let result = config.validate();

        if queue_length >= 2 {
            kani::assert(result.is_ok(), "validate() should accept queue_length >= 2");
        } else {
            kani::assert(result.is_err(), "validate() should reject queue_length < 2");
        }
    }

    /// Proof: validate() boundary condition at queue_length = 2
    ///
    /// Specifically verifies the minimum valid queue_length.
    #[kani::proof]
    #[kani::unwind(2)]
    fn proof_validate_boundary_at_two() {
        // queue_length = 1 should fail
        let config_one = InputQueueConfig { queue_length: 1 };
        kani::assert(
            config_one.validate().is_err(),
            "queue_length=1 should be invalid",
        );

        // queue_length = 2 should succeed
        let config_two = InputQueueConfig { queue_length: 2 };
        kani::assert(
            config_two.validate().is_ok(),
            "queue_length=2 should be valid",
        );
    }

    /// Proof: validate_frame_delay() enforces frame_delay < queue_length
    ///
    /// Verifies that validate_frame_delay returns Ok when frame_delay < queue_length
    /// and Err when frame_delay >= queue_length.
    #[kani::proof]
    #[kani::unwind(10)]
    fn proof_validate_frame_delay_constraint() {
        let queue_length: usize = kani::any();
        let frame_delay: usize = kani::any();

        // Keep bounds tractable
        kani::assume(queue_length >= 2 && queue_length <= 256);
        kani::assume(frame_delay <= 256);

        let config = InputQueueConfig { queue_length };
        let result = config.validate_frame_delay(frame_delay);

        if frame_delay < queue_length {
            kani::assert(
                result.is_ok(),
                "validate_frame_delay should accept delay < queue_length",
            );
        } else {
            kani::assert(
                result.is_err(),
                "validate_frame_delay should reject delay >= queue_length",
            );
        }
    }

    /// Proof: max_frame_delay() returns queue_length - 1 (with saturation)
    ///
    /// Verifies that max_frame_delay() correctly computes queue_length - 1,
    /// using saturating_sub to handle the edge case of queue_length = 0.
    #[kani::proof]
    #[kani::unwind(2)]
    fn proof_max_frame_delay_derivation() {
        let queue_length: usize = kani::any();
        kani::assume(queue_length <= 512);

        let config = InputQueueConfig { queue_length };
        let max_delay = config.max_frame_delay();

        // Should be queue_length - 1, or 0 if queue_length is 0
        let expected = queue_length.saturating_sub(1);
        kani::assert(
            max_delay == expected,
            "max_frame_delay should equal queue_length.saturating_sub(1)",
        );
    }

    /// Proof: max_frame_delay() is always a valid frame_delay
    ///
    /// Verifies that validate_frame_delay(max_frame_delay()) always succeeds
    /// for valid configurations (queue_length >= 2).
    #[kani::proof]
    #[kani::unwind(2)]
    fn proof_max_frame_delay_is_valid_delay() {
        let queue_length: usize = kani::any();
        kani::assume(queue_length >= 2 && queue_length <= 256);

        let config = InputQueueConfig { queue_length };
        let max_delay = config.max_frame_delay();
        let result = config.validate_frame_delay(max_delay);

        kani::assert(
            result.is_ok(),
            "max_frame_delay() should always be a valid frame_delay for valid configs",
        );
    }

    /// Proof: All presets are valid configurations
    ///
    /// Verifies that standard(), high_latency(), and minimal() presets
    /// all pass validate().
    #[kani::proof]
    #[kani::unwind(2)]
    fn proof_all_presets_valid() {
        let standard = InputQueueConfig::standard();
        let high_latency = InputQueueConfig::high_latency();
        let minimal = InputQueueConfig::minimal();

        kani::assert(
            standard.validate().is_ok(),
            "standard() preset should be valid",
        );
        kani::assert(
            high_latency.validate().is_ok(),
            "high_latency() preset should be valid",
        );
        kani::assert(
            minimal.validate().is_ok(),
            "minimal() preset should be valid",
        );
    }

    /// Proof: Presets have correct queue_length values
    ///
    /// Verifies that preset implementations return their expected values.
    /// Note: `standard()` uses `INPUT_QUEUE_LENGTH` which varies between
    /// Kani (8) and production (128) builds. The other presets use
    /// hardcoded values that don't change.
    #[kani::proof]
    #[kani::unwind(2)]
    fn proof_preset_values() {
        use crate::input_queue::INPUT_QUEUE_LENGTH;

        let standard = InputQueueConfig::standard();
        let high_latency = InputQueueConfig::high_latency();
        let minimal = InputQueueConfig::minimal();

        // standard() uses Self::default() which returns INPUT_QUEUE_LENGTH
        kani::assert(
            standard.queue_length == INPUT_QUEUE_LENGTH,
            "standard() should have queue_length=INPUT_QUEUE_LENGTH",
        );
        // high_latency() returns a hardcoded value
        kani::assert(
            high_latency.queue_length == 256,
            "high_latency() should have queue_length=256",
        );
        // minimal() returns a hardcoded value
        kani::assert(
            minimal.queue_length == 32,
            "minimal() should have queue_length=32",
        );
    }
}
