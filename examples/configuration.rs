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

// Allow needless_update because we explicitly show `..Default::default()` pattern for
// forward compatibility - even when all fields are specified, this pattern ensures
// the example code will continue to compile when new fields are added in future versions.
#![allow(clippy::needless_update)]

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
}

/// Basic configuration with sensible defaults
fn basic_configuration() {
    println!("--- Basic Configuration ---");

    let builder = SessionBuilder::<GameConfig>::new()
        // Required: Set the number of players
        .with_num_players(2)
        // Optional: Add input delay (reduces rollbacks at cost of latency)
        .with_input_delay(2)
        // Optional: Set expected framerate for timing calculations
        .with_fps(60)
        .expect("FPS must be > 0")
        // Optional: Enable desync detection every 100 frames
        .with_desync_detection_mode(DesyncDetection::On { interval: 100 })
        // Optional: Control how far ahead the game can predict (0 = lockstep)
        .with_max_prediction_window(8);

    println!("Builder configured: {:?}", builder);
    println!("  - 2 players with 2-frame input delay");
    println!("  - 60 FPS with 8-frame prediction window");
    println!("  - Desync detection enabled (every 100 frames)\n");
}

/// Using built-in presets for common network conditions
fn network_presets() {
    println!("--- Network Presets ---");

    // LAN/Local play - fast connections, minimal latency
    let lan_builder = SessionBuilder::<GameConfig>::new()
        .with_num_players(2)
        .with_sync_config(SyncConfig::lan())
        .with_protocol_config(ProtocolConfig::competitive())
        .with_time_sync_config(TimeSyncConfig::responsive());

    println!("LAN preset:");
    println!("  - Fast sync with fewer roundtrips");
    println!("  - Competitive timing (faster quality reports)");
    println!("  - Responsive time sync");
    println!("  Builder: {:?}\n", lan_builder);

    // High-latency networks (100-200ms RTT)
    let high_latency_builder = SessionBuilder::<GameConfig>::new()
        .with_num_players(2)
        .with_sync_config(SyncConfig::high_latency())
        .with_protocol_config(ProtocolConfig::high_latency())
        .with_time_sync_config(TimeSyncConfig::smooth())
        // Longer input delay helps with high latency
        .with_input_delay(4);

    println!("High-latency preset:");
    println!("  - Longer retry intervals to avoid flooding");
    println!("  - More tolerant warning thresholds");
    println!("  - Stable time sync (larger averaging window)");
    println!("  Builder: {:?}\n", high_latency_builder);

    // Lossy networks (5-15% packet loss)
    let lossy_builder = SessionBuilder::<GameConfig>::new()
        .with_num_players(2)
        .with_sync_config(SyncConfig::lossy())
        .with_protocol_config(ProtocolConfig::default())
        // Larger prediction window helps with packet loss
        .with_max_prediction_window(12);

    println!("Lossy network preset:");
    println!("  - More sync packets for reliable handshake");
    println!("  - Sync timeout to detect failures");
    println!("  - Larger prediction window");
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
        .with_num_players(2)
        // Minimal input delay for fastest response
        .with_input_delay(0)
        // Enable desync detection to catch cheating
        .with_desync_detection_mode(DesyncDetection::On { interval: 60 })
        // Use competitive presets
        .with_sync_config(SyncConfig::lan())
        .with_protocol_config(ProtocolConfig::competitive())
        .with_time_sync_config(TimeSyncConfig::responsive())
        // Moderate prediction window
        .with_max_prediction_window(6)
        // Fast disconnect detection
        .with_disconnect_timeout(Duration::from_millis(1500))
        .with_disconnect_notify_delay(Duration::from_millis(300))
        // High framerate for smooth gameplay
        .with_fps(120)
        .expect("FPS must be > 0");

    println!("Competitive setup:");
    println!("  - Zero input delay for fastest response");
    println!("  - Frequent desync detection (every 60 frames)");
    println!("  - Fast disconnect detection (1.5s timeout)");
    println!("  - 120 FPS target");
    println!("  Builder: {:?}\n", builder);
}

/// Configuration for casual online play
fn casual_online_setup() {
    println!("--- Casual Online Setup ---");

    let builder = SessionBuilder::<GameConfig>::new()
        .with_num_players(4) // Support up to 4 players
        // Moderate input delay for stability
        .with_input_delay(3)
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
        .with_num_players(2)
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
