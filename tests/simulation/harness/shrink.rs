//! Explicit, bounded, domain-aware shrinking for simulation failures.
//!
//! Shrinking is never invoked by [`super::RunReport::expect_pass`]. Callers opt
//! in after a deterministic failure and choose where accepted candidates are
//! emitted.

use super::schedule::{validate_schedule, Schedule, ScheduleEvent};
use super::{RunOptions, RunReport};
use crate::common::sim_net::{
    BandwidthPolicy, FragmentationPolicy, GilbertElliottPolicy, LinkPolicy,
};
use fortress_rollback::metrics::EventKind;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

/// Default hard cap on simulation executions performed by one shrink.
pub const DEFAULT_MAX_RUNS: usize = 500;
/// Default soft wall-clock admission bound for one shrink.
///
/// The shrinker checks this before starting each confirmation run. A run that
/// starts within the bound is allowed to finish, so an individual slow
/// simulation can make the final elapsed time exceed this value. The run-count
/// bound remains exact.
pub const DEFAULT_MAX_DURATION: Duration = Duration::from_secs(5 * 60);

/// Resource limits for an explicit shrink operation.
#[derive(Clone, Copy, Debug)]
pub struct ShrinkConfig {
    pub max_runs: usize,
    pub max_duration: Duration,
}

impl Default for ShrinkConfig {
    fn default() -> Self {
        Self {
            max_runs: DEFAULT_MAX_RUNS,
            max_duration: DEFAULT_MAX_DURATION,
        }
    }
}

/// Best schedule found before the resource bounds were reached.
#[derive(Clone, Debug)]
pub struct ShrinkResult {
    pub schedule: Schedule,
    pub options: RunOptions,
    pub runs: usize,
    pub elapsed: Duration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FinalSummary {
    final_confirmed: Vec<i32>,
    probe_confirmed: Vec<i32>,
    probe_peer_wire_by_link: BTreeMap<(usize, usize), super::PeerWireTotals>,
    pending_output_probe: Option<super::PendingOutputProbe>,
    net_stats: crate::common::sim_net::SimNetStats,
    blocked_drops_by_link: BTreeMap<(usize, usize), u64>,
    fragmentation_drops_by_link: BTreeMap<(usize, usize), u64>,
    link_stats_by_link: BTreeMap<(usize, usize), crate::common::sim_net::SimLinkStats>,
    load_game_state_observations: Vec<super::LoadGameStateObservation>,
    metrics: Vec<fortress_rollback::SessionMetrics>,
    peer_wire: Vec<super::PeerWireTotals>,
    confirmed_at_heal: Vec<i32>,
    confirmed_after_recovery: Vec<i32>,
    recovered_within_b: Option<bool>,
    violation_census: BTreeMap<super::oracle::ViolationSignature, u64>,
    peer_event_counts: BTreeMap<EventKind, u64>,
    peer_event_counts_by_peer: Vec<BTreeMap<EventKind, u64>>,
    peer_event_payload_counts_by_peer: Vec<BTreeMap<super::PeerEventKey, u64>>,
    spectator_applied_frames: usize,
    spectator_max_frame: Option<i32>,
    spectator_final_hosts: Option<usize>,
    verdict_recovered_within_b: Option<bool>,
    violation_allowlist_hits: Vec<super::oracle::ViolationAllowlistHit>,
}

impl From<&RunReport> for FinalSummary {
    fn from(report: &RunReport) -> Self {
        Self {
            final_confirmed: report.final_confirmed.clone(),
            probe_confirmed: report.probe_confirmed.clone(),
            probe_peer_wire_by_link: report.probe_peer_wire_by_link.clone(),
            pending_output_probe: report.pending_output_probe,
            net_stats: report.net_stats,
            blocked_drops_by_link: report.blocked_drops_by_link.clone(),
            fragmentation_drops_by_link: report.fragmentation_drops_by_link.clone(),
            link_stats_by_link: report.link_stats_by_link.clone(),
            load_game_state_observations: report.load_game_state_observations.clone(),
            metrics: report.metrics.clone(),
            peer_wire: report.peer_wire.clone(),
            confirmed_at_heal: report.confirmed_at_heal.clone(),
            confirmed_after_recovery: report.confirmed_after_recovery.clone(),
            recovered_within_b: report.recovered_within_b,
            violation_census: report.violation_census.clone(),
            peer_event_counts: report.peer_event_counts.clone(),
            peer_event_counts_by_peer: report.peer_event_counts_by_peer.clone(),
            peer_event_payload_counts_by_peer: report.peer_event_payload_counts_by_peer.clone(),
            spectator_applied_frames: report.spectator_applied_frames,
            spectator_max_frame: report.spectator_max_frame,
            spectator_final_hosts: report.spectator_final_hosts,
            verdict_recovered_within_b: report.verdict.recovered_within_b,
            violation_allowlist_hits: report.verdict.violation_allowlist_hits.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ReproductionIdentity {
    failure_sequence: Vec<String>,
    failure_multiset: BTreeMap<String, u64>,
    trace_hash: u64,
    final_summary: FinalSummary,
}

fn reproduction_identity(report: &RunReport, target_class: &str) -> Option<ReproductionIdentity> {
    let failure_sequence: Vec<String> = report
        .verdict
        .failures
        .iter()
        .map(|failure| failure.class().to_owned())
        .collect();
    if failure_sequence.first().map(String::as_str) != Some(target_class) {
        return None;
    }
    let mut failure_multiset = BTreeMap::new();
    for class in &failure_sequence {
        *failure_multiset.entry(class.clone()).or_default() += 1;
    }
    Some(ReproductionIdentity {
        failure_sequence,
        failure_multiset,
        trace_hash: report.trace_hash,
        final_summary: FinalSummary::from(report),
    })
}

fn remap_index(index: usize, removed: usize) -> Option<usize> {
    match index.cmp(&removed) {
        std::cmp::Ordering::Less => Some(index),
        std::cmp::Ordering::Equal => None,
        std::cmp::Ordering::Greater => Some(index - 1),
    }
}

fn remap_peer_frame(value: Option<(usize, i32)>, removed: usize) -> Option<(usize, i32)> {
    value.and_then(|(peer, frame)| remap_index(peer, removed).map(|mapped| (mapped, frame)))
}

fn remap_options(options: &RunOptions, removed: usize, steps: u32) -> RunOptions {
    let pending_output_probe_link = options
        .pending_output_probe_link
        .and_then(|(from, to)| Some((remap_index(from, removed)?, remap_index(to, removed)?)));
    RunOptions {
        corrupt_state_from: remap_peer_frame(options.corrupt_state_from, removed),
        corrupt_checksum_from: remap_peer_frame(options.corrupt_checksum_from, removed),
        probe_confirmed_at: options.probe_confirmed_at.filter(|probe| *probe < steps),
        pending_output_probe_link,
        corrupt_spectator_input_from: options.corrupt_spectator_input_from,
        corrupt_spectator_status_from: options.corrupt_spectator_status_from,
    }
}

fn remap_event(event: &ScheduleEvent, removed: usize) -> Option<ScheduleEvent> {
    let pair =
        |from: usize, to: usize| Some((remap_index(from, removed)?, remap_index(to, removed)?));
    match event {
        ScheduleEvent::SetLink { from, to, policy } => {
            let (from, to) = pair(*from, *to)?;
            Some(ScheduleEvent::SetLink {
                from,
                to,
                policy: policy.clone(),
            })
        },
        ScheduleEvent::Block { from, to, blocked } => {
            let (from, to) = pair(*from, *to)?;
            Some(ScheduleEvent::Block {
                from,
                to,
                blocked: *blocked,
            })
        },
        ScheduleEvent::Hold { from, to, holding } => {
            let (from, to) = pair(*from, *to)?;
            Some(ScheduleEvent::Hold {
                from,
                to,
                holding: *holding,
            })
        },
        ScheduleEvent::PeerStall { peer, steps } => Some(ScheduleEvent::PeerStall {
            peer: remap_index(*peer, removed)?,
            steps: *steps,
        }),
        ScheduleEvent::SetInputDelay { peer, delay } => Some(ScheduleEvent::SetInputDelay {
            peer: remap_index(*peer, removed)?,
            delay: *delay,
        }),
        ScheduleEvent::GracefulRemove { by, target } => {
            let (by, target) = pair(*by, *target)?;
            Some(ScheduleEvent::GracefulRemove { by, target })
        },
        ScheduleEvent::LegacyDisconnect { by, target } => {
            let (by, target) = pair(*by, *target)?;
            Some(ScheduleEvent::LegacyDisconnect { by, target })
        },
        ScheduleEvent::PeerKill { peer } => Some(ScheduleEvent::PeerKill {
            peer: remap_index(*peer, removed)?,
        }),
        ScheduleEvent::Rebind { peer } => Some(ScheduleEvent::Rebind {
            peer: remap_index(*peer, removed)?,
        }),
        ScheduleEvent::SpectatorHostKill { host } => Some(ScheduleEvent::SpectatorHostKill {
            host: remap_index(*host, removed)?,
        }),
        ScheduleEvent::HotJoin { slot } => Some(ScheduleEvent::HotJoin {
            slot: remap_index(*slot, removed)?,
        }),
        ScheduleEvent::HealAll => Some(ScheduleEvent::HealAll),
    }
}

fn hot_join_host(n_players: usize, slot: usize) -> Option<usize> {
    (0..n_players).find(|peer| *peer != slot)
}

fn prune_invalid_special_events(schedule: &mut Schedule) {
    let n = schedule.config.n_players;
    let mut retired = vec![false; n];
    schedule.events.retain(|(_, event)| match event {
        ScheduleEvent::PeerKill { peer } => {
            retired[*peer] = true;
            true
        },
        ScheduleEvent::SpectatorHostKill { host } => {
            let valid = schedule.config.spectator_hosts.contains(host) && !retired[*host];
            if valid {
                retired[*host] = true;
            }
            valid
        },
        ScheduleEvent::HotJoin { slot } => {
            !retired[*slot] && hot_join_host(n, *slot).is_some_and(|host| !retired[host])
        },
        ScheduleEvent::Rebind { peer } => !retired[*peer],
        ScheduleEvent::SetLink { .. }
        | ScheduleEvent::Block { .. }
        | ScheduleEvent::Hold { .. }
        | ScheduleEvent::PeerStall { .. }
        | ScheduleEvent::SetInputDelay { .. }
        | ScheduleEvent::GracefulRemove { .. }
        | ScheduleEvent::LegacyDisconnect { .. }
        | ScheduleEvent::HealAll => true,
    });
}

fn remove_peer(
    schedule: &Schedule,
    options: &RunOptions,
    removed: usize,
) -> Option<(Schedule, RunOptions)> {
    if schedule.config.n_players <= 2 || removed >= schedule.config.n_players {
        return None;
    }
    let mut candidate = schedule.clone();
    candidate.config.n_players -= 1;
    candidate.initial_links = schedule
        .initial_links
        .iter()
        .filter_map(|(from, to, policy)| {
            Some((
                remap_index(*from, removed)?,
                remap_index(*to, removed)?,
                policy.clone(),
            ))
        })
        .collect();
    candidate.events = schedule
        .events
        .iter()
        .filter_map(|(step, event)| remap_event(event, removed).map(|event| (*step, event)))
        .collect();

    if removed < candidate.config.clock_skew_ppm.len() {
        let _ = candidate.config.clock_skew_ppm.remove(removed);
    }
    candidate.config.starve_events = schedule
        .config
        .starve_events
        .iter()
        .filter_map(|peer| remap_index(*peer, removed))
        .collect();
    candidate.config.spectator_hosts = schedule
        .config
        .spectator_hosts
        .iter()
        .filter_map(|peer| remap_index(*peer, removed))
        .collect();
    prune_invalid_special_events(&mut candidate);
    let options = remap_options(options, removed, candidate.config.steps);
    Some((candidate, options))
}

fn truncate_schedule(
    schedule: &Schedule,
    options: &RunOptions,
    steps: u32,
) -> (Schedule, RunOptions) {
    let mut candidate = schedule.clone();
    candidate.config.steps = steps.max(2);
    let final_heal = actual_final_heal(schedule);
    candidate
        .events
        .retain(|(step, _)| *step < candidate.config.steps);
    restore_final_heal(&mut candidate, final_heal);
    let mut options = options.clone();
    options.probe_confirmed_at = options
        .probe_confirmed_at
        .filter(|probe| *probe < candidate.config.steps);
    (candidate, options)
}

fn actual_final_heal(schedule: &Schedule) -> Option<u32> {
    schedule
        .events
        .iter()
        .filter_map(|(step, event)| matches!(event, ScheduleEvent::HealAll).then_some(*step))
        .max()
}

fn restore_final_heal(schedule: &mut Schedule, preferred_step: Option<u32>) {
    schedule
        .events
        .retain(|(_, event)| !matches!(event, ScheduleEvent::HealAll));
    if let Some(preferred_step) = preferred_step {
        let heal_at = preferred_step.min(schedule.config.steps.saturating_sub(1));
        schedule.events.push((heal_at, ScheduleEvent::HealAll));
        schedule.events.sort_by_key(|(step, _)| *step);
        schedule.heal_at = heal_at;
    } else {
        schedule.heal_at = schedule.config.steps;
    }
}

#[derive(Clone, Copy)]
enum LinkSimplification {
    Loss,
    Fragmentation,
    Bandwidth,
    Duplication,
    Jitter,
    Delay,
    Retransmission,
    Clean,
}

fn simplify_link(policy: &mut LinkPolicy, simplification: LinkSimplification) {
    // Fault-model policies are simplified by removing one independent axis.
    // We deliberately do not tune GE probabilities, fragmentation probability,
    // or bandwidth numeric bounds: doing so can replace the reproduced failure
    // mechanism (for example, queue delay with oversize/tail drop).
    match simplification {
        LinkSimplification::Loss => {
            policy.drop_rate = 0.0;
            policy.burst_rate = 0.0;
            policy.burst_len = 0;
            policy.gilbert_elliott = None;
        },
        LinkSimplification::Fragmentation => policy.fragmentation = None,
        LinkSimplification::Bandwidth => policy.bandwidth = None,
        LinkSimplification::Duplication => policy.dup_rate = 0.0,
        LinkSimplification::Jitter => policy.jitter = Duration::ZERO,
        LinkSimplification::Delay => policy.base_delay = Duration::ZERO,
        LinkSimplification::Retransmission => policy.retransmit_delay = Duration::ZERO,
        LinkSimplification::Clean => *policy = LinkPolicy::clean(),
    }
}

fn simplify_links(schedule: &Schedule, simplification: LinkSimplification) -> Schedule {
    let mut candidate = schedule.clone();
    for (_, _, policy) in &mut candidate.initial_links {
        simplify_link(policy, simplification);
    }
    for (_, event) in &mut candidate.events {
        if let ScheduleEvent::SetLink { policy, .. } = event {
            simplify_link(policy, simplification);
        }
    }
    candidate
}

#[derive(Clone, Copy)]
enum LinkLocation {
    Initial(usize),
    SetLinkEvent(usize),
}

fn link_locations(schedule: &Schedule) -> Vec<LinkLocation> {
    let mut locations: Vec<LinkLocation> = (0..schedule.initial_links.len())
        .map(LinkLocation::Initial)
        .collect();
    locations.extend(
        schedule
            .events
            .iter()
            .enumerate()
            .filter_map(|(index, (_, event))| {
                matches!(event, ScheduleEvent::SetLink { .. })
                    .then_some(LinkLocation::SetLinkEvent(index))
            }),
    );
    locations
}

fn simplify_link_at(
    schedule: &Schedule,
    location: LinkLocation,
    simplification: LinkSimplification,
) -> Schedule {
    let mut candidate = schedule.clone();
    let policy = match location {
        LinkLocation::Initial(index) => candidate
            .initial_links
            .get_mut(index)
            .map(|(_, _, policy)| policy),
        LinkLocation::SetLinkEvent(index) => {
            candidate
                .events
                .get_mut(index)
                .and_then(|(_, event)| match event {
                    ScheduleEvent::SetLink { policy, .. } => Some(policy),
                    _ => None,
                })
        },
    };
    if let Some(policy) = policy {
        simplify_link(policy, simplification);
    }
    candidate
}

struct Evaluator<'a, R, H> {
    runner: &'a mut R,
    accepted: &'a mut H,
    target_class: &'a str,
    config: ShrinkConfig,
    started: Instant,
    runs: usize,
    required_classes: Option<std::collections::BTreeSet<String>>,
}

impl<R, H> Evaluator<'_, R, H>
where
    R: FnMut(&Schedule, &RunOptions) -> RunReport,
    H: FnMut(&Schedule, &RunOptions, &RunReport),
{
    fn has_budget(&self, additional_runs: usize) -> bool {
        self.runs.saturating_add(additional_runs) <= self.config.max_runs
            && self.started.elapsed() < self.config.max_duration
    }

    fn confirm(&mut self, schedule: &Schedule, options: &RunOptions) -> Option<RunReport> {
        if !self.has_budget(2) {
            return None;
        }
        if validate_schedule(schedule).is_err()
            || super::validate_run_options(schedule, options).is_err()
        {
            return None;
        }
        self.runs += 1;
        let first = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            (self.runner)(schedule, options)
        }))
        .ok()?;
        let first_identity = reproduction_identity(&first, self.target_class)?;
        if self.required_classes.as_ref().is_some_and(|required| {
            !required
                .iter()
                .all(|class| first_identity.failure_multiset.contains_key(class))
        }) {
            return None;
        }
        if !self.has_budget(1) {
            return None;
        }
        self.runs += 1;
        let second = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            (self.runner)(schedule, options)
        }))
        .ok()?;
        let second_identity = reproduction_identity(&second, self.target_class)?;
        if first_identity != second_identity {
            return None;
        }
        if self.required_classes.is_none() {
            self.required_classes = Some(first_identity.failure_multiset.keys().cloned().collect());
        }
        (self.accepted)(schedule, options, &second);
        Some(second)
    }
}

/// Minimizes a deterministic failure while preserving one stable oracle class.
///
/// Every accepted candidate is independently confirmed twice. The callback is
/// invoked after that confirmation so callers can continuously persist the
/// best-so-far schedule without coupling the shrinker to artifact policy.
pub fn shrink_failure<R, H>(
    schedule: &Schedule,
    options: &RunOptions,
    target_class: &str,
    config: ShrinkConfig,
    mut runner: R,
    mut on_accepted: H,
) -> Result<ShrinkResult, String>
where
    R: FnMut(&Schedule, &RunOptions) -> RunReport,
    H: FnMut(&Schedule, &RunOptions, &RunReport),
{
    let config = ShrinkConfig {
        max_runs: config.max_runs.min(DEFAULT_MAX_RUNS),
        max_duration: config.max_duration.min(DEFAULT_MAX_DURATION),
    };
    let mut evaluator = Evaluator {
        runner: &mut runner,
        accepted: &mut on_accepted,
        target_class,
        config,
        started: Instant::now(),
        runs: 0,
        required_classes: None,
    };
    if evaluator.confirm(schedule, options).is_none() {
        return Err(format!(
            "initial schedule did not reproduce {target_class} twice within shrink bounds"
        ));
    }
    let mut current = schedule.clone();
    let mut current_options = options.clone();

    // Remove peers greedily, restarting at index 0 after each accepted remap.
    let mut peer = 0;
    while current.config.n_players > 2 && evaluator.has_budget(2) {
        let Some((candidate, candidate_options)) = remove_peer(&current, &current_options, peer)
        else {
            break;
        };
        if evaluator.confirm(&candidate, &candidate_options).is_some() {
            current = candidate;
            current_options = candidate_options;
            peer = 0;
        } else {
            peer += 1;
            if peer >= current.config.n_players {
                break;
            }
        }
    }

    // Explore prefixes without assuming monotonicity. Exhaustive shortest-first
    // scanning is useful for small schedules but starves on long nightly runs:
    // a 500-run budget can never reach an event at step 600. Long schedules
    // therefore probe deterministic geometric and event-adjacent checkpoints,
    // then exhaustively search the 128-step window below the shortest accepted
    // checkpoint. Every candidate still stands on its own two-run confirmation;
    // checkpoints are ordering hints, never a monotonicity claim.
    let prefix_base = current.clone();
    let prefix_options = current_options.clone();
    let mut best = (current.clone(), current_options.clone());
    let mut checkpoints = std::collections::BTreeSet::new();
    if prefix_base.config.steps <= 256 {
        checkpoints.extend(2..prefix_base.config.steps);
    } else {
        let mut divided = prefix_base.config.steps / 2;
        while divided >= 2 {
            checkpoints.insert(divided);
            divided /= 2;
        }
        for (step, event) in &prefix_base.events {
            if matches!(event, ScheduleEvent::HealAll) {
                continue;
            }
            let mut delta = 1u32;
            while let Some(candidate) = step.checked_add(delta) {
                if candidate >= prefix_base.config.steps {
                    break;
                }
                checkpoints.insert(candidate.max(2));
                let Some(doubled) = delta.checked_mul(2) else {
                    break;
                };
                delta = doubled;
            }
        }
    }
    for steps in checkpoints {
        if !evaluator.has_budget(2) {
            break;
        }
        let (candidate, candidate_options) =
            truncate_schedule(&prefix_base, &prefix_options, steps);
        if evaluator.confirm(&candidate, &candidate_options).is_some() {
            best = (candidate, candidate_options);
            break;
        }
    }
    if best.0.config.steps < prefix_base.config.steps {
        let end = best.0.config.steps;
        let start = end.saturating_sub(128).max(2);
        for steps in start..end {
            if !evaluator.has_budget(2) {
                break;
            }
            let (candidate, candidate_options) =
                truncate_schedule(&prefix_base, &prefix_options, steps);
            if evaluator.confirm(&candidate, &candidate_options).is_some() {
                best = (candidate, candidate_options);
                break;
            }
        }
    }
    current = best.0;
    current_options = best.1;

    // Delta-debug the remaining event list by removing progressively smaller chunks.
    let mut chunk = current.events.len().div_ceil(2).max(1);
    while !current.events.is_empty() && evaluator.has_budget(2) {
        let mut start = 0usize;
        let mut accepted_chunk = false;
        while start < current.events.len() && evaluator.has_budget(2) {
            let end = start.saturating_add(chunk).min(current.events.len());
            let final_heal = actual_final_heal(&current);
            let mut candidate = current.clone();
            candidate.events.drain(start..end);
            restore_final_heal(&mut candidate, final_heal);
            if candidate != current && evaluator.confirm(&candidate, &current_options).is_some() {
                current = candidate;
                accepted_chunk = true;
            } else {
                start = end;
            }
        }
        if chunk == 1 {
            break;
        }
        if !accepted_chunk {
            chunk = chunk.div_ceil(2).max(1);
        }
    }

    // Collapse the frame-production model before simplifying its clock rates;
    // a failure that does not require rate-gated production should replay on
    // the historical lockstep model.
    if current.config.frame_model != super::schedule::FrameModel::Lockstep {
        let mut candidate = current.clone();
        candidate.config.frame_model = super::schedule::FrameModel::Lockstep;
        if evaluator.confirm(&candidate, &current_options).is_some() {
            current = candidate;
        }
    }

    // Simplify individual clock-rate entries before attempting the global
    // collapse. A failure may require one skewed peer while every other entry
    // is irrelevant.
    for index in 0..current.config.clock_skew_ppm.len() {
        if !evaluator.has_budget(2) {
            break;
        }
        let mut candidate = current.clone();
        if let Some(value) = candidate.config.clock_skew_ppm.get_mut(index) {
            *value = 0;
        }
        if candidate != current && evaluator.confirm(&candidate, &current_options).is_some() {
            current = candidate;
        }
    }

    // Collapse per-peer clock cadence back to the uniform default when it is
    // irrelevant to the target failure.
    if !current.config.clock_skew_ppm.is_empty() {
        let mut candidate = current.clone();
        candidate.config.clock_skew_ppm.clear();
        if evaluator.confirm(&candidate, &current_options).is_some() {
            current = candidate;
        }
    }

    // Reduce global cadence/configuration values independently. Each candidate
    // still passes the materialized-schedule validator before execution.
    let cadence_simplifications: [fn(&mut Schedule); 4] = [
        |schedule: &mut Schedule| schedule.config.step_dt_ms = 16,
        |schedule: &mut Schedule| schedule.config.input_delay = 0,
        |schedule: &mut Schedule| schedule.config.max_prediction = 8,
        |schedule: &mut Schedule| schedule.config.desync_interval = 30,
    ];
    for simplify in cadence_simplifications {
        if !evaluator.has_budget(2) {
            break;
        }
        let mut candidate = current.clone();
        simplify(&mut candidate);
        if candidate != current && evaluator.confirm(&candidate, &current_options).is_some() {
            current = candidate;
        }
    }

    // Reduce event-local cadence values without forcing all such events to
    // change together.
    for index in 0..current.events.len() {
        if !evaluator.has_budget(2) {
            break;
        }
        let mut candidate = current.clone();
        match candidate.events.get_mut(index).map(|(_, event)| event) {
            Some(ScheduleEvent::PeerStall { steps, .. }) => *steps = 1,
            Some(ScheduleEvent::SetInputDelay { delay, .. }) => *delay = 0,
            _ => continue,
        }
        if candidate != current && evaluator.confirm(&candidate, &current_options).is_some() {
            current = candidate;
        }
    }

    // Simplify every policy location independently before trying global
    // dimension collapse. A failure may require delay on one link but not loss
    // or jitter on any of the others.
    for simplification in [
        LinkSimplification::Loss,
        LinkSimplification::Fragmentation,
        LinkSimplification::Bandwidth,
        LinkSimplification::Duplication,
        LinkSimplification::Jitter,
        LinkSimplification::Delay,
        LinkSimplification::Retransmission,
        LinkSimplification::Clean,
    ] {
        for location in link_locations(&current) {
            if !evaluator.has_budget(2) {
                break;
            }
            let candidate = simplify_link_at(&current, location, simplification);
            if candidate != current && evaluator.confirm(&candidate, &current_options).is_some() {
                current = candidate;
            }
        }
        if !evaluator.has_budget(2) {
            break;
        }
        let candidate = simplify_links(&current, simplification);
        if candidate != current && evaluator.confirm(&candidate, &current_options).is_some() {
            current = candidate;
        }
    }

    Ok(ShrinkResult {
        schedule: current,
        options: current_options,
        runs: evaluator.runs,
        elapsed: evaluator.started.elapsed(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::stubs::StateStub;
    use crate::simulation::harness::oracle::{OracleFailure, Verdict};
    use crate::simulation::harness::run;
    use crate::simulation::harness::schedule::{
        BackgroundNoise, FrameModel, SimConfig, SCHEDULE_SCHEMA_VERSION,
    };

    fn clean_schedule(n_players: usize, steps: u32) -> Schedule {
        let mut config = SimConfig::smoke(n_players);
        config.noise = BackgroundNoise::Clean;
        config.steps = steps;
        let initial_links = (0..n_players)
            .flat_map(|from| {
                (0..n_players)
                    .filter(move |to| from != *to)
                    .map(move |to| (from, to, LinkPolicy::clean()))
            })
            .collect();
        Schedule {
            schema_version: SCHEDULE_SCHEMA_VERSION,
            seed: 0x5A11_0001,
            link_seed: 0x5A11_0002,
            config,
            initial_links,
            events: Vec::new(),
            heal_at: steps,
        }
    }

    fn schedule_with_heal(n_players: usize, steps: u32, heal_at: u32) -> Schedule {
        let mut schedule = clean_schedule(n_players, steps);
        schedule.events.push((heal_at, ScheduleEvent::HealAll));
        schedule.heal_at = heal_at;
        schedule
    }

    fn state_failure(peer: usize) -> OracleFailure {
        OracleFailure::StateDivergence {
            frame: 7,
            peer,
            first_author: 0,
            expected: StateStub { frame: 7, state: 1 },
            actual: StateStub { frame: 7, state: 2 },
        }
    }

    fn end_progress_failure(peer: usize) -> OracleFailure {
        OracleFailure::EndProgress {
            peer,
            state: fortress_rollback::SessionState::Running,
            confirmed: 0,
            required: 30,
        }
    }

    fn synthetic_report(
        schedule: &Schedule,
        failures: Vec<OracleFailure>,
        trace_hash: u64,
    ) -> RunReport {
        let n = schedule.config.n_players;
        let final_confirmed = vec![i32::try_from(schedule.config.steps).unwrap_or(i32::MAX); n];
        RunReport {
            replay_options: RunOptions::default(),
            replay_input_width_bytes: 4,
            verdict: Verdict {
                failures,
                recovered_within_b: None,
                violation_allowlist_hits: Vec::new(),
            },
            trace_hash,
            final_confirmed,
            trace_tail: Vec::new(),
            probe_confirmed: Vec::new(),
            probe_peer_wire_by_link: BTreeMap::new(),
            pending_output_probe: None,
            net_stats: crate::common::sim_net::SimNetStats::default(),
            blocked_drops_by_link: BTreeMap::new(),
            fragmentation_drops_by_link: BTreeMap::new(),
            link_stats_by_link: BTreeMap::new(),
            load_game_state_observations: Vec::new(),
            metrics: vec![fortress_rollback::SessionMetrics::default(); n],
            progress_samples: Vec::new(),
            frame_opportunities: Vec::new(),
            wait_frames_obeyed: Vec::new(),
            peer_wire: vec![super::super::PeerWireTotals::default(); n],
            confirmed_at_heal: Vec::new(),
            confirmed_after_recovery: Vec::new(),
            recovered_within_b: None,
            violation_census: BTreeMap::new(),
            peer_event_counts: BTreeMap::new(),
            peer_event_counts_by_peer: vec![BTreeMap::new(); n],
            peer_event_payload_counts_by_peer: vec![BTreeMap::new(); n],
            spectator_applied_frames: 0,
            spectator_max_frame: None,
            spectator_final_hosts: None,
        }
    }

    fn stable_state_failure(schedule: &Schedule) -> RunReport {
        synthetic_report(schedule, vec![state_failure(1)], 0x5151)
    }

    /// Explicit operator entry point for reducing a captured nightly failure.
    /// It stays ignored so ordinary test runs never spend the shrink budget.
    #[test]
    #[ignore = "manual failure-artifact shrink; requires FORTRESS_SIM_SHRINK_* paths"]
    fn shrink_failure_artifact_from_environment() {
        use crate::simulation::harness::artifact::{
            read_artifact, write_artifact, ExistingArtifact, FailureArtifact,
        };
        use crate::simulation::harness::{run_with_input, WideStubInput};

        let input = std::env::var("FORTRESS_SIM_SHRINK_ARTIFACT")
            .expect("FORTRESS_SIM_SHRINK_ARTIFACT must name an artifact");
        let output_root = std::env::var("FORTRESS_SIM_SHRINK_OUTPUT")
            .expect("FORTRESS_SIM_SHRINK_OUTPUT must name an output directory");
        let artifact = read_artifact(std::path::Path::new(&input))
            .unwrap_or_else(|error| panic!("invalid shrink artifact: {error}"));
        let target = artifact.failures[0].class.as_str();
        let width = artifact.replay_input_width_bytes;
        let max_runs = std::env::var("FORTRESS_SIM_SHRINK_MAX_RUNS")
            .ok()
            .map_or(DEFAULT_MAX_RUNS, |value| {
                value.parse::<usize>().expect("max runs is an integer")
            });
        let result = shrink_failure(
            &artifact.schedule,
            &artifact.replay_options,
            target,
            ShrinkConfig {
                max_runs,
                ..ShrinkConfig::default()
            },
            |schedule, options| match width {
                4 => run(schedule, options),
                32 => run_with_input::<WideStubInput>(schedule, options),
                other => panic!("unsupported replay input width {other}"),
            },
            |schedule, _, report| {
                let minimized = FailureArtifact::from_report(schedule, report);
                write_artifact(
                    std::path::Path::new(&output_root),
                    "minimized",
                    &minimized,
                    ExistingArtifact::Replace,
                )
                .unwrap_or_else(|error| panic!("failed to persist minimized artifact: {error}"));
            },
        )
        .unwrap_or_else(|error| panic!("shrink failed: {error}"));
        assert!(result.runs >= 2, "shrink must confirm the initial failure");
    }

    #[test]
    fn planted_eight_peer_state_divergence_shrinks_to_small_short_repro() {
        const ORIGINAL_STEPS: u32 = 160;
        let mut schedule = clean_schedule(8, ORIGINAL_STEPS);
        schedule.config.desync_interval = 1_000;
        let options = RunOptions {
            corrupt_state_from: Some((7, 8)),
            ..RunOptions::default()
        };
        let mut accepted = 0usize;
        let result = shrink_failure(
            &schedule,
            &options,
            "StateDivergence",
            ShrinkConfig {
                max_runs: 160,
                max_duration: Duration::from_secs(60),
            },
            run,
            |_, _, _| accepted += 1,
        )
        .expect("planted deterministic failure shrinks");

        assert!(result.schedule.config.n_players <= 3, "{result:#?}");
        assert!(
            result.schedule.config.steps <= ORIGINAL_STEPS / 4,
            "{result:#?}"
        );
        assert!(result.runs <= 160);
        assert!(accepted > 0, "accepted-candidate hook never fired");
        let replay = run(&result.schedule, &result.options);
        assert_eq!(
            replay.verdict.failures.first().map(OracleFailure::class),
            Some("StateDivergence")
        );
    }

    #[test]
    fn peer_removal_remaps_every_event_and_run_option_axis() {
        let mut schedule = clean_schedule(4, 30);
        schedule.config.clock_skew_ppm = vec![10, 20, 30, 40];
        schedule.config.starve_events = vec![0, 2, 3];
        schedule.config.spectator_hosts = vec![0, 3];
        schedule.events = vec![
            (
                1,
                ScheduleEvent::SetLink {
                    from: 2,
                    to: 3,
                    policy: LinkPolicy::clean(),
                },
            ),
            (
                2,
                ScheduleEvent::Block {
                    from: 0,
                    to: 1,
                    blocked: true,
                },
            ),
            (
                3,
                ScheduleEvent::Hold {
                    from: 3,
                    to: 0,
                    holding: true,
                },
            ),
            (4, ScheduleEvent::PeerStall { peer: 3, steps: 2 }),
            (5, ScheduleEvent::SetInputDelay { peer: 2, delay: 3 }),
            (6, ScheduleEvent::GracefulRemove { by: 0, target: 3 }),
            (7, ScheduleEvent::LegacyDisconnect { by: 2, target: 3 }),
            (8, ScheduleEvent::PeerKill { peer: 2 }),
            (10, ScheduleEvent::HotJoin { slot: 3 }),
            (11, ScheduleEvent::SpectatorHostKill { host: 0 }),
            (20, ScheduleEvent::HealAll),
        ];
        schedule.heal_at = 20;
        let options = RunOptions {
            corrupt_state_from: Some((3, 8)),
            corrupt_checksum_from: Some((1, 9)),
            probe_confirmed_at: Some(19),
            pending_output_probe_link: Some((3, 0)),
            corrupt_spectator_input_from: Some(11),
            corrupt_spectator_status_from: Some(12),
        };
        let (candidate, mapped) = remove_peer(&schedule, &options, 1).expect("can remove peer");

        assert_eq!(candidate.config.n_players, 3);
        assert_eq!(candidate.config.clock_skew_ppm, vec![10, 30, 40]);
        assert_eq!(candidate.config.starve_events, vec![0, 1, 2]);
        assert_eq!(candidate.config.spectator_hosts, vec![0, 2]);
        assert_eq!(mapped.corrupt_state_from, Some((2, 8)));
        assert_eq!(mapped.corrupt_checksum_from, None);
        assert_eq!(mapped.probe_confirmed_at, Some(19));
        assert_eq!(mapped.pending_output_probe_link, Some((2, 0)));
        assert_eq!(mapped.corrupt_spectator_input_from, Some(11));
        assert_eq!(mapped.corrupt_spectator_status_from, Some(12));
        assert_eq!(candidate.events.len(), schedule.events.len() - 1);
        assert!(matches!(
            candidate.events[0].1,
            ScheduleEvent::SetLink { from: 1, to: 2, .. }
        ));
        assert!(matches!(
            candidate.events[1].1,
            ScheduleEvent::Hold { from: 2, to: 0, .. }
        ));
        assert!(candidate
            .events
            .iter()
            .any(|(_, event)| matches!(event, ScheduleEvent::PeerStall { peer: 2, .. })));
        assert!(candidate
            .events
            .iter()
            .any(|(_, event)| matches!(event, ScheduleEvent::SetInputDelay { peer: 1, .. })));
        assert!(candidate
            .events
            .iter()
            .any(|(_, event)| matches!(event, ScheduleEvent::GracefulRemove { by: 0, target: 2 })));
        assert!(candidate.events.iter().any(|(_, event)| matches!(
            event,
            ScheduleEvent::LegacyDisconnect { by: 1, target: 2 }
        )));
        assert!(candidate
            .events
            .iter()
            .any(|(_, event)| matches!(event, ScheduleEvent::PeerKill { peer: 1 })));
        assert!(candidate
            .events
            .iter()
            .any(|(_, event)| matches!(event, ScheduleEvent::HotJoin { slot: 2 })));
        assert!(candidate
            .events
            .iter()
            .any(|(_, event)| matches!(event, ScheduleEvent::SpectatorHostKill { host: 0 })));
        assert!(matches!(
            candidate.events.last().map(|(_, event)| event),
            Some(ScheduleEvent::HealAll)
        ));
    }

    #[test]
    fn peer_removal_remaps_rebind_on_a_valid_current_schedule() {
        let mut schedule = clean_schedule(4, 20);
        schedule.events = vec![(9, ScheduleEvent::Rebind { peer: 3 })];

        let (candidate, _) =
            remove_peer(&schedule, &RunOptions::default(), 1).expect("can remove peer");

        assert_eq!(
            candidate.events,
            vec![(9, ScheduleEvent::Rebind { peer: 2 })]
        );
    }

    #[test]
    fn peer_removal_prunes_special_events_with_removed_dependencies() {
        let mut schedule = clean_schedule(4, 20);
        schedule.config.spectator_hosts = vec![1, 3];
        schedule.events = vec![
            (2, ScheduleEvent::PeerKill { peer: 0 }),
            (3, ScheduleEvent::HotJoin { slot: 2 }),
            (4, ScheduleEvent::SpectatorHostKill { host: 1 }),
        ];

        let (candidate, _) =
            remove_peer(&schedule, &RunOptions::default(), 1).expect("can remove peer");

        assert!(candidate
            .events
            .iter()
            .all(|(_, event)| !matches!(event, ScheduleEvent::HotJoin { .. })));
        assert!(candidate
            .events
            .iter()
            .all(|(_, event)| !matches!(event, ScheduleEvent::SpectatorHostKill { .. })));
        assert!(candidate
            .events
            .iter()
            .any(|(_, event)| matches!(event, ScheduleEvent::PeerKill { peer: 0 })));
    }

    #[test]
    fn sparse_clock_skew_remap_does_not_expand_implicit_zero_entries() {
        let mut schedule = clean_schedule(4, 20);
        schedule.config.clock_skew_ppm = vec![10, 20];

        let (removed_implicit, _) =
            remove_peer(&schedule, &RunOptions::default(), 3).expect("can remove peer");
        let (removed_explicit, _) =
            remove_peer(&schedule, &RunOptions::default(), 0).expect("can remove peer");

        assert_eq!(removed_implicit.config.clock_skew_ppm, vec![10, 20]);
        assert_eq!(removed_explicit.config.clock_skew_ppm, vec![20]);
    }

    #[test]
    fn confirmation_rejects_alternating_trace_hash_before_callback() {
        let schedule = clean_schedule(2, 2);
        let mut calls = 0usize;
        let mut accepted = 0usize;

        let result = shrink_failure(
            &schedule,
            &RunOptions::default(),
            "StateDivergence",
            ShrinkConfig {
                max_runs: 20,
                max_duration: Duration::from_secs(10),
            },
            |candidate, _| {
                calls += 1;
                synthetic_report(candidate, vec![state_failure(1)], calls as u64)
            },
            |_, _, _| accepted += 1,
        );

        assert!(result.is_err());
        assert_eq!(calls, 2);
        assert_eq!(accepted, 0);
    }

    #[test]
    fn confirmation_rejects_changed_failure_identity_and_final_summary() {
        for mismatch in ["failure-sequence", "final-summary"] {
            let schedule = clean_schedule(2, 2);
            let mut calls = 0usize;
            let mut accepted = 0usize;
            let result = shrink_failure(
                &schedule,
                &RunOptions::default(),
                "StateDivergence",
                ShrinkConfig {
                    max_runs: 20,
                    max_duration: Duration::from_secs(10),
                },
                |candidate, _| {
                    calls += 1;
                    let failures = if mismatch == "failure-sequence" && calls == 2 {
                        vec![state_failure(1), end_progress_failure(0)]
                    } else {
                        vec![state_failure(1)]
                    };
                    let mut report = synthetic_report(candidate, failures, 0x00A1_1CE5);
                    if mismatch == "final-summary" && calls == 2 {
                        report.final_confirmed[0] += 1;
                    }
                    report
                },
                |_, _, _| accepted += 1,
            );

            assert!(result.is_err(), "mismatch={mismatch}");
            assert_eq!(calls, 2, "mismatch={mismatch}");
            assert_eq!(accepted, 0, "mismatch={mismatch}");
        }
    }

    #[test]
    fn target_class_must_be_the_first_failure() {
        let schedule = clean_schedule(2, 2);
        let mut calls = 0usize;
        let mut accepted = 0usize;

        let result = shrink_failure(
            &schedule,
            &RunOptions::default(),
            "StateDivergence",
            ShrinkConfig {
                max_runs: 20,
                max_duration: Duration::from_secs(10),
            },
            |candidate, _| {
                calls += 1;
                synthetic_report(
                    candidate,
                    vec![end_progress_failure(0), state_failure(1)],
                    0x000F_1A57,
                )
            },
            |_, _, _| accepted += 1,
        );

        assert!(result.is_err());
        assert_eq!(calls, 1);
        assert_eq!(accepted, 0);
    }

    #[test]
    fn panicking_candidates_are_rejected_without_aborting_the_shrink() {
        let schedule = clean_schedule(3, 2);
        let result = shrink_failure(
            &schedule,
            &RunOptions::default(),
            "StateDivergence",
            ShrinkConfig {
                max_runs: 20,
                max_duration: Duration::from_secs(10),
            },
            |candidate, _| {
                assert_ne!(candidate.config.n_players, 2, "planted candidate panic");
                synthetic_report(candidate, vec![state_failure(1)], 0xC0DE_CAFE)
            },
            |_, _, _| {},
        )
        .expect("the original remains a valid best-so-far result");

        assert_eq!(result.schedule.config.n_players, 3);
        assert!(result.runs <= 20);
    }

    #[test]
    fn prefix_search_finds_an_isolated_short_failure_without_monotonicity() {
        let schedule = clean_schedule(2, 12);
        let result = shrink_failure(
            &schedule,
            &RunOptions::default(),
            "StateDivergence",
            ShrinkConfig {
                max_runs: 80,
                max_duration: Duration::from_secs(10),
            },
            |candidate, _| {
                let failures = matches!(candidate.config.steps, 4 | 12)
                    .then(|| vec![state_failure(1)])
                    .unwrap_or_default();
                synthetic_report(candidate, failures, 0x1501_A7ED)
            },
            |_, _, _| {},
        )
        .expect("the isolated short prefix reproduces");

        assert_eq!(result.schedule.config.steps, 4);
    }

    #[test]
    fn long_prefix_search_reaches_late_event_within_bounded_budget() {
        let mut schedule = clean_schedule(2, 5_000);
        schedule.events = vec![
            (618, ScheduleEvent::PeerStall { peer: 1, steps: 20 }),
            (4_750, ScheduleEvent::HealAll),
        ];
        schedule.heal_at = 4_750;
        let result = shrink_failure(
            &schedule,
            &RunOptions::default(),
            "StateDivergence",
            ShrinkConfig {
                max_runs: 80,
                max_duration: Duration::from_secs(10),
            },
            |candidate, _| {
                let failures = if matches!(candidate.config.steps, 634 | 5_000) {
                    vec![state_failure(1), end_progress_failure(1)]
                } else {
                    // Target class alone is insufficient: a short schedule can
                    // fail for a different mechanism. The baseline's class set
                    // must remain represented.
                    vec![state_failure(1)]
                };
                synthetic_report(candidate, failures, 0x1A7E_E7E0)
            },
            |_, _, _| {},
        )
        .expect("event-adjacent checkpoint reaches the late failure");

        assert_eq!(result.schedule.config.steps, 634);
        assert!(result.runs <= 80);
    }

    #[test]
    fn truncation_and_ddmin_preserve_one_final_heal_with_matching_anchor() {
        let mut schedule = schedule_with_heal(2, 12, 9);
        schedule.events.splice(
            0..0,
            [
                (
                    2,
                    ScheduleEvent::Block {
                        from: 0,
                        to: 1,
                        blocked: true,
                    },
                ),
                (
                    5,
                    ScheduleEvent::Hold {
                        from: 1,
                        to: 0,
                        holding: true,
                    },
                ),
            ],
        );
        let result = shrink_failure(
            &schedule,
            &RunOptions::default(),
            "StateDivergence",
            ShrinkConfig {
                max_runs: 60,
                max_duration: Duration::from_secs(10),
            },
            |candidate, _| {
                let failures = if candidate
                    .events
                    .iter()
                    .any(|(_, event)| matches!(event, ScheduleEvent::HealAll))
                {
                    vec![state_failure(1)]
                } else {
                    Vec::new()
                };
                synthetic_report(candidate, failures, 0x0EA1)
            },
            |_, _, _| {},
        )
        .expect("heal-dependent failure shrinks");

        validate_schedule(&result.schedule).expect("shrunk schedule remains materializable");
        let heals: Vec<u32> = result
            .schedule
            .events
            .iter()
            .filter_map(|(step, event)| matches!(event, ScheduleEvent::HealAll).then_some(*step))
            .collect();
        assert_eq!(heals, vec![result.schedule.heal_at]);
        assert_eq!(result.schedule.heal_at, result.schedule.config.steps - 1);
        assert!(matches!(
            result.schedule.events.last(),
            Some((_, ScheduleEvent::HealAll))
        ));
    }

    #[test]
    fn truncation_derives_heal_from_event_when_metadata_is_stale() {
        let mut schedule = schedule_with_heal(2, 12, 9);
        schedule.heal_at = 2;

        let (truncated, _) = truncate_schedule(&schedule, &RunOptions::default(), 6);

        assert_eq!(actual_final_heal(&truncated), Some(5));
        assert_eq!(truncated.heal_at, 5);
    }

    #[test]
    fn individual_link_simplification_keeps_only_the_required_dimension() {
        let mut schedule = clean_schedule(2, 2);
        let required = schedule
            .initial_links
            .iter_mut()
            .find(|(from, to, _)| (*from, *to) == (0, 1))
            .expect("required link exists");
        required.2.base_delay = Duration::from_millis(7);
        let noise = LinkPolicy {
            drop_rate: 0.2,
            dup_rate: 0.1,
            base_delay: Duration::from_millis(9),
            jitter: Duration::from_millis(3),
            burst_rate: 0.1,
            burst_len: 2,
            retransmit_delay: Duration::from_millis(4),
            gilbert_elliott: None,
            fragmentation: None,
            bandwidth: None,
        };
        schedule
            .initial_links
            .iter_mut()
            .find(|(from, to, _)| (*from, *to) == (1, 0))
            .expect("noise link exists")
            .2 = noise.clone();
        schedule.events.push((
            0,
            ScheduleEvent::SetLink {
                from: 1,
                to: 0,
                policy: noise,
            },
        ));

        let result = shrink_failure(
            &schedule,
            &RunOptions::default(),
            "StateDivergence",
            ShrinkConfig {
                max_runs: 160,
                max_duration: Duration::from_secs(10),
            },
            |candidate, _| {
                let required_delay = candidate
                    .initial_links
                    .iter()
                    .find(|(from, to, _)| (*from, *to) == (0, 1))
                    .map(|(_, _, policy)| policy.base_delay)
                    .unwrap_or_default();
                let retained_set_link = candidate
                    .events
                    .iter()
                    .any(|(_, event)| matches!(event, ScheduleEvent::SetLink { .. }));
                let failures = if required_delay > Duration::ZERO && retained_set_link {
                    vec![state_failure(1)]
                } else {
                    Vec::new()
                };
                synthetic_report(candidate, failures, 0x11A5)
            },
            |_, _, _| {},
        )
        .expect("one link dimension is sufficient");

        let required = &result
            .schedule
            .initial_links
            .iter()
            .find(|(from, to, _)| (*from, *to) == (0, 1))
            .expect("required link retained")
            .2;
        let irrelevant = &result
            .schedule
            .initial_links
            .iter()
            .find(|(from, to, _)| (*from, *to) == (1, 0))
            .expect("irrelevant link retained")
            .2;
        let set_link = result
            .schedule
            .events
            .iter()
            .find_map(|(_, event)| match event {
                ScheduleEvent::SetLink { policy, .. } => Some(policy),
                _ => None,
            })
            .expect("failure requires SetLink to remain");
        assert_eq!(required.base_delay, Duration::from_millis(7));
        assert_eq!(
            LinkPolicy {
                base_delay: Duration::ZERO,
                ..required.clone()
            },
            LinkPolicy::clean()
        );
        assert_eq!(irrelevant, &LinkPolicy::clean());
        assert_eq!(set_link, &LinkPolicy::clean());
    }

    #[test]
    fn loss_simplification_removes_gilbert_elliott_without_touching_other_axes() {
        let mut policy = LinkPolicy {
            dup_rate: 0.25,
            gilbert_elliott: Some(GilbertElliottPolicy {
                good_to_bad: 0.05,
                bad_to_good: 0.20,
                good_drop_rate: 0.01,
                bad_drop_rate: 0.80,
            }),
            ..LinkPolicy::clean()
        };

        simplify_link(&mut policy, LinkSimplification::Loss);

        assert_eq!(policy.gilbert_elliott, None);
        assert!((policy.dup_rate - 0.25).abs() < f64::EPSILON);
    }

    #[test]
    fn fragmentation_simplification_removes_only_fragmentation() {
        let policy = LinkPolicy {
            dup_rate: 0.25,
            fragmentation: Some(FragmentationPolicy {
                fragment_drop_rate: 0.2,
            }),
            ..LinkPolicy::clean()
        };
        let mut schedule = clean_schedule(2, 2);
        schedule.initial_links[0].2 = policy.clone();
        schedule.events.insert(
            0,
            (
                1,
                ScheduleEvent::SetLink {
                    from: 0,
                    to: 1,
                    policy,
                },
            ),
        );

        let simplified = simplify_links(&schedule, LinkSimplification::Fragmentation);

        assert_eq!(simplified.initial_links[0].2.fragmentation, None);
        assert!((simplified.initial_links[0].2.dup_rate - 0.25).abs() < f64::EPSILON);
        let ScheduleEvent::SetLink { policy, .. } = &simplified.events[0].1 else {
            panic!("first event must remain SetLink");
        };
        assert_eq!(policy.fragmentation, None);
        assert!((policy.dup_rate - 0.25).abs() < f64::EPSILON);
    }

    #[test]
    fn bandwidth_simplification_removes_only_bandwidth() {
        let mut policy = LinkPolicy {
            dup_rate: 0.25,
            bandwidth: Some(BandwidthPolicy {
                rate_bytes_per_second: 10_000,
                burst_bytes: 1_500,
                queue_capacity_bytes: 3_000,
            }),
            ..LinkPolicy::clean()
        };

        simplify_link(&mut policy, LinkSimplification::Bandwidth);

        assert_eq!(policy.bandwidth, None);
        assert!((policy.dup_rate - 0.25).abs() < f64::EPSILON);
    }

    #[test]
    fn individual_clock_and_cadence_simplification_preserves_required_axes() {
        let mut clock_schedule = clean_schedule(2, 2);
        clock_schedule.config.clock_skew_ppm = vec![100, 200];
        let clock_result = shrink_failure(
            &clock_schedule,
            &RunOptions::default(),
            "StateDivergence",
            ShrinkConfig {
                max_runs: 80,
                max_duration: Duration::from_secs(10),
            },
            |candidate, _| {
                let failures = if candidate.config.clock_skew_ppm.first() == Some(&100) {
                    vec![state_failure(1)]
                } else {
                    Vec::new()
                };
                synthetic_report(candidate, failures, 0xC10C)
            },
            |_, _, _| {},
        )
        .expect("one clock skew is sufficient");
        assert_eq!(clock_result.schedule.config.clock_skew_ppm, vec![100, 0]);

        let mut cadence_schedule = clean_schedule(2, 2);
        cadence_schedule.config.step_dt_ms = 25;
        cadence_schedule.config.input_delay = 3;
        cadence_schedule.config.max_prediction = 4;
        cadence_schedule.config.desync_interval = 41;
        let cadence_result = shrink_failure(
            &cadence_schedule,
            &RunOptions::default(),
            "StateDivergence",
            ShrinkConfig {
                max_runs: 80,
                max_duration: Duration::from_secs(10),
            },
            |candidate, _| {
                let failures = if candidate.config.step_dt_ms == 25 {
                    vec![state_failure(1)]
                } else {
                    Vec::new()
                };
                synthetic_report(candidate, failures, 0x00CA_DECE)
            },
            |_, _, _| {},
        )
        .expect("one cadence axis is sufficient");
        assert_eq!(cadence_result.schedule.config.step_dt_ms, 25);
        assert_eq!(cadence_result.schedule.config.input_delay, 0);
        assert_eq!(cadence_result.schedule.config.max_prediction, 8);
        assert_eq!(cadence_result.schedule.config.desync_interval, 30);
    }

    #[test]
    fn frame_model_shrinking_collapses_or_retains_the_required_gate() {
        let mut collapsible = clean_schedule(2, 2);
        collapsible.config.frame_model = FrameModel::SkewGated60Hz;
        let collapsed = shrink_failure(
            &collapsible,
            &RunOptions::default(),
            "StateDivergence",
            ShrinkConfig {
                max_runs: 40,
                max_duration: Duration::from_secs(10),
            },
            |candidate, _| synthetic_report(candidate, vec![state_failure(1)], 0xC011_A95E),
            |_, _, _| {},
        )
        .expect("failure persists without the frame gate");
        assert_eq!(collapsed.schedule.config.frame_model, FrameModel::Lockstep);

        let mut required = clean_schedule(2, 2);
        required.config.frame_model = FrameModel::SkewGated60Hz;
        required.config.clock_skew_ppm = vec![1_000, 2_000];
        let retained = shrink_failure(
            &required,
            &RunOptions::default(),
            "StateDivergence",
            ShrinkConfig {
                max_runs: 60,
                max_duration: Duration::from_secs(10),
            },
            |candidate, _| {
                let failures = if candidate.config.frame_model == FrameModel::SkewGated60Hz
                    && candidate.config.clock_skew_ppm.first() == Some(&1_000)
                {
                    vec![state_failure(1)]
                } else {
                    Vec::new()
                };
                synthetic_report(candidate, failures, 0x6A7E)
            },
            |_, _, _| {},
        )
        .expect("failure requires the frame gate and first peer skew");
        assert_eq!(
            retained.schedule.config.frame_model,
            FrameModel::SkewGated60Hz
        );
        assert_eq!(retained.schedule.config.clock_skew_ppm, vec![1_000, 0]);
    }

    #[test]
    fn invalid_inputs_and_bounds_do_not_overrun_the_runner_or_callback() {
        struct Case {
            max_runs: usize,
            max_duration: Duration,
            expected_runs: usize,
            expected_callbacks: usize,
            succeeds: bool,
        }
        let cases = [
            Case {
                max_runs: 1,
                max_duration: Duration::from_secs(10),
                expected_runs: 0,
                expected_callbacks: 0,
                succeeds: false,
            },
            Case {
                max_runs: 2,
                max_duration: Duration::from_secs(10),
                expected_runs: 2,
                expected_callbacks: 1,
                succeeds: true,
            },
            Case {
                max_runs: 20,
                max_duration: Duration::ZERO,
                expected_runs: 0,
                expected_callbacks: 0,
                succeeds: false,
            },
        ];
        for case in cases {
            let schedule = clean_schedule(2, 2);
            let mut runs = 0usize;
            let mut callbacks = 0usize;
            let result = shrink_failure(
                &schedule,
                &RunOptions::default(),
                "StateDivergence",
                ShrinkConfig {
                    max_runs: case.max_runs,
                    max_duration: case.max_duration,
                },
                |candidate, _| {
                    runs += 1;
                    stable_state_failure(candidate)
                },
                |_, _, _| callbacks += 1,
            );
            assert_eq!(result.is_ok(), case.succeeds);
            assert_eq!(runs, case.expected_runs);
            assert_eq!(callbacks, case.expected_callbacks);
        }

        let mut invalid = clean_schedule(2, 2);
        invalid.config.n_players = 1;
        let mut runs = 0usize;
        let mut callbacks = 0usize;
        let result = shrink_failure(
            &invalid,
            &RunOptions::default(),
            "StateDivergence",
            ShrinkConfig::default(),
            |candidate, _| {
                runs += 1;
                stable_state_failure(candidate)
            },
            |_, _, _| callbacks += 1,
        );
        assert!(result.is_err());
        assert_eq!(runs, 0);
        assert_eq!(callbacks, 0);
    }
}
