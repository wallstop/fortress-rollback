<!-- CATEGORY: Formal Verification -->
<!-- WHEN: Testing concurrent code with Loom, model checking thread interleavings -->

# Loom Concurrency Testing

Loom deterministically explores all possible thread interleavings to find concurrency bugs that random testing misses.

## Setup

```toml
# Cargo.toml
[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(loom)'] }

[target.'cfg(loom)'.dependencies]
loom = "0.7"
```

```bash
RUSTFLAGS="--cfg loom" cargo test --release              # Run loom tests
RUSTFLAGS="--cfg loom" cargo test --release test_name     # Specific test
LOOM_MAX_PREEMPTIONS=2 RUSTFLAGS="--cfg loom" cargo test --release  # Bounded
```

## Sync Abstraction Pattern (CRITICAL)

```rust
// src/sync.rs
#[cfg(loom)]
pub(crate) mod inner {
    pub use loom::sync::Arc;
    pub use loom::sync::Mutex;
    pub use loom::sync::atomic::{AtomicBool, AtomicUsize, AtomicU32, AtomicU64};
    pub use loom::sync::atomic::Ordering;
    pub use loom::thread;
    pub use loom::cell::UnsafeCell;
}

#[cfg(not(loom))]
pub(crate) mod inner {
    pub use std::sync::Arc;
    pub use std::sync::Mutex;
    pub use std::sync::atomic::{AtomicBool, AtomicUsize, AtomicU32, AtomicU64};
    pub use std::sync::atomic::Ordering;
    pub use std::thread;
    pub use std::cell::UnsafeCell;
}

pub(crate) use inner::*;
```

### Portable Model Helper

```rust
#[cfg(loom)]
pub fn model<F: Fn() + Sync + Send + 'static>(f: F) { loom::model(f); }

#[cfg(not(loom))]
pub fn model<F: FnOnce()>(f: F) { f(); }
```

## Writing Loom Tests

```rust
#![cfg(loom)]

use loom::sync::Arc;
use loom::sync::atomic::{AtomicUsize, Ordering};
use loom::thread;

#[test]
fn test_concurrent_increment() {
    loom::model(|| {
        let counter = Arc::new(AtomicUsize::new(0));

        let c1 = counter.clone();
        let t1 = thread::spawn(move || { c1.fetch_add(1, Ordering::SeqCst); });

        let c2 = counter.clone();
        let t2 = thread::spawn(move || { c2.fetch_add(1, Ordering::SeqCst); });

        t1.join().unwrap();
        t2.join().unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    });
}
```

### Testing Multiple Valid Outcomes

```rust
#[test]
#[cfg(loom)]
fn test_concurrent_saves() {
    loom::model(|| {
        let cell = Arc::new(GameStateCell::default());
        let c1 = cell.clone();
        let c2 = cell.clone();

        let t1 = thread::spawn(move || { c1.save(Frame::new(1), Some(100), Some(0xAAAA)); });
        let t2 = thread::spawn(move || { c2.save(Frame::new(2), Some(200), Some(0xBBBB)); });

        t1.join().unwrap();
        t2.join().unwrap();

        let loaded = cell.load();
        assert!(loaded == Some(100) || loaded == Some(200));
    });
}
```

### Bounded Model Checking

```rust
#[test]
#[cfg(loom)]
fn test_with_many_threads() {
    let mut builder = loom::model::Builder::new();
    builder.preemption_bound = Some(2);
    builder.check(|| {
        // ... test with larger state space
    });
}
```

## Handling Spin Loops (CRITICAL)

```rust
// WRONG: Loom cannot make progress
fn wait(flag: &AtomicBool) {
    while !flag.load(Ordering::Acquire) { }  // Infinite under loom
}

// CORRECT: Portable version
fn wait(flag: &AtomicBool) {
    while !flag.load(Ordering::Acquire) {
        #[cfg(loom)]
        loom::thread::yield_now();
        #[cfg(not(loom))]
        std::hint::spin_loop();
    }
}
```

Better yet: avoid spin loops for loom-tested code. Use `Mutex` + `Condvar`.

## Handling API Limitations

Some APIs (like `MappedMutexGuard`) don't exist in loom:

```rust
#[cfg(not(loom))]
pub fn data(&self) -> Option<GameStateAccessor<'_, T>> {
    // parking_lot MappedMutexGuard
}

#[cfg(loom)]
pub fn data(&self) -> Option<GameStateAccessor<'_, T>> {
    None  // Loom doesn't support this; tests should use load()
}
```

## Exposing Internals for Testing

```rust
// src/lib.rs
#[doc(hidden)]
pub mod __internal {
    pub use crate::sync_layer::GameStateCell;
}

// loom-tests/tests/test.rs
use fortress_rollback::__internal::GameStateCell;
```

## Separate Loom Test Crate

For projects with dependencies that don't support loom:

```
your-crate/
  src/
  loom-tests/
    Cargo.toml        # depends on your-crate, dev-depends on loom
    tests/basic.rs    # #![cfg(loom)]
```

```bash
cd loom-tests && RUSTFLAGS="--cfg loom" cargo test --release
```

## Debugging

| Variable | Purpose |
|----------|---------|
| `LOOM_LOG=trace` | Detailed logging |
| `LOOM_LOCATION=1` | Source locations |
| `LOOM_CHECKPOINT_FILE=f.json` | Save/restore state |
| `LOOM_MAX_PREEMPTIONS=2` | Limit preemptions |
| `LOOM_MAX_BRANCHES=10000` | Limit branches |

## Memory Ordering Support

| Ordering | Loom Support |
|----------|-------------|
| `Relaxed` | Partial (cannot model all reorderings) |
| `Acquire` | Full |
| `Release` | Full |
| `AcqRel` | Full |
| `SeqCst` | Treated as AcqRel (may produce false negatives) |

## Common Pitfalls

| Pitfall | Fix |
|---------|-----|
| Using `std::sync` instead of `loom::sync` | Import from loom in test files |
| Non-determinism outside loom (rand, time) | All non-determinism must come through loom |
| Missing `#[cfg(loom)]` guard | Tests hang under normal `cargo test` |
| Too many threads (>4) | Keep to 2-3 for tractable state space |
| SeqCst limitations | Loom treats as AcqRel; may miss some bugs |

## CI Integration

```yaml
loom-tests:
  runs-on: ${{ matrix.os }}
  strategy:
    fail-fast: false
    matrix:
      os: [ubuntu-latest, windows-latest, macos-latest]
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - name: Run loom tests
      working-directory: loom-tests
      run: cargo test --release
      env:
        RUSTFLAGS: "--cfg loom"
        LOOM_MAX_PREEMPTIONS: 3
```

Cross-platform because concurrency bugs manifest differently per OS scheduler.

## Loom vs Shuttle

| Aspect | Loom | Shuttle |
|--------|------|---------|
| Coverage | Exhaustive (bounded) | Randomized sampling |
| Scalability | Limited (2-4 threads) | Better (many threads) |
| Soundness | If it passes, it's correct | Bugs may be missed |
| Speed | Slower | Faster |

Consider Shuttle (`shuttle::check_random`) when loom's exhaustive testing doesn't scale.
