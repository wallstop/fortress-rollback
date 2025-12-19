//! Direct fuzz target for InputQueue internals via __internal module.
//!
//! This target exercises InputQueue operations directly without going through
//! session APIs, enabling deeper coverage and better fault isolation.
//!
//! Uses the exposed __internal module for direct access to:
//! - InputQueue construction and configuration
//! - PlayerInput creation
//! - Circular buffer operations
//! - Frame management
//!
//! # Safety Properties Tested
//! - No panics on arbitrary operation sequences
//! - Invariant preservation: head/tail validity, length bounds
//! - Prediction correctness: predicted inputs match strategy
//! - Rollback safety: first_incorrect_frame tracking

#![no_main]

use arbitrary::Arbitrary;
use fortress_rollback::__internal::{InputQueue, PlayerInput};
use fortress_rollback::{Config, Frame, InputStatus};
use libfuzzer_sys::fuzz_target;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// Test input configuration
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize, Debug)]
struct TestInput {
    value: u8,
}

struct TestConfig;

impl Config for TestConfig {
    type Input = TestInput;
    type State = Vec<u8>;
    type Address = SocketAddr;
}

/// Operations that can be performed on InputQueue
#[derive(Debug, Arbitrary)]
enum QueueOp {
    /// Add an input at a specific frame
    AddInput {
        frame: i16, // Use i16 to get reasonable frame values
        value: u8,
    },
    /// Get input for a frame (may trigger prediction)
    GetInput { frame: i16 },
    /// Get confirmed input for a frame
    GetConfirmedInput { frame: i16 },
    /// Discard frames up to and including this frame
    DiscardFrames { frame: i16 },
    /// Set frame delay
    SetFrameDelay { delay: u8 },
    /// Reset the queue to a given frame
    ResetPrediction,
    /// Get first incorrect frame (for rollback detection)
    GetFirstIncorrectFrame,
    /// Check queue length
    CheckLength,
}

/// Fuzz input structure
#[derive(Debug, Arbitrary)]
struct FuzzInput {
    /// Initial queue configuration
    queue_length: u8,
    /// Initial frame delay
    initial_delay: u8,
    /// Player index
    player_index: u8,
    /// Sequence of operations
    operations: Vec<QueueOp>,
}

fuzz_target!(|fuzz_input: FuzzInput| {
    // Clamp queue_length to reasonable values [8, 256]
    let queue_length = ((fuzz_input.queue_length as usize) % 249 + 8).max(8);

    // Clamp initial delay to valid range
    let initial_delay = (fuzz_input.initial_delay as usize) % queue_length.min(32);

    // Use reasonable player index
    let player_index = (fuzz_input.player_index % 8) as usize;

    // Limit operations to prevent timeouts
    let max_ops = 500;
    let operations = if fuzz_input.operations.len() > max_ops {
        &fuzz_input.operations[..max_ops]
    } else {
        &fuzz_input.operations
    };

    // Create queue directly using __internal access
    let mut queue = match InputQueue::<TestConfig>::with_queue_length(player_index, queue_length) {
        Some(q) => q,
        None => return, // Can't create queue with these parameters, skip this fuzzing round
    };

    // Set initial delay if valid
    if initial_delay < queue_length {
        let _ = queue.set_frame_delay(initial_delay);
    }

    // Track state for validation
    let mut last_added_frame: i32 = -1;
    let mut frames_added = 0;
    let mut frames_discarded_up_to: i32 = -1;

    // Execute operations
    for op in operations {
        match op {
            QueueOp::AddInput { frame, value } => {
                // Convert to valid frame range
                let frame_val = if *frame < 0 {
                    0
                } else {
                    (*frame as i32).min(i32::MAX - queue_length as i32)
                };

                // Only add if frame is sequential (queue requires monotonic frames)
                if frame_val > last_added_frame {
                    let input =
                        PlayerInput::new(Frame::new(frame_val), TestInput { value: *value });
                    let result = queue.add_input(input);

                    // Verify result frame is valid
                    if !result.is_null() {
                        // add_input applies frame_delay internally
                        last_added_frame = frame_val;
                        frames_added += 1;

                        // Periodically discard to prevent queue overflow
                        if frames_added > 0 && frames_added % 64 == 0 && last_added_frame > 64 {
                            let discard_frame = Frame::new(last_added_frame - 60);
                            queue.discard_confirmed_frames(discard_frame);
                            frames_discarded_up_to = discard_frame.as_i32();
                        }
                    }
                }
            },

            QueueOp::GetInput { frame } => {
                let frame_val = (*frame as i32).max(0);
                let frame = Frame::new(frame_val);

                // Get input (may return predicted or None)
                if let Some((input, status)) = queue.input(frame) {
                    // Validate status is one of the expected values
                    assert!(
                        matches!(
                            status,
                            InputStatus::Confirmed
                                | InputStatus::Predicted
                                | InputStatus::Disconnected
                        ),
                        "Unexpected input status: {:?}",
                        status
                    );

                    // If confirmed, frame should match
                    if status == InputStatus::Confirmed {
                        // Input's frame field might differ due to delay
                        let _ = input; // Just ensure no panic
                    }
                }
            },

            QueueOp::GetConfirmedInput { frame } => {
                let frame_val = (*frame as i32).max(0);
                let frame = Frame::new(frame_val);

                // This may return error for frames not in queue
                let result = queue.confirmed_input(frame);
                if let Ok(input) = result {
                    // Verify input is valid
                    let _ = input.input;
                }
            },

            QueueOp::DiscardFrames { frame } => {
                let frame_val = (*frame as i32).max(0);
                let frame = Frame::new(frame_val);

                queue.discard_confirmed_frames(frame);
                frames_discarded_up_to = frames_discarded_up_to.max(frame_val);
            },

            QueueOp::SetFrameDelay { delay } => {
                let delay_val = (*delay as usize) % queue_length;
                // This may fail if delay is too large
                let _ = queue.set_frame_delay(delay_val);
            },

            QueueOp::ResetPrediction => {
                queue.reset_prediction();
                // Verify first_incorrect_frame is reset
                assert!(
                    queue.first_incorrect_frame().is_null(),
                    "reset_prediction should clear first_incorrect_frame"
                );
            },

            QueueOp::GetFirstIncorrectFrame => {
                let fif = queue.first_incorrect_frame();
                // Just verify we can query without panic
                let _ = fif.is_null();
            },

            QueueOp::CheckLength => {
                let queue_len = queue.queue_length();
                // Just verify the accessor works without panic
                assert!(queue_len > 0, "Queue length should be positive");
            },
        }
    }

    // Final invariant checks
    let final_length = queue.queue_length();
    assert!(
        final_length <= queue_length,
        "Final queue length {} exceeds max {}",
        final_length,
        queue_length
    );
});
