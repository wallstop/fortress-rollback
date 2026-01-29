//! Configuration types for Fortress Rollback sessions.
//!
//! This module contains configuration structs that control various aspects of
//! session behavior including synchronization, network protocol, spectator
//! settings, and input queue sizing.
//!
//! # Overview
//!
//! | Config Type | Purpose | Key Presets |
//! |-------------|---------|-------------|
//! | `SyncConfig` | Sync handshake behavior | `lan()`, `mobile()`, `competitive()` |
//! | `ProtocolConfig` | Network protocol settings | `debug()`, `mobile()` |
//! | `SpectatorConfig` | Spectator session behavior | `broadcast()`, `fast_paced()` |
//! | `InputQueueConfig` | Input queue sizing | `high_latency()`, `minimal()` |
//! | `SaveMode` | Game state save strategy | `EveryFrame`, `Sparse` |
//!
//! # Example
//!
//! ```
//! use fortress_rollback::{SyncConfig, ProtocolConfig, SessionBuilder, Config};
//!
//! # struct MyConfig;
//! # impl Config for MyConfig {
//! #     type Input = u32;
//! #     type State = ();
//! #     type Address = std::net::SocketAddr;
//! # }
//! // Use presets for common scenarios
//! let builder = SessionBuilder::<MyConfig>::new()
//!     .with_sync_config(SyncConfig::mobile())
//!     .with_protocol_config(ProtocolConfig::mobile());
//! ```

use web_time::Duration;

use crate::input_queue::INPUT_QUEUE_LENGTH;
use crate::{FortressError, InvalidRequestKind};

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

    /// Configuration preset for lossy networks (5-15% unidirectional packet loss).
    ///
    /// Uses more sync packets for higher confidence and a sync timeout.
    ///
    /// # Note on Packet Loss Math
    ///
    /// When packet loss is applied bidirectionally (both send and receive),
    /// the effective loss rate compounds. For example:
    /// - 10% bidirectional loss = ~19% effective (1 - 0.9 × 0.9)
    /// - 15% bidirectional loss = ~28% effective (1 - 0.85 × 0.85)
    /// - 20% bidirectional loss = ~36% effective (1 - 0.8 × 0.8)
    /// - 30% bidirectional loss = ~51% effective (1 - 0.7 × 0.7)
    ///
    /// For bidirectional loss rates > 20%, consider using [`Self::mobile()`] instead.
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

    /// Configuration preset for extreme/hostile network conditions (testing).
    ///
    /// Designed for testing scenarios with very high packet loss, aggressive
    /// burst loss, or other extreme network impairments. Uses significantly
    /// more sync packets and longer timeouts to maximize chance of success.
    ///
    /// This preset is **not recommended for production use** as it has very
    /// long timeouts that could delay error detection in real scenarios.
    ///
    /// Characteristics addressed:
    /// - High burst loss (10%+ probability, 8+ packet bursts)
    /// - Combined high packet loss (>15%)
    /// - Extreme jitter and latency variation
    /// - Scenarios where multiple consecutive sync attempts may fail
    ///
    /// # Example
    ///
    /// ```
    /// use fortress_rollback::SyncConfig;
    ///
    /// // For testing with aggressive burst loss
    /// let config = SyncConfig::extreme();
    /// assert_eq!(config.num_sync_packets, 20);
    /// ```
    pub fn extreme() -> Self {
        Self {
            // Many more sync packets to survive multiple burst losses
            // With 10% burst probability and 8-packet bursts, we need enough
            // retries to statistically guarantee success
            num_sync_packets: 20,
            // Moderate retry interval - not too fast (flooding) nor too slow
            sync_retry_interval: Duration::from_millis(250),
            // Very generous timeout for sync (30 seconds)
            sync_timeout: Some(Duration::from_secs(30)),
            // Moderate retry interval during gameplay
            running_retry_interval: Duration::from_millis(250),
            // Frequent keepalives to detect issues
            keepalive_interval: Duration::from_millis(200),
        }
    }

    /// Configuration preset for stress testing under the most hostile conditions.
    ///
    /// This preset is specifically designed for automated testing scenarios where
    /// reliability is paramount, even at the cost of very long sync times. It uses
    /// aggressive parameters to survive the most hostile simulated network conditions.
    ///
    /// **ONLY USE FOR TESTING** - These settings would cause unacceptable delays
    /// in production. The 60-second sync timeout means users would wait up to a
    /// full minute before connection failure is reported.
    ///
    /// Characteristics addressed:
    /// - Extreme burst loss (10%+ probability with 8+ packet bursts)
    /// - Very high combined packet loss (>25%)
    /// - Multiple consecutive burst events during handshake
    /// - Slow CI environments with timing variability (macOS CI, coverage builds)
    ///
    /// # Probability Analysis
    ///
    /// With 10% burst probability and 8-packet bursts:
    /// - Each burst can drop 8 consecutive packets
    /// - With 150ms retry interval and 60s timeout: ~400 retry opportunities
    /// - With 40 required sync roundtrips spread across this window, the
    ///   probability of success is very high even under worst-case conditions
    ///
    /// # Example
    ///
    /// ```
    /// use fortress_rollback::SyncConfig;
    ///
    /// // For stress testing with extremely hostile network simulation
    /// let config = SyncConfig::stress_test();
    /// assert_eq!(config.num_sync_packets, 40);
    /// ```
    pub fn stress_test() -> Self {
        Self {
            // Double the sync packets compared to extreme - we have the timeout
            // budget to spare and this dramatically increases success probability
            num_sync_packets: 40,
            // Faster retry interval to get more attempts within the timeout window
            // 150ms gives ~400 attempts in 60 seconds
            sync_retry_interval: Duration::from_millis(150),
            // Very generous timeout for sync (60 seconds)
            // This is acceptable for automated testing but NOT for production
            sync_timeout: Some(Duration::from_secs(60)),
            // Match the faster retry interval for gameplay
            running_retry_interval: Duration::from_millis(150),
            // Frequent keepalives to detect issues quickly once connected
            keepalive_interval: Duration::from_millis(150),
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

    /// Multiplier for input history retention.
    ///
    /// Determines how many frames of received input history to retain.
    /// The protocol keeps inputs for `input_history_multiplier * max_prediction` frames
    /// behind the most recent received frame. This allows for packet reordering
    /// and delayed decoding without losing the ability to decode old packets.
    ///
    /// Higher values use more memory but are more tolerant of extreme packet reordering.
    ///
    /// Default: 2
    pub input_history_multiplier: usize,

    /// Optional seed for protocol RNG, enabling deterministic behavior.
    ///
    /// When set to `Some(seed)`, the protocol will use a deterministic RNG seeded
    /// with this value for generating:
    /// - Session magic numbers (protocol identifiers)
    /// - Sync request validation tokens
    ///
    /// This enables fully reproducible network sessions, which is useful for:
    /// - Replay systems
    /// - Deterministic testing
    /// - Debugging network issues
    ///
    /// When `None` (the default), the protocol uses non-deterministic random values
    /// for security (harder to predict session IDs) and uniqueness (different magic
    /// numbers for each session).
    ///
    /// # Example
    ///
    /// ```
    /// use fortress_rollback::ProtocolConfig;
    ///
    /// // For deterministic testing
    /// let config = ProtocolConfig {
    ///     protocol_rng_seed: Some(12345),
    ///     ..ProtocolConfig::default()
    /// };
    ///
    /// // Or use the deterministic preset
    /// let config = ProtocolConfig::deterministic(42);
    /// ```
    ///
    /// Default: `None` (non-deterministic)
    pub protocol_rng_seed: Option<u64>,
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
            input_history_multiplier: 2,
            protocol_rng_seed: None,
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
            input_history_multiplier: 2,
            protocol_rng_seed: None,
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
            input_history_multiplier: 3,
            protocol_rng_seed: None,
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
            input_history_multiplier: 4,
            protocol_rng_seed: None,
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
            // More history for packet reordering on mobile
            input_history_multiplier: 3,
            protocol_rng_seed: None,
        }
    }

    /// Configuration preset for deterministic/reproducible sessions.
    ///
    /// Uses a fixed RNG seed to ensure protocol behavior is reproducible
    /// across runs. This is essential for:
    /// - Replay systems
    /// - Deterministic testing
    /// - Debugging network issues
    /// - Cross-platform consistency
    ///
    /// # Arguments
    ///
    /// * `seed` - The RNG seed for protocol randomness
    ///
    /// # Example
    ///
    /// ```
    /// use fortress_rollback::ProtocolConfig;
    ///
    /// // Create a deterministic config with seed 42
    /// let config = ProtocolConfig::deterministic(42);
    /// assert_eq!(config.protocol_rng_seed, Some(42));
    /// ```
    pub fn deterministic(seed: u64) -> Self {
        Self {
            protocol_rng_seed: Some(seed),
            ..Self::default()
        }
    }

    /// Validates the protocol configuration.
    ///
    /// # Errors
    ///
    /// Returns `FortressError::InvalidRequest` if any configuration value is out of range.
    pub fn validate(&self) -> Result<(), FortressError> {
        // Validate quality_report_interval: 1ms to 10000ms
        if self.quality_report_interval < Duration::from_millis(1)
            || self.quality_report_interval > Duration::from_millis(10000)
        {
            return Err(InvalidRequestKind::DurationConfigOutOfRange {
                field: "quality_report_interval",
                min_ms: 1,
                max_ms: 10000,
                actual_ms: self.quality_report_interval.as_millis() as u64,
            }
            .into());
        }

        // Validate shutdown_delay: 1ms to 300000ms (5 minutes)
        if self.shutdown_delay < Duration::from_millis(1)
            || self.shutdown_delay > Duration::from_millis(300000)
        {
            return Err(InvalidRequestKind::DurationConfigOutOfRange {
                field: "shutdown_delay",
                min_ms: 1,
                max_ms: 300000,
                actual_ms: self.shutdown_delay.as_millis() as u64,
            }
            .into());
        }

        // Validate max_checksum_history: 1 to 1024
        if self.max_checksum_history < 1 || self.max_checksum_history > 1024 {
            return Err(InvalidRequestKind::ConfigValueOutOfRange {
                field: "max_checksum_history",
                min: 1,
                max: 1024,
                actual: self.max_checksum_history as u64,
            }
            .into());
        }

        // Validate pending_output_limit: 1 to 4096
        if self.pending_output_limit < 1 || self.pending_output_limit > 4096 {
            return Err(InvalidRequestKind::ConfigValueOutOfRange {
                field: "pending_output_limit",
                min: 1,
                max: 4096,
                actual: self.pending_output_limit as u64,
            }
            .into());
        }

        // Validate sync_retry_warning_threshold: 1 to 1000
        if self.sync_retry_warning_threshold < 1 || self.sync_retry_warning_threshold > 1000 {
            return Err(InvalidRequestKind::ConfigValueOutOfRange {
                field: "sync_retry_warning_threshold",
                min: 1,
                max: 1000,
                actual: self.sync_retry_warning_threshold as u64,
            }
            .into());
        }

        // Validate sync_duration_warning_ms: 1 to 300000 (5 minutes)
        if self.sync_duration_warning_ms < 1 || self.sync_duration_warning_ms > 300000 {
            return Err(InvalidRequestKind::ConfigValueOutOfRange {
                field: "sync_duration_warning_ms",
                min: 1,
                max: 300000,
                actual: self.sync_duration_warning_ms as u64,
            }
            .into());
        }

        // Validate input_history_multiplier: 1 to 16
        if self.input_history_multiplier < 1 || self.input_history_multiplier > 16 {
            return Err(InvalidRequestKind::ConfigValueOutOfRange {
                field: "input_history_multiplier",
                min: 1,
                max: 16,
                actual: self.input_history_multiplier as u64,
            }
            .into());
        }

        Ok(())
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
            return Err(InvalidRequestKind::FrameDelayTooLarge {
                delay: frame_delay,
                max_delay: self.max_frame_delay(),
            }
            .into());
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
            return Err(InvalidRequestKind::QueueLengthTooSmall {
                length: self.queue_length,
            }
            .into());
        }
        Ok(())
    }
}

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
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
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

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;

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

    // ========================================================================
    // InputQueueConfig Tests
    // ========================================================================

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
    fn sync_config_extreme_preset() {
        let config = SyncConfig::extreme();
        assert_eq!(config.num_sync_packets, 20);
        assert_eq!(config.sync_retry_interval, Duration::from_millis(250));
        assert_eq!(config.sync_timeout, Some(Duration::from_secs(30)));
        assert_eq!(config.running_retry_interval, Duration::from_millis(250));
        assert_eq!(config.keepalive_interval, Duration::from_millis(200));
    }

    #[test]
    fn sync_config_stress_test_preset() {
        let config = SyncConfig::stress_test();
        assert_eq!(config.num_sync_packets, 40);
        assert_eq!(config.sync_retry_interval, Duration::from_millis(150));
        assert_eq!(config.sync_timeout, Some(Duration::from_secs(60)));
        assert_eq!(config.running_retry_interval, Duration::from_millis(150));
        assert_eq!(config.keepalive_interval, Duration::from_millis(150));
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

    // ========================================================================
    // ProtocolConfig Validation Tests
    // ========================================================================

    #[test]
    fn test_protocol_config_validate_default_is_valid() {
        let config = ProtocolConfig::default();
        config.validate().unwrap();
    }

    #[test]
    fn test_protocol_config_validate_all_presets_are_valid() {
        let presets: &[(&str, ProtocolConfig)] = &[
            ("default", ProtocolConfig::default()),
            ("competitive", ProtocolConfig::competitive()),
            ("high_latency", ProtocolConfig::high_latency()),
            ("debug", ProtocolConfig::debug()),
            ("mobile", ProtocolConfig::mobile()),
        ];

        for (name, config) in presets {
            assert!(
                config.validate().is_ok(),
                "Preset '{}' should be valid, but validation failed: {:?}",
                name,
                config.validate()
            );
        }
    }

    #[test]
    fn test_protocol_config_validate_quality_report_interval_valid() {
        // Valid: minimum boundary (1ms)
        let config = ProtocolConfig {
            quality_report_interval: Duration::from_millis(1),
            ..ProtocolConfig::default()
        };
        config.validate().unwrap();

        // Valid: maximum boundary (10000ms)
        let config = ProtocolConfig {
            quality_report_interval: Duration::from_millis(10000),
            ..ProtocolConfig::default()
        };
        config.validate().unwrap();

        // Valid: middle value
        let config = ProtocolConfig {
            quality_report_interval: Duration::from_millis(500),
            ..ProtocolConfig::default()
        };
        config.validate().unwrap();
    }

    #[test]
    fn test_protocol_config_validate_quality_report_interval_too_low() {
        // Invalid: 0ms (below minimum)
        let config = ProtocolConfig {
            quality_report_interval: Duration::from_millis(0),
            ..ProtocolConfig::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::DurationConfigOutOfRange {
                    field: "quality_report_interval",
                    min_ms: 1,
                    max_ms: 10000,
                    ..
                }
            }
        ));
    }

    #[test]
    fn test_protocol_config_validate_quality_report_interval_too_high() {
        // Invalid: 10001ms (above maximum)
        let config = ProtocolConfig {
            quality_report_interval: Duration::from_millis(10001),
            ..ProtocolConfig::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::DurationConfigOutOfRange {
                    field: "quality_report_interval",
                    min_ms: 1,
                    max_ms: 10000,
                    ..
                }
            }
        ));
    }

    #[test]
    fn test_protocol_config_validate_shutdown_delay_valid() {
        // Valid: minimum boundary (1ms)
        let config = ProtocolConfig {
            shutdown_delay: Duration::from_millis(1),
            ..ProtocolConfig::default()
        };
        config.validate().unwrap();

        // Valid: maximum boundary (300000ms)
        let config = ProtocolConfig {
            shutdown_delay: Duration::from_millis(300000),
            ..ProtocolConfig::default()
        };
        config.validate().unwrap();

        // Valid: middle value
        let config = ProtocolConfig {
            shutdown_delay: Duration::from_millis(10000),
            ..ProtocolConfig::default()
        };
        config.validate().unwrap();
    }

    #[test]
    fn test_protocol_config_validate_shutdown_delay_too_low() {
        // Invalid: 0ms (below minimum)
        let config = ProtocolConfig {
            shutdown_delay: Duration::from_millis(0),
            ..ProtocolConfig::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::DurationConfigOutOfRange {
                    field: "shutdown_delay",
                    min_ms: 1,
                    max_ms: 300000,
                    ..
                }
            }
        ));
    }

    #[test]
    fn test_protocol_config_validate_shutdown_delay_too_high() {
        // Invalid: 300001ms (above maximum)
        let config = ProtocolConfig {
            shutdown_delay: Duration::from_millis(300001),
            ..ProtocolConfig::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::DurationConfigOutOfRange {
                    field: "shutdown_delay",
                    min_ms: 1,
                    max_ms: 300000,
                    ..
                }
            }
        ));
    }

    #[test]
    fn test_protocol_config_validate_max_checksum_history_valid() {
        // Valid: minimum boundary (1)
        let config = ProtocolConfig {
            max_checksum_history: 1,
            ..ProtocolConfig::default()
        };
        config.validate().unwrap();

        // Valid: maximum boundary (1024)
        let config = ProtocolConfig {
            max_checksum_history: 1024,
            ..ProtocolConfig::default()
        };
        config.validate().unwrap();

        // Valid: middle value
        let config = ProtocolConfig {
            max_checksum_history: 64,
            ..ProtocolConfig::default()
        };
        config.validate().unwrap();
    }

    #[test]
    fn test_protocol_config_validate_max_checksum_history_too_low() {
        // Invalid: 0 (below minimum)
        let config = ProtocolConfig {
            max_checksum_history: 0,
            ..ProtocolConfig::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::ConfigValueOutOfRange {
                    field: "max_checksum_history",
                    min: 1,
                    max: 1024,
                    ..
                }
            }
        ));
    }

    #[test]
    fn test_protocol_config_validate_max_checksum_history_too_high() {
        // Invalid: 1025 (above maximum)
        let config = ProtocolConfig {
            max_checksum_history: 1025,
            ..ProtocolConfig::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::ConfigValueOutOfRange {
                    field: "max_checksum_history",
                    min: 1,
                    max: 1024,
                    ..
                }
            }
        ));
    }

    #[test]
    fn test_protocol_config_validate_pending_output_limit_valid() {
        // Valid: minimum boundary (1)
        let config = ProtocolConfig {
            pending_output_limit: 1,
            ..ProtocolConfig::default()
        };
        config.validate().unwrap();

        // Valid: maximum boundary (4096)
        let config = ProtocolConfig {
            pending_output_limit: 4096,
            ..ProtocolConfig::default()
        };
        config.validate().unwrap();

        // Valid: middle value
        let config = ProtocolConfig {
            pending_output_limit: 256,
            ..ProtocolConfig::default()
        };
        config.validate().unwrap();
    }

    #[test]
    fn test_protocol_config_validate_pending_output_limit_too_low() {
        // Invalid: 0 (below minimum)
        let config = ProtocolConfig {
            pending_output_limit: 0,
            ..ProtocolConfig::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::ConfigValueOutOfRange {
                    field: "pending_output_limit",
                    min: 1,
                    max: 4096,
                    ..
                }
            }
        ));
    }

    #[test]
    fn test_protocol_config_validate_pending_output_limit_too_high() {
        // Invalid: 4097 (above maximum)
        let config = ProtocolConfig {
            pending_output_limit: 4097,
            ..ProtocolConfig::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::ConfigValueOutOfRange {
                    field: "pending_output_limit",
                    min: 1,
                    max: 4096,
                    ..
                }
            }
        ));
    }

    #[test]
    fn test_protocol_config_validate_sync_retry_warning_threshold_valid() {
        // Valid: minimum boundary (1)
        let config = ProtocolConfig {
            sync_retry_warning_threshold: 1,
            ..ProtocolConfig::default()
        };
        config.validate().unwrap();

        // Valid: maximum boundary (1000)
        let config = ProtocolConfig {
            sync_retry_warning_threshold: 1000,
            ..ProtocolConfig::default()
        };
        config.validate().unwrap();

        // Valid: middle value
        let config = ProtocolConfig {
            sync_retry_warning_threshold: 25,
            ..ProtocolConfig::default()
        };
        config.validate().unwrap();
    }

    #[test]
    fn test_protocol_config_validate_sync_retry_warning_threshold_too_low() {
        // Invalid: 0 (below minimum)
        let config = ProtocolConfig {
            sync_retry_warning_threshold: 0,
            ..ProtocolConfig::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::ConfigValueOutOfRange {
                    field: "sync_retry_warning_threshold",
                    min: 1,
                    max: 1000,
                    ..
                }
            }
        ));
    }

    #[test]
    fn test_protocol_config_validate_sync_retry_warning_threshold_too_high() {
        // Invalid: 1001 (above maximum)
        let config = ProtocolConfig {
            sync_retry_warning_threshold: 1001,
            ..ProtocolConfig::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::ConfigValueOutOfRange {
                    field: "sync_retry_warning_threshold",
                    min: 1,
                    max: 1000,
                    ..
                }
            }
        ));
    }

    #[test]
    fn test_protocol_config_validate_sync_duration_warning_ms_valid() {
        // Valid: minimum boundary (1)
        let config = ProtocolConfig {
            sync_duration_warning_ms: 1,
            ..ProtocolConfig::default()
        };
        config.validate().unwrap();

        // Valid: maximum boundary (300000)
        let config = ProtocolConfig {
            sync_duration_warning_ms: 300000,
            ..ProtocolConfig::default()
        };
        config.validate().unwrap();

        // Valid: middle value
        let config = ProtocolConfig {
            sync_duration_warning_ms: 5000,
            ..ProtocolConfig::default()
        };
        config.validate().unwrap();
    }

    #[test]
    fn test_protocol_config_validate_sync_duration_warning_ms_too_low() {
        // Invalid: 0 (below minimum)
        let config = ProtocolConfig {
            sync_duration_warning_ms: 0,
            ..ProtocolConfig::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::ConfigValueOutOfRange {
                    field: "sync_duration_warning_ms",
                    min: 1,
                    max: 300000,
                    ..
                }
            }
        ));
    }

    #[test]
    fn test_protocol_config_validate_sync_duration_warning_ms_too_high() {
        // Invalid: 300001 (above maximum)
        let config = ProtocolConfig {
            sync_duration_warning_ms: 300001,
            ..ProtocolConfig::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::ConfigValueOutOfRange {
                    field: "sync_duration_warning_ms",
                    min: 1,
                    max: 300000,
                    ..
                }
            }
        ));
    }

    #[test]
    fn test_protocol_config_validate_input_history_multiplier_valid() {
        // Valid: minimum boundary (1)
        let config = ProtocolConfig {
            input_history_multiplier: 1,
            ..ProtocolConfig::default()
        };
        config.validate().unwrap();

        // Valid: maximum boundary (16)
        let config = ProtocolConfig {
            input_history_multiplier: 16,
            ..ProtocolConfig::default()
        };
        config.validate().unwrap();

        // Valid: middle value
        let config = ProtocolConfig {
            input_history_multiplier: 4,
            ..ProtocolConfig::default()
        };
        config.validate().unwrap();
    }

    #[test]
    fn test_protocol_config_validate_input_history_multiplier_too_low() {
        // Invalid: 0 (below minimum)
        let config = ProtocolConfig {
            input_history_multiplier: 0,
            ..ProtocolConfig::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::ConfigValueOutOfRange {
                    field: "input_history_multiplier",
                    min: 1,
                    max: 16,
                    ..
                }
            }
        ));
    }

    #[test]
    fn test_protocol_config_validate_input_history_multiplier_too_high() {
        // Invalid: 17 (above maximum)
        let config = ProtocolConfig {
            input_history_multiplier: 17,
            ..ProtocolConfig::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::ConfigValueOutOfRange {
                    field: "input_history_multiplier",
                    min: 1,
                    max: 16,
                    ..
                }
            }
        ));
    }

    #[test]
    fn test_protocol_config_validate_multiple_invalid_fields() {
        // Test that validation stops at the first invalid field
        // (quality_report_interval is checked first)
        let config = ProtocolConfig {
            quality_report_interval: Duration::from_millis(0), // Invalid
            shutdown_delay: Duration::from_millis(0),          // Also invalid
            max_checksum_history: 0,                           // Also invalid
            ..ProtocolConfig::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        // Should report the first field that failed (quality_report_interval)
        assert!(matches!(
            err,
            FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::DurationConfigOutOfRange {
                    field: "quality_report_interval",
                    ..
                }
            }
        ));
    }

    #[test]
    fn test_protocol_config_validate_all_fields_at_boundaries() {
        // Test a config with all fields at their minimum valid values
        let config = ProtocolConfig {
            quality_report_interval: Duration::from_millis(1),
            shutdown_delay: Duration::from_millis(1),
            max_checksum_history: 1,
            pending_output_limit: 1,
            sync_retry_warning_threshold: 1,
            sync_duration_warning_ms: 1,
            input_history_multiplier: 1,
            protocol_rng_seed: None,
        };
        config.validate().unwrap();

        // Test a config with all fields at their maximum valid values
        let config = ProtocolConfig {
            quality_report_interval: Duration::from_millis(10000),
            shutdown_delay: Duration::from_millis(300000),
            max_checksum_history: 1024,
            pending_output_limit: 4096,
            sync_retry_warning_threshold: 1000,
            sync_duration_warning_ms: 300000,
            input_history_multiplier: 16,
            protocol_rng_seed: None,
        };
        config.validate().unwrap();
    }

    // ========================================================================
    // ProtocolConfig Deterministic RNG Seed Tests
    // ========================================================================

    #[test]
    fn test_protocol_config_deterministic_preset() {
        let config = ProtocolConfig::deterministic(12345);
        assert_eq!(config.protocol_rng_seed, Some(12345));
        // Other fields should be default
        assert_eq!(
            config.quality_report_interval,
            ProtocolConfig::default().quality_report_interval
        );
    }

    #[test]
    fn test_protocol_config_deterministic_different_seeds() {
        let config1 = ProtocolConfig::deterministic(1);
        let config2 = ProtocolConfig::deterministic(2);
        assert_ne!(config1.protocol_rng_seed, config2.protocol_rng_seed);
    }

    #[test]
    fn test_protocol_config_default_has_no_seed() {
        let config = ProtocolConfig::default();
        assert_eq!(config.protocol_rng_seed, None);
    }

    #[test]
    fn test_protocol_config_all_presets_have_no_seed() {
        // All presets except deterministic() should have no seed
        assert_eq!(ProtocolConfig::competitive().protocol_rng_seed, None);
        assert_eq!(ProtocolConfig::high_latency().protocol_rng_seed, None);
        assert_eq!(ProtocolConfig::debug().protocol_rng_seed, None);
        assert_eq!(ProtocolConfig::mobile().protocol_rng_seed, None);
    }

    #[test]
    fn test_protocol_config_seed_validates_ok() {
        // Config with seed should validate successfully
        let config = ProtocolConfig::deterministic(42);
        config.validate().unwrap();
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

    /// Proof: validate() accepts all queue_length >= 2.
    ///
    /// Verifies that InputQueueConfig.validate() returns Ok for any queue_length >= 2
    /// and Err for queue_length < 2.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Queue length validation correctness
    /// - Related: proof_validate_boundary_at_two, proof_all_presets_valid
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

    /// Proof: validate() boundary condition at queue_length = 2.
    ///
    /// Specifically verifies the minimum valid queue_length.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Boundary condition at minimum queue length
    /// - Related: proof_validate_accepts_valid_queue_lengths
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

    /// Proof: validate_frame_delay() enforces frame_delay < queue_length.
    ///
    /// Verifies that validate_frame_delay returns Ok when frame_delay < queue_length
    /// and Err when frame_delay >= queue_length.
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: Frame delay validation constraint
    /// - Related: proof_max_frame_delay_is_valid_delay, proof_max_frame_delay_derivation
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

    /// Proof: max_frame_delay() returns queue_length - 1 (with saturation).
    ///
    /// Verifies that max_frame_delay() correctly computes queue_length - 1,
    /// using saturating_sub to handle the edge case of queue_length = 0.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Max frame delay derivation correctness
    /// - Related: proof_max_frame_delay_is_valid_delay, proof_validate_frame_delay_constraint
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

    /// Proof: max_frame_delay() is always a valid frame_delay.
    ///
    /// Verifies that validate_frame_delay(max_frame_delay()) always succeeds
    /// for valid configurations (queue_length >= 2).
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: Max delay is always valid for valid configs
    /// - Related: proof_max_frame_delay_derivation, proof_validate_frame_delay_constraint
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

    /// Proof: All presets are valid configurations.
    ///
    /// Verifies that standard(), high_latency(), and minimal() presets
    /// all pass validate().
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: All factory presets pass validation
    /// - Related: proof_preset_values, proof_validate_accepts_valid_queue_lengths
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

    /// Proof: Presets have correct queue_length values.
    ///
    /// Verifies that preset implementations return their expected values.
    /// Note: `standard()` uses `INPUT_QUEUE_LENGTH` which varies between
    /// Kani (8) and production (128) builds. The other presets use
    /// hardcoded values that don't change.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Preset queue length values match specifications
    /// - Related: proof_all_presets_valid
    #[kani::proof]
    #[kani::unwind(2)]
    fn proof_preset_values() {
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
