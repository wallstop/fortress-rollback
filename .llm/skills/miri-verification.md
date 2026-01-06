# Miri Verification — Undefined Behavior Detection for Rust

> **This document provides comprehensive guidance for using Miri to detect undefined behavior in Rust code.**
> Miri is an interpreter for Rust's Mid-level Intermediate Representation (MIR) that can detect many forms of undefined behavior at runtime.

## Core Philosophy

**Miri finds undefined behavior by interpreting your code and checking for violations.** It achieves this by:

1. **Interpreting MIR** — Executes your program step-by-step on unoptimized MIR
2. **Tracking provenance** — Every pointer knows where it came from and what it can access
3. **Aliasing validation** — Checks Stacked Borrows or Tree Borrows rules
4. **Detecting UB** — Reports when code violates Rust's memory and safety model

### Key Insight

```rust
// This code compiles and runs "correctly" but contains UB
fn dangerous() {
    let mut x = 0u32;
    let ptr = &mut x as *mut u32;
    let r = &x;  // Creates shared reference while mutable pointer exists
    unsafe {
        *ptr = 42;  // UB: Writing through ptr while r exists (Stacked Borrows violation)
    }
    println!("{}", r);
}

// Miri catches this:
// error: Undefined Behavior: attempting a write access using <tag>
//        at alloc123, but that tag does not exist in the borrow stack
```

---

## What Miri Detects

### Definite Undefined Behavior (Always Detected)

| Category | Examples |
|----------|----------|
| **Memory access** | Out-of-bounds reads/writes, use-after-free |
| **Uninitialized data** | Reading uninitialized memory |
| **Alignment** | Misaligned pointer dereferences, misaligned references |
| **Type invariants** | Invalid `bool` (not 0/1), invalid enum discriminant, null references |
| **Intrinsic violations** | `unreachable_unchecked` reached, overlapping `copy_nonoverlapping` |
| **Data races** | Unsynchronized access to shared data |
| **Memory leaks** | Unreachable allocations at program end |

### Experimental: Aliasing Rules

Miri checks experimental aliasing models that may change:

| Model | Description | Flag |
|-------|-------------|------|
| **Stacked Borrows** (default) | Original strict model for reference/pointer aliasing | (default) |
| **Tree Borrows** | Newer, more permissive model | `-Zmiri-tree-borrows` |

---

## What Miri Does NOT Detect

Understanding limitations is critical for defense in depth:

| Limitation | Why | Mitigation |
|------------|-----|------------|
| **Non-deterministic bugs** | Miri tests ONE execution path | Use `-Zmiri-many-seeds` |
| **All thread interleavings** | Single execution per run | Use Loom for exhaustive concurrency |
| **Incomplete weak memory** | Not all C11 behaviors produced | Use Loom for atomics verification |
| **FFI/foreign functions** | Cannot interpret C code | Mock FFI, use sanitizers |
| **Platform-specific APIs** | Limited syscall support | Test on native with sanitizers |
| **Layout-dependent bugs** | May work "by chance" | Use `-Zrandomize-layout` |
| **Future UB** | Based on current understanding | Stay updated on Rust semantics |

---

## Installation and Setup

### Basic Installation

```bash
# Install Miri component on nightly
rustup +nightly component add miri

# Pin to nightly for a project (optional)
rustup override set nightly

# One-time setup (downloads dependencies)
cargo miri setup
```

### Running Miri

```bash
# Run all tests through Miri
cargo miri test

# Run specific test
cargo miri test test_name

# Run binary through Miri
cargo miri run

# Clean Miri artifacts (useful after toolchain updates)
cargo miri clean
```

---

## Essential Flags Reference

Flags are passed via the `MIRIFLAGS` environment variable:

```bash
MIRIFLAGS="-Zmiri-flag1 -Zmiri-flag2" cargo miri test
```

### Most Important Flags

| Flag | Purpose | When to Use |
|------|---------|-------------|
| `-Zmiri-disable-isolation` | Access host filesystem, env vars, real RNG | When tests need external resources |
| `-Zmiri-tree-borrows` | Use Tree Borrows instead of Stacked Borrows | When SB is too strict for valid code |
| `-Zmiri-many-seeds=0..N` | Test N different random executions | Finding race conditions (default: 64) |
| `-Zmiri-seed=<num>` | Set specific RNG seed | Reproducing a specific failure |
| `-Zmiri-strict-provenance` | Enforce strict pointer provenance | Catching int-to-pointer issues |
| `-Zmiri-symbolic-alignment-check` | Stricter alignment validation | Finding "lucky alignment" bugs |
| `-Zmiri-deterministic-concurrency` | Fully deterministic scheduling | Reproducible concurrent tests |

### Debugging Flags

| Flag | Purpose |
|------|---------|
| `-Zmiri-backtrace=full` | Full backtraces on errors |
| `-Zmiri-track-alloc-id=<id>` | Debug specific allocation lifecycle |
| `-Zmiri-track-pointer-tag=<tag>` | Debug specific pointer's borrow stack |
| `-Zmiri-report-progress` | Show execution progress |

### Performance Flags (Trade Safety for Speed)

| Flag | Purpose | ⚠️ Warning |
|------|---------|-----------|
| `-Zmiri-disable-stacked-borrows` | Skip aliasing checks | **UNSOUND** — misses UB |
| `-Zmiri-disable-validation` | Skip type invariant checks | **UNSOUND** — misses UB |
| `-Zmiri-disable-alignment-check` | Skip alignment checks | **UNSOUND** — misses UB |
| `-Zmiri-ignore-leaks` | Don't report memory leaks | May hide bugs |

### Cross-Platform Testing

```bash
# Test as Linux program (works from any host)
cargo miri test --target x86_64-unknown-linux-gnu

# Test big-endian behavior
cargo miri test --target s390x-unknown-linux-gnu

# Test 32-bit behavior
cargo miri test --target i686-unknown-linux-gnu
```

---

## Stacked Borrows vs Tree Borrows

### Stacked Borrows (Default)

The original aliasing model where borrows form a **stack**:

- Creating a reference pushes to the stack
- Using an older pointer pops everything above it
- Popped pointers are permanently invalid

**Strict behaviors that may reject valid code:**

```rust
// ❌ FAILS under Stacked Borrows: &mut asserts immediate uniqueness
let mut arr = [0, 1];
let from = arr.as_ptr();
let to = arr.as_mut_ptr().add(1);  // Invalidates `from`!
std::ptr::copy_nonoverlapping(from, to, 1);  // UB: from is invalid
```

### Tree Borrows (Experimental Alternative)

Enable with: `MIRIFLAGS="-Zmiri-tree-borrows"`

Borrows form a **tree** with delayed uniqueness:

- `&mut` starts "Reserved", becomes "Active" on first write
- More permissive with pointer arithmetic
- Better matches programmer intuition

```rust
// ✅ PASSES under Tree Borrows: delayed uniqueness
let mut arr = [0, 1];
let from = arr.as_ptr();
let to = arr.as_mut_ptr().add(1);  // `from` still valid (no write yet)
std::ptr::copy_nonoverlapping(from, to, 1);  // OK
```

### When to Use Tree Borrows

- Code rejected by Stacked Borrows that seems correct
- `container_of` patterns (pointer arithmetic beyond type bounds)
- Complex intrusive data structures
- When Stacked Borrows is too conservative

**Note:** Tree Borrows is experimental and may change. If code passes Tree Borrows but fails Stacked Borrows, document the deviation.

---

## Adapting Code for Miri Compatibility

### Skip Unsupported Tests

```rust
#[test]
#[cfg_attr(miri, ignore)]
fn uses_networking() {
    // Miri can't run network code
}

#[test]
#[cfg_attr(miri, ignore)]
fn calls_ffi() {
    // Miri can't interpret C code
}
```

### Reduce Iteration Counts

Miri is ~1000-7000x slower than native execution:

```rust
fn iteration_count() -> usize {
    if cfg!(miri) { 10 } else { 10_000 }
}

#[test]
fn stress_test() {
    for _ in 0..iteration_count() {
        // ... test logic
    }
}
```

### Fix Raw Pointer Aliasing Issues

**The Golden Rule:** Once you start using raw pointers, stay with raw pointers.

```rust
// ❌ FAILS: Box access invalidates raw pointer
struct BadList<T> {
    head: Option<Box<Node<T>>>,
    tail: *mut Node<T>,  // Points into the Box
}

impl<T> BadList<T> {
    fn push(&mut self, elem: T) {
        let node = Box::new(Node { elem, next: ptr::null_mut() });
        let raw = &*node as *const _ as *mut Node<T>;

        // Accessing self.head here invalidates `raw`!
        if let Some(ref mut head) = self.head {
            // ...
        }
    }
}

// ✅ WORKS: All raw pointers, no Box
struct GoodList<T> {
    head: *mut Node<T>,
    tail: *mut Node<T>,
}

impl<T> GoodList<T> {
    fn push(&mut self, elem: T) {
        let new = Box::into_raw(Box::new(Node { elem, next: ptr::null_mut() }));

        unsafe {
            if !self.tail.is_null() {
                (*self.tail).next = new;
            } else {
                self.head = new;
            }
            self.tail = new;
        }
    }

    // Remember to free with Box::from_raw in Drop!
}
```

### Handle Integer-to-Pointer Casts

Miri has limited ability to track provenance through integer casts:

```rust
// ⚠️ Problematic: Provenance lost through integer
let x = 42u64;
let ptr = &x as *const u64;
let addr = ptr as usize;
let recovered = addr as *const u64;  // Miri may lose provenance tracking
unsafe { *recovered }  // May cause Miri error

// ✅ Better: Use strict provenance APIs (nightly)
#![feature(strict_provenance)]
let ptr = std::ptr::without_provenance::<u64>(addr);
// Or use -Zmiri-permissive-provenance to silence warnings
```

---

## CI Integration

### GitHub Actions Example (Cross-Platform)

**Miri tests should run cross-platform** because undefined behavior detection can catch platform-specific issues like memory layout differences, alignment requirements, pointer sizes, and endianness.

```yaml
miri:
  name: Miri UB Check (${{ matrix.os }})
  runs-on: ${{ matrix.os }}
  timeout-minutes: 30
  strategy:
    fail-fast: false  # Run all platforms even if one fails
    matrix:
      os: [ubuntu-latest, windows-latest, macos-latest]

  steps:
    - uses: actions/checkout@v4

    - name: Install nightly with Miri
      uses: dtolnay/rust-toolchain@master
      with:
        toolchain: nightly
        components: miri

    - name: Cache cargo registry and build
      uses: actions/cache@v4
      with:
        path: |
          ~/.cargo/registry
          ~/.cargo/git
          target
        key: miri-${{ matrix.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
        restore-keys: |
          miri-${{ matrix.os }}-cargo-

    - name: Setup Miri
      run: cargo miri setup

    - name: Run Miri tests
      run: cargo miri test --lib
      env:
        MIRIFLAGS: "-Zmiri-disable-isolation"
```

### Why Cross-Platform Miri Testing?

| Platform | Potential Differences |
|----------|----------------------|
| **Linux** | Specific pointer sizes, alignment rules |
| **macOS** | Different allocation patterns, alignment |
| **Windows** | Different ABI, calling conventions |

Additionally, Miri supports cross-target testing even on a single platform:

```bash
# Test big-endian behavior (from any host)
cargo miri test --target s390x-unknown-linux-gnu

# Test 32-bit behavior
cargo miri test --target i686-unknown-linux-gnu
```

### Advanced CI with Multiple Seeds

```yaml
    - name: Run Miri with multiple seeds
      run: cargo miri test --lib
      env:
        MIRIFLAGS: "-Zmiri-disable-isolation -Zmiri-many-seeds=0..16"
```

### Parallel Execution with Nextest

```bash
# Run tests in parallel (each test isolated)
cargo miri nextest run -jN

# ⚠️ Warning: nextest won't detect data races BETWEEN tests
```

---

## Debugging Miri Errors

### Common Error: "tag does not exist in the borrow stack"

```
error: Undefined Behavior: attempting a read access using <1234>
       at alloc456[0x0], but that tag does not exist in the borrow stack
```

**What it means:** A pointer was used after its permission was revoked.

**How to debug:**

```bash
# Track the specific allocation
MIRIFLAGS="-Zmiri-track-alloc-id=456" cargo miri test

# Track the specific pointer tag
MIRIFLAGS="-Zmiri-track-pointer-tag=1234" cargo miri test
```

**Common fixes:**

1. Minimize reference↔pointer conversions
2. Convert to raw pointers earlier in the function
3. Try Tree Borrows if code seems correct

### Common Error: "memory access to dangling pointer"

```
error: Undefined Behavior: dereferencing pointer failed:
       pointer is dangling (pointing to freed memory)
```

**What it means:** Use-after-free.

**How to debug:**

```bash
MIRIFLAGS="-Zmiri-track-alloc-id=<id>" cargo miri test
```

Look for where the allocation was freed vs. where it was used.

### Common Error: "accessing memory with alignment X, but alignment Y is required"

**What it means:** Pointer doesn't meet alignment requirements.

**Stricter checking:**

```bash
MIRIFLAGS="-Zmiri-symbolic-alignment-check" cargo miri test
```

---

## Performance Optimization with Miri

While Miri is primarily for correctness, it can aid performance analysis:

### Enable Profiling

```bash
# Install the crox tool
cargo install --git https://github.com/rust-lang/miri --branch stable crox

# Run with profiling
MIRIFLAGS="-Zmiri-disable-isolation -Zmiri-measureme=profile" cargo miri run

# Convert to Chrome DevTools format
crox profile.mm_profdata
```

Load `chrome_profiler.json` in Chrome DevTools (Performance tab → Load profile).

### Interpreting Results

**Key insight:** Miri runs on unoptimized MIR, so:

- Relative function costs are meaningful
- Absolute times are NOT meaningful (~7000x slower than native)
- Structural inefficiencies (extra allocations, unnecessary work) often carry over to release builds
- Always verify optimizations with native benchmarks

---

## Combining Miri with Other Tools

| Tool | Purpose | When to Use Together |
|------|---------|---------------------|
| **Loom** | Exhaustive concurrency testing | Miri's weak memory is incomplete |
| **Kani** | Formal verification | Prove properties, not just test |
| **ThreadSanitizer** | Native race detection | FFI, production-like testing |
| **AddressSanitizer** | Native memory errors | FFI boundaries |
| **Fuzzing** | Input space exploration | Broader coverage than fixed tests |
| **Property testing** | Generate many inputs | `proptest`, `quickcheck` |

### Recommended Testing Pyramid

```
┌─────────────────────┐
│   Formal Proofs     │  ← Kani, TLA+ (critical invariants)
├─────────────────────┤
│       Miri          │  ← All unsafe code, UB detection
├─────────────────────┤
│       Loom          │  ← Concurrent data structures
├─────────────────────┤
│  Property Testing   │  ← Edge cases, invariants
├─────────────────────┤
│    Unit Tests       │  ← Basic functionality
└─────────────────────┘
```

---

## Best Practices Summary

### DO

- ✅ Run Miri in CI on all code with `unsafe`
- ✅ Use `-Zmiri-many-seeds` for concurrent code
- ✅ Cross-test with `--target s390x-unknown-linux-gnu` (big-endian)
- ✅ Use `cfg!(miri)` to reduce iteration counts
- ✅ Track down root causes of aliasing errors
- ✅ Try Tree Borrows if Stacked Borrows seems overly strict
- ✅ Document any `#[cfg_attr(miri, ignore)]` with reasoning

### DON'T

- ❌ Disable aliasing checks to "fix" failures (`-Zmiri-disable-stacked-borrows`)
- ❌ Ignore Miri errors without understanding them
- ❌ Assume passing Miri means code is sound (it's necessary, not sufficient)
- ❌ Use Miri's fake RNG for cryptographic testing
- ❌ Run full integration tests through Miri (too slow)

---

## Quick Reference

### Common Commands

```bash
# Basic test run
cargo miri test

# With isolation disabled (for filesystem/env access)
MIRIFLAGS="-Zmiri-disable-isolation" cargo miri test

# Test concurrency with multiple seeds
MIRIFLAGS="-Zmiri-many-seeds=0..16" cargo miri test

# Use Tree Borrows model
MIRIFLAGS="-Zmiri-tree-borrows" cargo miri test

# Strict provenance checking
MIRIFLAGS="-Zmiri-strict-provenance" cargo miri test

# Debug specific allocation
MIRIFLAGS="-Zmiri-track-alloc-id=123" cargo miri test

# Cross-platform (big-endian)
cargo miri test --target s390x-unknown-linux-gnu

# Clean Miri build artifacts
cargo miri clean
```

### Conditional Compilation

```rust
// Skip test under Miri
#[cfg_attr(miri, ignore)]

// Miri-specific code
if cfg!(miri) { /* ... */ }

// Miri-only module
#[cfg(miri)]
mod miri_tests { /* ... */ }
```

---

## Further Reading

- [Miri GitHub Repository](https://github.com/rust-lang/miri)
- [Stacked Borrows Paper](https://plv.mpi-sws.org/rustbelt/stacked-borrows/)
- [Tree Borrows Paper](https://perso.crans.org/vanille/treebor/) (PLDI 2025)
- [Ralf Jung's Blog](https://www.ralfj.de/blog/)
- [UCG (Unsafe Code Guidelines)](https://rust-lang.github.io/unsafe-code-guidelines/)

---

*This guide is part of the Fortress Rollback verification toolkit. Miri complements Loom (concurrency), Kani (formal verification), and Z3 (SMT proofs).*
