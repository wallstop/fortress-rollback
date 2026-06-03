<!-- SYNC: This wiki page is generated from docs/migration.md. Edit docs source. -->

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
- **New in Unreleased:** runtime input-delay adjustment (`set_input_delay`/`input_delay`), opt-in graceful peer drop (`DisconnectBehavior::ContinueWithout`, `with_disconnect_behavior`), explicit graceful removal (`remove_player`), and fail-closed redundant spectator divergence; exhaustive matches on `FortressEvent`, `FortressError`, `InvalidRequestKind`, and `InternalErrorKind` need new arms — see [Unreleased section](#unreleased-runtime-input-delay-disconnect-behavior-graceful-peer-removal-and-spectator-divergence).

## Dependency Changes

```toml
# Before
[dependencies]
ggrs = "0.11"

# After
[dependencies]
fortress-rollback = "0.8"  # current version
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

| Old Name         | New Name             |
| ---------------- | -------------------- |
| `GgrsError`      | `FortressError`      |
| `GgrsEvent<T>`   | `FortressEvent<T>`   |
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

## Input Trait Bounds (Breaking Change)

`Config::Input` now requires `Eq` in addition to `PartialEq`. This ensures reflexive
equality for deterministic rollback; non-reflexive types (e.g., `f32`, `f64`) would cause
phantom prediction misses because `NaN != NaN` can make the engine treat identical inputs
as different, triggering unnecessary rollbacks.

Most custom input types only need an extra derive:

```rust
// Before
#[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
struct MyInput {
    buttons: u8,
    stick_x: i8,
}

// After
#[derive(Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
struct MyInput {
    buttons: u8,
    stick_x: i8,
}
```

> **Note:** All primitive integer types (`u8`, `i8`, `u16`, `i16`, `u32`, `i32`, `u64`,
> `i64`, `u128`, `i128`, `usize`, `isize`) and `bool` already implement `Eq`, so input
> structs composed entirely of these types only need the added derive.

## Features

The `sync-send` feature flag remains compatible. Fortress Rollback adds several new features:

| Feature              | Description                            | New in Fortress |
| -------------------- | -------------------------------------- | --------------- |
| `sync-send`          | Multi-threaded trait bounds            | ❌ (existing)    |
| `tokio`              | Async Tokio UDP socket adapter         | ✅               |
| `json`               | JSON serialization for telemetry types | ✅               |
| `paranoid`           | Runtime invariant checking             | ✅               |
| `loom`               | Concurrency testing                    | ✅               |
| `z3-verification`    | Formal verification tests              | ✅               |
| `graphical-examples` | Interactive demos                      | ✅               |

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
    .with_input_delay(2)?
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
        // Handle desync according to your application's needs
        eprintln!("ERROR: Desync detected at frame {frame} — investigation required");
        // Application-specific response: could restart session, alert user, etc.
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
        Some(SyncHealth::DesyncDetected { frame, .. }) => {
            eprintln!("Desync detected at frame {frame:?}");
            break; // Exit with error state for application to handle
        }
        _ => continue, // Keep polling until verified
    }
}
```

See the [Session Termination Anti-Pattern](User-Guide#session-termination-the-last_confirmed_frame-anti-pattern) section in the User Guide for comprehensive examples, edge cases, and solutions.

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

## Session Trait (New)

Fortress Rollback now provides a unified `Session<T>` trait implemented by all session types (`P2PSession`, `SpectatorSession`, `SyncTestSession`). This lets you write generic code that works with any session.

**This is entirely additive — no migration is required.** Existing code using concrete session types continues to work unchanged.

### Adopting the Session Trait

If you have session-specific game loop code, you can optionally generalize it:

```rust
// Before: tied to P2PSession
fn run_frame(session: &mut P2PSession<MyConfig>, input: MyInput) -> FortressResult<()> {
    let player = session.local_player_handles()[0];
    session.add_local_input(player, input)?;
    let requests = session.advance_frame()?;
    // handle requests...
    Ok(())
}

// After: works with any session type
use fortress_rollback::prelude::*;

fn run_frame<T: Config>(
    session: &mut impl Session<T>,
    input: T::Input,
) -> FortressResult<()> {
    let player = session.local_player_handle_required()?;
    session.add_local_input(player, input)?;
    let requests = session.advance_frame()?;
    // handle requests...
    Ok(())
}
```

Key differences when using the trait:

- Use `session.local_player_handle_required()` (returns `Result`) instead of indexing into `local_player_handles()`
- Use `session.events()` to drain events (returns an `EventDrain` iterator)
- `poll_remote_clients()` and `current_state()` work on all session types (with sensible defaults for `SyncTestSession`)
- `network_stats()` is **not** on the trait — use it directly on `P2PSession` or `SpectatorSession`

The trait is available in the prelude: `use fortress_rollback::prelude::*;`

For comprehensive examples including a generic game loop, see the [User Guide — Using the Session Trait](User-Guide#using-the-session-trait).

## Unreleased: Runtime Input Delay, Disconnect Behavior, Graceful Peer Removal, and Spectator Divergence

The forthcoming release introduces fail-closed redundant spectator divergence plus three `P2PSession` capabilities: runtime input-delay adjustment, configurable disconnect behavior, and explicit graceful peer removal. The `P2PSession` behavior is **additive or compatibility-preserving**: existing applications that set input delay at construction time via [`SessionBuilder::with_input_delay`](User-Guide#complete-configuration-reference) keep working, sessions default to `DisconnectBehavior::Halt`, and the legacy `disconnect_player` continues to work unchanged. The spectator divergence behavior affects only failover spectators connected to redundant hosts that disagree. The breaking-change implications are limited to **exhaustive matches** on the public enums listed below.

### Backwards compatibility at a glance

- `SessionBuilder::with_disconnect_behavior` defaults to `DisconnectBehavior::Halt`, which preserves the legacy GGRS-style halt-on-drop semantics. Code that does not call `with_disconnect_behavior` keeps its current behavior.
- `P2PSession::disconnect_player` is unchanged. The new `remove_player` is added alongside it; you only need to migrate to `remove_player` if you want graceful drop.
- `P2PSession::set_input_delay` is a new method. Existing code that fixes the delay at construction time via `with_input_delay` continues to work; mid-session adjustment is opt-in.

### Breaking-change implications for exhaustive matches

`FortressEvent`, `FortressError`, `InvalidRequestKind`, and `InternalErrorKind` are **not** `#[non_exhaustive]`. Code that exhaustively matches on these enums must add arms for the new variants:

#### `FortressEvent` — new variants

```rust
// Before
match event {
    FortressEvent::Synchronizing { .. } => { /* ... */ },
    FortressEvent::Synchronized { .. } => { /* ... */ },
    FortressEvent::Disconnected { .. } => { /* ... */ },
    FortressEvent::NetworkInterrupted { .. } => { /* ... */ },
    FortressEvent::NetworkResumed { .. } => { /* ... */ },
    FortressEvent::WaitRecommendation { .. } => { /* ... */ },
    FortressEvent::DesyncDetected { .. } => { /* ... */ },
    FortressEvent::SyncTimeout { .. } => { /* ... */ },
    FortressEvent::ReplayDesync { .. } => { /* ... */ },
}

// After
match event {
    FortressEvent::Synchronizing { .. } => { /* ... */ },
    FortressEvent::Synchronized { .. } => { /* ... */ },
    FortressEvent::Disconnected { .. } => { /* ... */ },
    FortressEvent::NetworkInterrupted { .. } => { /* ... */ },
    FortressEvent::NetworkResumed { .. } => { /* ... */ },
    FortressEvent::WaitRecommendation { .. } => { /* ... */ },
    FortressEvent::DesyncDetected { .. } => { /* ... */ },
    FortressEvent::SyncTimeout { .. } => { /* ... */ },
    FortressEvent::ReplayDesync { .. } => { /* ... */ },
    // NEW: emitted on graceful drop. Always paired with `Disconnected` in the
    // same batch; see User Guide → Disconnect Behavior and Graceful Peer Drop.
    FortressEvent::PeerDropped { handle, addr } => {
        // Mark the peer as AI-controlled, show "left the game" UI, etc.
        let _ = (handle, addr);
    },
    // NEW: reserved for application-level heuristics. No built-in emitter
    // currently produces this event; you may bind-and-ignore
    // (`InputDelayRecommendation { .. } => {}`) if you do not consume it.
    // Using `_ => {}` would defeat the exhaustive-match check that prompted
    // this migration step.
    FortressEvent::InputDelayRecommendation {
        player_handle,
        current_delay,
        suggested_delay,
    } => {
        let _ = (player_handle, current_delay, suggested_delay);
    },
    // NEW: emitted by failover spectators when connected redundant hosts
    // disagree on the input for the same player/frame. Treat this as a
    // terminal spectator integrity failure and reconnect or abort spectating.
    FortressEvent::SpectatorDivergence {
        frame,
        player,
        primary_addr,
        conflicting_addr,
    } => {
        let _ = (frame, player, primary_addr, conflicting_addr);
    },
}
```

#### `FortressError` — new variant

```rust
// After
match err {
    // ... existing variants ...
    FortressError::SpectatorDivergence { frame, player } => {
        eprintln!(
            "Redundant spectator hosts disagreed for player {player} at frame {frame}"
        );
        // Fail closed: do not keep advancing this spectator session.
    },
}
```

Failover spectators created with `start_spectator_session_multi` no longer use
first-arrival wins for unresolved frames. The canonical source is the
highest-priority currently connected host by the order supplied to
`start_spectator_session_multi`; lower-priority host data is provisional while a
higher-priority host remains connected. If the canonical host disconnects before
a frame resolves, the next surviving host is promoted for unresolved frames only.
Connection status is copied from the chosen host's whole-frame snapshot rather
than merged across hosts. Connected hosts that provide conflicting input for the
same player/frame emit `FortressEvent::SpectatorDivergence`, record a
frame-sync violation, and make future `advance_frame` calls return
`FortressError::SpectatorDivergence`.

#### `InvalidRequestKind` — new variants

```rust
// After
match err_kind {
    // ... existing variants ...
    InvalidRequestKind::InputDelayDecreaseUnsupported { current, requested } => {
        eprintln!(
            "Cannot lower input delay from {current} to {requested} mid-session"
        );
    },
    InvalidRequestKind::InputDelayMidSessionMultiLocalUnsupported { local_players } => {
        eprintln!(
            "Mid-session input-delay increase is not supported with {local_players} local players"
        );
    },
    InvalidRequestKind::InputDelayMidSessionPendingOutputFull { delta, capacity } => {
        eprintln!(
            "Pending-output buffer full: needed {delta} slots, {capacity} available"
        );
    },
    InvalidRequestKind::PlayerAlreadyRemoved { handle } => {
        eprintln!("Peer {handle} was already removed; ignoring duplicate request");
    },
}
```

#### `InternalErrorKind` — new variant

```rust
// After
match internal_kind {
    // ... existing variants ...
    InternalErrorKind::InputQueueGapFillFailed { frame } => {
        // Library invariant violation while replicating gap-fill bytes during
        // a mid-session input-delay increase. Treat as a bug and report.
        eprintln!("internal: input-queue gap-fill failed at frame {frame}");
    },
}
```

If you currently use `_ =>` wildcard arms, no changes are required — but consider replacing the wildcard with explicit arms so future additions are caught at compile time.

#### `Replay::from_bytes` validation and bounds

`Replay::from_bytes()` now uses a replay-specific checked decoder instead of generic bincode container decoding. It requires `I: Copy`, matching the `Config::Input` contract, validates the decoded replay before returning, and rejects trailing bytes. Use `Replay::from_bytes_with_config(bytes, ReplayDecodeConfig::new().max_bytes(limit))` if your application wants to enforce its own replay file-size policy.

#### `RleDecodeReason` — new variant

`RleDecodeReason` (reported via `FortressError::InternalErrorStructured` / `CompressionError::RleDecode`) gains `DecodedLengthExceedsMaximum` and `AllocationFailed` variants. `DecodedLengthExceedsMaximum` is returned when received-input decompression rejects a malformed packet that declares a decoded length above the configured/default limit, instead of attempting an unbounded allocation. `AllocationFailed` is returned when reserving decoded output fails. Exhaustive matches must add arms:

```rust
// After
match reason {
    // ... existing variants ...
    RleDecodeReason::DecodedLengthExceedsMaximum { decoded_len, max } => {
        // A peer sent a decompression bomb: the declared decoded length
        // exceeds the configured/default limit. The packet was dropped; no allocation
        // was attempted. Usually indicates corruption or a malicious peer.
        eprintln!("rejected oversized decode: {decoded_len} > {max}");
    },
    RleDecodeReason::AllocationFailed { requested_len } => {
        eprintln!("could not reserve decoded output: {requested_len}");
    },
}
```

#### `DeltaDecodeReason` — new variant

`DeltaDecodeReason` (reported via `FortressError::InternalErrorStructured` / `CompressionError::DeltaDecode`) gains an `AllocationFailed` variant. It is returned when reserving decoded delta output fails. Exhaustive matches must add an arm:

```rust
// After
match reason {
    // ... existing variants ...
    DeltaDecodeReason::AllocationFailed { context, requested_elements } => {
        eprintln!("could not reserve {requested_elements} elements for {context}");
    },
}
```

If you currently use `_ =>` wildcard arms, no changes are required — but explicit arms catch future additions at compile time.

### Before / After: dynamically adjusting input delay

Previously, the only way to change a session's input delay was at construction time, by branching on measured network conditions and choosing a value before calling `start_p2p_session`. Mid-match adjustments required tearing down and rebuilding the session.

```rust
// Before: input delay is fixed for the lifetime of the session.
let session = SessionBuilder::<GameConfig>::new()
    .with_num_players(2)?
    .add_player(PlayerType::Local, PlayerHandle::new(0))?
    .add_player(PlayerType::Remote(addr), PlayerHandle::new(1))?
    .with_input_delay(2)?
    .start_p2p_session(socket)?;

// To change the delay, you would have to drop the session and rebuild it.
```

```rust
// After: read the current delay and increase it mid-session in response to
// network conditions. Decreases mid-session are not supported.
const MAX_INPUT_DELAY: usize = 8;
let local = PlayerHandle::new(0);
let stats = session.network_stats(remote_handle)?;
let current = session.input_delay(local)?;
if stats.ping > 120 && current < MAX_INPUT_DELAY {
    match session.set_input_delay(local, current.saturating_add(1).min(MAX_INPUT_DELAY)) {
        Ok(()) => { /* applied */ },
        Err(FortressError::InvalidRequestStructured {
            kind: InvalidRequestKind::InputDelayMidSessionPendingOutputFull { .. },
        }) => {
            // Try again next tick after acknowledgements catch up.
        },
        Err(other) => return Err(other),
    }
}
```

See the [User Guide — Adjusting Input Delay at Runtime](User-Guide#adjusting-input-delay-at-runtime) for the full constraint list and a complete example.

### Before / After: handling a peer disconnect gracefully

Previously, the only way to react to a peer disconnect was to observe `FortressEvent::Disconnected` and tear down the session — `P2PSession::disconnect_player` did not freeze the input queue, so under default `Halt` semantics the session simply stopped advancing.

```rust
// Before: a disconnect halts the session. There were two ways to react:
//
// 1. Observe `FortressEvent::Disconnected` from a network-driven drop and
//    tear down the match.
// 2. Call `disconnect_player(handle)` explicitly when the application
//    decided to drop a peer (kick, surrender, etc.). Under default `Halt`
//    semantics this also halts the session because `confirmed_frame()`
//    stops progressing once the peer is marked disconnected; the input
//    queue is **not** frozen and `FortressEvent::PeerDropped` is **not**
//    emitted.
session.disconnect_player(handle)?;
for event in session.events() {
    if let FortressEvent::Disconnected { addr } = event {
        eprintln!("Disconnected from {addr}; tearing down match");
        return Ok(());
    }
}
```

```rust
// After (option 1): opt in to automatic graceful drop on timeout.
let mut session = SessionBuilder::<GameConfig>::new()
    .with_num_players(3)?
    .add_player(PlayerType::Local, PlayerHandle::new(0))?
    .add_player(PlayerType::Remote(a1), PlayerHandle::new(1))?
    .add_player(PlayerType::Remote(a2), PlayerHandle::new(2))?
    .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
    .start_p2p_session(socket)?;

for event in session.events() {
    match event {
        FortressEvent::PeerDropped { handle, addr } => {
            eprintln!("Peer {handle} ({addr}) left; continuing with remaining peers");
        },
        FortressEvent::Disconnected { .. } => { /* paired event; legacy consumers */ },
        _ => {},
    }
}
```

```rust
// After (option 2): drop a specific peer immediately (kick / surrender / leave).
match session.remove_player(conceding_remote) {
    Ok(()) => {},
    Err(FortressError::InvalidRequestStructured {
        kind: InvalidRequestKind::PlayerAlreadyRemoved { .. },
    }) => {
        // Already removed (e.g., a timeout fired first). Treat as a no-op.
    },
    Err(other) => return Err(other),
}
```

The legacy `disconnect_player` is preserved for back-compat. New code should prefer `remove_player` for graceful drops; see [User Guide — Choosing Between `disconnect_player` and `remove_player`](User-Guide#choosing-between-disconnect_player-and-remove_player) for the full distinction.

### Before / After: `handles_by_address` now takes `&T::Address`

`PlayerRegistry::handles_by_address`, `PlayerRegistry::handles_by_address_iter`, and the `P2PSession` forwarders now borrow the address rather than taking ownership. Pass `&addr` instead of `addr` at every call site.

```rust
// Before: address taken by value (cloned at call site for owned variables).
let handles = session.handles_by_address(peer_addr);
for handle in session.handles_by_address_iter(peer_addr.clone()) {
    println!("{handle}");
}
```

```rust
// After: address borrowed; no clone required.
let handles = session.handles_by_address(&peer_addr);
for handle in session.handles_by_address_iter(&peer_addr) {
    println!("{handle}");
}
```

This change is mechanical: add a leading `&` to every call. There are no behavioral changes.

## More Information

For a complete comparison of features, bug fixes, and improvements, see [Fortress vs GGRS](Fortress-vs-GGRS).

## Reporting Issues

Please file new issues on the Fortress Rollback repo: <https://github.com/wallstop/fortress-rollback/issues>
