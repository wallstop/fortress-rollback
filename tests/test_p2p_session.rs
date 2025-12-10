mod stubs;

use fortress_rollback::{
    DesyncDetection, FortressError, FortressEvent, PlayerHandle, PlayerType, SessionBuilder,
    SessionState, UdpNonBlockingSocket,
};
use serial_test::serial;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use stubs::{GameStub, StubConfig, StubInput};

#[test]
#[serial]
fn test_add_more_players() -> Result<(), FortressError> {
    let socket = UdpNonBlockingSocket::bind_to_port(7777).unwrap();
    let remote_addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    let remote_addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8081);
    let remote_addr3 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8082);
    let spec_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8090);

    let _sess = SessionBuilder::<StubConfig>::new()
        .with_num_players(4)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(remote_addr1), PlayerHandle::new(1))?
        .add_player(PlayerType::Remote(remote_addr2), PlayerHandle::new(2))?
        .add_player(PlayerType::Remote(remote_addr3), PlayerHandle::new(3))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(4))?
        .start_p2p_session(socket)?;
    Ok(())
}

#[test]
#[serial]
fn test_start_session() -> Result<(), FortressError> {
    let socket = UdpNonBlockingSocket::bind_to_port(7777).unwrap();
    let remote_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    let spec_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8090);

    let _sess = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(remote_addr), PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spec_addr), PlayerHandle::new(2))?
        .start_p2p_session(socket)?;
    Ok(())
}

#[test]
#[serial]
fn test_disconnect_player() -> Result<(), FortressError> {
    let socket = UdpNonBlockingSocket::bind_to_port(7777).unwrap();
    let remote_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    let spec_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8090);

    let mut sess = SessionBuilder::<StubConfig>::new()
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
#[serial]
fn test_synchronize_p2p_sessions() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 7777);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8888);

    let socket1 = UdpNonBlockingSocket::bind_to_port(7777).unwrap();
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = UdpNonBlockingSocket::bind_to_port(8888).unwrap();
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .start_p2p_session(socket2)?;

    assert!(sess1.current_state() == SessionState::Synchronizing);
    assert!(sess2.current_state() == SessionState::Synchronizing);

    for _ in 0..50 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
    }

    assert!(sess1.current_state() == SessionState::Running);
    assert!(sess2.current_state() == SessionState::Running);

    Ok(())
}

#[test]
#[serial]
fn test_advance_frame_p2p_sessions() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 7777);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8888);

    let socket1 = UdpNonBlockingSocket::bind_to_port(7777).unwrap();
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = UdpNonBlockingSocket::bind_to_port(8888).unwrap();
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    assert!(sess1.current_state() == SessionState::Synchronizing);
    assert!(sess2.current_state() == SessionState::Synchronizing);

    for _ in 0..50 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
    }

    assert!(sess1.current_state() == SessionState::Running);
    assert!(sess2.current_state() == SessionState::Running);

    let mut stub1 = stubs::GameStub::new();
    let mut stub2 = stubs::GameStub::new();
    let reps = 10;
    for i in 0..reps {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        sess1
            .add_local_input(PlayerHandle::new(0), StubInput { inp: i })
            .unwrap();
        let requests1 = sess1.advance_frame().unwrap();
        stub1.handle_requests(requests1);
        sess2
            .add_local_input(PlayerHandle::new(1), StubInput { inp: i })
            .unwrap();
        let requests2 = sess2.advance_frame().unwrap();
        stub2.handle_requests(requests2);

        // gamestate evolves
        assert_eq!(stub1.gs.frame, i as i32 + 1);
        assert_eq!(stub2.gs.frame, i as i32 + 1);
    }

    Ok(())
}

#[test]
#[serial]
fn test_desyncs_detected() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 7777);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8888);
    let desync_mode = DesyncDetection::On { interval: 100 };

    let socket1 = UdpNonBlockingSocket::bind_to_port(7777).unwrap();
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .with_desync_detection_mode(desync_mode)
        .start_p2p_session(socket1)?;

    let socket2 = UdpNonBlockingSocket::bind_to_port(8888).unwrap();
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .with_desync_detection_mode(desync_mode)
        .start_p2p_session(socket2)?;

    while sess1.current_state() != SessionState::Running
        && sess2.current_state() != SessionState::Running
    {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
    }

    // drain events
    assert!(sess1.events().chain(sess2.events()).all(|e| matches!(
        e,
        FortressEvent::Synchronizing { .. } | FortressEvent::Synchronized { .. }
    )));

    let mut stub1 = stubs::GameStub::new();
    let mut stub2 = stubs::GameStub::new();

    // run normally for some frames (past first desync interval)
    for i in 0..110 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

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
    assert_eq!(sess1.events().len(), 0);
    assert_eq!(sess2.events().len(), 0);

    // run for some more frames with steady inputs
    for _ in 0..100 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

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
    assert_eq!(desync_addr1, addr2);
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
    assert_eq!(desync_addr2, addr1);
    assert_ne!(desync_local_checksum2, desync_remote_checksum2);

    // check that checksums match
    assert_eq!(desync_remote_checksum1, desync_local_checksum2);
    assert_eq!(desync_remote_checksum2, desync_local_checksum1);

    Ok(())
}

#[test]
#[serial]
fn test_desyncs_and_input_delay_no_panic() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 7777);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8888);
    let desync_mode = DesyncDetection::On { interval: 100 };

    let socket1 = UdpNonBlockingSocket::bind_to_port(7777).unwrap();
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .with_input_delay(5)
        .with_desync_detection_mode(desync_mode)
        .start_p2p_session(socket1)?;

    let socket2 = UdpNonBlockingSocket::bind_to_port(8888).unwrap();
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .with_input_delay(5)
        .with_desync_detection_mode(desync_mode)
        .start_p2p_session(socket2)?;

    while sess1.current_state() != SessionState::Running
        && sess2.current_state() != SessionState::Running
    {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
    }

    // drain events
    assert!(sess1.events().chain(sess2.events()).all(|e| matches!(
        e,
        FortressEvent::Synchronizing { .. } | FortressEvent::Synchronized { .. }
    )));

    let mut stub1 = stubs::GameStub::new();
    let mut stub2 = stubs::GameStub::new();

    // run normally for some frames (past first desync interval)
    for i in 0..150 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

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
#[serial]
fn test_three_player_session() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 7001);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 7002);
    let addr3 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 7003);

    // Player 1: local=0, remote=1,2
    let socket1 = UdpNonBlockingSocket::bind_to_port(7001).unwrap();
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_num_players(3)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .add_player(PlayerType::Remote(addr3), PlayerHandle::new(2))?
        .start_p2p_session(socket1)?;

    // Player 2: local=1, remote=0,2
    let socket2 = UdpNonBlockingSocket::bind_to_port(7002).unwrap();
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_num_players(3)
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Remote(addr3), PlayerHandle::new(2))?
        .start_p2p_session(socket2)?;

    // Player 3: local=2, remote=0,1
    let socket3 = UdpNonBlockingSocket::bind_to_port(7003).unwrap();
    let mut sess3 = SessionBuilder::<StubConfig>::new()
        .with_num_players(3)
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .add_player(PlayerType::Local, PlayerHandle::new(2))?
        .start_p2p_session(socket3)?;

    // All sessions should start in Synchronizing state
    assert_eq!(sess1.current_state(), SessionState::Synchronizing);
    assert_eq!(sess2.current_state(), SessionState::Synchronizing);
    assert_eq!(sess3.current_state(), SessionState::Synchronizing);

    // Synchronize all peers
    for _ in 0..100 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        sess3.poll_remote_clients();
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
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        sess3.poll_remote_clients();

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
#[serial]
fn test_four_player_session() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 7011);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 7012);
    let addr3 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 7013);
    let addr4 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 7014);

    // Player 1
    let socket1 = UdpNonBlockingSocket::bind_to_port(7011).unwrap();
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_num_players(4)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .add_player(PlayerType::Remote(addr3), PlayerHandle::new(2))?
        .add_player(PlayerType::Remote(addr4), PlayerHandle::new(3))?
        .start_p2p_session(socket1)?;

    // Player 2
    let socket2 = UdpNonBlockingSocket::bind_to_port(7012).unwrap();
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_num_players(4)
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Remote(addr3), PlayerHandle::new(2))?
        .add_player(PlayerType::Remote(addr4), PlayerHandle::new(3))?
        .start_p2p_session(socket2)?;

    // Player 3
    let socket3 = UdpNonBlockingSocket::bind_to_port(7013).unwrap();
    let mut sess3 = SessionBuilder::<StubConfig>::new()
        .with_num_players(4)
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .add_player(PlayerType::Local, PlayerHandle::new(2))?
        .add_player(PlayerType::Remote(addr4), PlayerHandle::new(3))?
        .start_p2p_session(socket3)?;

    // Player 4
    let socket4 = UdpNonBlockingSocket::bind_to_port(7014).unwrap();
    let mut sess4 = SessionBuilder::<StubConfig>::new()
        .with_num_players(4)
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .add_player(PlayerType::Remote(addr3), PlayerHandle::new(2))?
        .add_player(PlayerType::Local, PlayerHandle::new(3))?
        .start_p2p_session(socket4)?;

    // Synchronize all peers
    for _ in 0..150 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        sess3.poll_remote_clients();
        sess4.poll_remote_clients();
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
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        sess3.poll_remote_clients();
        sess4.poll_remote_clients();

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
///
/// Previously, if `first_incorrect_frame == current_frame == 0`, the code
/// would attempt to call `load_frame(0)` which would fail because you cannot
/// load the frame you're currently on (the guard checks `frame_to_load >= current_frame`).
///
/// The fix is to detect when `frame_to_load >= current_frame` and skip the rollback,
/// since we haven't actually advanced past the incorrect frame yet - we just need
/// to reset predictions.
///
/// This scenario can happen in terrible network conditions when:
/// 1. Session is at frame 0
/// 2. Remote player's input for frame 0 is predicted (not yet received)
/// 3. Actual input arrives differing from prediction during same advance_frame() cycle
/// 4. first_incorrect_frame = 0, current_frame = 0
/// 5. adjust_gamestate(0, ...) is called
#[test]
#[serial]
fn test_misprediction_at_frame_0_no_crash() -> Result<(), FortressError> {
    use std::time::Duration;

    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9910);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9911);

    let socket1 = UdpNonBlockingSocket::bind_to_port(9910).unwrap();
    let socket2 = UdpNonBlockingSocket::bind_to_port(9911).unwrap();

    // Create sessions with 0 input delay to maximize prediction window
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_input_delay(0)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_input_delay(0)
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize sessions
    for _ in 0..100 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        std::thread::sleep(Duration::from_millis(1));
    }

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

    // Now exchange messages - sess2's actual input (200) will differ from sess1's prediction (0)
    for _ in 0..10 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
    }

    // Advance sess2 normally
    let requests2 = sess2.advance_frame()?;
    stub2.handle_requests(requests2);

    // Continue exchanging - this may trigger the misprediction correction at frame 0
    // The key is that advance_frame should NOT panic with "must load frame in the past"
    for _ in 0..50 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
    }

    // Continue advancing frames to verify the session remains stable
    for i in 1..10 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        sess1.add_local_input(PlayerHandle::new(0), StubInput { inp: i })?;
        sess2.add_local_input(PlayerHandle::new(1), StubInput { inp: i })?;

        let requests1 = sess1.advance_frame()?;
        let requests2 = sess2.advance_frame()?;

        stub1.handle_requests(requests1);
        stub2.handle_requests(requests2);

        for _ in 0..5 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
        }
    }

    // Both sessions should have advanced past frame 0 without crashing
    assert!(stub1.gs.frame >= 10);
    assert!(stub2.gs.frame >= 10);

    Ok(())
}
