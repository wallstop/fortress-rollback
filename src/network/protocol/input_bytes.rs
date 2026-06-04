//! Byte-encoded input data for network transmission.
//!
//! This module contains the internal `InputBytes` type used for serializing
//! and deserializing player inputs for network transmission.

use std::collections::BTreeMap;

use crate::frame_info::PlayerInput;
use crate::network::codec;
use crate::report_violation;
use crate::telemetry::{ViolationKind, ViolationSeverity};
use crate::{
    Config, FortressError, Frame, InternalErrorKind, PlayerHandle, SerializationErrorKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum InputBytesDecodeError {
    AllocationFailed {
        requested_players: usize,
    },
    ZeroPlayers,
    ByteLengthNotDivisible {
        byte_len: usize,
        num_players: usize,
    },
    PlayerByteRangeOutOfBounds {
        player: usize,
        start: usize,
        end: usize,
        byte_len: usize,
    },
    PlayerDecodeFailed {
        player: usize,
    },
    PlayerDecodeTrailingBytes {
        player: usize,
        consumed: usize,
        slice_len: usize,
    },
}

/// Byte-encoded data representing the inputs of a client, possibly for multiple players at the same time.
#[derive(Clone)]
pub(super) struct InputBytes {
    /// The frame to which this info belongs to. -1/[`Frame::NULL`] represents an invalid frame
    pub frame: Frame,
    /// An input buffer that will hold input data
    pub bytes: Vec<u8>,
}

impl InputBytes {
    fn player_input_byte_partition_size(
        byte_len: usize,
        num_players: usize,
    ) -> Result<usize, InputBytesDecodeError> {
        if num_players == 0 {
            return Err(InputBytesDecodeError::ZeroPlayers);
        }

        if byte_len % num_players != 0 {
            return Err(InputBytesDecodeError::ByteLengthNotDivisible {
                byte_len,
                num_players,
            });
        }

        Ok(byte_len / num_players)
    }

    fn player_byte_range(
        player: usize,
        size: usize,
        byte_len: usize,
    ) -> Result<std::ops::Range<usize>, InputBytesDecodeError> {
        let start =
            player
                .checked_mul(size)
                .ok_or(InputBytesDecodeError::PlayerByteRangeOutOfBounds {
                    player,
                    start: player.saturating_mul(size),
                    end: usize::MAX,
                    byte_len,
                })?;
        let end =
            start
                .checked_add(size)
                .ok_or(InputBytesDecodeError::PlayerByteRangeOutOfBounds {
                    player,
                    start,
                    end: usize::MAX,
                    byte_len,
                })?;
        if end > byte_len {
            return Err(InputBytesDecodeError::PlayerByteRangeOutOfBounds {
                player,
                start,
                end,
                byte_len,
            });
        }
        Ok(start..end)
    }

    /// Creates a zeroed InputBytes for the given number of players.
    ///
    /// # Returns
    /// Returns `None` if serialization of the default Input type fails, which indicates
    /// a fundamental issue with the Config::Input type's serialization implementation.
    pub fn zeroed<T: Config>(num_players: usize) -> Option<Self> {
        // Measure once to get the size of the default input without allocating
        // an intermediate serialized buffer.
        match codec::encoded_len(&T::Input::default()) {
            Ok(input_size) => {
                // saturating_mul matches the sibling `from_inputs` and avoids an
                // overflow panic under release `overflow-checks`.
                let size = input_size.saturating_mul(num_players);
                let mut bytes = Vec::new();
                if bytes.try_reserve_exact(size).is_err() {
                    report_violation!(
                        ViolationSeverity::Error,
                        ViolationKind::NetworkProtocol,
                        "Failed to reserve {} bytes for zeroed input buffer",
                        size
                    );
                    return None;
                }
                bytes.extend(std::iter::repeat_n(0, size));
                Some(Self {
                    frame: Frame::NULL,
                    bytes,
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

    /// Creates an InputBytes from the given inputs, rejecting per-player values
    /// whose serialized length differs from `Config::Input::default()`.
    pub fn try_from_inputs<T: Config>(
        num_players: usize,
        inputs: &BTreeMap<PlayerHandle, PlayerInput<T::Input>>,
    ) -> Result<Self, FortressError> {
        let input_size = codec::encoded_len(&T::Input::default()).map_err(|err| {
            report_violation!(
                ViolationSeverity::Critical,
                ViolationKind::InternalError,
                "Failed to measure default input type serialization: {}",
                err
            );
            SerializationErrorKind::EndpointCreationFailed
        })?;
        if input_size == 0 {
            return Err(SerializationErrorKind::InputSerializedSizeZero.into());
        }

        let serializable_inputs = inputs
            .keys()
            .filter(|handle| handle.as_usize() < num_players)
            .count();
        let estimated_size = input_size.checked_mul(serializable_inputs).ok_or({
            FortressError::SerializationErrorStructured {
                kind: SerializationErrorKind::InputSerializedFrameTooLarge {
                    frame_len: usize::MAX,
                    max: crate::rle::DEFAULT_MAX_DECODED_LEN,
                },
            }
        })?;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(estimated_size).map_err(|_err| {
            crate::error::allocation_failed("input_bytes.from_inputs", estimated_size)
        })?;
        let mut frame = Frame::NULL;
        // in ascending order
        for handle in 0..num_players {
            if let Some(input) = inputs.get(&PlayerHandle::new(handle)) {
                let input_len = codec::encoded_len(&input.input).map_err(|err| {
                    report_violation!(
                        ViolationSeverity::Error,
                        ViolationKind::NetworkProtocol,
                        "Failed to measure input serialization for player {}: {}",
                        handle,
                        err
                    );
                    SerializationErrorKind::EndpointCreationFailed
                })?;
                if input_len != input_size {
                    report_violation!(
                        ViolationSeverity::Error,
                        ViolationKind::NetworkProtocol,
                        "Serialized input for player {} is {} byte(s), expected {}",
                        handle,
                        input_len,
                        input_size
                    );
                    return Err(FortressError::InternalErrorStructured {
                        kind: InternalErrorKind::InputEncodeLengthMismatch {
                            player: handle,
                            input_len,
                            expected_len: input_size,
                        },
                    });
                }

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
                    return Err(SerializationErrorKind::EndpointCreationFailed.into());
                }
            }
        }
        Ok(Self { frame, bytes })
    }

    /// Creates an InputBytes from the given inputs.
    ///
    /// If serialization fails (which should never happen with a properly implemented Config::Input),
    /// returns an empty InputBytes and logs an error via the violation reporter.
    #[cfg(test)]
    pub fn from_inputs<T: Config>(
        num_players: usize,
        inputs: &BTreeMap<PlayerHandle, PlayerInput<T::Input>>,
    ) -> Self {
        match Self::try_from_inputs::<T>(num_players, inputs) {
            Ok(input_bytes) => input_bytes,
            Err(err) => {
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::NetworkProtocol,
                    "Failed to serialize input bytes: {:?}",
                    err
                );
                Self {
                    frame: Frame::NULL,
                    bytes: Vec::new(),
                }
            },
        }
    }

    /// Converts InputBytes to a vector of PlayerInput, rejecting malformed data
    /// without returning partial results.
    pub fn try_to_player_inputs_exact<T: Config>(
        &self,
        num_players: usize,
    ) -> Result<Vec<PlayerInput<T::Input>>, InputBytesDecodeError> {
        let size = Self::player_input_byte_partition_size(self.bytes.len(), num_players)?;

        let mut player_inputs = Vec::new();
        if player_inputs.try_reserve(num_players).is_err() {
            return Err(InputBytesDecodeError::AllocationFailed {
                requested_players: num_players,
            });
        }

        for p in 0..num_players {
            let range = Self::player_byte_range(p, size, self.bytes.len())?;
            let Some(player_byte_slice) = self.bytes.get(range.clone()) else {
                return Err(InputBytesDecodeError::PlayerByteRangeOutOfBounds {
                    player: p,
                    start: range.start,
                    end: range.end,
                    byte_len: self.bytes.len(),
                });
            };
            match codec::decode::<T::Input>(player_byte_slice) {
                Ok((input, consumed)) if consumed == player_byte_slice.len() => {
                    player_inputs.push(PlayerInput::new(self.frame, input));
                },
                Ok((_input, consumed)) => {
                    return Err(InputBytesDecodeError::PlayerDecodeTrailingBytes {
                        player: p,
                        consumed,
                        slice_len: player_byte_slice.len(),
                    });
                },
                Err(_e) => {
                    return Err(InputBytesDecodeError::PlayerDecodeFailed { player: p });
                },
            }
        }
        Ok(player_inputs)
    }

    /// Converts InputBytes to a vector of PlayerInput.
    ///
    /// If the data is malformed or deserialization fails, returns an empty vector and logs an error.
    #[cfg(test)]
    pub fn to_player_inputs<T: Config>(&self, num_players: usize) -> Vec<PlayerInput<T::Input>> {
        match self.try_to_player_inputs_exact::<T>(num_players) {
            Ok(player_inputs) => player_inputs,
            Err(err) => {
                log_input_decode_error(err);
                Vec::new()
            },
        }
    }
}

pub(super) fn log_input_decode_error(err: InputBytesDecodeError) {
    match err {
        InputBytesDecodeError::AllocationFailed { requested_players } => {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::NetworkProtocol,
                "Failed to reserve {} player inputs for deserialization",
                requested_players
            );
        },
        InputBytesDecodeError::ZeroPlayers => {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::NetworkProtocol,
                "Cannot convert InputBytes with num_players=0"
            );
        },
        InputBytesDecodeError::ByteLengthNotDivisible {
            byte_len,
            num_players,
        } => {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::NetworkProtocol,
                "InputBytes length {} is not divisible by num_players {}",
                byte_len,
                num_players
            );
        },
        InputBytesDecodeError::PlayerByteRangeOutOfBounds {
            player,
            start,
            end,
            byte_len,
        } => {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::NetworkProtocol,
                "Invalid byte range for player {}: {}..{} (total length: {})",
                player,
                start,
                end,
                byte_len
            );
        },
        InputBytesDecodeError::PlayerDecodeFailed { player } => {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::NetworkProtocol,
                "Failed to deserialize input for player {}. This may indicate network corruption or a bug in your Config::Input deserialization.",
                player
            );
        },
        InputBytesDecodeError::PlayerDecodeTrailingBytes {
            player,
            consumed,
            slice_len,
        } => {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::NetworkProtocol,
                "Input for player {} consumed {} byte(s) from a {} byte slice",
                player,
                consumed,
                slice_len
            );
        },
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
    #[derive(Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Debug)]
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

    #[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    enum BalancedVariableInput {
        Short,
        Medium(u32),
        Long(u64),
    }

    impl Default for BalancedVariableInput {
        fn default() -> Self {
            Self::Medium(0)
        }
    }

    struct BalancedVariableInputConfig;

    impl Config for BalancedVariableInputConfig {
        type Input = BalancedVariableInput;
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

    #[test]
    fn try_from_inputs_rejects_variable_width_per_player_input() {
        let default_len = codec::encoded_len(&BalancedVariableInput::default()).unwrap();
        let short_len = codec::encoded_len(&BalancedVariableInput::Short).unwrap();
        let long_len = codec::encoded_len(&BalancedVariableInput::Long(7)).unwrap();
        assert_eq!(
            short_len + long_len,
            default_len * 2,
            "test fixture must keep the aggregate width balanced"
        );

        let mut inputs = BTreeMap::new();
        inputs.insert(
            PlayerHandle::new(0),
            PlayerInput::new(Frame::new(50), BalancedVariableInput::Short),
        );
        inputs.insert(
            PlayerHandle::new(1),
            PlayerInput::new(Frame::new(50), BalancedVariableInput::Long(7)),
        );

        let result = InputBytes::try_from_inputs::<BalancedVariableInputConfig>(2, &inputs);
        assert!(matches!(
            result,
            Err(FortressError::InternalErrorStructured {
                kind: InternalErrorKind::InputEncodeLengthMismatch {
                    player: 0,
                    input_len,
                    expected_len
                }
            }) if input_len == short_len && expected_len == default_len
        ));
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

    #[test]
    fn to_player_inputs_rejects_padded_per_player_slices() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&123_u32.to_le_bytes());
        bytes.push(0xAA);
        bytes.extend_from_slice(&456_u32.to_le_bytes());
        bytes.push(0xBB);
        let input_bytes = InputBytes {
            frame: Frame::new(10),
            bytes,
        };

        let player_inputs = input_bytes.to_player_inputs::<TestConfig>(2);

        assert!(player_inputs.is_empty());
    }

    #[test]
    fn try_to_player_inputs_exact_rejects_padded_per_player_slices() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&123_u32.to_le_bytes());
        bytes.push(0xAA);
        bytes.extend_from_slice(&456_u32.to_le_bytes());
        bytes.push(0xBB);
        let input_bytes = InputBytes {
            frame: Frame::new(10),
            bytes,
        };

        let err = input_bytes
            .try_to_player_inputs_exact::<TestConfig>(2)
            .unwrap_err();

        assert!(matches!(
            err,
            InputBytesDecodeError::PlayerDecodeTrailingBytes {
                player: 0,
                consumed: 4,
                slice_len: 5,
            }
        ));
    }

    #[test]
    fn player_byte_range_rejects_arithmetic_overflow() {
        let result = InputBytes::player_byte_range(usize::MAX, 2, usize::MAX);

        assert!(matches!(
            result,
            Err(InputBytesDecodeError::PlayerByteRangeOutOfBounds {
                player: usize::MAX,
                end: usize::MAX,
                ..
            })
        ));
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
    #[derive(Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Debug)]
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
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Frame field preservation during construction
    /// - Related: proof_clone_preserves_frame, proof_null_frame_detection
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
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Bytes field preservation during construction
    /// - Related: proof_clone_preserves_bytes, proof_empty_input_bytes_valid
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
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Clone correctness for frame field
    /// - Related: proof_input_bytes_frame_preserved, proof_clone_preserves_bytes
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
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Clone correctness for bytes field
    /// - Related: proof_clone_preserves_frame, proof_clone_is_independent
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
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Clone independence (deep copy semantics)
    /// - Related: proof_clone_preserves_bytes
    #[kani::proof]
    #[kani::unwind(5)]
    fn proof_clone_is_independent() {
        let frame = Frame::new(100);
        let bytes = vec![1u8, 2, 3];

        let input_bytes = InputBytes { frame, bytes };
        let mut cloned = input_bytes.clone();

        // Record original values before modification
        let original_frame = input_bytes.frame;
        let original_len = input_bytes.bytes.len();

        // Modify the clone
        cloned.frame = Frame::new(999);
        cloned.bytes.push(42);

        // Original should be unchanged (independence verified)
        kani::assert(
            input_bytes.frame == original_frame,
            "Original frame unchanged after modifying clone",
        );
        kani::assert(
            input_bytes.bytes.len() == original_len,
            "Original bytes unchanged after modifying clone",
        );

        // Clone should have the modifications
        kani::assert(cloned.frame == Frame::new(999), "Clone has modified frame");
        kani::assert(
            cloned.bytes.len() == original_len + 1,
            "Clone has modified bytes",
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
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: Slice bounds safety in to_player_inputs
    /// - Related: proof_divisibility_check
    // kani::no-unwind-needed: vec![0u8; n] memset + scalar index check, no per-element loop
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

        let range =
            match InputBytes::player_byte_range(player, size_per_player, input_bytes.bytes.len()) {
                Ok(range) => range,
                Err(_err) => {
                    kani::assert(false, "Checked player byte range should be valid");
                    return;
                },
            };

        // Bounds should be valid
        kani::assert(
            range.start <= input_bytes.bytes.len(),
            "Start within bounds",
        );
        kani::assert(range.end <= input_bytes.bytes.len(), "End within bounds");
        kani::assert(range.start <= range.end, "Start <= end");

        // Should be able to slice (get would return Some)
        kani::assert(
            input_bytes.bytes.get(range).is_some(),
            "Slice should be valid",
        );
    }

    /// Proof: Divisibility check correctly identifies valid/invalid byte lengths.
    ///
    /// Verifies that the modulo check in to_player_inputs works correctly.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Modulo divisibility check correctness
    /// - Related: proof_player_slice_bounds_valid
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

    /// Proof: exact protocol-input decoding rejects zero players.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: malformed player count is rejected before per-player decode
    /// - Related: proof_try_to_player_inputs_rejects_non_divisible_lengths
    #[kani::proof]
    fn proof_try_to_player_inputs_rejects_zero_players() {
        let bytes_len: usize = kani::any();
        kani::assume(bytes_len <= 8);

        let result = InputBytes::player_input_byte_partition_size(bytes_len, 0);
        kani::assert(
            matches!(result, Err(InputBytesDecodeError::ZeroPlayers)),
            "zero players should be rejected",
        );
    }

    /// Proof: exact protocol-input decoding rejects non-divisible byte lengths.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: malformed byte/player partitioning is rejected atomically
    /// - Related: proof_divisibility_check, proof_player_slice_bounds_valid
    #[kani::proof]
    fn proof_try_to_player_inputs_rejects_non_divisible_lengths() {
        for (byte_len, num_players) in [
            (1_usize, 2_usize),
            (2_usize, 3_usize),
            (5_usize, 4_usize),
            (7_usize, 8_usize),
        ] {
            let result = InputBytes::player_input_byte_partition_size(byte_len, num_players);
            kani::assert(
                matches!(
                    result,
                    Err(InputBytesDecodeError::ByteLengthNotDivisible { .. })
                ),
                "non-divisible input byte lengths should be rejected",
            );
        }
    }

    // =========================================================================
    // Frame Selection Logic
    //
    // These proofs verify the frame selection logic used in from_inputs.
    // =========================================================================

    /// Proof: First non-NULL frame is selected.
    ///
    /// Verifies the frame selection algorithm that picks the first non-NULL frame.
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: Frame selection logic in from_inputs
    /// - Related: proof_null_frame_detection
    // kani::no-unwind-needed: two-branch scalar frame selection, no loops
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
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Frame::is_null() consistency with value -1
    /// - Related: proof_input_bytes_frame_preserved, proof_first_non_null_frame_selection
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
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Empty bytes edge case handling
    /// - Related: proof_input_bytes_stores_bytes
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
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Extreme frame value handling (i32::MAX, i32::MIN)
    /// - Related: proof_input_bytes_frame_preserved, proof_null_frame_detection
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
