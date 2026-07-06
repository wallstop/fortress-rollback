//! Simulation fleet: seeded whole-mesh runs checked by the global oracle.
//!
//! The PR smoke fleet runs a fixed seed set per mesh size on every PR. The
//! meta-determinism and negative-control tests validate the harness itself:
//! a harness whose invariants cannot fire, or whose runs are not reproducible,
//! proves nothing.

use super::harness::schedule::{
    generate, AppModel, BackgroundNoise, DropPolicy, Schedule, ScheduleEvent, SimConfig,
    SCHEDULE_SCHEMA_VERSION,
};
use super::harness::{
    oracle::{OracleFailure, POST_HEAL_MIN_ADVANCE},
    run, RunOptions,
};
use crate::common::sim_net::LinkPolicy;
use fortress_rollback::SessionState;
use std::{collections::BTreeSet, time::Duration};

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

/// Builds a legacy-disconnect schedule: a clean 4-mesh in which peer `by`
/// calls the older `disconnect_player(target)` API at step 100. On success the
/// target leaves the harness; on error it stays live and the oracle records the
/// failed API call. This is intentionally not a graceful-convergence contract:
/// today's `disconnect_player` path is Halt-oriented and intersects D13, so the
/// useful property is that the op is executable, deterministic, and reported as
/// the expected post-heal non-recovery. `None` yields the identical schedule
/// without the disconnect, the control the premise compares to.
fn legacy_disconnect_schedule(disconnect: Option<(usize, usize)>) -> Schedule {
    clean_four_peer_lifecycle_schedule(
        DropPolicy::Halt,
        disconnect.map(|(by, target)| ScheduleEvent::LegacyDisconnect { by, target }),
    )
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
/// the current D13-adjacent observation: after a clean network heal, the live
/// peers do not recover within B and the oracle reports that non-recovery.
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
/// 0.1% faster in wall-clock time, so the fast peer produces ~43 more frames/hour
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
#[should_panic(expected = "step_dt_ms must be >= 1")]
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
