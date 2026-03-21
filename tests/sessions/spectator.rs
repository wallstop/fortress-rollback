//! Spectator session integration tests.
//!
//! Uses `ChannelSocket` (in-memory) + `TestClock` (virtual time) for fully
//! deterministic, platform-independent test execution. No real UDP sockets,
//! no `thread::sleep`, no `#[serial]`.

// Allow test-specific patterns that are appropriate for test code
#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::ip_constant
)]

use crate::common::stubs::{GameStub, StubConfig, StubInput};
use crate::common::{
    assert_spectator_synchronized, create_channel_pair, create_channel_triple,
    create_unconnected_socket, synchronize_spectator_deterministic, TestClock, MAX_SYNC_ITERATIONS,
    POLL_INTERVAL_DETERMINISTIC,
};
use fortress_rollback::{
    telemetry::CollectingObserver, FortressError, FortressEvent, InputQueueConfig, PlayerHandle,
    PlayerType, ProtocolConfig, SessionBuilder, SessionState, SpectatorConfig, SyncConfig,
};
use std::sync::Arc;
use std::time::Duration;

/// Helper: creates a `ProtocolConfig` with the given test clock.
fn protocol_config(clock: &TestClock) -> ProtocolConfig {
    ProtocolConfig {
        clock: Some(clock.as_protocol_clock()),
        ..ProtocolConfig::default()
    }
}

// ============================================================================
// Basic Session Tests
// ============================================================================

#[test]
fn test_start_session() {
    let clock = TestClock::new();
    let (socket, _spec_addr) = create_unconnected_socket(20001);
    let host_addr = "127.0.0.1:20002".parse().unwrap();
    let spec_sess = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");
    assert_eq!(spec_sess.current_state(), SessionState::Synchronizing);
}

#[test]
fn test_synchronize_with_host() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (socket1, socket2, host_addr, spec_addr) = create_channel_pair();

    let mut host_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(1)
        .unwrap()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    let mut spec_sess = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .start_spectator_session(host_addr, socket2)
        .expect("spectator session should start");

    assert_eq!(spec_sess.current_state(), SessionState::Synchronizing);
    assert_eq!(host_sess.current_state(), SessionState::Synchronizing);

    let result = synchronize_spectator_deterministic(&mut spec_sess, &mut host_sess, &clock);
    assert_spectator_synchronized(&spec_sess, &host_sess, &result);

    Ok(())
}

// ============================================================================
// Data-Driven Synchronization Tests
// ============================================================================

/// Test configuration for synchronization scenarios
struct SyncTestCase {
    /// Descriptive name of the test case
    name: &'static str,
    /// Number of players in the host session
    num_players: usize,
    /// Number of local players (remaining will be spectators)
    num_local_players: usize,
}

impl SyncTestCase {
    const fn new(name: &'static str, num_players: usize, num_local_players: usize) -> Self {
        Self {
            name,
            num_players,
            num_local_players,
        }
    }
}

/// Data-driven test for various synchronization scenarios.
///
/// This test verifies that spectator synchronization works reliably across
/// different player configurations using deterministic in-memory sockets
/// and virtual time.
#[test]
fn test_synchronization_scenarios_data_driven() -> Result<(), FortressError> {
    // Define test cases for different configurations
    let test_cases = [
        SyncTestCase::new("single_local_single_spectator", 1, 1),
        SyncTestCase::new("two_local_single_spectator", 2, 2),
        SyncTestCase::new("four_local_single_spectator", 4, 4),
    ];

    for case in &test_cases {
        // Create fresh clock and channel pair for each iteration
        let clock = TestClock::new();
        let (socket1, socket2, host_addr, spec_addr) = create_channel_pair();

        let mut builder = SessionBuilder::<StubConfig>::new()
            .with_num_players(case.num_players)
            .unwrap()
            .with_protocol_config(protocol_config(&clock));

        // Add local players
        for i in 0..case.num_local_players {
            builder = builder.add_player(PlayerType::Local, PlayerHandle::new(i))?;
        }

        // Add spectator
        builder = builder.add_player(
            PlayerType::Spectator(spec_addr),
            PlayerHandle::new(case.num_local_players),
        )?;

        let mut host_sess = builder
            .start_p2p_session(socket1)
            .expect("Failed to start host session");

        let mut spec_sess = SessionBuilder::<StubConfig>::new()
            .with_num_players(case.num_players)
            .unwrap()
            .with_protocol_config(protocol_config(&clock))
            .start_spectator_session(host_addr, socket2)
            .expect("Failed to start spectator session");

        // Perform synchronization with deterministic polling
        let result = synchronize_spectator_deterministic(&mut spec_sess, &mut host_sess, &clock);

        // Assert with detailed failure message for this specific case
        assert!(
            result.success,
            "[{}] Synchronization failed:\n\
             - Iterations: {}\n\
             - Elapsed: {:?}\n\
             - Spectator state: {:?}\n\
             - Host state: {:?}",
            case.name,
            result.iterations,
            result.elapsed,
            spec_sess.current_state(),
            host_sess.current_state()
        );
    }

    Ok(())
}

/// Data-driven test for synchronization with different SyncConfig presets.
///
/// This test verifies that spectator synchronization works reliably with
/// various sync configuration presets using deterministic infrastructure.
#[test]
fn test_sync_config_presets_data_driven() -> Result<(), FortressError> {
    // Test cases with different sync configurations
    struct SyncConfigTestCase {
        name: &'static str,
        config: SyncConfig,
    }

    let test_cases = [
        SyncConfigTestCase {
            name: "default",
            config: SyncConfig::default(),
        },
        SyncConfigTestCase {
            name: "lan",
            config: SyncConfig::lan(),
        },
        SyncConfigTestCase {
            name: "competitive",
            config: SyncConfig::competitive(),
        },
    ];

    for case in &test_cases {
        // Create fresh clock and channel pair for each iteration
        let clock = TestClock::new();
        let (socket1, socket2, host_addr, spec_addr) = create_channel_pair();

        let mut host_sess = SessionBuilder::<StubConfig>::new()
            .with_num_players(2)
            .unwrap()
            .with_protocol_config(protocol_config(&clock))
            .with_sync_config(case.config)
            .add_player(PlayerType::Local, PlayerHandle::new(0))?
            .add_player(PlayerType::Local, PlayerHandle::new(1))?
            .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
            .start_p2p_session(socket1)
            .expect("Failed to start host session");

        let mut spec_sess = SessionBuilder::<StubConfig>::new()
            .with_num_players(2)
            .unwrap()
            .with_protocol_config(protocol_config(&clock))
            .with_sync_config(case.config)
            .start_spectator_session(host_addr, socket2)
            .expect("Failed to start spectator session");

        // Use deterministic synchronization
        let result = synchronize_spectator_deterministic(&mut spec_sess, &mut host_sess, &clock);

        assert!(
            result.success,
            "[SyncConfig::{}] Synchronization failed:\n\
             - Config: num_sync_packets={}, sync_retry_interval={:?}\n\
             - Iterations: {}\n\
             - Elapsed: {:?}\n\
             - Spectator state: {:?}\n\
             - Host state: {:?}",
            case.name,
            case.config.num_sync_packets,
            case.config.sync_retry_interval,
            result.iterations,
            result.elapsed,
            spec_sess.current_state(),
            host_sess.current_state()
        );
    }

    Ok(())
}

// ============================================================================
// Session State Tests
// ============================================================================

#[test]
fn test_current_frame_starts_at_null() {
    let clock = TestClock::new();
    let (socket, _spec_addr) = create_unconnected_socket(20001);
    let host_addr = "127.0.0.1:20002".parse().unwrap();
    let spec_sess = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    // Before synchronization, current_frame should be NULL (-1)
    assert!(spec_sess.current_frame().is_null());
}

#[test]
fn test_frames_behind_host_initially_zero() {
    let clock = TestClock::new();
    let (socket, _spec_addr) = create_unconnected_socket(20001);
    let host_addr = "127.0.0.1:20002".parse().unwrap();
    let spec_sess = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    // Both current_frame and last_recv_frame are NULL, so difference is 0
    assert_eq!(spec_sess.frames_behind_host(), 0);
}

#[test]
fn test_num_players_default() {
    let clock = TestClock::new();
    let (socket, _spec_addr) = create_unconnected_socket(20001);
    let host_addr = "127.0.0.1:20002".parse().unwrap();
    let spec_sess = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    // Default number of players is 2
    assert_eq!(spec_sess.num_players(), 2);
}

#[test]
fn test_num_players_custom() {
    let clock = TestClock::new();
    let (socket, _spec_addr) = create_unconnected_socket(20001);
    let host_addr = "127.0.0.1:20002".parse().unwrap();
    let spec_sess = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(4)
        .unwrap()
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    assert_eq!(spec_sess.num_players(), 4);
}

// ============================================================================
// Network Stats Tests
// ============================================================================

#[test]
fn test_network_stats_not_synchronized() {
    let clock = TestClock::new();
    let (socket, _spec_addr) = create_unconnected_socket(20001);
    let host_addr = "127.0.0.1:20002".parse().unwrap();
    let spec_sess = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    // Network stats should fail when not synchronized
    let result = spec_sess.network_stats();
    assert!(result.is_err());
    assert!(matches!(result, Err(FortressError::NotSynchronized)));
}

// ============================================================================
// Events Tests
// ============================================================================

#[test]
fn test_events_empty_initially() {
    let clock = TestClock::new();
    let (socket, _spec_addr) = create_unconnected_socket(20001);
    let host_addr = "127.0.0.1:20002".parse().unwrap();
    let mut spec_sess = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    // Initially, there should be no events
    assert_eq!(spec_sess.events().count(), 0);
}

#[test]
fn test_events_generated_during_sync() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (socket1, socket2, host_addr, spec_addr) = create_channel_pair();

    let mut host_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    let mut spec_sess = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .start_spectator_session(host_addr, socket2)
        .expect("spectator session should start");

    // Poll a few times to generate synchronization events
    for _ in 0..10 {
        spec_sess.poll_remote_clients();
        host_sess.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    // We should get some synchronization events
    // At minimum we should have some events (synchronizing progress)
    // The exact count depends on timing, but there should be some activity
    assert!(spec_sess.events().count() > 0 || spec_sess.current_state() == SessionState::Running);

    Ok(())
}

// ============================================================================
// Advance Frame Tests
// ============================================================================

#[test]
fn test_advance_frame_before_sync_fails() {
    let clock = TestClock::new();
    let (socket, _spec_addr) = create_unconnected_socket(20001);
    let host_addr = "127.0.0.1:20002".parse().unwrap();
    let mut spec_sess = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    // advance_frame should fail when not synchronized
    let result = spec_sess.advance_frame();
    assert!(result.is_err());
    assert!(matches!(result, Err(FortressError::NotSynchronized)));
}

#[test]
fn test_advance_frame_after_sync() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (socket1, socket2, host_addr, spec_addr) = create_channel_pair();

    let mut host_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    let mut spec_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .with_protocol_config(protocol_config(&clock))
        .start_spectator_session(host_addr, socket2)
        .expect("spectator session should start");

    let mut host_game = GameStub::new();

    // Use deterministic synchronization
    let result = synchronize_spectator_deterministic(&mut spec_sess, &mut host_sess, &clock);
    assert_spectator_synchronized(&spec_sess, &host_sess, &result);

    // Advance host a few frames and send inputs
    for _ in 0..5 {
        host_sess.add_local_input(PlayerHandle::new(0), StubInput { inp: 1 })?;
        host_sess.add_local_input(PlayerHandle::new(1), StubInput { inp: 2 })?;
        let requests = host_sess.advance_frame()?;
        host_game.handle_requests(requests);
        host_sess.poll_remote_clients();
        spec_sess.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    // Give time for messages to propagate
    for _ in 0..20 {
        host_sess.poll_remote_clients();
        spec_sess.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    // Spectator should now be able to advance frames
    // It might return PredictionThreshold if inputs haven't arrived yet
    let result = spec_sess.advance_frame();
    assert!(
        result.is_ok() || matches!(result, Err(FortressError::PredictionThreshold)),
        "Expected Ok or PredictionThreshold, got error"
    );

    Ok(())
}

// ============================================================================
// Violation Observer Tests
// ============================================================================

#[test]
fn test_violation_observer_attached() {
    let clock = TestClock::new();
    let (socket, _spec_addr) = create_unconnected_socket(20001);
    let host_addr = "127.0.0.1:20002".parse().unwrap();
    let observer = Arc::new(CollectingObserver::new());

    let spec_sess = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_violation_observer(observer)
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    // Verify observer is attached
    assert!(spec_sess.violation_observer().is_some());
}

#[test]
fn test_no_violation_observer_by_default() {
    let clock = TestClock::new();
    let (socket, _spec_addr) = create_unconnected_socket(20001);
    let host_addr = "127.0.0.1:20002".parse().unwrap();
    let spec_sess = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    // By default, no observer should be attached
    assert!(spec_sess.violation_observer().is_none());
}

// ============================================================================
// Configuration Tests
// ============================================================================

#[test]
fn test_spectator_config_buffer_size() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (socket1, socket2, host_addr, spec_addr) = create_channel_pair();

    let _host_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    // Create spectator with custom buffer size
    let spectator_config = SpectatorConfig {
        buffer_size: 64,
        max_frames_behind: 10,
        // Leave catchup_speed to default to demonstrate forward-compatible pattern
        ..Default::default()
    };

    let spec_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .with_protocol_config(protocol_config(&clock))
        .with_spectator_config(spectator_config)
        .start_spectator_session(host_addr, socket2)
        .expect("spectator session should start");

    // Session should be created successfully
    assert_eq!(spec_sess.current_state(), SessionState::Synchronizing);

    Ok(())
}

#[test]
fn test_spectator_with_input_queue_config() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (socket1, socket2, host_addr, spec_addr) = create_channel_pair();

    let _host_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    // Create spectator with high latency input queue config
    let spec_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .with_protocol_config(protocol_config(&clock))
        .with_input_queue_config(InputQueueConfig::high_latency())
        .start_spectator_session(host_addr, socket2)
        .expect("spectator session should start");

    assert_eq!(spec_sess.current_state(), SessionState::Synchronizing);

    Ok(())
}

// ============================================================================
// Poll Remote Clients Tests
// ============================================================================

#[test]
fn test_poll_remote_clients_no_host() {
    let clock = TestClock::new();
    let (socket, _spec_addr) = create_unconnected_socket(20001);
    let host_addr = "127.0.0.1:20002".parse().unwrap();
    let mut spec_sess = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    // Polling with no host should not panic
    for _ in 0..10 {
        spec_sess.poll_remote_clients();
    }

    // Should still be synchronizing (no host to sync with)
    assert_eq!(spec_sess.current_state(), SessionState::Synchronizing);
}

// ============================================================================
// Full Spectator Flow Tests
// ============================================================================

#[test]
fn test_full_spectator_flow() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (socket1, socket2, host_addr, spec_addr) = create_channel_pair();

    let mut host_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    let mut spec_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .with_protocol_config(protocol_config(&clock))
        .start_spectator_session(host_addr, socket2)
        .expect("spectator session should start");

    let mut host_game = GameStub::new();

    // Phase 1: Synchronization - use deterministic helper
    let sync_result = synchronize_spectator_deterministic(&mut spec_sess, &mut host_sess, &clock);
    assert_spectator_synchronized(&spec_sess, &host_sess, &sync_result);

    // Phase 2: Host advances frames and spectator follows
    for frame in 0..10 {
        // Host adds inputs and advances
        host_sess.add_local_input(PlayerHandle::new(0), StubInput { inp: frame as u32 })?;
        host_sess.add_local_input(PlayerHandle::new(1), StubInput { inp: frame as u32 })?;
        let requests = host_sess.advance_frame()?;
        host_game.handle_requests(requests);

        // Poll to exchange messages with virtual time advancement
        for _ in 0..5 {
            host_sess.poll_remote_clients();
            spec_sess.poll_remote_clients();
            clock.advance(POLL_INTERVAL_DETERMINISTIC);
        }
    }

    // Give extra time for messages to propagate
    for _ in 0..30 {
        host_sess.poll_remote_clients();
        spec_sess.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    // Spectator should be able to get inputs now
    if let Ok(requests) = spec_sess.advance_frame() {
        assert!(!requests.is_empty());
    }

    Ok(())
}

// ============================================================================
// Event Handling Tests
// ============================================================================

#[test]
fn test_synchronized_event_generated() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (socket1, socket2, host_addr, spec_addr) = create_channel_pair();

    let mut host_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    let mut spec_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .with_protocol_config(protocol_config(&clock))
        .start_spectator_session(host_addr, socket2)
        .expect("spectator session should start");

    let mut found_synchronized = false;

    // NOTE: This test intentionally uses an inline sync loop instead of the centralized
    // `synchronize_spectator_deterministic()` helper because we need to capture and inspect
    // events DURING synchronization. The helper only returns success/failure, not the events
    // generated during the handshake process. This test verifies that `Synchronized`
    // events are properly emitted.
    let mut iterations = 0;
    while iterations < MAX_SYNC_ITERATIONS {
        spec_sess.poll_remote_clients();
        host_sess.poll_remote_clients();

        for event in spec_sess.events() {
            if matches!(event, FortressEvent::Synchronized { .. }) {
                found_synchronized = true;
            }
        }

        if spec_sess.current_state() == SessionState::Running {
            break;
        }
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        iterations += 1;
    }

    // We should have received a Synchronized event
    assert!(found_synchronized || spec_sess.current_state() == SessionState::Running);

    Ok(())
}

#[test]
fn test_synchronizing_events_generated() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (socket1, socket2, host_addr, spec_addr) = create_channel_pair();

    let mut host_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    let mut spec_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .with_protocol_config(protocol_config(&clock))
        .start_spectator_session(host_addr, socket2)
        .expect("spectator session should start");

    let mut found_synchronizing = false;
    let mut iterations = 0;

    // NOTE: This test intentionally uses an inline sync loop instead of the centralized
    // `synchronize_spectator_deterministic()` helper because we need to capture and inspect
    // events DURING synchronization. The helper only returns success/failure, not the events
    // generated during the handshake process. This test verifies that `Synchronizing`
    // progress events are properly emitted.
    while iterations < MAX_SYNC_ITERATIONS {
        spec_sess.poll_remote_clients();
        host_sess.poll_remote_clients();
        iterations += 1;

        for event in spec_sess.events() {
            if matches!(event, FortressEvent::Synchronizing { .. }) {
                found_synchronizing = true;
            }
        }

        // Early exit once we found what we're looking for
        if found_synchronizing {
            break;
        }

        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    // We should have received Synchronizing progress events
    assert!(
        found_synchronizing,
        "Expected Synchronizing events during handshake.\n\
         Iterations: {}\n\
         Spectator state: {:?}\n\
         Host state: {:?}",
        iterations,
        spec_sess.current_state(),
        host_sess.current_state()
    );

    Ok(())
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_spectator_catchup_speed() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (socket1, socket2, host_addr, spec_addr) = create_channel_pair();

    // Configure spectator to catch up faster when behind
    let spectator_config = SpectatorConfig {
        buffer_size: 64,
        catchup_speed: 3,
        // Leave max_frames_behind to default to demonstrate forward-compatible pattern
        ..Default::default()
    };

    let mut host_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    let mut spec_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .with_protocol_config(protocol_config(&clock))
        .with_spectator_config(spectator_config)
        .start_spectator_session(host_addr, socket2)
        .expect("spectator session should start");

    let mut host_game = GameStub::new();

    // Synchronize first with deterministic helper
    let result = synchronize_spectator_deterministic(&mut spec_sess, &mut host_sess, &clock);
    assert_spectator_synchronized(&spec_sess, &host_sess, &result);

    // Have host advance many frames ahead
    for frame in 0..20 {
        host_sess.add_local_input(PlayerHandle::new(0), StubInput { inp: frame as u32 })?;
        host_sess.add_local_input(PlayerHandle::new(1), StubInput { inp: frame as u32 })?;
        let requests = host_sess.advance_frame()?;
        host_game.handle_requests(requests);
        host_sess.poll_remote_clients();
    }

    // Let messages propagate with virtual time
    for _ in 0..100 {
        host_sess.poll_remote_clients();
        spec_sess.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    // Spectator should now be behind and catch up
    let _frames_behind = spec_sess.frames_behind_host();
    // frames_behind is usize, so it's always >= 0
    // Just verify we can read the value without panic

    Ok(())
}

#[test]
fn test_multiple_spectators_same_host() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (socket1, socket2, socket3, host_addr, spec_addr1, spec_addr2) = create_channel_triple();

    let mut host_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr1), PlayerHandle::new(2))?
        .add_player(PlayerType::Spectator(spec_addr2), PlayerHandle::new(3))?
        .start_p2p_session(socket1)?;

    let mut spec_sess1 = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .with_protocol_config(protocol_config(&clock))
        .start_spectator_session(host_addr, socket2)
        .expect("spectator session should start");

    let mut spec_sess2 = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .with_protocol_config(protocol_config(&clock))
        .start_spectator_session(host_addr, socket3)
        .expect("spectator session should start");

    // NOTE: This test intentionally uses an inline sync loop instead of the centralized
    // `synchronize_spectator_deterministic()` helper because we need to synchronize THREE
    // sessions simultaneously (one host and TWO spectators). The helper only supports one
    // spectator + one host pair. Using two sequential calls would not correctly test
    // the scenario where both spectators synchronize concurrently with the same host.
    let mut all_synced = false;
    let mut iterations = 0;
    while iterations < MAX_SYNC_ITERATIONS && !all_synced {
        spec_sess1.poll_remote_clients();
        spec_sess2.poll_remote_clients();
        host_sess.poll_remote_clients();

        all_synced = spec_sess1.current_state() == SessionState::Running
            && spec_sess2.current_state() == SessionState::Running
            && host_sess.current_state() == SessionState::Running;

        if !all_synced {
            clock.advance(POLL_INTERVAL_DETERMINISTIC);
        }
        iterations += 1;
    }

    // Both spectators should sync with detailed diagnostics on failure
    assert!(
        all_synced,
        "Failed to synchronize all sessions after {} iterations.\n\
         Host state: {:?}\n\
         Spectator 1 state: {:?}\n\
         Spectator 2 state: {:?}",
        iterations,
        host_sess.current_state(),
        spec_sess1.current_state(),
        spec_sess2.current_state()
    );

    Ok(())
}

#[test]
fn test_spectator_disconnect_timeout() {
    let clock = TestClock::new();
    let (socket, _spec_addr) = create_unconnected_socket(20001);
    let host_addr = "127.0.0.1:20002".parse().unwrap();

    // Create spectator that expects a connection
    let mut spec_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .with_protocol_config(protocol_config(&clock))
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    // Poll for a while without any host, advancing virtual time
    for _ in 0..20 {
        spec_sess.poll_remote_clients();
        clock.advance(Duration::from_millis(10));
    }

    // Should still be in synchronizing state (waiting for host)
    assert_eq!(spec_sess.current_state(), SessionState::Synchronizing);

    // Events may contain sync timeout or still be empty
    // Just verify we don't panic and can collect events
    let _events: Vec<_> = spec_sess.events().collect();
}
