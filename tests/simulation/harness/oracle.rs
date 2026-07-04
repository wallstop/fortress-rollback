//! Global invariant oracle for the whole-mesh simulation.
//!
//! The oracle observes every peer continuously during a run and issues a
//! final verdict. Its invariants are *global* — properties of the whole mesh
//! that no per-session assertion can express:
//!
//! - **(a) Confirmed-prefix agreement**: every peer's confirmed input stream
//!   is byte-identical to the first-observed (canonical) stream, frame by
//!   frame. Sampled incrementally because confirmed inputs evict from the
//!   session's history window.
//! - **(b) State agreement**: after the run, every peer's recorded
//!   post-advance state for each *globally confirmed* frame is identical.
//!   Speculative (not-yet-confirmed) frames legitimately differ mid-run and
//!   are never compared.
//! - **(b-cross) In-band cross-check**: `DesyncDetected` events must be
//!   consistent with (b): an event while recorded states agree exposes a
//!   false-positive detector; states diverging without an event exposes a
//!   silent desync. Either way the run fails with the full picture recorded.
//! - **(g) Session-error allowlist**: `advance_frame` while `Running` must
//!   not error (the prediction throttle is `Ok` with no requests, not an
//!   error). Any error fails the run with the step and peer recorded.
//! - **Violations**: telemetry violations at `Error`+ severity fail the run
//!   (Critical is never acceptable; the Error allowlist arrives with the
//!   lifecycle vocabulary in a later milestone, seeded by a fleet census).
//! - **(c-lite) End progress**: after heal + drain, every peer must be
//!   `Running` and have confirmed at least [`MIN_END_CONFIRMED`] frames — a
//!   coarse whole-mesh liveness check (the full bounded-liveness invariant
//!   arrives with the lifecycle vocabulary).

// Test infrastructure: not every test binary uses every helper.
#![allow(dead_code)]

use crate::common::stubs::{StateStub, StubInput};
use fortress_rollback::telemetry::{SpecViolation, ViolationSeverity};
use fortress_rollback::{Frame, SessionState};
use std::collections::BTreeMap;

/// Minimum confirmed frames every peer must reach by end of run (c-lite).
///
/// Deliberately conservative: the drain window alone is ≈250 steps of clean
/// network; a healthy mesh confirms hundreds of frames there.
pub const MIN_END_CONFIRMED: i32 = 30;

/// One concrete invariant violation, with enough context to debug.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OracleFailure {
    /// (a): a peer's confirmed inputs for `frame` differ from the canonical
    /// stream first observed from `first_author`.
    ConfirmedInputDivergence {
        frame: i32,
        peer: usize,
        first_author: usize,
        expected: Vec<u32>,
        actual: Vec<u32>,
    },
    /// (b): a peer's recorded post-advance state for a globally confirmed
    /// frame differs from the canonical state.
    StateDivergence {
        frame: i32,
        peer: usize,
        first_author: usize,
        expected: StateStub,
        actual: StateStub,
    },
    /// (b-cross): the in-band desync detector fired.
    InbandDesyncDetected { peer: usize, frame: i32 },
    /// (a)-sampling: confirmed inputs for a frame the session reported as
    /// confirmed could not be fetched (eviction outran sampling, or a bug).
    ConfirmedInputUnavailable {
        peer: usize,
        frame: i32,
        error: String,
    },
    /// (g): `advance_frame` returned an error while `Running`.
    SessionError {
        peer: usize,
        step: u32,
        error: String,
    },
    /// A telemetry violation at `Error`+ severity was reported.
    Violation { peer: usize, violation: String },
    /// (c-lite): a peer failed the end-of-run progress bar.
    EndProgress {
        peer: usize,
        state: SessionState,
        confirmed: i32,
        required: i32,
    },
}

/// Final verdict of a run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verdict {
    pub failures: Vec<OracleFailure>,
}

impl Verdict {
    #[must_use]
    pub fn passed(&self) -> bool {
        self.failures.is_empty()
    }
}

/// The oracle instance for one run.
pub struct Oracle {
    n_players: usize,
    /// frame → (first author, canonical per-slot inputs).
    canonical_inputs: BTreeMap<i32, (usize, Vec<u32>)>,
    failures: Vec<OracleFailure>,
    /// Cap so a systemically broken run reports a readable prefix instead of
    /// millions of identical failures.
    failure_cap: usize,
    /// Per-variant cap. Without it, a flood of one failure class evicts
    /// rarer classes entirely: the N=16 oracle-integrity control found the
    /// per-peer `InbandDesyncDetected` stream filling all 64 global slots
    /// before the end-of-run state comparison ran, so `StateDivergence`
    /// vanished from the report (PLAN.md Part V — the "silent cap"
    /// anti-pattern inside the oracle itself). Every class is now
    /// guaranteed representation.
    per_class_cap: usize,
}

impl Oracle {
    #[must_use]
    pub fn new(n_players: usize) -> Self {
        Self {
            n_players,
            canonical_inputs: BTreeMap::new(),
            failures: Vec::new(),
            failure_cap: 64,
            per_class_cap: 8,
        }
    }

    fn push_failure(&mut self, failure: OracleFailure) {
        let same_class = self
            .failures
            .iter()
            .filter(|f| std::mem::discriminant(*f) == std::mem::discriminant(&failure))
            .count();
        if same_class >= self.per_class_cap {
            return;
        }
        if self.failures.len() < self.failure_cap {
            self.failures.push(failure);
        }
    }

    /// (a): feed one peer's confirmed inputs for one frame, in ascending
    /// frame order per peer.
    pub fn observe_confirmed_inputs(&mut self, peer: usize, frame: i32, inputs: &[StubInput]) {
        let values: Vec<u32> = inputs.iter().map(|i| i.inp).collect();
        match self.canonical_inputs.get(&frame) {
            None => {
                self.canonical_inputs.insert(frame, (peer, values));
            },
            Some((first_author, canonical)) => {
                if *canonical != values {
                    let failure = OracleFailure::ConfirmedInputDivergence {
                        frame,
                        peer,
                        first_author: *first_author,
                        expected: canonical.clone(),
                        actual: values,
                    };
                    self.push_failure(failure);
                }
            },
        }
    }

    /// (a)-sampling: the session claimed `frame` confirmed but the inputs
    /// could not be fetched.
    pub fn observe_confirmed_unavailable(&mut self, peer: usize, frame: i32, error: &str) {
        self.push_failure(OracleFailure::ConfirmedInputUnavailable {
            peer,
            frame,
            error: error.to_owned(),
        });
    }

    /// (b-cross): a `DesyncDetected` event surfaced on `peer`.
    pub fn observe_desync_event(&mut self, peer: usize, frame: Frame) {
        self.push_failure(OracleFailure::InbandDesyncDetected {
            peer,
            frame: frame.as_i32(),
        });
    }

    /// (g): `advance_frame` errored while `Running`.
    pub fn observe_advance_error(
        &mut self,
        peer: usize,
        step: u32,
        error: &fortress_rollback::FortressError,
    ) {
        self.push_failure(OracleFailure::SessionError {
            peer,
            step,
            error: format!("{error:?}"),
        });
    }

    /// Telemetry violations collected for one peer over the whole run.
    pub fn observe_violations(&mut self, peer: usize, violations: &[SpecViolation]) {
        for violation in violations {
            if violation.severity >= ViolationSeverity::Error {
                self.push_failure(OracleFailure::Violation {
                    peer,
                    violation: format!(
                        "[{:?}/{:?}] {}",
                        violation.severity, violation.kind, violation.message
                    ),
                });
            }
        }
    }

    /// (b) + (c-lite): end-of-run checks. `recorded[i]` is peer `i`'s
    /// post-advance state map; `end_confirmed[i]` its final confirmed frame;
    /// `end_state[i]` its final session state.
    pub fn finalize(
        mut self,
        recorded: &[BTreeMap<i32, StateStub>],
        end_confirmed: &[Frame],
        end_state: &[SessionState],
    ) -> Verdict {
        assert_eq!(recorded.len(), self.n_players);
        assert_eq!(end_confirmed.len(), self.n_players);
        assert_eq!(end_state.len(), self.n_players);

        // (c-lite) end progress per peer.
        for peer in 0..self.n_players {
            let confirmed = end_confirmed
                .get(peer)
                .copied()
                .unwrap_or(Frame::NULL)
                .as_i32();
            let state = end_state
                .get(peer)
                .copied()
                .unwrap_or(SessionState::Synchronizing);
            if state != SessionState::Running || confirmed < MIN_END_CONFIRMED {
                self.push_failure(OracleFailure::EndProgress {
                    peer,
                    state,
                    confirmed,
                    required: MIN_END_CONFIRMED,
                });
            }
        }

        // (b) state agreement over the globally confirmed prefix.
        let global_confirmed = end_confirmed
            .iter()
            .map(|frame| frame.as_i32())
            .min()
            .unwrap_or(-1);
        // Recorded states are keyed by post-advance frame (frame N+1 holds
        // the result of simulating frame N with confirmed inputs ≤ N), so a
        // frame's state is final once the *previous* frame is confirmed;
        // comparing up to `global_confirmed` stays strictly inside the final
        // region.
        let mut canonical_states: BTreeMap<i32, (usize, StateStub)> = BTreeMap::new();
        for (peer, states) in recorded.iter().enumerate() {
            for (&frame, &state) in states.range(..=global_confirmed) {
                match canonical_states.get(&frame) {
                    None => {
                        canonical_states.insert(frame, (peer, state));
                    },
                    Some(&(first_author, canonical)) => {
                        if canonical != state {
                            self.push_failure(OracleFailure::StateDivergence {
                                frame,
                                peer,
                                first_author,
                                expected: canonical,
                                actual: state,
                            });
                        }
                    },
                }
            }
        }

        Verdict {
            failures: self.failures,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn inputs(values: &[u32]) -> Vec<StubInput> {
        values.iter().map(|&inp| StubInput { inp }).collect()
    }

    /// Negative control: the input-divergence invariant must fire on a
    /// seeded divergence and stay silent on agreement.
    #[test]
    fn oracle_detects_confirmed_input_divergence() {
        let mut oracle = Oracle::new(2);
        oracle.observe_confirmed_inputs(0, 5, &inputs(&[1, 2]));
        oracle.observe_confirmed_inputs(1, 5, &inputs(&[1, 2]));
        oracle.observe_confirmed_inputs(0, 6, &inputs(&[3, 4]));
        oracle.observe_confirmed_inputs(1, 6, &inputs(&[3, 9]));

        let verdict = oracle.finalize(
            &[BTreeMap::new(), BTreeMap::new()],
            &[Frame::new(60), Frame::new(60)],
            &[SessionState::Running, SessionState::Running],
        );
        assert!(!verdict.passed());
        assert!(verdict.failures.iter().any(|f| matches!(
            f,
            OracleFailure::ConfirmedInputDivergence {
                frame: 6,
                peer: 1,
                ..
            }
        )));
        assert!(
            !verdict
                .failures
                .iter()
                .any(|f| matches!(f, OracleFailure::ConfirmedInputDivergence { frame: 5, .. })),
            "agreeing frames must not fail"
        );
    }

    /// Negative control: the state-agreement invariant must fire on a seeded
    /// divergence within the confirmed prefix, and ignore speculative frames.
    #[test]
    fn oracle_detects_state_divergence_only_in_confirmed_prefix() {
        let oracle = Oracle::new(2);
        let mut a = BTreeMap::new();
        let mut b = BTreeMap::new();
        // Agreement at frame 10, divergence at 11 (inside confirmed prefix),
        // divergence at 50 (speculative — must be ignored).
        a.insert(
            10,
            StateStub {
                frame: 10,
                state: 4,
            },
        );
        b.insert(
            10,
            StateStub {
                frame: 10,
                state: 4,
            },
        );
        a.insert(
            11,
            StateStub {
                frame: 11,
                state: 6,
            },
        );
        b.insert(
            11,
            StateStub {
                frame: 11,
                state: 5,
            },
        );
        a.insert(
            50,
            StateStub {
                frame: 50,
                state: 1,
            },
        );
        b.insert(
            50,
            StateStub {
                frame: 50,
                state: 2,
            },
        );

        let verdict = oracle.finalize(
            &[a, b],
            &[Frame::new(20), Frame::new(30)],
            &[SessionState::Running, SessionState::Running],
        );
        let state_failures: Vec<_> = verdict
            .failures
            .iter()
            .filter(|f| matches!(f, OracleFailure::StateDivergence { .. }))
            .collect();
        assert_eq!(state_failures.len(), 1, "exactly the frame-11 divergence");
        assert!(matches!(
            state_failures[0],
            OracleFailure::StateDivergence { frame: 11, .. }
        ));
    }

    /// Negative control: end-progress fires for a stuck peer.
    #[test]
    fn oracle_detects_end_stall() {
        let oracle = Oracle::new(2);
        let verdict = oracle.finalize(
            &[BTreeMap::new(), BTreeMap::new()],
            &[Frame::new(500), Frame::new(3)],
            &[SessionState::Running, SessionState::Running],
        );
        assert!(verdict.failures.iter().any(|f| matches!(
            f,
            OracleFailure::EndProgress {
                peer: 1,
                confirmed: 3,
                ..
            }
        )));
    }

    /// Negative control: a desync event and an Error-severity violation each
    /// fail the run.
    #[test]
    fn oracle_records_desync_events_and_violations() {
        let mut oracle = Oracle::new(2);
        oracle.observe_desync_event(0, Frame::new(42));
        let verdict = oracle.finalize(
            &[BTreeMap::new(), BTreeMap::new()],
            &[Frame::new(500), Frame::new(500)],
            &[SessionState::Running, SessionState::Running],
        );
        assert!(verdict.failures.iter().any(|f| matches!(
            f,
            OracleFailure::InbandDesyncDetected { peer: 0, frame: 42 }
        )));
    }

    /// The failure cap keeps a systemically broken run readable.
    #[test]
    fn oracle_caps_recorded_failures() {
        let mut oracle = Oracle::new(2);
        oracle.observe_confirmed_inputs(0, 0, &inputs(&[1]));
        for frame in 0..1000 {
            oracle.observe_confirmed_inputs(0, frame, &inputs(&[1]));
            oracle.observe_confirmed_inputs(1, frame, &inputs(&[2]));
        }
        let verdict = oracle.finalize(
            &[BTreeMap::new(), BTreeMap::new()],
            &[Frame::new(500), Frame::new(500)],
            &[SessionState::Running, SessionState::Running],
        );
        assert!(verdict.failures.len() <= 64);
    }
}
