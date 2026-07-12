<!-- SYNC: This source doc syncs to wiki/Migration.md. -->

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
- **Browser clock migration in 0.10:** callbacks passed to `ChaosSocket::with_clock()` must return `web_time::Instant` instead of `std::time::Instant`; see [Browser ChaosSocket Clock Callbacks](#010-browser-chaossocket-clock-callbacks).
- **0.10 synchronization default:** `SyncConfig::default()` now emits a `SyncTimeout` event after 20 seconds; set `sync_timeout: None` explicitly to retain the previous unlimited-wait behavior.
- **0.10 wire protocol:** all peers in a session must upgrade together; protocol v1 intentionally rejects unversioned 0.9 packets.
- **New in 0.10:** runtime input-delay adjustment (`set_input_delay`/`input_delay`), opt-in graceful peer drop (`DisconnectBehavior::ContinueWithout`, `with_disconnect_behavior`), explicit graceful removal (`remove_player`), and fail-closed redundant spectator divergence; exhaustive matches on `FortressEvent`, `FortressError`, `InvalidRequestKind`, `InternalErrorKind`, `SerializationErrorKind`, `RleDecodeReason`, and `DeltaDecodeReason` need new arms — see [0.10 section](#010-runtime-input-delay-disconnect-behavior-graceful-peer-removal-and-spectator-divergence).

## Dependency Changes

```toml
# Before
[dependencies]
ggrs = "0.11"

# After
[dependencies]
fortress-rollback = "0.10"  # current version
```

If you were using a git/path dependency, point it to the new repository:

```toml
fortress-rollback = { git = "https://github.com/wallstop/fortress-rollback", branch = "main" }
# or
fortress-rollback = { path = "../fortress-rollback" }
```

## 0.9 → 0.10 Wire Protocol

Version 0.10 replaces the unversioned six-byte packet prefix with a protocol-v1
prefix containing sentinel bytes, an exact version, reserved flags, and a 32-bit
connection ID. Mixed 0.9/0.10 sessions cannot synchronize and must be upgraded as
one deployment unit.

| Receiver | Sender | Observation |
| -------- | ------ | ----------- |
| 0.10 | 0.9 | The packet is rejected as suspected legacy traffic and a rate-limited `NetworkProtocol` warning is reported. The default 20-second `SyncTimeout` event still fires while synchronization retries continue. |
| 0.9 | 0.10 | The v1 prefix appears as an unknown legacy body discriminant, so the packet is discarded and the old default waits indefinitely unless the application configured its own timeout. |

The `NonBlockingSocket` typed-message API is unchanged. Custom transports that
receive raw bytes should decode them with `network::codec::decode_message` and
use `classify_wire_bytes` for rate-limited diagnostics. Packet recordings,
byte-preserving relays, and replay fixtures must be re-recorded for protocol v1.

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
| `hot-join`           | Peers can join/rejoin a running session via a state snapshot (requires `Config::State: Serialize + DeserializeOwned`) | ✅               |
| `z3-verification-bundled` | `z3-verification` with a bundled Z3 build (no system Z3 needed) | ✅               |

> **Note:** The `json` feature enables `to_json()` and `to_json_pretty()` methods on telemetry types.
> Without this feature, the `serde_json` dependency is not included, reducing the default dependency count.

For detailed feature documentation, see the [User Guide](user-guide.md#feature-flags).

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

See the [User Guide - Complete Configuration Reference](user-guide.md#complete-configuration-reference) for full documentation.

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

See the [Session Termination Anti-Pattern](user-guide.md#session-termination-the-last_confirmed_frame-anti-pattern) section in the User Guide for comprehensive examples, edge cases, and solutions.

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

For comprehensive examples including a generic game loop, see the [User Guide — Using the Session Trait](user-guide.md#using-the-session-trait).

## 0.10: Browser ChaosSocket Clock Callbacks

`ChaosSocket::with_clock()` now accepts callbacks that return
`web_time::Instant`. The default clock needs no migration. Browser
`wasm32-unknown-unknown` callers with an injected clock must replace an explicit
`std::time::Instant` import and add `web-time` as a direct dependency:

```toml
[dependencies]
web-time = "1.1"
```

### Before / After: browser custom clock

```rust
// Before: this callback returns std::time::Instant.
use std::{sync::Arc, time::Instant};

let base = Instant::now();
let socket = ChaosSocket::new(inner_socket, chaos_config)
    .with_clock(Arc::new(move || base));
```

```rust
// After: this callback returns web_time::Instant on browser WASM.
use std::sync::Arc;
use web_time::Instant;

let base = Instant::now();
let socket = ChaosSocket::new(inner_socket, chaos_config)
    .with_clock(Arc::new(move || base));
```

Native and `wasm32-unknown-emscripten` callers remain source-compatible because
`web_time` re-exports `std::time::Instant` on those targets. Using the
cross-platform import everywhere keeps one clock implementation portable across
native, browser, and Godot Web builds.

## 0.10: Protocol-v1 Session Compatibility Handshake

Protocol-v1 peers now exchange deterministic session settings during the sync
handshake. Mixed 0.9/v1 traffic is rejected by framing before synchronization;
two v1 peers whose compatibility floor, player count, serialized input width,
FPS, maximum prediction, desync interval, compiled protocol features, or
canonical digest differ emit
`FortressEvent::IncompatibleSession` and remain synchronizing without further
retries or a later timeout.

Applications that exhaustively match `FortressEvent` must add an arm:

```rust
FortressEvent::IncompatibleSession { addr, reason } => {
    eprintln!("cannot synchronize with {addr}: {reason}");
    // Tear down this session and rebuild it with matching settings.
}
```

Applications that exhaustively match metrics categories must likewise add
`EventKind::IncompatibleSession`; its stable string is
`"incompatible_session"`.

The reason's `ours` and `theirs` values are oriented to the endpoint emitting
the event. `DisconnectBehavior` is intentionally not compared because it is a
local response policy. Custom socket APIs are unchanged, but byte recorders,
relays, and fixtures must use the new fixed-width sync request/reply bodies.

Network startup now rejects values that cannot be represented by the handshake
(`u16` player count/input width/prediction window and `u32` FPS/checksum
interval), plus `DesyncDetection::On { interval: 0 }`. Use
`DesyncDetection::Off` to disable checksum comparison.

## 0.10: Runtime Input Delay, Disconnect Behavior, Graceful Peer Removal, and Spectator Divergence

The forthcoming release introduces fail-closed redundant spectator divergence plus three `P2PSession` capabilities: runtime input-delay adjustment, configurable disconnect behavior, and explicit graceful peer removal. The `P2PSession` behavior is **additive or compatibility-preserving** for configurations whose prediction window plus input delay fits the input queue; previously unsafe larger combinations now fail construction as described below. Sessions default to `DisconnectBehavior::Halt`, and the legacy `disconnect_player` continues to work unchanged. The spectator divergence behavior affects only failover spectators connected to redundant hosts that disagree. The breaking-change implications are limited to **exhaustive matches** on the public enums listed below.

### Backwards compatibility at a glance

- `SessionBuilder::with_disconnect_behavior` defaults to `DisconnectBehavior::Halt`, which preserves the legacy GGRS-style halt-on-drop semantics. Code that does not call `with_disconnect_behavior` keeps its current behavior.
- `P2PSession::disconnect_player` is unchanged. The new `remove_player` is added alongside it; you only need to migrate to `remove_player` if you want graceful drop.
- `P2PSession::set_input_delay` is a new method. Existing safe construction-time delays continue to work; session construction and runtime increases now require `max_prediction + input_delay < input_queue_config.queue_length`. Configurations outside that bound previously risked overwriting rollback history during a full recovery batch and now return `InvalidRequestKind::ConfigValueOutOfRange` with field `"max_prediction + input_delay"`. Increase the input queue or reduce prediction/delay to migrate an unsafe combination.

### Breaking-change implications for exhaustive matches

`FortressEvent`, `FortressError`, `InvalidRequestKind`, `InternalErrorKind`, and `SerializationErrorKind` are **not** `#[non_exhaustive]`. Code that exhaustively matches on these enums must add arms for the new variants:

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

#### `InternalErrorKind` — new variants

```rust
// After
match internal_kind {
    // ... existing variants ...
    InternalErrorKind::DeltaEncodeEmptyReference => {
        eprintln!("internal: tried to delta-encode an empty reference frame");
    },
    InternalErrorKind::DeltaEncodeInputLengthMismatch { input_len, reference_len } => {
        eprintln!(
            "internal: input frame width {input_len} did not match reference width {reference_len}"
        );
    },
    InternalErrorKind::InputEncodeLengthMismatch {
        player,
        input_len,
        expected_len,
    } => {
        eprintln!(
            "internal: player {player} input encoded to {input_len} bytes, expected {expected_len}"
        );
    },
    InternalErrorKind::InputQueueGapFillFailed { frame } => {
        // Library invariant violation while replicating gap-fill bytes during
        // a mid-session input-delay increase. Treat as a bug and report.
        eprintln!("internal: input-queue gap-fill failed at frame {frame}");
    },
}
```

If you currently use `_ =>` wildcard arms, no changes are required — but consider replacing the wildcard with explicit arms so future additions are caught at compile time.

#### `SerializationErrorKind` — new variants

Network sessions now require `Config::Input::default()` to serialize to at
least one byte. Zero-byte input types cannot be represented by the input delta
stream because the receiver splits decoded bytes into fixed-width frames. Start
methods return `SerializationErrorKind::InputSerializedSizeZero` instead of
constructing an endpoint that can never send or receive input frames.
They also reject local or remote aggregate input frames larger than
`fortress_rollback::rle::DEFAULT_MAX_DECODED_LEN`, returning
`SerializationErrorKind::InputSerializedFrameTooLarge`.

```rust
// After
match serialization_kind {
    // ... existing variants ...
    SerializationErrorKind::InputSerializedSizeZero => {
        eprintln!("Config::Input must serialize to at least one byte");
    },
    SerializationErrorKind::InputSerializedFrameTooLarge { frame_len, max } => {
        eprintln!(
            "Config::Input aggregate frame is {frame_len} bytes, above receive cap {max}"
        );
    },
}
```

`Config::Input` values used in network sessions should also serialize to the
same byte length for every player and every value. Prefer structs of fixed-width
numeric and boolean fields. Avoid variable-length enums, strings, vectors, maps,
and other payloads whose encoded size can change per frame.

#### `Replay::from_bytes` validation and bounds

`Replay::from_bytes()` now uses a replay-specific checked decoder instead of generic bincode container decoding. It requires `I: Copy`, matching the `Config::Input` contract, validates the decoded replay before returning, and rejects trailing bytes. Use `Replay::from_bytes_with_config(bytes, ReplayDecodeConfig::new().max_bytes(limit))` if your application wants to enforce its own replay file-size policy.

#### `RleDecodeReason` — new variants

`RleDecodeReason` (reported via `FortressError::InternalErrorStructured` / `CompressionError::RleDecode`) gains `MalformedVarint`, `DecodedLengthExceedsMaximum`, and `AllocationFailed` variants. `MalformedVarint` is returned when an encoded run-length prefix cannot be decoded as a valid varint. `DecodedLengthExceedsMaximum` is returned when received-input decompression rejects a malformed packet that declares a decoded length above the configured/default limit, instead of attempting an unbounded allocation. `AllocationFailed` is returned when reserving decoded output fails. Exhaustive matches must add arms:

```rust
// After
match reason {
    // ... existing variants ...
    RleDecodeReason::MalformedVarint { offset } => {
        eprintln!("malformed RLE varint at offset {offset}");
    },
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

#### `DeltaDecodeReason` — new variants

`DeltaDecodeReason` (reported via `FortressError::InternalErrorStructured` / `CompressionError::DeltaDecode`) gains `DecodedFrameCountExceedsMaximum` and `AllocationFailed` variants. `DecodedFrameCountExceedsMaximum` is returned when delta decoding would split a decoded byte stream into too many per-frame buffers. `AllocationFailed` is returned when reserving decoded delta output fails. Exhaustive matches must add arms:

```rust
// After
match reason {
    // ... existing variants ...
    DeltaDecodeReason::DecodedFrameCountExceedsMaximum { frame_count, max } => {
        eprintln!("decoded too many frames: {frame_count} > {max}");
    },
    DeltaDecodeReason::AllocationFailed { context, requested_elements } => {
        eprintln!("could not reserve {requested_elements} elements for {context}");
    },
}
```

If you currently use `_ =>` wildcard arms, no changes are required — but explicit arms catch future additions at compile time.

#### `ProtocolConfig::pending_output_limit` — maximum value

`ProtocolConfig::pending_output_limit` now has a hard maximum:
`ProtocolConfig::MAX_PENDING_OUTPUT_LIMIT`. Larger values return
`InvalidRequestKind::ConfigValueOutOfRange` during configuration validation.
The limit keeps valid send batches aligned with the compression decoder's
per-frame output cap.

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

See the [User Guide — Adjusting Input Delay at Runtime](user-guide.md#adjusting-input-delay-at-runtime) for the full constraint list and a complete example.

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
// After (option 2): propose a coordinated drop (kick / surrender / leave).
match session.remove_player(conceding_remote) {
    // Intent accepted. Keep polling until PeerDropped proves commit.
    Ok(()) => {},
    Err(FortressError::InvalidRequestStructured {
        kind: InvalidRequestKind::PlayerAlreadyRemoved { .. },
    }) => {
        // Already removed (e.g., a timeout fired first). Treat as a no-op.
    },
    Err(other) => return Err(other),
}
```

The legacy `disconnect_player` is preserved for back-compat. New code should prefer `remove_player` for graceful drops, but must account for its asynchronous certificate: keep calling `poll_remote_clients`, treat `PeerDropped` as commit evidence, and treat a return to `Synchronizing` without that event as fail-closed. See [User Guide — Choosing Between `disconnect_player` and `remove_player`](user-guide.md#choosing-between-disconnect_player-and-remove_player) for the full distinction.

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

For a complete comparison of features, bug fixes, and improvements, see [Fortress vs GGRS](fortress-vs-ggrs.md).

## Reporting Issues

Please file new issues on the Fortress Rollback repo: <https://github.com/wallstop/fortress-rollback/issues>
