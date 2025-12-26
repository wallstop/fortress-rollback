<p align="center">
  <img src="../../assets/logo-small.svg" alt="Fortress Rollback" width="64">
</p>

# TLA+ Specifications for Fortress Rollback

This directory contains TLA+ specifications for formally verifying the correctness properties of Fortress Rollback.

## Quick Start

```bash
# Run all TLA+ verification (from project root)
./scripts/verify-tla.sh

# List available specs
./scripts/verify-tla.sh --list

# Verify specific spec
./scripts/verify-tla.sh NetworkProtocol

# Quick verification (smaller bounds)
./scripts/verify-tla.sh --quick
```

## Files

| File | Config | Status | Description |
|------|--------|--------|-------------|
| `NetworkProtocol.tla` | `NetworkProtocol.cfg` | ✓ CI | UDP protocol state machine |
| `InputQueue.tla` | `InputQueue.cfg` | ✓ CI | Circular buffer input queue |
| `Rollback.tla` | `Rollback.cfg` | ✓ CI | Rollback mechanism |
| `Concurrency.tla` | `Concurrency.cfg` | ✓ CI | GameStateCell thread safety |
| `ChecksumExchange.tla` | `ChecksumExchange.cfg` | ✓ CI | Checksum exchange for desync detection |

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

**Liveness:**

- Predictions eventually confirmed (with rollback)

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

**Safety:**

- Checksums are computed deterministically for each frame
- Checksum reports are only sent for confirmed frames
- Desync detection compares matching frame checksums

**Liveness:**

- Checksum exchange eventually completes for confirmed frames
- Desync is detected within bounded time after occurrence

## Running the Specifications

### Automated (Recommended)

Use the verification scripts from the project root:

```bash
# Run all enabled TLA+ specs in CI
./scripts/verify-tla.sh

# Run specific spec
./scripts/verify-tla.sh InputQueue

# Run quick verification (smaller state bounds)
./scripts/verify-tla.sh --quick

# List available specs
./scripts/verify-tla.sh --list
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
| `NetworkProtocol.cfg` | MAX_FRAME=5, randomId=1..5 | ~12,000 states |
| `InputQueue.cfg` | QUEUE_LENGTH=4, MAX_FRAME=6 | ~77,000 states |
| `Concurrency.cfg` | MAX_FRAME=4 | Small |
| `Rollback.cfg` | MAX_PREDICTION=1, MAX_FRAME=3 | ~52M states |

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
| `ChecksumExchange` | `src/checksum.rs`, `src/network/messages.rs` (ChecksumReport) |

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

## References

- [TLA+ Resources](https://lamport.azurewebsites.net/tla/tla.html)
- [Learn TLA+](https://learntla.com/)
- [TLA+ Video Course](https://lamport.azurewebsites.net/video/videos.html)
- [Specifying Systems (book)](https://lamport.azurewebsites.net/tla/book.html)
