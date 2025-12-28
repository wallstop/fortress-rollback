# Rust Performance Book - Key Optimization Techniques

> **Source**: [The Rust Performance Book](https://nnethercote.github.io/perf-book/) by Nicholas Nethercote
>
> This document summarizes the top actionable performance optimization techniques for Rust code.

---

## Table of Contents

1. [Profiling](#1-profiling)
2. [Benchmarking](#2-benchmarking)
3. [Build Configuration](#3-build-configuration)
4. [Type Sizes](#4-type-sizes)
5. [Heap Allocations](#5-heap-allocations)
6. [Standard Library Types](#6-standard-library-types)
7. [Hashing](#7-hashing)
8. [Iterators](#8-iterators)
9. [Inlining](#9-inlining)
10. [I/O](#10-io)
11. [Bounds Checks](#11-bounds-checks)

---

## 1. Profiling

### Tip 1: Choose the Right Profiler

| Profiler | Platform | Best For |
|----------|----------|----------|
| **perf** | Linux | General-purpose, hardware counters |
| **samply** | Mac/Linux/Windows | Sampling profiler with Firefox Profiler UI |
| **flamegraph** | Linux/DTrace platforms | Visual flame graphs |
| **DHAT** | Linux/Unix | Heap allocation profiling |
| **Cachegrind/Callgrind** | Linux/Unix | Instruction counts, cache simulation |

### Tip 2: Enable Debug Info for Release Builds

```toml
# Cargo.toml
[profile.release]
debug = "line-tables-only"
```

This allows profilers to show source line information in release builds without full debug overhead.

### Tip 3: Force Frame Pointers for Better Stack Traces

```bash
RUSTFLAGS="-C force-frame-pointers=yes" cargo build --release
```

Or in `.cargo/config.toml`:
```toml
[build]
rustflags = ["-C", "force-frame-pointers=yes"]
```

### Tip 4: Use v0 Symbol Mangling for Better Demangling

```bash
RUSTFLAGS="-C symbol-mangling-version=v0" cargo build --release
```

---

## 2. Benchmarking

### Tip 5: Use Proper Benchmarking Tools

| Tool | Use Case |
|------|----------|
| **Criterion** | Sophisticated microbenchmarks with statistical analysis |
| **Divan** | Modern alternative to Criterion |
| **Hyperfine** | Command-line benchmarking |
| **Bencher** | Continuous benchmarking in CI |

### Tip 6: Choose Appropriate Metrics

- **Wall-time**: What users perceive, but high variance
- **Instruction counts**: Lower variance, good for detecting regressions
- **Cycles**: CPU-level metric, less affected by system noise

> **Key principle**: Mediocre benchmarking is far better than no benchmarking.

---

## 3. Build Configuration

### Tip 7: Maximize Runtime Speed Configuration

```toml
# Cargo.toml - Maximum performance
[profile.release]
codegen-units = 1    # Better optimization, slower compile
lto = "fat"          # Whole-program optimization
panic = "abort"      # Smaller/faster, no unwinding
```

### Tip 8: Use CPU-Specific Instructions

```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

This enables AVX/SSE instructions specific to your CPU. Only use when broad compatibility isn't needed.

### Tip 9: Consider Alternative Allocators

**jemalloc** (Linux/Mac):
```toml
[dependencies]
tikv-jemallocator = "0.5"
```
```rust
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
```

**mimalloc** (Cross-platform):
```toml
[dependencies]
mimalloc = "0.1"
```
```rust
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
```

### Tip 10: Use Faster Linkers

**mold** (fastest, Linux):
```bash
RUSTFLAGS="-C link-arg=-fuse-ld=mold" cargo build --release
```

**lld** (fast, cross-platform):
```bash
RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo build --release
```

> No tradeoffs—always use a faster linker if available.

---

## 4. Type Sizes

### Tip 11: Measure Type Sizes

```bash
RUSTFLAGS=-Zprint-type-sizes cargo +nightly build --release
```

Or use `std::mem::size_of::<T>()` at runtime.

**Rule of thumb**: Types larger than 128 bytes use `memcpy`, which can show up in profiles.

### Tip 12: Box Large Enum Variants

```rust
// BEFORE: Enum is 104 bytes (size of largest variant)
enum Message {
    Small(u8),
    Large([u8; 100]),
}

// AFTER: Enum is ~16 bytes
enum Message {
    Small(u8),
    Large(Box<[u8; 100]>),  // Box the rare, large variant
}
```

### Tip 13: Use Smaller Integer Types

```rust
// If you know indices fit in u32, don't use usize everywhere
struct Index(u32);  // Instead of usize

// Coerce to usize at use points
let idx: usize = self.0 as usize;
```

### Tip 14: Convert Vec to Boxed Slice When Size is Fixed

```rust
// Vec: 3 words (ptr, len, capacity)
let v: Vec<u32> = vec![1, 2, 3];

// Box<[T]>: 2 words (ptr, len) - saves 8 bytes on 64-bit
let bs: Box<[u32]> = v.into_boxed_slice();

// Or collect directly into boxed slice
let bs: Box<[u32]> = (1..=3).collect();
```

### Tip 15: Use Static Assertions to Prevent Size Regressions

```rust
#[cfg(target_arch = "x86_64")]
static_assertions::assert_eq_size!(HotType, [u8; 64]);
```

---

## 5. Heap Allocations

### Tip 16: Use DHAT to Find Hot Allocations

Reducing allocation rates by ~10 allocations per million instructions can yield measurable improvements (~1%).

### Tip 17: Pre-allocate Collections

```rust
// BAD: Multiple reallocations
let mut v = Vec::new();
for i in 0..1000 {
    v.push(i);
}

// GOOD: Single allocation
let mut v = Vec::with_capacity(1000);
for i in 0..1000 {
    v.push(i);
}
```

### Tip 18: Use SmallVec for Short Vectors

```rust
use smallvec::{smallvec, SmallVec};

// Stores up to 4 elements inline, heap-allocates if more
let mut v: SmallVec<[u32; 4]> = smallvec![1, 2, 3];
v.push(4);  // Still inline
v.push(5);  // Now heap-allocated
```

### Tip 19: Use `clone_from` to Reuse Allocations

```rust
let mut v1: Vec<u32> = Vec::with_capacity(100);
let v2: Vec<u32> = vec![1, 2, 3];

// BAD: Allocates new storage
v1 = v2.clone();

// GOOD: Reuses v1's existing allocation
v1.clone_from(&v2);
assert_eq!(v1.capacity(), 100);  // Capacity preserved!
```

### Tip 20: Use Cow for Mixed Borrowed/Owned Data

```rust
use std::borrow::Cow;

// Avoids allocation for static strings
let mut messages: Vec<Cow<'static, str>> = vec![];
messages.push(Cow::Borrowed("static message"));           // No alloc
messages.push(Cow::Owned(format!("line {}", 42)));        // Alloc only when needed
messages.push("another static".into());                    // Convenient syntax
```

### Tip 21: Reuse Collections in Loops

```rust
// BAD: Allocates new Vec each iteration
for item in items {
    let results: Vec<_> = process(item).collect();
    handle(results);
}

// GOOD: Reuse the Vec
let mut results = Vec::new();
for item in items {
    results.extend(process(item));
    handle(&results);
    results.clear();  // Keeps capacity, removes elements
}
```

---

## 6. Standard Library Types

### Tip 22: Use swap_remove for O(1) Vec Removal

```rust
let mut v = vec![1, 2, 3, 4, 5];

// BAD: O(n) - shifts all subsequent elements
v.remove(1);

// GOOD: O(1) - swaps with last element (if order doesn't matter)
v.swap_remove(1);
```

### Tip 23: Prefer Lazy Evaluation Methods

```rust
// BAD: expensive() always evaluated
let result = option.ok_or(expensive());
let result = option.unwrap_or(expensive());

// GOOD: expensive() only evaluated if needed
let result = option.ok_or_else(|| expensive());
let result = option.unwrap_or_else(|| expensive());
```

Same applies to: `map_or` → `map_or_else`, `Result::or` → `or_else`

### Tip 24: Use Rc/Arc::make_mut for Clone-on-Write

```rust
use std::rc::Rc;

let mut data = Rc::new(vec![1, 2, 3]);

// Clone-on-write: clones only if refcount > 1
Rc::make_mut(&mut data).push(4);
```

---

## 7. Hashing

### Tip 25: Use Faster Hash Functions

| Crate | Speed | Quality | Best For |
|-------|-------|---------|----------|
| **rustc-hash** (FxHash) | Fastest | Low | Integer keys, internal use |
| **fnv** | Fast | Medium | Small keys |
| **ahash** | Fast | High | General purpose, AES hardware |

```rust
use rustc_hash::{FxHashMap, FxHashSet};

// Drop-in replacement for HashMap/HashSet
let mut map: FxHashMap<u32, String> = FxHashMap::default();
```

> The switch from default hasher to FxHash can give 4-84% speedups!

### Tip 26: Use Clippy to Enforce Hash Type Consistency

```toml
# clippy.toml
disallowed-types = ["std::collections::HashMap", "std::collections::HashSet"]
```

### Tip 27: Use nohash-hasher for Pre-hashed Keys

```rust
use nohash_hasher::IntMap;

// For keys that are already well-distributed (like random IDs)
let mut map: IntMap<u64, String> = IntMap::default();
```

---

## 8. Iterators

### Tip 28: Avoid Unnecessary collect()

```rust
// BAD: Collects into Vec, then iterates again
fn get_items() -> Vec<Item> {
    source.iter().filter(|x| x.valid).collect()
}
for item in get_items() { ... }

// GOOD: Return iterator, no intermediate allocation
fn get_items() -> impl Iterator<Item = &Item> {
    source.iter().filter(|x| x.valid)
}
```

### Tip 29: Implement size_hint for Custom Iterators

```rust
impl Iterator for MyIterator {
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.min_remaining, Some(self.max_remaining))
    }
}
```

This helps `collect()` pre-allocate the right capacity.

### Tip 30: Use chunks_exact for Better Codegen

```rust
// SLOWER: Handles remainder in every iteration
for chunk in data.chunks(4) { ... }

// FASTER: Compiler can optimize better
for chunk in data.chunks_exact(4) { ... }
// Handle remainder separately if needed
let remainder = data.chunks_exact(4).remainder();
```

### Tip 31: Use iter().copied() for Small Types

```rust
let numbers: Vec<u32> = vec![1, 2, 3, 4, 5];

// May generate suboptimal code (iterating references)
let sum: u32 = numbers.iter().sum();

// Often generates better code (iterating values)
let sum: u32 = numbers.iter().copied().sum();
```

---

## 9. Inlining

### Tip 32: Use Inline Attributes Strategically

```rust
// Small, hot functions - always inline
#[inline(always)]
fn hot_small_fn(x: u32) -> u32 {
    x + 1
}

// Large functions - never inline to reduce code size
#[inline(never)]
fn cold_large_fn() { ... }

// Suggestion to inline (compiler decides)
#[inline]
fn maybe_inline_fn() { ... }
```

### Tip 33: Split Hot/Cold Paths

```rust
// For functions with one hot call site and many cold ones:

#[inline(always)]
fn inlined_version() {
    // actual implementation
}

#[inline(never)]
fn uninlined_version() {
    inlined_version()  // calls the inlined version
}

// Use inlined_version() at hot call sites
// Use uninlined_version() at cold call sites
```

### Tip 34: Use #[cold] for Error Paths

```rust
#[cold]
fn handle_error(e: Error) -> ! {
    panic!("Fatal error: {}", e);
}

fn process(x: i32) -> Result<i32, Error> {
    if x < 0 {
        handle_error(Error::Negative);  // Compiler optimizes hot path
    }
    Ok(x * 2)
}
```

---

## 10. I/O

### Tip 35: Lock stdout/stderr for Repeated Writes

```rust
use std::io::Write;

// BAD: Locks/unlocks on every println!
for line in lines {
    println!("{}", line);
}

// GOOD: Lock once
let stdout = std::io::stdout();
let mut lock = stdout.lock();
for line in lines {
    writeln!(lock, "{}", line)?;
}
```

### Tip 36: Buffer File I/O

```rust
use std::io::{BufReader, BufWriter, Write};

// WRITING - BAD: System call per write
let mut out = File::create("out.txt")?;
for line in lines {
    writeln!(out, "{}", line)?;
}

// WRITING - GOOD: Buffered
let mut out = BufWriter::new(File::create("out.txt")?);
for line in lines {
    writeln!(out, "{}", line)?;
}
out.flush()?;

// READING - Use BufReader
let reader = BufReader::new(File::open("in.txt")?);
for line in reader.lines() {
    process(&line?);
}
```

### Tip 37: Reuse String Buffer When Reading Lines

```rust
use std::io::BufRead;

// BAD: Allocates new String per line
for line in reader.lines() {
    process(&line?);
}

// GOOD: Reuses buffer
let mut line = String::new();
while reader.read_line(&mut line)? != 0 {
    process(&line);
    line.clear();
}
```

---

## 11. Bounds Checks

### Tip 38: Help the Compiler Eliminate Bounds Checks

```rust
// APPROACH 1: Use iterators instead of indexing
for item in &vec {
    process(item);
}

// APPROACH 2: Take a slice before the loop
let slice = &vec[start..end];
for i in 0..slice.len() {
    process(slice[i]);  // Compiler knows bounds
}

// APPROACH 3: Add assertions
assert!(index < vec.len());
process(vec[index]);  // Bounds check eliminated
```

### Tip 39: Use get_unchecked Only as Last Resort

```rust
// SAFE: Compiler may still check bounds
let val = slice[i];

// UNSAFE: No bounds check - use only when provably safe
let val = unsafe { *slice.get_unchecked(i) };
```

---

## Quick Reference: Cargo.toml for Maximum Performance

```toml
[profile.release]
codegen-units = 1      # Better optimization
lto = "fat"            # Link-time optimization
panic = "abort"        # No unwinding overhead
strip = "symbols"      # Smaller binary

[profile.release.package."*"]
opt-level = 3          # Max optimization for dependencies
```

## Quick Reference: RUSTFLAGS for Performance

```bash
# All optimizations combined
RUSTFLAGS="-C target-cpu=native -C force-frame-pointers=yes" cargo build --release
```

---

## Further Reading

- [Rust Container Cheat Sheet](https://docs.google.com/presentation/d/1q-c7UAyrUlM-eZyTo1pd8SZ0qwA_wYxmPZVOQkoDmH4/)
- [Bounds Check Cookbook](https://github.com/Shnatsel/bounds-check-cookbook/)
- [min-sized-rust](https://github.com/johnthagen/min-sized-rust) - Binary size optimization
- [cargo-pgo](https://github.com/Kobzol/cargo-pgo) - Profile-guided optimization
- [cargo-wizard](https://github.com/Kobzol/cargo-wizard) - Interactive build configuration helper
