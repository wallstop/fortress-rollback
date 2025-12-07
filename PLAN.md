# Fortress Rollback Improvement Plan

**Version:** 2.7
**Last Updated:** December 2025
**Goal:** Transform Fortress Rollback into a production-grade, formally verified rollback networking library with >90% test coverage, absolute determinism guarantees, and exceptional usability.

---

## Current Status Summary

### Metrics
| Metric | Current | Target | Status |
|--------|---------|--------|--------|
| Library Tests | 180 | 100+ | ✅ Exceeded |
| Integration Tests | 74 | 30+ | ✅ Exceeded |
| Est. Coverage | ~89% | >90% | 🔄 Close |
| Clippy Warnings (lib) | 0 | 0 | ✅ Clean |
| Panics from Public API | 0 | 0 | ✅ |
| HashMap/HashSet Usage | 0 | 0 | ✅ |
| DefaultHasher Usage | 0 | 0 | ✅ |
| Miri Clean | 145/145 | All | ✅ |
| TLA+ Specs | 4 | 4 | ✅ Complete |
| Kani Proofs | 35 | 3+ | ✅ Complete |
| Rust Edition | 2021 | - | ✅ Rust 1.75+ compatible |
| Network Resilience Tests | 31/31 | 20 | ✅ Exceeded |
| Multi-Process Tests | 15/17 | 8 | ✅ Exceeded (2 ignored stress tests) |

### What's Complete ✅

- **Phase 0: Formal Specification** - `specs/FORMAL_SPEC.md`, `specs/API_CONTRACTS.md`, `specs/DETERMINISM_MODEL.md`
- **Phase 1: Foundation & Safety** - Project rebrand, deterministic collections, panic elimination, structured telemetry, session observers, core unit tests, property-based testing (15 tests), runtime invariant checking, paranoid mode, CI/CD pipeline
- **Phase 1.6: Type Safety** - `Frame` newtype with arithmetic ops, `PlayerHandle` newtype with bounds checking
- **Phase 1.7: Deterministic Hashing** - `fortress_rollback::hash` module with FNV-1a, all `HashSet` → `BTreeSet`
- **Phase 2.1: Miri Testing** - All 145 non-proptest library tests pass under Miri
- **Phase 2.2: Kani Formal Verification** - 35 Kani proofs covering Frame arithmetic, InputQueue bounds, SyncLayer consistency
- **Phase 2.4: TLA+ Specifications** - 4 specs complete (NetworkProtocol, InputQueue, Rollback, Concurrency)
- **Phase 3.1: Integration Tests** - Multi-player (3-4 players), rollback scenarios, spectator synchronization
- **Phase 3.2: Network Resilience** - ChaosSocket fault injection, 31 network condition tests
- **Phase 3.2b: Multi-Process Network Testing** - 17 multi-process tests with real UDP sockets
- **Phase 4.1: Documentation** - `docs/ARCHITECTURE.md`, `docs/USER_GUIDE.md`
- **Rust Compatibility** - Edition 2021 for Rust 1.75+ compatibility

### Next Priority Actions

| Priority | Task | Effort | Value | Status |
|----------|------|--------|-------|--------|
| **HIGH** | Phase 7.2: Sync Protocol Configurability | MEDIUM | HIGH | 📋 TODO |
| **HIGH** | Phase 7.3: Sync Telemetry & Observability | LOW | HIGH | 📋 TODO |
| **HIGH** | Phase 4.3: Document Network Limits | LOW | HIGH | 📋 TODO |
| **MEDIUM** | Phase 5.1: Benchmarking | MEDIUM | MEDIUM | 📋 TODO |
| **MEDIUM** | Phase 2.5: Z3 SMT Verification | HIGH | MEDIUM | 📋 TODO |
| **LOW** | Phase 2.3: Loom Concurrency Testing | MEDIUM | LOW | 📋 TODO |
| **LOW** | Phase 7.1: INPUT_QUEUE_LENGTH Configurability | HIGH | LOW | 📋 Deferred |

---

## Breaking Changes Policy

**Core Principle: Correctness Over Compatibility**

This project **explicitly permits breaking changes** when they:
1. Improve correctness and safety
2. Enhance determinism guarantees
3. Enable formal verification
4. Align with production-grade goals

---

## Remaining Work

### Phase 2: Formal Verification (Continued)

#### 2.3 Loom Concurrency Testing
- [ ] Test concurrent GameStateCell operations
- [ ] Verify no deadlocks in Mutex usage
- [ ] Test event queue concurrent push/pop

#### 2.5 Z3 SMT Solver Integration (HIGH VALUE)

**Status:** NOT STARTED

**Proposed Z3 Verification Targets:**

| Property | Description | Priority |
|----------|-------------|----------|
| Frame arithmetic overflow | Prove wrapping semantics are correct | HIGH |
| Circular buffer bounds | Prove head/tail always valid | HIGH |
| Rollback frame selection | Prove target is always <= current_frame | HIGH |
| Sparse saving correctness | Prove saved state available when needed | MEDIUM |
| Input consistency | Prove confirmed inputs never change | MEDIUM |

**Implementation Tasks:**
- [ ] Add `z3` as dev-dependency with optional feature
- [ ] Create `src/verification/z3_proofs.rs` module
- [ ] Write proofs for frame arithmetic safety
- [ ] Write proofs for circular buffer indexing
- [ ] Write proofs for rollback frame selection
- [ ] Add Z3 to CI (optional, may require Z3 installation)

#### 2.6 Creusot Deductive Verification (OPTIONAL)

Deductive verifier using Why3 platform. Lower priority since Kani covers many properties.

- [ ] Add `creusot-contracts` as optional dev-dependency
- [ ] Add Creusot annotations to critical types

#### 2.7 Prusti Contract Verification (OPTIONAL)

ETH Zurich's verifier with VS Code extension. Lower priority.

- [ ] Add `prusti-contracts` as optional dev-dependency
- [ ] Add contract annotations to critical functions

### Phase 3: Test Coverage (Continued)

#### 3.3 Metamorphic Testing
- [ ] Input permutation invariance tests
- [ ] Timing invariance tests
- [ ] Replay consistency tests

### Phase 4: Enhanced Usability

#### 4.2 Examples
- [ ] Advanced configuration examples
- [ ] Error handling examples

#### 4.3 Document Network Limits (HIGH PRIORITY)

**Status:** TODO - Low effort, high value

Add "Network Requirements" section to USER_GUIDE.md documenting:
- Supported conditions (<15% packet loss, <200ms RTT)
- Network conditions to avoid
- Tuning recommendations for marginal conditions

**Implementation Tasks:**
- [ ] Add "Network Requirements" section to USER_GUIDE.md
- [ ] Add network condition table to README.md
- [ ] Document `SyncConfig` presets for different scenarios
- [ ] Add troubleshooting guide for sync failures

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

### Phase 7: Configuration & Observability

#### 7.1 INPUT_QUEUE_LENGTH Configurability (DEFERRED)

**Status:** DEFERRED - High effort, low immediate value

Current value (128) is generous for typical games. Keep as compile-time constant.

#### 7.2 Sync Protocol Configurability (HIGH PRIORITY)

**Status:** TODO - Medium effort, high value

**Problem:** The sync protocol requires `NUM_SYNC_PACKETS = 5` successful roundtrips. With packet loss >15%, sync times become excessive.

**Proposed Solution:** Create a `SyncConfig` struct:

```rust
#[derive(Debug, Clone)]
pub struct SyncConfig {
    pub sync_packets: u32,              // Default: 5
    pub sync_retry_interval: Duration,  // Default: 200ms
    pub sync_timeout: Option<Duration>, // Default: None
    pub running_retry_interval: Duration, // Default: 200ms
    pub keepalive_interval: Duration,   // Default: 200ms
}
```

**Implementation Tasks:**
- [ ] Create `SyncConfig` struct in `src/sessions/builder.rs`
- [ ] Add `with_sync_config()` method to `SessionBuilder`
- [ ] Pass config through to `UdpProtocol::new()`
- [ ] Replace constants with config values in `protocol.rs`
- [ ] Add `SyncTimeout` error variant to `FortressError`
- [ ] Update `poll()` to check sync timeout
- [ ] Add unit tests for configurable sync
- [ ] Add integration test with custom sync config
- [ ] Update docs and examples

#### 7.3 Sync Telemetry & Observability (HIGH PRIORITY)

**Status:** TODO - Low effort, high value

Extend telemetry system with sync-specific violations:

```rust
pub enum ViolationKind {
    // ... existing variants ...
    Synchronization,  // Sync protocol issues
}
```

**Implementation Tasks:**
- [ ] Add `ViolationKind::Synchronization` to telemetry
- [ ] Add sync retry warning threshold constant
- [ ] Add sync duration warning threshold
- [ ] Emit warning violations when thresholds exceeded
- [ ] Enhance `Event::Synchronizing` with RTT and elapsed time
- [ ] Add tests for telemetry emission
- [ ] Update telemetry documentation

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

### Before 1.0 Stable
- TLA+ specs for all protocols ✅ (4/4 complete)
- Kani proofs for critical functions ✅ (35 proofs complete)
- Formal specification complete ✅
- Deterministic hashing ✅
- No known correctness issues ✅
- All multi-process tests pass with checksum validation ✅

---

## Progress Log (Recent)

### December 2025 - Sessions 14-18

**Session 18: Protocol Configurability Planning**
- Deep analysis of sync protocol and packet loss
- Designed Phase 7.2 (SyncConfig), 7.3 (Sync Telemetry), 4.3 (Network Docs)
- Decision: 15% is practical packet loss limit for rollback netcode

**Session 17: Network Testing Expansion**
- Network resilience tests: 20 → 31 tests
- Multi-process tests: 8 → 17 tests
- All tests pass reliably

**Session 16: Edge Cases & Theorem Prover Research**
- Analyzed 5 "rollback before frames exist" edge cases (all protected)
- Researched Z3, Creusot, Prusti, Haybale integration options
- Key finding: Library handles all edge cases correctly

**Session 15: Deterministic Hashing**
- Created `fortress_rollback::hash` module with FNV-1a
- Eliminated all non-deterministic hashing
- Added edge case tests

**Session 14: BUG-001 & BUG-002**
- BUG-001 (Multi-Process Desync): Fixed - windowed checksum computation
- BUG-002 (Network Test Timing): Fixed - increased timeouts

**Session 13: TLA+ Completion**
- Completed Concurrency.tla for GameStateCell
- TLA+ specs 100% complete (4/4)

**Session 12: Kani & Compatibility**
- 35 Kani proofs created
- Downgraded to Rust 2021 edition for 1.75+ compatibility

**Session 11: Documentation & Formal Specs**
- Created ARCHITECTURE.md, USER_GUIDE.md
- Completed Phase 0 formal specifications (FORMAL_SPEC.md, API_CONTRACTS.md, DETERMINISM_MODEL.md)
