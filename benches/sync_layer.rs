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

use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

// Placeholder benchmark - will be expanded once SyncLayer API is finalized
fn bench_sync_layer_placeholder(c: &mut Criterion) {
    c.bench_function("sync_layer_noop", |b| {
        b.iter(|| {
            // Placeholder - add real SyncLayer benchmarks
            black_box(42)
        });
    });
}

criterion_group!(benches, bench_sync_layer_placeholder);
criterion_main!(benches);
