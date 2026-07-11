//! Simulation fleet: seeded whole-mesh runs checked by the global oracle.
//!
//! The PR smoke fleet runs a fixed seed set per mesh size on every PR. The
//! meta-determinism and negative-control tests validate the harness itself:
//! a harness whose invariants cannot fire, or whose runs are not reproducible,
//! proves nothing.

use super::harness::schedule::{
    generate, validate_schedule, AppModel, BackgroundNoise, DropPolicy, FrameModel, ScenarioMix,
    Schedule, ScheduleEvent, SimConfig, SCHEDULE_SCHEMA_VERSION,
};
use super::harness::{
    oracle::{OracleFailure, ViolationAllowlistEntry, ViolationSignature, POST_HEAL_MIN_ADVANCE},
    run, RunOptions, RunReport,
};
use crate::common::sim_net::LinkPolicy;
use fortress_rollback::{telemetry::ViolationSeverity, SessionState};
use std::{collections::BTreeSet, time::Duration};

/// Fixed PR-smoke seed set. Nightly fleets derive disjoint seeds from the CI
/// run id; the PR set stays fixed so PR failures reproduce verbatim.
const PR_SMOKE_SEEDS: [u64; 8] = [1, 2, 3, 5, 8, 13, 21, 34];

const NIGHTLY_SHARDS: u64 = 8;
const NIGHTLY_SEEDS_PER_SHARD: u64 = 125;
const NIGHTLY_STEPS: u32 = 5_000;

fn run_smoke_fleet(n_players: usize) {
    for seed in PR_SMOKE_SEEDS {
        let schedule = generate(seed, SimConfig::smoke(n_players));
        let report = run(&schedule, &RunOptions::default());
        report.expect_pass(&schedule);
    }
}

fn nightly_seed(run_id: u64, shard: u64, offset: u64) -> u64 {
    let seeds_per_run = NIGHTLY_SHARDS.saturating_mul(NIGHTLY_SEEDS_PER_SHARD);
    run_id
        .wrapping_mul(seeds_per_run)
        .wrapping_add(shard.saturating_mul(NIGHTLY_SEEDS_PER_SHARD))
        .wrapping_add(offset)
}

fn nightly_schedule(seed: u64, n_players: usize, noise: BackgroundNoise) -> Schedule {
    let config = SimConfig {
        steps: NIGHTLY_STEPS,
        noise,
        disconnect_behavior: DropPolicy::ContinueWithout,
        scenario_mix: ScenarioMix::Lifecycle,
        ..SimConfig::smoke(n_players)
    };
    let mut schedule = generate(seed, config);

    // D14 is specifically retirement under lossy background traffic. Preserve
    // lossy lifecycle exploration for stalls and live reconfiguration while
    // keeping the known-red retirement cross-product out of the green tier.
    if noise != BackgroundNoise::Clean {
        for (_, event) in &mut schedule.events {
            let retired_peer = match event {
                ScheduleEvent::PeerKill { peer } => Some(*peer),
                ScheduleEvent::GracefulRemove { target, .. } => Some(*target),
                ScheduleEvent::SpectatorHostKill { host } => Some(*host),
                _ => None,
            };
            if let Some(peer) = retired_peer {
                *event = ScheduleEvent::PeerStall {
                    peer,
                    steps: 20 + u32::try_from(seed % 21).expect("stall window fits u32"),
                };
            }
        }
    }

    if noise == BackgroundNoise::ReliableFifo {
        let retired = schedule
            .events
            .iter()
            .filter_map(|(_, event)| match event {
                ScheduleEvent::PeerKill { peer }
                | ScheduleEvent::GracefulRemove { target: peer, .. }
                | ScheduleEvent::LegacyDisconnect { target: peer, .. }
                | ScheduleEvent::SpectatorHostKill { host: peer } => Some(*peer),
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        let mut eligible = (0..n_players)
            .filter(|peer| !retired.contains(peer))
            .collect::<Vec<_>>();
        if eligible.len() < 2 {
            // Two-player retirement stories have no pair that remains live;
            // their generated lifecycle event occurs after the early HOL
            // window below, so both initial endpoints remain meaningful.
            // This branch is structural rather than silently skipping the
            // reliable-FIFO assertion.
            eligible.clear();
            eligible.extend(0..n_players);
        }
        let from_index =
            usize::try_from(seed % u64::try_from(eligible.len()).expect("endpoint count fits u64"))
                .expect("peer index fits usize");
        let from = eligible[from_index];
        let peer_offset = 1 + usize::try_from(
            (seed / u64::try_from(eligible.len()).expect("endpoint count fits u64"))
                % u64::try_from(eligible.len() - 1).expect("remote count fits u64"),
        )
        .expect("peer offset fits usize");
        let to = eligible[(from_index + peer_offset) % eligible.len()];
        let start = 100 + u32::try_from((seed / 17) % 150).expect("HOL start fits u32");
        let window = 20 + u32::try_from((seed / 31) % 40).expect("HOL window fits u32");
        let fifo = schedule
            .initial_links
            .iter()
            .find_map(|(link_from, link_to, policy)| {
                (*link_from == from && *link_to == to).then(|| policy.clone())
            })
            .expect("nightly reliable mesh contains every directed link");
        let hol = LinkPolicy {
            drop_rate: 1.0,
            retransmit_delay: Duration::from_millis(400),
            ..fifo
        };
        schedule.events.extend([
            (
                start,
                ScheduleEvent::SetLink {
                    from,
                    to,
                    policy: hol,
                },
            ),
            (
                start + window,
                ScheduleEvent::SetLink {
                    from,
                    to,
                    policy: fifo,
                },
            ),
        ]);
        schedule.events.sort_by_key(|(step, _)| *step);
    }

    schedule
}

/// Runs one disjoint shard of the long simulation fleet. The explicit tier
/// gate prevents a manually selected ignored test from silently running a
/// costly workload with an accidental/default seed base.
fn run_nightly_shard(shard: u64) {
    assert!(
        shard < NIGHTLY_SHARDS,
        "nightly shard {shard} is out of range"
    );
    assert_eq!(
        std::env::var("FORTRESS_SIM_TIER").as_deref(),
        Ok("nightly"),
        "nightly simulation tests require FORTRESS_SIM_TIER=nightly"
    );
    let run_id = std::env::var("FORTRESS_SIM_SEED_BASE")
        .expect("nightly simulation tests require FORTRESS_SIM_SEED_BASE")
        .parse::<u64>()
        .expect("FORTRESS_SIM_SEED_BASE must be an unsigned integer");

    for offset in 0..NIGHTLY_SEEDS_PER_SHARD {
        let seed = nightly_seed(run_id, shard, offset);
        let n_players = 2 + usize::try_from(seed % 15).expect("seed modulo 15 fits usize");
        let noise = match seed % 4 {
            0 => BackgroundNoise::Clean,
            1 => BackgroundNoise::Mild,
            2 => BackgroundNoise::Rough,
            _ => BackgroundNoise::ReliableFifo,
        };
        let schedule = nightly_schedule(seed, n_players, noise);
        let report = run(&schedule, &RunOptions::default());
        report.expect_pass(&schedule);
        if noise == BackgroundNoise::ReliableFifo {
            assert!(
                report.net_stats.retransmit_delayed > 0,
                "nightly reliable-FIFO seed {seed} must exercise HOL retransmission: {:?}",
                report.net_stats
            );
        }
    }
}

macro_rules! nightly_shard_test {
    ($name:ident, $shard:expr) => {
        #[test]
        #[ignore = "long release-mode simulation fleet; nightly CI only"]
        fn $name() {
            run_nightly_shard($shard);
        }
    };
}

nightly_shard_test!(nightly_seed_shard_0_holds_invariants, 0);
nightly_shard_test!(nightly_seed_shard_1_holds_invariants, 1);
nightly_shard_test!(nightly_seed_shard_2_holds_invariants, 2);
nightly_shard_test!(nightly_seed_shard_3_holds_invariants, 3);
nightly_shard_test!(nightly_seed_shard_4_holds_invariants, 4);
nightly_shard_test!(nightly_seed_shard_5_holds_invariants, 5);
nightly_shard_test!(nightly_seed_shard_6_holds_invariants, 6);
nightly_shard_test!(nightly_seed_shard_7_holds_invariants, 7);

/// Known production defect discovered by the first lifecycle nightly shard:
/// a lossy one-caller graceful removal can lower the victim's eventual freeze
/// below a frame another survivor already returned as confirmed, rewriting
/// that survivor's historical input query. This remains ignored/known-red
/// until removal gains a coordinated receipt/backfill barrier; the green
/// nightly matrix excludes only this unsupported retirement × lossy-noise
/// cross-product above.
#[test]
#[ignore = "known confirmed-history rewrite under lossy graceful removal"]
fn lossy_graceful_remove_rewrites_confirmed_history_known_defect() {
    let schedule: Schedule = serde_json::from_str(include_str!(
        "known_failures/d14-lossy-graceful-remove.json"
    ))
    .expect("checked-in D14 schedule must deserialize");
    validate_schedule(&schedule).expect("checked-in D14 schedule must be valid");
    assert_eq!((schedule.config.n_players, schedule.config.steps), (5, 650));
    assert!(schedule
        .events
        .iter()
        .any(|(_, event)| matches!(event, ScheduleEvent::GracefulRemove { by: 3, target: 4 })));
    let report = run(&schedule, &RunOptions::default());
    assert!(
        report.verdict.failures.iter().any(|failure| matches!(
            failure,
            OracleFailure::ConfirmedInputDivergence { frame: 327, .. }
        )),
        "known defect no longer reproduces; flip this to a green regression: {:?}",
        report.verdict.failures
    );
}

#[test]
fn nightly_seed_windows_are_disjoint_and_repeatable() {
    let window = |run_id| {
        (0..NIGHTLY_SHARDS)
            .flat_map(|shard| {
                (0..NIGHTLY_SEEDS_PER_SHARD).map(move |offset| nightly_seed(run_id, shard, offset))
            })
            .collect::<BTreeSet<_>>()
    };
    let first = window(41);
    let repeat = window(41);
    let next = window(42);

    assert_eq!(first, repeat, "a rerun must use the identical seed window");
    assert_eq!(
        first.len(),
        usize::try_from(NIGHTLY_SHARDS * NIGHTLY_SEEDS_PER_SHARD).unwrap(),
        "every shard/offset pair must map to a distinct seed"
    );
    assert!(
        first.is_disjoint(&next),
        "adjacent CI run ids must not overlap nightly seed windows"
    );
}

#[test]
fn non_clean_nightly_lifecycle_rewrites_only_retirement_stories() {
    for noise in [
        BackgroundNoise::Mild,
        BackgroundNoise::Rough,
        BackgroundNoise::ReliableFifo,
    ] {
        for seed in [2, 3, 4] {
            let schedule = nightly_schedule(seed, 5, noise);
            assert!(
                !schedule.events.iter().any(|(_, event)| matches!(
                    event,
                    ScheduleEvent::PeerKill { .. }
                        | ScheduleEvent::GracefulRemove { .. }
                        | ScheduleEvent::SpectatorHostKill { .. }
                )),
                "non-clean retirement escaped quarantine: noise={noise:?} seed={seed}"
            );
            assert!(
                schedule.effective_lifecycle_event_count().unwrap_or(0) > 0,
                "retirement rewrite erased lifecycle coverage: noise={noise:?} seed={seed}"
            );
        }
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

/// M3 §6.2(f): empty-allowlist violation census. The default simulation
/// allowlist is intentionally empty; a passing non-injection smoke run should
/// not emit any `Error`/`Critical` telemetry signature. Warnings remain visible
/// through `RunReport::violation_census` without failing the oracle.
#[test]
#[ignore = "200-seed violation census; run manually/nightly after allowlist changes"]
// Deliberate census stdout: ignored/manual runs need the green signature ledger.
#[allow(clippy::print_stdout, clippy::disallowed_macros)]
fn violation_census_two_hundred_smoke_seeds_has_no_error_signatures() {
    let mut census: std::collections::BTreeMap<ViolationSignature, u64> =
        std::collections::BTreeMap::new();
    let mut allowlist_hits: std::collections::BTreeMap<ViolationAllowlistEntry, u64> =
        std::collections::BTreeMap::new();

    for seed in 0..200u64 {
        let schedule = generate(seed, SimConfig::smoke(4));
        let report = run(&schedule, &RunOptions::default());
        report.expect_pass(&schedule);
        for hit in report.verdict.violation_allowlist_hits {
            *allowlist_hits.entry(hit.entry).or_default() += hit.count;
        }
        for (key, count) in report.violation_census {
            *census.entry(key).or_default() += count;
        }
    }

    println!("violation allowlist hits: {allowlist_hits:#?}");
    println!("violation census: {census:#?}");

    let unexpected: Vec<_> = census
        .iter()
        .filter(|(key, _)| key.severity >= ViolationSeverity::Error)
        .collect();
    assert!(
        unexpected.is_empty(),
        "empty-allowlist census found Error+ telemetry signatures: {unexpected:#?}"
    );
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
    let mut total_stalls = 0u64;
    let mut total_wait_recs = 0u64;
    let mut max_confirmation_lag = 0u64;
    let mut max_event_queue_hw = 0u64;
    let mut max_checksum_hw = 0u64;

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
                total_stalls += m.stall_count;
                total_wait_recs += m.wait_recommendations;
                max_confirmation_lag = max_confirmation_lag.max(m.confirmation_lag_max);
                max_event_queue_hw = max_event_queue_hw.max(m.event_queue_high_water);
                max_checksum_hw = max_checksum_hw.max(m.checksum_history_high_water);
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
    // Wiring coverage for the pacing / high-water counters: mild loss over 600
    // frames reliably drives prediction-window stalls and wait recommendations,
    // runs the simulation ahead of confirmation, and fills the event queue and
    // checksum history — so every one of these sites is proven reachable, not
    // merely exercised by the direct-call unit tests.
    assert!(
        total_stalls > 0,
        "fleet recorded no prediction-window stalls"
    );
    assert!(
        total_wait_recs > 0,
        "fleet recorded no wait recommendations"
    );
    assert!(
        max_confirmation_lag > 0,
        "fleet never sampled a non-zero confirmation lag"
    );
    assert!(
        max_event_queue_hw > 0,
        "fleet never recorded an event-queue high-water mark"
    );
    assert!(
        max_checksum_hw > 0,
        "fleet never recorded a checksum-history high-water mark"
    );
}

/// M2 §5.3 prep: the mesh runner now folds each peer's per-remote
/// [`PeerMetrics`](fortress_rollback::PeerMetrics) into a per-player
/// `PeerWireTotals` — the bandwidth ledger the baseline sweep consumes. This
/// asserts, per peer, the by-kind/packet identities that hold by construction
/// (the aggregation preserves them), and mesh-wide that real wire traffic
/// flowed — exercising `P2PSession::peer_metrics` end-to-end under randomized
/// simulation, not just the direct-call unit tests.
#[test]
fn peer_wire_metrics_are_wired_across_smoke_fleet() {
    use fortress_rollback::MessageKind;

    let mut total_bytes_sent = 0u64;
    let mut total_input_msgs = 0u64;
    let mut total_input_pre = 0u64;
    let mut total_input_post = 0u64;

    for n_players in [2usize, 3, 4] {
        for seed in PR_SMOKE_SEEDS {
            let schedule = generate(seed, SimConfig::smoke(n_players));
            let report = run(&schedule, &RunOptions::default());
            report.expect_pass(&schedule);

            assert_eq!(
                report.peer_wire.len(),
                n_players,
                "one wire ledger per peer — seed {seed} n{n_players}"
            );
            for (i, w) in report.peer_wire.iter().enumerate() {
                let ctx = || format!("peer {i} seed {seed} n{n_players}: {w:?}");
                // The per-kind buckets partition the packet counters exactly —
                // aggregation across links preserves the per-link identity.
                assert_eq!(
                    w.sent_by_kind_total(),
                    w.packets_sent,
                    "sent by-kind total != packets_sent — {}",
                    ctx()
                );
                assert_eq!(
                    w.received_by_kind_total(),
                    w.packets_received,
                    "received by-kind total != packets_received — {}",
                    ctx()
                );
                // Every peer in a live mesh both puts bytes on and takes bytes
                // off the wire.
                assert!(
                    w.packets_sent > 0 && w.bytes_sent > 0,
                    "no outbound wire traffic — {}",
                    ctx()
                );
                assert!(
                    w.packets_received > 0 && w.bytes_received > 0,
                    "no inbound wire traffic — {}",
                    ctx()
                );
                // The gameplay stream rides Input packets in both directions —
                // must be non-trivial each way.
                assert!(
                    w.sent_by_kind(MessageKind::Input) > 0,
                    "no Input packets sent — {}",
                    ctx()
                );
                assert!(
                    w.received_by_kind(MessageKind::Input) > 0,
                    "no Input packets received — {}",
                    ctx()
                );

                total_bytes_sent += w.bytes_sent;
                total_input_msgs += w.sent_by_kind(MessageKind::Input);
                total_input_pre += w.input_bytes_pre_compression;
                total_input_post += w.input_bytes_post_compression;
            }
        }
    }

    assert!(total_bytes_sent > 0, "fleet put no bytes on the wire");
    assert!(total_input_msgs > 0, "fleet sent no Input packets");
    // The input-compression hook is wired on the send path: both pre- and
    // post-compression byte totals are recorded for the gameplay stream.
    assert!(
        total_input_pre > 0,
        "no pre-compression input bytes recorded"
    );
    assert!(
        total_input_post > 0,
        "no post-compression input bytes recorded"
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
/// in-order link profile (TCP / WebRTC-reliable model) with
/// head-of-line-blocking stalls modeled by `LinkPolicy::retransmit_delay`.
/// Zero jitter keeps `SimNet`'s per-link FIFO exact, so delivery is in-order —
/// the TCP contract. Each retransmit window delays would-drop sends by 400 ms
/// (> the 8-frame prediction window), so the session must stall-and-catch-up,
/// never diverge.
///
/// Expected green: the protocol's correctness must not depend on loss or
/// reordering being present. A failure here is a real transport-model bug.
fn run_tcp_model_mesh(n: usize) {
    let config = SimConfig {
        n_players: n,
        steps: 900,
        noise: BackgroundNoise::ReliableFifo,
        ..SimConfig::smoke(n)
    };
    let fifo = LinkPolicy {
        drop_rate: 0.0,
        dup_rate: 0.0,
        base_delay: Duration::from_millis(30),
        jitter: Duration::ZERO,
        burst_rate: 0.0,
        burst_len: 0,
        retransmit_delay: Duration::ZERO,
        gilbert_elliott: None,
        fragmentation: None,
        bandwidth: None,
    };
    let hol = LinkPolicy {
        drop_rate: 1.0,
        retransmit_delay: Duration::from_millis(400),
        ..fifo
    };
    let mut initial_links = Vec::new();
    for from in 0..n {
        for to in 0..n {
            if from != to {
                initial_links.push((from, to, fifo.clone()));
            }
        }
    }
    // Three HOL retransmit windows on distinct directed links, well before heal.
    let hold_windows: [(usize, usize, u32); 3] = [(0, 1, 100), (1, 0, 250), (n - 1, 0, 400)];
    let mut events = Vec::new();
    for (from, to, start) in hold_windows {
        events.push((
            start,
            ScheduleEvent::SetLink {
                from,
                to,
                policy: hol.clone(),
            },
        ));
        events.push((
            start + 25,
            ScheduleEvent::SetLink {
                from,
                to,
                policy: fifo.clone(),
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
    assert!(
        report.net_stats.retransmit_delayed > 0,
        "the TCP-model schedule must exercise reliable HOL retransmission: {:?}",
        report.net_stats
    );
    assert_eq!(
        report.net_stats.dropped_by_policy, 0,
        "reliable retransmission mode must delay would-drop sends instead of \
         dropping them: {:?}",
        report.net_stats
    );
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

/// D13 regression: `Halt` freezes each public confirmed prefix at its safe
/// pre-drop local bound. Removing dropped slots from the ordinary
/// fold must not expose the speculative default-input tail as confirmed.
#[test]
fn partition_under_halt_preserves_one_shared_confirmed_prefix_d13() {
    let schedule = split_brain_schedule(DropPolicy::Halt);
    let report = run(&schedule, &RunOptions::default());
    let failures = &report.verdict.failures;
    assert!(
        !failures.iter().any(|failure| matches!(
            failure,
            OracleFailure::ConfirmedInputDivergence { .. } | OracleFailure::StateDivergence { .. }
        )),
        "Halt exposed a divergent speculative tail as confirmed: {failures:?}"
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
    let post_heal = failures
        .iter()
        .filter(|failure| matches!(failure, OracleFailure::PostHealLiveness { .. }))
        .count();
    assert_eq!(post_heal, 4, "every halted peer must report non-recovery");
    assert_eq!(
        failures.len(),
        8,
        "unexpected Halt failure surface: {failures:?}"
    );
    assert!(
        failures.iter().all(|failure| matches!(
            failure,
            OracleFailure::EndProgress { .. } | OracleFailure::PostHealLiveness { .. }
        )),
        "Halt emitted an unexpected failure class: {failures:?}"
    );
    let min = report.final_confirmed.iter().copied().min().unwrap_or(-1);
    let max = report.final_confirmed.iter().copied().max().unwrap_or(-1);
    assert_eq!(
        min, max,
        "this symmetric partition must retain one consistent shared prefix: {:?}",
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

/// Step at which [`peer_hitch_schedule`] freezes peer 1. The recovery test
/// derives its mid-run probe step from this so the two never drift.
const PEER_HITCH_START: u32 = 200;

/// Builds a peer-hitch schedule: an otherwise-clean `n`-mesh in which peer 1
/// freezes for `stall` steps starting at [`PEER_HITCH_START`] — a *local* hang
/// (GC pause, frame-time spike, blocked save), not a network fault. `stall` is
/// kept well under the 2 s (~125-step) disconnect timeout, so the mesh predicts
/// past the frozen peer to the prediction wall, stalls waiting for its inputs,
/// then catches up when it resumes — a recoverable pause, never a drop. Passing
/// `None` for `stall` yields the identical schedule *without* the hitch, the
/// control the premise assertion compares against.
fn peer_hitch_schedule(n: usize, stall: Option<u32>) -> Schedule {
    let config = SimConfig {
        n_players: n,
        steps: 900,
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
    let heal_at = 650;
    let mut events = vec![(heal_at, ScheduleEvent::HealAll)];
    if let Some(steps) = stall {
        events.push((
            PEER_HITCH_START,
            ScheduleEvent::PeerStall { peer: 1, steps },
        ));
    }
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

/// M3 §6.1 lifecycle vocabulary — the peer-hitch fault (`ScheduleEvent::
/// PeerStall`). A peer that hangs for a bounded window shorter than the
/// disconnect timeout must be a *recoverable* pause: the mesh predicts past it
/// to the prediction wall, stalls for its inputs, then catches up on resume,
/// and the confirmed prefix stays byte-identical across every peer.
///
/// The test asserts the full freeze→recover arc, not just the converged
/// end-state (a clean drain always converges, so end-state alone has no teeth):
/// 1. **Premise** — the same clean schedule *with* and *without* the stall
///    produces different traces (the hitch is effective, not a silent no-op),
///    and two hitched runs fold the identical trace (determinism).
/// 2. **Freeze** — mid-stall (probed at the last frozen step), the hitched
///    mesh's confirmation is stalled far behind the clean run's: confirmation
///    is gated by the frozen peer's missing inputs, so the *whole* mesh's
///    confirmed frame is held back, not just the hitching peer's.
/// 3. **Recovery** — by end-of-run the hitched peer has caught up to within the
///    prediction window of the leader (no permanent lag — stronger than the
///    coarse end-progress bar, which a recovered-but-lagging peer would pass).
#[test]
fn peer_hitch_recovers_with_consistent_mesh() {
    const HITCH_STEPS: u32 = 50;
    // Probe the last frozen step: the stall spans [START, START+STEPS), so the
    // peer resumes at START+STEPS and START+STEPS-1 is the deepest freeze.
    const PROBE_AT: u32 = PEER_HITCH_START + HITCH_STEPS - 1;

    for n in [2usize, 4] {
        let clean = peer_hitch_schedule(n, None);
        let hitched = peer_hitch_schedule(n, Some(HITCH_STEPS));
        let opts = RunOptions {
            probe_confirmed_at: Some(PROBE_AT),
            ..RunOptions::default()
        };

        let clean_report = run(&clean, &opts);
        clean_report.expect_pass(&clean);
        let hitched_report = run(&hitched, &opts);
        hitched_report.expect_pass(&hitched);
        let hitched_again = run(&hitched, &opts);

        // (1) Premise + determinism.
        assert_ne!(
            clean_report.trace_hash, hitched_report.trace_hash,
            "the PeerStall must change execution — a silent no-op would leave \
             the trace identical (n={n})"
        );
        assert_eq!(
            hitched_report.trace_hash, hitched_again.trace_hash,
            "a PeerStall schedule must reproduce its exact trace (n={n})"
        );

        // (2) Freeze: mid-stall the whole hitched mesh trails the clean mesh —
        // even the furthest-along hitched peer is well behind the least-along
        // clean peer, because the frozen peer's missing inputs gate everyone's
        // confirmation. (Expected gap ≈ HITCH_STEPS; require a conservative
        // fraction so this is unambiguous, not a one-frame coincidence.)
        let clean_slowest = clean_report
            .probe_confirmed
            .iter()
            .copied()
            .min()
            .unwrap_or(-1);
        let hitched_fastest = hitched_report
            .probe_confirmed
            .iter()
            .copied()
            .max()
            .unwrap_or(-1);
        assert!(
            clean_slowest - hitched_fastest >= (HITCH_STEPS as i32) / 2,
            "the hitch must freeze mesh-wide confirmation mid-stall \
             (clean slowest={clean_slowest}, hitched fastest={hitched_fastest}, \
             n={n}): clean={:?} hitched={:?}",
            clean_report.probe_confirmed,
            hitched_report.probe_confirmed,
        );

        // (3) Recovery: by end-of-run the hitched peer is within the prediction
        // window of the leader — no permanent lag.
        let confirmed = &hitched_report.final_confirmed;
        let leader = confirmed.iter().copied().max().unwrap_or(-1);
        let hitched_peer = confirmed[1];
        assert!(
            (leader - hitched_peer) as usize <= hitched.config.max_prediction,
            "hitched peer 1 must catch up to within the prediction window \
             (leader={leader}, peer1={hitched_peer}, max_prediction={}): {confirmed:?}",
            hitched.config.max_prediction,
        );
    }
}

/// Negative control for the peer-hitch fault: a real state divergence seeded
/// into a *surviving* peer must still be caught while another peer is
/// hitching. A stall that blinded the oracle would make the fleet miss desyncs
/// exactly when a peer is under stress — the HD-1 "silent instrument" failure
/// mode (PLAN.md Part V), here checked against the new lifecycle op.
#[test]
fn oracle_catches_seeded_divergence_under_peer_hitch() {
    let schedule = peer_hitch_schedule(4, Some(50));
    // Corrupt peer 0 (a survivor, not the hitching peer 1) from frame 40 on —
    // well before the stall window, so the divergence is already established
    // and still live while peer 1 hitches.
    let options = RunOptions {
        corrupt_state_from: Some((0, 40)),
        ..RunOptions::default()
    };
    let report = run(&schedule, &options);
    assert!(
        !report.verdict.passed(),
        "a seeded divergence must fail the run even under a peer hitch"
    );
    assert!(
        report
            .verdict
            .failures
            .iter()
            .any(|f| matches!(f, OracleFailure::StateDivergence { .. })),
        "the oracle's state comparison must keep its teeth under a peer hitch: {:?}",
        report.verdict.failures
    );
}

/// A malformed schedule (here: a `PeerStall` naming a peer outside the mesh, as
/// a hand-edited or corrupt corpus artifact might) must fail loudly with a
/// clear message, not panic on a raw slice index — the fail-loud contract the
/// harness's up-front validation enforces for corpus replay.
#[test]
#[should_panic(expected = "out of range")]
fn run_rejects_out_of_range_peer_index() {
    let mut schedule = peer_hitch_schedule(2, None);
    schedule
        .events
        .push((100, ScheduleEvent::PeerStall { peer: 9, steps: 10 }));
    schedule.events.sort_by_key(|(step, _)| *step);
    let _ = run(&schedule, &RunOptions::default());
}

/// Builds an input-delay-change schedule: a clean `n`-mesh in which peer 1
/// raises its local input delay by `increase` frames at step 100 — a mid-session
/// *increase*, which gap-fills the newly delayed frames with replicated confirmed
/// inputs and flushes them to every remote (a reconfiguration path a fixed-delay
/// fleet never exercises). The new delay is `config.input_delay + increase`, so
/// it is always a genuine increase regardless of the config default (never a
/// silent no-op that would hollow out the premise). `None` yields the identical
/// schedule without the change, the control the premise compares to.
fn input_delay_change_schedule(n: usize, increase: Option<usize>) -> Schedule {
    let config = SimConfig {
        n_players: n,
        steps: 600,
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
    let heal_at = 400;
    let mut events = vec![(heal_at, ScheduleEvent::HealAll)];
    if let Some(increase) = increase {
        let delay = config.input_delay + increase;
        events.push((100, ScheduleEvent::SetInputDelay { peer: 1, delay }));
    }
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

/// M3 §6.1 lifecycle vocabulary — the mid-run input-delay change
/// (`ScheduleEvent::SetInputDelay`). Raising a peer's input delay mid-session
/// drives `P2PSession::set_input_delay`'s gap-fill/flush reconfiguration path
/// (replicated confirmed inputs pushed to every remote, connect-status stamped,
/// pending output flushed) — untouched by a fixed-delay fleet. The change is
/// local to one peer, so the mesh must still agree on every confirmed frame and
/// keep progressing.
///
/// The premise is asserted directly: the same clean schedule *with* and
/// *without* the delay change must produce different traces (the reconfiguration
/// demonstrably took effect, not a silent no-op), while both pass the oracle;
/// two changed runs fold the identical trace (determinism).
#[test]
fn input_delay_increase_keeps_mesh_consistent() {
    for n in [2usize, 4] {
        let base = input_delay_change_schedule(n, None);
        let changed = input_delay_change_schedule(n, Some(2));

        let base_report = run(&base, &RunOptions::default());
        base_report.expect_pass(&base);
        let changed_report = run(&changed, &RunOptions::default());
        changed_report.expect_pass(&changed);

        let changed_again = run(&changed, &RunOptions::default());
        assert_eq!(
            changed_report.trace_hash, changed_again.trace_hash,
            "a SetInputDelay schedule must reproduce its exact trace (n={n})"
        );
        assert_ne!(
            base_report.trace_hash, changed_report.trace_hash,
            "the SetInputDelay must change execution — a silent no-op would \
             leave the trace identical (n={n})"
        );
    }
}

/// Negative control for the input-delay change: a real state divergence seeded
/// into a survivor must still be caught while a peer reconfigures its input
/// delay — the reconfiguration path must not blind the oracle.
#[test]
fn oracle_catches_seeded_divergence_under_input_delay_change() {
    let schedule = input_delay_change_schedule(4, Some(2));
    let options = RunOptions {
        corrupt_state_from: Some((0, 40)),
        ..RunOptions::default()
    };
    let report = run(&schedule, &options);
    assert!(
        !report.verdict.passed(),
        "a seeded divergence must fail the run even under an input-delay change"
    );
    assert!(
        report
            .verdict
            .failures
            .iter()
            .any(|f| matches!(f, OracleFailure::StateDivergence { .. })),
        "the oracle's state comparison must keep its teeth under a delay change: {:?}",
        report.verdict.failures
    );
}

/// A `SetInputDelay` naming a peer outside the mesh must fail loudly with the
/// same clear message as the other events, not panic on a raw slice index — the
/// per-event fail-loud contract of the up-front validation (parallels
/// `run_rejects_out_of_range_peer_index`, since each event arm has its own
/// message and would otherwise be unexercised).
#[test]
#[should_panic(expected = "out of range")]
fn run_rejects_out_of_range_set_input_delay_peer() {
    let mut schedule = input_delay_change_schedule(2, None);
    schedule
        .events
        .push((100, ScheduleEvent::SetInputDelay { peer: 9, delay: 3 }));
    schedule.events.sort_by_key(|(step, _)| *step);
    let _ = run(&schedule, &RunOptions::default());
}

/// Builds the common clean 4-peer lifecycle schedule used by the planted drop
/// operations. `event`, when present, fires at step 100; `None` yields the
/// matched no-op control.
fn clean_four_peer_lifecycle_schedule(
    disconnect_behavior: DropPolicy,
    event: Option<ScheduleEvent>,
) -> Schedule {
    let n = 4;
    let config = SimConfig {
        n_players: n,
        steps: 900,
        disconnect_behavior,
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
    let heal_at = 650;
    let mut events = vec![(heal_at, ScheduleEvent::HealAll)];
    if let Some(event) = event {
        events.push((100, event));
    }
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

/// Builds a peer-kill schedule: a clean 4-mesh (`ContinueWithout`) in which peer
/// 1 crashes at step 100 — the harness stops driving it and detaches it from the
/// fabric, so it never returns (not even after `HealAll`). Under `ContinueWithout`
/// the three survivors time it out, freeze its slot, and keep confirming together
/// on their own timeline; the crashed peer is excluded from the oracle's
/// liveness checks. `None` yields the identical schedule without the kill, the
/// control the premise compares to.
fn peer_kill_schedule(kill: Option<usize>) -> Schedule {
    clean_four_peer_lifecycle_schedule(
        DropPolicy::ContinueWithout,
        kill.map(|peer| ScheduleEvent::PeerKill { peer }),
    )
}

/// Builds a graceful-remove schedule: a clean 4-mesh (`ContinueWithout`) in
/// which peer `by` calls `remove_player(target)` at step 100. On success the
/// target leaves the harness; on error it stays live and the oracle records the
/// failed API call. Only one survivor applies the user API; the other survivors
/// must learn the drop through protocol gossip and converge on the same frozen
/// slot value. `None` yields the identical schedule without the removal, the
/// control the premise compares to.
fn graceful_remove_schedule(remove: Option<(usize, usize)>) -> Schedule {
    clean_four_peer_lifecycle_schedule(
        DropPolicy::ContinueWithout,
        remove.map(|(by, target)| ScheduleEvent::GracefulRemove { by, target }),
    )
}

#[cfg(feature = "hot-join")]
fn hot_join_schedule(slot: Option<usize>) -> Schedule {
    let n = 2;
    let config = SimConfig {
        n_players: n,
        steps: 900,
        input_delay: 0,
        disconnect_behavior: DropPolicy::ContinueWithout,
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
    let heal_at = 650;
    let mut events = vec![(heal_at, ScheduleEvent::HealAll)];
    if let Some(slot) = slot {
        events.push((100, ScheduleEvent::HotJoin { slot }));
    }
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

#[cfg(feature = "hot-join")]
fn hot_join_after_runtime_remove_schedule() -> Schedule {
    let mut schedule = hot_join_schedule(None);
    schedule
        .events
        .push((100, ScheduleEvent::GracefulRemove { by: 0, target: 1 }));
    schedule
        .events
        .push((120, ScheduleEvent::HotJoin { slot: 1 }));
    schedule.events.sort_by_key(|(step, _)| *step);
    schedule
}

/// Builds a legacy-disconnect schedule: a clean 4-mesh in which peer `by`
/// calls the older `disconnect_player(target)` API at step 100. On success the
/// target leaves the harness; on error it stays live and the oracle records the
/// failed API call. This is intentionally not a graceful-convergence contract:
/// `disconnect_player` is Halt-oriented and terminal, so the useful property
/// is that the op is executable, deterministic, and reported as
/// the expected post-heal non-recovery. `None` yields the identical schedule
/// without the disconnect, the control the premise compares to.
fn legacy_disconnect_schedule(disconnect: Option<(usize, usize)>) -> Schedule {
    clean_four_peer_lifecycle_schedule(
        DropPolicy::Halt,
        disconnect.map(|(by, target)| ScheduleEvent::LegacyDisconnect { by, target }),
    )
}

fn spectator_host_kill_schedule(partition: Option<usize>, kill: Option<usize>) -> Schedule {
    let mut schedule = clean_four_peer_lifecycle_schedule(DropPolicy::ContinueWithout, None);
    schedule.config.spectator_hosts = vec![0, 2, 3];
    if let Some(host) = partition {
        for peer in 0..schedule.config.n_players {
            if peer == host {
                continue;
            }
            schedule.events.push((
                80,
                ScheduleEvent::Block {
                    from: host,
                    to: peer,
                    blocked: true,
                },
            ));
            schedule.events.push((
                80,
                ScheduleEvent::Block {
                    from: peer,
                    to: host,
                    blocked: true,
                },
            ));
        }
    }
    if let Some(host) = kill {
        schedule
            .events
            .push((140, ScheduleEvent::SpectatorHostKill { host }));
    }
    schedule.events.sort_by_key(|(step, _)| *step);
    schedule
}

/// M3 §6.1 lifecycle vocabulary — the peer crash (`ScheduleEvent::PeerKill`).
/// Under `ContinueWithout`, crashing one peer must degrade gracefully: the three
/// survivors converge on dropping it (freeze its slot to an agreed value) and
/// keep confirming *together* — no fork among them. The crashed peer is excluded
/// from the oracle's liveness checks (it can't be `Running`), which is the
/// alive-mask this op adds.
///
/// This is the green half — a clean single-peer removal is not a partition, so
/// the survivors must stay byte-consistent (contrast the symmetric split-brain,
/// which forks). Premise (with-vs-without the kill diverges the trace) and
/// determinism are asserted directly; the negative control below proves the
/// oracle still catches a real divergence among the survivors.
#[test]
fn peer_kill_survivors_converge_under_continue_without() {
    let base = peer_kill_schedule(None);
    let killed = peer_kill_schedule(Some(1));

    let base_report = run(&base, &RunOptions::default());
    base_report.expect_pass(&base);
    let killed_report = run(&killed, &RunOptions::default());
    killed_report.expect_pass(&killed);

    let killed_again = run(&killed, &RunOptions::default());
    assert_eq!(
        killed_report.trace_hash, killed_again.trace_hash,
        "a PeerKill schedule must reproduce its exact trace"
    );
    assert_ne!(
        base_report.trace_hash, killed_report.trace_hash,
        "the PeerKill must change execution — a silent no-op would leave the \
         trace identical"
    );

    // The three survivors kept confirming; the crashed peer stayed frozen far
    // behind them (excluded from the oracle's liveness checks, so it does not
    // fail end-progress and the run still passes).
    let confirmed = &killed_report.final_confirmed;
    for (peer, &c) in confirmed.iter().enumerate() {
        if peer == 1 {
            continue;
        }
        assert!(
            c > 200,
            "survivor {peer} must keep confirming after the crash: {confirmed:?}"
        );
    }
    assert!(
        confirmed[1] < confirmed[0],
        "the crashed peer 1 must stay frozen behind the survivors: {confirmed:?}"
    );
}

/// Negative control for the peer crash: a real state divergence seeded into a
/// *surviving* peer must still be caught while another peer is crashed — the
/// alive-mask must exclude only the dead peer, never blind the oracle to the
/// survivors (the HD-1 "silent instrument" failure mode, checked against the
/// new op).
#[test]
fn oracle_catches_seeded_divergence_under_peer_kill() {
    let schedule = peer_kill_schedule(Some(1));
    // Corrupt peer 0 (a survivor) from frame 300 — *after* the crash at step
    // 100. This exercises the alive-mask's effect on the recorded-state
    // comparison (b): that check spans the globally-confirmed prefix, and
    // without excluding the crashed peer the prefix would be pinned at its
    // frozen frame (~99), so (b) would never reach frame 300. The in-band desync
    // detector still catches the corruption independently (so the *run* fails
    // either way) — the mask is what keeps the oracle's own state-agreement
    // signal from going blind past the crash. Assert `StateDivergence`
    // specifically, the class the mask restores.
    let options = RunOptions {
        corrupt_state_from: Some((0, 300)),
        ..RunOptions::default()
    };
    let report = run(&schedule, &options);
    assert!(
        !report.verdict.passed(),
        "a seeded divergence in a survivor must fail the run even under a crash"
    );
    assert!(
        report
            .verdict
            .failures
            .iter()
            .any(|f| matches!(f, OracleFailure::StateDivergence { .. })),
        "the state comparison (b) must reach the post-crash survivor prefix via \
         the alive-mask and flag StateDivergence (not just the in-band detector): {:?}",
        report.verdict.failures
    );
}

/// A peer that diverges *before* it crashes must not escape detection by being
/// killed. Corrupt the soon-to-be-killed peer 1 from frame 40, then crash it at
/// step 100: its pre-death recorded states (frames 40..~99) diverge from the
/// survivors and must still surface as `StateDivergence`. The alive-mask excises
/// a dead peer from the liveness checks, not from pre-death state agreement — so
/// a determinism bug on a doomed peer cannot hide behind its own crash.
#[test]
fn oracle_catches_pre_kill_divergence_on_the_killed_peer() {
    let schedule = peer_kill_schedule(Some(1));
    let options = RunOptions {
        corrupt_state_from: Some((1, 40)),
        ..RunOptions::default()
    };
    let report = run(&schedule, &options);
    assert!(
        !report.verdict.passed(),
        "a pre-crash divergence on the killed peer must fail the run"
    );
    assert!(
        report
            .verdict
            .failures
            .iter()
            .any(|f| matches!(f, OracleFailure::StateDivergence { .. })),
        "the killed peer's pre-death states must still be compared (b): {:?}",
        report.verdict.failures
    );
}

/// M3 §6.1 lifecycle vocabulary — the user-driven graceful drop
/// (`ScheduleEvent::GracefulRemove`). Under `ContinueWithout`, one survivor's
/// `remove_player(target)` call must be enough for the live mesh to converge on
/// the target's frozen slot and keep confirming together. This covers the real
/// public API path, not just organic timeout/crash detection.
#[test]
fn graceful_remove_survivors_converge_under_continue_without() {
    let base = graceful_remove_schedule(None);
    let removed = graceful_remove_schedule(Some((0, 1)));

    let base_report = run(&base, &RunOptions::default());
    base_report.expect_pass(&base);
    let removed_report = run(&removed, &RunOptions::default());
    removed_report.expect_pass(&removed);

    let removed_again = run(&removed, &RunOptions::default());
    assert_eq!(
        removed_report.trace_hash, removed_again.trace_hash,
        "a GracefulRemove schedule must reproduce its exact trace"
    );
    assert_ne!(
        base_report.trace_hash, removed_report.trace_hash,
        "GracefulRemove must change execution — a silent no-op would leave the \
         trace identical"
    );

    let confirmed = &removed_report.final_confirmed;
    for (peer, &c) in confirmed.iter().enumerate() {
        if peer == 1 {
            continue;
        }
        assert!(
            c > 200,
            "survivor {peer} must keep confirming after graceful remove: {confirmed:?}"
        );
    }
    assert!(
        confirmed[1] < confirmed[0],
        "the removed peer 1 must stay frozen behind the survivors: {confirmed:?}"
    );
}

/// M3 §6.2(d) spectator convergence: a pre-planned redundant spectator attached
/// to live survivors must display the same input bytes the mesh applies over
/// its confirmed prefix and agree on dropped-slot `Disconnected` statuses, even
/// after one player is gracefully removed.
#[test]
fn preplanned_spectator_matches_graceful_remove_mesh_canon() {
    let mut schedule = graceful_remove_schedule(Some((0, 1)));
    schedule.config.spectator_hosts = vec![0, 2, 3];

    let report = run(&schedule, &RunOptions::default());
    report.expect_pass(&schedule);
    assert!(
        report.spectator_max_frame >= Some(POST_HEAL_MIN_ADVANCE),
        "spectator must display post-remove frames; the oracle's scenario floor \
         would fail the run if it stopped before the lifecycle event: {:?}",
        report.spectator_max_frame
    );

    let again = run(&schedule, &RunOptions::default());
    assert_eq!(
        report.trace_hash, again.trace_hash,
        "a preplanned spectator schedule must reproduce its exact trace"
    );
}

/// M3 §6.1 lifecycle vocabulary — hot-join reactivation
/// (`ScheduleEvent::HotJoin`). A cleanly dropped slot must be refillable by a
/// fresh peer at the same address, and that rejoined slot must return to the
/// live mesh rather than staying behind the oracle's dead-peer mask.
#[cfg(feature = "hot-join")]
#[test]
fn hot_join_reactivates_cleanly_dropped_slot() {
    use fortress_rollback::MessageKind;

    let base = hot_join_schedule(None);
    let base_report = run(&base, &RunOptions::default());
    base_report.expect_pass(&base);

    let slot = 1;
    let joined = hot_join_schedule(Some(slot));
    let report = run(&joined, &RunOptions::default());
    report.expect_pass(&joined);

    let again = run(&joined, &RunOptions::default());
    assert_eq!(
        report.trace_hash, again.trace_hash,
        "a HotJoin schedule for slot {slot} must reproduce its exact trace"
    );
    assert_ne!(
        base_report.trace_hash, report.trace_hash,
        "HotJoin slot {slot} must change execution — a silent no-op would \
         leave the trace identical"
    );

    for (peer, &confirmed) in report.final_confirmed.iter().enumerate() {
        assert!(
            confirmed > 200,
            "peer {peer} must be live and confirming after hot-join slot {slot}: {:?}",
            report.final_confirmed
        );
    }

    let join_requests: u64 = report
        .peer_wire
        .iter()
        .map(|wire| wire.sent_by_kind(MessageKind::JoinRequest))
        .sum();
    let snapshots: u64 = report
        .peer_wire
        .iter()
        .map(|wire| wire.sent_by_kind(MessageKind::StateSnapshot))
        .sum();
    assert!(
        join_requests > 0 && snapshots > 0,
        "hot-join slot {slot} wire traffic must be present \
         (join_requests={join_requests}, snapshots={snapshots})"
    );
}

/// A `HotJoin` event must never silently no-op against a slot already retired by
/// an earlier runtime lifecycle API. `GracefulRemove` is runtime-contingent, so
/// static validation cannot reject it; the runner must report it through the
/// oracle when the hot-join event fires.
#[cfg(feature = "hot-join")]
#[test]
fn hot_join_after_runtime_remove_fails_loudly() {
    let schedule = hot_join_after_runtime_remove_schedule();

    let report = run(&schedule, &RunOptions::default());
    assert!(
        !report.verdict.passed(),
        "hot-joining an already-retired slot must fail loudly"
    );
    assert!(
        report.verdict.failures.iter().any(|failure| {
            matches!(
                failure,
                OracleFailure::SessionError {
                    operation: "hot_join_slot_unavailable",
                    peer: 1,
                    ..
                }
            )
        }),
        "expected explicit hot_join_slot_unavailable failure, got {:?}",
        report.verdict.failures
    );
}

/// The hot-join generation boundary intentionally discards the departing
/// slot's trailing handoff-window confirmed-input authorship, because the
/// coordinator freezes that window when it cleanly removes the slot. It must
/// not hide settled pre-handoff determinism bugs.
#[cfg(feature = "hot-join")]
#[test]
fn oracle_catches_settled_pre_handoff_divergence_under_hot_join() {
    let schedule = hot_join_schedule(Some(1));
    let options = RunOptions {
        corrupt_state_from: Some((1, 80)),
        ..RunOptions::default()
    };

    let report = run(&schedule, &options);
    assert!(
        !report.verdict.passed(),
        "a pre-handoff divergence on the departing slot must still fail"
    );
    assert!(
        report
            .verdict
            .failures
            .iter()
            .any(|failure| matches!(failure, OracleFailure::StateDivergence { .. })),
        "expected StateDivergence before the hot-join handoff, got {:?}",
        report.verdict.failures
    );
}

/// M3 §6.1/§6.2(d): a redundant spectator must fail over when its first,
/// highest-priority configured host is partitioned away from the mesh and then
/// crashes. The event is distinct from a plain `PeerKill` so corpus schedules can
/// state the spectator precondition explicitly: the killed peer must be one of
/// `spectator_hosts`, and the spectator must keep displaying from the remaining
/// hosts after the crash.
#[test]
fn spectator_failover_survives_configured_host_kill_under_partition() {
    let base = spectator_host_kill_schedule(None, None);
    let partitioned = spectator_host_kill_schedule(Some(0), None);
    let killed = spectator_host_kill_schedule(Some(0), Some(0));

    let base_report = run(&base, &RunOptions::default());
    base_report.expect_pass(&base);
    let partitioned_report = run(&partitioned, &RunOptions::default());
    let report = run(&killed, &RunOptions::default());
    report.expect_pass(&killed);
    assert!(
        !partitioned_report.verdict.passed(),
        "partition-only control should fail closed while the stale host remains \
         connected; SpectatorHostKill is what removes it"
    );
    assert_eq!(
        partitioned_report.spectator_final_hosts,
        Some(3),
        "without the host-kill event the spectator should still retain all three hosts"
    );
    assert!(
        partitioned_report.verdict.failures.iter().any(|failure| {
            matches!(
                failure,
                OracleFailure::SpectatorDivergenceEvent { .. }
                    | OracleFailure::SpectatorSessionError { .. }
            )
        }),
        "partition-only control should fail through spectator divergence, not an \
         unrelated oracle class: {:?}",
        partitioned_report.verdict.failures
    );

    let again = run(&killed, &RunOptions::default());
    assert_eq!(
        report.trace_hash, again.trace_hash,
        "a SpectatorHostKill schedule must reproduce its exact trace"
    );
    assert_ne!(
        partitioned_report.trace_hash, report.trace_hash,
        "SpectatorHostKill itself must change execution beyond the partition — \
         a silent no-op would leave the partition-only trace identical"
    );

    assert!(
        report.spectator_max_frame >= Some(POST_HEAL_MIN_ADVANCE),
        "spectator must keep displaying after the configured host crash: {:?}",
        report.spectator_max_frame
    );
    assert_eq!(
        report.spectator_final_hosts,
        Some(2),
        "the spectator must remove the crashed host and keep the two remaining hosts"
    );
    assert!(
        report.net_stats.dropped_blocked > 0,
        "the host partition must actually drop traffic before the crash: {:?}",
        report.net_stats
    );
    let confirmed = &report.final_confirmed;
    for (peer, &c) in confirmed.iter().enumerate() {
        if peer == 0 {
            continue;
        }
        assert!(
            c > 200,
            "survivor {peer} must keep confirming after spectator host crash: {confirmed:?}"
        );
    }
    assert!(
        confirmed[0] < confirmed[1],
        "the killed spectator host must stay frozen behind survivors: {confirmed:?}"
    );
}

const SPECTATOR_FLOOR_STALL_AT: u32 = 30;
const SPECTATOR_FLOOR_REMOVE_AT: u32 = 80;

fn stalled_spectator_floor_schedule(steps: u32) -> Schedule {
    let mut schedule = graceful_remove_schedule(Some((0, 1)));
    schedule.config.steps = steps;
    schedule.config.spectator_hosts = vec![0, 2, 3];
    schedule.heal_at = steps - 1;
    schedule.events = vec![
        (
            SPECTATOR_FLOOR_STALL_AT,
            ScheduleEvent::PeerStall {
                peer: 0,
                steps: SPECTATOR_FLOOR_REMOVE_AT - SPECTATOR_FLOOR_STALL_AT,
            },
        ),
        (
            SPECTATOR_FLOOR_STALL_AT,
            ScheduleEvent::PeerStall {
                peer: 1,
                steps: SPECTATOR_FLOOR_REMOVE_AT - SPECTATOR_FLOOR_STALL_AT,
            },
        ),
        (
            SPECTATOR_FLOOR_STALL_AT,
            ScheduleEvent::PeerStall {
                peer: 2,
                steps: SPECTATOR_FLOOR_REMOVE_AT - SPECTATOR_FLOOR_STALL_AT,
            },
        ),
        (
            SPECTATOR_FLOOR_STALL_AT,
            ScheduleEvent::PeerStall {
                peer: 3,
                steps: SPECTATOR_FLOOR_REMOVE_AT - SPECTATOR_FLOOR_STALL_AT,
            },
        ),
        (
            SPECTATOR_FLOOR_REMOVE_AT,
            ScheduleEvent::GracefulRemove { by: 0, target: 1 },
        ),
    ];
    schedule
}

/// Regression for Bugbot f256657d: the spectator post-drop progress floor is a
/// displayed-frame threshold, not a schedule-step threshold. This schedule
/// stalls the live spectator hosts before removal, so virtual steps race far
/// ahead of their simulated frames; the run should still pass once the
/// spectator displays G frames past the survivors' actual removal-time frame.
#[test]
fn spectator_post_drop_floor_uses_display_frames_not_schedule_steps() {
    let schedule = stalled_spectator_floor_schedule(130);

    let report = run(&schedule, &RunOptions::default());
    report.expect_pass(&schedule);

    let max_frame = report
        .spectator_max_frame
        .expect("passing spectator run must display frames");
    let old_step_domain_floor = i32::try_from(SPECTATOR_FLOOR_REMOVE_AT)
        .unwrap_or(i32::MAX)
        .saturating_add(POST_HEAL_MIN_ADVANCE);
    assert!(
        max_frame < old_step_domain_floor,
        "regression premise failed: old step-domain floor {old_step_domain_floor} \
         would not have failed with spectator max {max_frame}"
    );
}

/// Negative half of the same regression: if the run ends before the spectator
/// displays G frames past the removal-time game frame, the oracle must report
/// scenario-floor progress missing. Without wiring the runtime floor through to
/// the oracle this would have passed the spectator check after any display.
#[test]
fn oracle_catches_spectator_missing_display_frame_floor_after_drop() {
    let schedule = stalled_spectator_floor_schedule(90);

    let report = run(&schedule, &RunOptions::default());
    assert!(
        !report.verdict.passed(),
        "a spectator that stops before the post-drop frame floor must fail"
    );
    assert!(
        report.verdict.failures.iter().any(|failure| {
            matches!(
                failure,
                OracleFailure::SpectatorProgressMissing {
                    required_min_frame: Some(required),
                    observed_max_frame: Some(observed),
                    ..
                } if observed < required
            )
        }),
        "spectator progress failure must carry the runtime display-frame floor: {:?}",
        report.verdict.failures
    );
}

/// Negative control for §6.2(d): corrupting only the spectator's displayed
/// record must fail the spectator oracle while leaving the mesh path itself
/// untouched.
#[test]
fn oracle_catches_spectator_input_divergence_under_graceful_remove() {
    let mut schedule = graceful_remove_schedule(Some((0, 1)));
    schedule.config.spectator_hosts = vec![0, 2, 3];
    let options = RunOptions {
        corrupt_spectator_input_from: Some(0),
        ..RunOptions::default()
    };

    let report = run(&schedule, &options);
    assert!(
        !report.verdict.passed(),
        "a seeded spectator-only divergence must fail"
    );
    assert!(
        report
            .verdict
            .failures
            .iter()
            .any(|failure| matches!(failure, OracleFailure::SpectatorInputDivergence { .. })),
        "spectator mismatch must be visible to the oracle: {:?}",
        report.verdict.failures
    );
}

/// Negative control for the dropped-slot half of §6.2(d): once the mesh freezes
/// a gracefully removed slot as `Disconnected`, the spectator must report that
/// same status for the same input bytes.
#[test]
fn oracle_catches_spectator_disconnected_status_divergence_under_graceful_remove() {
    let mut schedule = graceful_remove_schedule(Some((0, 1)));
    schedule.config.spectator_hosts = vec![0, 2, 3];
    let options = RunOptions {
        corrupt_spectator_status_from: Some(100),
        ..RunOptions::default()
    };

    let report = run(&schedule, &options);
    assert!(
        !report.verdict.passed(),
        "a seeded spectator status divergence must fail"
    );
    assert!(
        report
            .verdict
            .failures
            .iter()
            .any(|failure| matches!(failure, OracleFailure::SpectatorInputDivergence { .. })),
        "spectator status mismatch must be visible to the oracle: {:?}",
        report.verdict.failures
    );
}

/// Malformed spectator host lists are schedule bugs, not runtime indexing
/// panics. Reject them before sessions are built.
#[test]
fn run_rejects_malformed_spectator_hosts() {
    let cases = [
        (vec![9], "spectator_hosts peer 9 out of range"),
        (vec![0, 0], "duplicate spectator_hosts peer 0"),
    ];

    for (hosts, expected) in cases {
        let mut schedule = graceful_remove_schedule(None);
        schedule.config.spectator_hosts = hosts;
        assert_run_panics_with(&schedule, expected);
    }
}

/// `SpectatorHostKill` has a stricter fail-loud contract than `PeerKill`: it
/// must name a valid peer that is actually configured as a spectator host.
#[test]
fn run_rejects_malformed_spectator_host_kill_events() {
    let cases = [
        (ScheduleEvent::SpectatorHostKill { host: 9 }, "out of range"),
        (
            ScheduleEvent::SpectatorHostKill { host: 1 },
            "not configured in spectator_hosts",
        ),
    ];

    for (event, expected) in cases {
        let mut schedule = spectator_host_kill_schedule(None, None);
        schedule.events.push((100, event));
        schedule.events.sort_by_key(|(step, _)| *step);
        assert_run_panics_with(&schedule, expected);
    }

    let mut already_retired = spectator_host_kill_schedule(None, None);
    already_retired
        .events
        .push((90, ScheduleEvent::PeerKill { peer: 0 }));
    already_retired
        .events
        .push((100, ScheduleEvent::SpectatorHostKill { host: 0 }));
    already_retired.events.sort_by_key(|(step, _)| *step);
    assert_run_panics_with(&already_retired, "already retired");

    let mut skipped_prior_drop = spectator_host_kill_schedule(None, None);
    skipped_prior_drop
        .events
        .push((80, ScheduleEvent::PeerKill { peer: 2 }));
    skipped_prior_drop
        .events
        .push((90, ScheduleEvent::GracefulRemove { by: 2, target: 0 }));
    skipped_prior_drop
        .events
        .push((100, ScheduleEvent::SpectatorHostKill { host: 0 }));
    skipped_prior_drop.events.sort_by_key(|(step, _)| *step);
    let result = std::panic::catch_unwind(|| {
        let _ = run(&skipped_prior_drop, &RunOptions::default());
    });
    assert!(
        result.is_ok(),
        "a lifecycle drop from an already-dead caller is a runtime no-op and \
         must not make a later SpectatorHostKill look already retired"
    );

    let runtime_contingent_drops = [
        (
            "GracefulRemove",
            ScheduleEvent::GracefulRemove { by: 1, target: 0 },
        ),
        (
            "LegacyDisconnect",
            ScheduleEvent::LegacyDisconnect { by: 1, target: 0 },
        ),
    ];
    for (label, prior_drop) in runtime_contingent_drops {
        let mut schedule = spectator_host_kill_schedule(None, None);
        schedule.events.push((90, prior_drop));
        schedule
            .events
            .push((100, ScheduleEvent::SpectatorHostKill { host: 0 }));
        schedule.events.sort_by_key(|(step, _)| *step);
        let result = std::panic::catch_unwind(|| {
            let _ = run(&schedule, &RunOptions::default());
        });
        assert!(
            result.is_ok(),
            "{label} is a runtime-contingent API call and must not make a later \
             SpectatorHostKill fail static validation"
        );
    }
}

/// Negative control for graceful remove: the alive mask must exclude only the
/// removed peer. A real divergence seeded into a survivor after the removal must
/// still reach the state-agreement oracle, not be hidden by the retired target's
/// frozen confirmed frame.
#[test]
fn oracle_catches_seeded_divergence_under_graceful_remove() {
    let schedule = graceful_remove_schedule(Some((0, 1)));
    let options = RunOptions {
        corrupt_state_from: Some((2, 300)),
        ..RunOptions::default()
    };
    let report = run(&schedule, &options);
    assert!(
        !report.verdict.passed(),
        "a seeded divergence in a survivor must fail under graceful remove"
    );
    assert!(
        report
            .verdict
            .failures
            .iter()
            .any(|f| matches!(f, OracleFailure::StateDivergence { .. })),
        "the state comparison must keep its teeth after graceful remove: {:?}",
        report.verdict.failures
    );
}

/// M3 §6.1 lifecycle vocabulary — the legacy user disconnect
/// (`ScheduleEvent::LegacyDisconnect`). Unlike `GracefulRemove`,
/// `disconnect_player` takes the Halt-oriented path, so this test does NOT
/// assert survivor convergence. It pins the executable harness vocabulary and
/// the fail-closed observation: after a clean network heal, the live peers do
/// not recover within B and the oracle reports that non-recovery.
#[test]
fn legacy_disconnect_reports_halt_non_recovery() {
    let base = legacy_disconnect_schedule(None);
    let disconnected = legacy_disconnect_schedule(Some((0, 1)));

    let base_report = run(&base, &RunOptions::default());
    base_report.expect_pass(&base);
    let report = run(&disconnected, &RunOptions::default());

    assert!(
        !report.verdict.passed(),
        "legacy disconnect is expected to report Halt non-recovery today, not \
         pass as a graceful drop"
    );
    assert_eq!(
        report.recovered_within_b,
        Some(false),
        "a legacy-disconnected Halt mesh healed but did not recover"
    );

    let expected_live: BTreeSet<usize> = [0usize, 2, 3].into_iter().collect();
    let expected_advance_errors: BTreeSet<usize> = [2usize, 3].into_iter().collect();
    let mut advance_error_peers = BTreeSet::new();
    let mut end_progress_peers = BTreeSet::new();
    let mut post_heal_peers = BTreeSet::new();
    for failure in &report.verdict.failures {
        match failure {
            OracleFailure::SessionError {
                operation: "advance_frame",
                peer,
                error,
                ..
            } => {
                assert!(
                    expected_advance_errors.contains(peer),
                    "only peers that learn the propagated Halt through \
                     advance_frame should report NotSynchronized: {:?}",
                    report.verdict.failures
                );
                assert!(
                    error.contains("NotSynchronized"),
                    "legacy disconnect should surface only NotSynchronized \
                     advance errors, got {error:?}: {:?}",
                    report.verdict.failures
                );
                advance_error_peers.insert(*peer);
            },
            OracleFailure::EndProgress { peer, state, .. } => {
                assert!(
                    expected_live.contains(peer),
                    "target peer 1 is retired; only live peers should fail \
                     end-progress: {:?}",
                    report.verdict.failures
                );
                assert_eq!(
                    *state,
                    SessionState::Synchronizing,
                    "legacy disconnect should leave live peers Halt-stalled in \
                     Synchronizing today: {:?}",
                    report.verdict.failures
                );
                end_progress_peers.insert(*peer);
            },
            OracleFailure::PostHealLiveness { peer, .. } => {
                assert!(
                    expected_live.contains(peer),
                    "target peer 1 is retired; only live peers should fail \
                     post-heal liveness: {:?}",
                    report.verdict.failures
                );
                post_heal_peers.insert(*peer);
            },
            _ => panic!(
                "unexpected failure class for a clean legacy-disconnect Halt \
                 probe: {failure:?}\nall failures: {:?}",
                report.verdict.failures
            ),
        }
    }
    assert_eq!(
        advance_error_peers, expected_advance_errors,
        "peers 2 and 3 should learn the propagated Halt during advance_frame; \
         this keeps the expected-failure shape precise: {:?}",
        report.verdict.failures
    );
    assert_eq!(
        end_progress_peers, expected_live,
        "all and only live peers must fail end-progress after the legacy \
         disconnect; this catches regressions where only the caller halts: {:?}",
        report.verdict.failures
    );
    assert_eq!(
        post_heal_peers, expected_live,
        "all and only live peers must fail bounded post-heal liveness after \
         the legacy disconnect: {:?}",
        report.verdict.failures
    );
    assert!(
        !report.verdict.failures.iter().any(|f| matches!(
            f,
            OracleFailure::SessionError {
                operation: "disconnect_player",
                ..
            }
        )),
        "the harness must call disconnect_player successfully; failures are the \
         protocol outcome, not an API error: {:?}",
        report.verdict.failures
    );

    let again = run(&disconnected, &RunOptions::default());
    assert_eq!(
        report.trace_hash, again.trace_hash,
        "a LegacyDisconnect schedule must reproduce its exact trace"
    );
    assert_ne!(
        base_report.trace_hash, report.trace_hash,
        "LegacyDisconnect must change execution — a silent no-op would leave \
         the trace identical"
    );
}

/// A peer that diverges *before* a legacy disconnect must not escape detection
/// by being retired. This is the `LegacyDisconnect` analogue of the peer-kill
/// and graceful-remove alive-mask negative controls: retirement excludes the
/// target from liveness only, not from pre-retirement state agreement.
#[test]
fn oracle_catches_pre_disconnect_divergence_on_the_legacy_target() {
    let schedule = legacy_disconnect_schedule(Some((0, 1)));
    let options = RunOptions {
        corrupt_state_from: Some((1, 40)),
        ..RunOptions::default()
    };
    let report = run(&schedule, &options);
    assert!(
        !report.verdict.passed(),
        "a pre-disconnect divergence on the retired target must fail the run"
    );
    assert!(
        report
            .verdict
            .failures
            .iter()
            .any(|f| matches!(f, OracleFailure::StateDivergence { .. })),
        "the legacy-disconnected peer's pre-retirement states must still be \
         compared: {:?}",
        report.verdict.failures
    );
}

/// Malformed `LegacyDisconnect` events share the lifecycle-drop validation
/// contract with `GracefulRemove`: bad caller, bad target, and self-target are
/// rejected before the runner can raw-index or accidentally call the API on a
/// local handle.
#[test]
fn run_rejects_malformed_legacy_disconnect_events() {
    let cases = [
        (
            ScheduleEvent::LegacyDisconnect { by: 9, target: 1 },
            "out of range",
        ),
        (
            ScheduleEvent::LegacyDisconnect { by: 0, target: 9 },
            "out of range",
        ),
        (
            ScheduleEvent::LegacyDisconnect { by: 1, target: 1 },
            "target must be remote",
        ),
    ];

    for (event, expected) in cases {
        let mut schedule = legacy_disconnect_schedule(None);
        schedule.events.push((100, event));
        schedule.events.sort_by_key(|(step, _)| *step);
        let result = std::panic::catch_unwind(|| {
            let _ = run(&schedule, &RunOptions::default());
        });
        let Err(payload) = result else {
            panic!("malformed LegacyDisconnect event unexpectedly ran: {schedule:?}");
        };
        let message = panic_payload_to_string(payload.as_ref());
        assert!(
            message.contains(expected),
            "panic should mention {expected:?}, got {message:?}"
        );
    }
}

/// A checksum-only desync must fail even when the app model starves every
/// session event queue. Without the metrics-backed oracle path, `DesyncDetected`
/// can sit undrained (or be discarded by D9 trimming), states still agree, and
/// the run can report a false green.
#[test]
fn checksum_mismatch_metric_catches_starved_desync_event() {
    let config = SimConfig {
        n_players: 2,
        noise: BackgroundNoise::Clean,
        starve_events: vec![0, 1],
        event_queue_size: Some(10),
        ..SimConfig::smoke(2)
    };
    let mut schedule = generate(13, config);
    schedule.events.clear();
    schedule.heal_at = schedule.config.steps;
    let options = RunOptions {
        corrupt_checksum_from: Some((0, 40)),
        ..RunOptions::default()
    };
    let report = run(&schedule, &options);
    assert!(
        !report.verdict.passed(),
        "a checksum mismatch must fail even when DesyncDetected events are starved"
    );
    assert!(
        report
            .verdict
            .failures
            .iter()
            .any(|f| matches!(f, OracleFailure::ChecksumMismatchMetric { .. })),
        "metrics-backed checksum mismatch must be visible to the oracle: {:?}",
        report.verdict.failures
    );
}

/// A `PeerKill` naming a peer outside the mesh must fail loudly with the same
/// clear message as the other events (per-event fail-loud contract of the
/// up-front validation).
#[test]
#[should_panic(expected = "out of range")]
fn run_rejects_out_of_range_peer_kill() {
    let mut schedule = peer_kill_schedule(None);
    schedule
        .events
        .push((100, ScheduleEvent::PeerKill { peer: 9 }));
    schedule.events.sort_by_key(|(step, _)| *step);
    let _ = run(&schedule, &RunOptions::default());
}

fn panic_payload_to_string(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&'static str>() {
        (*message).to_owned()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "<non-string panic payload>".to_owned()
    }
}

/// Malformed `GracefulRemove` events must fail during up-front schedule
/// validation with clear corpus-replay diagnostics, not as raw indexing panics
/// in the runner body. The cases cover the whole class: bad caller, bad target,
/// and a self-target that would make the target local rather than remote.
#[test]
fn run_rejects_malformed_graceful_remove_events() {
    let cases = [
        (
            ScheduleEvent::GracefulRemove { by: 9, target: 1 },
            "out of range",
        ),
        (
            ScheduleEvent::GracefulRemove { by: 0, target: 9 },
            "out of range",
        ),
        (
            ScheduleEvent::GracefulRemove { by: 1, target: 1 },
            "target must be remote",
        ),
    ];

    for (event, expected) in cases {
        let mut schedule = graceful_remove_schedule(None);
        schedule.events.push((100, event));
        schedule.events.sort_by_key(|(step, _)| *step);
        let result = std::panic::catch_unwind(|| {
            let _ = run(&schedule, &RunOptions::default());
        });
        let Err(payload) = result else {
            panic!("malformed GracefulRemove event unexpectedly ran: {schedule:?}");
        };
        let message = panic_payload_to_string(payload.as_ref());
        assert!(
            message.contains(expected),
            "panic should mention {expected:?}, got {message:?}"
        );
    }
}

fn assert_run_panics_with(schedule: &Schedule, expected: &str) {
    let result = std::panic::catch_unwind(|| {
        let _ = run(schedule, &RunOptions::default());
    });
    let Err(payload) = result else {
        panic!("invalid schedule unexpectedly ran: {schedule:?}");
    };
    let message = panic_payload_to_string(payload.as_ref());
    assert!(
        message.contains(expected),
        "panic should mention {expected:?}, got {message:?}"
    );
}

/// Hand-authored/corpus schedules must satisfy the same invariants as generated
/// schedules. Otherwise the runner can silently run an empty mesh, skip a late
/// event, reorder lifecycle faults, or default a missing directed link to clean.
#[test]
fn run_rejects_invalid_materialized_schedule_invariants() {
    let valid = graceful_remove_schedule(None);

    let mut zero_players = valid.clone();
    zero_players.config.n_players = 0;
    zero_players.initial_links.clear();
    zero_players.events.clear();
    zero_players.heal_at = zero_players.config.steps;

    let mut unsorted_events = valid.clone();
    unsorted_events.events.push((10, ScheduleEvent::HealAll));

    let mut event_outside_run = valid.clone();
    event_outside_run
        .events
        .push((event_outside_run.config.steps, ScheduleEvent::HealAll));

    let mut missing_link = valid.clone();
    let _ = missing_link.initial_links.pop();

    let mut self_link = valid.clone();
    if let Some(first) = self_link.initial_links.first_mut() {
        first.1 = first.0;
    }

    let mut self_link_event = valid.clone();
    self_link_event.events.push((
        10,
        ScheduleEvent::Block {
            from: 0,
            to: 0,
            blocked: true,
        },
    ));
    self_link_event.events.sort_by_key(|(step, _)| *step);

    let mut duplicate_link = valid.clone();
    if let Some(first) = duplicate_link.initial_links.first().cloned() {
        let _ = duplicate_link.initial_links.pop();
        duplicate_link.initial_links.push(first);
    }

    let mut post_heal_fault = valid;
    post_heal_fault.events.push((
        post_heal_fault.heal_at + 1,
        ScheduleEvent::Block {
            from: 0,
            to: 1,
            blocked: true,
        },
    ));

    let cases = [
        (zero_players, "2..=16 players"),
        (unsorted_events, "sorted"),
        (event_outside_run, "outside the run"),
        (missing_link, "exactly one directed policy"),
        (self_link, "must not target itself"),
        (self_link_event, "must not target itself"),
        (duplicate_link, "duplicate initial link"),
        (post_heal_fault, "after the last HealAll"),
    ];

    for (schedule, expected) in cases {
        assert_run_panics_with(&schedule, expected);
    }
}

/// Builds a schedule that reliably drives `WaitRecommendation`s: an `n`-mesh
/// over clean links with **asymmetric one-way delay** (lower→higher index 20ms,
/// the reverse 120ms). The asymmetry biases the RTT/2 frame-advantage estimate
/// (defect D11), so a persistent advantage builds and the time-sync loop emits
/// skip recommendations — a symmetric delay produces none (both peers lockstep).
/// `app_model` selects whether the harness obeys them.
fn wait_rec_schedule(n: usize, app_model: AppModel) -> Schedule {
    let config = SimConfig {
        n_players: n,
        steps: 900,
        noise: BackgroundNoise::Clean,
        app_model,
        ..SimConfig::smoke(n)
    };
    let link = |ms: u64| LinkPolicy {
        drop_rate: 0.0,
        dup_rate: 0.0,
        base_delay: Duration::from_millis(ms),
        jitter: Duration::ZERO,
        burst_rate: 0.0,
        burst_len: 0,
        retransmit_delay: Duration::ZERO,
        gilbert_elliott: None,
        fragmentation: None,
        bandwidth: None,
    };
    let mut initial_links = Vec::new();
    for from in 0..n {
        for to in 0..n {
            if from != to {
                let ms = if from < to { 20 } else { 120 };
                initial_links.push((from, to, link(ms)));
            }
        }
    }
    let heal_at = 650;
    let mut events = vec![(heal_at, ScheduleEvent::HealAll)];
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

/// M3 §6.6-pre.3 — the WaitRecommendation-obeying app model. The reference
/// client and every prior fleet run *ignore* `WaitRecommendation`, leaving the
/// time-sync control loop **open**, so oscillation (H-OSC) is unobservable.
/// Obeying it (skip the recommended advances) closes the loop.
///
/// This is the H-OSC *precondition infra* plus an asymmetric-delay
/// side-observation — **not** the full H-OSC experiment, which is symmetric
/// delay + Obey (mutual-sleep contention, FM-3) and is still owed. Under this
/// one-sided (asymmetric-delay) obedience the closed loop must stay consistent
/// and live: every peer confirms a byte-identical *state* prefix whether obeying
/// or ignoring (both pass the oracle), even though the execution *trace* differs
/// (Obey paces the ahead peer differently). Obeying must not diverge or deadlock
/// the mesh.
///
/// Premise: under Obey (the run whose obedience is under test) the schedule
/// emits real `WaitRecommendation`s, and obeying vs ignoring produces different
/// traces (the skips actually happened).
#[test]
fn app_model_obey_wait_recommendation_stays_consistent() {
    for n in [2usize, 4] {
        let ignore = wait_rec_schedule(n, AppModel::Ignore);
        let obey = wait_rec_schedule(n, AppModel::Obey);

        let ignore_report = run(&ignore, &RunOptions::default());
        ignore_report.expect_pass(&ignore);
        let obey_report = run(&obey, &RunOptions::default());
        obey_report.expect_pass(&obey);

        // The Obey run itself must emit WaitRecommendations, or there was
        // nothing to obey (the H-OSC precondition — measured on the run under
        // test, not the Ignore control).
        let obey_wait_recs: u64 = obey_report
            .metrics
            .iter()
            .map(|m| m.wait_recommendations)
            .sum();
        assert!(
            obey_wait_recs > 0,
            "the delayed schedule must emit WaitRecommendations under Obey to \
             probe H-OSC (n={n}, got {obey_wait_recs})"
        );

        let obey_again = run(&obey, &RunOptions::default());
        assert!(!obey_report.progress_samples.is_empty(), "n={n}");
        assert!(obey_report.progress_samples.len() <= 12, "n={n}");
        assert_eq!(
            obey_report.progress_samples, obey_again.progress_samples,
            "bounded control-loop samples must replay exactly (n={n})"
        );
        for sample in &obey_report.progress_samples {
            assert_eq!(sample.endpoints.len(), n * (n - 1), "n={n}");
            assert_eq!(sample.link_queues.len(), n * (n - 1), "n={n}");
            assert!(
                sample
                    .endpoints
                    .windows(2)
                    .all(|pair| (pair[0].from, pair[0].to) < (pair[1].from, pair[1].to)),
                "endpoint samples must use stable directed-link order (n={n})"
            );
            assert!(
                sample
                    .link_queues
                    .iter()
                    .all(|queue| queue.queued_bytes == 0
                        && queue.queued_datagrams == 0
                        && queue.drain_delay_ns == 0),
                "delay-only links must not report bandwidth debt (n={n})"
            );
        }
        assert!(
            obey_report
                .progress_samples
                .iter()
                .flat_map(|sample| &sample.endpoints)
                .any(|endpoint| endpoint.ping_ms > 0),
            "the control-loop series must observe measured RTT (n={n})"
        );
        assert_eq!(
            obey_report.trace_hash, obey_again.trace_hash,
            "an Obey run must reproduce its exact trace (n={n})"
        );
        assert_ne!(
            ignore_report.trace_hash, obey_report.trace_hash,
            "obeying WaitRecommendation must change execution — the skips must \
             actually happen (n={n})"
        );
    }
}

/// H-OSC symmetric control: identical 100 ms one-way links do not activate
/// the discrete sleep controller in the deterministic equal-rate workload.
/// This falsifies the original perfectly-symmetric mutual-sleep premise; a
/// perturbation/jitter treatment is still required to test oscillation after
/// the loop is actually activated.
#[test]
fn symmetric_delay_does_not_create_mutual_sleep_h_osc() {
    for n in [2usize, 4] {
        let mut schedule = wait_rec_schedule(n, AppModel::Obey);
        schedule.config.steps = 900;
        for (_, _, policy) in &mut schedule.initial_links {
            policy.base_delay = Duration::from_millis(100);
        }
        schedule.events.clear();
        schedule.heal_at = schedule.config.steps;

        let first = run(&schedule, &RunOptions::default());
        let replay = run(&schedule, &RunOptions::default());
        first.expect_pass(&schedule);
        assert_eq!(first.trace_hash, replay.trace_hash, "n={n}");
        assert_eq!(first.progress_samples, replay.progress_samples, "n={n}");
        assert!(!first.progress_samples.is_empty(), "n={n}");
        assert_eq!(first.wait_frames_obeyed, vec![0; n], "n={n}");
        assert!(
            first
                .metrics
                .iter()
                .all(|metrics| metrics.wait_recommendations == 0),
            "perfectly symmetric delay must not manufacture a peer lead: {:?}",
            first.metrics
        );
        assert!(
            first.progress_samples.last().is_some_and(|sample| sample
                .endpoints
                .iter()
                .all(|endpoint| (190..=230).contains(&endpoint.ping_ms))),
            "every endpoint must observe the intended ≈200 ms RTT (n={n}): {:?}",
            first.progress_samples
        );
    }
}

/// H-ASYM matched experiment: a 10/200 ms one-way split has the same 210 ms
/// RTT as its symmetric control. It creates a measurable throughput/stall
/// asymmetry, but the reported advantages stay below the sleep dead band and
/// falsify the predicted one-sided `WaitRecommendation` mechanism.
#[test]
fn h_asym_biases_throughput_without_wait_recommendations() {
    let build = |asymmetric: bool| {
        let mut schedule = wait_rec_schedule(2, AppModel::Obey);
        for (from, to, policy) in &mut schedule.initial_links {
            policy.base_delay = if asymmetric {
                if from < to {
                    Duration::from_millis(10)
                } else {
                    Duration::from_millis(200)
                }
            } else {
                Duration::from_millis(105)
            };
        }
        schedule.events.clear();
        schedule.heal_at = schedule.config.steps;
        schedule
    };
    let symmetric = build(false);
    let asymmetric = build(true);
    let symmetric_report = run(&symmetric, &RunOptions::default());
    let asymmetric_report = run(&asymmetric, &RunOptions::default());
    let replay = run(&asymmetric, &RunOptions::default());
    symmetric_report.expect_pass(&symmetric);
    asymmetric_report.expect_pass(&asymmetric);
    assert_eq!(asymmetric_report.trace_hash, replay.trace_hash);
    assert_eq!(asymmetric_report.progress_samples, replay.progress_samples);
    assert_eq!(symmetric_report.wait_frames_obeyed, vec![0, 0]);
    assert_eq!(asymmetric_report.wait_frames_obeyed, vec![0, 0]);
    assert!(symmetric_report
        .metrics
        .iter()
        .all(|metrics| metrics.wait_recommendations == 0));
    assert!(asymmetric_report
        .metrics
        .iter()
        .all(|metrics| metrics.wait_recommendations == 0));
    assert_eq!(
        symmetric_report.metrics[0].visual_frames,
        symmetric_report.metrics[1].visual_frames
    );
    assert_eq!(
        symmetric_report.metrics[0].stall_count,
        symmetric_report.metrics[1].stall_count
    );
    assert!(
        asymmetric_report.metrics[1].visual_frames > asymmetric_report.metrics[0].visual_frames
    );
    assert!(asymmetric_report.metrics[0].stall_count > asymmetric_report.metrics[1].stall_count);
    let final_sample = asymmetric_report
        .progress_samples
        .last()
        .expect("matched asymmetric run records bounded samples");
    let symmetric_final = symmetric_report
        .progress_samples
        .last()
        .expect("matched symmetric control records bounded samples");
    assert_eq!(
        final_sample.current_frames[1] - final_sample.current_frames[0],
        7,
        "10/200 ms paths should produce the measured seven-frame throughput split"
    );
    assert_eq!(
        symmetric_final
            .endpoints
            .iter()
            .map(|endpoint| endpoint.ping_ms)
            .collect::<Vec<_>>(),
        final_sample
            .endpoints
            .iter()
            .map(|endpoint| endpoint.ping_ms)
            .collect::<Vec<_>>(),
        "control and treatment must observe the same RTT"
    );
    assert!(
        final_sample
            .endpoints
            .iter()
            .all(|endpoint| (210..=240).contains(&endpoint.ping_ms)),
        "both treatment endpoints must observe the matched ≈210 ms RTT"
    );
    assert_eq!(
        final_sample
            .endpoints
            .iter()
            .map(|endpoint| endpoint.remote_frame_advantage)
            .collect::<Vec<_>>(),
        vec![-1, 1],
        "the latest wire gauges stay below the three-frame recommendation dead band"
    );
}

/// Builds a clock-skew schedule: an `n`-mesh over clean links with a small
/// symmetric delay (40ms each way, so RTT is non-zero and a skewed clock
/// misreads it), and the given per-peer clock-rate skew in ppm. The delay holds
/// for the whole run — no `HealAll`, so `heal_all` never removes it — keeping
/// RTT non-zero throughout (constant conditions, unlike the heal-and-drain
/// schedules).
fn clock_skew_schedule(n: usize, skew: Vec<i32>) -> Schedule {
    let config = SimConfig {
        n_players: n,
        steps: 900,
        noise: BackgroundNoise::Clean,
        clock_skew_ppm: skew,
        ..SimConfig::smoke(n)
    };
    let delayed = LinkPolicy {
        drop_rate: 0.0,
        dup_rate: 0.0,
        base_delay: Duration::from_millis(40),
        jitter: Duration::ZERO,
        burst_rate: 0.0,
        burst_len: 0,
        retransmit_delay: Duration::ZERO,
        gilbert_elliott: None,
        fragmentation: None,
        bandwidth: None,
    };
    let mut initial_links = Vec::new();
    for from in 0..n {
        for to in 0..n {
            if from != to {
                initial_links.push((from, to, delayed.clone()));
            }
        }
    }
    Schedule {
        schema_version: SCHEDULE_SCHEMA_VERSION,
        seed: 0,
        link_seed: 0,
        config,
        initial_links,
        events: Vec::new(),
        // Past the last step: nothing to heal, the delay is constant.
        heal_at: 900,
    }
}

/// M3 §6.6-pre.2 — per-peer clock skew (H-SKEW precondition). Peers whose local
/// clocks run at different rates than the network's real time must not diverge
/// the mesh: the game logic is driven by per-(step, peer) inputs, not the clock,
/// so a bounded skew only perturbs *timing-gated* behavior (RTT measurement,
/// quality-report/keepalive cadence, frame-advantage estimate) — the confirmed
/// state prefix stays byte-identical. This pins that a skewed peer is tolerated.
///
/// Premise: a skewed peer's clock demonstrably alters execution (the trace
/// differs from the all-exact run) while both pass the oracle; determinism holds.
#[test]
fn clock_skew_is_tolerated_and_alters_execution() {
    for n in [2usize, 4] {
        let exact = clock_skew_schedule(n, Vec::new());
        // Peer 0's local clock runs 10% fast; the rest keep real time.
        let mut skew = vec![0; n];
        skew[0] = 100_000;
        let skewed = clock_skew_schedule(n, skew);

        let exact_report = run(&exact, &RunOptions::default());
        exact_report.expect_pass(&exact);
        let skewed_report = run(&skewed, &RunOptions::default());
        skewed_report.expect_pass(&skewed);

        let skewed_again = run(&skewed, &RunOptions::default());
        assert_eq!(
            skewed_report.trace_hash, skewed_again.trace_hash,
            "a clock-skew schedule must reproduce its exact trace (n={n})"
        );
        assert_ne!(
            exact_report.trace_hash, skewed_report.trace_hash,
            "a skewed clock must alter timing-gated execution (n={n})"
        );
    }
}

/// Builds a long-run clock-skew schedule: a 2-mesh over clean 30ms-delay links
/// where peer 0's clock runs `ppm` fast for `steps` steps, app model `Ignore`.
/// The harness advances every peer one frame per step (lockstep) and reads the
/// clock only for timestamps, so the skew perturbs timing-gated behavior (the
/// RTT gauge, quality-report/keepalive cadence) but does NOT gate frame
/// production — see the probe below for why this bounds what it can test.
fn clock_skew_long_run_schedule(steps: u32, ppm: i32) -> Schedule {
    let config = SimConfig {
        n_players: 2,
        steps,
        noise: BackgroundNoise::Clean,
        clock_skew_ppm: vec![ppm, 0],
        ..SimConfig::smoke(2)
    };
    let delayed = LinkPolicy {
        drop_rate: 0.0,
        dup_rate: 0.0,
        base_delay: Duration::from_millis(30),
        jitter: Duration::ZERO,
        burst_rate: 0.0,
        burst_len: 0,
        retransmit_delay: Duration::ZERO,
        gilbert_elliott: None,
        fragmentation: None,
        bandwidth: None,
    };
    let initial_links = vec![(0, 1, delayed.clone()), (1, 0, delayed)];
    Schedule {
        schema_version: SCHEDULE_SCHEMA_VERSION,
        seed: 0,
        link_seed: 0,
        config,
        initial_links,
        events: Vec::new(),
        heal_at: steps,
    }
}

/// Clock-skew consistency over a long run — the floor a real H-SKEW experiment
/// would build on, plus an honest note on why this harness cannot yet run that
/// experiment.
///
/// **What this validly shows:** per-peer clock skew (peer 0 at +0.1% over 10k
/// steps) does not break mesh **state consistency or liveness**. The skew shifts
/// timing-gated behavior (the RTT gauge, quality-report/keepalive cadence), and
/// the mesh still confirms a byte-identical prefix — the peers stay in step. It
/// extends the short +10% `clock_skew_is_tolerated_and_alters_execution`
/// observation to a realistic magnitude over a long run.
///
/// **What it does NOT do — H-SKEW (PLAN.md §13) is NOT executed here.** H-SKEW
/// predicts a *rate* effect: a clock running 0.1% fast drives the frame loop
/// 0.1% faster in wall-clock time, so the fast peer produces 216 more frames/hour
/// than the network confirms and `local_frame_advantage` accumulates. This
/// harness advances **every peer exactly one frame per step** (`harness/mod.rs`,
/// lockstep) and reads the clock only for timestamps, so that accumulation is
/// **structurally absent**: the frame-advantage delta `floor(half_rtt*fps/1000)`
/// stays 1 from 0% through ~+11% skew, so `average_frame_advantage` never reaches
/// `MIN_RECOMMENDATION` at *any* ppm or run length. Asserting "no lag creep / no
/// recommendations" here would be a tautology, not a falsification. **H-SKEW's
/// lag-creep and one-sided-recommendation fears remain OWED**, blocked on a
/// skew-gated frame model (advancing each peer at a rate driven by its own
/// skewed clock).
#[test]
#[ignore = "long-run consistency probe; run manually / in nightly"]
fn clock_skew_holds_consistency_over_a_long_run() {
    let schedule = clock_skew_long_run_schedule(10_000, 1_000); // peer 0 at +0.1%
    let report = run(&schedule, &RunOptions::default());
    report.expect_pass(&schedule); // state consistency + liveness over 10k steps

    // The peers must stay in step: a spread in their confirmed frames would be
    // the *first* sign the skew leaked into frame production (i.e. that a future
    // skew-gated frame model had begun to reproduce the H-SKEW drift). In this
    // lockstep model it is 0; assert it stays within the prediction window.
    let confirmed = &report.final_confirmed;
    let max = confirmed
        .iter()
        .copied()
        .max()
        .expect("final_confirmed is non-empty");
    let min = confirmed
        .iter()
        .copied()
        .min()
        .expect("final_confirmed is non-empty");
    let spread = max - min;
    assert!(
        (spread as usize) <= schedule.config.max_prediction,
        "skew must not drift the peers' confirmed frames apart (spread={spread}, \
         final_confirmed={confirmed:?})"
    );
}

fn skew_gated_schedule(steps: u32, ppm: i32, app_model: AppModel) -> Schedule {
    let mut schedule = clock_skew_long_run_schedule(steps, ppm);
    schedule.config.frame_model = FrameModel::SkewGated60Hz;
    schedule.config.app_model = app_model;
    schedule
}

/// The skew-gated model must turn clock-rate differences into actual frame
/// opportunities while preserving deterministic, byte-consistent simulation.
#[test]
fn skew_gated_frame_model_exercises_rate_drift_deterministically() {
    let exact = skew_gated_schedule(1_200, 0, AppModel::Obey);
    let mut exact = exact;
    exact.config.step_dt_ms = 8;
    let mut skewed = skew_gated_schedule(1_200, 1_000_000, AppModel::Obey);
    skewed.config.step_dt_ms = 8;

    let exact_report = run(&exact, &RunOptions::default());
    exact_report.expect_pass(&exact);
    let skewed_report = run(&skewed, &RunOptions::default());
    skewed_report.expect_pass(&skewed);
    let replay = run(&skewed, &RunOptions::default());

    assert_eq!(exact_report.frame_opportunities, vec![575, 575]);
    assert_eq!(skewed_report.frame_opportunities, vec![1_151, 575]);
    assert_eq!(skewed_report.trace_hash, replay.trace_hash);
    assert_eq!(skewed_report.progress_samples, replay.progress_samples);
    assert!(!skewed_report.progress_samples.is_empty());
    assert!(skewed_report.progress_samples.len() <= 12);
    assert_ne!(exact_report.trace_hash, skewed_report.trace_hash);
}

#[test]
fn schema_v15_skew_progress_preserves_legacy_trace_shape() {
    let mut schedule = skew_gated_schedule(1_200, 1_000, AppModel::Obey);
    schedule.schema_version = 15;
    schedule.config.step_dt_ms = 8;

    let first = run(&schedule, &RunOptions::default());
    let replay = run(&schedule, &RunOptions::default());
    first.expect_pass(&schedule);
    assert_eq!(first.trace_hash, replay.trace_hash);
    assert!(!first.progress_samples.is_empty());
    assert!(first
        .progress_samples
        .iter()
        .all(|sample| sample.endpoints.is_empty() && sample.link_queues.is_empty()));
    let encoded = serde_json::to_string(&first.progress_samples)
        .expect("legacy progress samples serialize deterministically");
    assert!(!encoded.contains("endpoints"));
    assert!(!encoded.contains("link_queues"));
}

/// H-SKEW hour-equivalent experiment: +0.1% produces exactly 216 additional
/// 60 Hz opportunities over one virtual hour. Nightly records both the bounded
/// lag/correction result and the currently-red work-amplification observation.
#[test]
#[ignore = "hour-equivalent H-SKEW experiment; selected by nightly CI"]
#[allow(clippy::print_stdout, clippy::disallowed_macros)]
fn h_skew_hour_equivalent_measures_lag_correction_and_cost() {
    let mut exact = skew_gated_schedule(240_001, 0, AppModel::Obey);
    exact.config.step_dt_ms = 15;
    let mut skewed = skew_gated_schedule(240_001, 1_000, AppModel::Obey);
    skewed.config.step_dt_ms = 15;

    let exact_report = run(&exact, &RunOptions::default());
    exact_report.expect_pass(&exact);
    let skewed_report = run(&skewed, &RunOptions::default());
    skewed_report.expect_pass(&skewed);
    let replay = run(&skewed, &RunOptions::default());

    assert_eq!(exact_report.frame_opportunities, vec![216_000, 216_000]);
    assert_eq!(skewed_report.frame_opportunities, vec![216_216, 216_000]);
    assert_eq!(skewed_report.trace_hash, replay.trace_hash);
    assert_eq!(skewed_report.progress_samples, replay.progress_samples);
    let exact_total_work = exact_report
        .metrics
        .iter()
        .map(|metrics| metrics.frames_advanced)
        .fold(0_u64, u64::saturating_add);
    let skewed_total_work = skewed_report
        .metrics
        .iter()
        .map(|metrics| metrics.frames_advanced)
        .fold(0_u64, u64::saturating_add);
    let exact_resimulation = exact_report
        .metrics
        .iter()
        .map(|metrics| metrics.resimulated_frames)
        .fold(0_u64, u64::saturating_add);
    let skewed_resimulation = skewed_report
        .metrics
        .iter()
        .map(|metrics| metrics.resimulated_frames)
        .fold(0_u64, u64::saturating_add);
    println!(
        "H-SKEW exact_opportunities={:?} exact_metrics={:?} \
         skewed_opportunities={:?} wait_frames_obeyed={:?} \
         skewed_metrics={:?} exact_total_work={} skewed_total_work={} \
         exact_resimulation={} skewed_resimulation={} samples={:?}",
        exact_report.frame_opportunities,
        exact_report.metrics,
        skewed_report.frame_opportunities,
        skewed_report.wait_frames_obeyed,
        skewed_report.metrics,
        exact_total_work,
        skewed_total_work,
        exact_resimulation,
        skewed_resimulation,
        skewed_report.progress_samples
    );
    assert!(
        skewed_total_work.saturating_mul(100) <= exact_total_work.saturating_mul(120),
        "H-SKEW total work amplification must not exceed 20%: \
         exact={exact_total_work}, skewed={skewed_total_work}"
    );
    assert!(
        skewed_resimulation.saturating_mul(100) <= exact_resimulation.saturating_mul(140),
        "H-SKEW resimulation amplification must not exceed 40%: \
         exact={exact_resimulation}, skewed={skewed_resimulation}"
    );
    assert!(
        skewed_total_work.saturating_mul(100) >= exact_total_work.saturating_mul(110),
        "the open H-SKEW-COST finding must remain directly observable until a \
         reviewed controller fix deliberately flips this assertion: \
         exact={exact_total_work}, skewed={skewed_total_work}"
    );
    assert!(
        skewed_resimulation.saturating_mul(100) >= exact_resimulation.saturating_mul(120),
        "the open H-SKEW-COST resimulation finding must remain directly observable until a \
         reviewed controller fix deliberately flips this assertion: \
         exact={exact_resimulation}, skewed={skewed_resimulation}"
    );

    assert_eq!(skewed_report.wait_frames_obeyed[1], 0);
    assert!(
        (200..=230).contains(&skewed_report.wait_frames_obeyed[0]),
        "the correction duty should track the injected 216-frame/hour drift \
         without escalating: {:?}",
        skewed_report.wait_frames_obeyed
    );
    assert_eq!(skewed_report.metrics[1].wait_recommendations, 0);
    assert!(
        skewed_report.metrics[0].wait_recommendations <= 120,
        "the fast peer should correct at most twice per minute, not saturate \
         the one-second recommendation cooldown: {:?}",
        skewed_report.metrics
    );
    assert!(
        skewed_report
            .metrics
            .iter()
            .all(|metric| metric.stall_count == 0),
        "clock correction must not exhaust the prediction window: {:?}",
        skewed_report.metrics
    );

    let final_sample = skewed_report
        .progress_samples
        .last()
        .expect("the skew-gated run records bounded progress samples");
    assert!(
        final_sample
            .confirmation_lag
            .iter()
            .all(|&lag| lag <= skewed.config.max_prediction as u64),
        "steady-state confirmation lag must remain inside max_prediction: {:?}",
        final_sample.confirmation_lag
    );
    let steady_samples = &skewed_report.progress_samples[6..];
    for peer in 0..2 {
        let min = steady_samples
            .iter()
            .map(|sample| sample.confirmation_lag[peer])
            .min()
            .expect("steady-state sample set is non-empty");
        let max = steady_samples
            .iter()
            .map(|sample| sample.confirmation_lag[peer])
            .max()
            .expect("steady-state sample set is non-empty");
        assert!(
            max.saturating_sub(min) <= 1,
            "steady-state lag must stay flat rather than creep (peer={peer}, \
             samples={steady_samples:?})"
        );
    }
}

/// H-SKEW cost-amplification diagnostic: repeat a ten-minute-equivalent matched
/// exact/skew experiment at several scheduler resolutions. A real clock-drift
/// cost should remain comparable as the deterministic outer-loop cadence gets
/// finer; a large cadence-dependent swing instead identifies sampling/phase
/// aliasing in the harness as a confounder.
#[test]
#[ignore = "H-SKEW scheduler-resolution cost matrix; run manually"]
#[allow(clippy::print_stdout, clippy::disallowed_macros)]
fn h_skew_cost_amplification_across_scheduler_resolutions() {
    const DURATION_MS: u32 = 600_000;

    for step_dt_ms in [5_u32, 8, 10, 15] {
        let steps = DURATION_MS / step_dt_ms + 1;
        let mut exact = skew_gated_schedule(steps, 0, AppModel::Obey);
        exact.config.step_dt_ms = u64::from(step_dt_ms);
        let exact_report = run(&exact, &RunOptions::default());
        exact_report.expect_pass(&exact);

        let total_work = |report: &RunReport| {
            report
                .metrics
                .iter()
                .map(|metrics| metrics.frames_advanced)
                .fold(0_u64, u64::saturating_add)
        };
        let resimulation = |report: &RunReport| {
            report
                .metrics
                .iter()
                .map(|metrics| metrics.resimulated_frames)
                .fold(0_u64, u64::saturating_add)
        };
        assert_eq!(exact_report.frame_opportunities, vec![36_000, 36_000]);
        let exact_work = total_work(&exact_report);
        let exact_resimulation = resimulation(&exact_report);
        let mut work_by_orientation = [0_u64; 2];
        let mut resimulation_by_orientation = [0_u64; 2];

        for fast_peer in 0..2 {
            let mut skewed = skew_gated_schedule(steps, 0, AppModel::Obey);
            skewed.config.step_dt_ms = u64::from(step_dt_ms);
            skewed.config.clock_skew_ppm = vec![0, 0];
            skewed.config.clock_skew_ppm[fast_peer] = 1_000;
            let skewed_report = run(&skewed, &RunOptions::default());
            skewed_report.expect_pass(&skewed);

            let skewed_work = total_work(&skewed_report);
            let skewed_resimulation = resimulation(&skewed_report);
            work_by_orientation[fast_peer] = skewed_work;
            resimulation_by_orientation[fast_peer] = skewed_resimulation;
            println!(
                "H-SKEW-COST step_dt_ms={step_dt_ms} fast_peer={fast_peer} \
                 exact_opportunities={:?} skewed_opportunities={:?} \
                 exact_work={exact_work} skewed_work={skewed_work} \
                 exact_resimulation={exact_resimulation} \
                 skewed_resimulation={skewed_resimulation} skips={:?}",
                exact_report.frame_opportunities,
                skewed_report.frame_opportunities,
                skewed_report.wait_frames_obeyed
            );

            let mut expected_opportunities = vec![36_000, 36_000];
            expected_opportunities[fast_peer] = 36_036;
            assert_eq!(skewed_report.frame_opportunities, expected_opportunities);
            assert!(
                skewed_work.saturating_mul(100) >= exact_work.saturating_mul(108),
                "the measured total-work excess must persist at {step_dt_ms} ms: \
                 exact={exact_work}, skewed={skewed_work}"
            );
            assert!(
                skewed_resimulation.saturating_mul(100) >= exact_resimulation.saturating_mul(115),
                "the measured resimulation excess must persist at {step_dt_ms} ms: \
                 exact={exact_resimulation}, skewed={skewed_resimulation}"
            );
            for peer in 0..2 {
                if peer == fast_peer {
                    assert!(
                        (30..=42).contains(&skewed_report.wait_frames_obeyed[peer]),
                        "correction duty must track the 36-frame drift at {step_dt_ms} ms: {:?}",
                        skewed_report.wait_frames_obeyed
                    );
                } else {
                    assert_eq!(skewed_report.wait_frames_obeyed[peer], 0);
                }
            }
        }
        assert_eq!(
            work_by_orientation[0], work_by_orientation[1],
            "fixed peer drive order must not explain H-SKEW total-work cost at {step_dt_ms} ms"
        );
        assert_eq!(
            resimulation_by_orientation[0], resimulation_by_orientation[1],
            "fixed peer drive order must not explain H-SKEW resimulation cost at {step_dt_ms} ms"
        );
    }
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

/// §6.6-pre.6h — the event-loss oracle. The harness normally drains every
/// peer's event queue each step, so the D9 event-discard telemetry
/// (`events_discarded_*`) never fires in the fleet. A *starved* peer — one whose
/// app model never services [`P2PSession::events`] — lets the session's bounded
/// queue fill until it trims the oldest events, exercising that overflow path
/// end-to-end.
///
/// Fire: on a Clean N=8 mesh with a small (10-slot) queue, the starved peer's
/// cold-start sync burst (`Synchronizing`, one per remote per retry) overflows
/// the queue (D9 > 0) while every draining peer stays clean. Neutralize: the
/// identical schedule with every peer draining discards nothing — proving
/// starvation, not the mesh itself, is the cause. Both runs pass the full
/// oracle: not servicing events must not desync or stall the mesh.
///
/// A Clean mesh is deliberate — the discards come from the initial sync
/// handshake (which every mesh runs regardless of noise), so the count is
/// deterministic and N-scaled rather than dependent on random faults.
///
/// **N and the cap are load-bearing together.** A *draining* peer empties its
/// queue every step, so its high-water mark is the largest single-poll burst:
/// ≈`N − 1` `Synchronizing` events (one per remote) during cold start. That
/// must stay strictly under the cap or a draining peer overflows too and the
/// "draining peers discard nothing" assertion fails. At N=8 the burst is 7 < 10
/// (margin 3, and the starved peer still accumulates ≈25 discards across polls);
/// the relation breaks at N≥12 (burst ≥ 11 > 10). If you bump N, raise the cap
/// past `N − 1` in step — the sub-assertion fails loudly if you forget, so this
/// stays honest, but keep them paired.
fn event_starvation_config(starve: bool) -> SimConfig {
    let n = 8;
    SimConfig {
        n_players: n,
        steps: 900,
        noise: BackgroundNoise::Clean,
        event_queue_size: Some(10),
        starve_events: if starve { vec![0] } else { Vec::new() },
        ..SimConfig::smoke(n)
    }
}

#[test]
fn event_starvation_overflows_the_bounded_event_queue() {
    let seed = 0xE7A2;
    let n = event_starvation_config(true).n_players;

    // Fire: peer 0 never drains its 10-slot queue; the session trims the
    // overflow and D9 fires — for peer 0 only.
    let starved_schedule = generate(seed, event_starvation_config(true));
    let starved = run(&starved_schedule, &RunOptions::default());
    // Starvation must not break the mesh (events are informational): the full
    // oracle still passes even though peer 0 ignores its event queue.
    starved.expect_pass(&starved_schedule);
    assert!(
        starved.metrics[0].events_discarded_total > 0,
        "starved peer 0 must overflow its bounded event queue (D9 telemetry); \
         got discards={} — too few events, raise N or lower the cap",
        starved.metrics[0].events_discarded_total
    );
    // Draining peers must NOT overflow: each empties its queue every step, so
    // its high-water is one cold-start poll burst (≈N-1 < cap at N=8). This is
    // load-bearing on N — see `event_starvation_config` (it fails loudly, not
    // silently, if a future N-bump pushes the burst past the cap).
    for i in 1..n {
        assert_eq!(
            starved.metrics[i].events_discarded_total, 0,
            "a draining peer must not overflow (single-poll burst < cap): \
             peer {i} discarded {}",
            starved.metrics[i].events_discarded_total
        );
    }

    // Neutralize: identical schedule, every peer draining → nobody overflows,
    // isolating starvation as the sole cause of D9.
    let drained_schedule = generate(seed, event_starvation_config(false));
    let drained = run(&drained_schedule, &RunOptions::default());
    drained.expect_pass(&drained_schedule);
    for i in 0..n {
        assert_eq!(
            drained.metrics[i].events_discarded_total, 0,
            "with every peer draining, no queue may overflow — starvation is the \
             only cause of D9: peer {i} discarded {}",
            drained.metrics[i].events_discarded_total
        );
    }
}

/// A `starve_events` entry naming a peer outside the mesh (a hand-edited or
/// corrupt corpus artifact) must fail loudly up front, not silently no-op —
/// the same fail-loud contract the event-index validation enforces.
#[test]
#[should_panic(expected = "out of range")]
fn run_rejects_out_of_range_starve_events_peer() {
    let config = SimConfig {
        n_players: 2,
        starve_events: vec![9],
        ..SimConfig::smoke(2)
    };
    let _ = run(&generate(0, config), &RunOptions::default());
}

/// An `event_queue_size` below the library minimum (10) must be rejected up
/// front with a clear message, mirroring `SessionBuilder::with_event_queue_size`,
/// rather than surfacing as a builder error deep in session construction.
#[test]
#[should_panic(expected = "event_queue_size must be >= 10")]
fn run_rejects_too_small_event_queue_size() {
    let config = SimConfig {
        n_players: 2,
        event_queue_size: Some(5),
        ..SimConfig::smoke(2)
    };
    let _ = run(&generate(0, config), &RunOptions::default());
}

/// A `step_dt_ms` of 0 never advances virtual time and makes the derived (c)
/// recovery window (`RECOVERY_WINDOW_MS / step_dt_ms`) meaningless — the runner
/// must reject it up front rather than let `recovery_window_steps()`'s
/// div-by-zero `.max(1)` guard silently paper over a broken config.
#[test]
#[should_panic(expected = "step_dt_ms must be within 1..=1000")]
fn run_rejects_zero_step_dt() {
    let config = SimConfig {
        n_players: 2,
        step_dt_ms: 0,
        ..SimConfig::smoke(2)
    };
    let _ = run(&generate(0, config), &RunOptions::default());
}

/// Heal step + length for the (c) bounded post-heal liveness tests. The recovery
/// window B (`recovery_window_steps`, 250 at 16ms/step) plus slack must fit
/// before `STEPS`, so the recovery anchor (`heal_at + B = 650`) is a genuine
/// mid-drain sample rather than an end-of-run clamp.
const HEAL_LIVENESS_HEAL_AT: u32 = 400;
const HEAL_LIVENESS_STEPS: u32 = 700;

/// Builds a schedule for the (c) bounded post-heal liveness checks: a clean
/// 4-mesh (`ContinueWithout`) that optionally emits a `HealAll` at
/// `HEAL_LIVENESS_HEAL_AT` and optionally freezes peer 1 for `stall` steps from
/// that same step. `heal = false` emits no `HealAll` (heal_at = steps) — the
/// "never heals" control, where (c) must stay inert. A short `stall` hitches
/// peer 1 but it catches up within the window (passes (c)); a `stall` spanning
/// the whole window pins it (fails (c)). The two differ only in stall length, so
/// the pair is a clean red-green premise.
fn heal_liveness_schedule(stall: Option<u32>, heal: bool) -> Schedule {
    let n = 4;
    let config = SimConfig {
        n_players: n,
        steps: HEAL_LIVENESS_STEPS,
        noise: BackgroundNoise::Clean,
        // ContinueWithout so the three survivors drop a long-pinned peer and
        // keep confirming — the (c) failure then isolates to the pinned peer,
        // rather than Halt taking the whole mesh down with it.
        disconnect_behavior: DropPolicy::ContinueWithout,
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
    if let Some(steps) = stall {
        events.push((
            HEAL_LIVENESS_HEAL_AT,
            ScheduleEvent::PeerStall { peer: 1, steps },
        ));
    }
    let heal_at = if heal {
        events.push((HEAL_LIVENESS_HEAL_AT, ScheduleEvent::HealAll));
        HEAL_LIVENESS_HEAL_AT
    } else {
        HEAL_LIVENESS_STEPS
    };
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

/// (c) POSITIVE: peer 1 hitches for 60 steps at the heal — well under the
/// recovery window B — and the mesh catches up, so every live peer advances ≥ G
/// and (c) passes with `recovered_within_b == Some(true)`. Also proves (c)
/// actually ran (anchors sampled per peer, each clears the floor) — not silently
/// inert.
#[test]
fn post_heal_liveness_passes_when_a_hitched_peer_recovers() {
    let schedule = heal_liveness_schedule(Some(60), true);
    let report = run(&schedule, &RunOptions::default());
    report.expect_pass(&schedule);
    assert_eq!(
        report.recovered_within_b,
        Some(true),
        "(c) must run and pass on a mesh that recovers: at_heal={:?} after={:?}",
        report.confirmed_at_heal,
        report.confirmed_after_recovery
    );
    assert_eq!(
        report.confirmed_at_heal.len(),
        4,
        "one heal anchor per peer"
    );
    assert_eq!(
        report.confirmed_after_recovery.len(),
        4,
        "one recovery anchor per peer"
    );
    for peer in 0..4 {
        let advanced = report.confirmed_after_recovery[peer] - report.confirmed_at_heal[peer];
        assert!(
            advanced >= POST_HEAL_MIN_ADVANCE,
            "peer {peer} must clear the G floor post-heal: advanced={advanced}"
        );
    }
}

/// (c) NEGATIVE CONTROL 1: a schedule that never emits `HealAll` must leave (c)
/// inert — `recovered_within_b == None`, no anchors sampled, no `PostHealLiveness`
/// failure. Pins that `heal_at` alone (without a `HealAll` event) is NOT treated
/// as a heal.
#[test]
fn post_heal_liveness_is_inert_without_a_heal() {
    let schedule = heal_liveness_schedule(None, false);
    let report = run(&schedule, &RunOptions::default());
    report.expect_pass(&schedule);
    assert_eq!(
        report.recovered_within_b, None,
        "(c) must be inert when the schedule never heals"
    );
    assert!(
        report.confirmed_at_heal.is_empty() && report.confirmed_after_recovery.is_empty(),
        "no HealAll ⇒ no heal anchors: at_heal={:?} after={:?}",
        report.confirmed_at_heal,
        report.confirmed_after_recovery
    );
    assert!(
        !report
            .verdict
            .failures
            .iter()
            .any(|f| matches!(f, OracleFailure::PostHealLiveness { .. })),
        "(c) must not fire without a heal: {:?}",
        report.verdict.failures
    );
}

/// (c) NEGATIVE CONTROL 3: when `step_dt_ms` is coarse enough that the recovery
/// window B is narrower than the G-frame floor (here 500ms/step ⇒ B=8 steps < G=10),
/// a healthy peer confirming ~1 frame/step physically cannot advance G frames in
/// the window. (c) must degrade to indeterminate (`None`) rather than charge a
/// false `PostHealLiveness` against every healthy peer — the run still passes.
#[test]
fn post_heal_liveness_is_indeterminate_when_window_narrower_than_g() {
    let mut schedule = heal_liveness_schedule(None, true);
    schedule.config.step_dt_ms = 500;
    assert!(
        schedule.config.recovery_window_steps() < u32::try_from(POST_HEAL_MIN_ADVANCE).unwrap(),
        "premise: this step_dt must make B narrower than G"
    );
    let report = run(&schedule, &RunOptions::default());
    report.expect_pass(&schedule);
    assert_eq!(
        report.recovered_within_b, None,
        "(c) must be indeterminate when the window is narrower than G, not a false failure"
    );
    assert!(
        report.confirmed_at_heal.is_empty() && report.confirmed_after_recovery.is_empty(),
        "an indeterminate (c) samples no anchors: at_heal={:?} after={:?}",
        report.confirmed_at_heal,
        report.confirmed_after_recovery
    );
    assert!(
        !report
            .verdict
            .failures
            .iter()
            .any(|f| matches!(f, OracleFailure::PostHealLiveness { .. })),
        "(c) must not fire a false failure on a too-narrow window: {:?}",
        report.verdict.failures
    );
}

/// (c) NEGATIVE CONTROL 2: the same mesh, but peer 1 is frozen across the ENTIRE
/// recovery window (B + 10 steps) — it cannot advance, so (c) must fire on it
/// (and only it) while the other three recover. The inverse is the positive test
/// above (identical schedule minus the long freeze), so the pin is provably what
/// flips (c) — the premise has teeth.
#[test]
fn post_heal_liveness_fires_on_a_pinned_peer() {
    let b = SimConfig::smoke(4).recovery_window_steps();
    let schedule = heal_liveness_schedule(Some(b + 10), true);
    let report = run(&schedule, &RunOptions::default());
    assert!(!report.verdict.passed(), "a pinned peer must fail the run");
    assert_eq!(
        report.recovered_within_b,
        Some(false),
        "(i) must report non-recovery when a peer is pinned"
    );
    let pinned = report
        .verdict
        .failures
        .iter()
        .filter(|f| matches!(f, OracleFailure::PostHealLiveness { peer: 1, .. }))
        .count();
    assert_eq!(
        pinned, 1,
        "(c) must fire on the pinned peer 1: {:?}",
        report.verdict.failures
    );
    assert!(
        !report.verdict.failures.iter().any(|f| matches!(f,
            OracleFailure::PostHealLiveness { peer, .. } if *peer != 1)),
        "only the pinned peer may fail (c): {:?}",
        report.verdict.failures
    );
}

/// Overlapping `PeerStall` events are cumulative in effect: a later short stall
/// during a long stall must not shorten the first stall's deadline. This pins the
/// runner to `max(existing_deadline, new_deadline)` semantics; otherwise the
/// second event below lets peer 1 resume immediately and falsely clears (c).
#[test]
fn overlapping_peer_stalls_keep_the_later_deadline() {
    let b = SimConfig::smoke(4).recovery_window_steps();
    let mut schedule = heal_liveness_schedule(None, true);
    schedule.events.push((
        HEAL_LIVENESS_HEAL_AT - 1,
        ScheduleEvent::PeerStall {
            peer: 1,
            steps: b + 11,
        },
    ));
    schedule.events.push((
        HEAL_LIVENESS_HEAL_AT,
        ScheduleEvent::PeerStall { peer: 1, steps: 1 },
    ));
    schedule.events.sort_by_key(|(step, _)| *step);

    let report = run(&schedule, &RunOptions::default());
    assert!(
        report
            .verdict
            .failures
            .iter()
            .any(|f| matches!(f, OracleFailure::PostHealLiveness { peer: 1, .. })),
        "overlapping short stall must not shorten the long pinned stall: {:?}",
        report.verdict.failures
    );
    assert_eq!(
        report.recovered_within_b,
        Some(false),
        "the long stall must remain pinned across the recovery window"
    );
}

/// (c)/(i) HALT INVERSE: a symmetric partition healed under `Halt` never recovers
/// (all peers halt in `Synchronizing`), so (c) legitimately reports
/// `Some(false)` — healed network, metastable mesh. This is a TRUE non-recovery
/// (the headline metastability case), not a false positive; (c) discriminates
/// recovered-vs-not, it does not merely pass.
#[test]
fn post_heal_liveness_reports_non_recovery_under_halt() {
    let schedule = split_brain_schedule(DropPolicy::Halt);
    let report = run(&schedule, &RunOptions::default());
    assert!(!report.verdict.passed());
    assert_eq!(
        report.recovered_within_b,
        Some(false),
        "a Halt-halted mesh healed but never recovered — (i) must say so"
    );
}
