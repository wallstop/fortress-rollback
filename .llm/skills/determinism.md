<!-- CATEGORY: Determinism & Rollback -->
<!-- WHEN: Ensuring determinism, replacing HashMap, float handling, cross-platform determinism -->

# Determinism in Rust Games

Given identical inputs, the game must produce bit-identical outputs on every run, on every machine.

---

## Sources of Non-Determinism

### Critical (Will Cause Desyncs)

| Source | Problem | Solution |
|--------|---------|----------|
| `HashMap` / `HashSet` | Random iteration order | `BTreeMap` / `BTreeSet` |
| `f32::sin()`, `cos()`, etc. | Platform-specific impls | `libm` or fixed-point |
| `rand::random()` | System entropy | Seeded `Pcg64` |
| `Instant::now()` / `SystemTime` | Wall clock varies | Frame counters only |
| Thread execution order | Scheduler-dependent | Single-threaded simulation |
| Memory addresses | ASLR, allocator | Never use pointers in logic |
| `usize` | 32 vs 64-bit | Use explicit `u32`/`u64` |

### Platform-Dependent

| Source | Problem | Solution |
|--------|---------|----------|
| Float precision | x87 vs SSE vs ARM | Control FPU or use fixed-point |
| Compiler optimizations | May reorder ops | Consistent compiler flags |
| FMA instruction | Different precision | Disable or ensure consistency |
| Denormals | Perf vs precision | Flush to zero |

---

## Collection Determinism

### Replacement Table

| Non-Deterministic | Deterministic Alternative | Notes |
|-------------------|---------------------------|-------|
| `HashMap<K,V>` | `BTreeMap<K,V>` | Keys must impl `Ord` |
| `HashSet<T>` | `BTreeSet<T>` | Sorted iteration |
| `HashMap<K,V>` | `IndexMap<K,V>` | Insertion-order; deterministic only if insertion order is |
| `HashMap` iteration | Sort keys first | When HashMap needed for perf |

### Performance Comparison

| Collection | Lookup | Insert | Deterministic |
|-----------|--------|--------|---------------|
| `HashMap` | O(1) | O(1) | No |
| `BTreeMap` | O(log n) | O(log n) | Yes (sorted) |
| `IndexMap` | O(1) | O(1) | If insertions are |
| `HashMap` + sort | O(1) | O(1) | Yes (at iteration) |

### Dependency Feature Leaks

```bash
# Check what features are enabled (watch for compile-time-rng, const-random)
cargo tree -f "{p} {f}" | grep ahash
cargo tree -f "{p} {f}" | grep rand
```

```toml
# Safe configurations
ahash = { version = "0.8", default-features = false }
rand = { version = "0.8", default-features = false }
rand_pcg = "0.3"
```

---

## Floating-Point Determinism

IEEE-754 guarantees correct rounding only for `+`, `-`, `*`, `/`, `sqrt`. Everything else (sin, cos, exp, log, pow, etc.) is platform-dependent.

### Solutions Table

| Approach | Crate | Determinism | Performance | Use Case |
|----------|-------|-------------|-------------|----------|
| `libm` | `libm` 0.2 | Cross-platform | Moderate | Easiest drop-in |
| Fixed-point | `fixed` 1.29 | Guaranteed | Good | Physics, positions |
| CORDIC trig | `cordic` 0.1 | Guaranteed | Good | Fixed-point trig |
| Soft-float | `softfloat-wrapper` 0.3 | Maximum | Slow | Absolute portability |
| Kahan sum | `accurate` 0.4 | Stable | Good | Large summations |

### libm Usage (Easiest)

```toml
# For Bevy projects
glam = { version = "0.29", features = ["libm"] }
```

### Fixed-Point (Most Reliable)

```rust
use fixed::types::I32F32;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Position { x: I32F32, y: I32F32 }

impl Position {
    fn distance_squared(&self, other: &Self) -> I32F32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx * dx + dy * dy
    }
}
```

### Parallel Reduction (Common Pitfall)

```rust
// Non-deterministic: reduction order varies
let sum: f64 = values.par_iter().sum();

// Deterministic: collect then sum sequentially
let values: Vec<f64> = values.par_iter().map(|x| expensive(x)).collect();
let sum: f64 = values.iter().sum();
```

### Compiler Reordering

```rust
// Force evaluation order with explicit parentheses
let result = ((a + b) + c) + d;
```

---

## WASM for Cross-Platform Determinism

WASM provides stronger guarantees: IEEE 754 specified semantics, canonical NaN, little-endian, fully specified behavior.

```rust
// NaN canonicalization at boundaries
fn canonicalize_float(x: f32) -> f32 {
    if x.is_nan() { f32::NAN } else { x }
}
```

Caveats: threading breaks determinism in WASM; host imports (time, random) are non-deterministic.

---

## Deterministic RNG

```rust
use rand_pcg::Pcg64;
use rand::SeedableRng;

struct GameState {
    rng: Pcg64,  // Must be saved/restored with state
}

impl GameState {
    fn new(seed: u64) -> Self {
        Self { rng: Pcg64::seed_from_u64(seed) }
    }
}
```

| Library | Notes |
|---------|-------|
| `rand_pcg` | Fast, portable, deterministic |
| `rand_chacha` | Cryptographic, very portable |

---

## ECS Determinism (Bevy)

Bevy queries do NOT guarantee iteration order. Always sort by a stable ID:

```rust
#[derive(Component, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Rollback(pub u32);

fn movement_system(mut query: Query<(&Rollback, &mut Position)>) {
    let mut items: Vec<_> = query.iter_mut().collect();
    items.sort_by_key(|(rb, _)| *rb);
    for (_, mut pos) in items { /* deterministic */ }
}
```

---

## Time Handling

```rust
// FORBIDDEN in game logic: Instant::now(), SystemTime, time.delta_seconds()

const TICK_RATE: f32 = 1.0 / 60.0;

#[derive(Resource, Clone)]
pub struct FrameCount(pub u32);
```

---

## Checksum Implementation

```rust
fn compute_checksum(state: &GameState) -> u64 {
    let mut hasher = seahash::SeaHasher::new(); // Portable, deterministic
    state.frame.hash(&mut hasher);
    // Hash floats as bits
    for player in &state.players {
        player.position.x.to_bits().hash(&mut hasher);
        player.position.y.to_bits().hash(&mut hasher);
    }
    hasher.finish()
}
```

Do NOT use `DefaultHasher` -- it is randomized.

---

## Testing for Determinism

1. **Replay test**: Run twice with same seed/inputs, compare checksums
2. **SyncTest**: Force rollback every frame via `start_synctest_session()`
3. **Cross-platform CI**: Compare checksums across ubuntu/windows/macos
4. **Stress test**: 1000+ runs to catch rare non-determinism

---

## Common Bugs

| Bug | Fix |
|-----|-----|
| Float accumulation drift | Reconstruct from integers: `pos = start + vel * (frame * TICK_RATE)` |
| Uninitialized padding in hashing | Zero padding explicitly in `new()` |
| ECS query order | Always sort by `Rollback` ID |
| `const` vs runtime libm | LLVM const-folding may differ from runtime |

---

## Checklist

- [ ] `BTreeMap`/`BTreeSet` instead of `HashMap`/`HashSet`
- [ ] Seeded RNG (`rand_pcg` or `rand_chacha`), saved with state
- [ ] Frame counter instead of wall clock time
- [ ] Fixed timestep for physics
- [ ] `libm` enabled OR fixed-point math
- [ ] No parallel float reductions
- [ ] All ECS queries sorted by stable ID
- [ ] `ahash` using `default-features = false`
- [ ] `Cargo.lock` committed, toolchain pinned
- [ ] Cross-platform CI checksums match
