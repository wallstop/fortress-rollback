//! Global invariant oracle for the whole-mesh simulation.
//!
//! The oracle observes every peer continuously during a run and issues a
//! final verdict. Its invariants are *global* — properties of the whole mesh
//! that no per-session assertion can express:
//!
//! - **(a) Confirmed-prefix agreement**: every peer's confirmed input stream
//!   has the same serialized-input fingerprint as the first-observed
//!   (canonical) stream, frame by frame. Sampled incrementally because
//!   confirmed inputs evict from the session's history window.
//! - **(b) State agreement**: after the run, every peer's recorded
//!   post-advance state for each *globally confirmed* frame is identical.
//!   Speculative (not-yet-confirmed) frames legitimately differ mid-run and
//!   are never compared.
//! - **(b-cross) In-band cross-check**: `DesyncDetected` events must be
//!   consistent with (b): an event while recorded states agree exposes a
//!   false-positive detector; states diverging without an event exposes a
//!   silent desync. Checksum-mismatch metrics are also consumed directly so a
//!   starved event queue cannot hide a detector finding. Either way the run
//!   fails with the full picture recorded.
//! - **(e) Freeze-frame convergence**: for every dropped slot, every live
//!   survivor must agree on the stable frame/value where that slot begins
//!   presenting [`InputStatus::Disconnected`].
//! - **(g) Session-error allowlist**: session APIs the harness expects to
//!   succeed must not error. Any error fails the run with the operation, step,
//!   and peer recorded.
//! - **Violations**: telemetry violations at `Error`+ severity fail the run
//!   (Critical is never acceptable; the Error allowlist arrives with the
//!   lifecycle vocabulary in a later milestone, seeded by a fleet census).
//! - **(c-lite) End progress**: after heal + drain, every peer must be
//!   `Running` and have confirmed at least [`MIN_END_CONFIRMED`] frames — a
//!   coarse whole-mesh liveness check (the full bounded-liveness invariant
//!   arrives with the lifecycle vocabulary).

// Test infrastructure: not every test binary uses every helper.
#![allow(dead_code)]

use crate::common::stubs::StateStub;
use fortress_rollback::hash::DeterministicHasher;
use fortress_rollback::telemetry::{SpecViolation, ViolationKind, ViolationSeverity};
use fortress_rollback::{Frame, InputStatus, PlayerHandle, SessionState};
use std::collections::BTreeMap;
use std::hash::Hasher;

/// Minimum confirmed frames every peer must reach by end of run (c-lite).
///
/// Deliberately conservative: the drain window alone is ≈250 steps of clean
/// network; a healthy mesh confirms hundreds of frames there.
pub const MIN_END_CONFIRMED: i32 = 30;

/// Minimum confirmed-frame advance (G) every live peer must make within the
/// bounded recovery window B after the last `HealAll` — the (c) liveness floor.
///
/// A floor, not a rate target: the harness advances one frame per step while
/// `Running`, so a healthy peer confirms ≈B frames (hundreds) over the window,
/// clearing this by ~25×. It is set `> max_prediction` (default 8) so a peer
/// that merely re-fills its prediction window once without truly resuming
/// confirmation does not clear it, and `< MIN_END_CONFIRMED` (30) so (c) stays a
/// strictly per-window bound complementary to (c-lite)'s absolute end bar.
pub const POST_HEAL_MIN_ADVANCE: i32 = 10;

/// Source that emitted telemetry violations consumed by the oracle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViolationSource {
    Peer(usize),
    Spectator,
}

/// One concrete invariant violation, with enough context to debug.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OracleFailure {
    /// (a): a peer's confirmed inputs for `frame` differ from the canonical
    /// stream first observed from `first_author`.
    ConfirmedInputDivergence {
        frame: i32,
        peer: usize,
        first_author: usize,
        expected: Vec<InputFingerprint>,
        actual: Vec<InputFingerprint>,
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
    /// (b-cross): the session recorded checksum mismatches in metrics. This is
    /// the event-queue-independent detector signal, so a peer whose app never
    /// drains `DesyncDetected` events still fails the run.
    ChecksumMismatchMetric { peer: usize, mismatches: u64 },
    /// (a)-sampling: confirmed inputs for a frame the session reported as
    /// confirmed could not be fetched (eviction outran sampling, or a bug).
    ConfirmedInputUnavailable {
        peer: usize,
        frame: i32,
        error: String,
    },
    /// (g): a session API returned an error while the harness expected it to
    /// succeed.
    SessionError {
        operation: &'static str,
        peer: usize,
        step: u32,
        error: String,
    },
    /// A telemetry violation at `Error`+ severity was reported.
    Violation {
        source: ViolationSource,
        violation: String,
    },
    /// The runner reported violations for a source that cannot exist in this
    /// mesh. This fails independently from the violation payload so spectator
    /// telemetry can never be smuggled through an out-of-range peer sentinel.
    InvalidViolationSource {
        source: ViolationSource,
        n_players: usize,
    },
    /// (c-lite): a peer failed the end-of-run progress bar.
    EndProgress {
        peer: usize,
        state: SessionState,
        confirmed: i32,
        required: i32,
    },
    /// Every peer was killed: there is nothing left to verify, so the run is a
    /// vacuous pass unless flagged. A schedule that crashes the whole mesh
    /// proves no correctness property and must not report success.
    NoLivePeers { n_players: usize },
    /// (c): a **live** peer failed the bounded post-heal liveness bar — after
    /// the last `HealAll`, its `confirmed_frame` did not advance by at least
    /// `required` (G) frames within the observed `window_steps` span (B, or B-1
    /// at an exact-boundary drain). Catches a peer pinned post-heal (or a mutual
    /// deadlock) that the coarse end-progress bar (c-lite) can miss — a peer may
    /// recover late and still clear the absolute end bar. Killed peers are
    /// excluded (dead, not pinned); the check is inert when the schedule never
    /// heals and skipped when the post-heal drain is too short for both anchors
    /// to be observable. `advanced` is `after - at_heal` and may read negative
    /// under the documented transient `confirmed_frame` dip — recorded raw so a
    /// dip is visible in the repro rather than hidden.
    PostHealLiveness {
        peer: usize,
        at_heal: i32,
        after: i32,
        advanced: i32,
        required: i32,
        window_steps: u32,
    },
    /// (e): live survivors disagree on the stable frame/value where a dropped
    /// slot became `Disconnected`.
    FreezeFrameDivergence {
        slot: usize,
        peer: usize,
        first_author: usize,
        expected: Option<FreezePoint>,
        actual: Option<FreezePoint>,
    },
    /// (e): a retired slot had live `Running` survivors, but none of them ever
    /// presented a stable `Disconnected` run for that slot.
    FreezeFrameMissing {
        slot: usize,
        live_running_peers: Vec<usize>,
    },
    /// (d): a redundant spectator reported an internal host disagreement.
    SpectatorDivergenceEvent { frame: i32, player: PlayerHandle },
    /// (d): a spectator API returned an error while the harness expected it to
    /// keep following at least one live host.
    SpectatorSessionError {
        operation: &'static str,
        step: u32,
        error: String,
    },
    /// (d): a configured spectator never displayed any frame by the end of a
    /// run with live `Running` mesh peers, or did not display through the
    /// runner-supplied frame floor needed by the scenario.
    SpectatorProgressMissing {
        live_running_peers: Vec<usize>,
        required_min_frame: Option<i32>,
        observed_max_frame: Option<i32>,
    },
    /// (d): the spectator displayed a frame inside the live mesh's confirmed
    /// prefix, but no live mesh peer had an applied-input record for it.
    SpectatorMeshCanonMissing {
        frame: i32,
        live_running_peers: Vec<usize>,
    },
    /// (d): the spectator's displayed input bytes or dropped-slot status do
    /// not match the mesh canon for the same confirmed frame.
    SpectatorInputDivergence {
        frame: i32,
        peer: usize,
        expected: Vec<(InputFingerprint, InputStatus)>,
        actual: Vec<(InputFingerprint, InputStatus)>,
    },
}

/// A survivor's stable dropped-slot freeze observation.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FreezePoint {
    pub frame: i32,
    pub input: InputFingerprint,
}

/// One reviewed telemetry violation pattern that the simulation oracle may
/// tolerate while still counting hits.
///
/// Matching is intentionally narrow: severity, kind, and source location must
/// match exactly, while the message uses a prefix so numeric/run-specific
/// suffixes do not force one entry per value. [`ViolationSeverity::Critical`] is
/// never allowlistable even if an entry matches.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ViolationAllowlistEntry {
    pub severity: ViolationSeverity,
    pub kind: ViolationKind,
    pub location: &'static str,
    pub message_prefix: &'static str,
    pub rationale: &'static str,
}

impl ViolationAllowlistEntry {
    #[must_use]
    pub fn matches(self, violation: &SpecViolation) -> bool {
        self.severity == violation.severity
            && self.kind == violation.kind
            && self.location == violation.location
            && violation.message.starts_with(self.message_prefix)
    }

    #[must_use]
    pub fn signature(self) -> ViolationSignature {
        ViolationSignature {
            severity: self.severity,
            kind: self.kind,
            location: self.location.to_owned(),
            message_prefix: self.message_prefix.to_owned(),
        }
    }
}

/// Stable telemetry signature used by both allowlist matching and census rows.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ViolationSignature {
    pub severity: ViolationSeverity,
    pub kind: ViolationKind,
    pub location: String,
    pub message_prefix: String,
}

impl ViolationSignature {
    #[must_use]
    pub fn from_violation(
        violation: &SpecViolation,
        allowlist: &[ViolationAllowlistEntry],
    ) -> Self {
        allowlist
            .iter()
            .find(|entry| entry.matches(violation))
            .map_or_else(
                || Self {
                    severity: violation.severity,
                    kind: violation.kind,
                    location: violation.location.to_owned(),
                    message_prefix: census_message_prefix(&violation.message),
                },
                |entry| entry.signature(),
            )
    }
}

/// Hit count for one allowlisted violation pattern.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViolationAllowlistHit {
    pub entry: ViolationAllowlistEntry,
    pub count: u64,
}

/// Reviewed default allowlist for the simulation oracle.
///
/// Empty after the current census: no `Error`+ telemetry violation is expected
/// in passing non-injection fleet runs. Keeping this explicit empty list makes
/// additions deliberate and reviewable.
pub const DEFAULT_VIOLATION_ALLOWLIST: &[ViolationAllowlistEntry] = &[];

/// Validate allowlist entries for reviewability.
///
/// The runner calls this for the reviewed default allowlist before each run, and
/// unit tests exercise the failure modes directly. Test-only injected allowlists
/// may call it too when they intend to model production-reviewed entries.
pub fn validate_violation_allowlist(entries: &[ViolationAllowlistEntry]) -> Result<(), String> {
    for (index, entry) in entries.iter().enumerate() {
        if entry.severity == ViolationSeverity::Critical {
            return Err(format!(
                "entry {index}: Critical violations are never allowlistable"
            ));
        }
        if entry.message_prefix.is_empty() {
            return Err(format!("entry {index}: message_prefix must be non-empty"));
        }
        if entry.rationale.trim().is_empty() {
            return Err(format!("entry {index}: rationale must be non-empty"));
        }
        if !has_file_line(entry.location) {
            return Err(format!(
                "entry {index}: location must include a parseable file:line, got {:?}",
                entry.location
            ));
        }
    }

    for (left_index, left) in entries.iter().enumerate() {
        for (right_index, right) in entries.iter().enumerate().skip(left_index + 1) {
            let same_source = left.severity == right.severity
                && left.kind == right.kind
                && left.location == right.location;
            let overlapping_prefix = left.message_prefix.starts_with(right.message_prefix)
                || right.message_prefix.starts_with(left.message_prefix);
            if same_source && overlapping_prefix {
                return Err(format!(
                    "entries {left_index} and {right_index}: overlapping message_prefix values"
                ));
            }
        }
    }

    Ok(())
}

fn has_file_line(location: &str) -> bool {
    let Some((file, line)) = location.rsplit_once(':') else {
        return false;
    };
    !file.is_empty() && line.parse::<u32>().is_ok()
}

fn census_message_prefix(message: &str) -> String {
    let first_numeric = message
        .char_indices()
        .find_map(|(index, ch)| ch.is_ascii_digit().then_some(index));
    let prefix = first_numeric
        .filter(|index| *index > 0)
        .and_then(|index| message.get(..index))
        .unwrap_or(message)
        .trim_end_matches(|ch: char| ch.is_whitespace() || matches!(ch, ':' | '=' | '#' | '-'))
        .trim();
    if prefix.is_empty() {
        message.to_owned()
    } else {
        prefix.to_owned()
    }
}

/// Stable fingerprint of one serialized input value.
///
/// The harness's game transition uses only a logical `u32` lane, but the oracle
/// also hashes all serialized bytes and stores their length, so the 32-byte
/// sweep axis cannot hide ordinary divergence in padding bytes. This is a
/// fingerprint, not a collision-proof byte vector.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct InputFingerprint {
    pub logical: u32,
    pub len: u32,
    pub hash: u64,
}

impl InputFingerprint {
    #[must_use]
    pub fn from_bytes(logical: u32, bytes: &[u8]) -> Self {
        let mut hasher = DeterministicHasher::new();
        hasher.write(bytes);
        Self {
            logical,
            len: u32::try_from(bytes.len()).unwrap_or(u32::MAX),
            hash: hasher.finish(),
        }
    }
}

/// Inputs for the (c) bounded post-heal liveness check, assembled by the runner
/// from the heal-anchored confirmed-frame snapshots. The [`Default`] is inert
/// (`ran = false`), so an oracle that is never handed a `HealLiveness` — or a
/// schedule that never heals — simply skips (c).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HealLiveness {
    /// The (c) check should run: an actual `HealAll` event fired AND enough
    /// post-heal drain remains for both anchors to be observable (the recovery
    /// anchor `heal + B` is at most the run's end). The runner owns this
    /// decision (heal detection + drain length); `false` ⇒ (c) is inert (no
    /// heal) or indeterminate (window too short), reported as
    /// `recovered_within_b = None`.
    pub ran: bool,
    /// Actual steps spanned by the two anchors — the span the advance is
    /// measured over, and the source of truth for the window (not a nominal B).
    /// Equals the derived B, or B-1 at the exact-boundary drain where the
    /// recovery anchor `heal + B` lands on the run's end and clamps to the last
    /// recorded step. Reported in `PostHealLiveness` so a failure states the
    /// real window.
    pub window_steps: u32,
    /// Minimum confirmed-frame advance required within the window (G).
    pub required_advance: i32,
    /// Confirmed frame per peer at the heal anchor (indexed by peer; empty if
    /// `!ran`).
    pub confirmed_at_heal: Vec<i32>,
    /// Confirmed frame per peer at the recovery anchor (indexed by peer; empty
    /// if `!ran`).
    pub confirmed_after: Vec<i32>,
}

/// Final verdict of a run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verdict {
    pub failures: Vec<OracleFailure>,
    /// (i) metastability: `Some(true)` iff the schedule healed, both post-heal
    /// anchors were observable, and every live peer advanced ≥ G within the
    /// window (the (c) check ran and passed); `Some(false)` when it ran and at
    /// least one live peer was pinned; `None` when (c) was inert (no heal), the
    /// window was too short to observe, or every peer was killed (no live peer
    /// to report recovery for — that is caught separately by `NoLivePeers`, not
    /// reported here as "recovered"). The explicit "recovered within B steps of
    /// heal: yes/no" signal.
    pub recovered_within_b: Option<bool>,
    /// Counts for reviewed telemetry violation patterns the oracle tolerated.
    /// Empty when no allowlisted pattern was observed. Critical violations never
    /// appear here because they are not allowlistable.
    pub violation_allowlist_hits: Vec<ViolationAllowlistHit>,
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
    canonical_inputs: BTreeMap<i32, (usize, Vec<InputFingerprint>)>,
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
    /// Peers that were retired mid-run (`PeerKill`, `GracefulRemove`, or
    /// `LegacyDisconnect`). A retired peer is excluded from the *liveness*
    /// checks only: it cannot satisfy the `Running`/end-progress bar (c-lite),
    /// and its frozen confirmed frame must not drag the globally-confirmed
    /// prefix below where the survivors agree.
    /// Its **pre-retirement** observations still count — recorded states it
    /// produced before leaving are compared in (b), and its confirmed-input
    /// samples stand in (a) — so a peer that diverged before it left cannot
    /// escape detection by being retired.
    dead: Vec<bool>,
    /// (c) bounded post-heal liveness inputs. Inert by default; the runner sets
    /// it via [`Self::set_heal_liveness`] once it has the heal-anchored
    /// confirmed snapshots. Oracle unit tests never set it, so (c) stays inert
    /// for them.
    heal: HealLiveness,
    /// Reviewed telemetry violation patterns accepted by this oracle instance.
    violation_allowlist: &'static [ViolationAllowlistEntry],
    /// Hit counts parallel to [`Self::violation_allowlist`].
    violation_allowlist_hits: Vec<u64>,
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
            dead: vec![false; n_players],
            heal: HealLiveness::default(),
            violation_allowlist: DEFAULT_VIOLATION_ALLOWLIST,
            violation_allowlist_hits: vec![0; DEFAULT_VIOLATION_ALLOWLIST.len()],
        }
    }

    /// Replaces the default telemetry violation allowlist. Test-only negative
    /// controls use this to prove matching and the Critical hard-fail rule; the
    /// harness runner uses the reviewed default.
    pub fn set_violation_allowlist(&mut self, entries: &'static [ViolationAllowlistEntry]) {
        self.violation_allowlist = entries;
        self.violation_allowlist_hits = vec![0; entries.len()];
    }

    /// Hands the oracle the (c) bounded post-heal liveness inputs (heal-anchored
    /// confirmed snapshots + the derived B/G bounds). Called once by the runner
    /// before [`Self::finalize`]; left unset (inert) by the oracle unit tests.
    pub fn set_heal_liveness(&mut self, heal: HealLiveness) {
        self.heal = heal;
    }

    /// Marks `peer` as retired (crashed or gracefully removed): it is excluded
    /// from the liveness checks in [`Self::finalize`]. Idempotent for in-range
    /// peers. An
    /// out-of-range peer is a programming error — the runner validates every
    /// event's peer index up front — so it panics loudly rather than silently
    /// leaving the mask unset.
    pub fn mark_peer_dead(&mut self, peer: usize) {
        assert!(
            peer < self.dead.len(),
            "mark_peer_dead: peer {peer} out of range (dead-mask len {})",
            self.dead.len()
        );
        self.dead[peer] = true;
    }

    /// Starts a new live generation for `peer` without marking it dead. A
    /// hot-join replacement is a different session at the same player slot; the
    /// old session may have been the first author for trailing confirmed-input
    /// samples that the serving host intentionally freezes differently when it
    /// removes the slot. Those trailing old input-frame samples must not become
    /// canonical for the replacement generation, while older settled samples
    /// keep their oracle coverage.
    pub fn begin_replacement_generation(&mut self, peer: usize, input_from_frame: i32) {
        assert!(
            peer < self.dead.len(),
            "begin_replacement_generation: peer {peer} out of range (dead-mask len {})",
            self.dead.len()
        );
        self.canonical_inputs
            .retain(|frame, (first_author, _)| *first_author != peer || *frame < input_from_frame);
    }

    /// Whether `peer` was retired mid-run.
    fn is_dead(&self, peer: usize) -> bool {
        self.dead.get(peer).copied().unwrap_or(false)
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
    pub fn observe_confirmed_inputs(
        &mut self,
        peer: usize,
        frame: i32,
        values: Vec<InputFingerprint>,
    ) {
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

    /// (b-cross): a session recorded checksum mismatches, even if its event
    /// queue was not drained and the corresponding `DesyncDetected` event was
    /// discarded or left buffered.
    pub fn observe_checksum_mismatches(&mut self, peer: usize, mismatches: u64) {
        if mismatches > 0 {
            self.push_failure(OracleFailure::ChecksumMismatchMetric { peer, mismatches });
        }
    }

    /// (g): a session API errored while the harness expected it to succeed.
    pub fn observe_session_error(
        &mut self,
        operation: &'static str,
        peer: usize,
        step: u32,
        error: &fortress_rollback::FortressError,
    ) {
        self.observe_runner_error(operation, peer, step, format!("{error:?}"));
    }

    /// (g): the harness detected an invalid lifecycle transition that is not a
    /// direct library API error.
    pub fn observe_runner_error(
        &mut self,
        operation: &'static str,
        peer: usize,
        step: u32,
        error: impl Into<String>,
    ) {
        self.push_failure(OracleFailure::SessionError {
            operation,
            peer,
            step,
            error: error.into(),
        });
    }

    /// (g): `advance_frame` errored while `Running`.
    pub fn observe_advance_error(
        &mut self,
        peer: usize,
        step: u32,
        error: &fortress_rollback::FortressError,
    ) {
        self.observe_session_error("advance_frame", peer, step, error);
    }

    /// (d): a redundant spectator observed host disagreement.
    pub fn observe_spectator_divergence_event(&mut self, frame: Frame, player: PlayerHandle) {
        self.push_failure(OracleFailure::SpectatorDivergenceEvent {
            frame: frame.as_i32(),
            player,
        });
    }

    /// (d): a spectator API errored while the harness expected it to continue.
    pub fn observe_spectator_error(
        &mut self,
        operation: &'static str,
        step: u32,
        error: &fortress_rollback::FortressError,
    ) {
        self.push_failure(OracleFailure::SpectatorSessionError {
            operation,
            step,
            error: format!("{error:?}"),
        });
    }

    /// Telemetry violations collected for one source over the whole run.
    pub fn observe_violations(&mut self, source: ViolationSource, violations: &[SpecViolation]) {
        if let ViolationSource::Peer(peer) = source {
            if peer >= self.n_players {
                self.push_failure(OracleFailure::InvalidViolationSource {
                    source,
                    n_players: self.n_players,
                });
                return;
            }
        }

        for violation in violations {
            let allowlist_hit = self
                .violation_allowlist
                .iter()
                .position(|entry| (*entry).matches(violation));
            if violation.severity != ViolationSeverity::Critical {
                if let Some(index) = allowlist_hit {
                    if let Some(count) = self.violation_allowlist_hits.get_mut(index) {
                        *count = count.saturating_add(1);
                    }
                    continue;
                }
            }
            if violation.severity >= ViolationSeverity::Error {
                self.push_failure(violation_failure(source, violation));
            }
        }
    }

    /// (b) + (c-lite): end-of-run checks. `recorded[i]` is peer `i`'s
    /// post-advance state map; `end_confirmed[i]` its final confirmed frame;
    /// `end_state[i]` its final session state.
    pub fn finalize(
        self,
        recorded: &[BTreeMap<i32, StateStub>],
        end_confirmed: &[Frame],
        end_state: &[SessionState],
    ) -> Verdict {
        self.finalize_with_applied_inputs(recorded, &[], end_confirmed, end_state)
    }

    /// Finalize with the per-frame [`InputStatus`] records needed by (e)
    /// freeze-frame convergence. Passing an empty `applied_inputs` slice keeps
    /// the (e) check inert for unit tests that only exercise older invariants.
    pub fn finalize_with_applied_inputs(
        self,
        recorded: &[BTreeMap<i32, StateStub>],
        applied_inputs: &[BTreeMap<i32, Vec<(InputFingerprint, InputStatus)>>],
        end_confirmed: &[Frame],
        end_state: &[SessionState],
    ) -> Verdict {
        self.finalize_with_applied_inputs_and_spectator(
            recorded,
            applied_inputs,
            end_confirmed,
            end_state,
            None,
            None,
        )
    }

    /// Finalize with an optional spectator-applied input/status record for
    /// §6.2(d). The spectator check compares only frames the spectator actually
    /// displayed and that fall inside the live mesh's globally confirmed prefix,
    /// so a lagging but healthy spectator does not create false failures.
    pub fn finalize_with_applied_inputs_and_spectator(
        mut self,
        recorded: &[BTreeMap<i32, StateStub>],
        applied_inputs: &[BTreeMap<i32, Vec<(InputFingerprint, InputStatus)>>],
        end_confirmed: &[Frame],
        end_state: &[SessionState],
        spectator_inputs: Option<&BTreeMap<i32, Vec<(InputFingerprint, InputStatus)>>>,
        spectator_min_frame: Option<i32>,
    ) -> Verdict {
        assert_eq!(recorded.len(), self.n_players);
        assert_eq!(end_confirmed.len(), self.n_players);
        assert_eq!(end_state.len(), self.n_players);
        if !applied_inputs.is_empty() {
            assert_eq!(applied_inputs.len(), self.n_players);
        }

        // (c-lite) end progress per peer. Retired peers are excluded — a peer
        // that left the harness cannot be `Running` and its frozen frame is not
        // its own fault.
        for peer in 0..self.n_players {
            if self.is_dead(peer) {
                continue;
            }
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

        // Guard against a vacuous pass: if every peer was retired, the excluded
        // end-checks below all skip and the run would report success with
        // nothing verified. A whole-mesh crash/removal proves no property.
        if self.n_players > 0 && (0..self.n_players).all(|peer| self.is_dead(peer)) {
            self.push_failure(OracleFailure::NoLivePeers {
                n_players: self.n_players,
            });
        }

        // (b) state agreement over the globally confirmed prefix. The prefix is
        // the minimum over *live* peers only — a retired peer's frozen confirmed
        // frame must not shrink the window the survivors are checked across.
        let global_confirmed = end_confirmed
            .iter()
            .enumerate()
            .filter(|(peer, _)| !self.is_dead(*peer))
            .map(|(_, frame)| frame.as_i32())
            .min()
            .unwrap_or(-1);
        // Recorded states are keyed by post-advance frame (frame N+1 holds
        // the result of simulating frame N with confirmed inputs ≤ N), so a
        // frame's state is final once the *previous* frame is confirmed;
        // comparing up to `global_confirmed` stays strictly inside the final
        // region. Retired peers are NOT skipped here: their *pre-retirement*
        // recorded states are real observations for the frames they produced, so
        // a peer that diverged before leaving is still caught. They only lack
        // states past retirement, so they never contribute past the survivor
        // prefix.
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

        // (e) freeze-frame convergence. Only slots that retired mid-run are
        // checked; only live survivors are compared. The applied-input record is
        // last-write-wins over rollback re-simulations, so each peer's map is the
        // end-of-run truth for frames it actually simulated. Limit each survivor
        // to its own confirmed prefix so speculative disconnected tails never
        // create false failures. The freeze point is the start of the final
        // trailing run of identical `(Disconnected, input)` observations. A
        // missing `Disconnected` run is represented as `None` and compared too:
        // one survivor seeing no stable freeze while another does is exactly the
        // class this invariant is meant to catch.
        if !applied_inputs.is_empty() {
            for slot in 0..self.n_players {
                if !self.is_dead(slot) {
                    continue;
                }
                let live_running_peers: Vec<usize> = (0..self.n_players)
                    .filter(|peer| {
                        !self.is_dead(*peer)
                            && end_state
                                .get(*peer)
                                .copied()
                                .unwrap_or(SessionState::Synchronizing)
                                == SessionState::Running
                    })
                    .collect();
                let mut canonical: Option<(usize, Option<FreezePoint>)> = None;
                let mut any_stable_freeze = false;
                for &peer in &live_running_peers {
                    let Some(records) = applied_inputs.get(peer) else {
                        continue;
                    };
                    let max_frame = end_confirmed
                        .get(peer)
                        .copied()
                        .unwrap_or(Frame::NULL)
                        .as_i32();
                    let observed = stable_freeze_point(records, slot, max_frame);
                    any_stable_freeze |= observed.is_some();
                    match canonical {
                        None => canonical = Some((peer, observed)),
                        Some((first_author, expected)) if expected != observed => {
                            self.push_failure(OracleFailure::FreezeFrameDivergence {
                                slot,
                                peer,
                                first_author,
                                expected,
                                actual: observed,
                            });
                        },
                        Some(_) => {},
                    }
                }
                if matches!(canonical, Some((_, None)))
                    && !live_running_peers.is_empty()
                    && !any_stable_freeze
                {
                    self.push_failure(OracleFailure::FreezeFrameMissing {
                        slot,
                        live_running_peers,
                    });
                }
            }
        }

        // (d) spectator convergence. A configured spectator must make progress
        // when the mesh still has live Running peers. For each displayed frame
        // inside the live mesh confirmed prefix, compare full input
        // fingerprints and require `Disconnected` statuses to agree with the
        // first live mesh peer that applied that frame. A mesh record may still
        // show a matching live slot as `Predicted` when no later rollback
        // needed to resimulate that early frame; the spectator stream receives
        // confirmed host inputs, so `Predicted` mesh vs `Confirmed` spectator is
        // accepted only when the fingerprint matches.
        if let Some(spectator_records) = spectator_inputs {
            let live_running_peers: Vec<usize> = (0..self.n_players)
                .filter(|peer| {
                    !self.is_dead(*peer)
                        && end_state
                            .get(*peer)
                            .copied()
                            .unwrap_or(SessionState::Synchronizing)
                            == SessionState::Running
                })
                .collect();
            let observed_max_frame = spectator_records.keys().next_back().copied();
            let missing_required_frame = match (spectator_min_frame, observed_max_frame) {
                (Some(required), Some(observed)) => observed < required,
                (Some(_), None) => true,
                (None, _) => spectator_records.is_empty(),
            };
            if missing_required_frame && !live_running_peers.is_empty() {
                self.push_failure(OracleFailure::SpectatorProgressMissing {
                    live_running_peers: live_running_peers.clone(),
                    required_min_frame: spectator_min_frame,
                    observed_max_frame,
                });
            }
            for (&frame, actual) in spectator_records.range(..=global_confirmed) {
                let Some((peer, expected)) =
                    first_live_applied_inputs(applied_inputs, &live_running_peers, frame)
                else {
                    self.push_failure(OracleFailure::SpectatorMeshCanonMissing {
                        frame,
                        live_running_peers: live_running_peers.clone(),
                    });
                    continue;
                };
                if !spectator_matches_mesh_canon(expected, actual) {
                    self.push_failure(OracleFailure::SpectatorInputDivergence {
                        frame,
                        peer,
                        expected: expected.clone(),
                        actual: actual.clone(),
                    });
                }
            }
        }

        // (c) bounded post-heal liveness. Inert unless the schedule healed AND
        // the post-heal drain is long enough for both anchors to be observable
        // (else the anchors would sample an incomplete recovery — indeterminate,
        // never a failure). Retired peers are excluded exactly like (c-lite): a
        // retired peer cannot advance and that is not its own fault. A *live*
        // peer whose confirmed frame did not advance by G within the observed
        // window is pinned (or mutually deadlocked) — fail it, per peer. This is
        // orthogonal to (c-lite): a peer can clear the absolute end bar
        // (recovered late, ends ≥ 30) yet fail this bounded bar (did not advance
        // ≥ G within the window), which is exactly the metastable stall (c-lite)
        // misses.
        // (c) runs only when the runner signalled it (a heal fired AND both
        // anchors are observable); otherwise it stays inert (no heal) or
        // indeterminate (window too short) — `None`, never a pass or a fail.
        let recovered_within_b = if self.heal.ran {
            let mut checked_any = false;
            let mut all_live_ok = true;
            for peer in 0..self.n_players {
                if self.is_dead(peer) {
                    continue;
                }
                checked_any = true;
                // A missing per-peer entry would be a runner bug; read it as
                // NULL (-1) so a plumbing bug fails the bar loudly rather than
                // panicking on an index.
                let at_heal = self.heal.confirmed_at_heal.get(peer).copied().unwrap_or(-1);
                let after = self.heal.confirmed_after.get(peer).copied().unwrap_or(-1);
                // Signed `saturating_sub`: guards only the (impossible for frame
                // numbers) i32 overflow, NOT the sign — a transient dip where
                // `after < at_heal` is preserved as a negative `advanced` and
                // reported raw (it then trips the `< required` bar and shows the
                // dip in the failure), never clamped to 0.
                let advanced = after.saturating_sub(at_heal);
                if advanced < self.heal.required_advance {
                    all_live_ok = false;
                    self.push_failure(OracleFailure::PostHealLiveness {
                        peer,
                        at_heal,
                        after,
                        advanced,
                        required: self.heal.required_advance,
                        window_steps: self.heal.window_steps,
                    });
                }
            }
            // An all-dead mesh has no live peer whose recovery to report: the
            // metastability signal is indeterminate (`None`), not "recovered".
            // (Without this guard the loop skips every peer and `all_live_ok`
            // stays `true`, so a fully-crashed mesh would read `Some(true)`.)
            // The verdict itself still fails independently via `NoLivePeers`.
            checked_any.then_some(all_live_ok)
        } else {
            None
        };

        Verdict {
            failures: self.failures,
            recovered_within_b,
            violation_allowlist_hits: self
                .violation_allowlist
                .iter()
                .copied()
                .zip(self.violation_allowlist_hits)
                .filter_map(|(entry, count)| {
                    (count > 0).then_some(ViolationAllowlistHit { entry, count })
                })
                .collect(),
        }
    }
}

fn first_live_applied_inputs<'a>(
    applied_inputs: &'a [BTreeMap<i32, Vec<(InputFingerprint, InputStatus)>>],
    live_running_peers: &[usize],
    frame: i32,
) -> Option<(usize, &'a Vec<(InputFingerprint, InputStatus)>)> {
    live_running_peers.iter().find_map(|&peer| {
        applied_inputs
            .get(peer)
            .and_then(|records| records.get(&frame))
            .map(|inputs| (peer, inputs))
    })
}

fn spectator_matches_mesh_canon(
    expected: &[(InputFingerprint, InputStatus)],
    actual: &[(InputFingerprint, InputStatus)],
) -> bool {
    expected.len() == actual.len()
        && expected.iter().zip(actual).all(
            |((expected_input, expected_status), (actual_input, actual_status))| {
                expected_input == actual_input
                    && match (expected_status, actual_status) {
                        (InputStatus::Disconnected, InputStatus::Disconnected) => true,
                        (InputStatus::Disconnected, InputStatus::Confirmed)
                        | (InputStatus::Disconnected, InputStatus::Predicted)
                        | (InputStatus::Confirmed, InputStatus::Disconnected)
                        | (InputStatus::Predicted, InputStatus::Disconnected)
                        | (InputStatus::Confirmed, InputStatus::Predicted)
                        | (InputStatus::Predicted, InputStatus::Predicted) => false,
                        (InputStatus::Confirmed, InputStatus::Confirmed)
                        | (InputStatus::Predicted, InputStatus::Confirmed) => true,
                    }
            },
        )
}

fn violation_failure(source: ViolationSource, violation: &SpecViolation) -> OracleFailure {
    OracleFailure::Violation {
        source,
        violation: format!(
            "[{:?}/{:?}] {}",
            violation.severity, violation.kind, violation.message
        ),
    }
}

fn stable_freeze_point(
    records: &BTreeMap<i32, Vec<(InputFingerprint, InputStatus)>>,
    slot: usize,
    max_frame: i32,
) -> Option<FreezePoint> {
    let mut candidate: Option<FreezePoint> = None;
    for (&frame, inputs) in records.range(..=max_frame) {
        let Some((input, status)) = inputs.get(slot).copied() else {
            candidate = None;
            continue;
        };
        if status == InputStatus::Disconnected {
            match candidate {
                Some(point) if point.input == input => {},
                _ => {
                    candidate = Some(FreezePoint { frame, input });
                },
            }
        } else {
            candidate = None;
        }
    }
    candidate
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn fp(input: u32) -> InputFingerprint {
        InputFingerprint::from_bytes(input, &input.to_le_bytes())
    }

    fn fp_with_bytes(input: u32, bytes: &[u8]) -> InputFingerprint {
        InputFingerprint::from_bytes(input, bytes)
    }

    fn inputs(values: &[u32]) -> Vec<InputFingerprint> {
        values.iter().copied().map(fp).collect()
    }

    fn input_statuses(values: &[u32]) -> Vec<(InputFingerprint, InputStatus)> {
        values
            .iter()
            .copied()
            .map(|value| (fp(value), InputStatus::Confirmed))
            .collect()
    }

    fn predicted_input_statuses(values: &[u32]) -> Vec<(InputFingerprint, InputStatus)> {
        values
            .iter()
            .copied()
            .map(|value| (fp(value), InputStatus::Predicted))
            .collect()
    }

    #[derive(Copy, Clone)]
    enum Slot2 {
        Missing,
        Present(u32, InputStatus),
    }

    fn confirmed(input: u32) -> Slot2 {
        Slot2::Present(input, InputStatus::Confirmed)
    }

    fn disconnected(input: u32) -> Slot2 {
        Slot2::Present(input, InputStatus::Disconnected)
    }

    fn slot2_records(
        frames: &[(i32, Slot2)],
    ) -> BTreeMap<i32, Vec<(InputFingerprint, InputStatus)>> {
        frames
            .iter()
            .map(|(frame, slot2)| {
                let mut inputs = vec![
                    (fp(1), InputStatus::Confirmed),
                    (fp(2), InputStatus::Confirmed),
                ];
                if let Slot2::Present(input, status) = slot2 {
                    inputs.push((fp(*input), *status));
                }
                (*frame, inputs)
            })
            .collect()
    }

    fn freeze_verdict(
        peer0: BTreeMap<i32, Vec<(InputFingerprint, InputStatus)>>,
        peer1: BTreeMap<i32, Vec<(InputFingerprint, InputStatus)>>,
    ) -> Verdict {
        freeze_verdict_with_states(
            peer0,
            peer1,
            [
                SessionState::Running,
                SessionState::Running,
                SessionState::Synchronizing,
            ],
        )
    }

    fn freeze_verdict_with_states(
        peer0: BTreeMap<i32, Vec<(InputFingerprint, InputStatus)>>,
        peer1: BTreeMap<i32, Vec<(InputFingerprint, InputStatus)>>,
        end_state: [SessionState; 3],
    ) -> Verdict {
        let mut oracle = Oracle::new(3);
        oracle.mark_peer_dead(2);
        let applied = [peer0, peer1, BTreeMap::new()];
        oracle.finalize_with_applied_inputs(
            &[BTreeMap::new(), BTreeMap::new(), BTreeMap::new()],
            &applied,
            &[Frame::new(60), Frame::new(60), Frame::new(9)],
            &end_state,
        )
    }

    #[test]
    fn freeze_point_uses_final_stable_disconnected_value() {
        let records = slot2_records(&[
            (0, confirmed(30)),
            (10, disconnected(30)),
            (11, disconnected(31)),
            (12, disconnected(31)),
        ]);

        assert_eq!(
            stable_freeze_point(&records, 2, 12),
            Some(FreezePoint {
                frame: 11,
                input: fp(31)
            }),
            "value changes inside a Disconnected tail reset the stable freeze point"
        );
        assert_ne!(
            stable_freeze_point(&records, 2, 12),
            Some(FreezePoint {
                frame: 10,
                input: fp(30)
            }),
            "the first Disconnected status alone is not a stable frozen value"
        );
    }

    #[test]
    fn freeze_point_resets_on_missing_slot() {
        let records = slot2_records(&[
            (0, confirmed(30)),
            (10, disconnected(30)),
            (11, Slot2::Missing),
            (12, disconnected(30)),
        ]);

        assert_eq!(
            stable_freeze_point(&records, 2, 12),
            Some(FreezePoint {
                frame: 12,
                input: fp(30)
            }),
            "a missing slot breaks the stable Disconnected run"
        );
    }

    #[test]
    fn freeze_point_compares_full_input_fingerprint_not_only_logical_value() {
        let first = fp_with_bytes(30, b"same-logical-a");
        let second = fp_with_bytes(30, b"same-logical-b");
        let records = BTreeMap::from([
            (
                10,
                vec![
                    (fp(1), InputStatus::Confirmed),
                    (fp(2), InputStatus::Confirmed),
                    (first, InputStatus::Disconnected),
                ],
            ),
            (
                11,
                vec![
                    (fp(1), InputStatus::Confirmed),
                    (fp(2), InputStatus::Confirmed),
                    (second, InputStatus::Disconnected),
                ],
            ),
        ]);

        assert_eq!(
            stable_freeze_point(&records, 2, 11),
            Some(FreezePoint {
                frame: 11,
                input: second,
            }),
            "a changed serialized fingerprint resets the freeze point even when the logical lane is unchanged",
        );
    }

    /// Negative control: the input-divergence invariant must fire on a
    /// seeded divergence and stay silent on agreement.
    #[test]
    fn oracle_detects_confirmed_input_divergence() {
        let mut oracle = Oracle::new(2);
        oracle.observe_confirmed_inputs(0, 5, inputs(&[1, 2]));
        oracle.observe_confirmed_inputs(1, 5, inputs(&[1, 2]));
        oracle.observe_confirmed_inputs(0, 6, inputs(&[3, 4]));
        oracle.observe_confirmed_inputs(1, 6, inputs(&[3, 9]));

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

    #[test]
    fn oracle_detects_confirmed_input_fingerprint_divergence() {
        let mut oracle = Oracle::new(2);
        oracle.observe_confirmed_inputs(0, 5, vec![fp_with_bytes(7, b"peer-a")]);
        oracle.observe_confirmed_inputs(1, 5, vec![fp_with_bytes(7, b"peer-b")]);

        let verdict = oracle.finalize(
            &[BTreeMap::new(), BTreeMap::new()],
            &[Frame::new(60), Frame::new(60)],
            &[SessionState::Running, SessionState::Running],
        );
        assert!(
            verdict.failures.iter().any(|failure| matches!(
                failure,
                OracleFailure::ConfirmedInputDivergence {
                    frame: 5,
                    peer: 1,
                    ..
                }
            )),
            "same logical input with different serialized identity must fail: {:?}",
            verdict.failures
        );
    }

    #[test]
    fn oracle_accepts_spectator_matching_mesh_canon() {
        let applied = [
            BTreeMap::from([(0, input_statuses(&[1, 2]))]),
            BTreeMap::from([(0, input_statuses(&[1, 2]))]),
        ];
        let spectator = BTreeMap::from([(0, input_statuses(&[1, 2]))]);

        let verdict = Oracle::new(2).finalize_with_applied_inputs_and_spectator(
            &[BTreeMap::new(), BTreeMap::new()],
            &applied,
            &[Frame::new(60), Frame::new(60)],
            &[SessionState::Running, SessionState::Running],
            Some(&spectator),
            None,
        );

        assert!(verdict.passed(), "matching spectator inputs must pass");
    }

    #[test]
    fn oracle_accepts_spectator_confirmed_when_mesh_record_was_predicted() {
        let applied = [
            BTreeMap::from([(0, predicted_input_statuses(&[1, 2]))]),
            BTreeMap::from([(0, predicted_input_statuses(&[1, 2]))]),
        ];
        let spectator = BTreeMap::from([(0, input_statuses(&[1, 2]))]);

        let verdict = Oracle::new(2).finalize_with_applied_inputs_and_spectator(
            &[BTreeMap::new(), BTreeMap::new()],
            &applied,
            &[Frame::new(60), Frame::new(60)],
            &[SessionState::Running, SessionState::Running],
            Some(&spectator),
            None,
        );

        assert!(
            verdict.passed(),
            "the spectator confirmed stream may outrun a mesh game record that never resimulated an early predicted frame"
        );
    }

    #[test]
    fn oracle_detects_spectator_input_divergence() {
        let applied = [
            BTreeMap::from([(0, input_statuses(&[1, 2]))]),
            BTreeMap::from([(0, input_statuses(&[1, 2]))]),
        ];
        let spectator = BTreeMap::from([(0, input_statuses(&[1, 9]))]);

        let verdict = Oracle::new(2).finalize_with_applied_inputs_and_spectator(
            &[BTreeMap::new(), BTreeMap::new()],
            &applied,
            &[Frame::new(60), Frame::new(60)],
            &[SessionState::Running, SessionState::Running],
            Some(&spectator),
            None,
        );

        assert!(
            verdict
                .failures
                .iter()
                .any(|failure| matches!(failure, OracleFailure::SpectatorInputDivergence { .. })),
            "spectator byte/status mismatch must fail: {:?}",
            verdict.failures
        );
    }

    #[test]
    fn oracle_detects_spectator_disconnected_status_divergence() {
        let mesh = vec![
            (fp(1), InputStatus::Confirmed),
            (fp(2), InputStatus::Disconnected),
        ];
        let spectator = BTreeMap::from([(
            20,
            vec![
                (fp(1), InputStatus::Confirmed),
                (fp(2), InputStatus::Confirmed),
            ],
        )]);
        let applied = [
            BTreeMap::from([(20, mesh.clone())]),
            BTreeMap::from([(20, mesh)]),
        ];

        let verdict = Oracle::new(2).finalize_with_applied_inputs_and_spectator(
            &[BTreeMap::new(), BTreeMap::new()],
            &applied,
            &[Frame::new(60), Frame::new(60)],
            &[SessionState::Running, SessionState::Running],
            Some(&spectator),
            None,
        );

        assert!(
            verdict
                .failures
                .iter()
                .any(|failure| matches!(failure, OracleFailure::SpectatorInputDivergence { .. })),
            "spectator must agree with mesh Disconnected statuses: {:?}",
            verdict.failures
        );
    }

    #[test]
    fn oracle_detects_configured_spectator_without_progress() {
        let applied = [
            BTreeMap::from([(0, input_statuses(&[1, 2]))]),
            BTreeMap::from([(0, input_statuses(&[1, 2]))]),
        ];
        let spectator = BTreeMap::new();

        let verdict = Oracle::new(2).finalize_with_applied_inputs_and_spectator(
            &[BTreeMap::new(), BTreeMap::new()],
            &applied,
            &[Frame::new(60), Frame::new(60)],
            &[SessionState::Running, SessionState::Running],
            Some(&spectator),
            None,
        );

        assert!(
            verdict
                .failures
                .iter()
                .any(|failure| matches!(failure, OracleFailure::SpectatorProgressMissing { .. })),
            "a configured spectator with no displayed frames must fail: {:?}",
            verdict.failures
        );
    }

    #[test]
    fn oracle_detects_spectator_missing_required_post_event_frame() {
        let applied = [
            BTreeMap::from([(0, input_statuses(&[1, 2]))]),
            BTreeMap::from([(0, input_statuses(&[1, 2]))]),
        ];
        let spectator = BTreeMap::from([(0, input_statuses(&[1, 2]))]);

        let verdict = Oracle::new(2).finalize_with_applied_inputs_and_spectator(
            &[BTreeMap::new(), BTreeMap::new()],
            &applied,
            &[Frame::new(60), Frame::new(60)],
            &[SessionState::Running, SessionState::Running],
            Some(&spectator),
            Some(10),
        );

        assert!(
            verdict.failures.iter().any(|failure| {
                matches!(
                    failure,
                    OracleFailure::SpectatorProgressMissing {
                        required_min_frame: Some(10),
                        observed_max_frame: Some(0),
                        ..
                    }
                )
            }),
            "spectator must display through the required scenario frame: {:?}",
            verdict.failures
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

    /// A killed peer is excluded from the liveness checks: it cannot fail
    /// end-progress (crashed, so never `Running`), while the *same* shortfall on
    /// a live peer still fails — the alive-mask must exclude only the dead.
    #[test]
    fn oracle_excludes_killed_peer_from_end_checks() {
        // Peer 1 is dead (stuck in Synchronizing); peers 0 and 2 are healthy.
        let recorded = [BTreeMap::new(), BTreeMap::new(), BTreeMap::new()];
        let confirmed = [Frame::new(500), Frame::new(3), Frame::new(500)];
        let states = [
            SessionState::Running,
            SessionState::Synchronizing,
            SessionState::Running,
        ];

        let mut killed = Oracle::new(3);
        killed.mark_peer_dead(1);
        let verdict = killed.finalize(&recorded, &confirmed, &states);
        assert!(
            !verdict
                .failures
                .iter()
                .any(|f| matches!(f, OracleFailure::EndProgress { peer: 1, .. })),
            "a killed peer must not fail end-progress: {:?}",
            verdict.failures
        );

        // Control: the identical shortfall on a *live* peer 1 does fail.
        let live = Oracle::new(3);
        let control = live.finalize(&recorded, &confirmed, &states);
        assert!(
            control
                .failures
                .iter()
                .any(|f| matches!(f, OracleFailure::EndProgress { peer: 1, .. })),
            "a live peer with the same shortfall must fail: {:?}",
            control.failures
        );
    }

    /// Killing *every* peer must not be a vacuous pass: with no live peer left,
    /// the excluded end-checks all skip, so the oracle flags `NoLivePeers`
    /// rather than reporting success for a mesh that proved nothing.
    #[test]
    fn oracle_flags_all_peers_killed_as_no_live_peers() {
        let mut oracle = Oracle::new(2);
        oracle.mark_peer_dead(0);
        oracle.mark_peer_dead(1);
        let verdict = oracle.finalize(
            &[BTreeMap::new(), BTreeMap::new()],
            &[Frame::new(500), Frame::new(500)],
            &[SessionState::Running, SessionState::Running],
        );
        assert!(
            verdict
                .failures
                .iter()
                .any(|f| matches!(f, OracleFailure::NoLivePeers { n_players: 2 })),
            "an all-crashed mesh must fail, not vacuously pass: {:?}",
            verdict.failures
        );
    }

    /// The (i) metastability signal must not read "recovered" for a mesh with no
    /// live peer left. Even when (c) is armed (`ran = true`), an all-dead mesh
    /// has no peer whose recovery to report, so `recovered_within_b` is `None`
    /// (indeterminate) rather than a vacuous `Some(true)` from an empty loop.
    #[test]
    fn recovered_within_b_is_none_when_every_peer_is_killed() {
        let mut oracle = Oracle::new(2);
        oracle.mark_peer_dead(0);
        oracle.mark_peer_dead(1);
        oracle.set_heal_liveness(HealLiveness {
            ran: true,
            window_steps: 250,
            required_advance: POST_HEAL_MIN_ADVANCE,
            confirmed_at_heal: vec![100, 100],
            confirmed_after: vec![100, 100],
        });
        let verdict = oracle.finalize(
            &[BTreeMap::new(), BTreeMap::new()],
            &[Frame::new(500), Frame::new(500)],
            &[SessionState::Running, SessionState::Running],
        );
        assert_eq!(
            verdict.recovered_within_b, None,
            "an all-dead mesh must be indeterminate, not vacuously recovered: {:?}",
            verdict.recovered_within_b
        );
        // And no phantom per-peer liveness failure is charged to a dead peer.
        assert!(
            !verdict
                .failures
                .iter()
                .any(|f| matches!(f, OracleFailure::PostHealLiveness { .. })),
            "dead peers must not be charged a (c) failure: {:?}",
            verdict.failures
        );
    }

    /// Negative control: a desync event and an `Error`-severity violation each
    /// fail the run, while a sub-`Error` (`Warning`) violation does not.
    #[test]
    fn oracle_records_desync_events_and_violations() {
        use fortress_rollback::telemetry::ViolationKind;

        let mut oracle = Oracle::new(2);
        oracle.observe_desync_event(0, Frame::new(42));
        oracle.observe_violations(
            ViolationSource::Peer(1),
            &[
                SpecViolation::new(
                    ViolationSeverity::Error,
                    ViolationKind::ChecksumMismatch,
                    "seeded error violation",
                    "oracle.rs",
                ),
                // A sub-`Error` violation must be ignored by the severity gate.
                SpecViolation::new(
                    ViolationSeverity::Warning,
                    ViolationKind::NetworkProtocol,
                    "seeded warning violation",
                    "oracle.rs",
                ),
            ],
        );
        let verdict = oracle.finalize(
            &[BTreeMap::new(), BTreeMap::new()],
            &[Frame::new(500), Frame::new(500)],
            &[SessionState::Running, SessionState::Running],
        );
        assert!(verdict.failures.iter().any(|f| matches!(
            f,
            OracleFailure::InbandDesyncDetected { peer: 0, frame: 42 }
        )));
        assert!(verdict.failures.iter().any(|f| matches!(
            f,
            OracleFailure::Violation {
                source: ViolationSource::Peer(1),
                ..
            }
        )));
        // Exactly one violation failure: the `Warning` must not have counted.
        assert_eq!(
            verdict
                .failures
                .iter()
                .filter(|f| matches!(f, OracleFailure::Violation { .. }))
                .count(),
            1,
            "only the Error-severity violation should fail the run"
        );
        assert!(
            verdict.violation_allowlist_hits.is_empty(),
            "default empty allowlist must record no hits"
        );
    }

    #[test]
    fn oracle_records_spectator_violations_without_peer_sentinel() {
        use fortress_rollback::telemetry::ViolationKind;

        let mut oracle = Oracle::new(2);
        oracle.observe_violations(
            ViolationSource::Spectator,
            &[SpecViolation::new(
                ViolationSeverity::Error,
                ViolationKind::InternalError,
                "seeded spectator error violation",
                "p2p_spectator_session.rs:1295",
            )],
        );

        let verdict = oracle.finalize(
            &[BTreeMap::new(), BTreeMap::new()],
            &[Frame::new(500), Frame::new(500)],
            &[SessionState::Running, SessionState::Running],
        );

        assert!(verdict.failures.iter().any(|failure| matches!(
            failure,
            OracleFailure::Violation {
                source: ViolationSource::Spectator,
                ..
            }
        )));
        assert!(
            !verdict.failures.iter().any(|failure| matches!(
                failure,
                OracleFailure::Violation {
                    source: ViolationSource::Peer(2),
                    ..
                }
            )),
            "spectator violations must not be encoded as peer == n_players: {:?}",
            verdict.failures
        );
    }

    #[test]
    fn oracle_rejects_out_of_range_peer_violation_source() {
        let mut oracle = Oracle::new(2);
        oracle.observe_violations(ViolationSource::Peer(2), &[]);

        let verdict = oracle.finalize(
            &[BTreeMap::new(), BTreeMap::new()],
            &[Frame::new(500), Frame::new(500)],
            &[SessionState::Running, SessionState::Running],
        );

        assert!(
            verdict.failures.iter().any(|failure| matches!(
                failure,
                OracleFailure::InvalidViolationSource {
                    source: ViolationSource::Peer(2),
                    n_players: 2,
                }
            )),
            "out-of-range peer violation sources must fail explicitly: {:?}",
            verdict.failures
        );
        assert!(
            !verdict
                .failures
                .iter()
                .any(|failure| matches!(failure, OracleFailure::Violation { .. })),
            "invalid sources must not create a confusing peer violation: {:?}",
            verdict.failures
        );
    }

    #[test]
    fn default_violation_allowlist_is_valid_and_reviewable() {
        validate_violation_allowlist(DEFAULT_VIOLATION_ALLOWLIST)
            .expect("reviewed default allowlist must stay valid");
    }

    #[test]
    fn violation_allowlist_validation_rejects_overbroad_entries() {
        use fortress_rollback::telemetry::ViolationKind;

        let critical = [ViolationAllowlistEntry {
            severity: ViolationSeverity::Critical,
            kind: ViolationKind::InternalError,
            location: "sync_test_session.rs:139",
            message_prefix: "critical seeded",
            rationale: "must be rejected",
        }];
        assert!(validate_violation_allowlist(&critical)
            .expect_err("Critical entries must be rejected")
            .contains("Critical"));

        let empty_prefix = [ViolationAllowlistEntry {
            severity: ViolationSeverity::Error,
            kind: ViolationKind::Synchronization,
            location: "p2p_session.rs:441",
            message_prefix: "",
            rationale: "must be rejected",
        }];
        assert!(validate_violation_allowlist(&empty_prefix)
            .expect_err("empty prefixes must be rejected")
            .contains("message_prefix"));

        let bad_location = [ViolationAllowlistEntry {
            severity: ViolationSeverity::Error,
            kind: ViolationKind::Synchronization,
            location: "p2p_session.rs",
            message_prefix: "sync retry budget exceeded",
            rationale: "must be rejected",
        }];
        assert!(validate_violation_allowlist(&bad_location)
            .expect_err("locations need file:line")
            .contains("file:line"));

        let overlapping = [
            ViolationAllowlistEntry {
                severity: ViolationSeverity::Error,
                kind: ViolationKind::Synchronization,
                location: "p2p_session.rs:441",
                message_prefix: "sync retry",
                rationale: "first reviewed prefix",
            },
            ViolationAllowlistEntry {
                severity: ViolationSeverity::Error,
                kind: ViolationKind::Synchronization,
                location: "p2p_session.rs:441",
                message_prefix: "sync retry budget exceeded",
                rationale: "overlaps first",
            },
        ];
        assert!(validate_violation_allowlist(&overlapping)
            .expect_err("overlapping prefixes must be rejected")
            .contains("overlapping"));
    }

    #[test]
    fn violation_signature_uses_reviewed_prefix_or_numeric_normalization() {
        use fortress_rollback::telemetry::ViolationKind;

        static ALLOWLIST: &[ViolationAllowlistEntry] = &[ViolationAllowlistEntry {
            severity: ViolationSeverity::Error,
            kind: ViolationKind::Synchronization,
            location: "p2p_session.rs:441",
            message_prefix: "sync retry budget exceeded",
            rationale: "seeded reviewed prefix",
        }];
        let allowlisted = SpecViolation::new(
            ViolationSeverity::Error,
            ViolationKind::Synchronization,
            "sync retry budget exceeded after 6 attempts",
            "p2p_session.rs:441",
        );
        assert_eq!(
            ViolationSignature::from_violation(&allowlisted, ALLOWLIST),
            ALLOWLIST[0].signature()
        );

        let uncategorized_a = SpecViolation::new(
            ViolationSeverity::Warning,
            ViolationKind::NetworkProtocol,
            "event queue overflow discarded 3 events",
            "p2p_session.rs:9606",
        );
        let uncategorized_b = SpecViolation::new(
            ViolationSeverity::Warning,
            ViolationKind::NetworkProtocol,
            "event queue overflow discarded 7 events",
            "p2p_session.rs:9606",
        );
        assert_eq!(
            ViolationSignature::from_violation(&uncategorized_a, DEFAULT_VIOLATION_ALLOWLIST),
            ViolationSignature::from_violation(&uncategorized_b, DEFAULT_VIOLATION_ALLOWLIST),
            "numeric suffixes should not fragment one census signature"
        );
    }

    #[test]
    fn oracle_allowlists_reviewed_error_violation_and_counts_hits() {
        use fortress_rollback::telemetry::ViolationKind;

        static ALLOWLIST: &[ViolationAllowlistEntry] = &[ViolationAllowlistEntry {
            severity: ViolationSeverity::Error,
            kind: ViolationKind::Synchronization,
            location: "p2p_session.rs:441",
            message_prefix: "sync retry budget exceeded",
            rationale: "seeded test entry; real entries must cite the reviewed source path",
        }];

        let mut oracle = Oracle::new(2);
        oracle.set_violation_allowlist(ALLOWLIST);
        oracle.observe_violations(
            ViolationSource::Peer(1),
            &[
                SpecViolation::new(
                    ViolationSeverity::Error,
                    ViolationKind::Synchronization,
                    "sync retry budget exceeded after 5 attempts",
                    "p2p_session.rs:441",
                ),
                SpecViolation::new(
                    ViolationSeverity::Error,
                    ViolationKind::Synchronization,
                    "sync retry budget exceeded after 6 attempts",
                    "p2p_session.rs:441",
                ),
            ],
        );

        let verdict = oracle.finalize(
            &[BTreeMap::new(), BTreeMap::new()],
            &[Frame::new(500), Frame::new(500)],
            &[SessionState::Running, SessionState::Running],
        );

        assert!(
            !verdict
                .failures
                .iter()
                .any(|failure| matches!(failure, OracleFailure::Violation { .. })),
            "allowlisted Error violations must not fail the run: {:?}",
            verdict.failures
        );
        assert_eq!(
            verdict.violation_allowlist_hits,
            vec![ViolationAllowlistHit {
                entry: ALLOWLIST[0],
                count: 2,
            }]
        );
    }

    #[test]
    fn oracle_counts_allowlisted_warning_without_failing() {
        use fortress_rollback::telemetry::ViolationKind;

        static ALLOWLIST: &[ViolationAllowlistEntry] = &[ViolationAllowlistEntry {
            severity: ViolationSeverity::Warning,
            kind: ViolationKind::NetworkProtocol,
            location: "p2p_session.rs:9606",
            message_prefix: "event queue overflow",
            rationale: "seeded test entry proving warning hit-count visibility",
        }];

        let mut oracle = Oracle::new(2);
        oracle.set_violation_allowlist(ALLOWLIST);
        oracle.observe_violations(
            ViolationSource::Peer(1),
            &[SpecViolation::new(
                ViolationSeverity::Warning,
                ViolationKind::NetworkProtocol,
                "event queue overflow discarded 3 events",
                "p2p_session.rs:9606",
            )],
        );

        let verdict = oracle.finalize(
            &[BTreeMap::new(), BTreeMap::new()],
            &[Frame::new(500), Frame::new(500)],
            &[SessionState::Running, SessionState::Running],
        );

        assert!(
            verdict.failures.is_empty(),
            "allowlisted warnings should remain non-failing: {:?}",
            verdict.failures
        );
        assert_eq!(
            verdict.violation_allowlist_hits,
            vec![ViolationAllowlistHit {
                entry: ALLOWLIST[0],
                count: 1,
            }]
        );
    }

    #[test]
    fn oracle_never_allowlists_critical_violation() {
        use fortress_rollback::telemetry::ViolationKind;

        static ALLOWLIST: &[ViolationAllowlistEntry] = &[ViolationAllowlistEntry {
            severity: ViolationSeverity::Critical,
            kind: ViolationKind::InternalError,
            location: "sync_test_session.rs:139",
            message_prefix: "critical seeded",
            rationale: "negative control only; Critical must still hard-fail",
        }];

        let mut oracle = Oracle::new(2);
        oracle.set_violation_allowlist(ALLOWLIST);
        oracle.observe_violations(
            ViolationSource::Peer(1),
            &[SpecViolation::new(
                ViolationSeverity::Critical,
                ViolationKind::InternalError,
                "critical seeded invariant break",
                "sync_test_session.rs:139",
            )],
        );

        let verdict = oracle.finalize(
            &[BTreeMap::new(), BTreeMap::new()],
            &[Frame::new(500), Frame::new(500)],
            &[SessionState::Running, SessionState::Running],
        );

        assert!(
            verdict.failures.iter().any(|failure| matches!(
                failure,
                OracleFailure::Violation {
                    source: ViolationSource::Peer(1),
                    ..
                }
            )),
            "Critical violations must fail even if an allowlist entry matches: {:?}",
            verdict.failures
        );
        assert!(
            verdict.violation_allowlist_hits.is_empty(),
            "Critical matches must not be counted as tolerated hits"
        );
    }

    /// Session errors carry the failing operation so non-advance harness calls
    /// are diagnosable from the oracle verdict.
    #[test]
    fn oracle_records_session_error_operation() {
        let mut oracle = Oracle::new(2);
        oracle.observe_session_error(
            "remove_player",
            1,
            7,
            &fortress_rollback::FortressError::NotSynchronized,
        );

        let verdict = oracle.finalize(
            &[BTreeMap::new(), BTreeMap::new()],
            &[Frame::new(500), Frame::new(500)],
            &[SessionState::Running, SessionState::Running],
        );
        assert_eq!(verdict.failures.len(), 1, "{:?}", verdict.failures);
        assert!(matches!(
            &verdict.failures[0],
            OracleFailure::SessionError {
                operation: "remove_player",
                peer: 1,
                step: 7,
                error
            } if error.contains("NotSynchronized")
        ));
    }

    /// Negative control for (e): once a slot is dropped, every live survivor must
    /// agree on the stable `Disconnected` frame and frozen value. This seeded
    /// disagreement fails even though the older state/liveness inputs are healthy,
    /// proving the freeze-frame oracle has its own teeth.
    #[test]
    fn oracle_detects_freeze_frame_divergence() {
        let verdict = freeze_verdict(
            slot2_records(&[
                (0, confirmed(30)),
                (10, disconnected(30)),
                (11, disconnected(30)),
            ]),
            slot2_records(&[
                (0, confirmed(30)),
                (10, disconnected(31)),
                (11, disconnected(31)),
            ]),
        );
        assert!(
            verdict.failures.iter().any(|failure| matches!(
                failure,
                OracleFailure::FreezeFrameDivergence {
                    slot: 2,
                    peer: 1,
                    first_author: 0,
                    expected,
                    actual,
                } if *expected == Some(FreezePoint { frame: 10, input: fp(30) })
                    && *actual == Some(FreezePoint { frame: 10, input: fp(31) })
            )),
            "expected a freeze-frame disagreement failure, got {:?}",
            verdict.failures
        );
    }

    /// Same frozen value, different stable frame: the oracle must compare the
    /// freeze frame as well as the value.
    #[test]
    fn oracle_detects_freeze_frame_start_divergence() {
        let verdict = freeze_verdict(
            slot2_records(&[
                (0, confirmed(30)),
                (10, disconnected(30)),
                (11, disconnected(30)),
            ]),
            slot2_records(&[
                (0, confirmed(30)),
                (10, confirmed(30)),
                (11, disconnected(30)),
            ]),
        );
        assert!(
            verdict.failures.iter().any(|failure| matches!(
                failure,
                OracleFailure::FreezeFrameDivergence {
                    slot: 2,
                    peer: 1,
                    first_author: 0,
                    expected,
                    actual,
                } if *expected == Some(FreezePoint { frame: 10, input: fp(30) })
                    && *actual == Some(FreezePoint { frame: 11, input: fp(30) })
            )),
            "expected a freeze-frame start disagreement, got {:?}",
            verdict.failures
        );
    }

    /// Mixed `Some`/`None`: one survivor freezing a retired slot while another
    /// never freezes it must fail as a divergence, not only the all-`None` missing
    /// case.
    #[test]
    fn oracle_detects_one_survivor_missing_freeze_frame() {
        let verdict = freeze_verdict(
            slot2_records(&[
                (0, confirmed(30)),
                (10, disconnected(30)),
                (11, disconnected(30)),
            ]),
            slot2_records(&[(0, confirmed(30)), (10, confirmed(30)), (11, confirmed(30))]),
        );
        assert!(
            verdict.failures.iter().any(|failure| matches!(
                failure,
                OracleFailure::FreezeFrameDivergence {
                    slot: 2,
                    peer: 1,
                    first_author: 0,
                    expected,
                    actual: None,
                } if *expected == Some(FreezePoint { frame: 10, input: fp(30) })
            )),
            "expected a mixed Some/None freeze-frame divergence, got {:?}",
            verdict.failures
        );
    }

    /// Mixed `None`/`Some` in the opposite author order should report the
    /// disagreement without also claiming that no survivor ever froze the slot.
    #[test]
    fn oracle_does_not_report_missing_when_later_survivor_has_freeze_frame() {
        let verdict = freeze_verdict(
            slot2_records(&[(0, confirmed(30)), (10, confirmed(30)), (11, confirmed(30))]),
            slot2_records(&[
                (0, confirmed(30)),
                (10, disconnected(30)),
                (11, disconnected(30)),
            ]),
        );
        assert!(
            verdict.failures.iter().any(|failure| matches!(
                failure,
                OracleFailure::FreezeFrameDivergence {
                    slot: 2,
                    peer: 1,
                    first_author: 0,
                    expected: None,
                    actual,
                } if *actual == Some(FreezePoint { frame: 10, input: fp(30) })
            )),
            "expected a mixed None/Some freeze-frame divergence, got {:?}",
            verdict.failures
        );
        assert!(
            !verdict
                .failures
                .iter()
                .any(|failure| matches!(failure, OracleFailure::FreezeFrameMissing { .. })),
            "a later stable freeze must suppress the all-missing diagnostic: {:?}",
            verdict.failures
        );
    }

    /// Non-`Running` live peers already fail end-progress; their incomplete or
    /// divergent freeze observations should not add secondary (e) failures.
    #[test]
    fn oracle_ignores_non_running_live_peer_for_freeze_frame_comparison() {
        let verdict = freeze_verdict_with_states(
            slot2_records(&[
                (0, confirmed(30)),
                (10, disconnected(30)),
                (11, disconnected(30)),
            ]),
            slot2_records(&[
                (0, confirmed(30)),
                (10, disconnected(31)),
                (11, disconnected(31)),
            ]),
            [
                SessionState::Running,
                SessionState::Synchronizing,
                SessionState::Synchronizing,
            ],
        );
        assert!(
            verdict.failures.iter().any(|failure| matches!(
                failure,
                OracleFailure::EndProgress {
                    peer: 1,
                    state: SessionState::Synchronizing,
                    ..
                }
            )),
            "the non-Running peer should still fail end-progress: {:?}",
            verdict.failures
        );
        assert!(
            !verdict.failures.iter().any(|failure| matches!(
                failure,
                OracleFailure::FreezeFrameDivergence { .. }
                    | OracleFailure::FreezeFrameMissing { .. }
            )),
            "non-Running peers must not add freeze-frame failures: {:?}",
            verdict.failures
        );
    }

    /// Disconnected observations past `end_confirmed` are speculative and must
    /// not perturb the stable freeze point used by the oracle.
    #[test]
    fn oracle_ignores_speculative_freeze_tail_beyond_confirmed() {
        let verdict = freeze_verdict(
            slot2_records(&[
                (0, confirmed(30)),
                (10, disconnected(30)),
                (61, disconnected(31)),
            ]),
            slot2_records(&[(0, confirmed(30)), (10, disconnected(30))]),
        );
        assert!(
            !verdict.failures.iter().any(|failure| matches!(
                failure,
                OracleFailure::FreezeFrameDivergence { .. }
                    | OracleFailure::FreezeFrameMissing { .. }
            )),
            "speculative post-confirmation tails must not fail (e): {:?}",
            verdict.failures
        );
    }

    /// Negative control for the all-`None` case: a retired slot with live,
    /// running survivors must eventually present a stable `Disconnected` run.
    /// Comparing `None == None` would otherwise false-green a mesh that kept
    /// confirming without ever freezing the dropped slot.
    #[test]
    fn oracle_detects_missing_freeze_frame_for_running_survivors() {
        let verdict = freeze_verdict(
            slot2_records(&[(0, confirmed(30)), (10, confirmed(30)), (11, confirmed(30))]),
            slot2_records(&[(0, confirmed(30)), (10, confirmed(30)), (11, confirmed(30))]),
        );
        assert!(
            verdict.failures.iter().any(|failure| matches!(
                failure,
                OracleFailure::FreezeFrameMissing {
                    slot: 2,
                    live_running_peers
                } if live_running_peers == &vec![0, 1]
            )),
            "expected a missing-freeze failure, got {:?}",
            verdict.failures
        );
    }

    /// The failure cap keeps a systemically broken run readable.
    #[test]
    fn oracle_caps_recorded_failures() {
        let mut oracle = Oracle::new(2);
        oracle.observe_confirmed_inputs(0, 0, inputs(&[1]));
        for frame in 0..1000 {
            oracle.observe_confirmed_inputs(0, frame, inputs(&[1]));
            oracle.observe_confirmed_inputs(1, frame, inputs(&[2]));
        }
        let verdict = oracle.finalize(
            &[BTreeMap::new(), BTreeMap::new()],
            &[Frame::new(500), Frame::new(500)],
            &[SessionState::Running, SessionState::Running],
        );
        assert!(verdict.failures.len() <= 64);
    }

    /// The per-class cap must isolate failure *variants*: a flood of one variant
    /// cannot evict a lone failure of another (the HD-1 "silent cap inside the
    /// instrument" regression). This also pins that `push_failure` discriminates
    /// by the failure's own `mem::discriminant`, not the always-equal
    /// discriminant of a reference — one noisy variant filling the cap would
    /// otherwise starve the rest.
    #[test]
    fn oracle_per_class_cap_preserves_rare_variants() {
        let mut oracle = Oracle::new(2);
        // Flood one variant far past the per-class cap (8).
        for frame in 0..100 {
            oracle.observe_desync_event(0, Frame::new(frame));
        }
        // A single failure of a different variant, pushed after the flood.
        oracle.observe_confirmed_unavailable(1, 5, "must survive the flood");

        let desyncs = oracle
            .failures
            .iter()
            .filter(|f| matches!(f, OracleFailure::InbandDesyncDetected { .. }))
            .count();
        let unavailable = oracle
            .failures
            .iter()
            .filter(|f| matches!(f, OracleFailure::ConfirmedInputUnavailable { .. }))
            .count();
        assert_eq!(desyncs, 8, "noisy variant is capped at per_class_cap");
        assert_eq!(
            unavailable, 1,
            "the rare variant is not evicted by the flood"
        );
    }
}
