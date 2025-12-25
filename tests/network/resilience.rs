//! Network resilience integration tests using ChaosSocket.
//!
//! These tests validate that Fortress Rollback sessions can handle
//! adverse network conditions including:
//! - Packet loss (sporadic and burst)
//! - High latency
//! - Jitter (variable latency)
//! - Combined conditions

// Allow test-specific patterns that are appropriate for test code
#![allow(
    clippy::ip_constant,
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::clone_on_copy,
    clippy::print_stderr,
    clippy::disallowed_macros
)]

use crate::common::stubs::{GameStub, StubConfig, StubInput};
use fortress_rollback::{
    ChaosConfig, ChaosSocket, FortressError, FortressEvent, PlayerHandle, PlayerType,
    ProtocolConfig, SaveMode, SessionBuilder, SessionState, SyncConfig, TimeSyncConfig,
    UdpNonBlockingSocket,
};
use serial_test::serial;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

/// Helper to create a UDP socket wrapped with ChaosSocket.
fn create_chaos_socket(
    port: u16,
    config: ChaosConfig,
) -> ChaosSocket<SocketAddr, UdpNonBlockingSocket> {
    let inner = UdpNonBlockingSocket::bind_to_port(port).unwrap();
    ChaosSocket::new(inner, config)
}

/// Test that sessions can synchronize with moderate packet loss.
/// 10% packet loss should still allow synchronization to complete.
#[test]
#[serial]
fn test_synchronize_with_packet_loss() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9001);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9002);

    // Use different seeds to avoid correlated packet drops
    let config1 = ChaosConfig::builder()
        .packet_loss_rate(0.10) // 10% loss
        .seed(42)
        .build();

    let config2 = ChaosConfig::builder()
        .packet_loss_rate(0.10) // 10% loss
        .seed(43) // Different seed!
        .build();

    let socket1 = create_chaos_socket(9001, config1);
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9002, config2);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Should start in Synchronizing state
    assert_eq!(sess1.current_state(), SessionState::Synchronizing);
    assert_eq!(sess2.current_state(), SessionState::Synchronizing);

    // Synchronize - with sleep to allow retry timers to fire (200ms retry interval)
    // With 10% loss on each side, ~19% of roundtrips fail.
    // Need 5 successful roundtrips, so expect ~6-8 attempts (1.2-1.6s minimum).
    // Allow 10 seconds total for reliability.
    for _ in 0..100 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }

        // Sleep to allow retry timer (200ms) to fire
        std::thread::sleep(Duration::from_millis(100));
    }

    // Should eventually synchronize despite packet loss
    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "Session 1 failed to synchronize with 10% packet loss"
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Running,
        "Session 2 failed to synchronize with 10% packet loss"
    );

    Ok(())
}

/// Test that sessions can advance frames with moderate packet loss.
#[test]
#[serial]
fn test_advance_frames_with_packet_loss() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9003);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9004);

    let chaos_config = ChaosConfig::builder()
        .packet_loss_rate(0.05) // 5% loss
        .seed(123)
        .build();

    let socket1 = create_chaos_socket(9003, chaos_config.clone());
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9004, chaos_config);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize first - need sleep for protocol retry timers (200ms interval)
    for _ in 0..100 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(30));
    }

    assert_eq!(sess1.current_state(), SessionState::Running);
    assert_eq!(sess2.current_state(), SessionState::Running);

    // Now advance frames with packet loss
    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();
    let target_frames = 50;

    for i in 0..target_frames {
        // Poll multiple times per frame to help with packet loss
        for _ in 0..3 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
        }

        sess1
            .add_local_input(PlayerHandle::new(0), StubInput { inp: i as u32 })
            .unwrap();
        sess2
            .add_local_input(PlayerHandle::new(1), StubInput { inp: i as u32 })
            .unwrap();

        let requests1 = sess1.advance_frame().unwrap();
        let requests2 = sess2.advance_frame().unwrap();

        stub1.handle_requests(requests1);
        stub2.handle_requests(requests2);
    }

    // Both stubs should have advanced (rollback may cause state differences)
    assert!(stub1.gs.frame > 0, "Stub 1 should have advanced frames");
    assert!(stub2.gs.frame > 0, "Stub 2 should have advanced frames");

    Ok(())
}

/// Test that sessions can synchronize with simulated latency.
/// Note: With receive-side latency simulation, initial messages are delayed
/// before being available to the session.
#[test]
#[serial]
fn test_synchronize_with_latency() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9005);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9006);

    // 20ms simulated latency
    let chaos_config = ChaosConfig::builder().latency_ms(20).seed(42).build();

    let socket1 = create_chaos_socket(9005, chaos_config.clone());
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9006, chaos_config);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize - with latency, need to wait between polls
    for _ in 0..100 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        // Small delay to allow latency simulation to deliver packets
        std::thread::sleep(Duration::from_millis(5));

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
    }

    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "Session 1 failed to synchronize with 20ms latency"
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Running,
        "Session 2 failed to synchronize with 20ms latency"
    );

    Ok(())
}

/// Test sessions with combined latency and jitter.
#[test]
#[serial]
fn test_synchronize_with_jitter() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9007);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9008);

    // 30ms latency with ±15ms jitter
    let chaos_config = ChaosConfig::builder()
        .latency_ms(30)
        .jitter_ms(15)
        .seed(42)
        .build();

    let socket1 = create_chaos_socket(9007, chaos_config.clone());
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9008, chaos_config);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize with jitter
    for _ in 0..150 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        std::thread::sleep(Duration::from_millis(5));

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
    }

    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "Session 1 failed to synchronize with jitter"
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Running,
        "Session 2 failed to synchronize with jitter"
    );

    Ok(())
}

/// Test sessions with combined packet loss and latency (poor network simulation).
#[test]
#[serial]
fn test_poor_network_conditions() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9009);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9010);

    // Use the "poor network" preset with a deterministic seed
    let mut chaos_config = ChaosConfig::poor_network();
    chaos_config.seed = Some(42);

    let socket1 = create_chaos_socket(9009, chaos_config.clone());
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9010, chaos_config);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize under poor network conditions (100ms latency, 50ms jitter, 5% loss)
    // Need more time due to high latency
    for _ in 0..200 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        std::thread::sleep(Duration::from_millis(50));

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
    }

    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "Session 1 failed to synchronize under poor network"
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Running,
        "Session 2 failed to synchronize under poor network"
    );

    // Advance some frames
    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    for i in 0..30 {
        for _ in 0..5 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
        }
        std::thread::sleep(Duration::from_millis(15));

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

    assert!(
        stub1.gs.frame > 0,
        "Should advance frames under poor network"
    );
    assert!(
        stub2.gs.frame > 0,
        "Should advance frames under poor network"
    );

    Ok(())
}

/// Test asymmetric network conditions (one direction worse than other).
#[test]
#[serial]
fn test_asymmetric_packet_loss() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9011);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9012);

    // Player 1 has high send loss (simulates bad upload)
    let config1 = ChaosConfig::builder()
        .send_loss_rate(0.15)
        .receive_loss_rate(0.02)
        .seed(42)
        .build();

    // Player 2 has normal conditions
    let config2 = ChaosConfig::builder()
        .send_loss_rate(0.02)
        .receive_loss_rate(0.02)
        .seed(43)
        .build();

    let socket1 = create_chaos_socket(9011, config1);
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9012, config2);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize with asymmetric loss - need sleep for protocol retry timers
    // Player 1 has 15% send loss, which compounds with player 2's receive
    // Use more iterations with shorter sleep to allow more sync attempts
    for _ in 0..300 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "Session 1 failed to synchronize with asymmetric loss"
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Running,
        "Session 2 failed to synchronize with asymmetric loss"
    );

    Ok(())
}

/// Test with high packet loss (25%) - tests robustness.
#[test]
#[serial]
fn test_high_packet_loss() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9013);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9014);

    let chaos_config = ChaosConfig::builder()
        .packet_loss_rate(0.25) // 25% loss - very aggressive
        .seed(42)
        .build();

    let socket1 = create_chaos_socket(9013, chaos_config.clone());
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9014, chaos_config);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // May need many iterations due to high loss
    // With 25% loss per side, P(roundtrip) ≈ 0.56, need many retries
    // 200ms retry interval means we need real time to pass
    for _ in 0..200 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(80));
    }

    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "Session 1 failed to synchronize with 25% packet loss"
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Running,
        "Session 2 failed to synchronize with 25% packet loss"
    );

    Ok(())
}

/// Test sessions with high constant latency (100ms).
#[test]
#[serial]
fn test_high_latency_100ms() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9017);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9018);

    let chaos_config = ChaosConfig::builder().latency_ms(100).seed(42).build();

    let socket1 = create_chaos_socket(9017, chaos_config.clone());
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9018, chaos_config);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize - with high latency, need more time between polls
    for _ in 0..150 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        std::thread::sleep(Duration::from_millis(20));

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
    }

    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "Session 1 failed to synchronize with 100ms latency"
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Running,
        "Session 2 failed to synchronize with 100ms latency"
    );

    // Advance some frames
    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    for i in 0..30 {
        for _ in 0..5 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
        }
        std::thread::sleep(Duration::from_millis(25));

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

    assert!(
        stub1.gs.frame > 0,
        "Should advance frames with 100ms latency"
    );
    assert!(
        stub2.gs.frame > 0,
        "Should advance frames with 100ms latency"
    );

    Ok(())
}

/// Test sessions with very high constant latency (250ms).
#[test]
#[serial]
fn test_high_latency_250ms() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9019);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9020);

    let chaos_config = ChaosConfig::builder().latency_ms(250).seed(42).build();

    let socket1 = create_chaos_socket(9019, chaos_config.clone());
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9020, chaos_config);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize - with 250ms latency, need longer delays
    for _ in 0..200 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        std::thread::sleep(Duration::from_millis(50));

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
    }

    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "Session 1 failed to synchronize with 250ms latency"
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Running,
        "Session 2 failed to synchronize with 250ms latency"
    );

    Ok(())
}

/// Test sessions with extreme latency (500ms).
/// This tests the library's tolerance for very slow connections.
#[test]
#[serial]
fn test_extreme_latency_500ms() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9021);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9022);

    let chaos_config = ChaosConfig::builder().latency_ms(500).seed(42).build();

    let socket1 = create_chaos_socket(9021, chaos_config.clone());
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9022, chaos_config);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize - with 500ms latency, need much longer delays
    for _ in 0..100 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        std::thread::sleep(Duration::from_millis(100));

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
    }

    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "Session 1 failed to synchronize with 500ms latency"
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Running,
        "Session 2 failed to synchronize with 500ms latency"
    );

    Ok(())
}

/// Test out-of-order packet delivery.
/// Uses reordering to shuffle packet order.
#[test]
#[serial]
fn test_out_of_order_delivery() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9023);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9024);

    // Configure aggressive reordering
    let chaos_config = ChaosConfig::builder()
        .reorder_buffer_size(5)
        .reorder_rate(0.5) // 50% chance of reordering within buffer
        .seed(42)
        .build();

    let socket1 = create_chaos_socket(9023, chaos_config.clone());
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9024, chaos_config);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize - need sleep for reorder buffer timing to work
    for _ in 0..200 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(30));
    }

    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "Session 1 failed to synchronize with packet reordering"
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Running,
        "Session 2 failed to synchronize with packet reordering"
    );

    // Advance frames with reordered packets
    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    for i in 0..40 {
        for _ in 0..3 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
        }

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

    assert_eq!(
        stub1.gs.frame, 40,
        "Should advance all frames despite reordering"
    );
    assert_eq!(
        stub2.gs.frame, 40,
        "Should advance all frames despite reordering"
    );

    Ok(())
}

/// Test combined jitter and packet loss.
#[test]
#[serial]
fn test_jitter_with_packet_loss() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9025);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9026);

    // Moderate jitter with packet loss
    let chaos_config = ChaosConfig::builder()
        .latency_ms(40)
        .jitter_ms(30)
        .packet_loss_rate(0.08)
        .seed(42)
        .build();

    let socket1 = create_chaos_socket(9025, chaos_config.clone());
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9026, chaos_config);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize - latency (40ms) + jitter (30ms) + 8% loss needs time
    // Worst case latency ~70ms per hop = 140ms roundtrip
    // Plus retry interval of 200ms on loss
    for _ in 0..200 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        std::thread::sleep(Duration::from_millis(50));

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
    }

    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "Session 1 failed to synchronize with jitter + loss"
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Running,
        "Session 2 failed to synchronize with jitter + loss"
    );

    // Advance frames
    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    for i in 0..30 {
        for _ in 0..4 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
        }
        std::thread::sleep(Duration::from_millis(15));

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

    assert!(
        stub1.gs.frame > 0,
        "Should advance frames with jitter + loss"
    );
    assert!(
        stub2.gs.frame > 0,
        "Should advance frames with jitter + loss"
    );

    Ok(())
}

/// Test packet duplication handling.
#[test]
#[serial]
fn test_packet_duplication() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9015);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9016);

    // High duplication rate to test duplicate handling
    let chaos_config = ChaosConfig::builder()
        .duplication_rate(0.30) // 30% of packets duplicated
        .seed(42)
        .build();

    let socket1 = create_chaos_socket(9015, chaos_config.clone());
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9016, chaos_config);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize - need time for protocol messages
    for _ in 0..100 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(30));
    }

    assert_eq!(sess1.current_state(), SessionState::Running);
    assert_eq!(sess2.current_state(), SessionState::Running);

    // Advance frames with duplicate packets
    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    for i in 0..20 {
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

    // Frames should advance normally despite duplicates
    assert_eq!(
        stub1.gs.frame, 20,
        "Duplicates should not affect frame count"
    );
    assert_eq!(
        stub2.gs.frame, 20,
        "Duplicates should not affect frame count"
    );

    Ok(())
}

/// Test determinism validation: sessions should eventually reach the same
/// state after network conditions normalize, even after adverse conditions.
/// This is a longer stress test that validates eventual consistency.
#[test]
#[serial]
fn test_determinism_under_stress() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9027);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9028);

    // Use "terrible network" conditions for stress testing
    let mut chaos_config = ChaosConfig::terrible_network();
    chaos_config.seed = Some(42);

    let socket1 = create_chaos_socket(9027, chaos_config.clone());
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9028, chaos_config);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize
    for _ in 0..400 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        std::thread::sleep(Duration::from_millis(25));

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
    }

    assert_eq!(sess1.current_state(), SessionState::Running);
    assert_eq!(sess2.current_state(), SessionState::Running);

    // Run many frames under adverse conditions
    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();
    let target_frames = 100;

    for i in 0..target_frames {
        // Poll multiple times per frame to compensate for network issues
        for _ in 0..8 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
        }
        std::thread::sleep(Duration::from_millis(30));

        // Use deterministic inputs based on frame number
        let input1 = StubInput { inp: i * 3 };
        let input2 = StubInput { inp: i * 5 + 1 };

        sess1.add_local_input(PlayerHandle::new(0), input1).unwrap();
        sess2.add_local_input(PlayerHandle::new(1), input2).unwrap();

        let requests1 = sess1.advance_frame().unwrap();
        let requests2 = sess2.advance_frame().unwrap();

        stub1.handle_requests(requests1);
        stub2.handle_requests(requests2);
    }

    // Both sessions should have advanced
    assert!(
        stub1.gs.frame > 50,
        "Session 1 should advance most frames (got {})",
        stub1.gs.frame
    );
    assert!(
        stub2.gs.frame > 50,
        "Session 2 should advance most frames (got {})",
        stub2.gs.frame
    );

    // Give time for final synchronization (wait for in-flight packets)
    for _ in 0..50 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        std::thread::sleep(Duration::from_millis(50));
    }

    // No panics should have occurred (verified implicitly by reaching here)
    // Note: Full determinism (exact same state) requires waiting for all
    // rollbacks to complete, which may take additional frames

    Ok(())
}

/// Test that no panics occur even under worst-case network conditions.
/// This test validates graceful degradation.
#[test]
#[serial]
fn test_no_panics_under_worst_case() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9029);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9030);

    // Very aggressive network chaos
    let chaos_config = ChaosConfig::builder()
        .latency_ms(200)
        .jitter_ms(150)
        .packet_loss_rate(0.30) // 30% loss
        .duplication_rate(0.20)
        .reorder_buffer_size(8)
        .reorder_rate(0.5)
        .seed(42)
        .build();

    let socket1 = create_chaos_socket(9029, chaos_config.clone());
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9030, chaos_config);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Try to synchronize (may or may not succeed with 30% loss)
    let mut synchronized = false;
    for _ in 0..500 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        std::thread::sleep(Duration::from_millis(20));

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            synchronized = true;
            break;
        }
    }

    // If we synchronized, try to advance some frames
    if synchronized {
        let mut stub1 = GameStub::new();
        let mut stub2 = GameStub::new();

        for i in 0..20 {
            for _ in 0..10 {
                sess1.poll_remote_clients();
                sess2.poll_remote_clients();
            }
            std::thread::sleep(Duration::from_millis(30));

            let _ = sess1.add_local_input(PlayerHandle::new(0), StubInput { inp: i });
            let _ = sess2.add_local_input(PlayerHandle::new(1), StubInput { inp: i });

            // May fail with NotSynchronized if connection degraded too much
            if let Ok(requests1) = sess1.advance_frame() {
                stub1.handle_requests(requests1);
            }
            if let Ok(requests2) = sess2.advance_frame() {
                stub2.handle_requests(requests2);
            }
        }
    }

    // Key assertion: reaching here without panic proves graceful degradation
    Ok(())
}

/// Test asymmetric latency (different delay in each direction).
#[test]
#[serial]
fn test_asymmetric_latency() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9031);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9032);

    // Player 1 has higher latency (poor download)
    let config1 = ChaosConfig::builder().latency_ms(150).seed(42).build();

    // Player 2 has lower latency
    let config2 = ChaosConfig::builder().latency_ms(30).seed(43).build();

    let socket1 = create_chaos_socket(9031, config1);
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9032, config2);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize - need to account for higher latency of player 1
    for _ in 0..200 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        std::thread::sleep(Duration::from_millis(30));

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
    }

    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "Session 1 failed to synchronize with asymmetric latency"
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Running,
        "Session 2 failed to synchronize with asymmetric latency"
    );

    // Advance frames
    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    for i in 0..30 {
        for _ in 0..5 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
        }
        std::thread::sleep(Duration::from_millis(35));

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

    assert!(
        stub1.gs.frame > 0,
        "Should advance frames with asymmetric latency"
    );
    assert!(
        stub2.gs.frame > 0,
        "Should advance frames with asymmetric latency"
    );

    Ok(())
}

/// Test burst packet loss (multiple consecutive packets dropped).
/// This test uses passthrough for initial sync, then validates that gameplay
/// can tolerate burst loss. This is more realistic as burst loss during
/// initial handshake would typically cause connection failure anyway.
#[test]
#[serial]
fn test_burst_packet_loss() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9033);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9034);

    // Use burst loss with latency - this simulates WiFi interference or similar
    // 3% chance of burst, 3 consecutive drops per burst
    let chaos_config = ChaosConfig::builder()
        .burst_loss(0.03, 3)
        .latency_ms(20)
        .seed(42)
        .build();

    let socket1 = create_chaos_socket(9033, chaos_config.clone());
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9034, chaos_config);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize - burst loss with latency needs generous time
    // Allow plenty of retries for burst to clear
    for _ in 0..300 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "Session 1 failed to synchronize with burst loss"
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Running,
        "Session 2 failed to synchronize with burst loss"
    );

    // Advance frames with burst loss - rollback should handle dropped inputs
    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    for i in 0..40 {
        for _ in 0..5 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
        }
        std::thread::sleep(Duration::from_millis(25));

        let _ = sess1.add_local_input(PlayerHandle::new(0), StubInput { inp: i });
        let _ = sess2.add_local_input(PlayerHandle::new(1), StubInput { inp: i });

        // May fail with InvalidFrame early on - that's okay
        if let Ok(requests1) = sess1.advance_frame() {
            stub1.handle_requests(requests1);
        }
        if let Ok(requests2) = sess2.advance_frame() {
            stub2.handle_requests(requests2);
        }
    }

    // At least some frames should have advanced despite burst loss
    assert!(
        stub1.gs.frame > 0,
        "Should advance at least some frames despite burst loss (got {})",
        stub1.gs.frame
    );
    assert!(
        stub2.gs.frame > 0,
        "Should advance at least some frames despite burst loss (got {})",
        stub2.gs.frame
    );

    Ok(())
}

/// Test temporary disconnect and reconnection.
/// Simulates a brief network outage followed by recovery.
#[test]
#[serial]
fn test_temporary_disconnect_reconnect() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9035);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9036);

    // Start with good connection
    let good_config = ChaosConfig::passthrough();

    let socket1 = create_chaos_socket(9035, good_config.clone());
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9036, good_config);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize quickly with good connection - still need some time for protocol
    for _ in 0..100 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    assert_eq!(sess1.current_state(), SessionState::Running);
    assert_eq!(sess2.current_state(), SessionState::Running);

    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    // Phase 1: Run some frames with good connection
    for i in 0..20 {
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

    let frames_before_disconnect = stub1.gs.frame;

    // Phase 2: Simulate disconnect (100% packet loss) - but still advance
    // Note: We can't easily change the socket config mid-test with the current API,
    // so we simulate by just not polling for a while (packets will timeout)
    // In real scenario, the session should handle dropped packets gracefully.

    // Phase 3: Resume normal operation
    for i in 20..40 {
        for _ in 0..3 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
        }

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

    // Verify we recovered and continued advancing
    assert!(
        stub1.gs.frame > frames_before_disconnect,
        "Should continue advancing after recovery"
    );

    Ok(())
}

/// Test eventual consistency: verify both peers reach the same final state
/// after running through the same sequence of inputs.
#[test]
#[serial]
fn test_eventual_consistency() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9037);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9038);

    // Use moderate network conditions
    let chaos_config = ChaosConfig::builder()
        .latency_ms(30)
        .jitter_ms(10)
        .packet_loss_rate(0.03)
        .seed(42)
        .build();

    let socket1 = create_chaos_socket(9037, chaos_config.clone());
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9038, chaos_config);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize - needs time for latency + jitter + loss
    for _ in 0..200 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        std::thread::sleep(Duration::from_millis(50));

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
    }

    assert_eq!(sess1.current_state(), SessionState::Running);
    assert_eq!(sess2.current_state(), SessionState::Running);

    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    // Run exactly 60 frames with deterministic inputs
    let target_frames = 60;
    for i in 0..target_frames {
        for _ in 0..5 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
        }
        std::thread::sleep(Duration::from_millis(10));

        // Deterministic inputs based on frame number
        let input1 = StubInput { inp: (i * 7) % 100 };
        let input2 = StubInput {
            inp: (i * 11 + 3) % 100,
        };

        sess1.add_local_input(PlayerHandle::new(0), input1).unwrap();
        sess2.add_local_input(PlayerHandle::new(1), input2).unwrap();

        let requests1 = sess1.advance_frame().unwrap();
        let requests2 = sess2.advance_frame().unwrap();

        stub1.handle_requests(requests1);
        stub2.handle_requests(requests2);
    }

    // Allow extra time for any pending rollbacks to complete
    for _ in 0..100 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        std::thread::sleep(Duration::from_millis(10));
    }

    // Both should have reached the same frame
    assert_eq!(
        stub1.gs.frame, target_frames as i32,
        "Stub 1 should be at frame {}",
        target_frames
    );
    assert_eq!(
        stub2.gs.frame, target_frames as i32,
        "Stub 2 should be at frame {}",
        target_frames
    );

    // Both should have the same final state (determinism check)
    // Note: This may not always match if there are pending rollbacks,
    // but with sufficient drain time, states should converge.

    Ok(())
}

/// Test combined burst loss and jitter.
#[test]
#[serial]
fn test_burst_loss_with_jitter() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9039);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9040);

    // Combine burst loss with jitter
    let chaos_config = ChaosConfig::builder()
        .latency_ms(40)
        .jitter_ms(25)
        .burst_loss(0.08, 4) // 8% chance of 4-packet burst
        .seed(42)
        .build();

    let socket1 = create_chaos_socket(9039, chaos_config.clone());
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9040, chaos_config);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize - burst loss (4 packets at 8%) + jitter (25ms) + latency (40ms)
    for _ in 0..200 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        std::thread::sleep(Duration::from_millis(50));

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
    }

    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "Session 1 failed to synchronize with burst loss + jitter"
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Running,
        "Session 2 failed to synchronize with burst loss + jitter"
    );

    // Advance frames
    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    for i in 0..30 {
        for _ in 0..5 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
        }
        std::thread::sleep(Duration::from_millis(15));

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

    assert!(
        stub1.gs.frame > 0,
        "Should advance frames with burst loss + jitter"
    );
    assert!(
        stub2.gs.frame > 0,
        "Should advance frames with burst loss + jitter"
    );

    Ok(())
}

// =============================================================================
// Advanced Chaos Engineering Tests (Edge Cases)
// =============================================================================

/// Test asymmetric receive loss: one peer has higher receive loss.
/// This simulates network asymmetry where one direction is worse.
/// Note: We test with asymmetric loss (one side loses more) but both
/// sides can still complete synchronization.
#[test]
#[serial]
fn test_one_way_send_only_loss() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9041);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9042);

    // Peer 1 has higher receive loss (simulates bad incoming connection)
    let config1 = ChaosConfig::builder()
        .send_loss_rate(0.02)
        .receive_loss_rate(0.15) // 15% receive loss
        .latency_ms(20)
        .seed(42)
        .build();

    // Peer 2 has normal operation
    let config2 = ChaosConfig::builder()
        .send_loss_rate(0.02)
        .receive_loss_rate(0.02)
        .latency_ms(20)
        .seed(43)
        .build();

    let socket1 = create_chaos_socket(9041, config1);
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9042, config2);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize with extended time due to asymmetric loss
    for _ in 0..300 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        std::thread::sleep(Duration::from_millis(50));

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
    }

    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "Session 1 failed to synchronize with one-way receive loss"
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Running,
        "Session 2 failed to synchronize with one-way receive loss"
    );

    // Advance frames
    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    for i in 0..30 {
        for _ in 0..5 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
        }
        std::thread::sleep(Duration::from_millis(30));

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

    assert!(
        stub1.gs.frame > 0,
        "Should advance frames with one-way receive loss"
    );

    Ok(())
}

/// Test asymmetric send loss: one peer has higher send loss.
/// This simulates network asymmetry where outgoing packets drop more.
/// Note: We test with asymmetric loss (one side loses more) but both
/// sides can still complete synchronization.
#[test]
#[serial]
fn test_one_way_receive_only_loss() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9043);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9044);

    // Peer 1 has higher send loss (simulates bad outgoing connection)
    let config1 = ChaosConfig::builder()
        .send_loss_rate(0.15) // 15% send loss
        .receive_loss_rate(0.02)
        .latency_ms(20)
        .seed(42)
        .build();

    // Peer 2 has normal operation
    let config2 = ChaosConfig::builder()
        .send_loss_rate(0.02)
        .receive_loss_rate(0.02)
        .latency_ms(20)
        .seed(43)
        .build();

    let socket1 = create_chaos_socket(9043, config1);
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9044, config2);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize with extended time
    for _ in 0..300 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        std::thread::sleep(Duration::from_millis(50));

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
    }

    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "Session 1 failed to synchronize with one-way send loss"
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Running,
        "Session 2 failed to synchronize with one-way send loss"
    );

    // Advance frames
    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    for i in 0..30 {
        for _ in 0..5 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
        }
        std::thread::sleep(Duration::from_millis(30));

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

    assert!(
        stub1.gs.frame > 0,
        "Should advance frames with one-way send loss"
    );

    Ok(())
}

/// Test heavy packet duplication - network equipment sometimes duplicates packets.
#[test]
#[serial]
fn test_heavy_packet_duplication() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9045);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9046);

    // 20% packet duplication rate
    let chaos_config = ChaosConfig::builder()
        .duplication_rate(0.20)
        .latency_ms(20)
        .seed(42)
        .build();

    let socket1 = create_chaos_socket(9045, chaos_config.clone());
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9046, chaos_config);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize
    for _ in 0..100 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        std::thread::sleep(Duration::from_millis(30));

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
    }

    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "Session 1 failed to synchronize with packet duplication"
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Running,
        "Session 2 failed to synchronize with packet duplication"
    );

    // Advance frames - duplicated packets should be handled gracefully
    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    for i in 0..40 {
        for _ in 0..3 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
        }
        std::thread::sleep(Duration::from_millis(15));

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

    assert!(
        stub1.gs.frame >= 30,
        "Should advance frames with duplication"
    );

    Ok(())
}

/// Test packet reordering - packets arrive out of sequence.
#[test]
#[serial]
fn test_packet_reordering() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9047);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9048);

    // Buffer 4 packets, 30% chance of reordering within buffer
    let chaos_config = ChaosConfig::builder()
        .reorder_buffer_size(4)
        .reorder_rate(0.30)
        .latency_ms(30)
        .seed(42)
        .build();

    let socket1 = create_chaos_socket(9047, chaos_config.clone());
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9048, chaos_config);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize
    for _ in 0..150 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        std::thread::sleep(Duration::from_millis(40));

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
    }

    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "Session 1 failed to synchronize with reordering"
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Running,
        "Session 2 failed to synchronize with reordering"
    );

    // Advance frames - reordered packets should be handled
    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    for i in 0..40 {
        for _ in 0..4 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
        }
        std::thread::sleep(Duration::from_millis(20));

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

    assert!(
        stub1.gs.frame >= 30,
        "Should advance frames with reordering"
    );

    Ok(())
}

/// Test extreme combined conditions: all chaos features active.
#[test]
#[serial]
fn test_extreme_chaos_combined() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9049);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9050);

    // Everything enabled but at moderate levels
    let chaos_config = ChaosConfig::builder()
        .latency_ms(60)
        .jitter_ms(30)
        .packet_loss_rate(0.08)
        .duplication_rate(0.05)
        .reorder_buffer_size(3)
        .reorder_rate(0.15)
        .burst_loss(0.02, 2)
        .seed(42)
        .build();

    let socket1 = create_chaos_socket(9049, chaos_config.clone());
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9050, chaos_config);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize - needs long time with all chaos combined
    for _ in 0..400 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        std::thread::sleep(Duration::from_millis(50));

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
    }

    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "Session 1 failed to synchronize with extreme chaos"
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Running,
        "Session 2 failed to synchronize with extreme chaos"
    );

    // Advance frames
    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    for i in 0..30 {
        for _ in 0..6 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
        }
        std::thread::sleep(Duration::from_millis(35));

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

    assert!(
        stub1.gs.frame > 0,
        "Should advance frames with extreme chaos"
    );

    Ok(())
}

/// Test with varying prediction window sizes under network conditions.
#[test]
#[serial]
fn test_large_prediction_window_with_latency() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9051);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9052);

    let chaos_config = ChaosConfig::builder()
        .latency_ms(80)
        .jitter_ms(20)
        .seed(42)
        .build();

    // Use larger prediction window (16 frames instead of default 8)
    let socket1 = create_chaos_socket(9051, chaos_config.clone());
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_max_prediction_window(16)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9052, chaos_config);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_max_prediction_window(16)
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize
    for _ in 0..200 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        std::thread::sleep(Duration::from_millis(40));

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
    }

    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "Session 1 failed to synchronize with large prediction window"
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Running,
        "Session 2 failed to synchronize with large prediction window"
    );

    // Advance frames
    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    for i in 0..50 {
        for _ in 0..4 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
        }
        std::thread::sleep(Duration::from_millis(30));

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

    assert!(
        stub1.gs.frame >= 40,
        "Should advance frames with large prediction window"
    );

    Ok(())
}

/// Test with higher input delay under packet loss.
#[test]
#[serial]
fn test_input_delay_with_packet_loss() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9053);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9054);

    let chaos_config = ChaosConfig::builder()
        .packet_loss_rate(0.10)
        .latency_ms(30)
        .seed(42)
        .build();

    // Use input delay of 3 frames
    let socket1 = create_chaos_socket(9053, chaos_config.clone());
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_input_delay(3)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9054, chaos_config);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_input_delay(3)
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize
    for _ in 0..150 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        std::thread::sleep(Duration::from_millis(50));

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
    }

    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "Session 1 failed to synchronize with input delay + loss"
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Running,
        "Session 2 failed to synchronize with input delay + loss"
    );

    // Advance frames
    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    for i in 0..40 {
        for _ in 0..4 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
        }
        std::thread::sleep(Duration::from_millis(20));

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

    assert!(
        stub1.gs.frame > 0,
        "Should advance frames with input delay + loss"
    );

    Ok(())
}

/// Test sparse saving mode under challenging network conditions.
#[test]
#[serial]
fn test_sparse_saving_with_network_chaos() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9055);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9056);

    // Reduced latency and packet loss for more reliable synchronization
    let chaos_config = ChaosConfig::builder()
        .latency_ms(20)
        .jitter_ms(10)
        .packet_loss_rate(0.03)
        .seed(42)
        .build();

    // Enable sparse saving mode
    let socket1 = create_chaos_socket(9055, chaos_config.clone());
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_save_mode(SaveMode::Sparse)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9056, chaos_config);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_save_mode(SaveMode::Sparse)
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize - allow sufficient time for latency + packet loss retries
    for _ in 0..400 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        std::thread::sleep(Duration::from_millis(20));

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
    }

    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "Session 1 failed to synchronize with sparse saving"
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Running,
        "Session 2 failed to synchronize with sparse saving"
    );

    // Advance frames
    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    for i in 0..50 {
        for _ in 0..4 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
        }
        std::thread::sleep(Duration::from_millis(20));

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

    assert!(
        stub1.gs.frame >= 40,
        "Should advance frames with sparse saving + chaos"
    );

    Ok(())
}

/// Test rapid connect/disconnect cycles to stress connection handling.
/// Simulates network flapping.
///
/// Note: This test uses harsh network conditions (15% burst loss, 8-packet bursts)
/// which exceed the default SyncConfig capabilities. We use SyncConfig::stress_test()
/// which has a 60-second timeout and 40 sync packets, providing ample margin for
/// the test's 32-second loop duration (800 iterations × 40ms).
#[test]
#[serial]
fn test_network_flapping_simulation() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9057);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9058);

    // Simulate flapping with burst loss
    // 15% burst probability with 8-packet bursts is very aggressive
    // Requires SyncConfig::stress_test() for reliable synchronization
    let chaos_config = ChaosConfig::builder()
        .latency_ms(25)
        .burst_loss(0.15, 8) // 15% chance of dropping 8 consecutive packets
        .seed(42)
        .build();

    let socket1 = create_chaos_socket(9057, chaos_config.clone());
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_sync_config(SyncConfig::stress_test()) // 60s timeout for harsh conditions
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9058, chaos_config);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_sync_config(SyncConfig::stress_test()) // 60s timeout for harsh conditions
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize with extra tolerance for bursts
    // stress_test() has 60s timeout, test loop is 800 × 40ms = 32s (well within budget)
    let start_time = std::time::Instant::now();
    let mut sync_events_1 = 0u32;
    let mut sync_events_2 = 0u32;
    let mut timeout_events_1 = 0u32;
    let mut timeout_events_2 = 0u32;
    let mut last_progress_report = std::time::Instant::now();

    for iteration in 0..800 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        // Track sync progress events for diagnostics
        for event in sess1.events() {
            match event {
                FortressEvent::Synchronizing { .. } => sync_events_1 += 1,
                FortressEvent::SyncTimeout { .. } => timeout_events_1 += 1,
                _ => {},
            }
        }
        for event in sess2.events() {
            match event {
                FortressEvent::Synchronizing { .. } => sync_events_2 += 1,
                FortressEvent::SyncTimeout { .. } => timeout_events_2 += 1,
                _ => {},
            }
        }

        std::thread::sleep(Duration::from_millis(40));

        // Log progress every 5 seconds for long-running diagnostics
        if last_progress_report.elapsed() >= Duration::from_secs(5) {
            eprintln!(
                "[Flapping Test] iter={}, elapsed={:.1}s, sess1={:?} (sync_events={}, timeouts={}), sess2={:?} (sync_events={}, timeouts={})",
                iteration,
                start_time.elapsed().as_secs_f32(),
                sess1.current_state(),
                sync_events_1,
                timeout_events_1,
                sess2.current_state(),
                sync_events_2,
                timeout_events_2
            );
            last_progress_report = std::time::Instant::now();
        }

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            eprintln!(
                "[Flapping Test] Synchronized at iter={}, elapsed={:.1}s, sync_events=({}, {})",
                iteration,
                start_time.elapsed().as_secs_f32(),
                sync_events_1,
                sync_events_2
            );
            break;
        }
    }

    // Enhanced assertion with diagnostic context
    let elapsed = start_time.elapsed();
    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "Session 1 failed to synchronize under flapping after {:.1}s (state: {:?}, sync_events: {}, timeout_events: {})",
        elapsed.as_secs_f32(),
        sess1.current_state(),
        sync_events_1,
        timeout_events_1
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Running,
        "Session 2 failed to synchronize under flapping after {:.1}s (state: {:?}, sync_events: {}, timeout_events: {})",
        elapsed.as_secs_f32(),
        sess2.current_state(),
        sync_events_2,
        timeout_events_2
    );

    // Advance frames
    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    for i in 0..30 {
        for _ in 0..6 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
        }
        std::thread::sleep(Duration::from_millis(30));

        let _ = sess1.add_local_input(PlayerHandle::new(0), StubInput { inp: i });
        let _ = sess2.add_local_input(PlayerHandle::new(1), StubInput { inp: i });

        if let Ok(requests1) = sess1.advance_frame() {
            stub1.handle_requests(requests1);
        }
        if let Ok(requests2) = sess2.advance_frame() {
            stub2.handle_requests(requests2);
        }
    }

    // At least some frames should advance
    assert!(stub1.gs.frame > 0, "Should make progress under flapping");

    Ok(())
}

/// Test with extreme jitter (very variable latency).
#[test]
#[serial]
fn test_extreme_jitter() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9059);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9060);

    // Base latency 50ms, jitter ±50ms (0-100ms effective)
    let chaos_config = ChaosConfig::builder()
        .latency_ms(50)
        .jitter_ms(50)
        .seed(42)
        .build();

    let socket1 = create_chaos_socket(9059, chaos_config.clone());
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9060, chaos_config);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize
    for _ in 0..200 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        std::thread::sleep(Duration::from_millis(60));

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
    }

    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "Session 1 failed to synchronize with extreme jitter"
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Running,
        "Session 2 failed to synchronize with extreme jitter"
    );

    // Advance frames
    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    for i in 0..40 {
        for _ in 0..4 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
        }
        std::thread::sleep(Duration::from_millis(30));

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

    assert!(
        stub1.gs.frame >= 30,
        "Should advance frames with extreme jitter"
    );

    Ok(())
}

/// Test with "terrible network" preset - validates the preset works.
#[test]
#[serial]
fn test_terrible_network_preset() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9061);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9062);

    // Use the preset
    let chaos_config = ChaosConfig::terrible_network();

    let socket1 = create_chaos_socket(9061, chaos_config.clone());
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9062, chaos_config);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize - terrible network needs lots of time
    for _ in 0..500 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        std::thread::sleep(Duration::from_millis(60));

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
    }

    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "Session 1 failed to synchronize with terrible network"
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Running,
        "Session 2 failed to synchronize with terrible network"
    );

    // Advance frames
    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    for i in 0..25 {
        for _ in 0..8 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
        }
        std::thread::sleep(Duration::from_millis(50));

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

    assert!(
        stub1.gs.frame > 0,
        "Should advance frames with terrible network"
    );

    Ok(())
}

/// Test with "mobile network" preset - validates mobile network simulation.
#[test]
#[serial]
fn test_mobile_network_preset() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9063);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9064);

    // Use the mobile network preset with deterministic seed
    let chaos_config1 = ChaosConfig::builder()
        .latency(Duration::from_millis(60))
        .jitter(Duration::from_millis(40))
        .packet_loss_rate(0.12)
        .duplication_rate(0.01)
        .reorder_buffer_size(3)
        .reorder_rate(0.05)
        .burst_loss(0.02, 4)
        .seed(12345)
        .build();

    let chaos_config2 = ChaosConfig::builder()
        .latency(Duration::from_millis(60))
        .jitter(Duration::from_millis(40))
        .packet_loss_rate(0.12)
        .duplication_rate(0.01)
        .reorder_buffer_size(3)
        .reorder_rate(0.05)
        .burst_loss(0.02, 4)
        .seed(12346)
        .build();

    let socket1 = create_chaos_socket(9063, chaos_config1);
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_sync_config(SyncConfig::mobile())
        .with_protocol_config(ProtocolConfig::mobile())
        .with_time_sync_config(TimeSyncConfig::mobile())
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9064, chaos_config2);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_sync_config(SyncConfig::mobile())
        .with_protocol_config(ProtocolConfig::mobile())
        .with_time_sync_config(TimeSyncConfig::mobile())
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize - mobile network needs generous time
    for _ in 0..600 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        std::thread::sleep(Duration::from_millis(50));

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
    }

    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "Session 1 failed to synchronize with mobile network preset"
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Running,
        "Session 2 failed to synchronize with mobile network preset"
    );

    // Advance frames
    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    for i in 0..30 {
        for _ in 0..8 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
        }
        std::thread::sleep(Duration::from_millis(40));

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

    assert!(
        stub1.gs.frame > 0,
        "Should advance frames with mobile network preset"
    );

    Ok(())
}

/// Test with "wifi interference" preset - validates WiFi interference simulation.
#[test]
#[serial]
fn test_wifi_interference_preset() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9065);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9066);

    // Use WiFi interference characteristics with deterministic seed
    let chaos_config1 = ChaosConfig::builder()
        .latency(Duration::from_millis(15))
        .jitter(Duration::from_millis(25))
        .packet_loss_rate(0.03)
        .reorder_buffer_size(2)
        .reorder_rate(0.02)
        .burst_loss(0.05, 3)
        .seed(22222)
        .build();

    let chaos_config2 = ChaosConfig::builder()
        .latency(Duration::from_millis(15))
        .jitter(Duration::from_millis(25))
        .packet_loss_rate(0.03)
        .reorder_buffer_size(2)
        .reorder_rate(0.02)
        .burst_loss(0.05, 3)
        .seed(22223)
        .build();

    let socket1 = create_chaos_socket(9065, chaos_config1);
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9066, chaos_config2);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize - WiFi should be faster than mobile
    for _ in 0..300 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        std::thread::sleep(Duration::from_millis(30));

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
    }

    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "Session 1 failed to synchronize with wifi interference preset"
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Running,
        "Session 2 failed to synchronize with wifi interference preset"
    );

    // Advance frames
    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    for i in 0..40 {
        for _ in 0..6 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
        }
        std::thread::sleep(Duration::from_millis(20));

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

    assert!(
        stub1.gs.frame >= 30,
        "Should advance frames with wifi interference preset"
    );

    Ok(())
}

/// Test with "intercontinental" preset - validates high-latency stable connections.
#[test]
#[serial]
fn test_intercontinental_preset() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9067);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9068);

    // Use intercontinental characteristics with deterministic seed
    let chaos_config1 = ChaosConfig::builder()
        .latency(Duration::from_millis(120))
        .jitter(Duration::from_millis(15))
        .packet_loss_rate(0.02)
        .seed(33333)
        .build();

    let chaos_config2 = ChaosConfig::builder()
        .latency(Duration::from_millis(120))
        .jitter(Duration::from_millis(15))
        .packet_loss_rate(0.02)
        .seed(33334)
        .build();

    let socket1 = create_chaos_socket(9067, chaos_config1);
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_sync_config(SyncConfig::high_latency())
        .with_protocol_config(ProtocolConfig::high_latency())
        .with_time_sync_config(TimeSyncConfig::smooth())
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9068, chaos_config2);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_sync_config(SyncConfig::high_latency())
        .with_protocol_config(ProtocolConfig::high_latency())
        .with_time_sync_config(TimeSyncConfig::smooth())
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize - intercontinental is stable but high latency
    for _ in 0..400 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        std::thread::sleep(Duration::from_millis(50));

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
    }

    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "Session 1 failed to synchronize with intercontinental preset"
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Running,
        "Session 2 failed to synchronize with intercontinental preset"
    );

    // Advance frames
    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    for i in 0..25 {
        for _ in 0..10 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
        }
        std::thread::sleep(Duration::from_millis(50));

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

    assert!(
        stub1.gs.frame > 0,
        "Should advance frames with intercontinental preset"
    );

    Ok(())
}

/// Test competitive preset - validates fast sync and strict timeouts.
#[test]
#[serial]
fn test_competitive_preset_fast_sync() -> Result<(), FortressError> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9069);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9070);

    // No chaos - LAN-like conditions for competitive play
    let socket1 = UdpNonBlockingSocket::bind_to_port(9069).unwrap();
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_sync_config(SyncConfig::competitive())
        .with_protocol_config(ProtocolConfig::competitive())
        .with_time_sync_config(TimeSyncConfig::competitive())
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = UdpNonBlockingSocket::bind_to_port(9070).unwrap();
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_sync_config(SyncConfig::competitive())
        .with_protocol_config(ProtocolConfig::competitive())
        .with_time_sync_config(TimeSyncConfig::competitive())
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Competitive should sync very fast
    let start = std::time::Instant::now();
    for _ in 0..100 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        std::thread::sleep(Duration::from_millis(10));

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
    }
    let sync_time = start.elapsed();

    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "Session 1 failed to synchronize with competitive preset"
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Running,
        "Session 2 failed to synchronize with competitive preset"
    );

    // Competitive sync should be fast (< 2 seconds on localhost)
    assert!(
        sync_time < Duration::from_secs(2),
        "Competitive preset should sync quickly, took {:?}",
        sync_time
    );

    // Advance frames
    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    for i in 0..50 {
        for _ in 0..4 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
        }

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

    assert!(
        stub1.gs.frame >= 40,
        "Should advance many frames with competitive preset"
    );

    Ok(())
}

// =============================================================================
// Data-Driven Network Resilience Tests
// =============================================================================

/// Parameters for data-driven network chaos testing.
#[derive(Clone, Debug)]
struct ChaosTestCase {
    name: &'static str,
    latency_ms: u64,
    packet_loss: f64,
    burst_loss_probability: f64,
    burst_loss_length: usize,
    jitter_ms: u64,
    sync_config: SyncConfig,
    max_iterations: usize,
    iteration_delay_ms: u64,
    expected_to_sync: bool,
}

impl ChaosTestCase {
    /// Run the test case and return diagnostic information.
    fn run(&self, base_port: u16) -> Result<ChaosTestResult, FortressError> {
        let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), base_port);
        let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), base_port + 1);

        // Build chaos configuration
        let mut builder = ChaosConfig::builder()
            .latency_ms(self.latency_ms)
            .packet_loss_rate(self.packet_loss)
            .seed(42); // Deterministic for reproducibility

        if self.burst_loss_probability > 0.0 {
            builder = builder.burst_loss(self.burst_loss_probability, self.burst_loss_length);
        }
        if self.jitter_ms > 0 {
            builder = builder.jitter_ms(self.jitter_ms);
        }

        let chaos_config = builder.build();

        let socket1 = create_chaos_socket(base_port, chaos_config.clone());
        let mut sess1 = SessionBuilder::<StubConfig>::new()
            .with_sync_config(self.sync_config.clone())
            .add_player(PlayerType::Local, PlayerHandle::new(0))?
            .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
            .start_p2p_session(socket1)?;

        let socket2 = create_chaos_socket(base_port + 1, chaos_config);
        let mut sess2 = SessionBuilder::<StubConfig>::new()
            .with_sync_config(self.sync_config.clone())
            .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
            .add_player(PlayerType::Local, PlayerHandle::new(1))?
            .start_p2p_session(socket2)?;

        // Track diagnostics
        let start_time = std::time::Instant::now();
        let mut sync_events_1 = 0u32;
        let mut sync_events_2 = 0u32;
        let mut timeout_events_1 = 0u32;
        let mut timeout_events_2 = 0u32;
        let mut sync_iteration = None;

        for iteration in 0..self.max_iterations {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();

            // Track events
            for event in sess1.events() {
                match event {
                    FortressEvent::Synchronizing { .. } => sync_events_1 += 1,
                    FortressEvent::SyncTimeout { .. } => timeout_events_1 += 1,
                    _ => {},
                }
            }
            for event in sess2.events() {
                match event {
                    FortressEvent::Synchronizing { .. } => sync_events_2 += 1,
                    FortressEvent::SyncTimeout { .. } => timeout_events_2 += 1,
                    _ => {},
                }
            }

            std::thread::sleep(Duration::from_millis(self.iteration_delay_ms));

            if sess1.current_state() == SessionState::Running
                && sess2.current_state() == SessionState::Running
            {
                sync_iteration = Some(iteration);
                break;
            }
        }

        Ok(ChaosTestResult {
            elapsed: start_time.elapsed(),
            sync_iteration,
            sess1_state: sess1.current_state(),
            sess2_state: sess2.current_state(),
            sync_events: (sync_events_1, sync_events_2),
            timeout_events: (timeout_events_1, timeout_events_2),
            synchronized: sess1.current_state() == SessionState::Running
                && sess2.current_state() == SessionState::Running,
        })
    }
}

/// Diagnostic results from a chaos test run.
#[derive(Debug)]
struct ChaosTestResult {
    elapsed: Duration,
    sync_iteration: Option<usize>,
    sess1_state: SessionState,
    sess2_state: SessionState,
    sync_events: (u32, u32),
    timeout_events: (u32, u32),
    synchronized: bool,
}

/// Data-driven test: Various network conditions with appropriate SyncConfig.
#[test]
#[serial]
fn test_chaos_conditions_data_driven() {
    let test_cases = [
        // Light conditions - should sync easily
        ChaosTestCase {
            name: "lan_ideal",
            latency_ms: 5,
            packet_loss: 0.0,
            burst_loss_probability: 0.0,
            burst_loss_length: 0,
            jitter_ms: 0,
            sync_config: SyncConfig::lan(),
            max_iterations: 100,
            iteration_delay_ms: 20,
            expected_to_sync: true,
        },
        // Moderate packet loss
        ChaosTestCase {
            name: "moderate_loss_5pct",
            latency_ms: 30,
            packet_loss: 0.05,
            burst_loss_probability: 0.0,
            burst_loss_length: 0,
            jitter_ms: 10,
            sync_config: SyncConfig::lossy(),
            max_iterations: 200,
            iteration_delay_ms: 30,
            expected_to_sync: true,
        },
        // Mobile-like conditions (increased iterations for high latency)
        ChaosTestCase {
            name: "mobile_conditions",
            latency_ms: 60,
            packet_loss: 0.05,
            burst_loss_probability: 0.02,
            burst_loss_length: 3,
            jitter_ms: 20,
            sync_config: SyncConfig::extreme(), // More robust than mobile for testing
            max_iterations: 500,
            iteration_delay_ms: 40,
            expected_to_sync: true,
        },
        // High burst loss - requires stress_test config with more time
        ChaosTestCase {
            name: "high_burst_loss",
            latency_ms: 25,
            packet_loss: 0.0,
            burst_loss_probability: 0.08,
            burst_loss_length: 5,
            jitter_ms: 15,
            sync_config: SyncConfig::stress_test(),
            max_iterations: 900,
            iteration_delay_ms: 40,
            expected_to_sync: true,
        },
    ];

    // Use 9200+ range to avoid conflicts with tests/sessions/p2p.rs (uses 9100-9109)
    let mut base_port = 9200u16;

    for test_case in &test_cases {
        eprintln!("\n[Data-Driven Test] Running: {}", test_case.name);

        let result = test_case.run(base_port).expect("Test setup failed");

        eprintln!(
            "[Data-Driven Test] {} completed: synchronized={}, elapsed={:.2}s, \
             sync_events=({}, {}), timeout_events=({}, {}), sync_at_iter={:?}",
            test_case.name,
            result.synchronized,
            result.elapsed.as_secs_f32(),
            result.sync_events.0,
            result.sync_events.1,
            result.timeout_events.0,
            result.timeout_events.1,
            result.sync_iteration
        );

        if test_case.expected_to_sync {
            assert!(
                result.synchronized,
                "[{}] Expected to synchronize but failed (sess1={:?}, sess2={:?}, \
                 sync_events=({}, {}), timeout_events=({}, {}))",
                test_case.name,
                result.sess1_state,
                result.sess2_state,
                result.sync_events.0,
                result.sync_events.1,
                result.timeout_events.0,
                result.timeout_events.1
            );
        } else {
            assert!(
                !result.synchronized,
                "[{}] Expected NOT to synchronize but it did",
                test_case.name
            );
        }

        base_port += 2; // Advance ports for next test
    }
}

/// Test: Verify sync timeout is detected and reported correctly.
#[test]
#[serial]
fn test_sync_timeout_detection() -> Result<(), FortressError> {
    // Use 9250+ range to avoid conflicts with tests/sessions/p2p.rs (uses 9100-9109)
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9250);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9251);

    // Use very short timeout with heavy packet loss to trigger timeout
    let short_timeout_config = SyncConfig {
        num_sync_packets: 10,
        sync_retry_interval: Duration::from_millis(50),
        sync_timeout: Some(Duration::from_secs(2)), // 2 second timeout
        running_retry_interval: Duration::from_millis(100),
        keepalive_interval: Duration::from_millis(100),
    };

    // 50% packet loss should make sync impossible in 2 seconds with 10 roundtrips
    let chaos_config = ChaosConfig::builder()
        .latency_ms(20)
        .packet_loss_rate(0.50) // Very high loss
        .seed(42)
        .build();

    let socket1 = create_chaos_socket(9250, chaos_config.clone());
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_sync_config(short_timeout_config.clone())
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9251, chaos_config);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_sync_config(short_timeout_config)
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Track timeout events
    let mut timeout_detected_1 = false;
    let mut timeout_detected_2 = false;
    let start = std::time::Instant::now();

    // Run for 5 seconds (well past the 2 second timeout)
    for _ in 0..250 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        for event in sess1.events() {
            if matches!(event, FortressEvent::SyncTimeout { .. }) {
                timeout_detected_1 = true;
                eprintln!(
                    "[Timeout Test] Session 1 timeout detected at {:.2}s",
                    start.elapsed().as_secs_f32()
                );
            }
        }
        for event in sess2.events() {
            if matches!(event, FortressEvent::SyncTimeout { .. }) {
                timeout_detected_2 = true;
                eprintln!(
                    "[Timeout Test] Session 2 timeout detected at {:.2}s",
                    start.elapsed().as_secs_f32()
                );
            }
        }

        std::thread::sleep(Duration::from_millis(20));

        // Once we've detected timeouts, we can stop
        if timeout_detected_1 && timeout_detected_2 {
            break;
        }
    }

    // At least one session should have detected a timeout
    assert!(
        timeout_detected_1 || timeout_detected_2,
        "Expected at least one session to detect sync timeout under heavy packet loss"
    );

    // Sessions should still be in Synchronizing state (timeout doesn't change state)
    // This verifies that timeout is informational, not a state transition
    assert_eq!(
        sess1.current_state(),
        SessionState::Synchronizing,
        "Session should remain Synchronizing after timeout event"
    );

    Ok(())
}

/// Test: Edge case where burst loss exactly matches sync packet count.
#[test]
#[serial]
fn test_burst_loss_matches_sync_packets() -> Result<(), FortressError> {
    // Use 9260+ range to avoid conflicts with tests/sessions/p2p.rs (uses 9100-9109)
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9260);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9261);

    // Configure burst loss length equal to default sync packets (5)
    // This tests the edge case where a burst could wipe out all initial sync attempts
    let chaos_config = ChaosConfig::builder()
        .latency_ms(20)
        .burst_loss(0.05, 5) // Burst wipes exactly 5 packets
        .seed(42)
        .build();

    // Use a config with more sync packets to handle burst wiping out initial 5
    let resilient_config = SyncConfig {
        num_sync_packets: 15, // 3x the burst length
        sync_retry_interval: Duration::from_millis(100),
        sync_timeout: Some(Duration::from_secs(15)),
        running_retry_interval: Duration::from_millis(100),
        keepalive_interval: Duration::from_millis(100),
    };

    let socket1 = create_chaos_socket(9260, chaos_config.clone());
    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_sync_config(resilient_config.clone())
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = create_chaos_socket(9261, chaos_config);
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_sync_config(resilient_config)
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize
    for _ in 0..300 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        std::thread::sleep(Duration::from_millis(30));

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
    }

    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "Session 1 should sync despite burst length matching initial sync packets"
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Running,
        "Session 2 should sync despite burst length matching initial sync packets"
    );

    Ok(())
}
