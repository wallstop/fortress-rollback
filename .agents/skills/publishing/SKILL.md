---
name: publishing
description: "Crate Publishing guidance for Fortress Rollback. Use when Publishing to crates.io, version bumps, release checklist."
---

# Crate Publishing

---

## Pre-Publish Validation

```bash
python3 scripts/release/workspace_locks.py check
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo doc --no-deps --all-features
cargo package --list      # Review what will be published
cargo publish --dry-run --locked

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

Fortress Rollback has one reviewed release path:

1. Dispatch **Release - Prepare PR** from `main`. Use its dry-run input first
   when inspecting the generated manifest, changelog, and every authoritative
   workspace lock.
2. Review the generated PR and require green CI before merge. Release
   preparation discovers workspace roots dynamically and synchronizes locks
   with Cargo; do not hand-edit locks.
3. Merge the prepared PR to `main`.
4. Dispatch **Release - Publish Crate** from `main` with the exact committed
   version. This is the sole manual publication entrypoint. It checks locks,
   packages with `--locked`, publishes, tags the merged commit, and creates or
   updates the GitHub release.

Never publish from a generated release branch, push a release tag manually, or
add another workflow trigger that can race publication. If a generator or
release-workflow defect is repaired on a generated release branch, land the
same fix on `main`; closing a release PR must not strand generator fixes.

The canonical lock commands are:

```bash
python3 scripts/release/workspace_locks.py list
python3 scripts/release/workspace_locks.py sync
python3 scripts/release/workspace_locks.py check
```

Do not bypass a failing `--locked` command. Full `cargo metadata --locked
--all-features` resolution is the freshness oracle; `--no-deps` is forbidden
for lock validation because it can pass with stale standalone locks.

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

### Publication Workflow Contract

`.github/workflows/publish.yml` must remain `workflow_dispatch`-only. It is the
sole manual publication entrypoint and must run
`scripts/release/workspace_locks.py check` before packaging or publication.

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
