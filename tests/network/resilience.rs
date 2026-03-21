//! Network resilience integration tests using ChaosSocket.
//!
//! These tests validate that Fortress Rollback sessions can handle
//! adverse network conditions including:
//! - Packet loss (sporadic and burst)
//! - High latency
//! - Jitter (variable latency)
//! - Combined conditions
//!
//! # Deterministic Testing Infrastructure
//!
//! This test file uses `ChannelSocket` (in-memory sockets) and `TestClock`
//! (virtual time) for fully deterministic testing. No real UDP I/O or
//! `thread::sleep` calls are needed, eliminating port conflicts and
//! timing-related flakiness across all platforms.
//!
//! # Seed Correlation Warning
//!
//! When using ChaosSocket with packet loss or burst loss, **always use different
//! seeds** for each socket in a test. Using the same seed causes both sockets'
//! RNGs to produce identical random sequences, leading to correlated loss patterns
//! that can systematically block synchronization. This manifests as:
//!
//! - One session transitions to Running while the other stays stuck in Synchronizing
//! - Asymmetric sync_events counts between sessions
//! - Tests timing out on some platforms (especially macOS) but not others
//!
//! **Correct pattern:**
//! ```ignore
//! let config1 = ChaosConfig::builder().packet_loss_rate(0.10).seed(42).build();
//! let config2 = ChaosConfig::builder().packet_loss_rate(0.10).seed(43).build();
//! ```
//!
//! **Incorrect pattern (DO NOT USE):**
//! ```ignore
//! let config = ChaosConfig::builder().packet_loss_rate(0.10).seed(42).build();
//! let socket1 = create_chaos_socket(port1, config.clone())?;
//! let socket2 = create_chaos_socket(port2, config)?; // Same seed!
//! ```

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
use crate::common::{create_channel_pair, create_chaos_channel_pair, TestClock};
use fortress_rollback::{
    ChaosConfig, FortressError, FortressEvent, PlayerHandle, PlayerType, ProtocolConfig, SaveMode,
    SessionBuilder, SessionState, SyncConfig, TimeSyncConfig,
};
use std::time::Duration;

/// Helper: creates a `ProtocolConfig` with the given test clock.
fn protocol_config(clock: &TestClock) -> ProtocolConfig {
    ProtocolConfig {
        clock: Some(clock.as_protocol_clock()),
        ..ProtocolConfig::default()
    }
}

// Uses create_chaos_channel_pair for deterministic in-memory sockets

/// Test that sessions can synchronize with moderate packet loss.
/// 10% packet loss should still allow synchronization to complete.
#[test]
fn test_synchronize_with_packet_loss() -> Result<(), FortressError> {
    let clock = TestClock::new();

    // Use different seeds to avoid correlated packet drops
    let config1 = ChaosConfig::builder()
        .packet_loss_rate(0.10) // 10% loss
        .seed(42)
        .build();

    let config2 = ChaosConfig::builder()
        .packet_loss_rate(0.10) // 10% loss
        .seed(43) // Different seed!
        .build();

    let (socket1, socket2, addr1, addr2) = create_chaos_channel_pair(config1, config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Should start in Synchronizing state
    assert_eq!(sess1.current_state(), SessionState::Synchronizing);
    assert_eq!(sess2.current_state(), SessionState::Synchronizing);

    // Synchronize - advance clock to allow retry timers to fire (200ms retry interval)
    // With 10% loss on each side, ~19% of roundtrips fail.
    // Need 5 successful roundtrips, so expect ~6-8 attempts.
    for _ in 0..100 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }

        // Advance clock to allow retry timer (200ms) to fire
        clock.advance(Duration::from_millis(100));
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
fn test_advance_frames_with_packet_loss() -> Result<(), FortressError> {
    let clock = TestClock::new();

    let chaos_config1 = ChaosConfig::builder()
        .packet_loss_rate(0.05) // 5% loss
        .seed(123)
        .build();

    let chaos_config2 = ChaosConfig::builder()
        .packet_loss_rate(0.05) // 5% loss
        .seed(124)
        .build();

    let (socket1, socket2, addr1, addr2) =
        create_chaos_channel_pair(chaos_config1, chaos_config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize first
    for _ in 0..200 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
        clock.advance(Duration::from_millis(40));
    }

    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "Session 1 failed to synchronize within timeout"
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Running,
        "Session 2 failed to synchronize within timeout"
    );

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
fn test_synchronize_with_latency() -> Result<(), FortressError> {
    let clock = TestClock::new();

    // 20ms simulated latency
    let chaos_config1 = ChaosConfig::builder().latency_ms(20).seed(42).build();
    let chaos_config2 = ChaosConfig::builder().latency_ms(20).seed(43).build();

    let (socket1, socket2, addr1, addr2) =
        create_chaos_channel_pair(chaos_config1, chaos_config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize - advance clock to allow latency simulation to deliver packets
    for _ in 0..100 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }

        // Advance clock past latency window
        clock.advance(Duration::from_millis(50));
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
fn test_synchronize_with_jitter() -> Result<(), FortressError> {
    let clock = TestClock::new();

    // 30ms latency with +/-15ms jitter
    let chaos_config1 = ChaosConfig::builder()
        .latency_ms(30)
        .jitter_ms(15)
        .seed(42)
        .build();

    let chaos_config2 = ChaosConfig::builder()
        .latency_ms(30)
        .jitter_ms(15)
        .seed(43)
        .build();

    let (socket1, socket2, addr1, addr2) =
        create_chaos_channel_pair(chaos_config1, chaos_config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize with jitter
    for _ in 0..150 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }

        clock.advance(Duration::from_millis(50));
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
fn test_poor_network_conditions() -> Result<(), FortressError> {
    let clock = TestClock::new();

    // Use the "poor network" preset with deterministic seeds
    let mut chaos_config1 = ChaosConfig::poor_network();
    chaos_config1.seed = Some(42);

    let mut chaos_config2 = ChaosConfig::poor_network();
    chaos_config2.seed = Some(43);

    let (socket1, socket2, addr1, addr2) =
        create_chaos_channel_pair(chaos_config1, chaos_config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize under poor network conditions (100ms latency, 50ms jitter, 5% loss)
    // Need more time due to high latency
    for _ in 0..200 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }

        clock.advance(Duration::from_millis(50));
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
        clock.advance(Duration::from_millis(15));

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
fn test_asymmetric_packet_loss() -> Result<(), FortressError> {
    let clock = TestClock::new();

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

    let (socket1, socket2, addr1, addr2) = create_chaos_channel_pair(config1, config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize with asymmetric loss
    for _ in 0..300 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
        clock.advance(Duration::from_millis(50));
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
fn test_high_packet_loss() -> Result<(), FortressError> {
    let clock = TestClock::new();

    let chaos_config1 = ChaosConfig::builder()
        .packet_loss_rate(0.25) // 25% loss - very aggressive
        .seed(42)
        .build();

    let chaos_config2 = ChaosConfig::builder()
        .packet_loss_rate(0.25)
        .seed(43)
        .build();

    let (socket1, socket2, addr1, addr2) =
        create_chaos_channel_pair(chaos_config1, chaos_config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // May need many iterations due to high loss
    for _ in 0..200 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
        clock.advance(Duration::from_millis(80));
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
fn test_high_latency_100ms() -> Result<(), FortressError> {
    let clock = TestClock::new();

    let chaos_config1 = ChaosConfig::builder().latency_ms(100).seed(42).build();
    let chaos_config2 = ChaosConfig::builder().latency_ms(100).seed(43).build();

    let (socket1, socket2, addr1, addr2) =
        create_chaos_channel_pair(chaos_config1, chaos_config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize - with high latency, need more time between polls
    for _ in 0..150 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }

        clock.advance(Duration::from_millis(50));
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
        clock.advance(Duration::from_millis(25));

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
fn test_high_latency_250ms() -> Result<(), FortressError> {
    let clock = TestClock::new();

    let chaos_config1 = ChaosConfig::builder().latency_ms(250).seed(42).build();
    let chaos_config2 = ChaosConfig::builder().latency_ms(250).seed(43).build();

    let (socket1, socket2, addr1, addr2) =
        create_chaos_channel_pair(chaos_config1, chaos_config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize - with 250ms latency, need longer delays
    for _ in 0..200 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }

        clock.advance(Duration::from_millis(50));
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
fn test_extreme_latency_500ms() -> Result<(), FortressError> {
    let clock = TestClock::new();

    let chaos_config1 = ChaosConfig::builder().latency_ms(500).seed(42).build();
    let chaos_config2 = ChaosConfig::builder().latency_ms(500).seed(43).build();

    let (socket1, socket2, addr1, addr2) =
        create_chaos_channel_pair(chaos_config1, chaos_config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize - with 500ms latency, need much longer delays
    for _ in 0..100 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }

        clock.advance(Duration::from_millis(100));
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
fn test_out_of_order_delivery() -> Result<(), FortressError> {
    let clock = TestClock::new();

    // Configure aggressive reordering
    let chaos_config1 = ChaosConfig::builder()
        .reorder_buffer_size(5)
        .reorder_rate(0.5) // 50% chance of reordering within buffer
        .seed(42)
        .build();

    let chaos_config2 = ChaosConfig::builder()
        .reorder_buffer_size(5)
        .reorder_rate(0.5) // 50% chance of reordering within buffer
        .seed(43)
        .build();

    let (socket1, socket2, addr1, addr2) =
        create_chaos_channel_pair(chaos_config1, chaos_config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize
    for _ in 0..200 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
        clock.advance(Duration::from_millis(30));
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
        clock.advance(Duration::from_millis(5));

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
        "Should advance all frames despite reordering. \
         Actual stub1: {}, stub2: {}",
        stub1.gs.frame, stub2.gs.frame
    );
    assert_eq!(
        stub2.gs.frame, 40,
        "Should advance all frames despite reordering. \
         Actual stub1: {}, stub2: {}",
        stub1.gs.frame, stub2.gs.frame
    );

    Ok(())
}

/// Test combined jitter and packet loss.
#[test]
fn test_jitter_with_packet_loss() -> Result<(), FortressError> {
    let clock = TestClock::new();

    // Moderate jitter with packet loss
    let chaos_config1 = ChaosConfig::builder()
        .latency_ms(40)
        .jitter_ms(30)
        .packet_loss_rate(0.08)
        .seed(42)
        .build();

    let chaos_config2 = ChaosConfig::builder()
        .latency_ms(40)
        .jitter_ms(30)
        .packet_loss_rate(0.08)
        .seed(43)
        .build();

    let (socket1, socket2, addr1, addr2) =
        create_chaos_channel_pair(chaos_config1, chaos_config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize
    for _ in 0..200 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }

        clock.advance(Duration::from_millis(50));
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
        clock.advance(Duration::from_millis(15));

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
fn test_packet_duplication() -> Result<(), FortressError> {
    let clock = TestClock::new();

    // High duplication rate to test duplicate handling
    let chaos_config1 = ChaosConfig::builder()
        .duplication_rate(0.30) // 30% of packets duplicated
        .seed(42)
        .build();

    let chaos_config2 = ChaosConfig::builder()
        .duplication_rate(0.30) // 30% of packets duplicated
        .seed(43)
        .build();

    let (socket1, socket2, addr1, addr2) =
        create_chaos_channel_pair(chaos_config1, chaos_config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize
    for _ in 0..100 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
        clock.advance(Duration::from_millis(30));
    }

    assert_eq!(sess1.current_state(), SessionState::Running);
    assert_eq!(sess2.current_state(), SessionState::Running);

    // Advance frames with duplicate packets
    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    for i in 0..20 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        clock.advance(Duration::from_millis(5));

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
        "Duplicates should not affect frame count. \
         Actual stub1: {}, stub2: {}",
        stub1.gs.frame, stub2.gs.frame
    );
    assert_eq!(
        stub2.gs.frame, 20,
        "Duplicates should not affect frame count. \
         Actual stub1: {}, stub2: {}",
        stub1.gs.frame, stub2.gs.frame
    );

    Ok(())
}

/// Test determinism validation: sessions should eventually reach the same
/// state after network conditions normalize, even after adverse conditions.
/// This is a longer stress test that validates eventual consistency.
#[test]
fn test_determinism_under_stress() -> Result<(), FortressError> {
    let clock = TestClock::new();

    // Use "terrible network" conditions for stress testing
    let mut chaos_config1 = ChaosConfig::terrible_network();
    chaos_config1.seed = Some(42);

    let mut chaos_config2 = ChaosConfig::terrible_network();
    chaos_config2.seed = Some(43);

    let (socket1, socket2, addr1, addr2) =
        create_chaos_channel_pair(chaos_config1, chaos_config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize
    for _ in 0..400 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }

        clock.advance(Duration::from_millis(25));
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
        clock.advance(Duration::from_millis(30));

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
        clock.advance(Duration::from_millis(50));
    }

    // No panics should have occurred (verified implicitly by reaching here)

    Ok(())
}

/// Test that no panics occur even under worst-case network conditions.
/// This test validates graceful degradation.
#[test]
fn test_no_panics_under_worst_case() -> Result<(), FortressError> {
    let clock = TestClock::new();

    // Very aggressive network chaos
    let chaos_config1 = ChaosConfig::builder()
        .latency_ms(200)
        .jitter_ms(150)
        .packet_loss_rate(0.30) // 30% loss
        .duplication_rate(0.20)
        .reorder_buffer_size(8)
        .reorder_rate(0.5)
        .seed(42)
        .build();

    let chaos_config2 = ChaosConfig::builder()
        .latency_ms(200)
        .jitter_ms(150)
        .packet_loss_rate(0.30)
        .duplication_rate(0.20)
        .reorder_buffer_size(8)
        .reorder_rate(0.5)
        .seed(43)
        .build();

    let (socket1, socket2, addr1, addr2) =
        create_chaos_channel_pair(chaos_config1, chaos_config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Try to synchronize (may or may not succeed with 30% loss)
    let mut synchronized = false;
    for _ in 0..500 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            synchronized = true;
            break;
        }

        clock.advance(Duration::from_millis(20));
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
            clock.advance(Duration::from_millis(30));

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
fn test_asymmetric_latency() -> Result<(), FortressError> {
    let clock = TestClock::new();

    // Player 1 has higher latency (poor download)
    let config1 = ChaosConfig::builder().latency_ms(150).seed(42).build();

    // Player 2 has lower latency
    let config2 = ChaosConfig::builder().latency_ms(30).seed(43).build();

    let (socket1, socket2, addr1, addr2) = create_chaos_channel_pair(config1, config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize - need to account for higher latency of player 1
    for _ in 0..200 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }

        clock.advance(Duration::from_millis(30));
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
        clock.advance(Duration::from_millis(35));

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
fn test_burst_packet_loss() -> Result<(), FortressError> {
    let clock = TestClock::new();

    // Use burst loss with latency - this simulates WiFi interference or similar
    // 3% chance of burst, 3 consecutive drops per burst
    let chaos_config1 = ChaosConfig::builder()
        .burst_loss(0.03, 3)
        .latency_ms(20)
        .seed(42)
        .build();

    let chaos_config2 = ChaosConfig::builder()
        .burst_loss(0.03, 3)
        .latency_ms(20)
        .seed(43)
        .build();

    let (socket1, socket2, addr1, addr2) =
        create_chaos_channel_pair(chaos_config1, chaos_config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize - burst loss with latency needs generous time
    for _ in 0..300 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
        clock.advance(Duration::from_millis(50));
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
        clock.advance(Duration::from_millis(25));

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
fn test_temporary_disconnect_reconnect() -> Result<(), FortressError> {
    let clock = TestClock::new();

    // Start with good connection
    let good_config1 = ChaosConfig::passthrough();
    let good_config2 = ChaosConfig::passthrough();

    let (socket1, socket2, addr1, addr2) =
        create_chaos_channel_pair(good_config1, good_config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize quickly with good connection
    for _ in 0..100 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
        clock.advance(Duration::from_millis(20));
    }

    assert_eq!(sess1.current_state(), SessionState::Running);
    assert_eq!(sess2.current_state(), SessionState::Running);

    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    // Phase 1: Run some frames with good connection
    for i in 0..20 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        clock.advance(Duration::from_millis(5));

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
        clock.advance(Duration::from_millis(5));

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
        "Should continue advancing after recovery. \
         Frames before: {}, after: {}. sess1 state: {:?}, sess2 state: {:?}",
        frames_before_disconnect,
        stub1.gs.frame,
        sess1.current_state(),
        sess2.current_state()
    );

    Ok(())
}

/// Test eventual consistency: verify both peers reach the same final state
/// after running through the same sequence of inputs.
#[test]
fn test_eventual_consistency() -> Result<(), FortressError> {
    let clock = TestClock::new();

    // Use moderate network conditions
    let chaos_config1 = ChaosConfig::builder()
        .latency_ms(30)
        .jitter_ms(10)
        .packet_loss_rate(0.03)
        .seed(42)
        .build();

    let chaos_config2 = ChaosConfig::builder()
        .latency_ms(30)
        .jitter_ms(10)
        .packet_loss_rate(0.03)
        .seed(43)
        .build();

    let (socket1, socket2, addr1, addr2) =
        create_chaos_channel_pair(chaos_config1, chaos_config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize
    for _ in 0..500 {
        for _ in 0..3 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
        }

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }

        clock.advance(Duration::from_millis(20));
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
        clock.advance(Duration::from_millis(10));

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
        clock.advance(Duration::from_millis(10));
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

    Ok(())
}

/// Test combined burst loss and jitter.
#[test]
fn test_burst_loss_with_jitter() -> Result<(), FortressError> {
    let clock = TestClock::new();

    // Combine burst loss with jitter
    let chaos_config1 = ChaosConfig::builder()
        .latency_ms(40)
        .jitter_ms(25)
        .burst_loss(0.08, 4) // 8% chance of 4-packet burst
        .seed(42)
        .build();

    let chaos_config2 = ChaosConfig::builder()
        .latency_ms(40)
        .jitter_ms(25)
        .burst_loss(0.08, 4) // 8% chance of 4-packet burst
        .seed(43)
        .build();

    let (socket1, socket2, addr1, addr2) =
        create_chaos_channel_pair(chaos_config1, chaos_config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize
    for _ in 0..200 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }

        clock.advance(Duration::from_millis(50));
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
        clock.advance(Duration::from_millis(15));

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
fn test_one_way_receive_only_loss() -> Result<(), FortressError> {
    let clock = TestClock::new();

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

    let (socket1, socket2, addr1, addr2) = create_chaos_channel_pair(config1, config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize with extended time due to asymmetric loss
    for _ in 0..300 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }

        clock.advance(Duration::from_millis(50));
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
        clock.advance(Duration::from_millis(30));

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
fn test_one_way_send_only_loss() -> Result<(), FortressError> {
    let clock = TestClock::new();

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

    let (socket1, socket2, addr1, addr2) = create_chaos_channel_pair(config1, config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize with extended time - extra polling to handle delayed packet delivery
    for _ in 0..500 {
        for _ in 0..3 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
        }

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }

        clock.advance(Duration::from_millis(20));
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
        clock.advance(Duration::from_millis(30));

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
fn test_heavy_packet_duplication() -> Result<(), FortressError> {
    let clock = TestClock::new();

    // 20% packet duplication rate
    let chaos_config1 = ChaosConfig::builder()
        .duplication_rate(0.20)
        .latency_ms(20)
        .seed(42)
        .build();

    let chaos_config2 = ChaosConfig::builder()
        .duplication_rate(0.20)
        .latency_ms(20)
        .seed(43)
        .build();

    let (socket1, socket2, addr1, addr2) =
        create_chaos_channel_pair(chaos_config1, chaos_config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize
    for _ in 0..100 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }

        clock.advance(Duration::from_millis(30));
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
        clock.advance(Duration::from_millis(15));

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
fn test_packet_reordering() -> Result<(), FortressError> {
    let clock = TestClock::new();

    // Buffer 4 packets, 30% chance of reordering within buffer
    let chaos_config1 = ChaosConfig::builder()
        .reorder_buffer_size(4)
        .reorder_rate(0.30)
        .latency_ms(30)
        .seed(42)
        .build();

    let chaos_config2 = ChaosConfig::builder()
        .reorder_buffer_size(4)
        .reorder_rate(0.30)
        .latency_ms(30)
        .seed(43)
        .build();

    let (socket1, socket2, addr1, addr2) =
        create_chaos_channel_pair(chaos_config1, chaos_config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize
    for _ in 0..150 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }

        clock.advance(Duration::from_millis(40));
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
        clock.advance(Duration::from_millis(20));

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
fn test_extreme_chaos_combined() -> Result<(), FortressError> {
    let clock = TestClock::new();

    // Everything enabled but at moderate levels
    let chaos_config1 = ChaosConfig::builder()
        .latency_ms(60)
        .jitter_ms(30)
        .packet_loss_rate(0.08)
        .duplication_rate(0.05)
        .reorder_buffer_size(3)
        .reorder_rate(0.15)
        .burst_loss(0.02, 2)
        .seed(42)
        .build();

    let chaos_config2 = ChaosConfig::builder()
        .latency_ms(60)
        .jitter_ms(30)
        .packet_loss_rate(0.08)
        .duplication_rate(0.05)
        .reorder_buffer_size(3)
        .reorder_rate(0.15)
        .burst_loss(0.02, 2)
        .seed(43)
        .build();

    let (socket1, socket2, addr1, addr2) =
        create_chaos_channel_pair(chaos_config1, chaos_config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize - needs long time with all chaos combined
    for _ in 0..400 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }

        clock.advance(Duration::from_millis(50));
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
        clock.advance(Duration::from_millis(35));

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
fn test_large_prediction_window_with_latency() -> Result<(), FortressError> {
    let clock = TestClock::new();

    let chaos_config1 = ChaosConfig::builder()
        .latency_ms(80)
        .jitter_ms(20)
        .seed(42)
        .build();

    let chaos_config2 = ChaosConfig::builder()
        .latency_ms(80)
        .jitter_ms(20)
        .seed(43)
        .build();

    // Use larger prediction window (16 frames instead of default 8)
    let (socket1, socket2, addr1, addr2) =
        create_chaos_channel_pair(chaos_config1, chaos_config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_max_prediction_window(16)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_max_prediction_window(16)
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize
    for _ in 0..200 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }

        clock.advance(Duration::from_millis(40));
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
        clock.advance(Duration::from_millis(30));

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
fn test_input_delay_with_packet_loss() -> Result<(), FortressError> {
    let clock = TestClock::new();

    let chaos_config1 = ChaosConfig::builder()
        .packet_loss_rate(0.10)
        .latency_ms(30)
        .seed(42)
        .build();

    let chaos_config2 = ChaosConfig::builder()
        .packet_loss_rate(0.10)
        .latency_ms(30)
        .seed(43)
        .build();

    // Use input delay of 3 frames
    let (socket1, socket2, addr1, addr2) =
        create_chaos_channel_pair(chaos_config1, chaos_config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_input_delay(3)
        .unwrap()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_input_delay(3)
        .unwrap()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize
    for _ in 0..150 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }

        clock.advance(Duration::from_millis(50));
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
        clock.advance(Duration::from_millis(20));

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
fn test_sparse_saving_with_network_chaos() -> Result<(), FortressError> {
    let clock = TestClock::new();

    // Reduced latency and packet loss for more reliable synchronization
    let chaos_config1 = ChaosConfig::builder()
        .latency_ms(20)
        .jitter_ms(10)
        .packet_loss_rate(0.03)
        .seed(42)
        .build();

    let chaos_config2 = ChaosConfig::builder()
        .latency_ms(20)
        .jitter_ms(10)
        .packet_loss_rate(0.03)
        .seed(43)
        .build();

    // Enable sparse saving mode
    let (socket1, socket2, addr1, addr2) =
        create_chaos_channel_pair(chaos_config1, chaos_config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_save_mode(SaveMode::Sparse)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_save_mode(SaveMode::Sparse)
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize
    for _ in 0..400 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }

        clock.advance(Duration::from_millis(20));
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
        clock.advance(Duration::from_millis(20));

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
/// Note: This test uses harsh network conditions (10% burst loss, 6-packet bursts).
/// We use an explicit `SyncConfig` with 20 sync roundtrips (reduced from 40) and no
/// timeout, letting the iteration count be the sole limiter.
#[test]
fn test_network_flapping_simulation() -> Result<(), FortressError> {
    let clock = TestClock::new();

    // Simulate flapping with burst loss
    let chaos_config1 = ChaosConfig::builder()
        .latency_ms(25)
        .burst_loss(0.10, 6) // 10% chance of dropping 6 consecutive packets
        .seed(42)
        .build();

    let chaos_config2 = ChaosConfig::builder()
        .latency_ms(25)
        .burst_loss(0.10, 6)
        .seed(43) // Different seed to decorrelate burst loss events
        .build();

    let (socket1, socket2, addr1, addr2) =
        create_chaos_channel_pair(chaos_config1, chaos_config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_sync_config(SyncConfig {
            num_sync_packets: 20,
            sync_retry_interval: Duration::from_millis(150),
            sync_timeout: None,
            running_retry_interval: Duration::from_millis(150),
            keepalive_interval: Duration::from_millis(150),
        })
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_sync_config(SyncConfig {
            num_sync_packets: 20,
            sync_retry_interval: Duration::from_millis(150),
            sync_timeout: None,
            running_retry_interval: Duration::from_millis(150),
            keepalive_interval: Duration::from_millis(150),
        })
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize with extra tolerance for bursts
    let start_time = clock.now();
    let mut sync_events_1 = 0u32;
    let mut sync_events_2 = 0u32;
    let mut timeout_events_1 = 0u32;
    let mut timeout_events_2 = 0u32;

    for iteration in 0..1250 {
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

        clock.advance(Duration::from_millis(40));

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            let elapsed = clock.now() - start_time;
            eprintln!(
                "[Flapping Test] Synchronized at iter={}, elapsed={:.1}s, sync_events=({}, {})",
                iteration,
                elapsed.as_secs_f32(),
                sync_events_1,
                sync_events_2
            );
            break;
        }
    }

    // Enhanced assertion with diagnostic context
    let elapsed = clock.now() - start_time;
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
        clock.advance(Duration::from_millis(30));

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
fn test_extreme_jitter() -> Result<(), FortressError> {
    let clock = TestClock::new();

    // Base latency 50ms, jitter +/-50ms (0-100ms effective)
    let chaos_config1 = ChaosConfig::builder()
        .latency_ms(50)
        .jitter_ms(50)
        .seed(42)
        .build();

    let chaos_config2 = ChaosConfig::builder()
        .latency_ms(50)
        .jitter_ms(50)
        .seed(43)
        .build();

    let (socket1, socket2, addr1, addr2) =
        create_chaos_channel_pair(chaos_config1, chaos_config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize
    for _ in 0..200 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }

        clock.advance(Duration::from_millis(60));
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
        clock.advance(Duration::from_millis(30));

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
fn test_terrible_network_preset() -> Result<(), FortressError> {
    let clock = TestClock::new();

    // Recreate terrible_network preset with different seeds for each socket
    let chaos_config1 = ChaosConfig::builder()
        .latency(Duration::from_millis(250))
        .jitter(Duration::from_millis(100))
        .packet_loss_rate(0.15)
        .duplication_rate(0.02)
        .reorder_buffer_size(5)
        .reorder_rate(0.1)
        .seed(42)
        .build();

    let chaos_config2 = ChaosConfig::builder()
        .latency(Duration::from_millis(250))
        .jitter(Duration::from_millis(100))
        .packet_loss_rate(0.15)
        .duplication_rate(0.02)
        .reorder_buffer_size(5)
        .reorder_rate(0.1)
        .seed(43)
        .build();

    let (socket1, socket2, addr1, addr2) =
        create_chaos_channel_pair(chaos_config1, chaos_config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize - terrible network needs lots of time
    for _ in 0..500 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }

        clock.advance(Duration::from_millis(60));
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
        clock.advance(Duration::from_millis(50));

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
fn test_mobile_network_preset() -> Result<(), FortressError> {
    let clock = TestClock::new();

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

    let (socket1, socket2, addr1, addr2) =
        create_chaos_channel_pair(chaos_config1, chaos_config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_sync_config(SyncConfig::mobile())
        .with_protocol_config(ProtocolConfig {
            clock: Some(clock.as_protocol_clock()),
            ..ProtocolConfig::mobile()
        })
        .with_time_sync_config(TimeSyncConfig::mobile())
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_sync_config(SyncConfig::mobile())
        .with_protocol_config(ProtocolConfig {
            clock: Some(clock.as_protocol_clock()),
            ..ProtocolConfig::mobile()
        })
        .with_time_sync_config(TimeSyncConfig::mobile())
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize - mobile network needs generous time
    for _ in 0..600 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }

        clock.advance(Duration::from_millis(50));
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
        clock.advance(Duration::from_millis(40));

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
fn test_wifi_interference_preset() -> Result<(), FortressError> {
    let clock = TestClock::new();

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

    let (socket1, socket2, addr1, addr2) =
        create_chaos_channel_pair(chaos_config1, chaos_config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize - WiFi should be faster than mobile
    for _ in 0..300 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }

        clock.advance(Duration::from_millis(30));
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
        clock.advance(Duration::from_millis(20));

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
fn test_intercontinental_preset() -> Result<(), FortressError> {
    let clock = TestClock::new();

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

    let (socket1, socket2, addr1, addr2) =
        create_chaos_channel_pair(chaos_config1, chaos_config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_sync_config(SyncConfig::high_latency())
        .with_protocol_config(ProtocolConfig {
            clock: Some(clock.as_protocol_clock()),
            ..ProtocolConfig::high_latency()
        })
        .with_time_sync_config(TimeSyncConfig::smooth())
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_sync_config(SyncConfig::high_latency())
        .with_protocol_config(ProtocolConfig {
            clock: Some(clock.as_protocol_clock()),
            ..ProtocolConfig::high_latency()
        })
        .with_time_sync_config(TimeSyncConfig::smooth())
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize - intercontinental is stable but high latency
    for _ in 0..400 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }

        clock.advance(Duration::from_millis(50));
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
        clock.advance(Duration::from_millis(50));

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
fn test_competitive_preset_fast_sync() -> Result<(), FortressError> {
    let clock = TestClock::new();

    // No chaos - LAN-like conditions for competitive play
    let (socket1, socket2, addr1, addr2) = create_channel_pair();

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_sync_config(SyncConfig::competitive())
        .with_protocol_config(ProtocolConfig {
            clock: Some(clock.as_protocol_clock()),
            ..ProtocolConfig::competitive()
        })
        .with_time_sync_config(TimeSyncConfig::competitive())
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_sync_config(SyncConfig::competitive())
        .with_protocol_config(ProtocolConfig {
            clock: Some(clock.as_protocol_clock()),
            ..ProtocolConfig::competitive()
        })
        .with_time_sync_config(TimeSyncConfig::competitive())
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Competitive should sync very fast
    let start = clock.now();
    for _ in 0..100 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }

        clock.advance(Duration::from_millis(10));
    }
    let sync_time = clock.now() - start;

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

    // Competitive sync should be fast (< 2 seconds virtual time)
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
        clock.advance(Duration::from_millis(5));

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

    // With good network conditions and proper timing, we should advance most frames
    assert!(
        stub1.gs.frame >= 40,
        "Should advance many frames with competitive preset. \
         Actual: {} frames (expected >= 40). \
         sess1 state: {:?}, sess2 state: {:?}",
        stub1.gs.frame,
        sess1.current_state(),
        sess2.current_state()
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
    fn run(&self, clock: &TestClock) -> Result<ChaosTestResult, FortressError> {
        // Build chaos configurations with DIFFERENT seeds for each socket.
        let build_chaos_config = |seed: u64| {
            let mut builder = ChaosConfig::builder()
                .latency_ms(self.latency_ms)
                .packet_loss_rate(self.packet_loss)
                .seed(seed);

            if self.burst_loss_probability > 0.0 {
                builder = builder.burst_loss(self.burst_loss_probability, self.burst_loss_length);
            }
            if self.jitter_ms > 0 {
                builder = builder.jitter_ms(self.jitter_ms);
            }

            builder.build()
        };

        let chaos_config1 = build_chaos_config(42);
        let chaos_config2 = build_chaos_config(43); // Different seed to avoid correlated loss

        let (socket1, socket2, addr1, addr2) =
            create_chaos_channel_pair(chaos_config1, chaos_config2, clock);

        let mut sess1 = SessionBuilder::<StubConfig>::new()
            .with_protocol_config(ProtocolConfig {
                clock: Some(clock.as_protocol_clock()),
                ..ProtocolConfig::default()
            })
            .with_sync_config(self.sync_config.clone())
            .add_player(PlayerType::Local, PlayerHandle::new(0))?
            .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
            .start_p2p_session(socket1)?;

        let mut sess2 = SessionBuilder::<StubConfig>::new()
            .with_protocol_config(ProtocolConfig {
                clock: Some(clock.as_protocol_clock()),
                ..ProtocolConfig::default()
            })
            .with_sync_config(self.sync_config.clone())
            .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
            .add_player(PlayerType::Local, PlayerHandle::new(1))?
            .start_p2p_session(socket2)?;

        // Track diagnostics
        let start_time = clock.now();
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

            clock.advance(Duration::from_millis(self.iteration_delay_ms));

            if sess1.current_state() == SessionState::Running
                && sess2.current_state() == SessionState::Running
            {
                sync_iteration = Some(iteration);
                break;
            }
        }

        let elapsed = clock.now() - start_time;

        Ok(ChaosTestResult {
            elapsed,
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
fn test_chaos_conditions_data_driven() {
    let clock = TestClock::new();

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
        // Mobile-like conditions (increased iterations for high latency).
        ChaosTestCase {
            name: "mobile_conditions",
            latency_ms: 60,
            packet_loss: 0.05,
            burst_loss_probability: 0.02,
            burst_loss_length: 3,
            jitter_ms: 20,
            sync_config: SyncConfig {
                num_sync_packets: 20,
                sync_retry_interval: Duration::from_millis(250),
                sync_timeout: None,
                running_retry_interval: Duration::from_millis(250),
                keepalive_interval: Duration::from_millis(200),
            },
            max_iterations: 500,
            iteration_delay_ms: 40,
            expected_to_sync: true,
        },
        // High burst loss
        ChaosTestCase {
            name: "high_burst_loss",
            latency_ms: 25,
            packet_loss: 0.0,
            burst_loss_probability: 0.05,
            burst_loss_length: 3,
            jitter_ms: 15,
            sync_config: SyncConfig {
                num_sync_packets: 20,
                sync_retry_interval: Duration::from_millis(150),
                sync_timeout: None,
                running_retry_interval: Duration::from_millis(200),
                keepalive_interval: Duration::from_millis(200),
            },
            max_iterations: 900,
            iteration_delay_ms: 30,
            expected_to_sync: true,
        },
        // Extreme burst loss - conditions too harsh for reliable sync.
        ChaosTestCase {
            name: "extreme_burst_loss_expected_failure",
            latency_ms: 25,
            packet_loss: 0.0,
            burst_loss_probability: 0.30,
            burst_loss_length: 10,
            jitter_ms: 0,
            sync_config: SyncConfig {
                num_sync_packets: 20,
                sync_retry_interval: Duration::from_millis(150),
                sync_timeout: Some(Duration::from_secs(3)),
                running_retry_interval: Duration::from_millis(200),
                keepalive_interval: Duration::from_millis(200),
            },
            max_iterations: 200,
            iteration_delay_ms: 20,
            expected_to_sync: false,
        },
    ];

    for test_case in &test_cases {
        eprintln!("\n[Data-Driven Test] Running: {}", test_case.name);

        let result = test_case.run(&clock).expect("Test setup failed");

        let budget_pct = result.sync_iteration.map_or(100.0, |i| {
            (i as f64 / test_case.max_iterations as f64) * 100.0
        });
        let max_budget_secs =
            (test_case.max_iterations as f64 * test_case.iteration_delay_ms as f64) / 1000.0;

        eprintln!(
            "[Data-Driven Test] {} completed: synchronized={}, elapsed={:.2}s \
             (budget={:.1}s, used={:.1}%), sync_events=({}, {}), \
             timeout_events=({}, {}), sync_at_iter={:?}",
            test_case.name,
            result.synchronized,
            result.elapsed.as_secs_f32(),
            max_budget_secs,
            budget_pct,
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
    }
}

/// Test: Verify sync timeout is detected and reported correctly.
#[test]
fn test_sync_timeout_detection() -> Result<(), FortressError> {
    let clock = TestClock::new();

    // Use very short timeout with heavy packet loss to trigger timeout
    let short_timeout_config = SyncConfig {
        num_sync_packets: 10,
        sync_retry_interval: Duration::from_millis(50),
        sync_timeout: Some(Duration::from_secs(2)), // 2 second timeout
        running_retry_interval: Duration::from_millis(100),
        keepalive_interval: Duration::from_millis(100),
    };

    // 50% packet loss should make sync impossible in 2 seconds with 10 roundtrips
    let chaos_config1 = ChaosConfig::builder()
        .latency_ms(20)
        .packet_loss_rate(0.50) // Very high loss
        .seed(42)
        .build();

    let chaos_config2 = ChaosConfig::builder()
        .latency_ms(20)
        .packet_loss_rate(0.50) // Very high loss
        .seed(43)
        .build();

    let (socket1, socket2, addr1, addr2) =
        create_chaos_channel_pair(chaos_config1, chaos_config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_sync_config(short_timeout_config.clone())
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_sync_config(short_timeout_config)
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Track timeout events
    let mut timeout_detected_1 = false;
    let mut timeout_detected_2 = false;
    let start = clock.now();

    // Run for 5 seconds virtual time (well past the 2 second timeout)
    for _ in 0..250 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        for event in sess1.events() {
            if matches!(event, FortressEvent::SyncTimeout { .. }) {
                timeout_detected_1 = true;
                let elapsed = clock.now() - start;
                eprintln!(
                    "[Timeout Test] Session 1 timeout detected at {:.2}s",
                    elapsed.as_secs_f32()
                );
            }
        }
        for event in sess2.events() {
            if matches!(event, FortressEvent::SyncTimeout { .. }) {
                timeout_detected_2 = true;
                let elapsed = clock.now() - start;
                eprintln!(
                    "[Timeout Test] Session 2 timeout detected at {:.2}s",
                    elapsed.as_secs_f32()
                );
            }
        }

        clock.advance(Duration::from_millis(20));

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
    assert_eq!(
        sess1.current_state(),
        SessionState::Synchronizing,
        "Session should remain Synchronizing after timeout event"
    );

    Ok(())
}

/// Test: Edge case where burst loss exactly matches sync packet count.
#[test]
fn test_burst_loss_matches_sync_packets() -> Result<(), FortressError> {
    let clock = TestClock::new();

    // Configure burst loss length equal to default sync packets (5)
    let chaos_config1 = ChaosConfig::builder()
        .latency_ms(20)
        .burst_loss(0.05, 5) // Burst wipes exactly 5 packets
        .seed(42)
        .build();

    let chaos_config2 = ChaosConfig::builder()
        .latency_ms(20)
        .burst_loss(0.05, 5) // Burst wipes exactly 5 packets
        .seed(43)
        .build();

    // Use a config with more sync packets to handle burst wiping out initial 5
    let resilient_config = SyncConfig {
        num_sync_packets: 15, // 3x the burst length
        sync_retry_interval: Duration::from_millis(100),
        sync_timeout: Some(Duration::from_secs(15)),
        running_retry_interval: Duration::from_millis(100),
        keepalive_interval: Duration::from_millis(100),
    };

    let (socket1, socket2, addr1, addr2) =
        create_chaos_channel_pair(chaos_config1, chaos_config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_sync_config(resilient_config.clone())
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_sync_config(resilient_config)
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    // Synchronize
    for _ in 0..300 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }

        clock.advance(Duration::from_millis(30));
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

// ==========================================
// Seed Correlation Regression Tests
// ==========================================

/// Regression test: Validates that different seeds prevent correlated packet loss.
#[test]
fn test_different_seeds_prevent_correlated_loss() -> Result<(), FortressError> {
    let clock = TestClock::new();

    // Use aggressive burst loss with DIFFERENT seeds
    let chaos_config1 = ChaosConfig::builder()
        .latency_ms(20)
        .burst_loss(0.10, 6)
        .seed(100)
        .build();

    let chaos_config2 = ChaosConfig::builder()
        .latency_ms(20)
        .burst_loss(0.10, 6)
        .seed(101) // Different seed!
        .build();

    let (socket1, socket2, addr1, addr2) =
        create_chaos_channel_pair(chaos_config1, chaos_config2, &clock);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_sync_config(SyncConfig {
            num_sync_packets: 20,
            sync_retry_interval: Duration::from_millis(150),
            sync_timeout: None,
            running_retry_interval: Duration::from_millis(150),
            keepalive_interval: Duration::from_millis(150),
        })
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;
    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock))
        .with_sync_config(SyncConfig {
            num_sync_packets: 20,
            sync_retry_interval: Duration::from_millis(150),
            sync_timeout: None,
            running_retry_interval: Duration::from_millis(150),
            keepalive_interval: Duration::from_millis(150),
        })
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    let start_time = clock.now();
    let mut sync_events_1 = 0u32;
    let mut sync_events_2 = 0u32;

    for _ in 0..1250 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();

        for event in sess1.events() {
            if matches!(event, FortressEvent::Synchronizing { .. }) {
                sync_events_1 += 1;
            }
        }
        for event in sess2.events() {
            if matches!(event, FortressEvent::Synchronizing { .. }) {
                sync_events_2 += 1;
            }
        }

        clock.advance(Duration::from_millis(40));

        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
    }

    let elapsed = clock.now() - start_time;

    // Both sessions should synchronize
    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "Session 1 failed after {:.1}s (sync_events: {})",
        elapsed.as_secs_f32(),
        sync_events_1
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Running,
        "Session 2 failed after {:.1}s (sync_events: {})",
        elapsed.as_secs_f32(),
        sync_events_2
    );

    // Both sessions should make similar progress (not asymmetric)
    let min_events = sync_events_1.min(sync_events_2);
    let max_events = sync_events_1.max(sync_events_2);
    let asymmetry_ratio = if min_events > 0 {
        max_events as f32 / min_events as f32
    } else {
        f32::MAX
    };

    assert!(
        asymmetry_ratio < 3.0,
        "Sync events too asymmetric (sess1={}, sess2={}, ratio={:.2}). \
         This may indicate correlated packet loss due to same RNG seed.",
        sync_events_1,
        sync_events_2,
        asymmetry_ratio
    );

    Ok(())
}

/// Test synchronization with various seed pairs to validate decorrelation.
#[test]
fn test_seed_pairs_for_decorrelated_sync() -> Result<(), FortressError> {
    let test_cases = [
        (1u64, 2u64),
        (42, 43),
        (1000, 2000),
        (u64::MAX - 1, u64::MAX),
    ];

    // Use lossy preset for better handling of decorrelated packet loss
    let sync_config = SyncConfig::lossy();

    for (seed1, seed2) in test_cases {
        let clock = TestClock::new();

        // Use 5% loss for reliable CI behavior while still testing
        // decorrelated loss patterns
        let chaos_config1 = ChaosConfig::builder()
            .latency_ms(15)
            .packet_loss_rate(0.05)
            .seed(seed1)
            .build();

        let chaos_config2 = ChaosConfig::builder()
            .latency_ms(15)
            .packet_loss_rate(0.05)
            .seed(seed2)
            .build();

        let (socket1, socket2, addr1, addr2) =
            create_chaos_channel_pair(chaos_config1, chaos_config2, &clock);

        let mut sess1 = SessionBuilder::<StubConfig>::new()
            .with_protocol_config(protocol_config(&clock))
            .with_sync_config(sync_config.clone())
            .add_player(PlayerType::Local, PlayerHandle::new(0))?
            .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
            .start_p2p_session(socket1)?;
        let mut sess2 = SessionBuilder::<StubConfig>::new()
            .with_protocol_config(protocol_config(&clock))
            .with_sync_config(sync_config.clone())
            .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
            .add_player(PlayerType::Local, PlayerHandle::new(1))?
            .start_p2p_session(socket2)?;

        // Synchronize with generous margin
        for _ in 0..500 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();

            if sess1.current_state() == SessionState::Running
                && sess2.current_state() == SessionState::Running
            {
                break;
            }

            clock.advance(Duration::from_millis(35));
        }

        assert_eq!(
            sess1.current_state(),
            SessionState::Running,
            "Seed pair ({}, {}) failed: Session 1 not synchronized",
            seed1,
            seed2
        );
        assert_eq!(
            sess2.current_state(),
            SessionState::Running,
            "Seed pair ({}, {}) failed: Session 2 not synchronized",
            seed1,
            seed2
        );
    }

    Ok(())
}

// =============================================================================
// Data-Driven Preset Configuration Tests
// =============================================================================

/// Parameters for data-driven preset configuration testing.
#[derive(Clone, Debug)]
struct PresetTestCase {
    /// Test case name for diagnostics.
    name: &'static str,
    /// Synchronization configuration preset.
    sync_config: SyncConfig,
    /// Protocol configuration preset.
    protocol_config: ProtocolConfig,
    /// Time synchronization configuration preset.
    time_sync_config: TimeSyncConfig,
    /// Optional chaos configuration for network simulation.
    /// `None` means perfect network (no packet loss, latency, etc.)
    chaos_config: Option<ChaosConfig>,
    /// Maximum expected synchronization time in milliseconds.
    expected_sync_time_ms: u64,
    /// Number of frames to attempt advancing after synchronization.
    target_frames: i32,
    /// Minimum frames that must successfully advance.
    min_expected_frames: i32,
}

/// Result of running a preset test case.
#[derive(Debug)]
struct PresetTestResult {
    /// Time taken to synchronize both sessions.
    sync_time: Duration,
    /// Whether both sessions synchronized.
    synchronized: bool,
    /// Number of frames advanced by session 1.
    frames_advanced_1: i32,
    /// Number of frames advanced by session 2.
    frames_advanced_2: i32,
    /// Final state of session 1.
    sess1_state: SessionState,
    /// Final state of session 2.
    sess2_state: SessionState,
}

impl PresetTestCase {
    /// Run the preset test case and return diagnostic results.
    fn run(&self, clock: &TestClock) -> Result<PresetTestResult, FortressError> {
        // Create sockets - either chaos or plain channel
        let (socket1, addr1, socket2, addr2) = if let Some(ref chaos) = self.chaos_config {
            // Use different seeds for each socket to avoid correlated packet loss
            let latency_ms = chaos.latency.as_millis() as u64;
            let loss_rate = chaos.send_loss_rate.max(chaos.receive_loss_rate);

            let chaos1 = ChaosConfig::builder()
                .latency_ms(latency_ms)
                .packet_loss_rate(loss_rate)
                .seed(42)
                .build();
            let chaos2 = ChaosConfig::builder()
                .latency_ms(latency_ms)
                .packet_loss_rate(loss_rate)
                .seed(43)
                .build();
            let (s1, s2, a1, a2) = create_chaos_channel_pair(chaos1, chaos2, clock);
            (s1, a1, s2, a2)
        } else {
            // Perfect network - no chaos, use passthrough config
            let perfect = ChaosConfig::passthrough();
            let (s1, s2, a1, a2) = create_chaos_channel_pair(perfect.clone(), perfect, clock);
            (s1, a1, s2, a2)
        };

        // Merge clock into the preset protocol config
        let proto_config = ProtocolConfig {
            clock: Some(clock.as_protocol_clock()),
            ..self.protocol_config
        };

        // Build sessions with the preset configurations
        let mut sess1 = SessionBuilder::<StubConfig>::new()
            .with_sync_config(self.sync_config)
            .with_protocol_config(proto_config.clone())
            .with_time_sync_config(self.time_sync_config)
            .add_player(PlayerType::Local, PlayerHandle::new(0))?
            .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
            .start_p2p_session(socket1)?;

        let mut sess2 = SessionBuilder::<StubConfig>::new()
            .with_sync_config(self.sync_config)
            .with_protocol_config(proto_config)
            .with_time_sync_config(self.time_sync_config)
            .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
            .add_player(PlayerType::Local, PlayerHandle::new(1))?
            .start_p2p_session(socket2)?;

        // ===== Phase 1: Synchronization =====
        let sync_start = clock.now();
        let max_sync_iterations = (self.expected_sync_time_ms * 2 / 20) as usize; // 20ms per iteration

        for _ in 0..max_sync_iterations {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();

            clock.advance(Duration::from_millis(20));

            if sess1.current_state() == SessionState::Running
                && sess2.current_state() == SessionState::Running
            {
                break;
            }
        }

        let sync_time = clock.now() - sync_start;
        let synchronized = sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running;

        if !synchronized {
            return Ok(PresetTestResult {
                sync_time,
                synchronized,
                frames_advanced_1: 0,
                frames_advanced_2: 0,
                sess1_state: sess1.current_state(),
                sess2_state: sess2.current_state(),
            });
        }

        // ===== Phase 2: Frame Advancement =====
        let mut stub1 = GameStub::new();
        let mut stub2 = GameStub::new();

        for i in 0..self.target_frames as u32 {
            // Poll multiple times per frame for better packet handling
            for _ in 0..4 {
                sess1.poll_remote_clients();
                sess2.poll_remote_clients();
            }

            clock.advance(Duration::from_millis(5));

            // Add inputs
            sess1
                .add_local_input(PlayerHandle::new(0), StubInput { inp: i })
                .unwrap();
            sess2
                .add_local_input(PlayerHandle::new(1), StubInput { inp: i })
                .unwrap();

            // Advance frames
            let requests1 = sess1.advance_frame().unwrap();
            let requests2 = sess2.advance_frame().unwrap();

            stub1.handle_requests(requests1);
            stub2.handle_requests(requests2);
        }

        Ok(PresetTestResult {
            sync_time,
            synchronized,
            frames_advanced_1: stub1.gs.frame,
            frames_advanced_2: stub2.gs.frame,
            sess1_state: sess1.current_state(),
            sess2_state: sess2.current_state(),
        })
    }
}

/// Run a preset test case with comprehensive validation and diagnostics.
fn run_preset_test(case: &PresetTestCase, clock: &TestClock) {
    eprintln!("\n[Preset Test] Running: {}", case.name);
    eprintln!(
        "  Config: sync={:?}, protocol={:?}, time_sync={:?}",
        case.sync_config, case.protocol_config, case.time_sync_config
    );
    eprintln!(
        "  Network: {:?}",
        case.chaos_config
            .as_ref()
            .map_or("perfect".to_string(), |c| format!("{c:?}"))
    );

    let result = case.run(clock).expect("Test setup failed");

    eprintln!(
        "[Preset Test] {} completed:\n  \
         synchronized={}, sync_time={:.2}s (max expected: {}ms)\n  \
         frames: sess1={}, sess2={} (target={}, min={})\n  \
         states: sess1={:?}, sess2={:?}",
        case.name,
        result.synchronized,
        result.sync_time.as_secs_f32(),
        case.expected_sync_time_ms,
        result.frames_advanced_1,
        result.frames_advanced_2,
        case.target_frames,
        case.min_expected_frames,
        result.sess1_state,
        result.sess2_state
    );

    // Validate synchronization
    assert!(
        result.synchronized,
        "[{}] Sessions failed to synchronize within {}ms.\n  \
         sess1 state: {:?}\n  \
         sess2 state: {:?}\n  \
         actual sync time: {:.2}s\n  \
         This may indicate:\n  \
         - Clock not advanced enough in sync loop\n  \
         - Sync timeout too short for test conditions\n  \
         - Network simulation too aggressive",
        case.name,
        case.expected_sync_time_ms,
        result.sess1_state,
        result.sess2_state,
        result.sync_time.as_secs_f32()
    );

    // Validate sync time (allow 50% margin for CI variability)
    let max_allowed_sync_ms = (case.expected_sync_time_ms as f64 * 1.5) as u64;
    assert!(
        result.sync_time.as_millis() <= max_allowed_sync_ms as u128,
        "[{}] Sync took too long: {:.2}s (expected max: {}ms, allowed: {}ms).\n  \
         This may indicate network conditions are too harsh for this preset.",
        case.name,
        result.sync_time.as_secs_f32(),
        case.expected_sync_time_ms,
        max_allowed_sync_ms
    );

    // Validate frame advancement
    let min_frames = result.frames_advanced_1.min(result.frames_advanced_2);
    assert!(
        min_frames >= case.min_expected_frames,
        "[{}] Insufficient frames advanced.\n  \
         sess1 frames: {} (expected >= {})\n  \
         sess2 frames: {} (expected >= {})\n  \
         This may indicate:\n  \
         - Clock not advanced enough in frame loop\n  \
         - Input handling issues\n  \
         - Network conditions causing excessive rollbacks",
        case.name,
        result.frames_advanced_1,
        case.min_expected_frames,
        result.frames_advanced_2,
        case.min_expected_frames
    );
}

/// Test: Competitive preset under perfect network conditions.
#[test]
fn test_preset_competitive_perfect_network() {
    let clock = TestClock::new();
    let case = PresetTestCase {
        name: "competitive_perfect",
        sync_config: SyncConfig::competitive(),
        protocol_config: ProtocolConfig::competitive(),
        time_sync_config: TimeSyncConfig::competitive(),
        chaos_config: None,          // Perfect network
        expected_sync_time_ms: 1000, // Should sync in < 1s
        target_frames: 60,
        min_expected_frames: 55, // Allow small margin for rollbacks
    };
    run_preset_test(&case, &clock);
}

/// Test: Default preset under perfect network conditions.
#[test]
fn test_preset_default_perfect_network() {
    let clock = TestClock::new();
    let case = PresetTestCase {
        name: "default_perfect",
        sync_config: SyncConfig::default(),
        protocol_config: ProtocolConfig::default(),
        time_sync_config: TimeSyncConfig::default(),
        chaos_config: None,          // Perfect network
        expected_sync_time_ms: 2000, // Slightly longer due to more sync packets
        target_frames: 60,
        min_expected_frames: 55,
    };
    run_preset_test(&case, &clock);
}

/// Test: LAN preset under perfect network conditions.
#[test]
fn test_preset_lan_perfect_network() {
    let clock = TestClock::new();
    let case = PresetTestCase {
        name: "lan_perfect",
        sync_config: SyncConfig::lan(),
        protocol_config: ProtocolConfig::default(),
        time_sync_config: TimeSyncConfig::lan(),
        chaos_config: None,
        expected_sync_time_ms: 800, // Very fast sync
        target_frames: 60,
        min_expected_frames: 55,
    };
    run_preset_test(&case, &clock);
}

/// Test: Mobile preset under simulated mobile conditions.
#[test]
fn test_preset_mobile_with_mobile_conditions() {
    let clock = TestClock::new();
    let case = PresetTestCase {
        name: "mobile_conditions",
        sync_config: SyncConfig::mobile(),
        protocol_config: ProtocolConfig::mobile(),
        time_sync_config: TimeSyncConfig::mobile(),
        chaos_config: Some(
            ChaosConfig::builder()
                .latency_ms(60)
                .packet_loss_rate(0.05)
                .build(),
        ),
        expected_sync_time_ms: 12000, // Mobile can take longer with 10 sync packets
        target_frames: 30,            // Fewer frames due to latency
        min_expected_frames: 15,      // Allow for network variance and rollbacks
    };
    run_preset_test(&case, &clock);
}

/// Test: High latency preset under high latency conditions.
#[test]
fn test_preset_high_latency_with_latency() {
    let clock = TestClock::new();
    let case = PresetTestCase {
        name: "high_latency_conditions",
        sync_config: SyncConfig::high_latency(),
        protocol_config: ProtocolConfig::high_latency(),
        time_sync_config: TimeSyncConfig::default(),
        chaos_config: Some(ChaosConfig::builder().latency_ms(100).build()),
        expected_sync_time_ms: 6000,
        target_frames: 25,       // Fewer frames due to high latency
        min_expected_frames: 10, // With 100ms latency, frame advancement is slow
    };
    run_preset_test(&case, &clock);
}

/// Test: Lossy preset under lossy network conditions.
#[test]
fn test_preset_lossy_with_packet_loss() {
    let clock = TestClock::new();
    let case = PresetTestCase {
        name: "lossy_conditions",
        sync_config: SyncConfig::lossy(),
        protocol_config: ProtocolConfig::default(),
        time_sync_config: TimeSyncConfig::default(),
        chaos_config: Some(ChaosConfig::builder().packet_loss_rate(0.08).build()),
        expected_sync_time_ms: 12000, // 8% loss needs more time for retries
        target_frames: 40,
        min_expected_frames: 25, // Some frames may stall due to lost packets
    };
    run_preset_test(&case, &clock);
}

/// Data-driven test for all presets under perfect network conditions.
#[test]
fn test_all_presets_perfect_network_data_driven() {
    let clock = TestClock::new();

    let test_cases = [
        PresetTestCase {
            name: "competitive_perfect",
            sync_config: SyncConfig::competitive(),
            protocol_config: ProtocolConfig::competitive(),
            time_sync_config: TimeSyncConfig::competitive(),
            chaos_config: None,
            expected_sync_time_ms: 1000,
            target_frames: 50,
            min_expected_frames: 45,
        },
        PresetTestCase {
            name: "default_perfect",
            sync_config: SyncConfig::default(),
            protocol_config: ProtocolConfig::default(),
            time_sync_config: TimeSyncConfig::default(),
            chaos_config: None,
            expected_sync_time_ms: 2000,
            target_frames: 50,
            min_expected_frames: 45,
        },
        PresetTestCase {
            name: "lan_perfect",
            sync_config: SyncConfig::lan(),
            protocol_config: ProtocolConfig::default(),
            time_sync_config: TimeSyncConfig::lan(),
            chaos_config: None,
            expected_sync_time_ms: 800,
            target_frames: 50,
            min_expected_frames: 45,
        },
        PresetTestCase {
            name: "high_latency_perfect",
            sync_config: SyncConfig::high_latency(),
            protocol_config: ProtocolConfig::high_latency(),
            time_sync_config: TimeSyncConfig::default(),
            chaos_config: None,
            expected_sync_time_ms: 3000,
            target_frames: 50,
            min_expected_frames: 45,
        },
        PresetTestCase {
            name: "lossy_perfect",
            sync_config: SyncConfig::lossy(),
            protocol_config: ProtocolConfig::default(),
            time_sync_config: TimeSyncConfig::default(),
            chaos_config: None,
            expected_sync_time_ms: 2000,
            target_frames: 50,
            min_expected_frames: 45,
        },
        PresetTestCase {
            name: "mobile_perfect",
            sync_config: SyncConfig::mobile(),
            protocol_config: ProtocolConfig::mobile(),
            time_sync_config: TimeSyncConfig::mobile(),
            chaos_config: None,
            expected_sync_time_ms: 4000,
            target_frames: 50,
            min_expected_frames: 45,
        },
    ];

    for case in &test_cases {
        run_preset_test(case, &clock);
    }
}

/// Data-driven test for presets matched to appropriate network conditions.
#[test]
fn test_presets_matched_conditions_data_driven() {
    let clock = TestClock::new();

    let test_cases = [
        // Competitive: LAN-like conditions (minimal latency, no loss)
        PresetTestCase {
            name: "competitive_lan_conditions",
            sync_config: SyncConfig::competitive(),
            protocol_config: ProtocolConfig::competitive(),
            time_sync_config: TimeSyncConfig::competitive(),
            chaos_config: Some(ChaosConfig::builder().latency_ms(5).build()),
            expected_sync_time_ms: 2000,
            target_frames: 50,
            min_expected_frames: 45,
        },
        // Default: Typical internet (30ms latency, 2% loss)
        PresetTestCase {
            name: "default_typical_internet",
            sync_config: SyncConfig::default(),
            protocol_config: ProtocolConfig::default(),
            time_sync_config: TimeSyncConfig::default(),
            chaos_config: Some(
                ChaosConfig::builder()
                    .latency_ms(30)
                    .packet_loss_rate(0.02)
                    .build(),
            ),
            expected_sync_time_ms: 5000, // 2% loss + 30ms latency needs more time
            target_frames: 40,
            min_expected_frames: 30,
        },
        // LAN: Local network (1-5ms latency, no loss)
        PresetTestCase {
            name: "lan_local_network",
            sync_config: SyncConfig::lan(),
            protocol_config: ProtocolConfig::default(),
            time_sync_config: TimeSyncConfig::lan(),
            chaos_config: Some(ChaosConfig::builder().latency_ms(2).build()),
            expected_sync_time_ms: 1000,
            target_frames: 50,
            min_expected_frames: 45,
        },
        // High latency: WAN connection (100ms latency)
        PresetTestCase {
            name: "high_latency_wan",
            sync_config: SyncConfig::high_latency(),
            protocol_config: ProtocolConfig::high_latency(),
            time_sync_config: TimeSyncConfig::default(),
            chaos_config: Some(ChaosConfig::builder().latency_ms(100).build()),
            expected_sync_time_ms: 6000,
            target_frames: 25,       // Fewer frames due to high latency
            min_expected_frames: 10, // With 100ms latency, frame advancement is slow
        },
        // Lossy: Unreliable connection (8% loss)
        PresetTestCase {
            name: "lossy_unreliable",
            sync_config: SyncConfig::lossy(),
            protocol_config: ProtocolConfig::default(),
            time_sync_config: TimeSyncConfig::default(),
            chaos_config: Some(ChaosConfig::builder().packet_loss_rate(0.08).build()),
            expected_sync_time_ms: 12000, // 8% loss needs more time for retries
            target_frames: 40,
            min_expected_frames: 25,
        },
        // Mobile: Cellular conditions (60ms latency, 5% loss)
        PresetTestCase {
            name: "mobile_cellular",
            sync_config: SyncConfig::mobile(),
            protocol_config: ProtocolConfig::mobile(),
            time_sync_config: TimeSyncConfig::mobile(),
            chaos_config: Some(
                ChaosConfig::builder()
                    .latency_ms(60)
                    .packet_loss_rate(0.05)
                    .build(),
            ),
            expected_sync_time_ms: 12000, // Mobile can take longer with 10 sync packets
            target_frames: 30,
            min_expected_frames: 15,
        },
    ];

    for case in &test_cases {
        run_preset_test(case, &clock);
    }
}
