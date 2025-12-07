# Fortress Rollback Improvement Plan

**Version:** 2.1
**Last Updated:** December 6, 2025 (Session 12)
**Goal:** Transform Fortress Rollback into a production-grade, formally verified rollback networking library with >90% test coverage, absolute determinism guarantees, and exceptional usability.

---

## Current Status Summary

### Metrics
| Metric | Current | Target | Status |
|--------|---------|--------|--------|
| Library Tests | 170 | 100+ | ✅ Exceeded |
| Integration Tests | ~40 | 30+ | ✅ Exceeded |
| Est. Coverage | ~89% | >90% | 🔄 Close |
| Clippy Warnings (lib) | 2 | 0 | 🔄 (too_many_arguments) |
| Panics from Public API | 0 | 0 | ✅ |
| HashMap Usage | 0 | 0 | ✅ |
| Miri Clean | 137/137 | All | ✅ |
| TLA+ Specs | 3 | 4 | 🟢 75% complete |
| Kani Proofs | 38 | 3+ | ✅ Complete |
| Rust Edition | 2021 | - | ✅ Rust 1.75+ compatible |

### What's Complete ✅

- **Phase 1: Foundation & Safety** - Project rebrand, deterministic collections, panic elimination, structured telemetry, session observers, core unit tests, property-based testing (15 tests), runtime invariant checking, paranoid mode, CI/CD pipeline
- **Phase 1.6: Type Safety** - `Frame` newtype with arithmetic ops, `PlayerHandle` newtype with bounds checking
- **Phase 2.1: Miri Testing** - All 137 non-proptest library tests pass under Miri with no undefined behavior detected. Miri CI job added.
- **Phase 3.1: Integration Tests** - Multi-player (3-4 players), rollback scenarios (deep, frequent, with varying input delays), spectator synchronization
- **Phase 3.2: Network Resilience** - ChaosSocket fault injection, 20 network condition tests (latency, jitter, packet loss, burst loss, reordering, duplication, asymmetric conditions), multi-process and Docker-based network testing, correctness validation under stress
- **Phase 4.1: Documentation** - Architecture guide (`docs/ARCHITECTURE.md`), User guide (`docs/USER_GUIDE.md`)
- **Phase 0: Formal Specification** - Complete formal specs (`specs/FORMAL_SPEC.md`, `specs/API_CONTRACTS.md`, `specs/DETERMINISM_MODEL.md`)
- **Phase 2.4: TLA+ Specifications** - 3 of 4 TLA+ specs complete (NetworkProtocol, InputQueue, Rollback)
- **Phase 2.2: Kani Formal Verification** - 38 Kani proofs covering:
  - Frame arithmetic safety (SAFE-6): 12 proofs for Frame operations
  - InputQueue buffer bounds (INV-4, INV-5): 14 proofs for circular buffer operations
  - SyncLayer state consistency: 12 proofs for frame management and rollback
- **Rust Compatibility** - Downgraded to edition 2021 for Rust 1.75+ compatibility:
  - Fixed nightly-only features (`is_multiple_of`, const fn floats)
  - Pinned dependencies (proptest 1.4.0, macroquad 0.3.25)
  - All 170 library tests pass

### Next Priority Actions
1. **📐 TLA+ Concurrency Spec (MEDIUM)** - Remaining spec:
   - `specs/tla/Concurrency.tla` for GameStateCell
2. **📊 Benchmarking (MEDIUM)** - Performance baseline needed
3. **🧪 Loom Concurrency Testing (LOW)** - Verify GameStateCell thread safety

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

#### 2.4 TLA+ Specifications ✅ COMPLETE (3 of 4)
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

- [ ] **Concurrent State Access** (`specs/tla/Concurrency.tla`)
  - GameStateCell operations
  - Properties: No data races, linearizable operations

#### 2.5 Z3 Verification
- [ ] Set up Z3 SMT solver integration
- [ ] Verify frame arithmetic (no overflow/underflow)
- [ ] Verify input queue index calculations
- [ ] Verify rollback frame selection algorithm

#### 2.6 Prusti Contracts (Optional)
- [ ] Add Prusti pre/post conditions to critical functions
- [ ] Verify InputQueue operations
- [ ] Verify SyncLayer state transitions

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

## Known Issues

### Dead Code in InputQueue
The `advance_queue_head` function contains gap-filling code for handling frame delay increases mid-session. This code can never be reached because `add_input` rejects inputs first due to its sequential frame check.

**Test:** `test_frame_delay_change_mid_session_drops_input`

### Remaining Tasks
- [ ] Reserve `fortress-rollback` on crates.io
- [ ] Protocol layer panic elimination (lower priority)
- [ ] Session type pattern for state machine enforcement (optional)

---

## Quality Gates

### Before Merging
- All tests pass
- Coverage maintained or improved
- No clippy warnings
- No panics in library code

### Before Release
- Coverage ≥ 90%
- Determinism tests pass on all platforms
- Examples compile and run

### Before 1.0 Stable
- TLA+ specs for all protocols (3/4 complete, Concurrency.tla remaining)
- Kani proofs for critical functions ✅ (38 proofs complete)
- Formal specification complete ✅ (FORMAL_SPEC.md, API_CONTRACTS.md, DETERMINISM_MODEL.md)
- No known correctness issues

---

## Progress Log

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
