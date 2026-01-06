//! Loom tests for GameStateCell thread safety.
//!
//! These tests verify that GameStateCell operations are thread-safe by
//! exhaustively exploring all possible interleavings using loom.
//!
//! Run with:
//! ```bash
//! cd loom-tests
//! RUSTFLAGS="--cfg loom" cargo test --release
//! ```

#![cfg(loom)]

use fortress_rollback::{Frame, GameStateCell};
use loom::sync::Arc;
use loom::thread;

/// Test concurrent save operations from multiple threads.
///
/// Verifies that concurrent saves don't corrupt the cell state and
/// that the final state is from one of the saving threads.
#[test]
fn test_concurrent_saves() {
    loom::model(|| {
        let cell: Arc<GameStateCell<u64>> = Arc::new(GameStateCell::default());
        let cell1 = cell.clone();
        let cell2 = cell.clone();

        let t1 = thread::spawn(move || {
            cell1.save(Frame::new(1), Some(100), Some(0xAAAA));
        });

        let t2 = thread::spawn(move || {
            cell2.save(Frame::new(2), Some(200), Some(0xBBBB));
        });

        t1.join().unwrap();
        t2.join().unwrap();

        // After both saves complete, cell should contain one of the values
        let loaded = cell.load();
        assert!(
            loaded == Some(100) || loaded == Some(200),
            "Cell should contain either 100 or 200, got {:?}",
            loaded
        );
    });
}

/// Test concurrent save and load operations.
///
/// Verifies that load always sees a consistent state - either the
/// initial empty state or a fully saved state.
#[test]
fn test_save_load_consistency() {
    loom::model(|| {
        let cell: Arc<GameStateCell<u64>> = Arc::new(GameStateCell::default());
        let cell_writer = cell.clone();
        let cell_reader = cell.clone();

        // Writer thread saves state
        let writer = thread::spawn(move || {
            cell_writer.save(Frame::new(1), Some(42), Some(0xDEADBEEF));
        });

        // Reader thread loads state
        let reader = thread::spawn(move || cell_reader.load());

        writer.join().unwrap();
        let loaded = reader.join().unwrap();

        // Reader should see either None (initial) or Some(42) (after save)
        // Never a partial/corrupted state
        assert!(
            loaded.is_none() || loaded == Some(42),
            "Loaded unexpected value: {:?}",
            loaded
        );
    });
}

/// Test multiple readers with single writer.
///
/// Verifies that multiple concurrent readers don't interfere with
/// each other or with the writer.
#[test]
fn test_multiple_readers_single_writer() {
    loom::model(|| {
        let cell: Arc<GameStateCell<u64>> = Arc::new(GameStateCell::default());

        // Pre-save initial value
        cell.save(Frame::new(0), Some(0), Some(0));

        let cell1 = cell.clone();
        let cell2 = cell.clone();
        let cell3 = cell.clone();

        // Writer updates the value
        let writer = thread::spawn(move || {
            cell1.save(Frame::new(1), Some(999), Some(0xFFFF));
        });

        // Multiple readers
        let reader1 = thread::spawn(move || cell2.load());
        let reader2 = thread::spawn(move || cell3.load());

        writer.join().unwrap();
        let r1 = reader1.join().unwrap();
        let r2 = reader2.join().unwrap();

        // Each reader should see either 0 or 999
        for (i, r) in [(1, r1), (2, r2)] {
            assert!(
                r == Some(0) || r == Some(999),
                "Reader {} saw unexpected value: {:?}",
                i,
                r
            );
        }
    });
}

/// Test frame advancement pattern.
///
/// Simulates the typical save-advance-save pattern used during rollback.
#[test]
fn test_frame_advancement_pattern() {
    loom::model(|| {
        let cell: Arc<GameStateCell<u64>> = Arc::new(GameStateCell::default());
        let cell1 = cell.clone();
        let cell2 = cell.clone();

        // Thread 1: saves frame 1
        let t1 = thread::spawn(move || {
            cell1.save(Frame::new(1), Some(10), Some(1));
        });

        // Thread 2: saves frame 2
        let t2 = thread::spawn(move || {
            cell2.save(Frame::new(2), Some(20), Some(2));
        });

        t1.join().unwrap();
        t2.join().unwrap();

        // Final state should be consistent
        let loaded = cell.load();
        assert!(
            loaded == Some(10) || loaded == Some(20),
            "Cell should contain either 10 or 20, got {:?}",
            loaded
        );
    });
}

/// Test with preemption bound for larger state spaces.
///
/// Uses bounded model checking to verify correctness with more threads.
#[test]
fn test_concurrent_access_bounded() {
    let mut builder = loom::model::Builder::new();
    builder.preemption_bound = Some(2);

    builder.check(|| {
        let cell: Arc<GameStateCell<u64>> = Arc::new(GameStateCell::default());

        let handles: Vec<_> = (0..3)
            .map(|i| {
                let c = cell.clone();
                thread::spawn(move || {
                    c.save(Frame::new(i as i32), Some(i as u64 * 100), Some(i as u128));
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        // After all saves, cell should contain one of the values
        let loaded = cell.load();
        assert!(
            loaded == Some(0) || loaded == Some(100) || loaded == Some(200),
            "Unexpected value: {:?}",
            loaded
        );
    });
}
