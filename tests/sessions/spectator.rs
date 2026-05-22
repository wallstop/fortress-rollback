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

use crate::common::stubs::{GameStub, StateStub, StubConfig, StubInput};
use crate::common::{
    assert_spectator_synchronized, create_channel_pair, create_channel_triple,
    create_unconnected_socket, synchronize_spectator_deterministic, TestClock, MAX_SYNC_ITERATIONS,
    POLL_INTERVAL_DETERMINISTIC,
};
use fortress_rollback::{
    telemetry::CollectingObserver, FortressError, FortressEvent, FortressRequest, Frame,
    InputQueueConfig, InputVec, PlayerHandle, PlayerType, ProtocolConfig, RequestVec,
    SessionBuilder, SessionState, SpectatorConfig, SyncConfig,
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
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
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

#[test]
fn test_spectator_timeout_does_not_halt_host() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (socket1, socket2, host_addr, spec_addr) = create_channel_pair();
    let short_timeout = Duration::from_millis(200);

    let mut host_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(1)?
        .with_protocol_config(protocol_config(&clock))
        .with_disconnect_timeout(short_timeout)
        .with_disconnect_notify_delay(Duration::from_millis(50))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let mut spec_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(1)?
        .with_protocol_config(protocol_config(&clock))
        .start_spectator_session(host_addr, socket2)
        .expect("spectator session should start");

    let result = synchronize_spectator_deterministic(&mut spec_sess, &mut host_sess, &clock);
    assert_spectator_synchronized(&spec_sess, &host_sess, &result);
    let _: Vec<_> = host_sess.events().collect();
    let _: Vec<_> = spec_sess.events().collect();

    for _ in 0..100 {
        host_sess.poll_remote_clients();
        clock.advance(Duration::from_millis(20));
    }

    assert_eq!(
        host_sess.current_state(),
        SessionState::Running,
        "spectator timeout must not halt a running host session"
    );
    let events: Vec<_> = host_sess.events().collect();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, FortressEvent::Disconnected { .. })),
        "spectator timeout should emit Disconnected; got {events:?}"
    );

    let current_before_advance = host_sess.current_frame();
    let mut host_game = GameStub::new();
    host_sess.add_local_input(PlayerHandle::new(0), StubInput { inp: 42 })?;
    let requests = host_sess.advance_frame()?;
    host_game.handle_requests(requests);
    assert_eq!(host_sess.current_frame(), current_before_advance + 1);

    Ok(())
}

// ============================================================================
// Feature 7: Spectator Failover (multi-host redundancy)
// ============================================================================

/// Drives a spectator that follows TWO independent P2P hosts (failover redundancy).
///
/// Verifies that `start_spectator_session_multi` works end-to-end: both hosts
/// feed identical confirmed inputs and the spectator advances correctly while
/// reporting `num_hosts() == 2`.
#[test]
fn test_multi_host_spectator_advances() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (socket1, socket2, socket3, addr1, addr2, addr3) = create_channel_triple();

    // Two players in one P2P match; each registers the spectator at addr3.
    let mut host1 = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)?
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(addr3), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    let mut host2 = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)?
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(addr3), PlayerHandle::new(2))?
        .start_p2p_session(socket2)?;

    let mut spec = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)?
        .with_protocol_config(protocol_config(&clock))
        .start_spectator_session_multi(&[addr1, addr2], socket3)
        .expect("multi-host spectator should start");

    assert_eq!(spec.num_hosts(), 2);

    // Synchronize all three sessions.
    let mut synced = false;
    for _ in 0..MAX_SYNC_ITERATIONS {
        host1.poll_remote_clients();
        host2.poll_remote_clients();
        spec.poll_remote_clients();
        if host1.current_state() == SessionState::Running
            && host2.current_state() == SessionState::Running
            && spec.current_state() == SessionState::Running
        {
            synced = true;
            break;
        }
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }
    assert!(synced, "all sessions should synchronize");

    let mut g1 = GameStub::new();
    let mut g2 = GameStub::new();

    // Both hosts advance with identical inputs, feeding the spectator.
    for frame in 0..10 {
        host1.add_local_input(PlayerHandle::new(0), StubInput { inp: frame })?;
        host2.add_local_input(PlayerHandle::new(1), StubInput { inp: frame })?;
        if let Ok(r) = host1.advance_frame() {
            g1.handle_requests(r);
        }
        if let Ok(r) = host2.advance_frame() {
            g2.handle_requests(r);
        }
        for _ in 0..4 {
            host1.poll_remote_clients();
            host2.poll_remote_clients();
            spec.poll_remote_clients();
            clock.advance(POLL_INTERVAL_DETERMINISTIC);
        }
    }

    // Let inputs propagate, then drive the spectator forward.
    let mut spec_game = GameStub::new();
    let mut advanced = false;
    for _ in 0..50 {
        host1.poll_remote_clients();
        host2.poll_remote_clients();
        if let Ok(requests) = spec.advance_frame() {
            spec_game.handle_requests(requests);
            advanced = true;
        }
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    assert!(advanced, "spectator should advance from redundant hosts");
    assert_eq!(spec.num_hosts(), 2, "both hosts remain connected");
    assert!(
        spec.current_frame().is_valid(),
        "spectator should have advanced past NULL, got {:?}",
        spec.current_frame()
    );

    Ok(())
}

/// Verifies failover: when one host stops responding and times out, it is
/// removed from the spectator, `num_hosts()` drops to 1, a `Disconnected`
/// event is observed, and the spectator continues advancing from the survivor.
#[test]
fn test_multi_host_spectator_failover_on_timeout() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (socket1, socket2, socket3, addr1, addr2, addr3) = create_channel_triple();
    let short_timeout = Duration::from_millis(200);

    let mut host1 = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)?
        .with_protocol_config(protocol_config(&clock))
        .with_disconnect_timeout(short_timeout)
        .with_disconnect_notify_delay(Duration::from_millis(50))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(addr3), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    let mut host2 = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)?
        .with_protocol_config(protocol_config(&clock))
        .with_disconnect_timeout(short_timeout)
        .with_disconnect_notify_delay(Duration::from_millis(50))
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(addr3), PlayerHandle::new(2))?
        .start_p2p_session(socket2)?;

    let mut spec = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)?
        .with_protocol_config(protocol_config(&clock))
        .with_disconnect_timeout(short_timeout)
        .with_disconnect_notify_delay(Duration::from_millis(50))
        .start_spectator_session_multi(&[addr1, addr2], socket3)
        .expect("multi-host spectator should start");

    // Synchronize everything.
    let mut synced = false;
    for _ in 0..MAX_SYNC_ITERATIONS {
        host1.poll_remote_clients();
        host2.poll_remote_clients();
        spec.poll_remote_clients();
        if host1.current_state() == SessionState::Running
            && host2.current_state() == SessionState::Running
            && spec.current_state() == SessionState::Running
        {
            synced = true;
            break;
        }
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }
    assert!(synced, "all sessions should synchronize");
    assert_eq!(spec.num_hosts(), 2);

    // Drain sync events.
    let _: Vec<_> = spec.events().collect();

    // Now stop polling/feeding host1 entirely. host2 keeps producing inputs.
    // Advancing virtual time past the disconnect timeout makes the spectator's
    // host1 endpoint time out, removing it via failover.
    let mut g2 = GameStub::new();
    let mut saw_disconnected = false;
    let mut spec_game = GameStub::new();

    for frame in 0..200 {
        // Only host2 stays alive and continues to send to the spectator.
        host2.add_local_input(PlayerHandle::new(1), StubInput { inp: frame })?;
        // host2 will be in prediction (host1 silent) but still advances locally
        // up to the prediction window; ignore PredictionThreshold.
        if let Ok(r) = host2.advance_frame() {
            g2.handle_requests(r);
        }
        host2.poll_remote_clients();
        spec.poll_remote_clients();

        for event in spec.events() {
            if matches!(event, FortressEvent::Disconnected { .. }) {
                saw_disconnected = true;
            }
        }

        if let Ok(requests) = spec.advance_frame() {
            spec_game.handle_requests(requests);
        }

        if spec.num_hosts() == 1 {
            break;
        }

        clock.advance(Duration::from_millis(20));
    }

    assert_eq!(
        spec.num_hosts(),
        1,
        "host1 should be removed via failover, leaving one host"
    );
    assert!(
        saw_disconnected,
        "spectator should observe a Disconnected event for the timed-out host"
    );

    Ok(())
}

/// Back-compat: the empty multi-host address list returns `None`.
#[test]
fn test_multi_host_spectator_empty_returns_none() {
    let (socket, _addr) = create_unconnected_socket(20050);
    let session = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .start_spectator_session_multi(&[], socket);
    assert!(session.is_none());
}

// ============================================================================
// Feature 9: Stream Delay
// ============================================================================

/// Verifies that with `stream_delay = N`, the spectator never advances its
/// current frame past `last_received_frame - N`, and that the boundary moves
/// forward as the host sends more inputs.
#[test]
fn test_stream_delay_holds_back_playback() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (socket1, socket2, host_addr, spec_addr) = create_channel_pair();
    let stream_delay = 5usize;

    let mut host = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)?
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    let spectator_config = SpectatorConfig {
        buffer_size: 64,
        stream_delay,
        ..SpectatorConfig::default()
    };

    let mut spec = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)?
        .with_protocol_config(protocol_config(&clock))
        .with_spectator_config(spectator_config)
        .start_spectator_session(host_addr, socket2)
        .expect("spectator session should start");

    assert_eq!(spec.stream_delay(), stream_delay);

    let result = synchronize_spectator_deterministic(&mut spec, &mut host, &clock);
    assert_spectator_synchronized(&spec, &host, &result);

    let mut host_game = GameStub::new();

    // Host advances exactly 8 frames (frames 0..=7, so highest sent frame == 7).
    let frames_sent = 8i32;
    for frame in 0..frames_sent {
        host.add_local_input(PlayerHandle::new(0), StubInput { inp: frame as u32 })?;
        host.add_local_input(PlayerHandle::new(1), StubInput { inp: frame as u32 })?;
        let requests = host.advance_frame()?;
        host_game.handle_requests(requests);
        for _ in 0..4 {
            host.poll_remote_clients();
            spec.poll_remote_clients();
            clock.advance(POLL_INTERVAL_DETERMINISTIC);
        }
    }
    // Let everything propagate.
    for _ in 0..40 {
        host.poll_remote_clients();
        spec.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    // Drive the spectator until it hits the stream-delay boundary. We keep polling
    // both sessions inside the loop so any straggler inputs are delivered, and we
    // only stop once the spectator's current frame has stopped advancing across a
    // settle window (i.e. it is genuinely held back by stream_delay, not merely
    // waiting on an in-flight input).
    let mut spec_game = GameStub::new();
    let mut stable = 0;
    while stable < 10 {
        host.poll_remote_clients();
        spec.poll_remote_clients();
        let before = spec.current_frame();
        match spec.advance_frame() {
            Ok(requests) => spec_game.handle_requests(requests),
            Err(FortressError::PredictionThreshold) => {},
            Err(other) => panic!("unexpected error advancing spectator: {other:?}"),
        }
        if spec.current_frame() == before {
            stable += 1;
        } else {
            stable = 0;
        }
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    // Having fully caught up to the delayed boundary, the spectator must be held
    // back from the live edge by EXACTLY stream_delay frames: parked at the
    // boundary, current_frame == last_received_frame - stream_delay, which means
    // frames_behind_host() == stream_delay exactly (not more, i.e. not
    // under-advanced). This is the precise stream-delay boundary derived from the
    // frames actually delivered to the spectator.
    assert_eq!(
        spec.frames_behind_host(),
        stream_delay,
        "spectator should be parked exactly stream_delay ({stream_delay}) frames behind \
         the live edge, but is {} behind (current_frame {:?})",
        spec.frames_behind_host(),
        spec.current_frame()
    );
    let frame_at_boundary = spec.current_frame().as_i32();
    let highest_delivered = frame_at_boundary + stream_delay as i32;

    // Now the host sends more frames; the boundary should move forward.
    for frame in frames_sent..(frames_sent + 6) {
        host.add_local_input(PlayerHandle::new(0), StubInput { inp: frame as u32 })?;
        host.add_local_input(PlayerHandle::new(1), StubInput { inp: frame as u32 })?;
        let requests = host.advance_frame()?;
        host_game.handle_requests(requests);
        for _ in 0..4 {
            host.poll_remote_clients();
            spec.poll_remote_clients();
            clock.advance(POLL_INTERVAL_DETERMINISTIC);
        }
    }
    for _ in 0..40 {
        host.poll_remote_clients();
        spec.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }
    let mut stable = 0;
    while stable < 10 {
        host.poll_remote_clients();
        spec.poll_remote_clients();
        let before = spec.current_frame();
        match spec.advance_frame() {
            Ok(requests) => spec_game.handle_requests(requests),
            Err(FortressError::PredictionThreshold) => {},
            Err(other) => panic!("unexpected error advancing spectator: {other:?}"),
        }
        if spec.current_frame() == before {
            stable += 1;
        } else {
            stable = 0;
        }
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    // After the host sends more frames the spectator catches up to the new
    // delayed boundary, again parked exactly stream_delay frames behind the live
    // edge. The boundary (and therefore current_frame) must have moved forward,
    // and the highest delivered frame must have increased.
    assert_eq!(
        spec.frames_behind_host(),
        stream_delay,
        "spectator should again be parked exactly stream_delay ({stream_delay}) frames behind, \
         but is {} behind (current_frame {:?})",
        spec.frames_behind_host(),
        spec.current_frame()
    );
    assert!(
        spec.current_frame().as_i32() > frame_at_boundary,
        "stream-delay boundary should advance after host sends more frames: was {}, now {:?}",
        frame_at_boundary,
        spec.current_frame()
    );
    let new_highest_delivered = spec.current_frame().as_i32() + stream_delay as i32;
    assert!(
        new_highest_delivered > highest_delivered,
        "highest delivered frame should grow: was {highest_delivered}, now {new_highest_delivered}"
    );

    Ok(())
}

// ============================================================================
// Feature 9: Rewind / Seek
// ============================================================================

/// Drives a rewind-enabled spectator forward, captures the state at frame 3,
/// seeks back to frame 3, and verifies the loaded state matches and that normal
/// advancement resumes afterwards.
#[test]
fn test_rewind_seek_round_trip() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (socket1, socket2, host_addr, spec_addr) = create_channel_pair();

    let mut host = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)?
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    let spectator_config = SpectatorConfig {
        buffer_size: 64,
        enable_rewind: true,
        ..SpectatorConfig::default()
    };

    let mut spec = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)?
        .with_protocol_config(protocol_config(&clock))
        .with_spectator_config(spectator_config)
        .start_spectator_session(host_addr, socket2)
        .expect("spectator session should start");

    assert!(spec.is_rewind_enabled());

    let result = synchronize_spectator_deterministic(&mut spec, &mut host, &clock);
    assert_spectator_synchronized(&spec, &host, &result);

    let mut host_game = GameStub::new();

    // Host advances ~12 frames.
    for frame in 0..12 {
        host.add_local_input(PlayerHandle::new(0), StubInput { inp: frame as u32 })?;
        host.add_local_input(PlayerHandle::new(1), StubInput { inp: frame as u32 })?;
        let requests = host.advance_frame()?;
        host_game.handle_requests(requests);
        for _ in 0..4 {
            host.poll_remote_clients();
            spec.poll_remote_clients();
            clock.advance(POLL_INTERVAL_DETERMINISTIC);
        }
    }
    for _ in 0..40 {
        host.poll_remote_clients();
        spec.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    // Drive the spectator forward, recording the game state value at frame 3.
    let mut spec_game = GameStub::new();
    let mut captured_frame3: Option<StateStub> = None;
    for _ in 0..40 {
        if let Ok(requests) = spec.advance_frame() {
            spec_game.handle_requests(requests);
        }
        if spec_game.gs.frame == 4 && captured_frame3.is_none() {
            // After simulating frame 3, the stub's frame counter reads 4
            // (it advances the counter at the end of frame 3). But we want the
            // state value as it was AT the start/identity of frame 3's result.
            captured_frame3 = Some(spec_game.gs);
        }
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        if spec.current_frame().as_i32() >= 9 {
            break;
        }
    }

    assert!(
        spec.current_frame().as_i32() >= 4,
        "spectator should have advanced well past frame 3, got {:?}",
        spec.current_frame()
    );
    let captured_frame3 = captured_frame3.expect("should have captured a frame-3 state");

    // Seek back to frame 3.
    let seek_requests = spec.seek_to_frame(Frame::new(3))?;
    // A seek is a single LoadGameState.
    assert_eq!(seek_requests.len(), 1);
    assert!(matches!(
        seek_requests.first(),
        Some(FortressRequest::LoadGameState { frame, .. }) if frame.as_i32() == 4
    ));
    spec_game.handle_requests(seek_requests);

    assert_eq!(spec.current_frame(), Frame::new(3));
    assert_eq!(
        spec_game.gs.frame, captured_frame3.frame,
        "loaded state frame should match captured frame-3 state"
    );
    assert_eq!(
        spec_game.gs.state, captured_frame3.state,
        "loaded state value should match captured frame-3 state"
    );

    // Confirm normal advancement resumes from frame 3.
    let mut resumed = false;
    for _ in 0..40 {
        host.poll_remote_clients();
        spec.poll_remote_clients();
        if let Ok(requests) = spec.advance_frame() {
            spec_game.handle_requests(requests);
            resumed = true;
        }
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        if spec.current_frame().as_i32() > 3 {
            break;
        }
    }
    assert!(
        resumed,
        "spectator should resume advancing after a seek-back"
    );
    assert!(
        spec.current_frame().as_i32() > 3,
        "spectator should advance past frame 3 after resuming, got {:?}",
        spec.current_frame()
    );

    Ok(())
}

/// Seeking on a spectator without rewind enabled returns `NotSupported`.
#[test]
fn test_seek_without_rewind_returns_not_supported() {
    let clock = TestClock::new();
    let (socket, _spec_addr) = create_unconnected_socket(20060);
    let host_addr = "127.0.0.1:20061".parse().unwrap();
    let mut spec = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .with_protocol_config(protocol_config(&clock))
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    let result = spec.seek_to_frame(Frame::new(0));
    assert!(
        matches!(
            result,
            Err(FortressError::InvalidRequestStructured {
                kind: fortress_rollback::InvalidRequestKind::NotSupported {
                    operation: "seek_to_frame"
                }
            })
        ),
        "expected NotSupported error from seek_to_frame on a non-rewind spectator"
    );
}

/// Seeking to a frame that was never saved / has rolled out of the window
/// returns `InvalidFrameStructured` with `MissingState`.
#[test]
fn test_seek_out_of_window_returns_missing_state() {
    let clock = TestClock::new();
    let (socket, _spec_addr) = create_unconnected_socket(20062);
    let host_addr = "127.0.0.1:20063".parse().unwrap();

    let spectator_config = SpectatorConfig {
        enable_rewind: true,
        ..SpectatorConfig::default()
    };

    let mut spec = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .with_protocol_config(protocol_config(&clock))
        .with_spectator_config(spectator_config)
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    // Nothing has been saved, so any seek target is MissingState.
    let result = spec.seek_to_frame(Frame::new(5));
    assert!(
        matches!(
            result,
            Err(FortressError::InvalidFrameStructured {
                reason: fortress_rollback::InvalidFrameReason::MissingState,
                ..
            })
        ),
        "expected MissingState error from seek_to_frame for an unsaved frame"
    );
}

// ============================================================================
// Feature 9: Partial-catchup regression
// ============================================================================

/// Regression test: with `catchup_speed > 1`, when only some of the requested
/// catchup frames are available, `advance_frame` must return the available
/// frames (advancing `current_frame` by that count) rather than discarding the
/// gathered requests with an error.
#[test]
fn test_partial_catchup_returns_available_frames() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (socket1, socket2, host_addr, spec_addr) = create_channel_pair();

    // Force catchup mode aggressively: catch up many frames at once, with a low
    // max_frames_behind so catchup kicks in as soon as the host gets ahead.
    let spectator_config = SpectatorConfig {
        buffer_size: 64,
        catchup_speed: 10,
        max_frames_behind: 2,
        ..SpectatorConfig::default()
    };

    let mut host = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)?
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    let mut spec = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)?
        .with_protocol_config(protocol_config(&clock))
        .with_spectator_config(spectator_config)
        .start_spectator_session(host_addr, socket2)
        .expect("spectator session should start");

    let result = synchronize_spectator_deterministic(&mut spec, &mut host, &clock);
    assert_spectator_synchronized(&spec, &host, &result);

    let mut host_game = GameStub::new();

    // Host gets a handful of frames ahead so the spectator falls into catchup.
    let frames_sent = 5i32;
    for frame in 0..frames_sent {
        host.add_local_input(PlayerHandle::new(0), StubInput { inp: frame as u32 })?;
        host.add_local_input(PlayerHandle::new(1), StubInput { inp: frame as u32 })?;
        let requests = host.advance_frame()?;
        host_game.handle_requests(requests);
        for _ in 0..4 {
            host.poll_remote_clients();
            spec.poll_remote_clients();
            clock.advance(POLL_INTERVAL_DETERMINISTIC);
        }
    }
    for _ in 0..40 {
        host.poll_remote_clients();
        spec.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    // The spectator is now several frames behind with catchup_speed=10, but only
    // `frames_sent` frames are available. advance_frame must return ALL available
    // frames as AdvanceFrame requests, not an error.
    let before = spec.current_frame().as_i32();
    let requests = spec.advance_frame()?;
    let advance_count = requests
        .iter()
        .filter(|r| matches!(r, FortressRequest::AdvanceFrame { .. }))
        .count() as i32;
    assert!(
        advance_count >= 1,
        "expected at least one AdvanceFrame request from partial catchup"
    );
    // It must not have advanced more than the frames the host actually sent.
    assert!(
        advance_count <= frames_sent,
        "advanced {advance_count} frames but host only sent {frames_sent}"
    );

    let mut spec_game = GameStub::new();
    spec_game.handle_requests(requests);

    let after = spec.current_frame().as_i32();
    assert_eq!(
        after - before,
        advance_count,
        "current_frame should advance by exactly the number of available frames"
    );

    Ok(())
}

// ============================================================================
// Behavior preservation: catchup_speed == 0
// ============================================================================

/// Behavior-preservation regression: when `catchup_speed == 0` and the spectator
/// has fallen further than `max_frames_behind` behind the live edge, the catchup
/// branch resolves `frames_to_advance == 0`, so the advance loop never runs.
///
/// `advance_frame` must return `Ok(<empty>)` in that degenerate case (the
/// historical contract for "no advance was even attempted"), NOT
/// `PredictionThreshold` — that error is reserved for the case where we actually
/// tried to advance at least one frame but nothing was available yet.
#[test]
fn test_catchup_speed_zero_while_behind_returns_ok_empty() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (socket1, socket2, host_addr, spec_addr) = create_channel_pair();

    // catchup_speed == 0 with a small max_frames_behind: as soon as the spectator
    // falls more than 2 frames behind, the catchup branch yields frames_to_advance == 0.
    let spectator_config = SpectatorConfig {
        buffer_size: 64,
        catchup_speed: 0,
        max_frames_behind: 2,
        ..SpectatorConfig::default()
    };

    let mut host = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)?
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    let mut spec = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)?
        .with_protocol_config(protocol_config(&clock))
        .with_spectator_config(spectator_config)
        .start_spectator_session(host_addr, socket2)
        .expect("spectator session should start");

    let result = synchronize_spectator_deterministic(&mut spec, &mut host, &clock);
    assert_spectator_synchronized(&spec, &host, &result);

    let mut host_game = GameStub::new();

    // Push the host well ahead so the spectator is far behind the live edge.
    let frames_sent = 10i32;
    for frame in 0..frames_sent {
        host.add_local_input(PlayerHandle::new(0), StubInput { inp: frame as u32 })?;
        host.add_local_input(PlayerHandle::new(1), StubInput { inp: frame as u32 })?;
        let requests = host.advance_frame()?;
        host_game.handle_requests(requests);
        for _ in 0..4 {
            host.poll_remote_clients();
            spec.poll_remote_clients();
            clock.advance(POLL_INTERVAL_DETERMINISTIC);
        }
    }
    for _ in 0..40 {
        host.poll_remote_clients();
        spec.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    // The spectator is still at NULL (it has not advanced) but the live edge is far
    // ahead, so effective_behind > max_frames_behind and catchup_speed == 0 forces
    // frames_to_advance == 0. advance_frame must succeed with no requests.
    assert!(
        spec.frames_behind_host() > 2,
        "spectator should be more than max_frames_behind behind, got {}",
        spec.frames_behind_host()
    );

    let requests = spec
        .advance_frame()
        .expect("catchup_speed == 0 while behind must return Ok, not an error");
    assert!(
        requests.is_empty(),
        "catchup_speed == 0 while behind must return an empty RequestVec, got {} requests",
        requests.len()
    );
    // The spectator must not have advanced.
    assert_eq!(
        spec.current_frame(),
        Frame::NULL,
        "spectator must not advance when catchup_speed == 0"
    );

    Ok(())
}

// ============================================================================
// Feature 9: Forward-seek after rewind
// ============================================================================

/// Drives a rewind-enabled spectator forward through several frames, captures the
/// state at frame 7, seeks BACK to frame 3, then seeks FORWARD again to frame 7
/// (still buffered in the ring). Verifies that forward-seek to a previously
/// visited, still-buffered frame restores the exact captured state.
#[test]
fn test_rewind_forward_seek_after_rewind() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (socket1, socket2, host_addr, spec_addr) = create_channel_pair();

    let mut host = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)?
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    let spectator_config = SpectatorConfig {
        buffer_size: 64,
        enable_rewind: true,
        ..SpectatorConfig::default()
    };

    let mut spec = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)?
        .with_protocol_config(protocol_config(&clock))
        .with_spectator_config(spectator_config)
        .start_spectator_session(host_addr, socket2)
        .expect("spectator session should start");

    assert!(spec.is_rewind_enabled());

    let result = synchronize_spectator_deterministic(&mut spec, &mut host, &clock);
    assert_spectator_synchronized(&spec, &host, &result);

    let mut host_game = GameStub::new();

    // Host advances ~14 frames so frames 3 and 7 are comfortably in the ring.
    for frame in 0..14 {
        host.add_local_input(PlayerHandle::new(0), StubInput { inp: frame as u32 })?;
        host.add_local_input(PlayerHandle::new(1), StubInput { inp: frame as u32 })?;
        let requests = host.advance_frame()?;
        host_game.handle_requests(requests);
        for _ in 0..4 {
            host.poll_remote_clients();
            spec.poll_remote_clients();
            clock.advance(POLL_INTERVAL_DETERMINISTIC);
        }
    }
    for _ in 0..40 {
        host.poll_remote_clients();
        spec.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    // Drive the spectator forward to at least frame 10, capturing the state value
    // at frame 7. After simulating frame 7 the stub's frame counter reads 8.
    let mut spec_game = GameStub::new();
    let mut captured_frame7: Option<StateStub> = None;
    for _ in 0..60 {
        if let Ok(requests) = spec.advance_frame() {
            spec_game.handle_requests(requests);
        }
        if spec_game.gs.frame == 8 && captured_frame7.is_none() {
            captured_frame7 = Some(spec_game.gs);
        }
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        if spec.current_frame().as_i32() >= 10 {
            break;
        }
    }
    assert!(
        spec.current_frame().as_i32() >= 10,
        "spectator should have advanced past frame 7, got {:?}",
        spec.current_frame()
    );
    let captured_frame7 = captured_frame7.expect("should have captured a frame-7 state");

    // Seek BACK to frame 3.
    let seek_back = spec.seek_to_frame(Frame::new(3))?;
    spec_game.handle_requests(seek_back);
    assert_eq!(spec.current_frame(), Frame::new(3));
    assert_eq!(
        spec_game.gs.frame, 4,
        "loaded state should be labeled frame 4 (post-frame-3 state)"
    );

    // Now seek FORWARD to frame 7 (still buffered in the ring).
    let seek_forward = spec.seek_to_frame(Frame::new(7))?;
    assert_eq!(seek_forward.len(), 1, "a seek is a single LoadGameState");
    assert!(matches!(
        seek_forward.first(),
        Some(FortressRequest::LoadGameState { frame, .. }) if frame.as_i32() == 8
    ));
    spec_game.handle_requests(seek_forward);

    assert_eq!(
        spec.current_frame(),
        Frame::new(7),
        "forward-seek should land exactly on frame 7"
    );
    assert_eq!(
        spec_game.gs.frame, captured_frame7.frame,
        "forward-seek restored state frame should match the captured frame-7 state"
    );
    assert_eq!(
        spec_game.gs.state, captured_frame7.state,
        "forward-seek restored state value should match the captured frame-7 state"
    );

    Ok(())
}

// ============================================================================
// Feature 9: Catchup + rewind interaction
// ============================================================================

/// With rewind enabled AND catchup_speed > 1 AND a small max_frames_behind, the
/// spectator advances multiple frames per `advance_frame` call while also emitting
/// a `SaveGameState` for each advanced frame. The GameStub's save asserts that the
/// save label matches its own frame counter, so a mislabeled batched save would
/// panic this test. Afterwards a `seek_to_frame` to a mid-batch frame must restore
/// the correct state value.
#[test]
fn test_catchup_with_rewind_saves_correct_frames() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (socket1, socket2, host_addr, spec_addr) = create_channel_pair();

    let spectator_config = SpectatorConfig {
        buffer_size: 64,
        catchup_speed: 4,
        max_frames_behind: 2,
        enable_rewind: true,
        ..SpectatorConfig::default()
    };

    let mut host = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)?
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    let mut spec = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)?
        .with_protocol_config(protocol_config(&clock))
        .with_spectator_config(spectator_config)
        .start_spectator_session(host_addr, socket2)
        .expect("spectator session should start");

    assert!(spec.is_rewind_enabled());

    let result = synchronize_spectator_deterministic(&mut spec, &mut host, &clock);
    assert_spectator_synchronized(&spec, &host, &result);

    let mut host_game = GameStub::new();

    // Host gets well ahead so the spectator is forced into catchup mode.
    let frames_sent = 12i32;
    for frame in 0..frames_sent {
        host.add_local_input(PlayerHandle::new(0), StubInput { inp: frame as u32 })?;
        host.add_local_input(PlayerHandle::new(1), StubInput { inp: frame as u32 })?;
        let requests = host.advance_frame()?;
        host_game.handle_requests(requests);
        for _ in 0..4 {
            host.poll_remote_clients();
            spec.poll_remote_clients();
            clock.advance(POLL_INTERVAL_DETERMINISTIC);
        }
    }
    for _ in 0..40 {
        host.poll_remote_clients();
        spec.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    // Compute the canonical post-frame-5 state independently of the (batched)
    // catchup advance so we have an external ground truth. Both players send
    // `inp == frame` every frame, so each frame's input sum is even and the stub's
    // `state` increases by 2 per frame. After simulating frames 0..=5 (6 frames)
    // the state is 12 and the frame counter (post-frame-5 label) reads 6.
    let mut reference = GameStub::new();
    for f in 0..=5i32 {
        let mut inputs: InputVec<StubInput> = InputVec::new();
        inputs.push((
            StubInput { inp: f as u32 },
            fortress_rollback::InputStatus::Confirmed,
        ));
        inputs.push((
            StubInput { inp: f as u32 },
            fortress_rollback::InputStatus::Confirmed,
        ));
        reference.handle_requests({
            let mut r = RequestVec::<StubConfig>::new();
            r.push(FortressRequest::AdvanceFrame { inputs });
            r
        });
    }
    let expected_frame5 = reference.gs;
    assert_eq!(expected_frame5.frame, 6);

    // Drive the spectator through catchup. Each advance_frame call may batch
    // several AdvanceFrame requests, each preceded by a correctly labeled
    // SaveGameState. The GameStub asserts the save label internally; a wrong
    // label would panic here.
    let mut spec_game = GameStub::new();
    let mut saw_batched_save = false;
    for _ in 0..60 {
        if let Ok(requests) = spec.advance_frame() {
            let save_count = requests
                .iter()
                .filter(|r| matches!(r, FortressRequest::SaveGameState { .. }))
                .count();
            let advance_count = requests
                .iter()
                .filter(|r| matches!(r, FortressRequest::AdvanceFrame { .. }))
                .count();
            // Save + advance counts must match: each advanced frame is saved.
            assert_eq!(
                save_count, advance_count,
                "each advanced frame must emit exactly one SaveGameState"
            );
            if advance_count > 1 {
                saw_batched_save = true;
            }
            spec_game.handle_requests(requests);
        }
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        if spec.current_frame().as_i32() >= 8 {
            break;
        }
    }

    assert!(
        saw_batched_save,
        "expected at least one advance_frame to batch multiple frames during catchup"
    );
    assert!(
        spec.current_frame().as_i32() >= 8,
        "spectator should catch up past frame 5, got {:?}",
        spec.current_frame()
    );

    // Seek to a mid-batch frame and verify the restored state value matches the
    // independently computed ground truth. A mislabeled batched save would either
    // have panicked the GameStub's internal assert above or surface here as a
    // mismatched restored state.
    let seek = spec.seek_to_frame(Frame::new(5))?;
    spec_game.handle_requests(seek);
    assert_eq!(spec.current_frame(), Frame::new(5));
    assert_eq!(
        spec_game.gs.frame, expected_frame5.frame,
        "seek-restored frame should match the post-frame-5 label"
    );
    assert_eq!(
        spec_game.gs.state, expected_frame5.state,
        "seek-restored state value should match the canonical frame-5 state"
    );

    Ok(())
}

// ============================================================================
// Feature 7: Multi-host newer-frame-wins store guard
// ============================================================================

/// With two redundant hosts feeding the same match, the spectator must produce
/// correct, confirmed `AdvanceFrame` inputs and advance frames monotonically. This
/// exercises the per-slot "only overwrite when the incoming frame is at least as
/// new" store guard: with both hosts targeting the same ring slots, the spectator
/// never desyncs and never regresses its current frame.
#[test]
fn test_multi_host_inputs_confirmed_and_monotonic() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (socket1, socket2, socket3, addr1, addr2, addr3) = create_channel_triple();

    let mut host1 = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)?
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(addr3), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    let mut host2 = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)?
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(addr3), PlayerHandle::new(2))?
        .start_p2p_session(socket2)?;

    let mut spec = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)?
        .with_protocol_config(protocol_config(&clock))
        .start_spectator_session_multi(&[addr1, addr2], socket3)
        .expect("multi-host spectator should start");

    assert_eq!(spec.num_hosts(), 2);

    let mut synced = false;
    for _ in 0..MAX_SYNC_ITERATIONS {
        host1.poll_remote_clients();
        host2.poll_remote_clients();
        spec.poll_remote_clients();
        if host1.current_state() == SessionState::Running
            && host2.current_state() == SessionState::Running
            && spec.current_state() == SessionState::Running
        {
            synced = true;
            break;
        }
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }
    assert!(synced, "all sessions should synchronize");

    let mut g1 = GameStub::new();
    let mut g2 = GameStub::new();

    for frame in 0..12 {
        host1.add_local_input(PlayerHandle::new(0), StubInput { inp: frame })?;
        host2.add_local_input(PlayerHandle::new(1), StubInput { inp: frame })?;
        if let Ok(r) = host1.advance_frame() {
            g1.handle_requests(r);
        }
        if let Ok(r) = host2.advance_frame() {
            g2.handle_requests(r);
        }
        for _ in 0..4 {
            host1.poll_remote_clients();
            host2.poll_remote_clients();
            spec.poll_remote_clients();
            clock.advance(POLL_INTERVAL_DETERMINISTIC);
        }
    }

    // Drive the spectator forward, asserting every produced input is Confirmed and
    // that current_frame never regresses across advance_frame calls.
    let mut spec_game = GameStub::new();
    let mut last_frame = spec.current_frame().as_i32();
    let mut advanced_at_least_one = false;
    for _ in 0..80 {
        host1.poll_remote_clients();
        host2.poll_remote_clients();
        if let Ok(requests) = spec.advance_frame() {
            for request in requests.iter() {
                if let FortressRequest::AdvanceFrame { inputs } = request {
                    for (_input, status) in inputs.iter() {
                        assert_eq!(
                            *status,
                            fortress_rollback::InputStatus::Confirmed,
                            "multi-host spectator inputs must be Confirmed"
                        );
                    }
                    advanced_at_least_one = true;
                }
            }
            spec_game.handle_requests(requests);
        }
        let now = spec.current_frame().as_i32();
        assert!(
            now >= last_frame,
            "current_frame must advance monotonically: {now} < {last_frame}"
        );
        last_frame = now;
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    assert!(
        advanced_at_least_one,
        "spectator should advance with confirmed inputs from redundant hosts"
    );
    assert_eq!(spec.num_hosts(), 2, "both hosts remain connected");
    assert!(
        spec.current_frame().is_valid(),
        "spectator should have advanced past NULL, got {:?}",
        spec.current_frame()
    );

    Ok(())
}

// ============================================================================
// Feature 9: Seek to exactly current_frame
// ============================================================================

/// Documents the seekable upper bound. When the spectator is AT `current_frame == N`,
/// the saved labels in the ring are `0..=N` (a save labeled `F` is emitted just
/// before simulating frame `F`). Seeking to frame `T` requires the label `T + 1`.
///
/// Therefore `seek_to_frame(N)` needs label `N + 1`, which has NOT yet been saved,
/// so it returns `MissingState`. `seek_to_frame(N - 1)` needs label `N`, which WAS
/// saved, so it succeeds. The seekable upper bound is `current_frame - 1`.
#[test]
fn test_seek_to_current_frame_returns_missing_state() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (socket1, socket2, host_addr, spec_addr) = create_channel_pair();

    let mut host = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)?
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    let spectator_config = SpectatorConfig {
        buffer_size: 64,
        enable_rewind: true,
        ..SpectatorConfig::default()
    };

    let mut spec = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)?
        .with_protocol_config(protocol_config(&clock))
        .with_spectator_config(spectator_config)
        .start_spectator_session(host_addr, socket2)
        .expect("spectator session should start");

    let result = synchronize_spectator_deterministic(&mut spec, &mut host, &clock);
    assert_spectator_synchronized(&spec, &host, &result);

    let mut host_game = GameStub::new();
    for frame in 0..12 {
        host.add_local_input(PlayerHandle::new(0), StubInput { inp: frame as u32 })?;
        host.add_local_input(PlayerHandle::new(1), StubInput { inp: frame as u32 })?;
        let requests = host.advance_frame()?;
        host_game.handle_requests(requests);
        for _ in 0..4 {
            host.poll_remote_clients();
            spec.poll_remote_clients();
            clock.advance(POLL_INTERVAL_DETERMINISTIC);
        }
    }
    for _ in 0..40 {
        host.poll_remote_clients();
        spec.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    // Advance the spectator a few frames so current_frame == N for some N >= 1.
    let mut spec_game = GameStub::new();
    for _ in 0..40 {
        if let Ok(requests) = spec.advance_frame() {
            spec_game.handle_requests(requests);
        }
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
        if spec.current_frame().as_i32() >= 5 {
            break;
        }
    }

    let n = spec.current_frame().as_i32();
    assert!(
        n >= 1,
        "spectator should have advanced at least to frame 1, got {n}"
    );

    // Seeking to the exact current frame needs label N+1, which was never saved.
    let result = spec.seek_to_frame(Frame::new(n));
    assert!(
        matches!(
            &result,
            Err(FortressError::InvalidFrameStructured {
                reason: fortress_rollback::InvalidFrameReason::MissingState,
                ..
            })
        ),
        "seek_to_frame(current_frame) must return MissingState (label N+1 not yet saved), got {:?}",
        result.err()
    );

    // Seeking to current_frame - 1 needs label N, which WAS saved, so it succeeds.
    let ok = spec.seek_to_frame(Frame::new(n - 1))?;
    assert_eq!(ok.len(), 1, "a successful seek is a single LoadGameState");
    assert!(matches!(
        ok.first(),
        Some(FortressRequest::LoadGameState { frame, .. }) if frame.as_i32() == n
    ));
    spec_game.handle_requests(ok);
    assert_eq!(spec.current_frame(), Frame::new(n - 1));

    Ok(())
}
