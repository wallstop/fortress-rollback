# Fortress Rollback Formal Specification (Draft)

> Status: Draft (2025-12-06)
> Scope: High-level invariants, session state machines, protocol guarantees. To be refined with TLA+/Z3 artifacts.

## Goals
- Capture safety and liveness invariants for rollback sessions.
- Define state machines for P2P, Spectator, and SyncTest sessions.
- Specify message ordering and timing assumptions.
- Provide a basis for TLA+ models and automated checks.

## System Model
- **Frames**: Discrete ticks. `Frame` is monotonic outside rollback; rollback rewinds to an earlier frame and replays.
- **Players**: Identified by `PlayerHandle` (0..N-1). Types: Local, Remote, Spectator.
- **Inputs**: Per-player per-frame inputs. Predicted vs confirmed.
- **Game State**: Saved/restored via `GameStateCell` requests.
- **Network**: Unreliable, unordered datagrams; protocol enforces ordering per-peer where required.

## Invariants (Safety)
- **Frame monotonicity**: During normal play, `current_frame` increases by 1; during rollback, `current_frame` rewinds to a saved frame and then advances deterministically to catch up.
- **Rollback bound**: `rollback_depth <= max_prediction_window` (from SessionBuilder).
- **Input consistency**: Once confirmed for `(player, frame)`, input is immutable.
- **Queue integrity**: For each player, `0 <= queued_inputs.len() <= INPUT_QUEUE_LENGTH`.
- **State availability**: For any requested rollback frame, a matching saved state exists or an error is surfaced (never panic).
- **Message causality**: For a given peer, protocol processes messages in non-decreasing frame order for confirms/acks.
- **Checksum sanity**: When desync detection is On, checksum comparisons only occur on confirmed frames.
- **No panics in library code**: All user-visible failures are `FortressError` returns.

## Liveness (Progress)
- **Eventual confirmation**: If network delivers all sent inputs within bounded time, all predicted inputs become confirmed within `max_prediction_window` frames.
- **Catch-up after rollback**: After a rollback, the session advances to the latest known frame without skipping frames.
- **Sync convergence** (P2P): If peers exchange sync messages reliably for `SYNC_RETRY_INTERVAL` windows, session transitions from Synchronizing → Running.
- **Spectator catch-up**: Spectator session eventually reaches host’s confirmed frame set given bounded delay and available state snapshots.

## Session State Machines (Sketch)
- **Common states**: `Synchronizing`, `Running`.
- **Transitions (P2P)**:
  - Init → Synchronizing (builder.start_p2p_session)
  - Synchronizing → Running (after required sync acks)
  - Running → Synchronizing (on disconnect/reconnect?) [open question]
- **Transitions (Spectator)**:
  - Init → Synchronizing (start_p2p_spectator_session)
  - Synchronizing → Running (after host sync)
- **Transitions (SyncTest)**:
  - Init → Running (no network; determinism harness)

## Message Ordering & Delivery Guarantees
- **Transport**: Unordered, unreliable; protocol layers enforce per-peer ordering for input confirmations.
- **Required ordering**: For each peer, confirm/ack messages are applied in frame order.
- **Retransmission**: Sync/handshake messages are retried up to configured intervals (see protocol.rs constants).
- **Drop tolerance**: Prediction window absorbs transient loss up to `max_prediction_window` frames.

## Timing Model
- **Tick rate**: Configured FPS; session expects user to call `advance_frame` at that cadence.
- **Input delay**: Optional delay shifts local input application by `input_delay` frames.
- **Frame advantage**: Peers may run slightly faster/slower (time sync) to converge.

## Error Semantics
- All public APIs return `Result<_, FortressError>`; no panics in library code.
- Invalid states (e.g., missing save on rollback) produce explicit errors, not aborts.

## TODO / Next Steps
- Encode P2P/Spectator state machines in TLA+ with message channels and rollback actions.
- Add precise pre/postconditions per API into API_CONTRACTS.md.
- Define formal checksum/desync detection property.
- Prove rollback bound and input immutability in Kani/Z3 for input_queue and sync_layer.
- Add liveness assumptions (fair delivery) to TLA+ specs.
