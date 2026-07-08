//! Premise-asserted simulation census rows for specific distributed failure modes.

use super::harness::schedule::{
    BackgroundNoise, Schedule, ScheduleEvent, SimConfig, SCHEDULE_SCHEMA_VERSION,
};
use super::harness::{run, RunOptions};
use crate::common::sim_net::LinkPolicy;
use std::time::Duration;

fn delayed_two_peer_schedule() -> Schedule {
    let mut config = SimConfig::smoke(2);
    config.steps = 520;
    config.noise = BackgroundNoise::Clean;
    config.input_delay = 0;
    config.max_prediction = 2;

    let delayed = LinkPolicy {
        base_delay: Duration::from_millis(120),
        ..LinkPolicy::clean()
    };
    let initial_links = vec![(0, 1, delayed.clone()), (1, 0, delayed)];
    let heal_at = 260;

    Schedule {
        schema_version: SCHEDULE_SCHEMA_VERSION,
        seed: 0xCE45_0001,
        link_seed: 0xCE45_0002,
        config,
        initial_links,
        events: vec![(heal_at, ScheduleEvent::HealAll)],
        heal_at,
    }
}

/// M3 §6.4 census: when RTT is far larger than `max_prediction`, peers should
/// throttle by returning `Ok(empty)` rather than diverging or surfacing advance
/// errors. The permanent oracle checks liveness and byte-consistent state; this
/// row also asserts the stall counter is non-zero so the high-RTT premise fired.
#[test]
fn high_rtt_beyond_prediction_window_throttles_without_divergence() {
    let schedule = delayed_two_peer_schedule();

    let report = run(&schedule, &RunOptions::default());
    report.expect_pass(&schedule);

    let stalls: u64 = report
        .metrics
        .iter()
        .map(|metrics| metrics.stall_count)
        .sum();
    assert!(
        stalls > 0,
        "high-RTT census row must exercise prediction-window throttling"
    );

    let again = run(&schedule, &RunOptions::default());
    assert_eq!(
        report.trace_hash, again.trace_hash,
        "high-RTT census row must reproduce its exact trace"
    );
}
