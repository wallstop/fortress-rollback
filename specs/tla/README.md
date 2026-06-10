<p align="center">
  <img src="../../docs/assets/logo-small.svg" alt="Fortress Rollback" width="64">
</p>

# TLA+ Specifications for Fortress Rollback

This directory contains TLA+ specifications for formally verifying the correctness properties of Fortress Rollback.

## Quick Start

```bash
# Run all TLA+ verification (from project root)
./scripts/verification/verify-tla.sh

# List available specs
./scripts/verification/verify-tla.sh --list

# Verify specific spec
./scripts/verification/verify-tla.sh NetworkProtocol

# Quick verification (smaller bounds)
./scripts/verification/verify-tla.sh --quick
```

## Files

| File | Config | Status | Description |
|------|--------|--------|-------------|
| `NetworkProtocol.tla` | `NetworkProtocol.cfg` | ✓ CI | Sync-handshake + peer-drop state machine (N=3 peers) |
| `InputQueue.tla` | `InputQueue.cfg` | ✓ CI | Circular buffer input queue |
| `Rollback.tla` | `Rollback.cfg` | ✓ CI | Rollback mechanism |
| `Concurrency.tla` | `Concurrency.cfg` | ✓ CI | GameStateCell thread safety |
| `ChecksumExchange.tla` | `ChecksumExchange.cfg` | ✓ CI | Checksum exchange for desync detection, per-(local,remote)-pair verdicts (N=3 peers) |
| `SpectatorSession.tla` | `SpectatorSession.cfg` | ✓ CI | Spectator session with frame delay and catchup |
| `TimeSync.tla` | `TimeSync.cfg` | ✓ CI | Time synchronization for peer frame rate coordination (pinned N=2, see cfg) |
| `PeerDrop.tla` | `PeerDrop.cfg` | ✓ CI | Halt vs ContinueWithout peer-drop policy model |
| `NPeerReactivation.tla` | `NPeerReactivation.cfg` | ✓ CI | N-peer mesh reconnection activation-frame agreement (Agreement C) (N=3 survivors) |

## Properties Verified

### NetworkProtocol.tla

**Safety:**

- Valid state transitions only (Initializing → Synchronizing → Running → etc.)
- Sync remaining counter never negative
- Only Running state processes game inputs

**Liveness:**

- Eventually synchronized (under fair scheduling)
- No deadlock

### InputQueue.tla

**Safety (from formal-spec.md):**

- INV-4: Queue length bounded by `QUEUE_LENGTH` (128)
- INV-5: Head and tail indices always valid
- FIFO ordering preserved
- No frame gaps in queue
- Runtime input delay stays within queue capacity
- Mid-session delay increases preserve contiguous queued frames
- Frozen queues reject later adds and preserve the final confirmed input

**Liveness:**

- Predictions eventually confirmed (with rollback)

### PeerDrop.tla

**Safety:**

- Halt transitions to `Synchronizing` after any dropped peer
- ContinueWithout freezes dropped players and keeps survivors independent
- `PeerDropped` events are emitted only by ContinueWithout
- Dropped players are excluded from survivor progress
- Rollback starts no later than every dropped player's `lastFrame + 1`

### NPeerReactivation.tla

Models "Agreement C" of N-peer mesh reconnection (1 coordinator, 3 survivors,
1 joiner) from `progress/session-18-npeer-mesh-reconnection-design.md` (§4.C/§5/§8).

**Safety:**

- `Agreement` (S1): any two peers (coordinator, any survivor, joiner) that both
  committed a frame committed the same committed **value** (the bytes in `val`, *not*
  the `mode`/value-source label) at that frame
- `NoConfirmedRewrite` (S2): committed history never reverts (every frame within `committedUpTo` stays definite)
- `NoSplitBrainOnAbort` (L1): no aborted state has the joiner real-at-`F` while a survivor is frozen-at-`F`

**Liveness (under weak fairness):**

- `EventuallyResolved`: the protocol always reaches a terminal `joined`/`aborted` state

**The cap and the abort lifecycle are modeled non-vacuously (have teeth):**

- Survivors start with `committedUpTo` *below* `L` and **race toward `F`** by committing
  frozen frames (`SurvivorAdvanceFrozen`). The keepalive-preserved cap (design §4.C —
  "confirmed = min over connected incl. K") **holds each survivor at `F-1 = L`**: a
  survivor cannot commit frame `F` frozen while `capHeld` is true. So the cap is an
  exercised, live constraint, not an assertion by construction.
- `CapCollapse` models the cap-collapse hazard (the coordinator dropping out of a
  survivor's connected-min mid-pause). The protocol's keepalive serve (`keepaliveServing`)
  is its guard: while the paused coordinator is serving, `CapCollapse` cannot fire.
- The joiner follows the **late-apply lifecycle** (§5): `JoinerBuffer` buffers the
  snapshot without applying it; `JoinerCommit` makes the joiner real-at-`F` *only* after
  every survivor has reopened (the `JoinCommitted` gate); `JoinerAbortDiscard` discards
  the buffer on abort, so the joiner is never real-at-`F` after an abort.
- Removing either guard (a scratch *naive* variant: `CapCollapse` ungated **and** the
  joiner committing real-at-`F` eagerly on buffer) makes TLC find a `NoSplitBrainOnAbort`
  counterexample — confirming both the cap and the gated commit are load-bearing.

**Precondition P-A is checkable, not assumed:** survivors hold *independent* frozen
values. The default config pins P-A (`AssumePA = TRUE`) and passes. `NPeerReactivation_NoPA.cfg`
(`AssumePA = FALSE`, not registered in CI) drops P-A and TLC reports an `Agreement`
counterexample — demonstrating P-A is necessary.

### Rollback.tla

**Safety (from formal-spec.md):**

- INV-1: Frame monotonicity (except during rollback)
- INV-2: Rollback target is valid frame
- INV-7: Confirmed frame in valid range
- INV-8: Saved frame in valid range
- SAFE-4: Rollback restores correct state (state exists for rollback target)

**Liveness (disabled for CI due to state space):**

- LIVE-4: Rollback completes

### Concurrency.tla

**Safety:**

- Mutual exclusion: At most one thread holds lock at a time
- No data races: Only lock holder can modify cell state
- Frame consistency: After save, cell frame matches saved frame
- Load returns saved: Load operation returns correct data
- Valid frame after save: Save never stores NULL_FRAME
- Wait queue FIFO: Threads acquire lock in request order

**Liveness:**

- No deadlock: Some action always enabled
- Operations complete: Every started operation eventually completes
- Fair lock acquisition: Waiting threads eventually get the lock

**Linearizability:**

- Operations appear atomic (guaranteed by mutex)

### ChecksumExchange.tla

All verdict state (`pendingChecksums`/`syncHealth`/`lastVerifiedFrame`) is keyed by ordered
(local, remote) peer pairs, mirroring the implementation's per-`UdpProtocol` endpoint state
(the F12 fix in `src/sessions/p2p_session.rs`). Checked at `PEERS = {p1, p2, p3}`,
`MAX_FRAME = 3`, `CHECKSUM_INTERVAL = 1` — the smallest model with two comparable checksum
frames per pair. Tractability at N=3 needs both `SYMMETRY Permutations(PEERS)` (sound: the
module's only `CHOOSE` ranges over integer frames, never peers, and liveness is disabled)
and an in-flight network cap of 2 (one broadcast outstanding; sound because
`ReceiveChecksum` commutes with every other action — see the `StateConstraint` comment in
the spec). Measured single-worker: N=3 completes in ~106s with 11,674,741 states generated
/ 1,469,194 distinct (depth 62); N=2 in ~1s with 38,160 generated / 9,167 distinct (depth
32). Without the cap-2 constraint and symmetry the same bounds do NOT terminate in CI
budget (killed at 28.6 min, 233M+ generated / 31.7M+ distinct, queue still growing).

**Safety:**

- Checksums are computed deterministically for each frame
- Checksum reports are only sent for confirmed frames
- No false positives, pair-precise: a `DesyncDetected` verdict for (p,q) requires p or q
  to have actually diverged
- Desync verdicts are terminal per pair (a match against one remote never clears another
  pair's verdict — the Session 27 cross-pair clobbering regression guard)
- `last_verified_frame` is monotonically increasing per pair
- F12 leakage guards: `InSync` for (p,q) requires a matching checksum in pair (p,q) itself;
  the `is_synchronized()` aggregate requires every pair individually verified; the
  `last_verified_frame()` aggregate is the min over pairs. The two aggregate invariants
  (`SynchronizedRequiresAllPairsVerified`, `AggregateVerifiedFrameSound`) are tautological
  given the current aggregate definitions — they are kept as regression tripwires against
  someone redefining the aggregates, not as added verification strength

**Liveness:**

- Defined but disabled (premises are unsound under a late `IntroduceDesync`; same as the
  earlier N=2 model)

## Running the Specifications

### Automated (Recommended)

Use the verification scripts from the project root:

```bash
# Run all enabled TLA+ specs in CI
./scripts/verification/verify-tla.sh

# Run specific spec
./scripts/verification/verify-tla.sh InputQueue

# Run quick verification (smaller state bounds)
./scripts/verification/verify-tla.sh --quick

# List available specs
./scripts/verification/verify-tla.sh --list
```

The script automatically downloads TLC tools if needed.

### Prerequisites for Manual Verification

1. Install TLA+ Toolbox or TLC command-line tools
2. Download from: <https://lamport.azurewebsites.net/tla/tools.html>

### Using TLA+ Toolbox

1. Open TLA+ Toolbox
2. File → Open Spec → Add New Spec
3. Select one of the `.tla` files
4. Create a new model (Model → New Model)
5. Configure constants (see `.cfg` files for values)
6. Run Model Checker

### Configuration Files

Each spec has a `.cfg` file with TLC-compatible settings:

| Config File | Key Constants | State Space |
|-------------|---------------|-------------|
| `NetworkProtocol.cfg` | PEERS={p1,p2,p3}, NUM_SYNC_PACKETS=1 | ~170,000 distinct states (~2.6M generated) |
| `InputQueue.cfg` | QUEUE_LENGTH=4, MAX_FRAME=6 | ~77,000 states |
| `Concurrency.cfg` | MAX_FRAME=4 | Small |
| `Rollback.cfg` | MAX_PREDICTION=1, MAX_FRAME=3 | ~1.8M distinct states (~29.2M generated) |
| `ChecksumExchange.cfg` | PEERS={p1,p2,p3}, MAX_FRAME=3, SYMMETRY | ~1.47M distinct states (~11.7M generated), ~106s single worker |

**Note on NULL_FRAME:** TLC config files don't support negative numbers,
so we use `NULL_FRAME = 999` as a sentinel value instead of -1.

### Legacy Manual Configurations

For TLA+ Toolbox or custom model checking:

#### NetworkProtocol.tla

```
CONSTANTS
    NUM_SYNC_PACKETS = 3    \* Reduced from 5 for faster checking
    PEERS = {p1, p2}        \* Two peers

INVARIANT
    TypeInvariant
    SyncRemainingNonNegative

PROPERTY
    ValidStateTransitions
```

#### InputQueue.tla

```
CONSTANTS
    QUEUE_LENGTH = 8        \* Reduced from 128 for faster checking
    MAX_FRAME = 20
    NULL_FRAME = -1

INVARIANT
    SafetyInvariant
```

#### Rollback.tla

```
CONSTANTS
    MAX_PREDICTION = 4      \* Reduced from 8 for faster checking
    MAX_FRAME = 15
    NUM_PLAYERS = 2
    NULL_FRAME = -1

INVARIANT
    SafetyInvariant

PROPERTY
    RollbackCompletes
```

#### Concurrency.tla

```
CONSTANTS
    THREADS = {t1, t2}      \* Two threads for basic checking
    MAX_FRAME = 5           \* Reduced for faster checking
    NULL_FRAME = -1

INVARIANT
    SafetyInvariant

PROPERTY
    OperationsComplete
    FairLockAcquisition
```

### Command Line

```bash
# Check NetworkProtocol
java -jar tla2tools.jar -config NetworkProtocol.cfg NetworkProtocol.tla

# Check InputQueue
java -jar tla2tools.jar -config InputQueue.cfg InputQueue.tla

# Check Rollback
java -jar tla2tools.jar -config Rollback.cfg Rollback.tla

# Check Concurrency
java -jar tla2tools.jar -config Concurrency.cfg Concurrency.tla
```

## Relationship to Implementation

These specifications model the key algorithms from:

| TLA+ Module | Rust Implementation |
|-------------|---------------------|
| `NetworkProtocol` | `src/network/protocol/mod.rs` (UdpProtocol) |
| `InputQueue` | `src/input_queue/mod.rs` (InputQueue) |
| `Rollback` | `src/sync_layer/mod.rs` (SyncLayer), `src/sessions/p2p_session.rs` |
| `Concurrency` | `src/sync_layer/game_state_cell.rs` (GameStateCell, GameStateAccessor) |
| `ChecksumExchange` | `src/sessions/p2p_session.rs` (sync_health, is_synchronized, last_verified_frame, compare_local_checksums_against_peers, check_checksum_send_interval), `src/network/protocol/mod.rs` (pending_checksums, last_verified_frame, on_checksum_report), `src/network/messages.rs` (ChecksumReport) |

## Extending the Specifications

### Adding New Properties

1. Define the property in TLA+ temporal logic
2. Add to the PROPERTY section of the model
3. Run model checker to verify

### Modeling New Features

1. Add variables to represent new state
2. Update `Init` for initial values
3. Add actions for state transitions
4. Update `Next` to include new actions
5. Add invariants/properties to verify

## Best Practices for TLA+ Development

> **See also:** [/.llm/skills/formal-verification/verification.md](../../.llm/skills/formal-verification/verification.md) — TLA+ and Z3 verification guide

### Specification Design

| Do | Don't |
|----|-------|
| Model essential behavior | Model implementation details |
| Use small constants for checking | Start with production-sized constants |
| Write type invariants first | Skip type checking |
| Add helper operators | Write monolithic actions |
| Separate safety and liveness | Mix them in one model run |

### Common Patterns

```tla
(* State machines: explicit states *)
States == {"Init", "Running", "Done"}
Trans(from, to) == state = from /\ state' = to

(* Bounded resources *)
ASSUME MAX_VALUE > 0
x' \in 0..MAX_VALUE

(* Nondeterministic choice *)
\/ action_a
\/ action_b

(* Existential: any valid value *)
\E v \in ValidValues: x' = v
```

### When Verification Fails

1. **Read the counterexample trace** — TLC shows exact state sequence
2. **Check if spec matches intent** — Is the model correct?
3. **Check if code matches spec** — Is the implementation correct?
4. **Add regression test** — Capture the bug scenario in Rust tests

### Performance Tips

- Start with tiny constants (MAX_FRAME=3, NUM_PLAYERS=2)
- Run safety checks before liveness
- Use `-workers auto` for parallel checking
- Add state constraints to limit exploration

## References

- [TLA+ Resources](https://lamport.azurewebsites.net/tla/tla.html)
- [Learn TLA+](https://learntla.com/)
- [TLA+ Video Course](https://lamport.azurewebsites.net/video/videos.html)
- [Specifying Systems (book)](https://lamport.azurewebsites.net/tla/book.html)
