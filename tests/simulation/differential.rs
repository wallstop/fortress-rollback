//! Bounded two-peer differential rollback oracle.
//!
//! The independent reference covers the nominal input stream, one strict
//! post-synchronization input-delay increase, and the exact `StateStub`
//! parity/sum transition, plus spectator display of that fixed history. It does
//! not claim equivalence for arbitrary game state semantics or runtime
//! membership changes.

use super::harness::artifact::{read_artifact, FailureArtifact, FAILURE_ARTIFACT_SCHEMA_VERSION};
use super::harness::oracle::{InputFingerprint, OracleFailure};
use super::harness::schedule::{
    BackgroundNoise, FrameModel, SavePolicy, Schedule, ScheduleEvent, SimConfig,
    SCHEDULE_SCHEMA_VERSION,
};
use super::harness::shrink::{shrink_failure, ShrinkConfig};
use super::harness::{
    actual_differential_input, reference_inputs, run, run_with_final_game_frame_offset,
    run_with_input, validate_run_options, DifferentialInputDelayControl, DifferentialMutation,
    DifferentialOptions, DifferentialTrajectory, PeerDriveOrder, RunOptions, RunReport, SimInput,
    WideStubInput,
};
use crate::common::sim_net::LinkPolicy;
use crate::common::stubs::StubInput;
use fortress_rollback::hash::fnv1a_hash;
use fortress_rollback::InputStatus;
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

fn assert_spectator_non_vacuous(report: &RunReport) {
    let history = spectator_inputs(report);
    assert!(
        history.len() >= 30,
        "spectator reference coverage was vacuous: {} frames",
        history.len()
    );
    for (frame, inputs) in history {
        assert_eq!(inputs.len(), 2, "spectator frame {frame} player count");
        assert!(
            inputs
                .iter()
                .all(|(_, status)| *status == InputStatus::Confirmed),
            "nominal spectator frame {frame} has a non-confirmed input: {inputs:?}"
        );
    }
}

fn assert_green_non_vacuous(report: &RunReport, delivery: Delivery) {
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
    assert_eq!(trajectory.confirmed_inputs.len(), 2);
    assert_eq!(trajectory.confirmed_states.len(), 2);
    assert!(
        trajectory
            .confirmed_inputs
            .iter()
            .all(|history| history.len() >= 30),
        "reference input coverage was vacuous: {trajectory:?}"
    );
    assert!(
        trajectory
            .confirmed_states
            .iter()
            .all(|history| history.len() >= 30),
        "reference state coverage was vacuous: {trajectory:?}"
    );
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
                assert_spectator_non_vacuous(&report);

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
    assert_spectator_non_vacuous(&report);
    assert_dynamic_confirmed_boundaries(&report);
    assert_post_control_rollback_evidence(&report, 120);

    let control = input_delay_control(&report);
    let resume = control.current_frame + 4;
    let history = spectator_inputs(&report);
    for frame in 0..=resume {
        let expected = reference_inputs::<StubInput>(frame, 2, 2, Some(control))
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
    let at = |frame| reference_inputs::<StubInput>(frame, 0, 2, Some(control));

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
        reference_inputs::<StubInput>(frame, 2, 2, Some(control))
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

    let mut wrong_size = valid_schedule.clone();
    wrong_size.config.n_players = 3;
    assert!(validate_run_options(&wrong_size, &valid_options).is_err());

    let mut wrong_cadence = valid_schedule.clone();
    wrong_cadence.config.frame_model = FrameModel::SkewGated60Hz;
    assert!(validate_run_options(&wrong_cadence, &valid_options).is_err());

    let mut wrong_delay = valid_schedule.clone();
    wrong_delay.config.input_delay = 1;
    assert!(validate_run_options(&wrong_delay, &valid_options).is_err());

    let mut no_prediction = valid_schedule.clone();
    no_prediction.config.max_prediction = 0;
    assert!(validate_run_options(&no_prediction, &valid_options).is_err());

    let mut spectator = valid_schedule.clone();
    spectator.config.spectator_hosts = vec![0];
    validate_run_options(&spectator, &valid_options)
        .expect("a preplanned spectator uses the same fixed input history");

    let forbidden_events = [
        ScheduleEvent::GracefulRemove { by: 0, target: 1 },
        ScheduleEvent::LegacyDisconnect { by: 0, target: 1 },
        ScheduleEvent::PeerKill { peer: 1 },
        ScheduleEvent::SpectatorHostKill { host: 0 },
        ScheduleEvent::HotJoin { slot: 1 },
        ScheduleEvent::Rebind { peer: 1 },
    ];
    for event in forbidden_events {
        let mut invalid = valid_schedule.clone();
        invalid.events = vec![(1, event)];
        assert!(validate_run_options(&invalid, &valid_options).is_err());
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
