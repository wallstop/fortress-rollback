<!-- CATEGORY: Publishing & Organization -->
<!-- WHEN: Publishing to crates.io, version bumps, release checklist -->

# Crate Publishing

---

## Pre-Publish Validation

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo doc --no-deps --all-features
cargo package --list      # Review what will be published
cargo publish --dry-run

# Semver check
cargo semver-checks check-release

# Security
cargo audit
```

---

## Cargo.toml Metadata

### Required Fields

```toml
[package]
name = "my-crate"
version = "0.1.0"
edition = "2021"
rust-version = "1.70"
authors = ["Name <email@example.com>"]
description = "A clear, concise description."
license = "MIT OR Apache-2.0"
repository = "https://github.com/username/my-crate"
readme = "README.md"
keywords = ["keyword1", "keyword2"]  # Max 5
categories = ["category1"]

exclude = [".github/", ".devcontainer/", "scripts/", "benches/"]
# Or use include for explicit allowlist
```

### Feature Flags

```toml
[features]
default = []
std = []
serde = ["dep:serde"]   # Use dep: syntax (Rust 1.60+)

[dependencies]
serde = { version = "1.0", optional = true }
```

### Dependency Best Practices

```toml
[dependencies]
serde = "1.0"                           # Use semver ranges
tokio = { version = "1.0", default-features = false, features = ["rt"] }
# Re-export deps exposed in public API
bytes = "1.0"  # pub use bytes::Bytes;

# AVOID: exact pins, git deps in published crates
```

---

## Semver Rules

| Change | Bump | Example |
|--------|------|---------|
| Breaking API change | Major (1.0 -> 2.0) | Remove pub item, change signature |
| New feature, backward compat | Minor (1.0 -> 1.1) | Add pub function |
| Bug fix, no API change | Patch (1.0.0 -> 1.0.1) | Fix bug |

Pre-1.0: minor bumps (0.1 -> 0.2) can be breaking.

```bash
cargo semver-checks check-release
cargo semver-checks check-release --baseline-version 1.0.0
```

---

## API Design Essentials

```rust
// Minimal public API -- hide internals
pub struct Widget { inner: InnerState }

// Use #[non_exhaustive] for extensible enums/structs
#[non_exhaustive]
pub enum Error {
    InvalidInput { message: String },
    Timeout { elapsed: Duration },
}

// Re-export dependency types exposed in API
pub use bytes::Bytes;

// Newtype pattern for type safety
pub struct UserId(u64);
impl UserId {
    pub fn new(id: u64) -> Self { Self(id) }
    pub fn as_u64(&self) -> u64 { self.0 }
}
```

### Documentation Lints

```rust
#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]
#![deny(unsafe_code)]
```

---

## Project Structure

```
my-crate/
├── Cargo.toml
├── README.md
├── LICENSE-MIT
├── LICENSE-APACHE
├── CHANGELOG.md
├── src/
│   ├── lib.rs       # Public API, re-exports
│   ├── error.rs
│   └── module/
├── examples/
├── benches/
└── tests/
    └── integration/
        └── main.rs  # Single integration test crate
```

---

## Release Workflow

```bash
# 1. Update version in Cargo.toml
# 2. Update CHANGELOG.md

# 3. Full validation
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo doc --no-deps --all-features

# 4. Check for breaking changes
cargo semver-checks check-release

# 5. Security audit
cargo audit

# 6. Verify package
cargo package --list
cargo publish --dry-run

# 7. Tag and publish
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin v1.0.0
cargo publish
```

### Post-Publish

- [ ] Verify on crates.io
- [ ] Verify docs on docs.rs
- [ ] Test: `cargo add your-crate`
- [ ] Create GitHub release

---

## CI Workflow

```yaml
name: CI
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { components: "rustfmt, clippy" }
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --check
      - run: cargo clippy --all-targets --all-features -- -D warnings
      - run: cargo test --all-features
      - run: cargo doc --no-deps --all-features
        env: { RUSTDOCFLAGS: "-D warnings" }

  msrv:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@master
        with: { toolchain: "1.70" }
      - run: cargo check --all-features

  semver:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: obi1kenobi/cargo-semver-checks-action@v2
```

### Publish Workflow (Tag-Triggered)

```yaml
name: Publish
on:
  push:
    tags: ['v*']

jobs:
  publish:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo publish --dry-run
      - run: cargo publish
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
```

---

## CHANGELOG Format

```markdown
# Changelog

## [Unreleased]

### Added
### Changed
### Deprecated
### Removed
### Fixed
### Security
```

Follow [Keep a Changelog](https://keepachangelog.com/) format.

---

## Common Mistakes

| Mistake | Fix |
|---------|-----|
| Exposing internal modules | Re-export types at crate root |
| Missing `#[non_exhaustive]` | Adding variants becomes breaking |
| Not re-exporting dep types | Users get version conflicts |
| Pinning exact dep versions | Blocks compatible updates |
| Publishing without dry-run | May include unwanted files |

---

## Automated Dependency Updates

```yaml
# .github/dependabot.yml
version: 2
updates:
  - package-ecosystem: "cargo"
    directory: "/"
    schedule: { interval: "weekly" }
    groups:
      minor-and-patch:
        update-types: ["minor", "patch"]
```
