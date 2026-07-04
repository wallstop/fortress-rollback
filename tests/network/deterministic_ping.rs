//! Deterministic RTT (ping) measurement tests.
//!
//! `NetworkStats::ping` must be derived from the injectable protocol clock
//! ([`ProtocolConfig::clock`]) so that (a) in-process simulations measure
//! *virtual* network latency rather than wall-clock scheduling noise, and
//! (b) the reported value is bit-identical across repeated runs of the same
//! virtual-time schedule. Both properties are prerequisites for deterministic
//! whole-mesh simulation testing.
//!
//! The schedule below is fully virtual: each iteration polls both sessions
//! and then advances the shared [`TestClock`] by one poll interval. A quality
//! report queued by one session is answered by its peer within the same
//! iteration (in-memory channel delivery), and the reply is processed by the
//! sender on the *next* iteration — exactly one clock advance later. The
//! round-trip time under virtual time is therefore exactly one poll interval.

// Allow test-specific patterns that are appropriate for test code
#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use crate::common::stubs::StubConfig;
use crate::common::{
    create_channel_pair, synchronize_sessions_deterministic, SyncConfig, TestClock,
    POLL_INTERVAL_DETERMINISTIC,
};
use fortress_rollback::{FortressError, PlayerHandle, PlayerType, ProtocolConfig, SessionBuilder};

/// Number of poll+advance iterations to run after synchronization.
///
/// Must be large enough that (a) several quality-report rounds complete
/// (default interval 200ms = 4 iterations at 50ms/step) and (b) more than
/// one virtual second elapses so `network_stats` reports instead of
/// returning `NotSynchronized` (its rate window needs `elapsed >= 1s`).
const MEASUREMENT_ITERATIONS: usize = 40;

/// Runs a fixed two-session schedule under fully virtual time and returns
/// the final RTT each session reports for its remote peer.
fn measure_ping_over_fixed_schedule() -> Result<(u128, u128), FortressError> {
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

    let ping1 = sess1.network_stats(PlayerHandle::new(1))?.ping;
    let ping2 = sess2.network_stats(PlayerHandle::new(0))?.ping;
    Ok((ping1, ping2))
}

/// With an injected clock, the reported RTT must equal the *virtual* time
/// between sending a quality report and processing its reply — exactly one
/// poll interval under this schedule — not the (near-zero) wall-clock time
/// the in-process channel actually takes.
#[test]
fn ping_with_injected_clock_reports_virtual_rtt() -> Result<(), FortressError> {
    let expected = POLL_INTERVAL_DETERMINISTIC.as_millis();
    let (ping1, ping2) = measure_ping_over_fixed_schedule()?;

    assert_eq!(
        ping1, expected,
        "session 1 ping must reflect virtual time ({expected}ms), got {ping1}ms"
    );
    assert_eq!(
        ping2, expected,
        "session 2 ping must reflect virtual time ({expected}ms), got {ping2}ms"
    );
    Ok(())
}

/// The same virtual-time schedule must produce bit-identical RTT values on
/// every run: ping measurement may not read any wall-clock source when a
/// clock is injected.
#[test]
fn ping_with_injected_clock_is_identical_across_runs() -> Result<(), FortressError> {
    let first = measure_ping_over_fixed_schedule()?;
    let second = measure_ping_over_fixed_schedule()?;

    assert_eq!(
        first, second,
        "identical virtual schedules must report identical ping values"
    );
    Ok(())
}
