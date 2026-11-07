//! Bounded two-peer differential rollback oracle.
//!
//! The independent reference covers the fixed nominal input stream and the
//! exact `StateStub` parity/sum transition. It does not claim equivalence for
//! arbitrary game state semantics, runtime membership changes, or spectators.

use super::harness::artifact::{read_artifact, FailureArtifact, FAILURE_ARTIFACT_SCHEMA_VERSION};
use super::harness::oracle::OracleFailure;
use super::harness::schedule::{
    BackgroundNoise, FrameModel, SavePolicy, Schedule, ScheduleEvent, SimConfig,
    SCHEDULE_SCHEMA_VERSION,
};
use super::harness::shrink::{shrink_failure, ShrinkConfig};
use super::harness::{
    actual_differential_input, run, run_with_input, validate_run_options, DifferentialMutation,
    DifferentialOptions, DifferentialTrajectory, PeerDriveOrder, RunOptions, RunReport,
    WideStubInput,
};
use crate::common::sim_net::LinkPolicy;
use fortress_rollback::hash::fnv1a_hash;
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
    assert!(validate_run_options(&spectator, &valid_options).is_err());

    let forbidden_events = [
        ScheduleEvent::SetInputDelay { peer: 0, delay: 2 },
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
