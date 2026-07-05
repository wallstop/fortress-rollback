//! Simulation fleet: seeded whole-mesh runs checked by the global oracle.
//!
//! The PR smoke fleet runs a fixed seed set per mesh size on every PR. The
//! meta-determinism and negative-control tests validate the harness itself:
//! a harness whose invariants cannot fire, or whose runs are not reproducible,
//! proves nothing.

use super::harness::schedule::{
    generate, BackgroundNoise, DropPolicy, Schedule, ScheduleEvent, SimConfig,
    SCHEDULE_SCHEMA_VERSION,
};
use super::harness::{oracle::OracleFailure, run, RunOptions};
use crate::common::sim_net::LinkPolicy;
use std::time::Duration;

/// Fixed PR-smoke seed set. Nightly fleets (later milestone) randomize seeds
/// from the CI run id; the PR set is fixed so PR failures are reproducible
/// verbatim from the log.
const PR_SMOKE_SEEDS: [u64; 8] = [1, 2, 3, 5, 8, 13, 21, 34];

fn run_smoke_fleet(n_players: usize) {
    for seed in PR_SMOKE_SEEDS {
        let schedule = generate(seed, SimConfig::smoke(n_players));
        let report = run(&schedule, &RunOptions::default());
        report.expect_pass(&schedule);
    }
}

#[test]
fn pr_smoke_two_player_mesh_holds_invariants() {
    run_smoke_fleet(2);
}

#[test]
fn pr_smoke_three_player_mesh_holds_invariants() {
    run_smoke_fleet(3);
}

#[test]
fn pr_smoke_four_player_mesh_holds_invariants() {
    run_smoke_fleet(4);
}

/// M2 §5.2: the always-on [`SessionMetrics`] counters must actually be wired to
/// the paths they measure. The PR-smoke fleet runs mild-loss meshes for 600
/// frames, which reliably drives rollbacks, prediction misses, and periodic
/// desync-checksum comparisons. This asserts, per peer, the structural
/// identities that must hold by construction, and mesh-wide that the network
/// paths feeding these counters are exercised (so none is silently dead).
#[test]
fn session_metrics_are_wired_across_smoke_fleet() {
    let mut total_visual = 0u64;
    let mut total_rollbacks = 0u64;
    let mut total_prediction_misses = 0u64;
    let mut total_checksum_comparisons = 0u64;
    let mut total_checksum_mismatches = 0u64;

    for n_players in [2usize, 3, 4] {
        for seed in PR_SMOKE_SEEDS {
            let schedule = generate(seed, SimConfig::smoke(n_players));
            let report = run(&schedule, &RunOptions::default());
            report.expect_pass(&schedule);

            for (i, m) in report.metrics.iter().enumerate() {
                let ctx = || format!("peer {i} seed {seed} n{n_players}: {m:?}");
                // Total simulation work splits exactly into forward (visual)
                // advances plus rollback re-simulation.
                assert_eq!(
                    m.frames_advanced,
                    m.visual_frames + m.resimulated_frames,
                    "frames_advanced != visual + resimulated — {}",
                    ctx()
                );
                // The depth histogram accounts for every rollback exactly once.
                assert_eq!(
                    m.rollback_depth_histogram.total(),
                    m.rollback_count,
                    "histogram total != rollback_count — {}",
                    ctx()
                );
                // Checksum comparisons split cleanly into matches + mismatches.
                assert_eq!(
                    m.checksums_compared,
                    m.checksums_matched + m.checksums_mismatched,
                    "checksum split mismatch — {}",
                    ctx()
                );
                // Every peer that reached a passing end-state advanced forward.
                assert!(m.visual_frames > 0, "no forward advances — {}", ctx());
                // A clean (green) run must never observe a checksum mismatch.
                assert_eq!(
                    m.checksums_mismatched,
                    0,
                    "unexpected checksum mismatch on a clean run — {}",
                    ctx()
                );

                total_visual += m.visual_frames;
                total_rollbacks += m.rollback_count;
                total_prediction_misses += m.prediction_miss_count;
                total_checksum_comparisons += m.checksums_compared;
                total_checksum_mismatches += m.checksums_mismatched;
            }
        }
    }

    assert!(total_visual > 0, "fleet recorded no forward advances");
    assert!(total_rollbacks > 0, "fleet recorded no rollbacks");
    assert!(
        total_prediction_misses > 0,
        "fleet recorded no prediction misses"
    );
    assert!(
        total_checksum_comparisons > 0,
        "fleet recorded no checksum comparisons"
    );
    assert_eq!(
        total_checksum_mismatches, 0,
        "clean fleet must have zero checksum mismatches"
    );
}

/// Meta-determinism: the same schedule must produce a bit-identical trace.
/// This guards the harness itself — any hidden wall-clock read, unseeded RNG,
/// or iteration-order nondeterminism in session, net, or harness breaks it.
#[test]
fn same_schedule_produces_identical_trace() {
    let schedule = generate(7, SimConfig::smoke(3));
    let first = run(&schedule, &RunOptions::default());
    let second = run(&schedule, &RunOptions::default());
    assert_eq!(
        first.trace_hash, second.trace_hash,
        "same schedule must reproduce the exact trace (final_confirmed {:?} vs {:?})",
        first.final_confirmed, second.final_confirmed
    );
    assert_eq!(first.final_confirmed, second.final_confirmed);
    assert_eq!(first.net_stats, second.net_stats);
}

/// Negative control (real divergence): corrupting one peer's simulated state
/// mid-run must trip BOTH detection layers — the oracle's recorded-state
/// comparison and the library's in-band desync detector.
#[test]
fn oracle_and_inband_detector_catch_seeded_state_divergence() {
    let schedule = generate(11, SimConfig::smoke(2));
    let options = RunOptions {
        corrupt_state_from: Some((1, 40)),
        ..RunOptions::default()
    };
    let report = run(&schedule, &options);
    assert!(
        !report.verdict.passed(),
        "a corrupted peer must fail the run"
    );
    assert!(
        report
            .verdict
            .failures
            .iter()
            .any(|f| matches!(f, OracleFailure::StateDivergence { .. })),
        "oracle state comparison must catch the divergence: {:?}",
        report.verdict.failures
    );
    assert!(
        report
            .verdict
            .failures
            .iter()
            .any(|f| matches!(f, OracleFailure::InbandDesyncDetected { .. })),
        "the library's desync detection must also catch it: {:?}",
        report.verdict.failures
    );
}

/// Negative control (detector cross-check): corrupting only the *checksums*
/// (states stay identical) must fire the in-band detector while the oracle's
/// state comparison stays clean — the false-positive-detector direction of
/// the cross-check.
#[test]
fn inband_detector_fires_on_checksum_lies_while_states_agree() {
    let schedule = generate(13, SimConfig::smoke(2));
    let options = RunOptions {
        corrupt_checksum_from: Some((0, 40)),
        ..RunOptions::default()
    };
    let report = run(&schedule, &options);
    assert!(
        report
            .verdict
            .failures
            .iter()
            .any(|f| matches!(f, OracleFailure::InbandDesyncDetected { .. })),
        "checksum lies must fire the in-band detector: {:?}",
        report.verdict.failures
    );
    assert!(
        !report
            .verdict
            .failures
            .iter()
            .any(|f| matches!(f, OracleFailure::StateDivergence { .. })),
        "states are identical — the oracle state comparison must stay clean: {:?}",
        report.verdict.failures
    );
}

/// Pinned regression: the cold-start gossip-mute mesh deadlock found by this
/// fleet on its first four-player run (seed 1). Transient startup loss froze
/// third-party connect-status caches at `Frame::NULL`; once every peer
/// exhausted its prediction window with fully-acked send queues, no gossip
/// carrier remained and the mesh deadlocked permanently at `confirmed == -1`
/// on a healed network with every session `Running`. Fixed by generalizing
/// the connect-status nudge to fire whenever the mesh-gossip fold holds
/// confirmation below local receipts (`P2PSession::
/// gossip_holds_confirmation_below_receipts`). Pinned separately from
/// `PR_SMOKE_SEEDS` so seed-set evolution can never unpin it.
#[test]
fn regression_cold_start_gossip_stall_seed1_four_players() {
    let schedule = generate(1, SimConfig::smoke(4));
    let report = run(&schedule, &RunOptions::default());
    report.expect_pass(&schedule);
}

/// Oracle-integrity negative control at N=16 (PLAN.md Part V): the Part III
/// H-16P "green at 16" verdict is only meaningful if the oracle still has
/// teeth at that scale — a corrupted peer in a 16-mesh must fail the run
/// exactly as it does in the N=2 control. Guards against the oracle silently
/// losing coverage as N grows (e.g., sampling or comparison short-circuits).
#[test]
#[ignore = "large-mesh oracle control; promote with the N=16 probes"]
fn oracle_catches_seeded_divergence_in_sixteen_player_mesh() {
    let schedule = generate(11, SimConfig::smoke(16));
    let options = RunOptions {
        corrupt_state_from: Some((7, 40)),
        ..RunOptions::default()
    };
    let report = run(&schedule, &options);
    assert!(
        !report.verdict.passed(),
        "a corrupted peer must fail an N=16 run"
    );
    assert!(
        report
            .verdict
            .failures
            .iter()
            .any(|f| matches!(f, OracleFailure::StateDivergence { .. })),
        "oracle state comparison must catch the divergence at N=16: {:?}",
        report.verdict.failures
    );
}

/// Meta-determinism at N=16: bit-identical trace reproduction must survive
/// the largest supported mesh (guards the harness itself at scale).
#[test]
#[ignore = "large-mesh determinism control; promote with the N=16 probes"]
fn same_schedule_produces_identical_trace_at_sixteen_players() {
    let schedule = generate(7, SimConfig::smoke(16));
    let first = run(&schedule, &RunOptions::default());
    let second = run(&schedule, &RunOptions::default());
    assert_eq!(
        first.trace_hash, second.trace_hash,
        "N=16 must reproduce bit-identically (final_confirmed {:?} vs {:?})",
        first.final_confirmed, second.final_confirmed
    );
}

/// Large-mesh probe fleets (N=12/16): the H-16P experiment rows. `#[ignore]`d
/// until the fleet proves them sustainably green and CI budgets are
/// recalibrated; run manually:
///
/// ```text
/// cargo nextest run --no-capture --run-ignored ignored-only -E 'test(probe_smoke_fleet)'
/// ```
#[test]
#[ignore = "large-mesh probe; promote to PR smoke after budget calibration"]
fn probe_smoke_fleet_twelve_player_mesh_holds_invariants() {
    run_smoke_fleet(12);
}

#[test]
#[ignore = "large-mesh probe; promote to PR smoke after budget calibration"]
fn probe_smoke_fleet_sixteen_player_mesh_holds_invariants() {
    run_smoke_fleet(16);
}

/// H-TCP-lite transport-model probe (PLAN.md §13 H-TCP): a reliable,
/// in-order, loss-free link profile (TCP / WebRTC-reliable model) with
/// head-of-line-blocking stalls approximated by capture-and-FIFO-release
/// holds. Zero jitter keeps `SimNet`'s per-link FIFO exact, so delivery is
/// in-order — the TCP contract. Each 25-step hold (≈400ms virtual) models a
/// retransmit stall far exceeding the 8-frame prediction window, so the
/// session must stall-and-catch-up, never diverge.
///
/// Expected green: the protocol's correctness must not depend on loss or
/// reordering being present. A failure here is a real transport-model bug.
fn run_tcp_model_mesh(n: usize) {
    let config = SimConfig {
        n_players: n,
        steps: 900,
        ..SimConfig::smoke(n)
    };
    let fifo = LinkPolicy {
        drop_rate: 0.0,
        dup_rate: 0.0,
        base_delay: Duration::from_millis(30),
        jitter: Duration::ZERO,
        burst_rate: 0.0,
        burst_len: 0,
    };
    let mut initial_links = Vec::new();
    for from in 0..n {
        for to in 0..n {
            if from != to {
                initial_links.push((from, to, fifo.clone()));
            }
        }
    }
    // Three HOL stalls on distinct directed links, well before heal.
    let hold_windows: [(usize, usize, u32); 3] = [(0, 1, 100), (1, 0, 250), (n - 1, 0, 400)];
    let mut events = Vec::new();
    for (from, to, start) in hold_windows {
        events.push((
            start,
            ScheduleEvent::Hold {
                from,
                to,
                holding: true,
            },
        ));
        events.push((
            start + 25,
            ScheduleEvent::Hold {
                from,
                to,
                holding: false,
            },
        ));
    }
    let heal_at = 650;
    events.push((heal_at, ScheduleEvent::HealAll));
    events.sort_by_key(|(step, _)| *step);
    let schedule = Schedule {
        schema_version: SCHEDULE_SCHEMA_VERSION,
        // Hand-built (not generator-derived); seeds recorded for provenance.
        seed: 0,
        link_seed: 0,
        config,
        initial_links,
        events,
        heal_at,
    };
    let report = run(&schedule, &RunOptions::default());
    report.expect_pass(&schedule);
}

#[test]
fn tcp_model_reliable_fifo_two_player_mesh_holds_invariants() {
    run_tcp_model_mesh(2);
}

#[test]
fn tcp_model_reliable_fifo_four_player_mesh_holds_invariants() {
    run_tcp_model_mesh(4);
}

#[test]
#[ignore = "large-mesh transport probe; promote after budget calibration"]
fn tcp_model_reliable_fifo_sixteen_player_mesh_holds_invariants() {
    run_tcp_model_mesh(16);
}

/// Builds the split-brain schedule: a clean 4-mesh symmetrically partitioned
/// into halves {0,1} × {2,3} from step 100 until `HealAll` at step 650 —
/// 550 steps ≈ 8.8 virtual seconds, far beyond the 2 s disconnect timeout,
/// so both halves fully commit to dropping the other before the network
/// heals.
fn split_brain_schedule(policy: DropPolicy) -> Schedule {
    let n = 4;
    let config = SimConfig {
        n_players: n,
        steps: 900,
        disconnect_behavior: policy,
        noise: BackgroundNoise::Clean,
        ..SimConfig::smoke(n)
    };
    let mut initial_links = Vec::new();
    for from in 0..n {
        for to in 0..n {
            if from != to {
                initial_links.push((from, to, LinkPolicy::clean()));
            }
        }
    }
    let mut events = Vec::new();
    for &a in &[0usize, 1] {
        for &b in &[2usize, 3] {
            for (from, to) in [(a, b), (b, a)] {
                events.push((
                    100,
                    ScheduleEvent::Block {
                        from,
                        to,
                        blocked: true,
                    },
                ));
            }
        }
    }
    let heal_at = 650;
    events.push((heal_at, ScheduleEvent::HealAll));
    events.sort_by_key(|(step, _)| *step);
    Schedule {
        schema_version: SCHEDULE_SCHEMA_VERSION,
        seed: 0,
        link_seed: 0,
        config,
        initial_links,
        events,
        heal_at,
    }
}

/// Red-documentation test for defect D13 (PLAN.md Part IV): `Halt` is not
/// fully fail-closed. Its rustdoc promises "no further frames advance once
/// any peer drops", but when the disconnect lands, the dropped slots are
/// excluded from the confirmation fold and the session confirms up to
/// `max_prediction` further frames with **fabricated default inputs** in the
/// dropped slots — divergently across the mesh. Observed on this schedule
/// (partition {0,1}×{2,3} at step 100, timeout ≈ frame 95): peer 1 confirmed
/// frames 96..=103 as `[3101, 3108, 0, 0]`… while peer 3 confirmed the same
/// frames as `[0, 0, 3115, 3122]`…, and end-of-run confirmation is
/// asymmetric even within a half (95 vs 103). This pins today's defective
/// behavior; the divergence assertions flip when the M4 fix lands (the
/// confirmed prefix must never extend past the last globally-agreed frame
/// under `Halt`).
#[test]
fn partition_under_halt_confirms_fabricated_frames_divergently_d13() {
    let schedule = split_brain_schedule(DropPolicy::Halt);
    let report = run(&schedule, &RunOptions::default());
    let failures = &report.verdict.failures;
    // RED (defect D13): the confirmed prefix forks by max_prediction frames.
    assert!(
        failures
            .iter()
            .any(|f| matches!(f, OracleFailure::ConfirmedInputDivergence { .. })),
        "expected today's defective fabricated-frame divergence: {failures:?}"
    );
    // Halt does end the session on every peer (that half of the contract
    // holds): all four stall in Synchronizing and fail end-progress.
    let stalled = failures
        .iter()
        .filter(|f| matches!(f, OracleFailure::EndProgress { .. }))
        .count();
    assert_eq!(
        stalled, 4,
        "all four peers must halt (EndProgress in Synchronizing): {failures:?}"
    );
    // The fabricated tail is bounded by the prediction window: no peer's
    // confirmation may exceed another's by more than max_prediction.
    let min = report.final_confirmed.iter().copied().min().unwrap_or(-1);
    let max = report.final_confirmed.iter().copied().max().unwrap_or(-1);
    assert!(
        (max - min) as usize <= schedule.config.max_prediction,
        "fabricated tail must be bounded by max_prediction ({} vs {}): {:?}",
        max - min,
        schedule.config.max_prediction,
        report.final_confirmed
    );
}

/// CAP documentation test, `ContinueWithout` side: the same partition FORKS
/// the session — each half drops the other, freezes their slots, and keeps
/// confirming on its own divergent timeline. Availability is chosen over
/// consistency, permanently: dropped endpoints are terminal, so healing the
/// network can never re-merge the halves. This pins the fork as documented,
/// deliberate behavior (and records whether the library's own in-band desync
/// detection can see across it).
#[test]
fn partition_under_continue_without_forks_into_divergent_halves() {
    let schedule = split_brain_schedule(DropPolicy::ContinueWithout);
    let report = run(&schedule, &RunOptions::default());
    let failures = &report.verdict.failures;
    assert!(
        failures
            .iter()
            .any(|f| matches!(
                f,
                OracleFailure::ConfirmedInputDivergence { .. }
                    | OracleFailure::StateDivergence { .. }
            )),
        "ContinueWithout must fork the mesh (divergence expected): {failures:?}\nfinal_confirmed={:?}",
        report.final_confirmed
    );
    for (peer, confirmed) in report.final_confirmed.iter().enumerate() {
        assert!(
            *confirmed > 200,
            "peer {peer} must keep confirming on its half of the fork \
             (availability): final_confirmed={:?}\nfailures={failures:?}",
            report.final_confirmed
        );
    }
    let inband = failures
        .iter()
        .filter(|f| matches!(f, OracleFailure::InbandDesyncDetected { .. }))
        .count();
    assert_eq!(
        inband, 0,
        "documenting: the fork is invisible to in-band desync detection \
         (ghost traffic from dropped endpoints is discarded): {failures:?}"
    );
}

/// Diagnostic probe for a failing schedule: prints per-peer progress every 50
/// steps. `#[ignore]`d — run manually while investigating a repro.
#[test]
#[ignore = "diagnostic probe; run manually against a repro seed"]
fn diagnose_repro() {
    use super::harness::diagnose;
    let schedule = generate(1, SimConfig::smoke(4));
    diagnose(&schedule);
}

/// Budget probe: measures the wall-clock cost of the largest supported mesh
/// over a long schedule. `#[ignore]`d — run manually to (re)calibrate fleet
/// sizes and CI budgets:
///
/// ```text
/// cargo nextest run --no-capture --run-ignored ignored-only -E 'test(budget_probe)'
/// ```
#[test]
#[ignore = "budget calibration probe; run manually"]
// Deliberate diagnostic stdout: the measured wall time IS the deliverable.
#[allow(clippy::print_stdout, clippy::disallowed_macros)]
fn budget_probe_sixteen_player_long_schedule() {
    let config = SimConfig {
        n_players: 16,
        steps: 5_000,
        ..SimConfig::smoke(16)
    };
    let schedule = generate(99, config);
    let started = std::time::Instant::now();
    let report = run(&schedule, &RunOptions::default());
    let elapsed = started.elapsed();
    println!(
        "budget probe: n=16 steps=5000 wall={elapsed:?} final_confirmed={:?} net={:?}",
        report.final_confirmed, report.net_stats
    );
    report.expect_pass(&schedule);
}

#[test]
#[ignore = "budget calibration probe; run manually"]
// Deliberate diagnostic stdout: the measured wall time IS the deliverable.
#[allow(clippy::print_stdout, clippy::disallowed_macros)]
fn budget_probe_eight_player_long_schedule() {
    let config = SimConfig {
        n_players: 8,
        steps: 5_000,
        ..SimConfig::smoke(8)
    };
    let schedule = generate(99, config);
    let started = std::time::Instant::now();
    let report = run(&schedule, &RunOptions::default());
    let elapsed = started.elapsed();
    println!(
        "budget probe: n=8 steps=5000 wall={elapsed:?} final_confirmed={:?} net={:?}",
        report.final_confirmed, report.net_stats
    );
    report.expect_pass(&schedule);
}
