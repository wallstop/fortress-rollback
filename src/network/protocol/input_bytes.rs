//! Byte-encoded input data for network transmission.
//!
//! This module contains the internal `InputBytes` type used for serializing
//! and deserializing player inputs for network transmission.

use std::collections::BTreeMap;

use crate::frame_info::PlayerInput;
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
        match bincode::serialized_size(&T::Input::default()) {
            Ok(input_size) => {
                let size = (input_size as usize) * num_players;
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
        let mut bytes = Vec::new();
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

                if let Err(e) = bincode::serialize_into(&mut bytes, &input.input) {
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
        let mut player_inputs = Vec::new();

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
            let player_byte_slice = &self.bytes[start..end];
            match bincode::deserialize::<T::Input>(player_byte_slice) {
                Ok(input) => player_inputs.push(PlayerInput::new(self.frame, input)),
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
