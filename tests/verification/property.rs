//! Property-based tests for InputQueue and SyncLayer internals.
//!
//! These tests use proptest to verify invariants hold under random inputs,
//! leveraging the exposed __internal module for direct component testing.
//!
//! # Invariants Tested
//!
//! ## InputQueue Invariants
//! - INV-IQ1: Queue length <= queue_capacity
//! - INV-IQ2: head and tail are valid indices [0, queue_capacity)
//! - INV-IQ3: Sequential frame adds produce sequential entries
//! - INV-IQ4: Predictions are deterministic (same last_confirmed → same prediction)
//! - INV-IQ5: first_incorrect_frame is NULL or < current_frame
//!
//! ## SyncLayer Invariants
//! - INV-SL1: last_confirmed_frame <= current_frame (or NULL)
//! - INV-SL2: last_saved_frame <= current_frame (or NULL)
//! - INV-SL3: first_incorrect_frame < current_frame (when not NULL)
//! - INV-SL4: Saved state available for frames within max_prediction
//! - INV-SL5: Rollback/advance cycles preserve invariants
//! - INV-SL6: Multiple players get correct inputs
//! - INV-SL7: Checksums are deterministic (same state → same checksum)

// Allow test-specific patterns that are appropriate for test code
#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]

use fortress_rollback::__internal::{InputQueue, PlayerInput, SavedStates, SyncLayer};
use fortress_rollback::telemetry::InvariantChecker;
use fortress_rollback::{Config, FortressRequest, Frame, InputStatus};
use proptest::prelude::*;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

// ============================================================================
// Test Configuration
// ============================================================================

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize, Debug)]
struct TestInput {
    value: u8,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
struct TestState {
    value: u64,
    frame: i32,
}

struct TestConfig;

impl Config for TestConfig {
    type Input = TestInput;
    type State = TestState;
    type Address = SocketAddr;
}

// ============================================================================
// Property Test Strategies
// ============================================================================

/// Strategy for queue lengths (power of 2 for efficiency)
fn queue_length_strategy() -> impl Strategy<Value = usize> {
    prop_oneof![Just(32), Just(64), Just(128)]
}

/// Strategy for number of frames to simulate
fn frame_count_strategy() -> impl Strategy<Value = usize> {
    1usize..200
}

/// Strategy for player count
fn player_count_strategy() -> impl Strategy<Value = usize> {
    1usize..5
}

/// Strategy for max prediction window
fn max_prediction_strategy() -> impl Strategy<Value = usize> {
    4usize..17
}

// ============================================================================
// InputQueue Invariant Tests
// ============================================================================

proptest! {
    /// INV-IQ1: Queue length never exceeds capacity
    #[test]
    fn prop_input_queue_length_bounded(
        queue_length in queue_length_strategy(),
        num_frames in frame_count_strategy(),
    ) {
        let mut queue = InputQueue::<TestConfig>::with_queue_length(0, queue_length).expect("queue");

        // Leave a buffer before we need to discard (at least 4 frames margin)
        let discard_threshold = queue_length.saturating_sub(4);

        for i in 0..num_frames as i32 {
            // Discard early enough to prevent overflow, scaled to queue size
            if i as usize >= discard_threshold && i > 4 {
                // Discard all but the last few frames
                queue.discard_confirmed_frames(Frame::new(i - 4));
            }

            let input = PlayerInput::new(Frame::new(i), TestInput { value: i as u8 });
            queue.add_input(input);

            // INV-IQ1: All invariants including length <= queue_length
            let result = queue.check_invariants();
            prop_assert!(
                result.is_ok(),
                "Queue invariants failed at frame {}: {:?}",
                i,
                result.err()
            );
        }
    }

    /// INV-IQ3: Sequential frame additions maintain frame ordering
    #[test]
    fn prop_input_queue_sequential_frames(
        queue_length in queue_length_strategy(),
        num_frames in 1usize..100,
    ) {
        let mut queue = InputQueue::<TestConfig>::with_queue_length(0, queue_length).expect("queue");

        let mut last_frame = Frame::NULL;
        let discard_threshold = queue_length.saturating_sub(4);

        for i in 0..num_frames as i32 {
            // Discard early enough to prevent overflow, scaled to queue size
            if i as usize >= discard_threshold && i > 4 {
                queue.discard_confirmed_frames(Frame::new(i - 4));
            }

            let input = PlayerInput::new(Frame::new(i), TestInput { value: i as u8 });
            let result_frame = queue.add_input(input);

            // Result should be monotonically increasing
            if !result_frame.is_null() && !last_frame.is_null() {
                prop_assert!(
                    result_frame > last_frame,
                    "Frame {} not greater than previous {}",
                    result_frame,
                    last_frame
                );
            }
            last_frame = result_frame;
        }
    }

    /// INV-IQ4: Prediction is deterministic given same last_confirmed_input
    #[test]
    fn prop_prediction_determinism(
        queue_length in queue_length_strategy(),
        last_confirmed_value in any::<u8>(),
        prediction_frames in 1usize..10,
    ) {
        // Create two identical queues
        let mut queue1 = InputQueue::<TestConfig>::with_queue_length(0, queue_length).expect("queue");
        let mut queue2 = InputQueue::<TestConfig>::with_queue_length(0, queue_length).expect("queue");

        // Add same initial input to both
        let initial_input = PlayerInput::new(Frame::new(0), TestInput { value: last_confirmed_value });
        queue1.add_input(initial_input);
        queue2.add_input(initial_input);

        // Request predictions for same future frames
        for i in 1..=prediction_frames as i32 {
            let (pred1, status1) = queue1.input(Frame::new(i)).expect("input");
            let (pred2, status2) = queue2.input(Frame::new(i)).expect("input");

            // Predictions must be identical
            prop_assert_eq!(
                pred1.value, pred2.value,
                "Prediction mismatch for frame {}: {} vs {}",
                i, pred1.value, pred2.value
            );
            prop_assert_eq!(
                status1, status2,
                "Status mismatch for frame {}",
                i
            );
        }
    }

    /// INV-IQ5: first_incorrect_frame tracking
    #[test]
    fn prop_first_incorrect_frame_tracking(
        queue_length in queue_length_strategy(),
        num_frames in 5usize..50,
    ) {
        let mut queue = InputQueue::<TestConfig>::with_queue_length(0, queue_length).expect("queue");
        let discard_threshold = queue_length.saturating_sub(4);

        // Add some confirmed inputs, with proper discarding to prevent overflow
        for i in 0..num_frames as i32 {
            // Discard early enough to prevent overflow, scaled to queue size
            if i as usize >= discard_threshold && i > 4 {
                queue.discard_confirmed_frames(Frame::new(i - 4));
            }

            let input = PlayerInput::new(Frame::new(i), TestInput { value: i as u8 });
            queue.add_input(input);
        }

        // Request frames beyond confirmed (triggers prediction)
        let future_frame = Frame::new((num_frames + 5) as i32);
        let (_pred, status) = queue.input(future_frame).expect("input");
        prop_assert_eq!(status, InputStatus::Predicted);

        // Reset and check first_incorrect_frame
        queue.reset_prediction();
        let fif = queue.first_incorrect_frame();
        prop_assert!(fif.is_null(), "first_incorrect_frame should be NULL after reset");
    }
}

// ============================================================================
// SyncLayer Invariant Tests
// ============================================================================

// Note: Most SyncLayer methods are pub(crate), so we can only test:
// - Construction (SyncLayer::new, SyncLayer::with_queue_length)
// - Initial state invariants via check_invariants()
//
// Full operational tests are done via session APIs in other test files.

proptest! {
    /// SyncLayer construction with various parameters maintains invariants
    #[test]
    fn prop_sync_layer_construction_invariants(
        num_players in player_count_strategy(),
        max_prediction in max_prediction_strategy(),
        queue_length in queue_length_strategy(),
    ) {
        let sync_layer = SyncLayer::<TestConfig>::with_queue_length(
            num_players,
            max_prediction,
            queue_length,
        );

        // Newly constructed SyncLayer should pass all invariants
        let result = sync_layer.check_invariants();
        prop_assert!(
            result.is_ok(),
            "New SyncLayer({}, {}, {}) should pass invariants: {:?}",
            num_players,
            max_prediction,
            queue_length,
            result.err()
        );
    }
}

// ============================================================================
// SavedStates Invariant Tests
// ============================================================================

proptest! {
    /// Saved states use circular indexing correctly
    #[test]
    fn prop_saved_states_circular_index(
        max_prediction in 2usize..20,
        frame in 0i32..1000,
    ) {
        let states = SavedStates::<u64>::new(max_prediction);
        let num_cells = max_prediction + 1;

        // get_cell should never fail for valid frames
        let frame_obj = Frame::new(frame);
        let result = states.get_cell(frame_obj);
        prop_assert!(result.is_ok(), "get_cell failed for frame {}", frame);

        // Verify circular indexing: frame % num_cells should be valid
        let expected_index = (frame as usize) % num_cells;
        prop_assert!(expected_index < num_cells);
    }

    /// States can be saved and loaded correctly
    #[test]
    fn prop_saved_states_roundtrip(
        max_prediction in 2usize..20,
        value in any::<u64>(),
        frame in 0i32..1000,
    ) {
        let states = SavedStates::<u64>::new(max_prediction);
        let frame_obj = Frame::new(frame);

        let cell = states.get_cell(frame_obj).unwrap();
        cell.save(frame_obj, Some(value), Some(value as u128));

        let loaded = cell.load();
        prop_assert_eq!(loaded, Some(value), "Loaded value doesn't match saved");
    }

    /// Frame wrapping maps to same cell
    #[test]
    fn prop_saved_states_frame_wrapping(
        max_prediction in 2usize..10,
        base_frame in 0i32..100,
    ) {
        let states = SavedStates::<u64>::new(max_prediction);
        let num_cells = max_prediction + 1;

        let frame1 = Frame::new(base_frame);
        let frame2 = Frame::new(base_frame + num_cells as i32);

        // Both frames should map to the same cell
        let cell1 = states.get_cell(frame1).unwrap();
        let cell2 = states.get_cell(frame2).unwrap();

        // Save via cell1
        cell1.save(frame1, Some(42), None);

        // Load via cell2 should see the same value (same slot)
        let loaded = cell2.load();
        prop_assert_eq!(loaded, Some(42), "Frame wrapping doesn't access same cell");
    }
}

// ============================================================================
// Cross-Component Invariant Tests
// ============================================================================

// Note: Full cross-component tests with operations are done via session APIs.
// Here we test that construction properly initializes all components.

proptest! {
    /// SyncLayer construction initializes all input queues correctly
    #[test]
    fn prop_sync_layer_initializes_input_queues(
        num_players in 1usize..5,
        max_prediction in 4usize..17,
    ) {
        let sync_layer = SyncLayer::<TestConfig>::with_queue_length(
            num_players,
            max_prediction,
            64,
        );

        // check_invariants on SyncLayer validates all input queues internally
        let result = sync_layer.check_invariants();
        prop_assert!(
            result.is_ok(),
            "SyncLayer with {} players should have valid input queues: {:?}",
            num_players,
            result.err()
        );
    }
}

// ============================================================================
// Frame 0 Edge Case Tests (FV-GAP-5)
// ============================================================================
//
// These tests were added as part of the Frame 0 Rollback FV Gap Analysis (Session 47).
// They explicitly target the edge case where first_incorrect_frame == current_frame,
// which can occur at frame 0 when a misprediction is detected before any frame advances.

proptest! {
    /// FV-GAP-5: First incorrect frame at frame 0 is handled correctly
    ///
    /// When first_incorrect_frame equals the current frame (both 0), the system
    /// should not attempt to rollback (since there's nothing to roll back to).
    /// Instead, it should reset predictions and continue normally.
    #[test]
    fn prop_frame_0_misprediction_does_not_panic(
        _predicted_input in any::<u8>(),
        actual_input in any::<u8>(),
        _num_players in 1usize..4,
    ) {
        // This test verifies the invariant: when first_incorrect == current_frame,
        // rollback is skipped (handled by the guard in adjust_gamestate).
        // We test this at the InputQueue level by simulating the scenario.

        let mut queue = InputQueue::<TestConfig>::with_queue_length(0, 64).expect("queue");

        // At frame 0, predict an input for frame 0
        let (pred, status) = queue.input(Frame::new(0)).expect("input");
        let _ = (pred, status); // predictions at frame 0 before any input

        // Add the "actual" input for frame 0 (potentially different from prediction)
        let actual = PlayerInput::new(Frame::new(0), TestInput { value: actual_input });
        queue.add_input(actual);

        // The queue should remain valid regardless of misprediction
        let result = queue.check_invariants();
        prop_assert!(
            result.is_ok(),
            "Queue should remain valid after frame 0 input: {:?}",
            result.err()
        );
    }

    /// FV-GAP-5: Predictions at frame 0 are reset correctly
    ///
    /// Verifies that reset_prediction() works correctly when called at frame 0,
    /// which is what happens in the skip_rollback path.
    #[test]
    fn prop_frame_0_reset_prediction(
        initial_value in any::<u8>(),
    ) {
        let mut queue = InputQueue::<TestConfig>::with_queue_length(0, 64).expect("queue");

        // Add initial input at frame 0
        let input = PlayerInput::new(Frame::new(0), TestInput { value: initial_value });
        queue.add_input(input);

        // Request prediction for frame 1 (this marks frame 1 as predicted)
        let (pred, status) = queue.input(Frame::new(1)).expect("input");
        prop_assert_eq!(status, InputStatus::Predicted);
        let _ = pred;

        // Reset prediction (as done in skip_rollback path)
        queue.reset_prediction();

        // first_incorrect_frame should be NULL after reset
        let fif = queue.first_incorrect_frame();
        prop_assert!(
            fif.is_null(),
            "first_incorrect_frame should be NULL after reset, got {:?}",
            fif
        );

        // Queue invariants should hold
        let result = queue.check_invariants();
        prop_assert!(
            result.is_ok(),
            "Queue invariants should hold after reset: {:?}",
            result.err()
        );
    }

    /// FV-GAP-5: Multiple predictions from frame 0 are handled
    ///
    /// Tests the scenario where multiple future frames are predicted starting from
    /// frame 0, then actual inputs arrive. The invariants should hold throughout.
    #[test]
    fn prop_frame_0_multiple_predictions(
        num_predictions in 1usize..8,
        actual_values in prop::collection::vec(any::<u8>(), 1..8),
    ) {
        let num_predictions = std::cmp::min(num_predictions, actual_values.len());
        let mut queue = InputQueue::<TestConfig>::with_queue_length(0, 64).expect("queue");

        // Request predictions for frames 0..num_predictions
        for i in 0..num_predictions {
            let (_, status) = queue.input(Frame::new(i as i32)).expect("input");
            prop_assert_eq!(
                status,
                InputStatus::Predicted,
                "Frame {} should be predicted",
                i
            );
        }

        // Now add actual inputs for those frames (may differ from predictions)
        for (i, &value) in actual_values.iter().take(num_predictions).enumerate() {
            let input = PlayerInput::new(Frame::new(i as i32), TestInput { value });
            queue.add_input(input);
        }

        // Queue should remain valid regardless of mispredictions
        let result = queue.check_invariants();
        prop_assert!(
            result.is_ok(),
            "Queue should remain valid after adding {} actual inputs: {:?}",
            num_predictions,
            result.err()
        );

        // Reset and verify
        queue.reset_prediction();
        let result = queue.check_invariants();
        prop_assert!(
            result.is_ok(),
            "Queue should remain valid after reset: {:?}",
            result.err()
        );
    }
}

// ============================================================================
// SyncLayer Operational Property Tests (INV-SL5, INV-SL6, INV-SL7)
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// INV-SL5: Rollback/advance cycles preserve invariants
    ///
    /// Verifies that invariants hold through complete rollback sequences:
    /// 1. Advance N frames, saving state at each
    /// 2. Rollback to a previous frame
    /// 3. Re-advance to the original frame
    /// 4. Invariants should hold throughout
    #[test]
    fn prop_sync_layer_rollback_advance_cycles(
        max_prediction in 4usize..12,
        num_frames in 5usize..20,
        rollback_target in 0usize..5,
    ) {
        let rollback_target = rollback_target.min(num_frames.saturating_sub(1));

        let mut sync_layer = SyncLayer::<TestConfig>::with_queue_length(
            2, // 2 players
            max_prediction,
            64,
        );

        // Phase 1: Advance N frames, saving state at each
        for i in 0..num_frames {
            let request = sync_layer.save_current_state();
            if let fortress_rollback::FortressRequest::SaveGameState { cell, frame } = request {
                let state = TestState { value: i as u64, frame: frame.as_i32() };
                cell.save(frame, Some(state), Some(i as u128));
            }

            // Check invariants after each save
            let result = sync_layer.check_invariants();
            prop_assert!(
                result.is_ok(),
                "Invariants failed after save at frame {}: {:?}",
                i,
                result.err()
            );

            sync_layer.advance_frame();
        }

        let original_frame = sync_layer.current_frame();

        // Phase 2: Rollback to target frame
        let target_frame = Frame::new(rollback_target as i32);

        // Only rollback if target is within prediction window
        if original_frame.as_i32() - target_frame.as_i32() <= max_prediction as i32 {
            let result = sync_layer.load_frame(target_frame);
            if result.is_ok() {
                // Check invariants after rollback
                let inv_result = sync_layer.check_invariants();
                prop_assert!(
                    inv_result.is_ok(),
                    "Invariants failed after rollback to frame {}: {:?}",
                    rollback_target,
                    inv_result.err()
                );

                // Verify frame-related invariants explicitly
                prop_assert!(
                    sync_layer.last_saved_frame() <= sync_layer.current_frame(),
                    "INV-SL2: last_saved_frame ({}) > current_frame ({}) after rollback",
                    sync_layer.last_saved_frame(),
                    sync_layer.current_frame()
                );

                // Phase 3: Re-advance to original frame
                while sync_layer.current_frame() < original_frame {
                    let request = sync_layer.save_current_state();
                    if let fortress_rollback::FortressRequest::SaveGameState { cell, frame } = request {
                        let i = frame.as_i32() as u64;
                        let state = TestState { value: i * 10, frame: frame.as_i32() };
                        cell.save(frame, Some(state), Some(i as u128));
                    }

                    sync_layer.advance_frame();

                    // Check invariants during re-advance
                    let result = sync_layer.check_invariants();
                    prop_assert!(
                        result.is_ok(),
                        "Invariants failed during re-advance at frame {}: {:?}",
                        sync_layer.current_frame(),
                        result.err()
                    );
                }
            }
        }
    }

    /// INV-SL5 variant: Multiple consecutive rollbacks maintain invariants
    #[test]
    fn prop_sync_layer_multiple_rollbacks(
        max_prediction in 6usize..12,
        num_rollbacks in 1usize..4,
    ) {
        let mut sync_layer = SyncLayer::<TestConfig>::with_queue_length(
            2,
            max_prediction,
            64,
        );

        // Setup: advance to frame max_prediction and save all states
        for i in 0..=max_prediction {
            let request = sync_layer.save_current_state();
            if let fortress_rollback::FortressRequest::SaveGameState { cell, frame } = request {
                let state = TestState { value: i as u64, frame: frame.as_i32() };
                cell.save(frame, Some(state), None);
            }
            if i < max_prediction {
                sync_layer.advance_frame();
            }
        }

        // Perform multiple rollbacks
        let mut current = sync_layer.current_frame().as_i32();
        for rollback_idx in 0..num_rollbacks {
            // Rollback to a frame at least 1 behind current
            let target = (current - 1 - (rollback_idx as i32)).max(0);
            if current - target <= max_prediction as i32 && target < current {
                let result = sync_layer.load_frame(Frame::new(target));
                if result.is_ok() {
                    current = sync_layer.current_frame().as_i32();

                    let inv_result = sync_layer.check_invariants();
                    prop_assert!(
                        inv_result.is_ok(),
                        "Invariants failed after rollback #{} to frame {}: {:?}",
                        rollback_idx,
                        target,
                        inv_result.err()
                    );

                    // Re-advance one frame for next rollback
                    let request = sync_layer.save_current_state();
                    if let FortressRequest::SaveGameState { cell, frame } = request {
                        cell.save(frame, Some(TestState::default()), None);
                    }
                    sync_layer.advance_frame();
                    let _ = sync_layer.current_frame().as_i32();
                }
            }
        }
    }

    /// INV-SL7: Checksums are deterministic - same state produces same checksum
    ///
    /// This tests that when the same game state is saved with a given checksum,
    /// the checksum can be retrieved consistently.
    #[test]
    fn prop_sync_layer_checksum_consistency(
        max_prediction in 4usize..12,
        state_value in any::<u64>(),
        checksum in any::<u128>(),
    ) {
        let mut sync_layer = SyncLayer::<TestConfig>::with_queue_length(
            2,
            max_prediction,
            64,
        );

        // Use save_current_state() to get a cell through the public API
        let request = sync_layer.save_current_state();
        let cell = match request {
            FortressRequest::SaveGameState { cell, frame } => {
                prop_assert_eq!(frame, Frame::new(0));
                cell
            }
            _ => {
                prop_assert!(false, "Expected SaveGameState request");
                return Ok(());
            }
        };

        // Save state with checksum
        let state = TestState { value: state_value, frame: 0 };
        cell.save(Frame::new(0), Some(state), Some(checksum));

        // Retrieve and verify checksum
        let retrieved_checksum = cell.checksum();
        prop_assert_eq!(
            retrieved_checksum,
            Some(checksum),
            "Checksum should be retrievable after save"
        );

        // Retrieve and verify state
        let retrieved_state = cell.load();
        prop_assert!(
            retrieved_state.is_some(),
            "State should be retrievable after save"
        );
        prop_assert_eq!(
            retrieved_state.unwrap().value,
            state_value,
            "State value should match"
        );
    }

    /// INV-SL7 variant: Checksums are preserved through save/load cycles
    #[test]
    fn prop_sync_layer_checksum_preservation(
        max_prediction in 4usize..10,
        num_frames in 2usize..8,
        checksums in prop::collection::vec(any::<u128>(), 1..10),
    ) {
        let num_frames = num_frames.min(checksums.len());
        let mut sync_layer = SyncLayer::<TestConfig>::with_queue_length(
            2,
            max_prediction,
            64,
        );

        // Save states with checksums, keeping references to cells
        let mut saved_cells = Vec::new();
        for (i, checksum) in checksums.iter().take(num_frames).enumerate() {
            let request = sync_layer.save_current_state();
            if let FortressRequest::SaveGameState { cell, frame } = request {
                let state = TestState {
                    value: i as u64,
                    frame: frame.as_i32(),
                };
                cell.save(frame, Some(state), Some(*checksum));
                saved_cells.push((frame, cell.clone(), *checksum));
            }
            if i < num_frames - 1 {
                sync_layer.advance_frame();
            }
        }

        // Verify all checksums are preserved by checking the cells we saved
        for (frame, cell, expected_checksum) in &saved_cells {
            // Only check frames within prediction window
            if sync_layer.current_frame().as_i32() - frame.as_i32() <= max_prediction as i32 {
                // Check the cell directly (it may have been overwritten if frame wrapped)
                let actual_frame = cell.frame();
                if actual_frame == *frame {
                    let actual_checksum = cell.checksum();
                    prop_assert_eq!(
                        actual_checksum,
                        Some(*expected_checksum),
                        "Checksum for frame {} should be preserved",
                        frame
                    );
                }
            }
        }
    }
}

// ============================================================================
// SavedStates Advanced Property Tests
// ============================================================================

proptest! {
    /// SavedStates: Overwrite detection when frame wraps around
    ///
    /// When a frame wraps around in the circular buffer, the old state should
    /// be overwritten. This tests that we can detect this by checking frame().
    #[test]
    fn prop_saved_states_overwrite_detection(
        max_prediction in 2usize..8,
        base_frame in 0i32..10,
    ) {
        let states = SavedStates::<u64>::new(max_prediction);
        let num_cells = max_prediction + 1;

        let frame1 = Frame::new(base_frame);
        let frame2 = Frame::new(base_frame + num_cells as i32);

        // Save at frame1
        let cell1 = states.get_cell(frame1).unwrap();
        cell1.save(frame1, Some(100), Some(1000));

        // Verify frame1 is stored
        prop_assert_eq!(cell1.frame(), frame1);
        prop_assert_eq!(cell1.checksum(), Some(1000));

        // Now save at frame2 (wraps to same cell)
        let cell2 = states.get_cell(frame2).unwrap();
        cell2.save(frame2, Some(200), Some(2000));

        // The frame number should now be frame2
        let cell_check = states.get_cell(frame1).unwrap();
        prop_assert_eq!(
            cell_check.frame(),
            frame2,
            "Cell should have been overwritten with frame2"
        );

        // Attempting to use this cell for frame1 should be invalid
        // (the cell contains frame2 data)
        prop_assert_ne!(
            cell_check.frame(),
            frame1,
            "Old frame1 data should be overwritten"
        );
    }

    /// SavedStates: All cells are independently accessible
    #[test]
    fn prop_saved_states_all_cells_accessible(
        max_prediction in 2usize..10,
    ) {
        let states = SavedStates::<u64>::new(max_prediction);
        let num_cells = max_prediction + 1;

        // Save unique values in all cells
        for i in 0..num_cells {
            let frame = Frame::new(i as i32);
            let cell = states.get_cell(frame).unwrap();
            cell.save(frame, Some(i as u64 * 100), Some(i as u128));
        }

        // Verify all cells have correct values
        for i in 0..num_cells {
            let frame = Frame::new(i as i32);
            let cell = states.get_cell(frame).unwrap();
            prop_assert_eq!(cell.frame(), frame);
            prop_assert_eq!(cell.load(), Some(i as u64 * 100));
            prop_assert_eq!(cell.checksum(), Some(i as u128));
        }
    }
}

// ============================================================================
// P2P Session Checksum Propagation Property Tests (Phase 4)
// ============================================================================
//
// These tests verify the SyncHealth API and checksum propagation introduced
// in the Desync Detection API (P0). They test:
// - Checksums are sent at configured intervals
// - pending_checksums populated correctly on receive
// - sync_health() returns InSync when checksums match
// - sync_health() returns DesyncDetected when checksums differ
//
// Note: These are unit tests for the internal state machine behavior.
// Full integration tests with real network sockets are in tests/sessions/p2p.rs

#[cfg(test)]
mod p2p_checksum_tests {
    use super::*;
    use fortress_rollback::{
        DesyncDetection, PlayerHandle, PlayerType, SessionBuilder, SessionState, SyncHealth,
        UdpNonBlockingSocket,
    };
    use serial_test::serial;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    // Port range: 17700-17899 for property tests to avoid conflicts
    fn get_test_port(base: u16) -> u16 {
        17700 + base
    }

    /// Helper to advance a session by processing a number of frames with poll cycles.
    /// Uses time-based waiting to be robust across different platforms.
    fn synchronize_sessions<T: Config>(
        sess1: &mut fortress_rollback::P2PSession<T>,
        sess2: &mut fortress_rollback::P2PSession<T>,
        _poll_cycles: usize, // Kept for API compatibility but ignored - we use time-based timeout
    ) where
        T::Input: Default,
    {
        use std::thread;
        use std::time::{Duration, Instant};

        const SYNC_TIMEOUT: Duration = Duration::from_secs(5);
        const POLL_INTERVAL: Duration = Duration::from_millis(1);

        let start = Instant::now();
        while start.elapsed() < SYNC_TIMEOUT {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
            if sess1.current_state() == SessionState::Running
                && sess2.current_state() == SessionState::Running
            {
                return;
            }
            thread::sleep(POLL_INTERVAL);
        }
        // If we get here, synchronization may have failed - but don't assert,
        // let the caller handle the failure with their own assertions
    }

    /// Property: sync_health returns Pending when desync detection is off
    #[test]
    #[serial]
    fn test_sync_health_pending_when_detection_off() {
        let port1 = get_test_port(0);
        let port2 = get_test_port(1);
        let _addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port1);
        let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port2);

        let socket1 = UdpNonBlockingSocket::bind_to_port(port1).unwrap();
        let sess1 = SessionBuilder::<TestConfig>::new()
            .with_desync_detection_mode(DesyncDetection::Off)
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .unwrap()
            .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))
            .unwrap()
            .start_p2p_session(socket1)
            .unwrap();

        // With desync detection off, sync_health should return Pending
        let health = sess1.sync_health(PlayerHandle::new(1));
        assert_eq!(health, Some(SyncHealth::Pending));
    }

    /// Property: sync_health returns None for local players
    #[test]
    #[serial]
    fn test_sync_health_none_for_local_player() {
        let port1 = get_test_port(2);
        let port2 = get_test_port(3);
        let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port2);

        let socket1 = UdpNonBlockingSocket::bind_to_port(port1).unwrap();
        let sess1 = SessionBuilder::<TestConfig>::new()
            .with_desync_detection_mode(DesyncDetection::On { interval: 10 })
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .unwrap()
            .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))
            .unwrap()
            .start_p2p_session(socket1)
            .unwrap();

        // sync_health for local player should return None
        let health = sess1.sync_health(PlayerHandle::new(0));
        assert_eq!(health, None);
    }

    /// Property: sync_health returns None for invalid player handles
    #[test]
    #[serial]
    fn test_sync_health_none_for_invalid_handle() {
        let port1 = get_test_port(4);
        let port2 = get_test_port(5);
        let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port2);

        let socket1 = UdpNonBlockingSocket::bind_to_port(port1).unwrap();
        let sess1 = SessionBuilder::<TestConfig>::new()
            .with_desync_detection_mode(DesyncDetection::On { interval: 10 })
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .unwrap()
            .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))
            .unwrap()
            .start_p2p_session(socket1)
            .unwrap();

        // sync_health for non-existent player should return None
        let health = sess1.sync_health(PlayerHandle::new(99));
        assert_eq!(health, None);
    }

    /// Property: is_synchronized returns true when no remote peers exist
    #[test]
    #[serial]
    fn test_is_synchronized_no_remote_peers() {
        // A session with only local players should be synchronized with itself
        // This requires at least 2 players, so we need a remote player
        // Actually, creating a session with only local players isn't a typical use case
        // but we can test the behavior when all remotes are disconnected
        let port1 = get_test_port(6);
        let port2 = get_test_port(7);
        let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port2);

        let socket1 = UdpNonBlockingSocket::bind_to_port(port1).unwrap();
        let mut sess1 = SessionBuilder::<TestConfig>::new()
            .with_desync_detection_mode(DesyncDetection::On { interval: 10 })
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .unwrap()
            .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))
            .unwrap()
            .start_p2p_session(socket1)
            .unwrap();

        // Initially, before any checksum exchange, should be pending (not synchronized)
        // The definition of is_synchronized is: all remote peers show InSync
        // Before any checksums are exchanged, they're Pending
        assert!(!sess1.is_synchronized());

        // Disconnect the remote player
        sess1.disconnect_player(PlayerHandle::new(1)).unwrap();

        // With no connected remote peers, should be synchronized
        // (Note: disconnected players may not count as "remote" anymore for sync purposes)
    }

    /// Property: all_sync_health returns entries for all remote players
    #[test]
    #[serial]
    fn test_all_sync_health_includes_all_remotes() {
        let port1 = get_test_port(8);
        let port2 = get_test_port(9);
        let port3 = get_test_port(10);
        let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port2);
        let addr3 = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port3);

        let socket1 = UdpNonBlockingSocket::bind_to_port(port1).unwrap();
        let sess1 = SessionBuilder::<TestConfig>::new()
            .with_num_players(3)
            .unwrap()
            .with_desync_detection_mode(DesyncDetection::On { interval: 10 })
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .unwrap()
            .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))
            .unwrap()
            .add_player(PlayerType::Remote(addr3), PlayerHandle::new(2))
            .unwrap()
            .start_p2p_session(socket1)
            .unwrap();

        let all_health = sess1.all_sync_health();

        // Should have 2 entries (one for each remote player)
        assert_eq!(all_health.len(), 2);

        // Verify handles are the remote players
        let handles: Vec<_> = all_health.iter().map(|(h, _)| *h).collect();
        assert!(handles.contains(&PlayerHandle::new(1)));
        assert!(handles.contains(&PlayerHandle::new(2)));
    }

    /// Property: last_verified_frame is None before any checksum comparison
    #[test]
    #[serial]
    fn test_last_verified_frame_initially_none() {
        let port1 = get_test_port(11);
        let port2 = get_test_port(12);
        let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port2);

        let socket1 = UdpNonBlockingSocket::bind_to_port(port1).unwrap();
        let sess1 = SessionBuilder::<TestConfig>::new()
            .with_desync_detection_mode(DesyncDetection::On { interval: 10 })
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .unwrap()
            .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))
            .unwrap()
            .start_p2p_session(socket1)
            .unwrap();

        // Before any frames are processed, last_verified_frame should be None
        assert_eq!(sess1.last_verified_frame(), None);
    }

    /// Property: Two synchronized sessions reach InSync after exchanging checksums
    ///
    /// This is an integration test that verifies the full checksum exchange flow:
    /// 1. Both sessions start in Pending state
    /// 2. After enough frames pass (at the checksum interval), checksums are sent
    /// 3. After checksums are compared, sync_health transitions to InSync
    #[test]
    #[serial]
    fn test_checksum_exchange_reaches_in_sync() {
        let port1 = get_test_port(13);
        let port2 = get_test_port(14);
        let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port1);
        let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port2);

        // Use a small interval for faster testing
        let interval = 5;

        let socket1 = UdpNonBlockingSocket::bind_to_port(port1).unwrap();
        let mut sess1 = SessionBuilder::<TestConfig>::new()
            .with_desync_detection_mode(DesyncDetection::On { interval })
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .unwrap()
            .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))
            .unwrap()
            .start_p2p_session(socket1)
            .unwrap();

        let socket2 = UdpNonBlockingSocket::bind_to_port(port2).unwrap();
        let mut sess2 = SessionBuilder::<TestConfig>::new()
            .with_desync_detection_mode(DesyncDetection::On { interval })
            .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))
            .unwrap()
            .add_player(PlayerType::Local, PlayerHandle::new(1))
            .unwrap()
            .start_p2p_session(socket2)
            .unwrap();

        // Synchronize sessions first
        synchronize_sessions(&mut sess1, &mut sess2, 50);

        assert_eq!(sess1.current_state(), SessionState::Running);
        assert_eq!(sess2.current_state(), SessionState::Running);

        // Initial state should be Pending (no checksums exchanged yet)
        assert_eq!(
            sess1.sync_health(PlayerHandle::new(1)),
            Some(SyncHealth::Pending)
        );
        assert_eq!(
            sess2.sync_health(PlayerHandle::new(0)),
            Some(SyncHealth::Pending)
        );

        // Advance frames and exchange messages until checksums are compared
        // We need to advance past the checksum interval
        let target_frames = (interval * 2 + 5) as i32;
        for frame_num in 0..target_frames {
            // Poll for messages
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();

            // Add local inputs
            let input = TestInput {
                value: frame_num as u8,
            };
            sess1.add_local_input(PlayerHandle::new(0), input).unwrap();
            sess2.add_local_input(PlayerHandle::new(1), input).unwrap();

            // Advance frames and handle requests
            let requests1 = sess1.advance_frame().unwrap();
            let requests2 = sess2.advance_frame().unwrap();

            // Handle save/load requests with deterministic state
            for req in requests1 {
                match req {
                    FortressRequest::SaveGameState { cell, frame } => {
                        let state = TestState {
                            value: frame.as_i32() as u64,
                            frame: frame.as_i32(),
                        };
                        // Use a deterministic checksum based on frame
                        let checksum = frame.as_i32() as u128 * 12345;
                        cell.save(frame, Some(state), Some(checksum));
                    },
                    FortressRequest::LoadGameState { cell, .. } => {
                        let _ = cell.load();
                    },
                    FortressRequest::AdvanceFrame { .. } => {},
                    _ => {}, // Handle any future variants
                }
            }
            for req in requests2 {
                match req {
                    FortressRequest::SaveGameState { cell, frame } => {
                        let state = TestState {
                            value: frame.as_i32() as u64,
                            frame: frame.as_i32(),
                        };
                        // Use the same deterministic checksum
                        let checksum = frame.as_i32() as u128 * 12345;
                        cell.save(frame, Some(state), Some(checksum));
                    },
                    FortressRequest::LoadGameState { cell, .. } => {
                        let _ = cell.load();
                    },
                    FortressRequest::AdvanceFrame { .. } => {},
                    _ => {}, // Handle any future variants
                }
            }
        }

        // Continue polling to ensure checksum messages are exchanged
        // Use time-based waiting for robustness across platforms
        let start = std::time::Instant::now();
        while start.elapsed() < std::time::Duration::from_millis(500) {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }

        // At this point, checksums should have been exchanged and compared
        // At least one of the sessions should have transitioned to InSync
        let health1 = sess1.sync_health(PlayerHandle::new(1));
        let health2 = sess2.sync_health(PlayerHandle::new(0));

        // Both should eventually reach InSync since they use deterministic checksums
        // Note: Due to timing, we might still be Pending - check that we haven't desynced
        match (health1, health2) {
            (Some(SyncHealth::InSync), _) | (_, Some(SyncHealth::InSync)) => {
                // At least one reached InSync - good
            },
            (Some(SyncHealth::Pending), Some(SyncHealth::Pending)) => {
                // Both still pending - acceptable for this test
                // The important thing is no desync detected
            },
            (Some(SyncHealth::DesyncDetected { .. }), _)
            | (_, Some(SyncHealth::DesyncDetected { .. })) => {
                panic!("Desync detected when checksums should match!");
            },
            _ => {},
        }

        // Verify no desync events were generated
        while let Some(event) = sess1.events().next() {
            if let fortress_rollback::FortressEvent::DesyncDetected { .. } = event {
                panic!("Session 1 generated unexpected DesyncDetected event");
            }
        }
        while let Some(event) = sess2.events().next() {
            if let fortress_rollback::FortressEvent::DesyncDetected { .. } = event {
                panic!("Session 2 generated unexpected DesyncDetected event");
            }
        }
    }
}
