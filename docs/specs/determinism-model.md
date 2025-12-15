<p align="center">
  <img src="../../assets/logo-small.svg" alt="Fortress Rollback" width="64">
</p>

# Fortress Rollback Determinism Model

**Version:** 1.0
**Date:** December 6, 2025
**Status:** Complete

This document specifies the determinism requirements and guarantees of Fortress Rollback. Deterministic execution is **critical** for rollback networking—the same inputs must always produce the same outputs across all clients.

---

## Table of Contents

1. [Overview](#overview)
2. [Determinism Requirements](#determinism-requirements)
3. [Library Guarantees](#library-guarantees)
4. [User Responsibilities](#user-responsibilities)
5. [Common Determinism Pitfalls](#common-determinism-pitfalls)
6. [Verification Strategies](#verification-strategies)
7. [Platform Compatibility](#platform-compatibility)

---

## Overview

### What is Determinism?

In the context of rollback networking, **determinism** means:

```
∀ state S, inputs I:
    advance(S, I) always produces the same result
    regardless of:
        - which machine runs it
        - when it runs
        - how many times it runs
```

### Why Determinism Matters

Rollback networking relies on the ability to:

1. **Predict** the future by simulating with estimated inputs
2. **Rollback** to a previous state when predictions are wrong
3. **Resimulate** with corrected inputs

If resimulation produces different results than the original simulation, the game state becomes **desynced** between clients. Desyncs are fatal to gameplay.

---

## Determinism Requirements

### DETER-1: Collection Iteration Order

**Requirement:** All collection iteration must be deterministic.

**Implementation:**

- ✅ `BTreeMap` used throughout (deterministic iteration by key order)
- ✅ Zero `HashMap` usage in library code
- ✅ `Config::Address` requires `Ord` trait for BTreeMap keys

**Affected Code:**

```rust
// All maps use BTreeMap
local_inputs: BTreeMap<PlayerHandle, PlayerInput>
recv_inputs: BTreeMap<Frame, InputBytes>
remotes: BTreeMap<Address, UdpProtocol>
pending_checksums: BTreeMap<Frame, u128>
```

### DETER-2: No System Time Dependencies

**Requirement:** Game state must not depend on wall-clock time.

**Implementation:**

- ✅ Frame counter is the only "time" in game state
- ✅ `Instant` used only for network timeouts (not game logic)
- ✅ No `SystemTime` in game-critical paths

**User Responsibility:**

```rust
// WRONG - Non-deterministic!
fn update(state: &mut GameState) {
    state.timestamp = SystemTime::now(); // DON'T DO THIS
}

// CORRECT - Use frame number
fn update(state: &mut GameState) {
    state.frame += 1;
}
```

### DETER-3: No Random Number Generation

**Requirement:** No unseeded random numbers in game logic.

**Implementation:**

- ✅ Library uses `rand::random()` only for sync handshake (not game state)
- ✅ No RNG affects input processing or state management

**User Responsibility:**

```rust
// WRONG - Non-deterministic!
fn spawn_enemy(state: &mut GameState) {
    let x = rand::random::<f32>(); // DON'T DO THIS
    state.enemies.push(Enemy { x, y: 0.0 });
}

// CORRECT - Seeded RNG synchronized across clients
fn spawn_enemy(state: &mut GameState, rng: &mut SeededRng) {
    let x = rng.next_f32(); // RNG state is part of game state
    state.enemies.push(Enemy { x, y: 0.0 });
}
```

### DETER-4: Serialization Stability

**Requirement:** Serialized data must be identical across platforms.

**Implementation:**

- ✅ `bincode` with default configuration (little-endian, fixed-size integers)
- ✅ Input types require `Serialize + Deserialize`
- ✅ No platform-specific serialization

**Constraints on Input Types:**

```rust
// Must implement these traits
pub trait Config: 'static {
    type Input: Copy + Clone + PartialEq + Default
              + Serialize + DeserializeOwned;
    // ...
}
```

### DETER-5: No Uninitialized Memory

**Requirement:** All state must be explicitly initialized.

**Implementation:**

- ✅ `#![forbid(unsafe_code)]` prevents uninitialized memory access
- ✅ All arrays initialized with default values
- ✅ Input queues initialized with `BLANK_INPUT`

### DETER-6: Integer Arithmetic Consistency

**Requirement:** Integer operations must produce consistent results.

**Implementation:**

- ✅ Rust's defined overflow behavior (panic in debug, wrap in release)
- ✅ No reliance on overflow behavior in library code
- ✅ Frame arithmetic uses checked operations where needed

**User Responsibility:**

```rust,ignore
// Be explicit about overflow handling
let new_value = old_value.wrapping_add(delta); // Explicit wrapping
let new_value = old_value.saturating_add(delta); // Saturating
```

---

## Library Guarantees

Fortress Rollback provides the following determinism guarantees:

### G1: Input Processing Determinism

Given the same sequence of `add_local_input` and `add_remote_input` calls, the library will always produce the same sequence of `FortressRequest`s.

```
∀ input_sequence:
    session.process(input_sequence) → same requests
```

### G2: Rollback Determinism

When rolling back to frame F and resimulating with corrected inputs, the requests generated will be identical to what would have been generated if those inputs were available originally.

```
rollback_to(F) + resimulate(corrected_inputs)
    ≡ original_simulate(corrected_inputs)
```

### G3: State Cell Determinism

`GameStateCell` operations are deterministic:

- `save(F, state, checksum)` stores exactly what is provided
- `load()` returns exactly what was saved

### G4: No Hidden State

The library maintains no hidden state that affects game simulation. All relevant state is:

- Visible in the session struct
- Saved/restored via `SaveGameState`/`LoadGameState`

### G5: Collection Order Guarantee

All iteration over internal collections produces elements in a deterministic order (BTreeMap key order).

---

## User Responsibilities

The library guarantees determinism for its own operations, but users must ensure their game logic is also deterministic.

### R1: Deterministic Game State Updates

```rust
// User's advance function must be deterministic
fn advance_game(state: &mut GameState, inputs: &[(Input, InputStatus)]) {
    // Must produce same result for same inputs
    for (i, (input, status)) in inputs.iter().enumerate() {
        match status {
            InputStatus::Confirmed | InputStatus::Predicted => {
                apply_input(state, i, input);
            }
            InputStatus::Disconnected => {
                // Handle consistently
            }
        }
    }
    state.frame += 1;
}
```

### R2: Complete State Serialization

When saving state, ALL mutable game data must be included:

```rust,ignore
fn save_state(cell: GameStateCell<State>, frame: Frame, state: &GameState) {
    // Must save EVERYTHING that affects future frames
    cell.save(frame, Some(state.clone()), Some(compute_checksum(state)));
}
```

### R3: Deterministic Checksum

Checksums must be computed deterministically:

```rust,ignore
fn compute_checksum(state: &GameState) -> u128 {
    // Use a deterministic serialization
    let bytes = bincode::serialize(state).expect("serialize");
    // Use a deterministic hash
    fletcher16(&bytes) as u128
}
```

### R4: Avoid Platform-Specific Behavior

```rust,ignore
// AVOID: Platform-specific floating point
let result = (x as f64).sin(); // May vary slightly

// PREFER: Fixed-point math or cross-platform libraries
let result = fixed_sin(x);
```

---

## Common Determinism Pitfalls

### Pitfall 1: HashMap Iteration

```rust,ignore
// WRONG - HashMap iteration order is random
for (key, value) in hash_map.iter() {
    process(key, value); // Order affects result
}

// CORRECT - Use BTreeMap or sort keys
for (key, value) in btree_map.iter() {
    process(key, value); // Deterministic order
}
```

### Pitfall 2: Floating Point Inconsistency

```rust,ignore
// PROBLEMATIC - May vary across platforms
let x = 0.1 + 0.2; // Floating point representation

// SAFER - Fixed point or careful rounding
let x = (a * 1000 + b * 1000) / 1000; // Integer math
```

### Pitfall 3: Thread-Local State

```rust,ignore
// WRONG - Thread-local state not synchronized
thread_local! {
    static COUNTER: Cell<u32> = Cell::new(0);
}

// CORRECT - State in game struct
struct GameState {
    counter: u32,
}
```

### Pitfall 4: System Calls

```rust,ignore
// WRONG - Depends on file system state
let config = std::fs::read_to_string("config.txt")?;

// CORRECT - Configuration at startup, not during gameplay
struct GameState {
    config: Config, // Loaded once, part of state
}
```

### Pitfall 5: Pointer Addresses

```rust,ignore
// WRONG - Pointer addresses vary between runs
let id = &object as *const _ as usize;

// CORRECT - Use explicit IDs
struct Object {
    id: u64, // Assigned deterministically
}
```

### Pitfall 6: Allocation Order

```rust,ignore
// PROBLEMATIC - Vec reallocation can affect addresses
let mut objects: Vec<Object> = vec![];
for _ in 0..100 {
    objects.push(Object::new()); // May reallocate
}

// SAFER - Pre-allocate or use indices
let mut objects: Vec<Object> = Vec::with_capacity(100);
```

---

## Verification Strategies

### Strategy 1: SyncTestSession

Use `SyncTestSession` to detect non-determinism locally:

```rust,ignore
let mut session = SessionBuilder::<Config>::new()
    .with_num_players(1)
    .with_check_distance(4) // Compare last 4 frames
    .start_synctest_session()?;

// Run game loop
// Session will rollback and compare checksums
// Mismatch = non-determinism detected!
```

### Strategy 2: Checksum Comparison

Enable desync detection in P2P sessions:

```rust,ignore
let session = SessionBuilder::<Config>::new()
    .with_desync_detection_mode(DesyncDetection::On { interval: 100 })
    // ...
```

Monitor for `FortressEvent::DesyncDetected`:

```rust,ignore
for event in session.events() {
    if let FortressEvent::DesyncDetected { frame, local, remote, .. } = event {
        panic!("Desync at frame {}: local={}, remote={}", frame, local, remote);
    }
}
```

### Strategy 3: Replay Testing

Record inputs and replay on different machines:

```rust,ignore
// Record
let recorded_inputs: Vec<(Frame, Vec<Input>)> = record_session();

// Replay on another machine
let final_state = replay(recorded_inputs);

// Compare final states
assert_eq!(machine_a_state, machine_b_state);
```

### Strategy 4: Property-Based Testing

Use proptest to generate random input sequences:

```rust,ignore
proptest! {
    #[test]
    fn determinism_holds(inputs in any_input_sequence()) {
        let state1 = simulate(inputs.clone());
        let state2 = simulate(inputs.clone());
        assert_eq!(state1, state2);
    }
}
```

---

## Platform Compatibility

### Supported Platforms

| Platform | Determinism Status | Notes |
|----------|-------------------|-------|
| x86_64 Linux | ✅ Verified | Primary development platform |
| x86_64 Windows | ✅ Expected | Same architecture |
| x86_64 macOS | ✅ Expected | Same architecture |
| aarch64 Linux | ⚠️ Test Required | Different float rounding possible |
| aarch64 macOS | ⚠️ Test Required | M1/M2 chips |
| WASM | ⚠️ Test Required | Browser differences possible |

### Cross-Platform Testing

To verify cross-platform determinism:

1. Run the same input sequence on both platforms
2. Compare checksums at regular intervals
3. Any mismatch indicates a determinism bug

```bash
# Machine A (x86_64)
cargo run --example replay -- record session.inputs

# Machine B (aarch64)
cargo run --example replay -- verify session.inputs checksums.txt
```

### Known Platform Differences

| Issue | Affected | Mitigation |
|-------|----------|------------|
| Float representation | All | Use fixed-point math |
| Endianness | Big-endian rare | bincode handles this |
| Integer sizes | Historic | Rust guarantees sizes |
| Alignment | Embedded | Not typically an issue |

---

## Summary

### Library Provides

- ✅ Deterministic collection iteration (BTreeMap)
- ✅ No system time in game logic
- ✅ No RNG in game logic
- ✅ Stable serialization (bincode)
- ✅ No uninitialized memory (safe Rust)

### User Must Ensure

- ✅ Deterministic game state updates
- ✅ Complete state serialization
- ✅ Deterministic checksum computation
- ✅ No platform-specific behavior
- ✅ No HashMap in game logic
- ✅ No unseeded RNG
- ✅ No system time dependencies

### Verification

- Use `SyncTestSession` for local testing
- Enable `DesyncDetection::On` for networked testing
- Implement replay testing for cross-platform verification

---

## Revision History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2025-12-06 | Initial complete specification |
