//! M2 §5.3 baseline sweep: deterministic, virtual-time bandwidth/rollback
//! measurement over a controlled loss × RTT × jitter grid.
//!
//! Unlike the fleet (which layers randomized storyline faults to *find* bugs),
//! the sweep holds each cell's link conditions constant to *measure* steady-state
//! cost: per-player wire bandwidth, rollback rate/depth, confirmation lag, and
//! pacing pressure (stalls / wait-recommendations). Every cell is a thin wrapper
//! over the mesh runner ([`run`]) with a uniform per-link [`LinkPolicy`], so the
//! output is a pure function of `(seed, cell params)` — zero runner noise.
//!
//! One [`CellReport`] is produced per cell; the PR gate ([`sweep_pr_gate`])
//! checks a handful of representative cells for the load-bearing invariant
//! (`desync_incidents == 0`), liveness, and non-zero bandwidth, and that a cell
//! is bit-for-bit reproducible. `FORTRESS_SWEEP_OUT`, if set, receives the
//! reports as JSON Lines for offline analysis.
//!
use super::harness::schedule::{Schedule, SimConfig, SCHEDULE_SCHEMA_VERSION};
use super::harness::{
    run_with_input, PeerWireTotals, RunOptions, RunReport, SimInput, WideStubInput,
};
use crate::common::sim_net::LinkPolicy;
use crate::common::stubs::StubInput;
use fortress_rollback::{MessageKind, RollbackDepthHistogram, SessionMetrics};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::Duration;

/// Per-player serialized input widths covered by the sweep.
const INPUT_WIDTHS_BYTES: [u32; 2] = [StubInput::WIDTH_BYTES, WideStubInput::WIDTH_BYTES];

/// One sweep cell: a mesh size under constant link conditions.
#[derive(Clone, Copy, Debug)]
pub struct CellParams {
    pub label: &'static str,
    pub n_players: usize,
    pub loss_pct: f64,
    /// Round-trip time; split evenly as `rtt_ms / 2` one-way delay per direction
    /// (integer division — an odd `rtt_ms` truncates by 1ms one-way).
    pub rtt_ms: u64,
    pub jitter_ms: u64,
    /// Serialized byte width of one player input under the harness input type.
    pub input_width_bytes: u32,
    pub steps: u32,
    pub seed: u64,
}

impl CellParams {
    /// One-way link delay (half the RTT).
    fn one_way_delay(self) -> Duration {
        Duration::from_millis(self.rtt_ms / 2)
    }

    /// The uniform per-directed-link policy for this cell's constant conditions.
    fn link_policy(self) -> LinkPolicy {
        LinkPolicy {
            drop_rate: self.loss_pct / 100.0,
            dup_rate: 0.0,
            base_delay: self.one_way_delay(),
            jitter: Duration::from_millis(self.jitter_ms),
            burst_rate: 0.0,
            burst_len: 0,
            retransmit_delay: Duration::ZERO,
            gilbert_elliott: None,
            fragmentation: None,
            bandwidth: None,
        }
    }

    /// A materialized [`Schedule`] with constant conditions: the cell's uniform
    /// policy on every directed link and an empty event list (no fault events,
    /// no `HealAll`). The `heal_at` field is present in the struct but inert —
    /// it has no effect without a corresponding `HealAll` in `events`.
    fn schedule(self) -> Schedule {
        let config = SimConfig {
            n_players: self.n_players,
            steps: self.steps,
            ..SimConfig::smoke(self.n_players)
        };
        let policy = self.link_policy();
        let mut initial_links = Vec::new();
        for from in 0..self.n_players {
            for to in 0..self.n_players {
                if from != to {
                    initial_links.push((from, to, policy.clone()));
                }
            }
        }
        Schedule {
            schema_version: SCHEDULE_SCHEMA_VERSION,
            seed: self.seed,
            // Domain-separated from `seed` so link-noise rolls are an
            // independent stream (same convention as `schedule::generate()`).
            link_seed: self.seed ^ 0x1111_2222_3333_4444,
            config,
            initial_links,
            // No storyline events; heal_at is inert (no HealAll in `events`).
            events: Vec::new(),
            heal_at: self.steps,
        }
    }
}

fn cells_with_widths(base: impl IntoIterator<Item = CellParams>) -> Vec<CellParams> {
    let mut cells = Vec::new();
    for cell in base {
        for input_width_bytes in INPUT_WIDTHS_BYTES {
            cells.push(CellParams {
                input_width_bytes,
                ..cell
            });
        }
    }
    cells
}

/// The measured, serializable result for one cell. All rate columns are derived
/// from cumulative counters over the cell's virtual duration, so the value is a
/// deterministic function of `(seed, params)`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CellReport {
    pub schema: u32,
    pub label: String,
    pub version: String,
    pub git_sha: String,
    pub n_players: usize,
    pub loss_pct: f64,
    pub rtt_ms: u64,
    pub jitter_ms: u64,
    pub input_width_bytes: u32,
    pub steps: u32,
    pub seed: u64,
    /// Lowest final confirmed frame across peers (liveness floor).
    pub min_final_confirmed: i32,
    /// Mean per-player payload bytes sent per virtual second (pre-header). Each
    /// player's figure is summed across its `n_players - 1` remote links, so this
    /// grows ~linearly with mesh size — it is per-player, not per-link.
    pub bytes_sent_per_player_per_sec: f64,
    /// Mean per-player pre/post-compression input bytes per virtual second.
    pub input_bytes_pre_compression_per_player_per_sec: f64,
    pub input_bytes_post_compression_per_player_per_sec: f64,
    /// Mesh-total sent messages by kind (snake_case label → count).
    pub messages_sent_by_kind: BTreeMap<String, u64>,
    pub rollbacks_per_100_frames: f64,
    pub rollback_depth_p50: u32,
    pub rollback_depth_p99: u32,
    pub rollback_depth_max: u32,
    pub confirmation_lag_mean: f64,
    pub confirmation_lag_max: u64,
    pub stalls_per_min: f64,
    pub wait_recommendations: u64,
    /// Confirmed-frame checksum mismatches — the load-bearing invariant. **Any
    /// nonzero value is a real desync and fails the sweep.**
    pub desync_incidents: u64,
}

/// Sums one bucket-aligned rollback-depth histogram across peers.
fn merge_depth_histogram(metrics: &[SessionMetrics]) -> [u64; RollbackDepthHistogram::BUCKETS] {
    let mut merged = [0u64; RollbackDepthHistogram::BUCKETS];
    for m in metrics {
        for (slot, bucket) in merged.iter_mut().enumerate() {
            *bucket = bucket.saturating_add(m.rollback_depth_histogram.bucket(slot));
        }
    }
    merged
}

/// The `p`-quantile (`p` in `(0, 1]`) rollback depth from a merged histogram.
/// Bucket `i` covers depth `i + 1`; the final bucket (index 16) is reported as
/// depth 17 (the ">16" catch-all). Returns 0 when no rollbacks were recorded.
fn depth_percentile(hist: &[u64; RollbackDepthHistogram::BUCKETS], p: f64) -> u32 {
    let total: u64 = hist.iter().copied().fold(0u64, u64::saturating_add);
    if total == 0 {
        return 0;
    }
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    let threshold = (p * total as f64).ceil() as u64;
    let mut cumulative = 0u64;
    for (i, &count) in hist.iter().enumerate() {
        cumulative = cumulative.saturating_add(count);
        if cumulative >= threshold {
            return u32::try_from(i + 1).unwrap_or(u32::MAX);
        }
    }
    u32::try_from(RollbackDepthHistogram::BUCKETS).unwrap_or(u32::MAX)
}

/// Mesh-total sent-by-kind counts, keyed by the message-kind label.
fn messages_sent_by_kind(peer_wire: &[PeerWireTotals]) -> BTreeMap<String, u64> {
    let mut map = BTreeMap::new();
    for kind in MessageKind::ALL {
        let total = peer_wire
            .iter()
            .map(|w| w.sent_by_kind(kind))
            .fold(0u64, u64::saturating_add);
        map.insert(kind.as_str().to_owned(), total);
    }
    map
}

/// A cumulative `total` (summed over peers) as a mean per-player per-second
/// rate. `per_players` is always `>= 1` — the harness builds real sessions via
/// `SessionBuilder::with_num_players`, which rejects zero players at
/// construction — so only the virtual duration needs a divide-by-zero guard
/// (reachable via a degenerate `steps == 0` cell).
#[allow(clippy::cast_precision_loss)]
fn sum_to_rate_per_sec(total: u64, per_players: usize, virtual_secs: f64) -> f64 {
    if virtual_secs <= 0.0 {
        return 0.0;
    }
    (total as f64) / (per_players as f64) / virtual_secs
}

/// Runs one cell to completion and folds its per-peer metrics into a
/// [`CellReport`]. Deterministic: identical `(seed, params)` ⇒ identical report.
#[must_use]
pub fn run_cell(params: CellParams) -> CellReport {
    match params.input_width_bytes {
        StubInput::WIDTH_BYTES => run_cell_with_input::<StubInput>(params),
        WideStubInput::WIDTH_BYTES => run_cell_with_input::<WideStubInput>(params),
        other => panic!("unsupported sweep input_width_bytes {other}; expected 4 or 32"),
    }
}

fn run_cell_with_input<I: SimInput>(params: CellParams) -> CellReport {
    assert_eq!(
        params.input_width_bytes,
        I::WIDTH_BYTES,
        "cell width must match the selected harness input type"
    );
    let schedule = params.schedule();
    let report: RunReport = run_with_input::<I>(&schedule, &RunOptions::default());

    #[allow(clippy::cast_precision_loss)]
    let virtual_secs = f64::from(params.steps) * (schedule.config.step_dt_ms as f64) / 1000.0;
    let virtual_minutes = virtual_secs / 60.0;

    let metrics = &report.metrics;
    let n = params.n_players;

    let total_bytes_sent: u64 = report
        .peer_wire
        .iter()
        .map(|w| w.bytes_sent)
        .fold(0u64, u64::saturating_add);
    let total_input_pre: u64 = report
        .peer_wire
        .iter()
        .map(|w| w.input_bytes_pre_compression)
        .fold(0u64, u64::saturating_add);
    let total_input_post: u64 = report
        .peer_wire
        .iter()
        .map(|w| w.input_bytes_post_compression)
        .fold(0u64, u64::saturating_add);

    let total_rollbacks: u64 = metrics
        .iter()
        .map(|m| m.rollback_count)
        .fold(0u64, u64::saturating_add);
    let total_visual: u64 = metrics
        .iter()
        .map(|m| m.visual_frames)
        .fold(0u64, u64::saturating_add);
    let total_lag_sum: u64 = metrics
        .iter()
        .map(|m| m.confirmation_lag_sum)
        .fold(0u64, u64::saturating_add);
    let total_stalls: u64 = metrics
        .iter()
        .map(|m| m.stall_count)
        .fold(0u64, u64::saturating_add);
    let total_wait_recs: u64 = metrics
        .iter()
        .map(|m| m.wait_recommendations)
        .fold(0u64, u64::saturating_add);
    let desync_incidents: u64 = metrics
        .iter()
        .map(|m| m.checksums_mismatched)
        .fold(0u64, u64::saturating_add);
    let confirmation_lag_max: u64 = metrics
        .iter()
        .map(|m| m.confirmation_lag_max)
        .max()
        .unwrap_or(0);
    let rollback_depth_max: u32 = metrics
        .iter()
        .map(|m| m.max_rollback_depth)
        .max()
        .unwrap_or(0);

    let hist = merge_depth_histogram(metrics);

    #[allow(clippy::cast_precision_loss)]
    let rollbacks_per_100_frames = if total_visual == 0 {
        0.0
    } else {
        100.0 * (total_rollbacks as f64) / (total_visual as f64)
    };
    #[allow(clippy::cast_precision_loss)]
    let confirmation_lag_mean = if total_visual == 0 {
        0.0
    } else {
        (total_lag_sum as f64) / (total_visual as f64)
    };
    // As in `sum_to_rate_per_sec`, `n >= 1` is guaranteed upstream (the harness
    // rejects zero players at session construction), so only the virtual
    // duration needs a divide-by-zero guard.
    #[allow(clippy::cast_precision_loss)]
    let stalls_per_min = if virtual_minutes <= 0.0 {
        0.0
    } else {
        (total_stalls as f64) / (f64::from(u32::try_from(n).unwrap_or(u32::MAX))) / virtual_minutes
    };

    let min_final_confirmed = report
        .final_confirmed
        .iter()
        .copied()
        .min()
        .unwrap_or(i32::MIN);

    CellReport {
        schema: 2,
        label: format!("{}-{}b", params.label, I::WIDTH_BYTES),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        git_sha: std::env::var("FORTRESS_SWEEP_GIT_SHA").unwrap_or_else(|_| "unknown".to_owned()),
        n_players: n,
        loss_pct: params.loss_pct,
        rtt_ms: params.rtt_ms,
        jitter_ms: params.jitter_ms,
        input_width_bytes: I::WIDTH_BYTES,
        steps: params.steps,
        seed: params.seed,
        min_final_confirmed,
        bytes_sent_per_player_per_sec: sum_to_rate_per_sec(total_bytes_sent, n, virtual_secs),
        input_bytes_pre_compression_per_player_per_sec: sum_to_rate_per_sec(
            total_input_pre,
            n,
            virtual_secs,
        ),
        input_bytes_post_compression_per_player_per_sec: sum_to_rate_per_sec(
            total_input_post,
            n,
            virtual_secs,
        ),
        messages_sent_by_kind: messages_sent_by_kind(&report.peer_wire),
        rollbacks_per_100_frames,
        rollback_depth_p50: depth_percentile(&hist, 0.50),
        rollback_depth_p99: depth_percentile(&hist, 0.99),
        rollback_depth_max,
        confirmation_lag_mean,
        confirmation_lag_max,
        stalls_per_min,
        wait_recommendations: total_wait_recs,
        desync_incidents,
    }
}

/// Writes the reports to `FORTRESS_SWEEP_OUT` as JSON Lines, if that env var is
/// set. A best-effort artifact hook; failures are surfaced via `panic!` (test
/// context) so a misconfigured path is never silently dropped.
fn write_jsonl(reports: &[CellReport]) {
    let Ok(path) = std::env::var("FORTRESS_SWEEP_OUT") else {
        return;
    };
    use std::io::Write as _;
    let mut buf = String::new();
    for r in reports {
        let line = serde_json::to_string(r).expect("CellReport serializes");
        buf.push_str(&line);
        buf.push('\n');
    }
    let mut file = std::fs::File::create(&path).expect("FORTRESS_SWEEP_OUT is writable");
    file.write_all(buf.as_bytes()).expect("sweep output writes");
}

/// A checked-in baseline snapshot of one gate cell — the M5 wire-cost ledger.
/// Stores the cell's identity plus its measured cost/behavior metrics; the
/// volatile `version`/`git_sha` and the per-kind map are intentionally omitted
/// so the JSON stays a stable, reviewable diff. Regenerate with
/// `FORTRESS_SWEEP_BLESS=1` (see [`sweep_pr_gate`]).
#[derive(Clone, Debug, Serialize, Deserialize)]
struct BaselineCell {
    label: String,
    n_players: usize,
    loss_pct: f64,
    rtt_ms: u64,
    jitter_ms: u64,
    input_width_bytes: u32,
    steps: u32,
    bytes_sent_per_player_per_sec: f64,
    input_bytes_post_compression_per_player_per_sec: f64,
    rollbacks_per_100_frames: f64,
    rollback_depth_p50: u32,
    rollback_depth_p99: u32,
    rollback_depth_max: u32,
    confirmation_lag_mean: f64,
    confirmation_lag_max: u64,
    stalls_per_min: f64,
    min_final_confirmed: i32,
    desync_incidents: u64,
}

impl BaselineCell {
    fn from_report(r: &CellReport) -> Self {
        Self {
            label: r.label.clone(),
            n_players: r.n_players,
            loss_pct: r.loss_pct,
            rtt_ms: r.rtt_ms,
            jitter_ms: r.jitter_ms,
            input_width_bytes: r.input_width_bytes,
            steps: r.steps,
            bytes_sent_per_player_per_sec: r.bytes_sent_per_player_per_sec,
            input_bytes_post_compression_per_player_per_sec: r
                .input_bytes_post_compression_per_player_per_sec,
            rollbacks_per_100_frames: r.rollbacks_per_100_frames,
            rollback_depth_p50: r.rollback_depth_p50,
            rollback_depth_p99: r.rollback_depth_p99,
            rollback_depth_max: r.rollback_depth_max,
            confirmation_lag_mean: r.confirmation_lag_mean,
            confirmation_lag_max: r.confirmation_lag_max,
            stalls_per_min: r.stalls_per_min,
            min_final_confirmed: r.min_final_confirmed,
            desync_incidents: r.desync_incidents,
        }
    }
}

/// Path to the checked-in gate baseline.
fn baseline_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/simulation/baselines/sweep-v2.json")
}

/// Asserts `actual` is within `rel` (relative) plus `abs_floor` (absolute) of
/// `expected`. Every compared column is derived from integer counters over a
/// deterministic (virtual-time, integer-RNG) run, so it reproduces **exactly** —
/// same-platform and, as the 3-OS CI confirms, cross-platform. The tolerances
/// therefore only need to (a) keep the gate stable across trivial float noise
/// and (b) keep a near-zero baseline (a LAN cell's ~0.1 rollback rate) from
/// being brittle under a purely relative bound; the floors are kept small so the
/// gate still catches a real regression on a low-rollback cell.
fn assert_close(cell: &str, field: &str, actual: f64, expected: f64, rel: f64, abs_floor: f64) {
    let allowed = expected.abs().mul_add(rel, abs_floor);
    let diff = (actual - expected).abs();
    assert!(
        diff <= allowed,
        "baseline drift in cell {cell}: {field} = {actual}, baseline {expected} \
         (allowed ±{allowed:.3}). If intended, refresh with \
         `FORTRESS_SWEEP_BLESS=1 cargo test --test simulation sweep_pr_gate`."
    );
}

/// Compares the gate cells to the checked-in [`baseline_path`] ledger, or
/// regenerates it when `FORTRESS_SWEEP_BLESS` is set. Correctness columns
/// (`desync_incidents`) are exact; cost/behavior columns compare within
/// tolerance (plan §5.3: bytes ±5%, rollbacks ±10%), since a wire-format change
/// (M5's +6 B/packet) shows up here as a reviewed, over-tolerance delta.
fn check_or_bless_baseline(reports: &[CellReport]) {
    let current: Vec<BaselineCell> = reports.iter().map(BaselineCell::from_report).collect();
    let path = baseline_path();

    if std::env::var("FORTRESS_SWEEP_BLESS").is_ok() {
        let json = serde_json::to_string_pretty(&current).expect("baseline serializes");
        std::fs::create_dir_all(path.parent().expect("baseline path has a parent"))
            .expect("create baselines dir");
        std::fs::write(&path, format!("{json}\n")).expect("write baseline");
        return;
    }

    let raw = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read sweep baseline {}: {e}\nGenerate it with \
             `FORTRESS_SWEEP_BLESS=1 cargo test --test simulation sweep_pr_gate`.",
            path.display()
        )
    });
    let baseline: Vec<BaselineCell> = serde_json::from_str(&raw).expect("baseline parses");
    assert_eq!(
        baseline.len(),
        current.len(),
        "gate cell count changed — re-bless the baseline (FORTRESS_SWEEP_BLESS=1)"
    );
    for (cur, base) in current.iter().zip(&baseline) {
        assert_eq!(
            cur.label, base.label,
            "gate cell order/label changed — re-bless the baseline"
        );
        // The cell identity must match the baseline exactly: a reparameterization
        // (different players/loss/rtt/jitter/steps under the same label) changes
        // what the metrics mean, so it must re-bless rather than slip through the
        // metric tolerances below.
        assert_eq!(
            (
                cur.n_players,
                cur.loss_pct,
                cur.rtt_ms,
                cur.jitter_ms,
                cur.input_width_bytes,
                cur.steps,
            ),
            (
                base.n_players,
                base.loss_pct,
                base.rtt_ms,
                base.jitter_ms,
                base.input_width_bytes,
                base.steps,
            ),
            "cell {} parameters changed — re-bless the baseline",
            cur.label
        );
        // Correctness is exact both ways: a clean cell never desyncs.
        assert_eq!(cur.desync_incidents, 0, "cell {} desynced", cur.label);
        assert_eq!(
            base.desync_incidents, 0,
            "baseline cell {} has a desync",
            base.label
        );
        assert_close(
            &cur.label,
            "bytes_sent_per_player_per_sec",
            cur.bytes_sent_per_player_per_sec,
            base.bytes_sent_per_player_per_sec,
            0.05,
            0.5,
        );
        assert_close(
            &cur.label,
            "input_bytes_post_compression_per_player_per_sec",
            cur.input_bytes_post_compression_per_player_per_sec,
            base.input_bytes_post_compression_per_player_per_sec,
            0.05,
            0.5,
        );
        assert_close(
            &cur.label,
            "rollbacks_per_100_frames",
            cur.rollbacks_per_100_frames,
            base.rollbacks_per_100_frames,
            0.10,
            0.05,
        );
    }
}

/// The representative cells the PR gate checks (a fast subset of the full
/// matrix): LAN, wifi, and mobile 2-player profiles plus 4-player wifi and a
/// 4-player heavy-loss/high-RTT stress cell.
fn pr_gate_cells() -> Vec<CellParams> {
    const GATE_STEPS: u32 = 1000;
    const SEED: u64 = 1;
    cells_with_widths([
        CellParams {
            label: "2p-lan",
            n_players: 2,
            loss_pct: 0.0,
            rtt_ms: 10,
            jitter_ms: 0,
            input_width_bytes: 4,
            steps: GATE_STEPS,
            seed: SEED,
        },
        CellParams {
            label: "2p-wifi",
            n_players: 2,
            loss_pct: 1.0,
            rtt_ms: 50,
            jitter_ms: 20,
            input_width_bytes: 4,
            steps: GATE_STEPS,
            seed: SEED,
        },
        CellParams {
            label: "2p-mobile",
            n_players: 2,
            loss_pct: 5.0,
            rtt_ms: 100,
            jitter_ms: 20,
            input_width_bytes: 4,
            steps: GATE_STEPS,
            seed: SEED,
        },
        CellParams {
            label: "4p-wifi",
            n_players: 4,
            loss_pct: 1.0,
            rtt_ms: 50,
            jitter_ms: 20,
            input_width_bytes: 4,
            steps: GATE_STEPS,
            seed: SEED,
        },
        CellParams {
            label: "4p-loss15-rtt200",
            n_players: 4,
            loss_pct: 15.0,
            rtt_ms: 200,
            jitter_ms: 20,
            input_width_bytes: 4,
            steps: GATE_STEPS,
            seed: SEED,
        },
    ])
}

#[test]
fn sweep_pr_gate() {
    let cells = pr_gate_cells();
    let reports: Vec<CellReport> = cells.iter().copied().map(run_cell).collect();

    for r in &reports {
        assert_cell_health(r);
    }

    // Determinism: virtual time ⇒ a cell is bit-for-bit reproducible.
    let first = &reports[0];
    let replay = run_cell(pr_gate_cells()[0]);
    assert_eq!(*first, replay, "cell is not reproducible: {}", first.label);
    let wide = &reports[1];
    let wide_replay = run_cell(pr_gate_cells()[1]);
    assert_eq!(
        *wide, wide_replay,
        "wide-input cell is not reproducible: {}",
        wide.label
    );

    write_jsonl(&reports);

    // Regression gate against the checked-in cost ledger (M5 baseline).
    check_or_bless_baseline(&reports);
}

/// Returns the full baseline matrix used by the ignored
/// [`full_matrix_sweep`] test: 5000-frame cells across the loss × RTT ×
/// jitter grid plus scale spot rows, intended for nightly or offline capture
/// via `FORTRESS_SWEEP_OUT`, not PR CI.
fn full_matrix_cells() -> Vec<CellParams> {
    const STEPS: u32 = 5000;
    const SEED: u64 = 1;
    let losses = [0.0, 1.0, 5.0, 15.0];
    let rtts = [10u64, 50, 100, 200];
    let jitters = [0u64, 20];
    let mut base_cells = Vec::new();
    for &n_players in &[2usize, 4] {
        for &loss_pct in &losses {
            for &rtt_ms in &rtts {
                for &jitter_ms in &jitters {
                    base_cells.push(CellParams {
                        label: "matrix",
                        n_players,
                        loss_pct,
                        rtt_ms,
                        jitter_ms,
                        input_width_bytes: 4,
                        steps: STEPS,
                        seed: SEED,
                    });
                }
            }
        }
    }
    for (label, n_players, loss_pct, rtt_ms, jitter_ms) in [
        ("3p-regional", 3usize, 1.0, 100u64, 20u64),
        ("8p-regional", 8, 1.0, 100, 20),
        ("12p-regional", 12, 1.0, 100, 20),
        ("16p-regional", 16, 1.0, 100, 20),
    ] {
        base_cells.push(CellParams {
            label,
            n_players,
            loss_pct,
            rtt_ms,
            jitter_ms,
            input_width_bytes: 4,
            steps: STEPS,
            seed: SEED,
        });
    }
    cells_with_widths(base_cells)
}

#[test]
fn full_matrix_includes_scale_spot_rows() {
    let cells = full_matrix_cells();
    let expected_spots: [(&str, usize, f64, u64, u64); 4] = [
        ("3p-regional", 3usize, 1.0, 100u64, 20u64),
        ("8p-regional", 8, 1.0, 100, 20),
        ("12p-regional", 12, 1.0, 100, 20),
        ("16p-regional", 16, 1.0, 100, 20),
    ];
    assert_eq!(
        cells.len(),
        136,
        "full matrix should be the 64-cell grid plus 4 scale spot rows, across 2 input widths"
    );
    for (label, n_players, loss_pct, rtt_ms, jitter_ms) in expected_spots {
        for input_width_bytes in INPUT_WIDTHS_BYTES {
            assert!(
                cells.iter().any(|cell| {
                    cell.label == label
                        && cell.n_players == n_players
                        && cell.loss_pct.to_bits() == loss_pct.to_bits()
                        && cell.rtt_ms == rtt_ms
                        && cell.jitter_ms == jitter_ms
                        && cell.input_width_bytes == input_width_bytes
                }),
                "full matrix must include exact scale spot row {label} at {input_width_bytes}B"
            );
        }
    }
}

#[test]
fn input_width_axis_changes_measured_raw_input_bytes() {
    let base = CellParams {
        label: "2p-axis",
        n_players: 2,
        loss_pct: 0.0,
        rtt_ms: 10,
        jitter_ms: 0,
        input_width_bytes: StubInput::WIDTH_BYTES,
        steps: 300,
        seed: 1,
    };
    let narrow = run_cell(base);
    let wide = run_cell(CellParams {
        input_width_bytes: WideStubInput::WIDTH_BYTES,
        ..base
    });

    assert_eq!(narrow.input_width_bytes, StubInput::WIDTH_BYTES);
    assert_eq!(wide.input_width_bytes, WideStubInput::WIDTH_BYTES);
    assert_eq!(
        narrow.desync_incidents, 0,
        "narrow cell desynced: {narrow:?}"
    );
    assert_eq!(wide.desync_incidents, 0, "wide cell desynced: {wide:?}");
    assert!(
        wide.input_bytes_pre_compression_per_player_per_sec
            > narrow.input_bytes_pre_compression_per_player_per_sec,
        "wide input should increase raw input bytes: narrow={narrow:?} wide={wide:?}"
    );
}

fn assert_cell_health(r: &CellReport) {
    let ctx = || format!("cell {}: {r:?}", r.label);
    // The load-bearing invariant: constant-conditions meshes never desync.
    assert_eq!(r.desync_incidents, 0, "desync in a clean cell — {}", ctx());
    // Liveness: every cell confirms real frames (a stalled mesh would pin near
    // frame 0). 1000 steps ≈ 16 virtual seconds even at loss15/rtt200; the full
    // matrix runs 5000 steps, so this floor is intentionally conservative there.
    assert!(
        r.min_final_confirmed >= 50,
        "cell made too little progress — {}",
        ctx()
    );
    // Real wire traffic flowed, and the per-kind map covers every kind.
    assert!(
        r.bytes_sent_per_player_per_sec > 0.0,
        "no bandwidth recorded — {}",
        ctx()
    );
    assert_eq!(
        r.messages_sent_by_kind.len(),
        MessageKind::COUNT,
        "by-kind map missing categories — {}",
        ctx()
    );
    // Rollback-depth percentiles are ordered and bounded by the observed max —
    // a real cross-check between the histogram (percentile source) and the
    // independently-tracked `max_rollback_depth`.
    assert!(
        r.rollback_depth_p50 <= r.rollback_depth_p99,
        "p50 > p99 — {}",
        ctx()
    );
    assert!(
        r.rollback_depth_p99 <= r.rollback_depth_max,
        "p99 exceeds max — {}",
        ctx()
    );
}

#[test]
#[ignore = "full matrix; run offline with FORTRESS_SWEEP_OUT set"]
fn full_matrix_sweep() {
    let reports: Vec<CellReport> = full_matrix_cells().into_iter().map(run_cell).collect();
    write_jsonl(&reports);
    for r in &reports {
        assert_cell_health(r);
    }
}
