//! Internal random number generator implementation based on PCG32.
//!
//! This module provides a minimal, high-quality PRNG that replaces the `rand` crate
//! dependency, removing 6 transitive dependencies while maintaining equivalent functionality.
//!
//! # PCG32 Algorithm
//!
//! PCG (Permuted Congruential Generator) is a family of simple fast space-efficient
//! statistically good algorithms for random number generation. PCG32 specifically:
//! - Has 64 bits of state, producing 32-bit output
//! - Period of 2^64
//! - Passes TestU01 statistical tests
//! - Is fast and simple to implement
//!
//! Reference: <https://www.pcg-random.org/>
//!
//! # Usage
//!
//! ```rust
//! use fortress_rollback::rng::{Pcg32, Rng, SeedableRng, random};
//!
//! // Global random (thread-local)
//! let value: u32 = random();
//!
//! // Seeded RNG for deterministic behavior
//! let mut rng = Pcg32::seed_from_u64(12345);
//! let value = rng.gen_range(0..100);
//! ```

use crate::{
    report_violation,
    telemetry::{ViolationKind, ViolationSeverity},
};
use std::cell::RefCell;

/// PCG32 random number generator.
///
/// A minimal implementation of the PCG-XSH-RR variant with 64-bit state.
/// Suitable for game development and testing, but NOT cryptographically secure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pcg32 {
    state: u64,
    inc: u64,
}

/// Default increment for single-stream PCG32.
/// This is a standard value from the PCG paper.
const PCG_DEFAULT_INCREMENT: u64 = 1442695040888963407;

/// Multiplier constant for the LCG step.
/// This is the standard multiplier for 64-bit state PCG.
const PCG_MULTIPLIER: u64 = 6364136223846793005;

impl Pcg32 {
    /// Creates a new PCG32 generator with the given state and stream.
    ///
    /// The stream (increment) allows for multiple independent sequences.
    /// The increment must be odd; if even, it will be made odd by OR-ing with 1.
    #[must_use]
    pub const fn new(state: u64, stream: u64) -> Self {
        // The increment must be odd
        let inc = (stream << 1) | 1;
        // Initialize state to 0, then advance once, then add the initial state
        // This is the standard PCG seeding procedure
        let mut pcg = Self { state: 0, inc };
        // Can't call non-const fn in const context, so we inline the step
        pcg.state = pcg.state.wrapping_mul(PCG_MULTIPLIER).wrapping_add(pcg.inc);
        pcg.state = pcg.state.wrapping_add(state);
        pcg.state = pcg.state.wrapping_mul(PCG_MULTIPLIER).wrapping_add(pcg.inc);
        pcg
    }

    /// Generates the next 32-bit random value.
    #[inline]
    #[must_use]
    pub fn next_u32(&mut self) -> u32 {
        let old_state = self.state;
        // Advance internal state
        self.state = old_state
            .wrapping_mul(PCG_MULTIPLIER)
            .wrapping_add(self.inc);
        // Calculate output using XSH-RR (xor-shift, random rotate)
        let xorshifted = (((old_state >> 18) ^ old_state) >> 27) as u32;
        let rot = (old_state >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// Generates the next 64-bit random value by combining two 32-bit values.
    #[inline]
    #[must_use]
    pub fn next_u64(&mut self) -> u64 {
        let high = u64::from(self.next_u32());
        let low = u64::from(self.next_u32());
        (high << 32) | low
    }
}

/// Trait for seeding random number generators.
///
/// Provides a simple interface for creating deterministic RNG instances.
pub trait SeedableRng: Sized {
    /// Creates a new RNG seeded from a 64-bit value.
    ///
    /// Different seeds produce different (statistically independent) sequences.
    #[must_use]
    fn seed_from_u64(seed: u64) -> Self;

    /// Creates a new RNG with a random seed derived from system timing.
    ///
    /// This uses timing information and thread identity for entropy, which is
    /// sufficient for game PRNGs but NOT cryptographically secure.
    #[must_use]
    fn from_entropy() -> Self;
}

impl SeedableRng for Pcg32 {
    fn seed_from_u64(seed: u64) -> Self {
        Self::new(seed, PCG_DEFAULT_INCREMENT)
    }

    fn from_entropy() -> Self {
        Self::seed_from_u64(timing_entropy_seed())
    }
}

/// Trait for random number generation.
///
/// Provides methods for generating random values of various types.
pub trait Rng {
    /// Returns the next 32-bit random value.
    fn next_u32(&mut self) -> u32;

    /// Returns the next 64-bit random value.
    fn next_u64(&mut self) -> u64;

    /// Generates a random value of type `T`.
    fn gen<T: RandomValue>(&mut self) -> T {
        T::random(self)
    }

    /// Generates a random `u32` value in the given range `[low, high)`.
    ///
    /// # Empty Range Behavior
    /// If `range.is_empty()`, reports a violation via telemetry and returns `range.start`.
    fn gen_range(&mut self, range: std::ops::Range<u32>) -> u32 {
        let span = range.end.wrapping_sub(range.start);
        if span == 0 {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::Configuration,
                "gen_range called with empty range [{}..{})",
                range.start,
                range.end
            );
            return range.start;
        }

        // Use rejection sampling to avoid bias
        let threshold = span.wrapping_neg() % span;
        loop {
            let random_value = self.next_u32();
            if random_value >= threshold {
                return range.start.wrapping_add(random_value % span);
            }
        }
    }

    /// Generates a random `usize` value in the given range `[low, high)`.
    ///
    /// # Empty Range Behavior
    /// If `range.is_empty()`, reports a violation via telemetry and returns `range.start`.
    fn gen_range_usize(&mut self, range: std::ops::Range<usize>) -> usize {
        let span = range.end.wrapping_sub(range.start);
        if span == 0 {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::Configuration,
                "gen_range_usize called with empty range [{}..{})",
                range.start,
                range.end
            );
            return range.start;
        }

        if span <= u32::MAX as usize {
            // Use 32-bit arithmetic for smaller ranges
            let threshold = (span as u32).wrapping_neg() % (span as u32);
            loop {
                let random_value = self.next_u32();
                if random_value >= threshold {
                    return range
                        .start
                        .wrapping_add((random_value % span as u32) as usize);
                }
            }
        } else {
            // Use 64-bit arithmetic for larger ranges
            let span64 = span as u64;
            let threshold = span64.wrapping_neg() % span64;
            loop {
                let random_value = self.next_u64();
                if random_value >= threshold {
                    return range.start.wrapping_add((random_value % span64) as usize);
                }
            }
        }
    }

    /// Generates a random `i64` value in the given inclusive range `[low, high]`.
    ///
    /// # Empty Range Behavior
    /// If `start > end`, reports a violation via telemetry and returns `start`.
    fn gen_range_i64_inclusive(&mut self, range: std::ops::RangeInclusive<i64>) -> i64 {
        let start = *range.start();
        let end = *range.end();
        if start > end {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::Configuration,
                "gen_range_i64_inclusive called with invalid range [{}..={}]",
                start,
                end
            );
            return start;
        }

        // Calculate span as u64 to handle full i64 range
        let span = (end as i128 - start as i128 + 1) as u64;

        // Special case: full range
        if span == 0 {
            // This means the range is the entire i64 range (2^64 values when including overflow)
            return self.next_u64() as i64;
        }

        // Use rejection sampling for unbiased results
        let threshold = span.wrapping_neg() % span;
        loop {
            let random_value = self.next_u64();
            if random_value >= threshold {
                return start.wrapping_add((random_value % span) as i64);
            }
        }
    }

    /// Generates a random boolean with the given probability of being `true`.
    ///
    /// `probability` should be in the range `[0.0, 1.0]`.
    /// Values outside this range are clamped.
    fn gen_bool(&mut self, probability: f64) -> bool {
        let p = probability.clamp(0.0, 1.0);
        let threshold = (p * f64::from(u32::MAX)) as u32;
        self.next_u32() < threshold
    }

    /// Fills the given slice with random bytes.
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        let mut chunks = dest.chunks_exact_mut(4);
        for chunk in chunks.by_ref() {
            let val = self.next_u32().to_le_bytes();
            chunk.copy_from_slice(&val);
        }
        // Handle remaining bytes
        let remainder = chunks.into_remainder();
        if !remainder.is_empty() {
            let val = self.next_u32().to_le_bytes();
            if let Some(val_slice) = val.get(..remainder.len()) {
                remainder.copy_from_slice(val_slice);
            }
        }
    }
}

impl Rng for Pcg32 {
    #[inline]
    fn next_u32(&mut self) -> u32 {
        Self::next_u32(self)
    }

    #[inline]
    fn next_u64(&mut self) -> u64 {
        Self::next_u64(self)
    }
}

/// Trait for types that can be randomly generated.
pub trait RandomValue {
    /// Generates a random value of this type.
    fn random<R: Rng + ?Sized>(rng: &mut R) -> Self;
}

impl RandomValue for u8 {
    fn random<R: Rng + ?Sized>(rng: &mut R) -> Self {
        rng.next_u32() as Self
    }
}

impl RandomValue for u16 {
    fn random<R: Rng + ?Sized>(rng: &mut R) -> Self {
        rng.next_u32() as Self
    }
}

impl RandomValue for u32 {
    fn random<R: Rng + ?Sized>(rng: &mut R) -> Self {
        rng.next_u32()
    }
}

impl RandomValue for u64 {
    fn random<R: Rng + ?Sized>(rng: &mut R) -> Self {
        rng.next_u64()
    }
}

impl RandomValue for i8 {
    fn random<R: Rng + ?Sized>(rng: &mut R) -> Self {
        rng.next_u32() as Self
    }
}

impl RandomValue for i16 {
    fn random<R: Rng + ?Sized>(rng: &mut R) -> Self {
        rng.next_u32() as Self
    }
}

impl RandomValue for i32 {
    fn random<R: Rng + ?Sized>(rng: &mut R) -> Self {
        rng.next_u32() as Self
    }
}

impl RandomValue for i64 {
    fn random<R: Rng + ?Sized>(rng: &mut R) -> Self {
        rng.next_u64() as Self
    }
}

impl RandomValue for u128 {
    fn random<R: Rng + ?Sized>(rng: &mut R) -> Self {
        let high = Self::from(rng.next_u64());
        let low = Self::from(rng.next_u64());
        (high << 64) | low
    }
}

impl RandomValue for f32 {
    /// Generates a random `f32` in the range `[0.0, 1.0)`.
    fn random<R: Rng + ?Sized>(rng: &mut R) -> Self {
        // Use the upper 24 bits (f32 has 24 bits of mantissa precision)
        let val = rng.next_u32() >> 8;
        val as Self / (1u32 << 24) as Self
    }
}

impl RandomValue for f64 {
    /// Generates a random `f64` in the range `[0.0, 1.0)`.
    fn random<R: Rng + ?Sized>(rng: &mut R) -> Self {
        // Use the upper 53 bits (f64 has 53 bits of mantissa precision)
        let val = rng.next_u64() >> 11;
        val as Self / (1u64 << 53) as Self
    }
}

impl RandomValue for bool {
    fn random<R: Rng + ?Sized>(rng: &mut R) -> Self {
        rng.next_u32() & 1 == 1
    }
}

// Thread-local RNG for convenient random() function

thread_local! {
    static THREAD_RNG: RefCell<Pcg32> = RefCell::new(Pcg32::from_entropy());
}

/// Generates a random value using the thread-local RNG.
///
/// This is the simplest way to get a random value:
///
/// ```rust
/// use fortress_rollback::rng::random;
///
/// let value: u32 = random();
/// let coin_flip: bool = random();
/// ```
#[must_use]
pub fn random<T: RandomValue>() -> T {
    THREAD_RNG.with(|rng| {
        let mut rng = rng.borrow_mut();
        T::random(&mut *rng)
    })
}

/// Returns a reference to the thread-local RNG.
///
/// Useful when you need to call multiple RNG methods without
/// repeated thread-local lookups.
#[must_use]
pub fn thread_rng() -> ThreadRng {
    ThreadRng { _private: () }
}

/// A handle to the thread-local random number generator.
///
/// This is lightweight (zero-sized) and just provides access to the thread-local RNG.
#[derive(Debug)]
pub struct ThreadRng {
    _private: (),
}

impl Rng for ThreadRng {
    #[inline]
    fn next_u32(&mut self) -> u32 {
        THREAD_RNG.with(|rng| rng.borrow_mut().next_u32())
    }

    #[inline]
    fn next_u64(&mut self) -> u64 {
        THREAD_RNG.with(|rng| rng.borrow_mut().next_u64())
    }
}

/// Gets a timing-based seed for RNG initialization.
///
/// Combines multiple entropy sources:
/// - High-precision timing via `web_time::Instant`
/// - Thread identity for cross-thread uniqueness
///
/// # Non-Determinism Warning
///
/// This function is intentionally non-deterministic. It uses timing information
/// that varies between runs. This is appropriate for:
/// - Casual random number generation via `random()`
/// - Non-critical randomness in tests
/// - Network protocol identifiers where uniqueness matters more than reproducibility
///
/// For deterministic behavior (required for game state simulation), always use
/// [`Pcg32::seed_from_u64`] with a fixed seed instead.
///
/// This is NOT cryptographically secure, but provides sufficient
/// entropy for game PRNGs where unpredictability isn't critical.
fn timing_entropy_seed() -> u64 {
    use crate::hash::DeterministicHasher;
    use std::hash::{Hash, Hasher};
    use web_time::Instant;

    // Use timing for entropy - this is intentionally non-deterministic
    let now = Instant::now();

    // Mix in thread ID for additional entropy across threads
    // Use DeterministicHasher to ensure consistent hashing across platforms
    // (DefaultHasher uses a random seed which adds another layer of non-determinism)
    let thread_id = std::thread::current().id();
    let thread_hash = {
        let mut hasher = DeterministicHasher::new();
        thread_id.hash(&mut hasher);
        hasher.finish()
    };

    // Use elapsed nanoseconds for timing entropy
    // This is still non-deterministic (timing varies between runs) but uses
    // DeterministicHasher for consistent cross-platform behavior
    let timing_hash = {
        let mut hasher = DeterministicHasher::new();
        // Hash the debug representation of Instant for entropy
        // (Instant doesn't implement Hash, but its timing is captured here)
        let elapsed = now.elapsed();
        elapsed.as_nanos().hash(&mut hasher);
        hasher.finish()
    };

    // Combine timing and thread identity
    thread_hash
        .wrapping_mul(timing_hash)
        .wrapping_add(0x9e3779b97f4a7c15)
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
    fn test_pcg32_deterministic() {
        let mut rng1 = Pcg32::seed_from_u64(12345);
        let mut rng2 = Pcg32::seed_from_u64(12345);

        for _ in 0..1000 {
            assert_eq!(rng1.next_u32(), rng2.next_u32());
        }
    }

    #[test]
    fn test_pcg32_different_seeds() {
        let mut rng1 = Pcg32::seed_from_u64(12345);
        let mut rng2 = Pcg32::seed_from_u64(54321);

        // Should produce different sequences
        let mut same_count = 0;
        for _ in 0..100 {
            if rng1.next_u32() == rng2.next_u32() {
                same_count += 1;
            }
        }
        // Extremely unlikely to have more than a few collisions
        assert!(same_count < 10);
    }

    #[test]
    fn test_pcg32_distribution() {
        let mut rng = Pcg32::seed_from_u64(42);
        let mut buckets = [0u32; 16];

        // Generate many values and check distribution
        for _ in 0..16000 {
            let val = rng.next_u32();
            let bucket = (val >> 28) as usize; // Use top 4 bits
            buckets[bucket] += 1;
        }

        // Each bucket should have roughly 1000 values (16000/16)
        // Allow significant variance for statistical tests
        for &count in &buckets {
            assert!(count > 500, "Bucket too low: {count}");
            assert!(count < 1500, "Bucket too high: {count}");
        }
    }

    #[test]
    fn test_gen_range() {
        let mut rng = Pcg32::seed_from_u64(42);

        for _ in 0..1000 {
            let val = rng.gen_range(10..20);
            assert!(val >= 10);
            assert!(val < 20);
        }
    }

    #[test]
    fn test_gen_bool() {
        let mut rng = Pcg32::seed_from_u64(42);

        // Test edge cases
        for _ in 0..100 {
            assert!(!rng.gen_bool(0.0));
            assert!(rng.gen_bool(1.0));
        }

        // Test 50% probability
        let mut true_count = 0;
        for _ in 0..10000 {
            if rng.gen_bool(0.5) {
                true_count += 1;
            }
        }
        // Should be roughly 5000, allow variance
        assert!(true_count > 4500, "Too few trues: {true_count}");
        assert!(true_count < 5500, "Too many trues: {true_count}");
    }

    #[test]
    fn test_fill_bytes() {
        let mut rng = Pcg32::seed_from_u64(42);

        // Test various lengths
        for len in [0, 1, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17] {
            let mut buf = vec![0u8; len];
            rng.fill_bytes(&mut buf);

            // After filling, at least some bytes should be non-zero (for len > 0)
            if len > 0 {
                // It's extremely unlikely all bytes are zero
                let all_zero = buf.iter().all(|&b| b == 0);
                // Allow for very small buffers where this might happen
                if len >= 4 {
                    assert!(!all_zero, "fill_bytes produced all zeros for len={len}");
                }
            }
        }
    }

    #[test]
    fn test_random_types() {
        let mut rng = Pcg32::seed_from_u64(42);

        // Just verify these don't panic and produce varied values
        let _: u8 = rng.gen();
        let _: u16 = rng.gen();
        let _: u32 = rng.gen();
        let _: u64 = rng.gen();
        let _: u128 = rng.gen();
        let _: i8 = rng.gen();
        let _: i16 = rng.gen();
        let _: i32 = rng.gen();
        let _: i64 = rng.gen();
        let _: bool = rng.gen();

        // f32 and f64 should be in [0, 1)
        for _ in 0..1000 {
            let f: f32 = rng.gen();
            assert!(f >= 0.0);
            assert!(f < 1.0);

            let d: f64 = rng.gen();
            assert!(d >= 0.0);
            assert!(d < 1.0);
        }
    }

    #[test]
    fn test_thread_rng() {
        let val1: u32 = random();
        let val2: u32 = random();
        // Very unlikely to be equal
        assert_ne!(val1, val2, "Two random calls returned same value");
    }

    #[test]
    fn test_seedable_from_entropy() {
        // Just verify it doesn't panic
        let _rng = Pcg32::from_entropy();
    }

    // Test that known seed produces expected sequence (golden test)
    #[test]
    fn test_pcg32_golden() {
        let mut rng = Pcg32::seed_from_u64(0);

        // These values are from running the implementation with seed 0
        // They serve as a regression test to ensure we don't accidentally change the algorithm
        let expected = [
            0x348a463f_u32,
            0x4f205a1b_u32,
            0x2946c488_u32,
            0x805e36de_u32,
            0x79f994a9_u32,
        ];

        for &exp in &expected {
            assert_eq!(rng.next_u32(), exp, "Golden test failed");
        }
    }

    #[test]
    fn test_gen_range_usize_small() {
        let mut rng = Pcg32::seed_from_u64(42);

        for _ in 0..1000 {
            let val = rng.gen_range_usize(10..20);
            assert!(val >= 10);
            assert!(val < 20);
        }
    }

    #[test]
    fn test_gen_range_usize_large() {
        let mut rng = Pcg32::seed_from_u64(42);

        // Test with a range larger than u32::MAX
        let large_start: usize = (u32::MAX as usize) + 1000;
        let large_end: usize = large_start + 1000;

        for _ in 0..100 {
            let val = rng.gen_range_usize(large_start..large_end);
            assert!(val >= large_start);
            assert!(val < large_end);
        }
    }

    #[test]
    fn test_gen_range_i64_inclusive() {
        let mut rng = Pcg32::seed_from_u64(42);

        for _ in 0..1000 {
            let val = rng.gen_range_i64_inclusive(-100..=100);
            assert!(val >= -100);
            assert!(val <= 100);
        }

        // Test with negative-only range
        for _ in 0..100 {
            let val = rng.gen_range_i64_inclusive(-50..=-10);
            assert!(val >= -50);
            assert!(val <= -10);
        }
    }

    #[test]
    fn test_gen_range_single_value() {
        let mut rng = Pcg32::seed_from_u64(42);

        // Single value range should always return that value
        for _ in 0..100 {
            let val = rng.gen_range(42..43);
            assert_eq!(val, 42);
        }
    }

    #[test]
    fn test_next_u64_combines_correctly() {
        let mut rng = Pcg32::seed_from_u64(42);

        // Verify u64 covers full range (tests high bits are populated)
        let mut has_high_bits = false;
        for _ in 0..1000 {
            let val = rng.next_u64();
            if val > u64::from(u32::MAX) {
                has_high_bits = true;
                break;
            }
        }
        assert!(
            has_high_bits,
            "next_u64 should produce values with high bits set"
        );
    }

    // =========================================================================
    // Empty Range Tests (violation reporting with graceful fallback)
    // =========================================================================

    /// Tests that gen_range with an empty range (start == end) returns start
    /// instead of panicking. A violation is reported via telemetry.
    #[test]
    fn test_gen_range_empty_returns_start() {
        let mut rng = Pcg32::seed_from_u64(42);

        // Empty range (start == end)
        let result = rng.gen_range(100..100);
        assert_eq!(result, 100, "Empty range should return start value");

        // Test with different start values
        let result = rng.gen_range(0..0);
        assert_eq!(result, 0, "Empty range at 0 should return 0");

        let result = rng.gen_range(u32::MAX..u32::MAX);
        assert_eq!(result, u32::MAX, "Empty range at MAX should return MAX");
    }

    /// Tests that gen_range_usize with an empty range returns start
    /// instead of panicking. A violation is reported via telemetry.
    #[test]
    fn test_gen_range_usize_empty_returns_start() {
        let mut rng = Pcg32::seed_from_u64(42);

        // Empty range
        let result = rng.gen_range_usize(500..500);
        assert_eq!(result, 500, "Empty range should return start value");

        // Test with different start values
        let result = rng.gen_range_usize(0..0);
        assert_eq!(result, 0, "Empty range at 0 should return 0");
    }

    /// Tests that gen_range_i64_inclusive with an invalid range (start > end)
    /// returns start instead of panicking. A violation is reported via telemetry.
    #[test]
    #[allow(clippy::reversed_empty_ranges)] // Intentionally testing invalid ranges
    fn test_gen_range_i64_inclusive_invalid_returns_start() {
        let mut rng = Pcg32::seed_from_u64(42);

        // Invalid range (start > end)
        let result = rng.gen_range_i64_inclusive(100..=-50);
        assert_eq!(result, 100, "Invalid range should return start value");

        let result = rng.gen_range_i64_inclusive(50..=10);
        assert_eq!(result, 50, "Invalid range should return start value");

        // Negative range with start > end
        let result = rng.gen_range_i64_inclusive(-10..=-100);
        assert_eq!(result, -10, "Invalid range should return start value");
    }

    /// Tests that gen_range_i64_inclusive with a single value range (start == end)
    /// is valid and returns that value. This is NOT an error case.
    #[test]
    fn test_gen_range_i64_inclusive_single_value_is_valid() {
        let mut rng = Pcg32::seed_from_u64(42);

        // Single value range (start == end) is valid for inclusive ranges
        for _ in 0..10 {
            let result = rng.gen_range_i64_inclusive(42..=42);
            assert_eq!(
                result, 42,
                "Single value inclusive range should return that value"
            );
        }

        // Test with different values
        let result = rng.gen_range_i64_inclusive(-100..=-100);
        assert_eq!(result, -100, "Single value inclusive range should work");

        let result = rng.gen_range_i64_inclusive(0..=0);
        assert_eq!(result, 0, "Single value inclusive range should work");
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

    proptest! {
        /// Property: Same seed always produces identical sequence.
        ///
        /// This is critical for rollback networking - game state must be
        /// deterministically reproducible from the same seed.
        #[test]
        fn prop_determinism_same_seed_same_sequence(seed in any::<u64>()) {
            let mut rng1 = Pcg32::seed_from_u64(seed);
            let mut rng2 = Pcg32::seed_from_u64(seed);

            for _ in 0..100 {
                prop_assert_eq!(
                    rng1.next_u32(), rng2.next_u32(),
                    "Same seed must produce identical sequences"
                );
            }
        }

        /// Property: Different seeds produce different sequences.
        ///
        /// While collisions are possible, they should be astronomically rare.
        /// With PCG32's 64-bit state, two random seeds colliding in the first
        /// few outputs should essentially never happen.
        #[test]
        fn prop_different_seeds_different_sequences(seed1 in any::<u64>(), seed2 in any::<u64>()) {
            prop_assume!(seed1 != seed2);

            let mut rng1 = Pcg32::seed_from_u64(seed1);
            let mut rng2 = Pcg32::seed_from_u64(seed2);

            // Collect first 10 values
            let seq1: Vec<u32> = (0..10).map(|_| rng1.next_u32()).collect();
            let seq2: Vec<u32> = (0..10).map(|_| rng2.next_u32()).collect();

            // Sequences should differ (extremely unlikely to collide)
            prop_assert_ne!(seq1, seq2, "Different seeds should produce different sequences");
        }

        /// Property: gen_range output is always within the specified range.
        #[test]
        fn prop_gen_range_within_bounds(
            seed in any::<u64>(),
            start in 0u32..1000,
            span in 1u32..1000,
        ) {
            let end = start.saturating_add(span);
            prop_assume!(end > start); // Ensure valid range

            let mut rng = Pcg32::seed_from_u64(seed);

            for _ in 0..100 {
                let val = rng.gen_range(start..end);
                prop_assert!(val >= start, "gen_range output {} below start {}", val, start);
                prop_assert!(val < end, "gen_range output {} >= end {}", val, end);
            }
        }

        /// Property: gen_range_usize output is always within the specified range.
        #[test]
        fn prop_gen_range_usize_within_bounds(
            seed in any::<u64>(),
            start in 0usize..10000,
            span in 1usize..10000,
        ) {
            let end = start.saturating_add(span);
            prop_assume!(end > start);

            let mut rng = Pcg32::seed_from_u64(seed);

            for _ in 0..50 {
                let val = rng.gen_range_usize(start..end);
                prop_assert!(val >= start, "gen_range_usize output {} below start {}", val, start);
                prop_assert!(val < end, "gen_range_usize output {} >= end {}", val, end);
            }
        }

        /// Property: gen_range_i64_inclusive output is always within the specified range.
        #[test]
        fn prop_gen_range_i64_within_bounds(
            seed in any::<u64>(),
            start in -10000i64..10000,
            span in 1i64..1000,
        ) {
            let end = start.saturating_add(span);

            let mut rng = Pcg32::seed_from_u64(seed);

            for _ in 0..50 {
                let val = rng.gen_range_i64_inclusive(start..=end);
                prop_assert!(val >= start, "gen_range_i64 output {} below start {}", val, start);
                prop_assert!(val <= end, "gen_range_i64 output {} > end {}", val, end);
            }
        }

        /// Property: gen_bool probability is approximately respected.
        ///
        /// Tests that gen_bool(p) returns true approximately p% of the time.
        #[test]
        fn prop_gen_bool_probability(
            seed in any::<u64>(),
            probability in 0.1f64..0.9, // Avoid edge cases for statistical stability
        ) {
            let mut rng = Pcg32::seed_from_u64(seed);
            let samples = 1000;

            let true_count = (0..samples).filter(|_| rng.gen_bool(probability)).count();
            let actual_probability = true_count as f64 / samples as f64;

            // Allow 15% tolerance for statistical variance
            let tolerance = 0.15;
            prop_assert!(
                (actual_probability - probability).abs() < tolerance,
                "gen_bool({}) produced {}% true (expected ~{}%)",
                probability,
                actual_probability * 100.0,
                probability * 100.0
            );
        }

        /// Property: fill_bytes produces deterministic output for same seed.
        #[test]
        fn prop_fill_bytes_deterministic(
            seed in any::<u64>(),
            len in 0usize..256,
        ) {
            let mut rng1 = Pcg32::seed_from_u64(seed);
            let mut rng2 = Pcg32::seed_from_u64(seed);

            let mut buf1 = vec![0u8; len];
            let mut buf2 = vec![0u8; len];

            rng1.fill_bytes(&mut buf1);
            rng2.fill_bytes(&mut buf2);

            prop_assert_eq!(buf1, buf2, "fill_bytes must be deterministic for same seed");
        }

        /// Property: RandomValue generation is deterministic.
        #[test]
        fn prop_random_value_deterministic(seed in any::<u64>()) {
            let mut rng1 = Pcg32::seed_from_u64(seed);
            let mut rng2 = Pcg32::seed_from_u64(seed);

            // Test various types
            prop_assert_eq!(rng1.gen::<u8>(), rng2.gen::<u8>());
            prop_assert_eq!(rng1.gen::<u16>(), rng2.gen::<u16>());
            prop_assert_eq!(rng1.gen::<u32>(), rng2.gen::<u32>());
            prop_assert_eq!(rng1.gen::<u64>(), rng2.gen::<u64>());
            prop_assert_eq!(rng1.gen::<u128>(), rng2.gen::<u128>());
            prop_assert_eq!(rng1.gen::<i32>(), rng2.gen::<i32>());
            prop_assert_eq!(rng1.gen::<i64>(), rng2.gen::<i64>());
            prop_assert_eq!(rng1.gen::<bool>(), rng2.gen::<bool>());
        }

        /// Property: f32 generation is always in [0.0, 1.0).
        #[test]
        fn prop_f32_bounds(seed in any::<u64>()) {
            let mut rng = Pcg32::seed_from_u64(seed);

            for _ in 0..100 {
                let val: f32 = rng.gen();
                prop_assert!(val >= 0.0, "f32 gen produced {} < 0.0", val);
                prop_assert!(val < 1.0, "f32 gen produced {} >= 1.0", val);
            }
        }

        /// Property: f64 generation is always in [0.0, 1.0).
        #[test]
        fn prop_f64_bounds(seed in any::<u64>()) {
            let mut rng = Pcg32::seed_from_u64(seed);

            for _ in 0..100 {
                let val: f64 = rng.gen();
                prop_assert!(val >= 0.0, "f64 gen produced {} < 0.0", val);
                prop_assert!(val < 1.0, "f64 gen produced {} >= 1.0", val);
            }
        }

        /// Property: Clone produces identical RNG that generates same sequence.
        #[test]
        fn prop_clone_produces_identical_sequence(seed in any::<u64>(), advance in 0usize..100) {
            let mut rng1 = Pcg32::seed_from_u64(seed);

            // Advance RNG by some amount
            for _ in 0..advance {
                let _ = rng1.next_u32();
            }

            // Clone at this point
            let mut rng2 = rng1.clone();

            // Both should produce identical values going forward
            for _ in 0..50 {
                prop_assert_eq!(
                    rng1.next_u32(), rng2.next_u32(),
                    "Cloned RNG must produce identical sequence"
                );
            }
        }

        /// Property: Distribution is approximately uniform across all bits.
        ///
        /// For a uniform random generator, each bit position should be 0 or 1
        /// with roughly equal probability. With 1000 samples, we expect ~500
        /// for each bit position, with statistical variance.
        #[test]
        fn prop_uniform_bit_distribution(seed in any::<u64>()) {
            let mut rng = Pcg32::seed_from_u64(seed);
            let samples = 1000;

            let mut bit_counts = [0u32; 32];

            for _ in 0..samples {
                let val = rng.next_u32();
                for (bit, count) in bit_counts.iter_mut().enumerate() {
                    if (val >> bit) & 1 == 1 {
                        *count += 1;
                    }
                }
            }

            // Each bit should be set approximately half the time
            // With 1000 samples, expected = 500, stddev ≈ sqrt(1000 * 0.25) ≈ 15.8
            // Allow 4 standard deviations (99.99% confidence) = ~64
            // Use 30% tolerance for robustness (300)
            let expected = samples as f64 / 2.0;
            let tolerance = expected * 0.30;

            for (bit, &count) in bit_counts.iter().enumerate() {
                prop_assert!(
                    (count as f64 - expected).abs() < tolerance,
                    "Bit {} has count {} (expected ~{} +/- {})",
                    bit,
                    count,
                    expected,
                    tolerance
                );
            }
        }
    }
}
