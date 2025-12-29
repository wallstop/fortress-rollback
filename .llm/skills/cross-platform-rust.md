# Cross-Platform Rust Development Guide

> **A guide to writing Rust code that targets multiple platforms: native desktop, mobile (iOS/Android), and WebAssembly.**

## Overview

Rust's zero-cost abstractions and explicit platform handling make it excellent for cross-platform development. The key is a **shared core library** with platform-specific binding layers.

**Related Skills:**

- [cross-platform-games.md](cross-platform-games.md) — Game-specific cross-platform patterns
- [wasm-rust-guide.md](wasm-rust-guide.md) — WebAssembly deep dive
- [no-std-guide.md](no-std-guide.md) — `no_std` for embedded/WASM

---

## Cross-Compilation Tooling (2024-2025)

### Tool Comparison

| Tool | Best For | Setup Complexity | Features |
|------|----------|------------------|----------|
| **cross-rs** | Full cross-compile + testing | Medium (Docker) | 60+ targets, QEMU testing |
| **cargo-zigbuild** | Simple Linux/macOS builds | Low | glibc version control |
| **Native rustup** | Single-platform CI | Lowest | Just needs linker |

### cross-rs (Recommended for Complex Scenarios)

Docker-based cross-compilation with zero setup for most targets:

```bash
# Install
cargo install cross --git https://github.com/cross-rs/cross

# Use exactly like cargo
cross build --target aarch64-unknown-linux-gnu
cross test --target aarch64-unknown-linux-gnu  # Runs via QEMU!
```

**Configuration (Cross.toml):**

```toml
[build]
default-target = "x86_64-unknown-linux-gnu"

# Install target-specific dependencies
[build.pre-build]
commands = [
    "dpkg --add-architecture $CROSS_DEB_ARCH",
    "apt-get update && apt-get -y install libssl-dev:$CROSS_DEB_ARCH"
]

# Use zig as cross-linker for glibc version control
[target.aarch64-unknown-linux-gnu]
zig = "2.17"  # Target glibc 2.17

# Custom Docker image
[target.x86_64-unknown-linux-musl.dockerfile]
file = "./Dockerfile.musl"
```

### cargo-zigbuild (Simple glibc Targeting)

```bash
cargo install --locked cargo-zigbuild

# Cross-compile with specific glibc version
cargo zigbuild --target aarch64-unknown-linux-gnu.2.17

# macOS universal binary (ARM64 + x86_64)
rustup target add x86_64-apple-darwin aarch64-apple-darwin
cargo zigbuild --target universal2-apple-darwin
```

### Target Installation with rustup

```bash
# List available targets
rustup target list

# Add targets
rustup target add aarch64-unknown-linux-gnu
rustup target add wasm32-unknown-unknown
rustup target add aarch64-apple-ios

# Pin targets in rust-toolchain.toml
```

**rust-toolchain.toml:**

```toml
[toolchain]
channel = "1.83.0"
components = ["clippy", "rustfmt", "rust-src"]
targets = [
    "x86_64-unknown-linux-gnu",
    "x86_64-unknown-linux-musl",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
    "wasm32-unknown-unknown",
]
```

---

## Project Architecture

### Recommended Structure

```
project/
├── crates/
│   └── core/                  # Platform-agnostic Rust code
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           └── game_logic.rs
├── bindings/
│   ├── ffi/                   # C FFI for iOS/Android
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── wasm/                  # wasm-bindgen for Web
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   └── uniffi/                # Mozilla's uniffi bindings
│       ├── Cargo.toml
│       ├── src/lib.rs
│       └── interface.udl
├── platforms/
│   ├── android/               # Android app
│   ├── ios/                   # iOS Xcode project
│   └── web/                   # Web app
└── Cargo.toml                 # Workspace root
```

### Workspace Cargo.toml

```toml
[workspace]
members = [
    "crates/core",
    "bindings/ffi",
    "bindings/wasm",
]
resolver = "2"

[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
```

---

## Platform Abstraction Techniques

### Trait-Based Abstraction

```rust
// crates/core/src/platform.rs

/// Platform-agnostic clock interface
pub trait Clock {
    fn now_millis(&self) -> u64;
}

/// Platform-agnostic network interface
pub trait NetworkSocket {
    type Error;
    fn send(&mut self, data: &[u8]) -> Result<usize, Self::Error>;
    fn recv(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;
}

// Core logic uses traits, not concrete implementations
pub fn game_loop<C: Clock, N: NetworkSocket>(
    clock: &C,
    network: &mut N,
    state: &mut GameState,
) -> Result<(), N::Error> {
    let dt = clock.now_millis();
    state.update(dt);
    network.send(&state.serialize())?;
    Ok(())
}
```

### Platform-Specific Implementations

```rust
// Native implementation
#[cfg(not(target_arch = "wasm32"))]
pub struct NativeClock;

#[cfg(not(target_arch = "wasm32"))]
impl Clock for NativeClock {
    fn now_millis(&self) -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
}

// WASM implementation
#[cfg(target_arch = "wasm32")]
pub struct WasmClock;

#[cfg(target_arch = "wasm32")]
impl Clock for WasmClock {
    fn now_millis(&self) -> u64 {
        js_sys::Date::now() as u64
    }
}
```

---

## Build.rs Patterns for Cross-Platform

### Key Environment Variables

| Variable | Purpose |
|----------|---------|
| `TARGET` | Target triple being compiled for |
| `HOST` | Host triple (your machine) |
| `OUT_DIR` | Directory for build artifacts |
| `CARGO_CFG_TARGET_OS` | Target OS (linux, windows, macos, etc.) |
| `CARGO_CFG_TARGET_ARCH` | Target architecture (x86_64, aarch64, wasm32) |

### Platform Detection in build.rs

```rust
// build.rs
fn main() {
    let target = std::env::var("TARGET").unwrap();
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    // Register custom cfgs (required since Rust 1.80)
    println!("cargo::rustc-check-cfg=cfg(is_mobile)");
    println!("cargo::rustc-check-cfg=cfg(has_threading)");

    // Set custom cfg flags based on platform
    match target_os.as_str() {
        "ios" | "android" => println!("cargo::rustc-cfg=is_mobile"),
        _ => {}
    }

    // WASM doesn't have native threading
    if target_arch != "wasm32" {
        println!("cargo::rustc-cfg=has_threading");
    }

    // Rerun only when build.rs changes
    println!("cargo::rerun-if-changed=build.rs");
}
```

### Linking Native Libraries

```rust
// build.rs using the `cc` crate (handles cross-compilation automatically)
fn main() {
    cc::Build::new()
        .file("src/native_helper.c")
        .compile("native_helper");

    // Link system libraries platform-specifically
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    match target_os.as_str() {
        "linux" => println!("cargo::rustc-link-lib=dl"),
        "macos" => println!("cargo::rustc-link-lib=framework=CoreFoundation"),
        "windows" => println!("cargo::rustc-link-lib=user32"),
        _ => {}
    }

    println!("cargo::rerun-if-changed=src/native_helper.c");
}
```

### Modern Build Script Syntax (Rust 1.77+)

```rust
// Use cargo:: prefix (not cargo:) for modern syntax
println!("cargo::rerun-if-changed=src/");
println!("cargo::rustc-link-lib=static=mylib");
println!("cargo::rustc-link-search=native=/path/to/lib");
println!("cargo::rustc-cfg=my_feature");
println!("cargo::rustc-env=MY_VAR=value");
println!("cargo::metadata=key=value");  // Pass info to dependent crates
```

---

## Conditional Compilation

### Target-Based Compilation

```rust
// OS-specific
#[cfg(target_os = "android")]
fn platform_init() { /* Android-specific */ }

#[cfg(target_os = "ios")]
fn platform_init() { /* iOS-specific */ }

#[cfg(target_os = "windows")]
fn platform_init() { /* Windows-specific */ }

#[cfg(target_os = "linux")]
fn platform_init() { /* Linux-specific */ }

#[cfg(target_os = "macos")]
fn platform_init() { /* macOS-specific */ }

// Architecture-specific
#[cfg(target_arch = "wasm32")]
fn platform_init() { /* WASM-specific */ }

#[cfg(target_arch = "x86_64")]
fn platform_init() { /* x86-64-specific */ }

#[cfg(target_arch = "aarch64")]
fn platform_init() { /* ARM64-specific */ }
```

### Feature-Based Compilation

```toml
# Cargo.toml
[features]
default = ["std"]
std = []
alloc = []

# Platform features
mobile = []
desktop = []
web = ["wasm-bindgen", "js-sys", "web-sys"]

# Optional capabilities
networking = ["std"]
multithreading = ["std"]
```

```rust
// Use features in code
#[cfg(feature = "std")]
use std::collections::HashMap;

#[cfg(not(feature = "std"))]
use alloc::collections::BTreeMap as HashMap;

#[cfg(feature = "networking")]
mod network;

#[cfg(feature = "multithreading")]
use std::sync::Arc;

#[cfg(not(feature = "multithreading"))]
use core::cell::RefCell;
```

### Combining Conditions

```rust
// Multiple conditions with all/any
#[cfg(all(target_arch = "wasm32", feature = "web"))]
mod web_impl;

#[cfg(any(target_os = "ios", target_os = "android"))]
mod mobile_impl;

#[cfg(not(any(target_arch = "wasm32", target_os = "ios", target_os = "android")))]
mod desktop_impl;
```

---

## Platform-Specific Dependencies

### Cargo.toml Configuration

```toml
[dependencies]
# Always included
serde = { version = "1.0", default-features = false, features = ["derive"] }

# std-only dependencies
[dependencies.tokio]
version = "1.0"
optional = true

[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
js-sys = "0.3"
web-sys = { version = "0.3", features = ["console", "Window"] }
wasm-bindgen-futures = "0.4"

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
tokio = { version = "1.0", features = ["rt-multi-thread", "net", "time"] }

[target.'cfg(target_os = "android")'.dependencies]
android_logger = "0.14"
jni = "0.21"

[target.'cfg(target_os = "ios")'.dependencies]
objc = "0.2"
```

---

## FFI Bindings by Platform

### C FFI (iOS/Android)

```rust
// bindings/ffi/src/lib.rs
use core_lib::GameState;

#[repr(C)]
pub struct FfiGameState {
    ptr: *mut GameState,
}

#[no_mangle]
pub extern "C" fn game_state_new() -> FfiGameState {
    let state = Box::new(GameState::new());
    FfiGameState {
        ptr: Box::into_raw(state),
    }
}

#[no_mangle]
pub extern "C" fn game_state_update(state: &mut FfiGameState, dt: f32) {
    unsafe {
        if let Some(s) = state.ptr.as_mut() {
            s.update(dt);
        }
    }
}

#[no_mangle]
pub extern "C" fn game_state_free(state: FfiGameState) {
    if !state.ptr.is_null() {
        unsafe {
            drop(Box::from_raw(state.ptr));
        }
    }
}
```

### Mozilla uniffi (iOS/Android)

```
// bindings/uniffi/interface.udl
namespace game_lib {
    string get_version();
};

interface GameState {
    constructor();
    void update(f32 dt);
    string serialize();
};
```

```rust
// bindings/uniffi/src/lib.rs
uniffi::setup_scaffolding!();

#[derive(uniffi::Object)]
pub struct GameState {
    inner: core_lib::GameState,
}

#[uniffi::export]
impl GameState {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self { inner: core_lib::GameState::new() }
    }

    pub fn update(&self, dt: f32) {
        self.inner.update(dt);
    }

    pub fn serialize(&self) -> String {
        self.inner.serialize()
    }
}

#[uniffi::export]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
```

### swift-bridge (iOS)

```rust
// bindings/swift/src/lib.rs
#[swift_bridge::bridge]
mod ffi {
    extern "Rust" {
        type GameState;

        #[swift_bridge(init)]
        fn new() -> GameState;

        #[swift_bridge(swift_name = "update")]
        fn update(&mut self, dt: f32);

        fn serialize(&self) -> String;
    }
}

pub struct GameState {
    inner: core_lib::GameState,
}

impl GameState {
    pub fn new() -> Self {
        Self { inner: core_lib::GameState::new() }
    }

    pub fn update(&mut self, dt: f32) {
        self.inner.update(dt);
    }

    pub fn serialize(&self) -> String {
        self.inner.serialize()
    }
}
```

### wasm-bindgen (Web)

```rust
// bindings/wasm/src/lib.rs
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct GameState {
    inner: core_lib::GameState,
}

#[wasm_bindgen]
impl GameState {
    #[wasm_bindgen(constructor)]
    pub fn new() -> GameState {
        console_error_panic_hook::set_once();
        GameState { inner: core_lib::GameState::new() }
    }

    pub fn update(&mut self, dt: f32) {
        self.inner.update(dt);
    }

    pub fn serialize(&self) -> String {
        self.inner.serialize()
    }
}
```

---

## Build System Integration

### Android (Gradle + Cargo)

```groovy
// build.gradle
android {
    // ...

    externalNativeBuild {
        cmake {
            path "CMakeLists.txt"
        }
    }
}
```

```cmake
# CMakeLists.txt
cmake_minimum_required(VERSION 3.22)

include(FetchContent)
FetchContent_Declare(
    Corrosion
    GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git
    GIT_TAG v0.5
)
FetchContent_MakeAvailable(Corrosion)

corrosion_import_crate(MANIFEST_PATH ../../bindings/ffi/Cargo.toml)
```

### iOS (Xcode + cargo-xcode)

```bash
# Install cargo-xcode
cargo install cargo-xcode

# Generate Xcode project
cd bindings/swift
cargo xcode

# Or use build script
cargo build --target aarch64-apple-ios --release
cargo build --target aarch64-apple-ios-sim --release
```

### Web (wasm-pack)

```json
// package.json
{
  "scripts": {
    "build": "wasm-pack build bindings/wasm --target web --out-dir ../../platforms/web/pkg",
    "build:node": "wasm-pack build bindings/wasm --target nodejs"
  }
}
```

---

## Testing Across Platforms

### Core Tests (Platform-Agnostic)

```rust
// crates/core/src/lib.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_logic() {
        let mut state = GameState::new();
        state.update(0.016);
        assert!(state.is_valid());
    }
}
```

### Platform-Specific Tests

```rust
// WASM tests
#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_wasm_binding() {
        let state = super::GameState::new();
        assert!(state.serialize().len() > 0);
    }
}

// Native tests with threading
#[cfg(all(test, not(target_arch = "wasm32")))]
mod native_tests {
    #[test]
    fn test_multithreaded() {
        use std::thread;
        let handles: Vec<_> = (0..4)
            .map(|_| thread::spawn(|| super::GameState::new()))
            .collect();
        for h in handles {
            assert!(h.join().is_ok());
        }
    }
}
```

### CI Configuration

```yaml
# .github/workflows/cross-platform.yml
jobs:
  test:
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: macos-latest
            target: aarch64-apple-darwin
          - os: windows-latest
            target: x86_64-pc-windows-msvc
          - os: ubuntu-latest
            target: wasm32-unknown-unknown
          - os: ubuntu-latest
            target: aarch64-linux-android

    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - run: cargo test --target ${{ matrix.target }} --workspace
```

---

## Common Patterns

### Unified Error Type

```rust
// crates/core/src/error.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    InvalidState { reason: &'static str },
    NetworkError { message: String },
    ParseError { details: String },
}

// Platform-specific error conversion
#[cfg(target_arch = "wasm32")]
impl From<Error> for wasm_bindgen::JsValue {
    fn from(e: Error) -> Self {
        wasm_bindgen::JsValue::from_str(&format!("{:?}", e))
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}
```

### Platform Abstraction Layer

```rust
// crates/core/src/platform.rs

/// Platform-specific functionality
pub trait Platform {
    type Clock: Clock;
    type Random: Random;
    type Network: NetworkSocket;

    fn clock(&self) -> &Self::Clock;
    fn random(&mut self) -> &mut Self::Random;
    fn network(&mut self) -> &mut Self::Network;
}

// Use in core logic
pub fn run_frame<P: Platform>(platform: &mut P, state: &mut GameState) {
    let now = platform.clock().now_millis();
    let random_value = platform.random().next_u32();
    state.update_with(now, random_value);
}
```

### Serialization Across Platforms

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct GameState {
    pub frame: u64,
    pub entities: Vec<Entity>,
    // Using Vec<u8> for cross-platform byte serialization
    pub custom_data: Vec<u8>,
}

impl GameState {
    pub fn to_bytes(&self) -> Vec<u8> {
        // Use a deterministic serialization format
        bincode::serialize(self).expect("serialization should not fail")
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        bincode::deserialize(bytes)
            .map_err(|e| Error::ParseError { details: e.to_string() })
    }
}
```

---

## Platform-Specific Considerations

### Mobile (iOS/Android)

- **Battery life**: Avoid busy loops, use system timers
- **Background handling**: Save state when app backgrounds
- **Memory**: Monitor heap usage, avoid large allocations
- **Touch input**: Abstract touch coordinates to logical units

### WebAssembly

- **No threads**: Use `async`/`await` or Web Workers
- **No filesystem**: Use IndexedDB via `idb` crate
- **Binary size**: Optimize with `opt-level = "z"`, strip, LTO
- **Startup time**: Use streaming compilation

### Desktop

- **Windowing**: Use `winit` for cross-platform windows
- **GPU**: Use `wgpu` for cross-platform graphics
- **Audio**: Use `cpal` for cross-platform audio

---

## Mobile Development Tooling (2024-2025)

### iOS Build Setup

```bash
# Install targets
rustup target add aarch64-apple-ios           # Device (ARM64)
rustup target add aarch64-apple-ios-sim       # Simulator (Apple Silicon)
rustup target add x86_64-apple-ios            # Simulator (Intel)

# Build for device
cargo build --release --target aarch64-apple-ios

# Build for simulator (detect your Mac's architecture)
cargo build --release --target aarch64-apple-ios-sim  # M1/M2/M3
cargo build --release --target x86_64-apple-ios       # Intel
```

**Key iOS Tools:**

| Tool | Purpose |
|------|---------|
| `cargo-swift` | Generate Swift Packages from UniFFI |
| `cargo-xcode` | Generate Xcode project files |
| `cbindgen` | Generate C/C++ headers for FFI |

### Android Build Setup with cargo-ndk

```bash
# Install cargo-ndk
cargo install cargo-ndk

# Install targets
rustup target add aarch64-linux-android    # ARM64 (most modern devices)
rustup target add armv7-linux-androideabi  # ARMv7 (legacy devices)
rustup target add x86_64-linux-android     # x86_64 emulator
rustup target add i686-linux-android       # x86 emulator

# Build for multiple ABIs, output to jniLibs
cargo ndk -t arm64-v8a -t armeabi-v7a -o ./app/src/main/jniLibs build --release

# With specific platform level
cargo ndk --platform 24 -t arm64-v8a build --release
```

**Android Activity Types:**

- **NativeActivity**: Simpler, full Rust app without Java/Kotlin
- **GameActivity**: Better input handling (AGDK-based), recommended for games

### UniFFI Bindings (Recommended for Mobile)

UniFFI generates Swift, Kotlin, and Python bindings from Rust code:

```rust
// Using proc macros (modern approach)
uniffi::setup_scaffolding!();

#[derive(uniffi::Object)]
pub struct GameEngine {
    state: GameState,
}

#[uniffi::export]
impl GameEngine {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self { state: GameState::default() }
    }

    pub fn update(&mut self, delta_time: f32) {
        self.state.update(delta_time);
    }

    pub fn get_score(&self) -> u32 {
        self.state.score
    }
}

#[uniffi::export]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
```

Generate bindings:

```bash
# Swift (iOS)
cargo swift package -p my-game -n MyGame

# Kotlin (Android) - typically via gradle plugin
```

---

## CI/CD for Cross-Platform Projects

### GitHub Actions Multi-Platform Build

```yaml
# .github/workflows/cross-platform.yml
name: Cross-Platform CI

on: [push, pull_request]

jobs:
  build:
    strategy:
      fail-fast: false
      matrix:
        include:
          # Desktop
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
          - target: aarch64-unknown-linux-gnu
            os: ubuntu-latest
            use_cross: true
          - target: x86_64-apple-darwin
            os: macos-latest
          - target: aarch64-apple-darwin
            os: macos-latest
          - target: x86_64-pc-windows-msvc
            os: windows-latest

          # WASM
          - target: wasm32-unknown-unknown
            os: ubuntu-latest

          # Mobile (build only)
          - target: aarch64-linux-android
            os: ubuntu-latest
            use_cross: true
          - target: aarch64-apple-ios
            os: macos-latest

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Install cross
        if: matrix.use_cross
        run: cargo install cross --git https://github.com/cross-rs/cross

      - name: Install Linux dependencies
        if: matrix.os == 'ubuntu-latest' && !matrix.use_cross && matrix.target != 'wasm32-unknown-unknown'
        run: |
          sudo apt-get update
          sudo apt-get install -y libasound2-dev libudev-dev

      - name: Build (native)
        if: ${{ !matrix.use_cross }}
        run: cargo build --release --target ${{ matrix.target }}

      - name: Build (cross)
        if: matrix.use_cross
        run: cross build --release --target ${{ matrix.target }}

      - name: Test (native, non-mobile)
        if: ${{ !matrix.use_cross && !contains(matrix.target, 'ios') && !contains(matrix.target, 'android') && matrix.target != 'wasm32-unknown-unknown' }}
        run: cargo test --target ${{ matrix.target }}
```

### Cost-Optimized CI Strategy

Cross-compile on cheap Linux runners, test on native hardware only when needed:

```yaml
jobs:
  # Fast Linux builds for all targets
  cross-compile:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        target:
          - x86_64-unknown-linux-gnu
          - aarch64-unknown-linux-gnu
          - x86_64-pc-windows-gnu  # MinGW
    steps:
      - uses: actions/checkout@v4
      - name: Install cross
        run: cargo install cross --git https://github.com/cross-rs/cross
      - name: Build
        run: cross build --release --target ${{ matrix.target }}

  # Native runners only for final verification
  verify-macos:
    runs-on: macos-latest
    needs: cross-compile  # Only run after cross-compile succeeds
    steps:
      - uses: actions/checkout@v4
      - run: cargo test
```

---

## Tool Summary

| Platform | Binding Tool | Build System | CI Runner |
|----------|-------------|--------------|-----------|
| Linux (native) | N/A | cargo | ubuntu-latest |
| Linux (cross) | N/A | cross-rs | ubuntu-latest |
| macOS | N/A | cargo | macos-latest |
| Windows | N/A | cargo | windows-latest |
| iOS | uniffi, swift-bridge | cargo-swift, Xcode | macos-latest |
| Android | uniffi, jni-rs | cargo-ndk, Gradle | ubuntu-latest + cross |
| Web | wasm-bindgen | wasm-pack, Trunk | ubuntu-latest |

---

## Common Pitfalls

### Pitfall: Breaking Other Platforms Silently

**Problem:** Changes compile on your platform but break others.

**Solution:** CI must build all targets on every PR.

### Pitfall: Global Cargo Config Conflicts

**Problem:** `~/.cargo/config.toml` settings (like sccache) break in Docker.

**Solution:** Use project-local `.cargo/config.toml`, avoid global settings in CI.

### Pitfall: MinGW DLL Issues on Windows

**Problem:** Missing DLLs at runtime.

**Solution:** Ship DLLs with binary or use static linking where possible.

### Pitfall: glibc Version Mismatch

**Problem:** Binary requires newer glibc than target system has.

**Solution:** Use `cargo-zigbuild` with explicit glibc version or musl target.

```bash
cargo zigbuild --target x86_64-unknown-linux-gnu.2.17
```

---

## Checklist

### Project Setup

- [ ] Workspace structure with core library
- [ ] Platform-specific binding crates
- [ ] Feature flags for optional capabilities
- [ ] Shared dependencies in workspace
- [ ] `rust-toolchain.toml` with all targets listed

### Code Organization

- [ ] Core logic uses traits for platform abstraction
- [ ] No `std` dependency in core (use `alloc` if needed)
- [ ] Errors implement platform-specific conversions
- [ ] Serialization is deterministic and portable
- [ ] `build.rs` uses modern `cargo::` syntax

### Build & Test

- [ ] CI tests all target platforms
- [ ] Platform-specific tests exist
- [ ] Release builds are optimized
- [ ] Documentation covers platform differences
- [ ] cross-rs or cargo-zigbuild configured for cross-compilation

### Mobile

- [ ] UniFFI bindings for Swift/Kotlin
- [ ] cargo-ndk for Android builds
- [ ] XCFramework generation for iOS

---

*Cross-platform Rust enables writing high-performance code once and deploying everywhere.*
