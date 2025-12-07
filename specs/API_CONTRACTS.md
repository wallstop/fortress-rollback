# API Contracts (Draft)

> Status: Draft (2025-12-06)
> Purpose: Preconditions, postconditions, and invariants for public APIs. Complements FORMAL_SPEC.md.

## SessionBuilder
- `with_num_players(n)`
  - Pre: `n > 0`
  - Post: builder stores `num_players = n`
  - Invariants preserved: none broken
- `add_player(player_type, handle)`
  - Pre: `handle < num_players`; handle unique
  - Post: player registered with given type; total players <= `num_players`
  - Errors: `FortressError::InvalidRequest` if violated
- `with_max_prediction_window(w)`
  - Pre: `w >= 0`
  - Post: sets rollback bound to `w`
- `start_p2p_session(socket)` / `start_p2p_spectator_session(socket)` / `start_synctest_session()`
  - Pre: all required players added
  - Post: returns session in Synchronizing (or Running for synctest)
  - Errors: `InvalidRequest` if player setup incomplete

## Session APIs (P2P/Spectator/SyncTest)
- `add_local_input(handle, input)`
  - Pre: `handle` is local; session state is Running or Synchronizing; frame within prediction window
  - Post: local input enqueued at `current_frame`
  - Errors: `InvalidRequest`, `PredictionThreshold` if window exceeded
- `advance_frame()`
  - Pre: caller supplies required local inputs
  - Post: returns sequence of `GgrsRequest` (Save/Load/AdvanceFrame); `current_frame` increments or rolls back then catches up
  - Errors: `NotSynchronized` if not Running; `PredictionThreshold` if window exceeded; never panics
- `events()`
  - Post: drains pending `GgrsEvent`s; non-blocking
- `poll_remote_clients()`
  - Post: processes inbound messages, updates sync/input queues

## GameState Requests
- `SaveGameState { frame, cell }`
  - Pre: frame == session frame when issued
  - Post: user must call `cell.save(frame, data, checksum)`; after save, rollback to `frame` is defined
- `LoadGameState { frame, cell }`
  - Pre: cell contains previously saved state for `frame`
  - Post: user loads state; session sets `current_frame = frame`

## Invariants (cross-cutting)
- Confirmed inputs are immutable once set.
- Rollback depth never exceeds `max_prediction_window`.
- No public API panics; failures surface as `FortressError`.
- All frames processed in deterministic order after rollback catch-up.

## TODO / Next Steps
- Flesh out full function list with precise pre/post for every public API.
- Add error variants and examples inline (rustdoc-ready snippets).
- Link to formal state machines from FORMAL_SPEC.md.
- Add property-based contract tests once formalized.
