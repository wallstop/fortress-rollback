//! Benchmarks for compression operations
//!
//! Run with: cargo bench --bench compression
//!
//! This benchmark suite tests the compression pipeline used for network
//! transmission of game inputs, with realistic game input patterns.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use fortress_rollback::rle::{decode, encode};
use std::hint::black_box;

/// Simulate idle player inputs (same input repeated)
fn idle_inputs(frames: usize, input_size: usize) -> Vec<Vec<u8>> {
    let input = vec![0u8; input_size];
    vec![input; frames]
}

/// Simulate active player with periodic button presses
fn active_inputs(frames: usize, input_size: usize) -> Vec<Vec<u8>> {
    (0..frames)
        .map(|i| {
            let mut input = vec![0u8; input_size];
            // Button press every 5 frames
            if i % 5 == 0 {
                input[0] = 1 << (i % 8);
            }
            input
        })
        .collect()
}

/// Simulate fighting game with rapid inputs
fn fighting_game_inputs(frames: usize, input_size: usize) -> Vec<Vec<u8>> {
    (0..frames)
        .map(|i| {
            let mut input = vec![0u8; input_size];
            // 50% of frames have button changes
            if i % 2 == 0 {
                input[0] = ((i * 7) % 256) as u8;
            }
            // Occasional directional input
            if i % 3 == 0 && input_size > 1 {
                input[1] = ((i * 13) % 16) as u8;
            }
            input
        })
        .collect()
}

/// Simulate analog stick movement
fn analog_inputs(frames: usize, input_size: usize) -> Vec<Vec<u8>> {
    (0..frames)
        .map(|i| {
            let mut input = vec![0u8; input_size];
            // Smooth analog movement
            let angle = (i as f32 * 0.1).sin();
            if input_size >= 2 {
                input[0] = ((angle * 127.0) as i8) as u8;
                input[1] = ((angle.cos() * 127.0) as i8) as u8;
            }
            input
        })
        .collect()
}

/// XOR delta encode inputs against a reference (simulates the compression pipeline)
fn delta_encode(reference: &[u8], inputs: &[Vec<u8>]) -> Vec<u8> {
    let mut result = Vec::with_capacity(reference.len() * inputs.len());
    for input in inputs {
        for (r, i) in reference.iter().zip(input.iter()) {
            result.push(r ^ i);
        }
    }
    result
}

/// XOR delta decode
fn delta_decode(reference: &[u8], encoded: &[u8]) -> Vec<Vec<u8>> {
    encoded
        .chunks(reference.len())
        .map(|chunk| {
            reference
                .iter()
                .zip(chunk.iter())
                .map(|(r, e)| r ^ e)
                .collect()
        })
        .collect()
}

fn bench_rle_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("RLE encode");

    for size in [4, 8, 16, 64, 256] {
        let data = vec![0u8; size];
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("zeros", size), &data, |b, data| {
            b.iter(|| encode(black_box(data)));
        });
    }

    for size in [4, 8, 16, 64, 256] {
        let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("random", size), &data, |b, data| {
            b.iter(|| encode(black_box(data)));
        });
    }

    group.finish();
}

fn bench_rle_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("RLE decode");

    for size in [4, 8, 16, 64, 256] {
        let data = vec![0u8; size];
        let encoded = encode(&data);
        group.throughput(Throughput::Bytes(encoded.len() as u64));
        group.bench_with_input(BenchmarkId::new("zeros", size), &encoded, |b, encoded| {
            b.iter(|| decode(black_box(encoded)));
        });
    }

    group.finish();
}

fn bench_compression_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("Compression pipeline");

    let input_sizes = [4, 8]; // Typical game input sizes
    let frame_counts = [8, 16, 32]; // Typical pending input counts

    for &input_size in &input_sizes {
        for &frames in &frame_counts {
            let reference = vec![0u8; input_size];

            // Idle scenario
            let idle = idle_inputs(frames, input_size);
            let total_bytes = (input_size * frames) as u64;
            group.throughput(Throughput::Bytes(total_bytes));

            group.bench_with_input(
                BenchmarkId::new(format!("idle_encode_{}b", input_size), frames),
                &idle,
                |b, inputs| {
                    b.iter(|| {
                        let delta = delta_encode(&reference, black_box(inputs));
                        encode(black_box(&delta))
                    });
                },
            );

            // Active scenario
            let active = active_inputs(frames, input_size);
            group.bench_with_input(
                BenchmarkId::new(format!("active_encode_{}b", input_size), frames),
                &active,
                |b, inputs| {
                    b.iter(|| {
                        let delta = delta_encode(&reference, black_box(inputs));
                        encode(black_box(&delta))
                    });
                },
            );

            // Fighting game scenario
            let fighting = fighting_game_inputs(frames, input_size);
            group.bench_with_input(
                BenchmarkId::new(format!("fighting_encode_{}b", input_size), frames),
                &fighting,
                |b, inputs| {
                    b.iter(|| {
                        let delta = delta_encode(&reference, black_box(inputs));
                        encode(black_box(&delta))
                    });
                },
            );
        }
    }

    group.finish();
}

fn bench_compression_ratio(c: &mut Criterion) {
    let mut group = c.benchmark_group("Compression ratio analysis");

    let input_size = 4;
    let frames = 16;
    let reference = vec![0u8; input_size];

    let scenarios: [(&str, Vec<Vec<u8>>); 4] = [
        ("idle", idle_inputs(frames, input_size)),
        ("active", active_inputs(frames, input_size)),
        ("fighting", fighting_game_inputs(frames, input_size)),
        ("analog", analog_inputs(frames, input_size)),
    ];

    for (name, inputs) in &scenarios {
        let original_size = input_size * frames;
        let delta = delta_encode(&reference, inputs);
        let compressed = encode(&delta);

        let ratio = compressed.len() as f64 / original_size as f64;
        println!(
            "{}: {} -> {} bytes (ratio: {:.2})",
            name,
            original_size,
            compressed.len(),
            ratio
        );

        // Benchmark full roundtrip
        group.bench_function(BenchmarkId::new("roundtrip", *name), |b| {
            b.iter(|| {
                let delta = delta_encode(&reference, black_box(inputs));
                let compressed = encode(black_box(&delta));
                let decompressed = decode(black_box(&compressed)).unwrap();
                delta_decode(&reference, &decompressed)
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_rle_encode,
    bench_rle_decode,
    bench_compression_pipeline,
    bench_compression_ratio
);
criterion_main!(benches);
