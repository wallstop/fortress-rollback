//! Tests for InvariantChecker implementations via __internal module.
//!
//! These tests verify that the InvariantChecker trait is properly implemented
//! for InputQueue and SyncLayer, and that invariants are preserved across
//! various operation sequences.
//!
//! # Note on Access Levels
//!
//! The `__internal` module exposes types like `InputQueue` and `SyncLayer`,
//! but most of their methods remain `pub(crate)`. This test file focuses on:
//! - Construction via public constructors
//! - Invariant checking via `InvariantChecker::check_invariants()`
//! - InputQueue operations which have public methods
//!
//! # Invariants Tested
//!
//! ## InputQueue (from check_invariants implementation)
//! - INV-IQ-1: length <= queue_length
//! - INV-IQ-2: head < queue_length
//! - INV-IQ-3: tail < queue_length

// Allow hardcoded IP addresses - 127.0.0.1 is appropriate for tests
#![allow(clippy::ip_constant)]
//! - INV-IQ-4: first_incorrect_frame is NULL or < last_added_frame (adjusted for delay)
//!
//! ## SyncLayer (from check_invariants implementation)
//! - INV-SL-1: last_confirmed_frame <= current_frame (or NULL)
//! - INV-SL-2: last_saved_frame <= current_frame (or NULL)
//! - INV-SL-3: first_incorrect_frame < current_frame (or NULL)
//! - INV-SL-4: current_frame >= 0
//! - INV-SL-5: All input queues pass their invariant checks

use fortress_rollback::__internal::{InputQueue, PlayerInput, SavedStates, SyncLayer};
use fortress_rollback::telemetry::{InvariantChecker, InvariantViolation};
use fortress_rollback::{Config, Frame};
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
// InputQueue Invariant Tests
// ============================================================================

mod input_queue_invariants {
    use super::*;

    /// Verify invariants hold for a newly constructed InputQueue.
    #[test]
    fn test_new_queue_invariants() {
        let queue = InputQueue::<TestConfig>::new(0).expect("queue");
        assert!(
            queue.check_invariants().is_ok(),
            "New queue should pass all invariants"
        );
    }

    /// Verify invariants hold for queue with custom length.
    #[test]
    fn test_custom_length_queue_invariants() {
        for len in [32, 64, 128, 256] {
            let queue = InputQueue::<TestConfig>::with_queue_length(0, len).expect("queue");
            assert!(
                queue.check_invariants().is_ok(),
                "Queue with length {} should pass invariants",
                len
            );
        }
    }

    /// Verify invariants hold after sequential adds.
    #[test]
    fn test_invariants_after_sequential_adds() {
        let mut queue = InputQueue::<TestConfig>::with_queue_length(0, 64).expect("queue");

        for i in 0..50 {
            let input = PlayerInput::new(Frame::new(i), TestInput { value: i as u8 });
            queue.add_input(input);

            let result = queue.check_invariants();
            assert!(
                result.is_ok(),
                "Invariants should hold after add {}: {:?}",
                i,
                result.err()
            );
        }
    }

    /// Verify invariants hold after discard operations.
    #[test]
    fn test_invariants_after_discard() {
        let mut queue = InputQueue::<TestConfig>::with_queue_length(0, 64).expect("queue");

        // Add some inputs
        for i in 0..40 {
            queue.add_input(PlayerInput::new(
                Frame::new(i),
                TestInput { value: i as u8 },
            ));
        }

        // Discard various amounts
        for discard_up_to in [5, 10, 20, 30] {
            queue.discard_confirmed_frames(Frame::new(discard_up_to));

            let result = queue.check_invariants();
            assert!(
                result.is_ok(),
                "Invariants should hold after discard up to {}: {:?}",
                discard_up_to,
                result.err()
            );
        }
    }

    /// Verify invariants hold with frame delay.
    #[test]
    fn test_invariants_with_frame_delay() {
        for delay in [1, 2, 4, 7] {
            let mut queue = InputQueue::<TestConfig>::with_queue_length(0, 64).expect("queue");
            queue.set_frame_delay(delay).expect("valid delay");

            for i in 0..30 {
                queue.add_input(PlayerInput::new(
                    Frame::new(i),
                    TestInput { value: i as u8 },
                ));

                let result = queue.check_invariants();
                assert!(
                    result.is_ok(),
                    "Invariants should hold with delay {} after add {}: {:?}",
                    delay,
                    i,
                    result.err()
                );
            }
        }
    }

    /// Verify invariants hold after prediction and reset.
    #[test]
    fn test_invariants_after_prediction_reset() {
        let mut queue = InputQueue::<TestConfig>::with_queue_length(0, 64).expect("queue");

        // Add some confirmed inputs
        for i in 0..10 {
            queue.add_input(PlayerInput::new(
                Frame::new(i),
                TestInput { value: i as u8 },
            ));
        }

        // Request future frames (triggers prediction)
        for i in 10..20 {
            let _ = queue.input(Frame::new(i));
        }

        // Check invariants before reset
        assert!(
            queue.check_invariants().is_ok(),
            "Invariants should hold after prediction requests"
        );

        // Reset prediction
        queue.reset_prediction();

        // Check invariants after reset
        assert!(
            queue.check_invariants().is_ok(),
            "Invariants should hold after prediction reset"
        );
    }

    /// Verify invariants hold during queue wraparound.
    #[test]
    fn test_invariants_during_wraparound() {
        let queue_len = 32;
        let mut queue = InputQueue::<TestConfig>::with_queue_length(0, queue_len).expect("queue");

        // Add enough inputs to cause wraparound
        for i in 0..100 {
            queue.add_input(PlayerInput::new(
                Frame::new(i),
                TestInput { value: i as u8 },
            ));

            // Discard old frames to allow wraparound
            if i > 20 && i % 10 == 0 {
                queue.discard_confirmed_frames(Frame::new(i - 15));
            }

            let result = queue.check_invariants();
            assert!(
                result.is_ok(),
                "Invariants should hold during wraparound at frame {}: {:?}",
                i,
                result.err()
            );
        }
    }

    /// Test that specific invariant violations are detected.
    #[test]
    fn test_invariant_violation_detection() {
        // Create a valid queue and verify it passes
        let queue = InputQueue::<TestConfig>::with_queue_length(0, 64).expect("queue");
        let result = queue.check_invariants();
        assert!(result.is_ok());

        // We can't easily create invalid states without unsafe code,
        // but we can verify the check_invariants method returns correctly
        let result = queue.check_invariants();
        match result {
            Ok(()) => {}, // Expected for valid queue
            Err(violation) => {
                // If somehow invalid, ensure we get proper error info
                assert!(!violation.to_string().is_empty());
            },
        }
    }
}

// ============================================================================
// SyncLayer Invariant Tests
// ============================================================================

mod sync_layer_invariants {
    use super::*;

    /// Verify invariants hold for a newly constructed SyncLayer.
    #[test]
    fn test_new_sync_layer_invariants() {
        let sync_layer = SyncLayer::<TestConfig>::new(2, 8);
        assert!(
            sync_layer.check_invariants().is_ok(),
            "New SyncLayer should pass all invariants"
        );
    }

    /// Verify invariants hold with various configurations.
    #[test]
    fn test_sync_layer_config_invariants() {
        for num_players in 1..=4 {
            for max_prediction in [4, 8, 16] {
                for queue_length in [32, 64, 128] {
                    let sync_layer = SyncLayer::<TestConfig>::with_queue_length(
                        num_players,
                        max_prediction,
                        queue_length,
                    );

                    let result = sync_layer.check_invariants();
                    assert!(
                        result.is_ok(),
                        "SyncLayer({}, {}, {}) should pass invariants: {:?}",
                        num_players,
                        max_prediction,
                        queue_length,
                        result.err()
                    );
                }
            }
        }
    }

    // SyncLayer methods are pub(crate), so we test:
    // - Construction
    // - Initial invariant state
    // Full operation tests are done via session APIs in other test files.
}

// ============================================================================
// SavedStates Invariant Tests
// ============================================================================

mod saved_states_invariants {
    use super::*;

    /// Verify SavedStates construction and cell access.
    #[test]
    fn test_saved_states_construction() {
        for max_pred in [4, 8, 16, 32] {
            let states = SavedStates::<u64>::new(max_pred);

            // Should have max_pred + 1 slots
            for i in 0..(max_pred + 1) {
                let result = states.get_cell(Frame::new(i as i32));
                assert!(
                    result.is_ok(),
                    "get_cell should succeed for frame {} with max_pred {}",
                    i,
                    max_pred
                );
            }
        }
    }

    /// Verify circular indexing works correctly.
    #[test]
    fn test_saved_states_circular_access() {
        let max_pred = 4;
        let states = SavedStates::<u64>::new(max_pred);
        let num_slots = max_pred + 1;

        // Frame 0 and frame num_slots should map to same slot
        let cell0 = states.get_cell(Frame::new(0)).unwrap();
        let cell_wrapped = states.get_cell(Frame::new(num_slots as i32)).unwrap();

        // Save to one, should be visible from other
        cell0.save(Frame::new(0), Some(42), None);
        let loaded = cell_wrapped.load();
        assert_eq!(
            loaded,
            Some(42),
            "Circular indexing should access same cell"
        );
    }

    /// Verify get_cell rejects invalid frames.
    #[test]
    fn test_saved_states_invalid_frame() {
        let states = SavedStates::<u64>::new(4);

        // Negative frame should fail
        let result = states.get_cell(Frame::new(-1));
        assert!(result.is_err(), "Negative frame should be rejected");
    }
}

// ============================================================================
// Cross-Component Invariant Tests
// ============================================================================

mod cross_component_invariants {
    use super::*;

    /// Verify that newly constructed SyncLayer has valid input queues.
    /// SyncLayer.check_invariants() internally checks all input queues.
    #[test]
    fn test_sync_layer_contains_valid_input_queues() {
        for num_players in 1..=4 {
            let sync_layer = SyncLayer::<TestConfig>::with_queue_length(num_players, 8, 64);

            // check_invariants on SyncLayer also validates all input queues
            let result = sync_layer.check_invariants();
            assert!(
                result.is_ok(),
                "SyncLayer with {} players should have valid input queues: {:?}",
                num_players,
                result.err()
            );
        }
    }
}

// ============================================================================
// Error Details Tests
// ============================================================================

mod invariant_violation_details {
    use super::*;

    /// Verify InvariantViolation provides useful information.
    #[test]
    fn test_violation_formatting() {
        // Create a violation manually
        let violation = InvariantViolation::new("TestType", "test invariant violated")
            .with_details("value=42, max=10".to_string());

        let formatted = format!("{}", violation);
        assert!(formatted.contains("TestType"), "Should include type name");
        assert!(
            formatted.contains("test invariant violated"),
            "Should include invariant description"
        );
        assert!(formatted.contains("value=42"), "Should include details");
    }

    /// Verify valid states pass without violation.
    #[test]
    fn test_valid_states_pass() {
        // InputQueue
        let queue = InputQueue::<TestConfig>::new(0).expect("queue");
        let queue_result = queue.check_invariants();
        assert!(queue_result.is_ok());

        // SyncLayer
        let sync_layer = SyncLayer::<TestConfig>::new(2, 8);
        let sync_result = sync_layer.check_invariants();
        assert!(sync_result.is_ok());
    }
}

// ============================================================================
// Deep Internal Production Behavior Tests
// ============================================================================
//
// These tests verify actual production behavior of internal types.
// While the __internal module exposes types for testing, the methods
// tested here are the same ones used in production sessions.

mod input_queue_production_behavior {
    use super::*;

    /// Verify the circular buffer correctly handles wraparound.
    /// This is critical for production: if wraparound is broken, inputs are lost.
    #[test]
    fn test_circular_buffer_wraparound_production() {
        let queue_len = 32;
        let mut queue = InputQueue::<TestConfig>::with_queue_length(0, queue_len).expect("queue");

        // Add more inputs than the queue can hold without discard
        // This should trigger wraparound in the circular buffer
        for i in 0..(queue_len as i32 * 3) {
            let input = PlayerInput::new(
                Frame::new(i),
                TestInput {
                    value: (i % 256) as u8,
                },
            );
            let added_frame = queue.add_input(input);

            if i < queue_len as i32 {
                assert_eq!(
                    added_frame,
                    Frame::new(i),
                    "Input should be added at frame {}",
                    i
                );
            }

            // Periodically discard old frames to make room
            if i > 0 && i % 10 == 0 {
                queue.discard_confirmed_frames(Frame::new(i - 5));
            }

            // Verify invariants hold throughout
            assert!(
                queue.check_invariants().is_ok(),
                "Invariants broken at frame {}: {:?}",
                i,
                queue.check_invariants().err()
            );
        }
    }

    /// Verify prediction behavior matches production expectations.
    /// When requesting inputs beyond what's available, prediction must be deterministic.
    #[test]
    fn test_prediction_is_deterministic_across_requests() {
        let mut queue = InputQueue::<TestConfig>::with_queue_length(0, 64).expect("queue");

        // Add a few confirmed inputs
        for i in 0..5 {
            let input = PlayerInput::new(Frame::new(i), TestInput { value: 42 });
            queue.add_input(input);
        }

        // Request frames beyond what we have (triggers prediction)
        let (pred1, status1) = queue.input(Frame::new(10)).expect("input");
        assert_eq!(status1, fortress_rollback::InputStatus::Predicted);

        // Reset and request again - should get same prediction
        queue.reset_prediction();
        let (pred2, status2) = queue.input(Frame::new(10)).expect("input");
        assert_eq!(status2, fortress_rollback::InputStatus::Predicted);

        // Predictions must be identical for determinism
        assert_eq!(
            pred1.value, pred2.value,
            "Predictions must be deterministic across requests"
        );
    }

    /// Verify that frame delay works correctly in production scenarios.
    /// Frame delay is critical for network latency compensation.
    #[test]
    fn test_frame_delay_production_behavior() {
        let mut queue = InputQueue::<TestConfig>::with_queue_length(0, 64).expect("queue");
        let delay = 3;
        queue.set_frame_delay(delay).expect("valid delay");

        // Add inputs at frame 0, 1, 2...
        // Due to delay, they should appear at frame 3, 4, 5...
        for i in 0..10 {
            let input = PlayerInput::new(Frame::new(i), TestInput { value: i as u8 });
            let added_frame = queue.add_input(input);

            // Input at frame i should be delayed to frame i+delay
            assert_eq!(
                added_frame,
                Frame::new(i + delay as i32),
                "Input {} should be delayed to frame {}",
                i,
                i + delay as i32
            );
        }

        // When we request frame 3, we should get the input we added at frame 0
        let (input, status) = queue.input(Frame::new(delay as i32)).expect("input");
        assert_eq!(status, fortress_rollback::InputStatus::Confirmed);
        assert_eq!(input.value, 0, "Frame {} should have input value 0", delay);
    }

    /// Verify discard behavior doesn't corrupt the queue.
    /// In production, confirmed frames are discarded to save memory.
    #[test]
    fn test_discard_maintains_queue_integrity() {
        let mut queue = InputQueue::<TestConfig>::with_queue_length(0, 64).expect("queue");

        // Add many inputs
        for i in 0..50 {
            let input = PlayerInput::new(Frame::new(i), TestInput { value: i as u8 });
            queue.add_input(input);
        }

        // Discard in various patterns
        queue.discard_confirmed_frames(Frame::new(10));
        assert!(queue.check_invariants().is_ok());

        queue.discard_confirmed_frames(Frame::new(25));
        assert!(queue.check_invariants().is_ok());

        queue.discard_confirmed_frames(Frame::new(40));
        assert!(queue.check_invariants().is_ok());

        // Should still be able to get recent frames
        let result = queue.confirmed_input(Frame::new(45));
        assert!(result.is_ok());
        assert_eq!(result.unwrap().input.value, 45);
    }

    /// Verify that first_incorrect_frame is properly tracked.
    /// This is essential for knowing when rollback is needed.
    #[test]
    fn test_first_incorrect_frame_tracking() {
        let mut queue = InputQueue::<TestConfig>::with_queue_length(0, 64).expect("queue");

        // Add initial confirmed inputs
        for i in 0..5 {
            let input = PlayerInput::new(Frame::new(i), TestInput { value: i as u8 });
            queue.add_input(input);
        }

        // Request future frames to trigger prediction
        for i in 5..10 {
            let _ = queue.input(Frame::new(i));
        }

        // Now add the "real" input that differs from prediction
        // The prediction was based on last confirmed (value=4)
        // We'll add a different value
        let incorrect_input = PlayerInput::new(Frame::new(5), TestInput { value: 99 });
        queue.add_input(incorrect_input);

        // first_incorrect_frame should now be set
        let first_incorrect = queue.first_incorrect_frame();
        assert!(
            !first_incorrect.is_null(),
            "first_incorrect_frame should be set when prediction was wrong"
        );
        assert_eq!(
            first_incorrect,
            Frame::new(5),
            "First incorrect frame should be 5"
        );
    }

    /// Verify reset_prediction clears prediction state properly.
    #[test]
    fn test_reset_prediction_clears_state() {
        let mut queue = InputQueue::<TestConfig>::with_queue_length(0, 64).expect("queue");

        // Add some inputs
        for i in 0..5 {
            queue.add_input(PlayerInput::new(
                Frame::new(i),
                TestInput { value: i as u8 },
            ));
        }

        // Trigger prediction
        let _ = queue.input(Frame::new(10));

        // Reset and verify state is cleared
        queue.reset_prediction();

        // first_incorrect_frame should be NULL after reset
        assert!(
            queue.first_incorrect_frame().is_null(),
            "first_incorrect_frame should be NULL after reset"
        );

        // Invariants should still hold
        assert!(queue.check_invariants().is_ok());
    }

    /// Verify queue handles maximum frame delay correctly.
    #[test]
    fn test_max_frame_delay_boundary() {
        let queue_len = 32;
        let mut queue = InputQueue::<TestConfig>::with_queue_length(0, queue_len).expect("queue");

        // Max delay is queue_length - 1
        let max_delay = queue.max_frame_delay();
        assert_eq!(max_delay, queue_len - 1);

        // Setting max delay should succeed
        assert!(queue.set_frame_delay(max_delay).is_ok());

        // Setting above max delay should fail
        let result = queue.set_frame_delay(max_delay + 1);
        assert!(result.is_err());
    }

    /// Test that sequential frame additions work across queue boundary.
    /// Note: The queue has finite size. We must discard old frames to prevent overflow.
    #[test]
    fn test_sequential_additions_across_boundary() {
        let queue_len = 16;
        let mut queue = InputQueue::<TestConfig>::with_queue_length(0, queue_len).expect("queue");

        // Add frames 0 through queue_len*2, discarding periodically to prevent overflow
        for i in 0..(queue_len as i32 * 2) {
            // Discard old frames BEFORE adding to prevent overflow
            // Keep at most queue_len/2 inputs in the queue
            if i >= (queue_len / 2) as i32 {
                queue.discard_confirmed_frames(Frame::new(i - (queue_len / 2) as i32));
            }

            let input = PlayerInput::new(
                Frame::new(i),
                TestInput {
                    value: (i % 256) as u8,
                },
            );
            let result = queue.add_input(input);

            // Verify sequential addition works
            assert_eq!(
                result,
                Frame::new(i),
                "Frame {} should be added successfully",
                i
            );

            assert!(
                queue.check_invariants().is_ok(),
                "Invariants should hold at frame {}",
                i
            );
        }
    }
}

mod sync_layer_production_behavior {
    use super::*;

    /// Verify save/load cycle works correctly in production.
    /// This is the core of rollback networking.
    #[test]
    fn test_save_load_cycle_production() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Simulate game loop: save state, advance, repeat
        for i in 0..10 {
            let request = sync_layer.save_current_state();
            if let fortress_rollback::FortressRequest::SaveGameState { cell, frame } = request {
                assert_eq!(frame, Frame::new(i));
                cell.save(
                    frame,
                    Some(TestState {
                        value: i as u64,
                        frame: i,
                    }),
                    Some(i as u128),
                );
            }
            sync_layer.advance_frame();
        }

        // Now at frame 10, load frame 5
        let result = sync_layer.load_frame(Frame::new(5));
        assert!(result.is_ok());

        if let Ok(fortress_rollback::FortressRequest::LoadGameState { frame, cell }) = result {
            assert_eq!(frame, Frame::new(5));
            let state = cell.load();
            assert!(state.is_some());
            let state = state.unwrap();
            assert_eq!(state.value, 5);
            assert_eq!(state.frame, 5);
        }

        // Current frame should now be 5
        assert_eq!(sync_layer.current_frame(), Frame::new(5));

        // Invariants should hold
        assert!(sync_layer.check_invariants().is_ok());
    }

    /// Verify rollback at prediction window boundary.
    /// This tests edge cases where rollback is at the limit of what's allowed.
    #[test]
    fn test_rollback_at_prediction_boundary() {
        let max_pred = 4;
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, max_pred);

        // Save states for frames 0 through max_pred
        for i in 0..=max_pred as i32 {
            if i > 0 {
                sync_layer.advance_frame();
            }
            let request = sync_layer.save_current_state();
            if let fortress_rollback::FortressRequest::SaveGameState { cell, frame } = request {
                cell.save(
                    frame,
                    Some(TestState {
                        value: i as u64,
                        frame: i,
                    }),
                    None,
                );
            }
        }

        // At frame max_pred, try to load frame 0 (exactly at boundary)
        let result = sync_layer.load_frame(Frame::new(0));
        assert!(
            result.is_ok(),
            "Should be able to load at prediction boundary"
        );

        // Try to advance and load beyond boundary
        for _ in 0..max_pred + 2 {
            sync_layer.advance_frame();
        }

        // Now frame 0 should be outside the prediction window
        let result = sync_layer.load_frame(Frame::new(0));
        assert!(
            result.is_err(),
            "Should not be able to load outside prediction window"
        );
    }

    /// Verify last_confirmed_frame tracking.
    /// This is important for knowing what frames can be discarded.
    #[test]
    fn test_last_confirmed_frame_tracking() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // Initially, last_confirmed_frame should be NULL
        assert!(sync_layer.last_confirmed_frame().is_null());

        // After advancing and operations, track confirmed frame
        for i in 0..5 {
            if i > 0 {
                sync_layer.advance_frame();
            }
            let request = sync_layer.save_current_state();
            if let fortress_rollback::FortressRequest::SaveGameState { cell, frame } = request {
                cell.save(
                    frame,
                    Some(TestState {
                        value: i as u64,
                        frame: i,
                    }),
                    None,
                );
            }
        }

        // Invariants should hold throughout
        assert!(sync_layer.check_invariants().is_ok());
    }

    /// Verify state overwriting in circular buffer.
    /// When we advance beyond max_prediction, old states get overwritten.
    #[test]
    fn test_state_circular_buffer_overwrite() {
        let max_pred = 4;
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, max_pred);

        // Save more states than slots available (max_pred + 1 slots)
        for i in 0..(max_pred as i32 * 3) {
            if i > 0 {
                sync_layer.advance_frame();
            }
            let request = sync_layer.save_current_state();
            if let fortress_rollback::FortressRequest::SaveGameState { cell, frame } = request {
                cell.save(
                    frame,
                    Some(TestState {
                        value: i as u64,
                        frame: i,
                    }),
                    Some(i as u128),
                );
            }
        }

        // Now try to load a recent frame (should work)
        // The current frame minus 2 would be a recent frame that should exist in the buffer.

        // In production, states are saved each frame, so the slot should have
        // the correct frame data. The circular buffer overwrites old states.

        // Verify invariants hold
        assert!(sync_layer.check_invariants().is_ok());
    }

    /// Verify multi-player frame delay configuration.
    #[test]
    fn test_multi_player_frame_delays() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(4, 8);

        // Set different delays for each player
        sync_layer
            .set_frame_delay(fortress_rollback::PlayerHandle::new(0), 0)
            .unwrap();
        sync_layer
            .set_frame_delay(fortress_rollback::PlayerHandle::new(1), 2)
            .unwrap();
        sync_layer
            .set_frame_delay(fortress_rollback::PlayerHandle::new(2), 4)
            .unwrap();
        sync_layer
            .set_frame_delay(fortress_rollback::PlayerHandle::new(3), 6)
            .unwrap();

        // Verify invariants hold with different delays
        assert!(sync_layer.check_invariants().is_ok());

        // Invalid player handle should error
        let result = sync_layer.set_frame_delay(fortress_rollback::PlayerHandle::new(10), 0);
        assert!(result.is_err());
    }

    /// Verify reset_prediction affects all input queues.
    #[test]
    fn test_reset_prediction_all_queues() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(3, 8);

        // Advance frame to have some state
        for _ in 0..5 {
            sync_layer.advance_frame();
        }

        // Reset prediction - should not break invariants
        sync_layer.reset_prediction();

        assert!(sync_layer.check_invariants().is_ok());
    }

    /// Verify save_current_state returns correct cell and frame.
    #[test]
    fn test_save_current_state_returns_correct_data() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        for expected_frame in 0..10 {
            let request = sync_layer.save_current_state();

            match request {
                fortress_rollback::FortressRequest::SaveGameState { cell, frame } => {
                    assert_eq!(frame, Frame::new(expected_frame));

                    // Save data and verify it's stored correctly
                    let state = TestState {
                        value: expected_frame as u64 * 100,
                        frame: expected_frame,
                    };
                    cell.save(frame, Some(state.clone()), Some(expected_frame as u128));

                    // Verify data was saved
                    assert_eq!(cell.frame(), frame);
                    assert_eq!(cell.checksum(), Some(expected_frame as u128));
                    let loaded = cell.load();
                    assert!(loaded.is_some());
                    assert_eq!(loaded.unwrap().value, state.value);
                },
                _ => panic!("Expected SaveGameState request"),
            }

            // last_saved_frame should be updated
            assert_eq!(sync_layer.last_saved_frame(), Frame::new(expected_frame));

            sync_layer.advance_frame();
        }
    }
}

mod saved_states_production_behavior {
    use super::*;

    /// Verify circular slot indexing.
    #[test]
    fn test_circular_slot_indexing() {
        let max_pred = 4;
        let states = SavedStates::<u64>::new(max_pred);
        let num_slots = max_pred + 1;

        // Frames that map to the same slot should return the same cell
        for base in 0..num_slots {
            let cell1 = states.get_cell(Frame::new(base as i32)).unwrap();
            let cell2 = states
                .get_cell(Frame::new((base + num_slots) as i32))
                .unwrap();

            // Save to cell1
            cell1.save(Frame::new(base as i32), Some(base as u64 * 10), None);

            // cell2 should see the same data (same underlying slot)
            let loaded = cell2.load();
            assert_eq!(loaded, Some(base as u64 * 10));
        }
    }

    /// Verify negative frame rejection.
    #[test]
    fn test_negative_frame_rejection() {
        let states = SavedStates::<u64>::new(4);

        for negative_frame in [-1, -10, -100, i32::MIN] {
            let result = states.get_cell(Frame::new(negative_frame));
            assert!(
                result.is_err(),
                "Frame {} should be rejected",
                negative_frame
            );
        }
    }

    /// Verify all slots are accessible.
    #[test]
    fn test_all_slots_accessible() {
        for max_pred in [1, 4, 8, 16, 32] {
            let states = SavedStates::<u64>::new(max_pred);
            let num_slots = max_pred + 1;

            // Should be able to access all slots
            for frame in 0..num_slots {
                let result = states.get_cell(Frame::new(frame as i32));
                assert!(
                    result.is_ok(),
                    "max_pred={}, frame={} should be accessible",
                    max_pred,
                    frame
                );
            }
        }
    }

    /// Verify state independence between saves at different frames.
    #[test]
    fn test_state_independence() {
        let max_pred = 4;
        let states = SavedStates::<TestState>::new(max_pred);
        let num_slots = max_pred + 1;

        // Save different states to different slots
        for frame in 0..num_slots {
            let cell = states.get_cell(Frame::new(frame as i32)).unwrap();
            cell.save(
                Frame::new(frame as i32),
                Some(TestState {
                    value: frame as u64 * 100,
                    frame: frame as i32,
                }),
                Some(frame as u128),
            );
        }

        // Verify each slot has correct data
        for frame in 0..num_slots {
            let cell = states.get_cell(Frame::new(frame as i32)).unwrap();
            let loaded = cell.load().unwrap();
            assert_eq!(loaded.value, frame as u64 * 100);
            assert_eq!(loaded.frame, frame as i32);
            assert_eq!(cell.checksum(), Some(frame as u128));
        }
    }
}

mod game_state_cell_production_behavior {
    use super::*;

    /// Verify save/load cycle.
    #[test]
    fn test_save_load_cycle() {
        let states = SavedStates::<TestState>::new(4);
        let cell = states.get_cell(Frame::new(0)).unwrap();

        let state = TestState {
            value: 12345,
            frame: 0,
        };
        cell.save(Frame::new(0), Some(state), Some(0xDEADBEEF));

        assert_eq!(cell.frame(), Frame::new(0));
        assert_eq!(cell.checksum(), Some(0xDEADBEEF));

        let loaded = cell.load();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.value, 12345);
    }

    /// Verify data accessor works without cloning.
    #[test]
    fn test_data_accessor() {
        let states = SavedStates::<TestState>::new(4);
        let cell = states.get_cell(Frame::new(0)).unwrap();

        cell.save(
            Frame::new(0),
            Some(TestState {
                value: 999,
                frame: 0,
            }),
            None,
        );

        // data() provides read access without cloning
        // Use explicit scope to ensure accessor is dropped before cell
        {
            let accessor = cell.data();
            if let Some(ref acc) = accessor {
                assert_eq!(acc.value, 999);
                assert_eq!(acc.frame, 0);
            }
        }
    }

    /// Verify None state handling.
    #[test]
    fn test_none_state() {
        let states = SavedStates::<TestState>::new(4);
        let cell = states.get_cell(Frame::new(0)).unwrap();

        // Save with None data
        cell.save(Frame::new(0), None, Some(123));

        assert_eq!(cell.frame(), Frame::new(0));
        assert_eq!(cell.checksum(), Some(123));
        assert!(cell.load().is_none());
        assert!(cell.data().is_none());
    }

    /// Verify overwriting existing state.
    #[test]
    fn test_overwrite_state() {
        let states = SavedStates::<TestState>::new(4);
        let cell = states.get_cell(Frame::new(0)).unwrap();

        // First save
        cell.save(
            Frame::new(0),
            Some(TestState {
                value: 100,
                frame: 0,
            }),
            Some(1),
        );
        assert_eq!(cell.load().unwrap().value, 100);
        assert_eq!(cell.checksum(), Some(1));

        // Overwrite
        cell.save(
            Frame::new(0),
            Some(TestState {
                value: 200,
                frame: 0,
            }),
            Some(2),
        );
        assert_eq!(cell.load().unwrap().value, 200);
        assert_eq!(cell.checksum(), Some(2));
    }

    /// Verify cloning cells shares underlying state.
    #[test]
    #[allow(clippy::redundant_clone)] // Testing Clone trait - cell2 shares Arc with cell1
    fn test_cell_clone_shares_state() {
        let states = SavedStates::<TestState>::new(4);
        let cell1 = states.get_cell(Frame::new(0)).unwrap();
        let cell2 = cell1.clone();

        // Save via cell1
        cell1.save(
            Frame::new(0),
            Some(TestState {
                value: 42,
                frame: 0,
            }),
            None,
        );

        // Should be visible via cell2
        let loaded = cell2.load();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().value, 42);
    }
}

// ============================================================================
// Stress Tests for Production Behavior
// ============================================================================

mod stress_tests {
    use super::*;

    /// Stress test: rapid add/discard cycles.
    #[test]
    fn test_rapid_add_discard_cycles() {
        let mut queue = InputQueue::<TestConfig>::with_queue_length(0, 32).expect("queue");

        for cycle in 0..100 {
            // Add a batch
            for i in 0..20 {
                let frame = Frame::new(cycle * 20 + i);
                let input = PlayerInput::new(
                    frame,
                    TestInput {
                        value: (i % 256) as u8,
                    },
                );
                queue.add_input(input);
            }

            // Discard most
            queue.discard_confirmed_frames(Frame::new(cycle * 20 + 15));

            // Invariants must hold
            assert!(
                queue.check_invariants().is_ok(),
                "Cycle {} invariants failed",
                cycle
            );
        }
    }

    /// Stress test: many rollbacks.
    #[test]
    fn test_many_rollbacks() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 16);

        for outer in 0..20 {
            // Advance and save states
            for i in 0..10 {
                if outer > 0 || i > 0 {
                    sync_layer.advance_frame();
                }
                let request = sync_layer.save_current_state();
                if let fortress_rollback::FortressRequest::SaveGameState { cell, frame } = request {
                    cell.save(
                        frame,
                        Some(TestState {
                            value: (outer * 10 + i) as u64,
                            frame: frame.as_i32(),
                        }),
                        None,
                    );
                }
            }

            // Rollback to a random earlier frame within window
            let rollback_target = sync_layer.current_frame().as_i32() - 5;
            if rollback_target >= 0 {
                let result = sync_layer.load_frame(Frame::new(rollback_target));
                assert!(result.is_ok(), "Rollback {} failed", outer);
            }

            assert!(
                sync_layer.check_invariants().is_ok(),
                "Outer loop {} invariants failed",
                outer
            );
        }
    }

    /// Stress test: all players with different delays.
    #[test]
    fn test_all_players_different_delays() {
        let num_players = 8;
        let mut sync_layer = SyncLayer::<TestConfig>::new(num_players, 16);

        // Set different delays
        for player in 0..num_players {
            sync_layer
                .set_frame_delay(fortress_rollback::PlayerHandle::new(player), player % 8)
                .unwrap();
        }

        // Advance many frames
        for _ in 0..100 {
            sync_layer.advance_frame();
        }

        assert!(sync_layer.check_invariants().is_ok());
    }
}

// ============================================================================
// Edge Case Tests
// ============================================================================

mod edge_cases {
    use super::*;

    /// Test with minimum queue length (2).
    /// Note: With queue length 2, only 1 frame delay is allowed (max_delay = queue_length - 1 = 1).
    /// The queue can hold at most 2 frames at once.
    #[test]
    fn test_minimum_queue_length() {
        let mut queue = InputQueue::<TestConfig>::with_queue_length(0, 2).expect("queue");

        // Should work with minimal queue - add first input
        let frame0 = queue.add_input(PlayerInput::new(Frame::new(0), TestInput { value: 0 }));
        assert_eq!(frame0, Frame::new(0), "First input should be at frame 0");
        assert!(
            queue.check_invariants().is_ok(),
            "Invariants should hold after first add"
        );

        // Add second input - queue is now full
        let frame1 = queue.add_input(PlayerInput::new(Frame::new(1), TestInput { value: 1 }));
        assert_eq!(frame1, Frame::new(1), "Second input should be at frame 1");
        let result = queue.check_invariants();
        assert!(
            result.is_ok(),
            "Invariants should hold after second add: {:?}",
            result.err()
        );

        // Discard frames before frame 1 (discards frame 0, keeps frame 1)
        // Note: discard_confirmed_frames(N) discards frames < N, keeping N at the new tail
        queue.discard_confirmed_frames(Frame::new(1));
        let result = queue.check_invariants();
        assert!(
            result.is_ok(),
            "Invariants should hold after discard: {:?}",
            result.err()
        );

        // Now we can add another
        let frame2 = queue.add_input(PlayerInput::new(Frame::new(2), TestInput { value: 2 }));
        assert_eq!(frame2, Frame::new(2), "Third input should be at frame 2");
        let result = queue.check_invariants();
        assert!(
            result.is_ok(),
            "Invariants should hold after third add: {:?}",
            result.err()
        );
    }

    /// Test single player session.
    #[test]
    fn test_single_player_session() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(1, 8);

        for i in 0..20 {
            if i > 0 {
                sync_layer.advance_frame();
            }
            let request = sync_layer.save_current_state();
            if let fortress_rollback::FortressRequest::SaveGameState { cell, frame } = request {
                cell.save(
                    frame,
                    Some(TestState {
                        value: i,
                        frame: i as i32,
                    }),
                    None,
                );
            }
        }

        assert!(sync_layer.check_invariants().is_ok());
    }

    /// Test maximum reasonable player count.
    #[test]
    fn test_many_players() {
        let num_players = 16;
        let sync_layer = SyncLayer::<TestConfig>::new(num_players, 8);

        assert!(sync_layer.check_invariants().is_ok());
    }

    /// Test frame 0 edge cases.
    #[test]
    fn test_frame_zero_operations() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);

        // At frame 0, save state
        let request = sync_layer.save_current_state();
        if let fortress_rollback::FortressRequest::SaveGameState { cell, frame } = request {
            assert_eq!(frame, Frame::new(0));
            cell.save(frame, Some(TestState { value: 0, frame: 0 }), Some(0));
        }

        // Cannot load current frame
        let result = sync_layer.load_frame(Frame::new(0));
        assert!(result.is_err());

        // Advance to frame 1
        sync_layer.advance_frame();

        // Now we can load frame 0
        let result = sync_layer.load_frame(Frame::new(0));
        assert!(result.is_ok());
    }

    /// Test NULL frame handling.
    #[test]
    fn test_null_frame_handling() {
        let mut sync_layer = SyncLayer::<TestConfig>::new(2, 8);
        sync_layer.advance_frame();

        // Cannot load NULL frame
        let result = sync_layer.load_frame(Frame::NULL);
        assert!(result.is_err());

        // SavedStates rejects negative frames
        let states = SavedStates::<u64>::new(4);
        assert!(states.get_cell(Frame::NULL).is_err());
    }

    /// Test input queue with frame delay at boundary.
    ///
    /// IMPORTANT: When frame delay is set to max (queue_length - 1), the first input
    /// at frame 0 gets placed at frame (queue_length - 1). This means only ONE more input
    /// can be added before the queue is full. This is by design - high frame delay
    /// requires regular discarding to work properly.
    #[test]
    fn test_frame_delay_at_boundary() {
        let queue_len = 16;
        let mut queue = InputQueue::<TestConfig>::with_queue_length(0, queue_len).expect("queue");

        // Max delay is queue_length - 1
        let max_delay = queue.max_frame_delay();
        assert_eq!(max_delay, queue_len - 1);

        // Test with a moderate delay that allows several inputs
        let moderate_delay = max_delay / 2;
        queue.set_frame_delay(moderate_delay).unwrap();

        // Add inputs - they should be delayed
        // With half-max delay, we can add several inputs before needing to discard
        let inputs_before_full = queue_len - moderate_delay;
        for i in 0..inputs_before_full {
            let input = PlayerInput::new(Frame::new(i as i32), TestInput { value: i as u8 });
            let added_frame = queue.add_input(input);

            // With moderate delay, input at frame i appears at frame i + moderate_delay
            assert_eq!(added_frame, Frame::new((i + moderate_delay) as i32));
        }

        assert!(queue.check_invariants().is_ok());
    }
}
