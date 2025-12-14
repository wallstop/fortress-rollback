<p align="center">
  <img src="../assets/logo.svg" alt="Fortress Rollback" width="128">
</p>

# Fortress Rollback Architecture Guide

This document provides a comprehensive overview of Fortress Rollback's internal architecture, data flow, and design decisions. Understanding these concepts will help you integrate the library effectively and troubleshoot issues.

## Table of Contents

1. [Overview](#overview)
2. [Core Concepts](#core-concepts)
3. [Component Architecture](#component-architecture)
4. [Data Flow](#data-flow)
5. [Session Types](#session-types)
6. [Synchronization Protocol](#synchronization-protocol)
7. [Rollback Mechanics](#rollback-mechanics)
8. [State Management](#state-management)
9. [Network Protocol](#network-protocol)
10. [Error Handling](#error-handling)
11. [Type Safety Features](#type-safety-features)

---

## Overview

Fortress Rollback is a Rust implementation of rollback networking for real-time multiplayer games. It's based on the GGPO (Good Game Peace Out) network SDK concepts but with a modern, safe Rust API.

### Design Philosophy

- **100% Safe Rust**: No `unsafe` code blocks; leverages Rust's memory safety guarantees
- **Request-Based API**: Replaces callback-style API with a request-driven control flow
- **Determinism First**: Designed to work with deterministic game simulations
- **Type Safety**: Extensive use of newtypes and the type system to prevent misuse

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         User Application                         │
├─────────────────────────────────────────────────────────────────┤
│                        Session Layer                             │
│  ┌─────────────┐  ┌──────────────────┐  ┌───────────────────┐   │
│  │ P2PSession  │  │ SpectatorSession │  │  SyncTestSession  │   │
│  └─────────────┘  └──────────────────┘  └───────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│                      Synchronization Layer                       │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                      SyncLayer                           │    │
│  │  ┌──────────────┐  ┌────────────────┐  ┌─────────────┐  │    │
│  │  │ InputQueue[] │  │  SavedStates   │  │  TimeSync   │  │    │
│  │  └──────────────┘  └────────────────┘  └─────────────┘  │    │
│  └─────────────────────────────────────────────────────────┘    │
├─────────────────────────────────────────────────────────────────┤
│                        Network Layer                             │
│  ┌───────────────────┐  ┌────────────────┐  ┌────────────────┐  │
│  │   UdpProtocol     │  │   Messages     │  │  Compression   │  │
│  └───────────────────┘  └────────────────┘  └────────────────┘  │
├─────────────────────────────────────────────────────────────────┤
│                        Socket Layer                              │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │              NonBlockingSocket (trait)                    │   │
│  │  ┌───────────────────┐  ┌──────────────────────────────┐ │   │
│  │  │ UdpNonBlockingSocket│ │      Custom Sockets         │ │   │
│  │  └───────────────────┘  └──────────────────────────────┘ │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

---

## Core Concepts

### Frames

A **frame** is the fundamental unit of time in rollback networking. Each frame represents one discrete step of game simulation.

```rust
use fortress_rollback::Frame;

let frame = Frame::new(0);      // First frame
let next = frame + 1;           // Frame arithmetic
assert!(frame.is_valid());      // Check validity
assert!(Frame::NULL.is_null()); // NULL_FRAME represents "no frame"
```

Key properties:
- Frame numbers start at 0 and increment sequentially
- `Frame::NULL` (-1) represents "no frame" or "uninitialized"
- Frames are used to index inputs, saved states, and track synchronization

### Player Handles

A **PlayerHandle** uniquely identifies participants in a session.

```rust
use fortress_rollback::PlayerHandle;

let player = PlayerHandle::new(0);    // First player
let spectator = PlayerHandle::new(2); // Spectator (in 2-player game)

assert!(player.is_valid_player_for(2));    // Valid player handle
assert!(spectator.is_spectator_for(2));    // Spectator handle
```

Handle ranges:
- `0..num_players`: Active players who contribute inputs
- `num_players..`: Spectators who observe but don't contribute

### Player Types

```rust
pub enum PlayerType<A> {
    Local,         // Plays on this device
    Remote(A),     // Plays on a remote device (A = address)
    Spectator(A),  // Observes from a remote device
}
```

### Session States

```rust
pub enum SessionState {
    Synchronizing, // Establishing connection with remotes
    Running,       // Synchronized and accepting input
}
```

---

## Component Architecture

### SyncLayer

The `SyncLayer` is the heart of the synchronization system. It manages:

1. **Input Queues**: One per player, stores received and predicted inputs
2. **Saved States**: Circular buffer of game states for rollback
3. **Frame Tracking**: Current frame, confirmed frame, saved frame

```
SyncLayer
├── input_queues: Vec<InputQueue>  // One per player
├── saved_states: SavedStates      // Circular buffer of GameStateCell
├── current_frame: Frame           // Current simulation frame
├── last_confirmed_frame: Frame    // All inputs received up to here
└── last_saved_frame: Frame        // Most recently saved state
```

**Key Operations:**
- `add_local_input()`: Registers local player input
- `add_remote_input()`: Registers remote player input
- `synchronized_inputs()`: Returns inputs for current frame (confirmed or predicted)
- `save_current_state()`: Creates a save request
- `load_frame()`: Creates a load request for rollback

### InputQueue

Each player has an `InputQueue` that:
- Stores confirmed inputs in a circular buffer (128 entries)
- Generates predictions when inputs haven't arrived
- Tracks the first incorrect prediction for rollback detection

```
InputQueue
├── inputs: Vec<PlayerInput>     // Circular buffer [128]
├── head/tail: usize             // Buffer pointers
├── frame_delay: usize           // Input delay setting
├── prediction: PlayerInput      // Current prediction
├── first_incorrect_frame: Frame // First wrong prediction
└── last_requested_frame: Frame  // For discard protection
```

**Prediction Strategy:**
When an input hasn't arrived, the queue predicts the player will repeat their last input. This works well for games where players hold buttons (e.g., holding "forward" to move).

### GameStateCell

Game states are stored in `GameStateCell<T>` containers:

```rust
pub struct GameStateCell<T>(Arc<Mutex<GameState<T>>>);

// Usage in save/load requests:
match request {
    FortressRequest::SaveGameState { cell, frame } => {
        cell.save(frame, Some(game_state.clone()), Some(checksum));
    }
    FortressRequest::LoadGameState { cell, frame } => {
        game_state = cell.load().expect("State should exist");
    }
}
```

The `Arc<Mutex<>>` wrapper allows safe sharing between the library and user code.

### UdpProtocol

The `UdpProtocol` manages communication with a single remote endpoint:

```
UdpProtocol
├── state: ProtocolState         // Initializing/Synchronizing/Running/etc.
├── peer_addr: Address           // Remote address
├── pending_output: VecDeque     // Inputs waiting to be sent
├── recv_inputs: BTreeMap        // Received inputs by frame
├── time_sync_layer: TimeSync    // Frame advantage calculation
├── peer_connect_status: Vec     // Remote's view of all players
└── pending_checksums: BTreeMap  // For desync detection
```

**Protocol States:**
1. `Initializing`: Not yet started
2. `Synchronizing`: Exchanging sync packets
3. `Running`: Normal operation
4. `Disconnected`: Peer disconnected
5. `Shutdown`: Graceful shutdown in progress

---

## Data Flow

### Input Flow (Local Player)

```
1. User calls add_local_input(handle, input)
   │
2. Input stored in local_inputs map
   │
3. User calls advance_frame()
   │
4. Input sent to SyncLayer.add_local_input()
   │  └─► Stored in InputQueue with frame delay applied
   │
5. Input serialized and queued for all remote endpoints
   │
6. UdpProtocol.send_all_messages() transmits to peers
```

### Input Flow (Remote Player)

```
1. UDP packet received via socket.receive_all_messages()
   │
2. Routed to correct UdpProtocol by address
   │
3. UdpProtocol.handle_message() processes input
   │  └─► Decompressed and stored in recv_inputs
   │
4. Event::Input generated during poll()
   │
5. P2PSession.handle_event() receives Event::Input
   │
6. Input sent to SyncLayer.add_remote_input()
   │  └─► Stored in player's InputQueue
   │  └─► Prediction checked; first_incorrect_frame set if wrong
```

### Rollback Flow

```
1. advance_frame() detects first_incorrect_frame != NULL
   │
2. Determine frame_to_load (first incorrect or last saved)
   │
3. Generate LoadGameState request
   │  └─► User loads the saved state
   │
4. Reset prediction state
   │
5. For each frame from frame_to_load to current_frame:
   │   ├─► Generate SaveGameState request (if not sparse saving)
   │   ├─► Get corrected inputs from InputQueue
   │   └─► Generate AdvanceFrame request
   │
6. Now at original current_frame with corrected state
```

---

## Session Types

### P2PSession

The primary session type for peer-to-peer multiplayer:

```rust
let mut session = SessionBuilder::<MyConfig>::new()
    .with_num_players(2)
    .with_input_delay(2)
    .with_max_prediction_window(8)
    .add_player(PlayerType::Local, PlayerHandle::new(0))?
    .add_player(PlayerType::Remote(addr), PlayerHandle::new(1))?
    .start_p2p_session(socket)?;
```

**Features:**
- Full rollback support
- Automatic synchronization
- Wait recommendations for frame pacing
- Desync detection via checksums
- Spectator support

### SpectatorSession

For observers who don't contribute inputs:

```rust
let session = SessionBuilder::<MyConfig>::new()
    .with_num_players(2)
    .start_spectator_session(host_addr, socket);
```

**Features:**
- Receives confirmed inputs from host
- Catchup mechanism when falling behind
- No rollback needed (always has confirmed inputs)

### SyncTestSession

For testing determinism locally:

```rust
let session = SessionBuilder::<MyConfig>::new()
    .with_num_players(1)
    .with_check_distance(2)
    .start_synctest_session()?;
```

**Features:**
- Simulates rollback every frame
- Compares checksums after resimulation
- Detects non-deterministic behavior

---

## Synchronization Protocol

### Initial Sync

Before gameplay begins, peers exchange sync packets:

```
Peer A                              Peer B
   │                                   │
   │──── SyncRequest(random) ─────────►│
   │◄──── SyncReply(random) ───────────│
   │                                   │
   │    (repeat NUM_SYNC_PACKETS times)│
   │                                   │
   │  Both transition to Running state │
```

This establishes:
- Round-trip time measurement
- Magic number exchange for packet validation
- Confirmation that both peers are ready

### During Gameplay

Regular message exchange:

```
┌──────────────────────────────────────────────────────────────┐
│  Message                                                      │
│  ├── header: MessageHeader                                    │
│  │   ├── magic: u16          // Identifies this session       │
│  │   └── sequence: u16       // For ordering/ack              │
│  └── body: MessageBody                                        │
│      ├── Input { ... }       // Player inputs                 │
│      ├── InputAck { ... }    // Acknowledge received input    │
│      ├── QualityReport       // Frame advantage info          │
│      ├── QualityReply        // Response to quality report    │
│      └── ChecksumReport      // For desync detection          │
└──────────────────────────────────────────────────────────────┘
```

### Time Synchronization

The `TimeSync` component tracks frame advantage:

```
local_frame_advantage = estimated_remote_frame - local_frame
```

If you're ahead of your opponent (positive advantage), you receive `WaitRecommendation` events suggesting you slow down.

---

## Rollback Mechanics

### When Rollback Occurs

Rollback is triggered when:
1. A prediction is proven wrong (received input differs from prediction)
2. A remote player disconnects (need to resimulate with disconnect flag)

### Rollback Process

```rust
// Simplified rollback flow in adjust_gamestate()

let frame_to_load = if sparse_saving {
    last_saved_frame
} else {
    first_incorrect_frame
};

// 1. Load the old state
requests.push(sync_layer.load_frame(frame_to_load)?);
sync_layer.reset_prediction();

// 2. Resimulate each frame
for _ in 0..frames_to_resimulate {
    let inputs = sync_layer.synchronized_inputs(&connect_status);

    if !sparse_saving {
        requests.push(sync_layer.save_current_state());
    }

    sync_layer.advance_frame();
    requests.push(FortressRequest::AdvanceFrame { inputs });
}
```

### Sparse vs. Full Saving

**Full Saving (default):**
- Saves state every frame
- Minimal resimulation on rollback
- Higher memory/CPU for saves

**Sparse Saving:**
- Only saves at confirmed frames
- Longer rollbacks but fewer saves
- Better when save is expensive

---

## State Management

### Prediction Window

The `max_prediction` setting limits how far ahead simulation can run without confirmed inputs:

```
Confirmed    Current
Frame        Frame
  ↓            ↓
  5            13

  Prediction Window = 8 frames

  Frames 6-12 may need rollback if predictions were wrong
```

### Saved States Buffer

States are stored in a circular buffer of size `max_prediction + 1`:

```
max_prediction = 8
buffer size = 9

Frame:  [0] [1] [2] [3] [4] [5] [6] [7] [8]
               ↑                       ↑
           confirmed              current

After advancing past frame 8:
Frame:  [9] [1] [2] [3] [4] [5] [6] [7] [8]
         ↑                             ↑
      current                      oldest
```

### Connection Status

Each player has a `ConnectionStatus`:

```rust
pub struct ConnectionStatus {
    pub disconnected: bool,  // Is the player disconnected?
    pub last_frame: Frame,   // Last frame we received input for
}
```

This is used to:
- Determine the confirmed frame (minimum of all `last_frame` values)
- Skip disconnected players during input collection
- Handle disconnect rollbacks

---

## Network Protocol

### Message Compression

Inputs are delta-compressed:
1. First input sent in full
2. Subsequent inputs XOR'd against previous
3. Receiver reconstructs using same process

### Reliability

The protocol is UDP-based (unreliable) but handles:
- **Lost packets**: Inputs are resent until acknowledged
- **Out-of-order**: Sequence numbers for ordering
- **Duplicates**: Ignored via sequence tracking

### Keep-Alive

Regular packets maintain connection:
- `KEEP_ALIVE_INTERVAL`: 200ms
- `QualityReport` sent every 200ms
- Disconnect after `disconnect_timeout` (default 2s)

---

## Error Handling

### FortressError

All errors use a single enum:

```rust
pub enum FortressError {
    InvalidRequest { info: String },
    InvalidPlayerHandle { handle: PlayerHandle, max_handle: PlayerHandle },
    InvalidFrame { frame: Frame, reason: String },
    NotSynchronized,
    MissingInput { player_handle: PlayerHandle, frame: Frame },
}
```

### Violation Observer

For monitoring internal issues:

```rust
let observer = Arc::new(CollectingObserver::new());
let session = SessionBuilder::<MyConfig>::new()
    .with_violation_observer(observer.clone())
    .start_p2p_session(socket)?;

// Later, check for violations
for violation in observer.violations() {
    eprintln!("Violation: {} - {}", violation.component, violation.message);
}
```

---

## Type Safety Features

### Frame Newtype

Prevents mixing frame numbers with arbitrary integers:

```rust
let frame = Frame::new(5);
let next = frame + 1;        // OK: Frame + i32 -> Frame
let diff = next - frame;     // OK: Frame - Frame -> i32
// let bad = 5 + frame;      // Compile error: no i32 + Frame
```

### PlayerHandle Newtype

Prevents invalid handle usage:

```rust
let handle = PlayerHandle::new(0);
if handle.is_valid_player_for(num_players) {
    // Safe to use as player index
}
```

### Config Trait

Compile-time parameterization bundles all type requirements:

```rust
pub trait Config: 'static {
    type Input: Copy + Clone + PartialEq + Default + Serialize + DeserializeOwned;
    type State: Clone;
    type Address: Clone + PartialEq + Eq + Hash + Debug;
}
```

This ensures:
- Input types are serializable for network transmission
- State types are clonable for saving
- Addresses can be used as map keys

---

## Summary

Fortress Rollback provides a robust foundation for rollback networking:

1. **Sessions** manage the high-level game flow
2. **SyncLayer** coordinates input collection and state management
3. **InputQueue** handles per-player input storage and prediction
4. **UdpProtocol** manages peer communication
5. **Type-safe primitives** (Frame, PlayerHandle) prevent common errors

The request-based API gives you full control over when saves, loads, and advances happen, making integration with your game loop straightforward.

For practical usage examples, see the [User Guide](user-guide.md).
