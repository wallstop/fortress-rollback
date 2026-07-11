//! Premise-asserted simulation census rows for specific distributed failure modes.

use super::harness::schedule::{
    BackgroundNoise, DropPolicy, SavePolicy, Schedule, ScheduleEvent, SimConfig,
    SCHEDULE_SCHEMA_VERSION,
};
use super::harness::{
    oracle::OracleFailure, peer_addr, run, PeerEventKey, PeerEventPayload, RunOptions, RunReport,
    TraceSessionState,
};
use crate::common::sim_net::{GilbertElliottPolicy, LinkPolicy};
use fortress_rollback::{EventKind, PlayerHandle};
use std::time::Duration;

const SPARSE_DROP_AT: u32 = 180;
const SPARSE_HEAL_AT: u32 = 300;
const MULTI_STAGGER_START: u32 = 150;
const MULTI_DROP_AT: u32 = 178;
const MULTI_HEAL_AT: u32 = 300;
const ASYMMETRIC_PARTITION_START: u32 = 140;
const ASYMMETRIC_PARTITION_HEAL: u32 = 450;
const REBIND_AT: u32 = 180;
const REBIND_HEAL_AT: u32 = 300;
const GILBERT_ELLIOTT_START: u32 = 140;
const GILBERT_ELLIOTT_HEAL: u32 = 420;
#[cfg(feature = "hot-join")]
const NPEER_HOT_JOIN_AT: u32 = 140;
#[cfg(feature = "hot-join")]
const NPEER_HOT_JOIN_HEAL: u32 = 220;
#[cfg(feature = "hot-join")]
const NPEER_OVERLAP_HOT_JOIN_AT: u32 = 140;
#[cfg(feature = "hot-join")]
const NPEER_HOT_JOIN_PARTITION_AT: u32 = 156;

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

fn blocked_drop_count(report: &RunReport, from: usize, to: usize) -> u64 {
    report
        .blocked_drops_by_link
        .get(&(from, to))
        .copied()
        .unwrap_or(0)
}

#[cfg(feature = "hot-join")]
fn probe_wire(report: &RunReport, from: usize, to: usize) -> &super::harness::PeerWireTotals {
    report
        .probe_peer_wire_by_link
        .get(&(from, to))
        .unwrap_or_else(|| panic!("missing probe wire ledger for {from}->{to}"))
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

fn nat_rebind_schedule(include_rebind: bool) -> Schedule {
    let n = 3;
    let mut config = SimConfig::smoke(n);
    config.steps = 700;
    config.noise = BackgroundNoise::Clean;

    let mut events = Vec::new();
    if include_rebind {
        events.push((REBIND_AT, ScheduleEvent::Rebind { peer: 2 }));
    }
    events.push((REBIND_HEAL_AT, ScheduleEvent::HealAll));

    Schedule {
        schema_version: SCHEDULE_SCHEMA_VERSION,
        seed: 0xCE45_00A7,
        link_seed: 0xCE45_00A8,
        config,
        initial_links: clean_initial_links(n),
        events,
        heal_at: REBIND_HEAL_AT,
    }
}

fn gilbert_elliott_schedule(include_correlated_loss: bool) -> Schedule {
    let n = 2;
    let mut config = SimConfig::smoke(n);
    config.steps = 720;
    config.noise = BackgroundNoise::Clean;
    config.input_delay = 0;

    let mut events = Vec::new();
    if include_correlated_loss {
        events.push((
            GILBERT_ELLIOTT_START,
            ScheduleEvent::SetLink {
                from: 0,
                to: 1,
                policy: LinkPolicy {
                    gilbert_elliott: Some(GilbertElliottPolicy {
                        good_to_bad: 0.04,
                        bad_to_good: 0.15,
                        good_drop_rate: 0.0,
                        bad_drop_rate: 0.75,
                    }),
                    ..LinkPolicy::clean()
                },
            },
        ));
    }
    events.push((GILBERT_ELLIOTT_HEAL, ScheduleEvent::HealAll));

    Schedule {
        schema_version: SCHEDULE_SCHEMA_VERSION,
        seed: 0xCE45_0E10,
        link_seed: 0xCE45_0E11,
        config,
        initial_links: clean_initial_links(n),
        events,
        heal_at: GILBERT_ELLIOTT_HEAL,
    }
}

#[cfg(feature = "hot-join")]
fn clean_npeer_hot_join_schedule(n: usize) -> Schedule {
    let hot_join_slot = n.saturating_sub(1);
    let mut config = SimConfig::smoke(n);
    config.steps = 900;
    config.noise = BackgroundNoise::Clean;
    config.input_delay = 0;
    config.save_mode = SavePolicy::EveryFrame;
    config.disconnect_behavior = DropPolicy::ContinueWithout;

    let events = vec![
        (
            NPEER_HOT_JOIN_AT,
            ScheduleEvent::HotJoin {
                slot: hot_join_slot,
            },
        ),
        (NPEER_HOT_JOIN_HEAL, ScheduleEvent::HealAll),
    ];

    Schedule {
        schema_version: SCHEDULE_SCHEMA_VERSION,
        seed: 0xCE45_F100,
        link_seed: 0xCE45_F101,
        config,
        initial_links: clean_initial_links(n),
        events,
        heal_at: NPEER_HOT_JOIN_HEAL,
    }
}

#[cfg(feature = "hot-join")]
fn partition_during_floor_round_hot_join_schedule(include_partition: bool) -> Schedule {
    let mut schedule = clean_npeer_hot_join_schedule(4);
    for (step, event) in &mut schedule.events {
        if matches!(event, ScheduleEvent::HotJoin { .. }) {
            *step = NPEER_OVERLAP_HOT_JOIN_AT;
        }
    }
    schedule.events.insert(
        1,
        (
            NPEER_OVERLAP_HOT_JOIN_AT,
            ScheduleEvent::PeerStall { peer: 3, steps: 9 },
        ),
    );
    schedule
        .events
        .push((154, ScheduleEvent::PeerStall { peer: 1, steps: 1 }));
    if include_partition {
        for other in [0, 2, 3] {
            schedule.events.push((
                NPEER_HOT_JOIN_PARTITION_AT,
                ScheduleEvent::Block {
                    from: 1,
                    to: other,
                    blocked: true,
                },
            ));
            schedule.events.push((
                NPEER_HOT_JOIN_PARTITION_AT,
                ScheduleEvent::Block {
                    from: other,
                    to: 1,
                    blocked: true,
                },
            ));
        }
    }
    schedule.events.sort_by_key(|(step, _)| *step);
    schedule
}

/// M3 §28.3(c) prerequisite: deterministic replacement-generation protocol
/// identities let clean N-peer hot-joins clear the survivor commit barrier.
/// N=3 exercises one survivor acknowledgement; N=4 exercises two and is the
/// matched control for the later partition-during-floor-round row.
#[cfg(feature = "hot-join")]
#[test]
fn clean_npeer_hot_join_clears_survivor_commit_barrier() {
    use fortress_rollback::MessageKind;

    for n in [3, 4] {
        let schedule = clean_npeer_hot_join_schedule(n);
        let first = run(&schedule, &RunOptions::default());
        let replay = run(&schedule, &RunOptions::default());
        first.expect_pass(&schedule);
        assert_eq!(first.trace_hash, replay.trace_hash, "N={n}");
        assert_eq!(first.verdict, replay.verdict, "N={n}");
        assert_eq!(first.net_stats, replay.net_stats, "N={n}");
        assert_eq!(first.metrics, replay.metrics, "N={n}");
        assert_eq!(first.peer_wire, replay.peer_wire, "N={n}");
        assert_eq!(first.final_confirmed, replay.final_confirmed, "N={n}");
        assert_eq!(first.violation_census, replay.violation_census, "N={n}");
        assert_eq!(first.recovered_within_b, Some(true), "N={n}");
        assert!(
            first.final_confirmed.iter().all(|frame| *frame > 800),
            "N={n}: {:?}",
            first.final_confirmed
        );
        assert!(first.violation_census.is_empty(), "N={n}");
        assert_eq!(first.net_stats.dropped_by_policy, 0, "N={n}");
        assert_eq!(first.net_stats.retransmit_delayed, 0, "N={n}");
        assert_eq!(first.net_stats.dropped_blocked, 0, "N={n}");
        assert_eq!(first.net_stats.dropped_unattached, 0, "N={n}");
        assert_eq!(first.net_stats.duplicated, 0, "N={n}");
        assert_eq!(first.net_stats.held, 0, "N={n}");
        assert_eq!(first.net_stats.gilbert_elliott_loss_events, 0, "N={n}");
        assert!(
            first.trace_tail.last().is_some_and(|snapshot| snapshot
                .session_states
                .iter()
                .all(|state| *state == TraceSessionState::Running)),
            "N={n}"
        );
        for survivor in 1..n.saturating_sub(1) {
            let survivor_wire = first
                .peer_wire
                .get(survivor)
                .expect("survivor wire ledger exists");
            assert!(
                survivor_wire.sent_by_kind(MessageKind::ReactivateSlotAck) > 0,
                "N={n}: survivor {survivor} must ack the replacement generation"
            );
        }
        for kind in [
            MessageKind::JoinRequest,
            MessageKind::StateSnapshot,
            MessageKind::ReactivateSlot,
            MessageKind::ReactivateSlotAck,
            MessageKind::JoinCommitted,
        ] {
            assert!(
                first
                    .peer_wire
                    .iter()
                    .map(|wire| wire.sent_by_kind(kind))
                    .sum::<u64>()
                    > 0,
                "N={n}: {kind:?} traffic must occur"
            );
        }
        assert_eq!(
            first
                .peer_wire
                .iter()
                .map(|wire| wire.sent_by_kind(MessageKind::JoinAborted))
                .sum::<u64>(),
            0,
            "N={n}"
        );
    }
}

#[cfg(feature = "hot-join")]
#[test]
fn partition_lands_during_open_floor_round_and_hot_join_then_recovers() {
    use fortress_rollback::telemetry::{ViolationKind, ViolationSeverity};
    use fortress_rollback::MessageKind;

    let control_schedule = partition_during_floor_round_hot_join_schedule(false);
    let before_options = RunOptions {
        probe_confirmed_at: Some(NPEER_HOT_JOIN_PARTITION_AT - 3),
        ..RunOptions::default()
    };
    let before = run(&control_schedule, &before_options);
    before.expect_pass(&control_schedule);

    let overlap_options = RunOptions {
        probe_confirmed_at: Some(NPEER_HOT_JOIN_PARTITION_AT - 1),
        ..RunOptions::default()
    };
    let control = run(&control_schedule, &overlap_options);
    let control_replay = run(&control_schedule, &overlap_options);
    control.expect_pass(&control_schedule);
    assert_eq!(control.trace_hash, control_replay.trace_hash);
    assert_eq!(
        control.probe_peer_wire_by_link,
        control_replay.probe_peer_wire_by_link
    );
    for to in [0, 2] {
        let before_sent = probe_wire(&before, 1, to).sent_by_kind(MessageKind::FloorRequest);
        let before_received = probe_wire(&before, 1, to).received_by_kind(MessageKind::FloorReply);
        let overlap_sent = probe_wire(&control, 1, to).sent_by_kind(MessageKind::FloorRequest);
        let overlap_received =
            probe_wire(&control, 1, to).received_by_kind(MessageKind::FloorReply);
        assert_eq!(
            before_sent, before_received,
            "peer1->{to} must have no older floor request outstanding"
        );
        assert_eq!(
            overlap_sent,
            before_sent + 1,
            "exactly one new peer1->{to} floor request must open before the partition"
        );
        assert_eq!(
            overlap_received, before_received,
            "the newly opened peer1->{to} floor request must still await its reply"
        );
    }
    for survivor in [1, 2] {
        assert!(
            probe_wire(&control, 0, survivor).sent_by_kind(MessageKind::ReactivateSlot)
                > probe_wire(&before, 0, survivor).sent_by_kind(MessageKind::ReactivateSlot),
            "coordinator must open the survivor barrier toward peer {survivor} during the overlap step"
        );
    }
    assert!(probe_wire(&control, 3, 0).sent_by_kind(MessageKind::JoinRequest) > 0);
    assert_eq!(
        probe_wire(&control, 1, 0).sent_by_kind(MessageKind::ReactivateSlotAck),
        0
    );
    assert_eq!(
        probe_wire(&control, 0, 3).sent_by_kind(MessageKind::JoinCommitted),
        0
    );
    assert_eq!(control.probe_confirmed.get(3), Some(&-1));

    let schedule = partition_during_floor_round_hot_join_schedule(true);
    let first = run(&schedule, &overlap_options);
    let replay = run(&schedule, &overlap_options);
    first.expect_pass(&schedule);
    assert_eq!(first.probe_confirmed, control.probe_confirmed);
    assert_eq!(
        first.probe_peer_wire_by_link,
        control.probe_peer_wire_by_link
    );
    assert_eq!(first.trace_hash, replay.trace_hash);
    assert_eq!(first.verdict, replay.verdict);
    assert_eq!(first.net_stats, replay.net_stats);
    assert_eq!(first.blocked_drops_by_link, replay.blocked_drops_by_link);
    assert_eq!(first.metrics, replay.metrics);
    assert_eq!(first.peer_wire, replay.peer_wire);
    assert_eq!(first.final_confirmed, replay.final_confirmed);
    assert_eq!(first.probe_confirmed, replay.probe_confirmed);
    assert_eq!(
        first.probe_peer_wire_by_link,
        replay.probe_peer_wire_by_link
    );
    assert_eq!(first.violation_census, replay.violation_census);

    let partition_links = [(1, 0), (0, 1), (1, 2), (2, 1), (1, 3), (3, 1)];
    let blocked_total: u64 = partition_links
        .iter()
        .map(|&(from, to)| {
            let drops = blocked_drop_count(&first, from, to);
            assert!(drops > 0, "partition link {from}->{to} must drop traffic");
            drops
        })
        .sum();
    assert_eq!(first.blocked_drops_by_link.len(), partition_links.len());
    assert_eq!(first.net_stats.dropped_blocked, blocked_total);
    assert_eq!(first.net_stats.dropped_by_policy, 0);
    assert_eq!(first.net_stats.retransmit_delayed, 0);
    assert_eq!(first.net_stats.dropped_unattached, 0);
    assert_eq!(first.net_stats.duplicated, 0);
    assert_eq!(first.net_stats.held, 0);
    assert_eq!(first.net_stats.gilbert_elliott_loss_events, 0);
    assert_eq!(first.recovered_within_b, Some(true));
    assert_eq!(first.violation_census.len(), 1);
    let (signature, count) = first
        .violation_census
        .first_key_value()
        .expect("one synchronization warning is censused");
    assert_eq!(signature.severity, ViolationSeverity::Warning);
    assert_eq!(signature.kind, ViolationKind::Synchronization);
    assert!(signature
        .location
        .starts_with("src/network/protocol/mod.rs:"));
    assert_eq!(signature.message_prefix, "Excessive sync retries");
    assert_eq!(*count, 1);
    assert!(first.final_confirmed.iter().all(|frame| *frame > 700));
    assert!(first.trace_tail.last().is_some_and(|snapshot| snapshot
        .session_states
        .iter()
        .all(|state| *state == TraceSessionState::Running)));
    assert!(
        probe_wire(&first, 1, 0).sent_by_kind(MessageKind::ReactivateSlotAck) == 0,
        "the partition prefix must still be inside the open commit barrier"
    );
    assert!(
        first
            .peer_wire
            .get(1)
            .is_some_and(|wire| wire.sent_by_kind(MessageKind::ReactivateSlotAck) > 0),
        "isolated survivor must ack after heal"
    );
    assert!(
        first
            .peer_wire
            .iter()
            .map(|wire| wire.sent_by_kind(MessageKind::JoinCommitted))
            .sum::<u64>()
            > 0
    );
    assert_eq!(
        first
            .peer_wire
            .iter()
            .map(|wire| wire.sent_by_kind(MessageKind::JoinAborted))
            .sum::<u64>(),
        0
    );
    assert!(first
        .load_game_state_observations
        .iter()
        .any(|observation| observation.peer == 3 && observation.step > NPEER_HOT_JOIN_HEAL));
}

#[test]
fn correlated_loss_recovers_and_exhibits_two_state_bursts() {
    let control_schedule = gilbert_elliott_schedule(false);
    let control = run(&control_schedule, &RunOptions::default());
    control.expect_pass(&control_schedule);
    assert_eq!(control.recovered_within_b, Some(true));
    assert_eq!(control.net_stats.dropped_by_policy, 0);
    assert_eq!(control.net_stats.gilbert_elliott_good_sends, 0);
    assert_eq!(control.net_stats.gilbert_elliott_bad_sends, 0);
    assert_eq!(control.net_stats.gilbert_elliott_loss_events, 0);

    let schedule = gilbert_elliott_schedule(true);
    let first = run(&schedule, &RunOptions::default());
    let replay = run(&schedule, &RunOptions::default());
    first.expect_pass(&schedule);
    assert_eq!(first.trace_hash, replay.trace_hash);
    assert_eq!(first.verdict, replay.verdict);
    assert_eq!(first.net_stats, replay.net_stats);
    assert_eq!(first.metrics, replay.metrics);
    assert_eq!(first.final_confirmed, replay.final_confirmed);
    assert_eq!(first.violation_census, replay.violation_census);
    assert_eq!(first.recovered_within_b, Some(true));
    assert!(first.final_confirmed.iter().all(|frame| *frame > 100));

    let stats = first.net_stats;
    assert!(stats.gilbert_elliott_good_sends > 0, "{stats:?}");
    assert!(stats.gilbert_elliott_bad_sends > 0, "{stats:?}");
    assert!(stats.gilbert_elliott_good_to_bad >= 2, "{stats:?}");
    assert!(stats.gilbert_elliott_bad_to_good >= 2, "{stats:?}");
    assert!(stats.gilbert_elliott_loss_events > 0, "{stats:?}");
    assert!(stats.gilbert_elliott_max_loss_run >= 4, "{stats:?}");
    assert_eq!(
        stats.dropped_by_policy, stats.gilbert_elliott_loss_events,
        "GE is the schedule's only unreliable loss source"
    );
}

#[test]
fn nat_rebind_is_observable_and_fails_closed_deterministically() {
    let control = nat_rebind_schedule(false);
    run(&control, &RunOptions::default()).expect_pass(&control);

    let schedule = nat_rebind_schedule(true);
    let first = run(&schedule, &RunOptions::default());
    let replay = run(&schedule, &RunOptions::default());

    assert_eq!(first.trace_hash, replay.trace_hash);
    assert_eq!(first.verdict, replay.verdict);
    assert_eq!(first.net_stats, replay.net_stats);
    assert_eq!(first.metrics, replay.metrics);
    assert_eq!(first.violation_census, replay.violation_census);
    assert_eq!(first.recovered_within_b, Some(false));
    assert_eq!(
        first.verdict.failures.len(),
        6,
        "the exact failure multiset is two liveness failures per live peer: {:?}",
        first.verdict.failures,
    );
    let end_progress_failures = first
        .verdict
        .failures
        .iter()
        .filter(|failure| matches!(failure, OracleFailure::EndProgress { .. }))
        .count();
    let post_heal_failures = first
        .verdict
        .failures
        .iter()
        .filter(|failure| matches!(failure, OracleFailure::PostHealLiveness { .. }))
        .count();
    assert_eq!(
        (end_progress_failures, post_heal_failures),
        (3, 3),
        "NAT rebinding must produce only the pinned end-state and heal-anchored liveness failures: {:?}",
        first.verdict.failures,
    );
    assert!(
        first.verdict.failures.iter().all(|failure| matches!(
            failure,
            OracleFailure::EndProgress { .. } | OracleFailure::PostHealLiveness { .. }
        )),
        "NAT rebinding must not introduce runner/session or consistency failures: {:?}",
        first.verdict.failures,
    );
    assert!(
        first.net_stats.dropped_unattached > 0,
        "survivors must keep sending to the abandoned canonical address"
    );
    for peer in 0..2 {
        assert!(
            first.metrics[peer].unknown_source_packets > 0,
            "survivor {peer} must observe decoded traffic from the rebound source"
        );
    }
    assert_eq!(
        first.metrics[2].unknown_source_packets, 0,
        "the rebound peer receives no traffic from an unknown source"
    );
    let warning_count: u64 = first
        .violation_census
        .iter()
        .filter(|(signature, _)| signature.message_prefix.contains("unknown source address"))
        .map(|(_, count)| *count)
        .sum();
    assert_eq!(
        warning_count, 2,
        "the two receiving sessions must emit two lifetime-bounded warnings in aggregate"
    );
}

#[test]
fn rebind_after_runtime_retirement_is_a_deterministic_noop() {
    let mut schedule = nat_rebind_schedule(false);
    schedule.events = vec![
        (160, ScheduleEvent::GracefulRemove { by: 0, target: 2 }),
        (180, ScheduleEvent::Rebind { peer: 2 }),
        (REBIND_HEAL_AT, ScheduleEvent::HealAll),
    ];
    let mut control = schedule.clone();
    control
        .events
        .retain(|(_, event)| !matches!(event, ScheduleEvent::Rebind { .. }));

    let first = run(&schedule, &RunOptions::default());
    let replay = run(&schedule, &RunOptions::default());
    let without_rebind = run(&control, &RunOptions::default());
    assert_eq!(first.trace_hash, replay.trace_hash);
    assert_eq!(first.verdict, replay.verdict);
    assert_eq!(first.net_stats, replay.net_stats);
    assert_eq!(first.verdict, without_rebind.verdict);
    assert_eq!(first.net_stats, without_rebind.net_stats);
    assert_eq!(first.metrics, without_rebind.metrics);
    assert!(
        first.verdict.failures.iter().all(|failure| !matches!(
            failure,
            OracleFailure::SessionError { operation, .. }
                if operation.starts_with("rebind_")
        )),
        "a missed Rebind fire-time precondition must not become a runner failure: {:?}",
        first.verdict.failures,
    );
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

fn sparse_graceful_drop_rollback_schedule() -> Schedule {
    let n = 3;
    let mut config = SimConfig::smoke(n);
    config.steps = 680;
    config.noise = BackgroundNoise::Clean;
    config.disconnect_behavior = DropPolicy::ContinueWithout;
    config.save_mode = SavePolicy::Sparse;

    let mut events = vec![
        (
            SPARSE_DROP_AT,
            ScheduleEvent::GracefulRemove { by: 0, target: 2 },
        ),
        (SPARSE_HEAL_AT, ScheduleEvent::HealAll),
    ];
    events.sort_by_key(|(step, _)| *step);

    Schedule {
        schema_version: SCHEDULE_SCHEMA_VERSION,
        seed: 0xCE45_0020,
        link_seed: 0xCE45_0021,
        config,
        initial_links: clean_initial_links(n),
        events,
        heal_at: SPARSE_HEAL_AT,
    }
}

fn same_step_multi_drop_schedule() -> Schedule {
    let n = 4;
    let mut config = SimConfig::smoke(n);
    config.steps = 760;
    config.noise = BackgroundNoise::Clean;
    config.disconnect_behavior = DropPolicy::ContinueWithout;

    let mut events = vec![
        (
            MULTI_STAGGER_START,
            ScheduleEvent::Block {
                from: 2,
                to: 1,
                blocked: true,
            },
        ),
        (
            MULTI_STAGGER_START,
            ScheduleEvent::Block {
                from: 3,
                to: 0,
                blocked: true,
            },
        ),
        (
            MULTI_DROP_AT,
            ScheduleEvent::GracefulRemove { by: 0, target: 2 },
        ),
        (
            MULTI_DROP_AT,
            ScheduleEvent::GracefulRemove { by: 1, target: 3 },
        ),
        (MULTI_HEAL_AT, ScheduleEvent::HealAll),
    ];
    events.sort_by_key(|(step, _)| *step);

    Schedule {
        schema_version: SCHEDULE_SCHEMA_VERSION,
        seed: 0xCE45_0030,
        link_seed: 0xCE45_0031,
        config,
        initial_links: clean_initial_links(n),
        events,
        heal_at: MULTI_HEAL_AT,
    }
}

fn asymmetric_partition_schedule() -> Schedule {
    let n = 5;
    let mut config = SimConfig::smoke(n);
    config.steps = 900;
    config.noise = BackgroundNoise::Clean;
    config.disconnect_behavior = DropPolicy::ContinueWithout;

    let events = vec![
        (
            ASYMMETRIC_PARTITION_START,
            ScheduleEvent::Block {
                from: 4,
                to: 0,
                blocked: true,
            },
        ),
        (ASYMMETRIC_PARTITION_HEAL, ScheduleEvent::HealAll),
    ];

    Schedule {
        schema_version: SCHEDULE_SCHEMA_VERSION,
        seed: 0xCE45_0040,
        link_seed: 0xCE45_0041,
        config,
        initial_links: clean_initial_links(n),
        events,
        heal_at: ASYMMETRIC_PARTITION_HEAL,
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

/// M3 §6.4 census: sparse saving must survive the graceful-drop rollback path.
/// The row pins the test-only `SaveMode` schedule axis and proves a survivor
/// loaded prior state after the graceful remove, then lets the permanent oracle
/// check byte-consistent survivor state and freeze-frame agreement for the
/// retired slot.
#[test]
fn sparse_save_mode_survives_graceful_drop_rollback() {
    const SURVIVOR_A: usize = 0;
    const SURVIVOR_B: usize = 1;

    let schedule = sparse_graceful_drop_rollback_schedule();
    assert_eq!(
        schedule.config.save_mode,
        SavePolicy::Sparse,
        "census row must explicitly exercise sparse saving"
    );

    let report = run(&schedule, &RunOptions::default());
    report.expect_pass(&schedule);

    let survivor_post_drop_loads: Vec<_> = report
        .load_game_state_observations
        .iter()
        .filter(|load| load.step >= SPARSE_DROP_AT && [SURVIVOR_A, SURVIVOR_B].contains(&load.peer))
        .collect();
    assert!(
        survivor_post_drop_loads
            .iter()
            .any(|load| load.frame < SPARSE_DROP_AT as i32),
        "sparse graceful-drop row must observe survivor pre-drop LoadGameState after the drop: {:?}",
        report.load_game_state_observations
    );
    let rollbacks: u64 = report
        .metrics
        .iter()
        .map(|metrics| metrics.rollback_count)
        .sum();
    let resimulated: u64 = report
        .metrics
        .iter()
        .map(|metrics| metrics.resimulated_frames)
        .sum();
    assert!(
        rollbacks > 0 && resimulated > 0,
        "sparse graceful-drop row must exercise disconnect rollback repair: \
         rollbacks={rollbacks}, resimulated={resimulated}"
    );
    assert_eq!(
        report.recovered_within_b,
        Some(true),
        "sparse graceful-drop row must run and pass bounded post-heal liveness"
    );
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
        "sparse survivors must keep confirming after the disconnect rollback: {:?}",
        report.final_confirmed
    );

    let again = run(&schedule, &RunOptions::default());
    assert_eq!(
        report.trace_hash, again.trace_hash,
        "sparse graceful-drop row must reproduce its exact trace"
    );
}

/// M3 §6.4 census: two peers can drop in the same poll window after asymmetric
/// survivor receipt loss, and the live mesh still converges. This pins the
/// whole-mesh counterpart to the lower-level multi-drop guard: the schedule
/// proves both intended blocked links dropped traffic, both survivors loaded
/// prior state after the drops, and both survivors learned both graceful drops.
#[test]
fn same_step_multi_drop_after_asymmetric_block_converges() {
    const SURVIVORS: [usize; 2] = [0, 1];
    const DROPPED: [usize; 2] = [2, 3];

    let schedule = same_step_multi_drop_schedule();

    let report = run(&schedule, &RunOptions::default());
    report.expect_pass(&schedule);

    for (from, to) in [(2, 1), (3, 0)] {
        assert!(
            blocked_drop_count(&report, from, to) > 0,
            "same-step multi-drop row must drop traffic on intended blocked link {from}->{to}: {:?}",
            report.blocked_drops_by_link
        );
    }
    let survivor_post_drop_loads: Vec<_> = report
        .load_game_state_observations
        .iter()
        .filter(|load| load.step >= MULTI_DROP_AT && SURVIVORS.contains(&load.peer))
        .collect();
    for survivor in SURVIVORS {
        assert!(
            survivor_post_drop_loads
                .iter()
                .any(|load| load.peer == survivor && load.frame < MULTI_DROP_AT as i32),
            "survivor {survivor} must load pre-drop state after same-step drops: {:?}",
            report.load_game_state_observations
        );
    }
    let rollbacks: u64 = report
        .metrics
        .iter()
        .map(|metrics| metrics.rollback_count)
        .sum();
    let resimulated: u64 = report
        .metrics
        .iter()
        .map(|metrics| metrics.resimulated_frames)
        .sum();
    assert!(
        rollbacks > 0 && resimulated > 0,
        "same-step multi-drop row must exercise disconnect rollback: \
         rollbacks={rollbacks}, resimulated={resimulated}"
    );
    assert_eq!(
        report.recovered_within_b,
        Some(true),
        "same-step multi-drop row must run and pass bounded post-heal liveness"
    );
    for observer in SURVIVORS {
        for dropped in DROPPED {
            assert!(
                peer_event_payload_count(&report, observer, peer_dropped_key(dropped)) > 0,
                "survivor {observer} must observe PeerDropped for removed peer {dropped}: {:?}",
                report.peer_event_payload_counts_by_peer
            );
        }
    }
    for survivor in SURVIVORS {
        let confirmed = report
            .final_confirmed
            .get(survivor)
            .copied()
            .unwrap_or(i32::MIN);
        assert!(
            confirmed > 400,
            "survivor {survivor} must keep confirming after same-step drops: {:?}",
            report.final_confirmed
        );
    }

    let again = run(&schedule, &RunOptions::default());
    assert_eq!(
        report.trace_hash, again.trace_hash,
        "same-step multi-drop row must reproduce its exact trace"
    );
}

/// M3 §28.3 known-red census: a one-way partition lasts beyond the disconnect
/// timeout, so peer 0 drops peer 4 even though the reverse network direction
/// is never blocked.
/// This reproduces D14 at whole-mesh scale: the unilateral graceful freeze
/// rewrites already-confirmed history. The four-peer majority and one-peer
/// island both keep advancing, but their confirmed histories fork permanently.
/// Flip this to a green convergence regression when M5's coordinated
/// prepare/backfill/commit barrier lands.
#[test]
#[ignore = "known D14 confirmed-history rewrite under asymmetric partition"]
fn one_way_minority_partition_rewrites_confirmed_history_known_defect() {
    let schedule = asymmetric_partition_schedule();

    let report = run(&schedule, &RunOptions::default());

    assert!(
        blocked_drop_count(&report, 4, 0) > 0,
        "asymmetric-partition row must drop traffic on blocked link 4->0: {:?}",
        report.blocked_drops_by_link
    );
    assert_eq!(
        blocked_drop_count(&report, 0, 4),
        0,
        "reverse network direction 0->4 must never be blocked: {:?}",
        report.blocked_drops_by_link
    );
    for to in 1..=3 {
        assert_eq!(
            blocked_drop_count(&report, 4, to),
            0,
            "other 4->majority network directions must remain unblocked ({to}): {:?}",
            report.blocked_drops_by_link
        );
    }
    assert_eq!(
        report.net_stats.dropped_blocked,
        blocked_drop_count(&report, 4, 0),
        "4->0 must be the schedule's only blocked link: {:?}",
        report.blocked_drops_by_link
    );
    assert!(
        peer_event_payload_count(&report, 0, peer_dropped_key(4)) > 0,
        "peer 0 must time out and gracefully drop peer 4: {:?}",
        report.peer_event_payload_counts_by_peer
    );
    assert!(
        peer_event_payload_count(&report, 0, addr_event_key(EventKind::Disconnected, 4)) > 0,
        "peer 0 must surface peer 4's terminal disconnect: {:?}",
        report.peer_event_payload_counts_by_peer
    );
    assert!(
        peer_event_payload_count(&report, 0, addr_event_key(EventKind::NetworkInterrupted, 4)) > 0,
        "peer 0 must directly observe peer 4's one-way silence: {:?}",
        report.peer_event_payload_counts_by_peer
    );
    for observer in 1..=3 {
        assert!(
            peer_event_payload_count(&report, observer, peer_dropped_key(4)) > 0,
            "majority peer {observer} must learn peer 4's drop through gossip: {:?}",
            report.peer_event_payload_counts_by_peer
        );
        assert_eq!(
            peer_event_payload_count(
                &report,
                observer,
                addr_event_key(EventKind::NetworkInterrupted, 4)
            ),
            0,
            "majority peer {observer} must learn the drop before its own timeout: {:?}",
            report.peer_event_payload_counts_by_peer
        );
        assert!(
            peer_event_payload_count(
                &report,
                observer,
                addr_event_key(EventKind::Disconnected, 4)
            ) > 0,
            "majority peer {observer} must surface peer 4's gossiped disconnect: {:?}",
            report.peer_event_payload_counts_by_peer
        );
    }
    for observer in 0..=3 {
        for retained in 0..=3 {
            if observer == retained {
                continue;
            }
            assert_eq!(
                peer_event_payload_count(&report, observer, peer_dropped_key(retained)),
                0,
                "majority peer {observer} must retain majority peer {retained}: {:?}",
                report.peer_event_payload_counts_by_peer
            );
        }
    }
    for dropped in 0..=3 {
        assert!(
            peer_event_payload_count(&report, 4, peer_dropped_key(dropped)) > 0,
            "minority peer 4 must eventually drop majority peer {dropped}: {:?}",
            report.peer_event_payload_counts_by_peer
        );
        assert!(
            peer_event_payload_count(
                &report,
                4,
                addr_event_key(EventKind::NetworkInterrupted, dropped)
            ) > 0,
            "minority peer 4 must observe majority peer {dropped}'s silence: {:?}",
            report.peer_event_payload_counts_by_peer
        );
        assert!(
            peer_event_payload_count(&report, 4, addr_event_key(EventKind::Disconnected, dropped))
                > 0,
            "minority peer 4 must surface majority peer {dropped}'s disconnect: {:?}",
            report.peer_event_payload_counts_by_peer
        );
    }
    assert_eq!(
        report.recovered_within_b,
        Some(true),
        "both sides of the intentional ContinueWithout fork must keep advancing"
    );
    let confirmed_rewrites: Vec<_> = report
        .verdict
        .failures
        .iter()
        .filter_map(|failure| match failure {
            OracleFailure::ConfirmedInputDivergence {
                peer,
                first_author,
                expected,
                actual,
                ..
            } => Some((*peer, *first_author, expected, actual)),
            _ => None,
        })
        .collect();
    assert!(
        !confirmed_rewrites.is_empty()
            && confirmed_rewrites.iter().all(
                |(peer, first_author, expected, actual)| {
                    *peer < 4
                        && *first_author == 4
                        && expected.len() == 5
                        && actual.len() == 5
                        && expected.get(..4) == actual.get(..4)
                        && expected.get(4) != actual.get(4)
                }
            ),
        "every confirmed divergence must be D14 rewriting only peer 4's input at the majority: {:?}",
        report.verdict.failures
    );
    assert!(
        report
            .final_confirmed
            .iter()
            .all(|confirmed| *confirmed > 200),
        "the majority and minority island must remain available after the fork: {:?}",
        report.final_confirmed
    );
    let final_snapshot = report
        .trace_tail
        .last()
        .expect("a non-empty run must retain a final trace snapshot");
    assert!(
        final_snapshot
            .session_states
            .iter()
            .all(|state| *state == TraceSessionState::Running),
        "both sides must finish Running rather than wedged: {:?}",
        final_snapshot.session_states
    );
    let majority_state = final_snapshot
        .game_states
        .get(1)
        .expect("five-player trace must contain peer 1 state");
    assert!(
        final_snapshot
            .game_states
            .iter()
            .skip(1)
            .take(3)
            .all(|state| state == majority_state),
        "gossip-driven majority peers 1..=3 must converge on one final state: {:?}",
        final_snapshot.game_states
    );
    assert!(
        report.verdict.failures.iter().all(|failure| matches!(
            failure,
            OracleFailure::ConfirmedInputDivergence { .. }
                | OracleFailure::StateDivergence { .. }
                | OracleFailure::InbandDesyncDetected { .. }
                | OracleFailure::ChecksumMismatchMetric { .. }
        )),
        "known D14 divergence must be the complete failure surface: {:?}",
        report.verdict.failures
    );
    assert!(
        report.verdict.failures.iter().any(|failure| matches!(
            failure,
            OracleFailure::StateDivergence {
                peer: 4,
                first_author: 0,
                ..
            }
        )),
        "minority peer 4 must diverge in state from canonical majority peer 0: {:?}",
        report.verdict.failures
    );
    assert!(
        report
            .verdict
            .failures
            .iter()
            .filter_map(|failure| match failure {
                OracleFailure::StateDivergence {
                    peer, first_author, ..
                } => Some((*peer, *first_author)),
                _ => None,
            })
            .all(|pair| pair == (4, 0)),
        "all state divergence must be minority peer 4 versus canonical majority peer 0: {:?}",
        report.verdict.failures
    );

    let again = run(&schedule, &RunOptions::default());
    assert_eq!(
        report.trace_hash, again.trace_hash,
        "asymmetric-partition row must reproduce its exact trace"
    );
    assert_eq!(report.final_confirmed, again.final_confirmed);
    assert_eq!(report.blocked_drops_by_link, again.blocked_drops_by_link);
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
