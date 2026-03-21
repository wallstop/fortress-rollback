<!-- CATEGORY: Publishing & Organization -->
<!-- WHEN: Organizing workspace, splitting crates, module structure decisions -->

# Workspace Organization

---

## Core Principles

1. **Split on API boundaries, not LOC** -- code size alone is not a reason to split
2. **Minimize compile-time impact** -- many small crates does not equal faster compilation
3. **Reduce maintenance burden** -- each crate adds versioning, publishing, dependency overhead
4. **Design for reuse** -- only split when genuinely reusable across projects

---

## Decision Tree: Create a New Crate?

```
Is this used by external projects TODAY?
├── YES -> Has stable, well-defined API?
│          ├── YES -> Can be versioned independently?
│          │          ├── YES -> CREATE CRATE
│          │          └── NO -> Use module
│          └── NO -> Stabilize API first, use module
└── NO ->
    Need different feature flags than parent?
    ├── YES -> Will splitting reduce feature combinations?
    │          ├── YES -> CREATE CRATE
    │          └── NO -> Use cfg attributes
    └── NO ->
        Compile times a verified problem?
        ├── YES -> Will splitting enable parallel compilation?
        │          ├── YES -> Profile to confirm, then CREATE CRATE
        │          └── NO -> Use modules
        └── NO -> KEEP AS MODULE
```

### Invalid Reasons to Split

| Anti-Pattern | Why Wrong |
|--------------|-----------|
| "It's getting big" | Size alone is not valid |
| "Might be reusable someday" | YAGNI -- split when needed |
| "Cleaner separation" | Use modules within a crate |
| "Faster compilation" | Often slower (more linking) |

---

## Module Organization

### Flat vs Directory

```
# Flat -- simple, no submodules
src/
├── lib.rs
├── utils.rs
└── math.rs

# Directory -- module has submodules
src/
├── lib.rs
└── network/
    ├── mod.rs
    ├── tcp.rs
    └── udp.rs
```

Do NOT use a directory for single-file modules (`utils/mod.rs` when `utils.rs` suffices).

### Visibility Guidelines

```rust
pub struct PublicType;           // Public API
pub(crate) struct InternalType;  // Crate-internal
pub(super) fn helper();          // Parent module only
struct Private;                  // Module-private (default)

// Re-export for convenience -- flatten the API
pub use types::Config;
pub use error::{Error, Result};
```

### lib.rs as Coordinator

```rust
// src/lib.rs -- ~100 lines, not 5000
#![warn(missing_docs)]
#![deny(unsafe_code)]

mod types;
mod protocol;
mod network;
mod error;

pub use types::{Config, Session, Input};
pub use protocol::Protocol;
pub use error::{Error, Result};

pub(crate) mod internal;
```

---

## Workspace Setup

### Root Cargo.toml

```toml
[workspace]
resolver = "2"
members = ["crates/*"]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"

[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
thiserror = "1.0"

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }
```

### Member Crate

```toml
[package]
name = "my-project-core"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
serde.workspace = true

[lints]
workspace = true
```

### Directory Layout

```
my-workspace/
├── Cargo.toml          # Workspace root
├── Cargo.lock          # Shared lock file
├── .cargo/config.toml
├── crates/
│   ├── core/           # Shared types, minimal deps
│   ├── protocol/       # Domain logic
│   └── transport/      # I/O layer
├── tests/
│   └── integration/    # Workspace-level integration tests
└── benches/
```

### Workspace Commands

```bash
cargo build --workspace
cargo test --workspace
cargo test -p my-project-core
cargo clippy --workspace --all-targets
cargo doc --workspace
cargo publish -p crate_name
```

---

## Test Organization

### The Compilation Problem

Each file in `tests/` compiles as a separate crate. Use consolidated structure:

```
# SLOW: 4 separate compilation units
tests/
├── test_auth.rs
├── test_network.rs
└── test_protocol.rs

# FAST: Single crate with submodules
tests/
└── it/
    ├── main.rs      # Entry point
    ├── auth.rs
    ├── network.rs
    └── protocol.rs
```

```rust
// tests/it/main.rs
mod auth;
mod network;
mod protocol;
```

### Unit vs Integration Tests

- **Unit tests** (`#[cfg(test)] mod tests`): test internals, private functions
- **Integration tests** (`tests/`): test public API as a user would

---

## API Design for Modularity

```rust
// Minimal public API -- hide implementation
pub struct Session {
    state: SessionState,  // private
    config: Config,       // private
}

// Newtype for type safety
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameId(u32);

// Re-export dependency types exposed in API
pub use bytes::Bytes;

// Use #[non_exhaustive] for extensibility
#[non_exhaustive]
pub enum SessionEvent {
    Connected { peer_id: PeerId },
    Disconnected { peer_id: PeerId },
}
```

---

## Common Anti-Patterns

### Premature Splitting

```
# BAD: 5 crates, 800 lines total
my-project-core/      # 200 lines
my-project-types/     # 150 lines
my-project-utils/     # 100 lines

# GOOD: 1 crate with modules
src/
├── lib.rs
├── types.rs
├── utils.rs
└── core.rs
```

### Circular Module Dependencies

```rust
// BAD: auth uses network, network uses auth
// FIX: Extract shared types into a common module
mod types;  // Contains Credentials, Connection
mod auth;   // uses types::Credentials
mod network; // uses types::Connection
```

### Massive lib.rs

Keep `lib.rs` as a coordinator (~100 lines) with `mod` declarations and `pub use` re-exports. Put implementation in separate modules.

---

## Compilation Impact

| Factor | Effect |
|--------|--------|
| Many small crates | More object files, slower linking |
| Large public API | More codegen per dependent |
| Cross-crate calls | Less optimization without LTO |
| Incremental compilation | Works within crates, not across |
| Parallel compilation | Works across crates |

---

## Health Checks

```bash
cargo build --workspace --all-features
cargo clippy --workspace --all-targets
cargo nextest run --workspace
cargo tree --workspace --duplicates   # Inconsistent versions
cargo +nightly udeps --workspace      # Unused deps
```
