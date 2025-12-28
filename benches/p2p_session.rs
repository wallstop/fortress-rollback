//! Benchmarks for P2P session operations
//!
//! Run with: cargo bench --bench p2p_session
//!
//! These benchmarks measure the performance of key session operations that run
//! every frame (60+ times/second in typical games).

// Allow benchmark-specific patterns
#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::needless_pass_by_ref_mut,
    clippy::use_self
)]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use fortress_rollback::{
    Config, FortressRequest, Frame, PlayerHandle, SessionBuilder, SyncTestSession,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hint::black_box;
use std::net::SocketAddr;

/// Simple test input type for benchmarking
#[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize, Debug)]
struct BenchInput {
    buttons: u8,
    stick_x: i8,
    stick_y: i8,
}

/// Simple test state type for benchmarking
#[derive(Clone, Debug, Default)]
struct BenchState {
    frame: i32,
    // Additional state fields would be here in a real game
    #[allow(dead_code)]
    player_positions: [(i32, i32); 2],
}

/// Config type for benchmarks
struct BenchConfig;

impl Config for BenchConfig {
    type Input = BenchInput;
    type State = BenchState;
    type Address = SocketAddr;
}

/// Benchmark the SyncTestSession advance_frame without rollback
///
/// This measures the baseline performance of frame advancement
fn bench_advance_frame_no_rollback(c: &mut Criterion) {
    let mut group = c.benchmark_group("SyncTestSession");

    for num_players in [2, 4].iter() {
        group.bench_with_input(
            BenchmarkId::new("advance_frame_no_rollback", num_players),
            num_players,
            |b, &num_players| {
                // Create session with check_distance=0 (no rollback)
                let mut session: SyncTestSession<BenchConfig> = SessionBuilder::new()
                    .with_num_players(num_players)
                    .unwrap()
                    .with_check_distance(0)
                    .start_synctest_session()
                    .expect("Failed to create session");

                b.iter(|| {
                    // Add inputs for all players
                    for player in 0..num_players {
                        session
                            .add_local_input(
                                PlayerHandle::new(player),
                                BenchInput {
                                    buttons: player as u8,
                                    stick_x: 0,
                                    stick_y: 0,
                                },
                            )
                            .expect("Failed to add input");
                    }

                    // Advance frame and process requests
                    let requests = session.advance_frame().expect("Failed to advance frame");
                    black_box(&requests);

                    // Process requests (minimal work)
                    for request in requests {
                        match request {
                            FortressRequest::AdvanceFrame { inputs } => {
                                black_box(inputs);
                            },
                            FortressRequest::SaveGameState { cell, frame } => {
                                cell.save(frame, Some(BenchState::default()), None);
                            },
                            FortressRequest::LoadGameState { cell, .. } => {
                                black_box(cell.load());
                            },
                            _ => {},
                        }
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark the SyncTestSession advance_frame with rollback
///
/// This measures performance when rollback/resimulation occurs
fn bench_advance_frame_with_rollback(c: &mut Criterion) {
    let mut group = c.benchmark_group("SyncTestSession");

    // check_distance must be < max_prediction (default 8), so test 2, 4, 7
    for check_distance in [2, 4, 7].iter() {
        group.bench_with_input(
            BenchmarkId::new("advance_frame_with_rollback", check_distance),
            check_distance,
            |b, &check_distance| {
                let num_players = 2;

                // Create session with rollback enabled
                let mut session: SyncTestSession<BenchConfig> = SessionBuilder::new()
                    .with_num_players(num_players)
                    .unwrap()
                    .with_check_distance(check_distance)
                    .start_synctest_session()
                    .expect("Failed to create session");

                // State storage for rollback
                let mut states: HashMap<Frame, BenchState> = HashMap::new();
                let mut current_state = BenchState::default();

                // Warm up: advance past check_distance so rollbacks start happening
                for _ in 0..=(check_distance + 2) {
                    for player in 0..num_players {
                        session
                            .add_local_input(
                                PlayerHandle::new(player),
                                BenchInput {
                                    buttons: player as u8,
                                    stick_x: 0,
                                    stick_y: 0,
                                },
                            )
                            .expect("Failed to add input");
                    }

                    let requests = session.advance_frame().expect("Failed to advance frame");
                    for request in requests {
                        match request {
                            FortressRequest::AdvanceFrame { .. } => {
                                current_state.frame += 1;
                            },
                            FortressRequest::SaveGameState { cell, frame } => {
                                states.insert(frame, current_state.clone());
                                cell.save(frame, Some(current_state.clone()), None);
                            },
                            FortressRequest::LoadGameState { cell, frame } => {
                                if let Some(state) = cell.load() {
                                    current_state = state;
                                } else if let Some(state) = states.get(&frame) {
                                    current_state = state.clone();
                                }
                            },
                            _ => {},
                        }
                    }
                }

                b.iter(|| {
                    // Add inputs for all players
                    for player in 0..num_players {
                        session
                            .add_local_input(
                                PlayerHandle::new(player),
                                BenchInput {
                                    buttons: player as u8,
                                    stick_x: 0,
                                    stick_y: 0,
                                },
                            )
                            .expect("Failed to add input");
                    }

                    // Advance frame (will trigger rollback checks)
                    let requests = session.advance_frame().expect("Failed to advance frame");
                    black_box(&requests);

                    // Process all requests
                    for request in requests {
                        match request {
                            FortressRequest::AdvanceFrame { .. } => {
                                current_state.frame += 1;
                            },
                            FortressRequest::SaveGameState { cell, frame } => {
                                states.insert(frame, current_state.clone());
                                cell.save(frame, Some(current_state.clone()), None);
                            },
                            FortressRequest::LoadGameState { cell, frame } => {
                                if let Some(state) = cell.load() {
                                    current_state = state;
                                } else if let Some(state) = states.get(&frame) {
                                    current_state = state.clone();
                                }
                            },
                            _ => {},
                        }
                    }
                });
            },
        );
    }

    group.finish();
}

/// Number of iterations for sub-microsecond benchmarks.
///
/// Sub-10ns operations have high variance due to timer resolution, CPU frequency
/// scaling, and scheduler jitter. By iterating many times within each benchmark
/// sample, we move into the microsecond range where measurements are more stable.
const FAST_BENCH_ITERATIONS: usize = 1000;

/// Benchmark message serialization round trip
///
/// Note: These benchmarks iterate [`FAST_BENCH_ITERATIONS`] times internally to
/// get into microsecond range where measurements are more stable.
fn bench_message_serialization(c: &mut Criterion) {
    use fortress_rollback::network::codec;

    let mut group = c.benchmark_group("Message serialization");

    // Create a sample message with inputs
    let sample_input_bytes = vec![0u8; 12]; // Typical input size

    group.bench_function("round_trip_input_msg", |b| {
        b.iter(|| {
            for _ in 0..FAST_BENCH_ITERATIONS {
                // Serialize
                let bytes = codec::encode(&sample_input_bytes).expect("serialize");
                black_box(&bytes);

                // Deserialize
                let _decoded: Vec<u8> = codec::decode_value(&bytes).expect("deserialize");
            }
        });
    });

    // Benchmark BenchInput serialization (what actually gets sent)
    group.bench_function("input_serialize", |b| {
        let input = BenchInput {
            buttons: 0xFF,
            stick_x: 127,
            stick_y: -128,
        };
        b.iter(|| {
            for _ in 0..FAST_BENCH_ITERATIONS {
                let bytes = codec::encode(black_box(&input)).expect("serialize");
                black_box(bytes);
            }
        });
    });

    group.bench_function("input_deserialize", |b| {
        let input = BenchInput {
            buttons: 0xFF,
            stick_x: 127,
            stick_y: -128,
        };
        let bytes = codec::encode(&input).expect("serialize");
        b.iter(|| {
            for _ in 0..FAST_BENCH_ITERATIONS {
                let decoded: BenchInput =
                    codec::decode_value(black_box(&bytes)).expect("deserialize");
                black_box(decoded);
            }
        });
    });

    // Benchmark encode_into vs encode (allocation comparison)
    group.bench_function("input_encode_into_buffer", |b| {
        let input = BenchInput {
            buttons: 0xFF,
            stick_x: 127,
            stick_y: -128,
        };
        let mut buffer = [0u8; 64];
        b.iter(|| {
            for _ in 0..FAST_BENCH_ITERATIONS {
                let len = codec::encode_into(black_box(&input), &mut buffer).expect("serialize");
                black_box(len);
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_advance_frame_no_rollback,
    bench_advance_frame_with_rollback,
    bench_message_serialization,
);
criterion_main!(benches);
