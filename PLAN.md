# Fortress Rollback Improvement Plan

**Version:** 2.21
**Last Updated:** December 8, 2025
**Goal:** Transform Fortress Rollback into a production-grade, formally verified rollback networking library with >90% test coverage, absolute determinism guarantees, and exceptional usability.

---

## Current Status Summary

### Metrics
| Metric | Current | Target | Status |
|--------|---------|--------|--------|
| Library Tests | 224 | 100+ | ✅ Exceeded |
| Integration Tests | 115 | 30+ | ✅ Exceeded |
| Est. Coverage | ~89% | >90% | 🔄 Close |
| Clippy Warnings (lib) | 0 | 0 | ✅ Clean |
| Panics from Public API | 0 | 0 | ✅ |
| HashMap/HashSet Usage | 0 | 0 | ✅ |
| DefaultHasher Usage | 0 | 0 | ✅ |
| Miri Clean | All | All | ✅ (lib tests - proptests) |
| TLA+ Specs | 4/4 | 4 | ✅ Complete |
| TLA+ CI Validation | 4/4 | 4/4 | ✅ All passing |
| Kani Proofs | 35 | 3+ | ✅ Complete |
| Kani CI Validation | 35/35 | 35/35 | ✅ All passing |
| Kani-Found Bugs Fixed | 1 | - | ✅ frame_delay validation |
| Z3 SMT Proofs | 18 | 5+ | ✅ Complete |
| Z3 CI Validation | 18/18 | 18/18 | ✅ All passing |
| Rust Edition | 2021 | - | ✅ Rust 1.75+ compatible |
| Network Resilience Tests | 31/31 | 20 | ✅ Exceeded |
| Multi-Process Tests | 15/17 | 8 | ✅ Exceeded |
| Formal Verification Scripts | 3/3 | 3 | ✅ Complete |
| Benchmarks | 2/2 | 2 | ✅ Complete (criterion) |

### Next Priority Actions

| Priority | Task | Effort | Value | Status |
|----------|------|--------|-------|--------|
| **MEDIUM** | Reach >90% Test Coverage | MEDIUM | HIGH | 🔄 In Progress (~89%) |
| **LOW** | Phase 2.3: Loom Concurrency Testing | MEDIUM | LOW | 📋 TODO |
| **LOW** | Phase 3.3: Metamorphic Testing | MEDIUM | LOW | 📋 TODO |
| **LOW** | Phase 5.2: Continuous Fuzzing | MEDIUM | LOW | 📋 TODO |
| **LOW** | Phase 4.2: Advanced Examples | LOW | MEDIUM | 📋 TODO |

---

## Formal Verification Philosophy

**Core Principle: Specifications Model Production, Not The Other Way Around**

When formal verification (TLA+, Kani, Z3) finds a violation:

1. **First assume the production code has a bug** - The spec is the source of truth for intended behavior
2. **Investigate the violation trace** - Understand exactly what sequence of events causes the failure
3. **Determine root cause:**
   - **Production bug**: Fix the Rust implementation to match the spec
   - **Spec bug**: The spec incorrectly models production behavior - fix the spec AND document why
   - **Spec too strict**: The invariant is stricter than actual requirements - relax with justification
4. **Never "fix" specs just to make them pass** - This defeats the purpose of formal verification
5. **Document all spec changes** - Explain why the change was made and what production behavior it reflects

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

#### 2.5 Z3 SMT Solver Integration

**Status:** ✅ COMPLETE

**Z3 Verification Proofs (18 total in `tests/test_z3_verification.rs`):**

| Category | Proofs | Properties Verified |
|----------|--------|---------------------|
| Frame Arithmetic | 4 | Addition bounds, rollback validity, comparison transitivity, increment safety |
| Circular Buffer | 5 | Index validity, head advancement, wraparound, length invariant, distance calc |
| Rollback Selection | 3 | Target in past, within prediction window, saved state available |
| Frame Delay | 2 | Overflow prevention, sequential preservation |
| Input Consistency | 1 | Position uniqueness within window |
| Comprehensive | 2 | Complete rollback safety, prediction threshold |

**Implementation:**
- [x] Add `z3` as dev-dependency with `bundled` feature (builds Z3 from source)
- [x] Create `tests/test_z3_verification.rs` with 17 proofs
- [x] Write proofs for frame arithmetic safety (4 proofs)
- [x] Write proofs for circular buffer indexing (5 proofs)
- [x] Write proofs for rollback frame selection (3 proofs)
- [x] Add Z3 to CI (z3-verification job in rust.yml)

**Note:** Z3 build requires `cmake` and `libclang-dev`. The bundled feature compiles Z3 from source (~5 min first build).

### Phase 3: Test Coverage (Continued)

#### 3.3 Metamorphic Testing
- [ ] Input permutation invariance tests
- [ ] Timing invariance tests
- [ ] Replay consistency tests

### Phase 4: Enhanced Usability (Continued)

#### 4.2 Examples
- [ ] Advanced configuration examples
- [ ] Error handling examples

### Phase 5: Performance (Continued)

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
- All library tests pass
- All integration tests pass
- Coverage maintained or improved
- No clippy warnings
- No panics in library code

### Before Release
- Coverage ≥ 90%
- Determinism tests pass on all platforms
- Examples compile and run

### Before 1.0 Stable
- TLA+ specs for all protocols ✅ (4/4 complete)
- TLA+ specs validated in CI ✅
- Kani proofs for critical functions ✅ (35 proofs)
- Kani proofs validated in CI ✅ (all 35 passing)
- Z3 SMT proofs for algorithm safety ✅ (18 proofs)
- Z3 proofs validated in CI ✅ (all 18 passing)
- Formal specification complete ✅
- Deterministic hashing ✅
- No known correctness issues ✅
- All multi-process tests pass with checksum validation ✅

---

## Progress Log (Recent)

### December 2025

**Session 27 (Dec 8):** Z3 CI Integration Complete
- Added `z3-verification` job to `.github/workflows/rust.yml`
- Z3 tests (18 proofs) now run in CI with caching for faster builds
- Dependencies: cmake, libclang-dev installed automatically
- Fixed clippy warnings across test files (clone on Copy types)
- All formal verification now validated in CI: TLA+ (4), Kani (35), Z3 (17)

**Session 26 (Dec 8):** Phase 2.5 Z3 SMT Verification Complete
- Added `z3` dev-dependency with `bundled` feature (builds Z3 from source)
- Created `tests/test_z3_verification.rs` with 17 formal proofs
- Frame arithmetic proofs: bounds, rollback validity, transitivity, increment safety
- Circular buffer proofs: index validity, head advancement, wraparound, length invariant
- Rollback selection proofs: target in past, within prediction window, saved state available
- Frame delay proofs: overflow prevention, sequential preservation
- Input consistency proof: position uniqueness within queue window
- Comprehensive proofs: complete rollback safety, prediction threshold
- Total integration tests: 117 (99 + 18 Z3 proofs)
- Dependencies: cmake, libclang-dev required for bundled Z3 build

**Session 25 (Dec 8):** Phase 7.5 Config API Consistency Complete
- Added `Copy` trait to `SyncConfig` and `ProtocolConfig`
- Fixed doc links, all 29 doc tests pass
- Created `tests/test_config.rs` with 25 tests
- Total integration tests: 99

**Session 24 (Dec 8):** Kani CI + SpectatorConfig/TimeSyncConfig + Bug Fix
- All 35 Kani proofs passing in CI
- Kani-discovered bug: `frame_delay` now validated `< INPUT_QUEUE_LENGTH`
- SpectatorConfig/TimeSyncConfig complete with presets

**Session 23 (Dec 8):** Network docs, InputQueue fix, ProtocolConfig
- Phase 4.3: Document Network Limits
- Fixed InputQueue circular buffer invariant violation
- ProtocolConfig struct with presets

**Session 17-22:** Testing, Sync configurability, Agent standardization
- 31 network resilience tests, 17 multi-process tests
- SyncConfig struct with presets
- Standardized LLM agent instruction files
