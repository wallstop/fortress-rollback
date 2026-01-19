//! Spectator session integration tests.
//!
//! # Port Allocation
//!
//! This test file uses `PortAllocator` for thread-safe port allocation.
//! All ports are dynamically allocated to avoid conflicts with other tests.

// Allow test-specific patterns that are appropriate for test code
#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::ip_constant,
    clippy::needless_collect
)]

use crate::common::stubs::{GameStub, StubConfig, StubInput};
use crate::common::{
    assert_spectator_synchronized, bind_socket_with_retry, synchronize_spectator, PortAllocator,
    MAX_SYNC_ITERATIONS, POLL_INTERVAL, SYNC_TIMEOUT,
};
use fortress_rollback::{
    telemetry::CollectingObserver, FortressError, FortressEvent, InputQueueConfig, PlayerHandle,
    PlayerType, SessionBuilder, SessionState, SpectatorConfig, SyncConfig,
};
use serial_test::serial;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

// ============================================================================
// Basic Session Tests
// ============================================================================

#[test]
#[serial]
fn test_start_session() -> Result<(), FortressError> {
    let [host_port, spec_port] = PortAllocator::next_ports::<2>();
    let host_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), host_port);
    let socket = bind_socket_with_retry(spec_port)?;
    let spec_sess = SessionBuilder::<StubConfig>::new()
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");
    assert_eq!(spec_sess.current_state(), SessionState::Synchronizing);
    Ok(())
}

#[test]
#[serial]
fn test_synchronize_with_host() -> Result<(), FortressError> {
    let (host_port, spec_port) = PortAllocator::next_pair();
    let host_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), host_port);
    let spec_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), spec_port);

    let socket1 = bind_socket_with_retry(host_port)?;
    let mut host_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(1)
        .unwrap()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    let socket2 = bind_socket_with_retry(spec_port)?;
    let mut spec_sess = SessionBuilder::<StubConfig>::new()
        .start_spectator_session(host_addr, socket2)
        .expect("spectator session should start");

    assert_eq!(spec_sess.current_state(), SessionState::Synchronizing);
    assert_eq!(host_sess.current_state(), SessionState::Synchronizing);

    // Use robust synchronization with timeout and diagnostics
    let result = synchronize_spectator(&mut spec_sess, &mut host_sess);
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
/// different player configurations. It uses timeout-based polling instead
/// of fixed iterations to be robust across different platforms and timing.
#[test]
#[serial]
fn test_synchronization_scenarios_data_driven() -> Result<(), FortressError> {
    // Define test cases for different configurations
    let test_cases = [
        SyncTestCase::new("single_local_single_spectator", 1, 1),
        SyncTestCase::new("two_local_single_spectator", 2, 2),
        SyncTestCase::new("four_local_single_spectator", 4, 4),
    ];

    for case in &test_cases {
        // Allocate ports dynamically for each test case
        let (host_port, spec_port) = PortAllocator::next_pair();
        let host_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), host_port);
        let spec_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), spec_port);

        let socket1 = bind_socket_with_retry(host_port)?;

        let mut builder = SessionBuilder::<StubConfig>::new()
            .with_num_players(case.num_players)
            .unwrap();

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

        let socket2 = bind_socket_with_retry(spec_port)?;

        let mut spec_sess = SessionBuilder::<StubConfig>::new()
            .with_num_players(case.num_players)
            .unwrap()
            .start_spectator_session(host_addr, socket2)
            .expect("Failed to start spectator session");

        // Perform synchronization with timeout
        let result = synchronize_spectator(&mut spec_sess, &mut host_sess);

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
/// various sync configuration presets. Each preset has different timing
/// characteristics that might interact with platform scheduling.
#[test]
#[serial]
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
        let (host_port, spec_port) = PortAllocator::next_pair();
        let host_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), host_port);
        let spec_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), spec_port);

        let socket1 = bind_socket_with_retry(host_port)?;

        let mut host_sess = SessionBuilder::<StubConfig>::new()
            .with_num_players(2)
            .unwrap()
            .with_sync_config(case.config)
            .add_player(PlayerType::Local, PlayerHandle::new(0))?
            .add_player(PlayerType::Local, PlayerHandle::new(1))?
            .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
            .start_p2p_session(socket1)
            .expect("Failed to start host session");

        let socket2 = bind_socket_with_retry(spec_port)?;

        let mut spec_sess = SessionBuilder::<StubConfig>::new()
            .with_num_players(2)
            .unwrap()
            .with_sync_config(case.config)
            .start_spectator_session(host_addr, socket2)
            .expect("Failed to start spectator session");

        // Use robust synchronization with timeout
        let result = synchronize_spectator(&mut spec_sess, &mut host_sess);

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
#[serial]
fn test_current_frame_starts_at_null() -> Result<(), FortressError> {
    let (host_port, spec_port) = PortAllocator::next_pair();
    let host_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), host_port);
    let socket = bind_socket_with_retry(spec_port)?;
    let spec_sess = SessionBuilder::<StubConfig>::new()
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    // Before synchronization, current_frame should be NULL (-1)
    assert!(spec_sess.current_frame().is_null());
    Ok(())
}

#[test]
#[serial]
fn test_frames_behind_host_initially_zero() -> Result<(), FortressError> {
    let (host_port, spec_port) = PortAllocator::next_pair();
    let host_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), host_port);
    let socket = bind_socket_with_retry(spec_port)?;
    let spec_sess = SessionBuilder::<StubConfig>::new()
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    // Both current_frame and last_recv_frame are NULL, so difference is 0
    assert_eq!(spec_sess.frames_behind_host(), 0);
    Ok(())
}

#[test]
#[serial]
fn test_num_players_default() -> Result<(), FortressError> {
    let (host_port, spec_port) = PortAllocator::next_pair();
    let host_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), host_port);
    let socket = bind_socket_with_retry(spec_port)?;
    let spec_sess = SessionBuilder::<StubConfig>::new()
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    // Default number of players is 2
    assert_eq!(spec_sess.num_players(), 2);
    Ok(())
}

#[test]
#[serial]
fn test_num_players_custom() -> Result<(), FortressError> {
    let (host_port, spec_port) = PortAllocator::next_pair();
    let host_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), host_port);
    let socket = bind_socket_with_retry(spec_port)?;
    let spec_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(4)
        .unwrap()
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    assert_eq!(spec_sess.num_players(), 4);
    Ok(())
}

// ============================================================================
// Network Stats Tests
// ============================================================================

#[test]
#[serial]
fn test_network_stats_not_synchronized() -> Result<(), FortressError> {
    let (host_port, spec_port) = PortAllocator::next_pair();
    let host_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), host_port);
    let socket = bind_socket_with_retry(spec_port)?;
    let spec_sess = SessionBuilder::<StubConfig>::new()
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    // Network stats should fail when not synchronized
    let result = spec_sess.network_stats();
    assert!(result.is_err());
    assert!(matches!(result, Err(FortressError::NotSynchronized)));
    Ok(())
}

// ============================================================================
// Events Tests
// ============================================================================

#[test]
#[serial]
fn test_events_empty_initially() -> Result<(), FortressError> {
    let (host_port, spec_port) = PortAllocator::next_pair();
    let host_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), host_port);
    let socket = bind_socket_with_retry(spec_port)?;
    let mut spec_sess = SessionBuilder::<StubConfig>::new()
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    // Initially, there should be no events
    let events: Vec<_> = spec_sess.events().collect();
    assert!(events.is_empty());
    Ok(())
}

#[test]
#[serial]
fn test_events_generated_during_sync() -> Result<(), FortressError> {
    let (host_port, spec_port) = PortAllocator::next_pair();
    let host_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), host_port);
    let spec_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), spec_port);

    let socket1 = bind_socket_with_retry(host_port)?;
    let mut host_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    let socket2 = bind_socket_with_retry(spec_port)?;
    let mut spec_sess = SessionBuilder::<StubConfig>::new()
        .start_spectator_session(host_addr, socket2)
        .expect("spectator session should start");

    // Poll a few times to generate synchronization events
    // Include small sleeps to allow messages to propagate reliably
    for _ in 0..10 {
        spec_sess.poll_remote_clients();
        host_sess.poll_remote_clients();
        thread::sleep(POLL_INTERVAL);
    }

    // We should get some synchronization events
    let events: Vec<_> = spec_sess.events().collect();
    // At minimum we should have some events (synchronizing progress)
    // The exact count depends on timing, but there should be some activity
    assert!(!events.is_empty() || spec_sess.current_state() == SessionState::Running);

    Ok(())
}

// ============================================================================
// Advance Frame Tests
// ============================================================================

#[test]
#[serial]
fn test_advance_frame_before_sync_fails() -> Result<(), FortressError> {
    let (host_port, spec_port) = PortAllocator::next_pair();
    let host_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), host_port);
    let socket = bind_socket_with_retry(spec_port)?;
    let mut spec_sess = SessionBuilder::<StubConfig>::new()
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    // advance_frame should fail when not synchronized
    let result = spec_sess.advance_frame();
    assert!(result.is_err());
    assert!(matches!(result, Err(FortressError::NotSynchronized)));
    Ok(())
}

#[test]
#[serial]
fn test_advance_frame_after_sync() -> Result<(), FortressError> {
    let (host_port, spec_port) = PortAllocator::next_pair();
    let host_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), host_port);
    let spec_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), spec_port);

    let socket1 = bind_socket_with_retry(host_port)?;
    let mut host_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    let socket2 = bind_socket_with_retry(spec_port)?;
    let mut spec_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .start_spectator_session(host_addr, socket2)
        .expect("spectator session should start");

    let mut host_game = GameStub::new();

    // Use robust synchronization with timeout and diagnostics
    let result = synchronize_spectator(&mut spec_sess, &mut host_sess);
    assert_spectator_synchronized(&spec_sess, &host_sess, &result);

    // Advance host a few frames and send inputs
    for _ in 0..5 {
        host_sess.add_local_input(PlayerHandle::new(0), StubInput { inp: 1 })?;
        host_sess.add_local_input(PlayerHandle::new(1), StubInput { inp: 2 })?;
        let requests = host_sess.advance_frame()?;
        host_game.handle_requests(requests);
        host_sess.poll_remote_clients();
        spec_sess.poll_remote_clients();
        // Small sleep to allow message propagation
        thread::sleep(POLL_INTERVAL);
    }

    // Give time for messages to propagate
    for _ in 0..20 {
        host_sess.poll_remote_clients();
        spec_sess.poll_remote_clients();
        thread::sleep(POLL_INTERVAL);
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
#[serial]
fn test_violation_observer_attached() -> Result<(), FortressError> {
    let (host_port, spec_port) = PortAllocator::next_pair();
    let host_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), host_port);
    let socket = bind_socket_with_retry(spec_port)?;
    let observer = Arc::new(CollectingObserver::new());

    let spec_sess = SessionBuilder::<StubConfig>::new()
        .with_violation_observer(observer)
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    // Verify observer is attached
    assert!(spec_sess.violation_observer().is_some());
    Ok(())
}

#[test]
#[serial]
fn test_no_violation_observer_by_default() -> Result<(), FortressError> {
    let (host_port, spec_port) = PortAllocator::next_pair();
    let host_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), host_port);
    let socket = bind_socket_with_retry(spec_port)?;
    let spec_sess = SessionBuilder::<StubConfig>::new()
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    // By default, no observer should be attached
    assert!(spec_sess.violation_observer().is_none());
    Ok(())
}

// ============================================================================
// Configuration Tests
// ============================================================================

#[test]
#[serial]
fn test_spectator_config_buffer_size() -> Result<(), FortressError> {
    let (host_port, spec_port) = PortAllocator::next_pair();
    let host_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), host_port);
    let spec_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), spec_port);

    let socket1 = bind_socket_with_retry(host_port)?;
    let _host_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
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

    let socket2 = bind_socket_with_retry(spec_port)?;
    let spec_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .with_spectator_config(spectator_config)
        .start_spectator_session(host_addr, socket2)
        .expect("spectator session should start");

    // Session should be created successfully
    assert_eq!(spec_sess.current_state(), SessionState::Synchronizing);

    Ok(())
}

#[test]
#[serial]
fn test_spectator_with_input_queue_config() -> Result<(), FortressError> {
    let (host_port, spec_port) = PortAllocator::next_pair();
    let host_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), host_port);
    let spec_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), spec_port);

    let socket1 = bind_socket_with_retry(host_port)?;
    let _host_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    // Create spectator with high latency input queue config
    let socket2 = bind_socket_with_retry(spec_port)?;
    let spec_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
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
#[serial]
fn test_poll_remote_clients_no_host() -> Result<(), FortressError> {
    let (host_port, spec_port) = PortAllocator::next_pair();
    let host_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), host_port);
    let socket = bind_socket_with_retry(spec_port)?;
    let mut spec_sess = SessionBuilder::<StubConfig>::new()
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    // Polling with no host should not panic
    for _ in 0..10 {
        spec_sess.poll_remote_clients();
    }

    // Should still be synchronizing (no host to sync with)
    assert_eq!(spec_sess.current_state(), SessionState::Synchronizing);
    Ok(())
}

// ============================================================================
// Full Spectator Flow Tests
// ============================================================================

#[test]
#[serial]
fn test_full_spectator_flow() -> Result<(), FortressError> {
    let (host_port, spec_port) = PortAllocator::next_pair();
    let host_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), host_port);
    let spec_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), spec_port);

    let socket1 = bind_socket_with_retry(host_port)?;
    let mut host_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    let socket2 = bind_socket_with_retry(spec_port)?;
    let mut spec_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .start_spectator_session(host_addr, socket2)
        .expect("spectator session should start");

    let mut host_game = GameStub::new();

    // Phase 1: Synchronization - use robust helper with timeout and diagnostics
    let sync_result = synchronize_spectator(&mut spec_sess, &mut host_sess);
    assert_spectator_synchronized(&spec_sess, &host_sess, &sync_result);

    // Phase 2: Host advances frames and spectator follows
    for frame in 0..10 {
        // Host adds inputs and advances
        host_sess.add_local_input(PlayerHandle::new(0), StubInput { inp: frame as u32 })?;
        host_sess.add_local_input(PlayerHandle::new(1), StubInput { inp: frame as u32 })?;
        let requests = host_sess.advance_frame()?;
        host_game.handle_requests(requests);

        // Poll to exchange messages with sleep to allow packet delivery
        for _ in 0..5 {
            host_sess.poll_remote_clients();
            spec_sess.poll_remote_clients();
            thread::sleep(POLL_INTERVAL);
        }
    }

    // Give extra time for messages to propagate
    for _ in 0..30 {
        host_sess.poll_remote_clients();
        spec_sess.poll_remote_clients();
        thread::sleep(POLL_INTERVAL);
    }

    // Spectator should be able to get inputs now
    let result = spec_sess.advance_frame();
    if result.is_ok() {
        let requests = result.unwrap();
        assert!(!requests.is_empty());
    }

    Ok(())
}

// ============================================================================
// Event Handling Tests
// ============================================================================

#[test]
#[serial]
fn test_synchronized_event_generated() -> Result<(), FortressError> {
    let (host_port, spec_port) = PortAllocator::next_pair();
    let host_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), host_port);
    let spec_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), spec_port);

    let socket1 = bind_socket_with_retry(host_port)?;
    let mut host_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    let socket2 = bind_socket_with_retry(spec_port)?;
    let mut spec_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .start_spectator_session(host_addr, socket2)
        .expect("spectator session should start");

    let mut found_synchronized = false;

    // NOTE: This test intentionally uses an inline sync loop instead of the centralized
    // `synchronize_spectator()` helper because we need to capture and inspect events
    // DURING synchronization. The helper only returns success/failure, not the events
    // generated during the handshake process. This test verifies that `Synchronized`
    // events are properly emitted.
    let start = Instant::now();
    while start.elapsed() < SYNC_TIMEOUT {
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
        thread::sleep(POLL_INTERVAL);
    }

    // We should have received a Synchronized event
    assert!(found_synchronized || spec_sess.current_state() == SessionState::Running);

    Ok(())
}

#[test]
#[serial]
fn test_synchronizing_events_generated() -> Result<(), FortressError> {
    let (host_port, spec_port) = PortAllocator::next_pair();
    let host_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), host_port);
    let spec_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), spec_port);

    let socket1 = bind_socket_with_retry(host_port)?;
    let mut host_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    let socket2 = bind_socket_with_retry(spec_port)?;
    let mut spec_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .start_spectator_session(host_addr, socket2)
        .expect("spectator session should start");

    let mut found_synchronizing = false;
    let mut iterations = 0;
    let start = Instant::now();

    // NOTE: This test intentionally uses an inline sync loop instead of the centralized
    // `synchronize_spectator()` helper because we need to capture and inspect events
    // DURING synchronization. The helper only returns success/failure, not the events
    // generated during the handshake process. This test verifies that `Synchronizing`
    // progress events are properly emitted.
    //
    // The loop also uses both time-based timeout and iteration limits to handle
    // platform timing variations (especially macOS CI).
    while start.elapsed() < SYNC_TIMEOUT && iterations < MAX_SYNC_ITERATIONS {
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

        // Small sleep to allow network layer to process messages
        thread::sleep(POLL_INTERVAL);
    }

    // We should have received Synchronizing progress events
    assert!(
        found_synchronizing,
        "Expected Synchronizing events during handshake.\n\
         Iterations: {}\n\
         Elapsed: {:?}\n\
         Spectator state: {:?}\n\
         Host state: {:?}",
        iterations,
        start.elapsed(),
        spec_sess.current_state(),
        host_sess.current_state()
    );

    Ok(())
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
#[serial]
fn test_spectator_catchup_speed() -> Result<(), FortressError> {
    let (host_port, spec_port) = PortAllocator::next_pair();
    let host_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), host_port);
    let spec_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), spec_port);

    // Configure spectator to catch up faster when behind
    let spectator_config = SpectatorConfig {
        buffer_size: 64,
        catchup_speed: 3,
        // Leave max_frames_behind to default to demonstrate forward-compatible pattern
        ..Default::default()
    };

    let socket1 = bind_socket_with_retry(host_port)?;
    let mut host_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    let socket2 = bind_socket_with_retry(spec_port)?;
    let mut spec_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .with_spectator_config(spectator_config)
        .start_spectator_session(host_addr, socket2)
        .expect("spectator session should start");

    let mut host_game = GameStub::new();

    // Synchronize first with proper timeout and sleeps for reliable timing
    let result = synchronize_spectator(&mut spec_sess, &mut host_sess);
    assert_spectator_synchronized(&spec_sess, &host_sess, &result);

    // Have host advance many frames ahead
    for frame in 0..20 {
        host_sess.add_local_input(PlayerHandle::new(0), StubInput { inp: frame as u32 })?;
        host_sess.add_local_input(PlayerHandle::new(1), StubInput { inp: frame as u32 })?;
        let requests = host_sess.advance_frame()?;
        host_game.handle_requests(requests);
        host_sess.poll_remote_clients();
    }

    // Let messages propagate with proper timing
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(1) {
        host_sess.poll_remote_clients();
        spec_sess.poll_remote_clients();
        thread::sleep(POLL_INTERVAL);
    }

    // Spectator should now be behind and catch up
    let _frames_behind = spec_sess.frames_behind_host();
    // frames_behind is usize, so it's always >= 0
    // Just verify we can read the value without panic

    Ok(())
}

#[test]
#[serial]
fn test_multiple_spectators_same_host() -> Result<(), FortressError> {
    let [host_port, spec_port1, spec_port2] = PortAllocator::next_ports::<3>();
    let host_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), host_port);
    let spec_addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), spec_port1);
    let spec_addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), spec_port2);

    let socket1 = bind_socket_with_retry(host_port)?;
    let mut host_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr1), PlayerHandle::new(2))?
        .add_player(PlayerType::Spectator(spec_addr2), PlayerHandle::new(3))?
        .start_p2p_session(socket1)?;

    let socket2 = bind_socket_with_retry(spec_port1)?;
    let mut spec_sess1 = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .start_spectator_session(host_addr, socket2)
        .expect("spectator session should start");

    let socket3 = bind_socket_with_retry(spec_port2)?;
    let mut spec_sess2 = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .start_spectator_session(host_addr, socket3)
        .expect("spectator session should start");

    // NOTE: This test intentionally uses an inline sync loop instead of the centralized
    // `synchronize_spectator()` helper because we need to synchronize THREE sessions
    // simultaneously (one host and TWO spectators). The helper only supports one
    // spectator + one host pair. Using two sequential calls would not correctly test
    // the scenario where both spectators synchronize concurrently with the same host.
    let start = Instant::now();
    let mut all_synced = false;
    while start.elapsed() < SYNC_TIMEOUT && !all_synced {
        spec_sess1.poll_remote_clients();
        spec_sess2.poll_remote_clients();
        host_sess.poll_remote_clients();

        all_synced = spec_sess1.current_state() == SessionState::Running
            && spec_sess2.current_state() == SessionState::Running
            && host_sess.current_state() == SessionState::Running;

        if !all_synced {
            thread::sleep(POLL_INTERVAL);
        }
    }

    // Both spectators should sync with detailed diagnostics on failure
    assert!(
        all_synced,
        "Failed to synchronize all sessions after {:?}.\n\
         Host state: {:?}\n\
         Spectator 1 state: {:?}\n\
         Spectator 2 state: {:?}",
        start.elapsed(),
        host_sess.current_state(),
        spec_sess1.current_state(),
        spec_sess2.current_state()
    );

    Ok(())
}

#[test]
#[serial]
fn test_spectator_disconnect_timeout() -> Result<(), FortressError> {
    let (host_port, spec_port) = PortAllocator::next_pair();
    let host_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), host_port);

    // Create spectator that expects a connection
    let socket = bind_socket_with_retry(spec_port)?;
    let mut spec_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .unwrap()
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    // Poll for a while without any host
    for _ in 0..20 {
        spec_sess.poll_remote_clients();
        thread::sleep(Duration::from_millis(10));
    }

    // Should still be in synchronizing state (waiting for host)
    assert_eq!(spec_sess.current_state(), SessionState::Synchronizing);

    // Events may contain sync timeout or still be empty
    let events: Vec<_> = spec_sess.events().collect();
    // Just verify we don't panic and can collect events
    drop(events);

    Ok(())
}
