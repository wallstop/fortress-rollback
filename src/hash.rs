//! Deterministic hashing utilities.
//!
//! This module provides deterministic hashing functions that produce consistent
//! results across processes, platforms, and runs. Unlike `std::collections::hash_map::DefaultHasher`,
//! which uses a random seed for security, these hashers use fixed algorithms
//! that are essential for rollback networking where all peers must agree on checksums.
//!
//! # Why Deterministic Hashing?
//!
//! In rollback networking, peers exchange state checksums to detect desynchronization.
//! If different peers use different hash seeds (as `DefaultHasher` does), they will
//! produce different checksums for identical states, causing false desync detection.
//!
//! # Usage
//!
//! ```
//! use fortress_rollback::hash::{DeterministicHasher, fnv1a_hash};
//! use std::hash::{Hash, Hasher};
//!
//! // For types that implement Hash
//! let mut hasher = DeterministicHasher::new();
//! "hello".hash(&mut hasher);
//! let hash1 = hasher.finish();
//!
//! // Convenience function for hashable types
//! let hash2 = fnv1a_hash(&"hello");
//!
//! // Both produce the same deterministic result
//! assert_eq!(hash1, hash2);
//! ```
//!
//! # Algorithm
//!
//! This module uses FNV-1a (Fowler-Noll-Vo hash function, variant 1a), which is:
//! - Fast and simple
//! - Deterministic (no random seed)
//! - Good distribution for typical inputs
//! - Widely used and well-tested
//!
//! Note: FNV-1a is NOT cryptographically secure and should not be used for
//! security-sensitive applications. For game state checksums, this is fine.
//!
//! # Cross-Platform Integers
//!
//! `std::hash::Hasher`'s default integer writers encode with `to_ne_bytes`, so
//! a scalar `usize` field is four bytes on 32-bit targets (such as browser
//! `wasm32-unknown-unknown`) and eight bytes on 64-bit targets. Slice and
//! collection length prefixes have the same problem, because `Hash for [T]`
//! calls `write_length_prefix`, which routes through `write_usize`.
//! [`DeterministicHasher`] overrides every multi-byte integer writer with a
//! fixed-width little-endian encoding (`usize`/`isize` canonicalize to 64 bits),
//! matching the codec's `with_little_endian().with_fixed_int_encoding()` wire
//! policy.
//!
//! The overrides do not cover primitive slice *elements*. Std's
//! `Hash::hash_slice` specialization for primitive integers writes the
//! elements' native memory bytes straight through [`Hasher::write`],
//! bypassing the integer writers. A `[usize]`/`[isize]` element is therefore
//! still four bytes on `wasm32` and eight natively, and multi-byte elements use
//! native byte order. For a state checksum that must match across targets, hash
//! canonical bytes instead of relying on derived `Hash` for machine-width
//! primitive collections; [`crate::checksum::compute_checksum`] serializes with
//! the fixed-width little-endian codec.

use std::hash::{Hash, Hasher};

/// FNV-1a 64-bit offset basis constant.
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;

/// FNV-1a 64-bit prime constant.
const FNV_PRIME: u64 = 0x0100_0000_01b3;

/// A deterministic hasher using the FNV-1a algorithm.
///
/// This hasher produces consistent results across processes, platforms, and runs,
/// making it suitable for rollback networking where all peers must agree on checksums.
///
/// Scalar integers and collection length prefixes use fixed-width little-endian
/// encoding, with `usize`/`isize` canonicalized to 64 bits, so those values hash
/// identically on 32-bit WASM and 64-bit little-endian native targets. Std's
/// primitive slice `hash_slice` writes element bytes natively, so collections of
/// `usize`/`isize` (or multi-byte elements on big-endian targets) are not covered;
/// use [`compute_checksum`](crate::checksum::compute_checksum) for cross-target
/// state checksums.
///
/// # Example
///
/// ```
/// use fortress_rollback::hash::DeterministicHasher;
/// use std::hash::{Hash, Hasher};
///
/// let mut hasher = DeterministicHasher::new();
/// 42u32.hash(&mut hasher);
/// let hash = hasher.finish();
///
/// // Same value always produces the same hash
/// let mut hasher2 = DeterministicHasher::new();
/// 42u32.hash(&mut hasher2);
/// assert_eq!(hash, hasher2.finish());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeterministicHasher {
    state: u64,
}

impl DeterministicHasher {
    /// Creates a new `DeterministicHasher` with the standard FNV-1a offset basis.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: FNV_OFFSET_BASIS,
        }
    }
}

impl Default for DeterministicHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Hasher for DeterministicHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.state
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        // FNV-1a algorithm: for each byte, XOR then multiply
        for &byte in bytes {
            self.state ^= u64::from(byte);
            self.state = self.state.wrapping_mul(FNV_PRIME);
        }
    }

    // The `Hasher` defaults encode integers with `to_ne_bytes`, so a scalar
    // `usize` hashes 4 bytes on 32-bit targets (browser
    // `wasm32-unknown-unknown`) and 8 bytes on 64-bit native targets. Slice and
    // collection `Hash` impls length-prefix through `write_usize`, so the
    // pointer width alone makes a scalar or a length prefix hash differently in
    // a browser than natively. Pin every integer writer to fixed-width
    // little-endian bytes, matching the codec's
    // `with_little_endian().with_fixed_int_encoding()` wire policy. The
    // canonical width for `usize`/`isize` is 64 bits, mirroring bincode's
    // fixed-int encoding. Primitive slice elements are outside this override:
    // std's `Hash::hash_slice` writes their native memory bytes directly.

    #[inline]
    fn write_u16(&mut self, i: u16) {
        self.write(&i.to_le_bytes());
    }

    #[inline]
    fn write_u32(&mut self, i: u32) {
        self.write(&i.to_le_bytes());
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.write(&i.to_le_bytes());
    }

    #[inline]
    fn write_u128(&mut self, i: u128) {
        self.write(&i.to_le_bytes());
    }

    #[inline]
    fn write_usize(&mut self, i: usize) {
        // Widen to the canonical 64-bit width. `try_from` keeps the conversion
        // total on a hypothetical wider target instead of silently truncating.
        self.write(&u64::try_from(i).unwrap_or(u64::MAX).to_le_bytes());
    }

    #[inline]
    fn write_i16(&mut self, i: i16) {
        self.write(&i.to_le_bytes());
    }

    #[inline]
    fn write_i32(&mut self, i: i32) {
        self.write(&i.to_le_bytes());
    }

    #[inline]
    fn write_i64(&mut self, i: i64) {
        self.write(&i.to_le_bytes());
    }

    #[inline]
    fn write_i128(&mut self, i: i128) {
        self.write(&i.to_le_bytes());
    }

    #[inline]
    fn write_isize(&mut self, i: isize) {
        // Sign-extend to the canonical 64-bit width (see `write_usize`).
        let widened = i64::try_from(i).unwrap_or(if i.is_negative() { i64::MIN } else { i64::MAX });
        self.write(&widened.to_le_bytes());
    }
}

/// Computes a deterministic FNV-1a hash of the given value.
///
/// This is a convenience function that creates a [`DeterministicHasher`],
/// hashes the value, and returns the result.
///
/// # Portability
///
/// Scalar integers (including `usize`/`isize`) and slice/collection length
/// prefixes hash identically on 32-bit WASM and 64-bit little-endian native
/// targets. Primitive slice *elements* are hashed by the standard library as
/// native memory bytes, so a collection of `usize`/`isize` elements (and
/// multi-byte elements on big-endian targets) can still differ. For state
/// checksums that must match across targets, prefer
/// [`compute_checksum`](crate::checksum::compute_checksum), which serializes
/// with the fixed-width little-endian codec.
///
/// # Example
///
/// ```
/// use fortress_rollback::hash::fnv1a_hash;
///
/// let hash = fnv1a_hash(&42u32);
///
/// // Same value always produces the same hash
/// assert_eq!(hash, fnv1a_hash(&42u32));
///
/// // Different values produce different hashes (usually)
/// assert_ne!(hash, fnv1a_hash(&43u32));
/// ```
#[inline]
pub fn fnv1a_hash<T: Hash>(value: &T) -> u64 {
    let mut hasher = DeterministicHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

/// A [`std::hash::BuildHasher`] that creates [`DeterministicHasher`] instances.
///
/// This can be used with `HashMap` and `HashSet` if deterministic iteration order
/// is not required, but deterministic hashing is. However, for most cases in
/// rollback networking, prefer `BTreeMap` and `BTreeSet` for deterministic iteration.
///
/// # Example
///
/// ```
/// use fortress_rollback::hash::DeterministicBuildHasher;
/// use std::collections::HashMap;
///
/// // Create a HashMap with deterministic hashing
/// let map: HashMap<i32, &str, DeterministicBuildHasher> =
///     HashMap::with_hasher(DeterministicBuildHasher);
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct DeterministicBuildHasher;

impl std::hash::BuildHasher for DeterministicBuildHasher {
    type Hasher = DeterministicHasher;

    #[inline]
    fn build_hasher(&self) -> Self::Hasher {
        DeterministicHasher::new()
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_hasher_consistency() {
        // Same value should always produce the same hash
        let hash1 = fnv1a_hash(&42u32);
        let hash2 = fnv1a_hash(&42u32);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_deterministic_hasher_different_values() {
        // Different values should produce different hashes
        let hash1 = fnv1a_hash(&42u32);
        let hash2 = fnv1a_hash(&43u32);
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_deterministic_hasher_strings() {
        let hash1 = fnv1a_hash(&"hello");
        let hash2 = fnv1a_hash(&"hello");
        assert_eq!(hash1, hash2);

        let hash3 = fnv1a_hash(&"world");
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_deterministic_hasher_structs() {
        #[derive(Hash)]
        struct TestStruct {
            a: i32,
            b: String,
        }

        let s1 = TestStruct {
            a: 42,
            b: "test".to_string(),
        };
        let s2 = TestStruct {
            a: 42,
            b: "test".to_string(),
        };

        assert_eq!(fnv1a_hash(&s1), fnv1a_hash(&s2));
    }

    #[test]
    fn test_deterministic_hasher_empty() {
        // Empty write should still produce offset basis
        let hasher = DeterministicHasher::new();
        assert_eq!(hasher.finish(), FNV_OFFSET_BASIS);
    }

    #[test]
    fn test_deterministic_hasher_incremental() {
        // Incremental hashing should be consistent
        let mut hasher1 = DeterministicHasher::new();
        hasher1.write(b"hello");
        hasher1.write(b"world");
        let hash1 = hasher1.finish();

        let mut hasher2 = DeterministicHasher::new();
        hasher2.write(b"hello");
        hasher2.write(b"world");
        let hash2 = hasher2.finish();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_build_hasher() {
        use std::collections::HashMap;

        // Verify DeterministicBuildHasher can be used with HashMap
        let mut map: HashMap<i32, &str, DeterministicBuildHasher> =
            HashMap::with_hasher(DeterministicBuildHasher);
        map.insert(1, "one");
        map.insert(2, "two");

        assert_eq!(map.get(&1), Some(&"one"));
        assert_eq!(map.get(&2), Some(&"two"));
    }

    #[test]
    fn test_known_fnv1a_values() {
        // Test against known FNV-1a values for verification
        // FNV-1a("") = offset basis = 0xcbf29ce484222325
        let mut hasher = DeterministicHasher::new();
        hasher.write(b"");
        assert_eq!(hasher.finish(), 0xcbf2_9ce4_8422_2325);

        // FNV-1a("a") = 0xaf63dc4c8601ec8c
        let mut hasher = DeterministicHasher::new();
        hasher.write(b"a");
        assert_eq!(hasher.finish(), 0xaf63_dc4c_8601_ec8c);

        // FNV-1a("foobar") = 0x85944171f73967e8
        let mut hasher = DeterministicHasher::new();
        hasher.write(b"foobar");
        assert_eq!(hasher.finish(), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn integer_writers_match_fixed_width_little_endian_bytes() {
        let mut expected = DeterministicHasher::new();
        expected.write(&0x0102u16.to_le_bytes());
        expected.write(&0x0102_0304u32.to_le_bytes());
        expected.write(&0x0102_0304_0506_0708u64.to_le_bytes());
        expected.write(&0x0102_0304_0506_0708_090a_0b0c_0d0e_0f10u128.to_le_bytes());
        expected.write(&7u64.to_le_bytes());
        expected.write(&u64::try_from(usize::MAX).unwrap_or(u64::MAX).to_le_bytes());
        expected.write(&(-0x0102i16).to_le_bytes());
        expected.write(&(-0x0102_0304i32).to_le_bytes());
        expected.write(&(-0x0102_0304_0506_0708i64).to_le_bytes());
        expected.write(&(-9i64).to_le_bytes());
        expected.write(&i64::try_from(isize::MIN).unwrap_or(i64::MIN).to_le_bytes());

        let mut actual = DeterministicHasher::new();
        actual.write_u16(0x0102);
        actual.write_u32(0x0102_0304);
        actual.write_u64(0x0102_0304_0506_0708);
        actual.write_u128(0x0102_0304_0506_0708_090a_0b0c_0d0e_0f10);
        actual.write_usize(7);
        actual.write_usize(usize::MAX);
        actual.write_i16(-0x0102);
        actual.write_i32(-0x0102_0304);
        actual.write_i64(-0x0102_0304_0506_0708);
        actual.write_isize(-9);
        actual.write_isize(isize::MIN);

        assert_eq!(
            actual.finish(),
            expected.finish(),
            "integer writers must emit fixed-width little-endian bytes"
        );
    }

    #[test]
    fn fnv1a_hash_is_independent_of_pointer_width() {
        // The constants are the native (64-bit little-endian) values and are
        // asserted again from the browser WASM lane, so a 32-bit build that
        // falls back to `to_ne_bytes` for `usize`/`isize`/slice lengths is
        // detected as a cross-target checksum divergence.
        assert_eq!(
            fnv1a_hash(&[1u8, 2, 3]),
            0x01ef_76d4_29b1_1552,
            "slice length prefix must be a fixed-width u64"
        );
        assert_eq!(
            fnv1a_hash(&7usize),
            0x4bd7_a317_074c_5b62,
            "usize must hash as a fixed-width u64"
        );
        assert_eq!(
            fnv1a_hash(&(-9isize)),
            0x2ee4_949c_2c7d_0345,
            "isize must hash as a fixed-width i64"
        );
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
    clippy::indexing_slicing,
    clippy::tuple_array_conversions
)]
mod property_tests {
    use super::*;
    use crate::test_config::miri_case_count;
    use proptest::prelude::*;
    use std::hash::BuildHasher;

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: miri_case_count(),
            ..ProptestConfig::default()
        })]
        /// Property: Determinism - Same input always produces same hash
        ///
        /// This is critical for rollback networking where peers must agree on checksums.
        #[test]
        fn prop_hash_deterministic(input in any::<Vec<u8>>()) {
            let hash1 = {
                let mut hasher = DeterministicHasher::new();
                hasher.write(&input);
                hasher.finish()
            };
            let hash2 = {
                let mut hasher = DeterministicHasher::new();
                hasher.write(&input);
                hasher.finish()
            };
            prop_assert_eq!(hash1, hash2, "Same input must produce same hash");
        }

        /// Property: Determinism for integers - Same integer always produces same hash
        #[test]
        fn prop_hash_deterministic_u64(value in any::<u64>()) {
            let hash1 = fnv1a_hash(&value);
            let hash2 = fnv1a_hash(&value);
            prop_assert_eq!(hash1, hash2, "Same value must produce same hash");
        }

        /// Property: Incremental hashing consistency
        ///
        /// Verifies that writing data incrementally produces the same result
        /// regardless of how the writes are chunked.
        #[test]
        fn prop_incremental_hashing_consistent(
            part_a in any::<Vec<u8>>(),
            part_b in any::<Vec<u8>>(),
        ) {
            // Hash in two parts
            let hash_incremental = {
                let mut hasher = DeterministicHasher::new();
                hasher.write(&part_a);
                hasher.write(&part_b);
                hasher.finish()
            };

            // Hash concatenated
            let mut combined = part_a;
            combined.extend_from_slice(&part_b);
            let hash_combined = {
                let mut hasher = DeterministicHasher::new();
                hasher.write(&combined);
                hasher.finish()
            };

            prop_assert_eq!(
                hash_incremental, hash_combined,
                "Incremental and combined hashing must match"
            );
        }

        /// Property: Different inputs usually produce different hashes
        ///
        /// While collisions are possible, they should be rare for arbitrary inputs.
        /// We test this by ensuring small differences in input produce different hashes.
        #[test]
        fn prop_different_inputs_different_hashes(
            base in any::<u64>().prop_filter("non-max", |v| *v < u64::MAX),
        ) {
            let hash1 = fnv1a_hash(&base);
            let hash2 = fnv1a_hash(&(base + 1));
            // Adjacent integers should have different hashes
            prop_assert_ne!(hash1, hash2, "Adjacent values should produce different hashes");
        }

        /// Property: Empty input produces offset basis
        ///
        /// FNV-1a defines hash("") = offset_basis
        #[test]
        fn prop_empty_input_offset_basis(_seed in any::<u8>()) {
            let hasher = DeterministicHasher::new();
            prop_assert_eq!(hasher.finish(), FNV_OFFSET_BASIS);

            let mut hasher2 = DeterministicHasher::new();
            hasher2.write(&[]);
            prop_assert_eq!(hasher2.finish(), FNV_OFFSET_BASIS);
        }

        /// Property: Single byte hashing follows FNV-1a formula
        ///
        /// For single byte b: hash = (offset_basis XOR b) * prime
        #[test]
        fn prop_single_byte_fnv1a_formula(byte in any::<u8>()) {
            let mut hasher = DeterministicHasher::new();
            hasher.write(&[byte]);
            let actual = hasher.finish();

            // Manual FNV-1a calculation
            let expected = FNV_OFFSET_BASIS ^ u64::from(byte);
            let expected = expected.wrapping_mul(FNV_PRIME);

            prop_assert_eq!(actual, expected, "Single byte hash must follow FNV-1a formula");
        }

        /// Property: Order matters - ab != ba (usually)
        ///
        /// Hash functions should be sensitive to order of data.
        #[test]
        fn prop_order_sensitive(
            a in 1u8..=254,  // Avoid 0 and 255 for cleaner testing
            b in 1u8..=254,
        ) {
            prop_assume!(a != b);  // Only test when a and b are different

            let hash_ab = {
                let mut hasher = DeterministicHasher::new();
                hasher.write(&[a, b]);
                hasher.finish()
            };

            let hash_ba = {
                let mut hasher = DeterministicHasher::new();
                hasher.write(&[b, a]);
                hasher.finish()
            };

            prop_assert_ne!(hash_ab, hash_ba, "Order should affect hash result");
        }

        /// Property: fnv1a_hash convenience function matches manual hashing
        #[test]
        fn prop_convenience_function_matches_manual(value in any::<i32>()) {
            let convenience_hash = fnv1a_hash(&value);

            let manual_hash = {
                let mut hasher = DeterministicHasher::new();
                value.hash(&mut hasher);
                hasher.finish()
            };

            prop_assert_eq!(
                convenience_hash, manual_hash,
                "Convenience function must match manual hashing"
            );
        }

        /// Property: BuildHasher produces consistent hashers
        #[test]
        fn prop_build_hasher_consistent(input in any::<Vec<u8>>()) {
            let build_hasher = DeterministicBuildHasher;

            let hash1 = {
                let mut hasher = build_hasher.build_hasher();
                hasher.write(&input);
                hasher.finish()
            };

            let hash2 = {
                let mut hasher = build_hasher.build_hasher();
                hasher.write(&input);
                hasher.finish()
            };

            prop_assert_eq!(hash1, hash2, "BuildHasher must produce consistent hashers");
        }
    }

    /// Extended FNV-1a test vectors from various sources
    /// These verify our implementation against known correct values
    #[test]
    fn test_extended_fnv1a_vectors() {
        // Test vectors calculated using the FNV-1a algorithm definition
        // FNV-1a: for each byte b: hash = (hash XOR b) * FNV_prime
        let test_cases: &[(&[u8], u64)] = &[
            // Empty string
            (b"", 0xcbf2_9ce4_8422_2325),
            // Single characters
            (b"a", 0xaf63_dc4c_8601_ec8c),
            (b"b", 0xaf63_df4c_8601_f1a5),
            (b"c", 0xaf63_de4c_8601_eff2),
            // Common strings
            (b"foobar", 0x8594_4171_f739_67e8),
            (b"hello", 0xa430_d846_80aa_bd0b),
            (b"world", 0x4f59_ff5e_730c_8af3),
            // Numeric patterns
            (b"123", 0x456f_c218_1822_c4db),
            (b"0", 0xaf63_ad4c_8601_9caf),
            // Edge cases with special characters
            (b"\0", 0xaf63_bd4c_8601_b7df),   // null byte
            (b"\xff", 0xaf64_724c_8602_eb6e), // 0xFF
        ];

        for (input, expected) in test_cases {
            let mut hasher = DeterministicHasher::new();
            hasher.write(input);
            let actual = hasher.finish();
            assert_eq!(
                actual, *expected,
                "FNV-1a mismatch for input {:?}: expected 0x{:016x}, got 0x{:016x}",
                input, expected, actual
            );
        }
    }
}
