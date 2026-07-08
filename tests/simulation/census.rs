//! Premise-asserted simulation census rows for specific distributed failure modes.

use super::harness::schedule::{
    BackgroundNoise, DropPolicy, Schedule, ScheduleEvent, SimConfig, SCHEDULE_SCHEMA_VERSION,
};
use super::harness::{peer_addr, run, PeerEventKey, PeerEventPayload, RunOptions, RunReport};
use crate::common::sim_net::LinkPolicy;
use fortress_rollback::{EventKind, PlayerHandle};
use std::time::Duration;

fn clean_initial_links(n: usize) -> Vec<(usize, usize, LinkPolicy)> {
    let mut initial_links = Vec::new();
    for from in 0..n {
        for to in 0..n {
            if from != to {
                initial_links.push((from, to, LinkPolicy::clean()));
            }
        }
    }
    initial_links
}

fn peer_event_payload_count(report: &RunReport, peer: usize, key: PeerEventKey) -> u64 {
    report
        .peer_event_payload_counts_by_peer
        .get(peer)
        .and_then(|counts| counts.get(&key))
        .copied()
        .unwrap_or(0)
}

fn addr_event_key(kind: EventKind, peer: usize) -> PeerEventKey {
    PeerEventKey {
        kind,
        payload: PeerEventPayload::Addr(peer_addr(peer)),
    }
}

fn peer_dropped_key(peer: usize) -> PeerEventKey {
    PeerEventKey {
        kind: EventKind::PeerDropped,
        payload: PeerEventPayload::PlayerAddr {
            handle: PlayerHandle::new(peer),
            addr: peer_addr(peer),
        },
    }
}

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

fn frozen_queue_network_blip_schedule() -> Schedule {
    let n = 3;
    let mut config = SimConfig::smoke(n);
    config.steps = 780;
    config.noise = BackgroundNoise::Clean;
    config.disconnect_behavior = DropPolicy::ContinueWithout;

    let drop_at = 140;
    let blip_start = 260;
    let blip_end = 335;
    let heal_at = blip_end;
    let mut events = vec![
        (drop_at, ScheduleEvent::GracefulRemove { by: 0, target: 2 }),
        (
            blip_start,
            ScheduleEvent::Block {
                from: 0,
                to: 1,
                blocked: true,
            },
        ),
        (
            blip_start,
            ScheduleEvent::Block {
                from: 1,
                to: 0,
                blocked: true,
            },
        ),
        (
            blip_end,
            ScheduleEvent::Block {
                from: 0,
                to: 1,
                blocked: false,
            },
        ),
        (
            blip_end,
            ScheduleEvent::Block {
                from: 1,
                to: 0,
                blocked: false,
            },
        ),
        (heal_at, ScheduleEvent::HealAll),
    ];
    events.sort_by_key(|(step, _)| *step);

    Schedule {
        schema_version: SCHEDULE_SCHEMA_VERSION,
        seed: 0xCE45_0010,
        link_seed: 0xCE45_0011,
        config,
        initial_links: clean_initial_links(n),
        events,
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

/// M3 §6.4 census: after a graceful drop freezes a departed slot, a survivor
/// link can still suffer a sub-timeout interruption and resume without wedging
/// the remaining mesh. This pins the combined frozen-queue +
/// `NetworkInterrupted`/`NetworkResumed` row with direct premise assertions:
/// traffic is actually blocked, the user-facing interruption surfaces fire, and
/// the oracle proves post-blip liveness plus byte-consistent survivor state.
#[test]
fn frozen_queue_survivors_resume_after_network_blip() {
    const SURVIVOR_A: usize = 0;
    const SURVIVOR_B: usize = 1;

    let schedule = frozen_queue_network_blip_schedule();

    let report = run(&schedule, &RunOptions::default());
    report.expect_pass(&schedule);

    assert!(
        report.net_stats.dropped_blocked > 0,
        "network-blip census row must drop traffic on the blocked survivor link: {:?}",
        report.net_stats
    );
    assert_eq!(
        report.recovered_within_b,
        Some(true),
        "network-blip census row must run and pass the bounded post-heal liveness oracle"
    );
    for (observer, kind, remote) in [
        (SURVIVOR_A, EventKind::NetworkInterrupted, SURVIVOR_B),
        (SURVIVOR_B, EventKind::NetworkInterrupted, SURVIVOR_A),
        (SURVIVOR_A, EventKind::NetworkResumed, SURVIVOR_B),
        (SURVIVOR_B, EventKind::NetworkResumed, SURVIVOR_A),
    ] {
        assert!(
            peer_event_payload_count(&report, observer, addr_event_key(kind, remote)) > 0,
            "survivor {observer} must observe {kind:?} for survivor {remote}: {:?}",
            report.peer_event_payload_counts_by_peer
        );
    }
    for observer in [SURVIVOR_A, SURVIVOR_B] {
        assert!(
            peer_event_payload_count(&report, observer, peer_dropped_key(2)) > 0,
            "survivor {observer} must observe PeerDropped for removed peer 2: {:?}",
            report.peer_event_payload_counts_by_peer
        );
    }
    let survivor_0_confirmed = report.final_confirmed.first().copied().unwrap_or(i32::MIN);
    let survivor_1_confirmed = report.final_confirmed.get(1).copied().unwrap_or(i32::MIN);
    assert!(
        survivor_0_confirmed > 400 && survivor_1_confirmed > 400,
        "survivors must keep confirming after the frozen-slot blip: {:?}",
        report.final_confirmed
    );

    let again = run(&schedule, &RunOptions::default());
    assert_eq!(
        report.trace_hash, again.trace_hash,
        "network-blip census row must reproduce its exact trace"
    );
}
