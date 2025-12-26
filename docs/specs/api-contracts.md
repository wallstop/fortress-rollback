<p align="center">
  <img src="../../assets/logo-small.svg" alt="Fortress Rollback" width="64">
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

### `with_num_players(self, n: usize) -> Self`

```rust
/// Set the number of active players (not spectators).
```

**Pre:** None (any usize accepted)

**Post:** `self.num_players = n`

**Errors:** None (validated at session start)

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

- `InvalidRequest("Player handle already in use")` - handle duplicate
- `InvalidRequest("...handle should be between 0 and num_players")` - invalid player handle
- `InvalidRequest("...handle should be num_players or higher")` - invalid spectator handle

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

### `with_input_delay(self, delay: usize) -> Self`

```rust
/// Set input delay for local players.
```

**Pre:** None

**Post:** `self.input_delay = delay`

**Errors:** None

**Panics:** Never

---

### `with_fps(self, fps: usize) -> Result<Self, FortressError>`

```rust
/// Set expected update frequency.
```

**Pre:** `fps > 0`

**Post:** `self.fps = fps`

**Errors:**

- `InvalidRequest("FPS should be higher than 0")` - if `fps = 0`

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

### `with_sparse_saving_mode(self, sparse: bool) -> Self`

```rust
/// Enable sparse saving (fewer saves, longer potential rollbacks).
```

**Pre:** None

**Post:** `self.sparse_saving = sparse`

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

### `start_p2p_session(self, socket: impl NonBlockingSocket) -> Result<P2PSession, FortressError>`

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

- `InvalidRequest("Not enough players have been added...")` - missing players

**Panics:** Never

**Invariants Established:**

- INV-4: Queue length bounds
- INV-5: Queue index validity
- INV-11: No panics guarantee

---

### `start_spectator_session(self, host: Address, socket: impl NonBlockingSocket) -> SpectatorSession`

```rust
/// Create a spectator session connected to a host.
```

**Pre:** None (no player registration required)

**Post:**

- Session created in `Synchronizing` state
- Host endpoint begins synchronization

**Errors:** None

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

- `InvalidRequest("Check distance too big")` - if `check_distance >= max_prediction`

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

### `local_player_handles(&self) -> Vec<PlayerHandle>`

```rust
/// Get handles of all local players.
```

**Pre:** None

**Post:** Returns vector of handles where `player_type = Local`

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

- `InvalidPlayerHandle` - handle not registered or not local
- `InvalidRequest("Prediction threshold reached")` - too far ahead

**Panics:** Never

---

### `advance_frame(&mut self) -> Result<Vec<FortressRequest>, FortressError>`

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
- `InvalidRequest("Prediction threshold reached")` - exceeded max_prediction

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

### `events(&mut self) -> Drain<FortressEvent>`

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

**Pre:** `handle` is a remote player

**Post:** Returns stats (ping, bandwidth, etc.)

**Errors:**

- `InvalidPlayerHandle` - not a remote player
- `NotSynchronized` - stats not yet available

**Panics:** Never

---

### `disconnect_player(&mut self, handle: PlayerHandle) -> Result<(), FortressError>`

```rust
/// Manually disconnect a player.
```

**Pre:** `handle` is registered

**Post:**

- Player marked as disconnected
- Future inputs use default value

**Errors:**

- `InvalidPlayerHandle` - not registered

**Panics:** Never

---

## SpectatorSession

### `advance_frame(&mut self) -> Result<Vec<FortressRequest>, FortressError>`

```rust
/// Advance spectator simulation.
```

**Pre:** `current_state() = Running`

**Post:**

- Returns `AdvanceFrame` requests only (no save/load)
- May return multiple frames if catching up

**Errors:**

- `NotSynchronized` - not yet synchronized with host

**Panics:** Never

---

## SyncTestSession

### `advance_frame(&mut self) -> Result<Vec<FortressRequest>, FortressError>`

```rust
/// Advance with automatic rollback testing.
```

**Pre:** All local inputs provided

**Post:**

- Simulates rollback of `check_distance` frames
- Compares checksums for mismatch detection
- Returns requests including save/load/advance

**Errors:**

- `InvalidRequest` on checksum mismatch (desync detected)

**Panics:** Never

---

## GameStateCell

### `save(&self, frame: Frame, state: Option<T::State>, checksum: Option<u128>)`

```rust
/// Save game state into the cell.
```

**Pre:**

- Called in response to `SaveGameState` request
- `frame` matches request frame

**Post:**

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

```rust
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

```rust
FortressRequest::LoadGameState { cell, frame }
```

**Pre:** State was previously saved at `frame`

**Post (after handling):**

- `game_state = cell.load().expect("must exist")`
- Game state restored to frame `frame`

**User Responsibility:**

- Replace entire game state with loaded state
- Subsequent `AdvanceFrame` requests will resimulate

### AdvanceFrame Contract

```rust,ignore
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

| Error | Cause | Recovery |
|-------|-------|----------|
| `InvalidRequest { info }` | Invalid operation/parameter | Check info message, fix call |
| `InvalidPlayerHandle { handle, max }` | Handle out of range or wrong type | Use valid handle |
| `InvalidFrame { frame, reason }` | Frame out of valid range | Check frame bounds |
| `NotSynchronized` | Operation requires Running state | Wait for sync or call poll |
| `MissingInput { handle, frame }` | Confirmed input not available | Internal error, report bug |

---

## Cross-Cutting Invariants

These invariants are preserved across ALL public API calls:

1. **INV-3 (Input Immutability):** Confirmed inputs never change
2. **INV-4 (Queue Bounds):** `0 ≤ queue.length ≤ 128`
3. **INV-5 (Index Validity):** `head, tail ∈ [0, 128)`
4. **INV-11 (No Panics):** All errors are `Result::Err`, never panic

---

## Revision History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2025-12-06 | Complete API contracts |
| 0.1 | 2025-12-06 | Initial draft |
