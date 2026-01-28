//! State checksum utilities for rollback networking.
//!
//! This module provides deterministic checksum computation for game states,
//! essential for desync detection in peer-to-peer rollback networking.
//!
//! # Why Checksums?
//!
//! In rollback networking, peers periodically exchange state checksums to verify
//! they are simulating the same game state. If checksums differ, a desync has
//! occurred, allowing early detection and potential recovery.
//!
//! # Determinism Requirements
//!
//! For checksums to be useful, they must be **deterministic across all peers**:
//!
//! - Same state → same serialized bytes → same checksum
//! - Serialization must be platform-independent (fixed-size integers)
//! - Hash algorithm must be deterministic (no random seeds)
//!
//! # Usage
//!
//! The [`compute_checksum`] function provides a one-liner for state checksums:
//!
//! ```
//! use fortress_rollback::checksum::compute_checksum;
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Serialize, Deserialize)]
//! struct GameState {
//!     frame: u32,
//!     player_x: f32,
//!     player_y: f32,
//! }
//!
//! let state = GameState { frame: 100, player_x: 1.0, player_y: 2.0 };
//! let checksum = compute_checksum(&state)?;
//!
//! // Same state produces same checksum
//! let checksum2 = compute_checksum(&state)?;
//! assert_eq!(checksum, checksum2);
//! # Ok::<(), fortress_rollback::checksum::ChecksumError>(())
//! ```
//!
//! # Integration with `SaveGameState`
//!
//! Use checksums when handling [`FortressRequest::SaveGameState`]:
//!
//! ```ignore
//! use fortress_rollback::{FortressRequest, checksum::compute_checksum};
//!
//! match request {
//!     FortressRequest::SaveGameState { cell, frame } => {
//!         let checksum = compute_checksum(&game_state).ok();
//!         cell.save(frame, Some(game_state.clone()), checksum);
//!     }
//!     // ...
//! }
//! ```
//!
//! # Performance Considerations
//!
//! Computing checksums requires serializing the entire game state, which can be
//! expensive for large states. Consider:
//!
//! - **Checksum frequency**: Not every frame needs a checksum. Common patterns:
//!   - Every N frames (e.g., every 60 frames = once per second at 60 FPS)
//!   - Only for confirmed frames (after receiving remote inputs)
//!
//! - **State design**: Keep frequently-checksummed state minimal. Separate
//!   cosmetic state (particles, animations) from deterministic state (positions,
//!   health).
//!
//! # Alternative: Fletcher-16
//!
//! For a simpler, faster checksum (at the cost of collision resistance), use
//! [`fletcher16`]. This is commonly used in game networking:
//!
//! ```
//! use fortress_rollback::checksum::{compute_checksum_fletcher16, fletcher16};
//!
//! // Direct computation on serializable state
//! # use serde::{Serialize, Deserialize};
//! # #[derive(Serialize)]
//! # struct GameState { frame: u32 }
//! # let state = GameState { frame: 100 };
//! let checksum = compute_checksum_fletcher16(&state).expect("should serialize");
//!
//! // Or compute on raw bytes
//! let bytes = b"game state bytes";
//! let checksum_u16 = fletcher16(bytes);
//! ```
//!
//! [`FortressRequest::SaveGameState`]: crate::FortressRequest::SaveGameState

use crate::hash::DeterministicHasher;
use crate::network::codec::{encode, CodecError};
use serde::Serialize;
use std::hash::Hasher;

/// Computes a deterministic `u128` checksum of a serializable game state.
///
/// This function:
/// 1. Serializes the state using bincode with fixed-integer encoding
/// 2. Hashes the serialized bytes using FNV-1a
/// 3. Returns the hash as `u128` (matching [`GameStateCell::save`] signature)
///
/// # Returns
///
/// - `Ok(u128)` - The computed checksum
/// - `Err(ChecksumError)` - If serialization fails
///
/// # Determinism
///
/// The checksum is deterministic across:
/// - Multiple calls with the same state
/// - Different processes
/// - Different platforms (using fixed-integer encoding)
///
/// # Example
///
/// ```
/// use fortress_rollback::checksum::{compute_checksum, ChecksumError};
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct State {
///     frame: u32,
///     position: (f32, f32),
/// }
///
/// let state = State { frame: 42, position: (1.0, 2.0) };
/// let checksum = compute_checksum(&state)?;
///
/// // Deterministic: same state = same checksum
/// assert_eq!(checksum, compute_checksum(&state)?);
/// # Ok::<(), ChecksumError>(())
/// ```
///
/// # Integration with SaveGameState
///
/// ```ignore
/// FortressRequest::SaveGameState { cell, frame } => {
///     let checksum = compute_checksum(&game_state).ok();
///     cell.save(frame, Some(game_state.clone()), checksum);
/// }
/// ```
///
/// [`GameStateCell::save`]: crate::GameStateCell::save
pub fn compute_checksum<T: Serialize>(state: &T) -> Result<u128, ChecksumError> {
    let bytes = encode(state)?;
    Ok(hash_bytes_fnv1a(&bytes))
}

/// Computes a Fletcher-16 checksum of a serializable game state.
///
/// Fletcher-16 is a simpler, faster checksum algorithm that produces a 16-bit
/// result (returned as `u128` for API consistency). It has weaker collision
/// resistance than FNV-1a but is faster to compute.
///
/// # When to Use
///
/// - Performance is critical and collision resistance is less important
/// - State is small enough that collisions are unlikely
/// - Following existing conventions (many game networking implementations use Fletcher)
///
/// # Example
///
/// ```
/// use fortress_rollback::checksum::compute_checksum_fletcher16;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct State { frame: u32 }
///
/// let checksum = compute_checksum_fletcher16(&State { frame: 100 })
///     .expect("should succeed");
/// ```
pub fn compute_checksum_fletcher16<T: Serialize>(state: &T) -> Result<u128, ChecksumError> {
    let bytes = encode(state).map_err(|e| {
        ChecksumError::serialization_failed(ChecksumAlgorithm::Fletcher16, e.to_string())
    })?;
    Ok(u128::from(fletcher16(&bytes)))
}

/// Computes a deterministic FNV-1a hash of raw bytes and returns it as `u128`.
///
/// This is a lower-level function for when you've already serialized your state
/// or need to hash arbitrary byte data.
///
/// # Example
///
/// ```
/// use fortress_rollback::checksum::hash_bytes_fnv1a;
///
/// let bytes = b"some game state bytes";
/// let hash = hash_bytes_fnv1a(bytes);
///
/// // Deterministic
/// assert_eq!(hash, hash_bytes_fnv1a(bytes));
/// ```
#[inline]
#[must_use]
pub fn hash_bytes_fnv1a(bytes: &[u8]) -> u128 {
    let mut hasher = DeterministicHasher::new();
    hasher.write(bytes);
    u128::from(hasher.finish())
}

/// Computes the Fletcher-16 checksum of a byte slice.
///
/// Fletcher-16 is a simple, fast checksum algorithm that provides reasonable
/// error detection for small to medium data sizes. It's commonly used in game
/// networking for state validation.
///
/// # Algorithm
///
/// The algorithm maintains two running sums:
/// - `sum1`: Simple sum of all bytes (mod 255)
/// - `sum2`: Sum of all intermediate `sum1` values (mod 255)
///
/// The final checksum is `(sum2 << 8) | sum1`.
///
/// # Example
///
/// ```
/// use fortress_rollback::checksum::fletcher16;
///
/// let data = b"hello world";
/// let checksum = fletcher16(data);
///
/// // Deterministic
/// assert_eq!(checksum, fletcher16(data));
/// ```
///
/// # References
///
/// - [Wikipedia: Fletcher's checksum](https://en.wikipedia.org/wiki/Fletcher%27s_checksum)
#[inline]
#[must_use]
pub fn fletcher16(data: &[u8]) -> u16 {
    let mut sum1: u16 = 0;
    let mut sum2: u16 = 0;

    for &byte in data {
        sum1 = (sum1 + u16::from(byte)) % 255;
        sum2 = (sum2 + sum1) % 255;
    }

    (sum2 << 8) | sum1
}

/// Represents what checksum algorithm was being computed when an error occurred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChecksumAlgorithm {
    /// FNV-1a hashing (used by [`compute_checksum`]).
    Fnv1a,
    /// Fletcher-16 checksum (used by [`compute_checksum_fletcher16`]).
    Fletcher16,
}

impl std::fmt::Display for ChecksumAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fnv1a => write!(f, "FNV-1a"),
            Self::Fletcher16 => write!(f, "Fletcher-16"),
        }
    }
}

/// Errors that can occur during checksum computation.
///
/// # Why String for Error Messages?
///
/// Similar to [`CodecError`], this error type stores
/// the underlying error message as a `String` rather than a structured enum. This is
/// intentional:
///
/// 1. **Wraps CodecError**: Checksum computation uses the codec module for serialization,
///    and [`CodecError`] already stores bincode errors as strings (since bincode errors
///    are opaque).
///
/// 2. **Not on the hot path**: Checksum errors occur during exceptional conditions
///    (serialization failures), not during normal game loop execution. The allocation
///    cost of a String is negligible for error paths.
///
/// 3. **Diagnostic preservation**: The serialization error message contains useful
///    diagnostic information about why serialization failed (e.g., "sequence too long",
///    "unsupported type").
///
/// For hot-path error handling, we use structured enums. See
/// [`RleDecodeReason`] for an example.
///
/// [`CodecError`]: crate::network::codec::CodecError
/// [`RleDecodeReason`]: crate::RleDecodeReason
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChecksumError {
    /// Serialization of the state failed.
    ///
    /// This typically occurs if the state contains unsupported types or
    /// if there's an issue with the serde implementation.
    SerializationFailed {
        /// The checksum algorithm that was being computed.
        algorithm: ChecksumAlgorithm,
        /// The underlying error message from the serializer.
        ///
        /// This is a `String` because it wraps [`CodecError`],
        /// which stores bincode errors as strings (since bincode errors are opaque).
        message: String,
    },
}

impl ChecksumError {
    /// Creates a new serialization error for the given algorithm.
    pub fn serialization_failed(algorithm: ChecksumAlgorithm, message: impl Into<String>) -> Self {
        Self::SerializationFailed {
            algorithm,
            message: message.into(),
        }
    }
}

impl From<CodecError> for ChecksumError {
    fn from(err: CodecError) -> Self {
        // Default to FNV-1a since that's the primary checksum algorithm
        Self::SerializationFailed {
            algorithm: ChecksumAlgorithm::Fnv1a,
            message: err.to_string(),
        }
    }
}

impl std::fmt::Display for ChecksumError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SerializationFailed { algorithm, message } => {
                write!(
                    f,
                    "{algorithm} checksum failed: serialization error: {message}"
                )
            },
        }
    }
}

impl std::error::Error for ChecksumError {}

// =============================================================================
// Unit Tests
// =============================================================================
//
// Note: Many determinism and consistency tests have been consolidated into
// property tests in the `property_tests` module below. The remaining unit tests
// cover:
// - Edge cases with specific known values (empty data, single byte)
// - Error type Display/Debug implementations
// - Trait implementations (Copy, Eq)
// - Serialization field order semantics

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Edge case tests for specific known values
    // -------------------------------------------------------------------------

    #[test]
    fn fletcher16_empty() {
        // Fletcher-16 of empty data is 0
        assert_eq!(fletcher16(&[]), 0);
    }

    #[test]
    fn fletcher16_single_byte() {
        // Single byte: sum1 = byte, sum2 = byte, result = (byte << 8) | byte
        let result = fletcher16(&[1]);
        assert_eq!(result, (1 << 8) | 1);
    }

    #[test]
    fn hash_bytes_fnv1a_empty() {
        // Empty input should produce the FNV offset basis
        let hash = hash_bytes_fnv1a(&[]);
        assert_eq!(hash, u128::from(0xcbf2_9ce4_8422_2325_u64));
    }

    #[test]
    fn hash_bytes_fnv1a_single_byte() {
        // FNV-1a of [1]: offset_basis XOR 1, then multiply by prime
        // offset_basis = 0xcbf29ce484222325
        // prime = 0x100000001b3
        // 0xcbf29ce484222325 ^ 1 = 0xcbf29ce484222324
        // 0xcbf29ce484222324 * 0x100000001b3 = 0xaf63bc4c8601b62c
        let hash = hash_bytes_fnv1a(&[1]);
        assert_eq!(hash, u128::from(0xaf63_bc4c_8601_b62c_u64));
    }

    // -------------------------------------------------------------------------
    // Error type and Display tests
    // -------------------------------------------------------------------------

    #[test]
    fn checksum_error_display() {
        let err = ChecksumError::SerializationFailed {
            algorithm: ChecksumAlgorithm::Fnv1a,
            message: "test error".to_string(),
        };
        let display = format!("{err}");
        assert!(display.contains("checksum failed"));
        assert!(display.contains("test error"));
        assert!(display.contains("FNV-1a"));
    }

    #[test]
    fn checksum_error_fletcher16_display() {
        let err =
            ChecksumError::serialization_failed(ChecksumAlgorithm::Fletcher16, "serialize failed");
        let display = format!("{err}");
        assert!(display.contains("Fletcher-16"));
        assert!(display.contains("serialize failed"));
    }

    #[test]
    fn checksum_algorithm_display() {
        assert_eq!(format!("{}", ChecksumAlgorithm::Fnv1a), "FNV-1a");
        assert_eq!(format!("{}", ChecksumAlgorithm::Fletcher16), "Fletcher-16");
    }

    // -------------------------------------------------------------------------
    // Trait implementation tests
    // -------------------------------------------------------------------------

    #[test]
    fn checksum_algorithm_is_copy() {
        let alg = ChecksumAlgorithm::Fnv1a;
        let alg2 = alg;
        assert_eq!(alg, alg2);
    }

    #[test]
    fn checksum_error_equality() {
        let err1 = ChecksumError::serialization_failed(ChecksumAlgorithm::Fnv1a, "test");
        let err2 = ChecksumError::serialization_failed(ChecksumAlgorithm::Fnv1a, "test");
        let err3 = ChecksumError::serialization_failed(ChecksumAlgorithm::Fletcher16, "test");
        let err4 = ChecksumError::serialization_failed(ChecksumAlgorithm::Fnv1a, "different");

        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
        assert_ne!(err1, err4);
    }

    // -------------------------------------------------------------------------
    // Serialization semantics tests (field order matters)
    // -------------------------------------------------------------------------

    #[test]
    fn compute_checksum_struct_field_order_matters() {
        #[derive(Serialize)]
        struct State1 {
            a: u32,
            b: u32,
        }

        #[derive(Serialize)]
        struct State2 {
            b: u32,
            a: u32,
        }

        let s1 = State1 { a: 1, b: 2 };
        let s2 = State2 { a: 1, b: 2 };

        let checksum1 = compute_checksum(&s1).unwrap();
        let checksum2 = compute_checksum(&s2).unwrap();

        // Field order affects serialization, so checksums differ
        assert_ne!(checksum1, checksum2);
    }
}

// =============================================================================
// Property-Based Tests
// =============================================================================

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]
mod property_tests {
    use super::*;
    use crate::test_config::miri_case_count;
    use proptest::prelude::*;
    use serde::Deserialize;

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: miri_case_count(),
            ..ProptestConfig::default()
        })]
        /// Property: compute_checksum is deterministic for any serializable state
        #[test]
        fn prop_checksum_deterministic(
            frame in any::<u32>(),
            x in any::<f32>().prop_filter("finite", |f| f.is_finite()),
            y in any::<f32>().prop_filter("finite", |f| f.is_finite()),
        ) {
            #[derive(Serialize, Deserialize)]
            struct State { frame: u32, x: f32, y: f32 }

            let state = State { frame, x, y };
            let checksum1 = compute_checksum(&state).expect("should serialize");
            let checksum2 = compute_checksum(&state).expect("should serialize");

            prop_assert_eq!(checksum1, checksum2, "Same state must produce same checksum");
        }

        /// Property: different struct states produce different checksums
        ///
        /// This test verifies checksum differentiation for **struct-wrapped values**,
        /// which is the common case in game state serialization. The struct wrapper
        /// affects serialization format (field ordering, struct markers in some formats).
        ///
        /// See also: `prop_checksum_different_primitives` which tests raw primitives.
        /// Both are kept separate because:
        /// 1. Struct serialization may differ from primitive serialization
        /// 2. This test uses u32 (common frame counter type) vs u64 in the primitive test
        /// 3. Having both provides confidence that checksums differentiate at multiple levels
        #[test]
        fn prop_different_frames_different_checksums(
            frame1 in any::<u32>(),
            frame2 in any::<u32>(),
        ) {
            prop_assume!(frame1 != frame2);

            #[derive(Serialize)]
            struct State { frame: u32 }

            let checksum1 = compute_checksum(&State { frame: frame1 }).expect("should serialize");
            let checksum2 = compute_checksum(&State { frame: frame2 }).expect("should serialize");

            prop_assert_ne!(checksum1, checksum2, "Different frames should produce different checksums");
        }

        /// Property: fletcher16 is deterministic
        #[test]
        fn prop_fletcher16_deterministic(data in any::<Vec<u8>>()) {
            let checksum1 = fletcher16(&data);
            let checksum2 = fletcher16(&data);
            prop_assert_eq!(checksum1, checksum2);
        }

        /// Property: fletcher16 components are bounded by mod-255 arithmetic
        ///
        /// The Fletcher-16 algorithm computes two sums (sum1 and sum2) where each
        /// is reduced modulo 255. This test verifies that the output components
        /// respect those bounds: each byte of the checksum must be in [0, 254].
        #[test]
        fn prop_fletcher16_modular_bounds(data in any::<Vec<u8>>()) {
            let checksum = fletcher16(&data);
            // Extract the two components: high byte is sum2, low byte is sum1
            let high_byte = checksum >> 8;
            let low_byte = checksum & 0xFF;
            // Both components are computed mod 255, so must be <= 254
            prop_assert!(high_byte <= 254, "sum2 should be in range [0, 254] due to mod 255");
            prop_assert!(low_byte <= 254, "sum1 should be in range [0, 254] due to mod 255");
        }

        /// Property: hash_bytes_fnv1a is deterministic
        #[test]
        fn prop_hash_bytes_deterministic(data in any::<Vec<u8>>()) {
            let hash1 = hash_bytes_fnv1a(&data);
            let hash2 = hash_bytes_fnv1a(&data);
            prop_assert_eq!(hash1, hash2);
        }

        /// Property: hash_bytes_fnv1a fits in u64 range (since underlying is u64)
        #[test]
        fn prop_hash_bytes_in_u64_range(data in any::<Vec<u8>>()) {
            let hash = hash_bytes_fnv1a(&data);
            prop_assert!(hash <= u128::from(u64::MAX), "FNV-1a produces 64-bit result");
        }

        /// Property: compute_checksum_fletcher16 is deterministic
        #[test]
        fn prop_fletcher16_checksum_deterministic(value in any::<u64>()) {
            let checksum1 = compute_checksum_fletcher16(&value).expect("should serialize");
            let checksum2 = compute_checksum_fletcher16(&value).expect("should serialize");
            prop_assert_eq!(checksum1, checksum2);
        }

        // Note on "different inputs → different outputs" tests:
        //
        // We AVOID testing arbitrary byte sequences (Vec<u8>) because collisions are
        // unpredictable and would make tests flaky.
        //
        // We DO test primitives like u64 because:
        // 1. Different u64 values serialize to different byte patterns (bijective)
        // 2. With 64-bit hash space, collision probability is ~1/2^64 (negligible)
        // 3. This validates the hash algorithm distinguishes different serializations
        //
        // The key insight: determinism is the critical property, but differentiation
        // testing is acceptable when the input space guarantees distinct serializations.

        /// Property: different primitive values produce different checksums
        ///
        /// This test verifies checksum differentiation for **raw primitive values**,
        /// testing the hash algorithm's ability to distinguish between different byte
        /// patterns without struct wrapper overhead.
        ///
        /// See also: `prop_different_frames_different_checksums` which tests struct-wrapped
        /// values. Both are kept separate because:
        /// 1. Primitive serialization is simpler (just the bytes) vs struct serialization
        /// 2. This test uses u64 (larger value space) vs u32 in the struct test
        /// 3. Having both provides confidence at the serialization and hashing layers
        #[test]
        fn prop_checksum_different_primitives(
            val1 in any::<u64>(),
            val2 in any::<u64>(),
        ) {
            prop_assume!(val1 != val2);

            let checksum1 = compute_checksum(&val1).expect("should serialize");
            let checksum2 = compute_checksum(&val2).expect("should serialize");

            prop_assert_ne!(checksum1, checksum2, "Different values should produce different checksums");
        }

        /// Property: nested structures produce deterministic checksums
        #[test]
        fn prop_checksum_nested_deterministic(
            inner_val in any::<i32>(),
            outer_count in any::<u64>(),
        ) {
            #[derive(Serialize, Deserialize)]
            struct Inner { value: i32 }

            #[derive(Serialize, Deserialize)]
            struct Outer { inner: Inner, count: u64 }

            let state = Outer {
                inner: Inner { value: inner_val },
                count: outer_count,
            };

            let checksum1 = compute_checksum(&state).expect("should serialize");
            let checksum2 = compute_checksum(&state).expect("should serialize");

            prop_assert_eq!(checksum1, checksum2, "Nested structs must produce deterministic checksums");
        }

        /// Property: tuples produce deterministic checksums
        #[test]
        fn prop_checksum_tuples_deterministic(
            a in any::<i32>(),
            b in any::<i32>(),
            c in any::<i32>(),
        ) {
            let tuple = (a, b, c);
            let checksum1 = compute_checksum(&tuple).expect("should serialize");
            let checksum2 = compute_checksum(&tuple).expect("should serialize");

            prop_assert_eq!(checksum1, checksum2, "Tuples must produce deterministic checksums");
        }

        /// Property: vectors produce deterministic checksums
        #[test]
        fn prop_checksum_vecs_deterministic(data in any::<Vec<i32>>()) {
            let checksum1 = compute_checksum(&data).expect("should serialize");
            let checksum2 = compute_checksum(&data).expect("should serialize");

            prop_assert_eq!(checksum1, checksum2, "Vectors must produce deterministic checksums");
        }

        /// Property: different container types produce different checksums
        #[test]
        fn prop_checksum_different_containers(
            val_u32 in any::<u32>(),
            val_tuple in (any::<i32>(), any::<i32>(), any::<i32>()),
        ) {
            let checksum_u32 = compute_checksum(&val_u32).expect("should serialize");
            let checksum_tuple = compute_checksum(&val_tuple).expect("should serialize");

            // Different types should produce different checksums
            prop_assert_ne!(checksum_u32, checksum_tuple, "Different types should produce different checksums");
        }
    }
}
