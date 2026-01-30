<p align="center">
  <img src="assets/logo.svg" alt="Fortress Rollback" width="128">
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
fortress-rollback = "0.2"  # current version
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

## Result Type Alias Rename

The `Result` type alias has been renamed to `FortressResult` to avoid shadowing
the standard library's `Result` when using glob imports:

```rust
// Before
use fortress_rollback::Result;
fn my_function() -> Result<()> { ... }

// After (option 1: use the new name directly)
use fortress_rollback::FortressResult;
fn my_function() -> FortressResult<()> { ... }

// After (option 2: local alias if you prefer short names)
use fortress_rollback::FortressResult as Result;
fn my_function() -> Result<()> { ... }
```

## Input Vector Type Change (Breaking Change)

The `inputs` field in `FortressRequest::AdvanceFrame` now uses `InputVec<T::Input>` (a `SmallVec`)
instead of `Vec<(T::Input, InputStatus)>`. This avoids heap allocations for games with 1-4 players.

**Most code will work unchanged** since `InputVec` implements `Deref<Target = [(T::Input, InputStatus)]>`:

```rust
// These all work unchanged:
for (input, status) in inputs.iter() { ... }
let first_input = inputs[0];
let len = inputs.len();
```

If you explicitly typed the inputs as `Vec`, update the signature:

```rust
// Before
fn process_inputs(inputs: Vec<(MyInput, InputStatus)>) { ... }

// After (two options)
use fortress_rollback::InputVec;

// Option 1: Use InputVec directly
fn process_inputs(inputs: InputVec<MyInput>) { ... }

// Option 2: Accept any slice-like type (most flexible)
fn process_inputs(inputs: &[(MyInput, InputStatus)]) { ... }

// Option 3: Convert to Vec if needed (allocates)
fn process_inputs(inputs: impl Into<Vec<(MyInput, InputStatus)>>) {
    let inputs = inputs.into_iter().collect::<Vec<_>>();
    ...
}
```

The `InputVec` type alias is re-exported for convenience:

```rust
use fortress_rollback::InputVec;
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

Existing feature flags (`sync-send`, `wasm-bindgen`) remain compatible. Fortress Rollback adds several new features:

| Feature | Description | New in Fortress |
|---------|-------------|-----------------|
| `sync-send` | Multi-threaded trait bounds | ❌ (existing) |
| `wasm-bindgen` | WASM compatibility | ❌ (existing) |
| `tokio` | Async Tokio UDP socket adapter | ✅ |
| `json` | JSON serialization for telemetry types | ✅ |
| `paranoid` | Runtime invariant checking | ✅ |
| `loom` | Concurrency testing | ✅ |
| `z3-verification` | Formal verification tests | ✅ |
| `graphical-examples` | Interactive demos | ✅ |

> **Note:** The `json` feature enables `to_json()` and `to_json_pretty()` methods on telemetry types.
> Without this feature, the `serde_json` dependency is not included, reducing the default dependency count.

For detailed feature documentation, see the [User Guide](User-Guide#feature-flags).

## What Stayed the Same

- Request-driven API shape (Save/Load/Advance requests)
- Session types (`P2PSession`, `SpectatorSession`, `SyncTestSession`)
- Safe Rust guarantee (`#![forbid(unsafe_code)]`)

## What Improved

- Deterministic maps (no `HashMap` iteration order issues)
- Correctness-first positioning with ongoing formal verification work
- Documentation and branding aligned with the new name
- Consistent naming with `Fortress*` prefix on all public types

## New Configuration APIs

Fortress Rollback introduces structured configuration structs that replace scattered builder methods:

### Network Configuration Structs

```rust
use fortress_rollback::{SyncConfig, ProtocolConfig, TimeSyncConfig, SpectatorConfig, InputQueueConfig};

// Before: Limited configuration options
let builder = SessionBuilder::<MyConfig>::new()
    .with_fps(60)?
    .with_input_delay(2);

// After: Rich, preset-based configuration
let builder = SessionBuilder::<MyConfig>::new()
    .with_fps(60)?
    .with_input_delay(2)
    .with_sync_config(SyncConfig::high_latency())
    .with_protocol_config(ProtocolConfig::competitive())
    .with_time_sync_config(TimeSyncConfig::responsive())
    .with_spectator_config(SpectatorConfig::fast_paced())
    .with_input_queue_config(InputQueueConfig::high_latency());
```

### SaveMode Enum

```rust
use fortress_rollback::SaveMode;

// Before (deprecated)
builder.with_sparse_saving_mode(true);

// After (preferred)
builder.with_save_mode(SaveMode::Sparse);
```

### Violation Observer

Monitor internal specification violations:

```rust
use fortress_rollback::telemetry::CollectingObserver;
use std::sync::Arc;

let observer = Arc::new(CollectingObserver::new());
let builder = SessionBuilder::<MyConfig>::new()
    .with_violation_observer(observer.clone());

// After operations, check for violations
if !observer.is_empty() {
    for v in observer.violations() {
        eprintln!("Violation: {}", v);
    }
}
```

See the [User Guide - Complete Configuration Reference](User-Guide#complete-configuration-reference) for full documentation.

## New Desync Detection APIs

Fortress Rollback adds new APIs for detecting and monitoring desynchronization:

### SyncHealth API

The new `SyncHealth` enum and associated methods provide proper synchronization status checking:

```rust
use fortress_rollback::SyncHealth;

// Check sync status with a specific peer
match session.sync_health(peer_handle) {
    Some(SyncHealth::InSync) => println!("Synchronized"),
    Some(SyncHealth::Pending) => println!("Waiting for checksum data"),
    Some(SyncHealth::DesyncDetected { frame, .. }) => {
        panic!("Desync detected at frame {}", frame)
    }
    None => {} // Not a remote player
}

// Check all peers at once
if session.is_synchronized() {
    println!("All peers in sync");
}

// Get the highest verified frame
if let Some(frame) = session.last_verified_frame() {
    println!("Verified sync up to frame {}", frame);
}
```

### NetworkStats Checksum Fields

`NetworkStats` now includes desync detection fields:

```rust
let stats = session.network_stats(peer_handle)?;
println!("Last compared: {:?}", stats.last_compared_frame);
println!("Checksums match: {:?}", stats.checksums_match);
```

## Important Behavioral Differences

### Session Termination Pattern

**⚠️ Warning for GGRS users:** If you were using `confirmed_frame()` or `last_confirmed_frame()` to determine when to terminate a session, this pattern is incorrect and can lead to subtle bugs.

```rust
// ⚠️ WRONG: This was a common GGRS pattern that doesn't work correctly
if session.confirmed_frame() >= target_frames {
    break; // Dangerous! Peers may be at different frames!
}
```

The correct pattern uses the new `SyncHealth` API:

```rust
// ✓ CORRECT: Use sync_health() to verify peer synchronization
if session.confirmed_frame() >= target_frames {
    match session.sync_health(peer_handle) {
        Some(SyncHealth::InSync) => break, // Safe to exit
        Some(SyncHealth::DesyncDetected { .. }) => panic!("Desync!"),
        _ => continue, // Keep polling until verified
    }
}
```

See [Common Pitfalls](User-Guide#common-pitfalls) in the User Guide for full details.

### Desync Detection Default

**⚠️ Breaking Change:** Desync detection is now **enabled by default** with `DesyncDetection::On { interval: 60 }` (once per second at 60fps).

This is a deliberate departure from GGRS, which defaulted to `Off`. Fortress Rollback enables detection by default because:

- Silent desync is a correctness bug that's extremely difficult to debug
- The overhead is minimal (one checksum comparison per second)
- Early detection prevents subtle multiplayer issues from reaching production
- This aligns with our correctness-first philosophy

If you need to disable desync detection (e.g., for performance benchmarking), explicitly opt out:

```rust
use fortress_rollback::DesyncDetection;

let session = SessionBuilder::<GameConfig>::new()
    .with_desync_detection_mode(DesyncDetection::Off) // Explicit opt-out
    // ...
    .start_p2p_session(socket)?;
```

For tighter detection (e.g., competitive games with anti-cheat needs), reduce the interval:

```rust
use fortress_rollback::DesyncDetection;

let session = SessionBuilder::<GameConfig>::new()
    .with_desync_detection_mode(DesyncDetection::On { interval: 10 }) // 6 checks/sec at 60fps
    // ...
    .start_p2p_session(socket)?;
```

## More Information

For a complete comparison of features, bug fixes, and improvements, see [Fortress vs GGRS](Fortress-vs-GGRS).

## Reporting Issues

Please file new issues on the Fortress Rollback repo: <https://github.com/wallstop/fortress-rollback/issues>
