# Rollback Netcode Conversion Guide

> **A comprehensive guide for converting and adapting a codebase to use rollback netcode in Rust.**
> This document consolidates industry best practices from GGPO, GGRS, fighting game implementations, and community expertise.

## Table of Contents

1. [What is Rollback Netcode?](#what-is-rollback-netcode)
2. [Prerequisites and Requirements](#prerequisites-and-requirements)
3. [Determinism: The Foundation](#determinism-the-foundation)
4. [State Management](#state-management)
5. [Input Handling](#input-handling)
6. [Network Protocol Considerations](#network-protocol-considerations)
7. [Integration Patterns](#integration-patterns)
8. [Performance Optimization](#performance-optimization)
9. [Testing and Debugging](#testing-and-debugging)
10. [Common Pitfalls](#common-pitfalls)
11. [Step-by-Step Conversion Guide](#step-by-step-conversion-guide)

---

## What is Rollback Netcode?

Rollback netcode is a networking technique that reduces perceived latency in real-time multiplayer games by:

1. **Predicting** remote player inputs and continuing simulation without waiting
2. **Rolling back** to a previous state when actual inputs differ from predictions
3. **Re-simulating** forward to the current frame with corrected inputs

### How It Works (5 Steps)

```
1. Save game state at frame N
2. Predict remote inputs and simulate forward
3. When actual inputs arrive for frame N:
   - If prediction was correct: continue normally
   - If prediction was wrong:
     a. Load saved state from frame N
     b. Re-simulate with correct inputs
     c. Apply smoothing to hide corrections
4. Render the result
```

### When Rollback Is Appropriate

| ✅ Good For | ❌ Not Ideal For |
|------------|------------------|
| Fighting games | RTS with hundreds of units |
| Sports games | Very large game states (>16KB) |
| Action games with few entities | Turn-based games |
| Games with discrete inputs | Games requiring >150ms prediction |
| Connections with small lag spikes | Connections with 500ms+ spikes |

---

## Prerequisites and Requirements

### Absolute Requirements

1. **100% Determinism** — Same inputs MUST produce identical outputs on all machines
2. **State Serialization** — Must save/restore complete game state efficiently
3. **Fixed Timestep** — Simulation must run at consistent tick rate
4. **Efficient Simulation** — Must re-simulate multiple frames within one display frame

### Rust-Specific Advantages

Rust is well-suited for rollback netcode because:

- **IEEE-754 Compliance**: Rust's `f32`/`f64` are (mostly) IEEE-754 compliant
- **No JIT**: AOT compilation prevents runtime operation reordering
- **Memory Safety**: Eliminates bugs common in C-based GGPO implementations
- **ECS Ecosystem**: Bevy/Legion make state serialization easier
- **Fixed-Point Libraries**: Good ecosystem for deterministic math

---

## Determinism: The Foundation

### Critical: Sources of Non-Determinism

| Source | Problem | Solution |
|--------|---------|----------|
| **HashMap iteration** | Order varies between runs | Use `BTreeMap` or `IndexMap` |
| **Floating-point transcendentals** | `sin`/`cos`/etc. differ by platform | Use fixed-point or `libm` |
| **System time** | `Instant::now()` varies | Use frame counters only |
| **Thread scheduling** | Execution order varies | Single-threaded simulation |
| **Memory addresses** | Pointer values differ | Never use addresses in logic |
| **usize** | Different on 32/64-bit | Use explicit `u32`/`u64` |
| **Query iteration (ECS)** | Order not guaranteed | Sort by stable ID |

### Floating-Point Determinism

**Option 1: Control Floating-Point Precisely**

```rust
// For Bevy, enable libm in glam to disable SIMD
// Cargo.toml
glam = { version = "0.29", features = ["libm"] }
```

**Option 2: Use Fixed-Point Math (Recommended for Fighting Games)**

```rust
// Cargo.toml
[dependencies]
fixed = "1.29"
cordic = "0.1"  # For sin/cos/sqrt on fixed-point

// Usage
use fixed::types::I32F32;

let position: I32F32 = I32F32::from_num(10.5);
let velocity: I32F32 = I32F32::from_num(2.25);
let new_pos = position + velocity;
```

### Deterministic RNG

```rust
use rand_pcg::Pcg64;
use rand::SeedableRng;

// All peers must use the SAME seed
let mut rng = Pcg64::seed_from_u64(shared_game_seed);

// RNG state must be saved/restored with game state
struct GameState {
    rng: Pcg64,  // Include in snapshots!
    // ... other fields
}
```

### Deterministic Collections

```rust
// ❌ FORBIDDEN - Non-deterministic iteration
use std::collections::HashMap;
let map: HashMap<PlayerId, Position> = /* ... */;
for (id, pos) in &map {  // Order varies!
    // ...
}

// ✅ REQUIRED - Deterministic iteration
use std::collections::BTreeMap;
let map: BTreeMap<PlayerId, Position> = /* ... */;
for (id, pos) in &map {  // Ordered by key
    // ...
}

// ✅ ALTERNATIVE - Insertion-order preservation
use indexmap::IndexMap;
let map: IndexMap<PlayerId, Position> = /* ... */;
```

---

## State Management

### What Must Be Saved

Every piece of game state that affects simulation must be serialized:

```rust
struct GameState {
    // Player state
    players: Vec<Player>,

    // Entity state
    entities: Vec<Entity>,
    positions: Vec<Position>,
    velocities: Vec<Velocity>,

    // Global state
    frame_number: u32,
    rng_state: Pcg64,

    // Game-specific
    score: [u32; 2],
    timer: u32,

    // DO NOT INCLUDE:
    // - Render state (animations, particles)
    // - Audio state
    // - UI state
    // - Debug info
}
```

### Snapshot Strategies

**Full State Copy (Simple)**

```rust
impl GameState {
    fn save(&self) -> SavedState {
        SavedState {
            data: self.clone(),
            frame: self.frame_number,
        }
    }

    fn load(&mut self, saved: &SavedState) {
        *self = saved.data.clone();
    }
}
```

**Delta State (Optimized)**

```rust
// Only save what changed
struct DeltaSnapshot {
    frame: u32,
    changed_entities: Vec<(EntityId, EntityState)>,
    changed_players: Vec<(PlayerId, PlayerState)>,
}

// Track changes per frame
struct ChangeTracker {
    changed_this_frame: HashSet<EntityId>,
}
```

### Circular Buffer for Rollback States

```rust
const MAX_ROLLBACK_FRAMES: usize = 20;

struct StateBuffer {
    states: [Option<GameState>; MAX_ROLLBACK_FRAMES],
    current_index: usize,
}

impl StateBuffer {
    fn save(&mut self, state: GameState, frame: u32) {
        let index = (frame as usize) % MAX_ROLLBACK_FRAMES;
        self.states[index] = Some(state);
    }

    fn load(&self, frame: u32) -> Option<&GameState> {
        let index = (frame as usize) % MAX_ROLLBACK_FRAMES;
        self.states[index].as_ref()
    }
}
```

---

## Input Handling

### Input Type Design

```rust
// Compact representation using bitflags
use bitflags::bitflags;

bitflags! {
    #[derive(Clone, Copy, PartialEq, Eq, Default)]
    #[repr(transparent)]
    pub struct PlayerInput: u16 {
        const UP    = 0b0000_0001;
        const DOWN  = 0b0000_0010;
        const LEFT  = 0b0000_0100;
        const RIGHT = 0b0000_1000;
        const A     = 0b0001_0000;
        const B     = 0b0010_0000;
        const START = 0b0100_0000;
    }
}

// Must be POD (Plain Old Data) for network transmission
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameInput {
    pub buttons: PlayerInput,
    pub stick_x: i8,  // -128 to 127
    pub stick_y: i8,
}

// Implement bytemuck for zero-copy serialization
unsafe impl bytemuck::Pod for FrameInput {}
unsafe impl bytemuck::Zeroable for FrameInput {}
```

### Input Prediction Strategies

**Strategy 1: Repeat Last Input (Default)**

```rust
fn predict_input(last_known: FrameInput) -> FrameInput {
    last_known  // Assume player continues doing same thing
}
```

**Strategy 2: Input Decay (Rocket League Style)**

```rust
fn predict_input(last_known: FrameInput, frames_since_known: u32) -> FrameInput {
    let decay = match frames_since_known {
        0 => 1.0,
        1 => 0.66,
        2 => 0.33,
        _ => 0.0,
    };

    FrameInput {
        buttons: if decay > 0.5 { last_known.buttons } else { PlayerInput::empty() },
        stick_x: (last_known.stick_x as f32 * decay) as i8,
        stick_y: (last_known.stick_y as f32 * decay) as i8,
    }
}
```

### Input Queue Management

```rust
struct InputQueue {
    inputs: VecDeque<(Frame, FrameInput)>,
    last_confirmed_frame: Frame,
    prediction_head: Frame,
}

impl InputQueue {
    fn add_confirmed_input(&mut self, frame: Frame, input: FrameInput) {
        self.inputs.push_back((frame, input));
        self.last_confirmed_frame = frame;
    }

    fn get_input(&self, frame: Frame) -> (FrameInput, InputStatus) {
        // Check if we have confirmed input
        if let Some((_, input)) = self.inputs.iter().find(|(f, _)| *f == frame) {
            return (*input, InputStatus::Confirmed);
        }

        // Predict based on last known
        let predicted = self.predict_for_frame(frame);
        (predicted, InputStatus::Predicted)
    }
}

enum InputStatus {
    Confirmed,
    Predicted,
}
```

---

## Network Protocol Considerations

### Use UDP, Not TCP

**Why UDP:**

- No head-of-line blocking
- No automatic retransmission of stale data
- Lower latency for real-time data

**What You Need to Build on UDP:**

- Packet sequencing
- Selective acknowledgment
- Redundant input sending (send last N inputs in each packet)

```rust
#[repr(C)]
struct InputPacket {
    sequence: u32,
    start_frame: u32,
    ack_frame: u32,
    inputs: [FrameInput; 8],  // Last 8 inputs for redundancy
}
```

### Glenn Fiedler's Reliable UDP Pattern

```rust
// Include recent inputs in every packet for redundancy
struct GamePacket {
    sequence_number: u32,
    ack: u32,
    ack_bits: u32,  // Bitfield for last 32 packets

    // Include inputs for frames [start_frame, start_frame + count)
    input_start_frame: u32,
    input_count: u8,
    inputs: Vec<FrameInput>,
}
```

### Transport Abstraction

```rust
// Abstract over different transports
pub trait NetworkSocket: Send + Sync {
    fn send_to(&mut self, msg: &[u8], addr: SocketAddr) -> Result<(), NetworkError>;
    fn receive_from(&mut self) -> Result<Option<(Vec<u8>, SocketAddr)>, NetworkError>;
}

// Implementations for different backends
struct UdpSocket { /* ... */ }
struct SteamSocket { /* ... */ }
struct WebRtcSocket { /* ... */ }
```

---

## Integration Patterns

### Request-Response API (GGRS Style)

```rust
// Instead of callbacks, return requests that caller must fulfill
pub enum GgrsRequest<T: Config> {
    SaveGameState { cell: GameStateCell<T>, frame: Frame },
    LoadGameState { cell: GameStateCell<T>, frame: Frame },
    AdvanceFrame { inputs: Vec<(T::Input, InputStatus)> },
}

// Main loop
loop {
    // Collect local input
    session.add_local_input(local_handle, input)?;

    // Advance frame - returns requests
    match session.advance_frame() {
        Ok(requests) => {
            for request in requests {
                match request {
                    GgrsRequest::SaveGameState { cell, frame } => {
                        cell.save(frame, Some(game_state.clone()), checksum);
                    }
                    GgrsRequest::LoadGameState { cell, frame } => {
                        // LoadGameState is only requested for previously saved frames.
                        // Missing state indicates a library bug, but we handle gracefully.
                        if let Some(loaded) = cell.load() {
                            game_state = loaded;
                        } else {
                            eprintln!("WARNING: LoadGameState for frame {frame:?} but no state found");
                        }
                    }
                    GgrsRequest::AdvanceFrame { inputs } => {
                        game_state.advance(inputs);
                    }
                }
            }
        }
        Err(GgrsError::PredictionThreshold) => {
            // Too far ahead, skip this frame
        }
        Err(e) => return Err(e),
    }
}
```

### Bevy Integration Pattern

```rust
use bevy::prelude::*;
use bevy_ggrs::prelude::*;

// 1. Define your config
struct MyConfig;
impl ggrs::Config for MyConfig {
    type Input = PlayerInput;
    type State = u8;
    type Address = SocketAddr;
}

// 2. Register rollback components
fn main() {
    App::new()
        .add_plugins(GgrsPlugin::<MyConfig>::default())
        .rollback_component_with_clone::<Transform>()
        .rollback_component_with_clone::<Velocity>()
        .rollback_resource_with_clone::<FrameCount>()
        .add_systems(GgrsSchedule, (
            movement_system,
            collision_system,
        ).chain())
        .add_systems(ReadInputs, read_local_inputs)
        .run();
}

// 3. Sort queries by Rollback ID for determinism
fn movement_system(mut query: Query<(&Rollback, &mut Transform, &Velocity)>) {
    let mut items: Vec<_> = query.iter_mut().collect();
    items.sort_by_key(|(rb, _, _)| rb.id());

    for (_, mut transform, velocity) in items {
        transform.translation += velocity.0;
    }
}
```

---

## Performance Optimization

### Frame Budget Constraint

At 60 FPS with potential 8-frame rollback:

- Total frame budget: 16.67ms
- If rolling back 8 frames: ~2ms per tick maximum

### Optimization Techniques

**1. Separate Physics from Rendering**

```rust
struct PhysicsState {
    // Only simulation-relevant data
    positions: Vec<Position>,
    velocities: Vec<Velocity>,
}

struct RenderState {
    // Visual-only data (not rolled back)
    animations: Vec<Animation>,
    particles: Vec<Particle>,
}
```

**2. Delta Rollback (Only Track Changes)**

```rust
struct PropertyManager {
    changes: Vec<PropertyChange>,
}

struct PropertyChange {
    frame: Frame,
    entity: EntityId,
    property: PropertyId,
    old_value: Value,
    new_value: Value,
}

// Only ~5% of objects change per frame
// Massive savings over full state copy
```

**3. Deferred Destruction**

```rust
struct DeferredDestruction {
    entity: EntityId,
    destroy_at_frame: Frame,
    state_backup: EntityState,
}

// Keep destroyed entities in memory for MAX_ROLLBACK frames
// Instant restoration if rollback cancels destruction
```

**4. Sparse Saving**

```rust
// Only save confirmed frames, not every prediction
impl Session {
    fn set_sparse_saving(&mut self, enabled: bool) {
        self.sparse_saving = enabled;
    }
}
```

### Performance Monitoring

```rust
struct RollbackMetrics {
    frames_rolled_back: u32,
    max_rollback_depth: u32,
    average_resim_time_us: u64,
    state_save_time_us: u64,
    state_load_time_us: u64,
}
```

---

## Testing and Debugging

### SyncTest Session

Force rollback every frame to catch determinism bugs early:

```rust
// Creates a session that simulates rollbacks every frame
let session = SessionBuilder::<Config>::new()
    .with_num_players(2)
    .start_synctest_session()?;

// Run your game - any desync will be caught immediately
```

### Checksum Verification

```rust
use std::hash::{Hash, Hasher};
use seahash::SeaHasher;

fn compute_checksum(state: &GameState) -> u64 {
    let mut hasher = SeaHasher::new();
    state.hash(&mut hasher);
    hasher.finish()
}

// Compare checksums between peers periodically
fn verify_sync(local_checksum: u64, remote_checksum: u64, frame: Frame) {
    if local_checksum != remote_checksum {
        panic!("DESYNC detected at frame {frame}!");
    }
}
```

### Replay System

```rust
struct ReplayRecorder {
    inputs: Vec<(Frame, PlayerId, FrameInput)>,
    initial_state: GameState,
}

impl ReplayRecorder {
    fn record(&mut self, frame: Frame, player: PlayerId, input: FrameInput) {
        self.inputs.push((frame, player, input));
    }

    fn verify_determinism(&self) -> bool {
        // Run simulation twice with same inputs
        let result1 = self.simulate();
        let result2 = self.simulate();
        result1 == result2
    }
}
```

### Network Condition Simulation

```rust
struct ChaosSocket {
    inner: UdpSocket,
    latency_ms: u32,
    jitter_ms: u32,
    packet_loss_percent: f32,
}

impl NetworkSocket for ChaosSocket {
    fn send_to(&mut self, msg: &[u8], addr: SocketAddr) -> Result<(), NetworkError> {
        // Simulate packet loss
        if rand::random::<f32>() < self.packet_loss_percent {
            return Ok(());  // Drop packet
        }
        // Simulate latency + jitter
        let delay = self.latency_ms + rand::random::<u32>() % self.jitter_ms;
        // Queue for delayed send...
        self.inner.send_to(msg, addr)
    }
}
```

---

## Common Pitfalls

### ❌ Pitfall 1: Forgetting to Register State

```rust
// BAD: frame_count not registered for rollback
struct Game {
    entities: Vec<Entity>,  // ✅ Registered
    frame_count: u32,       // ❌ Forgot to include in snapshots!
}
```

### ❌ Pitfall 2: Using Events for Game State

```rust
// BAD: Bevy events are not rolled back
fn damage_system(mut events: EventWriter<DamageEvent>) {
    events.send(DamageEvent { /* ... */ });  // Lost on rollback!
}

// GOOD: Use components
fn damage_system(mut commands: Commands, query: Query<&Health>) {
    commands.entity(target).insert(PendingDamage(10));
}
```

### ❌ Pitfall 3: Non-Deterministic Query Iteration

```rust
// BAD: Query order not guaranteed
fn update_system(query: Query<&mut Position>) {
    for mut pos in &mut query {  // Order may differ between peers!
        // ...
    }
}

// GOOD: Sort by stable ID
fn update_system(query: Query<(&Rollback, &mut Position)>) {
    let mut items: Vec<_> = query.iter_mut().collect();
    items.sort_by_key(|(rb, _)| rb.id());
    // Now safe to iterate
}
```

### ❌ Pitfall 4: Using System Time

```rust
// BAD: Wall clock time varies
fn update(time: Res<Time>) {
    let delta = time.delta_seconds();  // Different on each machine!
}

// GOOD: Fixed timestep
const TICK_RATE: f32 = 1.0 / 60.0;
fn update() {
    let delta = TICK_RATE;  // Same everywhere
}
```

### ❌ Pitfall 5: Floating-Point Accumulation

```rust
// BAD: Accumulating floats diverges over time
position += velocity * delta;  // Small errors compound!

// GOOD: Reconstruct from integers or use fixed-point
position = start_position + velocity * frame_count;
```

---

## Step-by-Step Conversion Guide

### Phase 1: Achieve Determinism (MUST DO FIRST)

1. **Audit all game state** — List every variable that affects simulation
2. **Replace HashMap with BTreeMap** — Or sort before iteration
3. **Fix floating-point issues** — Enable `libm` or use fixed-point
4. **Use deterministic RNG** — `rand_pcg` with shared seed
5. **Remove system time dependencies** — Use frame counters
6. **Add replay feature** — Verify same inputs = same outputs

### Phase 2: Implement State Management

1. **Define serializable game state** — All simulation-relevant data
2. **Implement save/load** — Clone or serialize entire state
3. **Create circular buffer** — Store last N frames
4. **Add checksum computation** — For desync detection

### Phase 3: Add Input Handling

1. **Design compact input type** — Bitflags, POD
2. **Implement input queue** — Per-player input history
3. **Add input prediction** — Start with "repeat last"
4. **Track input status** — Confirmed vs Predicted

### Phase 4: Integrate Networking

1. **Abstract socket interface** — Support multiple transports
2. **Implement UDP protocol** — Or use existing (GGRS)
3. **Add redundant input sending** — Survive packet loss
4. **Implement time synchronization** — `frames_ahead()` tracking

### Phase 5: Build Rollback Loop

1. **Handle requests** — Save/Load/Advance pattern
2. **Implement rollback** — Load state, re-simulate
3. **Add smoothing** — Interpolate corrections
4. **Performance tune** — Profile and optimize

### Phase 6: Testing and Hardening

1. **Run SyncTest** — Every frame rollback
2. **Add checksum verification** — Catch desyncs
3. **Test with network simulation** — Latency, jitter, loss
4. **Stress test** — Many players, high latency

---

## Key Resources

| Resource | Description |
|----------|-------------|
| [GGPO.net](https://www.ggpo.net/) | Original rollback library and documentation |
| [GafferOnGames](https://gafferongames.com/) | Glenn Fiedler's networking articles (foundational) |
| [Gabriel Gambetta](https://www.gabrielgambetta.com/client-server-game-architecture.html) | Client prediction and reconciliation |
| [Killer Instinct Netcode](https://ki.infil.net/w02-netcode.html) | Practical fighting game netcode guide |
| [SnapNet Blog](https://www.snapnet.dev/blog/) | Modern netcode architecture articles |
| [8 Frames in 16ms](https://www.youtube.com/watch?v=7jb0FOcImdg) | Mortal Kombat rollback GDC talk |
| [Overwatch Netcode](https://www.gdcvault.com/play/1024001/) | Server-authoritative rollback at scale |

---

## Summary Checklist

Before shipping rollback netcode:

- [ ] **Determinism verified** — Replay produces identical results
- [ ] **All state registered** — Nothing escapes snapshots
- [ ] **No HashMap iteration** — BTreeMap or sorted
- [ ] **Fixed-point or controlled floats** — No platform divergence
- [ ] **Deterministic RNG** — Seeded and saved with state
- [ ] **Frame counters not time** — No system time in simulation
- [ ] **UDP-based protocol** — Not TCP
- [ ] **Redundant input sending** — Handles packet loss
- [ ] **SyncTest passing** — No desyncs with forced rollback
- [ ] **Checksum verification** — Catches desyncs early
- [ ] **Performance profiled** — Meets frame budget during rollback

---

*This guide consolidates best practices from GGPO, GGRS, fighting game developers, and the broader game networking community. Last updated: December 2025.*
