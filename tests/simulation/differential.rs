//! Bounded differential rollback oracle.
//!
//! The independent reference covers the nominal input stream, one strict
//! post-synchronization input-delay increase, and the exact `StateStub`
//! parity/sum transition, plus spectator display of that fixed history. Bounded
//! two-peer synchronous and three-peer asynchronous explicit removals, plus
//! three-peer automatic removals, freeze the target at their certified cut;
//! redundant spectator rows cover surviving-host priority orders and the exact
//! frozen fingerprint/status boundary. A four-peer prepared row composes the
//! first explicit removal with a player host crash, modeling both certified
//! cuts independently while the survivors converge and the spectator fails
//! over. Two- and three-peer hot-join rows either rewrite the snapshot frame
//! `A` (two-peer) or keep it as a frozen bridge and activate at `A + 1`
//! (N-peer). Input-queue capacity rows fill the queue from both the delay and
//! prediction sides, and a minimal-prediction row exercises long-run saved-state
//! ring wrap. Other membership widths and arbitrary game state semantics remain
//! outside this oracle's claim.

use super::harness::artifact::{read_artifact, FailureArtifact, FAILURE_ARTIFACT_SCHEMA_VERSION};
use super::harness::oracle::{InputFingerprint, OracleFailure};
#[cfg(feature = "hot-join")]
use super::harness::reference_inputs_with_cuts;
use super::harness::schedule::{
    BackgroundNoise, DropPolicy, FrameModel, SavePolicy, Schedule, ScheduleEvent, SimConfig,
    SCHEDULE_SCHEMA_VERSION,
};
use super::harness::shrink::{shrink_failure, ShrinkConfig};
#[cfg(feature = "hot-join")]
use super::harness::DifferentialHotJoinControl;
use super::harness::{
    actual_differential_input, peer_addr, reference_inputs, run, run_with_final_game_frame_offset,
    run_with_input, validate_run_options, DifferentialInputDelayControl, DifferentialMutation,
    DifferentialOptions, DifferentialRemovalControl, DifferentialTrajectory, PeerDriveOrder,
    PeerEventKey, PeerEventPayload, RunOptions, RunReport, SimInput, WideStubInput,
};
use crate::common::sim_net::LinkPolicy;
use crate::common::stubs::{StateStub, StubInput};
use fortress_rollback::hash::fnv1a_hash;
use fortress_rollback::{EventKind, InputStatus, PlayerHandle};
use std::collections::BTreeMap;
use std::time::Duration;

const STEPS: u32 = 240;
const SEED: u64 = 0xD1FF_E3E1;
const TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Copy, Debug)]
enum Delivery {
    Delayed,
    Held,
    Reordered,
}

fn schedule(save_mode: SavePolicy, input_delay: usize, delivery: Delivery) -> Schedule {
    let config = SimConfig {
        n_players: 2,
        steps: STEPS,
        input_delay,
        save_mode,
        noise: BackgroundNoise::Clean,
        ..SimConfig::smoke(2)
    };
    let delayed = LinkPolicy {
        base_delay: Duration::from_millis(64),
        ..LinkPolicy::clean()
    };
    let reordered = LinkPolicy {
        base_delay: Duration::from_millis(20),
        jitter: Duration::from_millis(80),
        ..LinkPolicy::clean()
    };
    let forward_policy = match delivery {
        Delivery::Reordered => reordered,
        Delivery::Delayed | Delivery::Held => delayed.clone(),
    };
    let initial_links = vec![(0, 1, forward_policy), (1, 0, delayed)];
    let events = if matches!(delivery, Delivery::Held) {
        vec![
            (
                40,
                ScheduleEvent::Hold {
                    from: 0,
                    to: 1,
                    holding: true,
                },
            ),
            (
                100,
                ScheduleEvent::Hold {
                    from: 0,
                    to: 1,
                    holding: false,
                },
            ),
        ]
    } else {
        Vec::new()
    };
    Schedule {
        schema_version: SCHEDULE_SCHEMA_VERSION,
        seed: SEED,
        link_seed: SEED ^ 0x1111_2222_3333_4444,
        config,
        initial_links,
        events,
        heal_at: STEPS,
    }
}

fn runtime_delay_schedule(
    save_mode: SavePolicy,
    old_delay: usize,
    new_delay: usize,
    delivery: Delivery,
) -> Schedule {
    let mut schedule = schedule(save_mode, old_delay, delivery);
    schedule.events.push((
        120,
        ScheduleEvent::SetInputDelay {
            peer: 1,
            delay: new_delay,
        },
    ));
    schedule.events.sort_by_key(|(step, _)| *step);
    schedule
}

fn graceful_remove_schedule(
    save_mode: SavePolicy,
    input_delay: usize,
    delivery: Delivery,
) -> Schedule {
    graceful_remove_schedule_peers(2, save_mode, input_delay, delivery)
}

fn graceful_remove_schedule_peers(
    n: usize,
    save_mode: SavePolicy,
    input_delay: usize,
    delivery: Delivery,
) -> Schedule {
    let mut schedule = schedule(save_mode, input_delay, delivery);
    if n != 2 {
        schedule.config.n_players = n;
        let forward_policy = schedule.initial_links[0].2.clone();
        let delayed_policy = schedule.initial_links[1].2.clone();
        let mut initial_links = Vec::new();
        for from in 0..n {
            for to in 0..n {
                if from != to {
                    let policy = if from == 0 {
                        &forward_policy
                    } else {
                        &delayed_policy
                    };
                    initial_links.push((from, to, policy.clone()));
                }
            }
        }
        schedule.initial_links = initial_links;
    }
    schedule
        .events
        .push((120, ScheduleEvent::GracefulRemove { by: 0, target: 1 }));
    schedule.events.sort_by_key(|(step, _)| *step);
    schedule
}

fn automatic_disconnect_schedule(
    save_mode: SavePolicy,
    input_delay: usize,
    delivery: Delivery,
) -> Schedule {
    let mut schedule = schedule(save_mode, input_delay, delivery);
    let forward_policy = schedule.initial_links[0].2.clone();
    let delayed_policy = schedule.initial_links[1].2.clone();
    schedule.config.n_players = 3;
    schedule.config.disconnect_behavior = DropPolicy::ContinueWithout;
    let mut initial_links = Vec::new();
    for from in 0..3 {
        for to in 0..3 {
            if from != to {
                let policy = if from == 0 {
                    &forward_policy
                } else {
                    &delayed_policy
                };
                initial_links.push((from, to, policy.clone()));
            }
        }
    }
    schedule.initial_links = initial_links;
    schedule
        .events
        .push((80, ScheduleEvent::PeerKill { peer: 2 }));
    schedule.events.sort_by_key(|(step, _)| *step);
    schedule
}

#[cfg(feature = "hot-join")]
fn hot_join_schedule() -> Schedule {
    hot_join_schedule_with_peers(2)
}

#[cfg(feature = "hot-join")]
fn hot_join_schedule_with_peers(n: usize) -> Schedule {
    let mut schedule = schedule(SavePolicy::EveryFrame, 0, Delivery::Delayed);
    schedule.config.max_prediction = 8;
    if n != 2 {
        schedule.config.n_players = n;
        // The N-peer serve pauses the whole mesh at the snapshot frame while
        // the replacement, the coordinator, and every survivor converge on the
        // reactivation handshake, so the bounded row needs headroom to observe
        // confirmed post-activation frames.
        schedule.config.steps = 400;
        schedule.heal_at = 400;
        let forward_policy = schedule.initial_links[0].2.clone();
        let delayed_policy = schedule.initial_links[1].2.clone();
        let mut initial_links = Vec::new();
        for from in 0..n {
            for to in 0..n {
                if from != to {
                    let policy = if from == 0 {
                        &forward_policy
                    } else {
                        &delayed_policy
                    };
                    initial_links.push((from, to, policy.clone()));
                }
            }
        }
        schedule.initial_links = initial_links;
    }
    // Hold the replacement's handshake long enough to force a non-empty
    // post-cut freeze window before activation.
    schedule
        .events
        .push((100, ScheduleEvent::HotJoin { slot: 1 }));
    schedule.events.push((
        100,
        ScheduleEvent::Hold {
            from: 1,
            to: 0,
            holding: true,
        },
    ));
    schedule.events.push((
        116,
        ScheduleEvent::Hold {
            from: 1,
            to: 0,
            holding: false,
        },
    ));
    schedule.events.sort_by_key(|(step, _)| *step);
    schedule
}

/// Hot-join replacement for slot 1 while the coordinator's removal certificate
/// for slot 2 is still in flight. `remove_player(1)` is queued behind the first
/// certificate, so both membership changes overlap at step 100.
#[cfg(feature = "hot-join")]
fn hot_join_during_removal_schedule() -> Schedule {
    let mut schedule = hot_join_schedule_with_peers(3);
    schedule.events.retain(|(_, event)| {
        !matches!(
            event,
            ScheduleEvent::HotJoin { .. } | ScheduleEvent::Hold { .. }
        )
    });
    schedule
        .events
        .push((100, ScheduleEvent::GracefulRemove { by: 0, target: 2 }));
    schedule
        .events
        .push((100, ScheduleEvent::HotJoin { slot: 1 }));
    schedule.events.sort_by_key(|(step, _)| *step);
    schedule
}

#[cfg(feature = "hot-join")]
fn hot_join_control(report: &RunReport) -> DifferentialHotJoinControl {
    report
        .differential_trajectory
        .as_ref()
        .and_then(|trajectory| trajectory.hot_join_control)
        .expect("successful hot-join is anchored")
}

#[cfg(feature = "hot-join")]
fn hot_join_reference(frame: i32, n: usize, control: DifferentialHotJoinControl) -> Vec<StubInput> {
    // Two-peer activation rewrites the snapshot frame itself. An N-peer serve
    // keeps the snapshot as a bridge frame and activates at `S + 1`.
    if frame > control.snapshot_frame || (n == 2 && frame == control.snapshot_frame) {
        return reference_inputs::<StubInput>(frame, 0, n, None, None);
    }
    let frozen = DifferentialRemovalControl {
        by: control.host,
        target: control.target,
        current_frame: control.freeze_frame,
        freeze_frame: control.freeze_frame,
        commit_step: control.removal_step,
        load_count_at_commit: 0,
    };
    reference_inputs::<StubInput>(frame, 0, n, None, Some(frozen))
}

fn options(order: PeerDriveOrder, mutation: Option<DifferentialMutation>) -> RunOptions {
    RunOptions {
        disconnect_timeout: Some(TIMEOUT),
        differential: Some(DifferentialOptions {
            drive_order: order,
            mutation,
        }),
        ..RunOptions::default()
    }
}

fn automatic_options(order: PeerDriveOrder, mutation: Option<DifferentialMutation>) -> RunOptions {
    RunOptions {
        disconnect_timeout: Some(Duration::from_secs(2)),
        ..options(order, mutation)
    }
}

fn total_metric(
    report: &RunReport,
    metric: impl Fn(&fortress_rollback::SessionMetrics) -> u64,
) -> u64 {
    report
        .metrics
        .iter()
        .map(metric)
        .fold(0_u64, u64::saturating_add)
}

/// Certified-prefix projection shared by redundant-spectator cross-order checks.
type SurvivorPrefixProjection = (
    i32,
    Option<BTreeMap<i32, Vec<(InputFingerprint, InputStatus)>>>,
    Vec<(
        BTreeMap<i32, Vec<InputFingerprint>>,
        BTreeMap<i32, StateStub>,
    )>,
);

fn prefix_until<T: Clone>(map: &BTreeMap<i32, T>, bound: i32) -> BTreeMap<i32, T> {
    map.range(..=bound)
        .map(|(frame, value)| (*frame, value.clone()))
        .collect()
}

fn stub_input(ordinal: u32, peer: usize) -> StubInput {
    let peer = u32::try_from(peer).expect("peer index fits in u32");
    StubInput {
        inp: ordinal
            .wrapping_mul(31)
            .wrapping_add(peer.wrapping_mul(7))
            .wrapping_add(1),
    }
}

fn stub_fingerprint(ordinal: u32, peer: usize) -> InputFingerprint {
    stub_input(ordinal, peer).fingerprint()
}

fn input_delay_control(report: &RunReport) -> DifferentialInputDelayControl {
    report
        .differential_trajectory
        .as_ref()
        .and_then(|trajectory| trajectory.input_delay_control)
        .expect("successful runtime delay change is anchored")
}

fn removal_control(report: &RunReport) -> DifferentialRemovalControl {
    report
        .differential_trajectory
        .as_ref()
        .and_then(|trajectory| trajectory.removal_control)
        .expect("successful removal is anchored")
}

fn assert_dynamic_confirmed_boundaries(report: &RunReport) {
    let control = input_delay_control(report);
    assert!(control.current_frame > 0);
    assert!(control.new_delay > control.old_delay);
    let old_delay = i32::try_from(control.old_delay).expect("old delay fits in i32");
    let new_delay = i32::try_from(control.new_delay).expect("new delay fits in i32");
    let gap_start = control.current_frame + old_delay;
    let resume = control.current_frame + new_delay;
    let trajectory = report
        .differential_trajectory
        .as_ref()
        .expect("differential trajectory retained");

    for observer in 0..2 {
        let history = &trajectory.confirmed_inputs[observer];
        for frame in 0..=resume {
            let inputs = history.get(&frame).unwrap_or_else(|| {
                panic!("observer {observer} never confirmed boundary-prefix frame {frame}")
            });
            assert_eq!(inputs.len(), 2);
            for player in 0..2 {
                let ordinal = if player == control.peer {
                    if frame < old_delay {
                        None
                    } else if frame < gap_start {
                        Some(frame - old_delay)
                    } else if frame < resume {
                        Some(control.current_frame - 1)
                    } else {
                        Some(frame - new_delay)
                    }
                } else if frame < old_delay {
                    None
                } else {
                    Some(frame - old_delay)
                };
                let expected = ordinal.map_or_else(
                    || StubInput::default().fingerprint(),
                    |ordinal| {
                        stub_fingerprint(
                            u32::try_from(ordinal).expect("ordinal non-negative"),
                            player,
                        )
                    },
                );
                assert_eq!(
                    inputs.get(player),
                    Some(&expected),
                    "observer {observer}, player {player}, frame {frame}, control {control:?}"
                );
            }
        }
    }
}

fn assert_post_control_rollback_evidence(report: &RunReport, event_step: u32) {
    let at_control = report
        .differential_input_delay_metrics_at_control
        .as_ref()
        .expect("successful control captures rollback baselines");
    assert_eq!(at_control.len(), report.metrics.len());
    assert!(
        report
            .metrics
            .iter()
            .zip(at_control)
            .any(|(after, before)| after.rollback_count > before.rollback_count),
        "no post-control rollback: before={at_control:?}, after={:?}",
        report.metrics
    );
    assert!(
        report
            .metrics
            .iter()
            .zip(at_control)
            .any(|(after, before)| after.resimulated_frames > before.resimulated_frames),
        "no post-control resimulation: before={at_control:?}, after={:?}",
        report.metrics
    );
    assert!(
        report
            .load_game_state_observations
            .iter()
            .any(|observation| observation.step >= event_step),
        "no post-control LoadGameState: {:?}",
        report.load_game_state_observations
    );
}

fn assert_post_removal_rollback_evidence(report: &RunReport) {
    let control = removal_control(report);
    let at_control = report
        .differential_removal_metrics_at_control
        .as_ref()
        .expect("successful removal captures rollback baselines");
    assert_eq!(at_control.len(), report.metrics.len());
    assert!(
        report
            .metrics
            .iter()
            .zip(at_control)
            .any(|(after, before)| after.rollback_count > before.rollback_count),
        "no post-removal rollback: before={at_control:?}, after={:?}",
        report.metrics
    );
    assert!(
        report
            .metrics
            .iter()
            .zip(at_control)
            .any(|(after, before)| after.resimulated_frames > before.resimulated_frames),
        "no post-removal resimulation: before={at_control:?}, after={:?}",
        report.metrics
    );
    assert!(
        report
            .load_game_state_observations
            .iter()
            .skip(control.load_count_at_commit)
            .any(|observation| observation.peer != control.target),
        "no survivor LoadGameState after first commit at step {} (load cursor {}): {:?}",
        control.commit_step,
        control.load_count_at_commit,
        report.load_game_state_observations
    );
}

fn assert_graceful_remove_boundaries(report: &RunReport, input_delay: usize) {
    let control = removal_control(report);
    assert_eq!((control.by, control.target), (0, 1));
    assert!(control.current_frame > 0);
    assert!(control.freeze_frame >= 0);
    assert!(report.final_confirmed[0] > control.freeze_frame + 20);
    assert!(report.final_confirmed[1] < report.final_confirmed[0]);
    assert!(report
        .peer_event_counts
        .get(&EventKind::PeerDropped)
        .is_some_and(|count| *count > 0));

    let trajectory = report
        .differential_trajectory
        .as_ref()
        .expect("differential trajectory retained");
    let delay = i32::try_from(input_delay).expect("input delay fits in i32");
    let frozen_ordinal = u32::try_from(control.freeze_frame - delay)
        .expect("test premise places the certified cut beyond the initial delay prefix");
    for frame in control.freeze_frame..=control.freeze_frame + 20 {
        let actual = trajectory.confirmed_inputs[0]
            .get(&frame)
            .unwrap_or_else(|| panic!("survivor never confirmed boundary frame {frame}"));
        let survivor_ordinal = u32::try_from(frame - delay).expect("boundary frame is post-delay");
        let target_ordinal = if frame <= control.freeze_frame {
            survivor_ordinal
        } else {
            frozen_ordinal
        };
        let expected = (0..report.final_confirmed.len())
            .map(|peer| {
                if peer == control.target {
                    stub_fingerprint(target_ordinal, peer)
                } else {
                    stub_fingerprint(survivor_ordinal, peer)
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(actual, &expected, "survivor input frame {frame}");
    }
}

fn assert_automatic_disconnect_boundaries(report: &RunReport, input_delay: usize) {
    let control = removal_control(report);
    assert_eq!(control.target, 2);
    assert!(control.by < 2);
    assert!(control.current_frame > 0);
    assert!(control.freeze_frame >= 0);
    assert!(control.commit_step > 80);

    let trajectory = report
        .differential_trajectory
        .as_ref()
        .expect("differential trajectory retained");
    assert_eq!(trajectory.confirmed_inputs.len(), 3);
    for survivor in 0..2 {
        assert!(report.final_confirmed[survivor] > control.freeze_frame + 20);
        let target_addr = peer_addr(control.target);
        for (kind, payload) in [
            (
                EventKind::NetworkInterrupted,
                PeerEventPayload::Addr(target_addr),
            ),
            (
                EventKind::PeerDropped,
                PeerEventPayload::PlayerAddr {
                    handle: PlayerHandle::new(control.target),
                    addr: target_addr,
                },
            ),
            (EventKind::Disconnected, PeerEventPayload::Addr(target_addr)),
        ] {
            assert_eq!(
                report.peer_event_counts_by_peer[survivor].get(&kind),
                Some(&1),
                "survivor {survivor} must observe exactly one {kind:?} overall"
            );
            let key = PeerEventKey { kind, payload };
            assert_eq!(
                report.peer_event_payload_counts_by_peer[survivor].get(&key),
                Some(&1),
                "survivor {survivor} must observe exactly one {key:?}"
            );
        }
        for frame in control.freeze_frame..=control.freeze_frame + 20 {
            let actual = trajectory.confirmed_inputs[survivor]
                .get(&frame)
                .unwrap_or_else(|| panic!("survivor {survivor} never confirmed frame {frame}"));
            let expected =
                reference_inputs::<StubInput>(frame, input_delay, 3, None, Some(control))
                    .into_iter()
                    .map(SimInput::fingerprint)
                    .collect::<Vec<_>>();
            assert_eq!(actual, &expected, "survivor {survivor}, frame {frame}");
        }
    }

    let frozen = reference_inputs::<StubInput>(
        control.freeze_frame + 1,
        input_delay,
        3,
        None,
        Some(control),
    );
    let nominal =
        reference_inputs::<StubInput>(control.freeze_frame + 1, input_delay, 3, None, None);
    assert_ne!(
        frozen, nominal,
        "the first post-cut input must exercise the freeze, not the nominal stream"
    );
}

fn check_runtime_delay_order(
    schedule: &Schedule,
    order: PeerDriveOrder,
    delivery: Delivery,
) -> DifferentialTrajectory {
    const EVENT_STEP: u32 = 120;

    let options = options(order, None);
    let report = run(schedule, &options);
    report.expect_pass(schedule);
    assert_green_non_vacuous(&report, delivery);
    assert_dynamic_confirmed_boundaries(&report);
    assert_post_control_rollback_evidence(&report, EVENT_STEP);

    let replay = run(schedule, &options);
    replay.expect_pass(schedule);
    assert_eq!(
        input_delay_control(&report),
        input_delay_control(&replay),
        "replay changed the successful control anchor"
    );
    assert_eq!(report.trace_hash, replay.trace_hash);
    assert_eq!(
        report.differential_trajectory,
        replay.differential_trajectory
    );

    let mut no_change = schedule.clone();
    no_change
        .events
        .retain(|(_, event)| !matches!(event, ScheduleEvent::SetInputDelay { .. }));
    let baseline = run(&no_change, &options);
    baseline.expect_pass(&no_change);
    assert_green_non_vacuous(&baseline, delivery);
    assert_eq!(
        baseline
            .differential_trajectory
            .as_ref()
            .and_then(|trajectory| trajectory.input_delay_control),
        None
    );
    assert_ne!(
        report.trace_hash, baseline.trace_hash,
        "matched no-change schedule has an identical trace"
    );
    assert_ne!(
        report.differential_trajectory, baseline.differential_trajectory,
        "matched no-change schedule has an identical confirmed trajectory"
    );

    report
        .differential_trajectory
        .expect("differential trajectory retained")
}

fn spectator_inputs(report: &RunReport) -> &BTreeMap<i32, Vec<(InputFingerprint, InputStatus)>> {
    report
        .differential_trajectory
        .as_ref()
        .and_then(|trajectory| trajectory.spectator_inputs.as_ref())
        .expect("spectator differential trajectory retained")
}

fn assert_spectator_non_vacuous(report: &RunReport, n: usize) {
    let history = spectator_inputs(report);
    assert!(
        history.len() >= 30,
        "spectator reference coverage was vacuous: {} frames",
        history.len()
    );
    for (frame, inputs) in history {
        assert_eq!(inputs.len(), n, "spectator frame {frame} player count");
        assert!(
            inputs
                .iter()
                .all(|(_, status)| *status == InputStatus::Confirmed),
            "nominal spectator frame {frame} has a non-confirmed input: {inputs:?}"
        );
    }
}

fn assert_spectator_removal_boundaries(
    report: &RunReport,
    baseline: &RunReport,
    input_delay: usize,
    n: usize,
) {
    let control = removal_control(report);
    let history = spectator_inputs(report);
    let baseline_history = spectator_inputs(baseline);
    assert!(
        history
            .keys()
            .next_back()
            .is_some_and(|frame| *frame >= control.freeze_frame + 20),
        "spectator did not display the frozen boundary window"
    );
    for frame in control.freeze_frame..=control.freeze_frame + 20 {
        let actual = history
            .get(&frame)
            .unwrap_or_else(|| panic!("spectator never displayed removal boundary frame {frame}"));
        let expected = reference_inputs::<StubInput>(frame, input_delay, n, None, Some(control))
            .into_iter()
            .enumerate()
            .map(|(player, input)| {
                (
                    input.fingerprint(),
                    if player == control.target && frame > control.freeze_frame {
                        InputStatus::Disconnected
                    } else {
                        InputStatus::Confirmed
                    },
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(actual, &expected, "spectator removal frame {frame}");
    }

    let at_cut = &history[&(control.freeze_frame)];
    let baseline_at_cut = &baseline_history[&(control.freeze_frame)];
    assert_eq!(at_cut, baseline_at_cut, "cut frame must remain nominal");
    assert_eq!(at_cut[control.target].1, InputStatus::Confirmed);

    let frozen = &history[&(control.freeze_frame + 1)];
    let baseline_frozen = &baseline_history[&(control.freeze_frame + 1)];
    assert_eq!(frozen[control.target].0, at_cut[control.target].0);
    assert_eq!(frozen[control.target].1, InputStatus::Disconnected);
    assert_eq!(baseline_frozen[control.target].1, InputStatus::Confirmed);
    assert_ne!(
        frozen[control.target].0, baseline_frozen[control.target].0,
        "frozen target fingerprint must differ from matched nominal input"
    );
}

fn assert_green_non_vacuous(report: &RunReport, delivery: Delivery) {
    assert_green_non_vacuous_with_retired_minimum(report, delivery, 30);
}

fn assert_green_non_vacuous_with_retired_minimum(
    report: &RunReport,
    delivery: Delivery,
    retired_minimum: usize,
) {
    assert!(
        total_metric(report, |metrics| metrics.rollback_count) > 0,
        "differential row performed no rollback: {:?}",
        report.metrics
    );
    assert!(
        total_metric(report, |metrics| metrics.resimulated_frames) > 0,
        "differential row resimulated no frames: {:?}",
        report.metrics
    );
    assert!(
        !report.load_game_state_observations.is_empty(),
        "differential row never issued LoadGameState"
    );
    let counter = match delivery {
        Delivery::Delayed => report.net_stats.base_delay_applications,
        Delivery::Held => report.net_stats.held,
        Delivery::Reordered => report.net_stats.reordered_deliveries,
    };
    assert!(counter > 0, "intended {delivery:?} delivery did not fire");
    let trajectory = report
        .differential_trajectory
        .as_ref()
        .expect("differential mode retains normalized history");
    assert_eq!(
        trajectory.confirmed_inputs.len(),
        report.final_confirmed.len()
    );
    assert_eq!(
        trajectory.confirmed_states.len(),
        report.final_confirmed.len()
    );
    let dead = &report
        .trace_tail
        .last()
        .expect("completed run retains a final snapshot")
        .dead;
    assert_eq!(dead.len(), trajectory.confirmed_inputs.len());
    for (peer, history) in trajectory.confirmed_inputs.iter().enumerate() {
        let required = if dead[peer] { retired_minimum } else { 30 };
        assert!(
            history.len() >= required,
            "reference input coverage was vacuous for peer {peer}: {} frames (required {required})",
            history.len()
        );
    }
    for (peer, history) in trajectory.confirmed_states.iter().enumerate() {
        let required = if dead[peer] { retired_minimum } else { 30 };
        assert!(
            history.len() >= required,
            "reference state coverage was vacuous for peer {peer}: {} frames (required {required})",
            history.len()
        );
    }
}

#[test]
fn save_delay_and_drive_order_matrix_matches_nominal_confirmed_history() {
    for save_mode in [SavePolicy::EveryFrame, SavePolicy::Sparse] {
        for input_delay in [0, 2] {
            let delivery = match (save_mode, input_delay) {
                (SavePolicy::EveryFrame, 0) => Delivery::Delayed,
                (SavePolicy::EveryFrame, 2) => Delivery::Reordered,
                (SavePolicy::Sparse, 0) => Delivery::Held,
                (SavePolicy::Sparse, 2) => Delivery::Delayed,
                _ => unreachable!(),
            };
            let schedule = schedule(save_mode, input_delay, delivery);
            let mut canonical: Option<DifferentialTrajectory> = None;
            let mut forward_trace_hash = None;
            for order in [
                PeerDriveOrder::Forward,
                PeerDriveOrder::Reverse,
                PeerDriveOrder::Rotating,
            ] {
                let options = options(order, None);
                let report = run(&schedule, &options);
                report.expect_pass(&schedule);
                assert_green_non_vacuous(&report, delivery);

                // Identical schedule + options replays the complete trace exactly.
                let replay = run(&schedule, &options);
                replay.expect_pass(&schedule);
                assert_eq!(report.trace_hash, replay.trace_hash);
                assert_eq!(report.final_confirmed, replay.final_confirmed);
                assert_eq!(
                    report.differential_trajectory,
                    replay.differential_trajectory
                );

                // Prove the order axis is live with the fixed EveryFrame/delay-0
                // schedule. Other cells need not retain visit-order-distinct
                // trace hashes once their observable event sequences coincide.
                match order {
                    PeerDriveOrder::Forward => forward_trace_hash = Some(report.trace_hash),
                    PeerDriveOrder::Reverse => {
                        if (save_mode, input_delay) == (SavePolicy::EveryFrame, 0) {
                            let forward = forward_trace_hash.expect("forward row runs first");
                            assert_ne!(
                                forward, report.trace_hash,
                                "forward/reverse traces must observe peer visit order"
                            );
                        }
                    },
                    PeerDriveOrder::Rotating => {},
                }

                // Across peer orders compare only normalized confirmed history;
                // visit-order trace contents are not part of that equality claim.
                if let Some(expected) = &canonical {
                    assert_eq!(
                        report.differential_trajectory.as_ref(),
                        Some(expected),
                        "confirmed trajectory changed for {save_mode:?}, delay {input_delay}, {order:?}"
                    );
                } else {
                    canonical = report.differential_trajectory.clone();
                }
            }
        }
    }
}

/// Input-queue capacity boundaries: one row fills the queue from the delay
/// side (`delay + prediction == MAX_FRAME_DELAY`) and one from the prediction
/// side. Both must retain the exact nominal confirmed history past the
/// boundary; a ring/indexing error shows up as a reference mismatch or queue
/// error rather than as a silent stall.
#[test]
fn input_queue_capacity_boundaries_match_nominal_history() {
    for (input_delay, max_prediction, steps) in [
        (
            fortress_rollback::__internal::MAX_FRAME_DELAY - 7,
            7,
            800_u32,
        ),
        (2, fortress_rollback::__internal::MAX_FRAME_DELAY - 2, 400),
    ] {
        let mut schedule = schedule(SavePolicy::EveryFrame, input_delay, Delivery::Delayed);
        schedule.config.max_prediction = max_prediction;
        schedule.config.steps = steps;
        schedule.heal_at = steps;
        let mut canonical: Option<DifferentialTrajectory> = None;
        for order in [
            PeerDriveOrder::Forward,
            PeerDriveOrder::Reverse,
            PeerDriveOrder::Rotating,
        ] {
            let options = options(order, None);
            let report = run(&schedule, &options);
            report.expect_pass(&schedule);
            // A full-delay queue leaves nothing to mispredict, so this row's
            // non-vacuity is exact boundary coverage plus delivered delay, not
            // rollback evidence (rollback rows live in the save/delay matrix).
            assert!(
                report.net_stats.base_delay_applications > 0,
                "delay {input_delay}/prediction {max_prediction} never applied delivery delay"
            );
            assert!(
                report.final_confirmed.iter().all(|frame| *frame > 120),
                "delay {input_delay}/prediction {max_prediction} must confirm past the boundary: {:?}",
                report.final_confirmed
            );
            let trajectory = report
                .differential_trajectory
                .as_ref()
                .expect("differential trajectory retained");
            assert!(
                trajectory.confirmed_states[0].len() > 100,
                "delay {input_delay}/prediction {max_prediction} retained only {} states",
                trajectory.confirmed_states[0].len()
            );
            let boundary = i32::try_from(input_delay.max(max_prediction)).expect("fits i32");
            for frame in [boundary, boundary + 1, boundary + 20] {
                assert!(
                    trajectory.confirmed_inputs[0].contains_key(&frame),
                    "missing confirmed boundary frame {frame}: {:?}",
                    trajectory.confirmed_inputs[0].keys().next_back()
                );
            }
            let replay = run(&schedule, &options);
            replay.expect_pass(&schedule);
            assert_eq!(report.trace_hash, replay.trace_hash);
            assert_eq!(
                report.differential_trajectory,
                replay.differential_trajectory
            );
            if let Some(expected) = &canonical {
                assert_eq!(
                    report.differential_trajectory.as_ref(),
                    Some(expected),
                    "boundary trajectory changed for delay {input_delay}, {order:?}"
                );
            } else {
                canonical = report.differential_trajectory.clone();
            }
        }
    }
}

/// Long-run saved-state ring wrap with a minimal prediction window: the
/// circular buffer holds two cells, so every saved state is overwritten almost
/// immediately and every rollback reloads across a wrap. The confirmed history
/// must still match the independent nominal reference exactly across all drive
/// orders and both save policies; a ring/indexing error surfaces as a reference
/// mismatch or a `LoadGameState` failure rather than as a silent stall.
#[test]
fn saved_state_ring_wrap_with_minimal_prediction_window_matches_nominal_history() {
    for save_mode in [SavePolicy::EveryFrame, SavePolicy::Sparse] {
        let mut schedule = schedule(save_mode, 0, Delivery::Delayed);
        schedule.config.max_prediction = 1;
        schedule.config.steps = 480;
        schedule.heal_at = 480;
        let mut canonical: Option<DifferentialTrajectory> = None;
        for order in [
            PeerDriveOrder::Forward,
            PeerDriveOrder::Reverse,
            PeerDriveOrder::Rotating,
        ] {
            let options = options(order, None);
            let report = run(&schedule, &options);
            report.expect_pass(&schedule);
            assert!(
                report.final_confirmed.iter().all(|frame| *frame > 80),
                "{save_mode:?} with max_prediction 1 must confirm deep into the wrap: {:?}",
                report.final_confirmed
            );
            let trajectory = report
                .differential_trajectory
                .as_ref()
                .expect("differential trajectory retained");
            assert!(
                trajectory.confirmed_states[0].len() > 60,
                "{save_mode:?} retained only {} confirmed states",
                trajectory.confirmed_states[0].len()
            );
            // Frames far past the two-cell ring must still be present and exact.
            for frame in [2, 3, 40, 41, 80] {
                assert!(
                    trajectory.confirmed_inputs[0].contains_key(&frame),
                    "{save_mode:?} missing confirmed frame {frame} after ring wrap: {:?}",
                    trajectory.confirmed_inputs[0].keys().next_back()
                );
            }
            let replay = run(&schedule, &options);
            replay.expect_pass(&schedule);
            assert_eq!(report.trace_hash, replay.trace_hash);
            assert_eq!(
                report.differential_trajectory,
                replay.differential_trajectory
            );
            if let Some(expected) = &canonical {
                assert_eq!(
                    report.differential_trajectory.as_ref(),
                    Some(expected),
                    "ring-wrap trajectory changed for {save_mode:?}, {order:?}"
                );
            } else {
                canonical = report.differential_trajectory.clone();
            }
        }
    }
}

#[test]
fn graceful_remove_matrix_matches_independent_frozen_history() {
    for (save_mode, input_delay, delivery) in [
        (SavePolicy::EveryFrame, 0, Delivery::Reordered),
        (SavePolicy::Sparse, 2, Delivery::Delayed),
    ] {
        let baseline_schedule = schedule(save_mode, input_delay, delivery);
        let schedule = graceful_remove_schedule(save_mode, input_delay, delivery);
        let baseline = run(&baseline_schedule, &options(PeerDriveOrder::Forward, None));
        baseline.expect_pass(&baseline_schedule);
        let mut canonical = None;

        for order in [
            PeerDriveOrder::Forward,
            PeerDriveOrder::Reverse,
            PeerDriveOrder::Rotating,
        ] {
            let options = options(order, None);
            let report = run(&schedule, &options);
            report.expect_pass(&schedule);
            assert_green_non_vacuous(&report, delivery);
            assert_graceful_remove_boundaries(&report, input_delay);
            assert_post_removal_rollback_evidence(&report);
            let control = removal_control(&report);
            let trajectory = report
                .differential_trajectory
                .as_ref()
                .expect("differential trajectory retained");

            let replay = run(&schedule, &options);
            replay.expect_pass(&schedule);
            assert_eq!(report.trace_hash, replay.trace_hash);
            assert_eq!(
                report.differential_trajectory,
                replay.differential_trajectory
            );

            let baseline_states = baseline
                .differential_trajectory
                .as_ref()
                .expect("baseline differential trajectory retained");
            let actual_cut_state = trajectory.confirmed_states[0]
                .get(&(control.freeze_frame + 1))
                .expect("survivor simulated the cut post-state F+1");
            let baseline_cut_state = baseline_states.confirmed_states[0]
                .get(&(control.freeze_frame + 1))
                .expect("matched baseline simulated F+1");
            assert_eq!(
                actual_cut_state, baseline_cut_state,
                "state at F+1 must still consume the nominal cut-frame input"
            );
            let actual_frozen_state = trajectory.confirmed_states[0]
                .get(&(control.freeze_frame + 2))
                .expect("survivor simulated the first frozen-input state F+2");
            let baseline_frozen_state = baseline_states.confirmed_states[0]
                .get(&(control.freeze_frame + 2))
                .expect("matched baseline simulated F+2");
            assert_ne!(
                actual_frozen_state, baseline_frozen_state,
                "state at F+2 must observe the first frozen input"
            );
            let actual_frozen_input = trajectory.confirmed_inputs[0]
                .get(&(control.freeze_frame + 1))
                .expect("survivor confirmed frozen input F+1");
            let baseline_frozen_input = baseline
                .differential_trajectory
                .as_ref()
                .and_then(|baseline| baseline.confirmed_inputs[0].get(&(control.freeze_frame + 1)))
                .expect("matched baseline confirmed input F+1");
            assert_ne!(
                actual_frozen_input, baseline_frozen_input,
                "input F+1 must differ from the matched no-removal control"
            );

            // Only the surviving fixed membership history is cross-order
            // canonical. The departed peer can legally retain different amounts
            // of its abandoned pre-removal trailing generation.
            let projection = (
                control,
                trajectory.confirmed_inputs[0].clone(),
                trajectory.confirmed_states[0].clone(),
            );
            if let Some(expected) = &canonical {
                assert_eq!(
                    &projection, expected,
                    "survivor history changed for {order:?}"
                );
            } else {
                canonical = Some(projection);
            }
        }
    }
}

#[test]
fn spectator_graceful_remove_matrix_matches_frozen_history_and_statuses() {
    for (save_mode, input_delay, delivery) in [
        (SavePolicy::EveryFrame, 0, Delivery::Reordered),
        (SavePolicy::Sparse, 2, Delivery::Delayed),
    ] {
        for (n, hosts) in [
            (2, vec![0]),
            (3, vec![0]),
            (3, vec![2]),
            (3, vec![0, 2]),
            (3, vec![2, 0]),
        ] {
            check_spectator_graceful_remove_row(n, save_mode, input_delay, delivery, hosts);
        }
    }
}

#[track_caller]
fn check_spectator_graceful_remove_row(
    n: usize,
    save_mode: SavePolicy,
    input_delay: usize,
    delivery: Delivery,
    hosts: Vec<usize>,
) {
    {
        let host_count = hosts.len();
        let mut schedule = graceful_remove_schedule_peers(n, save_mode, input_delay, delivery);
        schedule.config.spectator_hosts.clone_from(&hosts);
        let mut baseline_schedule = schedule.clone();
        baseline_schedule
            .events
            .retain(|(_, event)| !matches!(event, ScheduleEvent::GracefulRemove { .. }));
        let baseline = run(&baseline_schedule, &options(PeerDriveOrder::Forward, None));
        baseline.expect_pass(&baseline_schedule);
        assert_spectator_non_vacuous(&baseline, n);
        let mut canonical = None;

        for order in [
            PeerDriveOrder::Forward,
            PeerDriveOrder::Reverse,
            PeerDriveOrder::Rotating,
        ] {
            let options = options(order, None);
            let report = run(&schedule, &options);
            report.expect_pass(&schedule);
            assert_green_non_vacuous(&report, delivery);
            assert_graceful_remove_boundaries(&report, input_delay);
            assert_post_removal_rollback_evidence(&report);
            assert_spectator_removal_boundaries(&report, &baseline, input_delay, n);
            assert_eq!(
                report.spectator_final_hosts,
                Some(host_count),
                "hosts={hosts:?}: every configured surviving host must remain connected"
            );
            assert!(
                report.spectator_applied_frames > 0,
                "hosts={hosts:?}: spectator displayed no frames"
            );
            let control = removal_control(&report);

            let replay = run(&schedule, &options);
            replay.expect_pass(&schedule);
            assert_eq!(report.trace_hash, replay.trace_hash);
            assert_eq!(
                report.differential_trajectory,
                replay.differential_trajectory
            );
            assert_eq!(removal_control(&replay), control);

            let trajectory = report
                .differential_trajectory
                .as_ref()
                .expect("differential trajectory retained");
            // Only the certified prefix of surviving history is cross-order
            // canonical; the asynchronous certificate can land at a different
            // schedule step and every peer may retain order-dependent
            // speculative tails beyond the common confirmed prefix.
            let prefix_bound = control.freeze_frame + 20;
            let projection: SurvivorPrefixProjection = (
                control.freeze_frame,
                trajectory
                    .spectator_inputs
                    .as_ref()
                    .map(|history| prefix_until(history, prefix_bound)),
                (0..n)
                    .filter(|peer| *peer != control.target)
                    .map(|peer| {
                        (
                            prefix_until(&trajectory.confirmed_inputs[peer], prefix_bound),
                            prefix_until(&trajectory.confirmed_states[peer], prefix_bound),
                        )
                    })
                    .collect(),
            );
            if let Some(expected) = &canonical {
                assert_eq!(
                    &projection, expected,
                    "spectator history changed for {order:?}"
                );
            } else {
                canonical = Some(projection);
            }
        }
    }
}

/// A canonical spectator host silenced after a membership change must fail over
/// to the remaining survivor without inventing or losing history. The outage is
/// transport-only (`Block` to the spectator address), so the game mesh keeps its
/// confirmed history; only the spectator's observer feed changes.
#[test]
fn spectator_host_outage_after_removal_fails_over_to_surviving_host() {
    let n = 3;
    // Silencing the canonical host must promote the survivor; silencing the
    // lower-priority host must not disturb the canonical display. Both must
    // keep the exact certified frozen history across the outage.
    for outage_host in [0, 2] {
        check_spectator_host_outage(outage_host, n);
    }
}

#[track_caller]
fn check_spectator_host_outage(outage_host: usize, n: usize) {
    let mut schedule =
        graceful_remove_schedule_peers(n, SavePolicy::EveryFrame, 0, Delivery::Reordered);
    schedule.config.spectator_hosts = vec![0, 2];
    schedule.config.steps = 360;
    schedule.heal_at = 360;
    schedule.events.push((
        140,
        ScheduleEvent::Block {
            from: outage_host,
            to: n,
            blocked: true,
        },
    ));
    schedule.events.push((
        220,
        ScheduleEvent::Block {
            from: outage_host,
            to: n,
            blocked: false,
        },
    ));
    schedule.events.sort_by_key(|(step, _)| *step);
    let mut run_options = options(PeerDriveOrder::Forward, None);
    // Longer than the default 500 ms notify delay, far shorter than the 80-step
    // outage, so the spectator's host endpoint fails inside the blocked window.
    run_options.disconnect_timeout = Some(Duration::from_millis(600));

    let report = run(&schedule, &run_options);
    report.expect_pass(&schedule);
    let control = removal_control(&report);
    let outage_start = report
        .spectator_outage_start_frame
        .expect("the spectator-directed outage is observed");
    assert!(
        outage_start >= control.freeze_frame,
        "outage must begin after the certified removal cut: start={outage_start}, {control:?}"
    );
    assert_eq!(
        report.spectator_min_hosts_during_outage,
        Some(1),
        "the spectator must observe the canonical host failure and retain one survivor"
    );
    assert!(
        report
            .spectator_max_frame_during_outage
            .is_some_and(|frame| frame > outage_start),
        "the surviving host must advance the spectator during the outage: start={outage_start}, max={:?}",
        report.spectator_max_frame_during_outage
    );
    let history = spectator_inputs(&report);
    assert!(
        history
            .keys()
            .next_back()
            .is_some_and(|frame| *frame > outage_start + 5),
        "spectator did not resume past the outage window: {:?}",
        history.keys().next_back()
    );
    for frame in control.freeze_frame..=control.freeze_frame + 10 {
        let actual = history
            .get(&frame)
            .unwrap_or_else(|| panic!("spectator never displayed frame {frame}"));
        let expected = reference_inputs::<StubInput>(frame, 0, n, None, Some(control))
            .into_iter()
            .enumerate()
            .map(|(player, input)| {
                (
                    input.fingerprint(),
                    if player == control.target && frame > control.freeze_frame {
                        InputStatus::Disconnected
                    } else {
                        InputStatus::Confirmed
                    },
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            actual, &expected,
            "spectator frame {frame} across host failover"
        );
    }

    let replay = run(&schedule, &run_options);
    replay.expect_pass(&schedule);
    assert_eq!(report.trace_hash, replay.trace_hash);
    assert_eq!(report.verdict.failures, replay.verdict.failures);
    assert_eq!(
        report.differential_trajectory,
        replay.differential_trajectory
    );
}

#[test]
fn automatic_disconnect_matrix_matches_independent_frozen_history() {
    for (save_mode, input_delay, delivery) in [
        (SavePolicy::EveryFrame, 0, Delivery::Reordered),
        (SavePolicy::Sparse, 2, Delivery::Delayed),
    ] {
        let schedule = automatic_disconnect_schedule(save_mode, input_delay, delivery);
        let mut canonical = None;
        for order in [
            PeerDriveOrder::Forward,
            PeerDriveOrder::Reverse,
            PeerDriveOrder::Rotating,
        ] {
            let options = automatic_options(order, None);
            let report = run(&schedule, &options);
            report.expect_pass(&schedule);
            assert_green_non_vacuous_with_retired_minimum(&report, delivery, 1);
            assert_automatic_disconnect_boundaries(&report, input_delay);
            assert_post_removal_rollback_evidence(&report);
            let control = removal_control(&report);

            let replay = run(&schedule, &options);
            replay.expect_pass(&schedule);
            assert_eq!(report.trace_hash, replay.trace_hash);
            assert_eq!(report.verdict.failures, replay.verdict.failures);
            assert_eq!(
                report.differential_trajectory,
                replay.differential_trajectory
            );
            assert_eq!(removal_control(&replay), control);

            let trajectory = report
                .differential_trajectory
                .as_ref()
                .expect("differential trajectory retained");
            // Only the surviving fixed-membership history is cross-order
            // canonical. The killed peer can retain different abandoned tails.
            let projection = (
                control.target,
                control.freeze_frame,
                trajectory.confirmed_inputs[0].clone(),
                trajectory.confirmed_states[0].clone(),
                trajectory.confirmed_inputs[1].clone(),
                trajectory.confirmed_states[1].clone(),
            );
            if let Some(expected) = &canonical {
                assert_eq!(
                    &projection, expected,
                    "survivor history changed for {order:?}"
                );
            } else {
                canonical = Some(projection);
            }
        }
    }
}

#[test]
fn graceful_remove_reordered_row_prunes_abandoned_target_authorship() {
    let schedule = graceful_remove_schedule(SavePolicy::EveryFrame, 0, Delivery::Reordered);
    let report = run(&schedule, &options(PeerDriveOrder::Forward, None));
    report.expect_pass(&schedule);
    let control = removal_control(&report);
    let trajectory = report
        .differential_trajectory
        .as_ref()
        .expect("differential trajectory retained");
    assert!(
        trajectory.confirmed_inputs[1]
            .keys()
            .any(|frame| *frame > control.freeze_frame),
        "test premise requires target-authored confirmations above F={}: {:?}",
        control.freeze_frame,
        trajectory.confirmed_inputs[1].keys()
    );
    assert!(
        report.trace_tail.iter().any(|snapshot| {
            snapshot.dead[1] && snapshot.game_states[1].frame >= control.freeze_frame + 2
        }),
        "test premise requires an abandoned target state at F+2 or later"
    );
}

#[test]
fn graceful_remove_known_answer_preserves_cut_and_repeats_it_afterward() {
    for (input_delay, freeze_frame) in [(0, 5), (2, 7)] {
        let control = DifferentialRemovalControl {
            by: 0,
            target: 1,
            current_frame: freeze_frame + 2,
            freeze_frame,
            commit_step: 120,
            load_count_at_commit: 0,
        };
        let at = |frame| reference_inputs::<StubInput>(frame, input_delay, 2, None, Some(control));
        let delay = i32::try_from(input_delay).expect("input delay fits in i32");
        let cut_ordinal = u32::try_from(freeze_frame - delay).expect("cut is after delay prefix");
        assert_eq!(
            at(freeze_frame),
            vec![stub_input(cut_ordinal, 0), stub_input(cut_ordinal, 1)]
        );
        for frame in freeze_frame + 1..=freeze_frame + 3 {
            let survivor_ordinal = u32::try_from(frame - delay).expect("frame is after prefix");
            assert_eq!(
                at(frame),
                vec![stub_input(survivor_ordinal, 0), stub_input(cut_ordinal, 1)],
                "frozen frame {frame}"
            );
        }
    }
}

#[test]
fn graceful_remove_frozen_input_mutation_and_artifact_replay_are_detected() {
    let mut schedule = graceful_remove_schedule(SavePolicy::Sparse, 2, Delivery::Delayed);
    schedule.config.spectator_hosts = vec![0];
    let nominal = run(&schedule, &options(PeerDriveOrder::Forward, None));
    nominal.expect_pass(&schedule);
    let control = removal_control(&nominal);
    let mutation = DifferentialMutation::SharedInput {
        ordinal: u32::try_from(control.freeze_frame - 2).expect("cut ordinal is non-negative"),
    };
    let options = options(PeerDriveOrder::Forward, Some(mutation));
    let report = run(&schedule, &options);
    assert!(
        report.verdict.failures.iter().any(|failure| matches!(
            failure,
            OracleFailure::ReferenceInput { peer: 0, frame, .. }
                if *frame == control.freeze_frame + 1
        )),
        "missing survivor frozen-input failure at F+1: {:?}",
        report.verdict.failures
    );
    let spectator_frame = control.freeze_frame + 1;
    let spectator_actual = spectator_inputs(&report)[&spectator_frame][control.target].0;
    let spectator_expected =
        reference_inputs::<StubInput>(spectator_frame, 2, 2, None, Some(control))[control.target]
            .fingerprint();
    assert_ne!(
        spectator_actual, spectator_expected,
        "the retained spectator stream must contain the shared frozen-input lie"
    );
    assert!(!report.verdict.failures.iter().any(|failure| matches!(
        failure,
        OracleFailure::ReferenceState { .. }
            | OracleFailure::ConfirmedInputDivergence { .. }
            | OracleFailure::SpectatorInputDivergence { .. }
    )));

    let artifact = FailureArtifact::from_report(&schedule, &report);
    artifact.validate().expect("removal artifact validates");
    let encoded = serde_json::to_vec(&artifact).expect("removal artifact serializes");
    let decoded: FailureArtifact =
        serde_json::from_slice(&encoded).expect("removal artifact deserializes");
    decoded
        .validate()
        .expect("decoded removal artifact validates");
    let replay = run(&decoded.schedule, &decoded.replay_options);
    assert_eq!(replay.trace_hash, report.trace_hash);
    assert_eq!(
        replay.differential_trajectory,
        report.differential_trajectory
    );
    assert_eq!(replay.verdict.failures, report.verdict.failures);
    assert_eq!(removal_control(&replay), control);
}

#[test]
fn spectator_graceful_remove_rejects_disconnected_status_lie() {
    let mut schedule = graceful_remove_schedule(SavePolicy::EveryFrame, 0, Delivery::Reordered);
    schedule.config.spectator_hosts = vec![0];
    let nominal = run(&schedule, &options(PeerDriveOrder::Forward, None));
    nominal.expect_pass(&schedule);
    let control = removal_control(&nominal);

    let mut options = options(PeerDriveOrder::Forward, None);
    options.corrupt_spectator_status_from = Some(control.freeze_frame + 1);
    let report = run(&schedule, &options);
    assert!(report.verdict.failures.iter().any(|failure| matches!(
        failure,
        OracleFailure::ReferenceInput { peer: 2, frame, .. }
            if *frame == control.freeze_frame + 1
    )));

    let replay = run(&schedule, &options);
    assert_eq!(replay.trace_hash, report.trace_hash);
    assert_eq!(replay.verdict.failures, report.verdict.failures);
    assert_eq!(
        replay.differential_trajectory,
        report.differential_trajectory
    );
}

#[test]
fn graceful_remove_post_freeze_state_and_missing_state_mutations_are_detected() {
    let schedule = graceful_remove_schedule(SavePolicy::EveryFrame, 0, Delivery::Reordered);
    let nominal = run(&schedule, &options(PeerDriveOrder::Reverse, None));
    nominal.expect_pass(&schedule);
    let control = removal_control(&nominal);
    let frame = control.freeze_frame + 2;

    let wrong_state = run(
        &schedule,
        &options(
            PeerDriveOrder::Reverse,
            Some(DifferentialMutation::SharedState { frame }),
        ),
    );
    assert!(wrong_state.verdict.failures.iter().any(|failure| matches!(
        failure,
        OracleFailure::ReferenceState { peer: 0, frame: actual, .. } if *actual == frame
    )));

    let missing_state = run(
        &schedule,
        &options(
            PeerDriveOrder::Reverse,
            Some(DifferentialMutation::MissingState { peer: 0, frame }),
        ),
    );
    assert!(missing_state
        .verdict
        .failures
        .iter()
        .any(|failure| matches!(
            failure,
            OracleFailure::ReferenceStateMissing { peer: 0, frame: actual, .. } if *actual == frame
        )));

    let cut_post_state = control.freeze_frame + 1;
    assert!(nominal.final_confirmed[1] >= cut_post_state);
    let missing_cut_post_state = run(
        &schedule,
        &options(
            PeerDriveOrder::Reverse,
            Some(DifferentialMutation::MissingState {
                peer: 1,
                frame: cut_post_state,
            }),
        ),
    );
    assert!(missing_cut_post_state
        .verdict
        .failures
        .iter()
        .any(|failure| matches!(
            failure,
            OracleFailure::ReferenceStateMissing { peer: 1, frame: actual, .. }
                if *actual == cut_post_state
        )));

    let wrong_final_frame =
        run_with_final_game_frame_offset(&schedule, &options(PeerDriveOrder::Reverse, None), 1, -1);
    assert!(wrong_final_frame
        .verdict
        .failures
        .iter()
        .any(|failure| matches!(
            failure,
            OracleFailure::ReferenceState { peer: 1, expected, actual, .. }
                if actual.frame != expected.frame
        )));
}

#[test]
fn automatic_disconnect_frozen_input_mutation_and_artifact_replay_are_detected() {
    let schedule = automatic_disconnect_schedule(SavePolicy::Sparse, 2, Delivery::Delayed);
    let nominal = run(&schedule, &automatic_options(PeerDriveOrder::Forward, None));
    nominal.expect_pass(&schedule);
    let control = removal_control(&nominal);
    let mutation = DifferentialMutation::SharedInput {
        ordinal: u32::try_from(control.freeze_frame - 2).expect("cut ordinal is non-negative"),
    };
    let options = automatic_options(PeerDriveOrder::Forward, Some(mutation));
    let report = run(&schedule, &options);
    assert!(report.verdict.failures.iter().any(|failure| matches!(
        failure,
        OracleFailure::ReferenceInput { peer: 0 | 1, frame, .. }
            if *frame == control.freeze_frame + 1
    )));
    // A changed logical input may also change the independent reference state;
    // what must remain absent is mere cross-peer disagreement about the shared lie.
    assert!(!report.verdict.failures.iter().any(|failure| matches!(
        failure,
        OracleFailure::ConfirmedInputDivergence { .. } | OracleFailure::StateDivergence { .. }
    )));

    let artifact = FailureArtifact::from_report(&schedule, &report);
    artifact
        .validate()
        .expect("automatic-removal artifact validates");
    let encoded = serde_json::to_vec(&artifact).expect("automatic-removal artifact serializes");
    let decoded: FailureArtifact =
        serde_json::from_slice(&encoded).expect("automatic-removal artifact deserializes");
    decoded.validate().expect("decoded artifact validates");
    let replay = run(&decoded.schedule, &decoded.replay_options);
    assert_eq!(replay.trace_hash, report.trace_hash);
    assert_eq!(replay.verdict.failures, report.verdict.failures);
    assert_eq!(
        replay.differential_trajectory,
        report.differential_trajectory
    );
    assert_eq!(removal_control(&replay), removal_control(&report));
}

#[test]
fn automatic_disconnect_post_freeze_state_and_missing_state_mutations_are_detected() {
    let schedule = automatic_disconnect_schedule(SavePolicy::EveryFrame, 0, Delivery::Reordered);
    let nominal = run(&schedule, &automatic_options(PeerDriveOrder::Reverse, None));
    nominal.expect_pass(&schedule);
    let frame = removal_control(&nominal).freeze_frame + 2;

    let wrong_state = run(
        &schedule,
        &automatic_options(
            PeerDriveOrder::Reverse,
            Some(DifferentialMutation::SharedState { frame }),
        ),
    );
    assert!(wrong_state.verdict.failures.iter().any(|failure| matches!(
        failure,
        OracleFailure::ReferenceState { peer: 0 | 1, frame: actual, .. } if *actual == frame
    )));

    let missing_state = run(
        &schedule,
        &automatic_options(
            PeerDriveOrder::Reverse,
            Some(DifferentialMutation::MissingState { peer: 0, frame }),
        ),
    );
    assert!(missing_state
        .verdict
        .failures
        .iter()
        .any(|failure| matches!(
            failure,
            OracleFailure::ReferenceStateMissing { peer: 0, frame: actual, .. } if *actual == frame
        )));
}

/// Cross-order projection identity for the bounded hot-join row. In this
/// scenario the coordinator's snapshot/activation frame is order-stable, so
/// the complete normalized trajectory (including the activation frame) must be
/// identical across drive orders. If a future row allows the activation frame
/// to differ by order, the projection must exclude that frame instead of
/// weakening the confirmed-prefix comparison.
#[cfg(feature = "hot-join")]
#[test]
fn hot_join_cross_order_projection_is_identical() {
    for n in [2, 3] {
        let schedule = hot_join_schedule_with_peers(n);
        let mut canonical: Option<(DifferentialHotJoinControl, DifferentialTrajectory)> = None;
        for order in [
            PeerDriveOrder::Forward,
            PeerDriveOrder::Reverse,
            PeerDriveOrder::Rotating,
        ] {
            let report = run(&schedule, &options(order, None));
            report.expect_pass(&schedule);
            let observed = (
                hot_join_control(&report),
                report
                    .differential_trajectory
                    .clone()
                    .expect("differential trajectory retained"),
            );
            match &canonical {
                None => canonical = Some(observed),
                Some((control, trajectory)) => {
                    assert_eq!(
                        &observed.0, control,
                        "n={n} order={order:?} must certify the same cut and activation frame"
                    );
                    assert_eq!(
                        &observed.1, trajectory,
                        "n={n} order={order:?} must retain the same normalized trajectory"
                    );
                },
            }
        }
    }
}

#[cfg(feature = "hot-join")]
#[test]
fn hot_join_matches_independent_freeze_and_replacement_history() {
    for n in [2, 3] {
        let schedule = hot_join_schedule_with_peers(n);
        for order in [
            PeerDriveOrder::Forward,
            PeerDriveOrder::Reverse,
            PeerDriveOrder::Rotating,
        ] {
            check_hot_join_order(&schedule, order);
        }
    }
}

#[cfg(feature = "hot-join")]
#[track_caller]
fn check_hot_join_order(schedule: &Schedule, order: PeerDriveOrder) {
    let n = schedule.config.n_players;
    let options = options(order, None);
    let report = run(schedule, &options);
    report.expect_pass(schedule);
    let control = hot_join_control(&report);
    assert_eq!((control.host, control.target), (0, 1));
    assert_eq!(control.removal_step, 100);
    assert!(control.freeze_frame >= 0);
    assert!(control.handoff_floor >= 0);
    assert!(control.handoff_floor <= control.freeze_frame);
    assert!(control.snapshot_frame >= control.handoff_floor);
    assert!(control
        .load_step
        .is_some_and(|load| load > control.removal_step));
    assert!(
        control.snapshot_frame >= control.freeze_frame + 2,
        "held handshake must force a non-empty freeze window: {control:?}"
    );

    let trajectory = report
        .differential_trajectory
        .as_ref()
        .expect("differential trajectory retained");
    let survivors: Vec<usize> = (0..n).filter(|&peer| peer != control.target).collect();
    for &peer in &survivors {
        for frame in control.freeze_frame..=control.freeze_frame + 5 {
            if frame < control.snapshot_frame {
                assert_eq!(
                    trajectory.confirmed_inputs[peer].get(&frame),
                    Some(
                        &hot_join_reference(frame, n, control)
                            .into_iter()
                            .map(SimInput::fingerprint)
                            .collect()
                    ),
                    "survivor {peer} freeze input frame {frame}"
                );
            }
        }
    }
    for frame in control.snapshot_frame..=control.snapshot_frame + 20 {
        let expected = hot_join_reference(frame, n, control)
            .into_iter()
            .map(SimInput::fingerprint)
            .collect::<Vec<_>>();
        for peer in 0..n {
            if report.final_confirmed[peer] < frame {
                continue;
            }
            let actual = trajectory.confirmed_inputs[peer]
                .get(&frame)
                .unwrap_or_else(|| panic!("peer {peer} never confirmed post-resume frame {frame}"));
            assert_eq!(
                actual, &expected,
                "peer {peer} replacement input frame {frame}"
            );
        }
    }

    let frozen = hot_join_reference(control.freeze_frame + 1, n, control);
    let nominal = reference_inputs::<StubInput>(control.freeze_frame + 1, 0, n, None, None);
    assert_ne!(frozen, nominal, "freeze window must differ from nominal");
    let frozen_model = DifferentialRemovalControl {
        by: control.host,
        target: control.target,
        current_frame: control.freeze_frame,
        freeze_frame: control.freeze_frame,
        commit_step: control.removal_step,
        load_count_at_commit: 0,
    };
    let bridge_slot = hot_join_reference(control.snapshot_frame, n, control)[control.target];
    let frozen_at_snapshot =
        reference_inputs::<StubInput>(control.snapshot_frame, 0, n, None, Some(frozen_model))
            [control.target];
    if n == 2 {
        assert_ne!(
            bridge_slot, frozen_at_snapshot,
            "two-peer activation frame must be rewritten to the nominal stream"
        );
    } else {
        assert_eq!(
            bridge_slot, frozen_at_snapshot,
            "N-peer snapshot frame must carry the frozen predecessor as a bridge"
        );
        let activation_slot =
            hot_join_reference(control.snapshot_frame + 1, n, control)[control.target];
        assert_ne!(
            activation_slot, bridge_slot,
            "N-peer activation frame S+1 must author the replacement's input"
        );
    }

    assert!(
        trajectory.confirmed_states[1]
            .keys()
            .next()
            .is_some_and(|frame| *frame == control.snapshot_frame + 1),
        "replacement state history must start at A+1: control={control:?}, final={:?}, states={:?}",
        report.final_confirmed,
        trajectory.confirmed_states[1].keys().next()
    );
    let resumed_state = trajectory.confirmed_states[1][&(control.snapshot_frame + 1)];
    for &peer in &survivors {
        assert_eq!(
            trajectory.confirmed_states[peer].get(&(control.snapshot_frame + 1)),
            Some(&resumed_state),
            "survivor {peer} and replacement must agree on the first post-resume state"
        );
    }

    assert_eq!(
        report.peer_event_counts_by_peer[control.host].get(&EventKind::PeerJoined),
        Some(&1),
        "coordinator must observe exactly one PeerJoined"
    );
    let target_addr = peer_addr(control.target);
    for peer in 0..n {
        if peer == control.target {
            for kind in [
                EventKind::Disconnected,
                EventKind::PeerDropped,
                EventKind::JoinRequested,
                EventKind::PeerJoined,
            ] {
                assert_eq!(
                    report.peer_event_counts_by_peer[peer].get(&kind),
                    None,
                    "replacement slot {peer} must not inherit old-generation {kind:?}"
                );
            }
            continue;
        }
        assert_eq!(
            report.peer_event_counts_by_peer[peer].get(&EventKind::Disconnected),
            Some(&1),
            "survivor {peer} must observe exactly one old-generation Disconnected"
        );
        assert_eq!(
            report.peer_event_counts_by_peer[peer].get(&EventKind::PeerDropped),
            Some(&1),
            "survivor {peer} must observe exactly one old-generation PeerDropped"
        );
        for key in [
            PeerEventKey {
                kind: EventKind::Disconnected,
                payload: PeerEventPayload::Addr(target_addr),
            },
            PeerEventKey {
                kind: EventKind::PeerDropped,
                payload: PeerEventPayload::PlayerAddr {
                    handle: PlayerHandle::new(control.target),
                    addr: target_addr,
                },
            },
        ] {
            assert_eq!(
                report.peer_event_payload_counts_by_peer[peer].get(&key),
                Some(&1),
                "survivor {peer} must observe exactly one {key:?}"
            );
        }
    }
    for &peer in &survivors {
        if peer != control.host {
            for kind in [EventKind::JoinRequested, EventKind::PeerJoined] {
                assert_eq!(
                    report.peer_event_counts_by_peer[peer].get(&kind),
                    None,
                    "non-coordinator survivor {peer} must not emit {kind:?}"
                );
            }
        }
    }
    assert_eq!(
        report.peer_event_counts_by_peer[control.host].get(&EventKind::JoinRequested),
        Some(&1),
        "coordinator must observe exactly one JoinRequested"
    );
    let replay = run(schedule, &options);
    replay.expect_pass(schedule);
    assert_eq!(report.trace_hash, replay.trace_hash);
    assert_eq!(report.verdict.failures, replay.verdict.failures);
    assert_eq!(
        report.differential_trajectory,
        replay.differential_trajectory
    );
    assert_eq!(hot_join_control(&replay), control);
}

#[cfg(feature = "hot-join")]
#[test]
fn hot_join_activation_check_survives_long_drain() {
    // The activation frame is validated while it is still retained, so a long
    // post-resume drain must not turn a correct rewrite into an eviction error.
    let mut schedule = hot_join_schedule();
    schedule.config.steps = 400;
    schedule.heal_at = 400;
    let report = run(&schedule, &options(PeerDriveOrder::Forward, None));
    report.expect_pass(&schedule);
    let control = hot_join_control(&report);
    let activation = control.snapshot_frame;
    let expected = reference_inputs::<StubInput>(activation, 0, 2, None, None)
        .into_iter()
        .map(SimInput::fingerprint)
        .collect::<Vec<_>>();
    let trajectory = report
        .differential_trajectory
        .as_ref()
        .expect("differential trajectory retained");
    assert_eq!(
        trajectory.confirmed_inputs[0].get(&activation),
        Some(&expected),
        "long drain must retain the rewritten activation frame"
    );
    assert!(report.final_confirmed[0] > activation + 128);
}

/// A replacement may rejoin a live mesh while the coordinator's removal
/// certificate for a *different* slot is still in flight. The explicit removal
/// and the hot-join replacement are two independent certified membership cuts;
/// the composed nominal reference must describe both, and no survivor may
/// diverge across either cut or the replacement handoff.
#[cfg(feature = "hot-join")]
#[test]
fn hot_join_during_removal_certificate_certifies_both_cuts() {
    let schedule = hot_join_during_removal_schedule();
    let n = schedule.config.n_players;
    let mut canonical: Option<DifferentialTrajectory> = None;

    for order in [PeerDriveOrder::Forward, PeerDriveOrder::Reverse] {
        let report = run(&schedule, &options(order, None));
        report.expect_pass(&schedule);
        let trajectory = report
            .differential_trajectory
            .as_ref()
            .expect("differential trajectory retained");
        let removal = trajectory
            .removal_control
            .expect("the explicit removal must certify its cut");
        let hot_join = trajectory
            .hot_join_control
            .expect("the replacement must activate");

        assert_eq!((removal.by, removal.target), (0, 2));
        assert_eq!((hot_join.host, hot_join.target), (0, 1));
        assert_eq!(hot_join.removal_step, 100);
        assert!(
            removal.commit_step > hot_join.removal_step,
            "the hot-join must start while the removal certificate is still in flight: {removal:?} vs {hot_join:?}"
        );
        assert!(hot_join.freeze_frame >= 0);
        assert!(hot_join.handoff_floor <= hot_join.freeze_frame);
        assert!(hot_join.snapshot_frame >= hot_join.freeze_frame);
        assert!(hot_join.snapshot_frame > removal.freeze_frame);
        assert!(hot_join
            .load_step
            .is_some_and(|load| load > hot_join.removal_step));

        // Every confirmed frame must match the independent composed reference:
        // peer 2 frozen at the explicit removal cut, peer 1 frozen at the
        // hot-join cut and activated after its snapshot.
        for frame in [
            removal.freeze_frame,
            removal.freeze_frame + 1,
            hot_join.freeze_frame,
            hot_join.freeze_frame + 1,
            hot_join.snapshot_frame,
            hot_join.snapshot_frame + 1,
            hot_join.snapshot_frame + 20,
        ] {
            let expected: Vec<InputFingerprint> = reference_inputs_with_cuts::<StubInput>(
                frame,
                0,
                n,
                None,
                Some(removal),
                None,
                Some(hot_join),
            )
            .into_iter()
            .map(SimInput::fingerprint)
            .collect();
            for peer in 0..n {
                if report.final_confirmed[peer] < frame {
                    continue;
                }
                // A hot-join replacement generation has no retained history
                // below its snapshot frame.
                if peer == hot_join.target && frame < hot_join.snapshot_frame {
                    continue;
                }
                assert_eq!(
                    trajectory.confirmed_inputs[peer].get(&frame),
                    Some(&expected),
                    "{order:?}: peer {peer} composed input frame {frame}, removal={removal:?}, hot_join={hot_join:?}"
                );
            }
        }

        // Exact replay: the same schedule reproduces the same trace.
        let replay = run(&schedule, &options(order, None));
        assert_eq!(report.trace_hash, replay.trace_hash);

        match canonical.as_ref() {
            Some(canonical) => assert_eq!(
                trajectory, canonical,
                "the composed membership trajectory must be drive-order identical"
            ),
            None => canonical = Some(trajectory.clone()),
        }
    }

    // Mutation controls: a frozen-slot lie and a wrong post-activation state
    // must both be detected against the composed reference.
    let nominal = run(&schedule, &options(PeerDriveOrder::Forward, None));
    let trajectory = nominal
        .differential_trajectory
        .as_ref()
        .expect("differential trajectory retained");
    let removal = trajectory.removal_control.expect("removal control");
    let hot_join = trajectory.hot_join_control.expect("hot-join control");

    let frozen_lie = run(
        &schedule,
        &options(
            PeerDriveOrder::Forward,
            Some(DifferentialMutation::SharedInput {
                ordinal: u32::try_from(removal.freeze_frame).expect("non-negative cut"),
            }),
        ),
    );
    assert!(
        frozen_lie.verdict.failures.iter().any(|failure| matches!(
            failure,
            OracleFailure::ReferenceInput { frame, .. } if *frame == removal.freeze_frame
        )),
        "a lie on the explicit removal cut must be detected: {:?}",
        frozen_lie.verdict.failures
    );

    let activation = hot_join.snapshot_frame;
    let wrong_state = run(
        &schedule,
        &options(
            PeerDriveOrder::Forward,
            Some(DifferentialMutation::SharedState {
                frame: activation + 2,
            }),
        ),
    );
    assert!(
        wrong_state.verdict.failures.iter().any(|failure| matches!(
            failure,
            OracleFailure::ReferenceState { frame, .. } if *frame == activation + 2
        )),
        "a wrong post-activation state must be detected: {:?}",
        wrong_state.verdict.failures
    );
}

#[cfg(feature = "hot-join")]
#[test]
fn hot_join_mutations_and_replacement_floor_are_detected() {
    for n in [2, 3] {
        check_hot_join_mutations(n);
    }
}

#[cfg(feature = "hot-join")]
#[track_caller]
fn check_hot_join_mutations(n: usize) {
    let schedule = hot_join_schedule_with_peers(n);
    let nominal = run(&schedule, &options(PeerDriveOrder::Forward, None));
    nominal.expect_pass(&schedule);
    let control = hot_join_control(&nominal);
    let resume = control.snapshot_frame;

    let frozen_lie = run(
        &schedule,
        &options(
            PeerDriveOrder::Forward,
            Some(DifferentialMutation::SharedInput {
                ordinal: u32::try_from(control.freeze_frame).expect("non-negative cut"),
            }),
        ),
    );
    assert!(frozen_lie.verdict.failures.iter().any(|failure| matches!(
        failure,
        OracleFailure::ReferenceInput { peer: 0, frame, .. } if *frame == control.freeze_frame
    )));

    let resume_lie = run(
        &schedule,
        &options(
            PeerDriveOrder::Forward,
            Some(DifferentialMutation::SharedInput {
                ordinal: u32::try_from(resume + 2).expect("non-negative resume"),
            }),
        ),
    );
    assert!(
        resume_lie.verdict.failures.iter().any(|failure| matches!(
            failure,
            OracleFailure::ReferenceInput { frame, .. } if *frame == resume + 2
        )),
        "n={n} resume lie must fail at {}: control={control:?}, failures={:#?}",
        resume + 2,
        resume_lie.verdict.failures
    );

    let wrong_state = run(
        &schedule,
        &options(
            PeerDriveOrder::Forward,
            Some(DifferentialMutation::SharedState { frame: resume + 2 }),
        ),
    );
    assert!(wrong_state.verdict.failures.iter().any(|failure| matches!(
        failure,
        OracleFailure::ReferenceState { frame, .. } if *frame == resume + 2
    )));

    let missing_resume_state = run(
        &schedule,
        &options(
            PeerDriveOrder::Forward,
            Some(DifferentialMutation::MissingState {
                peer: 1,
                frame: resume + 3,
            }),
        ),
    );
    assert!(missing_resume_state
        .verdict
        .failures
        .iter()
        .any(|failure| matches!(
            failure,
            OracleFailure::ReferenceStateMissing { peer: 1, frame, .. } if *frame == resume + 3
        )));

    let missing_snapshot_state = run(
        &schedule,
        &options(
            PeerDriveOrder::Forward,
            Some(DifferentialMutation::MissingState {
                peer: 1,
                frame: resume,
            }),
        ),
    );
    assert!(
        !missing_snapshot_state
            .verdict
            .failures
            .iter()
            .any(|failure| matches!(
                failure,
                OracleFailure::ReferenceStateMissing { peer: 1, frame, .. } if *frame == resume
            )),
        "replacement snapshot frame legitimately has no recorded state"
    );

    let missing_history = run(
        &schedule,
        &options(
            PeerDriveOrder::Forward,
            Some(DifferentialMutation::MissingStateHistory { peer: 1 }),
        ),
    );
    assert!(missing_history
        .verdict
        .failures
        .iter()
        .any(|failure| matches!(
            failure,
            OracleFailure::ReferenceStateMissing { peer: 1, frame, .. } if *frame == resume + 1
        )));

    let artifact = FailureArtifact::from_report(&schedule, &resume_lie);
    artifact.validate().expect("hot-join artifact validates");
    let encoded = serde_json::to_vec(&artifact).expect("hot-join artifact serializes");
    let decoded: FailureArtifact =
        serde_json::from_slice(&encoded).expect("hot-join artifact deserializes");
    decoded
        .validate()
        .expect("decoded hot-join artifact validates");
    let replay = run(&decoded.schedule, &decoded.replay_options);
    assert_eq!(replay.trace_hash, resume_lie.trace_hash);
    assert_eq!(replay.verdict.failures, resume_lie.verdict.failures);
    assert_eq!(
        replay.differential_trajectory,
        resume_lie.differential_trajectory
    );
    assert_eq!(hot_join_control(&replay), control);
}

#[test]
fn spectator_matrix_matches_nominal_history_across_delivery_and_drive_order() {
    for save_mode in [SavePolicy::EveryFrame, SavePolicy::Sparse] {
        for delivery in [Delivery::Delayed, Delivery::Held, Delivery::Reordered] {
            let mut schedule = schedule(save_mode, 0, delivery);
            schedule.config.spectator_hosts = vec![0, 1];
            let mut canonical: Option<DifferentialTrajectory> = None;
            for order in [
                PeerDriveOrder::Forward,
                PeerDriveOrder::Reverse,
                PeerDriveOrder::Rotating,
            ] {
                let options = options(order, None);
                let report = run(&schedule, &options);
                report.expect_pass(&schedule);
                assert_green_non_vacuous(&report, delivery);
                assert_spectator_non_vacuous(&report, 2);

                let replay = run(&schedule, &options);
                replay.expect_pass(&schedule);
                assert_eq!(report.trace_hash, replay.trace_hash);
                assert_eq!(
                    report.differential_trajectory, replay.differential_trajectory,
                    "spectator replay changed normalized history"
                );

                if let Some(expected) = &canonical {
                    assert_eq!(
                        report.differential_trajectory.as_ref(),
                        Some(expected),
                        "spectator trajectory changed for {save_mode:?}, {delivery:?}, {order:?}"
                    );
                } else {
                    canonical = report.differential_trajectory.clone();
                }
            }
        }
    }
}

#[test]
fn spectator_runtime_delay_matches_independent_gap_and_resume() {
    let mut schedule = runtime_delay_schedule(SavePolicy::Sparse, 2, 4, Delivery::Reordered);
    schedule.config.spectator_hosts = vec![0];
    let report = run(&schedule, &options(PeerDriveOrder::Rotating, None));
    report.expect_pass(&schedule);
    assert_green_non_vacuous(&report, Delivery::Reordered);
    assert_spectator_non_vacuous(&report, 2);
    assert_dynamic_confirmed_boundaries(&report);
    assert_post_control_rollback_evidence(&report, 120);

    let control = input_delay_control(&report);
    let resume = control.current_frame + 4;
    let history = spectator_inputs(&report);
    for frame in 0..=resume {
        let expected = reference_inputs::<StubInput>(frame, 2, 2, Some(control), None)
            .into_iter()
            .map(SimInput::fingerprint)
            .collect::<Vec<_>>();
        let actual = history
            .get(&frame)
            .unwrap_or_else(|| panic!("spectator never displayed boundary-prefix frame {frame}"));
        assert_eq!(actual.len(), expected.len());
        for (player, ((actual, status), expected)) in actual.iter().zip(expected).enumerate() {
            assert_eq!(
                actual, &expected,
                "spectator player {player}, frame {frame}"
            );
            assert_eq!(*status, InputStatus::Confirmed);
        }
    }
}

#[test]
fn spectator_reference_rejects_shared_and_spectator_only_input_lies() {
    let mut schedule = schedule(SavePolicy::EveryFrame, 0, Delivery::Delayed);
    schedule.config.spectator_hosts = vec![0, 1];
    let shared = run(
        &schedule,
        &options(
            PeerDriveOrder::Forward,
            Some(DifferentialMutation::SharedInput { ordinal: 4 }),
        ),
    );
    assert!(
        shared
            .verdict
            .failures
            .iter()
            .any(|failure| matches!(failure, OracleFailure::ReferenceInput { peer: 2, .. })),
        "shared wrong inputs must fail the independent spectator reference: {:?}",
        shared.verdict.failures
    );

    let mut spectator_only_options = options(PeerDriveOrder::Forward, None);
    spectator_only_options.corrupt_spectator_input_from = Some(4);
    let spectator_only = run(&schedule, &spectator_only_options);
    assert!(
        spectator_only
            .verdict
            .failures
            .iter()
            .any(|failure| matches!(failure, OracleFailure::ReferenceInput { peer: 2, .. })),
        "a spectator-only lie must fail the independent reference: {:?}",
        spectator_only.verdict.failures
    );
}

#[test]
fn zero_to_three_at_frame_five_has_hard_coded_gap_boundaries() {
    let control = DifferentialInputDelayControl {
        peer: 1,
        current_frame: 5,
        old_delay: 0,
        new_delay: 3,
    };
    let at = |frame| reference_inputs::<StubInput>(frame, 0, 2, Some(control), None);

    assert_eq!(at(0), vec![stub_input(0, 0), stub_input(0, 1)]);
    assert_eq!(at(4), vec![stub_input(4, 0), stub_input(4, 1)]);
    for frame in 5..=7 {
        assert_eq!(
            at(frame),
            vec![
                stub_input(u32::try_from(frame).expect("gap frame non-negative"), 0),
                stub_input(4, 1),
            ],
            "gap frame {frame}"
        );
    }
    assert_eq!(at(8), vec![stub_input(8, 0), stub_input(5, 1)]);
}

#[test]
fn two_to_four_at_frame_eight_has_hard_coded_gap_boundaries() {
    let control = DifferentialInputDelayControl {
        peer: 1,
        current_frame: 8,
        old_delay: 2,
        new_delay: 4,
    };
    let at = |frame| {
        reference_inputs::<StubInput>(frame, 2, 2, Some(control), None)
            .into_iter()
            .map(SimInput::fingerprint)
            .collect::<Vec<_>>()
    };

    for frame in 0..=1 {
        assert_eq!(
            at(frame),
            vec![StubInput::default().fingerprint(); 2],
            "initial default frame {frame}"
        );
    }
    assert_eq!(at(9), vec![stub_fingerprint(7, 0), stub_fingerprint(7, 1)]);
    for frame in 10..=11 {
        assert_eq!(
            at(frame),
            vec![
                stub_fingerprint(
                    u32::try_from(frame - 2).expect("static ordinal non-negative"),
                    0,
                ),
                stub_fingerprint(7, 1),
            ],
            "gap frame {frame}"
        );
    }
    assert_eq!(
        at(12),
        vec![stub_fingerprint(10, 0), stub_fingerprint(8, 1)]
    );
}

#[test]
fn runtime_delay_increase_matrix_matches_controls_and_nominal_history() {
    for save_mode in [SavePolicy::EveryFrame, SavePolicy::Sparse] {
        for delivery in [Delivery::Delayed, Delivery::Held, Delivery::Reordered] {
            let schedule = runtime_delay_schedule(save_mode, 0, 2, delivery);
            let mut canonical: Option<DifferentialTrajectory> = None;
            for order in [
                PeerDriveOrder::Forward,
                PeerDriveOrder::Reverse,
                PeerDriveOrder::Rotating,
            ] {
                let trajectory = check_runtime_delay_order(&schedule, order, delivery);
                if let Some(expected) = &canonical {
                    assert_eq!(
                        trajectory.input_delay_control, expected.input_delay_control,
                        "control anchor changed for {save_mode:?}, {delivery:?}, {order:?}"
                    );
                    assert_eq!(
                        trajectory, *expected,
                        "trajectory changed for {save_mode:?}, {delivery:?}, {order:?}"
                    );
                } else {
                    canonical = Some(trajectory);
                }
            }
        }
    }

    let nonzero = runtime_delay_schedule(SavePolicy::Sparse, 2, 4, Delivery::Reordered);
    let mut canonical: Option<DifferentialTrajectory> = None;
    for order in [
        PeerDriveOrder::Forward,
        PeerDriveOrder::Reverse,
        PeerDriveOrder::Rotating,
    ] {
        let trajectory = check_runtime_delay_order(&nonzero, order, Delivery::Reordered);
        if let Some(expected) = &canonical {
            assert_eq!(trajectory.input_delay_control, expected.input_delay_control);
            assert_eq!(trajectory, *expected);
        } else {
            canonical = Some(trajectory);
        }
    }
}

#[test]
fn runtime_delay_increase_matches_nominal_gap_and_resume() {
    let schedule = runtime_delay_schedule(SavePolicy::EveryFrame, 0, 3, Delivery::Delayed);
    let trajectory =
        check_runtime_delay_order(&schedule, PeerDriveOrder::Forward, Delivery::Delayed);

    let control = trajectory
        .input_delay_control
        .expect("successful runtime delay change is anchored");
    assert!(control.current_frame > 0);
    assert_eq!(control.peer, 1);
    assert_eq!(control.old_delay, 0);
    assert_eq!(control.new_delay, 3);
}

#[test]
fn shared_input_mutation_covers_dynamic_gap_and_resume_boundaries() {
    let schedule = runtime_delay_schedule(SavePolicy::Sparse, 2, 4, Delivery::Delayed);
    let nominal = run(&schedule, &options(PeerDriveOrder::Rotating, None));
    nominal.expect_pass(&schedule);
    let control = input_delay_control(&nominal);
    let mutation = DifferentialMutation::SharedInput {
        ordinal: u32::try_from(control.current_frame - 1).expect("control frame is positive"),
    };
    let report = run(
        &schedule,
        &options(PeerDriveOrder::Rotating, Some(mutation)),
    );
    assert_eq!(input_delay_control(&report), control);

    let gap_start = control.current_frame + 2;
    let resume = control.current_frame + 4;
    let trajectory = report
        .differential_trajectory
        .as_ref()
        .expect("failed differential run retains actual history");
    for frame in gap_start..=resume {
        let ordinal = if frame < resume {
            control.current_frame - 1
        } else {
            control.current_frame
        };
        let ordinal = u32::try_from(ordinal).expect("boundary ordinal non-negative");
        let expected = actual_differential_input(stub_input(ordinal, 1), ordinal, Some(mutation))
            .fingerprint();
        for observer in 0..2 {
            let inputs = trajectory.confirmed_inputs[observer]
                .get(&frame)
                .unwrap_or_else(|| panic!("observer {observer} did not confirm frame {frame}"));
            assert_eq!(
                inputs.get(1),
                Some(&expected),
                "changed player boundary frame {frame}"
            );
        }
        assert!(
            report.verdict.failures.iter().any(|failure| matches!(
                failure,
                OracleFailure::ReferenceInput { frame: failed, .. } if *failed == frame
            )),
            "no typed reference failure at changed-player boundary {frame}"
        );
    }
    assert!(!report.verdict.failures.iter().any(|failure| matches!(
        failure,
        OracleFailure::ReferenceState { .. }
            | OracleFailure::ConfirmedInputDivergence { .. }
            | OracleFailure::StateDivergence { .. }
    )));
}

#[test]
fn shared_state_mutation_stays_live_after_dynamic_resume() {
    let schedule = runtime_delay_schedule(SavePolicy::EveryFrame, 0, 2, Delivery::Delayed);
    let report = run(
        &schedule,
        &options(
            PeerDriveOrder::Reverse,
            Some(DifferentialMutation::SharedState { frame: 100 }),
        ),
    );

    let control = input_delay_control(&report);
    assert!(100 > control.current_frame + 2);
    assert!(report
        .verdict
        .failures
        .iter()
        .any(|failure| matches!(failure, OracleFailure::ReferenceState { frame: 100, .. })));
    assert!(!report.verdict.failures.iter().any(|failure| matches!(
        failure,
        OracleFailure::ReferenceInput { .. }
            | OracleFailure::ConfirmedInputDivergence { .. }
            | OracleFailure::StateDivergence { .. }
    )));
}

#[test]
fn checksum_lie_stays_live_on_dynamic_delay_schedule() {
    let schedule = runtime_delay_schedule(SavePolicy::EveryFrame, 2, 4, Delivery::Reordered);
    let report = run(
        &schedule,
        &options(
            PeerDriveOrder::Rotating,
            Some(DifferentialMutation::ChecksumLie { frame: 100 }),
        ),
    );

    let control = input_delay_control(&report);
    assert!(100 > control.current_frame + 4);
    assert!(report.verdict.failures.iter().any(|failure| matches!(
        failure,
        OracleFailure::ChecksumMismatchMetric { mismatches, .. } if *mismatches > 0
    )));
    assert!(!report.verdict.failures.iter().any(|failure| matches!(
        failure,
        OracleFailure::ReferenceInput { .. }
            | OracleFailure::ReferenceState { .. }
            | OracleFailure::ConfirmedInputDivergence { .. }
            | OracleFailure::StateDivergence { .. }
    )));
}

#[test]
fn dynamic_delay_final_game_session_frame_mismatch_fails_reference() {
    let schedule = runtime_delay_schedule(SavePolicy::Sparse, 2, 4, Delivery::Reordered);
    let report =
        run_with_final_game_frame_offset(&schedule, &options(PeerDriveOrder::Forward, None), 0, -1);

    let control = input_delay_control(&report);
    assert_eq!(control.old_delay, 2);
    assert_eq!(control.new_delay, 4);
    let (frame, expected, actual) = report
        .verdict
        .failures
        .iter()
        .find_map(|failure| match failure {
            OracleFailure::ReferenceState {
                peer: 0,
                frame,
                expected,
                actual,
            } if actual.frame != *frame => Some((*frame, *expected, *actual)),
            _ => None,
        })
        .expect("final game/session frame mismatch reaches the typed reference");
    assert_eq!(expected.frame, frame);
    assert_eq!(actual.frame, frame - 1);
}

#[test]
fn runtime_delay_requires_running_at_a_positive_current_frame() {
    let mut schedule = runtime_delay_schedule(SavePolicy::EveryFrame, 0, 2, Delivery::Delayed);
    let event = schedule
        .events
        .iter_mut()
        .find(|(_, event)| matches!(event, ScheduleEvent::SetInputDelay { .. }))
        .expect("delay event exists");
    event.0 = 0;

    let report = run(&schedule, &options(PeerDriveOrder::Forward, None));
    assert!(report.verdict.failures.iter().any(|failure| matches!(
        failure,
        OracleFailure::SessionError {
            operation: "set_input_delay",
            ..
        }
    )));
    assert_eq!(
        report
            .differential_trajectory
            .as_ref()
            .and_then(|trajectory| trajectory.input_delay_control),
        None
    );
    assert!(report.differential_input_delay_metrics_at_control.is_none());
}

#[test]
fn shared_wrong_committed_inputs_fail_reference_without_legacy_divergence() {
    let schedule = schedule(SavePolicy::Sparse, 2, Delivery::Delayed);
    let mutation = DifferentialMutation::SharedInput { ordinal: 4 };
    let report = run(
        &schedule,
        &options(PeerDriveOrder::Rotating, Some(mutation)),
    );

    let mismatches: Vec<_> = report
        .verdict
        .failures
        .iter()
        .filter_map(|failure| match failure {
            OracleFailure::ReferenceInput {
                frame,
                expected,
                actual,
                ..
            } => Some((*frame, expected, actual)),
            _ => None,
        })
        .collect();
    assert!(
        mismatches.len() > 1,
        "shared input mutation must produce repeated typed mismatches: {mismatches:?}"
    );
    assert_eq!(
        mismatches.first().map(|(frame, _, _)| *frame),
        Some(6),
        "delay 2 plus mutated ordinal 4 must first mismatch committed frame 6"
    );
    assert!(mismatches.iter().all(|(frame, _, _)| *frame >= 6));
    assert!(mismatches
        .iter()
        .all(|(_, expected, actual)| expected != actual));
    assert!(!report.verdict.failures.iter().any(|failure| matches!(
        failure,
        OracleFailure::ConfirmedInputDivergence { .. }
            | OracleFailure::StateDivergence { .. }
            | OracleFailure::ReferenceState { .. }
    )));
}

#[test]
fn shared_wrong_states_fail_reference_without_legacy_state_divergence() {
    let schedule = schedule(SavePolicy::EveryFrame, 0, Delivery::Held);
    let mutation = DifferentialMutation::SharedState { frame: 8 };
    let report = run(&schedule, &options(PeerDriveOrder::Reverse, Some(mutation)));

    let mismatches: Vec<_> = report
        .verdict
        .failures
        .iter()
        .filter_map(|failure| match failure {
            OracleFailure::ReferenceState {
                frame,
                expected,
                actual,
                ..
            } => Some((*frame, expected, actual)),
            _ => None,
        })
        .collect();
    assert!(
        mismatches.len() > 1,
        "shared state mutation must produce repeated exact-state mismatches: {mismatches:?}"
    );
    assert_eq!(
        mismatches.first().map(|(frame, _, _)| *frame),
        Some(8),
        "state mutation must first mismatch its exact configured frame"
    );
    assert!(mismatches.iter().all(|(frame, _, _)| *frame >= 8));
    assert!(mismatches
        .iter()
        .all(|(_, expected, actual)| expected != actual));
    assert!(!report.verdict.failures.iter().any(|failure| matches!(
        failure,
        OracleFailure::ReferenceInput { .. }
            | OracleFailure::ConfirmedInputDivergence { .. }
            | OracleFailure::StateDivergence { .. }
            | OracleFailure::InbandDesyncDetected { .. }
            | OracleFailure::ChecksumMismatchMetric { .. }
    )));
}

#[test]
fn missing_interior_confirmed_state_has_explicit_reference_failure() {
    let schedule = schedule(SavePolicy::EveryFrame, 0, Delivery::Delayed);
    let mutation = DifferentialMutation::MissingState { peer: 1, frame: 8 };
    let report = run(&schedule, &options(PeerDriveOrder::Forward, Some(mutation)));

    assert!(report.verdict.failures.iter().any(|failure| matches!(
        failure,
        OracleFailure::ReferenceStateMissing {
            peer: 1,
            frame: 8,
            confirmed,
        } if *confirmed >= 8
    )));
    let trajectory = report
        .differential_trajectory
        .as_ref()
        .expect("differential trajectory retained");
    assert!(!trajectory.confirmed_states[1].contains_key(&8));
    assert!(trajectory.confirmed_states[1].contains_key(&7));
    assert!(trajectory.confirmed_states[1].contains_key(&9));
    assert!(!report.verdict.failures.iter().any(|failure| matches!(
        failure,
        OracleFailure::ReferenceInput { .. }
            | OracleFailure::ReferenceState { .. }
            | OracleFailure::ConfirmedInputDivergence { .. }
            | OracleFailure::StateDivergence { .. }
    )));
}

#[test]
fn missing_entire_peer_history_has_explicit_reference_failure() {
    let schedule = schedule(SavePolicy::Sparse, 2, Delivery::Reordered);
    let mutation = DifferentialMutation::MissingStateHistory { peer: 1 };
    let report = run(&schedule, &options(PeerDriveOrder::Reverse, Some(mutation)));

    assert!(report.verdict.failures.iter().any(|failure| matches!(
        failure,
        OracleFailure::ReferenceStateMissing {
            peer: 1,
            frame: 1,
            confirmed,
        } if *confirmed >= 30
    )));
    let trajectory = report
        .differential_trajectory
        .as_ref()
        .expect("differential trajectory retained");
    assert!(trajectory.confirmed_states[1].is_empty());
    assert!(trajectory.confirmed_states[0].len() >= 30);
    assert!(!report.verdict.failures.iter().any(|failure| matches!(
        failure,
        OracleFailure::ReferenceInput { .. }
            | OracleFailure::ReferenceState { .. }
            | OracleFailure::ConfirmedInputDivergence { .. }
            | OracleFailure::StateDivergence { .. }
    )));
}

#[test]
fn checksum_lie_fires_actual_detector_while_reference_stays_clean() {
    // Sparse checkpointing does not provide the periodic confirmed checksum
    // history needed by this detector control; use EveryFrame for that premise.
    let schedule = schedule(SavePolicy::EveryFrame, 2, Delivery::Reordered);
    let mutation = DifferentialMutation::ChecksumLie { frame: 8 };
    let report = run(
        &schedule,
        &options(PeerDriveOrder::Rotating, Some(mutation)),
    );

    assert!(
        report.verdict.failures.iter().any(|failure| matches!(
            failure,
            OracleFailure::ChecksumMismatchMetric { mismatches, .. } if *mismatches > 0
        )),
        "checksum-only lie did not reach the detector: failures={:?}, metrics={:?}",
        report.verdict.failures,
        report.metrics
    );
    assert!(!report.verdict.failures.iter().any(|failure| matches!(
        failure,
        OracleFailure::ReferenceInput { .. }
            | OracleFailure::ReferenceState { .. }
            | OracleFailure::ConfirmedInputDivergence { .. }
            | OracleFailure::StateDivergence { .. }
    )));
}

#[test]
fn wide_input_padding_difference_is_detected_by_typed_reference() {
    let schedule = schedule(SavePolicy::Sparse, 2, Delivery::Delayed);
    let mutation = DifferentialMutation::WidePadding { ordinal: 4 };
    let nominal = WideStubInput {
        inp: 0x1234_5678,
        padding: [11, 22, 33, 44, 55, 66, 77],
    };
    let before = actual_differential_input(nominal, 3, Some(mutation));
    assert_eq!(
        before, nominal,
        "mutation must respect its ordinal threshold"
    );
    let mutated = actual_differential_input(nominal, 4, Some(mutation));
    assert_eq!(mutated.inp, nominal.inp, "padding control changed inp");
    assert_ne!(
        mutated.padding[0], nominal.padding[0],
        "padding control did not change padding[0]"
    );
    assert_eq!(
        mutated.padding[1..],
        nominal.padding[1..],
        "padding control must leave all other padding words unchanged"
    );

    let report = run_with_input::<WideStubInput>(
        &schedule,
        &options(PeerDriveOrder::Rotating, Some(mutation)),
    );
    assert!(report
        .verdict
        .failures
        .iter()
        .any(|failure| matches!(failure, OracleFailure::ReferenceInput { .. })));
    assert!(!report.verdict.failures.iter().any(|failure| matches!(
        failure,
        OracleFailure::ReferenceState { .. }
            | OracleFailure::ConfirmedInputDivergence { .. }
            | OracleFailure::StateDivergence { .. }
    )));

    let clean =
        run_with_input::<WideStubInput>(&schedule, &options(PeerDriveOrder::Rotating, None));
    clean.expect_pass(&schedule);
    assert_green_non_vacuous(&clean, Delivery::Delayed);
}

#[test]
#[should_panic(expected = "invalid run options: WidePadding requires 32-byte WideStubInput")]
fn wide_padding_with_stub_input_is_rejected_before_execution() {
    let schedule = schedule(SavePolicy::EveryFrame, 0, Delivery::Delayed);
    let mutation = DifferentialMutation::WidePadding { ordinal: 4 };
    let _ = run(&schedule, &options(PeerDriveOrder::Forward, Some(mutation)));
}

#[test]
fn wide_padding_artifacts_require_width_32_and_valid_wide_replays() {
    let schedule = schedule(SavePolicy::Sparse, 2, Delivery::Delayed);
    let mutation = DifferentialMutation::WidePadding { ordinal: 4 };
    let options = options(PeerDriveOrder::Rotating, Some(mutation));
    let report = run_with_input::<WideStubInput>(&schedule, &options);
    assert_eq!(
        report.verdict.failures.first().map(OracleFailure::class),
        Some("ReferenceInput")
    );

    let artifact = FailureArtifact::from_report(&schedule, &report);
    assert_eq!(artifact.replay_input_width_bytes, 32);
    artifact.validate().expect("valid wide artifact validates");
    let encoded = serde_json::to_vec(&artifact).expect("wide artifact serializes");
    let decoded: FailureArtifact =
        serde_json::from_slice(&encoded).expect("wide artifact deserializes");
    decoded.validate().expect("wide roundtrip validates");
    let replay = run_with_input::<WideStubInput>(&decoded.schedule, &decoded.replay_options);
    assert_eq!(replay.trace_hash, report.trace_hash);
    assert_eq!(
        replay.verdict.failures.first().map(OracleFailure::class),
        Some("ReferenceInput")
    );

    let mut invalid = decoded;
    invalid.replay_input_width_bytes = 4;
    let error = invalid
        .validate()
        .expect_err("WidePadding artifact with narrow input must be rejected");
    assert!(error.contains("replay_input_width_bytes 32"));

    // Exercise the corpus reader boundary: invalid width metadata returns an
    // error during validation, before replay can reach session execution.
    let path = std::env::temp_dir().join(format!(
        "fortress-wide-padding-invalid-{}-{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock after epoch")
            .as_nanos()
    ));
    std::fs::write(
        &path,
        serde_json::to_vec(&invalid).expect("invalid fixture serializes"),
    )
    .expect("invalid fixture writes");
    let error = read_artifact(&path).expect_err("corpus reader rejects invalid width metadata");
    assert!(error.contains("replay_input_width_bytes 32"));
    std::fs::remove_file(path).expect("invalid fixture removes");
}

#[test]
fn differential_options_validate_the_bounded_scenario_contract() {
    let valid_schedule = schedule(SavePolicy::EveryFrame, 0, Delivery::Delayed);
    let valid_options = options(PeerDriveOrder::Forward, None);
    validate_run_options(&valid_schedule, &valid_options).expect("canonical options are valid");

    let mut nominal_three = valid_schedule.clone();
    nominal_three.config.n_players = 3;
    validate_run_options(&nominal_three, &valid_options)
        .expect("a nominal three-peer mesh uses the same fixed reference");
    let mut wrong_size = valid_schedule.clone();
    wrong_size.config.n_players = 4;
    assert!(validate_run_options(&wrong_size, &valid_options).is_err());

    let mut wrong_cadence = valid_schedule.clone();
    wrong_cadence.config.frame_model = FrameModel::SkewGated60Hz;
    assert!(validate_run_options(&wrong_cadence, &valid_options).is_err());

    let mut wrong_delay = valid_schedule.clone();
    wrong_delay.config.input_delay = fortress_rollback::__internal::MAX_FRAME_DELAY;
    assert!(validate_run_options(&wrong_delay, &valid_options).is_err());

    let mut no_prediction = valid_schedule.clone();
    no_prediction.config.max_prediction = 0;
    assert!(validate_run_options(&no_prediction, &valid_options).is_err());

    let mut spectator = valid_schedule.clone();
    spectator.config.spectator_hosts = vec![0];
    validate_run_options(&spectator, &valid_options)
        .expect("a preplanned spectator uses the same fixed input history");

    let forbidden_events = [
        ScheduleEvent::LegacyDisconnect { by: 0, target: 1 },
        ScheduleEvent::PeerKill { peer: 1 },
        ScheduleEvent::SpectatorHostKill { host: 0 },
        ScheduleEvent::Rebind { peer: 1 },
    ];
    for event in forbidden_events {
        let mut invalid = valid_schedule.clone();
        invalid.events = vec![(1, event)];
        assert!(validate_run_options(&invalid, &valid_options).is_err());
    }

    let removal = graceful_remove_schedule(SavePolicy::EveryFrame, 0, Delivery::Delayed);
    validate_run_options(&removal, &valid_options)
        .expect("the bounded two-peer graceful removal is supported");
    let mut duplicate = removal.clone();
    duplicate
        .events
        .push((121, ScheduleEvent::GracefulRemove { by: 0, target: 1 }));
    assert!(validate_run_options(&duplicate, &valid_options).is_err());
    let mut wrong_direction = removal.clone();
    wrong_direction.events = vec![(120, ScheduleEvent::GracefulRemove { by: 1, target: 0 })];
    assert!(validate_run_options(&wrong_direction, &valid_options).is_err());
    let mut with_spectator = removal.clone();
    with_spectator.config.spectator_hosts = vec![0];
    validate_run_options(&with_spectator, &valid_options)
        .expect("a surviving primary host supports spectator removal equivalence");
    let mut removed_primary_spectator = removal.clone();
    removed_primary_spectator.config.spectator_hosts = vec![1, 0];
    validate_run_options(&removed_primary_spectator, &valid_options)
        .expect("a removed canonical host is a runnable fail-closed row");
    let mut removed_only_host = removal.clone();
    removed_only_host.config.spectator_hosts = vec![1];
    assert!(
        validate_run_options(&removed_only_host, &valid_options).is_err(),
        "a host set without any surviving endpoint cannot display the match"
    );
    let mut removed_redundant_host = removal.clone();
    removed_redundant_host.config.spectator_hosts = vec![0, 1];
    validate_run_options(&removed_redundant_host, &valid_options)
        .expect("a surviving canonical host may list the removed endpoint redundantly");
    let removal_three =
        graceful_remove_schedule_peers(3, SavePolicy::EveryFrame, 0, Delivery::Delayed);
    validate_run_options(&removal_three, &valid_options)
        .expect("a three-peer graceful removal certifies the cut asynchronously");
    let mut survivor_hosts = removal_three;
    survivor_hosts.config.spectator_hosts = vec![0, 2];
    validate_run_options(&survivor_hosts, &valid_options)
        .expect("multiple surviving spectator hosts are supported");
    survivor_hosts.config.spectator_hosts = vec![2, 0];
    validate_run_options(&survivor_hosts, &valid_options)
        .expect("surviving host priority order is free");
    survivor_hosts.config.spectator_hosts = vec![0, 1];
    validate_run_options(&survivor_hosts, &valid_options)
        .expect("the removed endpoint may trail a surviving canonical host");
    survivor_hosts.config.spectator_hosts = vec![1, 0];
    validate_run_options(&survivor_hosts, &valid_options)
        .expect("a removed canonical host is a runnable fail-closed row");
    survivor_hosts.config.spectator_hosts = vec![1];
    assert!(
        validate_run_options(&survivor_hosts, &valid_options).is_err(),
        "a host set without any surviving endpoint cannot display the match"
    );

    // A second membership change (player host crash) composes with the first
    // explicit removal only in the bounded four-peer shape.
    let composed = host_crash_after_removal_schedule(2);
    validate_run_options(
        &composed,
        &host_crash_options(PeerDriveOrder::Forward, None),
    )
    .expect("the bounded composed-membership row is supported");
    let composed_rejects = |mutate: &dyn Fn(&mut Schedule)| {
        let mut invalid = composed.clone();
        mutate(&mut invalid);
        assert!(
            validate_run_options(&invalid, &host_crash_options(PeerDriveOrder::Forward, None))
                .is_err(),
            "the invalid composed-membership shape must be rejected"
        );
    };
    composed_rejects(&|schedule| schedule.config.n_players = 3);
    composed_rejects(&|schedule| schedule.config.disconnect_behavior = DropPolicy::Halt);
    composed_rejects(&|schedule| {
        schedule
            .events
            .push((300, ScheduleEvent::SpectatorHostKill { host: 3 }));
    });
    composed_rejects(&|schedule| schedule.config.spectator_hosts = vec![2]);
    composed_rejects(&|schedule| {
        schedule
            .events
            .push((300, ScheduleEvent::PeerKill { peer: 3 }));
    });
    let mut with_delay_change = removal;
    with_delay_change
        .events
        .push((121, ScheduleEvent::SetInputDelay { peer: 0, delay: 2 }));
    assert!(validate_run_options(&with_delay_change, &valid_options).is_err());

    let automatic = automatic_disconnect_schedule(SavePolicy::EveryFrame, 0, Delivery::Reordered);
    let automatic_options = automatic_options(PeerDriveOrder::Forward, None);
    validate_run_options(&automatic, &automatic_options)
        .expect("the bounded three-peer automatic disconnect is supported");
    let mut duplicate_kill = automatic.clone();
    duplicate_kill
        .events
        .push((41, ScheduleEvent::PeerKill { peer: 2 }));
    assert!(validate_run_options(&duplicate_kill, &automatic_options).is_err());
    let mut wrong_kill_target = automatic.clone();
    wrong_kill_target.events = vec![(40, ScheduleEvent::PeerKill { peer: 1 })];
    assert!(validate_run_options(&wrong_kill_target, &automatic_options).is_err());
    let mut halt = automatic.clone();
    halt.config.disconnect_behavior = DropPolicy::Halt;
    assert!(validate_run_options(&halt, &automatic_options).is_err());
    let mut automatic_spectator = automatic.clone();
    automatic_spectator.config.spectator_hosts = vec![0];
    assert!(validate_run_options(&automatic_spectator, &automatic_options).is_err());
    let mut automatic_delay_change = automatic.clone();
    automatic_delay_change
        .events
        .push((120, ScheduleEvent::SetInputDelay { peer: 0, delay: 2 }));
    assert!(validate_run_options(&automatic_delay_change, &automatic_options).is_err());
    let mut automatic_timeout = automatic_options;
    automatic_timeout.disconnect_timeout = None;
    assert!(validate_run_options(&automatic, &automatic_timeout).is_err());
    automatic_timeout.disconnect_timeout = Some(Duration::from_millis(u64::from(STEPS) * 16));
    assert!(validate_run_options(&automatic, &automatic_timeout).is_err());

    #[cfg(feature = "hot-join")]
    {
        let hot = hot_join_schedule();
        validate_run_options(&hot, &valid_options)
            .expect("the bounded two-peer hot-join is supported");
        let hot_three = hot_join_schedule_with_peers(3);
        validate_run_options(&hot_three, &valid_options)
            .expect("the bounded N-peer hot-join is supported");
        let mut wrong_n_peer_slot = hot_three;
        wrong_n_peer_slot
            .events
            .retain(|(_, event)| !matches!(event, ScheduleEvent::HotJoin { .. }));
        wrong_n_peer_slot
            .events
            .push((100, ScheduleEvent::HotJoin { slot: 2 }));
        assert!(validate_run_options(&wrong_n_peer_slot, &valid_options).is_err());
        let mut duplicate = hot.clone();
        duplicate
            .events
            .push((101, ScheduleEvent::HotJoin { slot: 1 }));
        assert!(validate_run_options(&duplicate, &valid_options).is_err());
        let mut wrong_slot = hot.clone();
        wrong_slot
            .events
            .retain(|(_, event)| !matches!(event, ScheduleEvent::HotJoin { .. }));
        wrong_slot
            .events
            .push((100, ScheduleEvent::HotJoin { slot: 0 }));
        assert!(validate_run_options(&wrong_slot, &valid_options).is_err());
        let mut composed_delay = hot.clone();
        composed_delay
            .events
            .push((120, ScheduleEvent::SetInputDelay { peer: 0, delay: 2 }));
        assert!(validate_run_options(&composed_delay, &valid_options).is_err());
        let mut composed_removal = hot.clone();
        composed_removal
            .events
            .push((120, ScheduleEvent::GracefulRemove { by: 0, target: 1 }));
        assert!(validate_run_options(&composed_removal, &valid_options).is_err());
        let mut with_spectator = hot.clone();
        with_spectator.config.spectator_hosts = vec![0];
        assert!(validate_run_options(&with_spectator, &valid_options).is_err());
        let mut nonzero_delay = hot.clone();
        nonzero_delay.config.input_delay = 2;
        assert!(validate_run_options(&nonzero_delay, &valid_options).is_err());
        let mut sparse = hot.clone();
        sparse.config.save_mode = SavePolicy::Sparse;
        assert!(validate_run_options(&sparse, &valid_options).is_err());
        let mut noisy = hot;
        noisy.config.noise = BackgroundNoise::Mild;
        assert!(validate_run_options(&noisy, &valid_options).is_err());
    }

    for (old_delay, new_delay) in [(0, 2), (2, 4)] {
        let dynamic = runtime_delay_schedule(
            SavePolicy::Sparse,
            old_delay,
            new_delay,
            Delivery::Reordered,
        );
        validate_run_options(&dynamic, &valid_options)
            .expect("one strict in-range runtime increase is supported");
    }
    let mut two_changes = runtime_delay_schedule(SavePolicy::Sparse, 0, 2, Delivery::Delayed);
    two_changes
        .events
        .push((121, ScheduleEvent::SetInputDelay { peer: 0, delay: 3 }));
    assert!(validate_run_options(&two_changes, &valid_options).is_err());
    for delay in [0, 2] {
        let mut not_an_increase =
            runtime_delay_schedule(SavePolicy::Sparse, 2, delay, Delivery::Delayed);
        assert!(validate_run_options(&not_an_increase, &valid_options).is_err());
        not_an_increase.events.clear();
        validate_run_options(&not_an_increase, &valid_options)
            .expect("the matched static schedule remains valid");
    }
    let mut wrong_peer = runtime_delay_schedule(SavePolicy::Sparse, 0, 2, Delivery::Delayed);
    wrong_peer.events = vec![(120, ScheduleEvent::SetInputDelay { peer: 2, delay: 2 })];
    assert!(validate_run_options(&wrong_peer, &valid_options).is_err());
    let mut excessive = runtime_delay_schedule(SavePolicy::Sparse, 0, 2, Delivery::Delayed);
    excessive.events = vec![(
        120,
        ScheduleEvent::SetInputDelay {
            peer: 1,
            delay: fortress_rollback::__internal::MAX_FRAME_DELAY + 1,
        },
    )];
    assert!(validate_run_options(&excessive, &valid_options).is_err());
    let mut excessive_storage = runtime_delay_schedule(SavePolicy::Sparse, 0, 2, Delivery::Delayed);
    excessive_storage.config.max_prediction = fortress_rollback::__internal::MAX_FRAME_DELAY;
    excessive_storage.events = vec![(120, ScheduleEvent::SetInputDelay { peer: 1, delay: 2 })];
    assert!(validate_run_options(&excessive_storage, &valid_options).is_err());

    let virtual_run = Duration::from_millis(u64::from(STEPS) * 16);
    let mut timeout = valid_options.clone();
    timeout.disconnect_timeout = Some(virtual_run);
    assert!(validate_run_options(&valid_schedule, &timeout).is_err());
    timeout.disconnect_timeout = None;
    assert!(validate_run_options(&valid_schedule, &timeout).is_err());

    let delay_two = schedule(SavePolicy::Sparse, 2, Delivery::Held);
    validate_run_options(&delay_two, &valid_options).expect("static delay two is supported");
}

#[test]
fn absent_differential_option_preserves_legacy_serde_and_trace() {
    let schedule = schedule(SavePolicy::EveryFrame, 0, Delivery::Delayed);
    let legacy = RunOptions::default();
    let encoded = serde_json::to_value(&legacy).expect("legacy options serialize");
    assert!(encoded
        .as_object()
        .is_some_and(|object| !object.contains_key("differential")));
    let decoded: RunOptions = serde_json::from_value(encoded).expect("legacy options deserialize");
    assert_eq!(decoded, legacy);

    let first = run(&schedule, &legacy);
    // Derived by running this exact schedule in a detached worktree at base
    // commit 6fb81cb, before differential mode existed. The tail digest covers
    // the complete retained trace, not merely another same-code replay.
    assert_eq!(first.trace_hash, 0xB3DC_C4D4_0515_E17C);
    assert_eq!(first.final_confirmed, vec![195, 195]);
    let tail_bytes = serde_json::to_vec(&first.trace_tail).expect("legacy trace tail serializes");
    assert_eq!(fnv1a_hash(&tail_bytes), 0x1E76_F4D0_D0C2_C16F);

    let replay = run(&schedule, &decoded);
    assert_eq!(first.trace_hash, replay.trace_hash);
    assert_eq!(first.trace_tail, replay.trace_tail);
    assert_eq!(first.differential_trajectory, None);
    assert_eq!(replay.differential_trajectory, None);
}

#[test]
fn rotating_sparse_delay_two_artifacts_roundtrip_and_reproduce_reference_failures() {
    for mutation in [
        DifferentialMutation::SharedInput { ordinal: 4 },
        DifferentialMutation::SharedState { frame: 8 },
        DifferentialMutation::MissingState { peer: 1, frame: 8 },
    ] {
        let schedule = schedule(SavePolicy::Sparse, 2, Delivery::Reordered);
        let options = options(PeerDriveOrder::Rotating, Some(mutation));
        let report = run(&schedule, &options);
        let expected_class = match mutation {
            DifferentialMutation::SharedInput { .. } => "ReferenceInput",
            DifferentialMutation::SharedState { .. } => "ReferenceState",
            DifferentialMutation::MissingState { .. } => "ReferenceStateMissing",
            _ => unreachable!(),
        };
        assert_eq!(
            report.verdict.failures.first().map(OracleFailure::class),
            Some(expected_class)
        );

        let artifact = FailureArtifact::from_report(&schedule, &report);
        artifact
            .validate()
            .expect("differential artifact validates");
        assert_eq!(
            artifact.artifact_schema_version,
            FAILURE_ARTIFACT_SCHEMA_VERSION
        );
        let encoded = serde_json::to_vec(&artifact).expect("artifact serializes");
        let decoded: FailureArtifact =
            serde_json::from_slice(&encoded).expect("artifact deserializes");
        decoded.validate().expect("decoded artifact validates");
        assert_eq!(decoded.replay_options, options);
        assert_eq!(decoded.schedule.config.save_mode, SavePolicy::Sparse);
        assert_eq!(decoded.schedule.config.input_delay, 2);

        let replay = run(&decoded.schedule, &decoded.replay_options);
        assert_eq!(replay.trace_hash, report.trace_hash);
        assert_eq!(
            replay.verdict.failures.first().map(OracleFailure::class),
            Some(expected_class)
        );
    }
}

#[test]
fn dynamic_delay_artifact_roundtrip_replays_failure_trace_trajectory_and_control() {
    let schedule = runtime_delay_schedule(SavePolicy::Sparse, 2, 4, Delivery::Reordered);
    let options = options(
        PeerDriveOrder::Rotating,
        Some(DifferentialMutation::SharedState { frame: 100 }),
    );
    let report = run(&schedule, &options);
    assert_eq!(
        report.verdict.failures.first().map(OracleFailure::class),
        Some("ReferenceState")
    );
    let control = input_delay_control(&report);
    assert!(100 > control.current_frame + 4);

    let artifact = FailureArtifact::from_report(&schedule, &report);
    artifact.validate().expect("dynamic artifact validates");
    assert_eq!(
        artifact.artifact_schema_version,
        FAILURE_ARTIFACT_SCHEMA_VERSION
    );
    let encoded = serde_json::to_vec(&artifact).expect("dynamic artifact serializes");
    let decoded: FailureArtifact =
        serde_json::from_slice(&encoded).expect("dynamic artifact deserializes");
    decoded
        .validate()
        .expect("decoded dynamic artifact validates");
    assert_eq!(decoded.schedule, schedule);
    assert_eq!(decoded.replay_options, options);

    let replay = run(&decoded.schedule, &decoded.replay_options);
    assert_eq!(
        replay.verdict.failures.first().map(OracleFailure::class),
        Some("ReferenceState")
    );
    assert_eq!(input_delay_control(&replay), control);
    assert_eq!(replay.trace_hash, report.trace_hash);
    assert_eq!(
        replay.differential_trajectory,
        report.differential_trajectory
    );
}

#[test]
fn differential_shrinking_canonicalizes_save_mode_and_drive_order() {
    let schedule = schedule(SavePolicy::Sparse, 2, Delivery::Delayed);
    let mutation = DifferentialMutation::SharedInput { ordinal: 4 };
    let options = options(PeerDriveOrder::Rotating, Some(mutation));
    let result = shrink_failure(
        &schedule,
        &options,
        "ReferenceInput",
        ShrinkConfig {
            max_runs: 120,
            max_duration: Duration::from_secs(60),
        },
        run,
        |_, _, _| {},
    )
    .expect("differential reference failure shrinks");

    assert_eq!(result.schedule.config.save_mode, SavePolicy::EveryFrame);
    let differential = result
        .options
        .differential
        .expect("shrinking retains differential mode");
    assert_eq!(differential.drive_order, PeerDriveOrder::Forward);
    assert_eq!(differential.mutation, Some(mutation));
    let replay = run(&result.schedule, &result.options);
    assert_eq!(
        replay.verdict.failures.first().map(OracleFailure::class),
        Some("ReferenceInput")
    );
}

/// Normalized certified prefix used for cross-order composed-membership identity.
type CertifiedPrefix = Vec<(
    BTreeMap<i32, Vec<InputFingerprint>>,
    BTreeMap<i32, StateStub>,
)>;

/// A removed player's co-located spectator endpoint is not withdrawn by
/// `remove_player`, so it must never become a source of truth after the
/// certified cut. Two halves of that contract are pinned here:
///
/// - When a **surviving** host is canonical, listing the removed endpoint
///   redundantly after it changes nothing: the spectator displays exactly the
///   certified frozen history that the independent reference derives.
/// - When the **removed** endpoint is canonical, the spectator fails closed:
///   it stops at that endpoint's last forwarded frame and never presents a
///   forked history, even though the surviving mesh keeps confirming.
#[test]
fn removed_player_spectator_endpoint_keeps_certified_history_or_fails_closed() {
    let schedule_with_hosts = |hosts: Vec<usize>| {
        let mut schedule =
            graceful_remove_schedule_peers(3, SavePolicy::EveryFrame, 0, Delivery::Reordered);
        schedule.config.spectator_hosts = hosts;
        schedule.config.steps = 360;
        schedule.heal_at = 360;
        schedule
    };

    // Surviving canonical host first, removed endpoint anywhere after it.
    for hosts in [vec![0, 2, 1], vec![2, 0, 1], vec![0, 1, 2]] {
        let schedule = schedule_with_hosts(hosts.clone());
        let report = run(&schedule, &options(PeerDriveOrder::Forward, None));
        report.expect_pass(&schedule);
        let control = removal_control(&report);
        assert!(
            report
                .spectator_max_frame
                .is_some_and(|frame| frame > control.freeze_frame + 5),
            "hosts {hosts:?} must keep displaying past the cut: max={:?}, {control:?}",
            report.spectator_max_frame
        );
        assert_eq!(
            report.spectator_final_hosts,
            Some(hosts.len()),
            "hosts {hosts:?} must retain the removed endpoint without losing a source"
        );
        assert_eq!(
            report
                .peer_event_counts
                .get(&EventKind::SpectatorDivergence)
                .copied()
                .unwrap_or(0),
            0,
            "hosts {hosts:?} must not invent a divergence"
        );
    }

    // Removed canonical host: fail closed, never advance past its last frame.
    let schedule = schedule_with_hosts(vec![1, 0, 2]);
    let report = run(&schedule, &options(PeerDriveOrder::Forward, None));
    let progress_missing: Vec<_> = report
        .verdict
        .failures
        .iter()
        .filter_map(|failure| match failure {
            OracleFailure::SpectatorProgressMissing {
                required_min_frame,
                observed_max_frame,
                ..
            } => Some((*required_min_frame, *observed_max_frame)),
            _ => None,
        })
        .collect();
    let (required, observed) = progress_missing
        .first()
        .copied()
        .expect("the stale canonical host must leave the display behind the mesh");
    assert!(
        observed.is_some_and(|frame| frame < required.unwrap_or(i32::MAX)),
        "the display must stop short of the mesh progress: required={required:?}, observed={observed:?}"
    );
    assert!(
        report.final_confirmed[0] > observed.unwrap_or(i32::MAX),
        "survivors must keep confirming past the stalled display: {:?}",
        report.final_confirmed
    );
    for failure in &report.verdict.failures {
        assert!(
            matches!(failure, OracleFailure::SpectatorProgressMissing { .. }),
            "the stale canonical host must fail closed without forking history: {failure:?}"
        );
    }
}

/// A player host crash composed with the first explicit removal: the second
/// certified membership cut must be modeled independently (each target freezes
/// at its own cut), the two survivors must stay on one confirmed history, and
/// the spectator must fail over to the surviving host after the killed one goes
/// silent. Four peers are required so two independent survivors remain.
#[test]
fn host_crash_after_removal_certifies_second_cut_and_spectator_failover() {
    for killed_host in [2_usize, 3] {
        let schedule = host_crash_after_removal_schedule(killed_host);
        let mut canonical: Option<CertifiedPrefix> = None;
        for order in [
            PeerDriveOrder::Forward,
            PeerDriveOrder::Reverse,
            PeerDriveOrder::Rotating,
        ] {
            let report = run(&schedule, &host_crash_options(order, None));
            report.expect_pass(&schedule);
            let first = removal_control(&report);
            let trajectory = report
                .differential_trajectory
                .as_ref()
                .expect("differential trajectory retained");
            let second = trajectory
                .second_removal_control
                .unwrap_or_else(|| panic!("killed host {killed_host} has no second certified cut"));
            assert_eq!(second.target, killed_host);
            assert!(
                second.freeze_frame > first.freeze_frame,
                "the second cut must follow the first removal: first={first:?}, second={second:?}"
            );
            assert!(
                report
                    .spectator_max_frame
                    .is_some_and(|frame| frame > second.freeze_frame + 5),
                "the spectator must keep displaying past the second cut: max={:?}, second={second:?}",
                report.spectator_max_frame
            );
            assert_eq!(
                report.spectator_final_hosts,
                Some(2),
                "the spectator must fail over to the surviving hosts after the kill"
            );
            // Both survivors converge past the second cut, and the killed peer
            // stops at its own certified cut.
            for (peer, confirmed) in report.final_confirmed.iter().enumerate() {
                if peer == first.target || peer == killed_host {
                    continue;
                }
                assert!(
                    *confirmed > second.freeze_frame + 5,
                    "survivor {peer} must keep confirming past the second cut: {:?} vs {second:?}",
                    report.final_confirmed
                );
            }
            assert_eq!(
                report
                    .peer_event_counts
                    .get(&EventKind::SpectatorDivergence)
                    .copied()
                    .unwrap_or(0),
                0,
                "host failover must not invent a spectator divergence"
            );

            // Cross-order identity covers the certified prefix both survivors
            // must share; per-order control bookkeeping (commit step, sampled
            // tail) is deliberately excluded, matching the other removal rows.
            let bound = second.freeze_frame;
            let projection: CertifiedPrefix = (0..schedule.config.n_players)
                .map(|peer| {
                    (
                        prefix_until(&trajectory.confirmed_inputs[peer], bound),
                        prefix_until(&trajectory.confirmed_states[peer], bound),
                    )
                })
                .collect();
            if let Some(expected) = &canonical {
                assert_eq!(
                    &projection, expected,
                    "composed-membership certified prefix changed for kill {killed_host}, {order:?}"
                );
            } else {
                canonical = Some(projection);
            }

            let replay = run(&schedule, &host_crash_options(order, None));
            replay.expect_pass(&schedule);
            assert_eq!(report.trace_hash, replay.trace_hash);
            assert_eq!(
                report.differential_trajectory,
                replay.differential_trajectory
            );
        }
    }
}

#[test]
fn host_crash_after_removal_post_cut_state_mutation_is_detected() {
    let schedule = host_crash_after_removal_schedule(2);
    let nominal = run(
        &schedule,
        &host_crash_options(PeerDriveOrder::Forward, None),
    );
    nominal.expect_pass(&schedule);
    let second = nominal
        .differential_trajectory
        .as_ref()
        .and_then(|trajectory| trajectory.second_removal_control)
        .expect("second certified cut");
    let frame = second.freeze_frame + 3;
    let report = run(
        &schedule,
        &host_crash_options(
            PeerDriveOrder::Forward,
            Some(DifferentialMutation::SharedState { frame }),
        ),
    );
    assert!(
        report
            .verdict
            .failures
            .iter()
            .any(|failure| matches!(failure, OracleFailure::ReferenceState { .. })),
        "a wrong post-second-cut state must be detected: {:?}",
        report.verdict.failures
    );
}

fn host_crash_options(order: PeerDriveOrder, mutation: Option<DifferentialMutation>) -> RunOptions {
    RunOptions {
        // Far shorter than the 240-step kill-to-end window, far longer than the
        // scheduled delivery delay, so the killed host is certified as dropped.
        disconnect_timeout: Some(Duration::from_millis(600)),
        ..options(order, mutation)
    }
}

fn host_crash_after_removal_schedule(killed_host: usize) -> Schedule {
    let mut schedule =
        graceful_remove_schedule_peers(4, SavePolicy::EveryFrame, 0, Delivery::Reordered);
    schedule.config.disconnect_behavior = DropPolicy::ContinueWithout;
    schedule.config.spectator_hosts = vec![0, 2, 3];
    schedule.config.steps = 480;
    schedule.heal_at = 480;
    schedule
        .events
        .push((240, ScheduleEvent::SpectatorHostKill { host: killed_host }));
    schedule.events.sort_by_key(|(step, _)| *step);
    schedule
}
