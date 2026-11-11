//! P1 pacing/liveness-cliff qualification: measured ablations over the
//! prediction window, input delay, and application wait compliance.
//!
//! `docs/tuning.md` places the mobile 2-player and severe 4-player rows at the
//! prediction-window boundary. This module turns that observation into a
//! durable, deterministic experiment rather than an operator anecdote:
//!
//! - the dominant stall mechanism is confirmation headroom, not a controller
//!   oscillation: raising `max_prediction` collapses advance stalls but
//!   superlinearly raises resimulation work;
//! - `WaitRecommendation` is near-silent under uniformly high RTT because the
//!   symmetric `RTT/2` frame-age estimate makes frame *advantage* ~0 even when
//!   confirmation is a one-way trip behind, so application pacing cannot
//!   substitute for a prediction window sized to the measured path;
//! - raising input delay reduces work at a fixed window but adds local input
//!   latency and does not move the structural boundary.
//!
//! The test is deterministic (virtual time and integer counters), so its
//! printed table reproduces exactly and can be cited in the tuning guide.
use super::baseline_sweep::CellParams;
use super::harness::schedule::AppModel;
use super::harness::{run_with_input, RunOptions, RunReport, SimInput};
use crate::common::stubs::StubInput;

/// One measured ablation row.
#[derive(Clone, Copy, Debug)]
struct PacingRow {
    max_prediction: usize,
    input_delay: usize,
    advance_stalls: u64,
    wait_recommendations: u64,
    wait_frames_obeyed: u64,
    rollbacks: u64,
    resimulated_frames: u64,
    confirmation_lag_mean: f64,
    min_final_confirmed: i32,
}

fn metric_sum<F>(report: &RunReport, mut field: F) -> u64
where
    F: FnMut(&fortress_rollback::SessionMetrics) -> u64,
{
    report
        .metrics
        .iter()
        .map(&mut field)
        .fold(0, u64::saturating_add)
}

fn measure(
    cell: CellParams,
    max_prediction: usize,
    input_delay: usize,
    app_model: AppModel,
) -> PacingRow {
    let mut schedule = cell.schedule();
    schedule.config.max_prediction = max_prediction;
    schedule.config.input_delay = input_delay;
    schedule.config.app_model = app_model;
    let report = run_with_input::<StubInput>(&schedule, &RunOptions::default());

    let visual = metric_sum(&report, |metrics| metrics.visual_frames);
    let lag_sum = metric_sum(&report, |metrics| metrics.confirmation_lag_sum);
    #[allow(clippy::cast_precision_loss)]
    let confirmation_lag_mean = if visual == 0 {
        0.0
    } else {
        lag_sum as f64 / visual as f64
    };
    PacingRow {
        max_prediction,
        input_delay,
        advance_stalls: metric_sum(&report, |metrics| metrics.stall_count),
        wait_recommendations: metric_sum(&report, |metrics| metrics.wait_recommendations),
        wait_frames_obeyed: report.wait_frames_obeyed.iter().copied().sum(),
        rollbacks: metric_sum(&report, |metrics| metrics.rollback_count),
        resimulated_frames: metric_sum(&report, |metrics| metrics.resimulated_frames),
        confirmation_lag_mean,
        min_final_confirmed: report
            .final_confirmed
            .iter()
            .copied()
            .min()
            .unwrap_or(i32::MIN),
    }
}

const STEPS: u32 = 1_000;
const SEED: u64 = 1;

fn mobile_cell() -> CellParams {
    CellParams {
        label: "2p-mobile",
        n_players: 2,
        loss_pct: 5.0,
        rtt_ms: 100,
        jitter_ms: 20,
        input_width_bytes: StubInput::WIDTH_BYTES,
        steps: STEPS,
        seed: SEED,
    }
}

fn severe_cell() -> CellParams {
    CellParams {
        label: "4p-severe",
        n_players: 4,
        loss_pct: 15.0,
        rtt_ms: 200,
        jitter_ms: 20,
        input_width_bytes: StubInput::WIDTH_BYTES,
        steps: STEPS,
        seed: SEED,
    }
}

/// The baseline rows must reproduce the checked-in gate ledger before the
/// ablation is meaningful: same cell identity, same measured stalls.
#[test]
fn pacing_ablation_baseline_matches_gate_ledger() {
    let mobile = measure(mobile_cell(), 8, 1, AppModel::Ignore);
    let severe = measure(severe_cell(), 8, 1, AppModel::Ignore);
    assert_eq!(mobile.advance_stalls, 8, "{mobile:?}");
    assert_eq!(severe.advance_stalls, 2_202, "{severe:?}");
    assert_eq!(mobile.min_final_confirmed, 947, "{mobile:?}");
    assert_eq!(severe.min_final_confirmed, 309, "{severe:?}");
}

/// A prediction window that does not cover the measured confirmation tail is
/// the dominant stall cause. Widening it removes stalls at a measured
/// resimulation cost; this is a capacity boundary, not controller oscillation.
#[test]
#[allow(clippy::print_stdout, clippy::disallowed_macros)]
fn pacing_cliff_tracks_prediction_headroom() {
    let mut rows = Vec::new();
    for cell in [mobile_cell(), severe_cell()] {
        for max_prediction in [8_usize, 12, 16, 24] {
            let row = measure(cell, max_prediction, 1, AppModel::Ignore);
            println!(
                "PACING {} mp={} delay={} stalls={} waits={} rollbacks={} resim={} lag_mean={:.2} final={}",
                cell.label,
                row.max_prediction,
                row.input_delay,
                row.advance_stalls,
                row.wait_recommendations,
                row.rollbacks,
                row.resimulated_frames,
                row.confirmation_lag_mean,
                row.min_final_confirmed,
            );
            rows.push((cell.label, row));
        }
    }

    for label in ["2p-mobile", "4p-severe"] {
        let cell_rows: Vec<PacingRow> = rows
            .iter()
            .filter(|(cell_label, _)| *cell_label == label)
            .map(|(_, row)| *row)
            .collect();
        assert_eq!(cell_rows.len(), 4, "{label} row count");

        for pair in cell_rows.windows(2) {
            assert!(
                pair[1].advance_stalls <= pair[0].advance_stalls,
                "{label}: raising max_prediction must not increase stalls: {pair:?}"
            );
            assert!(
                pair[1].confirmation_lag_mean >= pair[0].confirmation_lag_mean,
                "{label}: a wider window is filled to a deeper lag: {pair:?}"
            );
        }
        if label == "4p-severe" {
            for pair in cell_rows.windows(2) {
                assert!(
                    pair[1].resimulated_frames >= pair[0].resimulated_frames,
                    "the severe row must charge resimulation for the wider window: {pair:?}"
                );
            }
        }
    }

    let mobile_8 = rows
        .iter()
        .find(|(label, row)| *label == "2p-mobile" && row.max_prediction == 8)
        .map(|(_, row)| *row)
        .expect("2p-mobile mp=8 row");
    let mobile_16 = rows
        .iter()
        .find(|(label, row)| *label == "2p-mobile" && row.max_prediction == 16)
        .map(|(_, row)| *row)
        .expect("2p-mobile mp=16 row");
    assert!(
        mobile_16.advance_stalls < mobile_8.advance_stalls,
        "mobile stalls must fall with headroom: {mobile_8:?} -> {mobile_16:?}"
    );
    assert!(
        mobile_16.resimulated_frames <= mobile_8.resimulated_frames.saturating_mul(105) / 100,
        "the mobile row is already at its resimulation plateau; extra headroom must be \
         nearly free: {mobile_8:?} -> {mobile_16:?}"
    );

    let severe_8 = rows
        .iter()
        .find(|(label, row)| *label == "4p-severe" && row.max_prediction == 8)
        .map(|(_, row)| *row)
        .expect("4p-severe mp=8 row");
    let severe_16 = rows
        .iter()
        .find(|(label, row)| *label == "4p-severe" && row.max_prediction == 16)
        .map(|(_, row)| *row)
        .expect("4p-severe mp=16 row");
    assert!(
        severe_16.advance_stalls.saturating_mul(2) < severe_8.advance_stalls,
        "severe 4-player stalls must at least halve at mp=16: {severe_8:?} -> {severe_16:?}"
    );
    assert!(
        severe_16.resimulated_frames > severe_8.resimulated_frames.saturating_mul(4),
        "the stall reduction must carry the measured resimulation cost: {severe_8:?} -> {severe_16:?}"
    );
}

/// At equal RTT in both directions the frame-age estimate cancels most of the
/// transport delay, so `WaitRecommendation` is silent even while confirmation
/// sits a full one-way trip behind and the prediction window repeatedly fills.
/// Obeying the (absent) recommendation therefore cannot recover the stalls.
#[test]
fn wait_recommendation_cannot_replace_prediction_headroom() {
    let severe_base = measure(severe_cell(), 8, 1, AppModel::Ignore);
    let severe_obey = measure(severe_cell(), 8, 1, AppModel::Obey);
    assert_eq!(
        severe_obey.wait_recommendations, 0,
        "a uniform-RTT severe mesh must not emit frame-advantage waits: {severe_obey:?}"
    );
    assert_eq!(
        severe_obey.wait_frames_obeyed, 0,
        "no recommendation should reach the app: {severe_obey:?}"
    );
    assert_eq!(
        severe_obey.advance_stalls, severe_base.advance_stalls,
        "obeying an absent recommendation cannot change the stall count: \
         {severe_base:?} -> {severe_obey:?}"
    );

    let mobile_base = measure(mobile_cell(), 8, 1, AppModel::Ignore);
    let mobile_obey = measure(mobile_cell(), 8, 1, AppModel::Obey);
    assert!(
        mobile_obey.wait_recommendations > 0,
        "the mobile row is the positive control: a wait must still be observable: \
         {mobile_obey:?}"
    );
    assert!(
        mobile_obey.advance_stalls >= mobile_base.advance_stalls / 2,
        "one 3-frame wait cannot remove the structural stall boundary: \
         {mobile_base:?} -> {mobile_obey:?}"
    );
}

/// Input delay is a real tuning lever at a fixed window: it reduces stalls and
/// resimulation work, but it adds local input latency and leaves the
/// confirmation-headroom boundary in place.
#[test]
fn input_delay_trades_latency_for_work_at_a_fixed_window() {
    let delay_1 = measure(severe_cell(), 8, 1, AppModel::Ignore);
    let delay_3 = measure(severe_cell(), 8, 3, AppModel::Ignore);
    assert!(
        delay_3.advance_stalls < delay_1.advance_stalls,
        "delay 3 must reduce stalls at a fixed window: {delay_1:?} -> {delay_3:?}"
    );
    assert!(
        delay_3.resimulated_frames < delay_1.resimulated_frames,
        "delay 3 must reduce resimulation work at a fixed window: \
         {delay_1:?} -> {delay_3:?}"
    );
    assert!(
        delay_3.confirmation_lag_mean <= delay_1.confirmation_lag_mean,
        "the work reduction must not come from a deeper confirmation lag: \
         {delay_1:?} -> {delay_3:?}"
    );
}

/// Isolates the two candidate mechanisms at the default eight-frame window:
/// path latency versus retransmission/loss recovery. A lossless 200 ms path is
/// nearly as stall-bound as the 15% loss severe row, while 15% loss on a 10 ms
/// path is an order of magnitude cheaper. Latency headroom is primary; loss
/// adds recovery time on top but cannot explain the severe cliff.
#[test]
fn latency_headroom_dominates_loss_recovery() {
    let cell = |loss_pct: f64, rtt_ms: u64| CellParams {
        label: "4p-isolation",
        n_players: 4,
        loss_pct,
        rtt_ms,
        jitter_ms: 20,
        input_width_bytes: StubInput::WIDTH_BYTES,
        steps: STEPS,
        seed: SEED,
    };

    let lossless_high_latency = measure(cell(0.0, 200), 8, 1, AppModel::Ignore);
    let lossy_low_latency = measure(cell(15.0, 10), 8, 1, AppModel::Ignore);
    let clean_control = measure(cell(0.0, 10), 8, 1, AppModel::Ignore);

    assert_eq!(
        clean_control.advance_stalls, 0,
        "the clean low-latency control must not stall: {clean_control:?}"
    );
    assert!(
        lossless_high_latency.advance_stalls >= 1_500,
        "a lossless 200 ms path must remain stall-bound at the default window: \
         {lossless_high_latency:?}"
    );
    assert!(
        lossy_low_latency.advance_stalls <= 200,
        "15% loss on a 10 ms path must stay far below the latency cliff: \
         {lossy_low_latency:?}"
    );
    assert!(
        lossy_low_latency.advance_stalls.saturating_mul(5) < lossless_high_latency.advance_stalls,
        "latency headroom must dominate loss recovery by at least 5x: \
         lossy_low={lossy_low_latency:?}, lossless_high={lossless_high_latency:?}"
    );
}
