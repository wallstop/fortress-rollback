//! P2P session integration tests with enum-based inputs.

// Allow hardcoded IP addresses - 127.0.0.1 is appropriate for tests
#![allow(clippy::ip_constant)]

use crate::common::stubs_enum::{EnumInput, GameStubEnum, StubEnumConfig};
use fortress_rollback::{
    FortressError, PlayerHandle, PlayerType, SessionBuilder, SessionState, UdpNonBlockingSocket,
};
use serial_test::serial;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::thread;
use std::time::{Duration, Instant};

/// Maximum time to wait for synchronization to complete.
const SYNC_TIMEOUT: Duration = Duration::from_secs(5);

/// Time to sleep between poll iterations to allow for proper timing.
const POLL_INTERVAL: Duration = Duration::from_millis(1);

#[test]
#[serial]
fn test_advance_frame_p2p_sessions_enum() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 7777);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8888);

    let socket1 = UdpNonBlockingSocket::bind_to_port(7777).unwrap();
    let mut sess1 = SessionBuilder::<StubEnumConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = UdpNonBlockingSocket::bind_to_port(8888).unwrap();
    let mut sess2 = SessionBuilder::<StubEnumConfig>::new()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    assert!(sess1.current_state() == SessionState::Synchronizing);
    assert!(sess2.current_state() == SessionState::Synchronizing);

    // Use robust synchronization with time-based timeout
    let start = Instant::now();
    while start.elapsed() < SYNC_TIMEOUT {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
        thread::sleep(POLL_INTERVAL);
    }

    assert!(
        sess1.current_state() == SessionState::Running,
        "Session 1 failed to synchronize after {:?}, state: {:?}",
        start.elapsed(),
        sess1.current_state()
    );
    assert!(
        sess2.current_state() == SessionState::Running,
        "Session 2 failed to synchronize after {:?}, state: {:?}",
        start.elapsed(),
        sess2.current_state()
    );

    let mut stub1 = GameStubEnum::new();
    let mut stub2 = GameStubEnum::new();
    let reps = 10;
    for i in 0..reps {
        // Poll with multiple iterations and sleep to ensure packets are delivered.
        // This is crucial on systems with different scheduling behavior (e.g., macOS CI)
        // where tight loops may not give the network stack enough time.
        for _ in 0..3 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
            thread::sleep(POLL_INTERVAL);
        }

        sess1
            .add_local_input(
                PlayerHandle::new(0),
                if i % 2 == 0 {
                    EnumInput::Val1
                } else {
                    EnumInput::Val2
                },
            )
            .unwrap();
        let requests1 = sess1.advance_frame().unwrap();
        stub1.handle_requests(requests1);
        sess2
            .add_local_input(
                PlayerHandle::new(1),
                if i % 3 == 0 {
                    EnumInput::Val1
                } else {
                    EnumInput::Val2
                },
            )
            .unwrap();
        let requests2 = sess2.advance_frame().unwrap();
        stub2.handle_requests(requests2);

        // gamestate evolves
        assert_eq!(stub1.gs.frame, i + 1);
        assert_eq!(stub2.gs.frame, i + 1);
    }

    Ok(())
}
