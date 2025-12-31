# Loom Testing — Concurrency Permutation Testing for Rust

> **This document provides comprehensive guidance for writing and adapting code for loom testing.**
> Loom is a deterministic concurrency testing tool that explores all possible thread interleavings to find bugs that random testing might miss.

## Core Philosophy

**Loom transforms non-deterministic concurrent code into deterministic, reproducible tests.** It achieves this by:

1. **Controlling the scheduler** — Loom simulates the OS scheduler to explore all valid execution paths
2. **Modeling memory ordering** — Based on the C11 memory model (CDSChecker paper)
3. **State space reduction** — Uses techniques to avoid combinatorial explosion
4. **Deterministic reproduction** — Failing tests can be reproduced exactly

### Key Insight

```rust
// ❌ Traditional concurrent testing: Non-deterministic, unreproducible
#[test]
fn flaky_concurrent_test() {
    let counter = Arc::new(AtomicUsize::new(0));
    // Spawn threads, hope to catch race conditions...
    // This might pass millions of times then randomly fail
}

// ✅ Loom testing: Deterministic, exhaustive, reproducible
#[test]
#[cfg(loom)]
fn deterministic_concurrent_test() {
    loom::model(|| {
        let counter = Arc::new(AtomicUsize::new(0));
        // Loom explores ALL possible interleavings
        // If there's a bug, loom WILL find it
    });
}
```

---

## When to Use Loom vs Other Tools

| Tool | Best For | Limitations |
|------|----------|-------------|
| **Loom** | Small, critical concurrent code (lock-free data structures, synchronization primitives) | State space explosion with many threads/operations |
| **Shuttle** | Larger concurrent systems, randomized testing | Not exhaustive—may miss rare bugs |
| **Miri** | Undefined behavior detection, memory safety | Single-threaded focus, limited concurrency support |
| **ThreadSanitizer** | Runtime race detection in production-like tests | May miss bugs, runtime overhead |
| **Kani** | Formal verification of bounded properties | Limited concurrent support |

### Loom Shines For

- Lock-free data structures (queues, stacks, counters)
- Custom synchronization primitives
- Atomic state machines
- Producer-consumer patterns with bounded threads
- Testing memory ordering correctness

### Loom Is NOT Suitable For

- Tests with many threads (>4 causes state explosion)
- Large integration tests
- Code with significant I/O or external dependencies
- Unbounded loops without yield points
- Testing performance characteristics

---

## Installation and Setup

### Cargo.toml Configuration

```toml
# In your main crate's Cargo.toml

[lints.rust]
# Prevent warnings about the loom cfg flag
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(loom)'] }

[features]
# Optional: explicit loom feature for clarity
loom = []

# Loom dependency only when cfg(loom) is set
[target.'cfg(loom)'.dependencies]
loom = "0.7"
```

### Running Loom Tests

```bash
# Run loom tests (always use --release for speed)
RUSTFLAGS="--cfg loom" cargo test --release

# Run specific test
RUSTFLAGS="--cfg loom" cargo test --release test_name

# With debugging output
LOOM_LOG=trace RUSTFLAGS="--cfg loom" cargo test --release

# With preemption bound (for larger tests)
LOOM_MAX_PREEMPTIONS=2 RUSTFLAGS="--cfg loom" cargo test --release
```

---

## The Sync Abstraction Pattern (CRITICAL)

The most important pattern for loom compatibility is creating an abstraction layer that swaps between `std` and `loom` types.

### Pattern 1: Basic Sync Module

```rust
// src/sync.rs - The canonical pattern

/// Synchronization primitives that work with both std and loom.
///
/// Under normal compilation, uses high-performance std/parking_lot types.
/// Under loom testing (`RUSTFLAGS="--cfg loom"`), uses loom's model-checkable types.

#[cfg(loom)]
pub(crate) mod inner {
    pub use loom::sync::Arc;
    pub use loom::sync::Mutex;
    pub use loom::sync::MutexGuard;
    pub use loom::sync::RwLock;
    pub use loom::sync::Condvar;
    pub use loom::sync::atomic::{AtomicBool, AtomicUsize, AtomicU32, AtomicU64};
    pub use loom::sync::atomic::Ordering;
    pub use loom::thread;
    pub use loom::cell::UnsafeCell;
}

#[cfg(not(loom))]
pub(crate) mod inner {
    pub use std::sync::Arc;
    pub use std::sync::Mutex;
    pub use std::sync::MutexGuard;
    pub use std::sync::RwLock;
    pub use std::sync::Condvar;
    pub use std::sync::atomic::{AtomicBool, AtomicUsize, AtomicU32, AtomicU64};
    pub use std::sync::atomic::Ordering;
    pub use std::thread;
    pub use std::cell::UnsafeCell;
}

pub(crate) use inner::*;
```

### Pattern 2: With parking_lot (Production Performance)

```rust
// src/sync.rs - Using parking_lot for production

#[cfg(loom)]
pub(crate) mod inner {
    pub use loom::sync::Arc;
    pub use loom::sync::Mutex;
    pub use loom::sync::MutexGuard;
    pub use loom::thread;
}

#[cfg(not(loom))]
pub(crate) mod inner {
    // parking_lot is faster than std::sync
    pub use parking_lot::Mutex;
    pub use parking_lot::MutexGuard;
    pub use std::sync::Arc;
    pub use std::thread;
}

pub(crate) use inner::*;
```

### Pattern 3: Model Helper Function

```rust
// src/sync.rs - Portable test helper

/// Run a loom model test. Under loom, explores all interleavings.
/// In production tests, just runs the closure once.
#[cfg(loom)]
pub fn model<F>(f: F)
where
    F: Fn() + Sync + Send + 'static,
{
    loom::model(f);
}

#[cfg(not(loom))]
pub fn model<F>(f: F)
where
    F: FnOnce(),
{
    f();
}

/// Model with custom preemption bound (for larger state spaces)
#[cfg(loom)]
pub fn model_with_bound<F>(f: F, max_preemptions: usize)
where
    F: Fn() + Sync + Send + 'static,
{
    let mut builder = loom::model::Builder::new();
    builder.preemption_bound = Some(max_preemptions);
    builder.check(f);
}

#[cfg(not(loom))]
pub fn model_with_bound<F>(f: F, _max_preemptions: usize)
where
    F: FnOnce(),
{
    f();
}
```

---

## Writing Loom Tests

### Basic Test Structure

```rust
#![cfg(loom)]  // Only compile this file under loom

use loom::sync::Arc;
use loom::sync::atomic::{AtomicUsize, Ordering};
use loom::thread;

#[test]
fn test_concurrent_increment() {
    loom::model(|| {
        // Arrange: Set up shared state
        let counter = Arc::new(AtomicUsize::new(0));

        // Act: Spawn concurrent operations
        let counter1 = counter.clone();
        let t1 = thread::spawn(move || {
            counter1.fetch_add(1, Ordering::SeqCst);
        });

        let counter2 = counter.clone();
        let t2 = thread::spawn(move || {
            counter2.fetch_add(1, Ordering::SeqCst);
        });

        t1.join().unwrap();
        t2.join().unwrap();

        // Assert: Verify invariants hold for ALL interleavings
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    });
}
```

### Testing Multiple Outcomes

When concurrent operations can produce different valid outcomes:

```rust
#[test]
#[cfg(loom)]
fn test_concurrent_saves() {
    loom::model(|| {
        let cell = Arc::new(GameStateCell::default());
        let cell1 = cell.clone();
        let cell2 = cell.clone();

        let t1 = thread::spawn(move || {
            cell1.save(Frame::new(1), Some(100), Some(0xAAAA));
        });
        let t2 = thread::spawn(move || {
            cell2.save(Frame::new(2), Some(200), Some(0xBBBB));
        });

        t1.join().unwrap();
        t2.join().unwrap();

        // Either thread could have "won" - both outcomes are valid
        let loaded = cell.load();
        assert!(
            loaded == Some(100) || loaded == Some(200),
            "Loaded value must be from one of the saves"
        );
    });
}
```

### Bounded Model Checking

For tests with larger state spaces:

```rust
#[test]
#[cfg(loom)]
fn test_with_many_threads() {
    let mut builder = loom::model::Builder::new();
    builder.preemption_bound = Some(2);  // Limit preemptions

    builder.check(|| {
        let data = Arc::new(Mutex::new(Vec::new()));

        let handles: Vec<_> = (0..4).map(|i| {
            let data = data.clone();
            thread::spawn(move || {
                data.lock().unwrap().push(i);
            })
        }).collect();

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(data.lock().unwrap().len(), 4);
    });
}
```

---

## Handling Spin Loops (CRITICAL)

Loom requires special handling for spin loops. Without yield points, loom cannot properly explore interleavings and may hang or explode in state space.

### The Problem

```rust
// ❌ DANGEROUS: Spin loop without yield
fn wait_for_value(flag: &AtomicBool) {
    while !flag.load(Ordering::Acquire) {
        // Loom cannot make progress here!
        // This will cause infinite iterations or state explosion
    }
}
```

### The Solution

```rust
// ✅ CORRECT: Add yield_now() in spin loops
fn wait_for_value(flag: &AtomicBool) {
    while !flag.load(Ordering::Acquire) {
        loom::thread::yield_now();  // Give loom scheduler a chance
    }
}

// ✅ BETTER: Portable version
fn wait_for_value(flag: &AtomicBool) {
    while !flag.load(Ordering::Acquire) {
        #[cfg(loom)]
        loom::thread::yield_now();

        #[cfg(not(loom))]
        std::hint::spin_loop();
    }
}
```

### Best Practice: Avoid Spin Loops for Loom Testing

```rust
// ⚠️ Spin loops cause state explosion even with yield_now()
// Consider redesigning to use Mutex + Condvar for testability

// ❌ Hard to test with loom
struct SpinLock {
    locked: AtomicBool,
}

// ✅ Much easier to test with loom
struct BlockingLock {
    inner: Mutex<()>,
}
```

---

## Handling API Limitations Under Loom

Some APIs (like `MappedMutexGuard`) don't exist in loom. Handle gracefully:

### Pattern: Graceful Degradation

```rust
impl<T> GameStateCell<T> {
    /// Returns accessor to inner data.
    ///
    /// # Note
    /// Under loom testing, this returns `None` because loom's `MutexGuard`
    /// doesn't support projection. Use `load()` instead in loom tests.
    #[cfg(not(loom))]
    pub fn data(&self) -> Option<GameStateAccessor<'_, T>> {
        if let Ok(mapped) = parking_lot::MutexGuard::try_map(
            self.0.lock(),
            |state| state.data.as_mut()
        ) {
            Some(GameStateAccessor(mapped))
        } else {
            None
        }
    }

    #[cfg(loom)]
    pub fn data(&self) -> Option<GameStateAccessor<'_, T>> {
        // Loom doesn't support MappedMutexGuard
        // Return None - tests should use load() instead
        let _guard = self.0.lock().unwrap();
        None
    }
}
```

### Pattern: Separate Test Helpers

```rust
#[cfg(loom)]
impl<T: Clone> GameStateCell<T> {
    /// Loom-specific test helper that clones the inner value.
    /// Only available under loom testing.
    pub fn load_clone(&self) -> Option<T> {
        self.0.lock().unwrap().data.clone()
    }
}
```

---

## Exposing Internal Types for Testing

### Pattern: `__internal` Module

```rust
// src/lib.rs

/// Internal types exposed for testing. Not part of public API.
/// These types may change without notice.
#[doc(hidden)]
pub mod __internal {
    pub use crate::sync_layer::{GameStateCell, SavedStates, SyncLayer};
    pub use crate::input_queue::InputQueue;
}
```

Usage in tests:

```rust
// loom-tests/tests/game_state_cell.rs
use fortress_rollback::__internal::GameStateCell;
```

---

## Debugging Loom Failures

### Environment Variables

| Variable | Purpose | Example |
|----------|---------|---------|
| `LOOM_LOG` | Enable logging | `LOOM_LOG=trace` |
| `LOOM_LOCATION` | Include source locations | `LOOM_LOCATION=1` |
| `LOOM_CHECKPOINT_FILE` | Save/restore state | `LOOM_CHECKPOINT_FILE=check.json` |
| `LOOM_CHECKPOINT_INTERVAL` | Save frequency | `LOOM_CHECKPOINT_INTERVAL=1000` |
| `LOOM_MAX_PREEMPTIONS` | Limit preemptions | `LOOM_MAX_PREEMPTIONS=2` |
| `LOOM_MAX_BRANCHES` | Limit branches | `LOOM_MAX_BRANCHES=10000` |

### Debugging Workflow

```bash
# 1. First, get detailed logging
LOOM_LOG=trace LOOM_LOCATION=1 RUSTFLAGS="--cfg loom" cargo test --release failing_test 2>&1 | head -500

# 2. If test is long-running, use checkpoints
LOOM_CHECKPOINT_FILE=debug.json LOOM_CHECKPOINT_INTERVAL=1000 \
    RUSTFLAGS="--cfg loom" cargo test --release failing_test

# 3. Resume from checkpoint after fixing
LOOM_CHECKPOINT_FILE=debug.json RUSTFLAGS="--cfg loom" cargo test --release failing_test
```

### Adding Debug Output in Tests

```rust
#[test]
#[cfg(loom)]
fn test_with_debug() {
    loom::model(|| {
        let state = Arc::new(AtomicUsize::new(0));

        let s1 = state.clone();
        let t1 = thread::spawn(move || {
            let old = s1.fetch_add(1, Ordering::SeqCst);
            // This will appear in LOOM_LOG=trace output
            tracing::debug!("Thread 1: {} -> {}", old, old + 1);
        });

        // ...
    });
}
```

---

## Separate Loom Test Crate Pattern

For projects with dependencies that don't support loom (tokio, hyper, etc.), use a separate test crate:

### Directory Structure

```
your-crate/
├── src/
│   ├── lib.rs
│   └── sync.rs          # Sync abstraction
├── Cargo.toml
└── loom-tests/
    ├── Cargo.toml
    ├── README.md
    └── tests/
        ├── basic.rs
        └── data_structure.rs
```

### loom-tests/Cargo.toml

```toml
[package]
name = "your-crate-loom-tests"
version = "0.1.0"
edition = "2021"
publish = false

[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(loom)'] }

[dependencies]
your-crate = { path = ".." }

[dev-dependencies]
loom = "0.7"
```

### loom-tests/tests/basic.rs

```rust
#![cfg(loom)]

use loom::sync::Arc;
use loom::thread;
use your_crate::__internal::SomeType;

#[test]
fn test_something() {
    loom::model(|| {
        // Test code using loom primitives
    });
}
```

### Running

```bash
cd loom-tests
RUSTFLAGS="--cfg loom" cargo test --release
```

---

## Common Pitfalls and Solutions

### Pitfall 1: Using std Types Instead of loom Types

```rust
// ❌ WRONG: std::sync is invisible to loom
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

#[test]
#[cfg(loom)]
fn test() {
    loom::model(|| {
        let x = Arc::new(AtomicUsize::new(0));
        // Loom can't see these operations!
    });
}

// ✅ CORRECT: Use loom types
use loom::sync::Arc;
use loom::sync::atomic::AtomicUsize;

#[test]
#[cfg(loom)]
fn test() {
    loom::model(|| {
        let x = Arc::new(AtomicUsize::new(0));
        // Now loom controls these
    });
}
```

### Pitfall 2: Non-Determinism Outside Loom

```rust
// ❌ WRONG: rand is non-deterministic, invisible to loom
use rand::Rng;

#[test]
#[cfg(loom)]
fn test() {
    loom::model(|| {
        let n = rand::thread_rng().gen::<usize>() % 10;
        // Loom can't explore different random values!
    });
}

// ✅ CORRECT: All non-determinism must come through loom
#[test]
#[cfg(loom)]
fn test() {
    loom::model(|| {
        // Test ALL relevant values explicitly
        for n in 0..10 {
            test_with_value(n);
        }
    });
}
```

### Pitfall 3: Forgetting cfg(loom) Guard

```rust
// ❌ WRONG: Test runs under normal cargo test too (and hangs)
#[test]
fn test_concurrent() {
    loom::model(|| {  // Error: loom not available!
        // ...
    });
}

// ✅ CORRECT: Guard with cfg(loom)
#[test]
#[cfg(loom)]
fn test_concurrent() {
    loom::model(|| {
        // ...
    });
}
```

### Pitfall 4: State Explosion

```rust
// ❌ DANGEROUS: Too many threads/operations
#[test]
#[cfg(loom)]
fn test_too_big() {
    loom::model(|| {
        // 8 threads = astronomical state space
        let handles: Vec<_> = (0..8).map(|_| {
            thread::spawn(|| { /* ... */ })
        }).collect();
    });
}

// ✅ BETTER: Keep state space small
#[test]
#[cfg(loom)]
fn test_right_size() {
    loom::model(|| {
        // 2-3 threads is usually sufficient to find bugs
        let t1 = thread::spawn(|| { /* ... */ });
        let t2 = thread::spawn(|| { /* ... */ });
    });
}
```

### Pitfall 5: SeqCst Limitations

```rust
// ⚠️ WARNING: Loom treats SeqCst as AcqRel
// This means some SeqCst bugs may not be found

// Loom cannot detect this bug:
let x = AtomicUsize::new(0);
let y = AtomicUsize::new(0);

// Thread 1: x.store(1, SeqCst); y.load(SeqCst);
// Thread 2: y.store(1, SeqCst); x.load(SeqCst);
// SeqCst should prevent both loads seeing 0, but loom may miss this
```

---

## Memory Ordering Quick Reference

| Ordering | Loom Support | Notes |
|----------|--------------|-------|
| `Relaxed` | Partial | Cannot model all reorderings |
| `Acquire` | Full | |
| `Release` | Full | |
| `AcqRel` | Full | |
| `SeqCst` | Treated as AcqRel | May produce false negatives |

---

## Alternative: AWS Shuttle for Larger Tests

When loom's exhaustive testing doesn't scale, consider Shuttle:

```toml
[dev-dependencies]
shuttle = "0.8"
```

```rust
use shuttle::sync::{Arc, Mutex};
use shuttle::thread;

#[test]
fn test_with_shuttle() {
    // Random exploration, not exhaustive
    shuttle::check_random(|| {
        let data = Arc::new(Mutex::new(0));
        // ... test with many threads
    }, 1000);  // Run 1000 random schedules
}
```

### Loom vs Shuttle Trade-offs

| Aspect | Loom | Shuttle |
|--------|------|---------|
| Coverage | Exhaustive (for bounded state) | Randomized sampling |
| Scalability | Limited (2-4 threads) | Better (many threads) |
| Soundness | Sound (if it passes, it's correct) | Not sound (bugs may be missed) |
| Speed | Slower (explores everything) | Faster (samples randomly) |

---

## Integration with Project CI

### GitHub Actions Example (Cross-Platform)

**Loom tests should run cross-platform** because concurrency bugs can manifest differently across operating systems due to different scheduler behaviors, memory models, and threading implementations.

```yaml
loom-tests:
  name: Loom Concurrency Tests (${{ matrix.os }})
  runs-on: ${{ matrix.os }}
  timeout-minutes: 30
  strategy:
    fail-fast: false  # Run all platforms even if one fails
    matrix:
      os: [ubuntu-latest, windows-latest, macos-latest]

  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable

    - name: Cache cargo registry and build
      uses: actions/cache@v4
      with:
        path: |
          ~/.cargo/registry
          ~/.cargo/git
          loom-tests/target
        key: loom-${{ matrix.os }}-cargo-${{ hashFiles('loom-tests/Cargo.lock') }}
        restore-keys: |
          loom-${{ matrix.os }}-cargo-

    - name: Run loom tests
      working-directory: loom-tests
      run: cargo test --release
      env:
        RUSTFLAGS: "--cfg loom"
        LOOM_MAX_PREEMPTIONS: 3
```

### Why Cross-Platform Loom Testing?

| Platform | Potential Differences |
|----------|----------------------|
| **Linux** | Uses futex-based primitives, NPTL threading |
| **macOS** | GCD integration, different scheduler priorities |
| **Windows** | SRW locks, different thread scheduling |

A concurrency bug that's rare on Linux might be common on Windows due to different scheduler heuristics.

---

## Summary Checklist

When adapting code for loom testing:

- [ ] Create sync abstraction module (`src/sync.rs`) that swaps std/loom types
- [ ] Use `#![cfg(loom)]` at top of loom test files
- [ ] Import from `loom::` directly in tests, not through sync module
- [ ] Add `loom::thread::yield_now()` in any spin loops
- [ ] Use `Arc::new()` pattern for sharing data between threads
- [ ] Test multiple valid outcomes when order doesn't matter
- [ ] Keep thread count to 2-3 for tractable state space
- [ ] Use `Builder::preemption_bound` for larger tests
- [ ] Run with `--release` flag for performance
- [ ] Guard APIs that don't work under loom with graceful degradation
- [ ] Consider separate loom-tests crate if dependencies don't support loom
- [ ] Document loom limitations in affected APIs

---

*See also: [defensive-programming.md](defensive-programming.md) for error handling patterns, [rust-pitfalls.md](rust-pitfalls.md) for common concurrency bugs.*
