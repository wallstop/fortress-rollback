//! Deterministic whole-mesh simulation harness.
//!
//! Runs N real [`P2PSession`]s in one process over a [`SimNet`] fabric under
//! a single virtual clock, drives them with a materialized [`Schedule`], and
//! checks global invariants with the [`Oracle`]. Everything reproduces from
//! `(seed, SimConfig)`: sessions get seeded protocol RNGs, the network gets a
//! seeded fault stream, inputs are a pure function of `(step, peer)`, and the
//! step loop iterates peers in a fixed order.
//!
//! A run's [`RunReport::trace_hash`] folds every observable step artifact;
//! two runs of the same schedule must produce identical hashes (checked by
//! the meta-determinism test in the fleet).

// Test infrastructure: not every test binary uses every helper.
#![allow(dead_code)]

pub mod oracle;
pub mod schedule;

use crate::common::sim_net::{SimNet, SimSocket};
use crate::common::stubs::{StateStub, StubConfig, StubInput};
use crate::common::test_clock::TestClock;
use fortress_rollback::hash::fnv1a_hash;
use fortress_rollback::telemetry::CollectingObserver;
use fortress_rollback::{
    Config, DesyncDetection, FortressError, FortressEvent, FortressRequest, Frame, GameStateCell,
    InputStatus, InputVec, Message, MessageKind, P2PSession, PeerMetrics, PlayerHandle, PlayerType,
    ProtocolConfig, RequestVec, SessionBuilder, SessionState, SpectatorSession,
};
use oracle::{
    validate_violation_allowlist, HealLiveness, InputFingerprint, Oracle, Verdict,
    ViolationSignature, DEFAULT_VIOLATION_ALLOWLIST, POST_HEAL_MIN_ADVANCE,
};
use schedule::{AppModel, Schedule, ScheduleEvent};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::marker::PhantomData;
use std::net::SocketAddr;
use std::sync::Arc;

/// Input contract used by the deterministic simulation harness.
///
/// The production library already supports arbitrary fixed-width `Config::Input`
/// types; this trait keeps the harness's game/oracle semantics stable while the
/// M2 sweep varies only serialized input width.
pub trait SimInput:
    Copy + Clone + PartialEq + Eq + Default + Serialize + DeserializeOwned + Send + Sync + 'static
{
    type SessionConfig: Config<Input = Self, State = StateStub, Address = SocketAddr>
        + std::fmt::Debug;

    /// Serialized byte width of one input value under the crate's fixed-int
    /// bincode codec.
    const WIDTH_BYTES: u32;

    /// Deterministic input for `(step, peer)`.
    fn from_word(word: u32, step: u32, peer: usize) -> Self;

    /// State-transition value used by the harness oracle. This intentionally
    /// stays 32-bit for every input width so wide-input sweep cells isolate wire
    /// cost instead of changing game behavior.
    fn value(self) -> u32;

    /// Full serialized input identity observed by the oracle.
    fn fingerprint(self) -> InputFingerprint;
}

impl SimInput for StubInput {
    type SessionConfig = StubConfig;

    const WIDTH_BYTES: u32 = 4;

    fn from_word(word: u32, _step: u32, _peer: usize) -> Self {
        Self { inp: word }
    }

    fn value(self) -> u32 {
        self.inp
    }

    fn fingerprint(self) -> InputFingerprint {
        InputFingerprint::from_bytes(self.inp, &self.inp.to_le_bytes())
    }
}

/// A 32-byte fixed-width input for the M2 sweep width axis.
///
/// The first word drives the same game-state transition as [`StubInput`]; the
/// seven padding words are deterministic, varying payload so the bandwidth
/// counters measure a real wide input stream rather than a zero-filled artifact.
#[derive(Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct WideStubInput {
    pub inp: u32,
    pub padding: [u32; 7],
}

#[derive(Debug)]
pub struct WideStubConfig;

impl Config for WideStubConfig {
    type Input = WideStubInput;
    type State = StateStub;
    type Address = SocketAddr;
}

impl SimInput for WideStubInput {
    type SessionConfig = WideStubConfig;

    const WIDTH_BYTES: u32 = 32;

    fn from_word(word: u32, step: u32, peer: usize) -> Self {
        let mut padding = [0u32; 7];
        let peer_word = u32::try_from(peer).unwrap_or(u32::MAX);
        for (i, slot) in padding.iter_mut().enumerate() {
            let salt = u32::try_from(i).unwrap_or(u32::MAX).wrapping_add(1);
            *slot = word
                .rotate_left(salt)
                .wrapping_add(step.wrapping_mul(17))
                .wrapping_add(peer_word.wrapping_mul(97))
                .wrapping_add(salt.wrapping_mul(0x9E37));
        }
        Self { inp: word, padding }
    }

    fn value(self) -> u32 {
        self.inp
    }

    fn fingerprint(self) -> InputFingerprint {
        let mut bytes = [0u8; 32];
        let words = [
            self.inp,
            self.padding[0],
            self.padding[1],
            self.padding[2],
            self.padding[3],
            self.padding[4],
            self.padding[5],
            self.padding[6],
        ];
        for (chunk, word) in bytes.chunks_exact_mut(4).zip(words) {
            chunk.copy_from_slice(&word.to_le_bytes());
        }
        InputFingerprint::from_bytes(self.inp, &bytes)
    }
}

/// Options for fault-injection *inside the harness itself* — used by the
/// oracle's negative controls to prove the invariants actually fire.
#[derive(Clone, Debug, Default)]
pub struct RunOptions {
    /// Corrupt `(peer, from_frame)`'s simulated **state** from that frame on
    /// (a real divergence: state, checksums, and downstream frames all split).
    pub corrupt_state_from: Option<(usize, i32)>,
    /// Corrupt `(peer, from_frame)`'s saved **checksums only** (states stay
    /// identical): exercises the in-band detector cross-check path.
    pub corrupt_checksum_from: Option<(usize, i32)>,
    /// If set, snapshot every peer's confirmed frame at this step into
    /// [`RunReport::probe_confirmed`]. Lets a test observe mid-run confirmation
    /// (frozen during a hitch, converged after heal) — end-of-run state alone
    /// hides recovery dynamics because a clean drain always converges. Must be
    /// within `0..steps` (asserted up front, so a requested probe always fires).
    pub probe_confirmed_at: Option<u32>,
    /// Corrupt the configured spectator's first input fingerprint in each
    /// displayed frame at or after this frame. Negative controls only need one
    /// planted spectator-only mismatch to prove the §6.2(d) oracle compares
    /// the spectator path, not only the mesh peers.
    pub corrupt_spectator_input_from: Option<i32>,
    /// Corrupt the first displayed `Disconnected` spectator slot from this
    /// frame onward by reporting it as `Confirmed`. Negative controls use this
    /// to pin the dropped-slot status half of §6.2(d).
    pub corrupt_spectator_status_from: Option<i32>,
}

/// Outcome of one simulation run.
#[derive(Clone, Debug)]
pub struct RunReport {
    pub verdict: Verdict,
    /// Deterministic digest of the run's observable trace.
    pub trace_hash: u64,
    /// Each peer's final confirmed frame.
    pub final_confirmed: Vec<i32>,
    /// Each peer's confirmed frame sampled at [`RunOptions::probe_confirmed_at`]
    /// (empty when no probe step was requested). Indexed by peer.
    pub probe_confirmed: Vec<i32>,
    /// Network delivery/drop counters.
    pub net_stats: crate::common::sim_net::SimNetStats,
    /// Each peer's final [`SessionMetrics`] snapshot (indexed by peer).
    pub metrics: Vec<fortress_rollback::SessionMetrics>,
    /// Each peer's wire-traffic totals, aggregated over all of that peer's
    /// remote links from the always-on `PeerMetrics` counters (indexed by peer).
    /// This is the per-player bandwidth ledger the M2 baseline sweep consumes.
    pub peer_wire: Vec<PeerWireTotals>,
    /// (c) each peer's confirmed frame sampled at the heal anchor — the step of
    /// the last actual `ScheduleEvent::HealAll` (derived from the event stream,
    /// not `schedule.heal_at`, which can drift or be set without a `HealAll`);
    /// empty when the schedule never heals. Indexed by peer.
    pub confirmed_at_heal: Vec<i32>,
    /// (c) each peer's confirmed frame sampled at the recovery anchor — B steps
    /// after the heal, or the run's last step when that lands past the end (an
    /// exact-boundary drain, span B-1); empty when the schedule never heals.
    /// Indexed by peer.
    pub confirmed_after_recovery: Vec<i32>,
    /// (i) metastability: `Some(true/false)` iff the (c) bounded post-heal
    /// liveness check ran — a `HealAll` fired and both anchors are observable (a
    /// full recovery window; span B, or B-1 at an exact-boundary drain). The
    /// explicit "recovered within B steps of heal: yes/no". `None` when (c) was
    /// inert (no heal), the window was too short to observe, or every peer was
    /// killed. Mirrors [`Verdict::recovered_within_b`].
    pub recovered_within_b: Option<bool>,
    /// All telemetry violations observed by every peer, before the oracle's
    /// severity/allowlist policy is applied. Used by the §6.2(f) violation
    /// census so warning-only signatures stay visible even though they do not
    /// fail the run.
    pub violation_census: BTreeMap<ViolationSignature, u64>,
    /// Number of frames the configured spectator displayed and handed to the
    /// oracle. Zero when `SimConfig::spectator_hosts` is empty.
    pub spectator_applied_frames: usize,
    /// Highest frame the configured spectator displayed. `None` when
    /// `SimConfig::spectator_hosts` is empty or the spectator never advanced.
    pub spectator_max_frame: Option<i32>,
}

/// One peer's cumulative wire traffic, summed across every remote link it holds.
///
/// The mesh runner reads each peer session's per-remote [`PeerMetrics`] at
/// end-of-run and folds them into these totals, so a single value describes how
/// much a player put on / took off the wire regardless of mesh size. Byte counts
/// are wire-exact and payload-only (they match `PeerMetrics`, excluding UDP/IP
/// headers). The `messages_{sent,received}_by_kind` arrays are positional in
/// [`MessageKind::ALL`] order; read them by category with
/// [`sent_by_kind`](Self::sent_by_kind) / [`received_by_kind`](Self::received_by_kind).
#[derive(Clone, Debug, Default)]
pub struct PeerWireTotals {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub messages_sent_by_kind: [u64; MessageKind::COUNT],
    pub messages_received_by_kind: [u64; MessageKind::COUNT],
    pub input_bytes_pre_compression: u64,
    pub input_bytes_post_compression: u64,
}

impl PeerWireTotals {
    /// Folds one remote link's [`PeerMetrics`] snapshot into these totals.
    ///
    /// Cumulative counters add; the trailing gauges (`pending_*`, `ping_ms`,
    /// `remote_frame_advantage`) are deliberately dropped — an instantaneous
    /// gauge is not additive across links.
    fn add(&mut self, m: &PeerMetrics) {
        self.bytes_sent = self.bytes_sent.saturating_add(m.bytes_sent);
        self.bytes_received = self.bytes_received.saturating_add(m.bytes_received);
        self.packets_sent = self.packets_sent.saturating_add(m.packets_sent);
        self.packets_received = self.packets_received.saturating_add(m.packets_received);
        // Both arrays are laid out in `MessageKind::ALL` order (the same order
        // used to read them back), independent of the crate-private
        // `MessageKind::index`.
        for (slot, kind) in self.messages_sent_by_kind.iter_mut().zip(MessageKind::ALL) {
            *slot = slot.saturating_add(m.messages_sent_by_kind.get(kind));
        }
        for (slot, kind) in self
            .messages_received_by_kind
            .iter_mut()
            .zip(MessageKind::ALL)
        {
            *slot = slot.saturating_add(m.messages_received_by_kind.get(kind));
        }
        self.input_bytes_pre_compression = self
            .input_bytes_pre_compression
            .saturating_add(m.input_bytes_pre_compression);
        self.input_bytes_post_compression = self
            .input_bytes_post_compression
            .saturating_add(m.input_bytes_post_compression);
    }

    /// Messages of `kind` sent, summed across links.
    #[must_use]
    pub fn sent_by_kind(&self, kind: MessageKind) -> u64 {
        MessageKind::ALL
            .iter()
            .position(|k| *k == kind)
            .and_then(|i| self.messages_sent_by_kind.get(i).copied())
            .unwrap_or(0)
    }

    /// Messages of `kind` received, summed across links.
    #[must_use]
    pub fn received_by_kind(&self, kind: MessageKind) -> u64 {
        MessageKind::ALL
            .iter()
            .position(|k| *k == kind)
            .and_then(|i| self.messages_received_by_kind.get(i).copied())
            .unwrap_or(0)
    }

    /// Total messages sent across all kinds (equals [`Self::packets_sent`]).
    #[must_use]
    pub fn sent_by_kind_total(&self) -> u64 {
        self.messages_sent_by_kind
            .iter()
            .copied()
            .fold(0u64, u64::saturating_add)
    }

    /// Total messages received across all kinds (equals [`Self::packets_received`]).
    #[must_use]
    pub fn received_by_kind_total(&self) -> u64 {
        self.messages_received_by_kind
            .iter()
            .copied()
            .fold(0u64, u64::saturating_add)
    }
}

impl RunReport {
    /// Panics with a reproducible failure report if the run failed.
    #[track_caller]
    pub fn expect_pass(&self, schedule: &Schedule) {
        assert!(
            self.verdict.passed(),
            "simulation failed — reproduce with:\n  FORTRESS_SIM_REPRO seed={} n_players={} steps={} noise={:?}\nfinal_confirmed={:?}\nrecovered_within_b={:?}\nspectator_applied_frames={}\nspectator_max_frame={:?}\nallowlist_hits={:?}\nviolation_census={:?}\nnet={:?}\nfailures ({}):\n{}",
            schedule.seed,
            schedule.config.n_players,
            schedule.config.steps,
            schedule.config.noise,
            self.final_confirmed,
            self.recovered_within_b,
            self.spectator_applied_frames,
            self.spectator_max_frame,
            self.verdict.violation_allowlist_hits,
            self.violation_census,
            self.net_stats,
            self.verdict.failures.len(),
            self.verdict
                .failures
                .iter()
                .map(|f| format!("  - {f:?}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

/// The harness's game stub: `GameStub` semantics (shared `StateStub`
/// transition) plus recording and the negative-control corruption hooks.
struct SimGameStub<I: SimInput> {
    gs: StateStub,
    /// Post-advance state per frame; last write wins (rollback re-simulation
    /// overwrites), so confirmed frames hold their final state.
    recorded: BTreeMap<i32, StateStub>,
    /// Applied inputs per simulated frame; last write wins just like
    /// [`Self::recorded`], so a rollback re-simulation replaces stale transient
    /// statuses with the end-of-run truth. Used by the oracle's dropped-slot
    /// freeze-frame convergence check.
    applied_inputs: BTreeMap<i32, Vec<(InputFingerprint, InputStatus)>>,
    corrupt_state_from: Option<i32>,
    corrupt_checksum_from: Option<i32>,
    input_marker: PhantomData<I>,
}

impl<I: SimInput> SimGameStub<I> {
    fn new() -> Self {
        Self {
            gs: StateStub { frame: 0, state: 0 },
            recorded: BTreeMap::new(),
            applied_inputs: BTreeMap::new(),
            corrupt_state_from: None,
            corrupt_checksum_from: None,
            input_marker: PhantomData,
        }
    }

    fn handle_requests(&mut self, requests: RequestVec<I::SessionConfig>) {
        for request in requests {
            match request {
                FortressRequest::LoadGameState { cell, .. } => self.load(&cell),
                FortressRequest::SaveGameState { cell, frame } => self.save(&cell, frame),
                FortressRequest::AdvanceFrame { inputs } => self.advance(&inputs),
            }
        }
    }

    fn save(&self, cell: &GameStateCell<StateStub>, frame: Frame) {
        assert_eq!(self.gs.frame, frame.as_i32(), "save/state frame mismatch");
        let real_checksum = u128::from(fnv1a_hash(&self.gs));
        let checksum = match self.corrupt_checksum_from {
            Some(from) if frame.as_i32() >= from => real_checksum ^ 0xDEAD_BEEF_CAFE_BABE_u128,
            _ => real_checksum,
        };
        cell.save(frame, Some(self.gs), Some(checksum));
    }

    fn load(&mut self, cell: &GameStateCell<StateStub>) {
        self.gs = cell.load().expect("harness stub: missing saved state");
    }

    fn advance(&mut self, inputs: &InputVec<I>) {
        let frame = self.gs.frame;
        self.applied_inputs.insert(
            frame,
            inputs
                .iter()
                .map(|(input, status)| (input.fingerprint(), *status))
                .collect(),
        );

        // Same transition as GameStub/StateStub::advance_frame.
        let total: u32 = inputs.iter().map(|(input, _)| input.value()).sum();
        if total % 2 == 0 {
            self.gs.state += 2;
        } else {
            self.gs.state -= 1;
        }
        self.gs.frame += 1;

        if let Some(from) = self.corrupt_state_from {
            if self.gs.frame >= from {
                // A deterministic wrong turn: diverges the state transition
                // from this frame onward, exactly like a real determinism bug.
                self.gs.state ^= 1;
            }
        }
        self.recorded.insert(self.gs.frame, self.gs);
    }
}

struct PeerSlot<I: SimInput> {
    session: P2PSession<I::SessionConfig>,
    game: SimGameStub<I>,
    observer: Arc<CollectingObserver>,
    /// Highest frame whose confirmed inputs were sampled into the oracle.
    sampled_confirmed: i32,
}

struct SpectatorSlot<I: SimInput> {
    session: SpectatorSession<I::SessionConfig>,
    observer: Arc<CollectingObserver>,
    applied_inputs: BTreeMap<i32, Vec<(InputFingerprint, InputStatus)>>,
}

fn update_spectator_required_min_frame<I: SimInput>(
    peers: &[PeerSlot<I>],
    dead: &[bool],
    required_min_frame: &mut Option<i32>,
) {
    let Some(live_floor) = peers
        .iter()
        .enumerate()
        .filter(|(peer, _)| !dead[*peer])
        .map(|(_, slot)| slot.session.current_frame().as_i32())
        .min()
    else {
        return;
    };
    let required = live_floor.saturating_add(POST_HEAL_MIN_ADVANCE);
    *required_min_frame =
        Some(required_min_frame.map_or(required, |current| current.max(required)));
}

/// Pure per-peer input function: any deterministic mapping works; this one
/// varies across both axes so prediction is frequently wrong (exercising
/// rollback) and per-peer streams never collide.
fn input_for<I: SimInput>(step: u32, peer: usize) -> I {
    let p = u32::try_from(peer).unwrap_or(0);
    let word = step
        .wrapping_mul(31)
        .wrapping_add(p.wrapping_mul(7))
        .wrapping_add(1);
    I::from_word(word, step, peer)
}

fn corrupt_fingerprint(fingerprint: InputFingerprint) -> InputFingerprint {
    InputFingerprint {
        logical: fingerprint.logical.wrapping_add(1),
        len: fingerprint.len,
        hash: fingerprint.hash ^ 0xA5A5_5A5A_D3C1_B2E0,
    }
}

fn record_spectator_requests<I: SimInput>(
    spectator: &mut SpectatorSlot<I>,
    requests: RequestVec<I::SessionConfig>,
    start_frame: i32,
    corrupt_from: Option<i32>,
    corrupt_status_from: Option<i32>,
) {
    let mut frame = start_frame;
    for request in requests {
        if let FortressRequest::AdvanceFrame { inputs } = request {
            let mut values: Vec<(InputFingerprint, InputStatus)> = inputs
                .iter()
                .map(|(input, status)| (input.fingerprint(), *status))
                .collect();
            if corrupt_from.is_some_and(|from| frame >= from) {
                if let Some((fingerprint, _)) = values.first_mut() {
                    *fingerprint = corrupt_fingerprint(*fingerprint);
                }
            }
            if corrupt_status_from.is_some_and(|from| frame >= from) {
                for (_, status) in &mut values {
                    if *status == InputStatus::Disconnected {
                        *status = InputStatus::Confirmed;
                        break;
                    }
                }
            }
            spectator.applied_inputs.insert(frame, values);
            frame += 1;
        }
    }
}

/// Synthetic mesh addresses (never bound): `127.0.0.1:(20001 + i)`.
fn peer_addr(i: usize) -> SocketAddr {
    let port = 20001 + u16::try_from(i).expect("peer index fits in u16");
    ([127, 0, 0, 1], port).into()
}

/// Folds one hashable item into the running trace digest.
fn fold_trace<T: std::hash::Hash>(hash: &mut u64, item: &T) {
    *hash = fnv1a_hash(&(*hash, fnv1a_hash(item)));
}

/// Per-step progress dump for the diagnostic path.
// Deliberate diagnostic stdout: this path only runs under `--run-ignored`
// manual investigation, where print output IS the deliverable.
#[allow(clippy::print_stdout, clippy::disallowed_macros)]
fn print_step_summary<I: SimInput>(step: u32, peers: &[PeerSlot<I>], net: &SimNet<Message>) {
    let summary: Vec<String> = peers
        .iter()
        .map(|slot| {
            format!(
                "{:?} game_frame={} confirmed={}",
                slot.session.current_state(),
                slot.game.gs.frame,
                slot.session.confirmed_frame().as_i32()
            )
        })
        .collect();
    println!("step {step}: {summary:?}\n  net={:?}", net.stats());
    for (i, slot) in peers.iter().enumerate() {
        println!("  peer{i}: {}", slot.session.diagnostic_connect_status());
    }
}

fn drive_spectator<I: SimInput>(
    spectator: &mut SpectatorSlot<I>,
    step: u32,
    options: &RunOptions,
    oracle: &mut Oracle,
    trace_hash: &mut u64,
) {
    let start_frame = spectator.session.current_frame().as_i32().saturating_add(1);
    match spectator.session.advance_frame() {
        Ok(requests) => record_spectator_requests(
            spectator,
            requests,
            start_frame,
            options.corrupt_spectator_input_from,
            options.corrupt_spectator_status_from,
        ),
        Err(FortressError::PredictionThreshold | FortressError::NotSynchronized) => {},
        Err(error) => oracle.observe_spectator_error("advance_frame", step, &error),
    }

    let events: Vec<FortressEvent<I::SessionConfig>> = spectator.session.events().collect();
    for event in &events {
        if let FortressEvent::SpectatorDivergence { frame, player, .. } = event {
            oracle.observe_spectator_divergence_event(*frame, *player);
        }
        fold_trace(trace_hash, &format!("spectator:{event:?}"));
    }

    fold_trace(
        trace_hash,
        &(
            step,
            "spectator",
            spectator.session.current_state(),
            spectator.session.current_frame().as_i32(),
            spectator.session.num_hosts(),
        ),
    );
}

/// Diagnostic variant of [`run`]: prints per-peer progress every 50 steps.
/// For manual investigation of a repro seed (see `fleet::diagnose_repro`).
// Deliberate diagnostic stdout: this path only runs under `--run-ignored`
// manual investigation, where print output IS the deliverable.
#[allow(clippy::print_stdout, clippy::disallowed_macros)]
pub fn diagnose(schedule: &Schedule) {
    let report = run_inner::<StubInput>(schedule, &RunOptions::default(), true);
    println!(
        "final: confirmed={:?} net={:?} failures={:#?}",
        report.final_confirmed, report.net_stats, report.verdict.failures
    );
}

/// Runs one schedule to completion and reports.
#[must_use]
pub fn run(schedule: &Schedule, options: &RunOptions) -> RunReport {
    run_with_input::<StubInput>(schedule, options)
}

/// Runs one schedule with a specific fixed-width harness input type.
#[must_use]
pub fn run_with_input<I: SimInput>(schedule: &Schedule, options: &RunOptions) -> RunReport {
    run_inner::<I>(schedule, options, false)
}

fn run_inner<I: SimInput>(schedule: &Schedule, options: &RunOptions, diagnose: bool) -> RunReport {
    let n = schedule.config.n_players;
    assert!(
        (2..=16).contains(&n),
        "materialized simulation schedules must use 2..=16 players (got {n})"
    );
    assert!(
        schedule.config.steps >= 2,
        "materialized simulation schedules need at least 2 steps (got {})",
        schedule.config.steps
    );

    let clock = TestClock::new();
    let net: SimNet<Message> = SimNet::new(schedule.link_seed, clock.as_protocol_clock());

    let addrs: Vec<SocketAddr> = (0..n).map(peer_addr).collect();

    assert!(
        schedule
            .events
            .windows(2)
            .all(|pair| pair[0].0 <= pair[1].0),
        "schedule events must be sorted by nondecreasing step"
    );
    for (step, _) in &schedule.events {
        assert!(
            *step < schedule.config.steps,
            "schedule event at step {step} is outside the run (0..{})",
            schedule.config.steps
        );
    }
    if let Some(last_heal) = schedule
        .events
        .iter()
        .filter(|(_, event)| matches!(event, ScheduleEvent::HealAll))
        .map(|(step, _)| *step)
        .max()
    {
        for (step, event) in &schedule.events {
            assert!(
                *step <= last_heal || matches!(event, ScheduleEvent::HealAll),
                "non-HealAll event at step {step} occurs after the last HealAll at {last_heal}"
            );
        }
    }
    let expected_links = n * (n - 1);
    assert_eq!(
        schedule.initial_links.len(),
        expected_links,
        "initial_links must contain exactly one directed policy for every non-self pair"
    );
    let mut seen_links = BTreeSet::new();
    // Validate every peer index up front so a malformed or hand-edited corpus
    // schedule fails loudly with a clear message instead of panicking on a raw
    // slice index deep in the run (or, for `PeerStall` steps, silently in a
    // release build). Covers initial links, every event, and the probe step.
    for (from, to, _) in &schedule.initial_links {
        assert!(
            *from < n && *to < n,
            "initial link ({from} -> {to}) out of range for a {n}-peer mesh"
        );
        assert!(
            *from != *to,
            "initial link ({from} -> {to}) must not target itself"
        );
        assert!(
            seen_links.insert((*from, *to)),
            "duplicate initial link ({from} -> {to})"
        );
    }
    for (_, event) in &schedule.events {
        match event {
            ScheduleEvent::SetLink { from, to, .. }
            | ScheduleEvent::Block { from, to, .. }
            | ScheduleEvent::Hold { from, to, .. } => {
                assert!(
                    *from < n && *to < n,
                    "schedule event link ({from} -> {to}) out of range for a {n}-peer mesh"
                );
                assert!(
                    *from != *to,
                    "schedule event link ({from} -> {to}) must not target itself"
                );
            },
            ScheduleEvent::PeerStall { peer, steps } => {
                assert!(
                    *peer < n,
                    "PeerStall peer {peer} out of range for a {n}-peer mesh"
                );
                assert!(
                    *steps > 0,
                    "PeerStall steps must be > 0 (a 0-step stall freezes nothing)"
                );
            },
            ScheduleEvent::SetInputDelay { peer, .. } => assert!(
                *peer < n,
                "SetInputDelay peer {peer} out of range for a {n}-peer mesh"
            ),
            ScheduleEvent::GracefulRemove { by, target }
            | ScheduleEvent::LegacyDisconnect { by, target } => {
                assert!(
                    *by < n && *target < n,
                    "lifecycle drop ({by} -> {target}) out of range for a {n}-peer mesh"
                );
                assert!(
                    by != target,
                    "lifecycle drop target must be remote (by={by}, target={target})"
                );
            },
            ScheduleEvent::PeerKill { peer } => assert!(
                *peer < n,
                "PeerKill peer {peer} out of range for a {n}-peer mesh"
            ),
            ScheduleEvent::HealAll => {},
        }
    }
    if let Some(probe) = options.probe_confirmed_at {
        assert!(
            probe < schedule.config.steps,
            "probe_confirmed_at ({probe}) is outside the run (0..{})",
            schedule.config.steps
        );
    }
    for &ppm in &schedule.config.clock_skew_ppm {
        assert!(
            ppm >= -1_000_000,
            "clock_skew_ppm must be >= -1_000_000 (-100% = a frozen clock); a \
             lower value would run time backwards (got {ppm})"
        );
    }
    for &peer in &schedule.config.starve_events {
        assert!(
            peer < n,
            "starve_events peer {peer} out of range for a {n}-peer mesh"
        );
    }
    let mut spectator_host_enabled = vec![false; n];
    for &peer in &schedule.config.spectator_hosts {
        assert!(
            peer < n,
            "spectator_hosts peer {peer} out of range for a {n}-peer mesh"
        );
        assert!(
            !spectator_host_enabled[peer],
            "duplicate spectator_hosts peer {peer}"
        );
        spectator_host_enabled[peer] = true;
    }
    if let Some(size) = schedule.config.event_queue_size {
        assert!(
            size >= 10,
            "event_queue_size must be >= 10 (SessionBuilder rejects smaller); got {size}"
        );
    }
    // A 0ms step never advances virtual time and makes the derived (c) recovery
    // window (`RECOVERY_WINDOW_MS / step_dt_ms`) meaningless — reject it loudly
    // rather than let `recovery_window_steps()`'s `.max(1)` div-by-zero guard
    // silently paper over a broken config.
    assert!(
        schedule.config.step_dt_ms >= 1,
        "step_dt_ms must be >= 1 (a 0ms step never advances virtual time)"
    );

    for (from, to, policy) in &schedule.initial_links {
        net.set_link(addrs[*from], addrs[*to], policy.clone());
    }

    // Build one session per peer. Handles: peer i is Local handle i, Remote
    // handle j at addrs[j] for j != i. Protocol RNG seeded per peer so magic
    // numbers/sync tokens are reproducible.
    let spectator_addr = peer_addr(n);
    let spectator_handle = PlayerHandle::new(n);
    let mut peers: Vec<PeerSlot<I>> = (0..n)
        .map(|i| {
            let socket: SimSocket<Message> = net.attach(addrs[i]);
            let observer = Arc::new(CollectingObserver::new());
            // Per-peer clock: the exact base clock at 0 ppm (byte-identical to
            // before), or a rate-skewed clock modeling an unsynchronized local
            // clock (H-SKEW). A missing/short skew vector means "no skew".
            let ppm = schedule.config.clock_skew_ppm.get(i).copied().unwrap_or(0);
            let peer_clock = if ppm == 0 {
                clock.as_protocol_clock()
            } else {
                // ratio (1e6 + ppm) / 1e6. `ppm == -1_000_000` (-100%) is a
                // frozen clock (num = 0); anything below that would run time
                // backwards and is rejected up front, so the fallback is unused.
                let num = u64::try_from(1_000_000_i64 + i64::from(ppm)).unwrap_or(0);
                clock.as_skewed_protocol_clock(num, 1_000_000)
            };
            let protocol_config = ProtocolConfig {
                clock: Some(peer_clock),
                protocol_rng_seed: Some(fnv1a_hash(&(schedule.seed, i))),
                ..ProtocolConfig::default()
            };
            let mut builder = SessionBuilder::<I::SessionConfig>::new()
                .with_num_players(n)
                .expect("valid player count")
                .with_max_prediction_window(schedule.config.max_prediction)
                .with_input_delay(schedule.config.input_delay)
                .expect("valid input delay")
                .with_desync_detection_mode(DesyncDetection::On {
                    interval: schedule.config.desync_interval,
                })
                .with_disconnect_behavior(schedule.config.disconnect_behavior.into())
                .with_protocol_config(protocol_config)
                .with_violation_observer(Arc::clone(&observer) as Arc<_>);
            if let Some(size) = schedule.config.event_queue_size {
                // Validated `>= 10` up front, so the current min-cap check
                // cannot reject it; surface the real error (not a fixed string)
                // if the builder ever grows stricter validation.
                builder = builder.with_event_queue_size(size).unwrap_or_else(|error| {
                    panic!("with_event_queue_size({size}) rejected a pre-validated size: {error:?}")
                });
            }
            for (j, addr) in addrs.iter().enumerate() {
                let player_type = if j == i {
                    PlayerType::Local
                } else {
                    PlayerType::Remote(*addr)
                };
                builder = builder
                    .add_player(player_type, PlayerHandle::new(j))
                    .expect("valid player registration");
            }
            if spectator_host_enabled[i] {
                builder = builder
                    .add_player(PlayerType::Spectator(spectator_addr), spectator_handle)
                    .expect("valid spectator registration");
            }
            let session = builder.start_p2p_session(socket).expect("session starts");

            let mut game = SimGameStub::<I>::new();
            if let Some((peer, from)) = options.corrupt_state_from {
                if peer == i {
                    game.corrupt_state_from = Some(from);
                }
            }
            if let Some((peer, from)) = options.corrupt_checksum_from {
                if peer == i {
                    game.corrupt_checksum_from = Some(from);
                }
            }
            PeerSlot {
                session,
                game,
                observer,
                sampled_confirmed: -1,
            }
        })
        .collect();

    let mut spectator: Option<SpectatorSlot<I>> = (!schedule.config.spectator_hosts.is_empty())
        .then(|| {
            let socket: SimSocket<Message> = net.attach(spectator_addr);
            let observer = Arc::new(CollectingObserver::new());
            let protocol_config = ProtocolConfig {
                clock: Some(clock.as_protocol_clock()),
                protocol_rng_seed: Some(fnv1a_hash(&(schedule.seed, "spectator"))),
                ..ProtocolConfig::default()
            };
            let host_addrs: Vec<SocketAddr> = schedule
                .config
                .spectator_hosts
                .iter()
                .map(|&peer| addrs[peer])
                .collect();
            let mut builder = SessionBuilder::<I::SessionConfig>::new()
                .with_num_players(n)
                .expect("valid player count")
                .with_protocol_config(protocol_config)
                .with_violation_observer(Arc::clone(&observer) as Arc<_>);
            if let Some(size) = schedule.config.event_queue_size {
                builder = builder.with_event_queue_size(size).unwrap_or_else(|error| {
                    panic!(
                        "spectator with_event_queue_size({size}) rejected a \
                         pre-validated size: {error:?}"
                    )
                });
            }
            let session = builder
                .start_spectator_session_multi(&host_addrs, socket)
                .expect("spectator session starts");
            SpectatorSlot {
                session,
                observer,
                applied_inputs: BTreeMap::new(),
            }
        });

    let mut oracle = Oracle::new(n);
    validate_violation_allowlist(DEFAULT_VIOLATION_ALLOWLIST)
        .expect("reviewed default violation allowlist must stay valid");
    let mut trace_hash: u64 = 0xcbf2_9ce4_8422_2325; // FNV offset basis
    let mut next_event = 0usize;
    // Per-peer stall deadline (exclusive step): peer `i` is frozen while
    // `step < stalled_until[i]`. `0` means never stalled; a `PeerStall` event
    // sets it to `step + steps`.
    let mut stalled_until: Vec<u32> = vec![0; n];
    // Peers retired by lifecycle events (`PeerKill`, `GracefulRemove`, or
    // `LegacyDisconnect`): no longer driven, detached from the fabric, and
    // excluded from the oracle's liveness checks (their pre-retirement
    // observations still count for agreement).
    let mut dead: Vec<bool> = vec![false; n];
    // Per-peer count of advances still owed to an obeyed `WaitRecommendation`
    // (only accumulates under `AppModel::Obey`). While > 0 the peer polls but
    // does not advance, letting the others catch up — the closed time-sync loop.
    let app_model = schedule.config.app_model;
    let mut wait_skip: Vec<u32> = vec![0; n];
    // Peers whose app model never drains the session event queue (models a
    // wedged event consumer). Their bounded `event_queue` fills and the session
    // trims oldest events, firing the D9 `events_discarded_*` telemetry. Because
    // the harness normally drains (and feeds) events per step, starvation is the
    // only fleet path that exercises that overflow. Built by direct index (peers
    // validated in-range above) — O(n + |starve_events|), no per-peer rescan.
    let mut starves = vec![false; n];
    for &peer in &schedule.config.starve_events {
        starves[peer] = true;
    }
    // Confirmed-frame snapshot taken at `options.probe_confirmed_at`, if any.
    let mut probe_confirmed: Vec<i32> = Vec::new();

    // (c) bounded post-heal liveness anchors. The heal step is the ACTUAL last
    // `HealAll` event, not `schedule.heal_at` — a schedule can set `heal_at`
    // without emitting a heal (e.g. a no-fault clock-skew run sets it to `steps`
    // with no event), and a hand-authored schedule's `heal_at` field could drift
    // from where its event actually fires. Deriving both the anchor and the
    // window from the event keeps them consistent. (c) runs only when a heal
    // fired AND enough post-heal drain remains for both anchors to be observable
    // (`steps - heal_at >= B`, i.e. the recovery anchor `heal_at + B` is at most
    // the run's end); otherwise it is inert (no heal) or indeterminate (window
    // too short). The recovery anchor clamps to the last recorded step only at
    // the exact boundary `heal_at + B == steps`, giving a span of B-1 there and
    // exactly B otherwise — the runner reports that real span to the oracle, so
    // no case is silently mislabelled a full-B window.
    let heal_step = schedule
        .events
        .iter()
        .filter(|(_, event)| matches!(event, ScheduleEvent::HealAll))
        .map(|(step, _)| *step)
        .max();
    let b_steps = schedule.config.recovery_window_steps();
    let last_step = schedule.config.steps.saturating_sub(1);
    // A healthy peer confirms ~1 frame per step post-heal, so the observed
    // window (in steps) must be at least G wide for the G-frame floor to be
    // clearable at all; a narrower window cannot distinguish a pinned peer from
    // a healthy one, so (c) is indeterminate (`None`) rather than a false
    // `Some(false)` charged against every healthy run. Unreachable at the
    // default 16ms step_dt (span ~250 ≫ G=10); guards a pathologically coarse
    // step_dt / tiny B (and the exact-boundary B-1 span).
    let g_floor = u32::try_from(POST_HEAL_MIN_ADVANCE).unwrap_or(0);
    let (run_c, heal_anchor_at, recovery_anchor_at) = match heal_step {
        Some(heal_at) if schedule.config.steps.saturating_sub(heal_at) >= b_steps => {
            let heal_anchor = heal_at.min(last_step);
            let recovery_anchor = heal_at.saturating_add(b_steps).min(last_step);
            if recovery_anchor.saturating_sub(heal_anchor) >= g_floor {
                (true, heal_anchor, recovery_anchor)
            } else {
                (false, 0, 0)
            }
        },
        _ => (false, 0, 0),
    };
    let mut confirmed_at_heal: Vec<i32> = Vec::new();
    let mut confirmed_after_recovery: Vec<i32> = Vec::new();
    // A spectator floor is in displayed game-frame space, not schedule-step
    // space. Capture it only when a lifecycle event actually retires a peer,
    // using the slowest live survivor's current frame so stalled/waiting app
    // models are not charged for virtual time they never simulated.
    let spectator_enabled = spectator.is_some();
    let mut spectator_required_min_frame: Option<i32> = None;

    for step in 0..schedule.config.steps {
        // Apply control-plane events due at this step.
        while let Some((event_step, event)) = schedule.events.get(next_event) {
            if *event_step > step {
                break;
            }
            match event {
                ScheduleEvent::SetLink { from, to, policy } => {
                    net.set_link(addrs[*from], addrs[*to], policy.clone());
                },
                ScheduleEvent::Block { from, to, blocked } => {
                    net.set_blocked(addrs[*from], addrs[*to], *blocked);
                },
                ScheduleEvent::Hold { from, to, holding } => {
                    net.set_holding(addrs[*from], addrs[*to], *holding);
                },
                ScheduleEvent::PeerStall { peer, steps } => {
                    // `peer` in range and `steps > 0` are validated up front.
                    stalled_until[*peer] = stalled_until[*peer].max(step.saturating_add(*steps));
                },
                ScheduleEvent::SetInputDelay { peer, delay } => {
                    // Reconfigure the peer's own local input delay mid-run. A
                    // mid-session increase gap-fills and flushes to remotes; a
                    // failure (e.g. pending-output full) is a real error the
                    // oracle must surface, not swallow.
                    let handle = PlayerHandle::new(*peer);
                    if let Err(error) = peers[*peer].session.set_input_delay(handle, *delay) {
                        oracle.observe_session_error("set_input_delay", *peer, step, &error);
                    }
                },
                ScheduleEvent::GracefulRemove { by, target } => {
                    // User-driven graceful departure: one survivor explicitly
                    // removes the target and the target stops participating. The
                    // remaining live peers must learn the drop through gossip and
                    // keep a byte-consistent confirmed prefix.
                    if !dead[*by] && !dead[*target] {
                        let handle = PlayerHandle::new(*target);
                        if let Err(error) = peers[*by].session.remove_player(handle) {
                            oracle.observe_session_error("remove_player", *by, step, &error);
                        } else {
                            dead[*target] = true;
                            net.detach(addrs[*target]);
                            oracle.mark_peer_dead(*target);
                            if spectator_enabled {
                                update_spectator_required_min_frame(
                                    &peers,
                                    &dead,
                                    &mut spectator_required_min_frame,
                                );
                            }
                        }
                    }
                },
                ScheduleEvent::LegacyDisconnect { by, target } => {
                    // User-driven legacy disconnect: one survivor explicitly
                    // kicks the target through the older Halt-oriented API. On
                    // success the target stops participating; on error it
                    // stays live and the oracle records the failed API call.
                    // This deliberately does not assert graceful convergence;
                    // D13 tracks the current fabricated-frame Halt behavior.
                    if !dead[*by] && !dead[*target] {
                        let handle = PlayerHandle::new(*target);
                        if let Err(error) = peers[*by].session.disconnect_player(handle) {
                            oracle.observe_session_error("disconnect_player", *by, step, &error);
                        } else {
                            dead[*target] = true;
                            net.detach(addrs[*target]);
                            oracle.mark_peer_dead(*target);
                            if spectator_enabled {
                                update_spectator_required_min_frame(
                                    &peers,
                                    &dead,
                                    &mut spectator_required_min_frame,
                                );
                            }
                        }
                    }
                },
                ScheduleEvent::PeerKill { peer } => {
                    // Crash the peer: stop driving it, discard its inbox (so
                    // further traffic to it is dropped under the default
                    // `UnattachedPolicy::Drop`), and exclude it from the oracle's
                    // liveness checks. Its remaining mesh survives per the
                    // configured `DisconnectBehavior`. Idempotent.
                    if !dead[*peer] {
                        dead[*peer] = true;
                        net.detach(addrs[*peer]);
                        oracle.mark_peer_dead(*peer);
                        if spectator_enabled {
                            update_spectator_required_min_frame(
                                &peers,
                                &dead,
                                &mut spectator_required_min_frame,
                            );
                        }
                    }
                },
                ScheduleEvent::HealAll => net.heal_all(),
            }
            next_event += 1;
        }

        // Drive every peer in fixed order. A peer that is stalled (a local hang:
        // frozen for its stall window) or dead (crashed by `PeerKill`) is not
        // driven — it does not poll, drain events, add input, or advance, and
        // puts nothing on the wire. A stalled peer resumes when its window ends;
        // a dead peer never does. Either way its state is still folded into the
        // trace below so the digest stays uniform and reproduces bit-for-bit.
        for (i, slot) in peers.iter_mut().enumerate() {
            // Confirmed frame after this step's drive (or the frozen value for a
            // stalled/dead peer): read exactly once and reused for both the
            // trace fold and any probe snapshot.
            let confirmed = if !dead[i] && step >= stalled_until[i] {
                slot.session.poll_remote_clients();

                // A starved peer never drains its event queue (models a wedged
                // event consumer): the session's bounded queue fills and trims,
                // firing D9. Skipping the drain forgoes only this peer's own
                // event signals — its self-observed `DesyncDetected` and its
                // event trace folds. The oracle keeps full teeth on it anyway:
                // the primary state-agreement check reads its recorded state
                // directly, and any real divergence is still caught in-band by
                // its neighbors' own desync detection over the wire.
                if !starves[i] {
                    let events: Vec<FortressEvent<I::SessionConfig>> =
                        slot.session.events().collect();
                    for event in &events {
                        if let FortressEvent::DesyncDetected { frame, .. } = event {
                            oracle.observe_desync_event(i, *frame);
                        }
                        // Closed-loop app model: obey a `WaitRecommendation` by owing
                        // that many skipped advances (max so a stronger one wins).
                        if app_model == AppModel::Obey {
                            if let FortressEvent::WaitRecommendation { skip_frames } = event {
                                wait_skip[i] = wait_skip[i].max(*skip_frames);
                            }
                        }
                        fold_trace(&mut trace_hash, &format!("{event:?}"));
                    }
                }

                if slot.session.current_state() == SessionState::Running {
                    if wait_skip[i] > 0 {
                        // Obeying a WaitRecommendation: poll/receive this step
                        // (done above) but skip the advance so the ahead peer
                        // lets the others catch up. Count down only on steps that
                        // would otherwise advance (i.e. while `Running`) — a peer
                        // that briefly leaves `Running` mid-wait (transient
                        // resync) must not silently consume its owed skips, or it
                        // would stop obeying once it resumes.
                        wait_skip[i] -= 1;
                    } else {
                        for handle in slot.session.local_player_handles() {
                            if let Err(error) = slot
                                .session
                                .add_local_input(handle, input_for::<I>(step, i))
                            {
                                oracle.observe_session_error("add_local_input", i, step, &error);
                            }
                        }
                        match slot.session.advance_frame() {
                            Ok(requests) => slot.game.handle_requests(requests),
                            Err(error) => oracle.observe_advance_error(i, step, &error),
                        }
                    }
                }

                // Incrementally sample newly confirmed inputs (they evict).
                let confirmed = slot.session.confirmed_frame();
                if confirmed.is_valid() {
                    for frame in (slot.sampled_confirmed + 1)..=confirmed.as_i32() {
                        match slot.session.confirmed_inputs_for_frame(Frame::new(frame)) {
                            Ok(inputs) => {
                                let values: Vec<InputFingerprint> =
                                    inputs.iter().map(|input| input.fingerprint()).collect();
                                oracle.observe_confirmed_inputs(i, frame, values);
                            },
                            Err(error) => {
                                oracle.observe_confirmed_unavailable(
                                    i,
                                    frame,
                                    &format!("{error:?}"),
                                );
                            },
                        }
                        slot.sampled_confirmed = frame;
                    }
                }
                confirmed
            } else {
                // Stalled (local hang) or dead (crashed): no poll/advance, no
                // wire traffic — just its last confirmed frame, frozen.
                slot.session.confirmed_frame()
            };

            // Fold each peer's (possibly frozen) frame + confirmed state so the
            // trace digest is defined every step, stalled or not.
            fold_trace(
                &mut trace_hash,
                &(step, i, slot.game.gs, confirmed.as_i32()),
            );

            // Optional mid-run confirmation snapshot for recovery-dynamics
            // tests, reusing the value already read for this peer above.
            if options.probe_confirmed_at == Some(step) {
                probe_confirmed.push(confirmed.as_i32());
            }

            // (c) heal-anchored snapshots, reusing the same confirmed value. The
            // peer loop runs in fixed order, so each vector ends up indexed by
            // peer. Only populated when (c) runs (a heal fired with a full
            // recovery window). Captured for every peer — a stalled/dead peer
            // falls through to here with its frozen confirmed frame, so the
            // vectors stay length-n and correctly peer-indexed.
            if run_c && step == heal_anchor_at {
                confirmed_at_heal.push(confirmed.as_i32());
            }
            if run_c && step == recovery_anchor_at {
                confirmed_after_recovery.push(confirmed.as_i32());
            }
        }

        if let Some(spectator) = spectator.as_mut() {
            drive_spectator(spectator, step, options, &mut oracle, &mut trace_hash);
        }

        if diagnose && step % 50 == 0 {
            print_step_summary(step, &peers, &net);
        }

        clock.advance(schedule.config.step_dt());
    }

    let metrics: Vec<fortress_rollback::SessionMetrics> =
        peers.iter().map(|slot| slot.session.metrics()).collect();

    // Final observations.
    let mut violation_census: BTreeMap<ViolationSignature, u64> = BTreeMap::new();
    for ((i, slot), metric) in peers.iter().enumerate().zip(metrics.iter()) {
        let violations = slot.observer.violations();
        for violation in &violations {
            let signature =
                ViolationSignature::from_violation(violation, DEFAULT_VIOLATION_ALLOWLIST);
            *violation_census.entry(signature).or_default() += 1;
        }
        oracle.observe_violations(i, &violations);
        oracle.observe_checksum_mismatches(i, metric.checksums_mismatched);
    }
    if let Some(spectator) = &spectator {
        let violations = spectator.observer.violations();
        for violation in &violations {
            let signature =
                ViolationSignature::from_violation(violation, DEFAULT_VIOLATION_ALLOWLIST);
            *violation_census.entry(signature).or_default() += 1;
        }
        oracle.observe_violations(n, &violations);
    }
    let recorded: Vec<BTreeMap<i32, StateStub>> = peers
        .iter()
        .map(|slot| slot.game.recorded.clone())
        .collect();
    let applied_inputs: Vec<BTreeMap<i32, Vec<(InputFingerprint, InputStatus)>>> = peers
        .iter()
        .map(|slot| slot.game.applied_inputs.clone())
        .collect();
    let end_confirmed: Vec<Frame> = peers
        .iter()
        .map(|slot| slot.session.confirmed_frame())
        .collect();
    let end_state: Vec<SessionState> = peers
        .iter()
        .map(|slot| slot.session.current_state())
        .collect();
    let final_confirmed: Vec<i32> = end_confirmed.iter().map(|frame| frame.as_i32()).collect();
    for confirmed in &final_confirmed {
        fold_trace(&mut trace_hash, confirmed);
    }

    // Aggregate each peer's per-remote wire metrics into one per-player total.
    // Peer `i` holds a remote handle `PlayerHandle::new(j)` for every `j != i`
    // (see the builder loop above); `peer_metrics` succeeds for any remote
    // handle regardless of sync state.
    let n_players = schedule.config.n_players;
    let peer_wire: Vec<PeerWireTotals> = peers
        .iter()
        .enumerate()
        .map(|(i, slot)| {
            let mut totals = PeerWireTotals::default();
            for j in 0..n_players {
                if j == i {
                    continue;
                }
                // Peer `i` registered `PlayerHandle::new(j)` as a remote for
                // every `j != i` (see the builder loop above), so this MUST
                // resolve. An error is a real invariant break — fail loudly
                // rather than silently under-counting a bandwidth regression.
                let pm = slot
                    .session
                    .peer_metrics(PlayerHandle::new(j))
                    .unwrap_or_else(|e| {
                        panic!("peer {i}: peer_metrics(handle={j}) failed unexpectedly: {e:?}")
                    });
                totals.add(&pm);
            }
            totals
        })
        .collect();

    // Hand the oracle the (c) bounded post-heal liveness inputs. `window_steps`
    // is the ACTUAL anchor span (B, or B-1 at an exact-boundary drain where the
    // recovery anchor clamped to the last step), so the failure reports the real
    // window rather than a nominal one.
    oracle.set_heal_liveness(HealLiveness {
        ran: run_c,
        window_steps: recovery_anchor_at.saturating_sub(heal_anchor_at),
        required_advance: POST_HEAL_MIN_ADVANCE,
        confirmed_at_heal: confirmed_at_heal.clone(),
        confirmed_after: confirmed_after_recovery.clone(),
    });
    let spectator_applied_inputs = spectator.as_ref().map(|slot| &slot.applied_inputs);
    let spectator_applied_frames =
        spectator_applied_inputs.map_or(0, std::collections::BTreeMap::len);
    let spectator_max_frame =
        spectator_applied_inputs.and_then(|records| records.keys().next_back().copied());
    let verdict = oracle.finalize_with_applied_inputs_and_spectator(
        &recorded,
        &applied_inputs,
        &end_confirmed,
        &end_state,
        spectator_applied_inputs,
        spectator_required_min_frame,
    );
    let recovered_within_b = verdict.recovered_within_b;
    RunReport {
        verdict,
        trace_hash,
        final_confirmed,
        probe_confirmed,
        net_stats: net.stats(),
        metrics,
        peer_wire,
        confirmed_at_heal,
        confirmed_after_recovery,
        recovered_within_b,
        violation_census,
        spectator_applied_frames,
        spectator_max_frame,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fortress_rollback::network::codec;

    fn assert_input_width<I: SimInput>(input: I) {
        let encoded = codec::encode(&input).expect("harness input serializes");
        assert_eq!(
            encoded.len(),
            usize::try_from(I::WIDTH_BYTES).expect("width fits usize"),
            "SimInput::WIDTH_BYTES must match codec width"
        );
        assert_eq!(
            input.fingerprint(),
            InputFingerprint::from_bytes(input.value(), &encoded),
            "SimInput::fingerprint must cover the full codec bytes"
        );
    }

    #[test]
    fn sim_input_widths_match_codec() {
        assert_input_width(input_for::<StubInput>(7, 1));
        assert_input_width(input_for::<WideStubInput>(7, 1));
    }

    #[test]
    fn default_run_matches_explicit_stub_input_run() {
        let schedule = schedule::generate(
            7,
            schedule::SimConfig {
                steps: 180,
                ..schedule::SimConfig::smoke(2)
            },
        );

        let implicit = run(&schedule, &RunOptions::default());
        let explicit = run_with_input::<StubInput>(&schedule, &RunOptions::default());

        assert_eq!(implicit.trace_hash, explicit.trace_hash);
        assert_eq!(implicit.final_confirmed, explicit.final_confirmed);
        assert_eq!(implicit.net_stats, explicit.net_stats);
        assert_eq!(implicit.recovered_within_b, explicit.recovered_within_b);
        assert_eq!(implicit.violation_census, explicit.violation_census);
        assert_eq!(
            implicit.spectator_applied_frames,
            explicit.spectator_applied_frames
        );
        assert_eq!(implicit.spectator_max_frame, explicit.spectator_max_frame);
        assert_eq!(implicit.verdict.passed(), explicit.verdict.passed());
    }
}
