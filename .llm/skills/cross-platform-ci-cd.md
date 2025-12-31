# Cross-Platform CI/CD for Rust Projects

> **A guide to setting up continuous integration and deployment for Rust projects targeting multiple platforms.**

## Overview

Cross-platform CI/CD for Rust projects requires careful orchestration of builds across different operating systems, architectures, and target environments. This guide covers GitHub Actions patterns, but the concepts apply to other CI systems.

---

## Core Principle: Test Everything Cross-Platform

**All builds and tests should run on a cross-platform matrix whenever possible.** Platform-specific bugs (memory layout differences, threading behavior, endianness, OS-specific syscalls) can cause production failures that are invisible when testing on only one platform.

### What Should Run Cross-Platform

| Category | Cross-Platform Priority | Rationale |
|----------|------------------------|-----------|
| **Unit tests** | Required | Catch platform-specific logic bugs |
| **Integration tests** | Required | OS-specific behavior differences |
| **Loom concurrency tests** | Required | Threading/scheduler behavior varies |
| **Miri UB checks** | Required | Memory layout, alignment differ by platform |
| **Clippy/fmt** | One platform OK | Code is platform-agnostic |
| **Coverage** | One platform OK | Measures same code paths |
| **Security scanning** | One platform OK | Dependency analysis is platform-agnostic |
| **Formal verification (Kani)** | One platform OK | Proofs are platform-agnostic (Linux-only tool) |

### Standard Cross-Platform Matrix

```yaml
strategy:
  fail-fast: false  # Run all platforms even if one fails
  matrix:
    os: [ubuntu-latest, windows-latest, macos-latest]
```

### Why `fail-fast: false`?

Setting `fail-fast: false` ensures all platforms run to completion. This is critical because:

1. A Linux-only failure might mask a different Windows-only failure
2. Developers can fix multiple platform issues in one PR cycle
3. Provides complete visibility into cross-platform health

---

## Target Platform Matrix

### Common Rust Targets

| Target Triple | Platform | Tier | CI Runner |
|---------------|----------|------|-----------|
| `x86_64-unknown-linux-gnu` | Linux x64 (glibc) | 1 | `ubuntu-latest` |
| `x86_64-unknown-linux-musl` | Linux x64 (static) | 2 | `ubuntu-latest` + cross |
| `aarch64-unknown-linux-gnu` | Linux ARM64 | 2 | `ubuntu-latest` + cross |
| `x86_64-apple-darwin` | macOS Intel | 1 | `macos-latest` |
| `aarch64-apple-darwin` | macOS Apple Silicon | 1 | `macos-latest` |
| `x86_64-pc-windows-msvc` | Windows x64 | 1 | `windows-latest` |
| `x86_64-pc-windows-gnu` | Windows (MinGW) | 1 | `ubuntu-latest` + cross |
| `wasm32-unknown-unknown` | WebAssembly | 2 | `ubuntu-latest` |
| `aarch64-apple-ios` | iOS Device | 2 | `macos-latest` |
| `aarch64-linux-android` | Android ARM64 | 2 | `ubuntu-latest` + cross |

### Platform Tiers

- **Tier 1**: Guaranteed to work, full test suite runs
- **Tier 2**: Builds guaranteed, limited testing
- **Tier 3**: Best-effort support

---

## Basic Multi-Platform Workflow

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  # Fast checks on Linux first
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt

      - name: Format check
        run: cargo fmt --all -- --check

      - name: Clippy
        run: cargo clippy --all-targets -- -D warnings

      - name: Test
        run: cargo test

  # Multi-platform build matrix
  build:
    needs: check  # Don't waste runner time if checks fail
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: macos-latest
            target: x86_64-apple-darwin
          - os: macos-latest
            target: aarch64-apple-darwin
          - os: windows-latest
            target: x86_64-pc-windows-msvc

    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Build
        run: cargo build --release --target ${{ matrix.target }}

      - name: Test
        run: cargo test --target ${{ matrix.target }}
```

---

## Cross-Compilation Patterns

### Using cross-rs for Linux Targets

```yaml
jobs:
  cross-compile:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        target:
          - aarch64-unknown-linux-gnu
          - armv7-unknown-linux-gnueabihf
          - x86_64-unknown-linux-musl

    steps:
      - uses: actions/checkout@v4

      - name: Install cross
        run: |
          curl -L --proto '=https' --tlsv1.2 -sSf \
            https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
          cargo binstall cross --no-confirm

      - name: Build with cross
        run: cross build --release --target ${{ matrix.target }}

      - name: Test with cross (QEMU)
        run: cross test --target ${{ matrix.target }}
```

### Using cargo-zigbuild for glibc Targeting

```yaml
jobs:
  zigbuild:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: x86_64-unknown-linux-gnu

      - name: Install Zig
        uses: goto-bus-stop/setup-zig@v2

      - name: Install cargo-zigbuild
        run: cargo install --locked cargo-zigbuild

      - name: Build targeting glibc 2.17
        run: cargo zigbuild --release --target x86_64-unknown-linux-gnu.2.17
```

---

## WASM Build and Test

```yaml
jobs:
  wasm:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown

      - name: Install wasm-pack
        run: cargo install wasm-pack

      - name: Build WASM
        run: cargo build --target wasm32-unknown-unknown --release

      - name: Build with wasm-pack (for web)
        run: wasm-pack build --target web --release

      - name: Test in headless browser
        run: wasm-pack test --headless --chrome
```

### WASM with wasm-bindgen-test

```yaml
jobs:
  wasm-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown

      - name: Install wasm-pack
        run: cargo install wasm-pack

      - name: Setup Chrome
        uses: browser-actions/setup-chrome@latest

      - name: Run WASM tests
        run: |
          RUSTFLAGS='--cfg getrandom_backend="wasm_js"' \
          wasm-pack test --headless --chrome
```

---

## Mobile Build Workflows

### iOS Build (macOS Runner Required)

```yaml
jobs:
  ios:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-apple-ios, aarch64-apple-ios-sim

      - name: Build for iOS device
        run: cargo build --target aarch64-apple-ios --release

      - name: Build for iOS simulator
        run: cargo build --target aarch64-apple-ios-sim --release
```

### Android Build with cargo-ndk

```yaml
jobs:
  android:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-linux-android, armv7-linux-androideabi

      - name: Setup Android NDK
        uses: nttld/setup-ndk@v1
        with:
          ndk-version: r25c

      - name: Install cargo-ndk
        run: cargo install cargo-ndk

      - name: Build for Android
        run: |
          cargo ndk -t arm64-v8a -t armeabi-v7a \
            -o ./jniLibs build --release

      - name: Upload Android libraries
        uses: actions/upload-artifact@v4
        with:
          name: android-libs
          path: jniLibs/
```

---

## Release Workflow

### Automated Release on Tag

```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags:
      - 'v*'

permissions:
  contents: write

jobs:
  build-release:
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            artifact: my-app-linux-x64
          - os: ubuntu-latest
            target: x86_64-unknown-linux-musl
            artifact: my-app-linux-x64-static
            use_cross: true
          - os: macos-latest
            target: x86_64-apple-darwin
            artifact: my-app-macos-intel
          - os: macos-latest
            target: aarch64-apple-darwin
            artifact: my-app-macos-arm64
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            artifact: my-app-windows-x64

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Install cross
        if: matrix.use_cross
        run: cargo install cross --git https://github.com/cross-rs/cross

      - name: Build (native)
        if: ${{ !matrix.use_cross }}
        run: cargo build --release --target ${{ matrix.target }}

      - name: Build (cross)
        if: matrix.use_cross
        run: cross build --release --target ${{ matrix.target }}

      - name: Package (Unix)
        if: matrix.os != 'windows-latest'
        run: |
          cd target/${{ matrix.target }}/release
          tar czf ../../../${{ matrix.artifact }}.tar.gz my-app

      - name: Package (Windows)
        if: matrix.os == 'windows-latest'
        run: |
          cd target/${{ matrix.target }}/release
          7z a ../../../${{ matrix.artifact }}.zip my-app.exe

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.artifact }}
          path: ${{ matrix.artifact }}.*

  create-release:
    needs: build-release
    runs-on: ubuntu-latest

    steps:
      - name: Download all artifacts
        uses: actions/download-artifact@v4
        with:
          path: artifacts

      - name: Create release
        uses: softprops/action-gh-release@v1
        with:
          files: artifacts/**/*
          generate_release_notes: true
```

---

## Caching Strategies

### Basic Cargo Caching

```yaml
steps:
  - uses: actions/checkout@v4

  - uses: dtolnay/rust-toolchain@stable

  - name: Cache cargo registry
    uses: actions/cache@v4
    with:
      path: |
        ~/.cargo/registry
        ~/.cargo/git
        target
      key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
      restore-keys: |
        ${{ runner.os }}-cargo-
```

### Separate Caches for Better Hit Rates

```yaml
steps:
  - name: Cache cargo registry
    uses: actions/cache@v4
    with:
      path: ~/.cargo/registry
      key: cargo-registry-${{ hashFiles('**/Cargo.lock') }}

  - name: Cache cargo index
    uses: actions/cache@v4
    with:
      path: ~/.cargo/git
      key: cargo-git-${{ hashFiles('**/Cargo.lock') }}

  - name: Cache target directory
    uses: actions/cache@v4
    with:
      path: target
      key: target-${{ runner.os }}-${{ matrix.target }}-${{ hashFiles('**/Cargo.lock') }}
      restore-keys: |
        target-${{ runner.os }}-${{ matrix.target }}-
```

### Using sccache for Distributed Caching

```yaml
env:
  SCCACHE_GHA_ENABLED: "true"
  RUSTC_WRAPPER: "sccache"

steps:
  - uses: actions/checkout@v4

  - name: Setup sccache
    uses: mozilla-actions/sccache-action@v0.0.4

  - uses: dtolnay/rust-toolchain@stable

  - name: Build
    run: cargo build --release
```

---

## Cost Optimization

### Strategy: Cross-compile on Cheap Runners

macOS and Windows runners cost 10x and 2x more than Linux. Minimize their usage:

```yaml
jobs:
  # All cross-compilation on cheap Linux runners
  linux-builds:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        target:
          - x86_64-unknown-linux-gnu
          - x86_64-unknown-linux-musl
          - aarch64-unknown-linux-gnu
          - x86_64-pc-windows-gnu  # MinGW cross-compile
    steps:
      - uses: actions/checkout@v4
      - run: cargo install cross --git https://github.com/cross-rs/cross
      - run: cross build --release --target ${{ matrix.target }}

  # Native macOS only for final verification
  macos-verify:
    runs-on: macos-latest
    needs: linux-builds
    if: github.event_name == 'push' && github.ref == 'refs/heads/main'
    steps:
      - uses: actions/checkout@v4
      - run: cargo test

  # Native Windows only for MSVC builds
  windows-msvc:
    runs-on: windows-latest
    needs: linux-builds
    steps:
      - uses: actions/checkout@v4
      - run: cargo build --release
```

### Run Expensive Jobs Only on Main/Tags

```yaml
jobs:
  full-matrix:
    if: github.event_name == 'push' && (github.ref == 'refs/heads/main' || startsWith(github.ref, 'refs/tags/'))
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - run: cargo test

  quick-check:
    if: github.event_name == 'pull_request'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo fmt --check && cargo clippy && cargo test
```

---

## Dependency and Security Scanning

### cargo-deny for License and Security

```yaml
jobs:
  deny:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: EmbarkStudios/cargo-deny-action@v1
```

### cargo-audit for Vulnerabilities

```yaml
jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: rustsec/audit-check@v1
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
```

---

## Platform-Specific Dependencies

### Linux Build Dependencies

```yaml
steps:
  - name: Install Linux dependencies
    run: |
      sudo apt-get update
      sudo apt-get install -y \
        libasound2-dev \
        libudev-dev \
        libwayland-dev \
        libxkbcommon-dev \
        pkg-config
```

### macOS Code Signing

```yaml
steps:
  - name: Import signing certificate
    if: matrix.os == 'macos-latest'
    env:
      MACOS_CERTIFICATE: ${{ secrets.MACOS_CERTIFICATE }}
      MACOS_CERTIFICATE_PWD: ${{ secrets.MACOS_CERTIFICATE_PWD }}
    run: |
      echo "$MACOS_CERTIFICATE" | base64 --decode > certificate.p12
      security create-keychain -p temp build.keychain
      security default-keychain -s build.keychain
      security unlock-keychain -p temp build.keychain
      security import certificate.p12 -k build.keychain \
        -P "$MACOS_CERTIFICATE_PWD" -T /usr/bin/codesign
      security set-key-partition-list -S apple-tool:,apple: \
        -s -k temp build.keychain

  - name: Sign binary
    if: matrix.os == 'macos-latest'
    run: codesign --force --sign "$SIGNING_IDENTITY" target/release/my-app
```

---

## Documentation and Coverage

### Generate and Deploy Docs

```yaml
jobs:
  docs:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@nightly

      - name: Build docs
        run: RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc --all-features --no-deps

      - name: Deploy to GitHub Pages
        uses: peaceiris/actions-gh-pages@v3
        if: github.ref == 'refs/heads/main'
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: ./target/doc
```

### Code Coverage with cargo-llvm-cov

```yaml
jobs:
  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: llvm-tools-preview

      - name: Install cargo-llvm-cov
        uses: taiki-e/install-action@cargo-llvm-cov

      - name: Generate coverage
        run: cargo llvm-cov --all-features --lcov --output-path lcov.info

      - name: Upload to Codecov
        uses: codecov/codecov-action@v3
        with:
          files: lcov.info
```

---

## Complete Example Workflow

```yaml
# .github/workflows/complete-ci.yml
name: Complete CI

on:
  push:
    branches: [main]
    tags: ['v*']
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  # Stage 1: Quick checks (always run)
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - run: cargo fmt --check
      - run: cargo clippy --all-targets -- -D warnings
      - run: cargo test

  # Stage 2: Security checks
  security:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: EmbarkStudios/cargo-deny-action@v1

  # Stage 3: Multi-platform builds (after checks pass)
  build:
    needs: [check, security]
    strategy:
      fail-fast: false
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
      - uses: actions/upload-artifact@v4
        with:
          name: build-${{ matrix.target }}
          path: target/${{ matrix.target }}/release/

  # Stage 4: Cross-compiled builds (Linux runner)
  cross-build:
    needs: check
    runs-on: ubuntu-latest
    strategy:
      matrix:
        target: [aarch64-unknown-linux-gnu, armv7-unknown-linux-gnueabihf]
    steps:
      - uses: actions/checkout@v4
      - run: cargo install cross --git https://github.com/cross-rs/cross
      - run: cross build --release --target ${{ matrix.target }}

  # Stage 5: Release (only on tags)
  release:
    if: startsWith(github.ref, 'refs/tags/')
    needs: [build, cross-build]
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/download-artifact@v4
        with:
          path: artifacts
      - uses: softprops/action-gh-release@v1
        with:
          files: artifacts/**/*
          generate_release_notes: true
```

---

## Checklist

### Basic CI

- [ ] Format check (`cargo fmt --check`)
- [ ] Lint check (`cargo clippy -- -D warnings`)
- [ ] Unit tests (`cargo test`)
- [ ] Build verification for all targets

### Security

- [ ] cargo-deny for licenses
- [ ] cargo-audit for vulnerabilities
- [ ] Dependabot or Renovate for updates

### Multi-Platform

- [ ] Linux (glibc and musl)
- [ ] macOS (Intel and Apple Silicon)
- [ ] Windows (MSVC)
- [ ] WASM (if applicable)
- [ ] Mobile targets (if applicable)

### Optimization

- [ ] Cargo caching enabled
- [ ] Cross-compilation on Linux where possible
- [ ] Expensive jobs gated to main/tags
- [ ] Matrix fail-fast disabled for visibility

### Release

- [ ] Automated release on tag
- [ ] Artifacts for all platforms
- [ ] Release notes generation
- [ ] Code signing (if required)

---

*Well-configured CI/CD is essential for maintaining cross-platform Rust projects.*
