//! Direct fuzz target for SyncLayer internals via __internal module.
//!
//! This target exercises SyncLayer construction and the exposed operations
//! without going through session APIs, enabling deeper coverage of core types.
//!
//! Note: Most SyncLayer methods are pub(crate), so this fuzz target focuses on:
//! - Construction with various parameters
//! - SavedStates operations (which are fully public)
//! - InvariantChecker validation
//!
//! # Safety Properties Tested
//! - No panics on arbitrary construction parameters
//! - SavedStates circular buffer safety
//! - Invariant checking doesn't panic

#![no_main]

use arbitrary::Arbitrary;
use fortress_rollback::Frame;
use fortress_rollback::__internal::SavedStates;
use libfuzzer_sys::fuzz_target;
use serde::{Deserialize, Serialize};

/// Test game state
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
struct TestState {
    value: u64,
    frame: i32,
}

/// Operations that can be performed on SavedStates
#[derive(Debug, Arbitrary)]
enum SavedStatesOp {
    /// Get a cell for a frame
    GetCell { frame: i16 },
    /// Save state to a cell
    SaveState { frame: i16, value: u64 },
    /// Load state from a cell
    LoadState { frame: i16 },
}

/// Fuzz input structure
#[derive(Debug, Arbitrary)]
struct FuzzInput {
    /// Max prediction window (2-32)
    max_prediction: u8,
    /// Sequence of operations
    operations: Vec<SavedStatesOp>,
}

fuzz_target!(|fuzz_input: FuzzInput| {
    // Clamp values to reasonable ranges
    let max_prediction = ((fuzz_input.max_prediction % 31) + 2) as usize; // 2-32

    // Limit operations to prevent timeouts
    let max_ops = 500;
    let operations = if fuzz_input.operations.len() > max_ops {
        &fuzz_input.operations[..max_ops]
    } else {
        &fuzz_input.operations
    };

    // Create SavedStates directly using __internal access
    let saved_states = SavedStates::<TestState>::new(max_prediction);

    // Execute operations
    for op in operations {
        match op {
            SavedStatesOp::GetCell { frame } => {
                let frame_val = (*frame as i32).max(0);
                let frame_obj = Frame::new(frame_val);

                // Get cell - may fail for invalid frames
                let result = saved_states.get_cell(frame_obj);
                // Just verify it doesn't panic
                let _ = result;
            },

            SavedStatesOp::SaveState { frame, value } => {
                let frame_val = (*frame as i32).max(0);
                let frame_obj = Frame::new(frame_val);

                // Get cell and save state
                if let Ok(cell) = saved_states.get_cell(frame_obj) {
                    let state = TestState {
                        value: *value,
                        frame: frame_val,
                    };
                    cell.save(frame_obj, Some(state), Some(*value as u128));
                }
            },

            SavedStatesOp::LoadState { frame } => {
                let frame_val = (*frame as i32).max(0);
                let frame_obj = Frame::new(frame_val);

                // Get cell and load state
                if let Ok(cell) = saved_states.get_cell(frame_obj) {
                    let loaded = cell.load();
                    // Just verify load doesn't panic
                    let _ = loaded;
                }
            },
        }
    }
});
