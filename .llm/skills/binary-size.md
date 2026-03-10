<!-- CATEGORY: Performance & Quality -->
<!-- WHEN: Reducing binary size, WASM size optimization, LTO configuration -->

# Binary Size Optimization

---

## Quick Profiles

### Stable Rust -- Balanced

```toml
[profile.release]
opt-level = "z"        # Optimize for size (try "s" too -- sometimes smaller)
lto = true             # Link-time optimization
codegen-units = 1      # Better optimization
panic = "abort"        # Remove unwinding machinery
strip = true           # Strip symbols
```

### Nightly -- Maximum Reduction

```bash
RUSTFLAGS="-Zlocation-detail=none -Zfmt-debug=none" cargo +nightly build \
  -Z build-std=std,panic_abort \
  -Z build-std-features="optimize_for_size" \
  --target x86_64-unknown-linux-gnu --release
```

---

## Techniques by Complexity

| Technique | Rust | Impact | Behavior Change |
|-----------|------|--------|-----------------|
| Release build | 1.0+ | 60-70% | No |
| `strip = true` | 1.59+ | 50-80% | No |
| `opt-level = "z"` or `"s"` | 1.28+ | 10-30% | No |
| `lto = true` ("fat") | 1.0+ | 10-20% | No |
| `codegen-units = 1` | 1.0+ | 5-15% | No |
| `panic = "abort"` | 1.10+ | 10-20% | Yes: no unwinding |
| `-Zlocation-detail=none` | Nightly | 5-15% | No panic location |
| `-Zfmt-debug=none` | Nightly | 5-15% | No `{:?}` output |
| `-Z build-std` | Nightly | 20-50% | No |
| `-Cpanic=immediate-abort` | Nightly | 10-30% | No panic message |
| UPX compression | Post | 50-70% | AV flags, startup |
| `#![no_std]` | 1.30+ | ~10KB possible | Limited API |

Always try BOTH `"s"` and `"z"` -- depending on code, `"s"` can be smaller.

---

## Dependency Optimization

### Heavy vs Light Alternatives

| Category | Heavy | Lighter Alternative |
|----------|-------|---------------------|
| CLI parsing | `clap` | `lexopt`, `pico-args`, `argh` |
| Serialization | `serde` | `miniserde`, `nanoserde` |
| HTTP | `reqwest` | `ureq`, `minreq` |
| Async runtime | `tokio` | `smol`, `embassy` |
| Regex | `regex` | `regex-lite`, `memchr` |
| Error handling | `anyhow` | `thiserror` only |

### Minimize Features

```toml
serde = { version = "1.0", default-features = false, features = ["derive"] }
tokio = { version = "1.0", features = ["rt", "net"] }  # Not "full"
```

### Analysis Tools

```bash
cargo bloat --release --crates     # Size by crate
cargo bloat --release -n 30         # Top 30 functions
cargo llvm-lines --release | head -20  # Generic bloat
# WASM
twiggy top -n 20 target/wasm32-unknown-unknown/release/my_lib.wasm
```

---

## WASM-Specific

```toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

```bash
cargo build --release --target wasm32-unknown-unknown
wasm-opt -Oz -o optimized.wasm target/wasm32-unknown-unknown/release/my_lib.wasm
```

---

## Platform Notes

### Windows: MSVC vs GNU

MSVC produces significantly smaller binaries (~10MB vs ~100MB for GNU/MinGW). Prefer MSVC for releases.

### Linux: musl vs glibc

| Target | Trade-off |
|--------|-----------|
| `*-linux-gnu` | Smaller binary, needs glibc at runtime |
| `*-linux-musl` | Larger binary, fully portable static |

---

## Container Optimization

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

| Base Image | Size |
|------------|------|
| `debian:bookworm` | ~130 MB |
| `alpine:3` | ~7 MB |
| `distroless/static` | ~2 MB |
| `scratch` | 0 MB |

---

## Checklist

### Stable (No Behavior Changes)

- [ ] Build with `--release`
- [ ] `strip = true`
- [ ] Try both `opt-level = "z"` and `"s"`
- [ ] `lto = true`
- [ ] `codegen-units = 1`

### Stable (With Behavior Changes)

- [ ] `panic = "abort"` (if unwinding not needed)

### Nightly

- [ ] `-Zlocation-detail=none`
- [ ] `-Zfmt-debug=none`
- [ ] `-Z build-std=std,panic_abort`
- [ ] `-Z build-std-features="optimize_for_size"`

### Dependencies

- [ ] `cargo bloat --crates` to audit heavy deps
- [ ] Disable default features
- [ ] Consider lighter alternatives
- [ ] `cargo unused-features analyze`

### Post-Build

- [ ] UPX compression (if appropriate)
- [ ] Verify binary runs correctly

---

## Example Journey

| Step | Size | Reduction |
|------|------|-----------|
| Debug build | 4.2 MB | -- |
| Release | 410 KB | 90% |
| + strip | 310 KB | 93% |
| + opt-level=z | 290 KB | 93% |
| + LTO | 250 KB | 94% |
| + codegen-units=1 | 240 KB | 94% |
| + panic=abort | 180 KB | 96% |
| + build-std (nightly) | 51 KB | 99% |
| + UPX | 24 KB | 99.4% |

---

## Tools

| Tool | Purpose |
|------|---------|
| `cargo-bloat` | Find what takes space |
| `cargo-llvm-lines` | Find generic bloat |
| `cargo-unused-features` | Find unused features |
| `twiggy` | WASM code size profiler |
| `wasm-opt` | WASM binary optimizer |
| UPX | Executable packer |
