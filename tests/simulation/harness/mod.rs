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
    P2PSession, PlayerHandle, PlayerType, ProtocolConfig, RequestVec, SessionBuilder, SessionState,
};
use oracle::{Oracle, Verdict};
use schedule::{Schedule, ScheduleEvent};
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
}

/// Outcome of one simulation run.
#[derive(Clone, Debug)]
pub struct RunReport {
    pub verdict: Verdict,
    /// Deterministic digest of the run's observable trace.
    pub trace_hash: u64,
    /// Each peer's final confirmed frame.
    pub final_confirmed: Vec<i32>,
    /// Network delivery/drop counters.
    pub net_stats: crate::common::sim_net::SimNetStats,
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
            let protocol_config = ProtocolConfig {
                clock: Some(clock.as_protocol_clock()),
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
                ScheduleEvent::HealAll => net.heal_all(),
            }
            next_event += 1;
        }

        // Drive every peer in fixed order.
        for (i, slot) in peers.iter_mut().enumerate() {
            slot.session.poll_remote_clients();

            let events: Vec<FortressEvent<StubConfig>> = slot.session.events().collect();
            for event in &events {
                if let FortressEvent::DesyncDetected { frame, .. } = event {
                    oracle.observe_desync_event(i, *frame);
                }
                fold_trace(&mut trace_hash, &format!("{event:?}"));
            }

            if slot.session.current_state() == SessionState::Running {
                for handle in slot.session.local_player_handles() {
                    if let Err(error) = slot.session.add_local_input(handle, input_for(step, i)) {
                        oracle.observe_advance_error(i, step, &error);
                    }
                }
                match slot.session.advance_frame() {
                    Ok(requests) => slot.game.handle_requests(requests),
                    Err(error) => oracle.observe_advance_error(i, step, &error),
                }
            }

            // Incrementally sample newly confirmed inputs (they evict).
            let confirmed = slot.session.confirmed_frame();
            if confirmed.is_valid() {
                for frame in (slot.sampled_confirmed + 1)..=confirmed.as_i32() {
                    match slot.session.confirmed_inputs_for_frame(Frame::new(frame)) {
                        Ok(inputs) => oracle.observe_confirmed_inputs(i, frame, &inputs),
                        Err(error) => {
                            oracle.observe_confirmed_unavailable(i, frame, &format!("{error:?}"));
                        },
                    }
                    slot.sampled_confirmed = frame;
                }
            }

            fold_trace(
                &mut trace_hash,
                &(step, i, slot.game.gs, confirmed.as_i32()),
            );
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

    let verdict = oracle.finalize(&recorded, &end_confirmed, &end_state);
    RunReport {
        verdict,
        trace_hash,
        final_confirmed,
        net_stats: net.stats(),
    }
}
