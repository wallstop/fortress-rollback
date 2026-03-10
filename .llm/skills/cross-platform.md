<!-- CATEGORY: Platform -->
<!-- WHEN: Cross-platform builds, cfg attributes, platform-specific code, CI matrix -->
# Cross-Platform Rust Development

## Target Matrix

| Target Triple | Platform | Tier | CI Runner |
|---------------|----------|------|-----------|
| `x86_64-unknown-linux-gnu` | Linux x64 (glibc) | 1 | `ubuntu-latest` |
| `x86_64-unknown-linux-musl` | Linux x64 (static) | 2 | `ubuntu-latest` + cross |
| `aarch64-unknown-linux-gnu` | Linux ARM64 | 2 | `ubuntu-latest` + cross |
| `x86_64-apple-darwin` | macOS Intel | 1 | `macos-latest` |
| `aarch64-apple-darwin` | macOS Apple Silicon | 1 | `macos-latest` |
| `x86_64-pc-windows-msvc` | Windows x64 | 1 | `windows-latest` |
| `wasm32-unknown-unknown` | WebAssembly | 2 | `ubuntu-latest` |
| `aarch64-apple-ios` | iOS Device | 2 | `macos-latest` |
| `aarch64-linux-android` | Android ARM64 | 2 | `ubuntu-latest` + cross |

## Cross-Compilation Tools

| Tool | Best For | Notes |
|------|----------|-------|
| **cross-rs** | Full cross-compile + testing | Docker-based, 60+ targets, QEMU testing |
| **cargo-zigbuild** | glibc version control | Uses Zig linker, no Docker |
| **cargo-xwin** | Linux to Windows MSVC | Easy MSVC without Windows |
| **Native rustup** | Single-platform CI | Just needs linker |

## Conditional Compilation Patterns

### cfg Attributes

```rust
// OS-specific
#[cfg(target_os = "linux")]
fn platform_init() { /* Linux */ }

#[cfg(any(target_os = "ios", target_os = "android"))]
mod mobile_impl;

// Architecture-specific
#[cfg(target_arch = "wasm32")]
mod wasm_impl;

#[cfg(not(target_arch = "wasm32"))]
mod native_impl;

// Combining conditions
#[cfg(all(target_arch = "wasm32", feature = "web"))]
mod web_impl;

#[cfg(not(any(target_arch = "wasm32", target_os = "ios", target_os = "android")))]
mod desktop_impl;
```

### Feature-Based Compilation

```toml
[features]
default = ["std"]
std = []
web = ["wasm-bindgen", "js-sys", "web-sys"]
mobile = ["std", "touch-input"]
networking = ["std"]
```

### Platform-Specific Dependencies

```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
js-sys = "0.3"
web-sys = { version = "0.3", features = ["console", "Window"] }

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
tokio = { version = "1.0", features = ["rt-multi-thread", "net", "time"] }
```

### Custom cfg in build.rs (Rust 1.80+)

```rust
fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    println!("cargo::rustc-check-cfg=cfg(is_mobile)");
    println!("cargo::rustc-check-cfg=cfg(has_threading)");
    match target_os.as_str() {
        "ios" | "android" => println!("cargo::rustc-cfg=is_mobile"),
        _ => {}
    }
    if target_arch != "wasm32" {
        println!("cargo::rustc-cfg=has_threading");
    }
    println!("cargo::rerun-if-changed=build.rs");
}
```

## Project Architecture

```
project/
+-- crates/core/           # Platform-agnostic Rust code
+-- bindings/
|   +-- ffi/               # C FFI for iOS/Android
|   +-- wasm/              # wasm-bindgen for Web
|   +-- uniffi/            # Mozilla uniffi bindings
+-- platforms/
|   +-- android/           # Android app
|   +-- ios/               # iOS Xcode project
|   +-- web/               # Web app
+-- Cargo.toml             # Workspace root
```

### Trait-Based Platform Abstraction

```rust
pub trait Platform {
    type Clock: Clock;
    type Random: Random;
    type Network: NetworkSocket;
    fn clock(&self) -> &Self::Clock;
    fn random(&mut self) -> &mut Self::Random;
    fn network(&mut self) -> &mut Self::Network;
}

pub trait Clock { fn now_millis(&self) -> u64; }
pub trait NetworkSocket {
    type Error;
    fn send(&mut self, data: &[u8]) -> Result<usize, Self::Error>;
    fn recv(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;
}
```

## CI Matrix Strategy

```yaml
strategy:
  fail-fast: false  # Run all platforms even if one fails
  matrix:
    include:
      - { os: ubuntu-latest, target: x86_64-unknown-linux-gnu }
      - { os: ubuntu-latest, target: wasm32-unknown-unknown }
      - { os: macos-latest, target: aarch64-apple-darwin }
      - { os: windows-latest, target: x86_64-pc-windows-msvc }
runs-on: ${{ matrix.os }}
steps:
  - uses: actions/checkout@v4
  - uses: dtolnay/rust-toolchain@stable
    with:
      targets: ${{ matrix.target }}
  - run: cargo build --release --target ${{ matrix.target }}
```

### Cost Optimization

macOS runners cost 10x, Windows 2x more than Linux. Cross-compile on Linux, verify on native only when needed:

```yaml
jobs:
  cross-compile:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        target: [x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu]
    steps:
      - uses: actions/checkout@v4
      - run: cargo install cross --git https://github.com/cross-rs/cross
      - run: cross build --release --target ${{ matrix.target }}
  macos-verify:
    runs-on: macos-latest
    needs: cross-compile
    if: github.ref == 'refs/heads/main'
    steps:
      - uses: actions/checkout@v4
      - run: cargo test
```

## Platform-Specific Gotchas

| Gotcha | Problem | Solution |
|--------|---------|----------|
| Float determinism | Different FPU behavior across platforms | Use `libm` or fixed-point math |
| HashMap iteration | Order varies between runs/platforms | Use `BTreeMap` or sort before iteration |
| glibc mismatch | Binary needs newer glibc than target | `cargo zigbuild --target x86_64-unknown-linux-gnu.2.17` |
| MSVC vs MinGW | MinGW embeds debug symbols (~100MB vs ~10MB) | Prefer `x86_64-pc-windows-msvc` |
| musl DNS | No NSS support with musl | Special handling needed |
| UniFFI mutability | Assumes multi-thread mutation | Use `Mutex`/`RwLock` interior mutability |
| cross-rs images | Unstable `:main` tags break CI | Use env var passthrough, not custom images |
| Mobile memory | Aggressive background app killing | Save state on every pause |
| WASM no threads | Single-threaded execution | Use async/await or Web Workers |
| Audio on web | Browser blocks until interaction | Initialize audio on first click |

### cross-rs: Environment Passthrough (Recommended)

```toml
# Cross.toml -- avoid custom images, use env passthrough
[build.env]
passthrough = [
    "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER",
    "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUSTFLAGS",
]
```

## Game Engine Platform Support

| Engine | Desktop | Web (WASM) | iOS | Android |
|--------|---------|------------|-----|---------|
| **Bevy** | Excellent | Good (WebGL2/WebGPU) | Good | Improved |
| **Macroquad** | Excellent | Excellent (WebGL1) | Experimental | Experimental |
| **godot-rust** | Excellent | Via Godot | Via Godot | Via Godot |

## Binding Tools Summary

| Platform | Binding Tool | Build System |
|----------|-------------|--------------|
| iOS | uniffi, swift-bridge | cargo-swift, Xcode |
| Android | uniffi, jni-rs | cargo-ndk, Gradle |
| Web | wasm-bindgen | wasm-pack, Trunk |

## Mobile: Touch Input Abstraction

```rust
pub struct Touch { pub id: u64, pub x: f32, pub y: f32, pub phase: TouchPhase }
pub enum TouchPhase { Started, Moved, Ended, Cancelled }
pub trait InputHandler {
    fn on_touch(&mut self, touch: Touch);
    fn on_key(&mut self, key: KeyCode, pressed: bool);
}
```

## WASM Size Optimization

```toml
[profile.release]
opt-level = 'z'
lto = true
codegen-units = 1
panic = 'abort'
strip = true
```

Post-build: `wasm-opt -Oz -o output.wasm input.wasm`

## Testing Across Platforms

```rust
#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use wasm_bindgen_test::*;
    wasm_bindgen_test_configure!(run_in_browser);
    #[wasm_bindgen_test]
    fn test_wasm_binding() { /* ... */ }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod native_tests {
    #[test]
    fn test_multithreaded() { /* ... */ }
}
```

### What Must Run Cross-Platform

| Category | Cross-Platform? | Rationale |
|----------|----------------|-----------|
| Unit/integration tests | Required | Platform-specific logic bugs |
| Loom/Miri | Required | Threading/memory layout differs |
| Clippy/fmt/coverage | One platform OK | Platform-agnostic |
| Kani/security scanning | One platform OK | Platform-agnostic |
