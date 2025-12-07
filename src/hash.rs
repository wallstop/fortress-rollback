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
#[derive(Debug, Clone)]
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
}

/// Computes a deterministic FNV-1a hash of the given value.
///
/// This is a convenience function that creates a [`DeterministicHasher`],
/// hashes the value, and returns the result.
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
#[derive(Debug, Clone, Copy, Default)]
pub struct DeterministicBuildHasher;

impl std::hash::BuildHasher for DeterministicBuildHasher {
    type Hasher = DeterministicHasher;

    #[inline]
    fn build_hasher(&self) -> Self::Hasher {
        DeterministicHasher::new()
    }
}

#[cfg(test)]
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
}
