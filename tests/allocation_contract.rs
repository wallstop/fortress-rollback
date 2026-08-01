//! Deterministic heap-allocation contracts for documented and warmed hot paths.
//!
//! This integration test is intentionally a single test in its own process.
//! The process-local global allocator therefore cannot observe allocations from
//! another test, and every measured operation excludes construction and warm-up.

#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::unwrap_used)]
#![forbid(unsafe_code)]

use std::alloc::System;
use std::hint::black_box;
use std::net::SocketAddr;

use fortress_rollback::network::codec;
use fortress_rollback::{Config, PlayerHandle, SessionBuilder, SyncTestSession};
use stats_alloc::{Region, Stats, StatsAlloc, INSTRUMENTED_SYSTEM};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

struct AllocationConfig;

impl Config for AllocationConfig {
    type Input = u32;
    type State = u32;
    type Address = SocketAddr;
}

fn measure<T>(operation: impl FnOnce() -> T) -> (T, Stats) {
    let region = Region::new(GLOBAL);
    let result = operation();
    let stats = region.change();
    (result, stats)
}

#[track_caller]
fn assert_zero_allocations(label: &str, stats: Stats) {
    assert_eq!(stats.allocations, 0, "{label} allocation stats: {stats:?}");
    assert_eq!(
        stats.reallocations, 0,
        "{label} allocation stats: {stats:?}"
    );
    assert_eq!(
        stats.bytes_allocated, 0,
        "{label} allocation stats: {stats:?}"
    );
}

#[track_caller]
fn assert_allocation_ceiling(
    label: &str,
    stats: Stats,
    maximum_operations: usize,
    maximum_bytes: usize,
) {
    let operations = stats.allocations.saturating_add(stats.reallocations);
    assert!(
        operations <= maximum_operations,
        "{label} used {operations} allocation operations (ceiling {maximum_operations}): {stats:?}"
    );
    assert!(
        stats.bytes_allocated <= maximum_bytes,
        "{label} allocated {} bytes (ceiling {maximum_bytes}): {stats:?}",
        stats.bytes_allocated
    );
}

fn warmed_synctest_frame_stats(num_players: usize) -> Stats {
    let mut session: SyncTestSession<AllocationConfig> = SessionBuilder::new()
        .with_num_players(num_players)
        .expect("configure allocation-contract player count")
        .with_check_distance(0)
        .start_synctest_session()
        .expect("construct allocation-contract sync test");

    for player in 0..num_players {
        session
            .add_local_input(
                PlayerHandle::new(player),
                u32::try_from(player).expect("allocation-contract handle fits u32"),
            )
            .expect("add warm-up input");
    }
    black_box(session.advance_frame().expect("advance warm-up frame"));

    let (requests, stats) = measure(|| {
        for player in 0..num_players {
            session
                .add_local_input(
                    PlayerHandle::new(player),
                    u32::try_from(player).expect("allocation-contract handle fits u32"),
                )
                .expect("add measured input");
        }
        black_box(session.advance_frame().expect("advance measured frame"))
    });
    black_box(requests);
    stats
}

/// Allocation contracts are measured together so the instrumented allocator
/// cannot observe concurrently running tests in this integration-test process.
#[test]
fn warmed_hot_paths_obey_allocation_contracts() {
    // Counter sensitivity: this must detect one known allocation or every
    // zero-allocation assertion below would be capable of passing vacuously.
    let (allocation, control) = measure(|| vec![0_u8; 4_096]);
    black_box(&allocation);
    assert_eq!(
        control.allocations, 1,
        "known-allocation control: {control:?}"
    );
    assert_eq!(
        control.reallocations, 0,
        "known-allocation control: {control:?}"
    );
    assert_eq!(
        control.bytes_allocated, 4_096,
        "known-allocation control: {control:?}"
    );
    drop(allocation);

    // The zero-allocation helper deliberately permits deallocation: its
    // contract is about newly allocated memory, not every allocator event.
    let deallocation = vec![0_u8; 4_096];
    let ((), deallocation_control) = measure(|| drop(deallocation));
    assert_eq!(
        deallocation_control.deallocations, 1,
        "known-deallocation control: {deallocation_control:?}"
    );
    assert_zero_allocations("known-deallocation control", deallocation_control);

    // `encode_into` publicly documents that a caller-provided buffer avoids
    // allocation on the successful path.
    let mut buffer = [0_u8; 64];
    let (encoded_len, encode_stats) =
        measure(|| codec::encode_into(black_box(&42_u32), black_box(&mut buffer)));
    assert_eq!(encoded_len.expect("encode into fixed buffer"), 4);
    assert_zero_allocations("codec::encode_into", encode_stats);

    // Handle iterators publicly document zero allocation. The collecting
    // convenience API is allowed one bounded spill above HandleVec's inline
    // capacity of eight.
    let session: SyncTestSession<AllocationConfig> = SessionBuilder::new()
        .with_num_players(16)
        .expect("configure sixteen-player allocation contract")
        .start_synctest_session()
        .expect("construct sixteen-player allocation contract");
    let (handle_sum, iterator_stats) = measure(|| {
        black_box(&session)
            .local_player_handles_iter()
            .map(PlayerHandle::as_usize)
            .sum::<usize>()
    });
    assert_eq!(handle_sum, 120);
    assert_zero_allocations("local_player_handles_iter", iterator_stats);

    let (handles, collecting_stats) = measure(|| session.local_player_handles());
    assert_eq!(handles.len(), 16);
    assert_allocation_ceiling("sixteen-player HandleVec spill", collecting_stats, 1, 256);
    drop(handles);

    // The ceilings are based on the red-run observations (1/192 B, 1/192 B,
    // and 4/800 B). Byte ceilings leave bounded layout headroom while operation
    // counts stay tight enough to expose new per-player allocation growth.
    for (players, operation_ceiling, byte_ceiling) in [(2, 1, 256), (4, 1, 256), (16, 4, 1_024)] {
        let first = warmed_synctest_frame_stats(players);
        for repetition in 1..3 {
            assert_eq!(
                warmed_synctest_frame_stats(players),
                first,
                "warmed {players}-player sync-test frame repetition {repetition}"
            );
        }
        assert_allocation_ceiling(
            &format!("warmed {players}-player sync-test frame"),
            first,
            operation_ceiling,
            byte_ceiling,
        );
    }
}
