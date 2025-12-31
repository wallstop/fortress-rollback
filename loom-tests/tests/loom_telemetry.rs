//! Loom tests for CollectingObserver thread safety.
//!
//! These tests verify that CollectingObserver's Mutex-protected operations
//! are thread-safe by exhaustively exploring all possible interleavings using loom.
//!
//! Run with:
//! ```bash
//! cd loom-tests
//! RUSTFLAGS="--cfg loom" cargo test --release
//! ```

#![cfg(loom)]

use fortress_rollback::telemetry::{
    CollectingObserver, SpecViolation, ViolationKind, ViolationObserver, ViolationSeverity,
};
use loom::sync::Arc;
use loom::thread;

/// Creates a minimal test violation for testing purposes.
///
/// Uses static string slices to minimize allocation overhead during loom's
/// exhaustive interleaving exploration.
fn make_violation(id: u32, kind: ViolationKind) -> SpecViolation {
    SpecViolation::new(
        ViolationSeverity::Warning,
        kind,
        format!("test violation {}", id),
        "loom_telemetry.rs:1",
    )
}

/// Test concurrent violations write from multiple threads.
///
/// Verifies that concurrent `on_violation()` calls don't corrupt the
/// internal state and that the total count is correct.
#[test]
fn test_concurrent_violations_write() {
    loom::model(|| {
        let observer = Arc::new(CollectingObserver::new());
        let observer1 = observer.clone();
        let observer2 = observer.clone();

        // Thread 1 adds a violation
        let t1 = thread::spawn(move || {
            observer1.on_violation(&make_violation(1, ViolationKind::FrameSync));
        });

        // Thread 2 adds a violation
        let t2 = thread::spawn(move || {
            observer2.on_violation(&make_violation(2, ViolationKind::InputQueue));
        });

        t1.join().unwrap();
        t2.join().unwrap();

        // Both violations should be recorded
        assert_eq!(observer.len(), 2, "Should have exactly 2 violations");
    });
}

/// Test concurrent violations read from multiple threads.
///
/// Verifies that concurrent `violations()` calls return consistent snapshots
/// and don't interfere with each other.
#[test]
fn test_concurrent_violations_read() {
    loom::model(|| {
        let observer = Arc::new(CollectingObserver::new());

        // Pre-populate with violations
        observer.on_violation(&make_violation(1, ViolationKind::FrameSync));
        observer.on_violation(&make_violation(2, ViolationKind::InputQueue));

        let observer1 = observer.clone();
        let observer2 = observer.clone();

        // Both threads read the violations concurrently
        let reader1 = thread::spawn(move || {
            let violations = observer1.violations();
            assert_eq!(violations.len(), 2);
            violations.len()
        });

        let reader2 = thread::spawn(move || {
            let violations = observer2.violations();
            assert_eq!(violations.len(), 2);
            violations.len()
        });

        let r1 = reader1.join().unwrap();
        let r2 = reader2.join().unwrap();

        // Both readers should see the same count
        assert_eq!(r1, 2);
        assert_eq!(r2, 2);
    });
}

/// Test concurrent read and write operations.
///
/// Verifies that mixing reads and writes doesn't cause corruption.
/// Readers should see either the old state or the new state, never partial.
#[test]
fn test_concurrent_read_write() {
    loom::model(|| {
        let observer = Arc::new(CollectingObserver::new());

        // Pre-populate with one violation
        observer.on_violation(&make_violation(0, ViolationKind::FrameSync));

        let writer = observer.clone();
        let reader = observer.clone();

        // Writer adds a violation
        let write_handle = thread::spawn(move || {
            writer.on_violation(&make_violation(1, ViolationKind::InputQueue));
        });

        // Reader reads violations
        let read_handle = thread::spawn(move || reader.len());

        write_handle.join().unwrap();
        let read_count = read_handle.join().unwrap();

        // Reader should see either 1 (before write) or 2 (after write)
        assert!(
            read_count == 1 || read_count == 2,
            "Reader should see 1 or 2 violations, got {}",
            read_count
        );

        // After both complete, should have exactly 2
        assert_eq!(observer.len(), 2);
    });
}

/// Test clear() racing with write operations.
///
/// Verifies that `clear()` and `on_violation()` racing results in
/// consistent state - either the violation is cleared or it persists.
#[test]
fn test_violations_clear_concurrent() {
    loom::model(|| {
        let observer = Arc::new(CollectingObserver::new());

        // Pre-populate
        observer.on_violation(&make_violation(0, ViolationKind::FrameSync));

        let clearer = observer.clone();
        let writer = observer.clone();

        // One thread clears
        let clear_handle = thread::spawn(move || {
            clearer.clear();
        });

        // Another thread writes
        let write_handle = thread::spawn(move || {
            writer.on_violation(&make_violation(1, ViolationKind::InputQueue));
        });

        clear_handle.join().unwrap();
        write_handle.join().unwrap();

        // Final state depends on interleaving:
        // - If write then clear: 0 violations
        // - If clear then write: 1 violation
        // - If write during clear: could be 0 or 1
        let final_count = observer.len();
        assert!(
            final_count <= 1,
            "After clear racing with single write, should have 0 or 1 violations, got {}",
            final_count
        );
    });
}

/// Test bounded preemption stress test with more threads.
///
/// Uses `loom::model::Builder` with `preemption_bound` to limit the
/// state space while still testing more complex scenarios.
#[test]
fn test_bounded_preemption_stress() {
    let mut builder = loom::model::Builder::new();
    builder.preemption_bound = Some(2);

    builder.check(|| {
        let observer = Arc::new(CollectingObserver::new());

        // Spawn 3 writer threads
        let handles: Vec<_> = (0..3)
            .map(|i| {
                let obs = observer.clone();
                thread::spawn(move || {
                    obs.on_violation(&make_violation(i, ViolationKind::FrameSync));
                })
            })
            .collect();

        // Wait for all writers
        for h in handles {
            h.join().unwrap();
        }

        // Should have exactly 3 violations
        assert_eq!(observer.len(), 3);
    });
}

/// Test has_violation() with concurrent writes.
///
/// Verifies that `has_violation()` correctly detects violations
/// even when writes are happening concurrently.
#[test]
fn test_has_violation_concurrent() {
    loom::model(|| {
        let observer = Arc::new(CollectingObserver::new());

        let writer = observer.clone();
        let checker = observer.clone();

        // Writer adds a FrameSync violation
        let write_handle = thread::spawn(move || {
            writer.on_violation(&make_violation(1, ViolationKind::FrameSync));
        });

        // Checker looks for InputQueue (which won't exist)
        let check_handle = thread::spawn(move || {
            // InputQueue was never added, so this should be false
            checker.has_violation(ViolationKind::InputQueue)
        });

        write_handle.join().unwrap();
        let has_input_queue = check_handle.join().unwrap();

        // InputQueue was never added
        assert!(!has_input_queue);

        // FrameSync should exist after write completes
        assert!(observer.has_violation(ViolationKind::FrameSync));
    });
}

/// Test has_severity() with concurrent writes.
///
/// Verifies that `has_severity()` correctly detects severity levels
/// even when writes are happening concurrently.
#[test]
fn test_has_severity_concurrent() {
    loom::model(|| {
        let observer = Arc::new(CollectingObserver::new());

        let writer = observer.clone();
        let checker = observer.clone();

        // Writer adds a Warning severity violation
        let write_handle = thread::spawn(move || {
            observer.on_violation(&make_violation(1, ViolationKind::FrameSync));
        });

        // Checker looks for Critical (which won't exist)
        let check_handle = thread::spawn(move || {
            // Critical was never added
            checker.has_severity(ViolationSeverity::Critical)
        });

        write_handle.join().unwrap();
        let has_critical = check_handle.join().unwrap();

        // Critical severity was never added
        assert!(!has_critical);

        // Warning should exist after write
        assert!(writer.has_severity(ViolationSeverity::Warning));
    });
}

/// Test is_empty() with concurrent writes.
///
/// Verifies that `is_empty()` returns correct results during concurrent writes.
#[test]
fn test_is_empty_concurrent() {
    loom::model(|| {
        let observer = Arc::new(CollectingObserver::new());

        let writer = observer.clone();
        let checker = observer.clone();

        // Check empty before any writes (in parallel with write)
        let check_handle = thread::spawn(move || checker.is_empty());

        // Writer adds a violation
        let write_handle = thread::spawn(move || {
            writer.on_violation(&make_violation(1, ViolationKind::FrameSync));
        });

        let was_empty = check_handle.join().unwrap();
        write_handle.join().unwrap();

        // The check could happen before or after the write
        // so was_empty could be true or false - both are valid depending on interleaving.
        // After write completes, should not be empty.
        assert!(!observer.is_empty());

        // Note: was_empty depends on interleaving - it may be true (checked before write)
        // or false (checked after write). Both are valid outcomes.
        let _ = was_empty;
    });
}

/// Test multiple operations from the same thread interleaved with another.
///
/// This tests a more realistic pattern where one thread does multiple
/// operations while another thread is also active.
#[test]
fn test_multiple_ops_interleaved() {
    loom::model(|| {
        let observer = Arc::new(CollectingObserver::new());

        let obs1 = observer.clone();
        let obs2 = observer.clone();

        // Thread 1: write, read, write
        let t1 = thread::spawn(move || {
            obs1.on_violation(&make_violation(1, ViolationKind::FrameSync));
            let _ = obs1.len();
            obs1.on_violation(&make_violation(2, ViolationKind::FrameSync));
        });

        // Thread 2: write
        let t2 = thread::spawn(move || {
            obs2.on_violation(&make_violation(3, ViolationKind::InputQueue));
        });

        t1.join().unwrap();
        t2.join().unwrap();

        // All 3 violations should be recorded
        assert_eq!(observer.len(), 3);
        assert!(observer.has_violation(ViolationKind::FrameSync));
        assert!(observer.has_violation(ViolationKind::InputQueue));
    });
}
