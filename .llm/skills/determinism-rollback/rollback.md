<!-- CATEGORY: Determinism & Rollback -->
<!-- WHEN: Converting games to rollback netcode, engine integration, state management -->

# Rollback Netcode

Rollback netcode reduces perceived latency by predicting remote inputs, then rolling back and re-simulating when predictions are wrong.

---

## Requirements

1. **100% Determinism** -- same inputs produce identical outputs on all machines
2. **State serialization** -- save/restore complete game state efficiently
3. **Fixed timestep** -- consistent tick rate (e.g., 60 Hz)
4. **Fast re-simulation** -- must re-simulate up to 8 frames within one display frame (~2ms/tick)

---

## Conversion Checklist

### Phase 1: Determinism (Do First)

- [ ] Replace `HashMap` with `BTreeMap` (or sort before iteration)
- [ ] Enable `libm` or use fixed-point math
- [ ] Use seeded `rand_pcg` for RNG (saved with state)
- [ ] Replace `Instant::now()` with frame counters
- [ ] Sort all ECS queries by stable `Rollback` ID
- [ ] Add replay verification (same inputs = same outputs)

### Phase 2: State Management

- [ ] Define serializable game state (all simulation-relevant data)
- [ ] Implement save/load (Clone or serialize)
- [ ] Create circular buffer for last N frames
- [ ] Add checksum computation for desync detection

### Phase 3: Input System

- [ ] Design compact input type (bitflags, POD)
- [ ] Implement per-player input queue with prediction
- [ ] Track confirmed vs predicted input status

### Phase 4: Networking

- [ ] Use UDP (not TCP) -- no head-of-line blocking
- [ ] Send redundant inputs (last N inputs per packet)
- [ ] Implement time synchronization (`frames_ahead()`)

### Phase 5: Rollback Loop

- [ ] Handle Save/Load/Advance requests
- [ ] Add visual smoothing for corrections

---

## Request-Response API Pattern

```rust
pub enum RollbackRequest<T: Config> {
    SaveGameState { cell: GameStateCell<T>, frame: Frame },
    LoadGameState { cell: GameStateCell<T>, frame: Frame },
    AdvanceFrame { inputs: PlayerInputs<T> },
}

fn game_loop(session: &mut Session<Config>, game: &mut Game) -> Result<(), Error> {
    session.poll_remote_clients();
    session.add_local_input(local_handle, game.read_local_input())?;

    for request in session.advance_frame()? {
        match request {
            RollbackRequest::SaveGameState { cell, frame } => {
                cell.save(frame, Some(game.state.clone()), Some(game.checksum()));
            }
            RollbackRequest::LoadGameState { cell, frame } => {
                if let Some(loaded) = cell.load() {
                    game.state = loaded;
                }
            }
            RollbackRequest::AdvanceFrame { inputs } => {
                game.advance_frame(&inputs);
            }
        }
    }
    Ok(())
}
```

---

## State Management

### What Must Be Saved

```rust
struct SimulationState {
    players: Vec<Player>,
    entities: Vec<Entity>,
    frame_number: u32,
    rng_state: Pcg64,
    // DO NOT include: render state, audio, UI, debug info
}
```

### Separation of Concerns

```rust
struct Game {
    sim: SimulationState,           // Saved/restored during rollback
    presentation: PresentationState, // Never rolled back (animations, particles)
}
```

### Circular Buffer

```rust
const MAX_ROLLBACK_FRAMES: usize = 20;

struct StateBuffer {
    states: [Option<GameState>; MAX_ROLLBACK_FRAMES],
}

impl StateBuffer {
    fn save(&mut self, state: GameState, frame: u32) {
        self.states[(frame as usize) % MAX_ROLLBACK_FRAMES] = Some(state);
    }
    fn load(&self, frame: u32) -> Option<&GameState> {
        self.states[(frame as usize) % MAX_ROLLBACK_FRAMES].as_ref()
    }
}
```

---

## Bevy Integration

```rust
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
        .run();
}

fn movement_system(mut query: Query<(&Rollback, &mut Transform, &Velocity)>) {
    let mut items: Vec<_> = query.iter_mut().collect();
    items.sort_by_key(|(rb, _, _)| rb.id());
    for (_, mut transform, velocity) in items {
        transform.translation += velocity.0;
    }
}
```

### Bevy Features to Avoid in Rollback Systems

| Feature | Problem | Alternative |
|---------|---------|-------------|
| `Events<T>` | Not snapshot/restored | Use components |
| `Local<T>` | Not snapshot/restored | Use resources |
| `Added<T>` / `Changed<T>` | Triggers on every restore | Avoid in game logic |
| `Time` | Uses wall clock | Use frame counter resource |

---

## Input Design

```rust
use bitflags::bitflags;

bitflags! {
    #[derive(Clone, Copy, PartialEq, Eq, Default)]
    #[repr(transparent)]
    pub struct PlayerInput: u16 {
        const UP = 0b0000_0001;
        const DOWN = 0b0000_0010;
        const LEFT = 0b0000_0100;
        const RIGHT = 0b0000_1000;
        const A = 0b0001_0000;
        const B = 0b0010_0000;
    }
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameInput {
    pub buttons: PlayerInput,
    pub stick_x: i8,
    pub stick_y: i8,
}

// Default prediction: repeat last known input
fn predict_input(last_known: FrameInput) -> FrameInput { last_known }
```

---

## Network Protocol

```rust
#[repr(C)]
struct InputPacket {
    sequence: u32,
    start_frame: u32,
    ack_frame: u32,
    inputs: [FrameInput; 8], // Redundant: last 8 inputs
}
```

Use Glenn Fiedler's reliable UDP pattern: sequence number + ack + ack_bits bitfield.

---

## Checksum & Desync Detection

```rust
fn compute_checksum(state: &GameState) -> u64 {
    let mut hasher = seahash::SeaHasher::new();
    state.frame.hash(&mut hasher);
    for player in &state.players {
        player.position.x.to_bits().hash(&mut hasher);
        player.position.y.to_bits().hash(&mut hasher);
    }
    hasher.finish()
}
```

Exchange checksums periodically between peers. Mismatch = desync.

---

## Custom Engine Interface

```rust
pub trait RollbackEngine {
    type State: Clone;
    type Input: Copy + Default;

    fn save_state(&self) -> Self::State;
    fn load_state(&mut self, state: &Self::State);
    fn advance(&mut self, inputs: &[Self::Input]);
    fn checksum(&self) -> u64;
}
```

---

## Testing

- **SyncTest**: `SessionBuilder::start_synctest_session()` forces rollback every frame
- **Replay**: Record inputs, replay twice, compare final checksums
- **Network simulation**: Test with latency, jitter, and packet loss

---

## Performance Targets

- State save/load: < 1ms
- Re-simulate 8 frames: < 16ms total
- Checksum computation: < 0.5ms
- No allocations in hot path

---

## Common Pitfalls

| Pitfall | Fix |
|---------|-----|
| Forgot to register state for rollback | Audit all simulation-affecting fields |
| Using Bevy Events for game state | Use components instead |
| Non-deterministic query order | Sort by `Rollback` ID |
| Wall clock time in simulation | Use `TICK_RATE` constant |
| Float accumulation | Reconstruct from integers |
