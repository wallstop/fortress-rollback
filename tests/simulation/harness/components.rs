//! Component-scoped confirmed-history evidence for partition/membership rows.
//!
//! A session-level oracle compares every peer against one global confirmed
//! history. That is the right check while one mesh can still hear every member,
//! but it cannot describe a genuine partition: the global live-peer minimum
//! pins on whichever component failed closed and says nothing about a component
//! that kept confirming on its own. The CAP rows therefore declare a component
//! layout up front and compare each component's exposed confirmed prefix
//! independently.
//!
//! The classifier deliberately separates the three legal outcomes from the
//! defect: components that all stop confirm **fail closed**, a component that
//! continues alone is an explicit **split match**, but two components exposing
//! different values for the same confirmed frame are a **fork** and must never
//! be reported as a healthy continuation.

use super::oracle::InputFingerprint;
use super::TraceSessionState;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Declared component membership for a CAP row: one component id per logical
/// peer. Peers sharing an id are in the same partition component.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentScopeOptions {
    pub component: Vec<usize>,
}

impl ComponentScopeOptions {
    /// Validates the declared layout against the run's peer count.
    ///
    /// # Errors
    /// Returns a human-readable reason when the layout cannot describe a
    /// partition (wrong length, an id outside `0..n_players`, or fewer than two
    /// non-empty components).
    pub fn validate(&self, n_players: usize) -> Result<(), String> {
        if self.component.len() != n_players {
            return Err(format!(
                "component scope has {} entries for {n_players} peers",
                self.component.len()
            ));
        }
        let mut counts = Vec::new();
        for component in &self.component {
            if *component >= n_players {
                return Err(format!("component id {component} is not in 0..{n_players}"));
            }
            if counts.len() <= *component {
                counts.resize(component.saturating_add(1), 0_usize);
            }
            counts[*component] = counts[*component].saturating_add(1);
        }
        counts.retain(|count| *count > 0);
        if counts.len() < 2 {
            return Err("component scope requires at least two non-empty components".to_owned());
        }
        Ok(())
    }
}

/// Component-scoped evidence captured by the runner.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ComponentScopeEvidence {
    /// Component id per logical peer.
    pub component: Vec<usize>,
    /// Per-peer confirmed-input history sampled while the peer was alive. Frames
    /// are contiguous from zero for every peer that confirmed anything.
    pub confirmed_inputs: Vec<BTreeMap<i32, Vec<InputFingerprint>>>,
    /// Every peer's confirmed frame at the end of the run.
    pub final_confirmed: Vec<i32>,
    /// Every peer's session state at the end of the run.
    pub final_states: Vec<TraceSessionState>,
    /// Confirmed frame per peer sampled when the declared components were first
    /// isolated (the first cross-component `Block` or declared `PeerStall`).
    /// Empty when no declared disruption ever fired.
    pub cut_confirmed: Vec<i32>,
}

/// Outcome of the component-scoped comparison.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComponentOutcome {
    /// Every component stopped confirming at or before the partition cut and
    /// the components' shared confirmed prefix is identical.
    FailedClosed { shared_prefix: i32 },
    /// Every component confirmed strictly past the cut and every shared frame
    /// still agreed: the disruption healed without a second history.
    ContinuedTogether { shared_prefix: i32 },
    /// At least one, but not every, component confirmed strictly past the cut
    /// while every shared frame still agreed. This is a one-sided continuation:
    /// an explicit split-branch outcome, not a silent fork.
    SplitMatch {
        shared_prefix: i32,
        continued: Vec<usize>,
    },
    /// A component that failed closed exposed a bounded tail of its own live
    /// inputs after the last frame every component agreed on. When `continued`
    /// is non-empty, those components are the survivor branch and the tail is
    /// discarded with it; when it is empty, every component failed closed and
    /// the tail is discarded by the documented terminal (Halt) outcome. This is
    /// an explicit one-way-partition split decision, never a silent fork.
    /// Reconciliation requires rebuilding the failed component.
    SplitTail {
        shared_prefix: i32,
        continued: Vec<usize>,
        /// Frames the failed component confirmed past `shared_prefix`.
        tail: i32,
    },
    /// Two components exposed different confirmed values for the same frame.
    Forked {
        frame: i32,
        component: usize,
        other_component: usize,
    },
    /// Peers inside one component disagreed on a shared confirmed frame.
    InternalDivergence { component: usize, frame: i32 },
    /// The evidence cannot support a comparison (missing layout or histories).
    Invalid { reason: &'static str },
}

impl ComponentOutcome {
    /// True when the outcome is one of the legal terminal outcomes.
    #[must_use]
    pub fn is_legal(&self) -> bool {
        matches!(
            self,
            Self::FailedClosed { .. }
                | Self::ContinuedTogether { .. }
                | Self::SplitMatch { .. }
                | Self::SplitTail { .. }
        )
    }
}

/// Compares the declared components' exposed confirmed prefixes.
///
/// `allowance` is the number of frames past the partition cut that still counts
/// as "stopped at the cut" (delivery delay means components observe the cut a
/// couple of frames apart). Members of one component must agree on every frame
/// they all confirmed; components must agree on every frame they both
/// confirmed. A component whose last confirmed frame exceeds
/// `partition_cut + allowance` is reported as having continued.
///
/// A divergence is always reported as [`ComponentOutcome::Forked`]: a failed
/// closed component's post-cut tail is only accepted by
/// [`classify_component_scope_with_tail`].
#[must_use]
pub fn classify_component_scope(
    evidence: &ComponentScopeEvidence,
    allowance: i32,
) -> ComponentOutcome {
    classify_component_scope_with_tail(evidence, allowance, 0)
}

/// Compares the declared components' exposed confirmed prefixes, accepting a
/// bounded terminal tail from a failed-closed component.
///
/// A one-way (asymmetric) partition is terminal for the isolated peer: it keeps
/// receiving the majority's traffic while none of its own inputs arrive, so it
/// can confirm its prediction window past the majority's certified cut before
/// it detects the dead outbound path and fails closed. That tail is not part of
/// the survivor history. Divergence is therefore legal only when all of the
/// following hold:
///
/// - the first divergent frame is strictly after `partition_cut + allowance`
///   (the shared prefix is untouched);
/// - exactly one side of the divergence is a continued component, the other is
///   a component that ended every member in `Synchronizing`;
/// - every failed-closed component's floor is within `tail_allowance` frames of
///   the last shared frame.
///
/// Everything else remains a fork. A zero `tail_allowance` makes this identical
/// to [`classify_component_scope`].
#[must_use]
pub fn classify_component_scope_with_tail(
    evidence: &ComponentScopeEvidence,
    allowance: i32,
    tail_allowance: i32,
) -> ComponentOutcome {
    let n = evidence.component.len();
    if n < 2
        || evidence.confirmed_inputs.len() != n
        || evidence.cut_confirmed.len() != n
        || evidence.final_states.len() != n
    {
        return ComponentOutcome::Invalid {
            reason: "component evidence is incomplete",
        };
    }
    let mut members: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (peer, component) in evidence.component.iter().enumerate() {
        members.entry(*component).or_default().push(peer);
    }
    if members.len() < 2 || members.values().any(Vec::is_empty) {
        return ComponentOutcome::Invalid {
            reason: "component layout needs at least two non-empty components",
        };
    }
    let component_ids: Vec<usize> = members.keys().copied().collect();

    // Last frame each peer confirmed (None when it never confirmed anything).
    let last_confirmed: Vec<Option<i32>> = evidence
        .confirmed_inputs
        .iter()
        .map(|history| history.keys().next_back().copied())
        .collect();
    if last_confirmed.iter().any(Option::is_none) {
        return ComponentOutcome::Invalid {
            reason: "a peer confirmed no frames",
        };
    }

    // Component floors and internal agreement.
    let mut floors = Vec::with_capacity(component_ids.len());
    for component in &component_ids {
        let Some(peers) = members.get(component) else {
            return ComponentOutcome::Invalid {
                reason: "component membership vanished",
            };
        };
        let floor = peers
            .iter()
            .filter_map(|peer| last_confirmed[*peer])
            .min()
            .unwrap_or(0);
        floors.push(floor);
        // Compare every frame any member confirmed, not only the frames below
        // the component floor: after a heal one member may resume while another
        // stays failed closed, and their continued tails must still agree.
        let max_frame = peers
            .iter()
            .filter_map(|peer| last_confirmed[*peer])
            .max()
            .unwrap_or(0);
        for frame in 0..=max_frame {
            let mut expected: Option<&Vec<InputFingerprint>> = None;
            for peer in peers {
                let Some(value) = evidence.confirmed_inputs[*peer].get(&frame) else {
                    continue;
                };
                match expected {
                    Some(expected) if expected != value => {
                        return ComponentOutcome::InternalDivergence {
                            component: *component,
                            frame,
                        };
                    },
                    Some(_) => {},
                    None => expected = Some(value),
                }
            }
        }
    }

    let shared = floors.iter().copied().min().unwrap_or(0);
    // A component that confirmed beyond the cut continued on its own.
    let cut = evidence
        .cut_confirmed
        .iter()
        .copied()
        .min()
        .unwrap_or(shared)
        .saturating_add(allowance.max(0));
    let component_floor = |component: usize| {
        component_ids
            .iter()
            .position(|candidate| *candidate == component)
            .and_then(|position| floors.get(position).copied())
            .unwrap_or(shared)
    };
    let component_all_in = |component: usize, state: TraceSessionState| {
        members.get(&component).is_some_and(|peers| {
            peers.iter().all(|peer| {
                evidence
                    .final_states
                    .get(*peer)
                    .is_some_and(|observed| *observed == state)
            })
        })
    };
    // A continued component must have confirmed past the cut and still be
    // running at the end of the run. A failed-closed component whose floor is
    // above the cut is carrying a terminal tail, not a survivor branch.
    let running_components: Vec<usize> = component_ids
        .iter()
        .copied()
        .filter(|component| {
            component_all_in(*component, TraceSessionState::Running)
                && component_floor(*component) > cut
        })
        .collect();

    // Cross-component agreement over every frame any pair of peers in
    // different components both confirmed. Comparing only the common floor
    // would miss the post-heal shape where one member of each component
    // resumed past a stopped peer and their continued tails disagree.
    let mut first_divergence: Option<(i32, usize, usize)> = None;
    for i in 0..component_ids.len() {
        for j in (i + 1)..component_ids.len() {
            let Some(left_peers) = members.get(&component_ids[i]) else {
                return ComponentOutcome::Invalid {
                    reason: "component membership vanished",
                };
            };
            let Some(right_peers) = members.get(&component_ids[j]) else {
                return ComponentOutcome::Invalid {
                    reason: "component membership vanished",
                };
            };
            for left_peer in left_peers {
                for right_peer in right_peers {
                    let Some(left_last) = last_confirmed[*left_peer] else {
                        return ComponentOutcome::Invalid {
                            reason: "missing peer history",
                        };
                    };
                    let Some(right_last) = last_confirmed[*right_peer] else {
                        return ComponentOutcome::Invalid {
                            reason: "missing peer history",
                        };
                    };
                    for frame in 0..=left_last.min(right_last) {
                        let left = evidence.confirmed_inputs[*left_peer].get(&frame);
                        let right = evidence.confirmed_inputs[*right_peer].get(&frame);
                        if left != right {
                            let replace =
                                first_divergence.is_none_or(|(earliest, _, _)| frame < earliest);
                            if replace {
                                first_divergence =
                                    Some((frame, component_ids[i], component_ids[j]));
                            }
                            break;
                        }
                    }
                }
            }
        }
    }

    let Some((frame, left_component, right_component)) = first_divergence else {
        // No frame was confirmed by both a survivor and the failed component
        // past the shared prefix. A failed component may still have advanced a
        // terminal tail (Halt, or a one-way partition whose majority stopped
        // before the isolated peer did); it is discarded like the divergent
        // case below.
        let max_terminal_floor = component_ids
            .iter()
            .copied()
            .filter(|component| !running_components.contains(component))
            .map(component_floor)
            .max()
            .unwrap_or(shared);
        let tail = max_terminal_floor.saturating_sub(shared);
        let terminal_tail = component_ids
            .iter()
            .copied()
            .filter(|component| !running_components.contains(component))
            .all(|component| component_all_in(component, TraceSessionState::Synchronizing));
        if tail > 0 {
            if tail <= tail_allowance.max(0) && terminal_tail {
                return ComponentOutcome::SplitTail {
                    shared_prefix: shared,
                    continued: running_components,
                    tail,
                };
            }
            if tail_allowance > 0 {
                let other = component_ids
                    .iter()
                    .copied()
                    .find(|component| !running_components.contains(component))
                    .unwrap_or(component_ids[0]);
                let component = running_components
                    .first()
                    .copied()
                    .or_else(|| component_ids.iter().copied().find(|c| *c != other))
                    .unwrap_or(other);
                return ComponentOutcome::Forked {
                    frame: shared.saturating_add(1),
                    component,
                    other_component: other,
                };
            }
            // A zero tail allowance keeps the legacy classification: a
            // one-frame delivery-slack tail is not a defect unless the caller
            // explicitly opted into bounding terminal tails.
        }
        if running_components.is_empty() {
            return ComponentOutcome::FailedClosed {
                shared_prefix: shared,
            };
        }
        if running_components.len() == component_ids.len() {
            return ComponentOutcome::ContinuedTogether {
                shared_prefix: shared,
            };
        }
        return ComponentOutcome::SplitMatch {
            shared_prefix: shared,
            continued: running_components,
        };
    };

    // A divergence inside the delivery-ambiguity window rewrites the shared
    // prefix and is never a legal split.
    if frame <= cut {
        return ComponentOutcome::Forked {
            frame,
            component: left_component,
            other_component: right_component,
        };
    }
    // A diverging component is terminal only when every member ended
    // `Synchronizing`; a continued component must have ended `Running` past the
    // cut. Two running components (or two terminal ones) that disagree are a
    // fork.
    let left_running = component_all_in(left_component, TraceSessionState::Running)
        && component_floor(left_component) > cut;
    let right_running = component_all_in(right_component, TraceSessionState::Running)
        && component_floor(right_component) > cut;
    let left_terminal = component_all_in(left_component, TraceSessionState::Synchronizing);
    let right_terminal = component_all_in(right_component, TraceSessionState::Synchronizing);
    let split_shape = (left_running && right_terminal) || (left_terminal && right_running);
    if !split_shape {
        return ComponentOutcome::Forked {
            frame,
            component: left_component,
            other_component: right_component,
        };
    }
    // The first divergent pair being a survivor/terminal pair is not enough:
    // two surviving components can also disagree on a later frame. Compare
    // every survivor pair over their full shared range before accepting the
    // split.
    let last_shared = frame.saturating_sub(1);
    for (index, left) in running_components.iter().enumerate() {
        for right in running_components.iter().skip(index.saturating_add(1)) {
            let (Some(left_peers), Some(right_peers)) = (members.get(left), members.get(right))
            else {
                return ComponentOutcome::Invalid {
                    reason: "component membership vanished",
                };
            };
            for left_peer in left_peers {
                for right_peer in right_peers {
                    let (Some(left_last), Some(right_last)) =
                        (last_confirmed[*left_peer], last_confirmed[*right_peer])
                    else {
                        return ComponentOutcome::Invalid {
                            reason: "missing peer history",
                        };
                    };
                    for frame in (last_shared + 1)..=left_last.min(right_last) {
                        let left_value = evidence.confirmed_inputs[*left_peer].get(&frame);
                        let right_value = evidence.confirmed_inputs[*right_peer].get(&frame);
                        if left_value != right_value {
                            return ComponentOutcome::Forked {
                                frame,
                                component: *left,
                                other_component: *right,
                            };
                        }
                    }
                }
            }
        }
    }
    let max_terminal_floor = component_ids
        .iter()
        .copied()
        .filter(|component| !running_components.contains(component))
        .map(component_floor)
        .max()
        .unwrap_or(last_shared);
    let tail = max_terminal_floor.saturating_sub(last_shared);
    if tail <= tail_allowance.max(0) {
        return ComponentOutcome::SplitTail {
            shared_prefix: last_shared,
            continued: running_components,
            tail,
        };
    }
    ComponentOutcome::Forked {
        frame,
        component: left_component,
        other_component: right_component,
    }
}

/// Builds a minimal peer history from `(logical, value)` inputs.
#[cfg(test)]
fn history(frames: impl IntoIterator<Item = (i32, u32)>) -> BTreeMap<i32, Vec<InputFingerprint>> {
    frames
        .into_iter()
        .map(|(frame, value)| {
            (
                frame,
                vec![InputFingerprint::from_bytes(value, &value.to_le_bytes())],
            )
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::panic, clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn build_evidence(
        component: Vec<usize>,
        histories: Vec<BTreeMap<i32, Vec<InputFingerprint>>>,
        cut_confirmed: Vec<i32>,
    ) -> ComponentScopeEvidence {
        let states = vec![TraceSessionState::Synchronizing; histories.len()];
        build_evidence_with_states(component, histories, cut_confirmed, states)
    }

    fn build_evidence_with_states(
        component: Vec<usize>,
        histories: Vec<BTreeMap<i32, Vec<InputFingerprint>>>,
        cut_confirmed: Vec<i32>,
        final_states: Vec<TraceSessionState>,
    ) -> ComponentScopeEvidence {
        let final_confirmed = histories
            .iter()
            .map(|history| history.keys().next_back().copied().unwrap_or(-1))
            .collect();
        ComponentScopeEvidence {
            component,
            confirmed_inputs: histories,
            final_confirmed,
            final_states,
            cut_confirmed,
        }
    }

    #[test]
    fn symmetric_partition_at_a_common_cut_fails_closed() {
        let shared = history([(0, 7), (1, 7), (2, 7)]);
        let evidence = build_evidence(
            vec![0, 0, 1, 1],
            vec![shared.clone(), shared.clone(), shared.clone(), shared],
            vec![2, 2, 2, 2],
        );
        assert_eq!(
            classify_component_scope(&evidence, 0),
            ComponentOutcome::FailedClosed { shared_prefix: 2 }
        );
    }

    #[test]
    fn different_values_on_a_shared_frame_are_a_fork() {
        let left = history([(0, 7), (1, 7), (2, 7)]);
        let mut right = left.clone();
        right.insert(2, vec![InputFingerprint::from_bytes(9, &[9, 9])]);
        let evidence = build_evidence(
            vec![0, 0, 1, 1],
            vec![left.clone(), left, right.clone(), right],
            vec![2, 2, 2, 2],
        );
        assert_eq!(
            classify_component_scope(&evidence, 0),
            ComponentOutcome::Forked {
                frame: 2,
                component: 0,
                other_component: 1,
            }
        );
    }

    #[test]
    fn peers_inside_one_component_must_agree() {
        let shared = history([(0, 7), (1, 7)]);
        let mut divergent = shared.clone();
        divergent.insert(1, vec![InputFingerprint::from_bytes(8, &[8])]);
        let evidence = build_evidence(
            vec![0, 0, 1],
            vec![shared.clone(), divergent, shared],
            vec![1, 1, 1],
        );
        assert_eq!(
            classify_component_scope(&evidence, 0),
            ComponentOutcome::InternalDivergence {
                component: 0,
                frame: 1,
            }
        );
    }

    #[test]
    fn every_component_continuing_agrees_on_one_history() {
        let mut shared = history([(0, 7), (1, 7), (2, 7)]);
        for frame in 3..=6 {
            shared.insert(frame, vec![InputFingerprint::from_bytes(7, &[7, 7])]);
        }
        let evidence = build_evidence_with_states(
            vec![0, 1],
            vec![shared.clone(), shared],
            vec![2, 2],
            vec![TraceSessionState::Running, TraceSessionState::Running],
        );
        assert_eq!(
            classify_component_scope(&evidence, 0),
            ComponentOutcome::ContinuedTogether { shared_prefix: 6 }
        );
    }

    #[test]
    fn members_that_diverge_after_the_floor_are_caught() {
        let shared = history([(0, 7), (1, 7)]);
        let mut divergent = shared.clone();
        divergent.insert(1, vec![InputFingerprint::from_bytes(8, &[8])]);
        let evidence = build_evidence(
            vec![0, 0, 1],
            vec![shared.clone(), divergent, shared],
            vec![1, 1, 1],
        );
        assert_eq!(
            classify_component_scope(&evidence, 0),
            ComponentOutcome::InternalDivergence {
                component: 0,
                frame: 1,
            }
        );
    }

    #[test]
    fn post_floor_tails_across_components_are_compared() {
        // The heal shape: one member of each component resumed past its
        // stopped sibling and the two tails disagree at frame 2.
        let stopped = history([(0, 7), (1, 7)]);
        let mut left_tail = stopped.clone();
        left_tail.insert(2, vec![InputFingerprint::from_bytes(8, &[8])]);
        let mut right_tail = stopped.clone();
        right_tail.insert(2, vec![InputFingerprint::from_bytes(9, &[9])]);
        let evidence = build_evidence(
            vec![0, 0, 1, 1],
            vec![stopped.clone(), left_tail, stopped, right_tail],
            vec![1, 1, 1, 1],
        );
        assert_eq!(
            classify_component_scope(&evidence, 0),
            ComponentOutcome::Forked {
                frame: 2,
                component: 0,
                other_component: 1,
            }
        );
    }

    #[test]
    fn one_sided_continuation_is_a_split_match() {
        let shared = history([(0, 7), (1, 7), (2, 7)]);
        let mut continued = shared.clone();
        for frame in 3..=6 {
            continued.insert(frame, vec![InputFingerprint::from_bytes(7, &[7, 7])]);
        }
        let evidence = build_evidence_with_states(
            vec![0, 1],
            vec![shared, continued],
            vec![2, 2],
            vec![TraceSessionState::Synchronizing, TraceSessionState::Running],
        );
        assert_eq!(
            classify_component_scope(&evidence, 0),
            ComponentOutcome::SplitMatch {
                shared_prefix: 2,
                continued: vec![1],
            }
        );
    }

    #[test]
    fn missing_evidence_is_invalid_and_never_a_pass() {
        let shared = history([(0, 7)]);
        let evidence = build_evidence(vec![0, 0], vec![shared.clone(), shared.clone()], vec![]);
        assert!(matches!(
            classify_component_scope(&evidence, 0),
            ComponentOutcome::Invalid { .. }
        ));
        let evidence = build_evidence(vec![0, 0, 1], vec![shared.clone(), shared], vec![0, 0, 0]);
        assert!(matches!(
            classify_component_scope(&evidence, 0),
            ComponentOutcome::Invalid { .. }
        ));
        assert!(ComponentScopeOptions {
            component: vec![0, 0, 0],
        }
        .validate(3)
        .is_err());
    }

    fn split_tail_evidence(
        terminal_states: [TraceSessionState; 2],
        tail_frames: i32,
    ) -> ComponentScopeEvidence {
        let shared = history([(0, 7), (1, 7), (2, 7), (3, 7)]);
        let mut continued = shared.clone();
        let mut terminal = shared;
        for frame in 4..=(3 + tail_frames) {
            continued.insert(frame, vec![InputFingerprint::from_bytes(7, &[7, 8])]);
            terminal.insert(frame, vec![InputFingerprint::from_bytes(9, &[9, 8])]);
        }
        build_evidence_with_states(
            vec![0, 1],
            vec![continued, terminal],
            vec![3, 3],
            terminal_states.to_vec(),
        )
    }

    #[test]
    fn bounded_failed_closed_tail_after_the_cut_is_an_explicit_split() {
        let evidence = split_tail_evidence(
            [TraceSessionState::Running, TraceSessionState::Synchronizing],
            2,
        );
        assert_eq!(
            classify_component_scope_with_tail(&evidence, 0, 2),
            ComponentOutcome::SplitTail {
                shared_prefix: 3,
                continued: vec![0],
                tail: 2,
            }
        );
        assert!(classify_component_scope_with_tail(&evidence, 0, 2).is_legal());
    }

    #[test]
    fn tail_beyond_the_allowance_is_still_a_fork() {
        let evidence = split_tail_evidence(
            [TraceSessionState::Running, TraceSessionState::Synchronizing],
            3,
        );
        assert_eq!(
            classify_component_scope_with_tail(&evidence, 0, 2),
            ComponentOutcome::Forked {
                frame: 4,
                component: 0,
                other_component: 1,
            }
        );
    }

    #[test]
    fn a_diverging_tail_requires_the_failed_closed_state() {
        let evidence =
            split_tail_evidence([TraceSessionState::Running, TraceSessionState::Running], 2);
        assert_eq!(
            classify_component_scope_with_tail(&evidence, 0, 2),
            ComponentOutcome::Forked {
                frame: 4,
                component: 0,
                other_component: 1,
            }
        );
    }

    #[test]
    fn a_divergence_inside_the_cut_window_is_a_fork() {
        let shared = history([(0, 7), (1, 7), (2, 7)]);
        let mut divergent = shared.clone();
        divergent.insert(2, vec![InputFingerprint::from_bytes(9, &[9])]);
        let evidence = build_evidence(vec![0, 1], vec![shared, divergent], vec![2, 2]);
        assert_eq!(
            classify_component_scope_with_tail(&evidence, 0, 8),
            ComponentOutcome::Forked {
                frame: 2,
                component: 0,
                other_component: 1,
            }
        );
    }
}
