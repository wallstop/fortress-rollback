//! Bounded pass-path census artifacts for the deterministic simulation fleet.

use super::harness::RunReport;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub const SOMETIMES_STATE_COUNT: usize = 5;
pub type SometimesStateVector = [bool; SOMETIMES_STATE_COUNT];

pub const SOMETIMES_STATE_IDS: [&str; SOMETIMES_STATE_COUNT] = [
    "rollback.at_prediction_limit.v1",
    "floor_round.lower_floor_consumable.v1",
    "connect_status.nudge_sent.v1",
    "rollback.sparse_earlier_checkpoint_selected.v1",
    "input_ring.within_one_slot_of_capacity.v1",
];

pub const ORGANIC_SHARDS: u32 = 8;
pub const ORGANIC_SCHEDULES_PER_SHARD: u64 = 125;
pub const ORGANIC_STEPS_PER_SCHEDULE: u64 = 5_000;
pub const ORGANIC_SCHEDULES: u64 = 1_000;
pub const ORGANIC_STEPS: u64 = 5_000_000;
pub const MAX_CENSUS_ARTIFACT_BYTES: usize = 65_536;
pub const CENSUS_SCHEMA_VERSION: u32 = 1;

pub const H_SKEW_PROBE: &str =
    "simulation::fleet::h_skew_hour_equivalent_measures_lag_correction_and_cost";
pub const H_OSC_PROBE: &str =
    "simulation::fleet::h_osc_aggregation_pressure_is_measured_and_decays";
pub const H_BLOAT_PROBE: &str =
    "simulation::census::h_bloat_scale_fragmentation_interaction_is_bounded_and_recovers";
pub const H_POLLCAP_PROBE: &str =
    "simulation::census::h_pollcap_targeted_release_defers_without_starvation";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ProbeSpec {
    id: &'static str,
    materialized_schedules: u64,
    scheduled_steps: u64,
    executions: u64,
    peer_sessions: u64,
}

const TARGETED_PROBES: [ProbeSpec; 4] = [
    ProbeSpec {
        id: H_SKEW_PROBE,
        materialized_schedules: 2,
        scheduled_steps: 480_002,
        executions: 3,
        peer_sessions: 6,
    },
    ProbeSpec {
        id: H_OSC_PROBE,
        materialized_schedules: 9,
        scheduled_steps: 21_600,
        executions: 18,
        peer_sessions: 156,
    },
    ProbeSpec {
        id: H_BLOAT_PROBE,
        materialized_schedules: 8,
        scheduled_steps: 5_600,
        executions: 10,
        peer_sessions: 160,
    },
    ProbeSpec {
        id: H_POLLCAP_PROBE,
        materialized_schedules: 1,
        scheduled_steps: 800,
        executions: 3,
        peer_sessions: 48,
    },
];

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceDomain {
    Organic,
    Targeted,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateCount {
    pub id: String,
    pub schedules_hit: u64,
    pub peer_sessions_hit: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CensusAccumulator {
    schedule_count: u64,
    scheduled_steps: u64,
    execution_count: u64,
    peer_session_count: u64,
    schedules_hit: [u64; SOMETIMES_STATE_COUNT],
    peer_sessions_hit: [u64; SOMETIMES_STATE_COUNT],
    saturated: bool,
}

fn add_latched(value: &mut u64, amount: u64, saturated: &mut bool) {
    if let Some(sum) = value.checked_add(amount) {
        *value = sum;
    } else {
        *value = u64::MAX;
        *saturated = true;
    }
}

impl CensusAccumulator {
    pub fn record_materialized_schedule<'a>(
        &mut self,
        reports: impl IntoIterator<Item = &'a RunReport>,
        scheduled_steps: u64,
    ) -> Result<(), String> {
        let reports = reports.into_iter().collect::<Vec<_>>();
        self.record_materialized_vectors(
            reports
                .iter()
                .map(|report| report.sometimes_state_by_peer.as_slice()),
            scheduled_steps,
        )
    }

    fn record_materialized_vectors<'a>(
        &mut self,
        executions: impl IntoIterator<Item = &'a [SometimesStateVector]>,
        scheduled_steps: u64,
    ) -> Result<(), String> {
        let executions = executions.into_iter().collect::<Vec<_>>();
        let Some(first) = executions.first() else {
            return Err(
                "a materialized schedule requires at least one execution report".to_owned(),
            );
        };
        let peer_count = first.len();
        if !(2..=16).contains(&peer_count) {
            return Err(format!(
                "sometimes-state report has {peer_count} peers (expected 2..=16)"
            ));
        }
        let mut schedule_seen = [false; SOMETIMES_STATE_COUNT];
        let mut peer_hits = [0_u64; SOMETIMES_STATE_COUNT];
        for vectors in &executions {
            if vectors.len() != peer_count {
                return Err(
                    "executions of one materialized schedule disagree on peer count".to_owned(),
                );
            }
            for vector in *vectors {
                for (state_index, seen) in vector.iter().copied().enumerate() {
                    schedule_seen[state_index] |= seen;
                    if seen {
                        add_latched(&mut peer_hits[state_index], 1, &mut self.saturated);
                    }
                }
            }
        }

        add_latched(&mut self.schedule_count, 1, &mut self.saturated);
        add_latched(
            &mut self.scheduled_steps,
            scheduled_steps,
            &mut self.saturated,
        );
        add_latched(
            &mut self.execution_count,
            u64::try_from(executions.len()).unwrap_or(u64::MAX),
            &mut self.saturated,
        );
        let peer_count = u64::try_from(peer_count).unwrap_or(u64::MAX);
        let execution_count = u64::try_from(executions.len()).unwrap_or(u64::MAX);
        let Some(peer_sessions) = peer_count.checked_mul(execution_count) else {
            self.saturated = true;
            return Ok(());
        };
        add_latched(
            &mut self.peer_session_count,
            peer_sessions,
            &mut self.saturated,
        );
        for state_index in 0..SOMETIMES_STATE_COUNT {
            if schedule_seen[state_index] {
                add_latched(&mut self.schedules_hit[state_index], 1, &mut self.saturated);
            }
            add_latched(
                &mut self.peer_sessions_hit[state_index],
                peer_hits[state_index],
                &mut self.saturated,
            );
        }
        Ok(())
    }

    fn state_counts(&self) -> [StateCount; SOMETIMES_STATE_COUNT] {
        std::array::from_fn(|index| StateCount {
            id: SOMETIMES_STATE_IDS[index].to_owned(),
            schedules_hit: self.schedules_hit[index],
            peer_sessions_hit: self.peer_sessions_hit[index],
        })
    }
}

pub fn aggregate_peer_vectors(vectors: &[SometimesStateVector]) -> SometimesStateVector {
    let mut aggregate = [false; SOMETIMES_STATE_COUNT];
    for vector in vectors {
        for (target, seen) in aggregate.iter_mut().zip(vector) {
            *target |= seen;
        }
    }
    aggregate
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrganicShardReport {
    pub schema_version: u32,
    pub evidence_domain: EvidenceDomain,
    pub run_id: u64,
    pub shard_index: u32,
    pub shard_count: u32,
    pub schedule_count: u64,
    pub scheduled_steps: u64,
    pub execution_count: u64,
    pub peer_session_count: u64,
    pub saturated: bool,
    pub states: [StateCount; SOMETIMES_STATE_COUNT],
}

impl OrganicShardReport {
    pub fn new(run_id: u64, shard_index: u32, accumulator: &CensusAccumulator) -> Self {
        Self {
            schema_version: CENSUS_SCHEMA_VERSION,
            evidence_domain: EvidenceDomain::Organic,
            run_id,
            shard_index,
            shard_count: ORGANIC_SHARDS,
            schedule_count: accumulator.schedule_count,
            scheduled_steps: accumulator.scheduled_steps,
            execution_count: accumulator.execution_count,
            peer_session_count: accumulator.peer_session_count,
            saturated: accumulator.saturated,
            states: accumulator.state_counts(),
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema_version != CENSUS_SCHEMA_VERSION {
            return Err(format!("unsupported census schema {}", self.schema_version));
        }
        if self.evidence_domain != EvidenceDomain::Organic {
            return Err("organic shard has a non-organic evidence domain".to_owned());
        }
        if self.shard_count != ORGANIC_SHARDS || self.shard_index >= ORGANIC_SHARDS {
            return Err(format!(
                "invalid shard {}/{}",
                self.shard_index, self.shard_count
            ));
        }
        if self.saturated {
            return Err(format!("organic shard {} saturated", self.shard_index));
        }
        if self.schedule_count != ORGANIC_SCHEDULES_PER_SHARD
            || self.scheduled_steps
                != ORGANIC_SCHEDULES_PER_SHARD.saturating_mul(ORGANIC_STEPS_PER_SCHEDULE)
        {
            return Err(format!(
                "organic shard {} workload mismatch: schedules={}, steps={}",
                self.shard_index, self.schedule_count, self.scheduled_steps
            ));
        }
        if self.execution_count != self.schedule_count {
            return Err(format!(
                "organic shard {} schedule/execution counts disagree ({}/{})",
                self.shard_index, self.schedule_count, self.execution_count
            ));
        }
        let minimum_peer_sessions = ORGANIC_SCHEDULES_PER_SHARD.saturating_mul(2);
        let maximum_peer_sessions = ORGANIC_SCHEDULES_PER_SHARD.saturating_mul(16);
        if !(minimum_peer_sessions..=maximum_peer_sessions).contains(&self.peer_session_count) {
            return Err(format!(
                "organic shard {} peer-session count {} is outside {minimum_peer_sessions}..={maximum_peer_sessions}",
                self.shard_index, self.peer_session_count,
            ));
        }
        validate_state_counts(&self.states, self.schedule_count, self.peer_session_count)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetedProbeReport {
    pub schema_version: u32,
    pub evidence_domain: EvidenceDomain,
    pub run_id: u64,
    pub probe_id: String,
    pub materialized_schedules: u64,
    pub scheduled_steps: u64,
    pub execution_count: u64,
    pub peer_session_count: u64,
    pub saturated: bool,
    pub states: [StateCount; SOMETIMES_STATE_COUNT],
}

impl TargetedProbeReport {
    pub fn new(run_id: u64, probe_id: &str, accumulator: &CensusAccumulator) -> Self {
        Self {
            schema_version: CENSUS_SCHEMA_VERSION,
            evidence_domain: EvidenceDomain::Targeted,
            run_id,
            probe_id: probe_id.to_owned(),
            materialized_schedules: accumulator.schedule_count,
            scheduled_steps: accumulator.scheduled_steps,
            execution_count: accumulator.execution_count,
            peer_session_count: accumulator.peer_session_count,
            saturated: accumulator.saturated,
            states: accumulator.state_counts(),
        }
    }

    fn validate(&self) -> Result<usize, String> {
        if self.schema_version != CENSUS_SCHEMA_VERSION {
            return Err(format!("unsupported census schema {}", self.schema_version));
        }
        if self.evidence_domain != EvidenceDomain::Targeted {
            return Err(format!("targeted probe {} has wrong domain", self.probe_id));
        }
        let Some((index, spec)) = TARGETED_PROBES
            .iter()
            .enumerate()
            .find(|(_, spec)| spec.id == self.probe_id)
        else {
            return Err(format!("unknown targeted probe {}", self.probe_id));
        };
        let schedules_match = self.materialized_schedules == spec.materialized_schedules;
        let steps_match = self.scheduled_steps == spec.scheduled_steps;
        let executions_match = self.execution_count == spec.executions;
        if !schedules_match || !steps_match || !executions_match {
            return Err(format!(
                "targeted probe {} workload disagrees with contract (schedules/steps/executions={}/{}/{} vs {}/{}/{})",
                self.probe_id,
                self.materialized_schedules,
                self.scheduled_steps,
                self.execution_count,
                spec.materialized_schedules,
                spec.scheduled_steps,
                spec.executions
            ));
        }
        if self.saturated {
            return Err(format!("targeted probe {} saturated", self.probe_id));
        }
        if self.peer_session_count != spec.peer_sessions {
            return Err(format!(
                "targeted probe {} peer-session count {} disagrees with contract ({})",
                self.probe_id, self.peer_session_count, spec.peer_sessions,
            ));
        }
        validate_state_counts(
            &self.states,
            self.materialized_schedules,
            self.peer_session_count,
        )?;
        Ok(index)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomainSummary {
    pub schedule_count: u64,
    pub scheduled_steps: u64,
    pub execution_count: u64,
    pub peer_session_count: u64,
    pub states: [StateCount; SOMETIMES_STATE_COUNT],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct OrganicSummary(DomainSummary);

impl OrganicSummary {
    fn validate(&self) -> Result<(), String> {
        let summary = &self.0;
        if summary.schedule_count != ORGANIC_SCHEDULES
            || summary.scheduled_steps != ORGANIC_STEPS
            || summary.execution_count != ORGANIC_SCHEDULES
        {
            return Err(format!(
                "organic summary workload mismatch: schedules/steps/executions={}/{}/{}",
                summary.schedule_count, summary.scheduled_steps, summary.execution_count
            ));
        }
        let minimum_peer_sessions = ORGANIC_SCHEDULES.saturating_mul(2);
        let maximum_peer_sessions = ORGANIC_SCHEDULES.saturating_mul(16);
        if !(minimum_peer_sessions..=maximum_peer_sessions).contains(&summary.peer_session_count) {
            return Err(format!(
                "organic summary peer-session count {} is outside {minimum_peer_sessions}..={maximum_peer_sessions}",
                summary.peer_session_count
            ));
        }
        validate_state_counts(
            &summary.states,
            summary.schedule_count,
            summary.peer_session_count,
        )
    }
}

impl<'de> Deserialize<'de> for OrganicSummary {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let organic = Self(DomainSummary::deserialize(deserializer)?);
        organic.validate().map_err(serde::de::Error::custom)?;
        Ok(organic)
    }
}

/// Test-only planted-positive evidence derived from real run reports. Its
/// distinct type cannot be passed to the organic ratchet API.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlantedPositiveEvidence {
    states: [StateCount; SOMETIMES_STATE_COUNT],
}

impl PlantedPositiveEvidence {
    pub fn from_materialized_schedule<'a>(
        reports: impl IntoIterator<Item = &'a RunReport>,
        scheduled_steps: u64,
    ) -> Result<Self, String> {
        let mut accumulator = CensusAccumulator::default();
        accumulator.record_materialized_schedule(reports, scheduled_steps)?;
        Ok(Self {
            states: accumulator.state_counts(),
        })
    }

    pub fn state(&self, index: usize) -> Option<&StateCount> {
        self.states.get(index)
    }
}

/// Test-only fixed-negative evidence. It cannot be passed to the organic ratchet API.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NegativeControlEvidence(SometimesStateVector);

impl NegativeControlEvidence {
    pub fn from_vector(vector: SometimesStateVector) -> Self {
        Self(vector)
    }

    pub fn missing_ids(&self) -> Vec<&'static str> {
        missing_presence_ids(&self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProbeSummary {
    pub probe_id: String,
    pub materialized_schedules: u64,
    pub scheduled_steps: u64,
    pub execution_count: u64,
    pub peer_session_count: u64,
    pub states: [StateCount; SOMETIMES_STATE_COUNT],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetedSummary {
    pub probes: Vec<ProbeSummary>,
    pub totals: DomainSummary,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MergedCensusReport {
    pub schema_version: u32,
    pub run_id: u64,
    pub registry: [String; SOMETIMES_STATE_COUNT],
    pub organic: OrganicSummary,
    pub targeted: TargetedSummary,
}

fn validate_state_counts(
    states: &[StateCount; SOMETIMES_STATE_COUNT],
    schedules: u64,
    peer_sessions: u64,
) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for state in states {
        if !SOMETIMES_STATE_IDS.contains(&state.id.as_str()) {
            return Err(format!("unknown sometimes-state ID {}", state.id));
        }
        if !seen.insert(state.id.as_str()) {
            return Err(format!("duplicate sometimes-state ID {}", state.id));
        }
        if state.schedules_hit > schedules {
            return Err(format!(
                "{} schedules_hit {} exceeds denominator {schedules}",
                state.id, state.schedules_hit
            ));
        }
        if state.peer_sessions_hit > peer_sessions {
            return Err(format!(
                "{} peer_sessions_hit {} exceeds denominator {peer_sessions}",
                state.id, state.peer_sessions_hit
            ));
        }
        if state.schedules_hit > state.peer_sessions_hit {
            return Err(format!(
                "{} schedules_hit {} exceeds peer_sessions_hit {}",
                state.id, state.schedules_hit, state.peer_sessions_hit
            ));
        }
    }
    for expected in SOMETIMES_STATE_IDS {
        if !seen.contains(expected) {
            return Err(format!("missing sometimes-state ID {expected}"));
        }
    }
    for (state, expected) in states.iter().zip(SOMETIMES_STATE_IDS) {
        if state.id != expected {
            return Err(format!(
                "sometimes-state registry order mismatch: expected {expected}, got {}",
                state.id
            ));
        }
    }
    Ok(())
}

fn merge_count(target: &mut u64, value: u64) -> Result<(), String> {
    *target = target
        .checked_add(value)
        .ok_or_else(|| "census aggregation saturated".to_owned())?;
    Ok(())
}

fn merge_summary(
    accumulator: &mut CensusAccumulator,
    schedule_count: u64,
    scheduled_steps: u64,
    execution_count: u64,
    peer_session_count: u64,
    states: &[StateCount; SOMETIMES_STATE_COUNT],
) -> Result<(), String> {
    merge_count(&mut accumulator.schedule_count, schedule_count)?;
    merge_count(&mut accumulator.scheduled_steps, scheduled_steps)?;
    merge_count(&mut accumulator.execution_count, execution_count)?;
    merge_count(&mut accumulator.peer_session_count, peer_session_count)?;
    for (index, state) in states.iter().enumerate() {
        merge_count(&mut accumulator.schedules_hit[index], state.schedules_hit)?;
        merge_count(
            &mut accumulator.peer_sessions_hit[index],
            state.peer_sessions_hit,
        )?;
    }
    Ok(())
}

fn summary(accumulator: &CensusAccumulator) -> DomainSummary {
    DomainSummary {
        schedule_count: accumulator.schedule_count,
        scheduled_steps: accumulator.scheduled_steps,
        execution_count: accumulator.execution_count,
        peer_session_count: accumulator.peer_session_count,
        states: accumulator.state_counts(),
    }
}

fn read_bounded(path: &Path) -> Result<Vec<u8>, String> {
    let file = std::fs::File::open(path)
        .map_err(|error| format!("failed to open {}: {error}", path.display()))?;
    let limit = u64::try_from(MAX_CENSUS_ARTIFACT_BYTES)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let mut bytes = Vec::new();
    file.take(limit)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    if bytes.len() > MAX_CENSUS_ARTIFACT_BYTES {
        return Err(format!(
            "{} exceeds {MAX_CENSUS_ARTIFACT_BYTES} bytes",
            path.display()
        ));
    }
    Ok(bytes)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if bytes.len() > MAX_CENSUS_ARTIFACT_BYTES {
        return Err(format!(
            "serialized census artifact has {} bytes (max {MAX_CENSUS_ARTIFACT_BYTES})",
            bytes.len()
        ));
    }
    let directory = path
        .parent()
        .ok_or_else(|| format!("{} has no parent directory", path.display()))?;
    std::fs::create_dir_all(directory)
        .map_err(|error| format!("failed to create {}: {error}", directory.display()))?;
    let mut temporary = None;
    for _ in 0..16 {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let candidate = directory.join(format!(
            ".{}.{}.{}.tmp",
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("census"),
            std::process::id(),
            sequence
        ));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(mut file) => {
                if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
                    let _ = std::fs::remove_file(&candidate);
                    return Err(format!("failed to write {}: {error}", candidate.display()));
                }
                temporary = Some(candidate);
                break;
            },
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {},
            Err(error) => {
                return Err(format!("failed to create {}: {error}", candidate.display()));
            },
        }
    }
    let temporary = temporary.ok_or_else(|| "failed to allocate census temp file".to_owned())?;
    if let Err(error) = std::fs::hard_link(&temporary, path) {
        let _ = std::fs::remove_file(&temporary);
        return Err(format!(
            "failed to atomically publish {}: {error}",
            path.display()
        ));
    }
    let _ = std::fs::remove_file(temporary);
    Ok(())
}

fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    serde_json::to_vec(value).map_err(|error| format!("failed to serialize census JSON: {error}"))
}

fn census_root_from_env() -> Result<PathBuf, String> {
    let raw = std::env::var("FORTRESS_SIM_CENSUS_DIR")
        .map_err(|error| format!("FORTRESS_SIM_CENSUS_DIR is required: {error}"))?;
    if raw.trim().is_empty() {
        return Err("FORTRESS_SIM_CENSUS_DIR cannot be empty".to_owned());
    }
    Ok(PathBuf::from(raw))
}

pub fn write_organic_shard(report: &OrganicShardReport) -> Result<PathBuf, String> {
    write_organic_shard_at(&census_root_from_env()?, report)
}

fn write_organic_shard_at(root: &Path, report: &OrganicShardReport) -> Result<PathBuf, String> {
    report.validate()?;
    let path = root
        .join("published")
        .join(format!("shard-{}.json", report.shard_index));
    write_atomic(&path, &encode(report)?)?;
    Ok(path)
}

pub fn write_targeted_probe(report: &TargetedProbeReport) -> Result<PathBuf, String> {
    write_targeted_probe_at(&census_root_from_env()?, report)
}

pub fn publish_targeted_probe(
    probe_id: &str,
    accumulator: &CensusAccumulator,
) -> Result<Option<PathBuf>, String> {
    if std::env::var("FORTRESS_SIM_TIER").as_deref() != Ok("nightly") {
        return Ok(None);
    }
    let run_id = std::env::var("FORTRESS_SIM_SEED_BASE")
        .map_err(|error| {
            format!("FORTRESS_SIM_SEED_BASE is required for targeted census: {error}")
        })?
        .parse::<u64>()
        .map_err(|error| format!("FORTRESS_SIM_SEED_BASE must be an unsigned integer: {error}"))?;
    write_targeted_probe(&TargetedProbeReport::new(run_id, probe_id, accumulator)).map(Some)
}

fn write_targeted_probe_at(root: &Path, report: &TargetedProbeReport) -> Result<PathBuf, String> {
    let index = report.validate()?;
    let path = root.join("targeted").join(format!("probe-{index}.json"));
    write_atomic(&path, &encode(report)?)?;
    Ok(path)
}

fn json_files_bounded(directory: &Path, maximum: usize) -> Result<Vec<PathBuf>, String> {
    let entries = std::fs::read_dir(directory)
        .map_err(|error| format!("failed to read {}: {error}", directory.display()))?;
    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("failed to read directory entry: {error}"))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?;
        if !file_type.is_file() || path.extension().is_none_or(|extension| extension != "json") {
            return Err(format!("unexpected census artifact {}", path.display()));
        }
        paths.push(path);
        if paths.len() > maximum {
            return Err(format!(
                "{} has more than {maximum} JSON artifacts",
                directory.display()
            ));
        }
    }
    paths.sort();
    Ok(paths)
}

fn validate_aggregate_bytes(existing: usize, merged: usize) -> Result<usize, String> {
    let total = existing
        .checked_add(merged)
        .ok_or_else(|| "census byte count saturated".to_owned())?;
    if total > MAX_CENSUS_ARTIFACT_BYTES {
        return Err(format!(
            "published census artifacts have {total} bytes (max {MAX_CENSUS_ARTIFACT_BYTES})"
        ));
    }
    Ok(total)
}

pub fn merge_nightly_at(root: &Path) -> Result<(PathBuf, MergedCensusReport), String> {
    let published = root.join("published");
    let shard_paths = json_files_bounded(&published, usize::try_from(ORGANIC_SHARDS).unwrap_or(8))?;
    if shard_paths.len() != usize::try_from(ORGANIC_SHARDS).unwrap_or(8) {
        return Err(format!(
            "expected {ORGANIC_SHARDS} organic shard artifacts, found {}",
            shard_paths.len()
        ));
    }
    let mut run_id = None;
    let mut seen_shards = BTreeSet::new();
    let mut organic = CensusAccumulator::default();
    let mut published_bytes = 0usize;
    for path in &shard_paths {
        let bytes = read_bounded(path)?;
        published_bytes = published_bytes
            .checked_add(bytes.len())
            .ok_or_else(|| "census byte count saturated".to_owned())?;
        let report: OrganicShardReport = serde_json::from_slice(&bytes)
            .map_err(|error| format!("invalid organic shard {}: {error}", path.display()))?;
        report.validate()?;
        if !seen_shards.insert(report.shard_index) {
            return Err(format!("duplicate organic shard {}", report.shard_index));
        }
        match run_id {
            Some(expected) if expected != report.run_id => {
                return Err(format!(
                    "organic shard run_id mismatch: expected {expected}, got {}",
                    report.run_id
                ));
            },
            None => run_id = Some(report.run_id),
            _ => {},
        }
        merge_summary(
            &mut organic,
            report.schedule_count,
            report.scheduled_steps,
            report.execution_count,
            report.peer_session_count,
            &report.states,
        )?;
    }
    if seen_shards != (0..ORGANIC_SHARDS).collect() {
        return Err("organic shard index set is incomplete".to_owned());
    }
    if organic.schedule_count != ORGANIC_SCHEDULES
        || organic.scheduled_steps != ORGANIC_STEPS
        || organic.execution_count != ORGANIC_SCHEDULES
    {
        return Err(format!(
            "organic totals mismatch: schedules={}, steps={}, executions={}",
            organic.schedule_count, organic.scheduled_steps, organic.execution_count
        ));
    }

    let targeted_directory = root.join("targeted");
    let targeted_paths = json_files_bounded(&targeted_directory, TARGETED_PROBES.len())?;
    if targeted_paths.len() != TARGETED_PROBES.len() {
        return Err(format!(
            "expected {} targeted probe artifacts, found {}",
            TARGETED_PROBES.len(),
            targeted_paths.len()
        ));
    }
    let mut probes: Vec<Option<TargetedProbeReport>> = vec![None; TARGETED_PROBES.len()];
    let mut targeted = CensusAccumulator::default();
    for path in &targeted_paths {
        let bytes = read_bounded(path)?;
        let report: TargetedProbeReport = serde_json::from_slice(&bytes)
            .map_err(|error| format!("invalid targeted probe {}: {error}", path.display()))?;
        let index = report.validate()?;
        let organic_run_id = run_id.ok_or_else(|| "organic shard set has no run_id".to_owned())?;
        if report.run_id != organic_run_id {
            return Err(format!(
                "targeted probe {} run_id {} does not match organic run_id {organic_run_id}",
                report.probe_id, report.run_id
            ));
        }
        if probes[index].replace(report.clone()).is_some() {
            return Err(format!("duplicate targeted probe {}", report.probe_id));
        }
        merge_summary(
            &mut targeted,
            report.materialized_schedules,
            report.scheduled_steps,
            report.execution_count,
            report.peer_session_count,
            &report.states,
        )?;
    }
    if probes.iter().any(Option::is_none) {
        return Err("targeted probe set is incomplete".to_owned());
    }
    let probes = probes
        .into_iter()
        .map(|report| {
            let report = report.expect("targeted probe completeness checked");
            ProbeSummary {
                probe_id: report.probe_id,
                materialized_schedules: report.materialized_schedules,
                scheduled_steps: report.scheduled_steps,
                execution_count: report.execution_count,
                peer_session_count: report.peer_session_count,
                states: report.states,
            }
        })
        .collect();
    let merged = MergedCensusReport {
        schema_version: CENSUS_SCHEMA_VERSION,
        run_id: run_id.ok_or_else(|| "organic shard set has no run_id".to_owned())?,
        registry: SOMETIMES_STATE_IDS.map(str::to_owned),
        organic: OrganicSummary(summary(&organic)),
        targeted: TargetedSummary {
            probes,
            totals: summary(&targeted),
        },
    };
    let merged_bytes = encode(&merged)?;
    validate_aggregate_bytes(published_bytes, merged_bytes.len())?;
    let merged_path = published.join("merged.json");
    write_atomic(&merged_path, &merged_bytes)?;
    for path in targeted_paths {
        std::fs::remove_file(&path)
            .map_err(|error| format!("failed to consume {}: {error}", path.display()))?;
    }
    std::fs::remove_dir(&targeted_directory).map_err(|error| {
        format!(
            "failed to remove consumed targeted directory {}: {error}",
            targeted_directory.display()
        )
    })?;
    Ok((merged_path, merged))
}

fn missing_presence_ids(vector: &SometimesStateVector) -> Vec<&'static str> {
    SOMETIMES_STATE_IDS
        .into_iter()
        .zip(vector)
        .filter_map(|(id, seen)| (!seen).then_some(id))
        .collect()
}

pub fn missing_organic_presence_ids(summary: &OrganicSummary) -> Vec<&'static str> {
    let vector = summary
        .0
        .states
        .each_ref()
        .map(|state| state.schedules_hit > 0);
    missing_presence_ids(&vector)
}

pub fn evaluate_organic_presence(summary: &OrganicSummary) -> Result<(), Vec<&'static str>> {
    let missing = missing_organic_presence_ids(summary);
    if missing.is_empty() {
        Ok(())
    } else {
        Err(missing)
    }
}

#[test]
#[ignore = "merge-only nightly census validation; workflow runs after fleet"]
fn merge_nightly_sometimes_state_census() {
    assert_eq!(
        std::env::var("FORTRESS_SIM_TIER").as_deref(),
        Ok("nightly"),
        "nightly census merge requires FORTRESS_SIM_TIER=nightly"
    );
    let root = census_root_from_env().expect("nightly census root is configured");
    let _merged = merge_nightly_at(&root).expect("nightly census merges and validates");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "fortress-sometimes-state-{label}-{}-{}",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn organic_report(index: u32) -> OrganicShardReport {
        let mut accumulator = CensusAccumulator::default();
        let vectors = vec![[false; SOMETIMES_STATE_COUNT]; 2];
        for _ in 0..ORGANIC_SCHEDULES_PER_SHARD {
            accumulator
                .record_materialized_vectors([vectors.as_slice()], ORGANIC_STEPS_PER_SCHEDULE)
                .expect("synthetic organic schedule records");
        }
        OrganicShardReport::new(17, index, &accumulator)
    }

    fn targeted_report(spec: ProbeSpec) -> TargetedProbeReport {
        let mut accumulator = CensusAccumulator::default();
        let peer_counts: Vec<usize> = match spec.id {
            H_OSC_PROBE => vec![2, 2, 2, 8, 8, 8, 16, 16, 16],
            H_BLOAT_PROBE | H_POLLCAP_PROBE => {
                vec![16; usize::try_from(spec.materialized_schedules).unwrap_or(0)]
            },
            H_SKEW_PROBE => vec![2, 2],
            _ => unreachable!("targeted fixture uses the fixed probe registry"),
        };
        for index in 0..spec.materialized_schedules {
            let executions = if spec.id == H_OSC_PROBE {
                2
            } else if index == 0 {
                spec.executions - spec.materialized_schedules + 1
            } else {
                1
            };
            let scheduled_steps = if index == 0 {
                spec.scheduled_steps - spec.materialized_schedules + 1
            } else {
                1
            };
            let vectors = vec![
                [false; SOMETIMES_STATE_COUNT];
                peer_counts[usize::try_from(index).unwrap_or(0)]
            ];
            accumulator
                .record_materialized_vectors(
                    std::iter::repeat_n(
                        vectors.as_slice(),
                        usize::try_from(executions).unwrap_or(1),
                    ),
                    scheduled_steps,
                )
                .expect("synthetic targeted schedule records");
        }
        assert_eq!(accumulator.peer_session_count, spec.peer_sessions);
        TargetedProbeReport::new(17, spec.id, &accumulator)
    }

    fn write_complete_fixture(root: &Path) {
        for index in 0..ORGANIC_SHARDS {
            write_organic_shard_at(root, &organic_report(index)).expect("organic fixture writes");
        }
        for spec in TARGETED_PROBES {
            write_targeted_probe_at(root, &targeted_report(spec)).expect("targeted fixture writes");
        }
    }

    fn rewrite_with_json_padding(path: &Path, length: usize) {
        let mut bytes = read_bounded(path).expect("fixture reads before padding");
        assert!(
            bytes.len() <= length,
            "fixture already exceeds requested padding"
        );
        bytes.resize(length, b' ');
        std::fs::write(path, bytes).expect("padded JSON fixture rewrites");
    }

    #[test]
    fn registry_is_unique_and_serializes_in_fixed_order() {
        let unique: BTreeSet<_> = SOMETIMES_STATE_IDS.into_iter().collect();
        assert_eq!(unique.len(), SOMETIMES_STATE_COUNT);
        let encoded = serde_json::to_string(&SOMETIMES_STATE_IDS).expect("registry serializes");
        assert_eq!(
            encoded,
            "[\"rollback.at_prediction_limit.v1\",\"floor_round.lower_floor_consumable.v1\",\"connect_status.nudge_sent.v1\",\"rollback.sparse_earlier_checkpoint_selected.v1\",\"input_ring.within_one_slot_of_capacity.v1\"]"
        );
    }

    #[test]
    fn one_materialized_schedule_is_idempotent_across_replays() {
        let first = vec![
            [true, false, false, false, false],
            [true, true, false, false, false],
        ];
        let replay = first.clone();
        let mut accumulator = CensusAccumulator::default();
        accumulator
            .record_materialized_vectors([first.as_slice(), replay.as_slice()], 10)
            .expect("replayed schedule records");
        assert_eq!(accumulator.schedule_count, 1);
        assert_eq!(accumulator.execution_count, 2);
        assert_eq!(accumulator.peer_session_count, 4);
        assert_eq!(accumulator.schedules_hit, [1, 1, 0, 0, 0]);
        assert_eq!(accumulator.peer_sessions_hit, [4, 2, 0, 0, 0]);
    }

    #[test]
    fn materialized_schedule_rejects_missing_or_mismatched_peer_vectors() {
        let mut accumulator = CensusAccumulator::default();
        assert!(accumulator
            .record_materialized_vectors(std::iter::empty::<&[SometimesStateVector]>(), 10)
            .expect_err("missing execution rejects")
            .contains("at least one"));

        let no_peers: Vec<SometimesStateVector> = Vec::new();
        assert!(accumulator
            .record_materialized_vectors([no_peers.as_slice()], 10)
            .expect_err("legacy empty peer vector rejects")
            .contains("2..=16"));

        let two_peers = vec![[false; SOMETIMES_STATE_COUNT]; 2];
        let three_peers = vec![[false; SOMETIMES_STATE_COUNT]; 3];
        assert!(accumulator
            .record_materialized_vectors([two_peers.as_slice(), three_peers.as_slice()], 10)
            .expect_err("replay peer-count mismatch rejects")
            .contains("disagree"));
    }

    #[test]
    fn saturation_latches_without_wrapping() {
        let mut accumulator = CensusAccumulator {
            schedule_count: u64::MAX,
            ..CensusAccumulator::default()
        };
        let report = vec![[false; SOMETIMES_STATE_COUNT]; 2];
        accumulator
            .record_materialized_vectors([report.as_slice()], 1)
            .expect("saturating record succeeds");
        assert_eq!(accumulator.schedule_count, u64::MAX);
        assert!(accumulator.saturated);
    }

    #[test]
    fn atomic_shard_writer_refuses_duplicate_destination() {
        let root = temp_root("atomic");
        let report = organic_report(0);
        let path = write_organic_shard_at(&root, &report).expect("first shard write succeeds");
        assert!(write_organic_shard_at(&root, &report).is_err());
        let decoded: OrganicShardReport =
            serde_json::from_slice(&read_bounded(&path).expect("shard reads"))
                .expect("shard decodes");
        assert_eq!(decoded, report);
        let entries = std::fs::read_dir(path.parent().expect("shard has parent"))
            .expect("published directory reads")
            .count();
        assert_eq!(entries, 1, "atomic writer must not leak temp files");
        std::fs::remove_dir_all(root).expect("temp fixture removes");
    }

    #[test]
    fn concurrent_atomic_shard_writers_have_exactly_one_winner() {
        let root = temp_root("atomic-concurrent");
        let report = organic_report(0);
        let outcomes = std::thread::scope(|scope| {
            (0..8)
                .map(|_| scope.spawn(|| write_organic_shard_at(&root, &report)))
                .collect::<Vec<_>>()
                .into_iter()
                .map(|handle| handle.join().expect("writer thread joins"))
                .collect::<Vec<_>>()
        });
        assert_eq!(outcomes.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(outcomes.iter().filter(|result| result.is_err()).count(), 7);
        let published = root.join("published/shard-0.json");
        let decoded: OrganicShardReport =
            serde_json::from_slice(&read_bounded(&published).expect("winning shard reads"))
                .expect("winning shard decodes");
        assert_eq!(decoded, report);
        std::fs::remove_dir_all(root).expect("concurrent fixture removes");
    }

    #[test]
    fn complete_merge_publishes_exactly_nine_files_and_consumes_targeted_fragments() {
        let root = temp_root("complete");
        write_complete_fixture(&root);
        let (merged_path, merged) = merge_nightly_at(&root).expect("complete fixture merges");
        assert_eq!(merged.organic.0.schedule_count, ORGANIC_SCHEDULES);
        assert_eq!(merged.organic.0.scheduled_steps, ORGANIC_STEPS);
        assert!(merged
            .organic
            .0
            .states
            .iter()
            .all(|state| state.schedules_hit == 0));
        assert!(!root.join("targeted").exists());
        let published =
            json_files_bounded(&root.join("published"), 9).expect("published artifacts enumerate");
        assert_eq!(published.len(), 9);
        assert!(published.contains(&merged_path));
        let total: usize = published
            .iter()
            .map(|path| std::fs::metadata(path).expect("metadata reads").len() as usize)
            .sum();
        assert!(total <= MAX_CENSUS_ARTIFACT_BYTES);
        std::fs::remove_dir_all(root).expect("temp fixture removes");
    }

    #[test]
    fn merge_rejects_missing_and_duplicate_shards() {
        let missing_root = temp_root("missing");
        write_complete_fixture(&missing_root);
        std::fs::remove_file(missing_root.join("published/shard-7.json"))
            .expect("one shard removes");
        assert!(merge_nightly_at(&missing_root)
            .expect_err("missing shard fails")
            .contains("expected 8"));
        std::fs::remove_dir_all(missing_root).expect("missing fixture removes");

        let duplicate_root = temp_root("duplicate");
        write_complete_fixture(&duplicate_root);
        std::fs::copy(
            duplicate_root.join("published/shard-0.json"),
            duplicate_root.join("published/shard-7.json"),
        )
        .expect("duplicate shard copies");
        let error = merge_nightly_at(&duplicate_root).expect_err("duplicate shard fails");
        assert!(
            error.contains("duplicate organic shard"),
            "wrong error: {error}"
        );
        std::fs::remove_dir_all(duplicate_root).expect("duplicate fixture removes");
    }

    #[test]
    fn state_validation_rejects_unknown_duplicate_and_missing_ids() {
        let mut report = organic_report(0);
        report.states[0].id = "unknown.v1".to_owned();
        assert!(report
            .validate()
            .expect_err("unknown ID fails")
            .contains("unknown"));

        let mut report = organic_report(0);
        report.states[1].id = report.states[0].id.clone();
        assert!(report
            .validate()
            .expect_err("duplicate ID fails")
            .contains("duplicate"));

        let mut impossible = organic_report(0);
        impossible.states[0].schedules_hit = 1;
        impossible.states[0].peer_sessions_hit = 0;
        assert!(impossible
            .validate()
            .expect_err("schedule hits without peer hits fail")
            .contains("exceeds peer_sessions_hit"));

        let mut value = serde_json::to_value(organic_report(0)).expect("report encodes");
        value["states"]
            .as_array_mut()
            .expect("states is array")
            .pop();
        assert!(serde_json::from_value::<OrganicShardReport>(value).is_err());

        let mut value = serde_json::to_value(organic_report(0)).expect("report encodes");
        value
            .as_object_mut()
            .expect("report is an object")
            .insert("unexpected".to_owned(), serde_json::Value::Bool(true));
        assert!(serde_json::from_value::<OrganicShardReport>(value).is_err());
    }

    #[test]
    fn shard_validation_rejects_workload_peer_and_run_mismatches() {
        let mut schedules = organic_report(0);
        schedules.schedule_count -= 1;
        assert!(schedules
            .validate()
            .expect_err("wrong schedule count fails")
            .contains("workload mismatch"));

        let mut steps = organic_report(0);
        steps.scheduled_steps -= 1;
        assert!(steps
            .validate()
            .expect_err("wrong step count fails")
            .contains("workload mismatch"));

        let mut peers = organic_report(0);
        peers.peer_session_count = ORGANIC_SCHEDULES_PER_SHARD * 16 + 1;
        assert!(peers
            .validate()
            .expect_err("excess peer sessions fail")
            .contains("peer-session"));

        let spec = TARGETED_PROBES[0];
        let mut targeted_steps = targeted_report(spec);
        targeted_steps.scheduled_steps -= 1;
        assert!(targeted_steps
            .validate()
            .expect_err("targeted step mismatch fails")
            .contains("workload"));

        let mut targeted_peers = targeted_report(spec);
        targeted_peers.peer_session_count += 1;
        assert!(targeted_peers
            .validate()
            .expect_err("targeted peer denominator mismatch fails")
            .contains("peer-session"));

        let organic_root = temp_root("organic-run-id");
        write_complete_fixture(&organic_root);
        let path = organic_root.join("published/shard-7.json");
        let mut organic: OrganicShardReport =
            serde_json::from_slice(&read_bounded(&path).expect("organic fixture reads"))
                .expect("organic fixture decodes");
        organic.run_id += 1;
        std::fs::write(&path, encode(&organic).expect("organic fixture re-encodes"))
            .expect("organic fixture rewrites");
        assert!(merge_nightly_at(&organic_root)
            .expect_err("organic run mismatch fails")
            .contains("run_id"));
        std::fs::remove_dir_all(organic_root).expect("organic mismatch fixture removes");

        let targeted_root = temp_root("targeted-run-id");
        write_complete_fixture(&targeted_root);
        let path = targeted_root.join("targeted/probe-0.json");
        let mut report: TargetedProbeReport =
            serde_json::from_slice(&read_bounded(&path).expect("targeted fixture reads"))
                .expect("targeted fixture decodes");
        report.run_id += 1;
        std::fs::write(&path, encode(&report).expect("targeted fixture re-encodes"))
            .expect("targeted fixture rewrites");
        assert!(merge_nightly_at(&targeted_root)
            .expect_err("targeted run mismatch fails")
            .contains("run_id"));
        std::fs::remove_dir_all(targeted_root).expect("targeted mismatch fixture removes");
    }

    #[test]
    fn merge_rejects_missing_duplicate_and_unknown_targeted_probes() {
        let missing_root = temp_root("target-missing");
        write_complete_fixture(&missing_root);
        std::fs::remove_file(missing_root.join("targeted/probe-3.json"))
            .expect("target fragment removes");
        assert!(merge_nightly_at(&missing_root)
            .expect_err("missing target fails")
            .contains("expected 4"));
        std::fs::remove_dir_all(missing_root).expect("missing target fixture removes");

        let duplicate_root = temp_root("target-duplicate");
        write_complete_fixture(&duplicate_root);
        std::fs::copy(
            duplicate_root.join("targeted/probe-0.json"),
            duplicate_root.join("targeted/probe-3.json"),
        )
        .expect("duplicate target copies");
        assert!(merge_nightly_at(&duplicate_root)
            .expect_err("duplicate target fails")
            .contains("duplicate targeted probe"));
        std::fs::remove_dir_all(duplicate_root).expect("duplicate target fixture removes");

        let unknown_root = temp_root("target-unknown");
        write_complete_fixture(&unknown_root);
        let path = unknown_root.join("targeted/probe-3.json");
        let mut report: TargetedProbeReport =
            serde_json::from_slice(&read_bounded(&path).expect("target fixture reads"))
                .expect("target fixture decodes");
        report.probe_id = "unknown-target".to_owned();
        std::fs::write(&path, encode(&report).expect("target fixture re-encodes"))
            .expect("target fixture rewrites");
        assert!(merge_nightly_at(&unknown_root)
            .expect_err("unknown target fails")
            .contains("unknown targeted probe"));
        std::fs::remove_dir_all(unknown_root).expect("unknown target fixture removes");
    }

    #[test]
    fn artifact_bound_accepts_limit_and_rejects_one_byte_over() {
        let root = temp_root("bound");
        let path = root.join("published/bound.json");
        assert!(write_atomic(&path, &vec![b'x'; MAX_CENSUS_ARTIFACT_BYTES]).is_ok());
        assert!(write_atomic(
            &root.join("published/oversized.json"),
            &vec![b'x'; MAX_CENSUS_ARTIFACT_BYTES + 1]
        )
        .is_err());
        assert_eq!(
            validate_aggregate_bytes(MAX_CENSUS_ARTIFACT_BYTES - 1, 1),
            Ok(MAX_CENSUS_ARTIFACT_BYTES)
        );
        assert!(validate_aggregate_bytes(MAX_CENSUS_ARTIFACT_BYTES, 1).is_err());
        assert!(validate_aggregate_bytes(usize::MAX, 1).is_err());
        std::fs::remove_dir_all(root).expect("bound fixture removes");
    }

    #[test]
    fn merger_rejects_saturated_oversized_and_aggregate_oversized_evidence() {
        let saturated_root = temp_root("merge-saturated");
        write_complete_fixture(&saturated_root);
        let saturated_path = saturated_root.join("published/shard-0.json");
        let mut saturated: OrganicShardReport = serde_json::from_slice(
            &read_bounded(&saturated_path).expect("saturated fixture reads"),
        )
        .expect("saturated fixture decodes");
        saturated.saturated = true;
        std::fs::write(
            &saturated_path,
            encode(&saturated).expect("saturated fixture encodes"),
        )
        .expect("saturated fixture rewrites");
        assert!(merge_nightly_at(&saturated_root)
            .expect_err("saturated shard fails through merger")
            .contains("saturated"));
        std::fs::remove_dir_all(saturated_root).expect("saturated fixture removes");

        let oversized_root = temp_root("merge-file-oversized");
        write_complete_fixture(&oversized_root);
        rewrite_with_json_padding(
            &oversized_root.join("published/shard-0.json"),
            MAX_CENSUS_ARTIFACT_BYTES + 1,
        );
        assert!(merge_nightly_at(&oversized_root)
            .expect_err("oversized shard fails through bounded reader")
            .contains("exceeds"));
        std::fs::remove_dir_all(oversized_root).expect("oversized fixture removes");

        let aggregate_root = temp_root("merge-aggregate-oversized");
        write_complete_fixture(&aggregate_root);
        for index in 0..ORGANIC_SHARDS {
            rewrite_with_json_padding(
                &aggregate_root
                    .join("published")
                    .join(format!("shard-{index}.json")),
                9_000,
            );
        }
        assert!(merge_nightly_at(&aggregate_root)
            .expect_err("aggregate output bound fails through merger")
            .contains("published census artifacts"));
        std::fs::remove_dir_all(aggregate_root).expect("aggregate fixture removes");
    }

    #[test]
    fn all_zero_vector_lists_every_missing_registry_id() {
        let negative = NegativeControlEvidence::from_vector([false; SOMETIMES_STATE_COUNT]);
        assert_eq!(missing_presence_ids(&negative.0), SOMETIMES_STATE_IDS);

        let organic = OrganicSummary(DomainSummary {
            schedule_count: 1,
            scheduled_steps: 1,
            execution_count: 1,
            peer_session_count: 2,
            states: std::array::from_fn(|index| StateCount {
                id: SOMETIMES_STATE_IDS[index].to_owned(),
                schedules_hit: 0,
                peer_sessions_hit: 0,
            }),
        });
        assert_eq!(
            evaluate_organic_presence(&organic).expect_err("zero organic evidence rejects"),
            SOMETIMES_STATE_IDS
        );
    }

    #[test]
    fn organic_summary_deserialization_rejects_targeted_shaped_totals() {
        let states = std::array::from_fn(|index| StateCount {
            id: SOMETIMES_STATE_IDS[index].to_owned(),
            schedules_hit: 0,
            peer_sessions_hit: 0,
        });
        let targeted_shaped = DomainSummary {
            schedule_count: 20,
            scheduled_steps: 508_002,
            execution_count: 34,
            peer_session_count: 370,
            states,
        };
        let encoded = serde_json::to_vec(&targeted_shaped).expect("targeted totals encode");
        assert!(serde_json::from_slice::<OrganicSummary>(&encoded).is_err());
    }
}
