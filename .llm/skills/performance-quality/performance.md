<!-- CATEGORY: Performance & Quality -->
<!-- WHEN: Optimizing performance, profiling, build configuration, compile times -->

# Rust Performance & Build Optimization

---

## Profiling Tools

| Tool | Use Case | Command |
|------|----------|---------|
| `perf` | CPU profiling (Linux) | `perf record --call-graph dwarf cargo run --release` |
| `samply` | CPU profiling (cross-platform) | `samply record cargo run --release` |
| `cargo-flamegraph` | Flamegraph visualization | `cargo flamegraph` |
| DHAT | Heap profiling | Use `dhat` crate |
| Cachegrind | Cache analysis | `valgrind --tool=cachegrind` |
| `cargo-show-asm` | View assembly | `cargo asm module::function` |
| `hyperfine` | CLI benchmarking | `hyperfine 'target/release/app'` |
| `criterion` / `divan` | Rust microbenchmarks | Add to dev-dependencies |

```bash
# Enable debug info for profiling release builds
# Cargo.toml: [profile.release] debug = "line-tables-only"
RUSTFLAGS="-C force-frame-pointers=yes" cargo build --release
```

---

## Build Configuration

### Release Profile (Maximum Runtime Speed)

```toml
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
panic = "abort"
strip = "symbols"

[profile.release.build-override]
opt-level = 3          # Optimize build scripts/proc-macros
codegen-units = 256

[profile.dev.package."*"]
opt-level = 2          # Optimize deps in dev builds
```

### Profile Decision Matrix

| Goal | opt-level | lto | codegen-units | Trade-off |
|------|-----------|-----|---------------|-----------|
| Fastest compile | 0 | off | 256 | Slow runtime |
| Balanced | 2 | thin | 16 | Good middle ground |
| Fast runtime | 3 | thin | 1 | Slower compile |
| Maximum runtime | 3 | fat | 1 | Slowest compile |
| Smallest binary | z | fat | 1 | Slower runtime |

### Faster Linker

```toml
# .cargo/config.toml
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]

[target.x86_64-apple-darwin]
rustflags = ["-C", "link-arg=-fuse-ld=/opt/homebrew/bin/ld64.lld"]
```

Impact: 2-10x faster linking.

**Important:** When `.cargo/config.toml` specifies a custom linker (lld/mold), all cargo
commands that invoke the linker will fail if the linker is not installed. Pre-commit hook
scripts that run cargo must detect linker availability and fall back gracefully. Use
`scripts/build/cargo_linker.py:get_cargo_env()` in any Python script that runs cargo commands.
The pattern: `env = os.environ.copy(); env.update(get_cargo_env())`. For CI, override
via env vars: `CARGO_TARGET_<TRIPLE>_LINKER=cc` and `CARGO_TARGET_<TRIPLE>_RUSTFLAGS=""`.

### CPU-Specific Instructions

```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

---

## Compile Time Optimization

### Top Techniques

| Technique | Impact | Notes |
|-----------|--------|-------|
| Faster linker (mold/lld) | 2-10x linking | Always use if available |
| `cargo check` | 2-3x faster | For type-checking only |
| Disable debug info | 20-40% | `debug = false` or `"line-tables-only"` |
| Optimize build scripts | Variable | `[profile.dev.build-override] opt-level = 3` |
| Split into workspace crates | Variable | Enables parallel compilation |
| Remove unused deps | Variable | `cargo machete` or `cargo +nightly udeps` |
| Disable unused features | Variable | `default-features = false` |
| macOS: `split-debuginfo = "unpacked"` | Up to 70% | Faster incremental |
| `sccache` | Variable | Shared cache across projects |
| Cranelift backend (nightly) | 20-50% | Reduced runtime performance |

### Diagnosing Slow Builds

```bash
cargo build --timings          # Opens timing visualization
cargo +nightly rustc -- -Zself-profile  # Profile compilation
cargo llvm-lines | head -30    # Find expensive monomorphizations
CARGO_LOG="cargo::core::compiler::fingerprint=info" cargo build 2>&1 | grep dirty
```

### Minimize Generic Bloat

```rust
// Thin generic wrapper + non-generic inner function
pub fn process<T: AsRef<Path>>(path: T) {
    fn inner(path: &Path) { /* compiled once */ }
    inner(path.as_ref())
}
```

### CI Configuration

```toml
[profile.ci]
inherits = "dev"
debug = false
incremental = false

[profile.ci.package."*"]
opt-level = 1
```

```yaml
env:
  CARGO_INCREMENTAL: 0  # Not useful in CI
```

---

## Memory Allocation

### Pre-allocate Collections

```rust
let mut vec = Vec::with_capacity(expected_size);
```

### Reuse Allocations

```rust
let mut buf = String::new();
while reader.read_line(&mut buf)? > 0 {
    process(&buf);
    buf.clear();  // Keeps capacity
}

// clone_from reuses existing allocation
existing.clone_from(&source);
```

### SmallVec for Short Vectors

```rust
use smallvec::SmallVec;
let mut vec: SmallVec<[i32; 8]> = SmallVec::new(); // Stack for <=8 elements
```

### Alternative Allocators

```rust
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
```

---

## Collection Performance

| Pattern | Before | After | Impact |
|---------|--------|-------|--------|
| Pre-allocate | `Vec::new()` | `Vec::with_capacity(n)` | Fewer allocations |
| Faster hash | `HashMap` | `FxHashMap` | 4-84% faster |
| O(1) remove | `vec.remove(i)` | `vec.swap_remove(i)` | O(n) to O(1) |
| Single lookup | `contains_key` + `insert` | `entry().or_insert_with()` | Half the lookups |
| Reuse buffer | New `String` in loop | `buf.clear()` | Fewer allocations |

---

## Iterator Optimization

```rust
// Avoid intermediate collections
let result: i32 = items.iter().filter(|x| x.valid).map(|x| x.value).sum();

// chunks_exact enables better codegen than chunks
for chunk in data.chunks_exact(4) { process(chunk); }

// iter().copied() for small types
let sum: u32 = numbers.iter().copied().sum();
```

Implement `size_hint()` for custom iterators to enable `collect()` pre-allocation.

---

## Data Layout

```rust
// Box large enum variants
enum Message {
    Ping,
    Data(Box<[u8; 256]>),  // Not Data([u8; 256])
}

// Use right-sized integers
struct Record { count: u8, flags: u8 }  // Not usize

// Box<[T]> saves 8 bytes vs Vec<T> for fixed-size data
let data: Box<[u8]> = vec.into_boxed_slice();
```

Check type sizes: `RUSTFLAGS="-Zprint-type-sizes" cargo +nightly build --release`

---

## Inlining Strategy

```rust
#[inline]     // Small, frequently called, cross-crate hot paths
#[cold]       // Error paths -- prevents inlining, improves hot path
#[inline(never)]  // Large cold functions
```

Split hot/cold paths: factor error handling into `#[cold] #[inline(never)]` functions.

---

## I/O Performance

```rust
// Buffer file I/O
let reader = BufReader::new(File::open(path)?);
let writer = BufWriter::new(File::create(path)?);

// Lock stdout for repeated writes
let mut handle = io::stdout().lock();
for line in lines { writeln!(handle, "{}", line)?; }
```

---

## Lazy Evaluation

```rust
// Only evaluate default if needed
let value = option.unwrap_or_else(|| expensive_default());
let value = option.ok_or_else(|| Error::new())?;
```

---

## Benchmark Template

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench(c: &mut Criterion) {
    c.bench_function("my_fn", |b| b.iter(|| my_fn(black_box(input))));
}

criterion_group!(benches, bench);
criterion_main!(benches);
```

---

## Workspace Layout for Fast Builds

```
project/
├── Cargo.toml          # Workspace root
├── .cargo/config.toml  # Linker config
├── crates/
│   ├── core/           # No heavy deps
│   ├── protocol/       # Domain logic
│   └── cli/            # Binary, brings it together
└── tests/
    └── integration/    # Single integration test crate
```

Prefer wide dependency graphs (parallel) over linear chains (sequential).
