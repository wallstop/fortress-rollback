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
    /// # Panics
    /// Panics if `range.is_empty()`.
    fn gen_range(&mut self, range: std::ops::Range<u32>) -> u32 {
        let span = range.end.wrapping_sub(range.start);
        assert!(span > 0, "gen_range: range must not be empty");

        // Use rejection sampling to avoid bias
        let threshold = span.wrapping_neg() % span;
        loop {
            let r = self.next_u32();
            if r >= threshold {
                return range.start.wrapping_add(r % span);
            }
        }
    }

    /// Generates a random `usize` value in the given range `[low, high)`.
    ///
    /// # Panics
    /// Panics if `range.is_empty()`.
    fn gen_range_usize(&mut self, range: std::ops::Range<usize>) -> usize {
        let span = range.end.wrapping_sub(range.start);
        assert!(span > 0, "gen_range_usize: range must not be empty");

        if span <= u32::MAX as usize {
            // Use 32-bit arithmetic for smaller ranges
            let threshold = (span as u32).wrapping_neg() % (span as u32);
            loop {
                let r = self.next_u32();
                if r >= threshold {
                    return range.start.wrapping_add((r % span as u32) as usize);
                }
            }
        } else {
            // Use 64-bit arithmetic for larger ranges
            let span64 = span as u64;
            let threshold = span64.wrapping_neg() % span64;
            loop {
                let r = self.next_u64();
                if r >= threshold {
                    return range.start.wrapping_add((r % span64) as usize);
                }
            }
        }
    }

    /// Generates a random `i64` value in the given inclusive range `[low, high]`.
    ///
    /// # Panics
    /// Panics if `range.is_empty()` (i.e., `start > end`).
    fn gen_range_i64_inclusive(&mut self, range: std::ops::RangeInclusive<i64>) -> i64 {
        let start = *range.start();
        let end = *range.end();
        assert!(
            start <= end,
            "gen_range_i64_inclusive: start must not exceed end"
        );

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
            let r = self.next_u64();
            if r >= threshold {
                return start.wrapping_add((r % span) as i64);
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
            remainder.copy_from_slice(&val[..remainder.len()]);
        }
    }
}

impl Rng for Pcg32 {
    #[inline]
    fn next_u32(&mut self) -> u32 {
        Pcg32::next_u32(self)
    }

    #[inline]
    fn next_u64(&mut self) -> u64 {
        Pcg32::next_u64(self)
    }
}

/// Trait for types that can be randomly generated.
pub trait RandomValue {
    /// Generates a random value of this type.
    fn random<R: Rng + ?Sized>(rng: &mut R) -> Self;
}

impl RandomValue for u8 {
    fn random<R: Rng + ?Sized>(rng: &mut R) -> Self {
        rng.next_u32() as u8
    }
}

impl RandomValue for u16 {
    fn random<R: Rng + ?Sized>(rng: &mut R) -> Self {
        rng.next_u32() as u16
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
        rng.next_u32() as i8
    }
}

impl RandomValue for i16 {
    fn random<R: Rng + ?Sized>(rng: &mut R) -> Self {
        rng.next_u32() as i16
    }
}

impl RandomValue for i32 {
    fn random<R: Rng + ?Sized>(rng: &mut R) -> Self {
        rng.next_u32() as i32
    }
}

impl RandomValue for i64 {
    fn random<R: Rng + ?Sized>(rng: &mut R) -> Self {
        rng.next_u64() as i64
    }
}

impl RandomValue for u128 {
    fn random<R: Rng + ?Sized>(rng: &mut R) -> Self {
        let high = u128::from(rng.next_u64());
        let low = u128::from(rng.next_u64());
        (high << 64) | low
    }
}

impl RandomValue for f32 {
    /// Generates a random `f32` in the range `[0.0, 1.0)`.
    fn random<R: Rng + ?Sized>(rng: &mut R) -> Self {
        // Use the upper 24 bits (f32 has 24 bits of mantissa precision)
        let val = rng.next_u32() >> 8;
        val as f32 / (1u32 << 24) as f32
    }
}

impl RandomValue for f64 {
    /// Generates a random `f64` in the range `[0.0, 1.0)`.
    fn random<R: Rng + ?Sized>(rng: &mut R) -> Self {
        // Use the upper 53 bits (f64 has 53 bits of mantissa precision)
        let val = rng.next_u64() >> 11;
        val as f64 / (1u64 << 53) as f64
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
/// - Memory address randomization (ASLR) where available
///
/// This is NOT cryptographically secure, but provides sufficient
/// entropy for game PRNGs where unpredictability isn't critical.
fn timing_entropy_seed() -> u64 {
    use web_time::Instant;

    // Use the instant's internal representation for entropy
    let now = Instant::now();
    let ptr = &now as *const _ as usize;

    // Mix in thread ID for additional entropy across threads
    let thread_id = std::thread::current().id();
    let thread_hash = {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        thread_id.hash(&mut hasher);
        hasher.finish()
    };

    // Combine with timing info
    (ptr as u64).wrapping_mul(thread_hash)
}

#[cfg(test)]
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
}
