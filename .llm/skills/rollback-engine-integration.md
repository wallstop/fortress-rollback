# Game Engine Rollback Integration Patterns

> **Patterns and best practices for integrating rollback netcode with game engines in Rust.**
> Covers Bevy, custom engines, and engine-agnostic patterns.

## Overview

Integrating rollback netcode with a game engine requires careful architecture. This guide covers proven patterns for state management, determinism, and the rollback lifecycle.

---

## Core Architecture Patterns

### Pattern 1: Request-Response API

Instead of callbacks, return requests that the caller must fulfill in order. This gives explicit control over the rollback lifecycle.

```rust
/// Requests returned by advance_frame()
pub enum RollbackRequest<T: Config> {
    /// Save the current game state for potential rollback
    SaveGameState { 
        cell: GameStateCell<T>, 
        frame: Frame 
    },
    /// Load a previously saved state (rollback is happening)
    LoadGameState { 
        cell: GameStateCell<T>, 
        frame: Frame 
    },
    /// Advance the simulation one frame with these inputs
    AdvanceFrame { 
        inputs: PlayerInputs<T> 
    },
}

/// Main game loop
fn game_loop(session: &mut Session<Config>, game: &mut Game) -> Result<(), Error> {
    // 1. Poll network
    session.poll_remote_clients();
    
    // 2. Add local input
    let input = game.read_local_input();
    session.add_local_input(local_handle, input)?;
    
    // 3. Process requests in order
    for request in session.advance_frame()? {
        match request {
            RollbackRequest::SaveGameState { cell, frame } => {
                let checksum = game.compute_checksum();
                cell.save(frame, Some(game.state.clone()), Some(checksum));
            }
            RollbackRequest::LoadGameState { cell, .. } => {
                game.state = cell.load().expect("saved state must exist");
            }
            RollbackRequest::AdvanceFrame { inputs } => {
                game.advance_frame(&inputs);
            }
        }
    }
    
    Ok(())
}
```

### Pattern 2: Separation of Concerns

Separate simulation state (rolled back) from presentation state (not rolled back):

```rust
/// State that participates in rollback
#[derive(Clone)]
struct SimulationState {
    positions: Vec<Position>,
    velocities: Vec<Velocity>,
    health: Vec<Health>,
    rng: Pcg64,
    frame: u32,
}

/// State that is NOT rolled back (visual only)
struct PresentationState {
    animations: HashMap<EntityId, Animation>,
    particles: Vec<Particle>,
    sounds_to_play: Vec<SoundEffect>,
    camera_shake: f32,
}

/// The game manages both
struct Game {
    sim: SimulationState,      // Saved and restored
    presentation: PresentationState,  // Never rolled back
}

impl Game {
    fn advance_frame(&mut self, inputs: &PlayerInputs) {
        // Update simulation (deterministic)
        self.sim.update(inputs);
        
        // Derive presentation from simulation
        // (re-derived after rollback, so always correct)
        self.presentation.sync_with(&self.sim);
    }
}
```

### Pattern 3: Component Registration System

For ECS architectures, register which components participate in rollback:

```rust
/// Marker trait for rollback-able components
trait RollbackComponent: Clone + Send + Sync + 'static {}

/// Builder pattern for registering components
struct RollbackBuilder {
    components: Vec<ComponentRegistration>,
    resources: Vec<ResourceRegistration>,
}

impl RollbackBuilder {
    /// Register a component type for automatic snapshot/restore
    pub fn register_component<C: RollbackComponent>(&mut self) -> &mut Self {
        self.components.push(ComponentRegistration::new::<C>());
        self
    }
    
    /// Register a resource for automatic snapshot/restore
    pub fn register_resource<R: Clone + Send + Sync + 'static>(&mut self) -> &mut Self {
        self.resources.push(ResourceRegistration::new::<R>());
        self
    }
}

// Usage
let builder = RollbackBuilder::new()
    .register_component::<Position>()
    .register_component::<Velocity>()
    .register_component::<Health>()
    .register_resource::<FrameCount>()
    .register_resource::<RngState>();
```

---

## Bevy-Specific Patterns

### Schedule-Based Architecture

Bevy's schedule system maps well to rollback:

```rust
/// Custom schedules for rollback lifecycle
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub enum RollbackSchedules {
    /// Read local player inputs
    ReadInputs,
    /// Main simulation schedule (runs during rollback too)
    Advance,
    /// Save world state after advancing
    SaveWorld,
    /// Load world state (when rolling back)
    LoadWorld,
}

fn build_app() -> App {
    App::new()
        .init_schedule(RollbackSchedules::ReadInputs)
        .init_schedule(RollbackSchedules::Advance)
        .init_schedule(RollbackSchedules::SaveWorld)
        .init_schedule(RollbackSchedules::LoadWorld)
        // Add systems to appropriate schedules
        .add_systems(RollbackSchedules::ReadInputs, read_local_inputs)
        .add_systems(RollbackSchedules::Advance, (
            apply_inputs,
            movement_system,
            collision_system,
            damage_system,
        ).chain())
}
```

### Deterministic Query Iteration

**Critical: Bevy queries do not guarantee order.** Always sort by a stable ID:

```rust
/// Marker component with stable ID for deterministic ordering
#[derive(Component, Clone, Copy)]
pub struct Rollback(pub u32);

impl Rollback {
    pub fn id(&self) -> u32 {
        self.0
    }
}

/// WRONG: Non-deterministic iteration
fn bad_system(query: Query<&mut Position>) {
    for mut pos in &mut query {
        // ❌ Order may differ between peers!
    }
}

/// CORRECT: Sort by Rollback ID
fn good_system(mut query: Query<(&Rollback, &mut Position)>) {
    let mut items: Vec<_> = query.iter_mut().collect();
    items.sort_by_key(|(rb, _)| rb.id());
    
    for (_, mut pos) in items {
        // ✅ Deterministic order
    }
}

/// HELPER: Macro or extension trait for common pattern
trait DeterministicQuery<'w, 's, Q: QueryData, F: QueryFilter> {
    fn iter_deterministic(&mut self) -> impl Iterator<Item = Q::Item<'_>>;
}
```

### Snapshot Strategies

Different strategies for different component types:

```rust
/// Strategy trait for how to snapshot a component
pub trait SnapshotStrategy<C> {
    fn save(component: &C) -> Self::Snapshot;
    fn load(snapshot: &Self::Snapshot) -> C;
}

/// Clone strategy - for most components
pub struct CloneStrategy;

impl<C: Clone> SnapshotStrategy<C> for CloneStrategy {
    type Snapshot = C;
    
    fn save(component: &C) -> C {
        component.clone()
    }
    
    fn load(snapshot: &C) -> C {
        snapshot.clone()
    }
}

/// Copy strategy - for simple POD types
pub struct CopyStrategy;

impl<C: Copy> SnapshotStrategy<C> for CopyStrategy {
    type Snapshot = C;
    
    fn save(component: &C) -> C {
        *component
    }
    
    fn load(snapshot: &C) -> C {
        *snapshot
    }
}

/// Custom strategy - for complex serialization needs
pub struct SerdeStrategy;

impl<C: Serialize + DeserializeOwned> SnapshotStrategy<C> for SerdeStrategy {
    type Snapshot = Vec<u8>;
    
    fn save(component: &C) -> Vec<u8> {
        bincode::serialize(component).expect("serialization failed")
    }
    
    fn load(snapshot: &Vec<u8>) -> C {
        bincode::deserialize(snapshot).expect("deserialization failed")
    }
}
```

### Handling Entity References

Entity IDs can change during rollback. Use `MapEntities` for cross-references:

```rust
use bevy::ecs::entity::MapEntities;

/// Component that references another entity
#[derive(Component, Clone)]
struct Target {
    entity: Entity,
}

impl MapEntities for Target {
    fn map_entities<M: EntityMapper>(&mut self, mapper: &mut M) {
        self.entity = mapper.map_entity(self.entity);
    }
}

/// During rollback, entity IDs are remapped
fn remap_entities(world: &mut World, entity_map: &EntityMap) {
    // Components implementing MapEntities are automatically updated
}
```

### What NOT to Use in Rollback Systems

| Bevy Feature | Problem | Alternative |
|--------------|---------|-------------|
| `Events<T>` | Not snapshot/restored | Use components |
| `Local<T>` | Not snapshot/restored | Use resources |
| `Added<T>` / `Changed<T>` | Triggers on every restore | Avoid in game logic |
| `GlobalTransform` | Only updated in PostUpdate | Manually propagate |
| `Time` | Uses wall clock | Use frame counter resource |
| Standard states | Not integrated | Use rollback-aware states |

---

## Custom Engine Patterns

### Minimal Engine Interface

Define the minimum interface your rollback system needs:

```rust
/// Trait that any game engine must implement for rollback
pub trait RollbackEngine {
    /// The type representing complete game state
    type State: Clone;
    
    /// The type representing player input
    type Input: Copy + Default;
    
    /// Save the current state
    fn save_state(&self) -> Self::State;
    
    /// Restore to a previous state
    fn load_state(&mut self, state: &Self::State);
    
    /// Advance simulation by one frame with given inputs
    fn advance(&mut self, inputs: &[Self::Input]);
    
    /// Compute checksum for desync detection
    fn checksum(&self) -> u64;
}

/// Generic rollback manager that works with any engine
pub struct RollbackManager<E: RollbackEngine> {
    engine: E,
    saved_states: CircularBuffer<E::State>,
    input_history: InputHistory<E::Input>,
}

impl<E: RollbackEngine> RollbackManager<E> {
    pub fn rollback_to(&mut self, frame: Frame) {
        if let Some(state) = self.saved_states.get(frame) {
            self.engine.load_state(state);
        }
    }
    
    pub fn resimulate_to(&mut self, target_frame: Frame) {
        let current = self.current_frame();
        for frame in current..target_frame {
            let inputs = self.input_history.get_inputs(frame);
            self.engine.advance(&inputs);
        }
    }
}
```

### State Diffing for Delta Rollback

For large states, only track what changed:

```rust
/// Track property changes for efficient rollback
#[derive(Clone)]
struct PropertyChange {
    entity: EntityId,
    property: PropertyId,
    old_value: PropertyValue,
    frame: Frame,
}

struct DeltaRollback {
    changes: Vec<PropertyChange>,
    max_frames: u32,
}

impl DeltaRollback {
    /// Record a property change
    fn record_change(&mut self, entity: EntityId, property: PropertyId, 
                     old: PropertyValue, frame: Frame) {
        self.changes.push(PropertyChange {
            entity,
            property,
            old_value: old,
            frame,
        });
        
        // Prune old changes
        self.changes.retain(|c| frame - c.frame < self.max_frames);
    }
    
    /// Rollback all changes since target frame
    fn rollback_to(&self, world: &mut World, target_frame: Frame) {
        // Apply changes in reverse order
        for change in self.changes.iter().rev() {
            if change.frame > target_frame {
                world.set_property(change.entity, change.property, &change.old_value);
            }
        }
    }
}
```

---

## Input System Patterns

### Compact Input Representation

```rust
use bitflags::bitflags;

bitflags! {
    /// Compact input representation - fits in 2 bytes
    #[derive(Clone, Copy, PartialEq, Eq, Default)]
    #[repr(transparent)]
    pub struct ButtonInputs: u16 {
        const UP     = 1 << 0;
        const DOWN   = 1 << 1;
        const LEFT   = 1 << 2;
        const RIGHT  = 1 << 3;
        const A      = 1 << 4;
        const B      = 1 << 5;
        const X      = 1 << 6;
        const Y      = 1 << 7;
        const START  = 1 << 8;
        const SELECT = 1 << 9;
        const L1     = 1 << 10;
        const R1     = 1 << 11;
        const L2     = 1 << 12;
        const R2     = 1 << 13;
    }
}

/// Complete input for one player, one frame
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct PlayerInput {
    pub buttons: ButtonInputs,
    pub left_stick_x: i8,   // -128 to 127
    pub left_stick_y: i8,
    pub right_stick_x: i8,
    pub right_stick_y: i8,
    pub _padding: [u8; 2],  // Align to 8 bytes
}

// Enable zero-copy network transmission
unsafe impl bytemuck::Pod for PlayerInput {}
unsafe impl bytemuck::Zeroable for PlayerInput {}
```

### Input Status Tracking

```rust
/// Whether an input is confirmed or predicted
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InputStatus {
    /// Input received from the actual player
    Confirmed,
    /// Input predicted based on previous inputs
    Predicted,
}

/// Input with its status
#[derive(Clone, Copy)]
pub struct TimestampedInput<I> {
    pub input: I,
    pub status: InputStatus,
    pub frame: Frame,
}

/// Per-player input queue
pub struct PlayerInputQueue<I> {
    inputs: VecDeque<TimestampedInput<I>>,
    last_confirmed: Frame,
}

impl<I: Copy + Default> PlayerInputQueue<I> {
    /// Get input for a frame, predicting if necessary
    pub fn get(&self, frame: Frame) -> (I, InputStatus) {
        // Check for confirmed input
        if let Some(input) = self.inputs.iter()
            .find(|i| i.frame == frame && i.status == InputStatus::Confirmed) 
        {
            return (input.input, InputStatus::Confirmed);
        }
        
        // Predict based on last confirmed
        let predicted = self.predict_for(frame);
        (predicted, InputStatus::Predicted)
    }
    
    fn predict_for(&self, _frame: Frame) -> I {
        // Default: repeat last known input
        self.inputs.back()
            .map(|i| i.input)
            .unwrap_or_default()
    }
}
```

---

## Checksum and Desync Detection

### Computing Deterministic Checksums

```rust
use std::hash::{Hash, Hasher};

/// Use seahash for portable, fast checksums
fn checksum_hasher() -> impl Hasher {
    seahash::SeaHasher::new()
}

/// Compute checksum of game state
fn compute_checksum(state: &GameState) -> u64 {
    let mut hasher = checksum_hasher();
    
    // Hash in deterministic order
    hasher.write_u32(state.frame);
    
    for player in &state.players {
        player.hash(&mut hasher);
    }
    
    // Sort entities by ID before hashing
    let mut entities: Vec<_> = state.entities.iter().collect();
    entities.sort_by_key(|e| e.id);
    
    for entity in entities {
        entity.hash(&mut hasher);
    }
    
    hasher.finish()
}

/// Implement Hash carefully - include all state that affects simulation
impl Hash for Player {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        // Hash position as fixed-point or raw bits
        self.position.x.to_bits().hash(state);
        self.position.y.to_bits().hash(state);
        self.health.hash(state);
        self.facing.hash(state);
        // Include ALL simulation-relevant state
    }
}
```

### Desync Detection Protocol

```rust
/// Periodic checksum exchange
struct DesyncDetector {
    local_checksums: HashMap<Frame, u64>,
    remote_checksums: HashMap<Frame, u64>,
    check_interval: u32,
}

impl DesyncDetector {
    fn on_frame_complete(&mut self, frame: Frame, checksum: u64) {
        if frame % self.check_interval == 0 {
            self.local_checksums.insert(frame, checksum);
            // Send checksum to remote peer
        }
    }
    
    fn on_remote_checksum(&mut self, frame: Frame, checksum: u64) -> Option<DesyncInfo> {
        self.remote_checksums.insert(frame, checksum);
        
        // Check for mismatch
        if let Some(&local) = self.local_checksums.get(&frame) {
            if local != checksum {
                return Some(DesyncInfo {
                    frame,
                    local_checksum: local,
                    remote_checksum: checksum,
                });
            }
        }
        None
    }
}

#[derive(Debug)]
struct DesyncInfo {
    frame: Frame,
    local_checksum: u64,
    remote_checksum: u64,
}
```

---

## Time Synchronization

### Frame Advantage Tracking

```rust
/// Track how far ahead/behind we are vs remote peer
struct TimeSynchronizer {
    local_frame: Frame,
    remote_frame: Frame,
    rtt_ms: f32,
}

impl TimeSynchronizer {
    /// How many frames ahead of remote we are
    fn frames_ahead(&self) -> i32 {
        self.local_frame as i32 - self.remote_frame as i32
    }
    
    /// Recommended frame delay to apply
    fn recommended_delay(&self) -> f32 {
        let ahead = self.frames_ahead();
        if ahead > 2 {
            // Slow down - we're too far ahead
            1.1  // 10% slower
        } else if ahead < -2 {
            // Speed up - we're behind
            0.9  // 10% faster
        } else {
            1.0  // Normal speed
        }
    }
}

/// Apply time sync in game loop
fn game_loop_with_sync(sync: &TimeSynchronizer, base_fps: f32) {
    let adjusted_fps = base_fps * sync.recommended_delay();
    let frame_duration = Duration::from_secs_f32(1.0 / adjusted_fps);
    
    // Use adjusted frame duration for timing
}
```

### Skip Frames When Too Far Ahead

```rust
/// Event indicating we should skip frames
struct WaitRecommendation {
    skip_frames: u32,
}

fn handle_wait_recommendation(rec: WaitRecommendation, game: &mut Game) {
    // Option 1: Skip rendering for N frames
    for _ in 0..rec.skip_frames {
        game.advance_simulation_only();
    }
    
    // Option 2: Add artificial delay
    // std::thread::sleep(Duration::from_millis(rec.skip_frames as u64 * 16));
}
```

---

## Debugging Tools

### Visual Rollback Indicator

```rust
/// Debug overlay showing rollback status
struct RollbackDebugOverlay {
    recent_rollbacks: VecDeque<RollbackEvent>,
    max_display: usize,
}

struct RollbackEvent {
    frame: Frame,
    rollback_frames: u32,
    timestamp: Instant,
}

impl RollbackDebugOverlay {
    fn record_rollback(&mut self, current_frame: Frame, target_frame: Frame) {
        self.recent_rollbacks.push_back(RollbackEvent {
            frame: current_frame,
            rollback_frames: current_frame - target_frame,
            timestamp: Instant::now(),
        });
        
        if self.recent_rollbacks.len() > self.max_display {
            self.recent_rollbacks.pop_front();
        }
    }
    
    fn render(&self, ui: &mut Ui) {
        ui.label(format!("Recent rollbacks: {}", self.recent_rollbacks.len()));
        for event in &self.recent_rollbacks {
            let age = event.timestamp.elapsed().as_secs_f32();
            if age < 2.0 {
                ui.label(format!("Frame {}: rolled back {} frames", 
                    event.frame, event.rollback_frames));
            }
        }
    }
}
```

### State Diff Tool

```rust
/// Compare two states to find divergence
fn diff_states(a: &GameState, b: &GameState) -> Vec<StateDiff> {
    let mut diffs = Vec::new();
    
    if a.frame != b.frame {
        diffs.push(StateDiff::Frame { a: a.frame, b: b.frame });
    }
    
    for i in 0..a.players.len().max(b.players.len()) {
        let player_a = a.players.get(i);
        let player_b = b.players.get(i);
        
        match (player_a, player_b) {
            (Some(pa), Some(pb)) if pa != pb => {
                diffs.push(StateDiff::Player { 
                    index: i, 
                    diff: diff_players(pa, pb) 
                });
            }
            (Some(_), None) => diffs.push(StateDiff::PlayerMissing { index: i, in_a: true }),
            (None, Some(_)) => diffs.push(StateDiff::PlayerMissing { index: i, in_a: false }),
            _ => {}
        }
    }
    
    diffs
}

#[derive(Debug)]
enum StateDiff {
    Frame { a: Frame, b: Frame },
    Player { index: usize, diff: PlayerDiff },
    PlayerMissing { index: usize, in_a: bool },
    Entity { id: EntityId, diff: EntityDiff },
}
```

---

## Summary: Integration Checklist

### Before Integration
- [ ] Game loop uses fixed timestep
- [ ] All game state is serializable/cloneable
- [ ] No HashMap iteration in game logic
- [ ] No system time in game logic
- [ ] Deterministic RNG implemented

### During Integration
- [ ] Request-response pattern implemented
- [ ] All rollback components registered
- [ ] Query iteration sorted by stable ID
- [ ] Simulation separated from presentation
- [ ] Checksum computation implemented

### Testing
- [ ] SyncTest mode works (forced rollback every frame)
- [ ] Replay system verifies determinism
- [ ] Desync detection alerts on mismatch
- [ ] Network simulation tested (latency, jitter, loss)

### Performance
- [ ] State save/load under 1ms
- [ ] Can resimulate 8 frames in 16ms
- [ ] Checksum computation under 0.5ms
- [ ] No allocations in hot path

---

*Patterns derived from GGPO, GGRS, bevy_ggrs, and industry best practices.*
