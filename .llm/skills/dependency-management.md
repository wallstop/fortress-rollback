<!-- CATEGORY: Publishing & Organization -->
<!-- WHEN: Evaluating dependencies, supply chain security, cargo-deny -->

# Dependency Management

Every dependency is a liability: compile time, binary size, attack surface, maintenance burden.

---

## Evaluation Checklist

Before adding any dependency:

- [ ] Can this be done with std in <100 lines?
- [ ] Does the crate have >1M downloads?
- [ ] Is the license compatible (MIT/Apache-2.0)?
- [ ] Updated in last 6 months?
- [ ] Is it 1.0+ (or stable despite <1.0)?
- [ ] What transitive deps does it pull in?
- [ ] No RustSec advisories against it?
- [ ] From a known/trusted author or organization?

### When Dependencies ARE Worth It

Complex domains (crypto, compression, parsing), battle-tested code (serde, tokio), security-sensitive implementations, expertise gaps.

### Research Resources

| Resource | Purpose |
|----------|---------|
| [blessed.rs](https://blessed.rs) | Curated recommended crates |
| [lib.rs](https://lib.rs) | Better crate discovery |
| [RustSec](https://rustsec.org) | Security advisories |

---

## Analysis Commands

```bash
cargo tree                          # Full dependency graph
cargo tree -d                       # Duplicate dependencies
cargo tree -f "{p} {f}"             # Show features per package
cargo tree -i some_crate            # Why is this crate included?
cargo tree -e features              # Feature graph
cargo +nightly udeps                # Unused dependencies
cargo outdated                      # Newer versions available
```

---

## Version Management

```toml
# Use semver ranges -- NEVER pin exact versions
serde = "1.0"          # Gets 1.0.x patches
tokio = "1"            # Gets 1.x.x updates

# AVOID
serde = "=1.0.152"     # Locked, misses security fixes
```

Pre-1.0 crates (`0.x.y`) can break in minor versions. Prefer 1.0+ when available.

```bash
cargo update                            # Update all compatible
cargo update -p crate_name             # Update specific crate
cargo update -p crate_name --precise X  # Update to specific version
```

---

## Feature Management

```toml
# Disable unnecessary defaults
tokio = { version = "1", default-features = false, features = ["rt", "net"] }
serde = { version = "1.0", default-features = false, features = ["derive"] }
```

```bash
# Audit enabled features
cargo tree -f "{p} {f}"
cargo tree -f "{p} {f}" -i tokio  # Why is a feature enabled?
```

### Feature Propagation

```toml
[features]
default = ["std"]
std = ["serde/std", "dep-crate/std"]
serde = ["dep:serde"]  # Optional via feature

[dependencies]
serde = { version = "1.0", optional = true }
```

---

## Security

### cargo-audit

```bash
cargo audit              # Check RustSec advisories
cargo audit fix          # Auto-fix if possible
```

### cargo-deny

```bash
cargo deny init          # Initialize config
cargo deny check         # Run all checks
```

```toml
# deny.toml
[advisories]
vulnerability = "deny"
unmaintained = "warn"
yanked = "warn"

[licenses]
unlicensed = "deny"
allow = ["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Zlib"]

[bans]
multiple-versions = "warn"
wildcards = "deny"
deny = [
    { name = "openssl" },     # Prefer rustls
    { name = "openssl-sys" },
]

[sources]
unknown-registry = "deny"
unknown-git = "warn"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
```

### License Compatibility

| Safe | Caution | Avoid |
|------|---------|-------|
| MIT, Apache-2.0, BSD, ISC, Zlib | MPL-2.0, LGPL | GPL, AGPL |

### Unsafe Code Audit

```bash
cargo geiger  # Shows unsafe usage per crate
```

---

## Supply Chain Security CI

```yaml
jobs:
  security:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo install cargo-audit cargo-deny
      - run: cargo audit
      - run: cargo deny check
```

---

## Workspace Dependencies

```toml
# Root Cargo.toml -- define versions once
[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
my-core = { path = "crates/my-core" }

# Member Cargo.toml -- inherit
[dependencies]
serde = { workspace = true }
my-core = { workspace = true }
# Can override features while inheriting version
tracing = { workspace = true, features = ["log"] }
```

Benefits: consistent versions, single update point, cleaner manifests.

---

## Git Dependencies

```toml
# Use for: unreleased fixes, forks with patches, private crates
some-crate = { git = "https://github.com/org/repo", rev = "abc123" }
# Prefer rev= for reproducibility; branch= is not reproducible
# Cannot publish to crates.io with git dependencies
```

---

## Replacement Strategies

Replace dependencies proactively when: deprecated, unmaintained (1+ year), unpatched vulnerabilities, better alternatives exist, too many transitive deps.

---

## Fortress-Specific Requirements

All new dependencies must:

1. Pass `cargo deny check`
2. Support `no_std` (or be behind a feature flag)
3. Be deterministic (no hidden randomness)
4. Minimize features (only enable what's needed)
5. Be documented in Cargo.toml with a comment

### Recommended Crates

| Purpose | Crate | Notes |
|---------|-------|-------|
| Serialization | `serde`, `bincode` | Network protocol, save states |
| Hashing | `ahash`, `xxhash-rust` | Deterministic, fast |
| Compression | `lz4_flex` | Fast, pure Rust |
| Networking | `quinn`, `laminar` | QUIC, game UDP |

---

## Automated Updates

```yaml
# .github/dependabot.yml
version: 2
updates:
  - package-ecosystem: "cargo"
    directory: "/"
    schedule: { interval: "weekly" }
    groups:
      rust-dependencies:
        patterns: ["*"]
        update-types: ["minor", "patch"]
    open-pull-requests-limit: 10
```

---

## Agent Checklist

### Adding a Dependency

- [ ] Searched for std alternative
- [ ] Checked blessed.rs
- [ ] Verified license (MIT/Apache-2.0)
- [ ] Checked downloads (>100k preferred)
- [ ] Checked recent maintenance
- [ ] `cargo audit` -- no advisories
- [ ] `cargo tree` -- reviewed transitives
- [ ] Disabled unnecessary default features
- [ ] Added comment explaining why needed

### After Updating

- [ ] Full test suite passes
- [ ] `cargo clippy` clean
- [ ] `cargo deny check` passes
- [ ] No new duplicate dependencies
