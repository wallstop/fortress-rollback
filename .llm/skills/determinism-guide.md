# Determinism in Rust Game Development

> **A guide to achieving and verifying determinism in Rust games, essential for rollback netcode and replay systems.**

## Why Determinism Matters

Determinism means: **given identical inputs, the game produces bit-identical outputs on every run, on every machine.**

This is required for:
- **Rollback netcode** — Peers must reach the same state
- **Replay systems** — Must reproduce exact gameplay
- **Competitive integrity** — Identical game state for all players
- **Debugging** — Reproducible bugs are fixable bugs

---

## Sources of Non-Determinism

### 🔴 Critical: These WILL Cause Desyncs

| Source | Problem | Solution |
|--------|---------|----------|
| `HashMap` / `HashSet` iteration | Random ordering per run | Use `BTreeMap` / `BTreeSet` |
| `f32::sin()`, `cos()`, etc. | Different implementations | Use `libm` or fixed-point |
| `rand::random()` | System entropy | Seeded deterministic RNG |
| `Instant::now()` / `SystemTime` | Wall clock varies | Frame counters only |
| Thread execution order | Scheduler-dependent | Single-threaded simulation |
| Memory addresses | ASLR, allocator behavior | Never use pointers in logic |
| `usize` | 32 vs 64-bit differences | Use explicit `u32`/`u64` |

### 🟡 Moderate: Platform-Dependent

| Source | Problem | Solution |
|--------|---------|----------|
| Floating-point precision | x87 vs SSE vs ARM | Control FPU settings or use fixed-point |
| Compiler optimizations | May reorder operations | Consistent compiler flags |
| FMA (fused multiply-add) | Different intermediate precision | Disable or ensure consistency |
| Denormal handling | Performance vs precision | Flush denormals to zero |

### 🟢 Low Risk: Usually Safe

| Source | Notes |
|--------|-------|
| Integer arithmetic | Deterministic unless overflow |
| Bitwise operations | Fully deterministic |
| Array/Vec indexing | Deterministic if indices are |
| `BTreeMap` / `BTreeSet` | Deterministic iteration by key |
| `IndexMap` / `IndexSet` | Deterministic insertion order |

---

## Floating-Point Determinism

### The Problem

Floating-point math is NOT deterministic across platforms:

```rust
// These may produce DIFFERENT results on different machines:
let a = (0.1_f32 + 0.2_f32) * 0.3_f32;
let b = x.sin();  // Transcendental functions vary by implementation
let c = (a * b) + c;  // FMA may or may not be used
```

### Solution 1: Use `libm` (Easiest)

```toml
# Cargo.toml - for Bevy projects
[dependencies]
glam = { version = "0.29", features = ["libm"] }
```

This:
- Disables SIMD optimizations
- Uses portable software implementations
- Trades performance for determinism

### Solution 2: Fixed-Point Math (Most Reliable)

```toml
# Cargo.toml
[dependencies]
fixed = "1.29"
cordic = "0.1"  # For sin/cos/sqrt/atan2
```

```rust
use fixed::types::I32F32;

/// Fixed-point position - guaranteed deterministic
#[derive(Clone, Copy, PartialEq, Eq)]
struct Position {
    x: I32F32,
    y: I32F32,
}

impl Position {
    fn new(x: f32, y: f32) -> Self {
        Self {
            x: I32F32::from_num(x),
            y: I32F32::from_num(y),
        }
    }
    
    fn distance_squared(&self, other: &Self) -> I32F32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx * dx + dy * dy
    }
}

/// Deterministic trigonometry via CORDIC
fn rotate_point(x: I32F32, y: I32F32, angle: I32F32) -> (I32F32, I32F32) {
    let (sin, cos) = cordic::sin_cos(angle);
    let new_x = x * cos - y * sin;
    let new_y = x * sin + y * cos;
    (new_x, new_y)
}
```

### Solution 3: Integer-Only Physics

For maximum determinism, use integers directly:

```rust
/// Position in 1/256ths of a pixel
#[derive(Clone, Copy, PartialEq, Eq)]
struct SubpixelPosition {
    x: i32,  // x * 256
    y: i32,  // y * 256
}

impl SubpixelPosition {
    const SCALE: i32 = 256;
    
    fn from_pixels(x: f32, y: f32) -> Self {
        Self {
            x: (x * Self::SCALE as f32) as i32,
            y: (y * Self::SCALE as f32) as i32,
        }
    }
    
    fn to_pixels(&self) -> (f32, f32) {
        (
            self.x as f32 / Self::SCALE as f32,
            self.y as f32 / Self::SCALE as f32,
        )
    }
    
    fn add_velocity(&mut self, vx: i32, vy: i32) {
        self.x = self.x.wrapping_add(vx);
        self.y = self.y.wrapping_add(vy);
    }
}
```

---

## Deterministic Random Numbers

### Required: Seeded RNG

```rust
use rand_pcg::Pcg64;
use rand::SeedableRng;
use rand::Rng;

/// Game state includes RNG
struct GameState {
    rng: Pcg64,
    // ... other state
}

impl GameState {
    fn new(seed: u64) -> Self {
        Self {
            // All peers use same seed
            rng: Pcg64::seed_from_u64(seed),
        }
    }
    
    fn random_range(&mut self, min: i32, max: i32) -> i32 {
        self.rng.gen_range(min..max)
    }
}
```

### Important: RNG State Must Be Saved

```rust
impl GameState {
    fn save(&self) -> SavedState {
        SavedState {
            rng: self.rng.clone(),  // ← Don't forget!
            // ... other state
        }
    }
    
    fn load(&mut self, saved: &SavedState) {
        self.rng = saved.rng.clone();  // ← Restore RNG too!
        // ... other state
    }
}
```

### Recommended RNG Libraries

| Library | Type | Notes |
|---------|------|-------|
| `rand_pcg` | Pcg64, Pcg32 | Fast, portable, deterministic |
| `rand_chacha` | ChaCha12Rng | Cryptographic, very portable |
| `fastrand` | WyRand | Fastest, but check portability |

---

## Deterministic Collections

### HashMap → BTreeMap

```rust
use std::collections::{HashMap, BTreeMap};

// ❌ WRONG: Non-deterministic iteration
let mut scores: HashMap<PlayerId, i32> = HashMap::new();
for (id, score) in &scores {
    // Order varies between runs!
}

// ✅ CORRECT: Deterministic iteration by key
let mut scores: BTreeMap<PlayerId, i32> = BTreeMap::new();
for (id, score) in &scores {
    // Always iterates in sorted order
}
```

### HashSet → BTreeSet

```rust
use std::collections::{HashSet, BTreeSet};

// ❌ WRONG
let mut active: HashSet<EntityId> = HashSet::new();

// ✅ CORRECT
let mut active: BTreeSet<EntityId> = BTreeSet::new();
```

### IndexMap for Insertion Order

```rust
use indexmap::IndexMap;

// Maintains insertion order (deterministic if insertion order is)
let mut items: IndexMap<ItemId, Item> = IndexMap::new();
items.insert(ItemId(1), item1);
items.insert(ItemId(2), item2);

for (id, item) in &items {
    // Iterates in insertion order: 1, 2
}
```

### Sorting Before Iteration

When you must use HashMap for performance:

```rust
let map: HashMap<EntityId, Entity> = /* ... */;

// Sort keys before iterating
let mut keys: Vec<_> = map.keys().collect();
keys.sort();

for key in keys {
    let entity = &map[key];
    // Now deterministic
}
```

---

## WebAssembly for Cross-Platform Determinism

### Why WASM Helps

WebAssembly provides **stronger determinism guarantees** than native code:

| Challenge | Native Code Problem | WASM Solution |
|-----------|---------------------|---------------|
| Float inconsistency | x87 80-bit vs SSE 64-bit | IEEE 754 specified semantics |
| Compiler differences | GCC vs Clang optimizations | Single IR, consistent codegen |
| Platform ABI | Calling conventions vary | Canonical ABI |
| Endianness | Big vs little endian | Little endian specified |
| Undefined behavior | C/C++ UB varies | Fully specified semantics |

### WASM Float Determinism

WASM specifies IEEE 754-2019 compliance with canonical NaN handling:

```rust
// In native code: may differ across platforms
let result = (0.1_f32 + 0.2_f32) * 0.3_f32;

// In WASM: guaranteed identical on all conforming runtimes
// Same binary → same results everywhere
```

### NaN Canonicalization

WASM uses canonical NaN values, but be careful at boundaries:

```rust
// When serializing float state across platforms:
fn canonicalize_float(x: f32) -> f32 {
    if x.is_nan() {
        f32::NAN  // Use canonical NaN
    } else {
        x
    }
}

// Or use integer representation for state transfer
fn to_bits_safe(x: f32) -> u32 {
    if x.is_nan() {
        0x7FC00000  // Canonical quiet NaN
    } else {
        x.to_bits()
    }
}
```

### Architecture: WASM for Simulation

Consider compiling game logic to WASM for guaranteed determinism:

```rust
// Core game logic - compile to both native and WASM
// WASM version guarantees cross-platform determinism
mod game_logic {
    pub fn advance_frame(state: &mut GameState, inputs: &[Input]) {
        // All computation is deterministic in WASM
        for input in inputs {
            state.apply_input(input);
        }
        state.physics_tick();  // Float math works!
    }
}

// Platform-specific code - stays native for performance
mod platform {
    pub fn render(state: &GameState) { /* GPU calls */ }
    pub fn play_audio(events: &[AudioEvent]) { /* Audio API */ }
}
```

### WASM Determinism Caveats

⚠️ **Threading breaks determinism** — Avoid WASM threads in game logic:

```rust
// ❌ NON-DETERMINISTIC: Thread scheduling varies
use rayon::prelude::*;
let sum: f64 = values.par_iter().sum();

// ✅ DETERMINISTIC: Sequential in WASM
let sum: f64 = values.iter().sum();
```

⚠️ **Host imports may not be deterministic**:

```rust
// ❌ Host-provided time is non-deterministic
let now = js_sys::Date::now();

// ✅ Use frame counters for game logic
let game_time = frame_number * TICK_DURATION;
```

### See Also

For complete WASM development guidance:
- [wasm-rust-guide.md](wasm-rust-guide.md) — Rust to WASM compilation
- [wasm-threading.md](wasm-threading.md) — Threading and concurrency in WASM
- [wasm-portability.md](wasm-portability.md) — WASM determinism and sandboxing
- [cross-platform-rust.md](cross-platform-rust.md) — Multi-platform architecture
- [no-std-guide.md](no-std-guide.md) — `no_std` patterns for WASM

---

## ECS Determinism (Bevy)

### Query Iteration is Non-Deterministic

```rust
// ❌ WRONG: Order not guaranteed
fn update_system(query: Query<&mut Position>) {
    for mut pos in &mut query {
        // Order may differ between frames, runs, and peers!
    }
}
```

### Solution: Rollback Marker Component

```rust
/// Stable ID for deterministic ordering
#[derive(Component, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Rollback(pub u32);

/// Always sort by Rollback ID
fn update_system(mut query: Query<(&Rollback, &mut Position)>) {
    let mut items: Vec<_> = query.iter_mut().collect();
    items.sort_by_key(|(rb, _)| *rb);
    
    for (_, mut pos) in items {
        // Now deterministic!
    }
}
```

### Helper Trait for Deterministic Queries

```rust
/// Extension trait for deterministic query iteration
pub trait DeterministicIter<'w, 's> {
    type Item;
    fn iter_sorted(&'w mut self) -> Vec<Self::Item>;
}

impl<'w, 's, T: Component> DeterministicIter<'w, 's> 
    for Query<'w, 's, (&Rollback, &mut T)> 
{
    type Item = (Mut<'w, T>,);
    
    fn iter_sorted(&'w mut self) -> Vec<Self::Item> {
        let mut items: Vec<_> = self.iter_mut().collect();
        items.sort_by_key(|(rb, _)| rb.0);
        items.into_iter().map(|(_, t)| (t,)).collect()
    }
}
```

---

## Time Handling

### Never Use Wall Clock Time

```rust
// ❌ FORBIDDEN in game logic
let now = std::time::Instant::now();
let elapsed = start.elapsed();
let time = std::time::SystemTime::now();

// ❌ FORBIDDEN in Bevy
fn bad_system(time: Res<Time>) {
    let delta = time.delta_seconds();  // Varies per machine!
}
```

### Use Frame Counters

```rust
/// Deterministic frame counter
#[derive(Resource, Clone)]
pub struct FrameCount(pub u32);

fn good_system(mut frame: ResMut<FrameCount>) {
    frame.0 += 1;
    
    // Time in seconds (deterministic)
    let time_seconds = frame.0 as f32 / 60.0;
}

/// Fixed timestep
const TICK_RATE: f32 = 1.0 / 60.0;

fn physics_update(mut pos: Mut<Position>, vel: &Velocity) {
    // Always same delta
    pos.x += vel.x * TICK_RATE;
    pos.y += vel.y * TICK_RATE;
}
```

---

## Testing for Determinism

### Test 1: Replay Verification

```rust
#[test]
fn test_determinism_via_replay() {
    let seed = 12345u64;
    let inputs = generate_test_inputs();
    
    // Run twice with same seed and inputs
    let result1 = run_game(seed, &inputs);
    let result2 = run_game(seed, &inputs);
    
    // Must be identical
    assert_eq!(result1.final_state, result2.final_state);
    assert_eq!(result1.checksum, result2.checksum);
}
```

### Test 2: Multi-Instance Comparison

```rust
#[test]
fn test_determinism_multi_instance() {
    let seed = 12345u64;
    let inputs = generate_test_inputs();
    
    // Run many times
    let results: Vec<_> = (0..10)
        .map(|_| run_game(seed, &inputs))
        .collect();
    
    // All must match first
    let first = &results[0];
    for (i, result) in results.iter().enumerate().skip(1) {
        assert_eq!(
            first.checksum, result.checksum,
            "Run {} diverged from run 0", i
        );
    }
}
```

### Test 3: Forced Rollback (SyncTest)

```rust
/// Session that forces rollback every frame to catch determinism bugs
struct SyncTestSession {
    game: Game,
    shadow_game: Game,  // Second copy running in parallel
}

impl SyncTestSession {
    fn advance_frame(&mut self, input: Input) {
        // Advance both copies
        self.game.advance(input);
        self.shadow_game.advance(input);
        
        // Compare checksums
        let checksum1 = self.game.checksum();
        let checksum2 = self.shadow_game.checksum();
        
        assert_eq!(
            checksum1, checksum2,
            "DESYNC at frame {}: {} != {}",
            self.game.frame, checksum1, checksum2
        );
    }
}
```

### Test 4: Cross-Platform CI

```yaml
# .github/workflows/determinism.yml
name: Determinism Tests

on: [push, pull_request]

jobs:
  test:
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    runs-on: ${{ matrix.os }}
    
    steps:
      - uses: actions/checkout@v4
      - name: Run determinism tests
        run: cargo test determinism -- --nocapture
      
      - name: Save checksum
        run: cargo run --example generate_checksum > checksum-${{ matrix.os }}.txt
      
      - uses: actions/upload-artifact@v4
        with:
          name: checksums
          path: checksum-*.txt
  
  verify:
    needs: test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/download-artifact@v4
      - name: Compare checksums
        run: |
          # All checksums must match
          sort -u checksums/*.txt | wc -l | grep -q '^1$'
```

---

## Checksum Implementation

### Simple Checksum

```rust
use std::hash::{Hash, Hasher};

/// Portable hasher (don't use DefaultHasher - it's not portable!)
fn portable_hasher() -> impl Hasher {
    seahash::SeaHasher::new()
}

fn compute_checksum(state: &GameState) -> u64 {
    let mut hasher = portable_hasher();
    
    // Hash all game state in deterministic order
    state.frame.hash(&mut hasher);
    
    // Players in index order (deterministic)
    for player in &state.players {
        hash_player(player, &mut hasher);
    }
    
    // Entities sorted by ID
    let mut entities: Vec<_> = state.entities.iter().collect();
    entities.sort_by_key(|e| e.id);
    for entity in entities {
        hash_entity(entity, &mut hasher);
    }
    
    hasher.finish()
}

fn hash_player(player: &Player, hasher: &mut impl Hasher) {
    player.id.hash(hasher);
    // Hash floats as bits for consistency
    player.position.x.to_bits().hash(hasher);
    player.position.y.to_bits().hash(hasher);
    player.health.hash(hasher);
}
```

### Per-Component Checksum (Bevy)

```rust
/// Checksum for individual component types
fn component_checksum<C: Component + Hash>(
    query: Query<(&Rollback, &C)>
) -> u64 {
    let mut hasher = portable_hasher();
    
    // Sort by Rollback ID
    let mut items: Vec<_> = query.iter().collect();
    items.sort_by_key(|(rb, _)| rb.0);
    
    for (rb, component) in items {
        rb.0.hash(&mut hasher);
        component.hash(&mut hasher);
    }
    
    hasher.finish()
}
```

---

## Reproducible Builds

### The Problem

Even deterministic code can produce different binaries on different machines or builds:

```bash
# Build twice in same directory
cargo build --release
sha256sum target/release/my_app > checksum1.txt
cargo clean
cargo build --release
sha256sum target/release/my_app > checksum2.txt

diff checksum1.txt checksum2.txt
# Files differ!
```

### Solution: Windows MSVC Reproducibility

```bash
# Windows: Use /Brepro linker flag
set RUSTFLAGS=-Clink-arg=/Brepro
cargo build --release
```

### Solution: Lock Toolchain and Dependencies

```toml
# rust-toolchain.toml
[toolchain]
channel = "1.75.0"
components = ["rustfmt", "clippy"]
```

```toml
# Cargo.toml
# Always commit Cargo.lock for reproducibility
```

### Detecting Build Non-Reproducibility

```bash
# Build twice and compare
cargo clean && cargo build --release
mv target/release/myapp myapp_1
cargo clean && cargo build --release
diff <(sha256sum myapp_1) <(sha256sum target/release/myapp)
```

---

## WASM and No-Std Determinism

### WASM-Specific Concerns

WASM environments have unique determinism challenges:

```rust
// ❌ BUG: HashMap with system entropy doesn't exist in wasm32-unknown-unknown
use std::collections::HashMap;  // Uses SipHash with RandomState

// ✅ CORRECT: Use hashbrown with explicit hasher
use hashbrown::HashMap;
use ahash::RandomState;

// Or even better: deterministic hasher
use fnv::FnvHasher;
type DetMap<K, V> = HashMap<K, V, std::hash::BuildHasherDefault<FnvHasher>>;
```

### Feature Leaking in Dependencies

**Critical Issue:** Cargo features are additive. If ANY dependency enables a non-deterministic feature, it affects your entire build.

```toml
# ❌ Dangerous: Pin enables default features which includes const-random
[dependencies]
memory-db = "0.29"
ahash = "0.8"  # default-features includes compile-time-rng!

# ✅ Safe: Explicitly disable problematic features
[dependencies]
memory-db = { version = "0.29", default-features = false }
ahash = { version = "0.8", default-features = false }
```

### Detecting Feature Leaks

```bash
# Check what features are enabled
cargo tree -f "{p} {f}" | grep ahash
cargo tree -f "{p} {f}" | grep rand

# Look for compile-time-rng, const-random, std features
```

### Deterministic WASM Builds

```bash
# Build WASM twice and compare
cargo build --target wasm32-unknown-unknown --release
sha256sum target/wasm32-unknown-unknown/release/*.wasm > checksum.sha256
cargo clean
cargo build --target wasm32-unknown-unknown --release
sha256sum -c checksum.sha256
```

---

## Detecting Non-Determinism

### Using strace to Find Entropy Sources

```bash
# Linux: Detect getrandom syscalls during compilation
strace -f -e trace=getrandom cargo build 2>&1 | grep getrandom

# If you see calls to getrandom, something is using system entropy
```

### CI Determinism Check

```yaml
# .github/workflows/determinism-check.yml
name: Check Determinism

on: [push, pull_request]

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Build first time
        run: cargo build --release
        
      - name: Save checksum
        run: sha256sum target/release/myapp > first.sha256
        
      - name: Clean and rebuild
        run: |
          cargo clean
          cargo build --release
          
      - name: Compare checksums
        run: sha256sum -c first.sha256
```

### Runtime Determinism Testing

```rust
/// Test that same inputs produce same outputs
#[test]
fn test_runtime_determinism() {
    let inputs = vec![/* test inputs */];
    
    let result1 = run_simulation(42, &inputs);  // Seed 42
    let result2 = run_simulation(42, &inputs);  // Same seed
    
    assert_eq!(result1, result2, "Non-determinism detected!");
}

/// Run many times to find rare non-determinism
#[test]
fn stress_test_determinism() {
    let inputs = vec![/* test inputs */];
    
    let baseline = run_simulation(42, &inputs);
    
    for _ in 0..1000 {
        let result = run_simulation(42, &inputs);
        assert_eq!(baseline, result, "Rare non-determinism found!");
    }
}
```

---

## Proc-Macro Determinism

### The Risk

Proc macros execute arbitrary code at compile time and can:
- Access filesystem
- Make network requests  
- Use system RNG
- Behave differently based on environment

### Identifying Problematic Macros

```bash
# List all proc-macro dependencies
cargo tree --target x86_64-unknown-linux-gnu -e proc-macro

# Check for crates known to cause issues
cargo tree | grep -E "const-random|uuid|chrono"
```

### Safe Proc-Macro Usage

```rust
// ❌ DANGEROUS: UUID generated at compile time varies
const REQUEST_ID: Uuid = uuid::uuid!("67e55044-10b1-426f-9247-bb680e5fe0c8");

// ❌ DANGEROUS: Compile-time timestamp
const BUILD_TIME: &str = build_time::build_time_utc!();

// ✅ SAFE: Runtime generation with seeded RNG
fn new_request_id(rng: &mut impl Rng) -> [u8; 16] {
    rng.gen()
}
```

---

## Advanced Float Determinism

### Why `libm` Isn't Always Enough

```rust
// ❌ LLVM const-folding may use different algorithm than runtime!
// This can produce different results at compile vs runtime:
const X: f64 = libm::sin(1.0);  // Computed by LLVM
let y: f64 = libm::sin(1.0);    // Computed by libm at runtime
// X may not equal y!
```

### Parallel Reduction Non-Determinism

```rust
use rayon::prelude::*;

// ❌ NON-DETERMINISTIC: Reduction order varies
let sum: f64 = values.par_iter().sum();

// ❌ STILL NON-DETERMINISTIC: fold order varies
let sum: f64 = values.par_iter().fold(|| 0.0, |a, b| a + b).sum();

// ✅ DETERMINISTIC: Collect and sum sequentially
let values: Vec<f64> = values.par_iter().map(|x| expensive_compute(x)).collect();
let sum: f64 = values.iter().sum();  // Sequential sum

// ✅ DETERMINISTIC: Use Kahan summation for stability
use accurate::sum::Sum2;
let sum = values.iter().copied().collect::<Sum2<_>>().sum();
```

### Compensated Summation

```rust
/// Kahan summation for stable floating-point sums
fn kahan_sum(values: &[f64]) -> f64 {
    let mut sum = 0.0;
    let mut compensation = 0.0;
    
    for &value in values {
        let y = value - compensation;
        let t = sum + y;
        compensation = (t - sum) - y;
        sum = t;
    }
    
    sum
}
```

### Denormal Handling

```rust
// Denormals can cause performance and determinism issues

// Option 1: Flush to zero at program start (platform-specific)
#[cfg(target_arch = "x86_64")]
fn set_flush_denormals() {
    use std::arch::x86_64::*;
    unsafe {
        _MM_SET_FLUSH_ZERO_MODE(_MM_FLUSH_ZERO_ON);
        _MM_SET_DENORMALS_ZERO_MODE(_MM_DENORMALS_ZERO_ON);
    }
}

// Option 2: Manually clamp small values
fn clamp_denormal(x: f64) -> f64 {
    if x.abs() < f64::MIN_POSITIVE {
        0.0
    } else {
        x
    }
}
```

---

## Common Bugs and Fixes

### Bug: Accumulating Float Errors

```rust
// ❌ BUG: Small errors compound over time
position += velocity * delta;
position += velocity * delta;
// After many frames: significant drift between peers

// ✅ FIX: Reconstruct from integers
let frame_offset = current_frame - spawn_frame;
position = start_position + velocity * (frame_offset as f32 * TICK_RATE);
```

### Bug: Uninitialized Memory in Hashing

```rust
// ❌ BUG: Padding bytes may contain garbage
#[repr(C)]
struct Input {
    buttons: u8,
    _padding: [u8; 3],  // Uninitialized!
    stick_x: i32,
}

// ✅ FIX: Zero padding explicitly
#[repr(C)]
#[derive(Default)]
struct Input {
    buttons: u8,
    _padding: [u8; 3],
    stick_x: i32,
}

impl Input {
    fn new(buttons: u8, stick_x: i32) -> Self {
        Self {
            buttons,
            _padding: [0; 3],  // Explicit zeros
            stick_x,
        }
    }
}
```

### Bug: Different Iteration Order

```rust
// ❌ BUG: Query order not guaranteed
fn collision_system(mut query: Query<&mut Position>) {
    let positions: Vec<_> = query.iter().collect();
    // positions order may differ!
}

// ✅ FIX: Always sort
fn collision_system(mut query: Query<(&Rollback, &mut Position)>) {
    let mut positions: Vec<_> = query.iter_mut().collect();
    positions.sort_by_key(|(rb, _)| rb.0);
}
```

---

## Determinism Crate Recommendations

### Essential Crates

| Purpose | Crate | Notes |
|---------|-------|-------|
| **Math functions** | `libm` 0.2 | Portable transcendentals (sin, cos, ln, etc.) |
| **Deterministic RNG** | `rand_pcg` 0.3 | Fast, portable PCG family |
| **Alternative RNG** | `rand_chacha` 0.3 | Cryptographic strength |
| **Fixed-point math** | `fixed` 1.29 | Guaranteed bit-identical |
| **Fixed trig** | `cordic` 0.1 | sin/cos/sqrt/atan2 for fixed-point |
| **Ordered map** | `indexmap` 2.0 | Insertion-order iteration |
| **Stable hashing** | `seahash` 4.1 | Portable, deterministic hasher |
| **Alternative hash** | `fnv` 1.0 | Simple, fast, deterministic |
| **Accurate sums** | `accurate` 0.4 | Compensated summation |

### Crates to Avoid or Configure

| Crate | Problem | Solution |
|-------|---------|----------|
| `ahash` | Compile-time RNG by default | Use `default-features = false` |
| `uuid` | System entropy | Use deterministic version |
| `chrono` | System time | Mock or avoid in game logic |
| `rayon` | Non-deterministic work-stealing | Careful with float operations |
| `hashbrown` | RandomState by default | Use `with_hasher()` |

### Feature Flags to Watch

```toml
# ❌ These features often break determinism
ahash = { version = "0.8", features = ["compile-time-rng"] }  # BAD
rand = { version = "0.8", features = ["std_rng"] }  # Uses system entropy

# ✅ Safe configurations
ahash = { version = "0.8", default-features = false }
rand = { version = "0.8", default-features = false }
rand_pcg = "0.3"  # Seeded only
```

---

## Quick Reference: Determinism Checklist

### Setup
- [ ] Using `BTreeMap`/`BTreeSet` instead of `HashMap`/`HashSet`
- [ ] Using seeded RNG (`rand_pcg` or `rand_chacha`)
- [ ] Frame counter instead of wall clock time
- [ ] Fixed timestep for physics
- [ ] Checked `Cargo.lock` is committed
- [ ] Toolchain version pinned in `rust-toolchain.toml`

### Floating Point
- [ ] `libm` feature enabled OR using fixed-point math
- [ ] No transcendental functions without library
- [ ] Float comparisons handle NaN/epsilon properly
- [ ] Parallel reductions collect-then-sum, not parallel-sum
- [ ] Kahan/compensated summation for large sums

### Collections & Dependencies
- [ ] No HashMap iteration without sorting
- [ ] `ahash` using `default-features = false`
- [ ] Run `cargo tree -f "{p} {f}"` to audit features
- [ ] Check for `const-random`, `compile-time-rng` features

### ECS (Bevy)
- [ ] All queries sorted by stable Rollback ID
- [ ] RNG state included in snapshots
- [ ] All game-affecting state registered for rollback

### WASM/Cross-Platform
- [ ] WASM builds reproducible (SHA comparison)
- [ ] CI tests on multiple platforms
- [ ] No proc-macro compile-time randomness
- [ ] Consider WASM for cross-platform float determinism

### Testing
- [ ] Replay test passes (same inputs = same output)
- [ ] SyncTest mode works (forced rollback every frame)
- [ ] Cross-platform CI compares checksums
- [ ] Stress test with 1000+ runs catches rare non-determinism

---

*Determinism is the foundation of rollback netcode. Get this right first, then build networking on top.*

