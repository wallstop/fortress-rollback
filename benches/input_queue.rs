//! Benchmarks for InputQueue operations
//!
//! Run with: cargo bench --bench input_queue
//!
//! Note: InputQueue is internal, so we benchmark through the public session APIs.
//! For direct InputQueue benchmarks, the module would need to be made public.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use fortress_rollback::Frame;

fn bench_frame_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("Frame");

    group.bench_function("new", |b| {
        b.iter(|| Frame::new(black_box(42)));
    });

    group.bench_function("is_null", |b| {
        let frame = Frame::new(42);
        b.iter(|| black_box(frame).is_null());
    });

    group.bench_function("is_valid", |b| {
        let frame = Frame::new(42);
        b.iter(|| black_box(frame).is_valid());
    });

    group.finish();
}

fn bench_frame_arithmetic(c: &mut Criterion) {
    let mut group = c.benchmark_group("Frame arithmetic");

    for delta in [1i32, 10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::new("add", delta), delta, |b, &delta| {
            let frame = Frame::new(1000);
            b.iter(|| black_box(frame) + black_box(delta));
        });
    }

    group.finish();
}

criterion_group!(benches, bench_frame_operations, bench_frame_arithmetic);
criterion_main!(benches);
