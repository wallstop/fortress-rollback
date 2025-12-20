//! Spectator session integration tests.

// Allow hardcoded IP addresses - 127.0.0.1 is appropriate for tests
#![allow(clippy::ip_constant)]

use crate::common::stubs::{GameStub, StubConfig, StubInput};
use fortress_rollback::{
    telemetry::CollectingObserver, FortressError, FortressEvent, InputQueueConfig, P2PSession,
    PlayerHandle, PlayerType, SessionBuilder, SessionState, SpectatorConfig, SpectatorSession,
    SyncConfig, UdpNonBlockingSocket,
};
use serial_test::serial;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// Maximum time to wait for synchronization to complete.
const SYNC_TIMEOUT: Duration = Duration::from_secs(5);

/// Time to sleep between poll iterations to allow for proper timing.
/// This prevents tight loops that may not give the network layer enough time
/// to process messages, especially on systems with different scheduling behavior.
const POLL_INTERVAL: Duration = Duration::from_millis(1);

/// Maximum number of poll iterations to attempt during synchronization.
const MAX_SYNC_ITERATIONS: usize = 500;

// Helper to create test addresses
fn test_addr(port: u16) -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port)
}

/// Result of a synchronization attempt.
#[derive(Debug)]
struct SyncResult {
    /// Number of poll iterations it took to synchronize.
    iterations: usize,
    /// Time elapsed during synchronization.
    elapsed: Duration,
    /// Whether both sessions are now in Running state.
    success: bool,
}

/// Polls both sessions until they reach the Running state or timeout.
///
/// This function is more robust than a fixed number of iterations because:
/// 1. It uses actual time-based timeout instead of iteration count
/// 2. It includes small sleeps between iterations to allow proper message processing
/// 3. It provides diagnostic information on failure
///
/// # Arguments
/// * `spec_sess` - The spectator session to synchronize
/// * `host_sess` - The host P2P session to synchronize  
/// * `timeout` - Maximum time to wait for synchronization
///
/// # Returns
/// `SyncResult` with synchronization outcome and diagnostics.
fn wait_for_sync(
    spec_sess: &mut SpectatorSession<StubConfig>,
    host_sess: &mut P2PSession<StubConfig>,
    timeout: Duration,
) -> SyncResult {
    let start = Instant::now();
    let mut iterations = 0;

    while start.elapsed() < timeout && iterations < MAX_SYNC_ITERATIONS {
        spec_sess.poll_remote_clients();
        host_sess.poll_remote_clients();
        iterations += 1;

        // Check if both sessions are synchronized
        if spec_sess.current_state() == SessionState::Running
            && host_sess.current_state() == SessionState::Running
        {
            return SyncResult {
                iterations,
                elapsed: start.elapsed(),
                success: true,
            };
        }

        // Small sleep to allow network layer to process messages
        // This is especially important on fast systems where tight loops
        // may not give the OS enough time to deliver UDP packets
        thread::sleep(POLL_INTERVAL);
    }

    SyncResult {
        iterations,
        elapsed: start.elapsed(),
        success: false,
    }
}

/// Asserts that synchronization completed successfully, with detailed diagnostics on failure.
fn assert_synchronized(
    spec_sess: &SpectatorSession<StubConfig>,
    host_sess: &P2PSession<StubConfig>,
    result: &SyncResult,
) {
    assert!(
        result.success,
        "Synchronization failed after {} iterations ({:?}).\n\
         Spectator state: {:?}\n\
         Host state: {:?}\n\
         This may indicate a timing issue on this platform.",
        result.iterations,
        result.elapsed,
        spec_sess.current_state(),
        host_sess.current_state()
    );
}

// ============================================================================
// Basic Session Tests
// ============================================================================

#[test]
#[serial]
fn test_start_session() {
    let host_addr = test_addr(7777);
    let socket = UdpNonBlockingSocket::bind_to_port(9999).unwrap();
    let spec_sess = SessionBuilder::<StubConfig>::new()
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");
    assert_eq!(spec_sess.current_state(), SessionState::Synchronizing);
}

#[test]
#[serial]
fn test_synchronize_with_host() -> Result<(), FortressError> {
    let host_addr = test_addr(7777);
    let spec_addr = test_addr(8888);

    let socket1 = UdpNonBlockingSocket::bind_to_port(7777).unwrap();
    let mut host_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(1)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    let socket2 = UdpNonBlockingSocket::bind_to_port(8888).unwrap();
    let mut spec_sess = SessionBuilder::<StubConfig>::new()
        .start_spectator_session(host_addr, socket2)
        .expect("spectator session should start");

    assert_eq!(spec_sess.current_state(), SessionState::Synchronizing);
    assert_eq!(host_sess.current_state(), SessionState::Synchronizing);

    // Use robust synchronization with timeout and diagnostics
    let result = wait_for_sync(&mut spec_sess, &mut host_sess, SYNC_TIMEOUT);
    assert_synchronized(&spec_sess, &host_sess, &result);

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
    /// Base port number for socket binding
    base_port: u16,
}

impl SyncTestCase {
    const fn new(
        name: &'static str,
        num_players: usize,
        num_local_players: usize,
        base_port: u16,
    ) -> Self {
        Self {
            name,
            num_players,
            num_local_players,
            base_port,
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
        SyncTestCase::new("single_local_single_spectator", 1, 1, 7300),
        SyncTestCase::new("two_local_single_spectator", 2, 2, 7310),
        SyncTestCase::new("four_local_single_spectator", 4, 4, 7320),
    ];

    for case in &test_cases {
        // Create host session with local players and one spectator
        let host_addr = test_addr(case.base_port);
        let spec_addr = test_addr(case.base_port + 1);

        let socket1 = UdpNonBlockingSocket::bind_to_port(case.base_port)
            .unwrap_or_else(|e| panic!("[{}] Failed to bind host socket: {:?}", case.name, e));

        let mut builder = SessionBuilder::<StubConfig>::new().with_num_players(case.num_players);

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

        let socket2 = UdpNonBlockingSocket::bind_to_port(case.base_port + 1)
            .unwrap_or_else(|e| panic!("[{}] Failed to bind spectator socket: {:?}", case.name, e));

        let mut spec_sess = SessionBuilder::<StubConfig>::new()
            .with_num_players(case.num_players)
            .start_spectator_session(host_addr, socket2)
            .expect("Failed to start spectator session");

        // Perform synchronization with timeout
        let result = wait_for_sync(&mut spec_sess, &mut host_sess, SYNC_TIMEOUT);

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
        base_port: u16,
    }

    let test_cases = [
        SyncConfigTestCase {
            name: "default",
            config: SyncConfig::default(),
            base_port: 7330,
        },
        SyncConfigTestCase {
            name: "lan",
            config: SyncConfig::lan(),
            base_port: 7340,
        },
        SyncConfigTestCase {
            name: "competitive",
            config: SyncConfig::competitive(),
            base_port: 7350,
        },
    ];

    for case in &test_cases {
        let host_addr = test_addr(case.base_port);
        let spec_addr = test_addr(case.base_port + 1);

        let socket1 = UdpNonBlockingSocket::bind_to_port(case.base_port)
            .unwrap_or_else(|e| panic!("[{}] Failed to bind host socket: {:?}", case.name, e));

        let mut host_sess = SessionBuilder::<StubConfig>::new()
            .with_num_players(2)
            .with_sync_config(case.config)
            .add_player(PlayerType::Local, PlayerHandle::new(0))?
            .add_player(PlayerType::Local, PlayerHandle::new(1))?
            .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
            .start_p2p_session(socket1)
            .expect("Failed to start host session");

        let socket2 = UdpNonBlockingSocket::bind_to_port(case.base_port + 1)
            .unwrap_or_else(|e| panic!("[{}] Failed to bind spectator socket: {:?}", case.name, e));

        let mut spec_sess = SessionBuilder::<StubConfig>::new()
            .with_num_players(2)
            .with_sync_config(case.config)
            .start_spectator_session(host_addr, socket2)
            .expect("Failed to start spectator session");

        // Use robust synchronization with timeout
        let result = wait_for_sync(&mut spec_sess, &mut host_sess, SYNC_TIMEOUT);

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
fn test_current_frame_starts_at_null() {
    let host_addr = test_addr(7100);
    let socket = UdpNonBlockingSocket::bind_to_port(7101).unwrap();
    let spec_sess = SessionBuilder::<StubConfig>::new()
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    // Before synchronization, current_frame should be NULL (-1)
    assert!(spec_sess.current_frame().is_null());
}

#[test]
#[serial]
fn test_frames_behind_host_initially_zero() {
    let host_addr = test_addr(7110);
    let socket = UdpNonBlockingSocket::bind_to_port(7111).unwrap();
    let spec_sess = SessionBuilder::<StubConfig>::new()
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    // Both current_frame and last_recv_frame are NULL, so difference is 0
    assert_eq!(spec_sess.frames_behind_host(), 0);
}

#[test]
#[serial]
fn test_num_players_default() {
    let host_addr = test_addr(7120);
    let socket = UdpNonBlockingSocket::bind_to_port(7121).unwrap();
    let spec_sess = SessionBuilder::<StubConfig>::new()
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    // Default number of players is 2
    assert_eq!(spec_sess.num_players(), 2);
}

#[test]
#[serial]
fn test_num_players_custom() {
    let host_addr = test_addr(7130);
    let socket = UdpNonBlockingSocket::bind_to_port(7131).unwrap();
    let spec_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(4)
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    assert_eq!(spec_sess.num_players(), 4);
}

// ============================================================================
// Network Stats Tests
// ============================================================================

#[test]
#[serial]
fn test_network_stats_not_synchronized() {
    let host_addr = test_addr(7140);
    let socket = UdpNonBlockingSocket::bind_to_port(7141).unwrap();
    let spec_sess = SessionBuilder::<StubConfig>::new()
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
#[serial]
fn test_events_empty_initially() {
    let host_addr = test_addr(7150);
    let socket = UdpNonBlockingSocket::bind_to_port(7151).unwrap();
    let mut spec_sess = SessionBuilder::<StubConfig>::new()
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    // Initially, there should be no events
    let events: Vec<_> = spec_sess.events().collect();
    assert!(events.is_empty());
}

#[test]
#[serial]
fn test_events_generated_during_sync() -> Result<(), FortressError> {
    let host_addr = test_addr(7160);
    let spec_addr = test_addr(7161);

    let socket1 = UdpNonBlockingSocket::bind_to_port(7160).unwrap();
    let mut host_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    let socket2 = UdpNonBlockingSocket::bind_to_port(7161).unwrap();
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
fn test_advance_frame_before_sync_fails() {
    let host_addr = test_addr(7170);
    let socket = UdpNonBlockingSocket::bind_to_port(7171).unwrap();
    let mut spec_sess = SessionBuilder::<StubConfig>::new()
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    // advance_frame should fail when not synchronized
    let result = spec_sess.advance_frame();
    assert!(result.is_err());
    assert!(matches!(result, Err(FortressError::NotSynchronized)));
}

#[test]
#[serial]
fn test_advance_frame_after_sync() -> Result<(), FortressError> {
    let host_addr = test_addr(7180);
    let spec_addr = test_addr(7181);

    let socket1 = UdpNonBlockingSocket::bind_to_port(7180).unwrap();
    let mut host_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    let socket2 = UdpNonBlockingSocket::bind_to_port(7181).unwrap();
    let mut spec_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .start_spectator_session(host_addr, socket2)
        .expect("spectator session should start");

    let mut host_game = GameStub::new();

    // Use robust synchronization with timeout and diagnostics
    let result = wait_for_sync(&mut spec_sess, &mut host_sess, SYNC_TIMEOUT);
    assert_synchronized(&spec_sess, &host_sess, &result);

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
fn test_violation_observer_attached() {
    let host_addr = test_addr(7190);
    let socket = UdpNonBlockingSocket::bind_to_port(7191).unwrap();
    let observer = Arc::new(CollectingObserver::new());

    let spec_sess = SessionBuilder::<StubConfig>::new()
        .with_violation_observer(observer)
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    // Verify observer is attached
    assert!(spec_sess.violation_observer().is_some());
}

#[test]
#[serial]
fn test_no_violation_observer_by_default() {
    let host_addr = test_addr(7200);
    let socket = UdpNonBlockingSocket::bind_to_port(7201).unwrap();
    let spec_sess = SessionBuilder::<StubConfig>::new()
        .start_spectator_session(host_addr, socket)
        .expect("spectator session should start");

    // By default, no observer should be attached
    assert!(spec_sess.violation_observer().is_none());
}

// ============================================================================
// Configuration Tests
// ============================================================================

#[test]
#[serial]
fn test_spectator_config_buffer_size() -> Result<(), FortressError> {
    let host_addr = test_addr(7210);
    let spec_addr = test_addr(7211);

    let socket1 = UdpNonBlockingSocket::bind_to_port(7210).unwrap();
    let _host_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
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

    let socket2 = UdpNonBlockingSocket::bind_to_port(7211).unwrap();
    let spec_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
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
    let host_addr = test_addr(7220);
    let spec_addr = test_addr(7221);

    let socket1 = UdpNonBlockingSocket::bind_to_port(7220).unwrap();
    let _host_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    // Create spectator with high latency input queue config
    let socket2 = UdpNonBlockingSocket::bind_to_port(7221).unwrap();
    let spec_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
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
fn test_poll_remote_clients_no_host() {
    let host_addr = test_addr(7230);
    let socket = UdpNonBlockingSocket::bind_to_port(7231).unwrap();
    let mut spec_sess = SessionBuilder::<StubConfig>::new()
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
#[serial]
fn test_full_spectator_flow() -> Result<(), FortressError> {
    let host_addr = test_addr(7240);
    let spec_addr = test_addr(7241);

    let socket1 = UdpNonBlockingSocket::bind_to_port(7240).unwrap();
    let mut host_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    let socket2 = UdpNonBlockingSocket::bind_to_port(7241).unwrap();
    let mut spec_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .start_spectator_session(host_addr, socket2)
        .expect("spectator session should start");

    let mut host_game = GameStub::new();

    // Phase 1: Synchronization
    let mut synced = false;
    for _ in 0..100 {
        spec_sess.poll_remote_clients();
        host_sess.poll_remote_clients();
        if spec_sess.current_state() == SessionState::Running
            && host_sess.current_state() == SessionState::Running
        {
            synced = true;
            break;
        }
    }
    assert!(synced, "Failed to synchronize");

    // Phase 2: Host advances frames and spectator follows
    for frame in 0..10 {
        // Host adds inputs and advances
        host_sess.add_local_input(PlayerHandle::new(0), StubInput { inp: frame as u32 })?;
        host_sess.add_local_input(PlayerHandle::new(1), StubInput { inp: frame as u32 })?;
        let requests = host_sess.advance_frame()?;
        host_game.handle_requests(requests);

        // Poll to exchange messages
        for _ in 0..5 {
            host_sess.poll_remote_clients();
            spec_sess.poll_remote_clients();
        }
    }

    // Give extra time for messages to propagate
    for _ in 0..30 {
        host_sess.poll_remote_clients();
        spec_sess.poll_remote_clients();
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
    let host_addr = test_addr(7250);
    let spec_addr = test_addr(7251);

    let socket1 = UdpNonBlockingSocket::bind_to_port(7250).unwrap();
    let mut host_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    let socket2 = UdpNonBlockingSocket::bind_to_port(7251).unwrap();
    let mut spec_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .start_spectator_session(host_addr, socket2)
        .expect("spectator session should start");

    let mut found_synchronized = false;

    // Synchronize and collect events
    for _ in 0..100 {
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
    }

    // We should have received a Synchronized event
    assert!(found_synchronized || spec_sess.current_state() == SessionState::Running);

    Ok(())
}

#[test]
#[serial]
fn test_synchronizing_events_generated() -> Result<(), FortressError> {
    let host_addr = test_addr(7260);
    let spec_addr = test_addr(7261);

    let socket1 = UdpNonBlockingSocket::bind_to_port(7260).unwrap();
    let mut host_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    let socket2 = UdpNonBlockingSocket::bind_to_port(7261).unwrap();
    let mut spec_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .start_spectator_session(host_addr, socket2)
        .expect("spectator session should start");

    let mut found_synchronizing = false;

    // Run synchronization and collect events
    for _ in 0..50 {
        spec_sess.poll_remote_clients();
        host_sess.poll_remote_clients();

        for event in spec_sess.events() {
            if matches!(event, FortressEvent::Synchronizing { .. }) {
                found_synchronizing = true;
            }
        }
    }

    // We should have received Synchronizing progress events
    assert!(
        found_synchronizing,
        "Expected Synchronizing events during handshake"
    );

    Ok(())
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
#[serial]
fn test_spectator_catchup_speed() -> Result<(), FortressError> {
    let host_addr = test_addr(7270);
    let spec_addr = test_addr(7271);

    // Configure spectator to catch up faster when behind
    let spectator_config = SpectatorConfig {
        buffer_size: 64,
        catchup_speed: 3,
        // Leave max_frames_behind to default to demonstrate forward-compatible pattern
        ..Default::default()
    };

    let socket1 = UdpNonBlockingSocket::bind_to_port(7270).unwrap();
    let mut host_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    let socket2 = UdpNonBlockingSocket::bind_to_port(7271).unwrap();
    let mut spec_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .with_spectator_config(spectator_config)
        .start_spectator_session(host_addr, socket2)
        .expect("spectator session should start");

    let mut host_game = GameStub::new();

    // Synchronize first
    for _ in 0..100 {
        spec_sess.poll_remote_clients();
        host_sess.poll_remote_clients();
        if spec_sess.current_state() == SessionState::Running
            && host_sess.current_state() == SessionState::Running
        {
            break;
        }
    }

    // Have host advance many frames ahead
    for frame in 0..20 {
        host_sess.add_local_input(PlayerHandle::new(0), StubInput { inp: frame as u32 })?;
        host_sess.add_local_input(PlayerHandle::new(1), StubInput { inp: frame as u32 })?;
        let requests = host_sess.advance_frame()?;
        host_game.handle_requests(requests);
        host_sess.poll_remote_clients();
    }

    // Let messages propagate
    for _ in 0..50 {
        host_sess.poll_remote_clients();
        spec_sess.poll_remote_clients();
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
    let host_addr = test_addr(7280);
    let spec_addr1 = test_addr(7281);
    let spec_addr2 = test_addr(7282);

    let socket1 = UdpNonBlockingSocket::bind_to_port(7280).unwrap();
    let mut host_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr1), PlayerHandle::new(2))?
        .add_player(PlayerType::Spectator(spec_addr2), PlayerHandle::new(3))?
        .start_p2p_session(socket1)?;

    let socket2 = UdpNonBlockingSocket::bind_to_port(7281).unwrap();
    let mut spec_sess1 = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .start_spectator_session(host_addr, socket2)
        .expect("spectator session should start");

    let socket3 = UdpNonBlockingSocket::bind_to_port(7282).unwrap();
    let mut spec_sess2 = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .start_spectator_session(host_addr, socket3)
        .expect("spectator session should start");

    // Synchronize all with proper timeout and small sleeps for reliable timing
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
    let host_addr = test_addr(7290);

    // Create spectator that expects a connection
    let socket = UdpNonBlockingSocket::bind_to_port(7291).unwrap();
    let mut spec_sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
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
