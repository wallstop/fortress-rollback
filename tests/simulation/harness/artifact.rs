//! Stable JSON failure artifacts for deterministic simulation runs.

use super::oracle::OracleFailure;
use super::schedule::{validate_schedule, Schedule};
use super::{
    phase_control_sample_capacity, CpuFeedbackEvidence, HostileGossipEvidence, ProgressSample,
    ReceiptRangeEvidence, RetainedInputRangeEvidence, RunReport, TraceSnapshot,
    TRACE_STEP_EVENT_CAPACITY, TRACE_TAIL_CAPACITY, WAIT_POLICY_CONTROL_SAMPLE_CAPACITY,
};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Schema version for [`FailureArtifact`].
// Version 7 preserves hostile-gossip mutation and receiver-side diagnostic
// evidence. Older artifacts cannot reproduce that evidence and are rejected
// explicitly.
pub const FAILURE_ARTIFACT_SCHEMA_VERSION: u32 = 7;
/// Maximum serialized artifact size accepted at the filesystem boundary.
pub const MAX_FAILURE_ARTIFACT_BYTES: usize = 8 * 1024 * 1024;
/// Oracle failures are capped at 64 per run; artifacts preserve at most that cap.
pub const MAX_ARTIFACT_FAILURES: usize = 64;
/// Maximum diagnostic bytes retained for one failure.
pub const MAX_FAILURE_DETAIL_BYTES: usize = 4096;
/// Maximum bytes in the sanitized test-name path component.
pub const MAX_TEST_NAME_COMPONENT_BYTES: usize = 120;

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn validate_cpu_feedback_entries(
    entries: &[CpuFeedbackEvidence],
    expected: usize,
    schedule: &Schedule,
    context: &str,
) -> Result<(), String> {
    if entries.len() != expected {
        return Err(format!(
            "{context} has {} cpu_feedback entries (expected {expected})",
            entries.len()
        ));
    }
    let Some(policy) = schedule.config.cpu_feedback_policy else {
        return Ok(());
    };
    for (peer, evidence) in entries.iter().enumerate() {
        let expected_work = evidence
            .charged_frames
            .saturating_mul(u64::from(policy.simulated_frame_cost_us));
        if evidence.work_us != expected_work {
            return Err(format!(
                "{context} cpu_feedback[{peer}] work_us {} does not equal charged_frames {} × cost {}",
                evidence.work_us, evidence.charged_frames, policy.simulated_frame_cost_us
            ));
        }
        let exceeds_delay_cap = evidence.remaining_delay_steps > policy.max_poll_delay_steps
            || evidence.max_delay_streak > policy.max_poll_delay_steps;
        let reverses_streak_order = evidence.current_delay_streak > evidence.max_delay_streak;
        if exceeds_delay_cap || reverses_streak_order {
            return Err(format!(
                "{context} cpu_feedback[{peer}] exceeds the declared delay cap or streak ordering"
            ));
        }
        if (evidence.clamp_count == 0) != (evidence.clamped_delay_steps == 0) {
            return Err(format!(
                "{context} cpu_feedback[{peer}] has inconsistent clamp evidence"
            ));
        }
    }
    Ok(())
}

fn validate_receipt_range_evidence(
    evidence: Option<&ReceiptRangeEvidence>,
    expected_target: Option<usize>,
    peers: usize,
    steps: u32,
    context: &str,
) -> Result<(), String> {
    if evidence.is_some() != expected_target.is_some() {
        return Err(format!(
            "{context} receipt_range_probe presence does not match replay options"
        ));
    }
    let Some(evidence) = evidence else {
        return Ok(());
    };
    if Some(evidence.target) != expected_target || evidence.target >= peers {
        return Err(format!(
            "{context} receipt_range_probe target {} does not match the requested target",
            evidence.target
        ));
    }
    for (name, len) in [
        ("receipts", evidence.receipts.len()),
        ("connected", evidence.connected.len()),
        ("retained_ranges", evidence.retained_ranges.len()),
    ] {
        if len != peers {
            return Err(format!(
                "{context} receipt_range_probe has {len} {name} entries for {peers} peers"
            ));
        }
    }
    let target_has_observation = evidence
        .receipts
        .get(evidence.target)
        .is_some_and(Option::is_some)
        || evidence
            .connected
            .get(evidence.target)
            .is_some_and(Option::is_some)
        || evidence
            .retained_ranges
            .get(evidence.target)
            .is_some_and(Option::is_some);
    if target_has_observation {
        return Err(format!(
            "{context} receipt_range_probe must exclude the target's self-observation"
        ));
    }
    if evidence.receipts.iter().flatten().any(|frame| *frame < 0)
        || evidence
            .retained_ranges
            .iter()
            .flatten()
            .any(|range| range.first < 0 || range.first > range.last)
    {
        return Err(format!(
            "{context} receipt_range_probe contains an invalid frame or retained range"
        ));
    }
    for observer in 0..peers {
        let receipt = evidence.receipts[observer];
        let retained = evidence.retained_ranges[observer];
        if let (Some(true), Some(receipt), Some(retained)) =
            (evidence.connected[observer], receipt, retained)
        {
            if retained.last != receipt {
                return Err(format!(
                    "{context} receipt_range_probe observer {observer} retained range ends at {}, not receipt {receipt}",
                    retained.last
                ));
            }
        }
    }
    let Some(at_step) = evidence.at_step else {
        if evidence.max_spread != 0 {
            return Err(format!(
                "{context} receipt_range_probe has spread without a sample step"
            ));
        }
        if evidence.receipts.iter().any(Option::is_some)
            || evidence.connected.iter().any(Option::is_some)
            || evidence.retained_ranges.iter().any(Option::is_some)
        {
            return Err(format!(
                "{context} receipt_range_probe has observations without a sample step"
            ));
        }
        return Ok(());
    };
    if at_step >= steps {
        return Err(format!(
            "{context} receipt_range_probe step {at_step} is outside 0..{steps}"
        ));
    }
    let mut minimum = i32::MAX;
    let mut maximum = i32::MIN;
    let mut samples = 0_usize;
    for ((&connected, &receipt), &retained) in evidence
        .connected
        .iter()
        .zip(&evidence.receipts)
        .zip(&evidence.retained_ranges)
    {
        if let (Some(true), Some(frame), Some(_)) = (connected, receipt, retained) {
            minimum = minimum.min(frame);
            maximum = maximum.max(frame);
            samples = samples.saturating_add(1);
        }
    }
    if samples < 2 {
        return Err(format!(
            "{context} receipt_range_probe high-water sample has fewer than two connected observers"
        ));
    }
    let spread = u32::try_from(maximum.saturating_sub(minimum)).unwrap_or(u32::MAX);
    if evidence.max_spread != spread {
        return Err(format!(
            "{context} receipt_range_probe spread {} does not match receipt extrema {spread}",
            evidence.max_spread
        ));
    }
    Ok(())
}

fn validate_hostile_gossip_evidence(
    evidence: Option<&HostileGossipEvidence>,
    options: Option<super::HostileGossipOptions>,
    expected_probe_step: Option<u32>,
    context: &str,
) -> Result<(), String> {
    if evidence.is_some() != options.is_some() {
        return Err(format!(
            "{context} hostile_gossip evidence presence does not match replay options"
        ));
    }
    let (Some(evidence), Some(options)) = (evidence, options) else {
        return Ok(());
    };
    let Some(probe) = evidence.probe else {
        return Err(format!(
            "{context} hostile_gossip is missing receiver-side probe evidence"
        ));
    };
    if Some(probe.at_step) != expected_probe_step
        || probe.at_step < options.from_step
        || probe.at_step >= options.until_step
        || !probe.endpoint_running
        || probe.observer_confirmed < fortress_rollback::Frame::NULL.as_i32()
        || probe.status_frame < fortress_rollback::Frame::NULL.as_i32()
        || probe.round_floor < fortress_rollback::Frame::NULL.as_i32()
        || probe.effective_reported_frame < fortress_rollback::Frame::NULL.as_i32()
        || probe
            .direct_receipt
            .is_some_and(|frame| frame < fortress_rollback::Frame::NULL.as_i32())
        || probe
            .target_confirmed_bound
            .is_some_and(|frame| frame < fortress_rollback::Frame::NULL.as_i32())
        || probe.reply_seq > probe.request_seq
        || probe.prune_seq > probe.request_seq
    {
        return Err(format!(
            "{context} hostile_gossip receiver-side probe is inconsistent"
        ));
    }
    if evidence.messages_mutated == 0 {
        if evidence.first_step.is_some()
            || evidence.last_step.is_some()
            || evidence.last_before.is_some()
            || evidence.last_after.is_some()
            || evidence.last_round_seq.is_some()
        {
            return Err(format!(
                "{context} hostile_gossip has step evidence without a mutation"
            ));
        }
        return Ok(());
    }
    let (Some(first), Some(last)) = (evidence.first_step, evidence.last_step) else {
        return Err(format!(
            "{context} hostile_gossip mutations are missing step evidence"
        ));
    };
    if first > last || first < options.from_step || last >= options.until_step {
        return Err(format!(
            "{context} hostile_gossip mutation steps are outside the configured interval"
        ));
    }
    let (Some(before), Some(after)) = (evidence.last_before, evidence.last_after) else {
        return Err(format!(
            "{context} hostile_gossip mutations are missing before/after evidence"
        ));
    };
    if before < 0
        || after < 0
        || before == after
        || after != before.saturating_add(options.delta).max(0)
    {
        return Err(format!(
            "{context} hostile_gossip before/after evidence does not match the configured delta"
        ));
    }
    if (options.mode == super::HostileGossipMode::FloorReply) != evidence.last_round_seq.is_some() {
        return Err(format!(
            "{context} hostile_gossip round-sequence evidence does not match its mode"
        ));
    }
    Ok(())
}

/// Stable serialization identity for every oracle failure variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailureClass {
    ConfirmedInputDivergence,
    StateDivergence,
    InbandDesyncDetected,
    ChecksumMismatchMetric,
    ConfirmedInputUnavailable,
    SessionError,
    Violation,
    InvalidViolationSource,
    EndProgress,
    NoLivePeers,
    PostHealLiveness,
    FreezeFrameDivergence,
    FreezeFrameMissing,
    SpectatorDivergenceEvent,
    SpectatorSessionError,
    SpectatorProgressMissing,
    SpectatorMeshCanonMissing,
    SpectatorInputDivergence,
}

impl FailureClass {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ConfirmedInputDivergence => "ConfirmedInputDivergence",
            Self::StateDivergence => "StateDivergence",
            Self::InbandDesyncDetected => "InbandDesyncDetected",
            Self::ChecksumMismatchMetric => "ChecksumMismatchMetric",
            Self::ConfirmedInputUnavailable => "ConfirmedInputUnavailable",
            Self::SessionError => "SessionError",
            Self::Violation => "Violation",
            Self::InvalidViolationSource => "InvalidViolationSource",
            Self::EndProgress => "EndProgress",
            Self::NoLivePeers => "NoLivePeers",
            Self::PostHealLiveness => "PostHealLiveness",
            Self::FreezeFrameDivergence => "FreezeFrameDivergence",
            Self::FreezeFrameMissing => "FreezeFrameMissing",
            Self::SpectatorDivergenceEvent => "SpectatorDivergenceEvent",
            Self::SpectatorSessionError => "SpectatorSessionError",
            Self::SpectatorProgressMissing => "SpectatorProgressMissing",
            Self::SpectatorMeshCanonMissing => "SpectatorMeshCanonMissing",
            Self::SpectatorInputDivergence => "SpectatorInputDivergence",
        }
    }
}

impl From<&OracleFailure> for FailureClass {
    fn from(failure: &OracleFailure) -> Self {
        match failure {
            OracleFailure::ConfirmedInputDivergence { .. } => Self::ConfirmedInputDivergence,
            OracleFailure::StateDivergence { .. } => Self::StateDivergence,
            OracleFailure::InbandDesyncDetected { .. } => Self::InbandDesyncDetected,
            OracleFailure::ChecksumMismatchMetric { .. } => Self::ChecksumMismatchMetric,
            OracleFailure::ConfirmedInputUnavailable { .. } => Self::ConfirmedInputUnavailable,
            OracleFailure::SessionError { .. } => Self::SessionError,
            OracleFailure::Violation { .. } => Self::Violation,
            OracleFailure::InvalidViolationSource { .. } => Self::InvalidViolationSource,
            OracleFailure::EndProgress { .. } => Self::EndProgress,
            OracleFailure::NoLivePeers { .. } => Self::NoLivePeers,
            OracleFailure::PostHealLiveness { .. } => Self::PostHealLiveness,
            OracleFailure::FreezeFrameDivergence { .. } => Self::FreezeFrameDivergence,
            OracleFailure::FreezeFrameMissing { .. } => Self::FreezeFrameMissing,
            OracleFailure::SpectatorDivergenceEvent { .. } => Self::SpectatorDivergenceEvent,
            OracleFailure::SpectatorSessionError { .. } => Self::SpectatorSessionError,
            OracleFailure::SpectatorProgressMissing { .. } => Self::SpectatorProgressMissing,
            OracleFailure::SpectatorMeshCanonMissing { .. } => Self::SpectatorMeshCanonMissing,
            OracleFailure::SpectatorInputDivergence { .. } => Self::SpectatorInputDivergence,
        }
    }
}

/// One stable failure identity plus its full human-readable diagnostic.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactFailure {
    /// Stable oracle variant name used by replay and shrinking.
    pub class: FailureClass,
    /// Complete diagnostic at the time the artifact was captured.
    pub details: String,
}

/// A self-contained, replayable simulation failure.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FailureArtifact {
    /// Artifact layout version, independent of the embedded schedule schema.
    pub artifact_schema_version: u32,
    /// Full materialized schedule; replay never depends on the current generator.
    pub schedule: Schedule,
    /// Exact harness options used by the failed run.
    pub replay_options: super::RunOptions,
    /// Selects the deterministic harness input representation.
    pub replay_input_width_bytes: u32,
    /// Stable failure identities with complete diagnostics.
    pub failures: Vec<ArtifactFailure>,
    /// Deterministic digest of the complete run.
    pub trace_hash: u64,
    /// Per-peer final confirmed frames.
    pub final_confirmed: Vec<i32>,
    /// Bounded clock/control-loop samples retained for skew failures.
    #[serde(default)]
    pub progress_samples: Vec<ProgressSample>,
    /// Final per-peer frame-opportunity totals.
    #[serde(default)]
    pub frame_opportunities: Vec<u64>,
    /// Final per-peer obeyed-wait totals.
    #[serde(default)]
    pub wait_frames_obeyed: Vec<u64>,
    /// Largest recommendation payload emitted for each peer.
    #[serde(default)]
    pub wait_recommendation_max: Vec<u32>,
    /// Sum of all recommendation payloads emitted for each peer.
    #[serde(default)]
    pub wait_recommendation_frames: Vec<u64>,
    /// Recommendations accepted by each application-side actuator.
    #[serde(default)]
    pub wait_recommendations_accepted: Vec<u64>,
    /// Recommendation debt accepted by each application-side actuator.
    #[serde(default)]
    pub wait_frames_accepted: Vec<u64>,
    /// Final per-peer deterministic CPU-feedback evidence.
    #[serde(default)]
    pub cpu_feedback: Vec<CpuFeedbackEvidence>,
    /// Direct-receipt and retained-range high-water evidence, when requested.
    #[serde(default)]
    pub receipt_range_probe: Option<ReceiptRangeEvidence>,
    /// Captured hostile-gossip mutation and receiver-side diagnostic evidence.
    #[serde(default)]
    pub hostile_gossip: Option<HostileGossipEvidence>,
    /// Bounded final step snapshots.
    pub trace_tail: Vec<TraceSnapshot>,
}

impl FailureArtifact {
    /// Captures the stable replay surface from a failed report.
    #[must_use]
    pub fn from_report(schedule: &Schedule, report: &RunReport) -> Self {
        Self {
            artifact_schema_version: FAILURE_ARTIFACT_SCHEMA_VERSION,
            schedule: schedule.clone(),
            replay_options: report.replay_options.clone(),
            replay_input_width_bytes: report.replay_input_width_bytes,
            failures: report
                .verdict
                .failures
                .iter()
                .map(|failure| ArtifactFailure {
                    class: FailureClass::from(failure),
                    details: bounded_failure_details(format!("{failure:?}")),
                })
                .collect(),
            trace_hash: report.trace_hash,
            final_confirmed: report.final_confirmed.clone(),
            progress_samples: report.progress_samples.clone(),
            frame_opportunities: report.frame_opportunities.clone(),
            wait_frames_obeyed: report.wait_frames_obeyed.clone(),
            wait_recommendation_max: report.wait_recommendation_max.clone(),
            wait_recommendation_frames: report.wait_recommendation_frames.clone(),
            wait_recommendations_accepted: report.wait_recommendations_accepted.clone(),
            wait_frames_accepted: report.wait_frames_accepted.clone(),
            cpu_feedback: report.cpu_feedback.clone(),
            receipt_range_probe: report.receipt_range_probe.clone(),
            hostile_gossip: report.hostile_gossip,
            trace_tail: report.trace_tail.clone(),
        }
    }

    /// Rejects unknown schemas before replay or promotion.
    pub fn validate(&self) -> Result<(), String> {
        if self.artifact_schema_version != FAILURE_ARTIFACT_SCHEMA_VERSION {
            return Err(format!(
                "unsupported failure artifact schema {} (expected {})",
                self.artifact_schema_version, FAILURE_ARTIFACT_SCHEMA_VERSION
            ));
        }
        if !matches!(self.replay_input_width_bytes, 4 | 32) {
            return Err(format!(
                "unsupported replay input width {} (expected 4 or 32)",
                self.replay_input_width_bytes
            ));
        }
        if self.failures.is_empty() || self.failures.len() > MAX_ARTIFACT_FAILURES {
            return Err(format!(
                "failure artifact must contain 1..={MAX_ARTIFACT_FAILURES} failures (got {})",
                self.failures.len()
            ));
        }
        if self.failures.iter().any(|failure| {
            failure.details.is_empty() || failure.details.len() > MAX_FAILURE_DETAIL_BYTES
        }) {
            return Err(format!(
                "artifact failure details must contain 1..={MAX_FAILURE_DETAIL_BYTES} bytes"
            ));
        }
        if self.trace_tail.is_empty() || self.trace_tail.len() > TRACE_TAIL_CAPACITY {
            return Err(format!(
                "artifact trace tail has {} entries (required 1..={})",
                self.trace_tail.len(),
                TRACE_TAIL_CAPACITY
            ));
        }
        validate_schedule(&self.schedule)
            .map_err(|error| format!("invalid embedded materialized schedule: {error}"))?;
        super::validate_run_options(&self.schedule, &self.replay_options)
            .map_err(|error| format!("invalid replay options: {error}"))?;
        let n = self.schedule.config.n_players;
        for (name, value) in [
            ("corrupt_state_from", self.replay_options.corrupt_state_from),
            (
                "corrupt_checksum_from",
                self.replay_options.corrupt_checksum_from,
            ),
        ] {
            if value.is_some_and(|(peer, _)| peer >= n) {
                return Err(format!("replay option {name} has a peer outside 0..{n}"));
            }
        }
        if self.schedule.config.spectator_hosts.is_empty()
            && (self.replay_options.corrupt_spectator_input_from.is_some()
                || self.replay_options.corrupt_spectator_status_from.is_some())
        {
            return Err(
                "spectator replay corruption requires configured spectator hosts".to_owned(),
            );
        }
        if self.final_confirmed.len() != n {
            return Err(format!(
                "artifact has {} final confirmed frames for {n} peers",
                self.final_confirmed.len()
            ));
        }
        for (name, len) in [
            ("frame_opportunities", self.frame_opportunities.len()),
            ("wait_frames_obeyed", self.wait_frames_obeyed.len()),
        ] {
            if len != n {
                return Err(format!("artifact has {len} {name} entries for {n} peers"));
            }
        }
        let expected_cpu_feedback_entries = if self.schedule.config.cpu_feedback_policy.is_some() {
            n
        } else {
            0
        };
        validate_cpu_feedback_entries(
            &self.cpu_feedback,
            expected_cpu_feedback_entries,
            &self.schedule,
            "artifact",
        )?;
        validate_receipt_range_evidence(
            self.receipt_range_probe.as_ref(),
            self.replay_options.receipt_range_probe_target,
            n,
            self.schedule.config.steps,
            "artifact",
        )?;
        validate_hostile_gossip_evidence(
            self.hostile_gossip.as_ref(),
            self.replay_options.hostile_gossip,
            self.replay_options.probe_confirmed_at,
            "artifact",
        )?;
        let expected_wait_policy_entries = if self.schedule.schema_version >= 17 {
            n
        } else {
            0
        };
        for (name, len) in [
            (
                "wait_recommendation_max",
                self.wait_recommendation_max.len(),
            ),
            (
                "wait_recommendation_frames",
                self.wait_recommendation_frames.len(),
            ),
            (
                "wait_recommendations_accepted",
                self.wait_recommendations_accepted.len(),
            ),
            ("wait_frames_accepted", self.wait_frames_accepted.len()),
        ] {
            if len != expected_wait_policy_entries {
                return Err(format!(
                    "artifact has {len} {name} entries (expected {expected_wait_policy_entries} \
                     for schedule schema {})",
                    self.schedule.schema_version
                ));
            }
        }
        let wait_policy_samples = self.replay_options.phase_resolved_control_samples
            && self.schedule.schema_version >= 17
            && !self.schedule.config.wait_recommendation_policy.is_default();
        let maximum_progress_samples = if wait_policy_samples {
            WAIT_POLICY_CONTROL_SAMPLE_CAPACITY
        } else if self.replay_options.phase_resolved_control_samples {
            phase_control_sample_capacity(n)
        } else {
            12
        };
        if self.progress_samples.len() > maximum_progress_samples {
            return Err(format!(
                "artifact has {} progress samples (maximum {maximum_progress_samples})",
                self.progress_samples.len(),
            ));
        }
        for sample in &self.progress_samples {
            for (name, len) in [
                ("current_frames", sample.current_frames.len()),
                ("confirmed_frames", sample.confirmed_frames.len()),
                ("confirmation_lag", sample.confirmation_lag.len()),
                ("wait_recommendations", sample.wait_recommendations.len()),
                ("rollback_count", sample.rollback_count.len()),
                ("resimulated_frames", sample.resimulated_frames.len()),
                ("prediction_miss_count", sample.prediction_miss_count.len()),
                ("frame_opportunities", sample.frame_opportunities.len()),
                ("wait_frames_obeyed", sample.wait_frames_obeyed.len()),
            ] {
                if len != n {
                    return Err(format!(
                        "artifact progress sample at step {} has {len} {name} entries for {n} peers",
                        sample.step
                    ));
                }
            }
            validate_cpu_feedback_entries(
                &sample.cpu_feedback,
                expected_cpu_feedback_entries,
                &self.schedule,
                &format!("artifact progress sample at step {}", sample.step),
            )?;
            let expected_controller_evidence = if wait_policy_samples { n } else { 0 };
            for (name, len) in [
                (
                    "wait_controller_evaluations",
                    sample.wait_controller_evaluations.len(),
                ),
                (
                    "wait_controller_trigger_evaluations",
                    sample.wait_controller_trigger_evaluations.len(),
                ),
                (
                    "wait_controller_endpoint_evaluations",
                    sample.wait_controller_endpoint_evaluations.len(),
                ),
                (
                    "wait_controller_endpoint_trigger_evaluations",
                    sample.wait_controller_endpoint_trigger_evaluations.len(),
                ),
                (
                    "wait_controller_input_mismatches",
                    sample.wait_controller_input_mismatches.len(),
                ),
            ] {
                if len != expected_controller_evidence {
                    return Err(format!(
                        "artifact progress sample at step {} has {len} {name} entries \
                         (expected {expected_controller_evidence})",
                        sample.step
                    ));
                }
            }
            let expected_frames_ahead = if self.replay_options.phase_resolved_control_samples {
                n
            } else {
                0
            };
            if sample.frames_ahead.len() != expected_frames_ahead {
                return Err(format!(
                    "artifact progress sample at step {} has {} frames_ahead entries \
                     (expected {expected_frames_ahead})",
                    sample.step,
                    sample.frames_ahead.len()
                ));
            }
            let expected_average_advantage = if wait_policy_samples { n } else { 0 };
            if sample.max_endpoint_average_frame_advantage.len() != expected_average_advantage {
                return Err(format!(
                    "artifact progress sample at step {} has {} \
                     max_endpoint_average_frame_advantage entries (expected \
                     {expected_average_advantage})",
                    sample.step,
                    sample.max_endpoint_average_frame_advantage.len()
                ));
            }
            if sample.step >= self.schedule.config.steps {
                return Err(format!(
                    "artifact progress sample step {} is outside embedded schedule",
                    sample.step
                ));
            }
            let directed_links = n.saturating_mul(n.saturating_sub(1));
            for (name, len, include_in_wait_policy_samples) in [
                ("endpoints", sample.endpoints.len(), true),
                ("link_queues", sample.link_queues.len(), false),
            ] {
                let expected = if self.schedule.schema_version >= 16
                    && (!self.replay_options.phase_resolved_control_samples
                        || (wait_policy_samples && include_in_wait_policy_samples))
                {
                    directed_links
                } else {
                    0
                };
                if len != expected {
                    return Err(format!(
                        "artifact progress sample at step {} has {len} {name} entries \
                         (expected {expected} for schedule schema {})",
                        sample.step, self.schedule.schema_version
                    ));
                }
            }
            for (index, endpoint) in sample.endpoints.iter().enumerate() {
                let expected_from = index / n.saturating_sub(1);
                let expected_offset = index % n.saturating_sub(1);
                let expected_to = if expected_offset >= expected_from {
                    expected_offset.saturating_add(1)
                } else {
                    expected_offset
                };
                if (endpoint.from, endpoint.to) != (expected_from, expected_to) {
                    return Err(format!(
                        "artifact progress endpoint {index} is {}->{} (expected \
                         {expected_from}->{expected_to})",
                        endpoint.from, endpoint.to
                    ));
                }
            }
            if wait_policy_samples {
                for (peer, endpoints) in sample
                    .endpoints
                    .chunks_exact(n.saturating_sub(1))
                    .enumerate()
                {
                    let expected_max = endpoints
                        .iter()
                        .map(|endpoint| endpoint.average_frame_advantage)
                        .max()
                        .unwrap_or(0);
                    if sample.max_endpoint_average_frame_advantage[peer] != expected_max {
                        return Err(format!(
                            "artifact progress sample at step {} has aggregate advantage {} for \
                             peer {peer} (expected endpoint maximum {expected_max})",
                            sample.step, sample.max_endpoint_average_frame_advantage[peer]
                        ));
                    }
                }
            }
            for (index, queue) in sample.link_queues.iter().enumerate() {
                let expected_from = index / n.saturating_sub(1);
                let expected_offset = index % n.saturating_sub(1);
                let expected_to = if expected_offset >= expected_from {
                    expected_offset.saturating_add(1)
                } else {
                    expected_offset
                };
                if (queue.from, queue.to) != (expected_from, expected_to) {
                    return Err(format!(
                        "artifact progress link queue {index} is {}->{} (expected \
                         {expected_from}->{expected_to})",
                        queue.from, queue.to
                    ));
                }
            }
        }
        let expected_tail_len = usize::try_from(self.schedule.config.steps)
            .unwrap_or(usize::MAX)
            .min(TRACE_TAIL_CAPACITY);
        if self.trace_tail.len() != expected_tail_len {
            return Err(format!(
                "artifact trace tail has {} entries, expected {expected_tail_len} for {} steps",
                self.trace_tail.len(),
                self.schedule.config.steps
            ));
        }
        let first_expected_step = self
            .schedule
            .config
            .steps
            .saturating_sub(u32::try_from(expected_tail_len).unwrap_or(u32::MAX));
        for (offset, snapshot) in self.trace_tail.iter().enumerate() {
            let expected_step =
                first_expected_step.saturating_add(u32::try_from(offset).unwrap_or(u32::MAX));
            if snapshot.step != expected_step {
                return Err(format!(
                    "artifact trace step {} is not contiguous (expected {expected_step})",
                    snapshot.step
                ));
            }
            if snapshot.step >= self.schedule.config.steps {
                return Err(format!(
                    "trace step {} is outside embedded schedule (0..{})",
                    snapshot.step, self.schedule.config.steps
                ));
            }
            for (name, len) in [
                ("confirmed_frames", snapshot.confirmed_frames.len()),
                ("session_states", snapshot.session_states.len()),
                ("dead", snapshot.dead.len()),
                ("game_states", snapshot.game_states.len()),
            ] {
                if len != n {
                    return Err(format!(
                        "trace step {} has {len} {name} entries for {n} peers",
                        snapshot.step
                    ));
                }
            }
            validate_cpu_feedback_entries(
                &snapshot.cpu_feedback,
                expected_cpu_feedback_entries,
                &self.schedule,
                &format!("trace step {}", snapshot.step),
            )?;
            validate_receipt_range_evidence(
                snapshot.receipt_range_probe.as_ref(),
                self.replay_options.receipt_range_probe_target,
                n,
                snapshot.step.saturating_add(1),
                &format!("trace step {}", snapshot.step),
            )?;
            if snapshot.scheduled_events.len() > TRACE_STEP_EVENT_CAPACITY
                || snapshot.observed_events.len() > TRACE_STEP_EVENT_CAPACITY
            {
                return Err(format!(
                    "trace step {} exceeds the per-step event summary cap {}",
                    snapshot.step, TRACE_STEP_EVENT_CAPACITY
                ));
            }
            if (snapshot.scheduled_events_truncated > 0
                && snapshot.scheduled_events.len() != TRACE_STEP_EVENT_CAPACITY)
                || (snapshot.observed_events_truncated > 0
                    && snapshot.observed_events.len() != TRACE_STEP_EVENT_CAPACITY)
            {
                return Err(format!(
                    "trace step {} reports truncated events before filling the per-step cap",
                    snapshot.step
                ));
            }
            if snapshot
                .scheduled_events
                .iter()
                .any(|text| text.len() > super::TRACE_EVENT_TEXT_CAPACITY)
                || snapshot.observed_events.iter().any(|event| {
                    event.kind.len() > super::TRACE_EVENT_TEXT_CAPACITY
                        || event.details.len() > super::TRACE_EVENT_TEXT_CAPACITY
                })
            {
                return Err(format!(
                    "trace step {} contains an overlong event summary",
                    snapshot.step
                ));
            }
            let spectator_expected = !self.schedule.config.spectator_hosts.is_empty();
            if snapshot.spectator.is_some() != spectator_expected {
                return Err(format!(
                    "trace step {} spectator presence does not match the schedule",
                    snapshot.step
                ));
            }
            if let Some(spectator) = snapshot.spectator {
                if spectator.num_hosts > self.schedule.config.spectator_hosts.len() {
                    return Err(format!(
                        "trace step {} spectator has {} hosts (configured {})",
                        snapshot.step,
                        spectator.num_hosts,
                        self.schedule.config.spectator_hosts.len()
                    ));
                }
                if spectator.applied_frames == 0 && spectator.max_applied_frame.is_some() {
                    return Err(format!(
                        "trace step {} spectator has a maximum frame without applied frames",
                        snapshot.step
                    ));
                }
                if spectator.max_applied_frame.is_some_and(|frame| frame < 0) {
                    return Err(format!(
                        "trace step {} spectator has a negative applied frame",
                        snapshot.step
                    ));
                }
            }
        }
        if self
            .trace_tail
            .last()
            .is_some_and(|snapshot| snapshot.cpu_feedback != self.cpu_feedback)
        {
            return Err("artifact final cpu_feedback differs from the final trace step".to_owned());
        }
        if self
            .trace_tail
            .last()
            .is_some_and(|snapshot| snapshot.receipt_range_probe != self.receipt_range_probe)
        {
            return Err(
                "artifact final receipt_range_probe differs from the final trace step".to_owned(),
            );
        }
        Ok(())
    }
}

fn bounded_failure_details(mut details: String) -> String {
    const SUFFIX: &str = "...<truncated>";
    if details.len() <= MAX_FAILURE_DETAIL_BYTES {
        return details;
    }
    let mut end = MAX_FAILURE_DETAIL_BYTES.saturating_sub(SUFFIX.len());
    while !details.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    details.truncate(end);
    details.push_str(SUFFIX);
    details
}

/// Makes an untrusted test name safe as one filesystem path component.
#[must_use]
pub fn sanitize_test_name(test_name: &str) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in test_name.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    let sanitized: String = test_name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect();
    let sanitized = sanitized.trim_matches('_');
    let base = if sanitized.is_empty() {
        "unnamed-test"
    } else {
        sanitized
    };
    let suffix = format!("-{hash:016x}");
    let max_base = MAX_TEST_NAME_COMPONENT_BYTES.saturating_sub(suffix.len());
    let mut base = base.chars().take(max_base).collect::<String>();
    base.push_str(&suffix);
    base
}

/// Existing-destination policy for atomic artifact publication.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExistingArtifact {
    Refuse,
    Replace,
}

fn write_unique_temporary(directory: &Path, seed: u64, bytes: &[u8]) -> Result<PathBuf, String> {
    for _ in 0..16 {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temporary = directory.join(format!(".{seed}.{}.{}.tmp", std::process::id(), sequence));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
        {
            Ok(mut file) => {
                if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
                    let _ = std::fs::remove_file(&temporary);
                    return Err(format!(
                        "failed to write temporary simulation artifact {}: {error}",
                        temporary.display()
                    ));
                }
                return Ok(temporary);
            },
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {},
            Err(error) => {
                return Err(format!(
                    "failed to create temporary simulation artifact {}: {error}",
                    temporary.display()
                ));
            },
        }
    }
    Err("failed to allocate a unique temporary artifact name after 16 attempts".to_owned())
}

/// Writes an artifact below an explicit root and returns its final path.
pub fn write_artifact(
    root: &Path,
    test_name: &str,
    artifact: &FailureArtifact,
    existing: ExistingArtifact,
) -> Result<PathBuf, String> {
    artifact.validate()?;
    let directory = root.join(sanitize_test_name(test_name));
    std::fs::create_dir_all(&directory).map_err(|error| {
        format!(
            "failed to create simulation artifact directory {}: {error}",
            directory.display()
        )
    })?;
    let seed = artifact.schedule.seed;
    let path = directory.join(format!("{seed}.json"));
    let bytes = serde_json::to_vec_pretty(artifact)
        .map_err(|error| format!("failed to serialize simulation artifact: {error}"))?;
    if bytes.len() > MAX_FAILURE_ARTIFACT_BYTES {
        return Err(format!(
            "serialized failure artifact has {} bytes (max {MAX_FAILURE_ARTIFACT_BYTES})",
            bytes.len()
        ));
    }
    let temporary = write_unique_temporary(&directory, seed, &bytes)?;
    let publish = match existing {
        ExistingArtifact::Refuse => std::fs::hard_link(&temporary, &path),
        ExistingArtifact::Replace => std::fs::rename(&temporary, &path),
    };
    if let Err(error) = publish {
        let _ = std::fs::remove_file(&temporary);
        return Err(format!(
            "failed to atomically publish simulation artifact {}: {error}",
            path.display()
        ));
    }
    if existing == ExistingArtifact::Refuse {
        let _ = std::fs::remove_file(&temporary);
    }
    Ok(path)
}

/// Reads and validates a failure artifact before replay.
pub fn read_artifact(path: &Path) -> Result<FailureArtifact, String> {
    let file = std::fs::File::open(path).map_err(|error| {
        format!(
            "failed to read simulation artifact {}: {error}",
            path.display()
        )
    })?;
    let limit = u64::try_from(MAX_FAILURE_ARTIFACT_BYTES)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let mut bytes = Vec::new();
    file.take(limit).read_to_end(&mut bytes).map_err(|error| {
        format!(
            "failed to read simulation artifact {}: {error}",
            path.display()
        )
    })?;
    if bytes.len() > MAX_FAILURE_ARTIFACT_BYTES {
        return Err(format!(
            "simulation artifact {} exceeds {MAX_FAILURE_ARTIFACT_BYTES} bytes",
            path.display()
        ));
    }
    let artifact: FailureArtifact = serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "failed to decode simulation artifact {}: {error}",
            path.display()
        )
    })?;
    artifact.validate()?;
    Ok(artifact)
}

/// Writes a report to the repository-local default artifact directory.
pub fn write_report_artifact(
    test_name: &str,
    schedule: &Schedule,
    report: &RunReport,
) -> Result<PathBuf, String> {
    write_artifact(
        Path::new("target/sim-artifacts"),
        test_name,
        &FailureArtifact::from_report(schedule, report),
        ExistingArtifact::Replace,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::sim_net::LinkPolicy;
    use crate::simulation::harness::schedule::{
        AppModel, BackgroundNoise, FrameModel, ScheduleEvent, SimConfig, WaitRecommendationPolicy,
        SCHEDULE_SCHEMA_VERSION,
    };
    use crate::simulation::harness::{
        run, EndpointControlSample, HostileGossipMode, HostileGossipOptions, LinkQueueSample,
        ProgressSample, RunOptions, TraceGameState, TraceNetStats, TraceSessionState,
        TRACE_TAIL_CAPACITY,
    };

    fn temp_artifact_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "fortress-sim-artifact-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock after epoch")
                .as_nanos()
        ))
    }

    fn sample_artifact() -> FailureArtifact {
        let schedule = crate::simulation::harness::schedule::generate(
            17,
            crate::simulation::harness::schedule::SimConfig::smoke(2),
        );
        FailureArtifact {
            artifact_schema_version: FAILURE_ARTIFACT_SCHEMA_VERSION,
            schedule,
            replay_options: crate::simulation::harness::RunOptions::default(),
            replay_input_width_bytes: 4,
            failures: vec![ArtifactFailure {
                class: FailureClass::StateDivergence,
                details: "StateDivergence { frame: 7 }".to_owned(),
            }],
            trace_hash: 99,
            final_confirmed: vec![10, 10],
            progress_samples: Vec::new(),
            frame_opportunities: vec![600, 600],
            wait_frames_obeyed: vec![0, 0],
            wait_recommendation_max: vec![0, 0],
            wait_recommendation_frames: vec![0, 0],
            wait_recommendations_accepted: vec![0, 0],
            wait_frames_accepted: vec![0, 0],
            cpu_feedback: Vec::new(),
            receipt_range_probe: None,
            hostile_gossip: None,
            trace_tail: (0..TRACE_TAIL_CAPACITY)
                .map(|step| TraceSnapshot {
                    step: 536 + u32::try_from(step).expect("small trace step"),
                    confirmed_frames: vec![10, 10],
                    session_states: vec![TraceSessionState::Running; 2],
                    dead: vec![false; 2],
                    game_states: vec![
                        TraceGameState {
                            frame: 11,
                            value: 7
                        };
                        2
                    ],
                    cpu_feedback: Vec::new(),
                    receipt_range_probe: None,
                    scheduled_events: Vec::new(),
                    scheduled_events_truncated: 0,
                    observed_events: Vec::new(),
                    observed_events_truncated: 0,
                    net: TraceNetStats {
                        sent: 1,
                        delivered: 1,
                        dropped_by_policy: 0,
                        retransmit_delayed: 0,
                        dropped_blocked: 0,
                        dropped_unattached: 0,
                        duplicated: 0,
                        held: 0,
                        ..TraceNetStats::default()
                    },
                    spectator: None,
                })
                .collect(),
        }
    }

    #[test]
    fn artifact_schema_round_trips_and_writer_sanitizes_test_path() {
        let artifact = sample_artifact();
        let root = temp_artifact_root("roundtrip");
        let path = write_artifact(
            &root,
            "../../bad/test:name",
            &artifact,
            ExistingArtifact::Refuse,
        )
        .expect("artifact write succeeds");

        let expected_directory = root.join(sanitize_test_name("../../bad/test:name"));
        assert_eq!(path.parent(), Some(expected_directory.as_path()));
        assert!(
            path.starts_with(&root),
            "sanitized path escaped root: {path:?}"
        );
        let decoded = read_artifact(&path).expect("artifact schema round trips");
        assert_eq!(decoded, artifact);
        assert_eq!(
            decoded.artifact_schema_version,
            FAILURE_ARTIFACT_SCHEMA_VERSION
        );
        assert_eq!(decoded.trace_tail.len(), TRACE_TAIL_CAPACITY);

        std::fs::remove_dir_all(&root).expect("temporary artifact tree removes");
    }

    #[test]
    fn sanitized_test_names_are_bounded_stable_and_collision_resistant() {
        let first = sanitize_test_name("a/b");
        let second = sanitize_test_name("a:b");
        assert_ne!(
            first, second,
            "distinct originals must not alias after sanitizing"
        );
        assert_eq!(first, sanitize_test_name("a/b"));
        assert!(first.len() <= MAX_TEST_NAME_COMPONENT_BYTES);
        assert!(sanitize_test_name(&"x".repeat(10_000)).len() <= MAX_TEST_NAME_COMPONENT_BYTES);
    }

    #[test]
    fn artifact_writer_refuses_or_replaces_existing_destination_explicitly() {
        let root = temp_artifact_root("replace");
        let artifact = sample_artifact();
        let path = write_artifact(&root, "repeat", &artifact, ExistingArtifact::Refuse)
            .expect("first publication succeeds");
        assert!(write_artifact(&root, "repeat", &artifact, ExistingArtifact::Refuse).is_err());

        let mut replacement = artifact;
        replacement.trace_hash = 1234;
        write_artifact(&root, "repeat", &replacement, ExistingArtifact::Replace)
            .expect("explicit replacement succeeds");
        assert_eq!(
            read_artifact(&path).expect("replacement reads").trace_hash,
            1234
        );
        std::fs::remove_dir_all(root).expect("temporary artifact tree removes");
    }

    #[test]
    #[allow(clippy::needless_collect)] // all writers must start before any join
    fn parallel_refuse_writers_publish_exactly_once_without_temp_leaks() {
        let root = temp_artifact_root("parallel");
        let artifact = sample_artifact();
        let successes = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..8)
                .map(|_| {
                    let root = &root;
                    let artifact = &artifact;
                    scope.spawn(move || {
                        write_artifact(root, "parallel", artifact, ExistingArtifact::Refuse).is_ok()
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().expect("writer thread joins"))
                .filter(|success| *success)
                .count()
        });
        assert_eq!(successes, 1);
        let directory = root.join(sanitize_test_name("parallel"));
        let entries: Vec<_> = std::fs::read_dir(&directory)
            .expect("artifact directory reads")
            .collect::<Result<_, _>>()
            .expect("artifact entries read");
        assert_eq!(entries.len(), 1, "temporary files leaked: {entries:?}");
        std::fs::remove_dir_all(root).expect("temporary artifact tree removes");
    }

    #[test]
    fn artifact_reader_rejects_oversized_input_before_deserialization() {
        let root = temp_artifact_root("oversized");
        std::fs::create_dir_all(&root).expect("temporary root creates");
        let path = root.join("oversized.json");
        let file = std::fs::File::create(&path).expect("oversized file creates");
        file.set_len(u64::try_from(MAX_FAILURE_ARTIFACT_BYTES + 1).expect("size fits u64"))
            .expect("oversized file extends");
        let error = read_artifact(&path).expect_err("oversized artifact must fail");
        assert!(error.contains("exceeds"), "wrong error: {error}");
        std::fs::remove_dir_all(root).expect("temporary artifact tree removes");
    }

    #[test]
    fn artifact_validation_accepts_supported_legacy_and_rejects_future_schemas() {
        let mut artifact = sample_artifact();
        artifact.artifact_schema_version += 1;
        assert!(artifact.validate().is_err());

        let mut artifact = sample_artifact();
        artifact.schedule.schema_version = 1;
        artifact.wait_recommendation_max.clear();
        artifact.wait_recommendation_frames.clear();
        artifact.wait_recommendations_accepted.clear();
        artifact.wait_frames_accepted.clear();
        assert!(
            artifact.validate().is_ok(),
            "artifact envelope must preserve the schedule validator's compatibility window"
        );

        let mut artifact = sample_artifact();
        artifact.schedule.schema_version = SCHEDULE_SCHEMA_VERSION + 1;
        assert!(artifact.validate().is_err());
    }

    #[test]
    fn legacy_schedule_failure_builds_a_valid_v7_artifact() {
        let mut config = SimConfig::smoke(2);
        config.steps = 60;
        config.noise = BackgroundNoise::Clean;
        let mut schedule = crate::simulation::harness::schedule::generate(0x001E_6AC7, config);
        schedule.schema_version = 16;
        let report = run(
            &schedule,
            &RunOptions {
                corrupt_state_from: Some((1, 8)),
                ..RunOptions::default()
            },
        );
        assert!(!report.verdict.passed(), "negative control must fail");
        assert!(report.wait_recommendation_max.is_empty());
        assert!(report.wait_recommendation_frames.is_empty());
        assert!(report.wait_recommendations_accepted.is_empty());
        assert!(report.wait_frames_accepted.is_empty());

        let artifact = FailureArtifact::from_report(&schedule, &report);
        artifact
            .validate()
            .expect("legacy schedule report produces a valid v7 artifact");
    }

    #[test]
    fn artifact_v6_without_hostile_gossip_evidence_reaches_explicit_schema_rejection() {
        let mut value = serde_json::to_value(sample_artifact()).expect("artifact serializes");
        let object = value
            .as_object_mut()
            .expect("failure artifact serializes as an object");
        object.insert(
            "artifact_schema_version".to_owned(),
            serde_json::Value::from(6),
        );
        assert!(
            object.remove("hostile_gossip").is_some(),
            "fixture contains hostile_gossip"
        );

        let artifact: FailureArtifact =
            serde_json::from_value(value).expect("v6 envelope reaches validation");
        let error = artifact.validate().expect_err("v6 schema is unsupported");
        assert!(
            error.contains("unsupported failure artifact schema 6"),
            "wrong diagnostic: {error}"
        );
    }

    #[test]
    fn hostile_gossip_artifact_preserves_receiver_side_evidence() {
        let mut config = SimConfig::smoke(4);
        config.steps = 320;
        config.noise = BackgroundNoise::Clean;
        let schedule = crate::simulation::harness::schedule::generate(0xB2A4, config);
        let options = RunOptions {
            corrupt_state_from: Some((0, 220)),
            probe_confirmed_at: Some(180),
            hostile_gossip: Some(HostileGossipOptions {
                liar: 1,
                observer: 3,
                target: 2,
                delta: -9,
                from_step: 100,
                until_step: 181,
                mode: HostileGossipMode::ConnectionStatus,
            }),
            ..RunOptions::default()
        };
        let report = run(&schedule, &options);
        assert!(!report.verdict.passed(), "negative control must fail");
        let evidence = report.hostile_gossip.expect("hostile evidence");
        assert!(evidence.messages_mutated > 0);
        assert!(evidence.probe.is_some());

        let artifact = FailureArtifact::from_report(&schedule, &report);
        artifact.validate().expect("hostile artifact is valid");
        let encoded = serde_json::to_vec(&artifact).expect("artifact serializes");
        let decoded: FailureArtifact = serde_json::from_slice(&encoded).expect("artifact decodes");
        decoded
            .validate()
            .expect("decoded hostile artifact is valid");
        assert_eq!(decoded.hostile_gossip, Some(evidence));
        assert_eq!(decoded.replay_options, options);

        let mut tampered = decoded;
        tampered
            .hostile_gossip
            .as_mut()
            .and_then(|evidence| evidence.probe.as_mut())
            .expect("hostile-gossip probe")
            .observer_confirmed = fortress_rollback::Frame::NULL.as_i32() - 1;
        assert!(
            tampered.validate().is_err(),
            "artifact validation must reject an out-of-domain confirmed frame"
        );
    }

    #[test]
    fn schema_v18_cpu_feedback_artifact_preserves_and_validates_exact_evidence() {
        let mut config = SimConfig::smoke(2);
        config.steps = 80;
        config.noise = BackgroundNoise::Clean;
        config.cpu_feedback_policy =
            Some(crate::simulation::harness::schedule::CpuFeedbackPolicy {
                simulated_frame_cost_us: 8_001,
                max_poll_delay_steps: 8,
            });
        let schedule = crate::simulation::harness::schedule::generate(0xC0FE, config);
        let report = run(
            &schedule,
            &RunOptions {
                corrupt_state_from: Some((1, 8)),
                ..RunOptions::default()
            },
        );
        assert!(!report.verdict.passed(), "negative control must fail");
        assert_eq!(report.cpu_feedback.len(), 2);
        for (peer, evidence) in report.cpu_feedback.iter().enumerate() {
            assert_eq!(
                evidence.charged_frames,
                report.metrics[peer].frames_advanced
            );
            assert_eq!(evidence.work_us, evidence.charged_frames * 8_001);
        }

        let artifact = FailureArtifact::from_report(&schedule, &report);
        artifact.validate().expect("CPU artifact validates");

        let mut missing = artifact.clone();
        let _ = missing.cpu_feedback.pop();
        assert!(missing.validate().is_err());
        let mut wrong_work = artifact.clone();
        wrong_work.cpu_feedback[0].work_us += 1;
        assert!(wrong_work.validate().is_err());
        let mut wrong_progress = artifact.clone();
        let _ = wrong_progress.progress_samples[0].cpu_feedback.pop();
        assert!(wrong_progress.validate().is_err());
        let mut wrong_trace = artifact.clone();
        wrong_trace.trace_tail[0].cpu_feedback[0].max_delay_streak = 9;
        assert!(wrong_trace.validate().is_err());
        let mut wrong_final = artifact;
        wrong_final.cpu_feedback[0].charged_frames -= 1;
        wrong_final.cpu_feedback[0].work_us -= 8_001;
        assert!(wrong_final.validate().is_err());
    }

    #[test]
    fn receipt_range_artifact_preserves_and_validates_high_water_evidence() {
        let mut config = SimConfig::smoke(3);
        config.steps = 180;
        config.noise = BackgroundNoise::Clean;
        let schedule = crate::simulation::harness::schedule::generate(0x5241_4e47, config);
        let report = run(
            &schedule,
            &RunOptions {
                corrupt_state_from: Some((1, 8)),
                receipt_range_probe_target: Some(2),
                ..RunOptions::default()
            },
        );
        assert!(!report.verdict.passed(), "negative control must fail");
        assert!(report.receipt_range_probe.is_some());

        let artifact = FailureArtifact::from_report(&schedule, &report);
        artifact
            .validate()
            .expect("receipt-range artifact validates");

        let mut missing_entry = artifact.clone();
        let _ = missing_entry
            .receipt_range_probe
            .as_mut()
            .expect("probe exists")
            .receipts
            .pop();
        assert!(missing_entry.validate().is_err());

        let mut wrong_spread = artifact.clone();
        wrong_spread
            .receipt_range_probe
            .as_mut()
            .expect("probe exists")
            .max_spread += 1;
        assert!(wrong_spread.validate().is_err());

        let mut wrong_trace = artifact.clone();
        wrong_trace.trace_tail[0]
            .receipt_range_probe
            .as_mut()
            .expect("trace probe exists")
            .target = 1;
        assert!(wrong_trace.validate().is_err());

        let mut wrong_final = artifact;
        wrong_final
            .receipt_range_probe
            .as_mut()
            .expect("probe exists")
            .receipts[0] = None;
        assert!(wrong_final.validate().is_err());
    }

    #[test]
    fn receipt_range_validation_allows_incomplete_and_disconnected_nonparticipants() {
        let incomplete = ReceiptRangeEvidence {
            target: 3,
            max_spread: 10,
            at_step: Some(5),
            receipts: vec![None, Some(10), Some(0), None],
            connected: vec![Some(true), Some(true), Some(true), None],
            retained_ranges: vec![
                None,
                Some(RetainedInputRangeEvidence { first: 0, last: 10 }),
                Some(RetainedInputRangeEvidence { first: 0, last: 0 }),
                None,
            ],
        };
        validate_receipt_range_evidence(Some(&incomplete), Some(3), 4, 10, "test")
            .expect("an incomplete connected observer does not define the extrema");

        let mut disconnected = incomplete;
        disconnected.connected[0] = Some(false);
        disconnected.receipts[0] = Some(4);
        disconnected.retained_ranges[0] = Some(RetainedInputRangeEvidence { first: 0, last: 9 });
        validate_receipt_range_evidence(Some(&disconnected), Some(3), 4, 10, "test")
            .expect("a disconnected freeze may sit below retained history's physical end");

        let mut future_step = disconnected.clone();
        future_step.at_step = Some(10);
        assert!(
            validate_receipt_range_evidence(Some(&future_step), Some(3), 4, 10, "test").is_err()
        );

        let mut self_observation = disconnected;
        self_observation.receipts[3] = Some(0);
        assert!(
            validate_receipt_range_evidence(Some(&self_observation), Some(3), 4, 10, "test")
                .is_err()
        );
    }

    #[test]
    fn progress_control_samples_validate_order_and_preserve_legacy_defaults() {
        let sample = ProgressSample {
            step: 1,
            current_frames: vec![1, 1],
            confirmed_frames: vec![0, 0],
            confirmation_lag: vec![1, 1],
            wait_recommendations: vec![0, 0],
            rollback_count: vec![0, 0],
            resimulated_frames: vec![0, 0],
            prediction_miss_count: vec![0, 0],
            frame_opportunities: vec![1, 1],
            wait_frames_obeyed: vec![0, 0],
            cpu_feedback: Vec::new(),
            wait_controller_evaluations: Vec::new(),
            wait_controller_trigger_evaluations: Vec::new(),
            wait_controller_endpoint_evaluations: Vec::new(),
            wait_controller_endpoint_trigger_evaluations: Vec::new(),
            wait_controller_input_mismatches: Vec::new(),
            frames_ahead: Vec::new(),
            max_endpoint_average_frame_advantage: Vec::new(),
            endpoints: vec![
                EndpointControlSample {
                    from: 0,
                    to: 1,
                    ping_ms: 10,
                    remote_frame_advantage: 1,
                    average_frame_advantage: 0,
                    pending_output_len: 2,
                },
                EndpointControlSample {
                    from: 1,
                    to: 0,
                    ping_ms: 10,
                    remote_frame_advantage: -1,
                    average_frame_advantage: 0,
                    pending_output_len: 2,
                },
            ],
            link_queues: vec![
                LinkQueueSample {
                    from: 0,
                    to: 1,
                    queued_bytes: 100,
                    queued_datagrams: 1,
                    drain_delay_ns: 1_000,
                },
                LinkQueueSample {
                    from: 1,
                    to: 0,
                    queued_bytes: 0,
                    queued_datagrams: 0,
                    drain_delay_ns: 0,
                },
            ],
        };
        let mut artifact = sample_artifact();
        artifact.progress_samples.push(sample.clone());
        assert!(artifact.validate().is_ok());

        let mut incomplete = artifact.clone();
        let _ = incomplete.progress_samples[0].endpoints.pop();
        assert!(incomplete.validate().is_err());
        let mut stripped_endpoints = artifact.clone();
        stripped_endpoints.progress_samples[0].endpoints.clear();
        assert!(stripped_endpoints.validate().is_err());
        let mut stripped_queues = artifact.clone();
        stripped_queues.progress_samples[0].link_queues.clear();
        assert!(stripped_queues.validate().is_err());
        let mut misordered = artifact.clone();
        misordered.progress_samples[0].link_queues.swap(0, 1);
        assert!(misordered.validate().is_err());

        let mut phase_resolved = artifact.clone();
        phase_resolved.replay_options.phase_resolved_control_samples = true;
        phase_resolved.schedule.config.frame_model =
            super::super::schedule::FrameModel::SkewGated60Hz;
        phase_resolved.progress_samples[0].frames_ahead = vec![2, -2];
        phase_resolved.progress_samples[0].endpoints.clear();
        phase_resolved.progress_samples[0].link_queues.clear();
        assert!(phase_resolved.validate().is_ok());
        let mut over_capacity = phase_resolved.clone();
        over_capacity.progress_samples.resize(
            phase_control_sample_capacity(2) + 1,
            phase_resolved.progress_samples[0].clone(),
        );
        assert!(over_capacity.validate().is_err());
        let mut missing_phase_signal = phase_resolved;
        missing_phase_signal.progress_samples[0]
            .frames_ahead
            .clear();
        assert!(missing_phase_signal.validate().is_err());

        let mut value = serde_json::to_value(sample).expect("progress sample serializes");
        let object = value
            .as_object_mut()
            .expect("progress sample serializes as an object");
        assert!(object.remove("endpoints").is_some());
        assert!(object.remove("link_queues").is_some());
        let legacy: ProgressSample =
            serde_json::from_value(value).expect("legacy progress sample uses defaults");
        assert!(legacy.endpoints.is_empty());
        assert!(legacy.link_queues.is_empty());
    }

    #[test]
    fn schema_v18_h_osc_cpu_receipt_artifact_preserves_full_n16_evidence_under_size_cap() {
        let n = 16;
        let mut config = SimConfig::smoke(n);
        config.steps = 3_000;
        config.noise = BackgroundNoise::Clean;
        config.app_model = AppModel::Obey;
        config.frame_model = FrameModel::SkewGated60Hz;
        config.wait_recommendation_policy = WaitRecommendationPolicy {
            cooldown_frames: 60,
            max_skip_frames: Some(9),
            response_delay_frames: 30,
            smear_interval: 4,
        };
        config.cpu_feedback_policy =
            Some(crate::simulation::harness::schedule::CpuFeedbackPolicy {
                simulated_frame_cost_us: 1,
                max_poll_delay_steps: 3_000,
            });
        let schedule = crate::simulation::harness::schedule::generate(0xA10, config);
        let endpoints: Vec<_> = (0..n)
            .flat_map(|from| {
                (0..n)
                    .filter(move |&to| from != to)
                    .map(move |to| EndpointControlSample {
                        from,
                        to,
                        ping_ms: u128::MAX,
                        remote_frame_advantage: i32::MAX,
                        average_frame_advantage: i32::MIN,
                        pending_output_len: u64::MAX,
                    })
            })
            .collect();
        let maximum_cpu_evidence = CpuFeedbackEvidence {
            charged_frames: u64::MAX,
            work_us: u64::MAX,
            delayed_poll_steps: u64::MAX,
            current_delay_streak: 3_000,
            max_delay_streak: 3_000,
            remaining_delay_steps: 3_000,
            clamp_count: u64::MAX,
            clamped_delay_steps: u64::MAX,
        };
        let progress_samples: Vec<_> = (0..100_u32)
            .map(|index| ProgressSample {
                step: index.saturating_mul(30).saturating_add(29),
                current_frames: vec![i32::MAX; n],
                confirmed_frames: vec![i32::MAX; n],
                confirmation_lag: vec![u64::MAX; n],
                wait_recommendations: vec![u64::MAX; n],
                rollback_count: vec![u64::MAX; n],
                resimulated_frames: vec![u64::MAX; n],
                prediction_miss_count: vec![u64::MAX; n],
                frame_opportunities: vec![u64::MAX; n],
                wait_frames_obeyed: vec![u64::MAX; n],
                cpu_feedback: vec![maximum_cpu_evidence; n],
                wait_controller_evaluations: vec![u64::MAX; n],
                wait_controller_trigger_evaluations: vec![u64::MAX; n],
                wait_controller_endpoint_evaluations: vec![u64::MAX; n],
                wait_controller_endpoint_trigger_evaluations: vec![u64::MAX; n],
                wait_controller_input_mismatches: vec![0; n],
                frames_ahead: vec![i32::MAX; n],
                max_endpoint_average_frame_advantage: vec![i32::MIN; n],
                endpoints: endpoints.clone(),
                link_queues: Vec::new(),
            })
            .collect();
        let mut artifact = sample_artifact();
        artifact.schedule = schedule;
        artifact.replay_options.phase_resolved_control_samples = true;
        artifact.replay_options.receipt_range_probe_target = Some(n - 1);
        artifact.final_confirmed = vec![0; n];
        artifact.progress_samples = progress_samples;
        artifact.frame_opportunities = vec![0; n];
        artifact.wait_frames_obeyed = vec![0; n];
        artifact.wait_recommendation_max = vec![0; n];
        artifact.wait_recommendation_frames = vec![0; n];
        artifact.wait_recommendations_accepted = vec![0; n];
        artifact.wait_frames_accepted = vec![0; n];
        artifact.cpu_feedback = vec![maximum_cpu_evidence; n];
        let mut maximum_receipts = vec![Some(i32::MAX); n];
        maximum_receipts[0] = Some(0);
        maximum_receipts[n - 1] = None;
        let mut maximum_connections = vec![Some(true); n];
        maximum_connections[n - 1] = None;
        let mut maximum_ranges = maximum_receipts
            .iter()
            .map(|receipt| receipt.map(|last| RetainedInputRangeEvidence { first: 0, last }))
            .collect::<Vec<_>>();
        maximum_ranges[n - 1] = None;
        let maximum_receipt_evidence = ReceiptRangeEvidence {
            target: n - 1,
            max_spread: i32::MAX as u32,
            at_step: Some(2_935),
            receipts: maximum_receipts,
            connected: maximum_connections,
            retained_ranges: maximum_ranges,
        };
        artifact.receipt_range_probe = Some(maximum_receipt_evidence.clone());
        for (offset, snapshot) in artifact.trace_tail.iter_mut().enumerate() {
            snapshot.step = 2_936 + u32::try_from(offset).expect("tail offset fits u32");
            snapshot.confirmed_frames = vec![0; n];
            snapshot.session_states = vec![TraceSessionState::Running; n];
            snapshot.dead = vec![false; n];
            snapshot.game_states = vec![TraceGameState { frame: 0, value: 0 }; n];
            snapshot.cpu_feedback = vec![maximum_cpu_evidence; n];
            snapshot.receipt_range_probe = Some(maximum_receipt_evidence.clone());
        }
        artifact
            .validate()
            .expect("schema-v18 H-OSC/CPU shape validates");
        assert_eq!(
            artifact.progress_samples.len(),
            WAIT_POLICY_CONTROL_SAMPLE_CAPACITY
        );
        let mut over_capacity = artifact.clone();
        over_capacity.progress_samples.push(
            artifact
                .progress_samples
                .last()
                .expect("maximum shape has samples")
                .clone(),
        );
        assert!(
            over_capacity.validate().is_err(),
            "validator must reject an H-OSC artifact above the bounded sample cap"
        );

        let serialized = serde_json::to_vec_pretty(&artifact).expect("artifact serializes");
        assert!(
            serialized.len() <= MAX_FAILURE_ARTIFACT_BYTES,
            "N=16 H-OSC/CPU artifact uses {} bytes (cap {MAX_FAILURE_ARTIFACT_BYTES})",
            serialized.len()
        );
        let root = temp_artifact_root("h-osc-cpu-n16");
        let path = write_artifact(&root, "h-osc-cpu-n16", &artifact, ExistingArtifact::Refuse)
            .expect("bounded H-OSC/CPU artifact writes");
        assert_eq!(read_artifact(&path).expect("artifact reads"), artifact);
        std::fs::remove_dir_all(root).expect("temporary artifact tree removes");
    }

    #[test]
    fn artifact_validation_rejects_incomplete_or_inconsistent_envelopes() {
        let mutations: Vec<Box<dyn Fn(&mut FailureArtifact)>> = vec![
            Box::new(|artifact| artifact.failures.clear()),
            Box::new(|artifact| {
                let _ = artifact.final_confirmed.pop();
            }),
            Box::new(|artifact| {
                let _ = artifact.frame_opportunities.pop();
            }),
            Box::new(|artifact| {
                let _ = artifact.wait_frames_obeyed.pop();
            }),
            Box::new(|artifact| {
                let _ = artifact.wait_recommendation_max.pop();
            }),
            Box::new(|artifact| {
                let _ = artifact.wait_recommendation_frames.pop();
            }),
            Box::new(|artifact| {
                let _ = artifact.wait_recommendations_accepted.pop();
            }),
            Box::new(|artifact| {
                let _ = artifact.wait_frames_accepted.pop();
            }),
            Box::new(|artifact| artifact.trace_tail.clear()),
            Box::new(|artifact| artifact.trace_tail[1].step += 1),
            Box::new(|artifact| {
                let _ = artifact.trace_tail[0].dead.pop();
            }),
            Box::new(|artifact| {
                artifact.trace_tail[0].scheduled_events_truncated = 1;
            }),
            Box::new(|artifact| {
                artifact.trace_tail[0].scheduled_events =
                    vec!["x".repeat(super::super::TRACE_EVENT_TEXT_CAPACITY + 1)];
            }),
            Box::new(|artifact| {
                artifact.trace_tail[0].spectator = Some(super::super::TraceSpectatorState {
                    current_frame: 1,
                    num_hosts: 1,
                    applied_frames: 1,
                    max_applied_frame: Some(0),
                });
            }),
            Box::new(|artifact| {
                artifact.replay_options.corrupt_state_from = Some((usize::MAX, 0));
            }),
            Box::new(|artifact| {
                artifact.replay_options.probe_confirmed_at = Some(artifact.schedule.config.steps);
            }),
            Box::new(|artifact| {
                artifact.replay_options.pending_output_probe_link = Some((0, 0));
            }),
            Box::new(|artifact| {
                artifact.replay_options.pending_output_probe_link = Some((0, 1));
                artifact.schedule.events.push((
                    1,
                    super::super::schedule::ScheduleEvent::PeerKill { peer: 1 },
                ));
                artifact.schedule.events.sort_by_key(|(step, _)| *step);
            }),
            Box::new(|artifact| {
                artifact.replay_options.corrupt_spectator_input_from = Some(0);
            }),
        ];
        for mutate in mutations {
            let mut artifact = sample_artifact();
            mutate(&mut artifact);
            assert!(artifact.validate().is_err(), "accepted: {artifact:#?}");
        }
    }

    #[test]
    fn artifact_decode_rejects_unknown_owned_dto_fields() {
        let mut value = serde_json::to_value(sample_artifact()).expect("artifact serializes");
        value
            .as_object_mut()
            .expect("artifact is an object")
            .insert("future_field".to_owned(), serde_json::Value::Bool(true));
        assert!(serde_json::from_value::<FailureArtifact>(value).is_err());
    }

    #[test]
    fn expect_pass_writes_artifact_before_panicking() {
        let mut config = SimConfig::smoke(2);
        config.steps = 60;
        config.noise = BackgroundNoise::Clean;
        let schedule = crate::simulation::harness::schedule::generate(0xA471_FAC7, config);
        let report = run(
            &schedule,
            &RunOptions {
                corrupt_state_from: Some((1, 8)),
                ..RunOptions::default()
            },
        );
        assert!(!report.verdict.passed(), "negative control must fail");

        let test_name = std::thread::current()
            .name()
            .expect("libtest thread has a name")
            .to_owned();
        let path = Path::new("target/sim-artifacts")
            .join(sanitize_test_name(&test_name))
            .join(format!("{}.json", schedule.seed));
        let _ = std::fs::remove_file(&path);
        let panic = std::panic::catch_unwind(|| report.expect_pass(&schedule))
            .expect_err("expect_pass must panic for a failed verdict");
        let panic_text = panic
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| panic.downcast_ref::<&str>().map(|text| (*text).to_owned()))
            .expect("panic payload is text");

        assert!(
            panic_text.contains("artifact="),
            "panic omitted artifact status: {panic_text}"
        );
        let decoded = read_artifact(&path).expect("artifact exists before panic returns");
        assert!(decoded
            .failures
            .iter()
            .any(|failure| failure.class == FailureClass::StateDivergence));
        std::fs::remove_file(&path).expect("generated failure artifact removes");
        if let Some(parent) = path.parent() {
            let _ = std::fs::remove_dir(parent);
        }
    }

    #[test]
    fn run_report_retains_only_final_sixty_four_contiguous_steps() {
        let mut config = SimConfig::smoke(2);
        config.steps = 80;
        config.noise = BackgroundNoise::Clean;
        let mut schedule = crate::simulation::harness::schedule::generate(23, config);
        schedule.events = vec![
            (
                20,
                ScheduleEvent::Block {
                    from: 0,
                    to: 1,
                    blocked: true,
                },
            ),
            (
                21,
                ScheduleEvent::Block {
                    from: 0,
                    to: 1,
                    blocked: false,
                },
            ),
            (40, ScheduleEvent::HealAll),
        ];
        schedule.heal_at = 40;
        schedule.initial_links = vec![(0, 1, LinkPolicy::clean()), (1, 0, LinkPolicy::clean())];

        let report = run(&schedule, &RunOptions::default());
        assert_eq!(report.trace_tail.len(), TRACE_TAIL_CAPACITY);
        assert_eq!(report.trace_tail.first().map(|entry| entry.step), Some(16));
        assert_eq!(report.trace_tail.last().map(|entry| entry.step), Some(79));
        let fault = report
            .trace_tail
            .iter()
            .find(|entry| entry.step == 20)
            .expect("fault step stays in trace tail");
        assert_eq!(fault.scheduled_events.len(), 1);
        assert!(fault.scheduled_events[0].contains("Block"));
        assert_eq!(fault.game_states.len(), 2);
        assert!(
            fault.net.dropped_blocked > 0,
            "same-step network counters must expose the scheduled fault's effect: {fault:?}"
        );
    }
}
