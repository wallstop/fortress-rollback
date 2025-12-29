# Collection and Data Structure Determinism in Rust

This guide covers achieving deterministic behavior with Rust collections, essential for rollback netcode, replay systems, and any application requiring reproducible execution across machines and runs.

## Table of Contents

- [Why Collection Determinism Matters](#why-collection-determinism-matters)
- [HashMap Non-Determinism](#hashmap-non-determinism)
- [Deterministic Alternatives](#deterministic-alternatives)
- [Detecting Non-Determinism](#detecting-non-determinism)
- [Practical Patterns](#practical-patterns)
- [WASM and no_std Concerns](#wasm-and-no_std-concerns)
- [Migration Patterns](#migration-patterns)
- [Performance Tradeoffs](#performance-tradeoffs)

---

## Why Collection Determinism Matters

Collection iteration order can cause **silent desyncs** in networked applications. If two peers iterate over the same data but in different orders, their state diverges even though they processed the same inputs.

```rust
// ⚠️ DANGEROUS: Two peers may see different iteration orders
let map: HashMap<PlayerId, Player> = /* ... */;
for (id, player) in &map {
    update_player(player);  // Order affects game state!
}
```

This is particularly insidious because:

- The bug is **non-deterministic** — it may work in testing but fail in production
- The bug is **platform-dependent** — it may work on your machine but fail on others
- The bug is **run-dependent** — it may work 99 times and fail on the 100th

---

## HashMap Non-Determinism

### Why std::collections::HashMap Has Randomized Iteration Order

Rust's `HashMap` uses `RandomState` as its default hasher, which:

1. **Obtains entropy at runtime** via the `getrandom` syscall
2. **Creates a unique hash seed** for each `RandomState` instance
3. **Produces different hash values** for the same keys across runs

```rust
use std::collections::HashMap;

// Each time you run this program, iteration order may differ!
let mut map = HashMap::new();
map.insert("a", 1);
map.insert("b", 2);
map.insert("c", 3);

for (k, v) in &map {
    println!("{k}: {v}");  // Order is unpredictable!
}
```

### The RandomState Hasher and Security Implications

`RandomState` exists to prevent **HashDoS attacks**, where an attacker crafts inputs that all hash to the same bucket, degrading O(1) lookups to O(n).

```rust
use std::collections::hash_map::RandomState;

// RandomState generates random keys at construction
let s1 = RandomState::new();
let s2 = RandomState::new();

// These will produce DIFFERENT hashers
let h1 = s1.build_hasher();
let h2 = s2.build_hasher();
```

**Security trade-off:** Randomized hashing prevents DoS attacks but breaks determinism. For game state that isn't exposed to adversarial input, deterministic hashing is often acceptable.

### ahash and const-random: Compile-Time Randomness

The `ahash` crate (used by many popular crates including `hashbrown`) can introduce non-determinism in two ways:

#### 1. Runtime Randomness (Default)

```rust
use ahash::RandomState;

// Default: obtains entropy from OS at runtime
let state = RandomState::default();
```

#### 2. Compile-Time Randomness (`compile-time-rng` feature)

```toml
# Cargo.toml - This makes your binary non-reproducible!
[dependencies]
ahash = { version = "0.8", features = ["compile-time-rng"] }
```

With `compile-time-rng`, ahash generates random constants **at compile time**, meaning:

- Different builds produce different binaries
- Even `cargo clean && cargo build` creates a different binary
- CI builds may differ from local builds

**Warning:** The `const-random` crate used by ahash reads from `/dev/urandom` at compile time!

### How HashMap Randomization Leaks Through Dependencies

Non-determinism can **propagate silently** through your dependency tree:

```bash
# Check if any dependency uses ahash or getrandom
cargo tree | grep -E "ahash|getrandom|const-random"

# Example output showing potential issues:
# ├── bevy_utils v0.15.0
# │   └── ahash v0.8.11
# │       ├── getrandom v0.2.15
# │       └── const-random v0.1.18
```

Common culprits:

- `hashbrown` (powers `std::collections::HashMap`)
- `bevy_utils` (Bevy's default hasher)
- `indexmap` with default features
- `serde_json` (for deserializing into HashMap)

---

## Deterministic Alternatives

### BTreeMap/BTreeSet: Ordered Iteration

`BTreeMap` iterates in **key-sorted order**, making it fully deterministic.

```rust
use std::collections::BTreeMap;

let mut map = BTreeMap::new();
map.insert(3, "three");
map.insert(1, "one");
map.insert(2, "two");

// Always iterates in sorted order: 1, 2, 3
for (k, v) in &map {
    println!("{k}: {v}");
}
```

**Requirements:**

- Keys must implement `Ord` (not just `Hash + Eq`)
- Iteration is O(n) regardless of access pattern

```rust
// ✅ Works: PlayerId implements Ord
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct PlayerId(u32);

let players: BTreeMap<PlayerId, Player> = BTreeMap::new();
```

### IndexMap/IndexSet: Insertion-Order with O(1) Lookups

`IndexMap` maintains **insertion order** while providing hash-map performance.

```toml
# Cargo.toml
[dependencies]
indexmap = "2"
```

```rust
use indexmap::IndexMap;

let mut map = IndexMap::new();
map.insert("first", 1);
map.insert("second", 2);
map.insert("third", 3);

// Always iterates in insertion order: first, second, third
for (k, v) in &map {
    println!("{k}: {v}");
}

// O(1) lookups still work
assert_eq!(map.get("second"), Some(&2));
```

**Determinism caveat:** IndexMap is deterministic **only if insertion order is deterministic**. If you insert based on HashMap iteration, you inherit that non-determinism.

```rust
// ⚠️ WRONG: Insertion order depends on HashMap iteration
let hash_map: HashMap<K, V> = /* ... */;
let index_map: IndexMap<K, V> = hash_map.into_iter().collect();

// ✅ CORRECT: Sort before converting
let mut pairs: Vec<_> = hash_map.into_iter().collect();
pairs.sort_by_key(|(k, _)| k.clone());
let index_map: IndexMap<K, V> = pairs.into_iter().collect();
```

### Using HashMap::with_hasher() with Deterministic Hashers

You can use `HashMap` with a deterministic hasher for O(1) lookups without iteration-order guarantees:

```rust
use std::collections::HashMap;
use std::hash::BuildHasherDefault;

// FNV-1a: Simple, fast, deterministic
use fnv::FnvHasher;
type FnvHashMap<K, V> = HashMap<K, V, BuildHasherDefault<FnvHasher>>;

let mut map: FnvHashMap<i32, &str> = FnvHashMap::default();
map.insert(1, "one");

// Or use ahash with fixed seed
use ahash::RandomState;

let fixed_state = RandomState::with_seed(42);
let mut map = HashMap::with_hasher(fixed_state);
map.insert(1, "one");
```

**Important:** This makes hashing deterministic but **iteration order is still not guaranteed**! Use this only when you need deterministic hashes (e.g., for checksums) but don't iterate.

### Custom Deterministic Hasher Example

```rust
use std::hash::{Hash, Hasher};

/// FNV-1a hasher - deterministic across all platforms
pub struct DeterministicHasher {
    state: u64,
}

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0100_0000_01b3;

impl DeterministicHasher {
    pub const fn new() -> Self {
        Self { state: FNV_OFFSET_BASIS }
    }
}

impl Default for DeterministicHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Hasher for DeterministicHasher {
    fn finish(&self) -> u64 {
        self.state
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.state ^= u64::from(byte);
            self.state = self.state.wrapping_mul(FNV_PRIME);
        }
    }
}

// BuildHasher implementation for use with HashMap
#[derive(Clone, Copy, Default)]
pub struct DeterministicBuildHasher;

impl std::hash::BuildHasher for DeterministicBuildHasher {
    type Hasher = DeterministicHasher;

    fn build_hasher(&self) -> Self::Hasher {
        DeterministicHasher::new()
    }
}

// Usage
type DetHashMap<K, V> = std::collections::HashMap<K, V, DeterministicBuildHasher>;
```

### stable-hash Crate for Cross-Platform Stable Hashing

For hashes that must be stable across platforms, Rust versions, and time:

```toml
# Cargo.toml
[dependencies]
stable-hash = "0.4"
```

```rust
use stable_hash::StableHasher;
use std::hash::Hash;

fn stable_hash<T: Hash>(value: &T) -> u64 {
    let mut hasher = StableHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}
```

---

## Detecting Non-Determinism

### Using strace to Detect getrandom Syscalls

On Linux, use `strace` to detect if your program calls `getrandom`:

```bash
# During compilation (detects compile-time randomness)
strace -e getrandom cargo build 2>&1 | grep getrandom

# During test execution
strace -e getrandom cargo test 2>&1 | grep getrandom

# Example output showing randomness being used:
# getrandom("\x0b\xf9\x61\xc3\x41\x34\x99\x52", 8, GRND_NONBLOCK) = 8
```

If you see `getrandom` calls during compilation, you have compile-time randomness. If you see them during runtime, you have runtime randomness.

### Auditing Dependencies for HashMap Usage

```bash
# Find all HashMap usages in your crate
rg "HashMap" --type rust

# Find HashMap in dependencies (requires source)
rg "HashMap" -g '*.rs' ~/.cargo/registry/src/

# Check dependency features
cargo tree -f '{p} {f}' | grep -E "ahash|random"

# Find getrandom in dependency tree
cargo tree -i getrandom
```

### CI Tests for Reproducibility

```rust
#[test]
fn test_deterministic_iteration() {
    use std::collections::BTreeMap;

    // Run multiple times with same data in different insertion orders
    for _ in 0..10 {
        let mut map1 = BTreeMap::new();
        map1.insert(2, "two");
        map1.insert(1, "one");
        map1.insert(3, "three");

        let mut map2 = BTreeMap::new();
        map2.insert(3, "three");
        map2.insert(1, "one");
        map2.insert(2, "two");

        let vec1: Vec<_> = map1.iter().collect();
        let vec2: Vec<_> = map2.iter().collect();

        assert_eq!(vec1, vec2, "Iteration order must be deterministic");
    }
}

#[test]
fn test_game_state_reproducibility() {
    let seed = 12345u64;
    let inputs = generate_test_inputs();

    // Run game twice with same inputs
    let state1 = run_game(seed, &inputs);
    let state2 = run_game(seed, &inputs);

    // Must produce identical checksums
    assert_eq!(
        state1.checksum, state2.checksum,
        "Game state must be reproducible"
    );
}
```

**CI workflow for cross-platform verification:**

```yaml
# .github/workflows/determinism.yml
name: Determinism Check

on: [push, pull_request]

jobs:
  test:
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4
      - name: Generate checksum
        run: cargo run --release --example generate_checksum > checksum.txt
      - uses: actions/upload-artifact@v4
        with:
          name: checksum-${{ matrix.os }}
          path: checksum.txt

  verify:
    needs: test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/download-artifact@v4
      - name: Compare checksums
        run: |
          # All platforms must produce identical checksums
          cat checksum-*/checksum.txt | sort -u | wc -l | grep -q '^1$' \
            || (echo "CHECKSUMS DIFFER!" && exit 1)
```

---

## Practical Patterns

### Pattern 1: Sort Before Iterating Over HashMap

When you must use HashMap for performance but need deterministic iteration:

```rust
use std::collections::HashMap;

let map: HashMap<EntityId, Entity> = /* ... */;

// ✅ Sort keys before iterating
let mut keys: Vec<_> = map.keys().collect();
keys.sort();

for key in keys {
    let entity = &map[key];
    process_entity(entity);
}

// Or collect and sort key-value pairs
let mut pairs: Vec<_> = map.iter().collect();
pairs.sort_by_key(|(k, _)| *k);

for (key, value) in pairs {
    process(key, value);
}
```

### Pattern 2: BTreeMap for Iteration-Order-Sensitive Code

```rust
use std::collections::BTreeMap;

/// Player registry with deterministic iteration
struct PlayerRegistry {
    // BTreeMap ensures iteration is always by PlayerHandle order
    players: BTreeMap<PlayerHandle, Player>,
}

impl PlayerRegistry {
    fn update_all(&mut self, delta: f32) {
        // Iteration order is deterministic (sorted by PlayerHandle)
        for (handle, player) in &mut self.players {
            player.update(delta);
        }
    }

    fn compute_checksum(&self) -> u64 {
        let mut hasher = DeterministicHasher::new();
        // Order is guaranteed, so checksum is deterministic
        for (handle, player) in &self.players {
            handle.hash(&mut hasher);
            player.hash(&mut hasher);
        }
        hasher.finish()
    }
}
```

### Pattern 3: Feature Flags for Hasher Control

```toml
# Cargo.toml
[features]
default = ["std"]
std = []
deterministic = []  # Use fixed hasher for testing/debugging

[dependencies]
ahash = { version = "0.8", optional = true }
```

```rust
// src/hash.rs
#[cfg(feature = "deterministic")]
pub type GameHashMap<K, V> = std::collections::HashMap<K, V, DeterministicBuildHasher>;

#[cfg(not(feature = "deterministic"))]
pub type GameHashMap<K, V> = std::collections::HashMap<K, V>;

// Usage
let mut map: GameHashMap<EntityId, Entity> = GameHashMap::default();
```

### Pattern 4: IndexMap for Hash Performance with Determinism

```rust
use indexmap::IndexMap;

/// Component storage with deterministic iteration and fast lookups
struct ComponentStore<T> {
    // IndexMap provides O(1) lookups and insertion-order iteration
    components: IndexMap<EntityId, T>,
}

impl<T> ComponentStore<T> {
    fn new() -> Self {
        Self { components: IndexMap::new() }
    }

    fn insert(&mut self, id: EntityId, component: T) {
        // Insertion order is preserved
        self.components.insert(id, component);
    }

    fn get(&self, id: &EntityId) -> Option<&T> {
        // O(1) lookup
        self.components.get(id)
    }

    fn iter(&self) -> impl Iterator<Item = (&EntityId, &T)> {
        // Iterates in insertion order (deterministic if insertions are)
        self.components.iter()
    }

    /// Sort by key for fully deterministic iteration
    fn iter_sorted(&self) -> impl Iterator<Item = (&EntityId, &T)>
    where
        EntityId: Ord,
    {
        let mut pairs: Vec<_> = self.components.iter().collect();
        pairs.sort_by_key(|(k, _)| *k);
        pairs.into_iter()
    }
}
```

### Pattern 5: Type Aliases for Clarity

```rust
// src/collections.rs

/// Deterministic map - iteration order guaranteed by key ordering
pub type DetMap<K, V> = std::collections::BTreeMap<K, V>;

/// Deterministic set - iteration order guaranteed by value ordering
pub type DetSet<T> = std::collections::BTreeSet<T>;

/// Insertion-order map - deterministic if insertion order is deterministic
pub type InsertOrderMap<K, V> = indexmap::IndexMap<K, V>;

/// Fast lookup map - NOT deterministic for iteration
/// Only use when you never iterate, or always sort before iterating
pub type FastMap<K, V> = std::collections::HashMap<K, V>;
```

---

## WASM and no_std Concerns

### hashbrown Behavior in no_std

`hashbrown` (which powers `std::collections::HashMap`) requires a hasher in no_std:

```rust
// no_std environment
#![no_std]
extern crate alloc;

use hashbrown::HashMap;
use core::hash::BuildHasherDefault;

// Must provide a hasher - no default RandomState!
type NoStdHashMap<K, V> = HashMap<K, V, BuildHasherDefault<FnvHasher>>;

let mut map: NoStdHashMap<i32, &str> = HashMap::default();
```

### Feature Leaking Can Introduce Non-Determinism

Dependencies can enable features that add randomness:

```toml
# Your Cargo.toml
[dependencies]
some-crate = "1.0"

# some-crate's Cargo.toml might have:
[dependencies]
ahash = { version = "0.8", features = ["compile-time-rng"] }
# This affects YOUR binary too!
```

**Mitigation strategies:**

1. **Audit dependencies:**

   ```bash
   cargo tree -f '{p} {f}' | grep ahash
   ```

2. **Use `default-features = false`:**

   ```toml
   [dependencies]
   some-crate = { version = "1.0", default-features = false }
   ```

3. **Pin dependency versions** and audit changes:

   ```toml
   [dependencies]
   ahash = "=0.8.11"  # Exact version
   ```

### WASM-Specific Randomness

In WASM, `getrandom` behavior depends on the target:

- **wasm32-unknown-unknown:** No default entropy source
- **wasm32-wasi:** Uses WASI's `random_get`
- **wasm32-unknown-emscripten:** Uses Emscripten's entropy

```toml
# For deterministic WASM builds
[target.wasm32-unknown-unknown.dependencies]
getrandom = { version = "0.2", features = ["custom"] }

# Or disable getrandom entirely and use fixed hasher
```

---

## Migration Patterns

### Migrating HashMap to BTreeMap

```rust
// Before: Non-deterministic
use std::collections::HashMap;

struct GameState {
    entities: HashMap<EntityId, Entity>,
}

// After: Deterministic
use std::collections::BTreeMap;

struct GameState {
    entities: BTreeMap<EntityId, Entity>,
}

// Key type must implement Ord
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct EntityId(u32);
```

**API differences to handle:**

| HashMap Method | BTreeMap Equivalent | Notes |
|---------------|---------------------|-------|
| `new()` | `new()` | Same |
| `with_capacity(n)` | `new()` | BTreeMap doesn't pre-allocate |
| `reserve(n)` | N/A | BTreeMap doesn't support |
| `shrink_to_fit()` | N/A | Not applicable |
| `entry(k)` | `entry(k)` | Same API |

### Migrating HashMap to IndexMap

```rust
// Before
use std::collections::HashMap;

let mut map: HashMap<K, V> = HashMap::new();

// After
use indexmap::IndexMap;

let mut map: IndexMap<K, V> = IndexMap::new();
```

IndexMap is largely API-compatible with HashMap, plus additional methods:

```rust
use indexmap::IndexMap;

let mut map = IndexMap::new();
map.insert("a", 1);
map.insert("b", 2);

// IndexMap-specific methods
map.get_index(0);           // Get by insertion index
map.swap_remove("a");       // Remove and swap with last (O(1))
map.shift_remove("a");      // Remove and shift (preserves order, O(n))
map.sort_keys();            // Sort in-place by keys
map.reverse();              // Reverse order
```

### Gradual Migration with Type Aliases

```rust
// Phase 1: Create aliases pointing to old types
pub type PlayerMap<V> = std::collections::HashMap<PlayerId, V>;

// Phase 2: Update code to use aliases (no behavior change)
let players: PlayerMap<Player> = PlayerMap::new();

// Phase 3: Change alias to deterministic type
pub type PlayerMap<V> = std::collections::BTreeMap<PlayerId, V>;

// Phase 4: Fix any compilation errors (missing Ord, etc.)
```

---

## Performance Tradeoffs

| Collection | Lookup | Insert | Iterate | Deterministic | Memory |
|-----------|--------|--------|---------|---------------|--------|
| `HashMap` | O(1) | O(1) | O(n) | ❌ No | Lower |
| `BTreeMap` | O(log n) | O(log n) | O(n) | ✅ Yes (sorted) | Higher |
| `IndexMap` | O(1) | O(1) | O(n) | ⚠️ If insertions are | Higher |
| `HashMap` + sort | O(1) | O(1) | O(n log n) | ✅ Yes | Lower |

### When to Use Each

**Use `BTreeMap` when:**

- You iterate frequently
- Data set is small to medium (<10,000 entries)
- You need range queries (`range()`, `range_mut()`)
- Keys naturally have an ordering

**Use `IndexMap` when:**

- You need O(1) lookups AND deterministic iteration
- Insertion order has meaning
- Data set is large
- You're migrating from HashMap with minimal changes

**Use `HashMap` + sort when:**

- Lookups vastly outnumber iterations
- You only occasionally need to iterate
- Memory is a concern

**Use `HashMap` with fixed hasher when:**

- You NEVER iterate over entries
- You only need deterministic hashes (for checksums)
- Maximum lookup performance is required

### Benchmarking Example

```rust
use criterion::{criterion_group, criterion_main, Criterion};
use std::collections::{BTreeMap, HashMap};
use indexmap::IndexMap;

fn benchmark_iteration(c: &mut Criterion) {
    let n = 10_000;

    let hash_map: HashMap<i32, i32> = (0..n).map(|i| (i, i)).collect();
    let btree_map: BTreeMap<i32, i32> = (0..n).map(|i| (i, i)).collect();
    let index_map: IndexMap<i32, i32> = (0..n).map(|i| (i, i)).collect();

    c.bench_function("HashMap iterate", |b| {
        b.iter(|| {
            let mut keys: Vec<_> = hash_map.keys().collect();
            keys.sort();
            keys.iter().map(|k| hash_map[k]).sum::<i32>()
        })
    });

    c.bench_function("BTreeMap iterate", |b| {
        b.iter(|| hash_map.values().sum::<i32>())
    });

    c.bench_function("IndexMap iterate", |b| {
        b.iter(|| index_map.values().sum::<i32>())
    });
}

criterion_group!(benches, benchmark_iteration);
criterion_main!(benches);
```

---

## Quick Reference Checklist

### For Game State (Must Be Deterministic)

- [ ] Use `BTreeMap`/`BTreeSet` instead of `HashMap`/`HashSet`
- [ ] Or use `IndexMap`/`IndexSet` with deterministic insertion order
- [ ] Ensure all keys implement `Ord` (for BTreeMap) or maintain insertion order
- [ ] Never iterate over `HashMap` without sorting first
- [ ] Audit dependencies for `ahash`, `getrandom`, `const-random`

### For Checksums/Hashing

- [ ] Use a deterministic hasher (FNV-1a, SipHash with fixed key)
- [ ] Hash in deterministic order (sorted keys or stable index)
- [ ] Don't use `DefaultHasher` (it's randomized)

### For Testing

- [ ] Run same scenario multiple times, verify identical results
- [ ] Test on multiple platforms via CI
- [ ] Use `strace -e getrandom` to detect runtime randomness
- [ ] Compare checksums across runs and platforms

### For Dependencies

- [ ] Audit `cargo tree` for hash-related crates
- [ ] Use `default-features = false` where possible
- [ ] Consider pinning versions of hash crates
- [ ] Test with `--features deterministic` in CI

---

*Collection determinism is subtle but critical. When in doubt, use `BTreeMap` — the performance cost is usually acceptable, and the bugs you avoid are worth it.*
