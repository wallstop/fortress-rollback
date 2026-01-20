//! Shared test configuration and utilities for property-based testing.
//!
//! This module provides centralized configuration for proptest and other testing
//! frameworks, ensuring consistent behavior across all test modules.
//!
//! # Miri Support
//!
//! When running tests under Miri (the Rust mid-level IR interpreter for detecting
//! undefined behavior), we reduce the number of test cases to keep test times
//! reasonable while still providing coverage.
//!
//! # Usage
//!
//! ```ignore
//! use crate::test_config::miri_case_count;
//!
//! proptest! {
//!     #![proptest_config(ProptestConfig {
//!         cases: miri_case_count(),
//!         ..ProptestConfig::default()
//!     })]
//!     #[test]
//!     fn my_property_test(value in any::<u32>()) {
//!         // test body
//!     }
//! }
//! ```

/// Returns the number of test cases to run for property-based tests.
///
/// When running under Miri, returns a reduced count (5) for faster testing.
/// Otherwise, returns the standard count (256) for thorough coverage.
///
/// # Examples
///
/// ```ignore
/// use proptest::prelude::*;
/// use crate::test_config::miri_case_count;
///
/// proptest! {
///     #![proptest_config(ProptestConfig {
///         cases: miri_case_count(),
///         ..ProptestConfig::default()
///     })]
///     #[test]
///     fn test_something(value in any::<u32>()) {
///         // ...
///     }
/// }
/// ```
#[must_use]
pub const fn miri_case_count() -> u32 {
    if cfg!(miri) {
        5
    } else {
        256
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_miri_case_count_returns_expected_value() {
        // When not running under Miri, should return 256
        // When running under Miri, should return 5
        let count = miri_case_count();

        if cfg!(miri) {
            assert_eq!(count, 5);
        } else {
            assert_eq!(count, 256);
        }
    }
}
