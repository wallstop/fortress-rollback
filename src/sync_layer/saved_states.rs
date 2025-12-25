//! Container for saved game states used during rollback.
//!
//! This module provides [`SavedStates`] which manages a circular buffer of
//! [`GameStateCell`]s for rollback functionality.

use crate::sync_layer::GameStateCell;
use crate::{FortressError, Frame};

/// Container for saved game states used during rollback.
///
/// # Note
///
/// This type is re-exported in [`__internal`](crate::__internal) for testing and fuzzing.
/// It is not part of the stable public API.
pub struct SavedStates<T> {
    /// The vector of game state cells.
    pub states: Vec<GameStateCell<T>>,
}

impl<T> SavedStates<T> {
    /// Creates a new SavedStates container with the given capacity.
    #[must_use]
    pub fn new(max_pred: usize) -> Self {
        // we need to store the current frame plus the number of max predictions, so that we can
        // roll back to the very first frame even when we have predicted as far ahead as we can.
        let num_cells = max_pred + 1;
        let mut states = Vec::with_capacity(num_cells);
        for _ in 0..num_cells {
            states.push(GameStateCell::default());
        }

        Self { states }
    }

    /// Gets the cell for a given frame.
    pub fn get_cell(&self, frame: Frame) -> Result<GameStateCell<T>, FortressError> {
        if frame.as_i32() < 0 {
            return Err(FortressError::InvalidFrame {
                frame,
                reason: "frame must be non-negative".to_string(),
            });
        }
        let pos = frame.as_i32() as usize % self.states.len();
        self.states
            .get(pos)
            .cloned()
            .ok_or_else(|| FortressError::InternalError {
                context: format!("states index {} out of bounds", pos),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // SavedStates::new Tests
    // ========================================================================

    #[test]
    fn new_creates_correct_number_of_cells() {
        let saved_states: SavedStates<u32> = SavedStates::new(3);
        // max_prediction + 1 cells
        assert_eq!(saved_states.states.len(), 4);
    }

    #[test]
    fn new_with_zero_max_prediction() {
        let saved_states: SavedStates<u32> = SavedStates::new(0);
        // 0 + 1 = 1 cell
        assert_eq!(saved_states.states.len(), 1);
    }

    #[test]
    fn new_with_large_max_prediction() {
        let saved_states: SavedStates<u8> = SavedStates::new(100);
        assert_eq!(saved_states.states.len(), 101);
    }

    #[test]
    fn new_cells_are_default_initialized() {
        let saved_states: SavedStates<u32> = SavedStates::new(2);
        // All cells should have null frames (default)
        for cell in &saved_states.states {
            assert!(cell.frame().is_null());
        }
    }

    // ========================================================================
    // SavedStates::get_cell Tests
    // ========================================================================

    #[test]
    fn get_cell_valid_frame_returns_ok() {
        let saved_states: SavedStates<u32> = SavedStates::new(3);
        let result = saved_states.get_cell(Frame::new(0));
        assert!(result.is_ok());
    }

    #[test]
    fn get_cell_negative_frame_returns_error() {
        let saved_states: SavedStates<u32> = SavedStates::new(3);
        let result = saved_states.get_cell(Frame::new(-1));
        assert!(result.is_err());
        match result {
            Err(FortressError::InvalidFrame { frame, reason }) => {
                assert_eq!(frame.as_i32(), -1);
                assert!(reason.contains("non-negative"));
            },
            _ => panic!("Expected InvalidFrame error"),
        }
    }

    #[test]
    fn get_cell_null_frame_returns_error() {
        let saved_states: SavedStates<u32> = SavedStates::new(3);
        let result = saved_states.get_cell(Frame::NULL);
        assert!(result.is_err());
    }

    #[test]
    fn get_cell_circular_indexing_wraps_correctly() {
        let saved_states: SavedStates<u32> = SavedStates::new(3); // 4 cells

        // Store data in each cell to verify circular behavior
        let cell0 = saved_states.get_cell(Frame::new(0)).unwrap();
        cell0.save(Frame::new(0), Some(100), None);

        let cell1 = saved_states.get_cell(Frame::new(1)).unwrap();
        cell1.save(Frame::new(1), Some(101), None);

        // Frame 4 should wrap to index 0 (4 % 4 = 0)
        let cell4 = saved_states.get_cell(Frame::new(4)).unwrap();
        // Cell at index 0 still has the data from frame 0 save
        let loaded = cell4.load();
        assert_eq!(loaded, Some(100));
    }

    #[test]
    fn get_cell_returns_same_cell_for_wrapped_frames() {
        let saved_states: SavedStates<u32> = SavedStates::new(2); // 3 cells

        // Frame 0 and Frame 3 should map to the same cell (both % 3 = 0)
        let cell0 = saved_states.get_cell(Frame::new(0)).unwrap();
        cell0.save(Frame::new(0), Some(42), None);

        let cell3 = saved_states.get_cell(Frame::new(3)).unwrap();
        assert_eq!(cell3.load(), Some(42));
    }

    #[test]
    fn get_cell_large_frame_number() {
        let saved_states: SavedStates<u32> = SavedStates::new(3); // 4 cells

        // Very large frame number should still work via modulo
        let result = saved_states.get_cell(Frame::new(1_000_000));
        assert!(result.is_ok());

        // Verify it maps to the correct index (1_000_000 % 4 = 0)
        let cell = result.unwrap();
        cell.save(Frame::new(1_000_000), Some(999), None);
        assert_eq!(cell.load(), Some(999));
    }

    #[test]
    fn get_cell_each_index_accessible() {
        let saved_states: SavedStates<u32> = SavedStates::new(3); // 4 cells

        // Save different values in each cell
        for i in 0..4 {
            let cell = saved_states.get_cell(Frame::new(i)).unwrap();
            cell.save(Frame::new(i), Some(i as u32 * 10), None);
        }

        // Verify each cell has the correct value
        for i in 0..4 {
            let cell = saved_states.get_cell(Frame::new(i)).unwrap();
            assert_eq!(cell.load(), Some(i as u32 * 10));
        }
    }

    #[test]
    fn get_cell_with_checksum() {
        let saved_states: SavedStates<String> = SavedStates::new(1);
        let cell = saved_states.get_cell(Frame::new(0)).unwrap();

        let checksum: u128 = 0x1234_5678_9ABC_DEF0;
        cell.save(Frame::new(0), Some("test".to_string()), Some(checksum));

        assert_eq!(cell.checksum(), Some(checksum));
    }

    #[test]
    fn get_cell_single_cell_buffer() {
        // Edge case: only one cell (max_prediction = 0)
        let saved_states: SavedStates<u32> = SavedStates::new(0); // 1 cell

        // All frames should map to the same single cell
        let cell0 = saved_states.get_cell(Frame::new(0)).unwrap();
        cell0.save(Frame::new(0), Some(42), None);

        let cell1 = saved_states.get_cell(Frame::new(1)).unwrap();
        assert_eq!(cell1.load(), Some(42)); // Same cell

        let cell100 = saved_states.get_cell(Frame::new(100)).unwrap();
        assert_eq!(cell100.load(), Some(42)); // Still same cell
    }

    // ========================================================================
    // SavedStates Cell Interaction Tests
    // ========================================================================

    #[test]
    fn cells_are_cloned_references() {
        let saved_states: SavedStates<u32> = SavedStates::new(2);

        // Get the same cell twice
        let cell_a = saved_states.get_cell(Frame::new(0)).unwrap();
        let cell_b = saved_states.get_cell(Frame::new(0)).unwrap();

        // Save via cell_a
        cell_a.save(Frame::new(0), Some(123), None);

        // Should be visible via cell_b (they share the underlying Arc)
        assert_eq!(cell_b.load(), Some(123));
    }

    #[test]
    fn overwrite_cell_data() {
        let saved_states: SavedStates<u32> = SavedStates::new(1);
        let cell = saved_states.get_cell(Frame::new(0)).unwrap();

        cell.save(Frame::new(0), Some(100), None);
        assert_eq!(cell.load(), Some(100));

        cell.save(Frame::new(1), Some(200), None);
        assert_eq!(cell.load(), Some(200));
    }

    #[test]
    fn cells_independent_per_index() {
        let saved_states: SavedStates<u32> = SavedStates::new(2); // 3 cells

        let cell0 = saved_states.get_cell(Frame::new(0)).unwrap();
        let cell1 = saved_states.get_cell(Frame::new(1)).unwrap();
        let cell2 = saved_states.get_cell(Frame::new(2)).unwrap();

        cell0.save(Frame::new(0), Some(10), None);
        cell1.save(Frame::new(1), Some(20), None);
        cell2.save(Frame::new(2), Some(30), None);

        // Each cell should have its own data
        assert_eq!(cell0.load(), Some(10));
        assert_eq!(cell1.load(), Some(20));
        assert_eq!(cell2.load(), Some(30));
    }
}
