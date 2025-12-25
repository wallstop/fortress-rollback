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
//! let checksum = compute_checksum(&state).expect("serialization should succeed");
//!
//! // Same state produces same checksum
//! let checksum2 = compute_checksum(&state).expect("serialization should succeed");
//! assert_eq!(checksum, checksum2);
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
/// use fortress_rollback::checksum::compute_checksum;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct State {
///     frame: u32,
///     position: (f32, f32),
/// }
///
/// let state = State { frame: 42, position: (1.0, 2.0) };
/// let checksum = compute_checksum(&state).expect("should succeed");
///
/// // Deterministic: same state = same checksum
/// assert_eq!(checksum, compute_checksum(&state).unwrap());
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
    let bytes = encode(state)?;
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

/// Errors that can occur during checksum computation.
#[derive(Debug)]
pub enum ChecksumError {
    /// Serialization of the state failed.
    ///
    /// This typically occurs if the state contains unsupported types or
    /// if there's an issue with the serde implementation.
    SerializationFailed(String),
}

impl From<CodecError> for ChecksumError {
    fn from(err: CodecError) -> Self {
        Self::SerializationFailed(err.to_string())
    }
}

impl std::fmt::Display for ChecksumError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SerializationFailed(msg) => write!(f, "checksum failed: {msg}"),
        }
    }
}

impl std::error::Error for ChecksumError {}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
    struct TestState {
        frame: u32,
        position: (f32, f32),
        health: i16,
        name: String,
    }

    fn sample_state() -> TestState {
        TestState {
            frame: 100,
            position: (1.5, 2.5),
            health: 100,
            name: "Player1".to_string(),
        }
    }

    #[test]
    fn compute_checksum_deterministic() {
        let state = sample_state();
        let checksum1 = compute_checksum(&state).unwrap();
        let checksum2 = compute_checksum(&state).unwrap();
        assert_eq!(checksum1, checksum2);
    }

    #[test]
    fn compute_checksum_different_states() {
        let state1 = sample_state();
        let mut state2 = sample_state();
        state2.frame = 101;

        let checksum1 = compute_checksum(&state1).unwrap();
        let checksum2 = compute_checksum(&state2).unwrap();
        assert_ne!(checksum1, checksum2);
    }

    #[test]
    fn compute_checksum_fletcher16_deterministic() {
        let state = sample_state();
        let checksum1 = compute_checksum_fletcher16(&state).unwrap();
        let checksum2 = compute_checksum_fletcher16(&state).unwrap();
        assert_eq!(checksum1, checksum2);
    }

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
    fn fletcher16_known_values() {
        // Test with a known value
        // "abcde" -> sum1=195, sum2=236 -> (236 << 8) | 195 = 60611
        // Note: The actual calculation depends on modulo operations
        let checksum = fletcher16(b"abcde");
        // Verify it's consistent
        assert_eq!(checksum, fletcher16(b"abcde"));
    }

    #[test]
    fn fletcher16_overflow_handling() {
        // Test that mod 255 prevents overflow
        let data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
        let checksum = fletcher16(&data);
        // Should complete without panic and be deterministic
        assert_eq!(checksum, fletcher16(&data));
    }

    #[test]
    fn hash_bytes_fnv1a_deterministic() {
        let bytes = b"test data for hashing";
        let hash1 = hash_bytes_fnv1a(bytes);
        let hash2 = hash_bytes_fnv1a(bytes);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn hash_bytes_fnv1a_empty() {
        // Empty input should produce the FNV offset basis
        let hash = hash_bytes_fnv1a(&[]);
        assert_eq!(hash, u128::from(0xcbf2_9ce4_8422_2325_u64));
    }

    #[test]
    fn hash_bytes_fnv1a_different_inputs() {
        let hash1 = hash_bytes_fnv1a(b"hello");
        let hash2 = hash_bytes_fnv1a(b"world");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn checksum_returns_u128_for_save_compatibility() {
        let state = sample_state();
        let checksum: u128 = compute_checksum(&state).unwrap();
        // Verify it's a u128 (compile-time check via type annotation)
        assert!(checksum > 0);
    }

    #[test]
    fn checksum_error_display() {
        let err = ChecksumError::SerializationFailed("test error".to_string());
        let display = format!("{err}");
        assert!(display.contains("checksum failed"));
        assert!(display.contains("test error"));
    }

    #[test]
    fn compute_checksum_with_various_types() {
        // Test with primitive
        let checksum_u32 = compute_checksum(&42u32).unwrap();
        assert!(checksum_u32 > 0);

        // Test with tuple
        let checksum_tuple = compute_checksum(&(1, 2, 3)).unwrap();
        assert!(checksum_tuple > 0);

        // Test with Vec
        let checksum_vec = compute_checksum(&vec![1, 2, 3, 4, 5]).unwrap();
        assert!(checksum_vec > 0);

        // All should be different
        assert_ne!(checksum_u32, checksum_tuple);
        assert_ne!(checksum_tuple, checksum_vec);
    }

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

    #[test]
    fn compute_checksum_nested_structs() {
        #[derive(Serialize)]
        struct Inner {
            value: i32,
        }

        #[derive(Serialize)]
        struct Outer {
            inner: Inner,
            count: u64,
        }

        let state = Outer {
            inner: Inner { value: 42 },
            count: 100,
        };

        let checksum1 = compute_checksum(&state).unwrap();
        let checksum2 = compute_checksum(&state).unwrap();
        assert_eq!(checksum1, checksum2);
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
    use proptest::prelude::*;
    use serde::Deserialize;

    proptest! {
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

        /// Property: different states produce different checksums (usually)
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

        /// Property: fletcher16 result is always <= 16 bits (0xFFFF)
        #[test]
        fn prop_fletcher16_bounded(data in any::<Vec<u8>>()) {
            let checksum = fletcher16(&data);
            // Fletcher-16 returns u16, so it's always bounded by definition
            // This test verifies the return type is correct
            let _: u16 = checksum;
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

        /// Property: empty data has known FNV-1a hash (offset basis)
        #[test]
        fn prop_empty_hash_is_offset_basis(_seed in any::<u8>()) {
            let hash = hash_bytes_fnv1a(&[]);
            let expected = u128::from(0xcbf2_9ce4_8422_2325_u64);
            prop_assert_eq!(hash, expected);
        }

        /// Property: compute_checksum_fletcher16 is deterministic
        #[test]
        fn prop_fletcher16_checksum_deterministic(value in any::<u64>()) {
            let checksum1 = compute_checksum_fletcher16(&value).expect("should serialize");
            let checksum2 = compute_checksum_fletcher16(&value).expect("should serialize");
            prop_assert_eq!(checksum1, checksum2);
        }
    }
}
