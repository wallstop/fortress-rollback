//! # Configuration Examples for Fortress Rollback
//!
//! This example demonstrates the various configuration options available
//! when setting up a Fortress Rollback session. It covers:
//!
//! - Basic session configuration
//! - Network presets for different conditions
//! - Fine-tuned custom configurations
//! - Best practices for different scenarios
//!
//! Run with: `cargo run --example configuration`

// Allow example-specific patterns
#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::disallowed_macros,
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    // Allow needless_update because we explicitly show `..Default::default()` pattern for
    // forward compatibility - even when all fields are specified, this pattern ensures
    // the example code will continue to compile when new fields are added in future versions.
    clippy::needless_update
)]

use fortress_rollback::{
    Config, DesyncDetection, ProtocolConfig, SaveMode, SessionBuilder, SpectatorConfig, SyncConfig,
    TimeSyncConfig,
};
use std::net::SocketAddr;
use web_time::Duration;

// Define a minimal config for demonstration
struct GameConfig;

impl Config for GameConfig {
    type Input = u8;
    type State = Vec<u8>;
    type Address = SocketAddr;
}

fn main() {
    println!("=== Fortress Rollback Configuration Examples ===\n");

    basic_configuration();
    network_presets();
    custom_configuration();
    competitive_setup();
    casual_online_setup();
    spectator_setup();
    dynamic_configuration();
}

/// Basic configuration with sensible defaults
fn basic_configuration() {
    println!("--- Basic Configuration ---");

    let builder = SessionBuilder::<GameConfig>::new()
        // Required: Set the number of players
        .with_num_players(2).unwrap()
        // Optional: Add input delay (reduces rollbacks at cost of latency)
        .with_input_delay(2).unwrap()
        // Optional: Set expected framerate for timing calculations
        .with_fps(60)
        .expect("FPS must be > 0")
        // Optional: Customize desync detection interval (default: 60 frames)
        .with_desync_detection_mode(DesyncDetection::On { interval: 100 })
        // Optional: Control how far ahead the game can predict (0 = lockstep)
        .with_max_prediction_window(8);

    println!("Builder configured: {:?}", builder);
    println!("  - 2 players with 2-frame input delay");
    println!("  - 60 FPS with 8-frame prediction window");
    println!("  - Desync detection enabled (every 100 frames)\n");
}

/// Using built-in presets for common network conditions
///
/// NOTE: For even simpler configuration, use the new convenience presets:
/// - `with_lan_defaults()` - LAN/local play with minimal latency
/// - `with_internet_defaults()` - Typical online play (2-frame input delay)
/// - `with_high_latency_defaults()` - Mobile/unstable connections (4-frame input delay)
///
/// Example:
/// ```ignore
/// SessionBuilder::<GameConfig>::new()
///     .with_num_players(2).unwrap()
///     .with_lan_defaults()
///     .add_local_player(0).unwrap()
///     .add_remote_player(1, addr).unwrap()
///     .start_p2p_session(socket)
/// ```
fn network_presets() {
    println!("--- Network Presets ---");
    println!("TIP: Use with_lan_defaults(), with_internet_defaults(), or");
    println!("     with_high_latency_defaults() for quick configuration!\n");

    // LAN/Local play - fast connections, minimal latency
    // Equivalent to: .with_lan_defaults()
    let lan_builder = SessionBuilder::<GameConfig>::new()
        .with_num_players(2)
        .unwrap()
        .with_sync_config(SyncConfig::lan())
        .with_protocol_config(ProtocolConfig::competitive())
        .with_time_sync_config(TimeSyncConfig::responsive())
        .with_input_delay(0)
        .unwrap()
        .with_max_prediction_window(4);

    println!("LAN preset (< 20ms RTT):");
    println!("  - 0 input delay (immediate response)");
    println!("  - 4-frame prediction window (minimal needed)");
    println!("  - Fast sync with 3 packets, 100ms retry");
    println!("  - 10-frame time sync window (fast adaptation)");
    println!("  Builder: {:?}\n", lan_builder);

    // Regional internet (20-80ms RTT)
    // Equivalent to: .with_internet_defaults().with_max_prediction_window(8)
    let regional_builder = SessionBuilder::<GameConfig>::new()
        .with_num_players(2)
        .unwrap()
        .with_sync_config(SyncConfig::default())
        .with_protocol_config(ProtocolConfig::default())
        .with_time_sync_config(TimeSyncConfig::default())
        .with_input_delay(2)
        .unwrap()
        .with_max_prediction_window(8);

    println!("Regional preset (20-80ms RTT):");
    println!("  - 2-frame input delay (reduces rollbacks)");
    println!("  - 8-frame prediction window (handles jitter)");
    println!("  - Default sync: 5 packets, 200ms retry");
    println!("  Builder: {:?}\n", regional_builder);

    // High-latency networks (80-200ms RTT)
    // Similar to: .with_high_latency_defaults().with_max_prediction_window(12)
    let high_latency_builder = SessionBuilder::<GameConfig>::new()
        .with_num_players(2)
        .unwrap()
        .with_sync_config(SyncConfig::high_latency())
        .with_protocol_config(ProtocolConfig::high_latency())
        .with_time_sync_config(TimeSyncConfig::smooth())
        .with_input_delay(4)
        .unwrap()
        .with_max_prediction_window(12);

    println!("High-latency preset (80-200ms RTT):");
    println!("  - 4-frame input delay (~67ms at 60 FPS)");
    println!("  - 12-frame prediction window");
    println!("  - 400ms retry intervals to avoid flooding");
    println!("  - 60-frame time sync window (stable)");
    println!("  Builder: {:?}\n", high_latency_builder);

    // Lossy networks (5-15% packet loss)
    let lossy_builder = SessionBuilder::<GameConfig>::new()
        .with_num_players(2)
        .unwrap()
        .with_sync_config(SyncConfig::lossy())
        .with_protocol_config(ProtocolConfig::default())
        .with_time_sync_config(TimeSyncConfig::smooth())
        .with_input_delay(3)
        .unwrap()
        .with_max_prediction_window(15);

    println!("Lossy network preset (5-15% packet loss):");
    println!("  - 8 sync packets for reliable handshake");
    println!("  - 15-frame prediction window");
    println!("  - 3-frame input delay");
    println!("  Builder: {:?}\n", lossy_builder);
}

/// Custom fine-tuned configuration
fn custom_configuration() {
    println!("--- Custom Configuration ---");

    // Custom sync configuration
    let custom_sync = SyncConfig {
        // Require more successful roundtrips for confidence
        num_sync_packets: 7,
        // Retry more frequently on fast connections
        sync_retry_interval: Duration::from_millis(150),
        // Give up after 8 seconds
        sync_timeout: Some(Duration::from_secs(8)),
        // Fast retries during gameplay
        running_retry_interval: Duration::from_millis(100),
        // Keep connection alive
        keepalive_interval: Duration::from_millis(250),
        ..Default::default()
    };

    // Custom protocol configuration
    let custom_protocol = ProtocolConfig {
        // More frequent quality reports for better RTT tracking
        quality_report_interval: Duration::from_millis(150),
        // Longer shutdown delay for graceful cleanup
        shutdown_delay: Duration::from_millis(7000),
        // More checksum history for debugging desyncs
        max_checksum_history: 64,
        // Warn earlier about output queue buildup
        pending_output_limit: 96,
        // Lower sync retry threshold for earlier warnings
        sync_retry_warning_threshold: 8,
        // Warn if sync takes more than 2 seconds
        sync_duration_warning_ms: 2000,
        ..Default::default()
    };

    // Custom time sync configuration
    let custom_time_sync = TimeSyncConfig {
        // Balance between responsiveness and stability
        window_size: 45,
        ..Default::default()
    };

    let builder = SessionBuilder::<GameConfig>::new()
        .with_num_players(2)
        .unwrap()
        .with_sync_config(custom_sync)
        .with_protocol_config(custom_protocol)
        .with_time_sync_config(custom_time_sync);

    println!("Custom configuration:");
    println!("  - 7 sync packets, 8s timeout");
    println!("  - 150ms quality reports");
    println!("  - 64 checksum history slots");
    println!("  Builder: {:?}\n", builder);
}

/// Configuration for competitive/tournament play
fn competitive_setup() {
    println!("--- Competitive Setup ---");

    let builder = SessionBuilder::<GameConfig>::new()
        .with_num_players(2).unwrap()
        // Minimal input delay for fastest response (accept more rollbacks)
        .with_input_delay(1).unwrap()
        // Enable desync detection to catch cheating
        .with_desync_detection_mode(DesyncDetection::On { interval: 30 })
        // Use competitive presets
        .with_sync_config(SyncConfig::lan())
        .with_protocol_config(ProtocolConfig::competitive())
        .with_time_sync_config(TimeSyncConfig::responsive())
        // Moderate prediction window
        .with_max_prediction_window(6)
        // Fast disconnect detection (forfeit on disconnect)
        .with_disconnect_timeout(Duration::from_millis(1500))
        .with_disconnect_notify_delay(Duration::from_millis(300))
        // High framerate for smooth gameplay
        .with_fps(120)
        .expect("FPS must be > 0");

    println!("Competitive setup (requires < 100ms RTT):");
    println!("  - 1-frame input delay for fastest response");
    println!("  - Frequent desync detection (every 30 frames)");
    println!("  - Fast disconnect detection (1.5s timeout)");
    println!("  - 120 FPS target");
    println!("  - Recommended: Enforce RTT < 100ms in matchmaking");
    println!("  Builder: {:?}\n", builder);
}

/// Configuration for casual online play
fn casual_online_setup() {
    println!("--- Casual Online Setup ---");

    let builder = SessionBuilder::<GameConfig>::new()
        .with_num_players(4).unwrap() // Support up to 4 players
        // Moderate input delay for stability
        .with_input_delay(3).unwrap()
        // Less frequent desync checks (performance)
        .with_desync_detection_mode(DesyncDetection::On { interval: 300 })
        // Balanced presets
        .with_sync_config(SyncConfig::default())
        .with_protocol_config(ProtocolConfig::default())
        .with_time_sync_config(TimeSyncConfig::default())
        // Larger prediction window for variable latency
        .with_max_prediction_window(10)
        // More lenient disconnect handling
        .with_disconnect_timeout(Duration::from_millis(5000))
        .with_disconnect_notify_delay(Duration::from_millis(2000))
        // Enable sparse saving for better performance
        .with_save_mode(SaveMode::Sparse)
        // Standard 60 FPS
        .with_fps(60)
        .expect("FPS must be > 0");

    println!("Casual online setup:");
    println!("  - 4 players supported");
    println!("  - 3-frame input delay for stability");
    println!("  - Sparse saving enabled (performance)");
    println!("  - Lenient disconnect handling (5s timeout)");
    println!("  Builder: {:?}\n", builder);
}

/// Configuration for spectator sessions
fn spectator_setup() {
    println!("--- Spectator Setup ---");

    // Fast-paced game spectator config
    let fast_spectator = SpectatorConfig::fast_paced();
    println!("Fast-paced game spectator:");
    println!("  - Buffer: {} frames", fast_spectator.buffer_size);
    println!(
        "  - Catchup speed: {} frames/step",
        fast_spectator.catchup_speed
    );
    println!(
        "  - Max behind: {} frames",
        fast_spectator.max_frames_behind
    );

    // Slow connection spectator config
    let slow_spectator = SpectatorConfig::slow_connection();
    println!("\nSlow connection spectator:");
    println!("  - Buffer: {} frames", slow_spectator.buffer_size);
    println!(
        "  - Catchup speed: {} frames/step",
        slow_spectator.catchup_speed
    );
    println!(
        "  - Max behind: {} frames",
        slow_spectator.max_frames_behind
    );

    // Custom spectator config
    let custom_spectator = SpectatorConfig {
        // Large buffer for high-latency viewers
        buffer_size: 180, // 3 seconds at 60 FPS
        // Aggressive catch-up
        catchup_speed: 3,
        // Tolerate falling behind
        max_frames_behind: 30,
        ..Default::default()
    };

    let builder = SessionBuilder::<GameConfig>::new()
        .with_num_players(2).unwrap()
        .with_spectator_config(custom_spectator)
        // Use high-latency presets for spectator hosts
        .with_sync_config(SyncConfig::high_latency())
        .with_protocol_config(ProtocolConfig::high_latency());

    println!("\nCustom spectator setup:");
    println!("  - 3-second input buffer (180 frames)");
    println!("  - 3x catch-up speed when behind");
    println!("  - Tolerates up to 30 frames behind");
    println!("  Builder: {:?}\n", builder);
}

/// Dynamically choose configuration based on measured network conditions
fn dynamic_configuration() {
    println!("--- Dynamic Configuration Based on Network Conditions ---");

    // Example: Choose configuration based on RTT and packet loss
    let example_rtt_ms = 85;
    let example_packet_loss = 3.0;

    let (input_delay, sync_config, prediction_window) =
        choose_config_for_network(example_rtt_ms, example_packet_loss);

    let builder = SessionBuilder::<GameConfig>::new()
        .with_num_players(2)
        .unwrap()
        .with_input_delay(input_delay)
        .unwrap()
        .with_sync_config(sync_config)
        .with_max_prediction_window(prediction_window);

    println!(
        "For RTT={}ms, packet_loss={:.1}%:",
        example_rtt_ms, example_packet_loss
    );
    println!("  - Input delay: {} frames", input_delay);
    println!("  - Prediction window: {} frames", prediction_window);
    println!("  Builder: {:?}\n", builder);

    // Show decision table
    println!("Decision table for input delay:");
    println!("  RTT 0-20ms   → 0 frames (immediate)");
    println!("  RTT 21-60ms  → 1 frame (~17ms at 60 FPS)");
    println!("  RTT 61-100ms → 2 frames (~33ms)");
    println!("  RTT 101-150ms→ 3 frames (~50ms)");
    println!("  RTT 150ms+   → 4 frames (~67ms)\n");

    println!("Decision table for sync config:");
    println!("  Packet loss > 5%  → SyncConfig::lossy()");
    println!("  RTT > 100ms       → SyncConfig::high_latency()");
    println!("  RTT < 20ms        → SyncConfig::lan()");
    println!("  Otherwise         → SyncConfig::default()\n");
}

/// Helper function to choose configuration based on network conditions
fn choose_config_for_network(rtt_ms: u32, packet_loss_percent: f32) -> (usize, SyncConfig, usize) {
    // Choose input delay based on RTT
    let input_delay = match rtt_ms {
        0..=20 => 0,
        21..=60 => 1,
        61..=100 => 2,
        101..=150 => 3,
        _ => 4,
    };

    // Choose sync config based on conditions
    let sync_config = if packet_loss_percent > 5.0 {
        SyncConfig::lossy()
    } else if rtt_ms > 100 {
        SyncConfig::high_latency()
    } else if rtt_ms < 20 {
        SyncConfig::lan()
    } else {
        SyncConfig::default()
    };

    // Choose prediction window based on RTT
    let prediction_window = match rtt_ms {
        0..=50 => 6,
        51..=100 => 8,
        101..=150 => 10,
        _ => 12,
    };

    (input_delay, sync_config, prediction_window)
}
