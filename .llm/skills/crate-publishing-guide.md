# Crate Publishing Guide — Publishing Rust Libraries to crates.io

> **This document provides comprehensive guidance for publishing high-quality Rust crates.**
> Follow these practices to create maintainable, well-documented, and user-friendly libraries.

## TL;DR — Quick Reference

```bash
# Pre-publish validation
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-features
cargo doc --no-deps --all-features
cargo package --list  # Review what will be published
cargo publish --dry-run

# Semantic versioning checks
cargo install cargo-semver-checks
cargo semver-checks check-release

# Security audit
cargo install cargo-audit
cargo audit

# Outdated dependencies
cargo install cargo-outdated
cargo outdated
```

**Key Principles:**

1. **Minimal API surface** — Expose only what users need
2. **Complete metadata** — Fill out all Cargo.toml fields
3. **Document everything** — Use `#![warn(missing_docs)]`
4. **Follow semver strictly** — Use cargo-semver-checks
5. **Limit dependencies** — Each one is a liability

---

## Cargo.toml Metadata — Complete Package Configuration

### Required Fields

Every published crate MUST have these fields:

```toml
[package]
name = "my-crate"
version = "0.1.0"
edition = "2021"
rust-version = "1.70"  # MSRV - Minimum Supported Rust Version
authors = ["Your Name <your.email@example.com>"]
description = "A clear, concise description of what the crate does."
license = "MIT OR Apache-2.0"
repository = "https://github.com/username/my-crate"
readme = "README.md"
keywords = ["keyword1", "keyword2", "keyword3"]  # Max 5
categories = ["category1", "category2"]  # See crates.io/category_slugs
```

### Recommended Optional Fields

```toml
[package]
# Documentation link (defaults to docs.rs)
documentation = "https://docs.rs/my-crate"

# Homepage (if different from repository)
homepage = "https://my-crate.dev"

# Exclude unnecessary files from package
exclude = [
    ".github/",
    ".devcontainer/",
    "scripts/",
    "tests/",           # Large test suites
    "benches/",         # Benchmarks
    "*.md",             # Keep README.md via readme field
    "!README.md",
    "!CHANGELOG.md",
    "!LICENSE*",
]

# Or use include for explicit allowlist
include = [
    "src/**/*",
    "Cargo.toml",
    "README.md",
    "LICENSE-MIT",
    "LICENSE-APACHE",
    "CHANGELOG.md",  # Include changelog in package
]
```

> **Note:** Cargo.toml does not have a `changelog` field. To reference your changelog:
>
> - Include `CHANGELOG.md` in your package (shown above in the `include` list)
> - Link to it from your README.md
> - Add a link in your crate-level documentation: `//! [Changelog](https://github.com/username/my-crate/blob/main/CHANGELOG.md)`
> - docs.rs will automatically render and link to `CHANGELOG.md` if it's included in the package

### Badge Configuration

```toml
[badges]
maintenance = { status = "actively-developed" }
# Other options: experimental, passively-maintained, as-is, deprecated, none
```

### Dependency Best Practices

```toml
[dependencies]
# ✅ GOOD - Use stable 1.0+ crates when available
serde = "1.0"

# ✅ GOOD - Disable unnecessary default features
tokio = { version = "1.0", default-features = false, features = ["rt", "net"] }

# ✅ GOOD - Re-export dependencies exposed in public API
# This prevents version conflicts for users
bytes = "1.0"  # Re-exported as `pub use bytes;`

# ⚠️  CAUTION - Pre-1.0 crates may have breaking changes
rand = "0.8"   # Acceptable if no stable alternative

# ❌ AVOID - Pinning exact versions (blocks updates)
some-crate = "=1.2.3"  # Don't do this

# ❌ AVOID - Git dependencies in published crates
# git-dep = { git = "https://github.com/..." }  # Use crates.io version
```

### Feature Flags

```toml
[features]
default = []  # Prefer minimal defaults

# Document each feature in README and lib.rs
std = []           # Enable std library support
alloc = []         # Enable alloc without full std
serde = ["dep:serde"]  # Optional serde support
async = ["dep:tokio"]  # Async runtime support

# Use dep: syntax to avoid implicit features (Rust 1.60+)
[dependencies]
serde = { version = "1.0", optional = true }
tokio = { version = "1.0", optional = true }
```

---

## Project Structure — Organizing Crate Code

### Library Crate Structure

```
my-crate/
├── Cargo.toml
├── README.md
├── LICENSE-MIT
├── LICENSE-APACHE
├── CHANGELOG.md
├── src/
│   ├── lib.rs          # Library entry point
│   ├── error.rs        # Error types
│   ├── config.rs       # Configuration
│   └── module/
│       ├── mod.rs      # Module with submodules
│       ├── types.rs
│       └── impl.rs
├── examples/
│   ├── basic.rs
│   └── advanced.rs
├── benches/
│   └── benchmark.rs
└── tests/
    └── integration/
        └── main.rs     # Single integration test crate
```

### Module Organization

```rust
// src/lib.rs

//! # My Crate
//!
//! A brief description of what this crate does.
//!
//! ## Features
//!
//! - `std` - Enable std library support (default)
//! - `serde` - Enable serde serialization
//!
//! ## Example
//!
//! ```rust
//! use my_crate::Widget;
//!
//! let widget = Widget::new();
//! widget.do_something();
//! ```

#![warn(missing_docs)]
#![warn(missing_debug_implementations)]
#![warn(rust_2018_idioms)]
#![warn(unreachable_pub)]
#![deny(unsafe_code)]  // Or #![forbid(unsafe_code)] if no unsafe needed

// Re-export main types at crate root for convenience
pub use self::config::Config;
pub use self::error::{Error, Result};
pub use self::widget::Widget;

mod config;
mod error;
mod widget;

// Internal modules use pub(crate)
pub(crate) mod internal;
```

### Visibility Guidelines

```rust
// ✅ GOOD - Minimal public API
pub struct Widget {
    // Private field - implementation detail
    inner: InnerState,
}

impl Widget {
    /// Creates a new widget with default settings.
    pub fn new() -> Self { /* ... */ }

    /// Internal helper - not part of public API
    pub(crate) fn internal_method(&self) { /* ... */ }

    // Private method
    fn helper(&self) { /* ... */ }
}

// ✅ GOOD - Re-export for convenience
pub mod prelude {
    pub use crate::{Widget, Config, Error, Result};
}

// ✅ GOOD - Use #[non_exhaustive] for extensible enums
#[non_exhaustive]
pub enum ErrorKind {
    InvalidInput,
    Timeout,
    // Future variants won't break downstream
}

// ✅ GOOD - Use newtype pattern to hide implementation
pub struct UserId(u64);  // Users can't depend on it being u64

impl UserId {
    pub fn new(id: u64) -> Self { Self(id) }
    pub fn as_u64(&self) -> u64 { self.0 }
}
```

---

## Documentation — Writing Excellent Docs

### Enable Documentation Lints

```rust
// src/lib.rs
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]
#![warn(rustdoc::broken_intra_doc_links)]
#![warn(rustdoc::private_intra_doc_links)]
```

### Documentation Structure

```rust
/// A widget for processing data.
///
/// `Widget` provides high-performance data processing with
/// configurable options for various use cases.
///
/// # Examples
///
/// Basic usage:
///
/// ```rust
/// use my_crate::Widget;
///
/// let widget = Widget::new();
/// let result = widget.process("input data")?;
/// # Ok::<(), my_crate::Error>(())
/// ```
///
/// With custom configuration:
///
/// ```rust
/// use my_crate::{Widget, Config};
///
/// let config = Config::builder()
///     .timeout(Duration::from_secs(30))
///     .build();
/// let widget = Widget::with_config(config);
/// ```
///
/// # Errors
///
/// Returns [`Error::InvalidInput`] if the input is empty.
/// Returns [`Error::Timeout`] if processing exceeds the configured timeout.
///
/// # Panics
///
/// This function does not panic. (Or document when it does)
///
/// # Safety
///
/// (Only for unsafe functions - document all invariants)
pub struct Widget { /* ... */ }
```

### README.md Template

````markdown
# my-crate

[![Crates.io](https://img.shields.io/crates/v/my-crate.svg)](https://crates.io/crates/my-crate)
[![Documentation](https://docs.rs/my-crate/badge.svg)](https://docs.rs/my-crate)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](#license)
[![CI](https://github.com/username/my-crate/workflows/CI/badge.svg)](https://github.com/username/my-crate/actions)

A brief description of what this crate does.

## Features

- Feature 1: Description
- Feature 2: Description
- Feature 3: Description

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
my-crate = "0.1"
```

## Quick Start

```rust
use my_crate::Widget;

fn main() -> Result<(), my_crate::Error> {
    let widget = Widget::new();
    widget.do_something()?;
    Ok(())
}
```

## Cargo Features

| Feature | Default | Description |
|---------|---------|-------------|
| `std`   | Yes     | Enable std library support |
| `serde` | No      | Enable serde serialization |
| `async` | No      | Enable async runtime support |

## Minimum Supported Rust Version (MSRV)

This crate requires Rust 1.70 or later.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.
````

---

## API Design — Building User-Friendly Interfaces

### Minimize Public API Surface

```rust
// ❌ AVOID - Exposing implementation details
pub struct Parser {
    pub buffer: Vec<u8>,        // Users shouldn't access this
    pub state: ParserState,     // Implementation detail
    pub position: usize,        // Internal bookkeeping
}

// ✅ GOOD - Hide implementation, expose behavior
pub struct Parser {
    buffer: Vec<u8>,
    state: ParserState,
    position: usize,
}

impl Parser {
    pub fn new() -> Self { /* ... */ }
    pub fn parse(&mut self, input: &[u8]) -> Result<Ast, ParseError> { /* ... */ }
    pub fn is_complete(&self) -> bool { /* ... */ }
}
```

### Use Builder Pattern for Complex Configuration

```rust
/// Configuration for the widget.
#[derive(Debug, Clone)]
pub struct Config {
    timeout: Duration,
    max_retries: u32,
    buffer_size: usize,
}

impl Config {
    /// Creates a new configuration builder.
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }
}

/// Builder for [`Config`].
#[derive(Debug, Default)]
pub struct ConfigBuilder {
    timeout: Option<Duration>,
    max_retries: Option<u32>,
    buffer_size: Option<usize>,
}

impl ConfigBuilder {
    /// Sets the operation timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sets the maximum retry count.
    pub fn max_retries(mut self, retries: u32) -> Self {
        self.max_retries = Some(retries);
        self
    }

    /// Builds the configuration.
    pub fn build(self) -> Config {
        Config {
            timeout: self.timeout.unwrap_or(Duration::from_secs(30)),
            max_retries: self.max_retries.unwrap_or(3),
            buffer_size: self.buffer_size.unwrap_or(4096),
        }
    }
}
```

### Re-export Dependencies in Public API

```rust
// If your public API exposes types from dependencies,
// re-export them so users don't need to add the dependency

// src/lib.rs
pub use bytes::Bytes;  // Users can use my_crate::Bytes

// This prevents version conflicts:
// - User adds bytes = "1.5"
// - Your crate uses bytes = "1.4"
// - Without re-export: conflict!
// - With re-export: user uses your version
```

### Design for Extension with `#[non_exhaustive]`

```rust
/// Error type for widget operations.
#[derive(Debug)]
#[non_exhaustive]  // Allows adding variants without breaking change
pub enum Error {
    /// Invalid input was provided.
    InvalidInput { message: String },

    /// Operation timed out.
    Timeout { elapsed: Duration },

    /// Network error occurred.
    Network(std::io::Error),
}

/// Configuration options.
#[derive(Debug, Clone)]
#[non_exhaustive]  // Allows adding fields without breaking change
pub struct Options {
    pub timeout: Duration,
    pub retries: u32,
    // Future fields won't break users
}

impl Default for Options {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            retries: 3,
        }
    }
}
```

---

## Versioning — Semantic Versioning for Rust

### Semver Rules

| Change Type | Version Bump | Examples |
|-------------|--------------|----------|
| Breaking API change | Major (1.0.0 → 2.0.0) | Remove public item, change function signature |
| New feature, backward compatible | Minor (1.0.0 → 1.1.0) | Add new public function, add optional feature |
| Bug fix, no API change | Patch (1.0.0 → 1.0.1) | Fix bug, improve performance |

### Pre-1.0 Versioning

For crates before 1.0.0:

- **0.x.y** — Minor version bumps (0.1.0 → 0.2.0) can be breaking
- **0.x.y** — Patch version bumps (0.1.0 → 0.1.1) should be non-breaking

### Use cargo-semver-checks

```bash
# Install
cargo install cargo-semver-checks

# Check for breaking changes before release
cargo semver-checks check-release

# Compare against specific version
cargo semver-checks check-release --baseline-version 1.0.0
```

### CHANGELOG Best Practices

```markdown
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- New `Widget::with_config` constructor

### Changed
- Improved error messages for `ParseError`

### Deprecated
- `Widget::new_with_options` - use `Widget::with_config` instead

### Removed
- (Nothing)

### Fixed
- Fixed panic when input is empty

### Security
- Updated `vulnerable-dep` to fix CVE-XXXX-YYYY

## [1.0.0] - 2024-01-15

### Added
- Initial stable release
```

---

## Testing — Comprehensive Test Coverage

### Doctest Best Practices

```rust
/// Parses the input string.
///
/// # Examples
///
/// ```rust
/// use my_crate::parse;
///
/// let result = parse("hello")?;
/// assert_eq!(result.len(), 5);
/// # Ok::<(), my_crate::Error>(())
/// ```
///
/// Error handling:
///
/// ```rust
/// use my_crate::parse;
///
/// let result = parse("");
/// assert!(result.is_err());
/// ```
///
/// This example should compile but not run:
///
/// ```rust,no_run
/// # use my_crate::connect;
/// let conn = connect("server:1234")?;
/// # Ok::<(), my_crate::Error>(())
/// ```
///
/// This example should fail to compile:
///
/// ```rust,compile_fail
/// use my_crate::PrivateType;  // Error: not accessible
/// ```
pub fn parse(input: &str) -> Result<Parsed, Error> { /* ... */ }
```

### Feature-Gated Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_test() {
        // Always runs
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_roundtrip() {
        // Only runs with --features serde
    }

    #[test]
    #[cfg(feature = "std")]
    fn std_io_test() {
        // Only runs with std feature
    }
}
```

---

## CI/CD — Automated Quality Checks

### GitHub Actions Workflow

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2

      - name: Check formatting
        run: cargo fmt --check

      - name: Clippy
        run: cargo clippy --all-targets --all-features -- -D warnings

      - name: Run tests
        run: cargo test --all-features

      - name: Run doc tests
        run: cargo test --doc --all-features

      - name: Build docs
        run: cargo doc --no-deps --all-features
        env:
          RUSTDOCFLAGS: -D warnings

  msrv:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install MSRV Rust
        uses: dtolnay/rust-toolchain@master
        with:
          toolchain: "1.70"  # Match rust-version in Cargo.toml

      - name: Check MSRV
        run: cargo check --all-features

  semver:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Check semver
        uses: obi1kenobi/cargo-semver-checks-action@v2

  security:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Security audit
        uses: rustsec/audit-check@v2
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
```

### Pre-publish Checklist Workflow

```yaml
name: Publish

on:
  push:
    tags:
      - 'v*'

jobs:
  publish:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Verify version matches tag
        run: |
          CARGO_VERSION=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version')
          TAG_VERSION=${GITHUB_REF#refs/tags/v}
          if [ "$CARGO_VERSION" != "$TAG_VERSION" ]; then
            echo "Version mismatch: Cargo.toml=$CARGO_VERSION, tag=$TAG_VERSION"
            exit 1
          fi

      - name: Dry run
        run: cargo publish --dry-run

      - name: Publish to crates.io
        run: cargo publish
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
```

---

## Dependencies — Managing External Code

### Dependency Evaluation Checklist

Before adding a dependency, evaluate:

- [ ] **Necessity** — Can you implement this yourself in <100 lines?
- [ ] **Popularity** — Downloads, stars, dependents on crates.io
- [ ] **Maintenance** — Recent commits, responsive maintainers
- [ ] **License** — Compatible with your license (MIT/Apache-2.0)
- [ ] **Security** — Check RustSec advisories, run `cargo audit`
- [ ] **Stability** — Is it 1.0+? How often do they break semver?
- [ ] **Dependencies** — Does it pull in a huge transitive tree?
- [ ] **Features** — Can you disable features you don't need?

### Dependency Maintenance

```bash
# Check for outdated dependencies
cargo install cargo-outdated
cargo outdated

# Check for security vulnerabilities
cargo install cargo-audit
cargo audit

# Check for unmaintained/yanked crates
cargo install cargo-deny
cargo deny check

# Update dependencies
cargo update

# Update specific dependency
cargo update -p dependency-name
```

### Automated Dependency Updates

Set up Dependabot or Renovate:

```yaml
# .github/dependabot.yml
version: 2
updates:
  - package-ecosystem: "cargo"
    directory: "/"
    schedule:
      interval: "weekly"
    open-pull-requests-limit: 10
    groups:
      minor-and-patch:
        update-types:
          - "minor"
          - "patch"
```

---

## Security — Safe Publishing Practices

### Security Checklist

- [ ] Run `cargo audit` before every release
- [ ] Enable Dependabot/Renovate for automatic updates
- [ ] Review all dependencies for security advisories
- [ ] Use `#![forbid(unsafe_code)]` if possible
- [ ] Document all `unsafe` code with `// SAFETY:` comments
- [ ] Run Miri in CI for unsafe code: `cargo +nightly miri test`
- [ ] Consider fuzzing with cargo-fuzz for parsing/input handling

### Unsafe Code Documentation

```rust
// If you must use unsafe, document thoroughly:

/// # Safety
///
/// The caller must ensure:
/// - `ptr` is valid for reads of `len` bytes
/// - `ptr` is properly aligned for `T`
/// - The memory is initialized
pub unsafe fn read_bytes<T>(ptr: *const T, len: usize) -> &[u8] {
    // SAFETY: Caller guarantees ptr validity, alignment, and initialization.
    // We only read `len` bytes which caller has verified is within bounds.
    std::slice::from_raw_parts(ptr as *const u8, len)
}
```

---

## Publishing Checklist

### Before First Publish

- [ ] Choose a unique, descriptive crate name
- [ ] Create crates.io account and get API token
- [ ] Fill out all Cargo.toml metadata
- [ ] Write comprehensive README.md
- [ ] Add LICENSE-MIT and LICENSE-APACHE files
- [ ] Set up CI/CD pipeline
- [ ] Run `cargo publish --dry-run`
- [ ] Review package contents with `cargo package --list`

### Before Every Release

```bash
# 1. Update version in Cargo.toml
# 2. Update CHANGELOG.md

# 3. Run full validation
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo test --doc --all-features
cargo doc --no-deps --all-features

# 4. Check for breaking changes
cargo semver-checks check-release

# 5. Security audit
cargo audit

# 6. Verify package contents
cargo package --list
cargo publish --dry-run

# 7. Create git tag
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin v1.0.0

# 8. Publish
cargo publish
```

### After Publishing

- [ ] Verify crate appears on crates.io
- [ ] Verify docs render correctly on docs.rs
- [ ] Test installation: `cargo add your-crate`
- [ ] Announce release (GitHub release, social media, etc.)

---

## Common Mistakes to Avoid

### ❌ Exposing Too Much API

```rust
// ❌ BAD - Exposes internal module structure
pub mod internal;
pub mod helpers;
pub mod utils;

// ✅ GOOD - Flat, minimal API
pub use crate::widget::Widget;
pub use crate::error::Error;
```

### ❌ Forgetting `#[non_exhaustive]`

```rust
// ❌ BAD - Adding variants is breaking change
pub enum Error {
    Io(std::io::Error),
    Parse(String),
}

// ✅ GOOD - Future-proof
#[non_exhaustive]
pub enum Error {
    Io(std::io::Error),
    Parse(String),
}
```

### ❌ Not Re-exporting Dependency Types

```rust
// ❌ BAD - Users must add compatible bytes version
pub fn get_data(&self) -> bytes::Bytes { /* ... */ }

// ✅ GOOD - Re-export dependency
pub use bytes::Bytes;
pub fn get_data(&self) -> Bytes { /* ... */ }
```

### ❌ Pinning Exact Dependency Versions

```toml
# ❌ BAD - Blocks users from updating
serde = "=1.0.150"

# ✅ GOOD - Allows compatible updates
serde = "1.0"
```

### ❌ Publishing Without Dry Run

```bash
# ❌ BAD - Might include unwanted files
cargo publish

# ✅ GOOD - Always verify first
cargo package --list
cargo publish --dry-run
cargo publish
```

---

## Related Resources

- [The Cargo Book — Publishing](https://doc.rust-lang.org/cargo/reference/publishing.html)
- [API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Semver Specification](https://semver.org/)
- [Keep a Changelog](https://keepachangelog.com/)
- [RustSec Advisory Database](https://rustsec.org/)
- [cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks)
