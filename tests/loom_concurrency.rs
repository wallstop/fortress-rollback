//! Loom-based concurrency tests for Fortress Rollback.
//!
//! These tests use loom to exhaustively check all possible thread interleavings
//! for concurrent operations in the library.
//!
//! # Running Loom Tests
//!
//! Due to dependency conflicts (some deps like tokio/hyper disable modules under
//! `cfg(loom)`), loom tests should be run in a separate workspace or with a
//! minimal dependency set.
//!
//! ## Option 1: Run with only the loom test (recommended)
//!
//! Create a separate crate for loom tests that only depends on fortress-rollback
//! and loom, without the heavy dev-dependencies.
//!
//! ## Option 2: Use a workspace member
//!
//! Add a `loom-tests` workspace member that only includes loom as a dependency.
//!
//! ## Option 3: Feature-gated internal tests
//!
//! Put loom tests inside `src/` behind `#[cfg(all(test, loom))]` so they don't
//! pull in dev-dependencies.
//!
//! # Note on Current Limitations
//!
//! This file demonstrates the loom testing patterns but cannot be run with
//! `cargo test --test loom_concurrency` because other dev-dependencies
//! (macroquad -> hyper-util -> tokio) fail to compile under `cfg(loom)`.
//!
//! The solution is to either:
//! 1. Create a separate `loom-tests/` crate in the workspace
//! 2. Put these tests in `src/sync.rs` under `#[cfg(all(test, loom))]`

// Only compile this file under loom
#![cfg(loom)]

use loom::sync::{Arc, Mutex};
use loom::thread;

/// Test that demonstrates basic loom usage pattern.
///
/// This test verifies that concurrent increments to a shared counter
/// always result in the correct final value.
#[test]
fn test_counter_increment_correctness() {
    loom::model(|| {
        let counter = Arc::new(Mutex::new(0));
        let counter1 = counter.clone();
        let counter2 = counter.clone();

        let t1 = thread::spawn(move || {
            let mut lock = counter1.lock().unwrap();
            *lock += 1;
        });

        let t2 = thread::spawn(move || {
            let mut lock = counter2.lock().unwrap();
            *lock += 1;
        });

        t1.join().unwrap();
        t2.join().unwrap();

        // Regardless of interleaving, final value must be 2
        let final_value = *counter.lock().unwrap();
        assert_eq!(final_value, 2, "Counter should be 2 after two increments");
    });
}

/// Test that demonstrates checking for data races.
///
/// This pattern is useful for verifying that your concurrent data structure
/// maintains invariants across all possible interleavings.
#[test]
fn test_invariant_preservation() {
    loom::model(|| {
        // Invariant: vec.len() should always equal counter
        let data = Arc::new(Mutex::new((Vec::<i32>::new(), 0usize)));
        let data1 = data.clone();

        let t1 = thread::spawn(move || {
            let mut guard = data1.lock().unwrap();
            guard.0.push(42);
            guard.1 += 1;

            // Invariant check
            assert_eq!(guard.0.len(), guard.1, "Invariant violated!");
        });

        {
            let mut guard = data.lock().unwrap();
            guard.0.push(99);
            guard.1 += 1;

            // Invariant check
            assert_eq!(guard.0.len(), guard.1, "Invariant violated!");
        }

        t1.join().unwrap();

        // Final invariant check
        let guard = data.lock().unwrap();
        assert_eq!(guard.0.len(), guard.1, "Final invariant check failed!");
        assert_eq!(guard.0.len(), 2, "Should have exactly 2 elements");
    });
}

/// Example of testing a producer-consumer pattern.
///
/// This demonstrates how to test more complex concurrent algorithms.
#[test]
fn test_producer_consumer() {
    loom::model(|| {
        let queue = Arc::new(Mutex::new(Vec::new()));
        let queue_producer = queue.clone();
        let queue_consumer = queue.clone();

        // Producer
        let producer = thread::spawn(move || {
            for i in 0..2 {
                let mut q = queue_producer.lock().unwrap();
                q.push(i);
            }
        });

        // Consumer (just counts items)
        let consumer = thread::spawn(move || {
            let mut total_seen = 0;
            // Try to consume a few times
            for _ in 0..3 {
                let q = queue_consumer.lock().unwrap();
                total_seen = total_seen.max(q.len());
            }
            total_seen
        });

        producer.join().unwrap();
        let seen = consumer.join().unwrap();

        // Consumer may have seen 0, 1, or 2 items depending on interleaving
        assert!(seen <= 2, "Consumer saw more items than produced");
    });
}

/// Test demonstrating preemption bounds for large state spaces.
///
/// Complex tests may need preemption bounds to complete in reasonable time.
#[test]
fn test_with_preemption_bound() {
    let mut builder = loom::model::Builder::new();
    // Limit to 2 preemptions - still catches most bugs but runs faster
    builder.preemption_bound = Some(2);

    builder.check(|| {
        let data = Arc::new(Mutex::new(0));
        let handles: Vec<_> = (0..3)
            .map(|_| {
                let data = data.clone();
                thread::spawn(move || {
                    let mut lock = data.lock().unwrap();
                    *lock += 1;
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(*data.lock().unwrap(), 3);
    });
}

// =============================================================================
// TEMPLATE FOR TESTING GameStateCell (once integrated)
// =============================================================================

/*
/// Once the crate uses loom-compatible primitives, this test would verify
/// GameStateCell thread safety.
#[test]
fn test_game_state_cell_concurrent_save_load() {
    loom::model(|| {
        use fortress_rollback::sync::GameStateCell;
        use fortress_rollback::Frame;

        let cell = Arc::new(GameStateCell::<u64>::default());
        let cell1 = cell.clone();
        let cell2 = cell.clone();

        // Thread 1: Save state
        let t1 = thread::spawn(move || {
            cell1.save(Frame::new(1), Some(42), Some(0));
        });

        // Thread 2: Try to load
        let t2 = thread::spawn(move || {
            // Load might see old or new state depending on interleaving
            let _ = cell2.load();
        });

        t1.join().unwrap();
        t2.join().unwrap();

        // After both threads complete, state should be saved
        let loaded = cell.load();
        assert_eq!(loaded, Some(42));
    });
}
*/
