# High-Performance Rust — Optimization Guide for Agentic Workflows

> **This document provides actionable techniques for writing high-performance Rust code.**
> Use these patterns when optimizing code for speed, memory efficiency, and throughput.

## TL;DR — Quick Wins

```rust
// 1. Pre-allocate collections
let mut vec = Vec::with_capacity(expected_size);

// 2. Use iterators, avoid collect() where possible
items.iter().filter(|x| x.valid).for_each(|x| process(x));

// 3. Prefer &str and &[T] over &String and &Vec<T>
fn process(data: &str) { }  // Accepts both &str and &String

// 4. Use swap_remove for O(1) removal when order doesn't matter
vec.swap_remove(index);

// 5. Clone from instead of clone + assign
existing.clone_from(&source);  // Reuses allocation

// 6. Use unwrap_or_else for expensive defaults
option.unwrap_or_else(|| expensive_default());
```

---

## Philosophy: When to Optimize

### The Golden Rules

1. **Don't optimize blindly** — Profile first with tools like `perf`, `samply`, or `cargo-flamegraph`
2. **Algorithms first** — O(n) vs O(n²) matters more than micro-optimizations
3. **Measure, don't guess** — Use benchmarks to validate improvements
4. **Hot paths only** — 80% of time is spent in 20% of code; find and optimize that 20%
5. **Readability matters** — Only sacrifice clarity when measurements justify it

### Quick Profiling Commands

```bash
# CPU profiling with perf (Linux)
perf record --call-graph dwarf cargo run --release
perf report

# Flamegraph generation
cargo flamegraph

# Memory profiling with DHAT
cargo run --features dhat-heap

# Instruction-level profiling (deterministic)
valgrind --tool=cachegrind target/release/myapp
```

---

## Build Configuration for Performance

### Release Profile (Cargo.toml)

```toml
[profile.release]
opt-level = 3              # Maximum optimization
lto = "fat"                # Link-time optimization (slower build, faster binary)
codegen-units = 1          # Better optimization at cost of parallelism
panic = "abort"            # Smaller binary, no unwinding overhead
strip = "symbols"          # Remove symbols for smaller binary

[profile.release-with-debug]
inherits = "release"
debug = "line-tables-only" # Debug info for profiling
strip = false
```

### Target-Specific Optimizations

```bash
# Enable CPU-specific instructions
RUSTFLAGS="-C target-cpu=native" cargo build --release

# Check available features
rustc --print target-cpus
rustc --print target-features
```

### Faster Linker (Huge Build Speed Improvement)

```toml
# .cargo/config.toml
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]  # or lld

[target.x86_64-apple-darwin]
rustflags = ["-C", "link-arg=-fuse-ld=/opt/homebrew/bin/ld64.lld"]
```

---

## Memory Allocation Optimization

### Pre-allocate Collections

```rust
// ❌ Slow: Multiple reallocations
let mut vec = Vec::new();
for i in 0..1000 {
    vec.push(i);  // May reallocate multiple times
}

// ✅ Fast: Single allocation
let mut vec = Vec::with_capacity(1000);
for i in 0..1000 {
    vec.push(i);
}

// ✅ Even better: Use collect with size hint
let vec: Vec<_> = (0..1000).collect();  // Iterator provides size_hint
```

### Reuse Allocations

```rust
// ❌ Allocates new buffer each iteration
for line in reader.lines() {
    let line = line?;
    process(&line);
}

// ✅ Reuses buffer
let mut buf = String::new();
while reader.read_line(&mut buf)? > 0 {
    process(&buf);
    buf.clear();
}

// ✅ For Vec, use clone_from to reuse capacity
let mut result = Vec::new();
for item in items {
    result.clone_from(&item.data);  // Reuses allocation if capacity sufficient
    process(&result);
}
```

### SmallVec for Short Vectors

```rust
use smallvec::SmallVec;

// Stack-allocated for ≤8 elements, heap-allocated otherwise
let mut vec: SmallVec<[i32; 8]> = SmallVec::new();
vec.push(1);  // No heap allocation

// Great for: function arguments, temporary buffers, graph adjacency lists
```

### Box<[T]> for Fixed-Size Data

```rust
// ❌ Vec has 24 bytes overhead (ptr + len + capacity)
let data: Vec<u8> = compute_fixed_data();

// ✅ Box<[T]> has only 16 bytes overhead (ptr + len)
let data: Box<[u8]> = compute_fixed_data().into_boxed_slice();
```

### Alternative Allocators

```rust
// In Cargo.toml
// [dependencies]
// tikv-jemallocator = "0.5"  # or mimalloc

#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
```

---

## Collection Performance

### Choose the Right Collection

| Need | Collection | Time Complexity |
|------|------------|-----------------|
| Stack (LIFO) | `Vec` | O(1) push/pop |
| Queue (FIFO) | `VecDeque` | O(1) front/back |
| Unique set | `HashSet` / `BTreeSet` | O(1) / O(log n) |
| Key-value | `HashMap` / `BTreeMap` | O(1) / O(log n) |
| Sorted | `BTreeMap` / `BTreeSet` | O(log n) |
| Priority queue | `BinaryHeap` | O(log n) insert/pop |

### Faster Hashing

```rust
// Default hasher is DoS-resistant but slower
use std::collections::HashMap;
let map: HashMap<String, i32> = HashMap::new();

// ❌ For non-cryptographic use, default is overkill

// ✅ FxHash: 4-84% faster for internal data structures
use rustc_hash::FxHashMap;
let map: FxHashMap<String, i32> = FxHashMap::default();

// ✅ AHash: Fast and DoS-resistant (good default replacement)
use ahash::AHashMap;
let map: AHashMap<String, i32> = AHashMap::new();
```

### Efficient Removal

```rust
// ❌ O(n): Shifts all elements after index
vec.remove(index);

// ✅ O(1): When order doesn't matter
vec.swap_remove(index);

// ❌ O(n): Retains all, then removes
vec.retain(|x| x.valid);  // Fine if removing many elements

// ✅ O(1) amortized: Remove single element by swap
if let Some(pos) = vec.iter().position(|x| !x.valid) {
    vec.swap_remove(pos);
}
```

### Entry API for Single-Lookup Operations

```rust
// ❌ Two lookups
if !map.contains_key(&key) {
    map.insert(key, compute_value());
}
let value = map.get(&key).unwrap();

// ✅ Single lookup
let value = map.entry(key).or_insert_with(|| compute_value());
```

---

## Iterator Optimization

### Avoid Intermediate Collections

```rust
// ❌ Creates intermediate Vec
let filtered: Vec<_> = items.iter().filter(|x| x.valid).collect();
let result: i32 = filtered.iter().map(|x| x.value).sum();

// ✅ Direct iterator chain
let result: i32 = items.iter()
    .filter(|x| x.valid)
    .map(|x| x.value)
    .sum();
```

### Use chunks_exact Over chunks

```rust
// ❌ chunks() must handle remainder, prevents optimizations
for chunk in data.chunks(4) {
    process(chunk);  // chunk.len() may be < 4
}

// ✅ chunks_exact() guarantees size, enables better codegen
for chunk in data.chunks_exact(4) {
    process(chunk);  // chunk.len() is always 4
}
let remainder = data.chunks_exact(4).remainder();
```

### iter().copied() for Small Types

```rust
// ❌ Returns references, may prevent optimizations
for &x in slice.iter() {
    use_value(x);
}

// ✅ Explicit copy, clearer intent
for x in slice.iter().copied() {
    use_value(x);
}

// Works for Copy types: integers, floats, bools, etc.
```

### Implement size_hint for Custom Iterators

```rust
impl Iterator for MyIterator {
    type Item = Item;

    fn next(&mut self) -> Option<Self::Item> { /* ... */ }

    // ✅ Enables pre-allocation in collect()
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

// Even better: implement ExactSizeIterator if size is always known
impl ExactSizeIterator for MyIterator {}
```

---

## String Performance

### Accept &str Not &String

```rust
// ❌ Requires &String, forces allocation for string literals
fn process(s: &String) { }
process(&"hello".to_string());  // Unnecessary allocation

// ✅ Accepts both &str and &String via Deref
fn process(s: &str) { }
process("hello");              // No allocation
process(&my_string);           // Also works
```

### Efficient String Building

```rust
// ❌ Multiple allocations and copies
let result = a.to_string() + &b + &c;

// ✅ Single allocation with capacity
let mut result = String::with_capacity(a.len() + b.len() + c.len());
result.push_str(a);
result.push_str(&b);
result.push_str(&c);

// ✅ Or use format! for readability (optimized by compiler)
let result = format!("{}{}{}", a, b, c);
```

### Cow for Conditional Ownership

```rust
use std::borrow::Cow;

// Returns borrowed if no modification needed, owned if modified
fn normalize(s: &str) -> Cow<'_, str> {
    if s.contains('\t') {
        Cow::Owned(s.replace('\t', "    "))  // Allocates
    } else {
        Cow::Borrowed(s)  // No allocation
    }
}
```

---

## Lazy Evaluation

### Use _or_else Variants

```rust
// ❌ Always evaluates default, even if not needed
let value = option.unwrap_or(expensive_default());
let value = result.unwrap_or(compute_fallback());

// ✅ Only evaluates if needed
let value = option.unwrap_or_else(|| expensive_default());
let value = result.unwrap_or_else(|_| compute_fallback());

// Same pattern for ok_or vs ok_or_else
option.ok_or_else(|| Error::new())?;
```

### Lazy Statics

```rust
use std::sync::LazyLock;

// ✅ Initialized on first access
static REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\d+").unwrap()
});
```

---

## Inlining Strategy

### When to Use #[inline]

```rust
// ✅ Small, frequently called functions
#[inline]
fn add(a: i32, b: i32) -> i32 { a + b }

// ✅ Functions often called with constants (enables const-folding)
#[inline]
fn get_by_index(slice: &[u8], index: usize) -> Option<u8> {
    slice.get(index).copied()
}

// ✅ Cross-crate hot paths (otherwise can't be inlined)
#[inline]
pub fn hot_path(&self) -> bool { self.flag }
```

### When NOT to Use #[inline(always)]

```rust
// ❌ Don't use inline(always) liberally — trust the compiler
#[inline(always)]  // May bloat code size, hurt instruction cache
fn large_function() { /* many lines */ }

// ✅ Use inline(always) only with benchmarks proving benefit
// ✅ Use #[cold] for error paths to prevent inlining
#[cold]
fn handle_error(e: Error) -> ! {
    panic!("Fatal: {e}");
}
```

### Split Hot/Cold Paths

```rust
// ❌ Error handling code bloats hot path
fn process(data: &[u8]) -> Result<Output, Error> {
    if data.is_empty() {
        return Err(Error::Empty);  // Cold path inlined with hot path
    }
    // Hot path
}

// ✅ Factor out cold path
fn process(data: &[u8]) -> Result<Output, Error> {
    if data.is_empty() {
        return handle_empty();  // Cold, won't be inlined
    }
    // Hot path
}

#[cold]
#[inline(never)]
fn handle_empty() -> Result<Output, Error> {
    Err(Error::Empty)
}
```

---

## Bounds Check Elimination

### Help the Compiler Elide Checks

```rust
// ❌ Bounds check on every access
fn sum_first_four(slice: &[i32]) -> i32 {
    slice[0] + slice[1] + slice[2] + slice[3]  // 4 bounds checks
}

// ✅ Single bounds check, rest elided
fn sum_first_four(slice: &[i32]) -> i32 {
    assert!(slice.len() >= 4);  // Compiler uses this info
    slice[0] + slice[1] + slice[2] + slice[3]  // No bounds checks
}

// ✅ Even better: use slicing
fn sum_first_four(slice: &[i32]) -> i32 {
    let chunk = &slice[..4];  // Single bounds check
    chunk[0] + chunk[1] + chunk[2] + chunk[3]
}

// ✅ Best: use iterators
fn sum_first_four(slice: &[i32]) -> i32 {
    slice.iter().take(4).sum()  // No bounds checks needed
}
```

### get_unchecked — Last Resort

```rust
// Only when: 1) profiling shows bounds checks matter, 2) invariants proven
unsafe {
    // SAFETY: index is always < len due to [explain invariant]
    *slice.get_unchecked(index)
}
```

---

## Data Layout Optimization

### Struct Field Ordering

Rust reorders struct fields by default for better packing, but consider:

```rust
// Check type sizes
#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn type_sizes_are_optimal() {
        assert!(size_of::<MyStruct>() <= 64);  // Fits in cache line
    }
}

// Use -Zprint-type-sizes to analyze (nightly)
// RUSTFLAGS="-Zprint-type-sizes" cargo +nightly build --release
```

### Box Large Enum Variants

```rust
// ❌ Entire enum is size of largest variant (256+ bytes)
enum Message {
    Ping,
    Data([u8; 256]),  // Forces all Messages to be 256+ bytes
}

// ✅ Box the large variant (8 bytes + heap allocation)
enum Message {
    Ping,
    Data(Box<[u8; 256]>),
}
```

### Use Smaller Integer Types

```rust
// ❌ Using usize everywhere wastes memory
struct Record {
    count: usize,   // 8 bytes, but value is always < 256
    flags: usize,   // 8 bytes, but only 8 flags
}

// ✅ Right-size your integers
struct Record {
    count: u8,      // 1 byte
    flags: u8,      // 1 byte
}
```

---

## I/O Optimization

### Buffered I/O

```rust
use std::io::{BufReader, BufWriter};

// ❌ Unbuffered: many small syscalls
let file = File::open(path)?;
// Direct reads are slow

// ✅ Buffered: batched syscalls
let file = File::open(path)?;
let reader = BufReader::new(file);

// Same for writing
let file = File::create(path)?;
let writer = BufWriter::new(file);
```

### Lock stdout for Repeated Writes

```rust
use std::io::{self, Write};

// ❌ Locks stdout for each write
for line in lines {
    println!("{}", line);  // Acquires lock, writes, releases
}

// ✅ Lock once, write many
let stdout = io::stdout();
let mut handle = stdout.lock();
for line in lines {
    writeln!(handle, "{}", line)?;
}
```

---

## Parallel Processing

### Rayon for Data Parallelism

```rust
use rayon::prelude::*;

// ❌ Sequential
let results: Vec<_> = items.iter().map(|x| process(x)).collect();

// ✅ Parallel with minimal code change
let results: Vec<_> = items.par_iter().map(|x| process(x)).collect();

// ✅ Parallel sorting
let mut data = vec![3, 1, 4, 1, 5, 9];
data.par_sort();
```

### When to Parallelize

- **Do**: CPU-bound work on large datasets
- **Don't**: I/O-bound work (use async instead)
- **Don't**: Small datasets (overhead exceeds benefit)
- **Measure**: Parallel isn't always faster

---

## Quick Reference — Common Optimizations

| Pattern | Before | After | Impact |
|---------|--------|-------|--------|
| Pre-allocate | `Vec::new()` | `Vec::with_capacity(n)` | Fewer allocations |
| Faster hash | `HashMap` | `FxHashMap` | 4-84% faster |
| O(1) remove | `vec.remove(i)` | `vec.swap_remove(i)` | O(n) → O(1) |
| Reuse buffer | `String::new()` in loop | `buf.clear()` | Fewer allocations |
| Lazy default | `unwrap_or(x)` | `unwrap_or_else(|| x)` | Skip computation |
| Accept ref | `fn f(s: &String)` | `fn f(s: &str)` | More flexible |
| Iterator | `collect()` + `iter()` | Direct chain | No intermediate Vec |
| Bounds | `slice[0] + slice[1]` | `&slice[..2]` then index | Fewer checks |

---

## Profiling and Measurement

### Essential Tools

| Tool | Use Case | Command |
|------|----------|---------|
| `perf` | CPU profiling (Linux) | `perf record cargo run --release` |
| `samply` | CPU profiling (cross-platform) | `samply record cargo run --release` |
| `cargo-flamegraph` | Flamegraph visualization | `cargo flamegraph` |
| `DHAT` | Heap profiling | Use `dhat` crate |
| `Cachegrind` | Cache analysis | `valgrind --tool=cachegrind` |
| `cargo-show-asm` | View generated assembly | `cargo asm module::function` |
| `hyperfine` | CLI benchmarking | `hyperfine 'target/release/app'` |
| `criterion` | Rust benchmarking | Add to dev-dependencies |

### Benchmark Template

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_function(c: &mut Criterion) {
    c.bench_function("my_function", |b| {
        b.iter(|| my_function(black_box(input)))
    });
}

criterion_group!(benches, bench_function);
criterion_main!(benches);
```

---

## Further Reading

- [The Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Achieving Warp Speed with Rust (jFransham)](https://gist.github.com/jFransham/369a86eff00e5f280ed25121454acec1)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Rust Design Patterns](https://rust-unofficial.github.io/patterns/)

---

*These patterns should be applied thoughtfully based on profiling data. Premature optimization is the root of all evil — measure first, optimize second.*
