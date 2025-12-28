# Rust Compile Time Optimization Guide

A comprehensive guide to speeding up Rust compilation and configuring optimal build settings for different use cases.

## Table of Contents

1. [Cargo Profile Configurations](#cargo-profile-configurations)
2. [20 Techniques to Speed Up Compilation](#20-techniques-to-speed-up-compilation)
3. [Trade-offs: Compile Time vs Runtime Performance](#trade-offs-compile-time-vs-runtime-performance)
4. [Profiling and Diagnosing Slow Builds](#profiling-and-diagnosing-slow-builds)
5. [Project Structure Patterns](#project-structure-patterns)
6. [CI/CD Optimization](#cicd-optimization)

---

## Cargo Profile Configurations

### Complete Profile Reference

```toml
# Cargo.toml

# =============================================================================
# Development Profile - Optimized for fast compilation
# =============================================================================
[profile.dev]
opt-level = 0          # No optimizations (fastest compile)
debug = true           # Full debug info (use "line-tables-only" for faster builds)
split-debuginfo = "unpacked"  # Faster incremental on macOS
debug-assertions = true
overflow-checks = true
lto = false            # No link-time optimization
panic = "unwind"
incremental = true     # Enable incremental compilation
codegen-units = 256    # Maximum parallelism

# =============================================================================
# Release Profile - Optimized for runtime performance
# =============================================================================
[profile.release]
opt-level = 3          # Full optimizations
debug = false          # No debug info
strip = "symbols"      # Strip symbols for smaller binary
debug-assertions = false
overflow-checks = false
lto = "thin"           # Good balance of speed vs compile time
panic = "abort"        # Smaller binary, slightly faster
incremental = false    # Better optimization without incremental
codegen-units = 1      # Best optimization (slower compile)

# =============================================================================
# Custom Profile: Fast Release (for development with performance testing)
# =============================================================================
[profile.release-fast]
inherits = "release"
lto = false            # Skip LTO for faster builds
codegen-units = 16     # More parallelism
incremental = true     # Enable incremental

# =============================================================================
# Custom Profile: Maximum Performance
# =============================================================================
[profile.release-max]
inherits = "release"
lto = "fat"            # Full cross-crate LTO
codegen-units = 1      # Single codegen unit for best optimization

# =============================================================================
# Build scripts and proc-macros: Optimize these for faster execution
# =============================================================================
[profile.dev.build-override]
opt-level = 3          # Optimize build scripts (they run during build)
codegen-units = 256

[profile.release.build-override]
opt-level = 3
codegen-units = 256

# =============================================================================
# Dependency Optimization (debug builds with optimized deps)
# =============================================================================
[profile.dev.package."*"]
opt-level = 2          # Optimize all dependencies in dev builds
```

### Profile Settings Explained

| Setting | Values | Effect |
|---------|--------|--------|
| `opt-level` | `0`, `1`, `2`, `3`, `"s"`, `"z"` | Optimization level (0=none, 3=max, s/z=size) |
| `debug` | `true`, `false`, `"line-tables-only"` | Debug info generation |
| `lto` | `false`, `"thin"`, `"fat"`, `"off"` | Link-time optimization |
| `codegen-units` | `1`-`256` | Parallelism (1=best opt, 256=fastest compile) |
| `incremental` | `true`, `false` | Incremental compilation |
| `panic` | `"unwind"`, `"abort"` | Panic strategy |
| `strip` | `"none"`, `"debuginfo"`, `"symbols"` | Symbol stripping |

---

## 20 Techniques to Speed Up Compilation

### 1. Use a Faster Linker

The linker is often the biggest bottleneck. Use `mold` (Linux) or `lld` (cross-platform).

```toml
# .cargo/config.toml

# Linux with mold (fastest)
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]

# Cross-platform with lld
[build]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]
```

**Impact**: 2-10x faster linking, especially for large projects.

### 2. Use `cargo check` Instead of `cargo build`

```bash
# For type-checking only (2-3x faster than build)
cargo check

# Auto-run on file changes
cargo watch -c -x check
```

**Impact**: 2-3x faster feedback loop during development.

### 3. Disable Debug Info in Development

```toml
[profile.dev]
debug = false  # or "line-tables-only" to keep backtraces
```

**Impact**: 20-40% faster dev builds, smaller target directory.

### 4. Enable Incremental Compilation (Default for Dev)

```toml
[profile.dev]
incremental = true
```

Disable for CI builds where cache isn't preserved between runs:

```yaml
# CI environment variable
env:
  CARGO_INCREMENTAL: 0
```

### 5. Optimize Build Scripts and Proc-Macros

```toml
[profile.dev.build-override]
opt-level = 3
```

**Impact**: Faster build script execution, especially with heavy proc-macros like `serde`.

### 6. Use the Cranelift Backend (Nightly)

```bash
# Install
rustup component add rustc-codegen-cranelift-preview --toolchain nightly

# Use
RUSTFLAGS="-Zcodegen-backend=cranelift" cargo +nightly build
```

**Impact**: 20-50% faster debug builds (reduced runtime performance).

### 7. Enable Parallel Frontend (Nightly)

```bash
RUSTFLAGS="-Zthreads=8" cargo +nightly build
```

Or in config:

```toml
# .cargo/config.toml
[build]
rustflags = ["-Z", "threads=8"]
```

**Impact**: Up to 50% faster compilation on multi-core systems.

### 8. Split into Workspace Crates

Structure your project to maximize parallel compilation:

```
# Good: Wide dependency graph (parallel)
     +-  crate_b  -+
    /               \
crate_a  ->  crate_c  ->  crate_e (binary)
    \               /
     +-  crate_d  -+

# Bad: Linear dependency chain (sequential)
crate_a -> crate_b -> crate_c -> crate_d -> crate_e
```

**Impact**: Better CPU utilization, incremental builds only recompile changed crates.

### 9. Remove Unused Dependencies

```bash
# Find unused dependencies
cargo install cargo-machete && cargo machete
cargo install cargo-udeps --locked && cargo +nightly udeps

# Find duplicate dependency versions
cargo tree --duplicate
```

**Impact**: Fewer crates to compile, smaller dependency graph.

### 10. Disable Unused Features

```toml
[dependencies]
tokio = { version = "1", default-features = false, features = ["rt", "net"] }
serde = { version = "1", default-features = false, features = ["derive"] }
```

**Impact**: Smaller dependency tree, faster compilation.

### 11. Use Feature Flags for Expensive Code

```toml
[features]
default = []
json = ["serde_json"]  # Only compile when needed
```

```rust
#[cfg(feature = "json")]
use serde_json;
```

### 12. Replace Heavy Dependencies with Lighter Alternatives

| Heavy | Light Alternative |
|-------|-------------------|
| `serde` | `miniserde`, `nanoserde` |
| `reqwest` | `ureq` |
| `clap` | `lexopt`, `argh` |
| `regex` | `regex-lite` |

### 13. Use `sccache` for Distributed/Shared Caching

```bash
cargo install sccache
export RUSTC_WRAPPER=sccache
```

**Impact**: Shared cache across projects/machines; best for CI.

### 14. macOS: Use `split-debuginfo = "unpacked"`

```toml
[profile.dev]
split-debuginfo = "unpacked"
```

**Impact**: Up to 70% faster incremental builds on macOS.

### 15. macOS: Exclude from Gatekeeper

```bash
sudo spctl developer-mode enable-terminal
# Then add Terminal to Developer Tools in System Preferences
```

### 16. Windows: Use Dev Drive

Move these to a Dev Drive volume:
- `CARGO_HOME` (~/.cargo)
- Project code
- Target directory

**Impact**: 20-30% faster builds on Windows 11.

### 17. Minimize Generic Function Instantiations

```rust
// Bad: Generic function compiled for every type
pub fn process<T: AsRef<Path>>(path: T) {
    // Large implementation
}

// Good: Thin generic wrapper + non-generic inner function
pub fn process<T: AsRef<Path>>(path: T) {
    fn inner(path: &Path) {
        // Large implementation (compiled once)
    }
    inner(path.as_ref())
}
```

### 18. Reduce Proc-Macro Usage in Core Crates

Keep `serde` derives in leaf crates, not shared/core crates:

```toml
# shared_types/Cargo.toml
[features]
serde = ["dep:serde"]

[dependencies]
serde = { version = "1", optional = true }
```

```rust
// shared_types/src/lib.rs
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MyType { /* ... */ }
```

### 19. Use `cargo-hakari` for Workspace Feature Unification

```bash
cargo install cargo-hakari
cargo hakari init
cargo hakari generate
```

**Impact**: Prevents rebuilding dependencies with different feature sets.

### 20. Configure rust-analyzer to Use Separate Target Directory

```json
// .vscode/settings.json
{
    "rust-analyzer.cargo.targetDir": true
}
```

**Impact**: Prevents rust-analyzer and cargo from invalidating each other's caches.

---

## Trade-offs: Compile Time vs Runtime Performance

### Decision Matrix

| Goal | `opt-level` | `lto` | `codegen-units` | `debug` | Trade-off |
|------|-------------|-------|-----------------|---------|-----------|
| **Fastest compile** | 0 | off | 256 | false | Slow runtime |
| **Fast compile + debug** | 0 | false | 256 | line-tables-only | No optimization |
| **Balanced** | 2 | thin | 16 | false | Good middle ground |
| **Fast runtime** | 3 | thin | 1 | false | Slower compile |
| **Maximum runtime** | 3 | fat | 1 | false | Slowest compile |
| **Smallest binary** | z | fat | 1 | false | Slower runtime |

### LTO Trade-offs

| LTO Setting | Compile Time | Runtime Speed | Binary Size |
|-------------|--------------|---------------|-------------|
| `"off"` | Fastest | Baseline | Largest |
| `false` (thin local) | Fast | Good | Medium |
| `"thin"` | Slower | Better | Smaller |
| `"fat"` | Slowest | Best | Smallest |

### Codegen Units Trade-offs

| Codegen Units | Compile Time | Optimization Quality |
|---------------|--------------|----------------------|
| 256 (max) | Fastest | Lowest (most parallel) |
| 16 (release default) | Balanced | Good |
| 1 | Slowest | Best (single unit) |

---

## Profiling and Diagnosing Slow Builds

### 1. Cargo Build Timings

```bash
cargo build --timings
# Opens cargo-timing.html with visualization
```

Look for:
- **Red bars**: Crates waiting for dependencies (bottleneck)
- **Long single bars**: Crates that take too long
- **Proc-macro crates early**: They block dependent crates

### 2. Self-Profile Compilation

```bash
# Profile a single crate
cargo +nightly rustc -- -Zself-profile

# Analyze with summarize
cargo install --git https://github.com/rust-lang/measureme summarize
summarize summarize <profile-data>

# View in Chrome
# Open chrome://tracing and load the .json file
```

### 3. Find Expensive Monomorphizations

```bash
cargo install cargo-llvm-lines
cargo llvm-lines | head -30
```

Output shows which generic functions generate the most LLVM IR.

### 4. Measure Linker Time

```bash
cargo clean
cargo +nightly rustc --bin <name> -- -Z time-passes 2>&1 | grep link
```

### 5. Find Proc-Macro Code Generation

```bash
# Nightly only
RUSTFLAGS="-Zmacro-stats" cargo +nightly build
```

### 6. Find Rebuild Causes

```bash
export CARGO_LOG="cargo::core::compiler::fingerprint=info"
cargo build 2>&1 | grep -i dirty
```

Look for:
- `EnvVarChanged`: Environment variable differences
- `Stale`: File changes
- Feature flag mismatches

### 7. Expand Macros to See Generated Code

```bash
cargo install cargo-expand
cargo expand <module_path>
```

---

## Project Structure Patterns

### Recommended Workspace Layout

```
my_project/
├── Cargo.toml              # Workspace root
├── .cargo/
│   └── config.toml         # Build configuration
├── crates/
│   ├── core/               # Shared types, no heavy deps
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── protocol/           # Domain logic
│   ├── network/            # I/O, async runtime
│   └── cli/                # Binary, brings it all together
├── tests/
│   └── integration/        # Single integration test crate
└── benches/
```

### Workspace Cargo.toml

```toml
[workspace]
resolver = "2"
members = [
    "crates/*",
]

# Shared settings
[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.75"

# Shared dependencies
[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }

# Profile settings (only in workspace root)
[profile.dev]
debug = "line-tables-only"

[profile.dev.build-override]
opt-level = 3

[profile.release]
lto = "thin"
codegen-units = 1
strip = "symbols"
```

### Dependency Organization Principles

1. **Core crates have minimal dependencies**: No proc-macros, no async runtimes
2. **Feature-gate optional functionality**: `serde` support behind a feature
3. **Keep proc-macro heavy crates at the edges**: CLI, API boundaries
4. **Avoid diamond dependencies with different features**: Use `cargo-hakari`
5. **Consolidate duplicate dependency versions**: `cargo tree --duplicate`

---

## CI/CD Optimization

### GitHub Actions Configuration

```yaml
name: CI

on: [push, pull_request]

env:
  CARGO_INCREMENTAL: 0          # Disable incremental (not useful in CI)
  CARGO_NET_RETRY: 10           # Retry network issues
  CARGO_TERM_COLOR: always
  RUSTFLAGS: -D warnings        # Deny warnings globally

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2

      - name: Check
        run: cargo check --all-targets --all-features

      - name: Clippy
        run: cargo clippy --all-targets --all-features

  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - uses: taiki-e/install-action@nextest

      - name: Compile tests
        run: cargo test --no-run --locked

      - name: Run tests
        run: cargo nextest run
```

### CI-Specific Profile

```toml
# Cargo.toml
[profile.ci]
inherits = "dev"
debug = false
incremental = false

[profile.ci.package."*"]
opt-level = 1  # Slight optimization for test runtime
```

### Cache Strategy

What to cache:
- `~/.cargo/registry/index/`
- `~/.cargo/registry/cache/`
- `~/.cargo/git/db/`
- `target/` (selectively)

What NOT to cache:
- `target/debug/` or `target/release/` (local crates)
- `target/.rustc_info.json`

---

## Quick Reference: .cargo/config.toml

```toml
# .cargo/config.toml

[alias]
b = "build"
c = "check"
t = "test"
r = "run"
br = "build --release"
rr = "run --release"

[build]
# Use mold linker on Linux
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]

# Use lld on Windows
[target.x86_64-pc-windows-msvc]
linker = "rust-lld"

[env]
# Example: Set environment variables
# RUST_BACKTRACE = "1"

[net]
retry = 3
git-fetch-with-cli = true

[term]
verbose = false
color = "auto"
```

---

## Additional Resources

- [The Rust Performance Book - Compile Times](https://nnethercote.github.io/perf-book/compile-times.html)
- [The Rust Performance Book - Build Configuration](https://nnethercote.github.io/perf-book/build-configuration.html)
- [Cargo Profiles Reference](https://doc.rust-lang.org/cargo/reference/profiles.html)
- [rustc Codegen Options](https://doc.rust-lang.org/rustc/codegen-options/index.html)
- [Fast Rust Builds by matklad](https://matklad.github.io/2021/09/04/fast-rust-builds.html)
- [Tips for Faster Rust Compile Times](https://corrode.dev/blog/tips-for-faster-rust-compile-times/)
