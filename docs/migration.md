<p align="center">
  <img src="../assets/logo.svg" alt="Fortress Rollback" width="128">
</p>

# Migration Guide: ggrs → fortress-rollback

Fortress Rollback is the correctness-first, verified fork of the original `ggrs` crate. This guide explains how to migrate existing projects.

## TL;DR
- Update your dependency to `fortress-rollback` and change Rust imports to `fortress_rollback`.
- Ensure your `Config::Address` type implements `Ord` + `PartialOrd` (in addition to `Clone + Eq + Hash`).
- Rename types: `GgrsError` → `FortressError`, `GgrsEvent` → `FortressEvent`, `GgrsRequest` → `FortressRequest`.
- All examples/tests now import `fortress_rollback`; mirror that pattern in your code.

## Dependency Changes
```toml
# Before
[dependencies]
ggrs = "0.11"

# After
[dependencies]
fortress-rollback = "0.11"  # rename; version tracks the fork
```

If you were using a git/path dependency, point it to the new repository:
```toml
fortress-rollback = { git = "https://github.com/wallstop/fortress-rollback", branch = "main" }
# or
fortress-rollback = { path = "../fortress-rollback" }
```

## Import Path Changes
```rust
- use ggrs::{SessionBuilder, P2PSession};
+ use fortress_rollback::{SessionBuilder, P2PSession};
```

## Type Renames (Breaking Change)
All `Ggrs*` types have been renamed to `Fortress*` for consistency:

```rust
// Before
use ggrs::{GgrsError, GgrsEvent, GgrsRequest};

// After
use fortress_rollback::{FortressError, FortressEvent, FortressRequest};
```

| Old Name       | New Name           |
|----------------|--------------------|
| `GgrsError`    | `FortressError`    |
| `GgrsEvent<T>` | `FortressEvent<T>` |
| `GgrsRequest<T>` | `FortressRequest<T>` |

Update your pattern matching accordingly:
```rust
// Before
match request {
    GgrsRequest::SaveGameState { cell, frame } => { ... }
    GgrsRequest::LoadGameState { cell, frame } => { ... }
    GgrsRequest::AdvanceFrame { inputs } => { ... }
}

// After
match request {
    FortressRequest::SaveGameState { cell, frame } => { ... }
    FortressRequest::LoadGameState { cell, frame } => { ... }
    FortressRequest::AdvanceFrame { inputs } => { ... }
}
```

## Address Trait Bounds (Breaking Change)
`Config::Address` now requires `Ord` + `PartialOrd` so deterministic collections can be used internally.
Most standard address types already satisfy this. For custom types, derive the traits:
```rust
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
struct MyAddress {
    // ...
}
```

## Features
Feature flags remain the same (`sync-send`, `wasm-bindgen`). Enable them as before.

## What Stayed the Same
- Request-driven API shape (Save/Load/Advance requests)
- Session types (`P2PSession`, `SpectatorSession`, `SyncTestSession`)
- Safe Rust guarantee (`#![forbid(unsafe_code)]`)

## What Improved
- Deterministic maps (no `HashMap` iteration order issues)
- Correctness-first positioning with ongoing formal verification work
- Documentation and branding aligned with the new name
- Consistent naming with `Fortress*` prefix on all public types

## Reporting Issues
Please file new issues on the Fortress Rollback repo: https://github.com/wallstop/fortress-rollback/issues
