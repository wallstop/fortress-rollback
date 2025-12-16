//! Loom tests for SavedStates thread safety.
//!
//! These tests verify that SavedStates operations are thread-safe by
//! exhaustively exploring all possible interleavings using loom.
//!
//! Run with:
//! ```bash
//! cd loom-tests
//! RUSTFLAGS="--cfg loom" cargo test --release
//! ```

#![cfg(loom)]

use fortress_rollback::__internal::SavedStates;
use fortress_rollback::{Frame, GameStateCell};
use loom::sync::Arc;
use loom::thread;

/// Test concurrent access to different SavedStates cells.
///
/// Verifies that accessing different cells concurrently doesn't cause issues.
/// SavedStates uses get_cell with frame modulo, so this tests the cell array access.
#[test]
fn test_saved_states_concurrent_cell_access() {
    loom::model(|| {
        // Create SavedStates with max_pred=2 (3 cells total)
        let states: SavedStates<u64> = SavedStates::new(2);

        // Get cells for different frames (different slots)
        let cell0 = states.get_cell(Frame::new(0)).unwrap();
        let cell1 = states.get_cell(Frame::new(1)).unwrap();
        let cell2 = states.get_cell(Frame::new(2)).unwrap();

        // Each cell is independent, test concurrent saves
        let cell0_clone = cell0.clone();
        let cell1_clone = cell1.clone();
        let cell2_clone = cell2.clone();

        let t1 = thread::spawn(move || {
            cell0_clone.save(Frame::new(0), Some(100), Some(0xAAAA));
        });

        let t2 = thread::spawn(move || {
            cell1_clone.save(Frame::new(1), Some(200), Some(0xBBBB));
        });

        let t3 = thread::spawn(move || {
            cell2_clone.save(Frame::new(2), Some(300), Some(0xCCCC));
        });

        t1.join().unwrap();
        t2.join().unwrap();
        t3.join().unwrap();

        // Each cell should have its respective value
        assert_eq!(cell0.load(), Some(100));
        assert_eq!(cell1.load(), Some(200));
        assert_eq!(cell2.load(), Some(300));
    });
}

/// Test that wrapping frame access (via modulo) is consistent.
///
/// Frame 3 with max_pred=2 wraps to slot 0 (3 % 3 = 0).
#[test]
fn test_saved_states_frame_wrapping() {
    loom::model(|| {
        let states: SavedStates<u64> = SavedStates::new(2); // 3 cells

        // Frame 0 and Frame 3 map to the same cell (slot 0)
        let cell_frame0 = states.get_cell(Frame::new(0)).unwrap();
        let cell_frame3 = states.get_cell(Frame::new(3)).unwrap();

        // They should be the same cell (Arc pointing to same data)
        let cell0_for_save = cell_frame0.clone();
        let cell3_for_load = cell_frame3.clone();

        let t1 = thread::spawn(move || {
            cell0_for_save.save(Frame::new(0), Some(42), Some(0x1234));
        });

        t1.join().unwrap();

        // Both references should see the saved value
        let loaded_via_0 = cell_frame0.load();
        let loaded_via_3 = cell3_for_load.load();

        assert_eq!(loaded_via_0, Some(42));
        assert_eq!(loaded_via_3, Some(42));
    });
}

/// Test concurrent overwrite of the same cell slot.
///
/// When two frames map to the same slot, concurrent writes should
/// result in one of the values, not corruption.
#[test]
fn test_saved_states_concurrent_overwrite() {
    loom::model(|| {
        let states: SavedStates<u64> = SavedStates::new(2); // 3 cells

        // Both map to slot 0
        let cell_frame0 = states.get_cell(Frame::new(0)).unwrap();
        let cell_frame3 = states.get_cell(Frame::new(3)).unwrap();

        let cell_0_writer = cell_frame0.clone();
        let cell_3_writer = cell_frame3.clone();

        // Concurrent writes to same slot
        let t1 = thread::spawn(move || {
            cell_0_writer.save(Frame::new(0), Some(100), Some(0x100));
        });

        let t2 = thread::spawn(move || {
            cell_3_writer.save(Frame::new(3), Some(300), Some(0x300));
        });

        t1.join().unwrap();
        t2.join().unwrap();

        // Should be one or the other, not corrupted
        let loaded = cell_frame0.load();
        assert!(
            loaded == Some(100) || loaded == Some(300),
            "Unexpected value: {:?}",
            loaded
        );
    });
}

/// Test rollback pattern: save then load from different threads.
///
/// Simulates the typical rollback scenario where one thread saves state
/// and another needs to load it for rollback.
#[test]
fn test_rollback_save_load_pattern() {
    loom::model(|| {
        let states: SavedStates<u64> = SavedStates::new(4); // 5 cells

        let cell1 = states.get_cell(Frame::new(1)).unwrap();
        let cell2 = states.get_cell(Frame::new(2)).unwrap();

        // First, save frame 1
        cell1.save(Frame::new(1), Some(100), Some(1));

        let cell1_for_rollback = cell1.clone();
        let cell2_for_advance = cell2.clone();

        // Concurrent: main thread advances and saves frame 2
        //            rollback thread loads frame 1
        let rollback = thread::spawn(move || cell1_for_rollback.load());

        let advance = thread::spawn(move || {
            cell2_for_advance.save(Frame::new(2), Some(200), Some(2));
        });

        let rollback_result = rollback.join().unwrap();
        advance.join().unwrap();

        // Rollback should always get the saved value for frame 1
        assert_eq!(rollback_result, Some(100));

        // Frame 2 should have been saved
        assert_eq!(cell2.load(), Some(200));
    });
}

/// Test bounded preemption for larger scenarios.
#[test]
fn test_saved_states_bounded_preemption() {
    let mut builder = loom::model::Builder::new();
    builder.preemption_bound = Some(2);

    builder.check(|| {
        let states: SavedStates<u64> = SavedStates::new(3); // 4 cells

        // Multiple writers to different cells
        let handles: Vec<_> = (0..4)
            .map(|i| {
                let cell = states.get_cell(Frame::new(i as i32)).unwrap();
                thread::spawn(move || {
                    cell.save(Frame::new(i as i32), Some(i as u64 * 100), Some(i as u128));
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        // Verify each cell has its value
        for i in 0..4 {
            let cell = states.get_cell(Frame::new(i as i32)).unwrap();
            let loaded = cell.load();
            assert_eq!(loaded, Some(i as u64 * 100));
        }
    });
}
