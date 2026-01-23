# Rust Binary Size Optimization Guide

> **A comprehensive guide to minimizing Rust binary sizes for embedded, WASM, containers, and resource-constrained deployments.**

## Overview

Rust optimizes for **execution speed** by default, not binary size. For applications requiring minimal binaries (embedded systems, WASM, containers, CLI tools), many techniques are available—from simple Cargo flags to advanced nightly features.

**Related Skills:**

- [cross-platform-rust.md](cross-platform-rust.md) — Cross-compilation and platform targeting
- [no-std-guide.md](no-std-guide.md) — `no_std` for minimal binaries
- [wasm-rust-guide.md](wasm-rust-guide.md) — WASM-specific optimizations

---

## Quick Reference — Copy-Paste Profiles

### Stable Rust — Balanced (Recommended Starting Point)

```toml
# Cargo.toml
[profile.release]
opt-level = "z"        # Optimize for size
lto = true             # Link-time optimization
codegen-units = 1      # Better optimization (slower compile)
panic = "abort"        # Remove unwinding machinery
strip = true           # Strip symbols
```

### Stable Rust — Maximum Size Reduction

```toml
[profile.release]
opt-level = "z"
lto = "fat"            # Full LTO (slower compile, smaller binary)
codegen-units = 1
panic = "abort"
strip = "symbols"
```

### Nightly Rust — Extreme Optimization

```toml
[profile.release]
opt-level = "z"
lto = "fat"
codegen-units = 1
panic = "abort"
strip = "symbols"
```

```bash
# Build command (replace <target> with your target triple)
RUSTFLAGS="-Zlocation-detail=none -Zfmt-debug=none" cargo +nightly build \
  -Z build-std=std,panic_abort \
  -Z build-std-features="optimize_for_size" \
  --target x86_64-unknown-linux-gnu --release
```

---

## Optimization Techniques (Ordered by Complexity)

### 1. Build in Release Mode

**Rust Version:** 1.0+ | **Impact:** ~60-70% reduction

```bash
cargo build --release
```

Debug builds include extensive debug info and disable optimizations. This is the essential first step.

---

### 2. Strip Symbols

**Rust Version:** 1.59+ (native) | **Impact:** 50-80% reduction

Removes debugging symbols not needed for execution.

**Cargo.toml (Recommended):**

```toml
[profile.release]
strip = true           # Strip all symbols
# OR
strip = "symbols"      # Same as true
# OR
strip = "debuginfo"    # Strip debug info only, keep symbol names
```

**Manual (older Rust or external):**

```bash
strip target/release/my-binary          # Linux/macOS
llvm-strip target/release/my-binary     # Cross-platform
```

---

### 3. Optimize for Size

**Rust Version:** 1.28+ | **Impact:** Variable (10-30%)

```toml
[profile.release]
opt-level = "z"        # Aggressive size optimization
# OR
opt-level = "s"        # Size optimization (sometimes smaller than "z"!)
```

| Level | Description |
|-------|-------------|
| `0` | No optimization |
| `1` | Basic optimization |
| `2` | Standard optimization |
| `3` | Maximum speed optimization |
| `"s"` | Optimize for size |
| `"z"` | Aggressive size optimization |

> **Important:** Always try BOTH `"s"` and `"z"`! Depending on your code, `"s"` can produce smaller binaries than `"z"`.

---

### 4. Enable Link-Time Optimization (LTO)

**Rust Version:** 1.0+ | **Impact:** 10-20% reduction

LTO allows the linker to optimize across crate boundaries, enabling dead code elimination and better inlining.

```toml
[profile.release]
lto = true             # Enable LTO (equivalent to "fat")
# OR
lto = "fat"            # Full LTO - maximum optimization, slowest compile
# OR
lto = "thin"           # Thin LTO - faster compile, good optimization
```

| Mode | Compile Time | Binary Size | Performance |
|------|-------------|-------------|-------------|
| `false` | Fast | Largest | Good |
| `"thin"` | Medium | Smaller | Good |
| `true`/`"fat"` | Slow | Smallest | Best |

---

### 5. Reduce Codegen Units

**Rust Version:** 1.0+ | **Impact:** 5-15% reduction

```toml
[profile.release]
codegen-units = 1      # Single codegen unit
```

By default, Cargo uses 16 parallel codegen units for faster compilation. Setting to `1` allows more aggressive cross-function optimizations.

**Trade-off:** Significantly longer compile times.

---

### 6. Abort on Panic

**Rust Version:** 1.10+ | **Impact:** 10-20% reduction

```toml
[profile.release]
panic = "abort"        # Abort immediately on panic
```

Removes the entire panic unwinding infrastructure (catching panics, running destructors during unwind).

> ⚠️ **Behavior Change:** `catch_unwind` won't work; `Drop` implementations won't run on panic.

---

### 7. Remove Location Details (Nightly)

**Rust Version:** Nightly only | **Impact:** 5-15% reduction

```bash
RUSTFLAGS="-Zlocation-detail=none" cargo +nightly build --release
```

Removes file path, line number, and column information embedded for `panic!()` and `#[track_caller]`.

---

### 8. Remove fmt::Debug (Nightly)

**Rust Version:** Nightly only | **Impact:** 5-15% reduction

```bash
RUSTFLAGS="-Zfmt-debug=none" cargo +nightly build --release
```

Turns `#[derive(Debug)]` and `{:?}` formatting into no-ops.

> ⚠️ **Warning:** Breaks `dbg!()`, `assert!()` output, `unwrap()` error messages.

---

### 9. Rebuild libstd with build-std (Nightly)

**Rust Version:** Nightly only | **Impact:** 20-50% reduction

Pre-built `libstd` is optimized for speed. Rebuilding it with size optimizations can dramatically reduce binary size.

**Setup:**

```bash
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly
```

**Build:**

```bash
# Find your target triple
rustc -vV   # Look for "host: ..."

cargo +nightly build \
  -Z build-std=std,panic_abort \
  -Z build-std-features="optimize_for_size" \
  --target x86_64-unknown-linux-gnu --release
```

**Combined with other flags:**

```bash
RUSTFLAGS="-Zlocation-detail=none -Zfmt-debug=none" cargo +nightly build \
  -Z build-std=std,panic_abort \
  -Z build-std-features="optimize_for_size" \
  --target x86_64-apple-darwin --release
```

---

### 10. Panic Immediate Abort (Nightly)

**Rust Version:** Nightly only | **Impact:** Additional 10-30% reduction

Even with `panic = "abort"`, panic formatting code is included. This removes it entirely:

```bash
RUSTFLAGS="-Zunstable-options -Cpanic=immediate-abort" cargo +nightly build \
  -Z build-std=std,panic_abort \
  --target x86_64-unknown-linux-gnu --release
```

---

### 11. UPX Compression (Post-build)

**Impact:** 50-70% additional reduction

UPX is an executable packer that compresses binaries.

```bash
# Install UPX
# Ubuntu/Debian: apt install upx
# macOS: brew install upx
# Windows: choco install upx

# Standard compression
upx --best --lzma target/release/my-binary

# Maximum compression (slower decompression)
upx --ultra-brute target/release/my-binary
```

**Example Results:**

| Stage | Size | Reduction |
|-------|------|-----------|
| Debug build | 5.7 MB | Baseline |
| Release build | 3.0 MB | ~47% |
| + LTO + strip | 1.5 MB | ~74% |
| + UPX | 700 KB | **88% total** |

> ⚠️ **Warnings:**
>
> - UPX-packed binaries may trigger heuristic antivirus software
> - Startup time increases slightly (decompression)
> - May not work with all targets (e.g., Windows MSVC, some embedded)
> - Debugging becomes harder

---

### 12. #![no_std] — Remove Standard Library

**Rust Version:** 1.30+ | **Impact:** Minimal binary possible (~10KB)

For the absolute smallest binaries, remove the standard library entirely.

```rust
#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

// Platform-specific entry point required
```

> ⚠️ **Extreme:** Loses access to most Rust ecosystem, requires `unsafe` code for I/O.

See [no-std-guide.md](no-std-guide.md) for complete `no_std` guidance.

---

## Dependency Optimization

### Audit Dependencies for Size Impact

Heavy dependencies dramatically increase binary size:

| Category | Heavy Deps | Lighter Alternatives |
|----------|------------|---------------------|
| CLI parsing | `clap` (600KB+) | `lexopt`, `pico-args`, `argh` |
| Serialization | `serde` (large) | `miniserde`, `nanoserde` |
| HTTP | `reqwest` (large) | `ureq`, `attohttpc`, `minreq` |
| Async runtime | `tokio` (large) | `smol`, `async-std`, `embassy` |
| Regex | `regex` (large) | `regex-lite`, `memchr` |
| Logging | `tracing` | `log` + lightweight backend |
| Error handling | `anyhow` | `thiserror` only, or custom |

### Remove Unused Features

```toml
[dependencies]
# ❌ Pulls in everything
serde = "1.0"

# ✅ Only what you need
serde = { version = "1.0", default-features = false, features = ["derive"] }

# ❌ Full tokio
tokio = { version = "1.0", features = ["full"] }

# ✅ Only needed features
tokio = { version = "1.0", features = ["rt", "net"] }
```

### Find Unused Features

```bash
# Install unused feature detector
cargo install cargo-unused-features

# Analyze project
unused-features analyze
```

### Analyze Binary Composition

```bash
# What's taking space?
cargo install cargo-bloat
cargo bloat --release --crates   # Size by crate
cargo bloat --release -n 30       # Top 30 functions by size

# For WASM specifically
cargo install twiggy
twiggy top -n 20 target/wasm32-unknown-unknown/release/my_lib.wasm

# Generic functions taking too much space?
cargo install cargo-llvm-lines
cargo llvm-lines --release | head -20
```

---

## Platform-Specific Considerations

### Windows: MSVC vs GNU

**MSVC produces significantly smaller binaries:**

| Toolchain | Binary Size | Reason |
|-----------|-------------|--------|
| x86_64-pc-windows-gnu (MinGW) | 100 MB | Embeds debug symbols |
| x86_64-pc-windows-msvc | 10 MB | Symbols in separate .pdb |

**Prefer MSVC for Windows releases.**

### Linux: musl vs glibc

| Target | Binary Type | Trade-offs |
|--------|-------------|-----------|
| `*-linux-gnu` | Dynamic (needs glibc) | Smaller binary, runtime dependency |
| `*-linux-musl` | Static (self-contained) | Larger binary, fully portable |

For maximum portability with small size:

```toml
[profile.release]
strip = true
lto = true
# ... other optimizations
```

```bash
# Build static musl binary
cargo build --release --target x86_64-unknown-linux-musl
```

### WASM-Specific Optimization

```toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

```bash
# Additional WASM optimization with wasm-opt
cargo build --release --target wasm32-unknown-unknown
wasm-opt -Oz -o optimized.wasm target/wasm32-unknown-unknown/release/my_lib.wasm

# Analyze WASM size
twiggy top -n 20 optimized.wasm
```

---

## Container Optimization

### Multi-Stage Dockerfile

```dockerfile
# Build stage
FROM rust:1.83-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /app
COPY . .

RUN cargo build --release --target x86_64-unknown-linux-musl

# Runtime stage - distroless for minimal attack surface
FROM gcr.io/distroless/static-debian12

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/myapp /

ENTRYPOINT ["/myapp"]
```

**Base image sizes:**

| Image | Size | Notes |
|-------|------|-------|
| `debian:bookworm` | ~130 MB | Full system |
| `alpine:3` | ~7 MB | Minimal Linux |
| `gcr.io/distroless/static` | ~2 MB | Just the runtime |
| `scratch` | 0 MB | Empty (needs static binary) |

### Scratch Image (Smallest)

```dockerfile
FROM rust:1.83-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl && \
    strip target/x86_64-unknown-linux-musl/release/myapp

FROM scratch
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/myapp /myapp
ENTRYPOINT ["/myapp"]
```

---

## Size Optimization Checklist

### Stable Rust (Safe, No Behavior Changes)

- [ ] Build with `--release`
- [ ] Add `strip = true`
- [ ] Try both `opt-level = "z"` and `opt-level = "s"`
- [ ] Add `lto = true`
- [ ] Add `codegen-units = 1`

### Stable Rust (With Behavior Changes)

- [ ] Add `panic = "abort"` (if unwinding not needed)

### Nightly Rust (Additional Reductions)

- [ ] Use `-Zlocation-detail=none`
- [ ] Use `-Zfmt-debug=none`
- [ ] Use `-Z build-std=std,panic_abort`
- [ ] Use `-Z build-std-features="optimize_for_size"`
- [ ] Use `-Cpanic=immediate-abort`

### Dependency Hygiene

- [ ] Audit heavy dependencies with `cargo bloat --crates`
- [ ] Disable default features where possible
- [ ] Consider lighter alternatives for CLI, HTTP, serialization
- [ ] Run `cargo unused-features analyze`

### Post-Build

- [ ] Apply UPX compression (if appropriate)
- [ ] Verify binary runs correctly after optimization

---

## Tools Reference

| Tool | Purpose | Install |
|------|---------|---------|
| `cargo-bloat` | Find what's taking space | `cargo install cargo-bloat` |
| `cargo-llvm-lines` | Find generic bloat | `cargo install cargo-llvm-lines` |
| `cargo-unused-features` | Find unused features | `cargo install cargo-unused-features` |
| `twiggy` | WASM code size profiler | `cargo install twiggy` |
| `wasm-opt` | WASM binary optimizer | Part of Binaryen |
| `UPX` | Executable packer | System package manager |
| `strip` / `llvm-strip` | Remove symbols | Usually pre-installed |

---

## Trade-offs Summary

| Technique | Size Impact | Compile Time | Runtime Speed | Behavior Change |
|-----------|-------------|--------------|---------------|-----------------|
| Release build | ↓↓↓ | Slower | ↑↑↑ | No |
| Strip symbols | ↓↓ | None | None | No (harder debugging) |
| opt-level z/s | ↓ | Similar | ↓ | No |
| LTO | ↓↓ | ↑↑↑ | ↑ | No |
| codegen-units=1 | ↓ | ↑↑ | ↑ | No |
| panic=abort | ↓↓ | None | None | **Yes** - no unwinding |
| build-std | ↓↓↓ | ↑↑↑ | Varies | No |
| immediate-abort | ↓↓ | Slight | None | **Yes** - no panic msg |
| no_std | ↓↓↓↓ | Varies | Varies | **Yes** - limited API |
| UPX | ↓↓↓ | Post-build | ↓ startup | **Yes** - AV flags |

---

## Example: Real-World Optimization Journey

Starting point: "Hello World" CLI tool

| Step | Binary Size | Cumulative Reduction |
|------|-------------|---------------------|
| Debug build | 4.2 MB | — |
| Release build | 410 KB | 90% |
| + strip | 310 KB | 93% |
| + opt-level=z | 290 KB | 93% |
| + LTO | 250 KB | 94% |
| + codegen-units=1 | 240 KB | 94% |
| + panic=abort | 180 KB | 96% |
| + build-std (nightly) | 51 KB | 99% |
| + UPX | 24 KB | **99.4%** |

---

## Resources

- [min-sized-rust](https://github.com/johnthagen/min-sized-rust) — Comprehensive size reduction guide
- [Rust Binary Size Working Group](https://github.com/rust-lang/wg-binary-size) — Official efforts
- [Making Rust binaries smaller by default](https://kobzol.github.io/rust/cargo/2024/01/23/making-rust-binaries-smaller-by-default.html) — 2024 improvements
- [cargo-bloat](https://github.com/RazrFalcon/cargo-bloat) — Size analysis tool

---

*Optimizing for size is a trade-off. Start with stable techniques and only reach for nightly/extreme measures when necessary.*
