//! Isolated release-mode cost probe for the production confirmation fold.
//!
//! The fixture builds and warms a real synchronized full mesh outside the
//! timed loop. The measurement is per client: it retains the whole mesh but
//! times only one session's public `confirmed_frame()` fold.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use fortress_rollback::{
    Config, FortressRequest, Message, NonBlockingSocket, P2PSession, PlayerHandle, PlayerType,
    ProtocolConfig, SessionBuilder, SessionState, NULL_FRAME,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::hint::black_box;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use web_time::{Duration, Instant};

const PLAYER_COUNTS: [usize; 4] = [2, 4, 8, 16];
const MAX_RECEIVE_BATCH: usize = 256;
const SYNC_ATTEMPTS: usize = 256;
const WARMUP_FRAMES: usize = 8;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct BenchInput(u8);

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct BenchState;

struct BenchConfig;

impl Config for BenchConfig {
    type Input = BenchInput;
    type State = BenchState;
    type Address = SocketAddr;
}

type Inbox = VecDeque<(SocketAddr, Message)>;
type Fabric = Arc<Mutex<BTreeMap<SocketAddr, Inbox>>>;

struct MeshSocket {
    local_addr: SocketAddr,
    fabric: Fabric,
}

impl NonBlockingSocket<SocketAddr> for MeshSocket {
    fn send_to(&mut self, msg: &Message, addr: &SocketAddr) {
        let mut fabric = self.fabric.lock().expect("mesh fabric lock poisoned");
        fabric
            .get_mut(addr)
            .expect("destination registered before sessions start")
            .push_back((self.local_addr, msg.clone()));
    }

    fn receive_all_messages(&mut self) -> Vec<(SocketAddr, Message)> {
        let mut fabric = self.fabric.lock().expect("mesh fabric lock poisoned");
        let inbox = fabric
            .get_mut(&self.local_addr)
            .expect("local address registered before sessions start");
        let batch_len = inbox.len().min(MAX_RECEIVE_BATCH);
        inbox.drain(..batch_len).collect()
    }
}

struct ManualClock {
    offset_ms: Arc<AtomicU64>,
}

impl ManualClock {
    fn new() -> (Self, fortress_rollback::ClockFn) {
        let base = Instant::now();
        let offset_ms = Arc::new(AtomicU64::new(0));
        let clock_offset = Arc::clone(&offset_ms);
        let clock =
            Arc::new(move || base + Duration::from_millis(clock_offset.load(Ordering::Relaxed)));
        (Self { offset_ms }, clock)
    }

    fn advance(&self, duration: Duration) {
        let millis = u64::try_from(duration.as_millis()).expect("benchmark duration fits u64");
        self.offset_ms.fetch_add(millis, Ordering::Relaxed);
    }
}

fn addresses(num_players: usize) -> Vec<SocketAddr> {
    (0..num_players)
        .map(|index| {
            let port = 30_000_u16
                .checked_add(u16::try_from(index).expect("player count fits u16"))
                .expect("benchmark port fits u16");
            SocketAddr::from(([127, 0, 0, 1], port))
        })
        .collect()
}

fn poll_mesh(sessions: &mut [P2PSession<BenchConfig>], clock: &ManualClock) {
    for session in sessions {
        session.poll_remote_clients();
    }
    clock.advance(Duration::from_millis(50));
}

fn handle_requests(requests: impl IntoIterator<Item = FortressRequest<BenchConfig>>) {
    for request in requests {
        match request {
            FortressRequest::SaveGameState { cell, frame } => {
                cell.save(frame, Some(BenchState), None);
            },
            FortressRequest::LoadGameState { cell, .. } => {
                black_box(cell.load());
            },
            FortressRequest::AdvanceFrame { inputs } => {
                black_box(inputs);
            },
        }
    }
}

fn synchronized_mesh(num_players: usize) -> Vec<P2PSession<BenchConfig>> {
    let addresses = addresses(num_players);
    let fabric: Fabric = Arc::new(Mutex::new(
        addresses
            .iter()
            .copied()
            .map(|addr| (addr, VecDeque::new()))
            .collect(),
    ));
    let (clock, protocol_clock) = ManualClock::new();

    let mut sessions = Vec::with_capacity(num_players);
    for local_index in 0..num_players {
        let protocol_config = ProtocolConfig {
            protocol_rng_seed: Some(
                0x4831_3650_0000_0000_u64
                    .checked_add(u64::try_from(local_index).expect("player index fits u64"))
                    .expect("benchmark seed fits u64"),
            ),
            clock: Some(Arc::clone(&protocol_clock)),
            ..ProtocolConfig::default()
        };
        let mut builder = SessionBuilder::<BenchConfig>::new()
            .with_num_players(num_players)
            .expect("supported benchmark player count")
            .with_protocol_config(protocol_config);
        for (player_index, &addr) in addresses.iter().enumerate() {
            let player_type = if player_index == local_index {
                PlayerType::Local
            } else {
                PlayerType::Remote(addr)
            };
            builder = builder
                .add_player(player_type, PlayerHandle::new(player_index))
                .expect("add benchmark player");
        }
        sessions.push(
            builder
                .start_p2p_session(MeshSocket {
                    local_addr: addresses[local_index],
                    fabric: Arc::clone(&fabric),
                })
                .expect("start benchmark P2P session"),
        );
    }

    for _ in 0..SYNC_ATTEMPTS {
        poll_mesh(&mut sessions, &clock);
        if sessions
            .iter()
            .all(|session| session.current_state() == SessionState::Running)
        {
            break;
        }
    }
    assert!(
        sessions
            .iter()
            .all(|session| session.current_state() == SessionState::Running),
        "all benchmark sessions must synchronize"
    );

    for frame in 0..WARMUP_FRAMES {
        for (player_index, session) in sessions.iter_mut().enumerate() {
            session
                .add_local_input(
                    PlayerHandle::new(player_index),
                    BenchInput(u8::try_from(frame).expect("warmup frame fits u8")),
                )
                .expect("add warmup input");
        }
        for session in &mut sessions {
            let requests = session.advance_frame().expect("advance warmup frame");
            handle_requests(requests);
        }
        for _ in 0..3 {
            poll_mesh(&mut sessions, &clock);
        }
    }
    for _ in 0..8 {
        poll_mesh(&mut sessions, &clock);
    }

    assert!(
        sessions
            .iter()
            .all(|session| session.current_state() == SessionState::Running),
        "warmed benchmark sessions must remain running"
    );
    assert!(
        sessions
            .iter()
            .all(|session| session.confirmed_frame() > NULL_FRAME),
        "benchmark confirmation fold must be populated"
    );
    assert_eq!(
        fabric
            .lock()
            .expect("mesh fabric lock poisoned")
            .values()
            .map(VecDeque::len)
            .sum::<usize>(),
        0,
        "network work must be drained before measurement"
    );

    sessions
}

fn bench_confirmed_frame(c: &mut Criterion) {
    let mut group = c.benchmark_group("H-16P confirmed_frame");
    for num_players in PLAYER_COUNTS {
        let sessions = synchronized_mesh(num_players);
        group.bench_with_input(
            BenchmarkId::new("steady_mesh", format!("N={num_players}")),
            &num_players,
            |b, _| {
                b.iter(|| black_box(black_box(&sessions[0]).confirmed_frame()));
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_confirmed_frame);
criterion_main!(benches);
