//! Public-session protocol-v1 compatibility regressions.

#![allow(clippy::expect_used)]

use crate::common::stubs::StubConfig;
use crate::common::{create_channel_pair, TestClock, POLL_INTERVAL_DETERMINISTIC};
use fortress_rollback::{
    FortressError, FortressEvent, IncompatibleSessionReason, PlayerHandle, PlayerType,
    ProtocolConfig, SessionBuilder, SessionState,
};

fn protocol_config(clock: &TestClock, seed: u64) -> ProtocolConfig {
    ProtocolConfig {
        clock: Some(clock.as_protocol_clock()),
        protocol_rng_seed: Some(seed),
        ..ProtocolConfig::default()
    }
}

#[test]
fn mismatched_player_counts_fail_both_handshakes_once_without_timeout() -> Result<(), FortressError>
{
    let clock = TestClock::new();
    let (socket_a, socket_b, addr_a, addr_b) = create_channel_pair();
    let mut session_a = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock, 1))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr_b), PlayerHandle::new(1))?
        .start_p2p_session(socket_a)?;
    let mut session_b = SessionBuilder::<StubConfig>::new()
        .with_num_players(3)?
        .with_protocol_config(protocol_config(&clock, 2))
        .add_player(PlayerType::Remote(addr_a), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Local, PlayerHandle::new(2))?
        .start_p2p_session(socket_b)?;

    for _ in 0..6 {
        session_a.poll_remote_clients();
        session_b.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    let events_a: Vec<_> = session_a.events().collect();
    let events_b: Vec<_> = session_b.events().collect();
    assert_eq!(
        events_a
            .iter()
            .filter(|event| matches!(event, FortressEvent::IncompatibleSession { .. }))
            .count(),
        1
    );
    assert_eq!(
        events_b
            .iter()
            .filter(|event| matches!(event, FortressEvent::IncompatibleSession { .. }))
            .count(),
        1
    );
    assert!(events_a.iter().any(|event| matches!(
        event,
        FortressEvent::IncompatibleSession {
            addr,
            reason: IncompatibleSessionReason::NumPlayers { ours: 2, theirs: 3 },
        } if *addr == addr_b
    )));
    assert!(events_b.iter().any(|event| matches!(
        event,
        FortressEvent::IncompatibleSession {
            addr,
            reason: IncompatibleSessionReason::NumPlayers { ours: 3, theirs: 2 },
        } if *addr == addr_a
    )));
    assert!(events_a
        .iter()
        .all(|event| !matches!(event, FortressEvent::Synchronized { .. })));
    assert!(events_b
        .iter()
        .all(|event| !matches!(event, FortressEvent::Synchronized { .. })));
    assert_eq!(session_a.current_state(), SessionState::Synchronizing);
    assert_eq!(session_b.current_state(), SessionState::Synchronizing);

    clock.advance(std::time::Duration::from_secs(21));
    for _ in 0..3 {
        session_a.poll_remote_clients();
        session_b.poll_remote_clients();
    }
    assert!(session_a.events().all(|event| !matches!(
        event,
        FortressEvent::SyncTimeout { .. } | FortressEvent::Synchronized { .. }
    )));
    assert!(session_b.events().all(|event| !matches!(
        event,
        FortressEvent::SyncTimeout { .. } | FortressEvent::Synchronized { .. }
    )));
    assert_eq!(session_a.current_state(), SessionState::Synchronizing);
    assert_eq!(session_b.current_state(), SessionState::Synchronizing);

    Ok(())
}

#[test]
fn spectator_translates_host_config_mismatch_and_remains_synchronizing() -> Result<(), FortressError>
{
    let clock = TestClock::new();
    let (host_socket, spectator_socket, host_addr, spectator_addr) = create_channel_pair();
    let mut host = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock, 3))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Spectator(spectator_addr), PlayerHandle::new(2))?
        .start_p2p_session(host_socket)?;
    let mut spectator = SessionBuilder::<StubConfig>::new()
        .with_num_players(3)?
        .with_protocol_config(protocol_config(&clock, 4))
        .start_spectator_session(host_addr, spectator_socket)
        .expect("valid spectator configuration should start");

    for _ in 0..6 {
        spectator.poll_remote_clients();
        host.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    let events: Vec<_> = spectator.events().collect();
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, FortressEvent::IncompatibleSession { .. }))
            .count(),
        1
    );
    assert!(events.iter().any(|event| matches!(
        event,
        FortressEvent::IncompatibleSession {
            addr,
            reason: IncompatibleSessionReason::NumPlayers { ours: 3, theirs: 2 },
        } if *addr == host_addr
    )));
    assert!(events
        .iter()
        .all(|event| !matches!(event, FortressEvent::Synchronized { .. })));
    assert_eq!(spectator.current_state(), SessionState::Synchronizing);

    Ok(())
}
