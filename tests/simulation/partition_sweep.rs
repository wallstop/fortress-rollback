//! Expectation-aware evidence for symmetric partitions and directed fault injection.
//!
//! The ordinary oracle remains unchanged. This sweep interprets its verdict
//! only after checking the declared fault premise, and distinguishes
//! bounded recovery, expected finite safe failure, defects, and invalid
//! evidence. Every treatment is replayed with the same materialized inputs;
//! equality of the trace hash alone is never treated as replay equality.

use super::harness::artifact::{ArtifactFailure, FailureClass};
use super::harness::oracle::{
    InputFingerprint, OracleFailure, ViolationSource, MIN_END_CONFIRMED, POST_HEAL_MIN_ADVANCE,
};
use super::harness::schedule::{
    BackgroundNoise, DropPolicy, Schedule, ScheduleEvent, SimConfig, SCHEDULE_SCHEMA_VERSION,
};
use super::harness::{
    classify_component_scope, run_with_input, ComponentOutcome, ComponentScopeOptions,
    HealRecoveryObservation, PeerStallObservation, PeerWireTotals, RunOptions, RunReport, SimInput,
    TraceNetStats, TraceSessionState, TraceSnapshot,
};
use crate::common::sim_net::{LinkPolicy, SimReceiveStats};
use crate::common::stubs::StubInput;
use fortress_rollback::{EventKind, SessionMetrics, SessionState};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

const PARTITION_LEDGER_SCHEMA: u32 = 6;
const SEED: u64 = 0x5041_5254_4954_494F;
const STEPS: u32 = 900;
const CUT_STEP: u32 = 100;
const PRE_CUT_PROBE_STEP: u32 = CUT_STEP - 1;
const SHORT_HEAL_STEP: u32 = 180;
const LONG_HEAL_STEP: u32 = 450;
const REPEATED_CYCLE_STEPS: u32 = 20;
const POLL_FINAL_HEAL_STEP: u32 = 200;
const POLL_STALL_STEPS: [u32; 3] = [CUT_STEP, 140, 180];
const FLAP_FINAL_HEAL_STEP: u32 = 320;
const FLAP_CUT_STEPS: [u32; 3] = [CUT_STEP, 200, 300];
const FLAP_HEAL_STEPS: [u32; 3] = [120, 220, FLAP_FINAL_HEAL_STEP];
const DISCONNECT_TIMEOUT: Duration = Duration::from_secs(2);
const RECEIVE_MESSAGE_LIMIT: usize = 256;
const MIN_TERMINAL_PLATEAU_STEPS: u32 = 32;
const CORRUPTION_FRAME: i32 = 20;
const IN_WINDOW_CORRUPTION_FRAME: i32 = 120;
const CORRUPTED_PEER: usize = 1;
const ALL_SIZES: [usize; 5] = [2, 3, 4, 8, 16];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
enum ScenarioKind {
    ShortHealContinue,
    LongHealHalt,
    PermanentHalt,
    PermanentContinue,
    Duplication,
    Reorder,
    BurstLoss,
    AsymmetricDelay,
    RepeatedFlap,
    PollStall,
}

impl ScenarioKind {
    const ALL: [Self; 10] = [
        Self::ShortHealContinue,
        Self::LongHealHalt,
        Self::PermanentHalt,
        Self::PermanentContinue,
        Self::Duplication,
        Self::Reorder,
        Self::BurstLoss,
        Self::AsymmetricDelay,
        Self::RepeatedFlap,
        Self::PollStall,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::ShortHealContinue => "short-heal-continue",
            Self::LongHealHalt => "long-heal-halt",
            Self::PermanentHalt => "permanent-halt",
            Self::PermanentContinue => "permanent-continue",
            Self::Duplication => "duplication",
            Self::Reorder => "reorder",
            Self::BurstLoss => "burst-loss",
            Self::AsymmetricDelay => "asymmetric-delay",
            Self::RepeatedFlap => "repeated-flap",
            Self::PollStall => "poll-stall",
        }
    }

    const fn heal_step(self) -> Option<u32> {
        match self {
            Self::ShortHealContinue
            | Self::Duplication
            | Self::Reorder
            | Self::BurstLoss
            | Self::AsymmetricDelay => Some(SHORT_HEAL_STEP),
            Self::LongHealHalt => Some(LONG_HEAL_STEP),
            Self::RepeatedFlap => Some(FLAP_FINAL_HEAL_STEP),
            Self::PollStall => Some(POLL_FINAL_HEAL_STEP),
            Self::PermanentHalt | Self::PermanentContinue => None,
        }
    }

    fn heal_steps(self) -> Vec<u32> {
        match self {
            Self::RepeatedFlap => FLAP_HEAL_STEPS.to_vec(),
            _ => self.heal_step().into_iter().collect(),
        }
    }

    const fn drop_policy(self) -> DropPolicy {
        match self {
            Self::ShortHealContinue
            | Self::PermanentContinue
            | Self::Duplication
            | Self::Reorder
            | Self::BurstLoss
            | Self::AsymmetricDelay
            | Self::RepeatedFlap
            | Self::PollStall => DropPolicy::ContinueWithout,
            Self::LongHealHalt | Self::PermanentHalt => DropPolicy::Halt,
        }
    }

    const fn base_expectation(self) -> Expectation {
        match self {
            Self::ShortHealContinue
            | Self::Duplication
            | Self::Reorder
            | Self::BurstLoss
            | Self::AsymmetricDelay
            | Self::RepeatedFlap
            | Self::PollStall => Expectation::BoundedRecovery,
            Self::LongHealHalt | Self::PermanentHalt | Self::PermanentContinue => {
                Expectation::SafeFailure
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
enum Expectation {
    BoundedRecovery,
    SafeFailure,
    StateCorruption { peer: usize, from_frame: i32 },
    ChecksumCorruption { peer: usize, from_frame: i32 },
}

impl Expectation {
    const fn expected_classification(self) -> Classification {
        match self {
            Self::BoundedRecovery => Classification::BoundedRecovery,
            Self::SafeFailure => Classification::SafeFailure,
            Self::StateCorruption { .. } | Self::ChecksumCorruption { .. } => {
                Classification::Defect
            },
        }
    }

    const fn is_checksum_only(self) -> bool {
        matches!(self, Self::ChecksumCorruption { .. })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
enum Classification {
    BoundedRecovery,
    SafeFailure,
    Defect,
    Invalid,
}

#[derive(Clone, Copy, Debug)]
struct SweepCase {
    n_players: usize,
    scenario: ScenarioKind,
    expectation: Expectation,
}

impl SweepCase {
    const fn base(n_players: usize, scenario: ScenarioKind) -> Self {
        Self {
            n_players,
            scenario,
            expectation: scenario.base_expectation(),
        }
    }

    fn base_id(&self) -> String {
        format!("n{}-{}", self.n_players, self.scenario.label())
    }

    fn id(&self) -> String {
        let suffix = match self.expectation {
            Expectation::BoundedRecovery | Expectation::SafeFailure => "",
            Expectation::StateCorruption { .. } => "-state-control",
            Expectation::ChecksumCorruption { .. } => "-checksum-control",
        };
        format!("{}{suffix}", self.base_id())
    }

    fn matched_uninjected_row(&self) -> Option<String> {
        match self.expectation {
            Expectation::BoundedRecovery | Expectation::SafeFailure => None,
            Expectation::StateCorruption { .. } | Expectation::ChecksumCorruption { .. } => {
                Some(self.base_id())
            },
        }
    }
}

fn base_cases(n_players: usize) -> Vec<SweepCase> {
    ScenarioKind::ALL
        .into_iter()
        .map(|scenario| SweepCase::base(n_players, scenario))
        .collect()
}

fn controls(n_players: usize) -> Vec<SweepCase> {
    let state = |scenario, from_frame| SweepCase {
        n_players,
        scenario,
        expectation: Expectation::StateCorruption {
            peer: CORRUPTED_PEER,
            from_frame,
        },
    };
    match n_players {
        2 => vec![
            state(ScenarioKind::PermanentHalt, CORRUPTION_FRAME),
            state(ScenarioKind::LongHealHalt, CORRUPTION_FRAME),
            SweepCase {
                n_players,
                scenario: ScenarioKind::ShortHealContinue,
                expectation: Expectation::ChecksumCorruption {
                    peer: CORRUPTED_PEER,
                    from_frame: CORRUPTION_FRAME,
                },
            },
            state(ScenarioKind::Duplication, IN_WINDOW_CORRUPTION_FRAME),
            state(ScenarioKind::Reorder, IN_WINDOW_CORRUPTION_FRAME),
            state(ScenarioKind::BurstLoss, IN_WINDOW_CORRUPTION_FRAME),
            state(ScenarioKind::AsymmetricDelay, IN_WINDOW_CORRUPTION_FRAME),
            state(ScenarioKind::RepeatedFlap, IN_WINDOW_CORRUPTION_FRAME),
            SweepCase {
                n_players,
                scenario: ScenarioKind::PollStall,
                expectation: Expectation::StateCorruption {
                    peer: CORRUPTED_PEER,
                    from_frame: IN_WINDOW_CORRUPTION_FRAME,
                },
            },
        ],
        16 => vec![
            state(ScenarioKind::PermanentHalt, CORRUPTION_FRAME),
            state(ScenarioKind::Duplication, IN_WINDOW_CORRUPTION_FRAME),
            state(ScenarioKind::Reorder, IN_WINDOW_CORRUPTION_FRAME),
            state(ScenarioKind::BurstLoss, IN_WINDOW_CORRUPTION_FRAME),
            state(ScenarioKind::AsymmetricDelay, IN_WINDOW_CORRUPTION_FRAME),
            state(ScenarioKind::RepeatedFlap, IN_WINDOW_CORRUPTION_FRAME),
            SweepCase {
                n_players,
                scenario: ScenarioKind::PollStall,
                expectation: Expectation::StateCorruption {
                    peer: CORRUPTED_PEER,
                    from_frame: IN_WINDOW_CORRUPTION_FRAME,
                },
            },
        ],
        _ => Vec::new(),
    }
}

fn pr_cases() -> Vec<SweepCase> {
    let mut cases = base_cases(2);
    cases.extend(controls(2));
    cases
}

fn full_cases() -> Vec<SweepCase> {
    let mut cases = Vec::new();
    for n_players in ALL_SIZES {
        cases.extend(base_cases(n_players));
        cases.extend(controls(n_players));
    }
    cases
}

/// Directed links crossing the declared symmetric component boundary. Even
/// partition rows use a two-component bisection. Odd permanent-ContinueWithout
/// rows use singleton isolation: a bisection would let a majority commit and
/// continue, which needs the component-scoped oracle left to the CAP/membership
/// plan. Directed duplication, reordering, burst loss, and delay have no blocked-link premise.
fn intended_blocked_links(n_players: usize, scenario: ScenarioKind) -> BTreeSet<(usize, usize)> {
    if matches!(
        scenario,
        ScenarioKind::Duplication
            | ScenarioKind::Reorder
            | ScenarioKind::BurstLoss
            | ScenarioKind::AsymmetricDelay
            | ScenarioKind::PollStall
    ) {
        return BTreeSet::new();
    }
    let split = n_players / 2;
    let isolate_each = scenario == ScenarioKind::PermanentContinue && n_players % 2 == 1;
    let mut links = BTreeSet::new();
    for from in 0..n_players {
        for to in 0..n_players {
            if from != to && (isolate_each || ((from < split) != (to < split))) {
                links.insert((from, to));
            }
        }
    }
    links
}

fn intended_duplication_links(scenario: ScenarioKind) -> BTreeSet<(usize, usize)> {
    if scenario == ScenarioKind::Duplication {
        BTreeSet::from([(0, 1)])
    } else {
        BTreeSet::new()
    }
}

fn intended_reorder_links(scenario: ScenarioKind) -> BTreeSet<(usize, usize)> {
    if scenario == ScenarioKind::Reorder {
        BTreeSet::from([(0, 1)])
    } else {
        BTreeSet::new()
    }
}

fn intended_reorder_delivery_links(scenario: ScenarioKind) -> BTreeSet<(usize, usize)> {
    if matches!(
        scenario,
        ScenarioKind::Reorder | ScenarioKind::AsymmetricDelay
    ) {
        BTreeSet::from([(0, 1)])
    } else {
        BTreeSet::new()
    }
}

fn intended_burst_loss_links(scenario: ScenarioKind) -> BTreeSet<(usize, usize)> {
    if scenario == ScenarioKind::BurstLoss {
        BTreeSet::from([(0, 1)])
    } else {
        BTreeSet::new()
    }
}

fn intended_asymmetric_delay_links(scenario: ScenarioKind) -> BTreeSet<(usize, usize)> {
    if scenario == ScenarioKind::AsymmetricDelay {
        BTreeSet::from([(0, 1)])
    } else {
        BTreeSet::new()
    }
}

fn intended_base_delay_links(scenario: ScenarioKind) -> BTreeSet<(usize, usize)> {
    if matches!(
        scenario,
        ScenarioKind::Reorder | ScenarioKind::AsymmetricDelay
    ) {
        BTreeSet::from([(0, 1)])
    } else {
        BTreeSet::new()
    }
}

fn intended_peer_stalls(scenario: ScenarioKind) -> Vec<(u32, usize, u32)> {
    if scenario == ScenarioKind::PollStall {
        POLL_STALL_STEPS
            .into_iter()
            .map(|step| (step, 1, REPEATED_CYCLE_STEPS))
            .collect()
    } else {
        Vec::new()
    }
}

fn reorder_policy() -> LinkPolicy {
    LinkPolicy {
        base_delay: Duration::from_millis(20),
        jitter: Duration::from_millis(80),
        ..LinkPolicy::clean()
    }
}

fn burst_loss_policy() -> LinkPolicy {
    LinkPolicy {
        burst_rate: 0.05,
        burst_len: 3,
        ..LinkPolicy::clean()
    }
}

fn asymmetric_delay_policy() -> LinkPolicy {
    LinkPolicy {
        base_delay: Duration::from_millis(200),
        ..LinkPolicy::clean()
    }
}

fn materialize(case: SweepCase) -> (Schedule, RunOptions) {
    let n = case.n_players;
    let config = SimConfig {
        n_players: n,
        steps: STEPS,
        noise: BackgroundNoise::Clean,
        disconnect_behavior: case.scenario.drop_policy(),
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
    let mut events: Vec<(u32, ScheduleEvent)> = intended_blocked_links(n, case.scenario)
        .into_iter()
        .map(|(from, to)| {
            (
                CUT_STEP,
                ScheduleEvent::Block {
                    from,
                    to,
                    blocked: true,
                },
            )
        })
        .collect();
    if case.scenario == ScenarioKind::RepeatedFlap {
        events.extend(FLAP_CUT_STEPS.into_iter().skip(1).flat_map(|step| {
            intended_blocked_links(n, case.scenario)
                .into_iter()
                .map(move |(from, to)| {
                    (
                        step,
                        ScheduleEvent::Block {
                            from,
                            to,
                            blocked: true,
                        },
                    )
                })
        }));
    }
    events.extend(
        intended_duplication_links(case.scenario)
            .into_iter()
            .map(|(from, to)| {
                (
                    CUT_STEP,
                    ScheduleEvent::SetLink {
                        from,
                        to,
                        policy: LinkPolicy {
                            dup_rate: 1.0,
                            ..LinkPolicy::clean()
                        },
                    },
                )
            }),
    );
    events.extend(
        intended_reorder_links(case.scenario)
            .into_iter()
            .map(|(from, to)| {
                (
                    CUT_STEP,
                    ScheduleEvent::SetLink {
                        from,
                        to,
                        policy: reorder_policy(),
                    },
                )
            }),
    );
    events.extend(
        intended_burst_loss_links(case.scenario)
            .into_iter()
            .map(|(from, to)| {
                (
                    CUT_STEP,
                    ScheduleEvent::SetLink {
                        from,
                        to,
                        policy: burst_loss_policy(),
                    },
                )
            }),
    );
    events.extend(
        intended_asymmetric_delay_links(case.scenario)
            .into_iter()
            .map(|(from, to)| {
                (
                    CUT_STEP,
                    ScheduleEvent::SetLink {
                        from,
                        to,
                        policy: asymmetric_delay_policy(),
                    },
                )
            }),
    );
    events.extend(
        intended_peer_stalls(case.scenario)
            .into_iter()
            .map(|(step, peer, steps)| (step, ScheduleEvent::PeerStall { peer, steps })),
    );
    for heal in case.scenario.heal_steps() {
        events.push((heal, ScheduleEvent::HealAll));
    }
    events.sort_by_key(|(step, _)| *step);
    let schedule = Schedule {
        schema_version: SCHEDULE_SCHEMA_VERSION,
        seed: SEED,
        link_seed: SEED ^ 0x1111_2222_3333_4444,
        config,
        initial_links,
        events,
        // Inert provenance only. Recovery below is derived from HealAll events.
        heal_at: case.scenario.heal_step().unwrap_or(STEPS),
    };
    let mut options = RunOptions {
        receive_message_limit: Some(RECEIVE_MESSAGE_LIMIT),
        disconnect_timeout: Some(DISCONNECT_TIMEOUT),
        probe_confirmed_at: Some(PRE_CUT_PROBE_STEP),
        ..RunOptions::default()
    };
    match case.expectation {
        Expectation::BoundedRecovery | Expectation::SafeFailure => {},
        Expectation::StateCorruption { peer, from_frame } => {
            // The oracle's peer is the observer. For poll stalls, corrupt the
            // surviving peer so injection does not wait for peer 1 to resume.
            let injected_peer = if case.scenario == ScenarioKind::PollStall {
                0
            } else {
                peer
            };
            options.corrupt_state_from = Some((injected_peer, from_frame));
        },
        Expectation::ChecksumCorruption { peer, from_frame } => {
            options.corrupt_checksum_from = Some((peer, from_frame));
        },
    }
    (schedule, options)
}

fn actual_heal_steps(schedule: &Schedule) -> Vec<u32> {
    schedule
        .events
        .iter()
        .filter_map(|(step, event)| matches!(event, ScheduleEvent::HealAll).then_some(*step))
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct VerdictEvidence {
    failures: Vec<ArtifactFailure>,
    recovered_within_b: Option<bool>,
    /// The default oracle allowlist is empty. Full diagnostics are retained if
    /// that ever changes, so this remains the complete verdict surface.
    violation_allowlist_hits: Vec<String>,
}

impl From<&super::harness::oracle::Verdict> for VerdictEvidence {
    fn from(verdict: &super::harness::oracle::Verdict) -> Self {
        Self {
            failures: verdict
                .failures
                .iter()
                .map(|failure| ArtifactFailure {
                    class: FailureClass::from(failure),
                    // Unlike the size-capped standalone failure artifact, this
                    // ledger preserves the complete original diagnostic.
                    details: format!("{failure:?}"),
                })
                .collect(),
            recovered_within_b: verdict.recovered_within_b,
            violation_allowlist_hits: verdict
                .violation_allowlist_hits
                .iter()
                .map(|hit| format!("{hit:?}"))
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct DirectedCount {
    from: usize,
    to: usize,
    count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct DirectedWire {
    from: usize,
    to: usize,
    totals: PeerWireTotals,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct EventCount {
    kind: EventKind,
    count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct PeerEventCounts {
    peer: usize,
    counts: Vec<EventCount>,
}

fn directed_counts(map: &BTreeMap<(usize, usize), u64>) -> Vec<DirectedCount> {
    map.iter()
        .map(|(&(from, to), &count)| DirectedCount { from, to, count })
        .collect()
}

fn directed_wire(map: &BTreeMap<(usize, usize), PeerWireTotals>) -> Vec<DirectedWire> {
    map.iter()
        .map(|(&(from, to), totals)| DirectedWire {
            from,
            to,
            totals: totals.clone(),
        })
        .collect()
}

fn event_counts(map: &BTreeMap<EventKind, u64>) -> Vec<EventCount> {
    map.iter()
        .map(|(&kind, &count)| EventCount { kind, count })
        .collect()
}

/// Minimum frame named by a consistency observation, not a claim about causal
/// ordering. Checksum-only injection has no state-divergence frame to report.
fn first_observed_divergence(failures: &[OracleFailure], checksum_only: bool) -> Option<i32> {
    failures
        .iter()
        .filter_map(|failure| match failure {
            OracleFailure::ReferenceInput { frame, .. }
            | OracleFailure::ReferenceState { frame, .. }
            | OracleFailure::ReferenceStateMissing { frame, .. }
            | OracleFailure::ConfirmedInputDivergence { frame, .. }
            | OracleFailure::StateDivergence { frame, .. }
            | OracleFailure::SpectatorInputDivergence { frame, .. } => Some(*frame),
            OracleFailure::InbandDesyncDetected { frame, .. }
            | OracleFailure::SpectatorDivergenceEvent { frame, .. } => {
                (!checksum_only).then_some(*frame)
            },
            OracleFailure::ChecksumMismatchMetric { .. }
            | OracleFailure::ConfirmedInputUnavailable { .. }
            | OracleFailure::SessionError { .. }
            | OracleFailure::Violation { .. }
            | OracleFailure::InvalidViolationSource { .. }
            | OracleFailure::EndProgress { .. }
            | OracleFailure::NoLivePeers { .. }
            | OracleFailure::PostHealLiveness { .. }
            | OracleFailure::FreezeFrameDivergence { .. }
            | OracleFailure::FreezeFrameMissing { .. }
            | OracleFailure::SpectatorSessionError { .. }
            | OracleFailure::SpectatorProgressMissing { .. }
            | OracleFailure::SpectatorMeshCanonMissing { .. } => None,
        })
        .min()
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct RunEvidence {
    replay_options: RunOptions,
    replay_input_width_bytes: u32,
    verdict: VerdictEvidence,
    trace_hash: u64,
    final_confirmed: Vec<i32>,
    trace_tail: Vec<TraceSnapshot>,
    probe_confirmed: Vec<i32>,
    probe_peer_wire_by_link: Vec<DirectedWire>,
    confirmed_at_heal: Vec<i32>,
    confirmed_after_recovery: Vec<i32>,
    recovered_within_b: Option<bool>,
    blocked_drops_by_link: Vec<DirectedCount>,
    duplicated_sends_by_link: Vec<DirectedCount>,
    reordered_deliveries_by_link: Vec<DirectedCount>,
    base_delay_applications_by_link: Vec<DirectedCount>,
    burst_loss_events_by_link: Vec<DirectedCount>,
    max_consecutive_input_policy_loss_run_by_link: Vec<DirectedCount>,
    peer_event_counts: Vec<EventCount>,
    peer_event_counts_by_peer: Vec<PeerEventCounts>,
    max_confirmation_stall_steps_by_peer: Vec<u64>,
    metrics: Vec<SessionMetrics>,
    peer_wire: Vec<PeerWireTotals>,
    receive_stats_by_peer: Vec<SimReceiveStats>,
    first_running_step: Vec<Option<u32>>,
    first_synchronized_step: Vec<Vec<Option<u32>>>,
    peer_stalls: Vec<PeerStallObservation>,
    heal_recoveries: Vec<HealRecoveryObservation>,
    net: TraceNetStats,
    first_observed_divergence: Option<i32>,
}

impl RunEvidence {
    fn from_report(report: &RunReport, checksum_only: bool) -> Self {
        Self {
            replay_options: report.replay_options.clone(),
            replay_input_width_bytes: report.replay_input_width_bytes,
            verdict: VerdictEvidence::from(&report.verdict),
            trace_hash: report.trace_hash,
            final_confirmed: report.final_confirmed.clone(),
            trace_tail: report.trace_tail.clone(),
            probe_confirmed: report.probe_confirmed.clone(),
            probe_peer_wire_by_link: directed_wire(&report.probe_peer_wire_by_link),
            confirmed_at_heal: report.confirmed_at_heal.clone(),
            confirmed_after_recovery: report.confirmed_after_recovery.clone(),
            recovered_within_b: report.recovered_within_b,
            blocked_drops_by_link: directed_counts(&report.blocked_drops_by_link),
            duplicated_sends_by_link: report
                .link_stats_by_link
                .iter()
                .map(|(&(from, to), stats)| DirectedCount {
                    from,
                    to,
                    count: stats.duplicated_sends,
                })
                .collect(),
            reordered_deliveries_by_link: report
                .link_stats_by_link
                .iter()
                .map(|(&(from, to), stats)| DirectedCount {
                    from,
                    to,
                    count: stats.reordered_deliveries,
                })
                .collect(),
            base_delay_applications_by_link: report
                .link_stats_by_link
                .iter()
                .map(|(&(from, to), stats)| DirectedCount {
                    from,
                    to,
                    count: stats.base_delay_applications,
                })
                .collect(),
            burst_loss_events_by_link: report
                .link_stats_by_link
                .iter()
                .map(|(&(from, to), stats)| DirectedCount {
                    from,
                    to,
                    count: stats.burst_loss_events,
                })
                .collect(),
            max_consecutive_input_policy_loss_run_by_link: report
                .link_stats_by_link
                .iter()
                .map(|(&(from, to), stats)| DirectedCount {
                    from,
                    to,
                    count: stats.max_consecutive_input_policy_loss_run,
                })
                .collect(),
            peer_event_counts: event_counts(&report.peer_event_counts),
            peer_event_counts_by_peer: report
                .peer_event_counts_by_peer
                .iter()
                .enumerate()
                .map(|(peer, counts)| PeerEventCounts {
                    peer,
                    counts: event_counts(counts),
                })
                .collect(),
            max_confirmation_stall_steps_by_peer: report
                .max_confirmation_stall_steps_by_peer
                .clone(),
            metrics: report.metrics.clone(),
            peer_wire: report.peer_wire.clone(),
            receive_stats_by_peer: report.receive_stats_by_peer.clone(),
            first_running_step: report.first_running_step.clone(),
            first_synchronized_step: report.first_synchronized_step.clone(),
            peer_stalls: report.peer_stalls.clone(),
            heal_recoveries: report.heal_recoveries.clone(),
            net: TraceNetStats::from(report.net_stats),
            first_observed_divergence: first_observed_divergence(
                &report.verdict.failures,
                checksum_only,
            ),
        }
    }

    fn peer_event_count(&self, kind: EventKind) -> u64 {
        self.peer_event_counts
            .iter()
            .filter(|entry| entry.kind == kind)
            .fold(0_u64, |total, entry| total.saturating_add(entry.count))
    }

    fn peer_dropped_count(&self) -> u64 {
        self.peer_event_count(EventKind::PeerDropped)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct TerminalPlateau {
    start_step: u32,
    end_step: u32,
    observed_steps: u32,
    confirmed_frames: Vec<i32>,
    session_states: Vec<TraceSessionState>,
}

/// Longest identical suffix in the bounded trace. This is finite observed
/// evidence only; it says nothing about infinite-time liveness.
fn terminal_plateau(trace_tail: &[TraceSnapshot]) -> Option<TerminalPlateau> {
    let final_snapshot = trace_tail.last()?;
    let mut start = trace_tail.len() - 1;
    for index in (0..trace_tail.len() - 1).rev() {
        let snapshot = &trace_tail[index];
        if snapshot.confirmed_frames == final_snapshot.confirmed_frames
            && snapshot.session_states == final_snapshot.session_states
            && snapshot.dead == final_snapshot.dead
            && snapshot.game_states == final_snapshot.game_states
        {
            start = index;
        } else {
            break;
        }
    }
    Some(TerminalPlateau {
        start_step: trace_tail[start].step,
        end_step: final_snapshot.step,
        observed_steps: u32::try_from(trace_tail.len() - start).unwrap_or(u32::MAX),
        confirmed_frames: final_snapshot.confirmed_frames.clone(),
        session_states: final_snapshot.session_states.clone(),
    })
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct NegativeControlMatch {
    class: FailureClass,
    peer: usize,
    frame: i32,
    shared_confirmed_prefix_floor: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct Observation {
    evidence: RunEvidence,
    premise_violations: Vec<String>,
    terminal_plateau: Option<TerminalPlateau>,
    negative_control_match: Option<NegativeControlMatch>,
    classification: Classification,
}

fn stall_premise_violations(case: SweepCase, evidence: &RunEvidence) -> Vec<String> {
    let expected = intended_peer_stalls(case.scenario);
    let mut violations = Vec::new();
    if evidence.peer_stalls.len() != expected.len() {
        violations.push(format!(
            "actual stalls {} do not match declared stalls {}",
            evidence.peer_stalls.len(),
            expected.len()
        ));
    }
    for (index, (stall, (start_step, expected_peer, expected_steps))) in
        evidence.peer_stalls.iter().zip(expected).enumerate()
    {
        let end_step = start_step.saturating_add(expected_steps);
        if stall.peer != expected_peer
            || stall.start_step != start_step
            || stall.end_step != end_step
            || stall.observed_frozen_steps != expected_steps
            || stall.confirmed_at_freeze_end != stall.confirmed_at_start
            || stall.current_frame_at_freeze_end != stall.current_frame_at_start
            || stall.resume_step != Some(end_step)
            || stall.confirmed_at_resume.is_none()
        {
            violations.push(format!(
                "stall {index} did not preserve the declared half-open freeze/resume window: {stall:?}"
            ));
        }
    }
    violations
}

fn recovery_premise_violations(case: SweepCase, evidence: &RunEvidence) -> Vec<String> {
    let expected_heals = case.scenario.heal_steps();
    let mut violations = Vec::new();
    let actual_heals: Vec<u32> = evidence
        .heal_recoveries
        .iter()
        .map(|recovery| recovery.heal_step)
        .collect();
    if actual_heals != expected_heals {
        violations.push(format!(
            "recovery anchors {actual_heals:?} do not match declared heals {expected_heals:?}"
        ));
    }
    for (index, recovery) in evidence.heal_recoveries.iter().enumerate() {
        if recovery.confirmed_at_heal.len() != case.n_players {
            violations.push(format!(
                "heal {} lacks a complete confirmed baseline",
                recovery.heal_step
            ));
        }
        if recovery
            .first_all_running_step
            .is_some_and(|step| step < recovery.heal_step)
            || recovery
                .first_all_confirmed_advance_step
                .is_some_and(|step| step <= recovery.heal_step)
        {
            violations.push(format!(
                "heal {} has a recovery timestamp outside its causal window: {recovery:?}",
                recovery.heal_step
            ));
        }
        if case.scenario.base_expectation() == Expectation::BoundedRecovery
            && (recovery.first_all_running_step.is_none()
                || recovery.first_all_confirmed_advance_step.is_none())
        {
            violations.push(format!(
                "bounded-recovery heal {} never reached all-running/all-confirmed progress: {recovery:?}",
                recovery.heal_step
            ));
        }
        if case.scenario == ScenarioKind::RepeatedFlap {
            let next_cut = FLAP_CUT_STEPS.get(index + 1).copied().unwrap_or(STEPS);
            if recovery
                .first_all_confirmed_advance_step
                .is_none_or(|step| step >= next_cut)
            {
                violations.push(format!(
                    "flap heal {} did not confirm progress before step {next_cut}: {recovery:?}",
                    recovery.heal_step
                ));
            }
        }
    }
    violations
}

fn common_premise_violations(
    case: SweepCase,
    schedule: &Schedule,
    options: &RunOptions,
    evidence: &RunEvidence,
) -> Vec<String> {
    let n = case.n_players;
    let mut violations = Vec::new();
    if evidence.replay_options != *options {
        violations.push("report replay options differ from materialized options".to_owned());
    }
    if evidence.replay_input_width_bytes != StubInput::WIDTH_BYTES {
        violations.push(format!(
            "input width {} is not the declared 4-byte StubInput",
            evidence.replay_input_width_bytes
        ));
    }
    let expected_heals = case.scenario.heal_steps();
    if actual_heal_steps(schedule) != expected_heals {
        violations.push("actual HealAll events differ from the declared scenario".to_owned());
    }
    if evidence.max_confirmation_stall_steps_by_peer.len() != n {
        violations.push("confirmation-stall duration evidence is missing peers".to_owned());
    } else if case.scenario == ScenarioKind::PollStall
        && evidence.max_confirmation_stall_steps_by_peer[1] < u64::from(REPEATED_CYCLE_STEPS)
    {
        violations.push(
            "poll stalls did not produce the declared confirmation-stall duration".to_owned(),
        );
    }
    if evidence.first_running_step.len() != n {
        violations.push("pre-cut Running evidence is missing peer entries".to_owned());
    } else {
        for (peer, step) in evidence.first_running_step.iter().enumerate() {
            if !step.is_some_and(|step| step <= PRE_CUT_PROBE_STEP) {
                violations.push(format!("peer {peer} was not Running before the cut"));
            }
        }
    }
    if evidence.first_synchronized_step.len() != n
        || evidence
            .first_synchronized_step
            .iter()
            .any(|row| row.len() != n)
    {
        violations.push("pre-cut Synchronized evidence has the wrong shape".to_owned());
    } else {
        for from in 0..n {
            for to in 0..n {
                if from != to
                    && evidence.first_synchronized_step[from][to]
                        .is_none_or(|step| step > PRE_CUT_PROBE_STEP)
                {
                    violations.push(format!("directed pair {from}->{to} did not sync pre-cut"));
                }
            }
        }
    }
    if evidence.probe_confirmed.len() != n {
        violations.push("pre-cut confirmed-prefix probe is missing peers".to_owned());
    } else {
        let minimum = evidence.probe_confirmed.iter().copied().min().unwrap_or(-1);
        if minimum < MIN_END_CONFIRMED {
            violations.push(format!(
                "pre-cut confirmed prefix is not useful (minimum {minimum})"
            ));
        }
    }
    if evidence.probe_peer_wire_by_link.len() != n.saturating_mul(n - 1)
        || evidence
            .probe_peer_wire_by_link
            .iter()
            .any(|link| link.totals.packets_sent == 0 || link.totals.packets_received == 0)
    {
        violations.push("pre-cut directed wire evidence is incomplete or empty".to_owned());
    }
    let observed_blocked: BTreeSet<(usize, usize)> = evidence
        .blocked_drops_by_link
        .iter()
        .filter(|entry| entry.count > 0)
        .map(|entry| (entry.from, entry.to))
        .collect();
    let intended_blocked = intended_blocked_links(n, case.scenario);
    if observed_blocked != intended_blocked {
        violations.push(format!(
            "nonzero blocked-drop links {observed_blocked:?} do not equal intended links {intended_blocked:?}"
        ));
    }
    let observed_duplicated: BTreeSet<(usize, usize)> = evidence
        .duplicated_sends_by_link
        .iter()
        .filter(|entry| entry.count > 0)
        .map(|entry| (entry.from, entry.to))
        .collect();
    let intended_duplicated = intended_duplication_links(case.scenario);
    if observed_duplicated != intended_duplicated {
        violations.push(format!(
            "nonzero duplicated-send links {observed_duplicated:?} do not equal intended links {intended_duplicated:?}"
        ));
    }
    let duplicated_sum = evidence
        .duplicated_sends_by_link
        .iter()
        .fold(0_u64, |total, entry| total.saturating_add(entry.count));
    if evidence.net.duplicated != duplicated_sum {
        violations.push(format!(
            "global duplicated sends {} do not equal per-link sum {duplicated_sum}",
            evidence.net.duplicated
        ));
    }
    let observed_reordered: BTreeSet<(usize, usize)> = evidence
        .reordered_deliveries_by_link
        .iter()
        .filter(|entry| entry.count > 0)
        .map(|entry| (entry.from, entry.to))
        .collect();
    let intended_reordered = intended_reorder_delivery_links(case.scenario);
    if observed_reordered != intended_reordered {
        violations.push(format!(
            "nonzero reordered-delivery links {observed_reordered:?} do not equal intended links {intended_reordered:?}"
        ));
    }
    let reordered_sum = evidence
        .reordered_deliveries_by_link
        .iter()
        .fold(0_u64, |total, entry| total.saturating_add(entry.count));
    if evidence.net.reordered_deliveries != reordered_sum {
        violations.push(format!(
            "global reordered deliveries {} do not equal per-link sum {reordered_sum}",
            evidence.net.reordered_deliveries
        ));
    }
    let observed_base_delay: BTreeSet<(usize, usize)> = evidence
        .base_delay_applications_by_link
        .iter()
        .filter(|entry| entry.count > 0)
        .map(|entry| (entry.from, entry.to))
        .collect();
    let intended_base_delay = intended_base_delay_links(case.scenario);
    if observed_base_delay != intended_base_delay {
        violations.push(format!(
            "nonzero base-delay links {observed_base_delay:?} do not equal intended links {intended_base_delay:?}"
        ));
    }
    let base_delay_sum = evidence
        .base_delay_applications_by_link
        .iter()
        .fold(0_u64, |total, entry| total.saturating_add(entry.count));
    if evidence.net.base_delay_applications != base_delay_sum {
        violations.push(format!(
            "global base-delay applications {} do not equal per-link sum {base_delay_sum}",
            evidence.net.base_delay_applications
        ));
    }
    let observed_burst_loss: BTreeSet<(usize, usize)> = evidence
        .burst_loss_events_by_link
        .iter()
        .filter(|entry| entry.count > 0)
        .map(|entry| (entry.from, entry.to))
        .collect();
    let intended_burst_loss = intended_burst_loss_links(case.scenario);
    if observed_burst_loss != intended_burst_loss {
        violations.push(format!(
            "nonzero burst-loss-event links {observed_burst_loss:?} do not equal intended links {intended_burst_loss:?}"
        ));
    }
    let burst_loss_sum = evidence
        .burst_loss_events_by_link
        .iter()
        .fold(0_u64, |total, entry| total.saturating_add(entry.count));
    if evidence.net.burst_loss_events != burst_loss_sum {
        violations.push(format!(
            "global burst-loss events {} do not equal per-link sum {burst_loss_sum}",
            evidence.net.burst_loss_events
        ));
    }
    let observed_input_loss_runs: BTreeSet<(usize, usize)> = evidence
        .max_consecutive_input_policy_loss_run_by_link
        .iter()
        .filter(|entry| entry.count > 0)
        .map(|entry| (entry.from, entry.to))
        .collect();
    if observed_input_loss_runs != intended_burst_loss {
        violations.push(format!(
            "input-policy loss runs {observed_input_loss_runs:?} do not equal intended burst links {intended_burst_loss:?}"
        ));
    }
    if case.scenario == ScenarioKind::BurstLoss {
        let max_targeted_run = evidence
            .max_consecutive_input_policy_loss_run_by_link
            .iter()
            .find(|entry| (entry.from, entry.to) == (0, 1))
            .map_or(0, |entry| entry.count);
        if max_targeted_run < 3 {
            violations.push(format!(
                "directed burst premise requires a maximum actual 0->1 input-loss run of at least length 3 (maximum {max_targeted_run}); back-to-back bursts may coalesce"
            ));
        }
        let burst_events = evidence.net.burst_loss_events;
        if burst_events > 0 {
            let completed_bursts = burst_events.saturating_sub(1);
            let drop_floor_exclusive = 3_u64.saturating_mul(completed_bursts);
            let drop_ceiling_inclusive = 3_u64.saturating_mul(burst_events);
            let policy_drops = evidence.net.dropped_by_policy;
            if policy_drops <= drop_floor_exclusive || policy_drops > drop_ceiling_inclusive {
                violations.push(format!(
                    "global policy drops {policy_drops} are inconsistent with {burst_events} burst-loss events: expected {drop_floor_exclusive} < drops <= {drop_ceiling_inclusive}; HealAll may truncate only the final burst"
                ));
            }
        }
    }
    violations.extend(stall_premise_violations(case, evidence));
    violations.extend(recovery_premise_violations(case, evidence));
    violations
}

fn all_peers_advance(evidence: &RunEvidence, required: i32) -> Option<bool> {
    let n = evidence.final_confirmed.len();
    if evidence.confirmed_at_heal.len() != n || evidence.confirmed_after_recovery.len() != n {
        return None;
    }
    Some(
        evidence
            .confirmed_at_heal
            .iter()
            .zip(&evidence.confirmed_after_recovery)
            .all(|(&at_heal, &after)| after.saturating_sub(at_heal) >= required),
    )
}

fn no_peer_advances_past_floor(evidence: &RunEvidence) -> Option<bool> {
    let n = evidence.final_confirmed.len();
    if evidence.confirmed_at_heal.len() != n || evidence.confirmed_after_recovery.len() != n {
        return None;
    }
    Some(
        evidence
            .confirmed_at_heal
            .iter()
            .zip(&evidence.confirmed_after_recovery)
            .all(|(&at_heal, &after)| after.saturating_sub(at_heal) < POST_HEAL_MIN_ADVANCE),
    )
}

fn final_snapshot_has_state(evidence: &RunEvidence, expected: TraceSessionState) -> Option<bool> {
    let final_snapshot = evidence.trace_tail.last()?;
    let n = evidence.final_confirmed.len();
    Some(
        final_snapshot.session_states.len() == n
            && final_snapshot.dead.len() == n
            && final_snapshot.confirmed_frames == evidence.final_confirmed
            && final_snapshot
                .session_states
                .iter()
                .all(|state| *state == expected)
            && final_snapshot.dead.iter().all(|dead| !*dead),
    )
}

fn classify_bounded_recovery(
    evidence: &RunEvidence,
    premise_violations: &[String],
) -> Classification {
    if !premise_violations.is_empty() {
        return Classification::Invalid;
    }
    if !evidence.verdict.failures.is_empty()
        || !evidence.verdict.violation_allowlist_hits.is_empty()
        || evidence.peer_dropped_count() != 0
        || evidence.recovered_within_b != evidence.verdict.recovered_within_b
    {
        return Classification::Defect;
    }
    let Some(recovered) = evidence.recovered_within_b else {
        return Classification::Invalid;
    };
    let Some(all_advanced) = all_peers_advance(evidence, POST_HEAL_MIN_ADVANCE) else {
        return Classification::Invalid;
    };
    let Some(final_running) = final_snapshot_has_state(evidence, TraceSessionState::Running) else {
        return Classification::Invalid;
    };
    if recovered
        && all_advanced
        && final_running
        && !evidence.final_confirmed.is_empty()
        && evidence
            .final_confirmed
            .iter()
            .all(|frame| *frame >= MIN_END_CONFIRMED)
    {
        Classification::BoundedRecovery
    } else {
        Classification::Defect
    }
}

fn typed_safe_failure_diagnostics_hold(
    report: &RunReport,
    healed: bool,
    allow_drop_timeout: bool,
    recovery_window_steps: u32,
) -> bool {
    let n = report.final_confirmed.len();
    let mut saw_end_progress = false;
    let mut saw_post_heal = false;
    let mut end_peers = BTreeSet::new();
    let mut post_heal_peers = BTreeSet::new();
    for failure in &report.verdict.failures {
        match failure {
            OracleFailure::Violation {
                source: ViolationSource::Peer(peer),
                violation,
            } if allow_drop_timeout => {
                if *peer >= n
                    || !violation.starts_with("[Error/NetworkProtocol] coordinated graceful drop ")
                    || !violation.contains(" failed closed: Timeout;")
                {
                    return false;
                }
            },
            OracleFailure::EndProgress {
                peer,
                state,
                confirmed,
                required,
            } => {
                saw_end_progress = true;
                if *peer >= n
                    || *state != SessionState::Synchronizing
                    || *confirmed != report.final_confirmed[*peer]
                    || *required != MIN_END_CONFIRMED
                    || !end_peers.insert(*peer)
                {
                    return false;
                }
            },
            OracleFailure::PostHealLiveness {
                peer,
                at_heal,
                after,
                advanced,
                required,
                window_steps,
            } if healed => {
                saw_post_heal = true;
                if *peer >= n
                    || Some(*at_heal) != report.confirmed_at_heal.get(*peer).copied()
                    || Some(*after) != report.confirmed_after_recovery.get(*peer).copied()
                    || *advanced != after.saturating_sub(*at_heal)
                    || *advanced >= POST_HEAL_MIN_ADVANCE
                    || *required != POST_HEAL_MIN_ADVANCE
                    || *window_steps != recovery_window_steps
                    || !post_heal_peers.insert(*peer)
                {
                    return false;
                }
            },
            OracleFailure::PostHealLiveness { .. } => return false,
            OracleFailure::ReferenceInput { .. }
            | OracleFailure::ReferenceState { .. }
            | OracleFailure::ReferenceStateMissing { .. }
            | OracleFailure::ConfirmedInputDivergence { .. }
            | OracleFailure::StateDivergence { .. }
            | OracleFailure::InbandDesyncDetected { .. }
            | OracleFailure::ChecksumMismatchMetric { .. }
            | OracleFailure::ConfirmedInputUnavailable { .. }
            | OracleFailure::SessionError { .. }
            | OracleFailure::Violation { .. }
            | OracleFailure::InvalidViolationSource { .. }
            | OracleFailure::NoLivePeers { .. }
            | OracleFailure::FreezeFrameDivergence { .. }
            | OracleFailure::FreezeFrameMissing { .. }
            | OracleFailure::SpectatorDivergenceEvent { .. }
            | OracleFailure::SpectatorSessionError { .. }
            | OracleFailure::SpectatorProgressMissing { .. }
            | OracleFailure::SpectatorMeshCanonMissing { .. }
            | OracleFailure::SpectatorInputDivergence { .. } => return false,
        }
    }
    saw_end_progress && (!healed || saw_post_heal)
}

fn classify_safe_failure(
    case: SweepCase,
    schedule: &Schedule,
    report: &RunReport,
    evidence: &RunEvidence,
    premise_violations: &[String],
    plateau: Option<&TerminalPlateau>,
) -> Classification {
    let healed = case.scenario.heal_step().is_some();
    if !premise_violations.is_empty() {
        return Classification::Invalid;
    }
    if !evidence.verdict.violation_allowlist_hits.is_empty()
        || evidence.peer_dropped_count() != 0
        || (case.scenario == ScenarioKind::PermanentContinue
            && evidence.peer_event_count(EventKind::Disconnected) != 0)
        || evidence.recovered_within_b != evidence.verdict.recovered_within_b
    {
        return Classification::Defect;
    }
    let Some(final_synchronizing) =
        final_snapshot_has_state(evidence, TraceSessionState::Synchronizing)
    else {
        return Classification::Invalid;
    };
    let Some(plateau) = plateau else {
        return Classification::Invalid;
    };
    if !typed_safe_failure_diagnostics_hold(
        report,
        healed,
        case.scenario == ScenarioKind::PermanentContinue,
        schedule.config.recovery_window_steps(),
    ) || !final_synchronizing
        || evidence.final_confirmed.is_empty()
        || plateau.observed_steps < MIN_TERMINAL_PLATEAU_STEPS
        || plateau.confirmed_frames != evidence.final_confirmed
    {
        return Classification::Defect;
    }

    if healed {
        let Some(recovered) = evidence.recovered_within_b else {
            return Classification::Invalid;
        };
        let Some(no_peer_advanced) = no_peer_advances_past_floor(evidence) else {
            return Classification::Invalid;
        };
        if recovered || !no_peer_advanced {
            return Classification::Defect;
        }
    } else if evidence.recovered_within_b.is_some()
        || !evidence.confirmed_at_heal.is_empty()
        || !evidence.confirmed_after_recovery.is_empty()
    {
        return Classification::Defect;
    }
    Classification::SafeFailure
}

fn negative_control_match(
    case: SweepCase,
    report: &RunReport,
    evidence: &RunEvidence,
) -> Option<NegativeControlMatch> {
    let shared_confirmed_prefix_floor = if matches!(
        case.scenario,
        ScenarioKind::Duplication
            | ScenarioKind::Reorder
            | ScenarioKind::BurstLoss
            | ScenarioKind::AsymmetricDelay
            | ScenarioKind::RepeatedFlap
            | ScenarioKind::PollStall
    ) {
        evidence.confirmed_at_heal.iter().copied().min()?
    } else {
        evidence.probe_confirmed.iter().copied().min()?
    };
    let plausible =
        |frame: i32, from_frame: i32| frame >= from_frame && frame <= shared_confirmed_prefix_floor;
    match case.expectation {
        Expectation::StateCorruption { peer, from_frame } => {
            if from_frame > shared_confirmed_prefix_floor || evidence.peer_dropped_count() != 0 {
                return None;
            }
            let frame = report
                .verdict
                .failures
                .iter()
                .filter_map(|failure| match failure {
                    OracleFailure::StateDivergence {
                        peer: observed,
                        frame,
                        ..
                    } if *observed == peer && plausible(*frame, from_frame) => Some(*frame),
                    _ => None,
                })
                .min()?;
            (evidence.first_observed_divergence == Some(frame)).then_some(NegativeControlMatch {
                class: FailureClass::StateDivergence,
                peer,
                frame,
                shared_confirmed_prefix_floor,
            })
        },
        Expectation::ChecksumCorruption { peer, from_frame } => {
            if from_frame > shared_confirmed_prefix_floor
                || evidence.peer_dropped_count() != 0
                || evidence.first_observed_divergence.is_some()
                || report.verdict.failures.iter().any(|failure| {
                    matches!(
                        failure,
                        OracleFailure::StateDivergence { .. }
                            | OracleFailure::ConfirmedInputDivergence { .. }
                    )
                })
            {
                return None;
            }
            let frame = report
                .verdict
                .failures
                .iter()
                .filter_map(|failure| match failure {
                    OracleFailure::InbandDesyncDetected {
                        peer: observed,
                        frame,
                    } if *observed == peer && plausible(*frame, from_frame) => Some(*frame),
                    _ => None,
                })
                .min()?;
            let metric_observed = report.verdict.failures.iter().any(|failure| {
                matches!(
                    failure,
                    OracleFailure::ChecksumMismatchMetric {
                        peer: observed,
                        mismatches,
                    } if *observed == peer && *mismatches > 0
                )
            });
            metric_observed.then_some(NegativeControlMatch {
                class: FailureClass::InbandDesyncDetected,
                peer,
                frame,
                shared_confirmed_prefix_floor,
            })
        },
        Expectation::BoundedRecovery | Expectation::SafeFailure => None,
    }
}

fn observe(case: SweepCase, schedule: &Schedule, options: &RunOptions) -> Observation {
    let report = run_with_input::<StubInput>(schedule, options);
    let evidence = RunEvidence::from_report(&report, case.expectation.is_checksum_only());
    let premise_violations = common_premise_violations(case, schedule, options, &evidence);
    let plateau = terminal_plateau(&evidence.trace_tail);
    let control_match = negative_control_match(case, &report, &evidence);
    let classification = match case.expectation {
        Expectation::BoundedRecovery => classify_bounded_recovery(&evidence, &premise_violations),
        Expectation::SafeFailure => classify_safe_failure(
            case,
            schedule,
            &report,
            &evidence,
            &premise_violations,
            plateau.as_ref(),
        ),
        Expectation::StateCorruption { .. } | Expectation::ChecksumCorruption { .. } => {
            if premise_violations.is_empty() {
                Classification::Defect
            } else {
                Classification::Invalid
            }
        },
    };
    Observation {
        evidence,
        premise_violations,
        terminal_plateau: plateau,
        negative_control_match: control_match,
        classification,
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct ComputeIdentity {
    ledger_schema: u32,
    crate_version: String,
    git_sha: Option<String>,
    input_type: String,
    input_width_bytes: u32,
}

#[cfg(target_os = "linux")]
fn process_memory_bytes(field: &str) -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status
        .lines()
        .find(|line| line.starts_with(field) && line.as_bytes().get(field.len()) == Some(&b':'))?;
    let kib = line
        .split_once(':')?
        .1
        .trim()
        .strip_suffix(" kB")?
        .parse::<u64>()
        .ok()?;
    Some(kib.saturating_mul(1024))
}

#[cfg(not(target_os = "linux"))]
fn process_memory_bytes(_field: &str) -> Option<u64> {
    None
}

/// Nondeterministic host measurements live outside replayed `RunEvidence`.
/// RSS/peak RSS are process-wide because nextest runs each matrix in one process.
#[derive(Debug, Serialize)]
struct HostResourceEvidence {
    observed_elapsed_ns: u64,
    replay_elapsed_ns: u64,
    rss_before_bytes: Option<u64>,
    rss_after_observed_bytes: Option<u64>,
    rss_after_replay_bytes: Option<u64>,
    peak_rss_bytes: Option<u64>,
}

#[derive(Debug, Serialize)]
struct ScenarioRow {
    row_id: String,
    matched_uninjected_row: Option<String>,
    scenario: ScenarioKind,
    expectation: Expectation,
    schedule: Schedule,
    options: RunOptions,
    compute_identity: ComputeIdentity,
    actual_heal_steps: Vec<u32>,
    observed: Observation,
    replay: Observation,
    replay_mismatches: Vec<String>,
    host_resources: HostResourceEvidence,
    classification: Classification,
}

fn compare_replays(observed: &Observation, replay: &Observation) -> Vec<String> {
    let mut mismatches = Vec::new();
    // Compare the whole struct so a future evidence field cannot silently
    // escape the replay gate merely by missing a manual checklist entry.
    if observed.evidence != replay.evidence {
        mismatches.push("evidence (see both complete observations)".to_owned());
    }
    if observed.premise_violations != replay.premise_violations {
        mismatches.push("premise_violations".to_owned());
    }
    if observed.terminal_plateau != replay.terminal_plateau {
        mismatches.push("terminal_plateau".to_owned());
    }
    if observed.negative_control_match != replay.negative_control_match {
        mismatches.push("negative_control_match".to_owned());
    }
    if observed.classification != replay.classification {
        mismatches.push("classification".to_owned());
    }
    mismatches
}

fn compute_identity() -> ComputeIdentity {
    let output_requested = std::env::var_os("FORTRESS_PARTITION_SWEEP_OUT").is_some();
    let git_sha = std::env::var("FORTRESS_SWEEP_GIT_SHA")
        .ok()
        .filter(|sha| !sha.trim().is_empty());
    assert!(
        !(output_requested && git_sha.is_none()),
        "FORTRESS_PARTITION_SWEEP_OUT requires a non-empty FORTRESS_SWEEP_GIT_SHA; refusing an unknown-SHA ledger"
    );
    ComputeIdentity {
        ledger_schema: PARTITION_LEDGER_SCHEMA,
        crate_version: env!("CARGO_PKG_VERSION").to_owned(),
        git_sha,
        input_type: "StubInput".to_owned(),
        input_width_bytes: StubInput::WIDTH_BYTES,
    }
}

fn run_case(case: SweepCase, identity: &ComputeIdentity) -> ScenarioRow {
    let (schedule, options) = materialize(case);
    let rss_before_bytes = process_memory_bytes("VmRSS");
    let observed_started = std::time::Instant::now();
    let observed = observe(case, &schedule, &options);
    let observed_elapsed_ns =
        u64::try_from(observed_started.elapsed().as_nanos()).unwrap_or(u64::MAX);
    let rss_after_observed_bytes = process_memory_bytes("VmRSS");
    // The same materialized values and input type drive every replay.
    let replay_started = std::time::Instant::now();
    let replay = observe(case, &schedule, &options);
    let replay_elapsed_ns = u64::try_from(replay_started.elapsed().as_nanos()).unwrap_or(u64::MAX);
    let rss_after_replay_bytes = process_memory_bytes("VmRSS");
    let peak_rss_bytes = process_memory_bytes("VmHWM");
    let replay_mismatches = compare_replays(&observed, &replay);
    let classification = if replay_mismatches.is_empty() {
        observed.classification
    } else {
        Classification::Invalid
    };
    ScenarioRow {
        row_id: case.id(),
        matched_uninjected_row: case.matched_uninjected_row(),
        scenario: case.scenario,
        expectation: case.expectation,
        actual_heal_steps: actual_heal_steps(&schedule),
        schedule,
        options,
        compute_identity: identity.clone(),
        observed,
        replay,
        replay_mismatches,
        host_resources: HostResourceEvidence {
            observed_elapsed_ns,
            replay_elapsed_ns,
            rss_before_bytes,
            rss_after_observed_bytes,
            rss_after_replay_bytes,
            peak_rss_bytes,
        },
        classification,
    }
}

fn run_matrix(cases: Vec<SweepCase>) -> Vec<ScenarioRow> {
    let identity = compute_identity();
    cases
        .into_iter()
        .map(|case| run_case(case, &identity))
        .collect()
}

fn write_jsonl(rows: &[ScenarioRow]) {
    let Some(path) = std::env::var_os("FORTRESS_PARTITION_SWEEP_OUT") else {
        return;
    };
    use std::io::Write as _;
    let mut file = std::fs::File::create(&path).expect("FORTRESS_PARTITION_SWEEP_OUT is writable");
    for row in rows {
        serde_json::to_writer(&mut file, row).expect("partition evidence row serializes");
        file.write_all(b"\n")
            .expect("partition JSONL newline writes");
    }
    file.flush().expect("partition JSONL flushes");
}

fn assert_rows(rows: &[ScenarioRow]) {
    for row in rows {
        assert_eq!(
            row.classification,
            row.expectation.expected_classification(),
            "partition row {} did not meet its declared expectation: {row:#?}",
            row.row_id
        );
        assert!(
            row.replay_mismatches.is_empty(),
            "partition row {} replay diverged: {row:#?}",
            row.row_id
        );
        if matches!(
            row.expectation,
            Expectation::StateCorruption { .. } | Expectation::ChecksumCorruption { .. }
        ) {
            let base_id = row
                .matched_uninjected_row
                .as_ref()
                .expect("negative controls name their matched base row");
            let base = rows
                .iter()
                .find(|candidate| candidate.row_id == *base_id)
                .expect("negative controls include their matched base row");
            assert!(
                matches!(
                    base.classification,
                    Classification::BoundedRecovery | Classification::SafeFailure
                ),
                "negative control {} has an invalid or defective base row: {base:#?}",
                row.row_id
            );
            assert_eq!(
                row.schedule, base.schedule,
                "negative control {} changed its matched schedule",
                row.row_id
            );
            let mut uninjected_options = row.options.clone();
            uninjected_options.corrupt_state_from = None;
            uninjected_options.corrupt_checksum_from = None;
            assert_eq!(
                uninjected_options, base.options,
                "negative control {} changed options beyond its corruption",
                row.row_id
            );
            assert!(
                row.observed.negative_control_match.is_some()
                    && row.replay.negative_control_match.is_some(),
                "partition row {} lacked its typed, peer/frame-specific negative-control evidence: {row:#?}",
                row.row_id
            );
        }
    }
}

#[test]
fn partition_sweep_pr_gate() {
    let rows = run_matrix(pr_cases());
    write_jsonl(&rows);
    assert_rows(&rows);
}

#[test]
#[ignore = "full N=2/3/4/8/16 partition matrix; run release/offline"]
fn partition_sweep_full() {
    let rows = run_matrix(full_cases());
    write_jsonl(&rows);
    assert_rows(&rows);
}

#[test]
fn sweep_schedules_materialize_declared_fault_premises() {
    let cases = full_cases();
    assert_eq!(
        pr_cases().len(),
        19,
        "ten N=2 base rows plus nine matched controls"
    );
    assert_eq!(cases.len(), 66, "50 base rows plus 16 matched controls");
    for case in cases {
        let (schedule, options) = materialize(case);
        assert_eq!(schedule.seed, SEED);
        assert_eq!(schedule.config.steps, STEPS);
        assert_eq!(schedule.config.n_players, case.n_players);
        assert_eq!(
            schedule.config.disconnect_behavior,
            case.scenario.drop_policy()
        );
        assert_eq!(options.disconnect_timeout, Some(DISCONNECT_TIMEOUT));
        assert_eq!(options.probe_confirmed_at, Some(PRE_CUT_PROBE_STEP));
        let blocked_links = intended_blocked_links(case.n_players, case.scenario);
        let duplicated_links = intended_duplication_links(case.scenario);
        let reordered_links = intended_reorder_links(case.scenario);
        let burst_loss_links = intended_burst_loss_links(case.scenario);
        let asymmetric_delay_links = intended_asymmetric_delay_links(case.scenario);
        let peer_stalls = intended_peer_stalls(case.scenario);
        for &(from, to) in &blocked_links {
            assert!(
                blocked_links.contains(&(to, from)),
                "partition must be symmetric"
            );
            assert!(schedule.events.iter().any(|(step, event)| matches!(
                event,
                ScheduleEvent::Block { from: event_from, to: event_to, blocked: true }
                    if *step == CUT_STEP && *event_from == from && *event_to == to
            )));
        }
        let duplication_policy = LinkPolicy {
            dup_rate: 1.0,
            ..LinkPolicy::clean()
        };
        for &(from, to) in &duplicated_links {
            assert!(schedule.events.iter().any(|(step, event)| matches!(
                event,
                ScheduleEvent::SetLink { from: event_from, to: event_to, policy }
                    if *step == CUT_STEP
                        && *event_from == from
                        && *event_to == to
                        && policy == &duplication_policy
            )));
        }
        for &(from, to) in &reordered_links {
            assert!(schedule.events.iter().any(|(step, event)| matches!(
                event,
                ScheduleEvent::SetLink { from: event_from, to: event_to, policy }
                    if *step == CUT_STEP
                        && *event_from == from
                        && *event_to == to
                        && policy == &reorder_policy()
            )));
        }
        for &(from, to) in &burst_loss_links {
            assert!(schedule.events.iter().any(|(step, event)| matches!(
                event,
                ScheduleEvent::SetLink { from: event_from, to: event_to, policy }
                    if *step == CUT_STEP
                        && *event_from == from
                        && *event_to == to
                        && policy == &burst_loss_policy()
            )));
        }
        for &(from, to) in &asymmetric_delay_links {
            assert!(schedule.events.iter().any(|(step, event)| matches!(
                event,
                ScheduleEvent::SetLink { from: event_from, to: event_to, policy }
                    if *step == CUT_STEP
                        && *event_from == from
                        && *event_to == to
                        && policy == &asymmetric_delay_policy()
            )));
        }
        if case.scenario == ScenarioKind::PollStall {
            assert!(blocked_links.is_empty());
            assert_eq!(schedule.events.len(), 4, "three stalls plus one heal");
            for &(start, peer, steps) in &peer_stalls {
                assert!(schedule.events.iter().any(|(step, event)| matches!(
                    event,
                    ScheduleEvent::PeerStall { peer: event_peer, steps: event_steps }
                        if *step == start && *event_peer == peer && *event_steps == steps
                )));
            }
        } else if matches!(
            case.scenario,
            ScenarioKind::Duplication
                | ScenarioKind::Reorder
                | ScenarioKind::BurstLoss
                | ScenarioKind::AsymmetricDelay
        ) {
            assert!(blocked_links.is_empty());
            assert_eq!(
                schedule.events.len(),
                2,
                "directed faults must contain only SetLink and HealAll"
            );
            assert_eq!(
                schedule
                    .events
                    .iter()
                    .filter(|(step, _)| *step == CUT_STEP)
                    .count(),
                1,
                "directed faults must inject exactly one link"
            );
            assert!(schedule
                .events
                .iter()
                .all(|(_, event)| !matches!(event, ScheduleEvent::Block { .. })));
            match case.scenario {
                ScenarioKind::Duplication => {
                    assert_eq!(duplicated_links, BTreeSet::from([(0, 1)]));
                    assert!(reordered_links.is_empty());
                    assert!(burst_loss_links.is_empty());
                    assert!(asymmetric_delay_links.is_empty());
                },
                ScenarioKind::Reorder => {
                    assert!(duplicated_links.is_empty());
                    assert_eq!(reordered_links, BTreeSet::from([(0, 1)]));
                    assert!(burst_loss_links.is_empty());
                    assert!(asymmetric_delay_links.is_empty());
                },
                ScenarioKind::BurstLoss => {
                    assert_eq!(burst_loss_links, BTreeSet::from([(0, 1)]));
                    assert!(duplicated_links.is_empty());
                    assert!(reordered_links.is_empty());
                    assert!(asymmetric_delay_links.is_empty());
                },
                ScenarioKind::AsymmetricDelay => {
                    assert_eq!(asymmetric_delay_links, BTreeSet::from([(0, 1)]));
                    assert!(duplicated_links.is_empty());
                    assert!(reordered_links.is_empty());
                    assert!(burst_loss_links.is_empty());
                },
                ScenarioKind::ShortHealContinue
                | ScenarioKind::LongHealHalt
                | ScenarioKind::PermanentHalt
                | ScenarioKind::PermanentContinue
                | ScenarioKind::RepeatedFlap
                | ScenarioKind::PollStall => unreachable!(),
            }
        } else {
            assert!(!blocked_links.is_empty());
            assert!(duplicated_links.is_empty());
            assert!(reordered_links.is_empty());
            assert!(burst_loss_links.is_empty());
            assert!(asymmetric_delay_links.is_empty());
        }
        if case.scenario == ScenarioKind::RepeatedFlap {
            assert_eq!(
                schedule.events.len(),
                blocked_links.len() * 3 + 3,
                "three cuts for every cross-component link plus three heals"
            );
            for cut in FLAP_CUT_STEPS {
                for &(from, to) in &blocked_links {
                    assert!(schedule.events.iter().any(|(step, event)| matches!(
                        event,
                        ScheduleEvent::Block { from: event_from, to: event_to, blocked: true }
                            if *step == cut && *event_from == from && *event_to == to
                    )));
                }
            }
            for (cut, heal) in FLAP_CUT_STEPS.into_iter().zip(FLAP_HEAL_STEPS) {
                assert_eq!(heal - cut, REPEATED_CYCLE_STEPS);
                assert!(
                    u64::from(heal - cut) * schedule.config.step_dt_ms
                        < DISCONNECT_TIMEOUT.as_millis() as u64
                );
            }
        } else if case.scenario != ScenarioKind::PollStall {
            assert_eq!(peer_stalls, Vec::new());
        }
        for &(_, _, steps) in &peer_stalls {
            assert_eq!(steps, REPEATED_CYCLE_STEPS);
            assert!(
                u64::from(steps) * schedule.config.step_dt_ms
                    < DISCONNECT_TIMEOUT.as_millis() as u64
            );
        }
        let expected_heals = case.scenario.heal_steps();
        assert_eq!(actual_heal_steps(&schedule), expected_heals);
        if let Some(heal) = case.scenario.heal_step() {
            let fault_ms = u64::from(heal - CUT_STEP) * schedule.config.step_dt_ms;
            match case.scenario {
                ScenarioKind::ShortHealContinue
                | ScenarioKind::Duplication
                | ScenarioKind::Reorder
                | ScenarioKind::BurstLoss
                | ScenarioKind::AsymmetricDelay
                | ScenarioKind::PollStall => {
                    assert!(fault_ms < DISCONNECT_TIMEOUT.as_millis() as u64);
                    assert!(STEPS - heal >= schedule.config.recovery_window_steps());
                },
                ScenarioKind::RepeatedFlap => {
                    assert!(STEPS - heal >= schedule.config.recovery_window_steps());
                },
                ScenarioKind::LongHealHalt => {
                    assert!(fault_ms > DISCONNECT_TIMEOUT.as_millis() as u64);
                    assert!(STEPS - heal >= schedule.config.recovery_window_steps());
                },
                ScenarioKind::PermanentHalt | ScenarioKind::PermanentContinue => unreachable!(),
            }
        } else {
            assert_eq!(
                schedule.heal_at, STEPS,
                "inert heal_at must not imply a heal"
            );
        }
    }
}

#[test]
fn divergence_frame_uses_minimum_and_checksum_only_stays_unknown() {
    use crate::common::stubs::StateStub;
    let failures = vec![
        OracleFailure::InbandDesyncDetected { peer: 1, frame: 70 },
        OracleFailure::StateDivergence {
            frame: 40,
            peer: 1,
            first_author: 0,
            expected: StateStub {
                frame: 40,
                state: 0,
            },
            actual: StateStub {
                frame: 40,
                state: 1,
            },
        },
        OracleFailure::ChecksumMismatchMetric {
            peer: 1,
            mismatches: 2,
        },
    ];
    assert_eq!(first_observed_divergence(&failures, false), Some(40));
    let checksum_only = vec![
        OracleFailure::InbandDesyncDetected { peer: 1, frame: 70 },
        OracleFailure::ChecksumMismatchMetric {
            peer: 1,
            mismatches: 2,
        },
    ];
    assert_eq!(first_observed_divergence(&checksum_only, true), None);
    assert_eq!(first_observed_divergence(&failures, true), Some(40));
}

/// CAP matrix row 1: a symmetric two-component partition under
/// `ContinueWithout` must fail **closed** on every component at the same
/// certified cut, never keep confirming into two silent histories. Both the
/// even bisection (2/2) and the odd singleton isolation (2/1) are covered.
///
/// This is the component-scoped counterpart to the single-history oracle: the
/// global live-peer minimum cannot describe a partition, so each component's
/// exposed confirmed history is compared independently. The row asserts the
/// declared cut actually fired, both components stopped within the in-flight
/// delivery allowance of it, the shared confirmed prefix is byte-identical, all
/// peers end `Synchronizing`, and no `Disconnected` event was invented. The
/// mutation control flips one component's value on a shared confirmed frame and
/// requires the same comparison to report a fork, so a silent divergence cannot
/// be mistaken for a healthy terminal state.
#[test]
fn symmetric_partition_components_fail_closed_without_fork() {
    // Confirmation can drain a frame or two past the step at which the block
    // applies; anything beyond this is a component that kept confirming.
    const PARTITION_CONFIRM_ALLOWANCE: i32 = 2;

    // Even peer counts bisect into two equal components; odd peer counts
    // isolate a singleton, so the majority component has two independent
    // survivors. Both shapes must fail closed today.
    for component in [vec![0, 0, 1, 1], vec![0, 0, 1]] {
        let case = SweepCase::base(component.len(), ScenarioKind::PermanentContinue);
        check_partition_components_fail_closed(case, component, PARTITION_CONFIRM_ALLOWANCE);
    }
}

#[track_caller]
fn check_partition_components_fail_closed(case: SweepCase, component: Vec<usize>, allowance: i32) {
    let (schedule, mut options) = materialize(case);
    options.component_scope = Some(ComponentScopeOptions {
        component: component.clone(),
    });

    let report = run_with_input::<StubInput>(&schedule, &options);
    let evidence = report
        .component_scope
        .as_ref()
        .expect("component-scoped evidence was requested");
    assert_eq!(
        evidence.partition_confirmed.len(),
        component.len(),
        "the declared partition must actually fire"
    );
    assert_eq!(evidence.component, component);

    let cut = evidence
        .partition_confirmed
        .iter()
        .copied()
        .min()
        .expect("partition snapshot");
    assert!(
        evidence
            .final_confirmed
            .iter()
            .all(|frame| *frame + allowance >= cut),
        "every peer must have confirmed at least to within the allowance of the cut: {:?} vs cut {cut}",
        evidence.final_confirmed
    );

    let outcome = classify_component_scope(evidence, allowance);
    assert!(
        matches!(outcome, ComponentOutcome::FailedClosed { .. }),
        "a permanent partition must fail closed on every component, got {outcome:?}"
    );
    assert!(outcome.is_legal());
    assert!(
        evidence
            .final_states
            .iter()
            .all(|state| *state == TraceSessionState::Synchronizing),
        "every component must fail closed, got {:?}",
        evidence.final_states
    );
    assert_eq!(
        report
            .peer_event_counts
            .get(&EventKind::Disconnected)
            .copied()
            .unwrap_or(0),
        0,
        "a failed-closed partition must not invent a peer removal"
    );
    assert!(
        typed_safe_failure_diagnostics_hold(
            &report,
            false,
            true,
            schedule.config.recovery_window_steps(),
        ),
        "the terminal failures must stay the reviewed drop-timeout/EndProgress set: {:?}",
        report.verdict.failures
    );
    assert!(
        !report.verdict.failures.iter().any(|failure| matches!(
            failure,
            OracleFailure::ConfirmedInputDivergence { .. } | OracleFailure::StateDivergence { .. }
        )),
        "the failing-closed components must keep one confirmed history: {:?}",
        report.verdict.failures
    );

    // Mutation control: flipping one component's value on a shared confirmed
    // frame must be reported as a fork by the same comparison.
    let shared_frame = evidence
        .confirmed_inputs
        .first()
        .and_then(|history| history.keys().next_back().copied())
        .expect("component history");
    let mut forked = evidence.clone();
    // Rewrite every member of component 0 identically so the intra-component
    // check still passes and only the cross-component comparison can catch it.
    for peer in (0..component.len()).filter(|peer| component[*peer] == 0) {
        let Some(inputs) = forked.confirmed_inputs[peer].get_mut(&shared_frame) else {
            panic!("component history is missing the shared frame {shared_frame}");
        };
        let Some(slot) = inputs.first_mut() else {
            panic!("shared frame {shared_frame} has no rendered peers");
        };
        *slot = InputFingerprint::from_bytes(0xDEAD_BEEF, b"silent-fork");
    }
    assert_eq!(
        classify_component_scope(&forked, allowance),
        ComponentOutcome::Forked {
            frame: shared_frame,
            component: 0,
            other_component: 1,
        },
        "the mutation control must flip FailedClosed to Forked"
    );
}
