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
        Ok(self.states[pos].clone())
    }
}
