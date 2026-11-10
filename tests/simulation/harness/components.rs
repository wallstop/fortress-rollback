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
            Self::FailedClosed { .. } | Self::ContinuedTogether { .. } | Self::SplitMatch { .. }
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
#[must_use]
pub fn classify_component_scope(
    evidence: &ComponentScopeEvidence,
    allowance: i32,
) -> ComponentOutcome {
    let n = evidence.component.len();
    if n < 2 || evidence.confirmed_inputs.len() != n || evidence.cut_confirmed.len() != n {
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

    // Cross-component agreement over every frame any pair of peers in
    // different components both confirmed. Comparing only the common floor
    // would miss the post-heal shape where one member of each component
    // resumed past a stopped peer and their continued tails disagree.
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
                            return ComponentOutcome::Forked {
                                frame,
                                component: component_ids[i],
                                other_component: component_ids[j],
                            };
                        }
                    }
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
    let continued: Vec<usize> = component_ids
        .iter()
        .zip(&floors)
        .filter(|(_, floor)| **floor > cut)
        .map(|(component, _)| *component)
        .collect();
    if continued.is_empty() {
        ComponentOutcome::FailedClosed {
            shared_prefix: shared,
        }
    } else if continued.len() == component_ids.len() {
        ComponentOutcome::ContinuedTogether {
            shared_prefix: shared,
        }
    } else {
        ComponentOutcome::SplitMatch {
            shared_prefix: shared,
            continued,
        }
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
        let final_confirmed = histories
            .iter()
            .map(|history| history.keys().next_back().copied().unwrap_or(-1))
            .collect();
        let final_states = vec![TraceSessionState::Synchronizing; histories.len()];
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
        let evidence = build_evidence(vec![0, 1], vec![shared.clone(), shared], vec![2, 2]);
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
        let evidence = build_evidence(vec![0, 1], vec![shared, continued], vec![2, 2]);
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
}
