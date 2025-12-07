# TLA+ Specifications for Fortress Rollback

This directory contains TLA+ specifications for formally verifying the correctness properties of Fortress Rollback.

## Files

| File | Description |
|------|-------------|
| `NetworkProtocol.tla` | UDP protocol state machine for peer-to-peer communication |
| `InputQueue.tla` | Circular buffer input queue with prediction/confirmation |
| `Rollback.tla` | Rollback mechanism for state restoration and resimulation |
| `Concurrency.tla` | GameStateCell thread-safe state access via Arc<Mutex<T>> |

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

**Safety (from FORMAL_SPEC.md):**
- INV-4: Queue length bounded by `QUEUE_LENGTH` (128)
- INV-5: Head and tail indices always valid
- FIFO ordering preserved
- No frame gaps in queue

**Liveness:**
- Predictions eventually confirmed (with rollback)

### Rollback.tla

**Safety (from FORMAL_SPEC.md):**
- INV-2: Rollback depth bounded by `max_prediction`
- INV-6: State availability for rollback frames
- INV-7: Confirmed frame consistency
- INV-8: Saved frame consistency
- SAFE-4: Rollback restores correct state

**Liveness:**
- LIVE-3: Progress guaranteed
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

## Running the Specifications

### Prerequisites

1. Install TLA+ Toolbox or TLC command-line tools
2. Download from: https://lamport.azurewebsites.net/tla/tools.html

### Using TLA+ Toolbox

1. Open TLA+ Toolbox
2. File → Open Spec → Add New Spec
3. Select one of the `.tla` files
4. Create a new model (Model → New Model)
5. Configure constants (see below)
6. Run Model Checker

### Suggested Configurations

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
| `NetworkProtocol` | `src/network/protocol.rs` (UdpProtocol) |
| `InputQueue` | `src/input_queue.rs` (InputQueue) |
| `Rollback` | `src/sync_layer.rs` (SyncLayer), `src/sessions/p2p_session.rs` |
| `Concurrency` | `src/sync_layer.rs` (GameStateCell, GameStateAccessor) |

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
