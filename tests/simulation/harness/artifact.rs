//! Stable JSON failure artifacts for deterministic simulation runs.

use super::oracle::OracleFailure;
use super::schedule::{validate_schedule, Schedule};
use super::{RunReport, TraceSnapshot, TRACE_STEP_EVENT_CAPACITY, TRACE_TAIL_CAPACITY};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Schema version for [`FailureArtifact`].
// Version 2 makes end-of-step per-link probe counters identity-bearing. Older
// probe-bearing artifacts cannot reproduce that stronger trace identity and
// are rejected explicitly rather than reported as nondeterministic.
pub const FAILURE_ARTIFACT_SCHEMA_VERSION: u32 = 2;
/// Maximum serialized artifact size accepted at the filesystem boundary.
pub const MAX_FAILURE_ARTIFACT_BYTES: usize = 8 * 1024 * 1024;
/// Oracle failures are capped at 64 per run; artifacts preserve at most that cap.
pub const MAX_ARTIFACT_FAILURES: usize = 64;
/// Maximum diagnostic bytes retained for one failure.
pub const MAX_FAILURE_DETAIL_BYTES: usize = 4096;
/// Maximum bytes in the sanitized test-name path component.
pub const MAX_TEST_NAME_COMPONENT_BYTES: usize = 120;

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

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
        BackgroundNoise, ScheduleEvent, SimConfig, SCHEDULE_SCHEMA_VERSION,
    };
    use crate::simulation::harness::{
        run, RunOptions, TraceGameState, TraceNetStats, TraceSessionState, TRACE_TAIL_CAPACITY,
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
        assert_eq!(decoded.artifact_schema_version, 2);
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
        assert!(
            artifact.validate().is_ok(),
            "artifact envelope must preserve the schedule validator's compatibility window"
        );

        let mut artifact = sample_artifact();
        artifact.schedule.schema_version = SCHEDULE_SCHEMA_VERSION + 1;
        assert!(artifact.validate().is_err());
    }

    #[test]
    fn artifact_validation_rejects_incomplete_or_inconsistent_envelopes() {
        let mutations: Vec<Box<dyn Fn(&mut FailureArtifact)>> = vec![
            Box::new(|artifact| artifact.failures.clear()),
            Box::new(|artifact| {
                let _ = artifact.final_confirmed.pop();
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
