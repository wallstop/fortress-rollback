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
    Config, FortressRequest, Frame, Message, NonBlockingSocket, PlayerHandle, PlayerType,
    SessionBuilder, SyncTestSession,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hint::black_box;
use std::net::SocketAddr;
use std::time::Duration;

/// Simple test input type for benchmarking
#[derive(Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Debug)]
struct BenchInput {
    buttons: u8,
    stick_x: i8,
    stick_y: i8,
}

/// Simple test state type for benchmarking
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "hot-join", derive(Serialize, Deserialize))]
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

/// Socket used by accessor-only benchmarks that never poll or exchange data.
struct BenchSocket;

impl NonBlockingSocket<SocketAddr> for BenchSocket {
    fn send_to(&mut self, _msg: &Message, _addr: &SocketAddr) {}

    fn receive_all_messages(&mut self) -> Vec<(SocketAddr, Message)> {
        Vec::new()
    }
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
            // Intentional Session 117 gate drill: this temporary delay must
            // trip the 150% hard regression threshold on the draft drill PR.
            std::thread::sleep(Duration::from_millis(1));
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

/// Benchmarks the pull-based metrics snapshot and packet-hot-path wire-length
/// arithmetic added by M2. Both accessor calls are allocation-free; this does
/// not measure the separate always-on counter-update overhead.
fn bench_metrics_and_wire_length(c: &mut Criterion) {
    use fortress_rollback::network::codec;

    let remote_addr: SocketAddr = ([127, 0, 0, 1], 7_001).into();
    let session = SessionBuilder::<BenchConfig>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))
        .expect("add local player")
        .add_player(PlayerType::Remote(remote_addr), PlayerHandle::new(1))
        .expect("add remote player")
        .start_p2p_session(BenchSocket)
        .expect("create P2P session");

    let mut metrics_group = c.benchmark_group("P2PSession");
    metrics_group.bench_function("metrics", |b| {
        b.iter(|| black_box(black_box(&session).metrics()));
    });
    metrics_group.finish();

    // Decode one representative 2-player Input packet outside the timed loop.
    // This keeps the benchmark on the variable-length arithmetic hot path
    // without exposing protocol internals solely for Criterion.
    let mut wire = Vec::new();
    wire.extend_from_slice(&[0xF5, 0x52]); // header sentinel
    wire.push(fortress_rollback::PROTOCOL_VERSION);
    wire.push(0); // header flags
    wire.extend_from_slice(&1_u32.to_le_bytes()); // header connection ID
    wire.extend_from_slice(&2_u32.to_le_bytes()); // MessageBody::Input
    wire.extend_from_slice(&2_u64.to_le_bytes()); // peer_connect_status.len()
    for _ in 0..2 {
        wire.push(0); // disconnected = false
        wire.extend_from_slice(&(-1_i32).to_le_bytes()); // last_frame
        wire.extend_from_slice(&0_u16.to_le_bytes()); // epoch
    }
    wire.extend_from_slice(&0_i32.to_le_bytes()); // start_frame
    wire.extend_from_slice(&(-1_i32).to_le_bytes()); // ack_frame
    wire.extend_from_slice(&96_u64.to_le_bytes()); // compressed input bytes
    wire.extend(0_u8..96);
    let (input_message, consumed) = codec::decode_message(&wire).expect("decode benchmark Input");
    assert_eq!(consumed, wire.len());

    let mut wire_group = c.benchmark_group("Message");
    wire_group.bench_function("encoded_len", |b| {
        b.iter(|| {
            black_box(fortress_rollback::__internal::message_metadata(black_box(&input_message)).0)
        });
    });
    wire_group.finish();
}

criterion_group!(
    benches,
    bench_advance_frame_no_rollback,
    bench_advance_frame_with_rollback,
    bench_message_serialization,
    bench_metrics_and_wire_length,
);
criterion_main!(benches);
