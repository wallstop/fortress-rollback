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
   workspace lock. Dry-run mutates only the ephemeral runner checkout and runs
   the same offline issue metadata generation, prepared-state verification,
   package validation, and complete diff as a real preparation; it suppresses
   only branch and pull-request writes.
2. Review the generated PR and require green CI before merge. Release
   preparation discovers workspace roots dynamically and synchronizes locks
   with Cargo; do not hand-edit locks. The PR also finalizes its release date,
   issue-template version, semantic-version classification, and prepared-source
   digest so the reviewed tree is the tree that will be tagged and published.
   Protect `main` with the repository ruleset in
   `.github/rulesets/main-protection.json`. Its stable **Verify prepared release
   state** check must remain required with strict up-to-date checking enabled.
   This is the supported policy for this repository; do not require or recommend
   a merge queue. The workflow's `merge_group` support is forward-compatible
   defense in depth, not an enforcement dependency. A release PR combined with
   another change intentionally fails and must be regenerated from current
   `main`.
3. Merge the prepared PR to `main`.
4. Dispatch **Release - Publish Crate** from `main` with the exact committed
   version. This is the sole manual publication entrypoint. It checks locks,
   resolves or atomically creates the annotated release tag as an immutable
   checkpoint after a successful package/checksum preflight, publishes, and
   creates or updates the GitHub release. A retry always checks out the requested
   tag before verification and packaging, even when `main` has advanced.

Never publish from a generated release branch, push a release tag manually, or
add another workflow trigger that can race publication. If a generator or
release-workflow defect is repaired on a generated release branch, land the
same fix on `main`; closing a release PR must not strand generator fixes.

Preparation reruns reuse only a release branch whose single preparation commit
is based on the dispatched `main` commit and whose immutable release state
matches the requested previous version, target version, and bump. The committed
release date wins across date changes. A matching open pull request is reused;
a missing pull request is created, while conflicting branch or pull-request
state fails closed. Every `release/v*` pull request runs the unfiltered release
state CI guard, so changing an otherwise unrelated tracked path cannot evade
the full-tree digest check.

GitHub does not grant ordinary workflow tokens repository-administration access,
so workflow code cannot repair its own ruleset. Check drift with
`python3 scripts/release/main_ruleset.py check --repo wallstop/fortress-rollback`
and a `GH_TOKEN` that can read repository administration. An administrator can
repair drift by changing `check` to `apply`; the helper creates or updates only
the exactly named declarative ruleset and never prints the token. Without the
required strict check, a previously green but stale PR could bypass the
prospective-tree run.

Publication must not edit or push the default branch. All release metadata is
part of the reviewed preparation PR. Retries reconcile the locally packaged
checksum with crates.io: absence permits publication, an exact checksum match
continues idempotently, and every mismatch fails closed. A failed Cargo client
does not prove the upload failed; reconcile registry state before deciding.
The crate manifest and Cargo command must both explicitly allow only and select
the `crates-io` registry.

Treat a pre-existing release tag as untrusted input until a helper from the
workflow-dispatch commit has validated it. Keep that trusted helper checkout
separate from the candidate source and disable checkout credential persistence.
Preserve and compare both the annotated tag object's direct object ID and its
direct commit target; reject lightweight tags, nested tags, side-branch targets,
and any lookup/fetch/push race that changes either identity. A retry may use an
older tagged commit only when it is an ancestor of the recorded dispatch SHA.
Re-query the exact remote checkpoint after fetching or racing a push and again
immediately before crates.io publication and GitHub Release mutation. Never run
release-control code from candidate source before this trust boundary passes.

When the requested tag does not exist, never assume the workflow-dispatch SHA
is still the prepared release: unrelated commits may have advanced `main`.
Strictly parse the requested-version metadata with dispatch-trusted code, use
its previous version to validate the exact annotated previous-release tag, and
require that peeled commit on the dispatch SHA's first-parent history. The
previous tag may be arbitrarily old: validate its ancestry and exact
first-parent position independently of the fixed recent-history bound used for
prepared-candidate discovery. Verify every requested-version tree in that
bounded candidate window with the dispatch-trusted release-state verifier, and
require the selected candidate to follow the previous tag on the same trusted
first-parent chain. Exactly one may become the candidate; zero matches,
multiple matches, a missing, off-chain, or misordered previous tag, and a
prepared candidate beyond the discovery bound all fail closed.

The release and generic nightly toolchains are repository-pinned. Never replace
their canonical version files or local installers with floating `stable` or
`nightly` channels in required release/CI jobs.

Required release workflows also use one exact Python runtime. Any third-party
Python package needed by those workflows must come from the reviewed release
requirements lock with artifact hashes and `--require-hashes`; never add a
floating `pip install` to an irreversible release path.

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
