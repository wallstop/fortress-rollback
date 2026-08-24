//! Benchmarks for endpoint-free, all-local P2P frame advancement.
//!
//! Run with: `cargo bench --bench p2p_all_local`

#![allow(clippy::expect_used)]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use fortress_rollback::{
    Config, FortressRequest, Message, NonBlockingSocket, PlayerHandle, SessionBuilder,
};
use serde::{Deserialize, Serialize};
use std::hint::black_box;
use std::net::SocketAddr;

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
struct BenchInput {
    buttons: u8,
}

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "hot-join", derive(Serialize, Deserialize))]
struct BenchState {
    frame: i32,
}

struct BenchConfig;

impl Config for BenchConfig {
    type Input = BenchInput;
    type State = BenchState;
    type Address = SocketAddr;
}

struct BenchSocket;

impl NonBlockingSocket<SocketAddr> for BenchSocket {
    fn send_to(&mut self, _msg: &Message, _addr: &SocketAddr) {}

    fn receive_all_messages(&mut self) -> Vec<(SocketAddr, Message)> {
        Vec::new()
    }
}

/// Covers local-input staging, confirmation, sync-layer input assembly,
/// request construction, and the endpoint-free poll fast path.
fn bench_all_local_p2p_frame(c: &mut Criterion) {
    let mut group = c.benchmark_group("P2PSession");

    for num_players in [2, 4, 16] {
        group.bench_with_input(
            BenchmarkId::new("all_local_advance_frame", num_players),
            &num_players,
            |b, &num_players| {
                let mut builder = SessionBuilder::<BenchConfig>::new()
                    .with_num_players(num_players)
                    .expect("configure all-local P2P benchmark");
                for player in 0..num_players {
                    builder = builder
                        .add_local_player(player)
                        .expect("add all-local P2P benchmark player");
                }
                let mut session = builder
                    .start_p2p_session(BenchSocket)
                    .expect("create all-local P2P benchmark session");
                let mut state = BenchState::default();

                b.iter(|| {
                    for player in 0..num_players {
                        session
                            .add_local_input(
                                PlayerHandle::new(player),
                                BenchInput {
                                    buttons: u8::try_from(player).unwrap_or(u8::MAX),
                                },
                            )
                            .expect("add all-local benchmark input");
                    }

                    for request in session
                        .advance_frame()
                        .expect("advance all-local P2P benchmark frame")
                    {
                        match request {
                            FortressRequest::SaveGameState { cell, frame } => {
                                cell.save(frame, Some(state.clone()), None);
                            },
                            FortressRequest::LoadGameState { cell, .. } => {
                                if let Some(saved) = cell.load() {
                                    state = saved;
                                }
                            },
                            FortressRequest::AdvanceFrame { inputs } => {
                                state.frame = state.frame.saturating_add(1);
                                black_box(inputs);
                            },
                        }
                    }
                    black_box(&state);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_all_local_p2p_frame);
criterion_main!(benches);
