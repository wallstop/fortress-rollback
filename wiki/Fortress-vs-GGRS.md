<p align="center">
  <img src="assets/logo.svg" alt="Fortress Rollback" width="128">
</p>

# Fortress Rollback vs GGRS

This document summarizes the key differences between **Fortress Rollback** (this library) and the original **GGRS** (good game rollback system), as well as the bugs discovered and fixed during the fork's development.

## Quick Summary

| Category | GGRS | Fortress Rollback |
|----------|------|-------------------|
| **Determinism** | `HashMap`/`HashSet` (non-deterministic iteration) | `BTreeMap`/`BTreeSet` (guaranteed order) |
| **Panic Safety** | Some `assert!` and `panic!` in library code | All converted to recoverable errors |
| **Test Coverage** | Basic test suite | ~1500 tests (~92% coverage) |
| **Formal Verification** | None | TLA+, Z3 SMT proofs, Kani proofs |
| **Hashing** | `DefaultHasher` (random seed per process) | FNV-1a deterministic hashing |
| **Dependencies** | `bitfield-rle`, `varinteger`, `rand` | Internal implementations (fewer deps) |
| **Type Safety** | `Config::Address` requires `Hash` | `Config::Address` requires `Hash` + `Ord` |
| **Desync Detection** | Off by default (opt-in) | On by default (`interval: 60`) |
| **WASM Support** | Requires `wasm-bindgen` + `getrandom/js` features | Works out of the box, no special features |
| **Time API** | `std::time::Instant` (not WASM-compatible) | `web_time::Instant` (cross-platform) |
| **Spectator Handles** | May include local players in spectator lists | Explicit `is_spectator_for()` validation |
| **Error Types** | String-based allocation on hot paths | Dual-variant pattern (Copy for hot paths) |
| **Build-time Validation** | Some panics at runtime | Returns `Result` at build time |

---

## Breaking Changes from GGRS

### 1. Crate and Type Renames

```rust
// Before (GGRS)
use ggrs::{GgrsError, GgrsEvent, GgrsRequest};

// After (Fortress Rollback)
use fortress_rollback::{FortressError, FortressEvent, FortressRequest};
```

### 2. `Config::Address` Now Requires `Ord`

```rust
// Before: Only needed Clone + PartialEq + Eq + Hash + Debug
// After: Also needs PartialOrd + Ord

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
struct MyAddress { /* ... */ }
```

**Why:** Enables deterministic iteration using `BTreeMap` instead of `HashMap`.

### 3. New Type Aliases

```rust
// FortressResult type alias for ergonomic error handling
pub type FortressResult<T, E = FortressError> = std::result::Result<T, E>;

// InputVec uses SmallVec for better performance (no heap allocation for 1-4 players)
pub type InputVec<I> = SmallVec<[(I, InputStatus); 4]>;
```

**Why InputVec matters:** `FortressRequest::AdvanceFrame` now provides inputs as `InputVec<T::Input>` instead of `Vec`. This avoids heap allocation for games with 1-4 players. The type implements `Deref<Target = [...]>`, so most code works unchanged.

### 4. New Configuration APIs

Fortress Rollback introduces structured configuration with presets:

```rust
use fortress_rollback::{SyncConfig, ProtocolConfig, TimeSyncConfig};

let session = SessionBuilder::<MyConfig>::new()
    .with_sync_config(SyncConfig::high_latency())
    .with_protocol_config(ProtocolConfig::competitive())
    .with_time_sync_config(TimeSyncConfig::responsive())
    .start_p2p_session()?;
```

---

## Bugs Fixed (By Priority)

### Critical - Would cause crashes or game-breaking issues

#### 1. Frame 0 Rollback Crash

**Likelihood:** High (occurs under poor network conditions early in game)

**When this occurs:** During the initial synchronization phase when players connect with high latency or packet loss, frame 0 can receive corrected remote inputs before the local client has advanced to frame 1.

**Issue:** When a misprediction was detected at frame 0 (the first frame) and a remote input correction arrived before advancing past it, the session would crash with:

```text
InvalidFrame { frame: Frame(0), reason: "must load frame in the past" }
```

**Root Cause:** `adjust_gamestate()` attempted `load_frame(0)` which fails because you cannot load the frame you're currently on.

**Fix:** Added guard to detect when `frame_to_load >= current_frame` and skip rollback (just reset predictions), since we haven't advanced past the incorrect frame yet.

---

#### 2. Multi-Process Checksum Desync (BUG-001)

**Likelihood:** High (occurs in any multi-process game with desync detection)

**Issue:** When running multiple game instances (e.g., testing P2P locally), checksum comparisons would fail even with identical game states.

**Root Cause:** Two issues:

1. `std::collections::hash_map::DefaultHasher` uses a random seed per process, so different processes produce different hashes for identical data
2. Checksum computation over all frames failed when older frames were discarded from input queue at different times on different peers

**Fix:**

- Created new `fortress_rollback::hash` module with FNV-1a deterministic hashing
- Window-based checksum computation using last 64 frames ensures frames are always available for both peers

---

#### 3. Spectator Handle Bug

**Likelihood:** Medium-High (affects any game with spectators)

**Issue:** In original GGRS, methods like `spectator_handles()` could accidentally include local players in the spectator list, leading to incorrect input routing and state corruption.

**Root Cause:** No explicit validation that handle indices represent spectators vs players. Player handles are `0` to `num_players - 1`, while spectator handles are `num_players` and above.

**Fix:** Fortress implements explicit handle classification:

```rust
// Explicit spectator validation
impl PlayerHandle {
    pub fn is_spectator_for(&self, num_players: usize) -> bool {
        self.0 >= num_players
    }
}

// Pseudo-code: Simplified for illustration (actual API uses SessionBuilder::add_player)
pub fn add_spectator(
    &mut self,
    handle: PlayerHandle,
    address: A,
) -> Result<(), FortressError> {
    if !handle.is_spectator_for(self.num_players) {
        return Err(InvalidRequestKind::InvalidSpectatorHandle {
            handle,
            num_players: self.num_players,
        }.into());
    }
    // ...
}
```

---

### High - Could cause desyncs or incorrect behavior

#### 4. Non-Deterministic Collection Iteration

**Likelihood:** Medium-High (depends on player count and frame timing)

**Issue:** GGRS used `HashMap` and `HashSet` throughout. While the values stored were correct, iteration order is not guaranteed and can vary between:

- Different runs of the same program
- Different platforms (x86 vs ARM)
- Different compiler versions

**Impact:** Any code that iterated over these collections (player inputs, checksums, network endpoints) could behave differently across peers, leading to subtle desyncs.

**Fix:** Replaced all `HashMap` with `BTreeMap` and `HashSet` with `BTreeSet`. All collection iteration now has predictable, sorted ordering.

---

#### 5. False Positive Desync Detection

**Likelihood:** Medium (occurs during rollbacks with active desync checking)

**Issue:** In `P2PSession::advance_frame()`, it was possible for a desync to incorrectly be detected when:

1. A checksum-changing rollback was enqueued
2. A to-be-rolled-back frame was marked as confirmed
3. That frame's still-incorrect checksum was sent to peers

**Fix:** Reordered operations to ensure checksums are computed after rollback completion. (Fixed in upstream GGRS 0.11, carried forward in Fortress)

---

### Medium - Edge cases or quality-of-life issues

#### 6. `assert!` Panics in Production Code

**Likelihood:** Low-Medium (depends on edge case triggers)

**Issue:** Several `assert!` macros in library code would panic on unexpected conditions instead of returning errors:

- `src/rng.rs` - Empty or invalid ranges in random number generation
- `src/input_queue.rs` - Queue length validation
- Various other locations

**Fix:** Converted all `assert!` to `report_violation!` macro with graceful recovery. Library code now returns `Result` types instead of panicking.

---

#### 7. Spectator Confirmed Input Panic

**Likelihood:** Low (spectator with missing data)

**Issue:** `InputQueue::confirmed_input` would panic if called when data was missing.

**Fix:** Now returns `Result<T, FortressError>` and bubbles the error up through the spectator session.

---

#### 8. Invalid Array Index in TimeSync

**Likelihood:** Low (negative frame numbers)

**Issue:** Potential out-of-bounds array access when frame numbers were negative or in edge cases.

**Fix:** Added bounds checking that skips updates rather than panicking.

---

## Platform Improvements

### WASM Compilation Simplification

**GGRS WASM considerations:** While GGRS uses the `instant` crate for cross-platform time (which works on WASM), it still depends on the `rand` crate which may require additional configuration for WASM targets depending on your RNG needs.

**Fortress solution:** Works on WASM out of the box with no special features needed:

```toml
# Fortress - just works
[dependencies]
fortress-rollback = "0.6"
```

**How this works:**

1. **Custom PCG32 RNG** - Eliminates the `rand` crate dependency entirely
2. **`web_time::Instant`** - Cross-platform timing that works on native and WASM without conditional compilation

### Cross-Platform Time Synchronization

Both GGRS (via the `instant` crate) and Fortress (via `web_time`) provide cross-platform `Instant`. Fortress uses `web_time::Instant` which provides a unified API:

- **Native platforms**: Delegates to `std::time::Instant`
- **WASM**: Uses `performance.now()` from the Web Performance API

This means the same code works across all platforms without feature flags or conditional compilation.

---

## New Features

### Custom PCG32 Random Number Generator

Fortress implements its own PCG32 (Permuted Congruential Generator) to eliminate external dependencies:

```rust
use fortress_rollback::rng::Pcg32;

let mut rng = Pcg32::seed_from_u64(12345);
let value: u32 = rng.next_u32();
let range_value: u32 = rng.gen_range(1..100);
```

**Why not use `rand`?**

- **Fewer dependencies** - `rand` pulls in `getrandom` and platform-specific crates
- **No WASM complexity** - `getrandom` requires `features = ["js"]` for WASM
- **Full determinism** - No platform-specific entropy sources that could differ

**Important:** The RNG is only used for testing and network simulation (ChaosSocket). Game state should never depend on this RNG - games must implement their own seeded RNG synchronized across peers.

### Deterministic Hashing Module

```rust
use fortress_rollback::hash::{fnv1a_hash, DeterministicHasher};

// Hash any serializable data deterministically
let checksum = fnv1a_hash(&game_state);

// Or use the hasher directly
let mut hasher = DeterministicHasher::new();
hasher.write(&data);
let hash = hasher.finish();
```

### Confirmed Inputs API

```rust
// Get confirmed inputs for computing deterministic checksums
let inputs = session.confirmed_inputs_for_frame(frame)?;
```

### Structured Error Types (Hot Path Optimization)

Fortress uses a dual-variant error pattern to avoid heap allocation on hot paths:

```rust
pub enum FortressError {
    // Hot path: zero allocation (Copy types only)
    InvalidFrameStructured {
        frame: Frame,
        reason: InvalidFrameReason,
    },

    // Backward compatibility: allocates a String
    InvalidFrame {
        frame: Frame,
        reason: String,
    },

    // ... other variants
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidFrameReason {
    NullFrame,
    Negative,
    MustBeNonNegative,
    NotInPast { current_frame: Frame },
    OutsidePredictionWindow { current_frame: Frame, max_prediction: usize },
    WrongSavedFrame { saved_frame: Frame },
    NotConfirmed { confirmed_frame: Frame },
    NullOrNegative,
    Custom(&'static str),
}
```

**Why this matters:** In rollback netcode, error handling can occur thousands of times per second during prediction. String allocation on every error would cause significant GC pressure and latency spikes.

### Violation Pipeline

Fortress Rollback replaces GGRS's `assert!` panics with a structured telemetry system. When internal invariants are violated, instead of crashing, the library:

1. **Reports the violation** with structured context (severity, category, location, frame number)
2. **Attempts graceful recovery** where possible
3. **Notifies observers** for debugging, metrics, or alerting

**Why this matters:** In GGRS, a network glitch or edge case could cause `assert!` to panic and crash your game. Fortress Rollback instead logs the issue and continues, allowing you to debug production issues without losing player sessions.

#### Violation Severities

```rust
ViolationSeverity::Warning   // Unexpected but recovered automatically
ViolationSeverity::Error     // Serious issue, may degrade behavior
ViolationSeverity::Critical  // State may be corrupted, desync possible
```

#### Violation Categories

```rust
ViolationKind::FrameSync          // Frame counter mismatch
ViolationKind::InputQueue         // Input sequence gaps or corruption
ViolationKind::StateManagement    // State save/load issues
ViolationKind::NetworkProtocol    // Protocol state machine errors
ViolationKind::ChecksumMismatch   // Desync detection triggered
ViolationKind::Configuration      // Invalid parameter combinations
ViolationKind::Synchronization    // Excessive sync retries, timeouts
ViolationKind::InternalError      // Library bugs (should never happen)
ViolationKind::Invariant          // Runtime invariant check failed
ViolationKind::ArithmeticOverflow // Frame counter overflow detected
```

#### Violation Observer Integration

Sessions support pluggable violation observers:

```rust
use fortress_rollback::telemetry::{
    TracingObserver,
    CollectingObserver,
    CompositeObserver,
    ViolationSeverity,
};
use std::sync::Arc;

// TracingObserver (default): logs to the `tracing` crate
let tracing_observer = Arc::new(TracingObserver::new());

// CollectingObserver: stores violations for testing/debugging
let collecting_observer = Arc::new(CollectingObserver::new());

// CompositeObserver: forwards to multiple observers
let mut composite = CompositeObserver::new();
composite.add(tracing_observer.clone());
composite.add(collecting_observer.clone());
let composite = Arc::new(composite);

let session = SessionBuilder::new()
    .with_violation_observer(composite)
    .start_p2p_session()?;

// After gameplay, check for issues
if !collecting_observer.is_empty() {
    for violation in collecting_observer.violations_at_severity(ViolationSeverity::Error) {
        log::error!("Session issue: {} at {}", violation.message, violation.location);
    }
}
```

### Configuration Presets

**Why this exists:** GGRS uses hardcoded constants for network timing, buffer sizes, and sync behavior. This works for "average" conditions but fails in edge cases. Fortress Rollback exposes these as configurable structs with presets, so you can tune behavior for LAN tournaments, mobile networks, or high-latency WAN play.

**Key configuration areas:**

| Config | Controls | Why It Matters |
|--------|----------|----------------|
| `SyncConfig` | Connection handshake timing | Faster sync on LAN, more retries on lossy networks |
| `ProtocolConfig` | Network quality reporting, timeouts | Affect disconnect detection sensitivity |
| `TimeSyncConfig` | Frame timing window size | Trade-off between smoothness and responsiveness |
| `SpectatorConfig` | Spectator buffer and catch-up | Smooth streaming vs low latency viewing |
| `InputQueueConfig` | Input buffer size | Memory vs max rollback distance |

Built-in presets for common network scenarios:

```rust
// Sync configuration presets
SyncConfig::lan()               // LAN/local network (fast sync)
SyncConfig::high_latency()      // High RTT connections (100-200ms)
SyncConfig::lossy()             // Packet loss environments (5-15%)
SyncConfig::mobile()            // Mobile/cellular networks
SyncConfig::competitive()       // Esports/tournament (fast, strict)

// Protocol configuration presets
ProtocolConfig::competitive()   // Low-latency competitive play
ProtocolConfig::high_latency()  // High RTT tolerance
ProtocolConfig::mobile()        // Mobile network tolerance
ProtocolConfig::debug()         // Development/debugging

// Time sync configuration presets
TimeSyncConfig::lan()           // LAN play (small window)
TimeSyncConfig::responsive()    // Prioritize responsiveness
TimeSyncConfig::smooth()        // Prioritize smoothness
TimeSyncConfig::mobile()        // Very large window for jitter
TimeSyncConfig::competitive()   // Fast adaptation

// Spectator configuration presets
SpectatorConfig::local()        // Local viewing (minimal latency)
SpectatorConfig::fast_paced()   // Fast-paced action games
SpectatorConfig::slow_connection() // Poor network spectators
SpectatorConfig::mobile()       // Mobile spectators
SpectatorConfig::broadcast()    // Streaming/tournament broadcasts

// Input queue configuration presets
InputQueueConfig::standard()    // Default (128 frames)
InputQueueConfig::high_latency() // High-latency tolerance (256 frames)
InputQueueConfig::minimal()     // Memory-constrained (32 frames)

// Chaos socket presets for testing
ChaosConfig::passthrough()      // No chaos (testing baseline)
ChaosConfig::poor_network()     // Typical poor conditions
ChaosConfig::terrible_network() // Extreme conditions
ChaosConfig::mobile_network()   // Mobile/cellular simulation
ChaosConfig::wifi_interference() // WiFi with interference
ChaosConfig::intercontinental() // High-latency stable connection
```

---

## Code Quality Improvements

### Formal Verification

- **7 TLA+ specifications** covering core protocols (Rollback, InputQueue, NetworkProtocol, TimeSync, ChecksumExchange, SpectatorSession, Concurrency)
- **115 Kani proofs** for bounded model checking
- **54 Z3 SMT proofs** for algorithmic correctness

### Testing

- **~1500 tests** (unit + integration + property-based)
- **~92% code coverage**
- Property-based tests with proptest
- Mutation testing (95% detection rate)
- Loom concurrency tests
- Multi-process network tests

### Documentation

- Comprehensive rustdoc with examples
- Architecture documentation with ASCII diagrams
- User guide with configuration reference
- Migration guide from GGRS

### Dependencies Reduced

| Dependency | GGRS | Fortress |
|------------|------|----------|
| `bitfield-rle` | Required | Internal RLE implementation |
| `varinteger` | Required | Internal implementation |
| `rand` | Required | Internal PCG32 RNG |
| `getrandom` | Required (transitive) | Not needed |
| Time handling | `std::time::Instant` | `web_time::Instant` |

---

## Migration Checklist

- [ ] Update `Cargo.toml`: `ggrs = "0.11"` -> `fortress-rollback = "0.6"`
- [ ] Update imports: `use ggrs::*` -> `use fortress_rollback::*`
- [ ] Rename types: `GgrsError` -> `FortressError`, etc.
- [ ] Add `Ord` + `PartialOrd` to your `Config::Address` type
- [ ] Remove WASM-specific feature flags (`wasm-bindgen`, `getrandom/js`)
- [ ] (Optional) Update to new configuration APIs for better presets
- [ ] (Optional) Add violation observer for debugging

See the full [Migration Guide](Migration) for detailed instructions.

---

## Summary

Fortress Rollback is a **correctness-first fork** of GGRS. The main benefits are:

1. **Determinism guaranteed** - No more subtle desyncs from collection iteration order
2. **Panic-free** - All library code returns `Result` types
3. **Battle-tested** - ~1500 tests and formal verification
4. **Better debugging** - Violation observers and deterministic hashing
5. **WASM-ready** - Works on all platforms without special feature flags
6. **Zero-allocation hot paths** - Structured error types avoid heap allocation

For most users, migration is straightforward: rename imports, add `Ord` to your address type, remove WASM feature flags, and enjoy more reliable netcode.

See the [Migration Guide](Migration) for step-by-step instructions.
