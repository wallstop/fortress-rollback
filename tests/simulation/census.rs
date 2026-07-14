//! Premise-asserted simulation census rows for specific distributed failure modes.

use super::harness::schedule::{
    AppModel, BackgroundNoise, DropPolicy, SavePolicy, Schedule, ScheduleEvent, SimConfig,
    SCHEDULE_SCHEMA_VERSION,
};
use super::harness::{
    oracle::{OracleFailure, ViolationSource},
    peer_addr, run, run_with_input, HostileGossipMode, HostileGossipOptions, PeerEventKey,
    PeerEventPayload, RunOptions, RunReport, TraceSessionState, WideStubInput,
};
use crate::common::sim_net::{
    BandwidthPolicy, FragmentationPolicy, GilbertElliottPolicy, LinkPolicy,
};
use fortress_rollback::{
    telemetry::{ViolationKind, ViolationSeverity},
    EventKind, PlayerHandle, SessionState,
};
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
const INPUT_WINDOW_FAULT_START: u32 = 160;
const FRAGMENTATION_BACKLOG_START: u32 = 140;
const FRAGMENTATION_BACKLOG_RELEASE: u32 = 204;
const FRAGMENTATION_HEAL: u32 = 320;
const BANDWIDTH_START: u32 = 140;
const BANDWIDTH_HEAL: u32 = 240;
const POLLCAP_RELEASE: u32 = 220;
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

fn pollcap_schedule(plant_target_backlog: bool, steps: u32) -> Schedule {
    let n = 16;
    let mut config = SimConfig::smoke(n);
    config.steps = steps;
    config.noise = BackgroundNoise::Clean;

    let mut events = Vec::new();
    if plant_target_backlog {
        for from in 1..n {
            events.push((
                0,
                ScheduleEvent::Hold {
                    from,
                    to: 0,
                    holding: true,
                },
            ));
        }
    }
    events.push((POLLCAP_RELEASE, ScheduleEvent::HealAll));

    Schedule {
        schema_version: SCHEDULE_SCHEMA_VERSION,
        seed: 0xCE45_CA90,
        link_seed: 0xCE45_CA91,
        config,
        initial_links: clean_initial_links(n),
        events,
        heal_at: POLLCAP_RELEASE,
    }
}

fn assert_all_peer_pairs_synchronized(report: &RunReport) {
    let n = report.first_synchronized_step.len();
    assert_eq!(n, 16);
    for local in 0..n {
        assert_eq!(report.first_synchronized_step[local].len(), n);
        for remote in 0..n {
            if local == remote {
                assert_eq!(report.first_synchronized_step[local][remote], None);
            } else {
                assert!(
                    report.first_synchronized_step[local][remote].is_some(),
                    "missing Synchronized event for {local}->{remote}"
                );
            }
        }
    }
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

fn bandwidth_queue_schedule(include_constraint: bool) -> Schedule {
    let n = 2;
    let mut config = SimConfig::smoke(n);
    config.steps = 650;
    config.noise = BackgroundNoise::Clean;
    config.input_delay = 0;

    let mut events = Vec::new();
    if include_constraint {
        events.push((
            BANDWIDTH_START,
            ScheduleEvent::SetLink {
                from: 0,
                to: 1,
                policy: LinkPolicy {
                    bandwidth: Some(BandwidthPolicy {
                        rate_bytes_per_second: 1_000,
                        burst_bytes: 512,
                        queue_capacity_bytes: 2_048,
                    }),
                    ..LinkPolicy::clean()
                },
            },
        ));
    }
    events.push((BANDWIDTH_HEAL, ScheduleEvent::HealAll));

    Schedule {
        schema_version: SCHEDULE_SCHEMA_VERSION,
        seed: 0xCE45_B10A,
        link_seed: 0xCE45_B10B,
        config,
        initial_links: clean_initial_links(n),
        events,
        heal_at: BANDWIDTH_HEAL,
    }
}

fn scale_bandwidth_fragmentation_schedule(
    app_model: AppModel,
    include_bandwidth: bool,
    fragment_drop_rate: f64,
) -> Schedule {
    let mut schedule = fragmentation_schedule(fragment_drop_rate);
    schedule.config.app_model = app_model;

    let constrained = LinkPolicy {
        fragmentation: Some(FragmentationPolicy { fragment_drop_rate }),
        bandwidth: include_bandwidth.then_some(BandwidthPolicy {
            rate_bytes_per_second: 8_750,
            burst_bytes: 8_192,
            queue_capacity_bytes: 32_768,
        }),
        ..LinkPolicy::clean()
    };
    let release_policy = schedule
        .events
        .iter_mut()
        .find_map(|(step, event)| match event {
            ScheduleEvent::SetLink { from, to, policy }
                if *step == FRAGMENTATION_BACKLOG_RELEASE && (*from, *to) == (0, 1) =>
            {
                Some(policy)
            },
            _ => None,
        })
        .expect("fragmentation schedule has the 0->1 release policy");
    *release_policy = constrained;
    let block_step = schedule
        .events
        .iter_mut()
        .find_map(|(step, event)| match event {
            ScheduleEvent::Block { from, to, blocked }
                if (*from, *to, *blocked) == (0, 1, true) =>
            {
                Some(step)
            },
            _ => None,
        })
        .expect("fragmentation schedule has the 0->1 block event");
    // 44 pending 32-byte inputs exceed the 1,472-byte fragmentation threshold
    // while staying far below the 128-entry protocol redundancy bound.
    *block_step = FRAGMENTATION_BACKLOG_RELEASE - 44;

    schedule
}

fn fragmentation_schedule(fragment_drop_rate: f64) -> Schedule {
    let n = 16;
    let mut config = SimConfig::smoke(n);
    config.steps = 700;
    config.noise = BackgroundNoise::Clean;
    config.input_delay = 0;
    config.max_prediction = 120;

    let fragmentation = LinkPolicy {
        fragmentation: Some(FragmentationPolicy { fragment_drop_rate }),
        ..LinkPolicy::clean()
    };
    let events = vec![
        (
            FRAGMENTATION_BACKLOG_START,
            ScheduleEvent::Block {
                from: 0,
                to: 1,
                blocked: true,
            },
        ),
        (
            FRAGMENTATION_BACKLOG_RELEASE,
            ScheduleEvent::SetLink {
                from: 0,
                to: 1,
                policy: fragmentation,
            },
        ),
        (
            FRAGMENTATION_BACKLOG_RELEASE,
            ScheduleEvent::Block {
                from: 0,
                to: 1,
                blocked: false,
            },
        ),
        (FRAGMENTATION_HEAL, ScheduleEvent::HealAll),
    ];

    Schedule {
        schema_version: SCHEDULE_SCHEMA_VERSION,
        seed: 0xCE45_F12A,
        link_seed: 0xCE45_F12B,
        config,
        initial_links: clean_initial_links(n),
        events,
        heal_at: FRAGMENTATION_HEAL,
    }
}

fn input_window_boundary_schedule(heal_at: u32, include_loss: bool) -> Schedule {
    let n = 2;
    let mut config = SimConfig::smoke(n);
    config.steps = 900;
    config.step_dt_ms = 8;
    config.noise = BackgroundNoise::Clean;
    config.input_delay = 0;
    config.max_prediction = 127;

    let policy = if include_loss {
        LinkPolicy {
            gilbert_elliott: Some(GilbertElliottPolicy {
                good_to_bad: 1.0,
                bad_to_good: 0.0,
                good_drop_rate: 0.0,
                bad_drop_rate: 1.0,
            }),
            ..LinkPolicy::clean()
        }
    } else {
        LinkPolicy::clean()
    };

    Schedule {
        schema_version: SCHEDULE_SCHEMA_VERSION,
        seed: 0xCE45_1280,
        link_seed: 0xCE45_1281,
        config,
        initial_links: clean_initial_links(n),
        events: vec![
            (
                INPUT_WINDOW_FAULT_START,
                ScheduleEvent::SetLink {
                    from: 0,
                    to: 1,
                    policy,
                },
            ),
            (heal_at, ScheduleEvent::HealAll),
        ],
        heal_at,
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
fn constrained_uplink_builds_queue_then_tail_drops_and_recovers() {
    let control_schedule = bandwidth_queue_schedule(false);
    let control = run(&control_schedule, &RunOptions::default());
    control.expect_pass(&control_schedule);
    assert_eq!(control.net_stats.bandwidth_admitted_datagrams, 0);
    assert_eq!(control.net_stats.bandwidth_queued_datagrams, 0);
    assert_eq!(control.net_stats.bandwidth_tail_drops, 0);

    let schedule = bandwidth_queue_schedule(true);
    let first = run(&schedule, &RunOptions::default());
    let replay = run(&schedule, &RunOptions::default());
    first.expect_pass(&schedule);
    assert_eq!(first.trace_hash, replay.trace_hash);
    assert_eq!(first.progress_samples, replay.progress_samples);
    assert!(!first.progress_samples.is_empty());
    assert_eq!(first.net_stats, replay.net_stats);
    assert_eq!(first.link_stats_by_link, replay.link_stats_by_link);
    assert_eq!(first.recovered_within_b, Some(true));

    let stats = first.net_stats;
    assert!(stats.bandwidth_admitted_datagrams > 0, "{stats:?}");
    assert!(stats.bandwidth_queued_datagrams > 0, "{stats:?}");
    assert!(stats.bandwidth_max_queue_delay_ns > 0, "{stats:?}");
    assert!(stats.bandwidth_tail_drops > 0, "{stats:?}");
    assert_eq!(stats.bandwidth_oversize_drops, 0, "{stats:?}");
    assert_eq!(stats.bandwidth_reservation_cap_drops, 0, "{stats:?}");
    assert_eq!(
        stats.bandwidth_reservation_cap_dropped_bytes, 0,
        "{stats:?}"
    );
    assert_eq!(stats.bandwidth_time_overflow_drops, 0, "{stats:?}");
    assert_eq!(stats.dropped_by_policy, 0, "{stats:?}");
    assert_eq!(stats.dropped_blocked, 0, "{stats:?}");
    assert!(first.final_confirmed.iter().all(|frame| *frame > 400));
    assert!(
        first
            .progress_samples
            .iter()
            .flat_map(|sample| &sample.link_queues)
            .any(|queue| queue.from == 0
                && queue.to == 1
                && queue.queued_bytes > 0
                && queue.queued_datagrams > 0
                && queue.drain_delay_ns > 0),
        "the bounded series must observe live queue growth on 0->1: {:?}",
        first.progress_samples
    );
    assert!(
        first
            .progress_samples
            .last()
            .is_some_and(|sample| sample
                .link_queues
                .iter()
                .all(|queue| queue.queued_bytes == 0
                    && queue.queued_datagrams == 0
                    && queue.drain_delay_ns == 0)),
        "the final bounded sample must prove every bandwidth queue drained"
    );
    assert_eq!(
        control
            .metrics
            .iter()
            .map(|metrics| metrics.wait_recommendations)
            .sum::<u64>(),
        0
    );
    assert!(
        first.metrics[0].wait_recommendations > first.metrics[1].wait_recommendations,
        "the one-way queue must induce the predicted one-sided recommendation bias: {:?}",
        first.metrics
    );

    let constrained = first.link_stats_by_link[&(0, 1)];
    assert!(constrained.bandwidth_queued_datagrams > 0);
    assert_eq!(
        constrained.bandwidth_admitted_datagrams,
        stats.bandwidth_admitted_datagrams
    );
    assert!(constrained.bandwidth_max_queue_bytes > 0);
    assert!(constrained.bandwidth_max_queue_delay_ns > 0);
    assert!(constrained.bandwidth_tail_drops > 0);
    for (&link, stats) in &first.link_stats_by_link {
        if link != (0, 1) {
            assert_eq!(stats.bandwidth_admitted_bytes, 0, "link={link:?}");
            assert_eq!(stats.bandwidth_tail_drops, 0, "link={link:?}");
        }
    }
}

/// H-BLOAT matched treatment: compare an app that ignores one-way
/// queue-induced wait recommendations with one that obeys them, using the live
/// queue series plus cumulative maxima as recovery evidence. The harness keeps
/// polling while it skips simulation advances, so this row covers pacing
/// feedback, not an application that also suspends network service.
#[test]
fn h_bloat_obedience_reduces_work_without_changing_sampled_queue() {
    let mut ignore = bandwidth_queue_schedule(true);
    ignore.config.app_model = AppModel::Ignore;
    let mut obey = ignore.clone();
    obey.config.app_model = AppModel::Obey;

    let ignore_report = run(&ignore, &RunOptions::default());
    let obey_report = run(&obey, &RunOptions::default());
    let replay = run(&obey, &RunOptions::default());
    ignore_report.expect_pass(&ignore);
    obey_report.expect_pass(&obey);
    assert_eq!(obey_report.trace_hash, replay.trace_hash);
    assert_eq!(obey_report.progress_samples, replay.progress_samples);

    let queue_series = |report: &RunReport| {
        report
            .progress_samples
            .iter()
            .map(|sample| {
                let queue = sample
                    .link_queues
                    .iter()
                    .find(|queue| queue.from == 0 && queue.to == 1)
                    .expect("complete directed-link queue sample");
                (sample.step, queue.queued_bytes, queue.drain_delay_ns)
            })
            .collect::<Vec<_>>()
    };
    let ignore_queue = queue_series(&ignore_report);
    let obey_queue = queue_series(&obey_report);
    let total_work = |report: &RunReport| {
        report
            .metrics
            .iter()
            .map(|metrics| metrics.frames_advanced)
            .sum::<u64>()
    };
    assert!(ignore_queue.iter().any(|(_, bytes, _)| *bytes > 0));
    assert!(obey_queue.iter().any(|(_, bytes, _)| *bytes > 0));
    assert!(obey_report
        .wait_frames_obeyed
        .iter()
        .any(|skips| *skips > 0));
    assert_eq!(
        ignore_queue, obey_queue,
        "obeying must not change the bounded queue/drain samples"
    );
    assert_eq!(
        ignore_report.net_stats.bandwidth_admitted_bytes,
        obey_report.net_stats.bandwidth_admitted_bytes
    );
    assert_eq!(
        ignore_report.net_stats.bandwidth_tail_dropped_bytes,
        obey_report.net_stats.bandwidth_tail_dropped_bytes
    );
    assert_eq!(
        ignore_report.net_stats.bandwidth_max_queue_bytes,
        obey_report.net_stats.bandwidth_max_queue_bytes
    );
    assert_eq!(
        ignore_report.net_stats.bandwidth_max_queue_delay_ns,
        obey_report.net_stats.bandwidth_max_queue_delay_ns
    );
    let ignore_work = total_work(&ignore_report);
    let obey_work = total_work(&obey_report);
    assert!(
        obey_work.saturating_mul(100) <= ignore_work.saturating_mul(75),
        "obeying should reduce stress-work by at least 25%: \
         ignore={ignore_work}, obey={obey_work}"
    );
    let resimulation = |report: &RunReport| {
        report
            .metrics
            .iter()
            .map(|metrics| metrics.resimulated_frames)
            .sum::<u64>()
    };
    let ignore_resimulation = resimulation(&ignore_report);
    let obey_resimulation = resimulation(&obey_report);
    assert!(
        obey_resimulation.saturating_mul(100) <= ignore_resimulation.saturating_mul(50),
        "obeying should halve resimulation in this matched stress row: \
         ignore={ignore_resimulation}, obey={obey_resimulation}"
    );
    assert!(obey_report
        .progress_samples
        .last()
        .is_some_and(|sample| sample
            .link_queues
            .iter()
            .all(|queue| queue.queued_bytes == 0)));
}

#[test]
fn input_redundancy_window_recovers_at_limit_and_fails_closed_beyond() {
    let options = RunOptions {
        probe_confirmed_at: Some(INPUT_WINDOW_FAULT_START - 1),
        pending_output_probe_link: Some((0, 1)),
        ..RunOptions::default()
    };

    let control_schedule = input_window_boundary_schedule(287, false);
    let control = run(&control_schedule, &options);
    control.expect_pass(&control_schedule);
    assert_eq!(control.net_stats.dropped_by_policy, 0);
    assert_eq!(
        control.link_stats_by_link[&(0, 1)].max_consecutive_input_policy_loss_run,
        0
    );

    let run_replayed = |heal_at| {
        let schedule = input_window_boundary_schedule(heal_at, true);
        let first = run(&schedule, &options);
        let replay = run(&schedule, &options);
        assert_eq!(first.trace_hash, replay.trace_hash, "heal={heal_at}");
        assert_eq!(first.verdict, replay.verdict, "heal={heal_at}");
        assert_eq!(first.net_stats, replay.net_stats, "heal={heal_at}");
        assert_eq!(first.metrics, replay.metrics, "heal={heal_at}");
        assert_eq!(
            first.final_confirmed, replay.final_confirmed,
            "heal={heal_at}"
        );
        assert_eq!(
            first.violation_census, replay.violation_census,
            "heal={heal_at}"
        );
        assert_eq!(
            first.pending_output_probe, replay.pending_output_probe,
            "heal={heal_at}"
        );
        (schedule, first)
    };

    let (below_schedule, below) = run_replayed(286);
    below.expect_pass(&below_schedule);
    let below_pending = below.pending_output_probe.expect("probe requested");
    assert_eq!(below_pending.limit, 128);
    assert_eq!(below_pending.max, 127);
    assert_eq!(below_pending.at_heal, Some(127));
    assert_eq!(below_pending.first_reached_limit_at, None);
    assert_eq!(
        below.link_stats_by_link[&(0, 1)].max_consecutive_input_policy_loss_run,
        126
    );
    assert_eq!(below.recovered_within_b, Some(true));

    let (at_schedule, at) = run_replayed(287);
    at.expect_pass(&at_schedule);
    let at_pending = at.pending_output_probe.expect("probe requested");
    assert_eq!(at_pending.limit, 128);
    assert_eq!(at_pending.max, 128);
    assert_eq!(at_pending.at_heal, Some(128));
    assert_eq!(at_pending.first_reached_limit_at, Some(287));
    assert_eq!(
        at.link_stats_by_link[&(0, 1)].max_consecutive_input_policy_loss_run,
        127
    );
    assert_eq!(at.recovered_within_b, Some(true));
    assert!(at.violation_census.is_empty());

    // The next lost Input cannot grow the bounded pending-output queue beyond
    // 128. The sender disconnects before a 129th entry is retained; the oracle
    // therefore sees only the pinned terminal liveness failures, never queue
    // corruption, unavailable confirmed input, or state divergence.
    let (_over_schedule, over) = run_replayed(288);
    let over_pending = over.pending_output_probe.expect("probe requested");
    assert_eq!(over_pending.limit, 128);
    assert_eq!(over_pending.max, 128);
    assert_eq!(over_pending.at_heal, Some(128));
    assert_eq!(over_pending.first_reached_limit_at, Some(287));
    assert_eq!(
        over.link_stats_by_link[&(0, 1)].max_consecutive_input_policy_loss_run,
        128
    );
    assert_eq!(
        peer_event_payload_count(&over, 0, addr_event_key(EventKind::Disconnected, 1)),
        1
    );
    assert_eq!(over.recovered_within_b, Some(false));
    assert!(over.violation_census.is_empty());
    assert_eq!(over.verdict.failures.len(), 4);
    assert!(over.verdict.failures.iter().all(|failure| matches!(
        failure,
        OracleFailure::EndProgress { .. } | OracleFailure::PostHealLiveness { .. }
    )));
}

#[test]
#[ignore = "nightly N=16 32-byte fragmentation experiment"]
fn oversized_input_fragment_loss_is_bounded_and_recovers() {
    let options = RunOptions {
        probe_confirmed_at: Some(FRAGMENTATION_BACKLOG_RELEASE - 1),
        pending_output_probe_link: Some((0, 1)),
        ..RunOptions::default()
    };
    let control_schedule = fragmentation_schedule(0.0);
    let control = run_with_input::<WideStubInput>(&control_schedule, &options);
    control.expect_pass(&control_schedule);

    let schedule = fragmentation_schedule(0.25);
    let first = run_with_input::<WideStubInput>(&schedule, &options);
    let replay = run_with_input::<WideStubInput>(&schedule, &options);
    first.expect_pass(&schedule);
    assert_eq!(first.trace_hash, replay.trace_hash);
    assert_eq!(first.verdict, replay.verdict);
    assert_eq!(first.net_stats, replay.net_stats);
    assert_eq!(first.pending_output_probe, replay.pending_output_probe);
    assert_eq!(
        first.fragmentation_drops_by_link,
        replay.fragmentation_drops_by_link
    );
    assert_eq!(first.blocked_drops_by_link, replay.blocked_drops_by_link);
    assert_eq!(first.link_stats_by_link, replay.link_stats_by_link);

    let control_probe = control.pending_output_probe.expect("control probe");
    let probe = first.pending_output_probe.expect("fault probe");
    assert!(
        probe
            .at_probe
            .is_some_and(|value| (1..128).contains(&value)),
        "{probe:?}"
    );
    assert_eq!(probe.at_probe, control_probe.at_probe);
    assert_eq!(probe.at_heal, control_probe.at_heal);
    for evidence in [control_probe, probe] {
        assert!(evidence.max < 128, "{evidence:?}");
        assert!(
            evidence.after_recovery.is_some_and(|value| value <= 1),
            "{evidence:?}"
        );
        assert!(evidence.final_value <= 1, "{evidence:?}");
    }

    let control_link = control.link_stats_by_link[&(0, 1)];
    let link = first.link_stats_by_link[&(0, 1)];
    assert!(
        control_link.max_encoded_input_bytes > 1_472,
        "{control_link:?}"
    );
    assert!(
        control_link.input_sends_over_1472_bytes > 0,
        "{control_link:?}"
    );
    assert!(link.max_encoded_input_bytes > 1_472, "{link:?}");
    assert!(link.input_sends_over_1472_bytes > 0, "{link:?}");
    assert!(first.net_stats.fragmentation_input_eligible_sends > 0);
    assert!(first.net_stats.fragmentation_max_fragments_per_send >= 2);
    assert_eq!(first.net_stats.fragmentation_fragment_cap_hits, 0);
    assert_eq!(control.net_stats.fragmentation_input_loss_events, 0);
    assert!(first.net_stats.fragmentation_input_loss_events > 0);
    assert_eq!(
        first
            .fragmentation_drops_by_link
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        vec![(0, 1)]
    );
    assert_eq!(
        first
            .blocked_drops_by_link
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        vec![(0, 1)]
    );
    assert_eq!(first.recovered_within_b, Some(true));
    assert!(first.trace_tail.last().is_some_and(|snapshot| snapshot
        .session_states
        .iter()
        .all(|state| *state == TraceSessionState::Running)));
}

/// H-POLLCAP clean control: ordinary N=16 cold start never reaches the
/// per-destination decoded-message cap, and all 240 directed handshakes finish
/// before the storm schedule's release anchor.
#[test]
fn h_pollcap_clean_cold_start_has_no_cap_deferral() {
    let schedule = pollcap_schedule(false, 280);
    let options = RunOptions {
        receive_message_limit: Some(256),
        ..RunOptions::default()
    };
    let first = run(&schedule, &options);
    let replay = run(&schedule, &options);
    first.expect_pass(&schedule);
    assert_eq!(first.trace_hash, replay.trace_hash);
    assert_eq!(first.verdict, replay.verdict);
    assert_eq!(first.receive_stats_by_peer, replay.receive_stats_by_peer);
    assert_eq!(first.first_running_step, replay.first_running_step);
    assert_eq!(
        first.first_synchronized_step,
        replay.first_synchronized_step
    );
    assert_all_peer_pairs_synchronized(&first);
    assert!(
        first
            .first_running_step
            .iter()
            .all(|step| step.is_some_and(|step| step < POLLCAP_RELEASE)),
        "{:?}",
        first.first_running_step
    );
    assert!(first
        .first_synchronized_step
        .iter()
        .flatten()
        .flatten()
        .all(|step| *step < POLLCAP_RELEASE));
    assert!(first
        .receive_stats_by_peer
        .iter()
        .all(|stats| { stats.cap_limited_polls == 0 && stats.max_remainder_after_drain == 0 }));
    assert_eq!(first.net_stats.dropped_by_policy, 0);
    assert_eq!(first.net_stats.dropped_blocked, 0);
    assert_eq!(first.net_stats.dropped_unattached, 0);
    assert_eq!(first.net_stats.duplicated, 0);
}

/// H-POLLCAP planted release storm: the same target backlog is drained with
/// caps 256 and 512, isolating bounded FIFO deferral from loss or starvation.
#[test]
#[ignore = "nightly N=16 planted per-destination receive storm"]
#[allow(clippy::print_stdout, clippy::disallowed_macros)]
fn h_pollcap_targeted_release_defers_without_starvation() {
    let schedule = pollcap_schedule(true, 800);
    let low_options = RunOptions {
        receive_message_limit: Some(256),
        ..RunOptions::default()
    };
    let high_options = RunOptions {
        receive_message_limit: Some(512),
        ..RunOptions::default()
    };
    let low = run(&schedule, &low_options);
    let high = run(&schedule, &high_options);
    let replay = run(&schedule, &low_options);
    low.expect_pass(&schedule);
    high.expect_pass(&schedule);
    assert_eq!(low.trace_hash, replay.trace_hash);
    assert_eq!(low.verdict, replay.verdict);
    assert_eq!(low.net_stats, replay.net_stats);
    assert_eq!(low.receive_stats_by_peer, replay.receive_stats_by_peer);
    assert_eq!(low.first_running_step, replay.first_running_step);
    assert_eq!(low.first_synchronized_step, replay.first_synchronized_step);
    assert_all_peer_pairs_synchronized(&low);
    assert_all_peer_pairs_synchronized(&high);

    let low_target = low.receive_stats_by_peer[0];
    let high_target = high.receive_stats_by_peer[0];
    println!(
        "H-POLLCAP low={low_target:?} high={high_target:?} low_running={:?} high_running={:?}",
        low.first_running_step, high.first_running_step
    );
    assert_eq!(
        low_target.max_due_before_drain,
        high_target.max_due_before_drain
    );
    assert_eq!(low_target.max_due_before_drain, 270, "{low_target:?}");
    assert_eq!(low_target.cap_limited_polls, 1, "{low_target:?}");
    assert_eq!(low_target.max_returned_batch, 256);
    assert_eq!(low_target.max_remainder_after_drain, 14, "{low_target:?}");
    assert!(low_target.drained_after_cap, "{low_target:?}");
    assert_eq!(high_target.cap_limited_polls, 0, "{high_target:?}");
    assert_eq!(high_target.max_returned_batch, 270, "{high_target:?}");
    assert_eq!(high_target.max_remainder_after_drain, 0, "{high_target:?}");
    assert!(!high_target.drained_after_cap, "{high_target:?}");
    assert!(low.receive_stats_by_peer[1..]
        .iter()
        .all(|stats| stats.cap_limited_polls == 0));
    for (low_step, high_step) in low.first_running_step.iter().zip(&high.first_running_step) {
        let low_step = low_step.expect("low-cap peer reaches Running");
        let high_step = high_step.expect("high-cap peer reaches Running");
        assert!((high_step..=high_step + 1).contains(&low_step));
    }
    for (low_step, high_step) in low
        .first_synchronized_step
        .iter()
        .flatten()
        .zip(high.first_synchronized_step.iter().flatten())
    {
        match (*low_step, *high_step) {
            (Some(low_step), Some(high_step)) => {
                assert!((high_step..=high_step + 1).contains(&low_step));
            },
            (None, None) => {},
            pair => panic!("mismatched synchronization evidence: {pair:?}"),
        }
    }
    assert!(low.net_stats.held > 256, "{:?}", low.net_stats);
    for net in [low.net_stats, high.net_stats] {
        assert_eq!(net.dropped_by_policy, 0);
        assert_eq!(net.dropped_blocked, 0);
        assert_eq!(net.dropped_unattached, 0);
        assert_eq!(net.duplicated, 0);
        assert_eq!(net.fragmentation_loss_events, 0);
        assert_eq!(net.bandwidth_tail_drops, 0);
        assert_eq!(net.bandwidth_oversize_drops, 0);
        assert_eq!(net.bandwidth_reservation_cap_drops, 0);
        assert_eq!(net.bandwidth_time_overflow_drops, 0);
    }
    assert_eq!(low.recovered_within_b, Some(true));
    assert_eq!(high.recovered_within_b, Some(true));
}

/// H-BLOAT scale interaction: an N=16 mesh with incompressible 32-byte inputs
/// releases a bounded pending-input backlog into one constrained directed
/// link. No-bandwidth and zero-fragment-loss controls separate queue activation
/// and fragment loss from the matched app treatments. The pinned 8.75 KB/s
/// rate, 8 KB burst, and 32 KB tail-drop queue exercise a bounded near-capacity
/// case, not a general model of every N=16 uplink. The backlog intentionally
/// crosses the production IPv4 fragmentation alarm boundary on peers 0 and 1;
/// those two exact alarms are premise evidence, while every other oracle
/// failure remains fatal.
#[test]
#[ignore = "nightly N=16 32-byte bandwidth/fragmentation interaction"]
#[allow(clippy::print_stdout, clippy::disallowed_macros)]
fn h_bloat_scale_fragmentation_interaction_is_bounded_and_recovers() {
    let options = RunOptions {
        probe_confirmed_at: Some(FRAGMENTATION_BACKLOG_RELEASE - 1),
        pending_output_probe_link: Some((0, 1)),
        ..RunOptions::default()
    };
    let unconstrained_ignore = scale_bandwidth_fragmentation_schedule(AppModel::Ignore, false, 0.0);
    let unconstrained = scale_bandwidth_fragmentation_schedule(AppModel::Obey, false, 0.0);
    let fragment_ignore = scale_bandwidth_fragmentation_schedule(AppModel::Ignore, false, 0.25);
    let fragment_obey = scale_bandwidth_fragmentation_schedule(AppModel::Obey, false, 0.25);
    let bandwidth_ignore = scale_bandwidth_fragmentation_schedule(AppModel::Ignore, true, 0.0);
    let bandwidth_obey = scale_bandwidth_fragmentation_schedule(AppModel::Obey, true, 0.0);
    let ignore = scale_bandwidth_fragmentation_schedule(AppModel::Ignore, true, 0.25);
    let obey = scale_bandwidth_fragmentation_schedule(AppModel::Obey, true, 0.25);

    let unconstrained_ignore_report =
        run_with_input::<WideStubInput>(&unconstrained_ignore, &options);
    let unconstrained_report = run_with_input::<WideStubInput>(&unconstrained, &options);
    let fragment_ignore_report = run_with_input::<WideStubInput>(&fragment_ignore, &options);
    let fragment_obey_report = run_with_input::<WideStubInput>(&fragment_obey, &options);
    let bandwidth_ignore_report = run_with_input::<WideStubInput>(&bandwidth_ignore, &options);
    let bandwidth_obey_report = run_with_input::<WideStubInput>(&bandwidth_obey, &options);
    let ignore_report = run_with_input::<WideStubInput>(&ignore, &options);
    let obey_report = run_with_input::<WideStubInput>(&obey, &options);
    let ignore_replay = run_with_input::<WideStubInput>(&ignore, &options);
    let replay = run_with_input::<WideStubInput>(&obey, &options);
    let assert_expected_fragmentation_alarms =
        |label: &str, report: &RunReport, encoded_bytes: [usize; 2]| {
            let expected_alarm = |source, destination, encoded_bytes| OracleFailure::Violation {
                source: ViolationSource::Peer(source),
                violation: format!(
                    "[Error/NetworkProtocol] Message queued for {destination:?} reaches the \
                 IPv4/UDP fragmentation boundary: kind=input, encoded_bytes={encoded_bytes}, \
                 threshold=1472. Further fragmentation alarms are suppressed for this \
                 endpoint era; inspect PeerMetrics::fragmentation_risk_messages_sent for \
                 the cumulative count."
                ),
            };
            let expected_failures = [
                expected_alarm(0, peer_addr(1), encoded_bytes[0]),
                expected_alarm(1, peer_addr(0), encoded_bytes[1]),
            ];
            assert_eq!(
                report.verdict.failures.as_slice(),
                &expected_failures,
                "{label} emitted an unexpected oracle failure: {:?}",
                report.verdict.failures
            );
            assert_eq!(
                report.violation_census.len(),
                2,
                "{label} emitted an unexpected violation signature: {:?}",
                report.violation_census
            );

            let census_count = |severity| {
                report
                    .violation_census
                    .iter()
                    .filter(|(signature, _)| {
                        signature.severity == severity
                            && signature.kind == ViolationKind::NetworkProtocol
                            && signature.message_prefix == "Message queued for"
                    })
                    .map(|(_, count)| *count)
                    .sum::<u64>()
            };
            assert_eq!(census_count(ViolationSeverity::Warning), 2, "{label}");
            assert_eq!(census_count(ViolationSeverity::Error), 2, "{label}");
        };
    for (label, report, encoded_bytes) in [
        (
            "unconstrained Ignore",
            &unconstrained_ignore_report,
            [1501, 1501],
        ),
        ("unconstrained Obey", &unconstrained_report, [1501, 1507]),
        ("fragment Ignore", &fragment_ignore_report, [1501, 1501]),
        ("fragment Obey", &fragment_obey_report, [1501, 1507]),
        ("bandwidth Ignore", &bandwidth_ignore_report, [1501, 1501]),
        ("bandwidth Obey", &bandwidth_obey_report, [1501, 1507]),
        ("matched Ignore", &ignore_report, [1501, 1501]),
        ("matched Obey", &obey_report, [1501, 1507]),
        ("matched Ignore replay", &ignore_replay, [1501, 1501]),
        ("matched Obey replay", &replay, [1501, 1507]),
    ] {
        assert_expected_fragmentation_alarms(label, report, encoded_bytes);
    }
    assert_eq!(ignore_report.trace_hash, ignore_replay.trace_hash);
    assert_eq!(ignore_report.verdict, ignore_replay.verdict);
    assert_eq!(
        ignore_report.violation_census,
        ignore_replay.violation_census
    );
    assert_eq!(
        ignore_report.progress_samples,
        ignore_replay.progress_samples
    );
    assert_eq!(ignore_report.net_stats, ignore_replay.net_stats);
    assert_eq!(obey_report.trace_hash, replay.trace_hash);
    assert_eq!(obey_report.verdict, replay.verdict);
    assert_eq!(obey_report.violation_census, replay.violation_census);
    assert_eq!(obey_report.progress_samples, replay.progress_samples);
    assert_eq!(obey_report.net_stats, replay.net_stats);
    assert_eq!(obey_report.link_stats_by_link, replay.link_stats_by_link);
    assert_eq!(ignore_report.recovered_within_b, Some(true));
    assert_eq!(obey_report.recovered_within_b, Some(true));

    for report in [
        &unconstrained_ignore_report,
        &unconstrained_report,
        &fragment_ignore_report,
        &fragment_obey_report,
    ] {
        assert_eq!(report.net_stats.bandwidth_admitted_datagrams, 0);
        assert_eq!(report.recovered_within_b, Some(true));
    }
    assert_eq!(
        unconstrained_report
            .net_stats
            .fragmentation_input_loss_events,
        0
    );
    assert_eq!(
        unconstrained_report.wait_frames_obeyed,
        [vec![3, 3], vec![0; 14]].concat(),
        "the backlog-only control should pace only the disturbed endpoint pair"
    );
    for report in [&fragment_ignore_report, &fragment_obey_report] {
        assert!(report.net_stats.fragmentation_input_loss_events > 0);
    }
    assert!(
        bandwidth_ignore_report.net_stats.bandwidth_queued_datagrams > 0,
        "the zero-fragment Ignore control must activate the configured bucket"
    );
    assert_eq!(
        bandwidth_obey_report.net_stats.bandwidth_queued_datagrams, 0,
        "obedience must drain the zero-fragment control without queueing"
    );
    for report in [&bandwidth_ignore_report, &bandwidth_obey_report] {
        assert_eq!(report.net_stats.fragmentation_input_loss_events, 0);
        assert_eq!(report.recovered_within_b, Some(true));
    }

    for report in [&ignore_report, &obey_report] {
        let stats = report.net_stats;
        assert!(stats.bandwidth_queued_datagrams > 0, "{stats:?}");
        assert!(stats.bandwidth_max_queue_bytes > 0, "{stats:?}");
        assert!(stats.bandwidth_max_queue_delay_ns > 0, "{stats:?}");
        assert!(stats.fragmentation_input_eligible_sends > 0, "{stats:?}");
        assert!(stats.fragmentation_input_loss_events > 0, "{stats:?}");
        assert!(stats.fragmentation_max_fragments_per_send >= 2, "{stats:?}");
        assert_eq!(stats.fragmentation_fragment_cap_hits, 0, "{stats:?}");
        assert_eq!(stats.bandwidth_oversize_drops, 0, "{stats:?}");
        assert_eq!(stats.bandwidth_reservation_cap_drops, 0, "{stats:?}");
        assert_eq!(stats.bandwidth_time_overflow_drops, 0, "{stats:?}");
        assert!(report.final_confirmed.iter().all(|frame| *frame > 400));
        let pending = report.pending_output_probe.expect("pending-output probe");
        assert!(
            pending
                .at_probe
                .is_some_and(|value| (1..128).contains(&value)),
            "{pending:?}"
        );
        assert!(pending.max < 128, "{pending:?}");
        assert!(
            pending.after_recovery.is_some_and(|value| value <= 1),
            "{pending:?}"
        );
        assert!(pending.final_value <= 1, "{pending:?}");
        let link = report.link_stats_by_link[&(0, 1)];
        assert!(link.max_encoded_input_bytes > 1_472, "{link:?}");
        assert!(link.input_sends_over_1472_bytes > 0, "{link:?}");
        assert!(report.progress_samples.iter().any(|sample| sample
            .link_queues
            .iter()
            .any(|queue| queue.from == 0 && queue.to == 1 && queue.queued_bytes > 0)));
        assert!(report.progress_samples.last().is_some_and(|sample| sample
            .link_queues
            .iter()
            .all(|queue| queue.queued_bytes == 0
                && queue.queued_datagrams == 0
                && queue.drain_delay_ns == 0)));
    }

    assert!(
        obey_report
            .wait_frames_obeyed
            .iter()
            .any(|skips| *skips > 0),
        "the treatment must actually honor at least one wait recommendation"
    );
    assert!(
        obey_report.wait_frames_obeyed.iter().sum::<u64>()
            > bandwidth_obey_report.wait_frames_obeyed.iter().sum::<u64>()
            && bandwidth_obey_report.wait_frames_obeyed[2..]
                .iter()
                .all(|skips| *skips == 0)
            && obey_report.wait_frames_obeyed[2..]
                .iter()
                .all(|skips| *skips > 0),
        "fragment loss plus the constrained link must expand pacing beyond both controls"
    );
    assert!(
        obey_report
            .net_stats
            .bandwidth_admitted_bytes
            .saturating_mul(100)
            <= ignore_report
                .net_stats
                .bandwidth_admitted_bytes
                .saturating_mul(95),
        "honoring waits should retain the measured >=5% admitted-byte reduction"
    );
    assert!(
        obey_report
            .net_stats
            .bandwidth_tail_dropped_bytes
            .saturating_mul(100)
            <= ignore_report
                .net_stats
                .bandwidth_tail_dropped_bytes
                .saturating_mul(20),
        "honoring waits should retain the measured >=80% tail-drop reduction"
    );
    assert!(
        obey_report
            .net_stats
            .bandwidth_max_queue_delay_ns
            .saturating_mul(100)
            <= ignore_report
                .net_stats
                .bandwidth_max_queue_delay_ns
                .saturating_mul(60),
        "honoring waits should retain the measured >=40% maximum-delay reduction"
    );
    let total_work = |report: &RunReport| {
        report
            .metrics
            .iter()
            .map(|metrics| metrics.frames_advanced)
            .sum::<u64>()
    };
    let total_resimulation = |report: &RunReport| {
        report
            .metrics
            .iter()
            .map(|metrics| metrics.resimulated_frames)
            .sum::<u64>()
    };
    let ignore_work = total_work(&ignore_report);
    let obey_work = total_work(&obey_report);
    let ignore_resimulation = total_resimulation(&ignore_report);
    let obey_resimulation = total_resimulation(&obey_report);
    let sampled_drain_step = |report: &RunReport| {
        let last_nonempty = report.progress_samples.iter().rposition(|sample| {
            sample
                .link_queues
                .iter()
                .any(|queue| queue.from == 0 && queue.to == 1 && queue.queued_bytes > 0)
        });
        last_nonempty
            .and_then(|index| report.progress_samples.get(index.saturating_add(1)))
            .map(|sample| sample.step)
            .expect("a bounded sample follows the last nonempty 0->1 queue")
    };
    let ignore_drain_step = sampled_drain_step(&ignore_report);
    let obey_drain_step = sampled_drain_step(&obey_report);
    assert!(
        obey_drain_step <= ignore_drain_step,
        "obedience must not delay sampled queue drain: {ignore_drain_step}->{obey_drain_step}"
    );
    assert!(
        obey_work.saturating_mul(100) >= ignore_work.saturating_mul(150)
            && obey_resimulation.saturating_mul(100) >= ignore_resimulation.saturating_mul(200),
        "the scale treatment's work-amplification finding must stay visible: \
         work={ignore_work}->{obey_work}, resimulation={ignore_resimulation}->{obey_resimulation}"
    );
    println!(
        "H-BLOAT-SCALE zero_fragment_skips={:?} admitted_bytes={}->{} \
         tail_dropped_bytes={}->{} \
         max_queue_bytes={}->{} max_queue_delay_ns={}->{} drain_step={}->{} \
         obey_skips={:?} \
         work={ignore_work}->{obey_work} resimulation={ignore_resimulation}->{obey_resimulation}",
        bandwidth_obey_report.wait_frames_obeyed,
        ignore_report.net_stats.bandwidth_admitted_bytes,
        obey_report.net_stats.bandwidth_admitted_bytes,
        ignore_report.net_stats.bandwidth_tail_dropped_bytes,
        obey_report.net_stats.bandwidth_tail_dropped_bytes,
        ignore_report.net_stats.bandwidth_max_queue_bytes,
        obey_report.net_stats.bandwidth_max_queue_bytes,
        ignore_report.net_stats.bandwidth_max_queue_delay_ns,
        obey_report.net_stats.bandwidth_max_queue_delay_ns,
        ignore_drain_step,
        obey_drain_step,
        obey_report.wait_frames_obeyed,
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

fn h_ring_double_failure_relay_schedule() -> Schedule {
    const N: usize = 4;
    const TARGET: usize = 3;
    const LOW_ORIGIN: usize = 1;
    let mut config = SimConfig::smoke(N);
    config.steps = 600;
    config.noise = BackgroundNoise::Clean;
    config.disconnect_behavior = DropPolicy::ContinueWithout;
    config.input_delay = 0;
    config.max_prediction = fortress_rollback::__internal::MAX_FRAME_DELAY;

    let events = vec![
        // Build the target-receipt gradient: the low origin stops receiving
        // target inputs while the observer and relay continue receiving them.
        (
            50,
            ScheduleEvent::Block {
                from: TARGET,
                to: LOW_ORIGIN,
                blocked: true,
            },
        ),
        // Heal before any lifecycle failure. H-RING asks whether this receipt
        // gradient can evict the frozen slot *before* the double-failure relay
        // choreography begins.
        (180, ScheduleEvent::HealAll),
    ];

    Schedule {
        schema_version: SCHEDULE_SCHEMA_VERSION,
        seed: 0xCE45_5249,
        link_seed: 0xCE45_524A,
        config,
        initial_links: clean_initial_links(N),
        events,
        heal_at: 180,
    }
}

fn hostile_connection_status_schedule() -> Schedule {
    const N: usize = 4;
    let mut config = SimConfig::smoke(N);
    // Leave a complete recovery window after the step-251 HealAll. The
    // bounded-liveness oracle requires `steps - heal_at >= B`.
    config.steps = 502;
    config.noise = BackgroundNoise::Clean;
    config.input_delay = 0;
    config.max_prediction = 32;

    Schedule {
        schema_version: SCHEDULE_SCHEMA_VERSION,
        seed: 0xCE45_B200,
        link_seed: 0xCE45_B201,
        config,
        initial_links: clean_initial_links(N),
        events: vec![(251, ScheduleEvent::HealAll)],
        heal_at: 251,
    }
}

fn hostile_floor_reply_schedule() -> Schedule {
    const N: usize = 4;
    const PRUNED_ORIGIN: usize = 0;
    let mut config = SimConfig::smoke(N);
    config.steps = 650;
    config.noise = BackgroundNoise::Clean;
    config.disconnect_behavior = DropPolicy::ContinueWithout;
    config.input_delay = 0;
    config.max_prediction = 32;

    Schedule {
        schema_version: SCHEDULE_SCHEMA_VERSION,
        seed: 0xCE45_B400,
        link_seed: 0xCE45_B401,
        config,
        initial_links: clean_initial_links(N),
        events: vec![
            (
                50,
                ScheduleEvent::PeerKill {
                    peer: PRUNED_ORIGIN,
                },
            ),
            (300, ScheduleEvent::HealAll),
        ],
        heal_at: 300,
    }
}

fn hostile_options(
    mode: HostileGossipMode,
    delta: i32,
    from_step: u32,
    until_step: u32,
) -> RunOptions {
    RunOptions {
        probe_confirmed_at: until_step.checked_sub(1),
        hostile_gossip: Some(HostileGossipOptions {
            liar: 1,
            observer: 3,
            target: 2,
            delta,
            from_step,
            until_step,
            mode,
        }),
        ..RunOptions::default()
    }
}

/// B2: one peer's valid but false third-party connection-status frame can only
/// lower the selected observer's minimum by the lie magnitude. An inflated
/// claim cannot lift the bound because the honest direct/witness terms remain.
#[test]
fn hostile_connection_status_lie_has_bounded_single_observer_effect() {
    const OBSERVER: usize = 3;
    const DELTA: i32 = 12;
    let schedule = hostile_connection_status_schedule();
    let control_options = hostile_options(HostileGossipMode::ConnectionStatus, 0, 150, 251);
    let control = run(&schedule, &control_options);
    let low = run(
        &schedule,
        &hostile_options(HostileGossipMode::ConnectionStatus, -DELTA, 150, 251),
    );
    let high = run(
        &schedule,
        &hostile_options(HostileGossipMode::ConnectionStatus, DELTA, 150, 251),
    );
    control.expect_pass(&schedule);
    low.expect_pass(&schedule);
    high.expect_pass(&schedule);

    let control_confirmed = control.probe_confirmed[OBSERVER];
    let control_evidence = control.hostile_gossip.expect("matched B2 control evidence");
    assert_eq!(control_evidence.messages_mutated, 0);
    let control_probe = control_evidence.probe.expect("matched B2 control probe");
    assert!(!control_probe.status_disconnected);
    assert_eq!(
        control_probe.effective_reported_frame,
        control_probe.status_frame
    );
    assert_eq!(
        control_probe.target_confirmed_bound,
        Some(control_confirmed)
    );
    for (name, report, delta) in [("low", &low, -DELTA), ("high", &high, DELTA)] {
        let evidence = report
            .hostile_gossip
            .expect("hostile B2 run must preserve mutation evidence");
        let probe = evidence
            .probe
            .expect("hostile B2 run must preserve receiver evidence");
        assert!(evidence.messages_mutated > 0, "{name}: {evidence:?}");
        assert_eq!(evidence.last_step, Some(250), "{name}: {evidence:?}");
        assert_eq!(probe.at_step, 250, "{name}: {probe:?}");
        assert_eq!(probe.status_frame, evidence.last_after.unwrap_or(i32::MIN));
        assert_eq!(probe.effective_reported_frame, probe.status_frame);
        assert!(!probe.status_disconnected, "{name}: {probe:?}");
        assert_eq!(probe.direct_receipt, control_probe.direct_receipt);
        assert_eq!(evidence.last_before, Some(control_probe.status_frame));
        assert!(evidence.last_before.is_some_and(|before| before >= DELTA));
        assert!(
            !probe.relay_topology,
            "{name}: B2 must use the ordinary fold"
        );
        assert_eq!(evidence.last_round_seq, None);
        assert_eq!(
            evidence.last_after,
            evidence
                .last_before
                .map(|before| before.saturating_add(delta).max(0))
        );
        assert!(
            report.probe_confirmed[OBSERVER].abs_diff(control_confirmed) <= DELTA.unsigned_abs(),
            "{name}: a ±{DELTA} lie exceeded its exact magnitude bound: control={}, hostile={}, evidence={evidence:?}",
            control_confirmed,
            report.probe_confirmed[OBSERVER]
        );
    }
    let low_probe = low
        .hostile_gossip
        .and_then(|evidence| evidence.probe)
        .expect("low B2 probe");
    let high_probe = high
        .hostile_gossip
        .and_then(|evidence| evidence.probe)
        .expect("high B2 probe");
    assert_eq!(
        low.probe_confirmed[OBSERVER],
        control_confirmed - DELTA,
        "the low treatment must non-vacuously pin confirmation by the exact lie"
    );
    assert_eq!(
        low_probe.target_confirmed_bound,
        Some(control_confirmed - DELTA)
    );
    assert_eq!(
        high.probe_confirmed[OBSERVER], control_confirmed,
        "an inflated single-peer claim must not raise the honest mesh minimum"
    );
    assert_eq!(high_probe.target_confirmed_bound, Some(control_confirmed));
    assert_eq!(
        low.final_confirmed, control.final_confirmed,
        "the observer must recover exactly after the bounded lie stops"
    );
    assert_eq!(high.final_confirmed, control.final_confirmed);
    assert_eq!(control.recovered_within_b, Some(true));
    assert_eq!(low.recovered_within_b, Some(true));
    assert_eq!(high.recovered_within_b, Some(true));
}

/// B4: after a genuine prune engages the floor-round topology, a low valid
/// floor conservatively wedges one observer; a high valid floor can prematurely
/// release it, but the truthful status and other honest terms bound both effects.
#[test]
fn hostile_floor_reply_lie_has_bounded_wedge_and_release_effects() {
    const OBSERVER: usize = 3;
    const DELTA: i32 = 12;
    let schedule = hostile_floor_reply_schedule();
    let control_options = hostile_options(HostileGossipMode::FloorReply, 0, 150, 300);
    let control = run(&schedule, &control_options);
    let low = run(
        &schedule,
        &hostile_options(HostileGossipMode::FloorReply, -DELTA, 150, 300),
    );
    let high = run(
        &schedule,
        &hostile_options(HostileGossipMode::FloorReply, DELTA, 150, 300),
    );
    control.expect_pass(&schedule);
    low.expect_pass(&schedule);
    high.expect_pass(&schedule);

    let control_confirmed = control.probe_confirmed[OBSERVER];
    let control_evidence = control.hostile_gossip.expect("matched B4 control evidence");
    assert_eq!(control_evidence.messages_mutated, 0);
    let control_probe = control_evidence.probe.expect("matched B4 control probe");
    assert!(control_probe.relay_topology && control_probe.round_fresh);
    assert!(!control_probe.status_disconnected);
    assert!(
        control_probe.round_floor < control_probe.status_frame,
        "the truthful floor must be a live, binding term: {control_probe:?}"
    );
    assert_eq!(
        control_probe.effective_reported_frame,
        control_probe.round_floor
    );
    assert_eq!(
        control_probe.target_confirmed_bound,
        Some(control_confirmed)
    );
    for (name, report, delta) in [("low", &low, -DELTA), ("high", &high, DELTA)] {
        let evidence = report
            .hostile_gossip
            .expect("hostile B4 run must preserve mutation evidence");
        let probe = evidence
            .probe
            .expect("hostile B4 run must preserve receiver evidence");
        assert!(evidence.messages_mutated > 0, "{name}: {evidence:?}");
        assert!(
            evidence
                .last_step
                .is_some_and(|step| (150..300).contains(&step)),
            "{name}: {evidence:?}"
        );
        assert!(
            probe.relay_topology,
            "{name}: floor consumption must be live"
        );
        assert!(
            probe.endpoint_running && probe.round_fresh,
            "{name}: {probe:?}"
        );
        assert!(!probe.status_disconnected, "{name}: {probe:?}");
        assert_eq!(probe.status_frame, control_probe.status_frame);
        assert_eq!(probe.direct_receipt, control_probe.direct_receipt);
        assert_eq!(evidence.last_before, Some(control_probe.round_floor));
        assert!(evidence.last_before.is_some_and(|before| before >= DELTA));
        assert_eq!(probe.round_floor, evidence.last_after.unwrap_or(i32::MIN));
        assert_eq!(Some(probe.reply_seq), evidence.last_round_seq);
        assert!(probe.reply_seq > probe.prune_seq && probe.reply_seq <= probe.request_seq);
        assert_eq!(
            evidence.last_after,
            evidence
                .last_before
                .map(|before| before.saturating_add(delta).max(0))
        );
        assert!(
            report.probe_confirmed[OBSERVER].abs_diff(control_confirmed) <= DELTA.unsigned_abs(),
            "{name}: a ±{DELTA} floor exceeded its exact magnitude bound: control={}, hostile={}, evidence={evidence:?}",
            control_confirmed,
            report.probe_confirmed[OBSERVER]
        );
    }
    let low_evidence = low.hostile_gossip.expect("low B4 evidence");
    let low_probe = low_evidence.probe.expect("low B4 receiver evidence");
    let high_evidence = high.hostile_gossip.expect("high B4 evidence");
    let high_probe = high_evidence.probe.expect("high B4 receiver evidence");
    assert_eq!(
        low_probe.effective_reported_frame,
        control_probe.round_floor - DELTA
    );
    assert_eq!(
        low.probe_confirmed[OBSERVER],
        control_confirmed - DELTA,
        "the low floor must non-vacuously wedge by the exact lie: {low_evidence:?}"
    );
    assert_eq!(
        high_probe.effective_reported_frame, high_probe.status_frame,
        "an inflated floor must remain capped by the truthful status term: {high_evidence:?}"
    );
    assert!(
        high_probe
            .target_confirmed_bound
            .is_some_and(|bound| bound <= high_probe.status_frame),
        "the aggregate bound must not exceed the liar's truthful status term: {high_evidence:?}"
    );
    assert!(
        high.probe_confirmed[OBSERVER] > control_confirmed,
        "the high treatment must non-vacuously exercise bounded premature release: {high_evidence:?}"
    );
    assert!(
        high_probe
            .target_confirmed_bound
            .is_some_and(|bound| bound > control_confirmed),
        "the high floor must raise the live aggregate target bound: {high_evidence:?}"
    );
    assert_eq!(low.final_confirmed, control.final_confirmed);
    assert_eq!(high.final_confirmed, control.final_confirmed);
    assert_eq!(control.recovered_within_b, Some(true));
    assert_eq!(low.recovered_within_b, Some(true));
    assert_eq!(high.recovered_within_b, Some(true));
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

/// Two simultaneous drops whose survivor certificates cross incompatible
/// asymmetric cuts cannot be serialized safely. The observers must hold their
/// exposed prefixes and fail closed without emitting either uncertified drop.
#[test]
fn same_step_multi_drop_after_asymmetric_block_fails_closed() {
    const SURVIVORS: [usize; 2] = [0, 1];
    const DROPPED: [usize; 2] = [2, 3];

    let schedule = same_step_multi_drop_schedule();

    let report = run(&schedule, &RunOptions::default());
    for (from, to) in [(2, 1), (3, 0)] {
        assert!(
            blocked_drop_count(&report, from, to) > 0,
            "same-step multi-drop row must drop traffic on intended blocked link {from}->{to}: {:?}",
            report.blocked_drops_by_link
        );
    }
    assert_eq!(
        report.recovered_within_b,
        Some(false),
        "fail-closed sessions deliberately do not resume after heal"
    );
    for observer in SURVIVORS {
        for dropped in DROPPED {
            assert_eq!(
                peer_event_payload_count(&report, observer, peer_dropped_key(dropped)),
                0,
                "observer {observer} must not emit an uncertified drop for {dropped}: {:?}",
                report.peer_event_payload_counts_by_peer
            );
        }
    }
    assert!(
        !report.verdict.failures.iter().any(|failure| matches!(
            failure,
            OracleFailure::ConfirmedInputDivergence { .. }
                | OracleFailure::StateDivergence { .. }
                | OracleFailure::InbandDesyncDetected { .. }
                | OracleFailure::ChecksumMismatchMetric { .. }
        )),
        "the held prefixes must remain divergence-free: {:?}",
        report.verdict.failures
    );
    let final_snapshot = report.trace_tail.last().expect("non-empty trace");
    for observer in SURVIVORS {
        assert_eq!(
            final_snapshot.session_states.get(observer),
            Some(&TraceSessionState::Synchronizing),
            "observer {observer} must fail closed: {:?}",
            final_snapshot.session_states
        );
    }

    let again = run(&schedule, &RunOptions::default());
    assert_eq!(
        report.trace_hash, again.trace_hash,
        "same-step multi-drop row must reproduce its exact trace"
    );
}

/// D14 asymmetric-partition regression: the four reachable survivors commit
/// one non-retracting drop of peer 4, while the isolated peer cannot gather the
/// declared participant certificate and fails closed. No confirmed input may
/// be rewritten on either side of the partition.
#[test]
fn one_way_minority_partition_commits_majority_and_fails_minority_closed_d14() {
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
    for survivor in 0..=3 {
        assert_eq!(
            peer_event_payload_count(&report, 4, peer_dropped_key(survivor)),
            0,
            "minority peer 4 must not commit without a survivor certificate: {:?}",
            report.peer_event_payload_counts_by_peer
        );
        assert!(
            peer_event_payload_count(
                &report,
                4,
                addr_event_key(EventKind::NetworkInterrupted, survivor)
            ) > 0,
            "minority peer 4 must observe majority peer {survivor}'s silence: {:?}",
            report.peer_event_payload_counts_by_peer
        );
        assert_eq!(
            peer_event_payload_count(
                &report,
                4,
                addr_event_key(EventKind::Disconnected, survivor)
            ),
            0,
            "a failed certificate must not emit a committed disconnect: {:?}",
            report.peer_event_payload_counts_by_peer
        );
    }
    assert!(
        !report.verdict.failures.iter().any(|failure| matches!(
            failure,
            OracleFailure::ConfirmedInputDivergence { .. }
                | OracleFailure::StateDivergence { .. }
                | OracleFailure::InbandDesyncDetected { .. }
                | OracleFailure::ChecksumMismatchMetric { .. }
        )),
        "the coordinated barrier must eliminate every D14 divergence: {:?}",
        report.verdict.failures
    );
    let final_snapshot = report
        .trace_tail
        .last()
        .expect("a non-empty run must retain a final trace snapshot");
    assert_eq!(
        final_snapshot.session_states.get(..4),
        Some(
            &[
                TraceSessionState::Running,
                TraceSessionState::Running,
                TraceSessionState::Running,
                TraceSessionState::Running,
            ][..]
        ),
        "the certified majority must stay available: {:?}",
        final_snapshot.session_states
    );
    assert_eq!(
        final_snapshot.session_states.get(4),
        Some(&TraceSessionState::Synchronizing),
        "the uncertified minority must fail closed: {:?}",
        final_snapshot.session_states
    );
    let majority_state = final_snapshot
        .game_states
        .first()
        .expect("five-player trace must contain peer 0 state");
    assert!(
        final_snapshot
            .game_states
            .iter()
            .take(4)
            .all(|state| state == majority_state),
        "the certified majority must converge on one final state: {:?}",
        final_snapshot.game_states
    );
    assert!(
        report.verdict.failures.iter().all(|failure| match failure {
            OracleFailure::Violation { violation, .. } => {
                violation.contains("coordinated graceful drop")
                    && violation.contains("failed closed: Timeout")
            },
            OracleFailure::EndProgress {
                peer: 4,
                state: fortress_rollback::SessionState::Synchronizing,
                ..
            }
            | OracleFailure::PostHealLiveness { peer: 4, .. } => true,
            _ => false,
        }),
        "only the expected minority fail-closed diagnostics may remain: {:?}",
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

/// H-RING: directly measure the maximum target-receipt stagger admitted by the
/// N=4 double-failure-relay topology. The low connected receipt pins the source
/// at the exact ring boundary, so the hypothesized >128-frame stagger and
/// physical eviction do not occur before the first failure. When the next
/// input arrives, the queue must fail closed rather than evicting the frozen
/// value or silently confirming divergent state.
#[test]
fn full_ring_receipt_stagger_fails_closed_at_retained_boundary() {
    const OBSERVER: usize = 0;
    const LOW_ORIGIN: usize = 1;
    const TARGET: usize = 3;
    let schedule = h_ring_double_failure_relay_schedule();
    let options = RunOptions {
        receipt_range_probe_target: Some(TARGET),
        ..RunOptions::default()
    };

    let report = run(&schedule, &options);
    let probe = report
        .receipt_range_probe
        .as_ref()
        .expect("H-RING requests receipt/range evidence");
    assert_eq!(probe.target, TARGET);
    assert_eq!(
        probe.max_spread,
        u32::try_from(fortress_rollback::__internal::MAX_FRAME_DELAY)
            .expect("frame delay bound fits u32"),
        "H-RING must fill, but not cross, the inclusive input-ring boundary: {probe:?}"
    );
    assert_eq!(
        probe.connected[OBSERVER],
        Some(true),
        "the target must still be connected at the high observer's premise cut"
    );
    assert_eq!(
        probe.connected[LOW_ORIGIN],
        Some(true),
        "the low receipt must still participate in the connected fold at the boundary cut"
    );
    let freeze = probe.receipts[LOW_ORIGIN].expect("low origin receipt");
    let high = probe.receipts[OBSERVER].expect("high observer receipt");
    assert_eq!(
        u32::try_from(high.saturating_sub(freeze)).expect("oriented receipt spread fits u32"),
        probe.max_spread,
        "the retained-range observer must be an actual high-water extremum: {probe:?}"
    );
    let observer_range = probe.retained_ranges[OBSERVER].expect("observer retained range");
    assert_eq!(
        observer_range.first, freeze,
        "the boundary receipt must remain physically retained: {probe:?}"
    );
    assert_eq!(
        observer_range.last, high,
        "the retained range must end at the measured high receipt: {probe:?}"
    );
    assert!(
        report
            .blocked_drops_by_link
            .get(&(TARGET, LOW_ORIGIN))
            .copied()
            .unwrap_or(0)
            > 0,
        "the target->low-origin cut must actually drop traffic"
    );
    assert_eq!(report.recovered_within_b, Some(false));
    assert!(
        report.verdict.failures.iter().any(|failure| matches!(
            failure,
            OracleFailure::Violation { violation, .. }
                if violation.contains("Input queue capacity 128 exhausted")
        )),
        "the full-ring boundary must take the explicit capacity fail-closed path: {:?}",
        report.verdict.failures
    );
    let expected_failure = |failure: &OracleFailure| match failure {
        OracleFailure::Violation {
            source: ViolationSource::Peer(peer),
            violation,
        } => {
            *peer == OBSERVER
                && (violation.contains("Input queue capacity 128 exhausted")
                    || violation.contains("Input sequence violation"))
        },
        OracleFailure::EndProgress { peer, state, .. } => {
            matches!(*peer, OBSERVER | TARGET) && *state == SessionState::Synchronizing
        },
        OracleFailure::PostHealLiveness { peer, .. } => *peer < 4,
        _ => false,
    };
    assert!(
        report.verdict.failures.iter().all(expected_failure),
        "H-RING emitted an unexpected oracle failure outside the explicit capacity fail-safe: {:?}",
        report.verdict.failures
    );
    assert_eq!(
        report
            .verdict
            .failures
            .iter()
            .filter(|failure| matches!(failure, OracleFailure::EndProgress { .. }))
            .count(),
        2,
        "exactly the observer and target must end fail-closed"
    );
    for peer in [OBSERVER, TARGET] {
        assert_eq!(
            report
                .verdict
                .failures
                .iter()
                .filter(|failure| matches!(
                    failure,
                    OracleFailure::EndProgress { peer: actual, .. } if *actual == peer
                ))
                .count(),
            1,
            "expected one end-progress failure for peer {peer}"
        );
    }
    for peer in 0..4 {
        assert_eq!(
            report
                .verdict
                .failures
                .iter()
                .filter(|failure| matches!(
                    failure,
                    OracleFailure::PostHealLiveness { peer: actual, .. } if *actual == peer
                ))
                .count(),
            1,
            "expected one bounded post-heal failure for peer {peer}"
        );
    }
    assert_eq!(
        report
            .trace_tail
            .last()
            .and_then(|snapshot| snapshot.session_states.get(OBSERVER)),
        Some(&TraceSessionState::Synchronizing),
        "the high observer must remain in the explicit fail-closed state"
    );

    let replay = run(&schedule, &options);
    assert_eq!(report.trace_hash, replay.trace_hash);
    assert_eq!(report.receipt_range_probe, replay.receipt_range_probe);
}
