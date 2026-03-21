//! P2P session integration tests.
//!
//! # Deterministic Testing Infrastructure
//!
//! This test file uses `ChannelSocket` (in-memory sockets) and `TestClock`
//! (virtual time) for fully deterministic testing. No real UDP I/O or
//! `thread::sleep` calls are needed, eliminating port conflicts and
//! timing-related flakiness.

// Allow test-specific patterns that are appropriate for test code
#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::ip_constant
)]

use crate::common::stubs::{CorruptibleGameStub, GameStub, StubConfig, StubInput};
use crate::common::{
    create_channel_pair, create_channel_quad, create_channel_triple, create_unconnected_socket,
    drain_sync_events, poll_with_advance, synchronize_sessions_deterministic, SyncConfig,
    TestClock, POLL_INTERVAL_DETERMINISTIC,
};
use fortress_rollback::{
    DesyncDetection, FortressError, FortressEvent, PlayerHandle, PlayerType, ProtocolConfig,
    SessionBuilder, SessionState,
};
use std::net::SocketAddr;

/// Helper: creates a `ProtocolConfig` with the given test clock.
fn protocol_config(clock: &TestClock) -> ProtocolConfig {
    ProtocolConfig {
        clock: Some(clock.as_protocol_clock()),
        ..ProtocolConfig::default()
    }
}

#[test]
fn test_add_more_players() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (socket, _addr0) = create_unconnected_socket(10000);
    let remote_addr1: SocketAddr = ([127, 0, 0, 1], 10001).into();
    let remote_addr2: SocketAddr = ([127, 0, 0, 1], 10002).into();
    let remote_addr3: SocketAddr = ([127, 0, 0, 1], 10003).into();
    let spec_addr: SocketAddr = ([127, 0, 0, 1], 10004).into();

    let _sess = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(4)
        .unwrap()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(remote_addr1), PlayerHandle::new(1))?
        .add_player(PlayerType::Remote(remote_addr2), PlayerHandle::new(2))?
        .add_player(PlayerType::Remote(remote_addr3), PlayerHandle::new(3))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(4))?
        .start_p2p_session(socket)?;
    Ok(())
}

#[test]
fn test_start_session() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (socket, _addr0) = create_unconnected_socket(10000);
    let remote_addr: SocketAddr = ([127, 0, 0, 1], 10001).into();
    let spec_addr: SocketAddr = ([127, 0, 0, 1], 10002).into();

    let _sess = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(remote_addr), PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket)?;
    Ok(())
}

#[test]
fn test_disconnect_player() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (socket, _addr0) = create_unconnected_socket(10000);
    let remote_addr: SocketAddr = ([127, 0, 0, 1], 10001).into();
    let spec_addr: SocketAddr = ([127, 0, 0, 1], 10002).into();

    let mut sess = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(remote_addr), PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket)?;

    assert!(sess.disconnect_player(PlayerHandle::new(5)).is_err()); // invalid handle
    assert!(sess.disconnect_player(PlayerHandle::new(0)).is_err()); // for now, local players cannot be disconnected
    assert!(sess.disconnect_player(PlayerHandle::new(1)).is_ok());
    assert!(sess.disconnect_player(PlayerHandle::new(1)).is_err()); // already disconnected
    assert!(sess.disconnect_player(PlayerHandle::new(2)).is_ok());

    Ok(())
}

#[test]
fn test_synchronize_p2p_sessions() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (s1, s2, a1, a2) = create_channel_pair();

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .start_p2p_session(s1)?;

    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Remote(a1), PlayerHandle::new(0))?
        .start_p2p_session(s2)?;

    assert!(sess1.current_state() == SessionState::Synchronizing);
    assert!(sess2.current_state() == SessionState::Synchronizing);

    synchronize_sessions_deterministic(&mut sess1, &mut sess2, &clock, &SyncConfig::default())
        .expect("Sessions should synchronize");

    assert!(sess1.current_state() == SessionState::Running);
    assert!(sess2.current_state() == SessionState::Running);

    Ok(())
}

/// Tests P2P frame advancement between two synchronized sessions.
///
/// Uses the generic `run_p2p_frame_advancement_test_deterministic` helper.
#[test]
fn test_advance_frame_p2p_sessions() -> Result<(), FortressError> {
    use crate::common::run_p2p_frame_advancement_test_deterministic;

    run_p2p_frame_advancement_test_deterministic::<StubConfig, GameStub>(
        |i| StubInput { inp: i },
        10,
    )
}

#[test]
fn test_desyncs_detected() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (s1, s2, a1, a2) = create_channel_pair();
    let desync_mode = DesyncDetection::On { interval: 100 };

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .with_desync_detection_mode(desync_mode)
        .start_p2p_session(s1)?;

    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(a1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .with_desync_detection_mode(desync_mode)
        .start_p2p_session(s2)?;

    synchronize_sessions_deterministic(&mut sess1, &mut sess2, &clock, &SyncConfig::default())
        .expect("Sessions should synchronize");
    drain_sync_events(&mut sess1, &mut sess2);

    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    // run normally for some frames (past first desync interval)
    for i in 0..110 {
        poll_with_advance(&mut sess1, &mut sess2, &clock, 3);

        sess1
            .add_local_input(PlayerHandle::new(0), StubInput { inp: i })
            .unwrap();
        sess2
            .add_local_input(PlayerHandle::new(1), StubInput { inp: i })
            .unwrap();

        let requests1 = sess1.advance_frame().unwrap();
        let requests2 = sess2.advance_frame().unwrap();

        stub1.handle_requests(requests1);
        stub2.handle_requests(requests2);
    }

    // check that there are no unexpected events yet
    let unexpected_events1: Vec<_> = sess1.events().collect();
    let unexpected_events2: Vec<_> = sess2.events().collect();
    assert_eq!(
        unexpected_events1.len(),
        0,
        "Session 1 should have no events after normal frames. Got: {:?}",
        unexpected_events1
    );
    assert_eq!(
        unexpected_events2.len(),
        0,
        "Session 2 should have no events after normal frames. Got: {:?}",
        unexpected_events2
    );

    // run for some more frames with steady inputs
    for _ in 0..100 {
        poll_with_advance(&mut sess1, &mut sess2, &clock, 3);

        // mess up state for peer 1 BEFORE handling requests
        stub1.gs.state = 1234;

        // Use steady inputs - with RepeatLastConfirmed, after first frame predictions will match
        // and no more rollbacks occur, allowing the corrupted state to persist.
        sess1
            .add_local_input(PlayerHandle::new(0), StubInput { inp: 0 })
            .unwrap();
        sess2
            .add_local_input(PlayerHandle::new(1), StubInput { inp: 1 })
            .unwrap();

        let requests1 = sess1.advance_frame().unwrap();
        let requests2 = sess2.advance_frame().unwrap();

        stub1.handle_requests(requests1);
        stub2.handle_requests(requests2);
    }

    // check that we got desync events
    let sess1_events: Vec<_> = sess1.events().collect();
    let sess2_events: Vec<_> = sess2.events().collect();
    assert_eq!(sess1_events.len(), 1);
    assert_eq!(sess2_events.len(), 1);

    let FortressEvent::DesyncDetected {
        frame: desync_frame1,
        local_checksum: desync_local_checksum1,
        remote_checksum: desync_remote_checksum1,
        addr: desync_addr1,
    } = sess1_events[0]
    else {
        panic!("no desync for peer 1");
    };
    assert_eq!(desync_frame1, 200);
    assert_eq!(desync_addr1, a2);
    assert_ne!(desync_local_checksum1, desync_remote_checksum1);

    let FortressEvent::DesyncDetected {
        frame: desync_frame2,
        local_checksum: desync_local_checksum2,
        remote_checksum: desync_remote_checksum2,
        addr: desync_addr2,
    } = sess2_events[0]
    else {
        panic!("no desync for peer 2");
    };
    assert_eq!(desync_frame2, 200);
    assert_eq!(desync_addr2, a1);
    assert_ne!(desync_local_checksum2, desync_remote_checksum2);

    // check that checksums match
    assert_eq!(desync_remote_checksum1, desync_local_checksum2);
    assert_eq!(desync_remote_checksum2, desync_local_checksum1);

    Ok(())
}

#[test]
fn test_desyncs_and_input_delay_no_panic() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (s1, s2, a1, a2) = create_channel_pair();
    let desync_mode = DesyncDetection::On { interval: 100 };

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .with_input_delay(5)
        .unwrap()
        .with_desync_detection_mode(desync_mode)
        .start_p2p_session(s1)?;

    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(a1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .with_input_delay(5)
        .unwrap()
        .with_desync_detection_mode(desync_mode)
        .start_p2p_session(s2)?;

    synchronize_sessions_deterministic(&mut sess1, &mut sess2, &clock, &SyncConfig::default())
        .expect("Sessions should synchronize");
    drain_sync_events(&mut sess1, &mut sess2);

    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    // run normally for some frames (past first desync interval)
    for i in 0..150 {
        poll_with_advance(&mut sess1, &mut sess2, &clock, 3);

        sess1
            .add_local_input(PlayerHandle::new(0), StubInput { inp: i })
            .unwrap();
        sess2
            .add_local_input(PlayerHandle::new(1), StubInput { inp: i })
            .unwrap();

        let requests1 = sess1.advance_frame().unwrap();
        let requests2 = sess2.advance_frame().unwrap();

        stub1.handle_requests(requests1);
        stub2.handle_requests(requests2);
    }

    Ok(())
}

/// Test 3-player P2P session synchronization and frame advancement.
/// This tests the multi-player scenario with 3 independent peers.
#[test]
fn test_three_player_session() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (s1, s2, s3, a1, a2, a3) = create_channel_triple();
    let pc = protocol_config(&clock);

    // Player 1: local=0, remote=1,2
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(pc.clone())
        .with_num_players(3)
        .unwrap()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .add_player(PlayerType::Remote(a3), PlayerHandle::new(2))?
        .start_p2p_session(s1)?;

    // Player 2: local=1, remote=0,2
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(pc.clone())
        .with_num_players(3)
        .unwrap()
        .add_player(PlayerType::Remote(a1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Remote(a3), PlayerHandle::new(2))?
        .start_p2p_session(s2)?;

    // Player 3: local=2, remote=0,1
    let mut sess3 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(pc)
        .with_num_players(3)
        .unwrap()
        .add_player(PlayerType::Remote(a1), PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .add_player(PlayerType::Local, PlayerHandle::new(2))?
        .start_p2p_session(s3)?;

    // All sessions should start in Synchronizing state
    assert_eq!(sess1.current_state(), SessionState::Synchronizing);
    assert_eq!(sess2.current_state(), SessionState::Synchronizing);
    assert_eq!(sess3.current_state(), SessionState::Synchronizing);

    // Synchronize all peers
    for _ in 0..100 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        sess3.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    // All sessions should now be Running
    assert_eq!(sess1.current_state(), SessionState::Running);
    assert_eq!(sess2.current_state(), SessionState::Running);
    assert_eq!(sess3.current_state(), SessionState::Running);

    // Create game stubs for each player
    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();
    let mut stub3 = GameStub::new();

    // Advance frames
    let reps = 20;
    for i in 0..reps {
        // Poll with virtual time advancement
        for _ in 0..3 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
            sess3.poll_remote_clients();
            clock.advance(POLL_INTERVAL_DETERMINISTIC);
        }

        // Each player adds their local input
        sess1
            .add_local_input(PlayerHandle::new(0), StubInput { inp: i })
            .unwrap();
        sess2
            .add_local_input(PlayerHandle::new(1), StubInput { inp: i })
            .unwrap();
        sess3
            .add_local_input(PlayerHandle::new(2), StubInput { inp: i })
            .unwrap();

        // Advance frames
        let requests1 = sess1.advance_frame().unwrap();
        let requests2 = sess2.advance_frame().unwrap();
        let requests3 = sess3.advance_frame().unwrap();

        stub1.handle_requests(requests1);
        stub2.handle_requests(requests2);
        stub3.handle_requests(requests3);

        // All game states should have advanced
        assert_eq!(stub1.gs.frame, i as i32 + 1);
        assert_eq!(stub2.gs.frame, i as i32 + 1);
        assert_eq!(stub3.gs.frame, i as i32 + 1);
    }

    Ok(())
}

/// Test 4-player P2P session synchronization and frame advancement.
#[test]
fn test_four_player_session() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (s1, s2, s3, s4, a1, a2, a3, a4) = create_channel_quad();
    let pc = protocol_config(&clock);

    // Player 1
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(pc.clone())
        .with_num_players(4)
        .unwrap()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .add_player(PlayerType::Remote(a3), PlayerHandle::new(2))?
        .add_player(PlayerType::Remote(a4), PlayerHandle::new(3))?
        .start_p2p_session(s1)?;

    // Player 2
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(pc.clone())
        .with_num_players(4)
        .unwrap()
        .add_player(PlayerType::Remote(a1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Remote(a3), PlayerHandle::new(2))?
        .add_player(PlayerType::Remote(a4), PlayerHandle::new(3))?
        .start_p2p_session(s2)?;

    // Player 3
    let mut sess3 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(pc.clone())
        .with_num_players(4)
        .unwrap()
        .add_player(PlayerType::Remote(a1), PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .add_player(PlayerType::Local, PlayerHandle::new(2))?
        .add_player(PlayerType::Remote(a4), PlayerHandle::new(3))?
        .start_p2p_session(s3)?;

    // Player 4
    let mut sess4 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(pc)
        .with_num_players(4)
        .unwrap()
        .add_player(PlayerType::Remote(a1), PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .add_player(PlayerType::Remote(a3), PlayerHandle::new(2))?
        .add_player(PlayerType::Local, PlayerHandle::new(3))?
        .start_p2p_session(s4)?;

    // Synchronize all peers
    for _ in 0..150 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        sess3.poll_remote_clients();
        sess4.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    // All sessions should be Running
    assert_eq!(sess1.current_state(), SessionState::Running);
    assert_eq!(sess2.current_state(), SessionState::Running);
    assert_eq!(sess3.current_state(), SessionState::Running);
    assert_eq!(sess4.current_state(), SessionState::Running);

    // Create game stubs for each player
    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();
    let mut stub3 = GameStub::new();
    let mut stub4 = GameStub::new();

    // Advance frames
    let reps = 15;
    for i in 0..reps {
        // Poll with virtual time advancement
        for _ in 0..3 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
            sess3.poll_remote_clients();
            sess4.poll_remote_clients();
            clock.advance(POLL_INTERVAL_DETERMINISTIC);
        }

        // Each player adds their local input
        sess1
            .add_local_input(PlayerHandle::new(0), StubInput { inp: i })
            .unwrap();
        sess2
            .add_local_input(PlayerHandle::new(1), StubInput { inp: i })
            .unwrap();
        sess3
            .add_local_input(PlayerHandle::new(2), StubInput { inp: i })
            .unwrap();
        sess4
            .add_local_input(PlayerHandle::new(3), StubInput { inp: i })
            .unwrap();

        // Advance frames
        let requests1 = sess1.advance_frame().unwrap();
        let requests2 = sess2.advance_frame().unwrap();
        let requests3 = sess3.advance_frame().unwrap();
        let requests4 = sess4.advance_frame().unwrap();

        stub1.handle_requests(requests1);
        stub2.handle_requests(requests2);
        stub3.handle_requests(requests3);
        stub4.handle_requests(requests4);

        // All game states should have advanced
        assert_eq!(stub1.gs.frame, i as i32 + 1);
        assert_eq!(stub2.gs.frame, i as i32 + 1);
        assert_eq!(stub3.gs.frame, i as i32 + 1);
        assert_eq!(stub4.gs.frame, i as i32 + 1);
    }

    Ok(())
}

/// Regression test for frame 0 rollback edge case.
///
/// This test verifies that when a misprediction is detected at frame 0
/// (the first frame), the session handles it gracefully without crashing.
#[test]
fn test_misprediction_at_frame_0_no_crash() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (s1, s2, a1, a2) = create_channel_pair();

    // Create sessions with 0 input delay to maximize prediction window
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_input_delay(0)
        .unwrap()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .start_p2p_session(s1)?;

    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_input_delay(0)
        .unwrap()
        .add_player(PlayerType::Remote(a1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(s2)?;

    synchronize_sessions_deterministic(&mut sess1, &mut sess2, &clock, &SyncConfig::default())
        .expect("Sessions should synchronize");

    assert_eq!(sess1.current_state(), SessionState::Running);
    assert_eq!(sess2.current_state(), SessionState::Running);

    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    // Add local inputs at frame 0
    // Use different inputs to force a misprediction when input arrives
    sess1.add_local_input(PlayerHandle::new(0), StubInput { inp: 100 })?;
    sess2.add_local_input(PlayerHandle::new(1), StubInput { inp: 200 })?;

    // Advance sess1 before inputs are exchanged - this should predict sess2's input
    sess1.poll_remote_clients();
    let requests1 = sess1.advance_frame()?;
    stub1.handle_requests(requests1);

    // Now exchange messages with virtual time to allow proper message processing
    // sess2's actual input (200) will differ from sess1's prediction (0)
    poll_with_advance(&mut sess1, &mut sess2, &clock, 10);

    // Advance sess2 normally
    let requests2 = sess2.advance_frame()?;
    stub2.handle_requests(requests2);

    // Continue exchanging - this may trigger the misprediction correction at frame 0
    // The key is that advance_frame should NOT panic with "must load frame in the past"
    poll_with_advance(&mut sess1, &mut sess2, &clock, 50);

    // Continue advancing frames to verify the session remains stable
    for i in 1..10 {
        poll_with_advance(&mut sess1, &mut sess2, &clock, 1);

        sess1.add_local_input(PlayerHandle::new(0), StubInput { inp: i })?;
        sess2.add_local_input(PlayerHandle::new(1), StubInput { inp: i })?;

        let requests1 = sess1.advance_frame()?;
        let requests2 = sess2.advance_frame()?;

        stub1.handle_requests(requests1);
        stub2.handle_requests(requests2);

        poll_with_advance(&mut sess1, &mut sess2, &clock, 5);
    }

    // Both sessions should have advanced past frame 0 without crashing
    assert!(stub1.gs.frame >= 10);
    assert!(stub2.gs.frame >= 10);

    Ok(())
}

// ============================================================================
// Data-Driven Tests for Session Synchronization
// ============================================================================

/// Test case configuration for data-driven synchronization tests.
#[derive(Debug, Clone)]
struct SyncTestCase {
    /// Name of the test case for error reporting
    name: &'static str,
    /// Input delay for session 1
    input_delay_1: usize,
    /// Input delay for session 2
    input_delay_2: usize,
    /// Number of frames to advance after sync
    frames_to_advance: u32,
    /// Whether to expect successful sync
    expect_success: bool,
}

/// Data-driven tests for session synchronization with various configurations.
#[test]
fn test_synchronization_data_driven() {
    let test_cases = [
        SyncTestCase {
            name: "zero_input_delay",
            input_delay_1: 0,
            input_delay_2: 0,
            frames_to_advance: 20,
            expect_success: true,
        },
        SyncTestCase {
            name: "symmetric_input_delay_2",
            input_delay_1: 2,
            input_delay_2: 2,
            frames_to_advance: 20,
            expect_success: true,
        },
        SyncTestCase {
            name: "symmetric_input_delay_5",
            input_delay_1: 5,
            input_delay_2: 5,
            frames_to_advance: 20,
            expect_success: true,
        },
        SyncTestCase {
            name: "asymmetric_input_delay_0_3",
            input_delay_1: 0,
            input_delay_2: 3,
            frames_to_advance: 20,
            expect_success: true,
        },
        SyncTestCase {
            name: "asymmetric_input_delay_3_0",
            input_delay_1: 3,
            input_delay_2: 0,
            frames_to_advance: 20,
            expect_success: true,
        },
        SyncTestCase {
            name: "asymmetric_input_delay_2_5",
            input_delay_1: 2,
            input_delay_2: 5,
            frames_to_advance: 20,
            expect_success: true,
        },
        SyncTestCase {
            name: "high_input_delay_8",
            input_delay_1: 8,
            input_delay_2: 8,
            frames_to_advance: 30,
            expect_success: true,
        },
    ];

    for case in test_cases.iter() {
        let result = run_sync_test_case(case);

        if case.expect_success {
            assert!(
                result.is_ok(),
                "Test case '{}' should succeed but got error: {:?}",
                case.name,
                result.err()
            );
        } else {
            assert!(
                result.is_err(),
                "Test case '{}' should fail but succeeded",
                case.name
            );
        }
    }
}

/// Runs a single synchronization test case using deterministic infrastructure.
#[track_caller]
fn run_sync_test_case(case: &SyncTestCase) -> Result<(), Box<dyn std::error::Error>> {
    let clock = TestClock::new();
    let (s1, s2, a1, a2) = create_channel_pair();

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .with_input_delay(case.input_delay_1)
        .unwrap()
        .start_p2p_session(s1)?;

    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(a1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .with_input_delay(case.input_delay_2)
        .unwrap()
        .start_p2p_session(s2)?;

    // Synchronize using deterministic helper
    synchronize_sessions_deterministic(&mut sess1, &mut sess2, &clock, &SyncConfig::default())?;

    // Drain sync events
    drain_sync_events(&mut sess1, &mut sess2);

    // Advance frames
    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    for i in 0..case.frames_to_advance {
        poll_with_advance(&mut sess1, &mut sess2, &clock, 3);

        sess1.add_local_input(PlayerHandle::new(0), StubInput { inp: i })?;
        sess2.add_local_input(PlayerHandle::new(1), StubInput { inp: i })?;

        let requests1 = sess1.advance_frame()?;
        let requests2 = sess2.advance_frame()?;

        stub1.handle_requests(requests1);
        stub2.handle_requests(requests2);
    }

    // Verify frames advanced
    assert!(
        stub1.gs.frame >= case.frames_to_advance as i32,
        "[{}] stub1 should have advanced to at least frame {}, got {} (input_delay_1: {}, input_delay_2: {})",
        case.name,
        case.frames_to_advance,
        stub1.gs.frame,
        case.input_delay_1,
        case.input_delay_2
    );
    assert!(
        stub2.gs.frame >= case.frames_to_advance as i32,
        "[{}] stub2 should have advanced to at least frame {}, got {} (input_delay_1: {}, input_delay_2: {})",
        case.name,
        case.frames_to_advance,
        stub2.gs.frame,
        case.input_delay_1,
        case.input_delay_2
    );

    // Verify no unexpected events
    let events1: Vec<_> = sess1.events().collect();
    let events2: Vec<_> = sess2.events().collect();
    assert!(
        events1.is_empty(),
        "[{}] Session 1 should have no unexpected events, got: {:?}",
        case.name,
        events1
    );
    assert!(
        events2.is_empty(),
        "[{}] Session 2 should have no unexpected events, got: {:?}",
        case.name,
        events2
    );

    Ok(())
}

/// Test that the synchronization helper correctly handles edge cases.
///
/// This test specifically verifies the fix for the original bug where
/// using `&&` instead of `||` in the synchronization condition could
/// cause the loop to exit prematurely when only one session was Running.
#[test]
fn test_sync_helper_both_sessions_must_be_running() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (s1, s2, a1, a2) = create_channel_pair();

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .start_p2p_session(s1)?;

    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(a1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(s2)?;

    // Initial state should be Synchronizing
    assert_eq!(
        sess1.current_state(),
        SessionState::Synchronizing,
        "Session 1 should start in Synchronizing state"
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Synchronizing,
        "Session 2 should start in Synchronizing state"
    );

    // Use the deterministic helper to synchronize
    let result =
        synchronize_sessions_deterministic(&mut sess1, &mut sess2, &clock, &SyncConfig::default());

    // Should succeed
    assert!(
        result.is_ok(),
        "Synchronization should succeed: {:?}",
        result
    );

    // CRITICAL: Both sessions MUST be Running after the helper returns
    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "Session 1 MUST be Running after synchronize_sessions returns"
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Running,
        "Session 2 MUST be Running after synchronize_sessions returns"
    );

    Ok(())
}

/// Test desync detection with various checksum intervals (data-driven).
///
/// Uses `CorruptibleGameStub` which corrupts checksums during save operations
/// rather than corrupting state before `handle_requests`.
#[test]
fn test_desync_detection_intervals_data_driven() -> Result<(), FortressError> {
    struct DesyncTestCase {
        name: &'static str,
        interval: u32,
        max_prediction: usize,
        corrupt_from_frame: i32,
        total_frames: u32,
        expected_desync_frame: i32,
    }

    let test_cases = [
        DesyncTestCase {
            name: "interval_10_quick_detection",
            interval: 10,
            max_prediction: 16,
            corrupt_from_frame: 15,
            total_frames: 50,
            expected_desync_frame: 20,
        },
        DesyncTestCase {
            name: "interval_25_medium_detection",
            interval: 25,
            max_prediction: 32,
            corrupt_from_frame: 30,
            total_frames: 80,
            expected_desync_frame: 50,
        },
        DesyncTestCase {
            name: "interval_20_corruption_just_after_checksum",
            interval: 20,
            max_prediction: 24,
            corrupt_from_frame: 21,
            total_frames: 70,
            expected_desync_frame: 40,
        },
        DesyncTestCase {
            name: "interval_20_corruption_just_before_checksum",
            interval: 20,
            max_prediction: 24,
            corrupt_from_frame: 19,
            total_frames: 50,
            expected_desync_frame: 20,
        },
        DesyncTestCase {
            name: "interval_15_corruption_at_boundary",
            interval: 15,
            max_prediction: 20,
            corrupt_from_frame: 30,
            total_frames: 60,
            expected_desync_frame: 30,
        },
        DesyncTestCase {
            name: "interval_10_corruption_from_start",
            interval: 10,
            max_prediction: 16,
            corrupt_from_frame: 0,
            total_frames: 30,
            expected_desync_frame: 10,
        },
        DesyncTestCase {
            name: "interval_40_large_detection",
            interval: 40,
            max_prediction: 48,
            corrupt_from_frame: 45,
            total_frames: 100,
            expected_desync_frame: 80,
        },
        DesyncTestCase {
            name: "interval_12_corruption_at_first_checksum",
            interval: 12,
            max_prediction: 16,
            corrupt_from_frame: 12,
            total_frames: 40,
            expected_desync_frame: 12,
        },
    ];

    for case in test_cases.iter() {
        let clock = TestClock::new();
        let (s1, s2, a1, a2) = create_channel_pair();
        let desync_mode = DesyncDetection::On {
            interval: case.interval,
        };

        let mut sess1 = SessionBuilder::<StubConfig>::new()
            .with_protocol_config(protocol_config(&clock))
            .add_player(PlayerType::Local, PlayerHandle::new(0))?
            .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
            .with_desync_detection_mode(desync_mode)
            .with_max_prediction_window(case.max_prediction)
            .start_p2p_session(s1)?;

        let mut sess2 = SessionBuilder::<StubConfig>::new()
            .with_protocol_config(protocol_config(&clock))
            .add_player(PlayerType::Remote(a1), PlayerHandle::new(0))?
            .add_player(PlayerType::Local, PlayerHandle::new(1))?
            .with_desync_detection_mode(desync_mode)
            .with_max_prediction_window(case.max_prediction)
            .start_p2p_session(s2)?;

        // Synchronize
        synchronize_sessions_deterministic(&mut sess1, &mut sess2, &clock, &SyncConfig::default())
            .unwrap_or_else(|e| panic!("[{}] sync failed: {}", case.name, e));
        drain_sync_events(&mut sess1, &mut sess2);

        // Use CorruptibleGameStub for peer 1
        let mut stub1 = CorruptibleGameStub::with_corruption_from(case.corrupt_from_frame);
        let mut stub2 = GameStub::new();

        // Run all frames
        for frame_num in 0..case.total_frames {
            poll_with_advance(&mut sess1, &mut sess2, &clock, 3);

            sess1
                .add_local_input(PlayerHandle::new(0), StubInput { inp: frame_num })
                .unwrap();
            sess2
                .add_local_input(PlayerHandle::new(1), StubInput { inp: frame_num })
                .unwrap();

            let requests1 = sess1.advance_frame().unwrap();
            let requests2 = sess2.advance_frame().unwrap();

            stub1.handle_requests(requests1);
            stub2.handle_requests(requests2);
        }

        // Check for desync events
        let events1: Vec<_> = sess1.events().collect();
        let events2: Vec<_> = sess2.events().collect();

        assert!(
            !events1.is_empty(),
            "[{}] Expected desync event for Session 1, got none. \
             Config: interval={}, max_pred={}, corrupt_from={}, total_frames={}, \
             expected_desync_frame={}",
            case.name,
            case.interval,
            case.max_prediction,
            case.corrupt_from_frame,
            case.total_frames,
            case.expected_desync_frame
        );
        assert!(
            !events2.is_empty(),
            "[{}] Expected desync event for Session 2, got none. \
             Config: interval={}, max_pred={}, corrupt_from={}, total_frames={}, \
             expected_desync_frame={}",
            case.name,
            case.interval,
            case.max_prediction,
            case.corrupt_from_frame,
            case.total_frames,
            case.expected_desync_frame
        );

        // Verify the desync frame matches expectation
        if let FortressEvent::DesyncDetected {
            frame,
            local_checksum,
            remote_checksum,
            ..
        } = &events1[0]
        {
            assert_eq!(
                *frame,
                case.expected_desync_frame,
                "[{}] Desync frame mismatch. \
                 Expected frame {} (first checksum frame >= corrupt_from {}). \
                 Got frame {}. local_checksum={:#x}, remote_checksum={:#x}. \
                 Config: interval={}, max_pred={}, total_frames={}",
                case.name,
                case.expected_desync_frame,
                case.corrupt_from_frame,
                frame,
                local_checksum,
                remote_checksum,
                case.interval,
                case.max_prediction,
                case.total_frames
            );
            assert_ne!(
                local_checksum, remote_checksum,
                "[{}] Checksums should differ in DesyncDetected event",
                case.name
            );
        } else {
            panic!(
                "[{}] Expected DesyncDetected event as first event, got {:?}. \
                 All events: {:?}.",
                case.name, events1[0], events1,
            );
        }

        // Verify Session 2 also detected at the same frame
        if let FortressEvent::DesyncDetected { frame, .. } = &events2[0] {
            assert_eq!(
                *frame, case.expected_desync_frame,
                "[{}] Session 2 desync frame mismatch. All Session 2 events: {:?}.",
                case.name, events2,
            );
        } else if !events2.is_empty() {
            panic!(
                "[{}] Session 2: Expected DesyncDetected event as first event, got {:?}.",
                case.name, events2[0],
            );
        }
    }

    Ok(())
}

// ============================================================================
// Polling Robustness Tests
// ============================================================================

/// Test case for timing robustness testing.
#[derive(Debug, Clone)]
struct TimingTestCase {
    /// Name of the test case for error reporting
    name: &'static str,
    /// Number of poll iterations per frame
    polls_per_frame: usize,
    /// Number of frames to advance
    frames: u32,
    /// Input delay for both sessions
    input_delay: usize,
}

/// Data-driven tests for polling robustness.
#[test]
fn test_polling_robustness_data_driven() {
    let test_cases = [
        TimingTestCase {
            name: "single_poll_many_frames",
            polls_per_frame: 1,
            frames: 30,
            input_delay: 0,
        },
        TimingTestCase {
            name: "triple_poll_many_frames",
            polls_per_frame: 3,
            frames: 30,
            input_delay: 0,
        },
        TimingTestCase {
            name: "heavy_poll_few_frames",
            polls_per_frame: 10,
            frames: 10,
            input_delay: 0,
        },
        TimingTestCase {
            name: "single_poll_with_delay",
            polls_per_frame: 1,
            frames: 20,
            input_delay: 3,
        },
        TimingTestCase {
            name: "triple_poll_with_delay",
            polls_per_frame: 3,
            frames: 20,
            input_delay: 3,
        },
        TimingTestCase {
            name: "high_delay_triple_poll",
            polls_per_frame: 3,
            frames: 25,
            input_delay: 7,
        },
    ];

    for case in test_cases.iter() {
        let result = run_timing_test_case(case);

        assert!(
            result.is_ok(),
            "Test case '{}' should succeed but got error: {:?}",
            case.name,
            result.err()
        );
    }
}

/// Runs a single timing robustness test case using deterministic infrastructure.
#[track_caller]
fn run_timing_test_case(case: &TimingTestCase) -> Result<(), Box<dyn std::error::Error>> {
    let clock = TestClock::new();
    let (s1, s2, a1, a2) = create_channel_pair();

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .with_input_delay(case.input_delay)
        .unwrap()
        .start_p2p_session(s1)?;

    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(a1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .with_input_delay(case.input_delay)
        .unwrap()
        .start_p2p_session(s2)?;

    // Synchronize using deterministic helper
    synchronize_sessions_deterministic(&mut sess1, &mut sess2, &clock, &SyncConfig::default())?;

    // Drain sync events
    drain_sync_events(&mut sess1, &mut sess2);

    // Advance frames with the specified polling pattern
    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    for i in 0..case.frames {
        // Poll with the specified number of iterations using virtual time
        for _ in 0..case.polls_per_frame {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
            clock.advance(POLL_INTERVAL_DETERMINISTIC);
        }

        sess1.add_local_input(PlayerHandle::new(0), StubInput { inp: i })?;
        sess2.add_local_input(PlayerHandle::new(1), StubInput { inp: i })?;

        let requests1 = sess1.advance_frame()?;
        let requests2 = sess2.advance_frame()?;

        stub1.handle_requests(requests1);
        stub2.handle_requests(requests2);
    }

    // Verify frames advanced
    assert!(
        stub1.gs.frame >= case.frames as i32,
        "[{}] stub1 should have advanced to at least frame {}, got {} \
         (polls_per_frame: {}, input_delay: {})",
        case.name,
        case.frames,
        stub1.gs.frame,
        case.polls_per_frame,
        case.input_delay
    );
    assert!(
        stub2.gs.frame >= case.frames as i32,
        "[{}] stub2 should have advanced to at least frame {}, got {} \
         (polls_per_frame: {}, input_delay: {})",
        case.name,
        case.frames,
        stub2.gs.frame,
        case.polls_per_frame,
        case.input_delay
    );

    Ok(())
}

// ==========================================
// P2PSession local_player_handle_required Tests
// ==========================================

#[test]
fn p2p_session_local_player_handle_required_with_one_local_returns_ok() -> Result<(), FortressError>
{
    let clock = TestClock::new();
    let (socket, _addr0) = create_unconnected_socket(10000);
    let remote_addr: SocketAddr = ([127, 0, 0, 1], 10001).into();

    let sess = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(remote_addr), PlayerHandle::new(1))?
        .start_p2p_session(socket)?;

    let result = sess.local_player_handle_required();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), PlayerHandle::new(0));
    Ok(())
}

#[test]
fn p2p_session_local_player_handle_required_with_zero_local_returns_error(
) -> Result<(), FortressError> {
    use fortress_rollback::InvalidRequestKind;

    let clock = TestClock::new();
    let (socket, _addr0) = create_unconnected_socket(10000);
    let remote_addr1: SocketAddr = ([127, 0, 0, 1], 10001).into();
    let remote_addr2: SocketAddr = ([127, 0, 0, 1], 10002).into();

    let sess = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(2)?
        .add_player(PlayerType::Remote(remote_addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(remote_addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket)?;

    let result = sess.local_player_handle_required();
    assert!(result.is_err());

    let err = result.unwrap_err();
    match err {
        FortressError::InvalidRequestStructured {
            kind: InvalidRequestKind::NoLocalPlayers,
        } => (), // Expected
        _ => panic!("Expected NoLocalPlayers error, got: {:?}", err),
    }
    Ok(())
}

#[test]
fn p2p_session_local_player_handle_required_with_multiple_local_returns_error(
) -> Result<(), FortressError> {
    use fortress_rollback::InvalidRequestKind;

    let clock = TestClock::new();
    let (socket, _addr0) = create_unconnected_socket(10000);

    let sess = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(2)?
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket)?;

    let result = sess.local_player_handle_required();
    assert!(result.is_err());

    let err = result.unwrap_err();
    match err {
        FortressError::InvalidRequestStructured {
            kind: InvalidRequestKind::MultipleLocalPlayers { count },
        } => {
            assert_eq!(count, 2);
        },
        _ => panic!("Expected MultipleLocalPlayers error, got: {:?}", err),
    }
    Ok(())
}

// ==========================================
// P2PSession remote_player_handle_required Tests
// ==========================================

#[test]
fn p2p_session_remote_player_handle_required_with_one_remote_returns_ok(
) -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (socket, _addr0) = create_unconnected_socket(10000);
    let remote_addr: SocketAddr = ([127, 0, 0, 1], 10001).into();

    let sess = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(remote_addr), PlayerHandle::new(1))?
        .start_p2p_session(socket)?;

    let result = sess.remote_player_handle_required();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), PlayerHandle::new(1));
    Ok(())
}

#[test]
fn p2p_session_remote_player_handle_required_with_zero_remote_returns_error(
) -> Result<(), FortressError> {
    use fortress_rollback::InvalidRequestKind;

    let clock = TestClock::new();
    let (socket, _addr0) = create_unconnected_socket(10000);

    let sess = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(2)?
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket)?;

    let result = sess.remote_player_handle_required();
    assert!(result.is_err());

    let err = result.unwrap_err();
    match err {
        FortressError::InvalidRequestStructured {
            kind: InvalidRequestKind::NoRemotePlayers,
        } => (), // Expected
        _ => panic!("Expected NoRemotePlayers error, got: {:?}", err),
    }
    Ok(())
}

#[test]
fn p2p_session_remote_player_handle_required_with_multiple_remote_returns_error(
) -> Result<(), FortressError> {
    use fortress_rollback::InvalidRequestKind;

    let clock = TestClock::new();
    let (socket, _addr0) = create_unconnected_socket(10000);
    let remote_addr1: SocketAddr = ([127, 0, 0, 1], 10001).into();
    let remote_addr2: SocketAddr = ([127, 0, 0, 1], 10002).into();

    let sess = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_num_players(3)?
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(remote_addr1), PlayerHandle::new(1))?
        .add_player(PlayerType::Remote(remote_addr2), PlayerHandle::new(2))?
        .start_p2p_session(socket)?;

    let result = sess.remote_player_handle_required();
    assert!(result.is_err());

    let err = result.unwrap_err();
    match err {
        FortressError::InvalidRequestStructured {
            kind: InvalidRequestKind::MultipleRemotePlayers { count },
        } => {
            assert_eq!(count, 2);
        },
        _ => panic!("Expected MultipleRemotePlayers error, got: {:?}", err),
    }
    Ok(())
}
