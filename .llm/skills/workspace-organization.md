# Workspace Organization — Rust Crate and Module Structure

> **A comprehensive guide for organizing Rust workspaces, splitting code into crates, and structuring modules for maintainability, compile-time performance, and clean API boundaries.**

---

## Table of Contents

1. [Organization Philosophy](#organization-philosophy)
2. [Module Organization Patterns](#module-organization-patterns)
3. [When to Split Into Crates](#when-to-split-into-crates)
4. [Workspace Setup](#workspace-setup)
5. [Test Organization](#test-organization)
6. [API Design for Modularity](#api-design-for-modularity)
7. [Decision Trees](#decision-trees)
8. [Common Anti-Patterns](#common-anti-patterns)
9. [Verification Checklist](#verification-checklist)

---

## Organization Philosophy

### Core Principles

1. **Split on API Boundaries, Not LOC** — Code size is NOT a reason to split; logical cohesion is
2. **Minimize Compile-Time Impact** — Many small crates ≠ faster compilation
3. **Reduce Maintenance Burden** — Each crate is a potential failure point (versioning, publishing, dependencies)
4. **Preserve Cross-Crate Optimizations** — LTO is required to optimize across crate boundaries
5. **Design for Reuse** — Only split when genuinely reusable across projects

### The 1000 LOC Myth

**There is NO magic number for when to split.** Some modules naturally grow large while remaining cohesive:

```rust
// ✅ ACCEPTABLE - Large but cohesive module
// src/protocol.rs - 3000 lines of tightly coupled protocol implementation
// All code serves the same purpose and changes together

// ❌ PROBLEMATIC - Split arbitrarily by size
// src/protocol_part1.rs - 1000 lines
// src/protocol_part2.rs - 1000 lines
// src/protocol_part3.rs - 1000 lines
// Now you have artificial boundaries and cross-module coupling
```

### Compilation Time Reality

| Factor | Impact |
|--------|--------|
| Many small crates | More object files → slower linking (worst-case quadratic) |
| Large public API surface | More codegen work per dependent crate |
| Cross-crate calls | Less optimization without LTO |
| Incremental compilation | Works within crates, not across |
| Parallel compilation | Works across crates, helps large workspaces |

---

## Module Organization Patterns

### Library Crates

Entry point: `src/lib.rs`

```
my-lib/
├── Cargo.toml
├── src/
│   ├── lib.rs          # Public API, re-exports
│   ├── types.rs        # Core types
│   ├── utils.rs        # Internal utilities
│   └── error.rs        # Error types
```

### Binary Crates

Entry point: `src/main.rs`

```
my-app/
├── Cargo.toml
├── src/
│   ├── main.rs         # Entry point, CLI parsing
│   ├── lib.rs          # Shared library code (optional)
│   ├── config.rs       # Configuration
│   └── commands/       # Subcommands
│       ├── mod.rs
│       ├── run.rs
│       └── build.rs
```

### Multi-Binary with Shared Library

```
my-tools/
├── Cargo.toml
├── src/
│   ├── lib.rs          # Shared code between binaries
│   ├── common/
│   │   ├── mod.rs
│   │   └── utils.rs
│   └── bin/
│       ├── tool1.rs    # First binary
│       └── tool2.rs    # Second binary
```

### Flat vs Directory-Based Modules

**Use flat structure for small, simple modules:**

```rust
// src/lib.rs
mod utils;      // src/utils.rs
mod math;       // src/math.rs
mod types;      // src/types.rs

pub use types::*;
```

**Use directory-based for complex, multi-file modules:**

```rust
// src/lib.rs
mod network;    // src/network/mod.rs

// src/network/mod.rs
mod connection;     // src/network/connection.rs
mod protocol;       // src/network/protocol.rs
mod serialization;  // src/network/serialization.rs

pub use connection::Connection;
pub use protocol::Protocol;
```

### Module Visibility Guidelines

```rust
// ✅ PREFERRED - Explicit visibility
pub struct PublicType;           // Part of public API
pub(crate) struct InternalType;  // Crate-internal only
pub(super) fn helper();          // Parent module only
struct Private;                  // Module-private (default)

// ✅ PREFERRED - Re-export for convenience
// src/lib.rs
mod types;
mod error;

pub use types::Config;           // Flatten the API
pub use error::{Error, Result};  // Users import from crate root

// ❌ AVOID - Deep import paths for common types
// Force users to: use my_crate::types::config::settings::Config;
```

### When to Use `mod.rs` vs Named Files

```
# ✅ PREFERRED - Named file when module is self-contained
src/
├── lib.rs
├── utils.rs        # Single file, no submodules
└── math.rs         # Single file, no submodules

# ✅ PREFERRED - mod.rs when module has submodules
src/
├── lib.rs
└── network/
    ├── mod.rs      # Coordinates submodules
    ├── tcp.rs
    └── udp.rs

# ❌ AVOID - mod.rs for single-file modules
src/
├── lib.rs
└── utils/
    └── mod.rs      # Unnecessary directory, just use utils.rs
```

---

## When to Split Into Crates

### Valid Reasons to Split

| Reason | Example |
|--------|---------|
| **Genuinely reusable** | A serialization library used across multiple projects |
| **Different feature flags** | Core library vs optional integrations |
| **Clear API boundary** | Protocol implementation vs transport layer |
| **Parallel compilation benefits** | Large workspace where splitting enables parallelism |
| **Different stability guarantees** | Stable public API vs experimental internals |
| **Different MSRV requirements** | Core supports older Rust, extras use newer features |

### Invalid Reasons to Split

| Anti-Pattern | Why It's Wrong |
|--------------|----------------|
| "It's getting big" | Size alone is not a valid reason |
| "Other projects split this way" | Your project has different needs |
| "Might be reusable someday" | YAGNI - split when actually needed |
| "Cleaner separation" | Use modules within a crate instead |
| "Faster compilation" | Often makes it slower (more linking) |

### The Reusability Test

Before splitting, ask:

1. **Is this used by external projects TODAY?** (Not "might be someday")
2. **Does it have a stable, well-defined API?**
3. **Can it be versioned independently?**
4. **Would changes here NOT require changes elsewhere?**

If any answer is "no," keep it as a module.

---

## Workspace Setup

### Basic Workspace Structure

```toml
# /Cargo.toml (workspace root)
[workspace]
resolver = "2"
members = [
    "crates/core",
    "crates/protocol",
    "crates/transport",
]

# Shared dependencies
[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
thiserror = "1.0"
tokio = { version = "1.0", features = ["full"] }

# Shared package metadata
[workspace.package]
version = "0.1.0"
authors = ["Your Team"]
edition = "2021"
license = "MIT OR Apache-2.0"
repository = "https://github.com/org/project"
```

### Member Crate Configuration

```toml
# /crates/core/Cargo.toml
[package]
name = "my-project-core"
version.workspace = true
authors.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
serde.workspace = true
thiserror.workspace = true

# Crate-specific dependency
bytemuck = "1.0"
```

### Workspace-Level Configuration

```toml
# /Cargo.toml (continued)

# Shared lints across workspace
[workspace.lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[workspace.lints.clippy]
all = "warn"
pedantic = "warn"
nursery = "warn"
cargo = "warn"
```

### Directory Layout

```
my-workspace/
├── Cargo.toml              # Workspace root
├── Cargo.lock              # Shared lock file
├── .cargo/
│   └── config.toml         # Cargo configuration
├── crates/
│   ├── core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs
│   ├── protocol/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs
│   └── transport/
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs
├── examples/
│   └── basic.rs
├── tests/                  # Workspace-level integration tests
│   └── integration.rs
└── benches/
    └── benchmarks.rs
```

### Running Commands in Workspaces

```bash
# Build all crates
cargo build --workspace

# Test all crates
cargo test --workspace

# Test specific crate
cargo test -p my-project-core

# Run from any directory
cd crates/protocol
cargo test  # Still runs just protocol tests

# Run workspace-wide with all features
cargo test --workspace --all-features
```

---

## Test Organization

### The Integration Test Compilation Problem

Each file in `tests/` compiles as a **separate crate**:

```
# ❌ SLOW - Each file is a separate crate
tests/
├── test_auth.rs        # Crate 1
├── test_network.rs     # Crate 2
├── test_protocol.rs    # Crate 3
└── test_storage.rs     # Crate 4
# = 4 separate compilation units + linking each
```

### Consolidated Test Structure

```
# ✅ FAST - Single crate with submodules
tests/
├── it/
│   ├── main.rs         # Single crate entry point
│   ├── auth.rs
│   ├── network.rs
│   ├── protocol.rs
│   └── storage.rs
└── common/
    └── mod.rs          # Shared test utilities
```

```rust
// tests/it/main.rs
mod auth;
mod network;
mod protocol;
mod storage;

// Shared setup can go here
use my_crate::Config;

fn default_test_config() -> Config {
    Config::default()
}

// tests/it/auth.rs
use super::default_test_config;
use my_crate::auth::*;

#[test]
fn test_login() {
    let config = default_test_config();
    // ...
}
```

### Test Utilities Module

```rust
// tests/common/mod.rs
use my_crate::*;

/// Test fixture for network tests
pub struct TestNetwork {
    pub peers: Vec<MockPeer>,
}

impl TestNetwork {
    pub fn new(peer_count: usize) -> Self {
        Self {
            peers: (0..peer_count).map(|_| MockPeer::new()).collect(),
        }
    }

    pub fn connect_all(&mut self) {
        // ...
    }
}

/// Helper for generating test data
pub fn random_test_data(size: usize) -> Vec<u8> {
    (0..size).map(|i| i as u8).collect()
}
```

### Unit vs Integration Test Placement

```rust
// src/lib.rs

pub fn parse_header(data: &[u8]) -> Result<Header, ParseError> {
    // ...
}

// ✅ Unit tests: test internals, private functions
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_header() {
        // Test internal behavior
    }

    #[test]
    fn test_parse_edge_cases() {
        // Test boundary conditions
    }
}

// ✅ Integration tests (tests/it/): test public API as a user would
// tests/it/parsing.rs
use my_crate::parse_header;

#[test]
fn test_real_world_headers() {
    // Test with realistic data
}
```

---

## API Design for Modularity

### Minimize Public API Surface

```rust
// ✅ PREFERRED - Minimal public API
pub struct Session {
    // Private fields
    state: SessionState,
    config: Config,
}

impl Session {
    /// Create a new session with the given configuration.
    pub fn new(config: Config) -> Result<Self, Error> { ... }

    /// Advance the session by one frame.
    pub fn advance(&mut self, input: Input) -> Result<AdvanceResult, Error> { ... }

    /// Get the current session state.
    pub fn state(&self) -> &SessionState { ... }
}

// Internal methods not exposed
impl Session {
    fn validate_state(&self) -> Result<(), Error> { ... }
    fn apply_rollback(&mut self) -> Result<(), Error> { ... }
}
```

### Hide Implementation Details

```rust
// ✅ PREFERRED - Newtype wrapping dependencies
/// Frame identifier (wraps internal representation)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameId(u32);

impl FrameId {
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    pub fn as_u32(self) -> u32 {
        self.0
    }
}

// ❌ AVOID - Exposing raw types
pub type FrameId = u32;  // No type safety, can't add methods later
```

### Re-export Dependencies in Your API

```rust
// If your API exposes types from dependencies, re-export them

// src/lib.rs
pub use bytes::Bytes;  // Users don't need to depend on `bytes` directly

pub struct Message {
    pub payload: Bytes,  // Uses re-exported type
}

// Cargo.toml
[dependencies]
bytes = "1.0"
# Users of your crate get `bytes` transitively for the exposed types
```

### Use `#[non_exhaustive]` for Extensibility

```rust
// ✅ PREFERRED - Allow adding variants without breaking changes
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum SessionEvent {
    Connected { peer_id: PeerId },
    Disconnected { peer_id: PeerId },
    InputReceived { frame: FrameId, input: Input },
    // Future: can add new variants without semver break
}

// Users must have a wildcard arm:
match event {
    SessionEvent::Connected { peer_id } => { ... }
    SessionEvent::Disconnected { peer_id } => { ... }
    SessionEvent::InputReceived { frame, input } => { ... }
    _ => { /* handle unknown future variants */ }
}

// ✅ PREFERRED - Allow adding fields without breaking changes
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct Config {
    pub max_players: usize,
    pub input_delay: u32,
    // Future: can add new fields
}

// Users must use struct update syntax:
let config = Config {
    max_players: 4,
    input_delay: 2,
    ..Default::default()
};
```

---

## Decision Trees

### Should I Create a New Crate?

```
START: Do I need a new crate?
│
├── Is this functionality used by external projects TODAY?
│   ├── YES → Does it have a stable, well-defined API?
│   │         ├── YES → Can it be versioned independently?
│   │         │         ├── YES → CREATE NEW CRATE ✓
│   │         │         └── NO → Use module, reconsider later
│   │         └── NO → Use module, stabilize API first
│   └── NO → Continue below
│
├── Do you need different feature flags than the parent crate?
│   ├── YES → Does splitting reduce total feature combinations?
│   │         ├── YES → CREATE NEW CRATE ✓
│   │         └── NO → Use cfg attributes instead
│   └── NO → Continue below
│
├── Are compile times a verified problem?
│   ├── YES → Will splitting enable parallel compilation?
│   │         ├── YES → Profile to confirm, then CREATE CRATE ✓
│   │         └── NO → Splitting won't help, use modules
│   └── NO → Continue below
│
└── KEEP AS MODULE - Crate splitting not justified
```

### Should I Use a Directory-Based Module?

```
START: Flat file or directory?
│
├── Does this module have submodules?
│   ├── YES → USE DIRECTORY (module_name/mod.rs)
│   └── NO → Continue below
│
├── Is the module > 500 lines with distinct sections?
│   ├── YES → Consider splitting into submodules → USE DIRECTORY
│   └── NO → USE FLAT FILE (module_name.rs)
│
└── Default: USE FLAT FILE (module_name.rs)
```

### Module vs Function Visibility

```
START: What visibility should this have?
│
├── Is this part of the public API for users?
│   ├── YES → pub
│   └── NO → Continue below
│
├── Is this used by other modules in the crate?
│   ├── YES → Is it an implementation detail?
│   │         ├── YES → pub(crate)
│   │         └── NO → Could become pub, use pub(crate) for now
│   └── NO → Continue below
│
├── Is this used only by the parent module?
│   ├── YES → pub(super)
│   └── NO → Keep private (no pub)
│
└── Default: Keep private (no pub)
```

---

## Common Anti-Patterns

### Anti-Pattern 1: Premature Crate Splitting

```rust
// ❌ PROBLEMATIC - Split before needed
my-project/
├── my-project-core/        # 200 lines
├── my-project-types/       # 150 lines
├── my-project-utils/       # 100 lines
├── my-project-macros/      # 50 lines (only if proc macros needed)
└── my-project/             # 300 lines, depends on all above

// Result: 5 crates, 800 total lines, massive overhead

// ✅ PREFERRED - Single crate with modules
my-project/
└── src/
    ├── lib.rs
    ├── types.rs
    ├── utils.rs
    └── core.rs

// Result: 1 crate, same functionality, minimal overhead
```

### Anti-Pattern 2: Circular Module Dependencies

```rust
// ❌ PROBLEMATIC - Circular dependencies indicate poor boundaries
// src/auth.rs
use crate::network::Connection;  // auth depends on network

// src/network.rs
use crate::auth::Credentials;    // network depends on auth

// ✅ PREFERRED - Extract shared types, break cycle
// src/types.rs
pub struct Credentials { ... }
pub struct Connection { ... }

// src/auth.rs
use crate::types::Credentials;
// No dependency on network

// src/network.rs
use crate::types::Connection;
// No dependency on auth
```

### Anti-Pattern 3: Over-Using `mod.rs`

```rust
// ❌ PROBLEMATIC - Unnecessary nesting
src/
└── utils/
    └── mod.rs       # Just one file, no submodules

// ✅ PREFERRED - Flat when appropriate
src/
├── lib.rs
└── utils.rs         # Simple, direct
```

### Anti-Pattern 4: Massive `lib.rs`

```rust
// ❌ PROBLEMATIC - Everything in lib.rs
// src/lib.rs - 5000 lines with all implementation

// ✅ PREFERRED - lib.rs as coordinator
// src/lib.rs - ~100 lines
mod types;
mod protocol;
mod network;
mod error;

pub use types::{Config, Session, Input};
pub use protocol::Protocol;
pub use network::{Socket, Connection};
pub use error::{Error, Result};
```

### Anti-Pattern 5: Deep Re-Export Chains

```rust
// ❌ PROBLEMATIC - Re-export through many layers
// types.rs re-exports from inner.rs
// inner.rs re-exports from deep.rs
// deep.rs has the actual types
pub use crate::types::inner::deep::ActualType;

// ✅ PREFERRED - Direct re-exports
// lib.rs re-exports from where types are defined
pub use crate::deep::ActualType;
```

---

## Verification Checklist

### Before Reorganizing

```bash
# 1. Ensure all tests pass
cargo nextest run --workspace

# 2. Document current public API
cargo doc --no-deps --open
# Take notes on what's exposed

# 3. Check for external dependents
# If this is a published crate, this is a breaking change concern
```

### After Reorganizing

```bash
# 1. Verify compilation
cargo build --workspace --all-features

# 2. Check for new warnings
cargo clippy --workspace --all-targets

# 3. Verify tests still pass
cargo nextest run --workspace

# 4. Compare public API
cargo doc --no-deps --open
# Ensure API surface matches expectations

# 5. Check for visibility issues
# Look for "private type in public interface" errors

# 6. Verify unused code detection
cargo clippy -- -W dead_code
```

### Workspace Health Checks

```bash
# Check for circular dependencies
cargo tree --workspace --edges no-dev | grep -i cycle

# Check dependency versions are consistent
cargo tree --workspace --duplicates

# Verify all crates build independently
for crate in crates/*; do
    (cd "$crate" && cargo build)
done

# Check for unused dependencies
cargo +nightly udeps --workspace
```

---

## Quick Reference

### Module Declaration Patterns

| Pattern | Location | Use When |
|---------|----------|----------|
| `mod foo;` in lib.rs | `src/foo.rs` | Simple module, no submodules |
| `mod foo;` in lib.rs | `src/foo/mod.rs` | Module with submodules |
| `pub mod foo;` | Same as above | Module is part of public API |
| `pub(crate) mod foo;` | Same as above | Internal module, cross-module access |

### Visibility Modifiers

| Modifier | Scope | Use For |
|----------|-------|---------|
| `pub` | Everywhere | Public API |
| `pub(crate)` | Current crate | Crate internals shared across modules |
| `pub(super)` | Parent module | Helper functions for parent |
| `pub(in path)` | Specific module | Fine-grained control |
| (none) | Current module | Implementation details |

### Workspace Commands

| Command | Purpose |
|---------|---------|
| `cargo build --workspace` | Build all crates |
| `cargo test --workspace` | Test all crates |
| `cargo test -p crate_name` | Test specific crate |
| `cargo doc --workspace` | Document all crates |
| `cargo publish -p crate_name` | Publish specific crate |

---

## Summary

**Key Takeaways:**

1. **Don't split prematurely** — Use modules within a crate until you have a proven need for separation
2. **Split on API boundaries** — Not on arbitrary size limits
3. **Consolidate integration tests** — Use `tests/it/main.rs` pattern to avoid compilation overhead
4. **Minimize public surface** — Use `pub(crate)` for internals, re-export for convenience
5. **Workspace for coordination** — Share dependencies, lints, and configuration at workspace level
6. **Verify after changes** — Run tests, check API surface, look for new warnings
