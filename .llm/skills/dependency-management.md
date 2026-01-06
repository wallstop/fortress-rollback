# Dependency Management — Sustainable Rust Crate Dependencies

> **This document provides comprehensive guidance for managing dependencies in Rust crates.**
> Every dependency is a liability, not an asset. Choose wisely, update regularly, and minimize where possible.

## TL;DR — Quick Reference

```bash
# Dependency analysis
cargo tree                          # View full dependency graph
cargo tree -d                       # Find duplicate dependencies
cargo tree -f "{p} {f}"             # Show features per package
cargo tree -i some_crate            # Why is this crate included?

# Maintenance tools
cargo install cargo-udeps cargo-outdated cargo-audit cargo-deny
cargo +nightly udeps                # Find unused dependencies
cargo outdated                      # Check for newer versions
cargo update                        # Update to latest compatible versions

# Security
cargo audit                         # Check RustSec advisories
cargo deny check                    # Comprehensive dependency checks
```

**Key Principles:**

1. **Dependencies are liabilities** — Each adds compile time, complexity, and attack surface
2. **Prefer std** — Use standard library before reaching for crates
3. **Never pin exact versions** — Leads to outdated, vulnerable dependencies
4. **Update regularly** — Run `cargo update` frequently, automate with Dependabot
5. **Audit security** — Run `cargo audit` in CI on every build

---

## Dependencies as Liabilities

### The True Cost of Dependencies

Every dependency you add comes with hidden costs:

| Cost | Impact |
|------|--------|
| **Compile time** | Each crate adds to build time, especially with proc-macros |
| **Binary size** | More code means larger binaries |
| **Complexity** | More code to understand and debug |
| **Documentation** | Users must understand dep interactions |
| **Attack surface** | More code means more potential vulnerabilities |
| **Maintenance** | You inherit the dep's maintenance burden |
| **Breaking changes** | API changes can break your code |

```rust
// ❌ ANTI-PATTERN: Adding a dependency for trivial functionality
// Cargo.toml: is-even = "1.0"

fn check_even(n: u32) -> bool {
    is_even::is_even(n)  // Pulled in a whole crate for this?
}

// ✅ CORRECT: Write trivial functionality yourself
fn check_even(n: u32) -> bool {
    n % 2 == 0
}
```

### When Dependencies Make Sense

Dependencies ARE worth it when:

- **Complex domain** — Cryptography, compression, parsing formats
- **Battle-tested** — Widely used, well-audited code (e.g., `serde`, `tokio`)
- **Expertise gap** — You lack domain expertise to implement correctly
- **Time-critical** — Business needs outweigh long-term maintenance cost
- **Security-sensitive** — Professional implementations of crypto, auth, etc.

---

## Evaluating Dependencies

### Decision Framework

Before adding any dependency, answer these questions:

```
┌─────────────────────────────────────────────────────────────────┐
│                  DEPENDENCY EVALUATION CHECKLIST                │
├─────────────────────────────────────────────────────────────────┤
│ □ Can this be done with std?                                    │
│ □ Is the functionality complex enough to justify a dep?         │
│ □ Does the crate have > 1 million downloads?                    │
│ □ Is the license compatible (MIT/Apache-2.0)?                   │
│ □ Has it been updated in the last 6 months?                     │
│ □ Are issues being addressed?                                   │
│ □ Is there a RustSec advisory against it?                       │
│ □ Is it from a known/trusted author or organization?            │
│ □ Is it version 1.0+ (or stable despite < 1.0)?                 │
│ □ What transitive dependencies does it pull in?                 │
└─────────────────────────────────────────────────────────────────┘
```

### Research Resources

| Resource | URL | Use Case |
|----------|-----|----------|
| **blessed.rs** | <https://blessed.rs> | Curated list of recommended crates |
| **lib.rs** | <https://lib.rs> | Better crate discovery than crates.io |
| **crates.io** | <https://crates.io> | Official registry, check downloads/versions |
| **RustSec** | <https://rustsec.org> | Security advisories database |
| **docs.rs** | <https://docs.rs> | Documentation quality check |

### Checking Crate Health

```bash
# Check downloads and recent versions on crates.io
# Look for: consistent releases, growing downloads

# Check GitHub/GitLab repository
# Look for: recent commits, issues being addressed, PR activity

# Check transitive dependencies
cargo tree -i suspect_crate

# Example output showing why rand is included:
# rand v0.8.5
# ├── my_crate v0.1.0
# │   └── some_dep v1.2.3
# └── another_dep v2.0.0
```

### License Compatibility

```rust
// ✅ SAFE - Permissive licenses, compatible with any project
// MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Zlib

// ⚠️  CAUTION - Copyleft licenses, may have implications
// MPL-2.0 (file-level copyleft)
// LGPL-2.1, LGPL-3.0 (library copyleft)

// ❌ AVOID - Strong copyleft, makes your entire crate copyleft
// GPL-2.0, GPL-3.0, AGPL-3.0
```

---

## Limiting Dependencies

### Prefer Standard Library

The Rust standard library provides many utilities people reach for crates to get:

```rust
// ❌ ANTI-PATTERN: Using external crates for std functionality

// itertools for basic iteration? std has most of it
use itertools::Itertools;
vec.into_iter().unique().collect();

// ✅ CORRECT: Use std where possible
use std::collections::HashSet;
let unique: HashSet<_> = vec.into_iter().collect();
let unique: Vec<_> = unique.into_iter().collect();

// ❌ ANTI-PATTERN: External crate for HashMap
use hashbrown::HashMap;

// ✅ CORRECT: std HashMap (backed by hashbrown anyway since Rust 1.36)
use std::collections::HashMap;

// ❌ ANTI-PATTERN: rand for simple cases where not needed
use rand::random;
let id = random::<u64>();

// ✅ CORRECT: If truly random not needed, use deterministic approach
// Or if randomness IS needed, rand is a reasonable choice
```

### Standard Library Collections

`std::collections` provides:

- `Vec<T>` — Growable array
- `VecDeque<T>` — Double-ended queue
- `LinkedList<T>` — Doubly-linked list
- `HashMap<K, V>` — Hash table
- `BTreeMap<K, V>` — Sorted map
- `HashSet<T>` — Hash set
- `BTreeSet<T>` — Sorted set
- `BinaryHeap<T>` — Priority queue

### Finding Unused Dependencies

```bash
# Install cargo-udeps (requires nightly)
cargo install cargo-udeps

# Find unused dependencies
cargo +nightly udeps

# Example output:
# unused dependencies:
#     `my_crate v0.1.0`
#         └── serde_json
#         └── deprecated_crate
```

---

## Version Management

### Never Pin Exact Versions

```toml
# ❌ ANTI-PATTERN: Pinning exact versions
[dependencies]
serde = "=1.0.152"        # Locked forever, misses security fixes
tokio = "=1.25.0"         # Can't get bug fixes
rand = "=0.8.5"           # Stuck on old version

# ✅ CORRECT: Use semver ranges
[dependencies]
serde = "1.0"             # Gets 1.0.x patches automatically
tokio = "1"               # Gets 1.x.x updates
rand = "0.8"              # Gets 0.8.x updates

# ✅ ACCEPTABLE: Minor version constraint when needed
[dependencies]
some_crate = "1.5"        # Requires features from 1.5, gets 1.5.x patches
```

### Regular Updates

```bash
# Update to latest compatible versions (respects Cargo.toml constraints)
cargo update

# Check for outdated dependencies
cargo install cargo-outdated
cargo outdated

# Example output:
# Name             Project  Compat   Latest   Kind
# ----             -------  ------   ------   ----
# serde            1.0.152  1.0.193  1.0.193  Normal
# tokio            1.25.0   1.35.1   1.35.1   Normal
# old_crate        0.5.0    0.5.0    1.0.0    Normal  ← Major update available
```

### Automated Updates

Configure Dependabot in `.github/dependabot.yml`:

```yaml
version: 2
updates:
  - package-ecosystem: "cargo"
    directory: "/"
    schedule:
      interval: "weekly"
    # Group minor/patch updates to reduce PR noise
    groups:
      rust-dependencies:
        patterns:
          - "*"
        update-types:
          - "minor"
          - "patch"
    # Limit open PRs
    open-pull-requests-limit: 10
    # Add reviewers
    reviewers:
      - "your-team"
    # Labels for filtering
    labels:
      - "dependencies"
      - "rust"
```

### Handling Major Version Updates

```bash
# Don't skip major versions - harder to upgrade later
# If you're on 1.x and 3.0 is out, upgrade to 2.0 first

# Check CHANGELOG for breaking changes
# Test thoroughly after major updates

# Example migration workflow:
cargo update -p some_crate --precise 2.0.0  # Update specific crate
cargo test                                    # Run full test suite
cargo clippy                                  # Check for new warnings
```

---

## Stability Considerations

### Pre-1.0 vs 1.0+ Crates

```toml
# 1.0+ crates follow semver strictly:
# - 1.0.x → patch releases, bug fixes only
# - 1.x.0 → minor releases, new features, backward compatible
# - x.0.0 → major releases, breaking changes allowed

# Pre-1.0 crates (0.x.y) can break in MINOR versions:
# - 0.1.x → patch releases
# - 0.x.0 → can contain breaking changes!

[dependencies]
# ✅ Stable - follows semver
serde = "1.0"           # Breaking changes only in 2.0
tokio = "1"             # Stable since 1.0

# ⚠️  Less stable - breaking changes possible in 0.x
rand = "0.8"            # Well-maintained but technically pre-1.0
hyper = "0.14"          # Stable in practice despite version
```

### Evaluating Real Stability

Version number isn't everything. Consider:

```
┌─────────────────────────────────────────────────────────────────┐
│                    STABILITY EVALUATION                         │
├─────────────────────────────────────────────────────────────────┤
│ Positive Signals:                                               │
│   • Frequent maintenance commits                                │
│   • Active issue triage                                         │
│   • Clear deprecation policy                                    │
│   • Good documentation                                          │
│   • Used by major projects                                      │
│   • Backed by organization (not just individual)                │
│                                                                 │
│ Warning Signs:                                                  │
│   • No commits in 1+ year                                       │
│   • Issues piling up without response                           │
│   • Frequent breaking changes                                   │
│   • Single maintainer with no succession plan                   │
│   • Deprecated without replacement                              │
└─────────────────────────────────────────────────────────────────┘
```

---

## Feature Management

### Disable Unnecessary Features

```toml
# ❌ ANTI-PATTERN: Using default features when you don't need them
[dependencies]
tokio = "1"                    # Pulls in ALL default features
reqwest = "0.11"               # Includes default TLS, cookies, etc.

# ✅ CORRECT: Disable defaults, add only what you need
[dependencies]
tokio = { version = "1", default-features = false, features = ["rt", "net"] }
reqwest = { version = "0.11", default-features = false, features = ["json", "rustls-tls"] }

# Example: serde without derive macro (if not needed)
serde = { version = "1.0", default-features = false }

# Example: serde with derive (most common)
serde = { version = "1.0", features = ["derive"] }
```

### Auditing Features

```bash
# See what features are enabled for each dependency
cargo tree -f "{p} {f}"

# Example output:
# my_crate v0.1.0 (features: [])
# ├── serde v1.0.193 (features: [derive, std])
# ├── tokio v1.35.1 (features: [rt-multi-thread, net, sync, macros])
# │   ├── tokio-macros v2.2.0 (features: [])
# │   └── ...

# Find why a feature is enabled
cargo tree -f "{p} {f}" -i tokio

# Check if a feature can be disabled
# Try building without it and see what breaks
```

### Feature Propagation

```toml
# Your crate can expose features that control dependencies
[features]
default = ["std"]
std = ["serde/std", "dep-crate/std"]
serde = ["dep:serde"]          # Optional dependency via feature

[dependencies]
serde = { version = "1.0", optional = true }
dep-crate = { version = "1.0", default-features = false }
```

---

## Security

### Cargo Audit

```bash
# Install
cargo install cargo-audit

# Run security audit
cargo audit

# Example output:
# Crate:     smallvec
# Version:   0.6.10
# Warning:   unsound
# Title:     smallvec creates uninitialized value of any type
# Solution:  upgrade to >= 0.6.14 OR >= 1.0.0
#
# error: 1 vulnerability found!

# Fix by updating
cargo update -p smallvec
```

### Cargo Deny

`cargo-deny` provides comprehensive dependency checking:

```bash
# Install
cargo install cargo-deny

# Initialize config
cargo deny init

# Run all checks
cargo deny check
```

Configure in `deny.toml`:

```toml
[advisories]
db-path = "~/.cargo/advisory-db"
db-urls = ["https://github.com/rustsec/advisory-db"]
vulnerability = "deny"
unmaintained = "warn"
yanked = "warn"
notice = "warn"

[licenses]
unlicensed = "deny"
allow = [
    "MIT",
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Zlib",
]

[bans]
multiple-versions = "warn"
wildcards = "deny"
highlight = "all"

# Ban specific crates
deny = [
    # Example: ban openssl in favor of rustls
    { name = "openssl" },
    { name = "openssl-sys" },
]

[sources]
unknown-registry = "deny"
unknown-git = "warn"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
```

### CI Integration

Add to your CI workflow (`.github/workflows/ci.yml`):

```yaml
jobs:
  security:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install cargo-audit
        run: cargo install cargo-audit

      - name: Run cargo audit
        run: cargo audit

      - name: Install cargo-deny
        run: cargo install cargo-deny

      - name: Run cargo deny
        run: cargo deny check
```

### Unsafe Code Audit

```bash
# Install cargo-geiger
cargo install cargo-geiger

# Audit unsafe code in dependencies
cargo geiger

# Example output shows unsafe usage per crate:
# Functions  Coverage  Crate
# 0/0        100.0%    my_crate
# 2/5         60.0%    some_dep  ← Has unsafe code
```

---

## Workspace Dependencies

### Centralized Version Management

For workspaces with multiple crates, use `[workspace.dependencies]`:

```toml
# Root Cargo.toml
[workspace]
members = ["crates/*"]
resolver = "2"

[workspace.dependencies]
# Define versions once at workspace level
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
thiserror = "1.0"
anyhow = "1.0"
tracing = "0.1"

# Internal crates
my-core = { path = "crates/my-core" }
my-utils = { path = "crates/my-utils" }
```

```toml
# crates/my-app/Cargo.toml
[package]
name = "my-app"
version = "0.1.0"

[dependencies]
# Inherit from workspace
serde = { workspace = true }
tokio = { workspace = true }
thiserror = { workspace = true }

# Internal dependency
my-core = { workspace = true }

# Can override features while inheriting version
tracing = { workspace = true, features = ["log"] }
```

### Benefits of Workspace Dependencies

1. **Consistent versions** — All crates use the same dependency versions
2. **Single update point** — Update once, applies everywhere
3. **Reduced conflicts** — Prevents version mismatches between crates
4. **Cleaner manifests** — Member Cargo.toml files are simpler

---

## Git Dependencies

### When to Use Git Dependencies

```toml
# Use git dependencies for:
# - Testing unreleased fixes/features
# - Using forks with patches
# - Private crates not on crates.io
# - Pre-release testing before publishing

[dependencies]
# Specific branch
some-crate = { git = "https://github.com/org/some-crate", branch = "main" }

# Specific tag
some-crate = { git = "https://github.com/org/some-crate", tag = "v1.0.0" }

# Specific commit (most reproducible)
some-crate = { git = "https://github.com/org/some-crate", rev = "abc123" }

# Private repository (uses SSH)
private-crate = { git = "git@github.com:org/private-crate.git" }
```

### Cautions with Git Dependencies

```toml
# ⚠️  CAUTION: Git dependencies have drawbacks

# 1. Not publishable to crates.io
# If you want to publish, you must use crates.io dependencies

# 2. Branch references can change
# Using branch = "main" means builds aren't reproducible
# Prefer rev = "commit-hash" for reproducibility

# 3. Can break unexpectedly
# The upstream repo can be deleted, rebased, or changed

# 4. Slower builds
# Git deps are fetched fresh more often than cached crates.io deps
```

---

## Dependency Replacement Strategies

### When to Replace Dependencies

Replace dependencies proactively when:

- **Deprecated** — Maintainer announced deprecation
- **Unmaintained** — No updates for 1+ years, issues ignored
- **Security issues** — Unpatched vulnerabilities
- **Better alternatives** — Newer crates with better design
- **Bloated** — Dependency pulls too many transitive deps

### Finding Alternatives

```bash
# Check blessed.rs for recommended alternatives
# Check lib.rs for similar crates
# Search GitHub for "awesome rust" lists

# Compare candidates:
# - Feature parity
# - Dependency count
# - Maintenance activity
# - Community adoption
```

### Migration Example

```rust
// Example: Migrating from failure to thiserror

// ❌ OLD: Using deprecated failure crate
use failure::{Error, Fail};

#[derive(Debug, Fail)]
enum MyError {
    #[fail(display = "IO error: {}", _0)]
    Io(#[cause] std::io::Error),
}

// ✅ NEW: Using thiserror (modern, maintained)
use thiserror::Error;

#[derive(Debug, Error)]
enum MyError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

---

## Fortress-Specific Guidelines

### Required Dependencies Review

All new dependencies in Fortress Rollback must:

1. **Pass `cargo deny check`** — No license or security issues
2. **Support `no_std`** — Or be behind a feature flag if std-only
3. **Be deterministic** — No hidden randomness or non-determinism
4. **Minimize features** — Only enable what's actually needed
5. **Be documented** — Comment why the dependency is needed

### Cargo.toml Documentation

```toml
[dependencies]
# Serialization - required for network protocol and save states
serde = { version = "1.0", default-features = false, features = ["derive", "alloc"] }

# Error handling - zero-cost error types with derives
thiserror = "1.0"

# Deterministic hashing - required for game state verification
# NOTE: Must use fixed-state hasher, not RandomState
ahash = { version = "0.8", default-features = false }

# Property testing - dev only
[dev-dependencies]
proptest = "1.0"
```

### Recommended Crates for Game Networking

| Purpose | Crate | Notes |
|---------|-------|-------|
| Serialization | `serde`, `bincode` | Use for network protocol |
| Hashing | `ahash`, `xxhash-rust` | Deterministic, fast |
| Compression | `lz4_flex` | Fast, pure Rust |
| Networking | `quinn`, `laminar` | QUIC, game-oriented UDP |
| Crypto | `ring`, `rustls` | If encryption needed |

---

## Checklist for AI Agents

When adding or updating dependencies:

```markdown
## Pre-Addition Checklist
- [ ] Searched for std alternative first
- [ ] Checked blessed.rs for recommendations
- [ ] Verified license compatibility (MIT/Apache-2.0)
- [ ] Checked crate downloads (> 100k preferred)
- [ ] Checked recent maintenance activity
- [ ] Ran `cargo audit` - no advisories
- [ ] Reviewed transitive dependencies with `cargo tree`
- [ ] Disabled unnecessary default features
- [ ] Added comment explaining why dependency is needed

## Post-Update Checklist
- [ ] Ran full test suite
- [ ] Ran `cargo clippy`
- [ ] Ran `cargo deny check`
- [ ] Verified no new duplicate dependencies
- [ ] Updated any version comments if needed
```

---

## Common Commands Reference

```bash
# === Analysis ===
cargo tree                              # Full dependency tree
cargo tree -d                           # Duplicate dependencies
cargo tree -f "{p} {f}"                 # With features
cargo tree -i crate_name                # Inverse (why included)
cargo tree -e features                  # Feature graph

# === Maintenance ===
cargo update                            # Update all compatible
cargo update -p crate_name              # Update specific crate
cargo update -p crate_name --precise X  # Update to specific version
cargo +nightly udeps                    # Find unused deps
cargo outdated                          # Check for newer versions

# === Security ===
cargo audit                             # RustSec advisories
cargo audit fix                         # Auto-fix if possible
cargo deny check                        # Comprehensive checks
cargo geiger                            # Unsafe code audit

# === Inspection ===
cargo metadata --format-version 1       # Machine-readable dep info
cargo pkgid crate_name                  # Get exact package ID
```
