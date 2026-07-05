//! Deterministic per-peer metrics ([`PeerMetrics`]) accounting tests.
//!
//! `P2PSession::peer_metrics` must report wire-exact, always-on per-peer
//! counters routed through the real receive path (poll → endpoint
//! `handle_message`), and — like ping measurement — must be bit-identical across
//! repeated runs of the same virtual-time schedule (no wall-clock leakage), a
//! prerequisite for deterministic whole-mesh simulation.
//!
//! Input-specific accounting (compression bytes, per-`MessageKind` tallies of a
//! single message) is asserted exactly at the unit level in
//! `network::protocol`; here the focus is the public accessor plus the
//! poll → endpoint routing that feeds the received-side counters.

// Allow test-specific patterns that are appropriate for test code
#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use crate::common::stubs::StubConfig;
use crate::common::{
    create_channel_pair, synchronize_sessions_deterministic, SyncConfig, TestClock,
    POLL_INTERVAL_DETERMINISTIC,
};
use fortress_rollback::{
    FortressError, PeerMetrics, PlayerHandle, PlayerType, ProtocolConfig, SessionBuilder,
};

/// Number of poll+advance iterations to run after synchronization — enough for
/// several quality-report rounds to flow in both directions.
const MEASUREMENT_ITERATIONS: usize = 40;

/// Runs a fixed two-session schedule under virtual time and returns the final
/// [`PeerMetrics`] each session reports for its single remote peer.
fn measure_peer_metrics_over_fixed_schedule() -> Result<(PeerMetrics, PeerMetrics), FortressError> {
    let clock = TestClock::new();
    let (s1, s2, a1, a2) = create_channel_pair();

    let protocol_config = |seed: u64| ProtocolConfig {
        clock: Some(clock.as_protocol_clock()),
        protocol_rng_seed: Some(seed),
        ..ProtocolConfig::default()
    };

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(101))
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .start_p2p_session(s1)?;

    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(202))
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .add_player(PlayerType::Remote(a1), PlayerHandle::new(0))?
        .start_p2p_session(s2)?;

    synchronize_sessions_deterministic(&mut sess1, &mut sess2, &clock, &SyncConfig::default())
        .expect("sessions should synchronize under virtual time");

    for _ in 0..MEASUREMENT_ITERATIONS {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        clock.advance(POLL_INTERVAL_DETERMINISTIC);
    }

    let m1 = sess1.peer_metrics(PlayerHandle::new(1))?;
    let m2 = sess2.peer_metrics(PlayerHandle::new(0))?;
    Ok((m1, m2))
}

/// End-to-end wiring: the receive path (poll → endpoint `handle_message`)
/// populates the per-peer counters read back through the public accessor, and
/// the send/receive per-kind tallies stay in lockstep with the packet counters.
#[test]
fn peer_metrics_account_for_bidirectional_traffic() -> Result<(), FortressError> {
    let (m1, m2) = measure_peer_metrics_over_fixed_schedule()?;

    for (label, m) in [("session 1", m1), ("session 2", m2)] {
        assert!(m.packets_sent > 0, "{label}: no packets sent");
        assert!(m.packets_received > 0, "{label}: no packets received");
        assert!(m.bytes_sent > 0, "{label}: no bytes sent");
        assert!(m.bytes_received > 0, "{label}: no bytes received");
        // One kind bucket per packet, in each direction — the invariant the
        // recording sites uphold by construction.
        assert_eq!(
            m.messages_sent_by_kind.total(),
            m.packets_sent,
            "{label}: sent kind total != packets_sent — {m:?}"
        );
        assert_eq!(
            m.messages_received_by_kind.total(),
            m.packets_received,
            "{label}: received kind total != packets_received — {m:?}"
        );
    }
    Ok(())
}

/// The same virtual-time schedule must produce bit-identical [`PeerMetrics`] on
/// every run: per-peer accounting may not read any wall-clock source.
#[test]
fn peer_metrics_are_identical_across_runs() -> Result<(), FortressError> {
    let first = measure_peer_metrics_over_fixed_schedule()?;
    let second = measure_peer_metrics_over_fixed_schedule()?;
    assert_eq!(
        first, second,
        "identical virtual schedules must report identical peer metrics"
    );
    Ok(())
}

/// The accessor validates the handle: a local player's own handle or an unknown
/// handle is an error, never a silently-wrong or zeroed snapshot. A remote
/// handle resolves even before any traffic (the counters are valid from
/// endpoint construction).
#[test]
fn peer_metrics_rejects_non_remote_handles() -> Result<(), FortressError> {
    let clock = TestClock::new();
    let (s1, _s2, _a1, a2) = create_channel_pair();
    let sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(ProtocolConfig {
            clock: Some(clock.as_protocol_clock()),
            protocol_rng_seed: Some(101),
            ..ProtocolConfig::default()
        })
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(a2), PlayerHandle::new(1))?
        .start_p2p_session(s1)?;

    // The local player's own handle is not a remote/spectator.
    assert!(sess1.peer_metrics(PlayerHandle::new(0)).is_err());
    // An unknown handle is rejected.
    assert!(sess1.peer_metrics(PlayerHandle::new(99)).is_err());
    // The remote handle resolves.
    assert!(sess1.peer_metrics(PlayerHandle::new(1)).is_ok());
    Ok(())
}
