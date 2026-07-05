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
/// raises its local input delay from the smoke default (1) to `delay` frames at
/// step 100 — a mid-session *increase*, which gap-fills the newly delayed
/// frames with replicated confirmed inputs and flushes them to every remote (a
/// reconfiguration path a fixed-delay fleet never exercises). `None` yields the
/// identical schedule without the change, the control the premise compares to.
fn input_delay_change_schedule(n: usize, delay: Option<usize>) -> Schedule {
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
    if let Some(delay) = delay {
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
        let changed = input_delay_change_schedule(n, Some(3));

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
    let schedule = input_delay_change_schedule(4, Some(3));
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
