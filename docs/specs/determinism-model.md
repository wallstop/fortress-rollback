<!-- SYNC: This source doc syncs to wiki/Determinism-Model.md. -->

<p align="center">
  <img src="../assets/logo-small.svg" alt="Fortress Rollback" width="64">
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

- ✅ Library uses its own internal PCG32 RNG (`rng::random()`) only for sync handshake nonces (not game state)
- ✅ No `rand` crate dependency — custom `Pcg32` implementation in `src/rng.rs`
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

- ✅ The bincode-compatible codec is configured with `standard().with_little_endian().with_fixed_int_encoding()` (fixed-size integers are an explicit override of the format's variable-int default, ensuring identical serialized bytes across platforms)
- ✅ `hash::DeterministicHasher` writes scalar integers and collection length prefixes as fixed-width little-endian bytes (`usize`/`isize` canonicalize to 64 bits), so derived `Hash` values match on 32-bit browser WASM and 64-bit little-endian native targets. Std hashes primitive slice *elements* as native memory bytes, so collections of `usize`/`isize` are outside this guarantee; use `compute_checksum` for state checksums
- ✅ Input types require `Serialize + Deserialize`
- ✅ No platform-specific serialization

**Constraints on Input Types:**

```rust
// Must implement these traits
pub trait Config: 'static {
    type Input: Copy + Clone + PartialEq + Eq + Default
              + Serialize + DeserializeOwned;
    // ...
}
```

### DETER-5: No Uninitialized Memory

**Requirement:** All state must be explicitly initialized.

**Implementation:**

- ✅ `#![forbid(unsafe_code)]` prevents uninitialized memory access
- ✅ All arrays initialized with default values
- ✅ Input queues initialized with blank inputs via `PlayerInput::blank_input(Frame::NULL)`

### DETER-6: Integer Arithmetic Consistency

**Requirement:** Integer operations must produce consistent results.

**Implementation:**

- ✅ Rust's defined overflow behavior (panic in debug, wrap in release)
- ✅ No reliance on overflow behavior in library code
- ✅ Frame arithmetic uses checked operations where needed

**User Responsibility:**

```rust
// Be explicit about overflow handling
let old_value: u32 = 100;
let delta: u32 = 7;

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

```rust
fn save_state(cell: GameStateCell<State>, frame: Frame, state: &GameState) {
    // Must save EVERYTHING that affects future frames
    cell.save(frame, Some(state.clone()), Some(compute_checksum(state)));
}
```

### R3: Deterministic Checksum

Checksums must be computed deterministically:

```rust
use fortress_rollback::network::codec::encode;

fn compute_checksum(state: &GameState) -> Option<u128> {
    // Use the library's codec for deterministic serialization
    let bytes = encode(state).ok()?;
    // Use a deterministic hash
    Some(fletcher16(&bytes) as u128)
}
```

> **Note:** The `network::codec` module ensures a consistent bincode-compatible configuration (fixed-size integers) across all serialization calls. The maintained `bincode-next` implementation is exactly pinned, and immutable golden vectors guard the serialized contract.

### R4: Avoid Platform-Specific Behavior

```rust
// AVOID: Platform-specific floating point
let result = (x as f64).sin(); // May vary slightly

// PREFER: Fixed-point math or cross-platform libraries
let result = fixed_sin(x);
```

---

## Common Determinism Pitfalls

### Pitfall 1: HashMap Iteration

```rust
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

```rust
// PROBLEMATIC - May vary across platforms
let x = 0.1 + 0.2; // Floating point representation

// SAFER - Fixed point or careful rounding
let x = (a * 1000 + b * 1000) / 1000; // Integer math
```

### Pitfall 3: Thread-Local State

```rust
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

```rust
// WRONG - Depends on file system state
let config = std::fs::read_to_string("config.txt")?;

// CORRECT - Configuration at startup, not during gameplay
struct GameState {
    config: Config, // Loaded once, part of state
}
```

### Pitfall 5: Pointer Addresses

```rust
// WRONG - Pointer addresses vary between runs
let id = &object as *const _ as usize;

// CORRECT - Use explicit IDs
struct Object {
    id: u64, // Assigned deterministically
}
```

### Pitfall 6: Allocation Order

```rust
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

```rust
let mut session = SessionBuilder::<Config>::new()
    .with_num_players(1)?
    .with_check_distance(4) // Compare last 4 frames
    .start_synctest_session()?;

// Run game loop
// Session will rollback and compare checksums
// Mismatch = non-determinism detected!
```

### Strategy 2: Checksum Comparison

Enable desync detection in P2P sessions:

```rust
let session = SessionBuilder::<Config>::new()
    .with_desync_detection_mode(DesyncDetection::On { interval: 100 })
    // ...
```

Monitor for `FortressEvent::DesyncDetected`:

```rust
for event in session.events() {
    if let FortressEvent::DesyncDetected { frame, local_checksum, remote_checksum, addr } = event {
        eprintln!("Desync at frame {}: local={:#x}, remote={:#x}, addr={}", frame, local_checksum, remote_checksum, addr);
    }
}
```

### Strategy 3: Replay Testing

Record inputs and replay on different machines:

```rust
// Record
let recorded_inputs: Vec<(Frame, Vec<Input>)> = record_session();

// Replay on another machine
let final_state = replay(recorded_inputs);

// Compare final states
assert_eq!(machine_a_state, machine_b_state);
```

### Strategy 4: Property-Based Testing

Use proptest to generate random input sequences:

```rust
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

| Platform       | Determinism Status | Notes                             |
| -------------- | ------------------ | --------------------------------- |
| x86_64 Linux   | ✅ Verified         | Primary development platform      |
| x86_64 Windows | ✅ Verified         | Workspace suite (minus nightly simulation and multi-process tiers) plus the native replay fixture runs on every main/merge-queue CI run |
| x86_64 macOS   | ⚠️ Test Required    | No Intel macOS CI lane: `macos-latest` is Apple silicon (the aarch64 row below covers it) |
| i686 Linux     | ✅ Verified         | Workspace suite (minus nightly simulation, multi-process, and nightly-only tiers) runs natively with a 32-bit `usize` on every main/merge-queue CI run, pinning pointer-width hashing, checksums, and allocation diagnostics; the feature-gated hot-join session suite additionally runs on this lane |
| aarch64 Linux  | ✅ Verified         | Workspace suite (minus proptest-heavy property tests and nightly simulation/multi-process tiers) runs under QEMU on every main/merge-queue CI run; integer-only replay, hash, and checksum parity confirmed; the hot-join session suite also runs on this lane |
| aarch64 macOS  | ✅ Verified         | Workspace suite (minus nightly simulation and multi-process tiers) plus the native replay fixture runs natively on GitHub's arm64 `macos-latest` runner (Apple silicon) on every main/merge-queue CI run |
| WASM (browser) | ✅ Verified         | Captured 4,096-frame replay and machine-width hash/checksum parity run in Chromium on every merge queue; tab-suspension and timeout behavior covered in the browser lane |
| WASM (Emscripten) | ✅ Verified     | The same replay, machine-width, compression, and coordinated-drop wire codec parity checks run in the Godot web-export lane |
| WASM (wasip1)     | ✅ Verified     | A wasmtime-run probe executes the captured replay, machine-width, compression, and wire codec parity checks on every CI run |

### Cross-Platform Testing

To verify cross-platform determinism, record a match once with replay recording enabled, then
replay it with validation on the second target. The replay stores the confirmed inputs for each
frame whose retrieval succeeded; a frame whose retrieval fails is recorded with default placeholder
inputs and increments `metadata.skipped_frames`. It also stores the checksum the recording peer
observed for each frame that had one. A `FortressEvent::ReplayDesync` identifies a divergent frame
only when that frame has a recorded checksum; a frame without a replay checksum or without an
application-provided state checksum is not compared. Check the checksum coverage of both the replay
and the application on each target, and review `metadata.skipped_frames`, before treating playback
as a complete determinism check.
See the [Match Replay System](../replay.md#validation-mode) for the recording and validation loops.

The repository keeps this comparison running in CI rather than relying on manual review:

- `tests/replay-fixture` replays a captured 4,096-frame two-peer schedule in Chromium and in the
  Godot/Emscripten lane, comparing request-stream and state-checksum digests against native
  constants.
- `check_compression_parity` pins the XOR-delta plus RLE bytes that every session input packet
  carries in Chromium and in the Godot/Emscripten lane against a native digest; replay playback
  alone does not exercise that pipeline.
- `smoke_traces_match_checked_in_cross_architecture_digests` folds a whole deterministic-simulation
  trace (confirmed inputs, session states, game-state checksums, event streams, and simulated wire
  statistics) into a digest pinned to the x86_64 reference, and executes natively on i686 and under
  qemu-aarch64 in the `cross-arch-runtime` CI matrix.
- `machine_width_state` pins fixed-width serialization and hashing of `usize`/`isize` fields and
  collection length prefixes on 32-bit and 64-bit targets.
- The checked-in simulation corpus replays (mixed reliable/unreliable transports and the extended
  ring-recycling schedule) execute on native i686 and under qemu-aarch64 in the same
  `cross-arch-runtime` matrix, covering ring-index and frame arithmetic with a non-x86_64 `usize`.

### Known Platform Differences

| Issue                | Affected        | Mitigation             |
| -------------------- | --------------- | ---------------------- |
| Float representation | All             | Use fixed-point math   |
| Endianness           | Big-endian rare | The codec and `hash::DeterministicHasher` encode fixed-width little-endian bytes |
| Pointer width        | 32-bit WASM vs 64-bit native | The codec encodes `usize` as a fixed 64-bit integer, and `hash::DeterministicHasher` canonicalizes scalar `usize`/`isize` values and collection length prefixes to 64 bits |
| Primitive integer collections | All | Std hashes slice elements as native memory bytes; use `compute_checksum`/the codec for cross-target state containing `Vec<usize>`/`Vec<isize>` or other multi-byte integer collections |
| Alignment            | Embedded        | Fields are explicitly serialized; never hash or transmit raw struct memory |

---

## Summary

### Library Provides

- ✅ Deterministic collection iteration (BTreeMap)
- ✅ No system time in game logic
- ✅ No RNG in game logic
- ✅ Stable serialization (bincode)
- ✅ Fixed-width little-endian integer encoding for wire data and scalar/length-prefix hashing
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

| Version | Date       | Changes                        |
| ------- | ---------- | ------------------------------ |
| 1.0     | 2025-12-06 | Initial complete specification |
