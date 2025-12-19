//! Synchronization primitives abstraction for loom testing compatibility.
//!
//! This module provides a unified interface to synchronization primitives that works
//! with both production code (using `parking_lot` for performance) and loom tests
//! (using `loom::sync` for model checking).
//!
//! # Usage
//!
//! Import from this module instead of directly from `parking_lot` or `std::sync`:
//!
//! ```ignore
//! // Instead of:
//! use parking_lot::Mutex;
//! use std::sync::Arc;
//!
//! // Use:
//! use crate::sync::{Arc, Mutex};
//! ```
//!
//! # Loom Testing
//!
//! Run loom tests from the isolated `loom-tests/` crate:
//! ```bash
//! cd loom-tests
//! RUSTFLAGS="--cfg loom" cargo test --release
//! ```
//!
//! ## MappedMutexGuard Handling
//!
//! `parking_lot::MappedMutexGuard` allows projecting a mutex guard to a sub-field.
//! Loom doesn't have an equivalent. This module provides `MappedGuardWrapper` which:
//!
//! - Under production: Uses the efficient `MappedMutexGuard`
//! - Under loom: Holds a reference to the full guard (still thread-safe, but can't project)

// ============================================================================
// LOOM CONFIGURATION
// ============================================================================

/// When running under loom (`RUSTFLAGS="--cfg loom"`), use loom's types
#[cfg(loom)]
pub(crate) mod inner {
    pub use loom::sync::Arc;
    pub use loom::sync::Mutex;
    #[allow(unused_imports)] // Used for API consistency
    pub use loom::sync::MutexGuard;
    #[allow(unused_imports)] // Used for API consistency
    pub use loom::thread;

    /// Yield to the loom scheduler. This is important for testing spin-loops
    /// and other constructs that assume fair scheduling.
    #[inline]
    #[allow(dead_code)] // May not be used in all loom tests
    pub fn yield_now() {
        loom::thread::yield_now();
    }

    /// Under loom, MappedMutexGuard doesn't exist.
    /// We use the regular MutexGuard and project to the data.
    /// This is a type alias for compatibility - actual usage will differ.
    #[allow(dead_code)] // May not be used under loom - data() returns None
    pub type MappedMutexGuard<'a, T> = std::marker::PhantomData<&'a T>;
}

/// In production, use parking_lot for performance
#[cfg(not(loom))]
pub(crate) mod inner {
    pub use parking_lot::MappedMutexGuard;
    pub use parking_lot::Mutex;
    #[allow(unused_imports)] // Used for loom compatibility abstraction
    pub use parking_lot::MutexGuard;
    pub use std::sync::Arc;
    #[allow(unused_imports)] // Used for loom compatibility abstraction
    pub use std::thread;

    /// No-op in production - only meaningful under loom
    #[inline]
    #[allow(dead_code)] // Used via loom compatibility abstraction in tests
    pub fn yield_now() {
        std::thread::yield_now();
    }
}

// Re-export at module level for convenience
pub(crate) use inner::*;

// ============================================================================
// TESTING UTILITIES
// ============================================================================

/// Run a loom model test. Under loom, this explores all possible
/// thread interleavings. In production, it just runs the closure once.
#[cfg(loom)]
#[allow(dead_code)] // Available for loom tests in tests/ or loom-tests/
pub fn model<F>(f: F)
where
    F: Fn() + Sync + Send + 'static,
{
    loom::model(f);
}

/// Run a loom model test with custom configuration.
#[cfg(loom)]
#[allow(dead_code)] // Available for loom tests in tests/ or loom-tests/
pub fn model_with_config<F>(f: F, max_preemptions: Option<usize>)
where
    F: Fn() + Sync + Send + 'static,
{
    let mut builder = loom::model::Builder::new();
    if let Some(bound) = max_preemptions {
        builder.preemption_bound = Some(bound);
    }
    builder.check(f);
}

/// In production, just run the closure once
#[cfg(not(loom))]
#[allow(dead_code)] // Available for production code that wants loom-compatible testing
pub fn model<F>(f: F)
where
    F: FnOnce(),
{
    f();
}

/// In production, just run the closure once (ignores config)
#[cfg(not(loom))]
#[allow(dead_code)] // Available for production code that wants loom-compatible testing
pub fn model_with_config<F>(f: F, _max_preemptions: Option<usize>)
where
    F: FnOnce(),
{
    f();
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(all(test, loom))]
mod loom_tests {
    use super::*;

    #[test]
    fn test_mutex_basic() {
        model(|| {
            let mutex = Arc::new(Mutex::new(0));
            let mutex2 = mutex.clone();

            let handle = thread::spawn(move || {
                let mut guard = mutex2.lock();
                *guard += 1;
            });

            {
                let mut guard = mutex.lock();
                *guard += 1;
            }

            handle.join().unwrap();

            let final_value = *mutex.lock();
            assert_eq!(final_value, 2);
        });
    }

    #[test]
    fn test_concurrent_reads_writes() {
        model(|| {
            let data = Arc::new(Mutex::new(vec![1, 2, 3]));
            let data2 = data.clone();

            let reader = thread::spawn(move || {
                let guard = data2.lock();
                guard.len()
            });

            {
                let mut guard = data.lock();
                guard.push(4);
            }

            let len = reader.join().unwrap();
            // Length could be 3 or 4 depending on interleaving
            assert!(len == 3 || len == 4);
        });
    }
}

#[cfg(all(test, not(loom)))]
mod tests {
    use super::*;

    #[test]
    fn test_mutex_basic() {
        let mutex = Arc::new(Mutex::new(0));
        {
            let mut guard = mutex.lock();
            *guard = 42;
        }
        assert_eq!(*mutex.lock(), 42);
    }

    #[test]
    fn test_model_runs_closure() {
        let mut called = false;
        model(|| {
            called = true;
        });
        assert!(called);
    }
}
