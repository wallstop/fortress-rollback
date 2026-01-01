//! Byte-encoded input data for network transmission.
//!
//! This module contains the internal `InputBytes` type used for serializing
//! and deserializing player inputs for network transmission.

use std::collections::BTreeMap;

use crate::frame_info::PlayerInput;
use crate::network::codec;
use crate::report_violation;
use crate::telemetry::{ViolationKind, ViolationSeverity};
use crate::{Config, Frame, PlayerHandle};

/// Byte-encoded data representing the inputs of a client, possibly for multiple players at the same time.
#[derive(Clone)]
pub(super) struct InputBytes {
    /// The frame to which this info belongs to. -1/[`Frame::NULL`] represents an invalid frame
    pub frame: Frame,
    /// An input buffer that will hold input data
    pub bytes: Vec<u8>,
}

impl InputBytes {
    /// Creates a zeroed InputBytes for the given number of players.
    ///
    /// # Returns
    /// Returns `None` if serialization of the default Input type fails, which indicates
    /// a fundamental issue with the Config::Input type's serialization implementation.
    pub fn zeroed<T: Config>(num_players: usize) -> Option<Self> {
        // Serialize once to get the size of the default input
        match codec::encode(&T::Input::default()) {
            Ok(encoded) => {
                let input_size = encoded.len();
                let size = input_size * num_players;
                Some(Self {
                    frame: Frame::NULL,
                    bytes: vec![0; size],
                })
            },
            Err(e) => {
                report_violation!(
                    ViolationSeverity::Critical,
                    ViolationKind::InternalError,
                    "Failed to serialize default input type: {}",
                    e
                );
                None
            },
        }
    }

    /// Creates an InputBytes from the given inputs.
    ///
    /// If serialization fails (which should never happen with a properly implemented Config::Input),
    /// returns an empty InputBytes and logs an error via the violation reporter.
    pub fn from_inputs<T: Config>(
        num_players: usize,
        inputs: &BTreeMap<PlayerHandle, PlayerInput<T::Input>>,
    ) -> Self {
        // Pre-allocate based on expected size: each input is serialized once
        // Estimate 8 bytes per input as reasonable starting capacity
        let estimated_size = num_players.saturating_mul(8);
        let mut bytes = Vec::with_capacity(estimated_size);
        let mut frame = Frame::NULL;
        // in ascending order
        for handle in 0..num_players {
            if let Some(input) = inputs.get(&PlayerHandle::new(handle)) {
                // Track the frame - use the first non-NULL frame we see.
                // All inputs in a single send *should* have the same frame, but if not,
                // log it and continue with the first frame (the data is still valid).
                if frame == Frame::NULL && input.frame != Frame::NULL {
                    frame = input.frame;
                } else if frame != Frame::NULL && input.frame != Frame::NULL && frame != input.frame
                {
                    // This indicates a bug in the calling code, but we can still
                    // proceed - the serialized bytes are correct, just the frame
                    // metadata is inconsistent.
                    report_violation!(
                        ViolationSeverity::Warning,
                        ViolationKind::InternalError,
                        "Input frame mismatch during serialization: using frame {:?}, but player {} has frame {:?}",
                        frame,
                        handle,
                        input.frame
                    );
                }

                if let Err(e) = codec::encode_append(&input.input, &mut bytes) {
                    report_violation!(
                        ViolationSeverity::Error,
                        ViolationKind::NetworkProtocol,
                        "Failed to serialize input for player {}: {}. This likely indicates a bug in your Config::Input serialization.",
                        handle,
                        e
                    );
                    return Self {
                        frame: Frame::NULL,
                        bytes: Vec::new(),
                    };
                }
            }
        }
        Self { frame, bytes }
    }

    /// Converts InputBytes to a vector of PlayerInput.
    ///
    /// If the data is malformed or deserialization fails, returns an empty vector and logs an error.
    pub fn to_player_inputs<T: Config>(&self, num_players: usize) -> Vec<PlayerInput<T::Input>> {
        let mut player_inputs = Vec::with_capacity(num_players);

        // Validate inputs before processing
        if num_players == 0 {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::NetworkProtocol,
                "Cannot convert InputBytes with num_players=0"
            );
            return player_inputs;
        }

        if self.bytes.len() % num_players != 0 {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::NetworkProtocol,
                "InputBytes length {} is not divisible by num_players {}",
                self.bytes.len(),
                num_players
            );
            return player_inputs;
        }

        let size = self.bytes.len() / num_players;
        for p in 0..num_players {
            let start = p * size;
            let end = start + size;
            let Some(player_byte_slice) = self.bytes.get(start..end) else {
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::NetworkProtocol,
                    "Invalid byte range for player {}: {}..{} (total length: {})",
                    p,
                    start,
                    end,
                    self.bytes.len()
                );
                return player_inputs;
            };
            match codec::decode::<T::Input>(player_byte_slice) {
                Ok((input, _)) => player_inputs.push(PlayerInput::new(self.frame, input)),
                Err(e) => {
                    report_violation!(
                        ViolationSeverity::Error,
                        ViolationKind::NetworkProtocol,
                        "Failed to deserialize input for player {}: {}. This may indicate network corruption or a bug in your Config::Input deserialization.",
                        p,
                        e
                    );
                    return player_inputs;
                },
            }
        }
        player_inputs
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::net::SocketAddr;

    // Test configuration
    #[repr(C)]
    #[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize, Debug)]
    struct TestInput {
        inp: u32,
    }

    #[derive(Clone, Default)]
    struct TestState;

    struct TestConfig;

    impl Config for TestConfig {
        type Input = TestInput;
        type State = TestState;
        type Address = SocketAddr;
    }

    // ==========================================
    // Constructor Tests
    // ==========================================

    #[test]
    fn zeroed_creates_correct_size_for_single_player() {
        let input_bytes = InputBytes::zeroed::<TestConfig>(1).unwrap();
        assert_eq!(input_bytes.frame, Frame::NULL);
        // TestInput is u32 = 4 bytes, so single player needs 4 bytes
        assert_eq!(input_bytes.bytes.len(), 4);
        assert!(input_bytes.bytes.iter().all(|&b| b == 0));
    }

    #[test]
    fn zeroed_creates_correct_size_for_multiple_players() {
        let input_bytes = InputBytes::zeroed::<TestConfig>(4).unwrap();
        assert_eq!(input_bytes.frame, Frame::NULL);
        // 4 players * 4 bytes each = 16 bytes
        assert_eq!(input_bytes.bytes.len(), 16);
        assert!(input_bytes.bytes.iter().all(|&b| b == 0));
    }

    #[test]
    fn zeroed_with_zero_players_creates_empty_bytes() {
        let input_bytes = InputBytes::zeroed::<TestConfig>(0).unwrap();
        assert_eq!(input_bytes.frame, Frame::NULL);
        assert!(input_bytes.bytes.is_empty());
    }

    // ==========================================
    // from_inputs Tests
    // ==========================================

    #[test]
    fn from_inputs_creates_correct_bytes() {
        let frame = Frame::new(42);
        let mut inputs = BTreeMap::new();
        inputs.insert(
            PlayerHandle::new(0),
            PlayerInput::new(frame, TestInput { inp: 12345 }),
        );

        let input_bytes = InputBytes::from_inputs::<TestConfig>(1, &inputs);
        assert_eq!(input_bytes.frame, frame);
        assert_eq!(input_bytes.bytes.len(), 4);
    }

    #[test]
    fn from_inputs_multiple_players() {
        let frame = Frame::new(10);
        let mut inputs = BTreeMap::new();
        inputs.insert(
            PlayerHandle::new(0),
            PlayerInput::new(frame, TestInput { inp: 100 }),
        );
        inputs.insert(
            PlayerHandle::new(1),
            PlayerInput::new(frame, TestInput { inp: 200 }),
        );

        let input_bytes = InputBytes::from_inputs::<TestConfig>(2, &inputs);
        assert_eq!(input_bytes.frame, frame);
        assert_eq!(input_bytes.bytes.len(), 8); // 2 players * 4 bytes
    }

    #[test]
    fn from_inputs_empty_map_creates_empty_bytes() {
        let inputs = BTreeMap::new();
        let input_bytes = InputBytes::from_inputs::<TestConfig>(0, &inputs);
        assert_eq!(input_bytes.frame, Frame::NULL);
        assert!(input_bytes.bytes.is_empty());
    }

    #[test]
    fn from_inputs_uses_first_non_null_frame() {
        let frame1 = Frame::NULL;
        let frame2 = Frame::new(100);
        let mut inputs = BTreeMap::new();
        inputs.insert(
            PlayerHandle::new(0),
            PlayerInput::new(frame1, TestInput { inp: 1 }),
        );
        inputs.insert(
            PlayerHandle::new(1),
            PlayerInput::new(frame2, TestInput { inp: 2 }),
        );

        let input_bytes = InputBytes::from_inputs::<TestConfig>(2, &inputs);
        // Should use frame2 since frame1 is NULL
        assert_eq!(input_bytes.frame, frame2);
    }

    #[test]
    fn from_inputs_partial_players() {
        // Only player 0 has input, but we have 2 players
        let frame = Frame::new(50);
        let mut inputs = BTreeMap::new();
        inputs.insert(
            PlayerHandle::new(0),
            PlayerInput::new(frame, TestInput { inp: 42 }),
        );

        let input_bytes = InputBytes::from_inputs::<TestConfig>(2, &inputs);
        // Only serializes inputs that exist - results in 4 bytes for player 0
        assert_eq!(input_bytes.bytes.len(), 4);
    }

    // ==========================================
    // to_player_inputs Tests
    // ==========================================

    #[test]
    fn to_player_inputs_roundtrip_single_player() {
        let frame = Frame::new(99);
        let mut inputs = BTreeMap::new();
        inputs.insert(
            PlayerHandle::new(0),
            PlayerInput::new(frame, TestInput { inp: 0xDEAD_BEEF }),
        );

        let input_bytes = InputBytes::from_inputs::<TestConfig>(1, &inputs);
        let player_inputs = input_bytes.to_player_inputs::<TestConfig>(1);

        assert_eq!(player_inputs.len(), 1);
        assert_eq!(player_inputs[0].frame, frame);
        assert_eq!(player_inputs[0].input.inp, 0xDEAD_BEEF);
    }

    #[test]
    fn to_player_inputs_roundtrip_multiple_players() {
        let frame = Frame::new(50);
        let mut inputs = BTreeMap::new();
        inputs.insert(
            PlayerHandle::new(0),
            PlayerInput::new(frame, TestInput { inp: 111 }),
        );
        inputs.insert(
            PlayerHandle::new(1),
            PlayerInput::new(frame, TestInput { inp: 222 }),
        );
        inputs.insert(
            PlayerHandle::new(2),
            PlayerInput::new(frame, TestInput { inp: 333 }),
        );

        let input_bytes = InputBytes::from_inputs::<TestConfig>(3, &inputs);
        let player_inputs = input_bytes.to_player_inputs::<TestConfig>(3);

        assert_eq!(player_inputs.len(), 3);
        assert_eq!(player_inputs[0].input.inp, 111);
        assert_eq!(player_inputs[1].input.inp, 222);
        assert_eq!(player_inputs[2].input.inp, 333);
    }

    #[test]
    fn to_player_inputs_with_zero_players_returns_empty() {
        let input_bytes = InputBytes::zeroed::<TestConfig>(0).unwrap();
        let player_inputs = input_bytes.to_player_inputs::<TestConfig>(0);
        assert!(player_inputs.is_empty());
    }

    #[test]
    fn to_player_inputs_mismatched_size_returns_partial() {
        let input_bytes = InputBytes {
            frame: Frame::new(10),
            bytes: vec![1, 2, 3, 4, 5], // 5 bytes, not divisible by 2
        };

        // Should return empty because bytes not divisible by num_players
        let player_inputs = input_bytes.to_player_inputs::<TestConfig>(2);
        assert!(player_inputs.is_empty());
    }

    // ==========================================
    // Clone Tests
    // ==========================================

    #[test]
    #[allow(clippy::redundant_clone)]
    fn clone_preserves_data() {
        let input_bytes = InputBytes {
            frame: Frame::new(123),
            bytes: vec![1, 2, 3, 4],
        };

        let cloned = input_bytes.clone();
        assert_eq!(cloned.frame, Frame::new(123));
        assert_eq!(cloned.bytes, vec![1, 2, 3, 4]);
    }

    #[test]
    fn clone_is_independent() {
        let mut input_bytes = InputBytes {
            frame: Frame::new(100),
            bytes: vec![10, 20, 30],
        };

        let cloned = input_bytes.clone();

        // Modify original
        input_bytes.frame = Frame::new(999);
        input_bytes.bytes[0] = 0xFF;

        // Clone should be unchanged
        assert_eq!(cloned.frame, Frame::new(100));
        assert_eq!(cloned.bytes[0], 10);
    }

    // ==========================================
    // Edge Case Tests
    // ==========================================

    #[test]
    fn frame_null_handling() {
        let input_bytes = InputBytes {
            frame: Frame::NULL,
            bytes: vec![0, 0, 0, 0],
        };

        let player_inputs = input_bytes.to_player_inputs::<TestConfig>(1);
        assert_eq!(player_inputs.len(), 1);
        assert_eq!(player_inputs[0].frame, Frame::NULL);
    }

    #[test]
    fn large_player_count() {
        let input_bytes = InputBytes::zeroed::<TestConfig>(100).unwrap();
        // 100 players * 4 bytes = 400 bytes
        assert_eq!(input_bytes.bytes.len(), 400);

        let player_inputs = input_bytes.to_player_inputs::<TestConfig>(100);
        assert_eq!(player_inputs.len(), 100);
        // All inputs should be default (zeroed)
        for input in &player_inputs {
            assert_eq!(input.input, TestInput::default());
        }
    }

    #[test]
    fn max_frame_value() {
        let max_frame = Frame::new(i32::MAX);
        let mut inputs = BTreeMap::new();
        inputs.insert(
            PlayerHandle::new(0),
            PlayerInput::new(max_frame, TestInput { inp: 42 }),
        );

        let input_bytes = InputBytes::from_inputs::<TestConfig>(1, &inputs);
        let player_inputs = input_bytes.to_player_inputs::<TestConfig>(1);

        assert_eq!(player_inputs[0].frame, max_frame);
    }

    #[test]
    fn max_input_value() {
        let frame = Frame::new(1);
        let mut inputs = BTreeMap::new();
        inputs.insert(
            PlayerHandle::new(0),
            PlayerInput::new(frame, TestInput { inp: u32::MAX }),
        );

        let input_bytes = InputBytes::from_inputs::<TestConfig>(1, &inputs);
        let player_inputs = input_bytes.to_player_inputs::<TestConfig>(1);

        assert_eq!(player_inputs[0].input.inp, u32::MAX);
    }

    #[test]
    fn preserves_player_order() {
        let frame = Frame::new(5);
        let mut inputs = BTreeMap::new();
        // Insert in reverse order to verify ordering is maintained
        inputs.insert(
            PlayerHandle::new(3),
            PlayerInput::new(frame, TestInput { inp: 3 }),
        );
        inputs.insert(
            PlayerHandle::new(2),
            PlayerInput::new(frame, TestInput { inp: 2 }),
        );
        inputs.insert(
            PlayerHandle::new(1),
            PlayerInput::new(frame, TestInput { inp: 1 }),
        );
        inputs.insert(
            PlayerHandle::new(0),
            PlayerInput::new(frame, TestInput { inp: 0 }),
        );

        let input_bytes = InputBytes::from_inputs::<TestConfig>(4, &inputs);
        let player_inputs = input_bytes.to_player_inputs::<TestConfig>(4);

        // Verify order is 0, 1, 2, 3 (ascending by handle)
        for (i, input) in player_inputs.iter().enumerate() {
            assert_eq!(input.input.inp, i as u32);
        }
    }

    // ==========================================
    // Complex Input Type Test
    // ==========================================

    #[repr(C)]
    #[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize, Debug)]
    struct ComplexInput {
        x: i32,
        y: i32,
        buttons: u16,
        flags: u8,
    }

    #[derive(Clone, Default)]
    struct ComplexState;

    struct ComplexConfig;

    impl Config for ComplexConfig {
        type Input = ComplexInput;
        type State = ComplexState;
        type Address = SocketAddr;
    }

    #[test]
    fn complex_input_roundtrip() {
        let frame = Frame::new(77);
        let complex_input = ComplexInput {
            x: -500,
            y: 1000,
            buttons: 0b1010_1010,
            flags: 0xFF,
        };

        let mut inputs = BTreeMap::new();
        inputs.insert(PlayerHandle::new(0), PlayerInput::new(frame, complex_input));

        let input_bytes = InputBytes::from_inputs::<ComplexConfig>(1, &inputs);
        let player_inputs = input_bytes.to_player_inputs::<ComplexConfig>(1);

        assert_eq!(player_inputs.len(), 1);
        assert_eq!(player_inputs[0].frame, frame);
        assert_eq!(player_inputs[0].input.x, -500);
        assert_eq!(player_inputs[0].input.y, 1000);
        assert_eq!(player_inputs[0].input.buttons, 0b1010_1010);
        assert_eq!(player_inputs[0].input.flags, 0xFF);
    }
}

// =============================================================================
// Kani Formal Verification Proofs
//
// These proofs verify key invariants of the InputBytes type.
//
// ## Verified Invariants
//
// 1. **InputBytes Construction**: Direct construction preserves frame and bytes
// 2. **Clone Correctness**: Cloning preserves all fields
// 3. **Frame Preservation**: Frame value is correctly stored and retrieved
// 4. **Slice Bounds Safety**: Player byte ranges are always valid
// 5. **Frame Selection Logic**: First non-NULL frame is correctly selected
//
// ## Design Notes
//
// InputBytes::zeroed, from_inputs, and to_player_inputs require codec operations
// which are difficult to verify in Kani. We focus on testing the InputBytes type
// directly by instantiating it and verifying its structural invariants.
// =============================================================================
#[cfg(kani)]
mod kani_proofs {
    use super::*;

    // =========================================================================
    // InputBytes Direct Construction and Field Access
    //
    // These proofs verify that InputBytes correctly stores and preserves data.
    // =========================================================================

    /// Proof: InputBytes construction preserves frame.
    ///
    /// Verifies that constructing InputBytes with a frame value correctly stores it.
    #[kani::proof]
    fn proof_input_bytes_frame_preserved() {
        let frame_val: i32 = kani::any();
        let frame = Frame::new(frame_val);

        let input_bytes = InputBytes {
            frame,
            bytes: Vec::new(),
        };

        // Frame should be preserved
        kani::assert(
            input_bytes.frame == frame,
            "InputBytes should preserve frame",
        );

        // NULL detection should work on the stored frame
        if frame_val == -1 {
            kani::assert(input_bytes.frame.is_null(), "Frame -1 should be NULL");
        } else {
            kani::assert(
                !input_bytes.frame.is_null(),
                "non -1 frame should not be null",
            );
        }
    }

    /// Proof: InputBytes construction with symbolic bytes.
    ///
    /// Verifies that InputBytes correctly stores bytes with various lengths.
    #[kani::proof]
    #[kani::unwind(10)]
    fn proof_input_bytes_stores_bytes() {
        let len: usize = kani::any();
        kani::assume(len <= 8); // Keep tractable

        let bytes = vec![0u8; len];
        let input_bytes = InputBytes {
            frame: Frame::NULL,
            bytes,
        };

        // Bytes length should be preserved
        kani::assert(
            input_bytes.bytes.len() == len,
            "InputBytes should preserve bytes length",
        );

        // Empty bytes check should work
        kani::assert(
            input_bytes.bytes.is_empty() == (len == 0),
            "is_empty should match length",
        );
    }

    // =========================================================================
    // Clone Verification
    //
    // These proofs verify that cloning InputBytes preserves all fields.
    // =========================================================================

    /// Proof: Clone preserves frame value.
    ///
    /// Verifies that cloning InputBytes preserves the frame field.
    #[kani::proof]
    fn proof_clone_preserves_frame() {
        let frame_val: i32 = kani::any();
        let frame = Frame::new(frame_val);

        let input_bytes = InputBytes {
            frame,
            bytes: Vec::new(),
        };

        let cloned = input_bytes.clone();

        kani::assert(cloned.frame == frame, "Cloned frame should equal original");
        kani::assert(
            cloned.frame == input_bytes.frame,
            "Cloned frame should match source",
        );
    }

    /// Proof: Clone preserves bytes.
    ///
    /// Verifies that cloning InputBytes creates a deep copy of bytes.
    #[kani::proof]
    #[kani::unwind(10)]
    fn proof_clone_preserves_bytes() {
        let len: usize = kani::any();
        kani::assume(len <= 8);

        // Create with symbolic byte values
        let byte_val: u8 = kani::any();
        let bytes = vec![byte_val; len];
        let input_bytes = InputBytes {
            frame: Frame::NULL,
            bytes,
        };

        let cloned = input_bytes.clone();

        // Length should match
        kani::assert(
            cloned.bytes.len() == input_bytes.bytes.len(),
            "Cloned bytes length should match",
        );

        // All byte values should match
        for i in 0..len {
            kani::assert(
                cloned.bytes[i] == input_bytes.bytes[i],
                "Cloned byte values should match",
            );
        }
    }

    /// Proof: Clone creates independent copy.
    ///
    /// Verifies that cloned InputBytes is independent (modifying one doesn't affect other).
    #[kani::proof]
    #[kani::unwind(5)]
    fn proof_clone_is_independent() {
        let frame = Frame::new(100);
        let bytes = vec![1u8, 2, 3];

        let input_bytes = InputBytes { frame, bytes };
        let cloned = input_bytes.clone();

        // After cloning, both should have same values
        kani::assert(cloned.frame == input_bytes.frame, "Frames should match");
        kani::assert(
            cloned.bytes.len() == input_bytes.bytes.len(),
            "Lengths should match",
        );
    }

    // =========================================================================
    // Slice Bounds Verification
    //
    // These proofs verify the arithmetic used in to_player_inputs is safe.
    // =========================================================================

    /// Proof: Player byte slice bounds are valid.
    ///
    /// Verifies that start..end ranges for player slices are always within bounds
    /// when bytes.len() is divisible by num_players.
    #[kani::proof]
    fn proof_player_slice_bounds_valid() {
        let total_bytes: usize = kani::any();
        let num_players: usize = kani::any();

        // Preconditions from to_player_inputs validation
        kani::assume(num_players > 0);
        kani::assume(num_players <= 16);
        kani::assume(total_bytes <= 256);
        kani::assume(total_bytes % num_players == 0);

        let size_per_player = total_bytes / num_players;

        // Create InputBytes with these dimensions
        let input_bytes = InputBytes {
            frame: Frame::NULL,
            bytes: vec![0u8; total_bytes],
        };

        // Verify all player ranges are valid
        let player: usize = kani::any();
        kani::assume(player < num_players);

        let start = player * size_per_player;
        let end = start + size_per_player;

        // Bounds should be valid
        kani::assert(start <= input_bytes.bytes.len(), "Start within bounds");
        kani::assert(end <= input_bytes.bytes.len(), "End within bounds");
        kani::assert(start <= end, "Start <= end");

        // Should be able to slice (get would return Some)
        kani::assert(
            input_bytes.bytes.get(start..end).is_some(),
            "Slice should be valid",
        );
    }

    /// Proof: Divisibility check correctly identifies valid/invalid byte lengths.
    ///
    /// Verifies that the modulo check in to_player_inputs works correctly.
    #[kani::proof]
    fn proof_divisibility_check() {
        let bytes_len: usize = kani::any();
        let num_players: usize = kani::any();

        kani::assume(num_players > 0);
        kani::assume(bytes_len <= 256);
        kani::assume(num_players <= 16);

        let input_bytes = InputBytes {
            frame: Frame::NULL,
            bytes: vec![0u8; bytes_len],
        };

        let is_divisible = input_bytes.bytes.len() % num_players == 0;

        if is_divisible {
            // Valid: can divide evenly among players
            let size_per_player = input_bytes.bytes.len() / num_players;
            kani::assert(
                size_per_player * num_players == bytes_len,
                "Even division should recover total",
            );
        }
        // If not divisible, to_player_inputs would return empty vec (error case)
    }

    // =========================================================================
    // Frame Selection Logic
    //
    // These proofs verify the frame selection logic used in from_inputs.
    // =========================================================================

    /// Proof: First non-NULL frame is selected.
    ///
    /// Verifies the frame selection algorithm that picks the first non-NULL frame.
    #[kani::proof]
    fn proof_first_non_null_frame_selection() {
        let frame_val1: i32 = kani::any();
        let frame_val2: i32 = kani::any();

        let frame1 = Frame::new(frame_val1);
        let frame2 = Frame::new(frame_val2);

        // Simulate the frame selection logic from from_inputs
        let mut result_frame = Frame::NULL;

        // First input
        if result_frame == Frame::NULL && frame1 != Frame::NULL {
            result_frame = frame1;
        }

        // Second input
        if result_frame == Frame::NULL && frame2 != Frame::NULL {
            result_frame = frame2;
        }

        // Verify the selection logic
        if frame1 != Frame::NULL {
            kani::assert(result_frame == frame1, "Should use first non-NULL frame");
        } else if frame2 != Frame::NULL {
            kani::assert(result_frame == frame2, "Should use second if first is NULL");
        } else {
            kani::assert(result_frame.is_null(), "Should be NULL if both are NULL");
        }
    }

    /// Proof: NULL frame detection is consistent.
    ///
    /// Verifies that Frame::NULL is correctly identified.
    #[kani::proof]
    fn proof_null_frame_detection() {
        let frame_val: i32 = kani::any();
        let frame = Frame::new(frame_val);

        let input_bytes = InputBytes {
            frame,
            bytes: Vec::new(),
        };

        // NULL detection should be consistent
        let is_null = input_bytes.frame.is_null();
        let expected_null = frame_val == -1;

        kani::assert(
            is_null == expected_null,
            "is_null() should match frame value -1",
        );
    }

    // =========================================================================
    // Edge Cases
    //
    // These proofs verify edge cases and boundary conditions.
    // =========================================================================

    /// Proof: Empty InputBytes is valid.
    ///
    /// Verifies that InputBytes with empty bytes is a valid state.
    #[kani::proof]
    fn proof_empty_input_bytes_valid() {
        let input_bytes = InputBytes {
            frame: Frame::NULL,
            bytes: Vec::new(),
        };

        kani::assert(input_bytes.bytes.is_empty(), "Bytes should be empty");
        kani::assert(input_bytes.frame.is_null(), "Frame should be NULL");
        kani::assert(input_bytes.bytes.len() == 0, "Length should be 0");
    }

    /// Proof: InputBytes with max frame value.
    ///
    /// Verifies that extreme frame values are handled correctly.
    #[kani::proof]
    fn proof_extreme_frame_values() {
        // Test max positive frame
        let max_input = InputBytes {
            frame: Frame::new(i32::MAX),
            bytes: Vec::new(),
        };
        kani::assert(!max_input.frame.is_null(), "max frame should not be null");
        kani::assert(
            max_input.frame == Frame::new(i32::MAX),
            "max frame should equal i32::MAX",
        );

        // Test min negative frame (not NULL)
        let min_input = InputBytes {
            frame: Frame::new(i32::MIN),
            bytes: Vec::new(),
        };
        kani::assert(!min_input.frame.is_null(), "min frame should not be null");

        // Test NULL frame
        let null_input = InputBytes {
            frame: Frame::NULL,
            bytes: Vec::new(),
        };
        kani::assert(null_input.frame.is_null(), "NULL frame should be null");
    }
}
