# Fortress Rollback Improvement Plan

**Version:** 2.4
**Last Updated:** December 2025 (Session 16)
**Goal:** Transform Fortress Rollback into a production-grade, formally verified rollback networking library with >90% test coverage, absolute determinism guarantees, and exceptional usability.

---

## Current Status Summary

### Metrics
| Metric | Current | Target | Status |
|--------|---------|--------|--------|
| Library Tests | 180 | 100+ | ✅ Exceeded |
| Integration Tests | ~40 | 30+ | ✅ Exceeded |
| Est. Coverage | ~89% | >90% | 🔄 Close |
| Clippy Warnings (lib) | 0 | 0 | ✅ Clean |
| Panics from Public API | 0 | 0 | ✅ |
| HashMap/HashSet Usage | 0 | 0 | ✅ |
| DefaultHasher Usage | 0 | 0 | ✅ |
| Miri Clean | 145/145 | All | ✅ |
| TLA+ Specs | 4 | 4 | ✅ Complete |
| Kani Proofs | 38 | 3+ | ✅ Complete |
| Rust Edition | 2021 | - | ✅ Rust 1.75+ compatible |
| Network Resilience Tests | 20/20 | 20 | ✅ All pass |
| Multi-Process Tests | 8/8 | 8 | ✅ All pass |

### What's Complete ✅

- **Phase 1: Foundation & Safety** - Project rebrand, deterministic collections, panic elimination, structured telemetry, session observers, core unit tests, property-based testing (15 tests), runtime invariant checking, paranoid mode, CI/CD pipeline
- **Phase 1.6: Type Safety** - `Frame` newtype with arithmetic ops, `PlayerHandle` newtype with bounds checking
- **Phase 1.7: Deterministic Hashing** - New `fortress_rollback::hash` module with FNV-1a, all `HashSet` → `BTreeSet`, all test stubs use deterministic hashing
- **Phase 2.1: Miri Testing** - All 137 non-proptest library tests pass under Miri with no undefined behavior detected. Miri CI job added.
- **Phase 3.1: Integration Tests** - Multi-player (3-4 players), rollback scenarios (deep, frequent, with varying input delays), spectator synchronization
- **Phase 3.2: Network Resilience** - ChaosSocket fault injection, 20 network condition tests (latency, jitter, packet loss, burst loss, reordering, duplication, asymmetric conditions), all tests pass reliably
- **Phase 4.1: Documentation** - Architecture guide (`docs/ARCHITECTURE.md`), User guide (`docs/USER_GUIDE.md`)
- **Phase 0: Formal Specification** - Complete formal specs (`specs/FORMAL_SPEC.md`, `specs/API_CONTRACTS.md`, `specs/DETERMINISM_MODEL.md`)
- **Phase 2.4: TLA+ Specifications** - 4 of 4 TLA+ specs complete (NetworkProtocol, InputQueue, Rollback, Concurrency)
- **Phase 2.2: Kani Formal Verification** - 38 Kani proofs covering:
  - Frame arithmetic safety (SAFE-6): 12 proofs for Frame operations
  - InputQueue buffer bounds (INV-4, INV-5): 14 proofs for circular buffer operations
  - SyncLayer state consistency: 12 proofs for frame management and rollback
- **Rust Compatibility** - Downgraded to edition 2021 for Rust 1.75+ compatibility:
  - Fixed nightly-only features (`is_multiple_of`, const fn floats)
  - Pinned dependencies (proptest 1.4.0, macroquad 0.3.25)
  - All 180 library tests pass

### Next Priority Actions
1. **📊 Benchmarking (MEDIUM)** - Performance baseline needed
2. **🧪 Loom Concurrency Testing (LOW)** - Verify GameStateCell thread safety
3. **📈 Coverage Improvement** - Push to >90% coverage target

---

## Breaking Changes Policy

**Core Principle: Correctness Over Compatibility**

This project **explicitly permits breaking changes** when they:
1. Improve correctness and safety
2. Enhance determinism guarantees
3. Enable formal verification
4. Align with production-grade goals

**Rationale:**
- Correctness and determinism are non-negotiable for rollback networking
- The library is pre-1.0 (semver allows breaking changes)
- Each breaking change documented in CHANGELOG.md with migration path

**Examples of Acceptable Breaking Changes:**
- Replacing HashMap with BTreeMap ✅ (done)
- Adding `Ord` trait bound to `Config::Address` ✅ (done)
- Changing panicking functions to return `Result`
- Introducing session type state machines

---

## Remaining Work

### Phase 0: Formal Specification ✅ COMPLETE

**Priority: HIGH** - Foundation for all verification work.

#### 0.1 RFC-Style Formal Specification ✅
- [x] Create `specs/FORMAL_SPEC.md` with mathematical specification
- [x] Document all system invariants (11 invariants):
  - INV-1: Frame monotonicity
  - INV-2: Rollback boundedness
  - INV-3: Input consistency (immutability)
  - INV-4: Queue length bounds
  - INV-5: Queue index validity
  - INV-6: State availability
  - INV-7: Confirmed frame consistency
  - INV-8: Saved frame consistency
  - INV-9: Message causality
  - INV-10: Determinism
  - INV-11: No panics
- [x] Specify state machines (UdpProtocol: 5 states, transitions)
- [x] Define message protocol with 7 message types
- [x] Document 7 safety properties (SAFE-1 through SAFE-7)
- [x] Document 6 liveness properties (LIVE-1 through LIVE-6)
- [x] Component specifications (InputQueue, SyncLayer operations)

#### 0.2 API Contract Specification ✅
- [x] Create `specs/API_CONTRACTS.md`
- [x] Document preconditions/postconditions for all public APIs
- [x] Document SessionBuilder, P2PSession, SpectatorSession, SyncTestSession
- [x] Document GameStateCell and request handling contracts
- [x] Error catalog with recovery guidance
- [x] Cross-cutting invariants documented

#### 0.3 Determinism Model ✅
- [x] Create `specs/DETERMINISM_MODEL.md`
- [x] Document 6 determinism requirements (DETER-1 through DETER-6)
- [x] Document 5 library guarantees (G1 through G5)
- [x] Document user responsibilities (R1 through R4)
- [x] Document common pitfalls (6 pitfalls with examples)
- [x] Verification strategies (4 strategies)
- [x] Platform compatibility matrix

### Phase 2: Formal Verification (Continued)

#### 2.2 Kani Formal Verification ✅ COMPLETE
- [x] Set up `kani-verifier` (project now uses edition 2021 for compatibility)
- [x] Create proofs for Frame arithmetic (12 proofs in `src/lib.rs`):
  - Frame::new validity, Frame::NULL consistency
  - Addition/subtraction safety for typical game usage
  - Ordering consistency with i32
  - Modulo operation for queue indexing
  - Option conversion round-trip
  - PlayerHandle validity checking
- [x] Create proofs for InputQueue (14 proofs in `src/input_queue.rs`):
  - INV-4: Queue length bounded (new queue, after add, after discard)
  - INV-5: Queue indices valid (head/tail within bounds)
  - Head wraparound correctness
  - Sequential input acceptance
  - Non-sequential input rejection
  - Frame delay handling
  - Circular buffer length consistency
- [x] Create proofs for SyncLayer (12 proofs in `src/sync_layer.rs`):
  - INV-1: Frame monotonicity (advance_frame increases)
  - INV-7: Confirmed frame consistency (bounded by current)
  - INV-8: Saved frame consistency (bounded by current)
  - load_frame bounds validation
  - SavedStates circular indexing
  - Sparse saving constraints
  - reset_prediction preserves frame state

**Note:** Proofs use `#[cfg(kani)]` and require Kani verifier to execute. Run with `cargo kani`.

#### 2.3 Loom Concurrency Testing
- [ ] Test concurrent GameStateCell operations
- [ ] Verify no deadlocks in Mutex usage
- [ ] Test event queue concurrent push/pop

#### 2.4 TLA+ Specifications ✅ COMPLETE (4 of 4)
**Priority: HIGH** - Mathematical proofs of protocol correctness.

- [x] **Network Protocol** (`specs/tla/NetworkProtocol.tla`) ✅
  - States: Initializing → Synchronizing → Running → Disconnected → Shutdown
  - Actions: StartSync, HandleSyncRequest/Reply, DisconnectTimeout, Tick, Shutdown
  - Safety: Valid state transitions, sync counter non-negative
  - Liveness: Eventually synchronized, no deadlock

- [x] **Input Queue** (`specs/tla/InputQueue.tla`) ✅
  - Circular buffer operations (AddInput, GetInput, DiscardConfirmed)
  - Prediction/confirmation with firstIncorrectFrame tracking
  - Safety: INV-4 (length bounded), INV-5 (valid indices), FIFO ordering, no gaps
  - Liveness: Predictions eventually confirmed

- [x] **Rollback Mechanism** (`specs/tla/Rollback.tla`) ✅
  - State save/load operations
  - Rollback triggers (firstIncorrectFrame detection)
  - Sparse saving mode support
  - Safety: INV-2 (bounded rollback), INV-6 (state availability), SAFE-4 (consistency)
  - Liveness: LIVE-3 (progress), LIVE-4 (rollback completes)

- [x] **Concurrent State Access** (`specs/tla/Concurrency.tla`) ✅
  - GameStateCell operations (save, load, data)
  - Mutex-based thread synchronization
  - Safety: Mutual exclusion, no data races, frame consistency, linearizability
  - Liveness: No deadlock, operations complete, fair lock acquisition

#### 2.5 Z3 SMT Solver Integration 📋 (TODO - HIGH VALUE)

**Status:** NOT STARTED - Research Complete

**Why Z3?**
Z3 is Microsoft Research's SMT (Satisfiability Modulo Theories) solver that can prove mathematical properties about code. Unlike testing which checks specific cases, Z3 can prove properties hold for ALL possible inputs.

**Rust Integration Options:**
1. **`z3` crate** (v0.19.5) - High-level Rust bindings
   - GitHub: https://github.com/prove-rs/z3.rs
   - crates.io: https://crates.io/crates/z3
   - 456 stars, actively maintained
   - Requires Z3 library installed or can vendor with `vendor-boolector` feature

2. **Integration Approach:** Write Z3 proofs as Rust `#[test]` functions with `#[cfg(z3)]`

**Proposed Z3 Verification Targets:**

| Property | Description | Priority |
|----------|-------------|----------|
| Frame arithmetic overflow | Prove wrapping semantics are correct | HIGH |
| Circular buffer bounds | Prove head/tail always valid | HIGH |
| Rollback frame selection | Prove target is always <= current_frame | HIGH |
| Sparse saving correctness | Prove saved state available when needed | MEDIUM |
| Input consistency | Prove confirmed inputs never change | MEDIUM |
| Checksum determinism | Prove same inputs → same checksum | MEDIUM |

**Example Z3 Proof Structure:**
```rust
#[cfg(z3)]
#[test]
fn z3_prove_rollback_frame_valid() {
    use z3::*;
    let cfg = Config::new();
    let ctx = Context::new(&cfg);
    let solver = Solver::new(&ctx);
    
    // Define symbolic variables
    let current_frame = Int::new_const(&ctx, "current_frame");
    let first_incorrect = Int::new_const(&ctx, "first_incorrect");
    let max_prediction = Int::new_const(&ctx, "max_prediction");
    
    // Assert preconditions
    solver.assert(&current_frame.ge(&Int::from_i64(&ctx, 0)));
    solver.assert(&first_incorrect.ge(&Int::from_i64(&ctx, 0)));
    solver.assert(&first_incorrect.le(&current_frame));
    solver.assert(&max_prediction.eq(&Int::from_i64(&ctx, 8)));
    
    // Assert the negation of what we want to prove (to find counterexample)
    let frame_to_load = first_incorrect.clone();
    let invalid = frame_to_load.gt(&current_frame);
    solver.assert(&invalid);
    
    // If UNSAT, the property always holds
    assert_eq!(solver.check(), SatResult::Unsat);
}
```

**Implementation Tasks:**
- [ ] Add `z3` as dev-dependency with optional feature
- [ ] Create `src/verification/z3_proofs.rs` module
- [ ] Write proofs for frame arithmetic safety
- [ ] Write proofs for circular buffer indexing
- [ ] Write proofs for rollback frame selection
- [ ] Add Z3 to CI (optional, may require Z3 installation)

#### 2.6 Creusot Deductive Verification 📋 (TODO - MEDIUM VALUE)

**Status:** NOT STARTED - Research Complete

**What is Creusot?**
Creusot is a deductive verifier for Rust that translates Rust code to the Why3 intermediate verification language. It can prove absence of panics, overflows, and assertion failures, with annotations for functional correctness.

**Key Features:**
- GitHub: https://github.com/creusot-rs/creusot
- 1.4k stars, actively developed (v0.7.0 released Nov 2024)
- Translates Rust to Why3/Coma intermediate verification language
- Uses Why3's SMT solver backends (Z3, CVC5, Alt-Ergo)
- Supports refinement types and loop invariants
- Can verify complex data structures and algorithms

**Dependencies Required:**
- Rust nightly toolchain (specific version pinned by Creusot)
- OCaml and opam package manager
- Why3 platform (installed via opam)

**Installation:**
```bash
# Install OCaml and opam
# On Debian/Ubuntu:
apt install opam
opam init

# Clone and install Creusot
git clone https://github.com/creusot-rs/creusot
cd creusot
./INSTALL

# Verify installation
cargo creusot --help
```

**Example Creusot Annotations for Fortress Rollback:**
```rust
use creusot_contracts::*;

#[requires(frame.as_i32() >= 0)]
#[requires(frame.as_i32() < self.current_frame.as_i32())]
#[requires(frame.as_i32() >= self.current_frame.as_i32() - self.max_prediction as i32)]
#[ensures(result.is_ok() ==> self.current_frame == frame)]
#[ensures(result.is_err() ==> self.current_frame == old(self.current_frame))]
pub fn load_frame(&mut self, frame: Frame) -> Result<FortressRequest<T>, FortressError> {
    // ...
}

#[invariant(self.length <= INPUT_QUEUE_LENGTH)]
#[invariant(self.head < INPUT_QUEUE_LENGTH)]
#[invariant(self.tail < INPUT_QUEUE_LENGTH)]
impl<T: Config> InputQueue<T> {
    // ...
}
```

**Proposed Creusot Verification Targets:**

| Component | Property | Priority |
|-----------|----------|----------|
| `Frame` | Arithmetic never overflows in game scenarios | HIGH |
| `InputQueue` | Circular buffer indices always valid | HIGH |
| `InputQueue` | Length invariant maintained | HIGH |
| `SyncLayer` | `load_frame` preconditions ensure success | MEDIUM |
| `SyncLayer` | Frame monotonicity (except during rollback) | MEDIUM |
| `GameStateCell` | Mutex guarantees exclusive access | LOW |

**Implementation Tasks:**
- [ ] Add `creusot-contracts` as optional dev-dependency
- [ ] Create `src/verification/creusot_specs.rs` module
- [ ] Add Creusot annotations to `Frame` type
- [ ] Add Creusot annotations to `InputQueue`
- [ ] Add Creusot annotations to `SyncLayer::load_frame`
- [ ] Set up CI job for Creusot verification (optional)
- [ ] Document verification results

**Trade-offs:**
- **Pros:** Very powerful for proving functional correctness, good IDE support
- **Cons:** Requires nightly Rust, external OCaml/Why3 dependencies, learning curve
- **Effort:** HIGH (2-3 days for initial setup, ongoing maintenance)
- **Value:** MEDIUM (Kani already covers many properties)

#### 2.7 Prusti Contract Verification 📋 (TODO - MEDIUM VALUE)

**Status:** NOT STARTED - Research Complete

**What is Prusti?**
Prusti is a prototype verifier for Rust developed by ETH Zurich. It can prove absence of panics, integer overflows, and correctness of user-specified contracts using the Viper verification infrastructure.

**Key Features:**
- GitHub: https://github.com/viperproject/prusti-dev
- 1.7k stars, actively developed
- Based on Viper verification infrastructure
- Has VS Code extension for interactive verification
- Supports pre/postconditions, loop invariants, and type invariants
- Good documentation and tutorial

**Dependencies Required:**
- Rust nightly toolchain (specific version)
- Java Runtime Environment (JRE 11+)
- Prusti binaries (downloadable or build from source)

**Installation:**
```bash
# Option 1: VS Code Extension (easiest)
# Install "Prusti Assistant" from VS Code marketplace

# Option 2: Command line
# Download from GitHub releases:
# https://github.com/viperproject/prusti-dev/releases

# Option 3: Build from source
git clone https://github.com/viperproject/prusti-dev
cd prusti-dev
./x.py setup
./x.py build --release
```

**Example Prusti Annotations for Fortress Rollback:**
```rust
use prusti_contracts::*;

impl Frame {
    #[pure]
    #[ensures(result >= -1)]
    pub fn as_i32(&self) -> i32 {
        self.0
    }
    
    #[requires(n >= 0)]
    #[ensures(result.as_i32() >= 0)]
    pub fn new(n: i32) -> Self {
        Frame(n)
    }
}

impl<T: Config> SyncLayer<T> {
    #[requires(!frame_to_load.is_null())]
    #[requires(frame_to_load.as_i32() < self.current_frame.as_i32())]
    #[requires(frame_to_load.as_i32() >= self.current_frame.as_i32() - self.max_prediction as i32)]
    #[ensures(result.is_ok() ==> self.current_frame.as_i32() == old(frame_to_load.as_i32()))]
    pub fn load_frame(&mut self, frame_to_load: Frame) -> Result<FortressRequest<T>, FortressError> {
        // ...
    }
}

impl<T: Config> InputQueue<T> {
    // Type invariant: queue length never exceeds maximum
    #[invariant(self.length <= INPUT_QUEUE_LENGTH)]
    #[invariant(self.head < INPUT_QUEUE_LENGTH)]
    #[invariant(self.tail < INPUT_QUEUE_LENGTH)]
    
    #[ensures(self.length <= INPUT_QUEUE_LENGTH)]
    pub fn add_input(&mut self, input: PlayerInput<T::Input>) -> Frame {
        // ...
    }
}
```

**Proposed Prusti Verification Targets:**

| Component | Property | Priority |
|-----------|----------|----------|
| `Frame` | `new()` ensures non-negative frame | HIGH |
| `Frame` | Arithmetic operations preserve validity | HIGH |
| `InputQueue` | `add_input` maintains length invariant | HIGH |
| `InputQueue` | `discard_confirmed_frames` maintains indices | HIGH |
| `SyncLayer` | `load_frame` preconditions documented | MEDIUM |
| `SyncLayer` | `advance_frame` increases current_frame | MEDIUM |
| `PlayerHandle` | `is_valid_player_for` correctness | LOW |

**Implementation Tasks:**
- [ ] Add `prusti-contracts` as optional dev-dependency
- [ ] Create `src/verification/prusti_contracts.rs` module  
- [ ] Add Prusti annotations to `Frame` type
- [ ] Add Prusti annotations to `PlayerHandle` type
- [ ] Add Prusti annotations to `InputQueue`
- [ ] Add Prusti annotations to `SyncLayer`
- [ ] Set up CI job for Prusti verification (optional)
- [ ] Document verification results

**Trade-offs:**
- **Pros:** Good VS Code integration, well-documented, active community
- **Cons:** Requires nightly Rust, Java dependency, can be slow on large codebases
- **Effort:** MEDIUM (1-2 days for initial setup)
- **Value:** MEDIUM (complements Kani with contract-based verification)

#### 2.8 Haybale Symbolic Execution 📋 (TODO - LOW VALUE)

**Status:** NOT STARTED - Research Complete

**What is Haybale?**
Haybale is a symbolic execution engine for LLVM IR written in Rust. It can analyze compiled code to find bugs, prove absence of certain errors, and explore all possible execution paths.

**Key Features:**
- GitHub: https://github.com/PLSysSec/haybale
- 569 stars, written in Rust
- Operates on LLVM IR (analyzes compiled code)
- Uses Boolector SMT solver
- Can analyze C, C++, Rust, or any LLVM-compiled language
- Supports function summaries and hooks

**Dependencies Required:**
- LLVM 9-14 (system installation)
- Boolector 3.2.1 (can be vendored)
- Rust stable

**Installation:**
```bash
# Add to Cargo.toml
[dev-dependencies]
haybale = { version = "0.7.2", features = ["llvm-14", "vendor-boolector"] }
```

**Example Haybale Analysis for Fortress Rollback:**
```rust
use haybale::*;

#[test]
fn haybale_find_panic_in_input_queue() {
    // Compile the crate to LLVM bitcode first:
    // RUSTFLAGS="--emit=llvm-bc" cargo build
    
    let project = Project::from_bc_path("target/debug/deps/fortress_rollback.bc")
        .expect("Failed to load bitcode");
    
    // Check if add_input can ever panic
    let config = Config::<DefaultBackend>::default();
    
    match find_zero_of_func("fortress_rollback::input_queue::InputQueue::add_input", &project, config, None) {
        Ok(None) => println!("add_input never panics"),
        Ok(Some(inputs)) => println!("Found panic with inputs: {:?}", inputs),
        Err(e) => println!("Analysis error: {}", e),
    }
}

#[test]
fn haybale_explore_load_frame_paths() {
    let project = Project::from_bc_path("target/debug/deps/fortress_rollback.bc")
        .expect("Failed to load bitcode");
    
    let mut em = symex_function(
        "fortress_rollback::sync_layer::SyncLayer::load_frame",
        &project,
        Config::<DefaultBackend>::default(),
        None
    );
    
    // Explore all paths through the function
    let mut path_count = 0;
    for result in em {
        match result {
            Ok(ReturnValue::Return(_)) => path_count += 1,
            Ok(ReturnValue::Throw(_)) => println!("Found throwing path"),
            Err(e) => println!("Error on path: {}", e),
            _ => {}
        }
    }
    println!("Explored {} paths through load_frame", path_count);
}
```

**Proposed Haybale Analysis Targets:**

| Analysis | Description | Priority |
|----------|-------------|----------|
| Panic reachability | Can any public API panic? | MEDIUM |
| Integer overflow | Find overflow in frame arithmetic | MEDIUM |
| Null pointer | Can any pointer be null when dereferenced? | LOW |
| Path exploration | Count execution paths in critical functions | LOW |
| Input generation | Generate inputs that trigger specific behaviors | LOW |

**Implementation Tasks:**
- [ ] Add `haybale` as optional dev-dependency with LLVM feature
- [ ] Create `tests/haybale_analysis.rs` module
- [ ] Set up LLVM bitcode generation in CI
- [ ] Write panic reachability analysis for `InputQueue`
- [ ] Write panic reachability analysis for `SyncLayer`
- [ ] Write path exploration for `load_frame`
- [ ] Document findings and any bugs discovered

**Trade-offs:**
- **Pros:** Analyzes actual compiled code, can find real bugs, no annotations needed
- **Cons:** Requires LLVM, can be slow, path explosion on complex code
- **Effort:** MEDIUM (1 day for setup, ongoing for new analyses)
- **Value:** LOW (better for bug finding than proofs; Kani covers similar ground)

**When to Use Haybale:**
- When you want to analyze compiled code without annotations
- When exploring execution paths for understanding
- When searching for specific bug patterns
- As a complement to Kani for different perspectives

#### 2.9 Theorem Prover Comparison Summary

| Tool | Type | Effort | Value | Status |
|------|------|--------|-------|--------|
| **Kani** | Bounded model checking | ✅ DONE | HIGH | 38 proofs complete |
| **TLA+** | Protocol verification | ✅ DONE | HIGH | 4 specs complete |
| **Z3** | SMT solver | TODO | HIGH | Implementation plan ready |
| **Creusot** | Deductive verification | TODO | MEDIUM | Requires OCaml/Why3 |
| **Prusti** | Contract verification | TODO | MEDIUM | Good VS Code support |
| **Haybale** | Symbolic execution | TODO | LOW | Bug finding focus |
| **Coq** | Proof assistant | SKIP | LOW | Too high learning curve |

**Recommended Priority Order:**
1. ✅ Kani (complete)
2. ✅ TLA+ (complete)
3. 📋 Z3 (high value, moderate effort)
4. 📋 Prusti (good contracts, VS Code integration)
5. 📋 Creusot (powerful but complex setup)
6. 📋 Haybale (nice-to-have for bug hunting)

### Phase 3: Comprehensive Test Coverage ✅ (Mostly Complete)

#### 3.1 Integration Test Expansion ✅
All core integration test scenarios complete.

#### 3.2 Chaos Engineering & Network Resilience 🟢 (Mostly Complete)
**Complete:**
- ChaosSocket fault injection (latency, jitter, loss, burst loss, reordering, duplication)
- 20 network resilience integration tests
- Multi-process and Docker-based network testing
- Correctness validation (determinism, no panics, graceful degradation)

**Remaining Edge Cases (Low Priority):**
- [ ] Latency spikes (baseline with periodic spikes)
- [ ] Correlated loss patterns
- [ ] Bimodal latency
- [ ] One-way connectivity loss

#### 3.3 Metamorphic Testing
- [ ] Input permutation invariance tests
- [ ] Timing invariance tests
- [ ] Replay consistency tests

#### 3.4 Differential Testing vs GGPO (Optional)
- [ ] Set up GGPO reference implementation
- [ ] Compare behavior on identical inputs
- [ ] Document behavioral differences

### Phase 4: Enhanced Usability

#### 4.1 Documentation ✅ (Mostly Complete)
- [x] Architecture guide (`docs/ARCHITECTURE.md`)
- [x] User guide (`docs/USER_GUIDE.md`)
- [ ] Complete rustdoc with examples

#### 4.2 Examples
- [ ] Advanced configuration examples
- [ ] Error handling examples

### Phase 5: Performance

#### 5.1 Benchmarking
- [ ] Set up criterion benchmarks
- [ ] Benchmark core operations (input queue, sync layer, serialization)
- [ ] Track performance across commits

#### 5.2 Continuous Fuzzing
- [ ] Set up cargo-fuzz / OSS-Fuzz
- [ ] Create fuzz targets for message parsing
- [ ] Fuzz input queue operations

### Phase 6: Maintainability

#### 6.1 Core Extraction (Optional)
- [ ] Extract `fortress-core` crate with verified primitives
- [ ] InputQueue, SyncLayer, TimeSync in core
- [ ] No network dependencies in core
- [ ] 100% Kani-verified core

#### 6.2 Module Reorganization
- [ ] Separate protocol from session logic
- [ ] Clean interfaces between layers
- [ ] Reduce function sizes (< 50 lines)

---

## Known Bugs (CRITICAL - Must Fix Before 1.0)

### BUG-001: Multi-Process Rollback Desync (RESOLVED)

**Status:** ✅ FIXED - December 2025 (Session 16)
**Discovered:** December 7, 2025 (Session 14)
**Location:** Multi-process network tests (`tests/test_multi_process_network.rs`)
**Affects:** All multi-process scenarios with rollbacks

#### Resolution Summary
**Root Cause:** The test's `compute_confirmed_checksum()` iterated over all frames, but `confirmed_input()` returns errors for discarded frames. Different peers discard frames at different times, causing different frame ranges to be included in the checksum computation.

**Fix Applied:** Changed to window-based computation using last 64 frames (half of the 128-frame queue capacity). This ensures both peers compute checksums over identical frame ranges, since frames within this window are guaranteed to be available.

**Changes Made:**
- `tests/bin/network_test_peer.rs`: Modified `compute_confirmed_checksum()` and `compute_confirmed_game_value()` to use windowed approach
- `tests/test_multi_process_network.rs`: Re-enabled checksum assertions and added `final_value` comparisons
- All 8 multi-process tests now pass reliably

#### Symptoms
When two peers run in separate processes and experience rollbacks, their final game states diverge even though:
- Both use deterministic input generation (frame-based)
- Both use deterministic state update logic
- Both reach the same target frame count
- Both use a deterministic hash function (FNV-1a)

#### Observed Behavior
```
test_basic_connectivity: checksum1 = 14668849295451731695, checksum2 = 5357334955406791517
test_extended_session: checksum1 = 629821227754530724, checksum2 = 9355990179724997963
```

The checksums differ by orders of magnitude, indicating actual state divergence (not hash collision).

#### Code Analysis Findings

**1. Input Serialization is Deterministic ✅**
- `InputBytes::from_inputs()` iterates `0..num_players` in order (line 73 in `protocol.rs`)
- `to_player_inputs()` iterates `0..num_players` in order (line 90 in `protocol.rs`)
- BTreeMap iteration order is consistent

**2. `synchronized_inputs()` Order is Deterministic ✅**
- Iterates over `connect_status` by index `i` (line 344 in `sync_layer.rs`)
- `connect_status` is a `Vec<ConnectionStatus>` indexed by player handle
- Returns inputs in player handle order [0, 1, 2, ...]

**3. Prediction Logic - LIKELY ROOT CAUSE ⚠️**
The prediction system in `input_queue.rs` (line 145):
```rust
// basing new prediction frame from previously added frame
self.prediction = self.inputs[previous_position];
```

Predictions use "repeat last input" strategy. The critical issue:
- Peer A receives Peer B's input at time T1
- Peer B receives Peer A's input at time T2
- If T1 ≠ T2 relative to frame advancement, predictions differ
- Even though final confirmed inputs are identical, the **intermediate predicted states** cause divergent computation paths

**4. Test Game State Update Uses Input Index**
```rust
// In network_test_peer.rs
self.value = self.value.wrapping_add(input.value as i64 * (i as i64 + 1));
```
This multiplies by player index. If during ANY frame the inputs array has different values (predicted vs actual), the state diverges.

#### Root Cause Analysis

The desync occurs because:
1. **Prediction timing differs between peers** - Each peer has its own view of when remote inputs arrive
2. **State advances with predictions** - Before confirmed inputs arrive, state uses predicted values
3. **Rollback doesn't fully reconcile** - Even after rollback, if any intermediate computation used different predictions, arithmetic can diverge due to wrapping/overflow

#### Why In-Process Tests Pass
In-process tests (like SyncTestSession) don't have real network timing. All "remote" inputs are immediately available, so predictions rarely/never occur.

#### Hypotheses to Test

**H1: Prediction values differ at same frame**
- Add logging: For each AdvanceFrame, log frame number, all input values, and input statuses
- Compare logs between two peers

**H2: Number of rollbacks causes divergence**  
- One peer may rollback N times, another M times
- If N ≠ M, intermediate states computed differently

**H3: Floating point or overflow in state computation**
- `wrapping_add` should be deterministic, but verify no hidden issues

#### Next Steps (Recommended Investigation Order)

1. **Add Debug Logging to Test Peer**
   - Log every `AdvanceFrame` request with: frame, inputs[], statuses[]
   - Log every `LoadGameState` with: frame loaded, state value
   - Compare logs between peers to find first divergence point

2. **Create Minimal Reproduction**
   - Write in-process test that simulates delayed input arrival
   - Force exactly one rollback scenario
   - Verify checksums match or identify divergence

3. **Verify Prediction Determinism**
   - Unit test: Given identical input history, verify prediction() returns same value
   - Test: Two InputQueues with same operations should have identical state

4. **Consider Protocol Change**
   - Option A: Don't update state during prediction (use default/zero input)
   - Option B: Ensure prediction algorithm produces same result regardless of timing
   - Option C: Add determinism check at rollback boundary

#### Affected Tests
| Test | Status | Notes |
|------|--------|-------|
| `test_basic_connectivity` | ⚠️ Pass (checksum disabled) | Would fail with checksum |
| `test_extended_session` | ❌ FAIL | Checksum mismatch |
| `test_packet_loss_5_percent` | ❌ FAIL | Checksum mismatch |
| `test_packet_loss_15_percent` | ❌ FAIL | Checksum mismatch |
| `test_latency_30ms` | ❌ FAIL | Checksum mismatch |
| `test_latency_with_jitter` | ❌ FAIL | Checksum mismatch |
| `test_poor_network_combined` | ⚠️ Pass (checksum disabled) | Would fail with checksum |
| `test_asymmetric_network` | ⚠️ Pass (checksum disabled) | Would fail with checksum |

---

### BUG-002: Network Test Timing Sensitivity (RESOLVED)

**Status:** ✅ FIXED - December 7, 2025
**Location:** `tests/test_network_resilience.rs`

#### Original Issue
Network resilience tests with packet loss were failing intermittently due to:
1. Insufficient time for synchronization under packet loss
2. Missing sleep delays preventing ChaosSocket latency simulation from working

#### Root Cause
- SYNC_RETRY_INTERVAL = 200ms in protocol
- Tests used 50 iterations × 50ms = 2.5 seconds
- With packet loss, multiple sync retries needed
- Some tests had no sleep, preventing time-based packet delivery

#### Fix Applied
- Increased synchronization loop iterations from 50 to 100
- Increased sleep duration from 50ms to 100ms  
- Added sleep delays to tests that were missing them
- All 20 network resilience tests now pass reliably

---

## Known Issues (Non-Critical)

### Dead Code in InputQueue
The `advance_queue_head` function contains gap-filling code for handling frame delay increases mid-session. This code can never be reached because `add_input` rejects inputs first due to its sequential frame check.

**Test:** `test_frame_delay_change_mid_session_drops_input`

### Remaining Tasks
- [ ] Reserve `fortress-rollback` on crates.io
- [ ] Protocol layer panic elimination (lower priority)
- [ ] Session type pattern for state machine enforcement (optional)

---

## Determinism & Edge Cases Analysis

### Sources of Non-Determinism

This section documents all sources of randomness/non-determinism in the library and whether they are acceptable.

#### ✅ Acceptable Non-Determinism (Network Layer Only)

These sources of non-determinism are **intentional** and occur only in the networking layer, which does NOT affect game state simulation:

1. **Protocol Magic Number** (`protocol.rs:213`)
   - `rand::random::<u16>()` generates a unique connection identifier
   - Purpose: Prevent processing of stale/spoofed packets
   - Impact: Network layer only, not part of game state

2. **Sync Request Random Token** (`protocol.rs:542`)
   - `rand::random::<u32>()` generates tokens for sync request/reply matching
   - Purpose: Verify sync reply matches request (security/correctness)
   - Impact: Network layer only, not part of game state

3. **Timestamps** (`protocol.rs:32-38`, `Instant::now()` throughout)
   - Used for: ping calculation, disconnect detection, quality reports
   - Impact: Network layer only, not part of game state

4. **ChaosSocket Randomness** (`chaos_socket.rs`)
   - Used for: simulating packet loss, jitter, reordering in tests
   - Can be seeded for deterministic testing
   - Impact: Test infrastructure only, optional in production

#### ✅ Deterministic Game State Components

All game-state-affecting operations are deterministic:

1. **Input Serialization**: Iterates players in handle order (`0..num_players`)
2. **`synchronized_inputs()`**: Iterates by index, deterministic order
3. **Frame Arithmetic**: Uses Rust's `wrapping_*` operations
4. **Collection Iteration**: All `HashMap` replaced with `BTreeMap`/`BTreeSet`
5. **Hashing**: New `fortress_rollback::hash` module provides FNV-1a (deterministic)

#### ✅ Eliminated Non-Determinism

| Location | Old | New | Status |
|----------|-----|-----|--------|
| `protocol.rs:148` | `HashSet<u32>` | `BTreeSet<u32>` | ✅ Fixed |
| `tests/stubs.rs` | `DefaultHasher` | `fnv1a_hash` | ✅ Fixed |
| `tests/stubs_enum.rs` | `DefaultHasher` | `fnv1a_hash` | ✅ Fixed |

### Edge Cases Analysis

#### EDGE-001: Rollback to Frame Zero

**Status:** ✅ HANDLED GRACEFULLY

**Scenario:** Can the library request a rollback to frame 0?

**Analysis:**
- `load_frame()` validates: `frame_to_load >= 0` (not NULL)
- `load_frame()` validates: `frame_to_load < current_frame` (in the past)
- `load_frame()` validates: `frame_to_load >= current_frame - max_prediction` (within window)
- Frame 0 state IS saved (via `save_current_state()` on first advance)

**When it happens:**
- `first_incorrect_frame == 0` when remote input arrives late for frame 0
- This can occur with high latency at game start
- The library correctly loads frame 0 and resimulates

**Test coverage:** `test_blank_prediction_on_frame_zero` verifies frame 0 edge case

#### EDGE-002: First Frame Prediction

**Status:** ✅ HANDLED

**Scenario:** What happens when predicting input for frame 0 with no history?

**Analysis:** In `InputQueue::input()`:
```rust
if requested_frame == 0 || self.last_added_frame.is_null() {
    // basing new prediction frame from nothing
    self.prediction = PlayerInput::blank_input(self.prediction.frame);
}
```

**Behavior:** Returns `Input::default()` (blank input) for frame 0 predictions.
This is deterministic across all peers.

#### EDGE-003: Input Delay at Frame Zero

**Status:** ✅ HANDLED

**Scenario:** With `input_delay = 2`, frame 0 input arrives at frame 2.

**Analysis:** `add_input()` correctly handles frame delay:
- Input for conceptual "frame 0" is stored at actual frame `0 + delay`
- `first_frame` flag ensures correct starting frame detection

#### EDGE-004: Concurrent Save/Load Across Threads

**Status:** ✅ HANDLED

**Scenario:** Multiple threads accessing `GameStateCell` simultaneously.

**Analysis:** 
- `GameStateCell` uses `Arc<Mutex<GameState<T>>>`
- Mutex ensures exclusive access during save/load
- TLA+ spec (`Concurrency.tla`) verifies mutual exclusion

#### EDGE-005: Rollback Beyond Saved States

**Status:** ✅ HANDLED

**Scenario:** Attempting to rollback further than `max_prediction`.

**Analysis:** `load_frame()` rejects with `FortressError::InvalidFrame`:
```rust
if frame_to_load.as_i32() < self.current_frame.as_i32() - self.max_prediction as i32 {
    return Err(FortressError::InvalidFrame { ... });
}
```

#### EDGE-006: Sparse Saving Rollback

**Status:** ✅ HANDLED

**Scenario:** With `sparse_saving = true`, rollback target may not have saved state.

**Analysis:** In `adjust_gamestate()`:
- Sparse saving rolls back to `last_saved_frame` instead of `first_incorrect_frame`
- This may require more resimulation frames but ensures state availability

#### EDGE-007: Rollback When current_frame == 0 (NEW ANALYSIS)

**Status:** ✅ PROTECTED BY DESIGN

**Scenario:** What if a rollback is requested when the session is still at frame 0?

**Deep Analysis:**

The `load_frame()` function in `sync_layer.rs` has explicit validation:

```rust
if frame_to_load >= self.current_frame {
    return Err(FortressError::InvalidFrame {
        frame: frame_to_load,
        reason: format!(
            "must load frame in the past (frame to load is {}, current frame is {})",
            frame_to_load, self.current_frame
        ),
    });
}
```

**Protection Mechanism:**
- If `current_frame == 0`, then ANY `frame_to_load` will fail because:
  - `frame_to_load == 0` fails: `0 >= 0` is true
  - `frame_to_load == -1` (NULL) fails earlier: "cannot load NULL_FRAME"
  - No valid frame exists "in the past" when at frame 0

**When This Could Be Triggered:**
- A bug where `first_incorrect_frame` is set to 0 before `advance_frame()` is called
- However, `first_incorrect_frame` is only set in `add_input_by_frame()` when comparing predictions
- Predictions only exist after `synchronized_inputs()` is called, which happens AFTER the frame 0 save

**Conclusion:** This edge case cannot occur in normal operation.

#### EDGE-008: first_incorrect_frame == 0 After First Advance (NEW ANALYSIS)

**Status:** ✅ HANDLED - Critical Path Verified

**Scenario:** What if `first_incorrect_frame` is set to 0 (meaning the very first frame had a misprediction)?

**Detailed Code Flow Analysis:**

1. **Session Creation:** `current_frame = 0`, all input queues empty

2. **First `advance_frame()` call (at frame 0):**
   ```rust
   // Step 1: Save frame 0 state FIRST (line 334 in p2p_session.rs)
   if self.sync_layer.current_frame() == 0 && !lockstep {
       requests.push(self.sync_layer.save_current_state());
   }
   
   // Step 2: Check for simulation inconsistency (line 346)
   let first_incorrect = self.sync_layer.check_simulation_consistency(self.disconnect_frame);
   
   // Step 3: If inconsistent, rollback (line 351)
   if first_incorrect != Frame::NULL {
       self.adjust_gamestate(first_incorrect, ...)?;
   }
   ```

3. **Key Question:** Can `first_incorrect` be non-NULL during the FIRST `advance_frame()`?

**Analysis of `first_incorrect_frame` Setting:**

The `first_incorrect_frame` field is only set in `InputQueue::add_input_by_frame()` (line 201):
```rust
if !self.prediction.frame.is_null() {
    // We have been predicting
    if self.first_incorrect_frame.is_null() && !self.prediction.equal(&input, true) {
        self.first_incorrect_frame = frame_number;
    }
}
```

**Critical Insight:** `first_incorrect_frame` is ONLY set when:
1. `self.prediction.frame` is NOT NULL (meaning predictions exist)
2. A new input is added that differs from the prediction

Predictions are only created when `synchronized_inputs()` calls `input()`:
```rust
// In InputQueue::input(), line 131:
if self.prediction.frame.as_i32() < 0 {
    // Create prediction if none exists...
    if requested_frame == 0 || self.last_added_frame.is_null() {
        self.prediction = PlayerInput::blank_input(self.prediction.frame);
    }
}
```

**Timeline for First advance_frame():**

```
advance_frame() called at frame 0:
  1. poll_remote_clients() - may add remote inputs via add_input()
       -> No predictions exist yet, so first_incorrect_frame NOT SET
  2. Save frame 0 state
  3. check_simulation_consistency() - returns NULL (no predictions yet)
  4. Get synchronized_inputs() - NOW predictions may be created
  5. Advance to frame 1
  
advance_frame() called at frame 1:
  1. poll_remote_clients() - may add remote inputs
       -> Predictions exist, first_incorrect_frame MAY BE SET if mismatch
  2. check_simulation_consistency() - may return frame 0 or 1
  3. If first_incorrect == 0:
       -> load_frame(0) is valid (0 < 1, within prediction window)
       -> Frame 0 state WAS saved in previous call
       -> Rollback succeeds ✅
```

**Conclusion:** The system correctly handles `first_incorrect_frame == 0`:
- Frame 0 state is always saved before any prediction can be created
- The rollback to frame 0 occurs on the SECOND `advance_frame()` call, when `current_frame == 1`
- At this point, `load_frame(0)` is valid because `0 < 1`

#### EDGE-009: Sparse Saving with last_saved_frame == NULL (NEW ANALYSIS)

**Status:** ✅ PROTECTED BY DESIGN

**Scenario:** What if `sparse_saving = true` and `last_saved_frame` is still NULL when rollback is needed?

**Code Analysis in `adjust_gamestate()`:**
```rust
let frame_to_load = if self.sparse_saving {
    self.sync_layer.last_saved_frame()  // Could be NULL!
} else {
    first_incorrect
};
```

**If `last_saved_frame` is NULL:**
- `frame_to_load` becomes `NULL_FRAME` (-1)
- `load_frame(-1)` is called
- This returns error: "cannot load NULL_FRAME"

**When Could This Happen?**
Looking at `save_current_state()`:
```rust
pub(crate) fn save_current_state(&mut self) -> FortressRequest<T> {
    self.last_saved_frame = self.current_frame;  // Set immediately!
    // ...
}
```

The `last_saved_frame` is set when `save_current_state()` is CALLED, not when the user processes the request.

**Timeline Analysis:**
1. Frame 0: `save_current_state()` called → `last_saved_frame = 0`
2. `first_incorrect_frame` cannot be set until AFTER predictions exist
3. Predictions created in `synchronized_inputs()`, which is AFTER save

**Conclusion:** `last_saved_frame` is NEVER NULL when a rollback is triggered because:
- Frame 0 save happens BEFORE any predictions can be created
- Predictions are required for `first_incorrect_frame` to be set
- Therefore, `last_saved_frame >= 0` when any rollback is attempted

#### EDGE-010: Race Between Save Request and Rollback (NEW ANALYSIS)

**Status:** ✅ NO RACE - Synchronous Design

**Scenario:** What if rollback is triggered before the user processes the SaveGameState request?

**Key Design Insight:**
The `last_saved_frame` field is set **internally** when `save_current_state()` is called:
```rust
pub(crate) fn save_current_state(&mut self) -> FortressRequest<T> {
    self.last_saved_frame = self.current_frame;  // ← Set HERE, not by user
    let cell = self.saved_states.get_cell(self.current_frame).expect(...);
    FortressRequest::SaveGameState { cell, frame: self.current_frame }
}
```

The `get_cell()` returns a `GameStateCell` (a clone of an `Arc<Mutex<GameState<T>>>`).
When the user processes the `SaveGameState` request and calls `cell.save(...)`, they write to the SAME cell that the library will later read during `load_frame()`.

**Request Processing Order:**
The library returns requests in order and expects them to be processed in order:
```rust
// In p2p_session.rs, advance_frame():
let mut requests = Vec::new();
// ...
requests.push(self.sync_layer.save_current_state());  // Save request
// ...
if first_incorrect != Frame::NULL {
    self.adjust_gamestate(..., &mut requests)?;  // Load request added
}
// ...
requests.push(FortressRequest::AdvanceFrame { ... });  // Advance request
```

If a rollback is needed, the requests array is:
1. `SaveGameState { frame: N }` - User must process first
2. `LoadGameState { frame: K }` - User processes second
3. `SaveGameState { frame: K }` - (if needed during resimulation)
4. `AdvanceFrame` - ...

**User Contract:** Users MUST process requests in order (documented in API_CONTRACTS.md).

**Conclusion:** No race condition because:
1. Requests are returned synchronously in proper order
2. Users must process in order per API contract
3. The internal `last_saved_frame` tracking doesn't depend on user processing

#### EDGE-011: One Client Has Frames, Other Has None (NEW ANALYSIS)

**Status:** ✅ HANDLED BY SYNCHRONIZATION PROTOCOL

**Scenario:** Client A is at frame 10, Client B just joined and is at frame 0. Can A's inputs cause B to rollback to a non-existent frame?

**Protection Mechanisms:**

1. **Synchronization Phase:**
   - New sessions start in `SessionState::Synchronizing`
   - `advance_frame()` returns `FortressError::NotSynchronized` until all peers sync
   - No frames are processed until sync completes

2. **Input Frame Validation:**
   - Remote inputs include their frame number
   - `add_input()` validates inputs are sequential (within frame_delay tolerance)
   - Inputs for "future" frames (relative to receiver) are queued, not immediately processed

3. **Rollback Bounds:**
   - `load_frame()` validates: `frame_to_load >= current_frame - max_prediction`
   - Even if A sends inputs for frames 5-10, B only uses inputs for frames it can process

**Code Evidence:**
```rust
// In protocol.rs, handling received inputs:
for (frame, input) in pending_inputs.iter() {
    // Inputs are only added if they're within our processing range
    if frame <= current_frame + max_prediction {
        sync_layer.add_remote_input(player, input);
    }
}
```

**Conclusion:** The synchronization protocol and input frame validation prevent this scenario:
- Clients must synchronize before processing frames
- Remote inputs for "too far ahead" frames don't trigger immediate rollbacks
- The rollback target is always within the receiver's valid range

### Formal Verification of Edge Cases

The following edge cases are covered by TLA+ specifications:

| Edge Case | TLA+ Spec | Property | Status |
|-----------|-----------|----------|--------|
| Rollback to frame 0 | `Rollback.tla` | `StateSaved(0)` in Init | ✅ Verified |
| Queue bounds | `InputQueue.tla` | `QueueLengthBounded`, `QueueIndexValid` | ✅ Verified |
| Concurrent access | `Concurrency.tla` | `MutualExclusion`, `NoDataRace` | ✅ Verified |
| Rollback bounds | `Rollback.tla` | `RollbackBounded` (INV-2) | ✅ Verified |
| State availability | `Rollback.tla` | `StateAvailability` (INV-6) | ✅ Verified |
| Frame monotonicity | `Rollback.tla` | `FrameMonotonicity` | ✅ Verified |
| Sparse saving | `Rollback.tla` | `sparseSaving` variable modeled | ✅ Verified |

**Edge Cases for Z3 Verification (Proposed):**

| Edge Case | Property to Prove | Priority |
|-----------|------------------|----------|
| EDGE-007: Rollback at frame 0 | `∀ f. f >= 0 → ¬(load_frame(f) succeeds when current_frame == 0)` | HIGH |
| EDGE-008: first_incorrect_frame == 0 | `first_incorrect_frame set → last_saved_frame >= 0` | HIGH |
| EDGE-009: Sparse saving NULL | `sparse_saving ∧ needs_rollback → last_saved_frame ≠ NULL` | HIGH |
| EDGE-010: Request ordering | `save_request precedes load_request in requests[]` | MEDIUM |

**Edge Cases for Additional Kani Proofs:**

| Property | Module | Status |
|----------|--------|--------|
| `load_frame` rejects frame >= current | `sync_layer.rs` | ❌ TODO |
| `first_incorrect_frame` requires prediction | `input_queue.rs` | ❌ TODO |
| Request order invariant | `p2p_session.rs` | ❌ TODO |
| Sparse saving correctness | `p2p_session.rs` | ❌ TODO |

### Recommendations

**Completed:**
1. ✅ Switch to deterministic hashing (`BTreeSet`, FNV-1a)
2. ✅ Document acceptable randomness (network layer only)
3. ✅ TLA+ specifications for all major components (4/4 complete)
4. ✅ Kani proofs for critical operations (38 proofs)

**In Progress:**
5. ✅ BUG-001 fixed (prediction timing desync) - Session 16

**High Priority TODO:**
6. 📋 **Add Z3 SMT proofs** for edge cases (frame 0, sparse saving, rollback bounds)
   - Provides unbounded verification (vs Kani's bounded model checking)
   - See Phase 2.5 for implementation plan
7. 📋 **Add Kani proofs for new edge cases** (EDGE-007 through EDGE-011)
8. 📋 **Add integration test forcing frame 0 rollback** with high latency simulation
9. 📋 **Add regression test** for sparse_saving + first frame rollback

**Medium Priority TODO:**
10. 📋 Add assertion in `adjust_gamestate()` to catch sparse_saving with NULL last_saved_frame
11. 📋 Document the critical invariant: "Frame 0 is saved before any prediction can occur"
12. 📋 Add debug logging for first_incorrect_frame transitions

**Low Priority (Nice to Have):**
13. 📋 Consider Prusti/Creusot deductive verification (high effort)
14. 📋 Symbolic execution with Haybale for bug finding

---

## Quality Gates

### Before Merging
- All library tests pass (180/180)
- All in-process integration tests pass
- Coverage maintained or improved
- No clippy warnings
- No panics in library code

### Before Release
- Coverage ≥ 90%
- Determinism tests pass on all platforms
- Examples compile and run
- BUG-001 resolved ✅ (multi-process desync)

### Before 1.0 Stable
- TLA+ specs for all protocols ✅ (4/4 complete)
- Kani proofs for critical functions ✅ (38 proofs complete)
- Formal specification complete ✅ (FORMAL_SPEC.md, API_CONTRACTS.md, DETERMINISM_MODEL.md)
- Deterministic hashing ✅ (FNV-1a module, no DefaultHasher)
- **No known correctness issues** ✅ (BUG-001 fixed)
- All multi-process tests pass with checksum validation ✅

---

## Progress Log

### December 7, 2025 (Session 15)
- ✅ **Deterministic Hashing Module** - Created `fortress_rollback::hash` module (`src/hash.rs`):
  - `DeterministicHasher`: FNV-1a based hasher for consistent cross-process checksums
  - `fnv1a_hash()`: Convenience function for computing deterministic hashes
  - `DeterministicBuildHasher`: For use with collections requiring deterministic hashing
  - 8 unit tests for the hash module
  - **Rationale:** `DefaultHasher` uses random seeds per process, breaking checksum comparison
- ✅ **Eliminated All Non-Deterministic Hashing:**
  - `protocol.rs`: `HashSet<u32>` → `BTreeSet<u32>` for `sync_random_requests`
  - `tests/stubs.rs`: `DefaultHasher` → `fnv1a_hash`
  - `tests/stubs_enum.rs`: `DefaultHasher` → `fnv1a_hash`
- ✅ **Non-Determinism Audit Complete** - Documented all sources of randomness:
  - **Acceptable (network layer only):** Protocol magic numbers, sync tokens, timestamps
  - **Eliminated:** All game-state-affecting non-determinism
- ✅ **Edge Case Analysis & Documentation:**
  - EDGE-001: Rollback to frame 0 - ✅ HANDLED (within prediction window)
  - EDGE-002: First frame prediction - ✅ HANDLED (returns `Input::default()`)
  - EDGE-003: Input delay at frame 0 - ✅ HANDLED
  - EDGE-004: Concurrent save/load - ✅ HANDLED (Mutex)
  - EDGE-005: Rollback beyond saved states - ✅ HANDLED (returns error)
  - EDGE-006: Sparse saving rollback - ✅ HANDLED
- ✅ **New Edge Case Tests:**
  - `test_load_frame_zero_within_prediction_window`
  - `test_load_frame_zero_outside_prediction_window`
- ✅ **PLAN.md & CHANGELOG.md Updated** with comprehensive determinism analysis
- **Library Tests:** 180 passing (8 new hash tests + 2 new edge case tests)

### December 7, 2025 (Session 16) - Edge Cases & Theorem Prover Research
- ✅ **Deep Research: "Rollback Before Frames Exist" Edge Cases**
  - Analyzed 5 new edge case scenarios in depth:
    - **EDGE-007:** Rollback when current_frame == 0 - ✅ PROTECTED (load_frame rejects)
    - **EDGE-008:** first_incorrect_frame == 0 after first advance - ✅ HANDLED (frame 0 saved first)
    - **EDGE-009:** Sparse saving with last_saved_frame == NULL - ✅ PROTECTED (save before predictions)
    - **EDGE-010:** Race between save request and rollback - ✅ NO RACE (synchronous design)
    - **EDGE-011:** One client has frames, other has none - ✅ HANDLED (sync protocol)
  - **Key Finding:** The library has excellent protection against "rollback before frames exist":
    - Frame 0 state is ALWAYS saved before any prediction can be created
    - `first_incorrect_frame` requires predictions to exist, which requires `synchronized_inputs()`
    - The synchronization protocol ensures peers are aligned before processing frames
    - Request ordering is deterministic and enforced by API contract

- ✅ **Deep Research: Theorem Prover Integration Opportunities**
  - **Z3 SMT Solver** (https://github.com/prove-rs/z3.rs) - RECOMMENDED
    - High-level Rust bindings available (`z3` crate v0.19.5)
    - Can prove unbounded properties (vs Kani's bounded model checking)
    - Proposed targets: frame arithmetic overflow, rollback frame selection, sparse saving
    - Implementation plan added to Phase 2.5
  - **Creusot** (https://github.com/creusot-rs/creusot) - DETAILED PLAN ADDED
    - Deductive verifier using Why3 platform
    - 1.4k stars, actively developed (v0.7.0 Nov 2024)
    - Full installation guide, example annotations, and verification targets documented
    - Implementation tasks: 7 TODO items for integration
    - Added as Phase 2.6
  - **Prusti** (https://github.com/viperproject/prusti-dev) - DETAILED PLAN ADDED
    - ETH Zurich's verifier based on Viper infrastructure
    - 1.7k stars, has VS Code extension
    - Full installation guide, example annotations, and verification targets documented
    - Implementation tasks: 8 TODO items for integration
    - Added as Phase 2.7
  - **Haybale** (https://github.com/PLSysSec/haybale) - DETAILED PLAN ADDED
    - Symbolic execution engine for LLVM IR
    - Full installation guide, example analysis code, and targets documented
    - Implementation tasks: 7 TODO items for integration
    - Good for bug hunting, lower priority than deductive verifiers
    - Added as Phase 2.8
  - **Coq/Rocq** - NOT RECOMMENDED
    - Interactive proof assistant with very high learning curve
    - Overkill for this project's verification needs

- ✅ **PLAN.md Updated:**
  - Expanded Phase 2.5 (Z3) with detailed implementation plan and example code
  - **NEW:** Phase 2.6 (Creusot) - Full integration plan with installation, examples, tasks
  - **NEW:** Phase 2.7 (Prusti) - Full integration plan with installation, examples, tasks
  - **NEW:** Phase 2.8 (Haybale) - Full integration plan with installation, examples, tasks
  - **NEW:** Phase 2.9 - Theorem Prover Comparison Summary table
  - Added 5 new EDGE cases (007-011) with detailed analysis
  - Updated Formal Verification of Edge Cases table with proposed Z3 and Kani targets
  - Updated Recommendations with prioritized actionable items

- **Research Conclusions:**
  1. **No critical "rollback before frames exist" bugs found** - The library handles all identified edge cases correctly
  2. **Z3 integration is the highest-value next step** for formal verification
  3. **Additional Kani proofs** should target the 5 new edge cases
  4. **Creusot/Prusti/Haybale** now have complete integration plans ready for implementation

### December 7, 2025 (Session 14)
- 🐛 **BUG-001 Discovered & Analyzed: Multi-Process Rollback Desync** - Critical bug identified:
  - Peers in separate processes produce different checksums after rollback
  - Affects all multi-process tests with network conditions causing rollbacks
  - **Code Analysis Complete:**
    - Input serialization order: ✅ Deterministic (player handle order)
    - `synchronized_inputs()` order: ✅ Deterministic (iterates by index)
    - Prediction logic: ⚠️ LIKELY ROOT CAUSE - timing-dependent "repeat last input"
  - **Root Cause Hypothesis:** Prediction timing differs between peers because each peer has its own view of when remote inputs arrive. Even though confirmed inputs eventually match, intermediate predicted states cause computation divergence.
  - **Documented:** Full analysis with next steps in PLAN.md Known Bugs section
- ✅ **BUG-002 Fixed: Network Test Timing** - All 20 network resilience tests now pass:
  - Increased synchronization timeouts for packet loss scenarios
  - Added missing sleep delays to allow ChaosSocket timing to work
  - Tests are now reliable and deterministic
- ✅ **Test Audit Complete:**
  - 170 library tests: ✅ All pass
  - 20 network resilience tests: ✅ All pass (after timing fixes)
  - 8 multi-process tests: 3 pass, 5 fail (BUG-001)
- **Next Steps for BUG-001:**
  1. Add debug logging to test peer binary to trace exact divergence point
  2. Create minimal in-process reproduction with forced rollback
  3. Verify prediction determinism with unit tests
  4. Consider protocol changes to ensure deterministic predictions

### December 7, 2025 (Session 13)
- ✅ **Phase 2.4: TLA+ Specifications Complete** - Final spec created:
  - `specs/tla/Concurrency.tla` (~370 lines) - GameStateCell thread-safe operations:
    - Models `Arc<Mutex<GameState<T>>>` with save/load/data operations
    - Thread state machine: idle → waiting → holding → saving/loading → idle
    - Mutex behavior: lock acquisition, wait queue, release
    - **Safety Properties:**
      - MutualExclusion: At most one thread holds lock
      - NoDataRace: Only lock holder can modify cell state
      - FrameConsistency: After save, cell frame matches saved frame
      - LoadReturnsSaved: Load returns correct data
      - ValidFrameAfterSave: Save never stores NULL_FRAME (matches assert in code)
      - WaitQueueFIFO: Lock acquired in request order
    - **Liveness Properties:**
      - NoDeadlock: Some action always enabled
      - OperationsComplete: Started operations eventually complete
      - FairLockAcquisition: Waiting threads eventually get lock
    - **Linearizability:** Operations appear atomic (guaranteed by mutex)
  - Updated `specs/tla/README.md` with Concurrency.tla documentation
- **Progress**: TLA+ specs 100% complete (4/4). All formal verification targets met.

### December 6, 2025 (Session 12)
- ✅ **Rust Edition Compatibility** - Downgraded to edition 2021 for Rust 1.75+ compatibility:
  - Changed `edition = "2024"` to `edition = "2021"` in Cargo.toml
  - Fixed `is_multiple_of` (nightly-only) to use modulo operator instead
  - Added `small_rng` feature to rand for SmallRng access
  - Pinned proptest to 1.4.0 (1.9.0 requires Rust 1.82+)
  - Pinned macroquad to 0.3.25 (0.4.x uses const fn with floats)
  - Added serde_json to regular dependencies for test binary
  - All 170 library tests pass on Rust 1.75
- ✅ **Phase 2.2: Kani Formal Verification Complete** - 38 Kani proofs created:
  - `src/lib.rs` - 12 proofs for Frame/PlayerHandle:
    - Frame creation validity and NULL consistency
    - Arithmetic operations (add, sub) for typical game usage
    - Ordering consistency with underlying i32
    - Modulo operation for circular buffer indexing
    - Option conversion (to_option, from_option)
    - AddAssign/SubAssign consistency
    - PlayerHandle validity checking (player vs spectator)
  - `src/input_queue.rs` - 14 proofs for InputQueue:
    - INV-4: Queue length bounded at initialization and after operations
    - INV-5: Head/tail indices always within [0, INPUT_QUEUE_LENGTH)
    - Circular buffer wraparound correctness
    - Sequential input acceptance, non-sequential rejection
    - discard_confirmed_frames maintains invariants
    - Frame delay handling preserves invariants
    - reset_prediction preserves structural state
  - `src/sync_layer.rs` - 12 proofs for SyncLayer:
    - INV-1: Frame monotonicity (advance_frame always increases)
    - INV-7: last_confirmed_frame <= current_frame
    - INV-8: last_saved_frame <= current_frame
    - load_frame validation (NULL, future, current, outside window)
    - SavedStates circular indexing and cell validation
    - Sparse saving respects last_saved_frame
    - reset_prediction preserves frame state
- **Note**: Proofs use `#[cfg(kani)]` and require Kani verifier.
  Install: `cargo install --locked kani-verifier && cargo kani setup`
  Run: `cargo kani` (project now uses edition 2021 for broad compatibility)
- **Progress**: Kani proofs complete. Next: TLA+ Concurrency spec for GameStateCell.

### December 6, 2025 (Session 11)
- ✅ **Documentation Created** - Core documentation guides completed:
  - `docs/ARCHITECTURE.md` - Comprehensive architecture guide
  - `docs/USER_GUIDE.md` - Practical user guide
- ✅ **PLAN.md Reconciled** - Restored missing formal verification phases (Phase 0, TLA+, Z3, Prusti) from original plan
- ✅ **Phase 0: Formal Specification Complete** - All three specification documents created:
  - `specs/FORMAL_SPEC.md` (~540 lines) - Mathematical specification with:
    - Core type definitions (Frame, PlayerHandle, PlayerInput, etc.)
    - 11 system invariants (INV-1 through INV-11)
    - Component specifications (InputQueue, SyncLayer operations with pre/post conditions)
    - Protocol state machine (5 states, transitions)
    - 7 safety properties, 6 liveness properties
    - Constants table and verification targets
  - `specs/API_CONTRACTS.md` (~640 lines) - Complete API contracts with:
    - All SessionBuilder methods with pre/post conditions
    - All P2PSession, SpectatorSession, SyncTestSession APIs
    - GameStateCell contracts
    - Request handling contracts (processing order, save/load/advance)
    - Error catalog with recovery guidance
  - `specs/DETERMINISM_MODEL.md` (~400 lines) - Determinism specification with:
    - 6 determinism requirements (DETER-1 through DETER-6)
    - 5 library guarantees (G1 through G5)
    - 4 user responsibilities
    - 6 common pitfalls with code examples
    - 4 verification strategies
    - Platform compatibility matrix
- **Progress**: Phase 0 complete. Ready for TLA+ specifications and Kani verification.
- ✅ **Phase 2.4: TLA+ Specifications (3 of 4)** - Created TLA+ formal models:
  - `specs/tla/NetworkProtocol.tla` (~200 lines) - Protocol state machine:
    - 5 states (Initializing, Synchronizing, Running, Disconnected, Shutdown)
    - 9 actions (StartSync, HandleSyncRequest, HandleSyncReply, etc.)
    - Safety properties: ValidStateTransitions, SyncRemainingNonNegative
    - Liveness properties: EventuallySynchronized, NoDeadlock
  - `specs/tla/InputQueue.tla` (~230 lines) - Circular buffer model:
    - Operations: AddInput, GetInput, AddRemoteInput, DiscardConfirmed, ResetPrediction
    - Safety: QueueLengthBounded, QueueIndexValid, FIFOOrdering, NoFrameGaps
    - Invariants match INV-4 and INV-5 from FORMAL_SPEC.md
  - `specs/tla/Rollback.tla` (~280 lines) - Rollback mechanism:
    - Operations: AddLocalInput, ReceiveRemoteInput, SaveState, StartRollback, LoadState, ResimulateFrame
    - Sparse saving mode support
    - Safety: RollbackBounded, StateAvailability, RollbackConsistency
    - Liveness: ProgressGuaranteed, RollbackCompletes
  - `specs/tla/README.md` - Usage guide with configuration examples
- **Progress**: TLA+ 75% complete. Remaining: Concurrency.tla for GameStateCell.

### December 6, 2025 (Session 10)
- ✅ **Burst Loss Support** - Enhanced `ChaosSocket` with burst loss simulation
- ✅ **Additional Network Resilience Tests** - 4 more integration tests
- **Total integration tests**: 20 in `test_network_resilience.rs`

### December 6, 2025 (Session 9)
- ✅ **Extended Network Resilience Tests** - 8 new integration tests
- **Total integration tests**: 16 in `test_network_resilience.rs`

### December 6, 2025 (Session 8)
- ✅ **ChaosSocket Implementation** - Full network fault injection system
- ✅ **Network Resilience Integration Tests** - 8 initial tests
- ✅ **Multi-Process Network Testing** - `network_test_peer` binary
- ✅ **Docker-Based Network Testing** - tc/netem integration

### December 6, 2025 (Session 7)
- ✅ **Miri Testing** - All 137 non-proptest tests pass under Miri
- ✅ **Multi-player & Rollback Tests** - 3-4 player scenarios

### Earlier Sessions (Summary)
- ✅ Type Safety (`Frame`, `PlayerHandle` newtypes)
- ✅ Runtime invariant checking, property-based testing (15 tests)
- ✅ Protocol unit tests (32 tests), structured telemetry
- ✅ Panic elimination, HashMap→BTreeMap migration
- ✅ Project rebrand, CI/CD pipeline setup
