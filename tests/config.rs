//! Integration tests for configuration structs.
//!
//! These tests verify that:
//! 1. All config structs have consistent APIs (Default, Copy, Clone, etc.)
//! 2. Sessions work correctly with default configs
//! 3. Sessions work correctly with custom configs
//! 4. Preset methods return sensible values
//! 5. Configs are properly applied to sessions
//!
//! # Port Allocation
//!
//! This test file uses ports **9100-9109**. When adding new tests that bind
//! to UDP ports, ensure they don't conflict with other test files:
//!
//! | Test File                     | Port Range      |
//! |-------------------------------|-----------------|
//! | tests/config.rs               | 9100-9109       |
//! | tests/sessions/p2p.rs         | 9100-9109, 19001+ |
//! | tests/network/resilience.rs   | 9001-9070, 9200-9299 |
//!
//! **Important**: Even with `#[serial]`, tests in different crates can run
//! in parallel. Choose non-overlapping port ranges to avoid "Address already
//! in use" errors in CI.

// Allow test-specific patterns that are appropriate for test code
#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::ip_constant
)]

// Shared test infrastructure
#[path = "common/mod.rs"]
mod common;

use common::stubs::StubConfig;
use fortress_rollback::{
    FortressError, PlayerHandle, PlayerType, ProtocolConfig, SessionBuilder, SpectatorConfig,
    SyncConfig, TimeSyncConfig, UdpNonBlockingSocket,
};
use serial_test::serial;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use web_time::Duration;

// ============================================================================
// SyncConfig Tests
// ============================================================================

#[test]
fn test_sync_config_default() {
    let config = SyncConfig::default();

    assert_eq!(config.num_sync_packets, 5);
    assert_eq!(config.sync_retry_interval, Duration::from_millis(200));
    assert_eq!(config.sync_timeout, None);
    assert_eq!(config.running_retry_interval, Duration::from_millis(200));
    assert_eq!(config.keepalive_interval, Duration::from_millis(200));
}

#[test]
fn test_sync_config_new_equals_default() {
    assert_eq!(SyncConfig::new(), SyncConfig::default());
}

#[test]
fn test_sync_config_presets() {
    // High latency preset should have longer intervals
    let high_latency = SyncConfig::high_latency();
    assert!(high_latency.sync_retry_interval > Duration::from_millis(200));
    assert!(high_latency.sync_timeout.is_some());

    // Lossy preset should have more sync packets
    let lossy = SyncConfig::lossy();
    assert!(lossy.num_sync_packets > 5);
    assert!(lossy.sync_timeout.is_some());

    // LAN preset should have shorter intervals
    let lan = SyncConfig::lan();
    assert!(lan.sync_retry_interval < Duration::from_millis(200));
    assert!(lan.num_sync_packets < 5);

    // Mobile preset should have more sync packets and longer intervals than high_latency
    let mobile = SyncConfig::mobile();
    assert!(mobile.num_sync_packets > high_latency.num_sync_packets);
    assert!(mobile.sync_retry_interval > Duration::from_millis(300));
    assert!(mobile.sync_timeout.is_some());
    // Mobile timeout should be longer than lossy
    assert!(mobile.sync_timeout.unwrap() > lossy.sync_timeout.unwrap());

    // Competitive preset should have fast intervals but strict timeout
    let competitive = SyncConfig::competitive();
    assert!(competitive.sync_retry_interval <= lan.sync_retry_interval);
    assert!(competitive.sync_timeout.is_some());
    // Competitive timeout should be shorter than lan
    assert!(competitive.sync_timeout.unwrap() < lan.sync_timeout.unwrap());
}

#[test]
fn test_sync_config_mobile_exact_values() {
    let mobile = SyncConfig::mobile();

    // Verify exact values for mobile preset
    assert_eq!(mobile.num_sync_packets, 10);
    assert_eq!(mobile.sync_retry_interval, Duration::from_millis(350));
    assert_eq!(mobile.sync_timeout, Some(Duration::from_secs(15)));
    assert_eq!(mobile.running_retry_interval, Duration::from_millis(350));
    assert_eq!(mobile.keepalive_interval, Duration::from_millis(300));
}

#[test]
fn test_sync_config_competitive_exact_values() {
    let competitive = SyncConfig::competitive();

    // Verify exact values for competitive preset
    assert_eq!(competitive.num_sync_packets, 4);
    assert_eq!(competitive.sync_retry_interval, Duration::from_millis(100));
    assert_eq!(competitive.sync_timeout, Some(Duration::from_secs(3)));
    assert_eq!(
        competitive.running_retry_interval,
        Duration::from_millis(100)
    );
    assert_eq!(competitive.keepalive_interval, Duration::from_millis(100));
}

#[test]
fn test_sync_config_copy_clone() {
    let config1 = SyncConfig::high_latency();

    // Test Copy
    let config2 = config1;
    assert_eq!(config1, config2);

    // Test Clone
    let config3 = config1;
    assert_eq!(config1, config3);
}

// ============================================================================
// ProtocolConfig Tests
// ============================================================================

#[test]
fn test_protocol_config_default() {
    let config = ProtocolConfig::default();

    assert_eq!(config.quality_report_interval, Duration::from_millis(200));
    assert_eq!(config.shutdown_delay, Duration::from_millis(5000));
    assert_eq!(config.max_checksum_history, 32);
    assert_eq!(config.pending_output_limit, 128);
    assert_eq!(config.sync_retry_warning_threshold, 10);
    assert_eq!(config.sync_duration_warning_ms, 3000);
}

#[test]
fn test_protocol_config_new_equals_default() {
    assert_eq!(ProtocolConfig::new(), ProtocolConfig::default());
}

#[test]
fn test_protocol_config_presets() {
    // Competitive preset should have faster quality reports
    let competitive = ProtocolConfig::competitive();
    assert!(competitive.quality_report_interval < Duration::from_millis(200));
    assert!(competitive.shutdown_delay < Duration::from_millis(5000));

    // High latency preset should have higher limits
    let high_latency = ProtocolConfig::high_latency();
    assert!(high_latency.pending_output_limit > 128);
    assert!(high_latency.sync_retry_warning_threshold > 10);

    // Debug preset should have lower thresholds for easier observation
    let debug = ProtocolConfig::debug();
    assert!(debug.sync_retry_warning_threshold < 10);
    assert!(debug.max_checksum_history > 32);

    // Mobile preset should have high tolerance for retries and long shutdown delay
    let mobile = ProtocolConfig::mobile();
    assert!(mobile.pending_output_limit >= high_latency.pending_output_limit);
    assert!(mobile.sync_retry_warning_threshold > high_latency.sync_retry_warning_threshold);
    assert!(mobile.shutdown_delay > high_latency.shutdown_delay);
    assert!(mobile.sync_duration_warning_ms > high_latency.sync_duration_warning_ms);
}

#[test]
fn test_protocol_config_mobile_exact_values() {
    let mobile = ProtocolConfig::mobile();

    // Verify exact values for mobile preset
    assert_eq!(mobile.quality_report_interval, Duration::from_millis(350));
    assert_eq!(mobile.shutdown_delay, Duration::from_millis(15000));
    assert_eq!(mobile.max_checksum_history, 64);
    assert_eq!(mobile.pending_output_limit, 256);
    assert_eq!(mobile.sync_retry_warning_threshold, 25);
    assert_eq!(mobile.sync_duration_warning_ms, 12000);
}

#[test]
fn test_protocol_config_copy_clone() {
    let config1 = ProtocolConfig::competitive();

    // Test Copy
    let config2 = config1;
    assert_eq!(config1, config2);

    // Test Clone
    let config3 = config1;
    assert_eq!(config1, config3);
}

// ============================================================================
// SpectatorConfig Tests
// ============================================================================

#[test]
fn test_spectator_config_default() {
    let config = SpectatorConfig::default();

    assert_eq!(config.buffer_size, 60);
    assert_eq!(config.catchup_speed, 1);
    assert_eq!(config.max_frames_behind, 10);
}

#[test]
fn test_spectator_config_new_equals_default() {
    assert_eq!(SpectatorConfig::new(), SpectatorConfig::default());
}

#[test]
fn test_spectator_config_presets() {
    // Fast-paced preset should have larger buffer and faster catchup
    let fast_paced = SpectatorConfig::fast_paced();
    assert!(fast_paced.buffer_size > 60);
    assert!(fast_paced.catchup_speed > 1);

    // Slow connection preset should have larger buffer
    let slow_connection = SpectatorConfig::slow_connection();
    assert!(slow_connection.buffer_size > 60);
    assert!(slow_connection.max_frames_behind > 10);

    // Local preset should have smaller buffer
    let local = SpectatorConfig::local();
    assert!(local.buffer_size < 60);
    assert!(local.max_frames_behind < 10);

    // Broadcast preset should have very large buffer for streaming
    let broadcast = SpectatorConfig::broadcast();
    assert!(broadcast.buffer_size > slow_connection.buffer_size);
    assert!(broadcast.max_frames_behind > slow_connection.max_frames_behind);
    // Broadcast should have slow catchup to avoid visual stuttering
    assert_eq!(broadcast.catchup_speed, 1);

    // Mobile preset should have larger buffer than default
    let mobile = SpectatorConfig::mobile();
    assert!(mobile.buffer_size > 60);
    assert!(mobile.max_frames_behind > 10);
    // Mobile should be between slow_connection and broadcast
    assert!(mobile.buffer_size <= broadcast.buffer_size);
}

#[test]
fn test_spectator_config_broadcast_exact_values() {
    let broadcast = SpectatorConfig::broadcast();

    // Verify exact values for broadcast preset
    // 3 seconds of buffer at 60 FPS
    assert_eq!(broadcast.buffer_size, 180);
    // Slow catchup to avoid stuttering on stream
    assert_eq!(broadcast.catchup_speed, 1);
    // Can fall far behind before catching up
    assert_eq!(broadcast.max_frames_behind, 30);
}

#[test]
fn test_spectator_config_mobile_exact_values() {
    let mobile = SpectatorConfig::mobile();

    // Verify exact values for mobile preset
    // 2 seconds of buffer at 60 FPS
    assert_eq!(mobile.buffer_size, 120);
    assert_eq!(mobile.catchup_speed, 1);
    assert_eq!(mobile.max_frames_behind, 25);
}

#[test]
fn test_spectator_config_copy_clone() {
    let config1 = SpectatorConfig::fast_paced();

    // Test Copy
    let config2 = config1;
    assert_eq!(config1, config2);

    // Test Clone
    let config3 = config1;
    assert_eq!(config1, config3);
}

// ============================================================================
// TimeSyncConfig Tests
// ============================================================================

#[test]
fn test_time_sync_config_default() {
    let config = TimeSyncConfig::default();

    assert_eq!(config.window_size, 30);
}

#[test]
fn test_time_sync_config_new_equals_default() {
    assert_eq!(TimeSyncConfig::new(), TimeSyncConfig::default());
}

#[test]
fn test_time_sync_config_presets() {
    // Responsive preset should have smaller window
    let responsive = TimeSyncConfig::responsive();
    assert!(responsive.window_size < 30);

    // Smooth preset should have larger window
    let smooth = TimeSyncConfig::smooth();
    assert!(smooth.window_size > 30);

    // LAN preset should have small window
    let lan = TimeSyncConfig::lan();
    assert!(lan.window_size < 30);

    // Mobile preset should have largest window for smoothing jitter
    let mobile = TimeSyncConfig::mobile();
    assert!(mobile.window_size > smooth.window_size);

    // Competitive preset should have smaller window than default for responsiveness
    let competitive = TimeSyncConfig::competitive();
    assert!(competitive.window_size < 30);
    // But larger than LAN (assumes slightly worse conditions)
    assert!(competitive.window_size > lan.window_size);
}

#[test]
fn test_time_sync_config_mobile_exact_values() {
    let mobile = TimeSyncConfig::mobile();

    // Verify exact value for mobile preset
    // Very large window to smooth out mobile network jitter
    assert_eq!(mobile.window_size, 90);
}

#[test]
fn test_time_sync_config_competitive_exact_values() {
    let competitive = TimeSyncConfig::competitive();

    // Verify exact value for competitive preset
    // Small window for responsive sync, but not as small as LAN
    assert_eq!(competitive.window_size, 20);
}

#[test]
fn test_time_sync_config_copy_clone() {
    let config1 = TimeSyncConfig::responsive();

    // Test Copy
    let config2 = config1;
    assert_eq!(config1, config2);

    // Test Clone
    let config3 = config1;
    assert_eq!(config1, config3);
}

// ============================================================================
// SessionBuilder Config Integration Tests
// ============================================================================

#[test]
#[serial]
fn test_session_with_default_configs() -> Result<(), FortressError> {
    // A session built with no explicit config should work with defaults
    let socket = UdpNonBlockingSocket::bind_to_port(9100).unwrap();
    let remote_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9101);

    let _sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(remote_addr), PlayerHandle::new(1))?
        .start_p2p_session(socket)?;

    Ok(())
}

#[test]
#[serial]
fn test_session_with_all_custom_configs() -> Result<(), FortressError> {
    // A session built with explicit configs for everything should also work
    let socket = UdpNonBlockingSocket::bind_to_port(9102).unwrap();
    let remote_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9103);

    let _sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .with_sync_config(SyncConfig::high_latency())
        .with_protocol_config(ProtocolConfig::competitive())
        .with_spectator_config(SpectatorConfig::fast_paced())
        .with_time_sync_config(TimeSyncConfig::responsive())
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(remote_addr), PlayerHandle::new(1))?
        .start_p2p_session(socket)?;

    Ok(())
}

#[test]
#[serial]
fn test_session_with_mixed_configs() -> Result<(), FortressError> {
    // A session built with some explicit and some default configs
    let socket = UdpNonBlockingSocket::bind_to_port(9104).unwrap();
    let remote_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9105);

    let _sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        // Only set sync and time sync config, leave others at defaults
        .with_sync_config(SyncConfig::lan())
        .with_time_sync_config(TimeSyncConfig::lan())
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(remote_addr), PlayerHandle::new(1))?
        .start_p2p_session(socket)?;

    Ok(())
}

#[test]
#[serial]
fn test_session_with_custom_protocol_config() -> Result<(), FortressError> {
    // Test custom field values (not using a preset)
    let socket = UdpNonBlockingSocket::bind_to_port(9106).unwrap();
    let remote_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9107);

    let custom_protocol_config = ProtocolConfig {
        quality_report_interval: Duration::from_millis(150),
        shutdown_delay: Duration::from_millis(4000),
        max_checksum_history: 64,
        pending_output_limit: 200,
        // Leave some fields to default to demonstrate forward-compatible pattern
        ..Default::default()
    };

    let _sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .with_protocol_config(custom_protocol_config)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(remote_addr), PlayerHandle::new(1))?
        .start_p2p_session(socket)?;

    Ok(())
}

#[test]
#[serial]
fn test_session_with_custom_sync_config() -> Result<(), FortressError> {
    // Test custom field values (not using a preset)
    let socket = UdpNonBlockingSocket::bind_to_port(9108).unwrap();
    let remote_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9109);

    let custom_sync_config = SyncConfig {
        num_sync_packets: 7,
        sync_retry_interval: Duration::from_millis(250),
        sync_timeout: Some(Duration::from_secs(8)),
        // Leave some fields to default to demonstrate forward-compatible pattern
        ..Default::default()
    };

    let _sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .with_sync_config(custom_sync_config)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(remote_addr), PlayerHandle::new(1))?
        .start_p2p_session(socket)?;

    Ok(())
}

#[test]
#[serial]
fn test_synctest_session_with_default_configs() -> Result<(), FortressError> {
    // SyncTest session should also work with default configs
    let _sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_synctest_session()?;

    Ok(())
}

// ============================================================================
// Config Consistency Tests
// ============================================================================

#[test]
fn test_all_configs_have_consistent_api() {
    // All configs should have:
    // 1. Default implementation
    // 2. new() constructor that equals default
    // 3. At least one preset method
    // 4. Copy + Clone traits

    // SyncConfig
    let _: SyncConfig = SyncConfig::default();
    let _: SyncConfig = SyncConfig::new();
    let _: SyncConfig = SyncConfig::high_latency();
    let _: SyncConfig = SyncConfig::lossy();
    let _: SyncConfig = SyncConfig::lan();
    let _: SyncConfig = SyncConfig::mobile();
    let _: SyncConfig = SyncConfig::competitive();

    // ProtocolConfig
    let _: ProtocolConfig = ProtocolConfig::default();
    let _: ProtocolConfig = ProtocolConfig::new();
    let _: ProtocolConfig = ProtocolConfig::competitive();
    let _: ProtocolConfig = ProtocolConfig::high_latency();
    let _: ProtocolConfig = ProtocolConfig::debug();
    let _: ProtocolConfig = ProtocolConfig::mobile();

    // SpectatorConfig
    let _: SpectatorConfig = SpectatorConfig::default();
    let _: SpectatorConfig = SpectatorConfig::new();
    let _: SpectatorConfig = SpectatorConfig::fast_paced();
    let _: SpectatorConfig = SpectatorConfig::slow_connection();
    let _: SpectatorConfig = SpectatorConfig::local();
    let _: SpectatorConfig = SpectatorConfig::broadcast();
    let _: SpectatorConfig = SpectatorConfig::mobile();

    // TimeSyncConfig
    let _: TimeSyncConfig = TimeSyncConfig::default();
    let _: TimeSyncConfig = TimeSyncConfig::new();
    let _: TimeSyncConfig = TimeSyncConfig::responsive();
    let _: TimeSyncConfig = TimeSyncConfig::smooth();
    let _: TimeSyncConfig = TimeSyncConfig::lan();
    let _: TimeSyncConfig = TimeSyncConfig::mobile();
    let _: TimeSyncConfig = TimeSyncConfig::competitive();
}

#[test]
fn test_all_configs_are_debug() {
    // All configs should implement Debug for easier troubleshooting
    let sync_config = SyncConfig::default();
    let protocol_config = ProtocolConfig::default();
    let spectator_config = SpectatorConfig::default();
    let time_sync_config = TimeSyncConfig::default();

    // This should compile - testing that Debug is implemented
    let _ = format!("{:?}", sync_config);
    let _ = format!("{:?}", protocol_config);
    let _ = format!("{:?}", spectator_config);
    let _ = format!("{:?}", time_sync_config);
}

#[test]
fn test_all_configs_are_eq() {
    // All configs should implement PartialEq and Eq
    assert_eq!(SyncConfig::default(), SyncConfig::new());
    assert_ne!(SyncConfig::default(), SyncConfig::high_latency());

    assert_eq!(ProtocolConfig::default(), ProtocolConfig::new());
    assert_ne!(ProtocolConfig::default(), ProtocolConfig::competitive());

    assert_eq!(SpectatorConfig::default(), SpectatorConfig::new());
    assert_ne!(SpectatorConfig::default(), SpectatorConfig::fast_paced());

    assert_eq!(TimeSyncConfig::default(), TimeSyncConfig::new());
    assert_ne!(TimeSyncConfig::default(), TimeSyncConfig::responsive());
}
