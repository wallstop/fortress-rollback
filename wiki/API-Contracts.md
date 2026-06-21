<!-- SYNC: This wiki page is generated from docs/specs/api-contracts.md. Edit docs source. -->

<p align="center">
  <img src="assets/logo-small.svg" alt="Fortress Rollback" width="64">
</p>

# Fortress Rollback API Contracts

**Version:** 1.0
**Date:** December 6, 2025
**Status:** Complete

This document specifies preconditions, postconditions, and invariants for all public APIs. It complements formal-spec.md and serves as a reference for verification and documentation.

---

## Table of Contents

1. [Contract Notation](#contract-notation)
2. [SessionBuilder](#sessionbuilder)
3. [P2PSession](#p2psession)
4. [SpectatorSession](#spectatorsession)
5. [SyncTestSession](#synctestsession)
6. [GameStateCell](#gamestatecell)
7. [Request Handling](#request-handling)
8. [Error Catalog](#error-catalog)
9. [Event Catalog](#event-catalog)
10. [Cross-Cutting Invariants](#cross-cutting-invariants)
11. [Revision History](#revision-history)

---

## Contract Notation

Each API is documented with:

- **Signature**: The function signature
- **Pre**: Preconditions that must hold before calling
- **Post**: Postconditions guaranteed after successful return
- **Errors**: Conditions that cause specific errors
- **Panics**: Should always be "Never" for public APIs
- **Invariants**: Properties preserved across the call

---

## SessionBuilder

### `SessionBuilder::new() -> Self`

```rust
/// Creates a new session builder with default configuration.
```

**Pre:** None

**Post:**

- `num_players = 2`
- `max_prediction = 8`
- `fps = 60`
- `input_delay = 0`
- `save_mode = SaveMode::EveryFrame`
- `desync_detection = On { interval: 60 }`
- `disconnect_timeout = 2000ms`
- `disconnect_notify_start = 500ms`

**Errors:** None

**Panics:** Never

---

### `with_num_players(self, n: usize) -> Result<Self, FortressError>`

```rust
/// Set the number of active players (not spectators).
```

**Pre:** `n > 0`

**Post:** `self.num_players = n`

**Errors:**

- `InvalidRequestStructured { kind: ZeroPlayers }` - if `n = 0`

**Panics:** Never

---

### `add_player(self, player_type: PlayerType, handle: PlayerHandle) -> Result<Self, FortressError>`

```rust
/// Register a player with the session.
```

**Pre:**

- `handle` not already registered
- For `Local` or `Remote`: `handle.0 < num_players`
- For `Spectator`: `handle.0 >= num_players`

**Post:**

- Player registered with given type
- For `Local`: `local_players += 1`

**Errors:**

- `InvalidRequestStructured { kind: PlayerHandleInUse { handle } }` - handle duplicate
- `InvalidRequestStructured { kind: InvalidLocalPlayerHandle { handle, num_players } }` / `InvalidRequestStructured { kind: InvalidRemotePlayerHandle { handle, num_players } }` - invalid Local/Remote handle (`handle.0 >= num_players`)
- `InvalidRequestStructured { kind: InvalidSpectatorHandle { handle, num_players } }` - invalid spectator handle (`handle.0 < num_players`)

**Panics:** Never

---

### `with_max_prediction_window(self, window: usize) -> Self`

```rust
/// Set maximum prediction frames. 0 = lockstep mode.
```

**Pre:** None

**Post:**

- `self.max_prediction = window`
- `window = 0` → session operates in lockstep (no rollbacks)

**Errors:** None

**Panics:** Never

---

### `with_input_delay(self, delay: usize) -> Result<Self, FortressError>`

```rust
/// Set input delay for local players.
```

**Pre:** `delay <= queue_length - 1` (default max: 127)

**Post:** `self.input_delay = delay`

**Errors:**

- `InvalidRequestStructured { kind: FrameDelayTooLarge { delay, max_delay } }` - if `delay` exceeds `input_queue_config.max_frame_delay()`

**Panics:** Never

---

### `with_fps(self, fps: usize) -> Result<Self, FortressError>`

```rust
/// Set expected update frequency.
```

**Pre:** `fps > 0`

**Post:** `self.fps = fps`

**Errors:**

- `InvalidRequestStructured { kind: ZeroFps }` - if `fps = 0`

**Panics:** Never

---

### `with_desync_detection_mode(self, mode: DesyncDetection) -> Self`

```rust
/// Enable/disable checksum-based desync detection.
```

**Pre:** None

**Post:** `self.desync_detection = mode`

**Errors:** None

**Panics:** Never

---

### `with_sparse_saving_mode(self, sparse_saving: bool) -> Self` *(deprecated)*

> **Deprecated since 0.2.0:** Use `with_save_mode(SaveMode::Sparse)` instead.

```rust
/// Enable sparse saving (fewer saves, longer potential rollbacks).
```

**Pre:** None

**Post:**

- `sparse_saving = true` → `self.save_mode = SaveMode::Sparse`
- `sparse_saving = false` → `self.save_mode = SaveMode::EveryFrame`

**Errors:** None

**Panics:** Never

---

### `with_disconnect_timeout(self, timeout: Duration) -> Self`

```rust
/// Set peer disconnect timeout.
```

**Pre:** None

**Post:** `self.disconnect_timeout = timeout`

**Errors:** None

**Panics:** Never

---

### `with_disconnect_behavior(self, behavior: DisconnectBehavior) -> Self`

```rust
/// Configure how a P2PSession reacts when the disconnect timeout fires for a
/// remote peer.
```

**Pre:** None

**Post:** `self.disconnect_behavior = behavior`

**Errors:** None

**Panics:** Never

**Notes:**

- Default is `DisconnectBehavior::Halt`, preserving the legacy GGRS-style halt-on-drop semantics.
- `DisconnectBehavior::ContinueWithout` enables graceful peer drop on the **automatic** disconnect-timeout path: the dropped peer's input queue is frozen at the last confirmed input, `FortressEvent::PeerDropped` and `FortressEvent::Disconnected` are both emitted, and remaining peers continue advancing.
- The setting governs only the automatic-timeout path. The explicit `P2PSession::remove_player` always performs a graceful drop regardless of this setting; the legacy `P2PSession::disconnect_player` retains its non-graceful semantics regardless of this setting.

---

### `start_p2p_session(self, socket: impl NonBlockingSocket<T::Address> + 'static) -> Result<P2PSession<T>, FortressError>`

```rust
/// Consume builder and create a P2P session.
```

**Pre:**

- All player handles `0..num_players` have been registered via `add_player`
- At least one local player

**Post:**

- Session created in `Synchronizing` state
- All remote endpoints begin synchronization
- Socket ownership transferred to session

**Errors:**

- `InvalidRequestStructured { kind: NotEnoughPlayers { expected, actual } }` - not all player handles `0..num_players` have been registered

**Panics:** Never

**Invariants Established:**

- INV-4: Queue length bounds
- INV-5: Queue index validity
- INV-11: No panics guarantee

---

### `start_spectator_session(self, host_addr: T::Address, socket: impl NonBlockingSocket<T::Address> + 'static) -> Option<SpectatorSession<T>>`

```rust
/// Create a spectator session connected to a host.
```

**Pre:** None (no player registration required)

**Post:**

- Returns `Some(session)` with session created in `Synchronizing` state
- Host endpoint begins synchronization
- Returns `None` if protocol configuration validation, spectator configuration
  validation, or protocol initialization fails (e.g., serialization issues)

**Spectator configuration validation:**

- `SpectatorConfig::buffer_size` must be greater than `0`
- `SpectatorConfig::stream_delay` must be smaller than `buffer_size`
- `SpectatorConfig::catchup_speed == 0` is accepted; if catch-up mode is
  reached with zero speed, no frame is attempted and `advance_frame` returns
  `Ok(<empty>)`

**Errors:** None (returns `Option`, not `Result`)

**Panics:** Never

---

### `start_synctest_session(self) -> Result<SyncTestSession, FortressError>`

```rust
/// Create a local determinism testing session.
```

**Pre:** `check_distance < max_prediction`

**Post:**

- Session created (no network, immediate Running state equivalent)
- Rollback simulation enabled

**Errors:**

- `InvalidRequestStructured { kind: CheckDistanceTooLarge { check_dist, max_prediction } }` - if `check_distance >= max_prediction`

**Panics:** Never

---

## P2PSession

### `current_state(&self) -> SessionState`

```rust
/// Get the current session state.
```

**Pre:** None

**Post:** Returns `Synchronizing` or `Running`

**Errors:** None

**Panics:** Never

---

### `local_player_handles(&self) -> HandleVec`

```rust
/// Get handles of all local players.
```

**Pre:** None

**Post:** Returns `HandleVec` of handles where `player_type = Local`

**Errors:** None

**Panics:** Never

---

### `poll_remote_clients(&mut self)`

```rust
/// Process incoming network messages.
```

**Pre:** None

**Post:**

- All pending messages from socket processed
- Input queues updated with remote inputs
- Protocol state machines advanced
- Events queued for retrieval

**Errors:** None (errors converted to events)

**Panics:** Never

**Side Effects:**

- May trigger state transitions (Synchronizing → Running)
- May queue `Synchronized`, `Disconnected`, `NetworkInterrupted` events

---

### `add_local_input(&mut self, handle: PlayerHandle, input: T::Input) -> Result<(), FortressError>`

```rust
/// Add input for a local player.
```

**Pre:**

- `handle` is a local player
- `current_state() = Running` OR input is being buffered
- Not exceeding prediction threshold

**Post:**

- Input stored in `local_inputs` map
- Input will be transmitted to remotes on next `advance_frame`

**Errors:**

- `InvalidRequestStructured { kind: NotLocalPlayer { handle } }` - handle is not a registered local player

**Panics:** Never

---

### `advance_frame(&mut self) -> FortressResult<RequestVec<T>>`

```rust
/// Advance the simulation by one frame, handling rollbacks as needed.
```

**Pre:**

- `current_state() = Running`
- All local players have provided input via `add_local_input`

**Post:**

- Returns sequence of requests to be processed **in order**
- `current_frame` incremented (after processing requests)
- If rollback needed: `LoadGameState` followed by `SaveGameState`/`AdvanceFrame` pairs
- If no rollback: `SaveGameState` (unless sparse) then `AdvanceFrame`

**Errors:**

- `NotSynchronized` - if `current_state() != Running`
- `InvalidRequestStructured { kind: MissingLocalInput }` - not all local players provided input

**Panics:** Never

**Request Sequence (no rollback, full saving):**

```
[SaveGameState { frame: N }, AdvanceFrame { inputs }]
```

**Request Sequence (with rollback):**

```
[LoadGameState { frame: K },
 SaveGameState { frame: K }, AdvanceFrame { inputs_K },
 SaveGameState { frame: K+1 }, AdvanceFrame { inputs_K+1 },
 ...
 SaveGameState { frame: N }, AdvanceFrame { inputs_N }]
```

**Invariants Preserved:**

- INV-1: Frame monotonicity (within rollback bounds)
- INV-2: Rollback boundedness
- INV-7: Confirmed frame consistency
- INV-8: Saved frame consistency

---

### `events(&mut self) -> EventDrain<'_, T>`

```rust
/// Drain all pending events.
```

**Pre:** None

**Post:**

- Returns iterator over pending events
- Event queue emptied

**Errors:** None

**Panics:** Never

---

### `frames_ahead(&self) -> i32`

```rust
/// Get recommended frame delay (for pacing).
```

**Pre:** None

**Post:** Returns frame advantage estimate

**Errors:** None

**Panics:** Never

---

### `network_stats(&self, handle: PlayerHandle) -> Result<NetworkStats, FortressError>`

```rust
/// Get network statistics for a remote player.
```

**Pre:** `handle` is a remote player or spectator

**Post:** Returns stats (ping, bandwidth, etc.)

**Errors:**

- `InvalidRequestStructured { kind: NotRemotePlayerOrSpectator { handle } }` - handle is neither a remote player nor a spectator
- `NotSynchronized` - stats not yet available

**Panics:** Never

---

### `disconnect_player(&mut self, handle: PlayerHandle) -> Result<(), FortressError>`

```rust
/// Manually disconnect a player (legacy halt-on-drop semantics).
```

**Pre:** None — all caller-side conditions are validated and returned via the Errors section below.

**Post:**

- Every player handle owned by the dropped endpoint is marked as disconnected on the local connection-status table (multi-handle endpoints — multiple handles sharing a single address — are wound down in full)
- The corresponding network endpoint is disconnected
- Future inputs for any disconnected handle use the default value (the input queue is **not** frozen — see `remove_player` for graceful drop, which freezes the queue and replays the last confirmed input)

**Errors:**

- `InvalidRequestStructured { kind: DisconnectInvalidHandle { handle } }` - handle not registered
- `InvalidRequestStructured { kind: DisconnectLocalPlayer { handle } }` - handle refers to a local player
- `InvalidRequestStructured { kind: AlreadyDisconnected { handle } }` - handle was already disconnected
- `InternalErrorStructured { kind: DisconnectStatusNotFound { handle } }` - internal-invariant violation (a registered remote handle has no corresponding connection-status entry); should not occur in correct code, treat as a library bug

**Panics:** Never

**Notes:**

- Does **not** freeze the player's input queue and does **not** emit `FortressEvent::PeerDropped`.
- Always preserves halt-on-drop semantics regardless of the configured `DisconnectBehavior`: remaining peers no longer produce confirmed inputs from the dropped peer's endpoint, so `advance_frame` cannot make progress past that peer's last confirmed frame.
- For an explicit graceful drop, prefer `remove_player`.
- When `player_handle` is **Remote**, operates on the Remote endpoint at the address only — a `Spectator` endpoint registered at the same `T::Address` is independent and is **not** affected, remaining running until it disconnects on its own. When `player_handle` is **Spectator**, only that specific spectator endpoint is disconnected; any Remote endpoint at the same address is left running. Co-locating a `Remote` and a `Spectator` at the same address is unusual; this note documents the behavior for that edge case.

---

### `remove_player(&mut self, player_handle: PlayerHandle) -> Result<(), FortressError>`

```rust
/// Remove a remote player from the session and continue with the remaining
/// peers (graceful drop), regardless of the configured DisconnectBehavior.
```

**Pre:** None — all caller-side conditions are validated and returned via the Errors section below.

**Post:**

- Every non-spectator player handle owned by the dropped endpoint is marked disconnected on the local connection-status table
- Every non-spectator handle's input queue is **frozen**: it repeats its last confirmed input forever for remaining peers' simulation
- The corresponding network endpoint is disconnected
- One `FortressEvent::PeerDropped { handle, addr }` per non-spectator handle at the dropped address is queued, **followed by** exactly one `FortressEvent::Disconnected { addr }` in the same batch
- `confirmed_frame()` continues to advance for remaining peers

**Errors:**

- `InvalidRequestStructured { kind: DisconnectInvalidHandle { handle } }` - handle not registered, or refers to a spectator
- `InvalidRequestStructured { kind: DisconnectLocalPlayer { handle } }` - handle refers to a local player
- `InvalidRequestStructured { kind: PlayerAlreadyRemoved { handle } }` - handle is already marked disconnected (either via a prior `remove_player` call, via auto-removal under `DisconnectBehavior::ContinueWithout`, or via a previous explicit `disconnect_player` call)
- `InternalErrorStructured { kind: DisconnectStatusNotFound { handle } | IndexOutOfBounds(..) }` - internal-invariant violation (a registered handle has no corresponding input queue or connection-status entry); should not occur in correct code, treat as a library bug

**Panics:** Never

**Notes:**

- Always opts in to graceful-drop semantics regardless of the session's `DisconnectBehavior`. The configured `DisconnectBehavior` only governs the **automatic** disconnect-timeout path.
- The `PeerDropped` event coexists with the legacy `Disconnected` event; new code should match on `PeerDropped` for graceful-drop-aware handling.
- Operates on the **Remote** endpoint at the targeted address only. A `Spectator` endpoint registered at the same `T::Address` is an independent endpoint and is **not** affected — it remains running until it disconnects on its own. Co-locating a `Remote` and a `Spectator` at the same address is unusual; this note documents the behavior for that edge case.

---

### `disconnect_behavior(&self) -> DisconnectBehavior`

```rust
/// Return the configured DisconnectBehavior for this session.
```

**Pre:** None

**Post:** Returns the `DisconnectBehavior` set via `SessionBuilder::with_disconnect_behavior` (default `Halt`).

**Errors:** None

**Panics:** Never

---

### `set_input_delay(&mut self, player_handle: PlayerHandle, delay: usize) -> Result<(), FortressError>`

```rust
/// Adjust the input delay for a local player at runtime.
```

**Pre:** `delay` is within the configured `max_frame_delay()` of the input queue (set via `InputQueueConfig::max_frame_delay`; defaults to `queue_length - 1`). All other caller-side conditions are validated and returned via the Errors section below.

**Post:**

- The local player's frame-delay is set to `delay`
- **No-op case** (`delay == current_delay`): no further side effects
- **Initial-setup case** (no inputs added yet): the new delay applies cleanly with no gap-fill replication. Decreases are also permitted in this case.
- **Mid-session increase case** (`delay > current_delay` after inputs have been added on a peer with exactly one local player):
  - The input queue replicates the most recently added input across `delta = delay - current_delay` new gap frames
  - The same replicated frames are pushed onto every remote endpoint's pending-output buffer and flushed
  - The local connection-status `last_frame` is advanced to match the queue's new `last_added_frame`
  - Remote peers' input sequences remain strictly monotonic

**Errors:**

- `InvalidRequestStructured { kind: NotLocalPlayer { handle } }` - handle is not a local player
- `InvalidRequestStructured { kind: FrameDelayTooLarge { delay, max_delay } }` - `delay` exceeds `queue_length - 1`
- `InvalidRequestStructured { kind: InputDelayDecreaseUnsupported { current, requested } }` - `requested < current` and inputs have already been added
- `InvalidRequestStructured { kind: InputDelayMidSessionMultiLocalUnsupported { local_players } }` - mid-session increase attempted with more than one local player on this peer
- `InvalidRequestStructured { kind: InputDelayMidSessionPendingOutputFull { delta, capacity } }` - mid-session increase would push more gap-fill frames into a remote's pending-output buffer than the configured `pending_output_limit` allows
- `InternalErrorStructured { kind: InputQueueGapFillFailed { frame } }` - internal invariant violation while replicating gap-fill bytes (should be reported as a bug)

**Panics:** Never

**Invariants Preserved:**

- INV-3 (Input Immutability): confirmed inputs are not modified by gap-fill replication
- INV-4 (Queue Bounds): the queue length is unchanged

---

### `input_delay(&self, player_handle: PlayerHandle) -> Result<usize, FortressError>`

```rust
/// Return the current input delay (in frames) for a local player.
```

**Pre:** None — all caller-side conditions are validated and returned via the Errors section below.

**Post:** Returns the current frame-delay for `player_handle`

**Errors:**

- `InvalidRequestStructured { kind: NotLocalPlayer { handle } }` - handle is not a local player

**Panics:** Never

---

## SpectatorSession

### `advance_frame(&mut self) -> FortressResult<RequestVec<T>>`

```rust
/// Advance spectator simulation.
```

**Pre:** `current_state() = Running`

**Post:**

- Returns one `AdvanceFrame` request per advanced frame.
- If `SpectatorConfig::enable_rewind` is enabled, each advanced frame is
  preceded by a `SaveGameState` request for the same frame label.
- May return multiple frames if catching up.
- A failover spectator with no remaining hosts may still advance through
  already-buffered frames. After the buffered viewable frames drain,
  `PredictionThreshold` is returned.
- For redundant hosts, unresolved frames use the highest-priority currently
  connected host by `start_spectator_session_multi` order as the canonical
  source. Lower-priority host snapshots are provisional while a higher-priority
  host remains connected; if the higher-priority host disconnects before a
  frame resolves, the next surviving host is promoted only for unresolved
  frames.
- Host connection status is copied from the selected canonical host's
  whole-frame snapshot and is never synthesized player-by-player across hosts.
- Connected redundant hosts that disagree on the same player/frame emit
  `FortressEvent::SpectatorDivergence`, record a frame-sync violation, and
  latch a terminal `FortressError::SpectatorDivergence` for future
  `advance_frame` calls. Already advanced frames are not rewritten.
- Duplicate host addresses route inbound packets to the first matching endpoint.

**Errors:**

- `NotSynchronized` - not yet synchronized with host
- `PredictionThreshold` - no viewable frame is available yet, or all cleanly
  disconnected hosts have drained their buffered viewable frames
- `SpectatorDivergence { frame, player }` - connected redundant hosts disagreed
  on the same player/frame and the spectator has failed closed

**Panics:** Never

---

## SyncTestSession

### `advance_frame(&mut self) -> FortressResult<RequestVec<T>>`

```rust
/// Advance with automatic rollback testing.
```

**Pre:** All local inputs provided

**Post:**

- Simulates rollback of `check_distance` frames
- Compares checksums for mismatch detection
- Returns requests including save/load/advance

**Errors:**

- `MismatchedChecksum { current_frame, mismatched_frames }` on checksum mismatch (desync detected during resimulation)
- `InvalidRequestStructured { kind: MissingLocalInput }` - not all players provided input

**Panics:** Never

---

## GameStateCell

### `save(&self, frame: Frame, state: Option<T::State>, checksum: Option<u128>) -> bool`

```rust
/// Save game state into the cell.
```

**Pre:**

- Called in response to `SaveGameState` request
- `frame` matches request frame

**Post:**

- Returns `true` if the save succeeded
- Returns `false` if `frame` is `Frame::NULL` (save rejected)
- State stored and retrievable via `load()`
- Checksum stored for desync detection (if provided)

**Errors:** None

**Panics:** Never

---

### `load(&self) -> Option<T::State>`

```rust
/// Load game state from the cell.
```

**Pre:** `save()` was previously called

**Post:** Returns cloned state

**Errors:** None (returns `None` if empty)

**Panics:** Never

---

## Request Handling

### Processing Order Contract

**CRITICAL:** Requests from `advance_frame()` MUST be processed in the exact order returned.

```rust
// CORRECT
for request in session.advance_frame()? {
    match request {
        FortressRequest::LoadGameState { cell, .. } => { /* load */ }
        FortressRequest::SaveGameState { cell, frame } => { /* save */ }
        FortressRequest::AdvanceFrame { inputs } => { /* advance */ }
    }
}

// INCORRECT - DO NOT reorder or skip requests
```

### SaveGameState Contract

```text
FortressRequest::SaveGameState { cell, frame }
```

**Pre:** `game_state.frame == frame` (your state matches requested frame)

**Post (after handling):**

- `cell.save(frame, Some(state), checksum)` called
- State is now loadable for rollback

**User Responsibility:**

- Clone entire game state
- Compute checksum if desync detection enabled
- Call `cell.save()` before processing next request

### LoadGameState Contract

```text
FortressRequest::LoadGameState { cell, frame }
```

**Pre:** State was previously saved at `frame`

**Post (after handling):**

- `if let Some(state) = cell.load() { game_state = state; }`
- Game state restored to frame `frame`

**User Responsibility:**

- Replace entire game state with loaded state
- Subsequent `AdvanceFrame` requests will resimulate

### AdvanceFrame Contract

```text
FortressRequest::AdvanceFrame { inputs }
```

**Pre:** Game state is at the correct frame

**Post (after handling):**

- Game state advanced by one frame
- `game_state.frame += 1` (or equivalent)

**User Responsibility:**

- Apply all inputs to game state deterministically
- Handle `InputStatus::Disconnected` appropriately
- Increment frame counter

---

## Error Catalog

### Legacy Variants

| Error                                        | Cause                                | Recovery                      |
| -------------------------------------------- | ------------------------------------ | ----------------------------- |
| `InvalidRequest { info }`                    | Invalid operation/parameter (legacy) | Check info message, fix call  |
| `InvalidPlayerHandle { handle, max_handle }` | Handle out of range or wrong type    | Use valid handle              |
| `InvalidFrame { frame, reason }`             | Frame out of valid range (legacy)    | Check frame bounds            |
| `NotSynchronized`                            | Operation requires Running state     | Wait for sync or call poll    |
| `MissingInput { player_handle, frame }`      | Confirmed input not available        | Internal error, report bug    |
| `PredictionThreshold`                        | Prediction window exceeded           | Wait before adding more input |

### Structured Variants (Preferred)

| Error                                                   | Cause                                    | Recovery                               |
| ------------------------------------------------------- | ---------------------------------------- | -------------------------------------- |
| `InvalidRequestStructured { kind }`                     | Invalid operation with structured reason | Match on `InvalidRequestKind` variants |
| `InvalidFrameStructured { frame, reason }`              | Frame invalid with structured reason     | Match on `InvalidFrameReason` variants |
| `InternalErrorStructured { kind }`                      | Library bug with structured context      | Report bug with error details          |
| `SerializationErrorStructured { kind }`                 | Serialization failure                    | Check input data format                |
| `FrameArithmeticOverflow { frame, operand, operation }` | Frame arithmetic overflow                | Check frame bounds                     |

### Selected `InvalidRequestKind` Variants — Runtime Input Delay and Peer Removal

| Variant                                                       | Source API                                                | Cause                                                                                                                                                                                                        | Recovery                                                                                                        |
| ------------------------------------------------------------- | --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------- |
| `InputDelayDecreaseUnsupported { current, requested }`        | `P2PSession::set_input_delay`                             | `requested < current` after inputs have been added                                                                                                                                                           | Mid-session decreases are not supported; carry the lower delay over to the next session                         |
| `InputDelayMidSessionMultiLocalUnsupported { local_players }` | `P2PSession::set_input_delay`                             | Mid-session increase attempted with more than one local player on this peer                                                                                                                                  | Set the delay before adding inputs (typically via `SessionBuilder::with_input_delay`) when running multi-local  |
| `InputDelayMidSessionPendingOutputFull { delta, capacity }`   | `P2PSession::set_input_delay`                             | Mid-session increase would enqueue `delta` gap-fill frames, exceeding remote `pending_output_limit` `capacity`                                                                                               | Apply the change in smaller increments, or wait for the remote to acknowledge outstanding inputs and retry      |
| `PlayerAlreadyRemoved { handle }`                             | `P2PSession::remove_player`                               | `remove_player` called when the handle is already marked disconnected — either by a previous `remove_player` call, by auto-removal via `ContinueWithout`, or by a previous explicit `disconnect_player` call | Treat as a no-op; the peer is already in the graceful-drop terminal state                                       |
| `NotLocalPlayer { handle }` *(pre-existing variant)*          | `P2PSession::set_input_delay` / `P2PSession::input_delay` | `handle` is not registered as a local player (it may be a remote player, spectator, or unregistered)                                                                                                         | Pass a registered local player handle (use `SessionBuilder::add_player(PlayerType::Local, ..)` to register one) |

### Selected `InternalErrorKind` Variants — Runtime Input Delay

| Variant                             | Source API                    | Cause                                                                    | Recovery                                                                   |
| ----------------------------------- | ----------------------------- | ------------------------------------------------------------------------ | -------------------------------------------------------------------------- |
| `InputQueueGapFillFailed { frame }` | `P2PSession::set_input_delay` | Mid-session gap-fill replication failed an internal invariant at `frame` | Report as a library bug with the failing `frame` and the call's parameters |

---

## Event Catalog

`FortressEvent<T>` is **not** `#[non_exhaustive]`. Adding new variants is a breaking change for exhaustive matches; recent additions are listed below.

### Selected `FortressEvent` Variants — Disconnect, Graceful Drop, and Input Delay

| Variant                                                                      | When emitted                                                                                                                                                         | Coexisting events                                                                                                                                                         |
| ---------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `PeerDropped { handle, addr }`                                               | Auto-removal under `DisconnectBehavior::ContinueWithout` after a disconnect timeout, **or** explicit `P2PSession::remove_player` call                                | One event per non-spectator handle at the dropped address; followed by exactly one `Disconnected { addr }` after all `PeerDropped` for the same address in the same batch |
| `Disconnected { addr }`                                                      | Always emitted on peer drop (legacy event); under `Halt` it appears alone, under graceful drop it appears once per address after that address's `PeerDropped` events | Optionally preceded by one or more `PeerDropped { handle, addr }` (graceful drop, one per handle at the dropped address)                                                  |
| `InputDelayRecommendation { player_handle, current_delay, suggested_delay }` | Reserved for application-level heuristics or future automatic emitters. **No built-in emitter currently produces this event.**                                       | None                                                                                                                                                                      |
| `SpectatorDivergence { frame, player, primary_addr, conflicting_addr }`      | A failover spectator received conflicting same-frame input for `player` from two connected redundant hosts                                                          | Followed by terminal `FortressError::SpectatorDivergence` on future `advance_frame` calls                                                                                 |

---

## Cross-Cutting Invariants

These invariants are preserved across ALL public API calls:

1. **INV-3 (Input Immutability):** Confirmed inputs never change
2. **INV-4 (Queue Bounds):** `0 ≤ queue.length ≤ 128`
3. **INV-5 (Index Validity):** `head, tail ∈ [0, 128)`
4. **INV-11 (No Panics):** All errors are `Result::Err`, never panic

---

## Revision History

| Version | Date       | Changes                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| ------- | ---------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1.1     | 2026-05-07 | Added contracts for runtime input delay (`P2PSession::set_input_delay`, `P2PSession::input_delay`), configurable disconnect behavior (`SessionBuilder::with_disconnect_behavior`, `P2PSession::disconnect_behavior`), and explicit graceful peer removal (`P2PSession::remove_player`). Documented new `InvalidRequestKind`/`InternalErrorKind` variants and the new `FortressEvent::PeerDropped` and `FortressEvent::InputDelayRecommendation` events. Added Event Catalog. |
| 1.0     | 2025-12-06 | Complete API contracts                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| 0.1     | 2025-12-06 | Initial draft                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
