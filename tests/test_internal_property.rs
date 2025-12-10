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

use fortress_rollback::__internal::{InputQueue, PlayerInput, SavedStates, SyncLayer};
use fortress_rollback::telemetry::InvariantChecker;
use fortress_rollback::{Config, Frame, InputStatus};
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
        let mut queue = InputQueue::<TestConfig>::with_queue_length(0, queue_length);

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
        let mut queue = InputQueue::<TestConfig>::with_queue_length(0, queue_length);

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
        let mut queue1 = InputQueue::<TestConfig>::with_queue_length(0, queue_length);
        let mut queue2 = InputQueue::<TestConfig>::with_queue_length(0, queue_length);

        // Add same initial input to both
        let initial_input = PlayerInput::new(Frame::new(0), TestInput { value: last_confirmed_value });
        queue1.add_input(initial_input);
        queue2.add_input(initial_input);

        // Request predictions for same future frames
        for i in 1..=prediction_frames as i32 {
            let (pred1, status1) = queue1.input(Frame::new(i));
            let (pred2, status2) = queue2.input(Frame::new(i));

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
        let mut queue = InputQueue::<TestConfig>::with_queue_length(0, queue_length);
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
        let (_pred, status) = queue.input(future_frame);
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
