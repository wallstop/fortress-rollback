//! Benchmarks for SyncLayer operations
//!
//! Run with: cargo bench --bench sync_layer

// Allow benchmark-specific patterns
#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use fortress_rollback::{__internal::SyncLayer, Config, FortressRequest};
use serde::{Deserialize, Serialize};
use std::hint::black_box;
use std::net::SocketAddr;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct BenchInput(u8);

struct BenchConfig;

impl Config for BenchConfig {
    type Input = BenchInput;
    type State = u32;
    type Address = SocketAddr;
}

/// Measures a representative microsecond-scale unit of synchronization work.
///
/// Setup is excluded from the timed region. Each sample saves state and advances
/// the rollback ring for 256 frames, keeping the measured batch well above the
/// shared runner's timer-resolution floor.
fn bench_sync_layer_frame_sequence(c: &mut Criterion) {
    c.bench_function("SyncLayer/256_frame_save_advance", |b| {
        b.iter_batched(
            || SyncLayer::<BenchConfig>::new(4, 8),
            |mut sync_layer| {
                for _ in 0..256 {
                    if let FortressRequest::SaveGameState { cell, frame } =
                        sync_layer.save_current_state()
                    {
                        black_box(cell.save(frame, Some(frame.as_i32() as u32), None));
                    }
                    sync_layer.advance_frame();
                }
                black_box(sync_layer.current_frame());
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, bench_sync_layer_frame_sequence);
criterion_main!(benches);
