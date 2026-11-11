//! Representative-state gameplay/performance qualification.
//!
//! The existing deterministic sweep uses a two-integer state stub. This target
//! runs real `P2PSession` meshes over an in-memory fabric with configurable
//! snapshot payloads, per-frame simulation work, player counts, input delay,
//! and prediction window. It records frame-service percentiles, rollback work,
//! allocations, bandwidth, confirmation progress, and input latency.
//!
//! The smoke row runs in the normal test suite. The full tier ledger is
//! release-only and manual/nightly:
//!
//! ```bash
//! cargo nextest run --release --test representative_perf --run-ignored ignored-only \
//!   -E 'test(representative_tier_ledger)' --no-capture
//! ```
//!
//! Set `FORTRESS_PERF_OUT=/path/report.jsonl` to retain the machine-readable
//! ledger. Reference budgets and the interpretation live in `docs/tuning.md`.
#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_arguments,
    clippy::print_stdout,
    clippy::disallowed_macros
)]

use fortress_rollback::{
    Config, FortressRequest, Message, NonBlockingSocket, P2PSession, PlayerHandle, PlayerType,
    ProtocolConfig, RequestVec, SessionBuilder, SessionState,
};
use serde::{Deserialize, Serialize};
use stats_alloc::{Region, Stats, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::alloc::System;
use std::collections::{BTreeMap, VecDeque};
use std::hint::black_box;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use web_time::{Duration, Instant as ClockInstant};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

const STEP_MS: u64 = 16;
const SYNC_ATTEMPTS: usize = 512;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct RepInput {
    buttons: u32,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "hot-join", derive(Serialize, Deserialize))]
struct RepState {
    frame: i32,
    payload: Vec<u8>,
    digest: u64,
}

struct RepConfig;

impl Config for RepConfig {
    type Input = RepInput;
    type State = RepState;
    type Address = SocketAddr;
}

struct RepGame {
    state: RepState,
    sim_work_bytes: usize,
}

impl RepGame {
    fn new(state_bytes: usize, sim_work_bytes: usize) -> Self {
        Self {
            state: RepState {
                frame: 0,
                payload: vec![0; state_bytes],
                digest: 0xcbf2_9ce4_8422_2325,
            },
            sim_work_bytes,
        }
    }

    fn handle_requests(&mut self, requests: RequestVec<RepConfig>) {
        for request in requests {
            match request {
                FortressRequest::LoadGameState { cell, .. } => {
                    self.state = cell.load().expect("representative save present");
                },
                FortressRequest::SaveGameState { cell, frame } => {
                    cell.save(
                        frame,
                        Some(self.state.clone()),
                        Some(u128::from(self.state.digest)),
                    );
                },
                FortressRequest::AdvanceFrame { inputs } => {
                    let sum: u64 = inputs
                        .iter()
                        .map(|(input, _)| u64::from(input.buttons))
                        .sum();
                    self.state.digest = self
                        .state
                        .digest
                        .wrapping_mul(0x100_0000_01b3)
                        .wrapping_add(sum);
                    // Representative simulation work: touch a configured
                    // number of state bytes in complete passes, without
                    // changing allocation shape.
                    let len = self.state.payload.len().max(1);
                    let mut remaining = self.sim_work_bytes;
                    while remaining > 0 {
                        let chunk = remaining.min(len);
                        for byte in self.state.payload.iter_mut().take(chunk) {
                            *byte = byte.wrapping_add(sum as u8);
                        }
                        remaining -= chunk;
                    }
                    self.state.frame += 1;
                },
            }
        }
    }
}

type Inbox = VecDeque<(SocketAddr, Message)>;
type Fabric = Arc<Mutex<BTreeMap<SocketAddr, Inbox>>>;

struct MeshSocket {
    local_addr: SocketAddr,
    fabric: Fabric,
    drop_period: usize,
    sent: u64,
}

impl NonBlockingSocket<SocketAddr> for MeshSocket {
    fn send_to(&mut self, msg: &Message, addr: &SocketAddr) {
        self.sent = self.sent.saturating_add(1);
        let drop_period = u64::try_from(self.drop_period).unwrap_or(0);
        if drop_period > 0 && self.sent % drop_period == 0 {
            return;
        }
        let mut fabric = self.fabric.lock().expect("fabric lock");
        fabric
            .get_mut(addr)
            .expect("destination registered")
            .push_back((self.local_addr, msg.clone()));
    }

    fn receive_all_messages(&mut self) -> Vec<(SocketAddr, Message)> {
        let mut fabric = self.fabric.lock().expect("fabric lock");
        let inbox = fabric.get_mut(&self.local_addr).expect("local registered");
        let batch_len = inbox.len().min(256);
        inbox.drain(..batch_len).collect()
    }
}

struct ManualClock {
    offset_ms: Arc<AtomicU64>,
}

impl ManualClock {
    fn new() -> (Self, fortress_rollback::ClockFn) {
        let base = ClockInstant::now();
        let offset_ms = Arc::new(AtomicU64::new(0));
        let clock_offset = Arc::clone(&offset_ms);
        let clock =
            Arc::new(move || base + Duration::from_millis(clock_offset.load(Ordering::Relaxed)));
        (Self { offset_ms }, clock)
    }

    fn advance(&self, duration: Duration) {
        let millis = u64::try_from(duration.as_millis()).expect("fits u64");
        self.offset_ms.fetch_add(millis, Ordering::Relaxed);
    }
}

fn addresses(num_players: usize) -> Vec<SocketAddr> {
    (0..num_players)
        .map(|index| {
            SocketAddr::from((
                [127, 0, 0, 1],
                32_000_u16
                    .checked_add(u16::try_from(index).expect("player fits u16"))
                    .expect("port fits u16"),
            ))
        })
        .collect()
}

fn build_mesh(
    players: usize,
    input_delay: usize,
    max_prediction: usize,
    state_bytes: usize,
    sim_work_bytes: usize,
    drop_period: usize,
) -> (Vec<P2PSession<RepConfig>>, Vec<RepGame>, ManualClock) {
    let addresses = addresses(players);
    let fabric: Fabric = Arc::new(Mutex::new(
        addresses
            .iter()
            .copied()
            .map(|addr| (addr, VecDeque::new()))
            .collect(),
    ));
    let (clock, protocol_clock) = ManualClock::new();
    let mut sessions = Vec::with_capacity(players);
    let mut games = Vec::with_capacity(players);
    for local_index in 0..players {
        let protocol_config = ProtocolConfig {
            protocol_rng_seed: Some(
                0x5250_4552_0000_0000_u64
                    .checked_add(u64::try_from(local_index).expect("fits u64"))
                    .expect("seed fits"),
            ),
            clock: Some(Arc::clone(&protocol_clock)),
            ..ProtocolConfig::default()
        };
        let mut builder = SessionBuilder::<RepConfig>::new()
            .with_num_players(players)
            .expect("supported player count")
            .with_input_delay(input_delay)
            .expect("input delay")
            .with_max_prediction_window(max_prediction)
            .with_protocol_config(protocol_config);
        for (player_index, &addr) in addresses.iter().enumerate() {
            let player_type = if player_index == local_index {
                PlayerType::Local
            } else {
                PlayerType::Remote(addr)
            };
            builder = builder
                .add_player(player_type, PlayerHandle::new(player_index))
                .expect("add player");
        }
        sessions.push(
            builder
                .start_p2p_session(MeshSocket {
                    local_addr: addresses[local_index],
                    fabric: Arc::clone(&fabric),
                    drop_period,
                    sent: 0,
                })
                .expect("start session"),
        );
        games.push(RepGame::new(state_bytes, sim_work_bytes));
    }

    for _ in 0..SYNC_ATTEMPTS {
        for session in &mut sessions {
            session.poll_remote_clients();
        }
        clock.advance(Duration::from_millis(STEP_MS));
        if sessions
            .iter()
            .all(|session| session.current_state() == SessionState::Running)
        {
            return (sessions, games, clock);
        }
    }
    panic!("representative mesh did not synchronize");
}

fn percentile(sorted: &[u128], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let count = sorted.len();
    let index = ((p * count as f64).ceil() as usize)
        .saturating_sub(1)
        .min(count - 1);
    sorted[index] as f64 / 1_000_000.0
}

#[derive(Debug, Serialize)]
struct TierReport {
    tier: &'static str,
    players: usize,
    state_bytes: usize,
    sim_work_bytes: usize,
    input_delay: usize,
    max_prediction: usize,
    frames: usize,
    input_latency_ms: f64,
    p50_ms: f64,
    p95_ms: f64,
    p99_ms: f64,
    p999_ms: f64,
    rollbacks: u64,
    resimulated_frames: u64,
    max_rollback_depth: u32,
    bytes_sent_per_player: u64,
    bytes_received_per_player: u64,
    allocations: u64,
    net_allocated_bytes: u64,
    min_final_confirmed: i32,
}

fn run_tier(
    tier: &'static str,
    players: usize,
    state_bytes: usize,
    sim_work_bytes: usize,
    input_delay: usize,
    max_prediction: usize,
    frames: usize,
    drop_period: usize,
) -> TierReport {
    let (mut sessions, mut games, clock) = build_mesh(
        players,
        input_delay,
        max_prediction,
        state_bytes,
        sim_work_bytes,
        drop_period,
    );
    let mut samples = Vec::with_capacity(frames * players);
    let region = Region::new(GLOBAL);
    for frame in 0..frames {
        for (index, session) in sessions.iter_mut().enumerate() {
            session
                .add_local_input(
                    PlayerHandle::new(index),
                    RepInput {
                        buttons: u32::try_from(frame).unwrap_or(u32::MAX)
                            ^ u32::try_from(index).unwrap_or(0),
                    },
                )
                .expect("local input");
        }
        for (index, session) in sessions.iter_mut().enumerate() {
            let start = Instant::now();
            let requests = session.advance_frame().expect("advance");
            games[index].handle_requests(requests);
            black_box(&games[index].state);
            samples.push(start.elapsed().as_nanos());
        }
        for session in &mut sessions {
            session.poll_remote_clients();
        }
        clock.advance(Duration::from_millis(STEP_MS));
    }
    let stats: Stats = region.change();
    let mut sorted = samples;
    sorted.sort_unstable();

    let mut rollbacks = 0_u64;
    let mut resimulated_frames = 0_u64;
    let mut max_depth = 0_u32;
    let mut bytes_sent = 0_u64;
    let mut bytes_received = 0_u64;
    let mut final_confirmed = Vec::with_capacity(players);
    for session in &sessions {
        let metrics = session.metrics();
        rollbacks = rollbacks.saturating_add(metrics.rollback_count);
        resimulated_frames = resimulated_frames.saturating_add(metrics.resimulated_frames);
        max_depth = max_depth.max(metrics.max_rollback_depth);
        final_confirmed.push(session.confirmed_frame().as_i32());
        for handle in session.remote_player_handles() {
            if let Ok(peer) = session.peer_metrics(handle) {
                bytes_sent = bytes_sent.saturating_add(peer.bytes_sent);
                bytes_received = bytes_received.saturating_add(peer.bytes_received);
            }
        }
    }
    let players_u64 = u64::try_from(players).unwrap_or(1);
    TierReport {
        tier,
        players,
        state_bytes,
        sim_work_bytes,
        input_delay,
        max_prediction,
        frames,
        input_latency_ms: input_delay as f64 * STEP_MS as f64,
        p50_ms: percentile(&sorted, 0.50),
        p95_ms: percentile(&sorted, 0.95),
        p99_ms: percentile(&sorted, 0.99),
        p999_ms: percentile(&sorted, 0.999),
        rollbacks,
        resimulated_frames,
        max_rollback_depth: max_depth,
        bytes_sent_per_player: bytes_sent / players_u64,
        bytes_received_per_player: bytes_received / players_u64,
        allocations: u64::try_from(stats.allocations).unwrap_or(u64::MAX),
        net_allocated_bytes: u64::try_from(
            stats
                .bytes_allocated
                .saturating_sub(stats.bytes_deallocated),
        )
        .unwrap_or(u64::MAX),
        min_final_confirmed: final_confirmed.iter().copied().min().unwrap_or(i32::MIN),
    }
}

fn write_report(report: &TierReport) {
    if let Ok(path) = std::env::var("FORTRESS_PERF_OUT") {
        let line = serde_json::to_string(report).expect("report serializes");
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .expect("perf output writable");
        use std::io::Write as _;
        writeln!(file, "{line}").expect("perf output writes");
    }
    println!(
        "PERF tier={} players={} state={}B work={}B frames={} input_latency={:.1}ms \
         p50={:.3}ms p95={:.3}ms p99={:.3}ms p99.9={:.3}ms rollbacks={} resim={} \
         depth={} tx/player={}B rx/player={}B allocs={} net_alloc={}B confirmed_min={}",
        report.tier,
        report.players,
        report.state_bytes,
        report.sim_work_bytes,
        report.frames,
        report.input_latency_ms,
        report.p50_ms,
        report.p95_ms,
        report.p99_ms,
        report.p999_ms,
        report.rollbacks,
        report.resimulated_frames,
        report.max_rollback_depth,
        report.bytes_sent_per_player,
        report.bytes_received_per_player,
        report.allocations,
        report.net_allocated_bytes,
        report.min_final_confirmed,
    );
}

/// Normal-suite smoke: the representative workload path produces correct,
/// progressing sessions and a populated report. The full tier ledger is
/// release-only to keep debug CI bounded.
#[test]
fn representative_two_player_smoke() {
    let report = run_tier("2p-smoke", 2, 4096, 1024, 1, 8, 48, 0);
    assert_eq!(report.players, 2);
    assert_eq!(report.input_delay, 1);
    assert!(report.p99_ms > 0.0);
    assert!(
        report.min_final_confirmed > 20,
        "representative smoke must confirm real frames: {report:?}"
    );
    write_report(&report);
}

/// Release-only representative tier ledger. Run manually/nightly; the chosen
/// budgets and reference distributions are documented in `docs/tuning.md`.
#[test]
#[ignore = "release-mode representative tier ledger; run manually or nightly"]
fn representative_tier_ledger() {
    let tiers = [
        ("2p-rep-64k", 2, 64 * 1024, 1024 * 1024, 0, 8, 300, 5),
        ("4p-rep-256k", 4, 256 * 1024, 4 * 1024 * 1024, 0, 8, 240, 5),
        ("16p-rep-64k", 16, 64 * 1024, 2 * 1024 * 1024, 0, 8, 120, 20),
    ];
    let mut reports = Vec::new();
    for (tier, players, state, work, delay, prediction, frames, drop) in tiers {
        let report = run_tier(tier, players, state, work, delay, prediction, frames, drop);
        assert!(
            report.min_final_confirmed > i32::try_from(frames / 2).unwrap_or(0),
            "tier {tier} did not confirm at least half its frames: {report:?}"
        );
        if std::env::var("FORTRESS_PERF_ENFORCE").is_ok() {
            let budget_ms = if report.players >= 16 { 4.0 } else { 2.0 };
            assert!(
                report.p99_ms <= budget_ms,
                "tier {} p99 {:.3}ms exceeds the {:.1}ms reference budget: {report:?}",
                report.tier,
                report.p99_ms,
                budget_ms
            );
        }
        write_report(&report);
        reports.push(report);
    }
    assert_eq!(reports.len(), 3);
}
