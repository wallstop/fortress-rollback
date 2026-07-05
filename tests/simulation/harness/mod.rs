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
use fortress_rollback::telemetry::{CollectingObserver, ViolationSeverity};
use fortress_rollback::{
    DesyncDetection, FortressEvent, FortressRequest, Frame, GameStateCell, InputVec, Message,
    MessageKind, P2PSession, PeerMetrics, PlayerHandle, PlayerType, ProtocolConfig, RequestVec,
    SessionBuilder, SessionState,
};
use oracle::{Oracle, Verdict};
use schedule::{AppModel, Schedule, ScheduleEvent};
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::Arc;

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
            "simulation failed — reproduce with:\n  FORTRESS_SIM_REPRO seed={} n_players={} steps={} noise={:?}\nfinal_confirmed={:?}\nnet={:?}\nfailures ({}):\n{}",
            schedule.seed,
            schedule.config.n_players,
            schedule.config.steps,
            schedule.config.noise,
            self.final_confirmed,
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
struct SimGameStub {
    gs: StateStub,
    /// Post-advance state per frame; last write wins (rollback re-simulation
    /// overwrites), so confirmed frames hold their final state.
    recorded: BTreeMap<i32, StateStub>,
    corrupt_state_from: Option<i32>,
    corrupt_checksum_from: Option<i32>,
}

impl SimGameStub {
    fn new() -> Self {
        Self {
            gs: StateStub { frame: 0, state: 0 },
            recorded: BTreeMap::new(),
            corrupt_state_from: None,
            corrupt_checksum_from: None,
        }
    }

    fn handle_requests(&mut self, requests: RequestVec<StubConfig>) {
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

    fn advance(&mut self, inputs: &InputVec<StubInput>) {
        // Same transition as GameStub/StateStub::advance_frame.
        let total: u32 = inputs.iter().map(|(input, _)| input.inp).sum();
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

struct PeerSlot {
    session: P2PSession<StubConfig>,
    game: SimGameStub,
    observer: Arc<CollectingObserver>,
    /// Highest frame whose confirmed inputs were sampled into the oracle.
    sampled_confirmed: i32,
}

/// Pure per-peer input function: any deterministic mapping works; this one
/// varies across both axes so prediction is frequently wrong (exercising
/// rollback) and per-peer streams never collide.
fn input_for(step: u32, peer: usize) -> StubInput {
    let p = u32::try_from(peer).unwrap_or(0);
    StubInput {
        inp: step
            .wrapping_mul(31)
            .wrapping_add(p.wrapping_mul(7))
            .wrapping_add(1),
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
fn print_step_summary(step: u32, peers: &[PeerSlot], net: &SimNet<Message>) {
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

/// Diagnostic variant of [`run`]: prints per-peer progress every 50 steps.
/// For manual investigation of a repro seed (see `fleet::diagnose_repro`).
// Deliberate diagnostic stdout: this path only runs under `--run-ignored`
// manual investigation, where print output IS the deliverable.
#[allow(clippy::print_stdout, clippy::disallowed_macros)]
pub fn diagnose(schedule: &Schedule) {
    let report = run_inner(schedule, &RunOptions::default(), true);
    println!(
        "final: confirmed={:?} net={:?} failures={:#?}",
        report.final_confirmed, report.net_stats, report.verdict.failures
    );
}

/// Runs one schedule to completion and reports.
#[must_use]
pub fn run(schedule: &Schedule, options: &RunOptions) -> RunReport {
    run_inner(schedule, options, false)
}

fn run_inner(schedule: &Schedule, options: &RunOptions, diagnose: bool) -> RunReport {
    let n = schedule.config.n_players;
    let clock = TestClock::new();
    let net: SimNet<Message> = SimNet::new(schedule.link_seed, clock.as_protocol_clock());

    let addrs: Vec<SocketAddr> = (0..n).map(peer_addr).collect();

    // Validate every peer index up front so a malformed or hand-edited corpus
    // schedule fails loudly with a clear message instead of panicking on a raw
    // slice index deep in the run (or, for `PeerStall` steps, silently in a
    // release build). Covers initial links, every event, and the probe step.
    for (from, to, _) in &schedule.initial_links {
        assert!(
            *from < n && *to < n,
            "initial link ({from} -> {to}) out of range for a {n}-peer mesh"
        );
    }
    for (_, event) in &schedule.events {
        match event {
            ScheduleEvent::SetLink { from, to, .. }
            | ScheduleEvent::Block { from, to, .. }
            | ScheduleEvent::Hold { from, to, .. } => assert!(
                *from < n && *to < n,
                "schedule event link ({from} -> {to}) out of range for a {n}-peer mesh"
            ),
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

    for (from, to, policy) in &schedule.initial_links {
        net.set_link(addrs[*from], addrs[*to], policy.clone());
    }

    // Build one session per peer. Handles: peer i is Local handle i, Remote
    // handle j at addrs[j] for j != i. Protocol RNG seeded per peer so magic
    // numbers/sync tokens are reproducible.
    let mut peers: Vec<PeerSlot> = (0..n)
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
            let mut builder = SessionBuilder::<StubConfig>::new()
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
            let session = builder.start_p2p_session(socket).expect("session starts");

            let mut game = SimGameStub::new();
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

    let mut oracle = Oracle::new(n);
    let mut trace_hash: u64 = 0xcbf2_9ce4_8422_2325; // FNV offset basis
    let mut next_event = 0usize;
    // Per-peer stall deadline (exclusive step): peer `i` is frozen while
    // `step < stalled_until[i]`. `0` means never stalled; a `PeerStall` event
    // sets it to `step + steps`.
    let mut stalled_until: Vec<u32> = vec![0; n];
    // Peers killed by a `PeerKill` event: no longer driven, detached from the
    // fabric, and excluded from the oracle's liveness checks (their pre-death
    // observations still count for agreement).
    let mut dead: Vec<bool> = vec![false; n];
    // Per-peer count of advances still owed to an obeyed `WaitRecommendation`
    // (only accumulates under `AppModel::Obey`). While > 0 the peer polls but
    // does not advance, letting the others catch up — the closed time-sync loop.
    let app_model = schedule.config.app_model;
    let mut wait_skip: Vec<u32> = vec![0; n];
    // Confirmed-frame snapshot taken at `options.probe_confirmed_at`, if any.
    let mut probe_confirmed: Vec<i32> = Vec::new();

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
                    stalled_until[*peer] = step.saturating_add(*steps);
                },
                ScheduleEvent::SetInputDelay { peer, delay } => {
                    // Reconfigure the peer's own local input delay mid-run. A
                    // mid-session increase gap-fills and flushes to remotes; a
                    // failure (e.g. pending-output full) is a real error the
                    // oracle must surface, not swallow.
                    let handle = PlayerHandle::new(*peer);
                    if let Err(error) = peers[*peer].session.set_input_delay(handle, *delay) {
                        oracle.observe_advance_error(*peer, step, &error);
                    }
                },
                ScheduleEvent::PeerKill { peer } => {
                    // Crash the peer: stop driving it, discard its inbox (so
                    // further traffic to it is dropped under the default
                    // `UnattachedPolicy::Drop`), and exclude it from the oracle's
                    // liveness checks. Its remaining mesh survives per the
                    // configured `DisconnectBehavior`. Idempotent.
                    dead[*peer] = true;
                    net.detach(addrs[*peer]);
                    oracle.mark_peer_dead(*peer);
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

                let events: Vec<FortressEvent<StubConfig>> = slot.session.events().collect();
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
                            if let Err(error) =
                                slot.session.add_local_input(handle, input_for(step, i))
                            {
                                oracle.observe_advance_error(i, step, &error);
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
                            Ok(inputs) => oracle.observe_confirmed_inputs(i, frame, &inputs),
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
        }

        if diagnose && step % 50 == 0 {
            print_step_summary(step, &peers, &net);
        }

        clock.advance(schedule.config.step_dt());
    }

    // Final observations.
    for (i, slot) in peers.iter().enumerate() {
        let severe = slot
            .observer
            .violations_at_severity(ViolationSeverity::Error);
        oracle.observe_violations(i, &severe);
    }
    let recorded: Vec<BTreeMap<i32, StateStub>> = peers
        .iter()
        .map(|slot| slot.game.recorded.clone())
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

    let metrics: Vec<fortress_rollback::SessionMetrics> =
        peers.iter().map(|slot| slot.session.metrics()).collect();

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

    let verdict = oracle.finalize(&recorded, &end_confirmed, &end_state);
    RunReport {
        verdict,
        trace_hash,
        final_confirmed,
        probe_confirmed,
        net_stats: net.stats(),
        metrics,
        peer_wire,
    }
}
