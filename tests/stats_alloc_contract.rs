//! Pins the `stats_alloc` realloc contract that backs the network soak oracle.
//!
//! This is intentionally the only test in its own process. The measured region
//! is the process-global allocator, so a concurrently running test that keeps
//! more than the noise allowance live would otherwise perturb the net-live
//! bound.

#![forbid(unsafe_code)]
#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]

use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::alloc::System;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

/// Net change in outstanding heap bytes over one measurement window.
///
/// Mirrors the oracle in `tests/network/soak.rs`; see that module for the
/// double-count rationale.
fn net_live_heap_bytes(stats: stats_alloc::Stats) -> i64 {
    let allocated = i64::try_from(stats.bytes_allocated).unwrap_or(i64::MAX);
    let deallocated = i64::try_from(stats.bytes_deallocated).unwrap_or(i64::MAX);
    allocated.saturating_sub(deallocated)
}

#[test]
fn stats_alloc_realloc_growth_is_already_embedded_in_allocated_bytes() {
    // Pins stats_alloc's realloc contract end-to-end for one buffer: an exact
    // initial allocation followed by one exact growth must leave
    // net_live_heap_bytes equal to the final outstanding size even though
    // bytes_reallocated also carries the growth delta. If the deltas were not
    // embedded -- or were added again here -- this would report 3 MiB instead
    // of 2 MiB.
    //
    // The region measures the process-global allocator, so the harness may
    // allocate or reallocate a few dozen bytes on another thread while this
    // test runs (observed under QEMU). Size the growth far above that noise
    // and allow a small slack on the net-live assertion. The raw allocation
    // counts are global and cannot be pinned exactly; only the
    // growth-embedding and double-count bounds matter here.
    const INITIAL_BYTES: usize = 1_024 * 1_024;
    const GROWN_BYTES: usize = 2 * INITIAL_BYTES;
    const HARNESS_NOISE_BYTES: i64 = 64 * 1_024;

    let region = Region::new(GLOBAL);
    let mut buffer: Vec<u8> = Vec::new();
    buffer.reserve_exact(INITIAL_BYTES);
    buffer.reserve_exact(GROWN_BYTES);
    assert_eq!(buffer.len(), 0);
    assert_eq!(buffer.capacity(), GROWN_BYTES);
    let stats = region.change();
    assert!(
        stats.reallocations >= 1,
        "the growth must be observed as a reallocation: {stats:?}"
    );
    assert!(
        stats.bytes_allocated >= GROWN_BYTES,
        "realloc growth must be embedded in bytes_allocated: {stats:?}"
    );
    let net_live = net_live_heap_bytes(stats);
    let grown = i64::try_from(GROWN_BYTES).unwrap_or(i64::MAX);
    assert!(
        (net_live - grown).abs() <= HARNESS_NOISE_BYTES,
        "realloc growth must count exactly once toward outstanding bytes: {stats:?}"
    );
    let doubled = grown + i64::try_from(INITIAL_BYTES).unwrap_or(i64::MAX);
    assert!(
        net_live < doubled - HARNESS_NOISE_BYTES,
        "realloc growth must not be counted twice: {stats:?}"
    );
}
