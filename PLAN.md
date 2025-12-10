# Fortress Rollback Improvement Plan

**Version:** 2.42
**Last Updated:** December 10, 2025
**Status:** ✅ Primary Goals Achieved, Flaky Test Under Verification
**Goal:** Transform Fortress Rollback into a production-grade, formally verified rollback networking library with >90% test coverage, absolute determinism guarantees, and exceptional usability.

---

## Current Status Summary

### Metrics
| Metric | Current | Target | Status |
|--------|---------|--------|--------|
| Library Tests | 315 | 100+ | ✅ Exceeded |
| Integration Tests | 206 | 30+ | ✅ Exceeded |
| Est. Coverage | ~92% | >90% | ✅ Complete |
| Clippy Warnings (lib) | 0 | 0 | ✅ Clean |
| Panics from Public API | 0 | 0 | ✅ |
| HashMap/HashSet Usage | 0 | 0 | ✅ |
| DefaultHasher Usage | 0 | 0 | ✅ |
| Miri Clean | All | All | ✅ (lib tests - proptests) |
| Runtime Panics (prod code) | 0 | 0 | ✅ Eliminated |
| Telemetry Coverage | All violations | All | ✅ Complete |
| TLA+ Specs | 4/4 | 4 | ✅ Complete |
| TLA+ CI Validation | 4/4 | 4/4 | ✅ All passing |
| Kani Proofs | 42 | 3+ | ✅ Complete |
| Kani CI Validation | 42/42 | 42/42 | ✅ All passing |
| Kani-Found Bugs Fixed | 1 | - | ✅ frame_delay validation |
| Z3 SMT Proofs | 25 | 5+ | ✅ Complete |
| Z3 CI Validation | 25/25 | 25/25 | ✅ All passing |
| Rust Edition | 2021 | - | ✅ Rust 1.75+ compatible |
| Network Resilience Tests | 31/31 | 20 | ✅ Exceeded |
| Multi-Process Tests | 30/30 | 8 | ✅ Exceeded |
| Formal Verification Scripts | 3/3 | 3 | ✅ Complete |
| Benchmarks | 2/2 | 2 | ✅ Complete (criterion) |
| Flaky Tests | 1 | 0 | 🔍 Under Verification (`test_terrible_network_preset`) |
| Advanced Examples | 2 | 2 | ✅ Complete |
| Metamorphic Tests | 16/16 | 10 | ✅ Complete |
| Loom Tests | 5 | 5 | ✅ Complete |
| Spec-Production Alignment | Audited | Audited | ✅ Complete |
| Graceful Error Handling | Audited | Audited | ✅ Complete |
| Configurable Constants | InputQueueConfig | API | ✅ Complete |
| Spec Realignment (Phase 10) | Complete | Complete | ✅ Complete |
| FV Gap Analysis (Phase 11) | Complete | Complete | ✅ Complete |
| Fuzz Targets | 6/6 | 3+ | ✅ Complete |
| **Internal Module (`__internal`)** | ✅ Exposed | Exposed | ✅ Complete |
| **Internal Property Tests** | 9/9 passing | 9 tests | ✅ Complete |
| **Internal Invariant Tests** | ✅ | 20+ tests | ✅ Complete |
| **Loom SavedStates Tests** | ✅ | 4 tests | ✅ Complete |
| **SpectatorSession Tests** | 22 | 20+ | ✅ Enhanced |
| **Defensive Programming Audit** | Complete | Full | ✅ Complete |
| **Defensive Clippy Lints** | 2 enabled | 2 | ✅ Complete |
| **#[must_use] Coverage** | 24 | 21+ | ✅ Complete |
| **SaveMode Enum (Boolean→Enum)** | ✅ | Complete | ✅ Complete |
| **Library Tests** | 315 | 100+ | ✅ Exceeded |
| **Total Tests** | 555 | 130+ | ✅ Exceeded |

### Next Priority Actions

| Priority | Task | Effort | Value | Status |
|----------|------|--------|-------|--------|
| **HIGH** | 🔍 Verify `test_terrible_network_preset` stability (200+ runs) | LOW | HIGH | 📋 Pending Verification |
| **MEDIUM** | Phase 6.1: Core Extraction | HIGH | HIGH | 📋 Planned |
| **MEDIUM** | Phase 6.2: Module Reorganization | MEDIUM | HIGH | 📋 Planned |
| **LOW** | OSS-Fuzz Integration | MEDIUM | MEDIUM | 📋 Optional |
| ~~**LOW**~~ | ~~Defensive Programming Patterns Audit~~ | ~~LOW~~ | ~~MEDIUM~~ | ✅ Complete |

### 🎉 Project Status: All Primary Goals Achieved, Flaky Test Verification Pending

The project has exceeded all original targets. All phases are now complete:
- ✅ `__internal` module created exposing `InputQueue`, `SyncLayer`, `SavedStates`, `TimeSync`, `UdpProtocol`, compression functions
- ✅ 2 new direct fuzz targets: `fuzz_input_queue_direct`, `fuzz_sync_layer_direct`
- ✅ New test files: `test_internal_invariants.rs`, `test_internal_property.rs`
- ✅ Loom tests for `SavedStates` concurrency
- ✅ 8 new Z3 proofs for internal invariants
- ✅ **All 9 proptest tests passing** (fixed in Session 39)
- ✅ **SpectatorSession comprehensive tests** (20 new tests, Session 40)
- ✅ **SaveMode enum** (Session 43) - Replaced boolean `sparse_saving` with self-documenting enum
- ✅ **Defensive programming audit complete** (Session 44) - `#[non_exhaustive]` on enums, destructuring in Debug impls, Forward Compatibility docs on config structs

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

## 🔍 PENDING VERIFICATION: Flaky Test Investigation

### `test_terrible_network_preset` Failure Analysis

**Status:** 🔍 Under Verification - No production/test fix has been made in this session. Need to run 200+ iterations to confirm stability.

**Preliminary Investigation (Session 45):**
- Ran 90 consecutive iterations: **0 failures observed**
- Hypothesis: Root cause may have been addressed by prior "Major bug fix" commit (e3f6a55, Dec 8)
- **NOT YET VERIFIED** - Need 200+ successful runs to confirm

**Original Error (for historical reference):**
```
thread 'test_terrible_network_preset' panicked at tests/test_network_resilience.rs:2472:47:
called `Result::unwrap()` on an `Err` value: InvalidFrame { frame: Frame(0), reason: "must load frame in the past (frame to load is 0, current frame is 0)" }
```

**Verification Required:**
- [ ] Run `test_terrible_network_preset` **200+ times** with zero failures
- [ ] If any failure occurs, perform root cause analysis
- [ ] Only mark as resolved after production/test fix is made AND verified

**Run Command:**
```bash
# Run test 200+ times and count failures
count=0; total=200; for i in $(seq 1 $total); do 
  echo "Run $i/$total"; 
  if ! cargo test --test test_network_resilience test_terrible_network_preset 2>&1 | grep -q "test result: ok"; then 
    ((count++)); 
    echo "FAILURE at run $i"; 
  fi; 
done; echo "Results: $count failures out of $total runs"
```

**Hypothesized Fix (from prior commit - unverified):**
- The "Major bug fix" commit may have addressed edge cases in frame-0 handling during rollback
- Session state management improvements may prevent rollback attempts before first frame is saved
- These changes may have eliminated the race condition, but this needs verification

---

## Remaining Work
   - `GameStateAccessor` type is conditionally compiled
   - Production path unchanged; tests use `load()` which requires `Clone`

3. ✅ **Added loom as conditional dependency**
   - `[target.'cfg(loom)'.dependencies] loom = "0.7"` in Cargo.toml
   - Added `'cfg(loom)'` to `check-cfg` in `[lints.rust]`

4. ✅ **Wrote 5 comprehensive GameStateCell loom tests**
   - `test_concurrent_saves` - Multiple threads saving concurrently
   - `test_save_load_consistency` - Save and load never see partial state
   - `test_multiple_readers_single_writer` - MRSW pattern verification
   - `test_frame_advancement_pattern` - Rollback save pattern simulation
   - `test_concurrent_access_bounded` - Bounded model checking with 3 threads

**Run loom tests:**
```bash
cd loom-tests
RUSTFLAGS="--cfg loom" cargo test --release
```

**Architecture Notes:**
- Main library uses `parking_lot` in production for performance
- When `--cfg loom` is set, uses `loom::sync` primitives for exhaustive testing
- `GameStateCell::data()` returns `None` under loom (MappedMutexGuard not available)
- Tests should use `load()` for loom-compatible data access

**Future Enhancement (Optional):**
- Test SyncLayer concurrent operations (input queue, rollback)
- Additional loom tests for multi-lock paths if added in future

### Phase 3: Test Coverage (Continued)

#### 3.3 Metamorphic Testing ✅
- [x] Input permutation invariance tests (5 tests)
- [x] Timing invariance tests (2 tests)
- [x] Replay consistency tests (4 tests)
- [x] Property-based metamorphic tests (5 proptest tests)

### Phase 4: Enhanced Usability (Continued)

#### 4.2 Examples ✅
- [x] Advanced configuration examples (`examples/configuration.rs`)
- [x] Error handling examples (`examples/error_handling.rs`)

### Phase 5: Performance (Continued)

#### 5.2 Continuous Fuzzing ✅

**Status:** Complete (Session 37, Dec 9)

**Priority:** HIGH

**Completed:**
- [x] Set up cargo-fuzz infrastructure with nightly toolchain
- [x] Created 4 comprehensive fuzz targets:
  - `fuzz_message_parsing` - Tests bincode deserialization of network messages
  - `fuzz_session_config` - Tests SessionBuilder and InputQueueConfig validation
  - `fuzz_input_queue` - Tests SyncTestSession with arbitrary operations
  - `fuzz_compression` - Tests XOR encoding and byte pattern handling
- [x] Verified all fuzz targets run successfully
- [x] Found and properly handled init-time panics (input_delay > queue_length)

**Run fuzzing:**
```bash
# Run a specific fuzzer (recommended: at least 60 seconds)
cargo +nightly fuzz run fuzz_message_parsing -- -max_total_time=60

# List all available fuzz targets
cargo +nightly fuzz list

# Run all fuzzers briefly
for target in $(cargo +nightly fuzz list); do
  cargo +nightly fuzz run $target -- -max_total_time=10
done
```

**Fuzz Target Details:**

| Target | Purpose | Coverage |
|--------|---------|----------|
| `fuzz_message_parsing` | Network message deserialization | 258+ edges |
| `fuzz_session_config` | Configuration validation | 82+ edges |
| `fuzz_input_queue` | Session operations and rollback | 426+ edges |
| `fuzz_compression` | XOR encoding patterns | 155+ edges |

**Files Added:**
- `fuzz/Cargo.toml` - Fuzz target configuration
- `fuzz/fuzz_targets/fuzz_message_parsing.rs`
- `fuzz/fuzz_targets/fuzz_session_config.rs`
- `fuzz/fuzz_targets/fuzz_input_queue.rs`
- `fuzz/fuzz_targets/fuzz_compression.rs`

**Future Work (Optional):**
- [ ] Set up OSS-Fuzz for continuous public fuzzing
- [ ] Add corpus seeds for better initial coverage
- [ ] Add CI job to run fuzzing on PRs

### Phase 6: Maintainability (Optional)

#### 6.1 Core Extraction 📋

**Priority:** HIGH

- [ ] Extract `fortress-core` crate with verified primitives
- [ ] InputQueue, SyncLayer, TimeSync in core
- [ ] No network dependencies in core
- [ ] 100% Kani-verified core

#### 6.2 Module Reorganization 📋

**Priority:** HIGH

- [ ] Separate protocol from session logic
- [ ] Clean interfaces between layers
- [ ] Reduce function sizes (< 50 lines)

#### 6.3 Spec-Production Alignment Audit ✅

**Status:** Complete (Session 33, Dec 9). Comprehensive audit of TLA+, Kani, and Z3 specs against production code.

**Completed:**
1. ✅ **Audited TLA+ specs** - Verified all 4 specs (NetworkProtocol, InputQueue, Rollback, Concurrency) align with production code
2. ✅ **Audited Kani proofs** - All 35 proofs verify production behavior correctly
3. ✅ **Audited Z3 SMT proofs** - All 18 proofs use production-scale constants where tractable
4. ✅ **Documented intentional divergences** - Created `specs/SPEC_DIVERGENCES.md` explaining:
   - TLA+ uses small constants (QUEUE_LENGTH=3, MAX_PREDICTION=1-3) for model checking tractability
   - Kani uses INPUT_QUEUE_LENGTH=8 for symbolic execution tractability
   - Z3 uses production values (128, 8, -1) where possible
5. ✅ **Added spec-production linkage comments** to key source files:
   - `src/input_queue.rs`: INPUT_QUEUE_LENGTH, MAX_FRAME_DELAY
   - `src/sync_layer.rs`: SyncLayer struct and fields (max_prediction, last_confirmed_frame, etc.)
   - `src/lib.rs`: Frame type and NULL_FRAME constant
   - `src/sessions/builder.rs`: DEFAULT_MAX_PREDICTION_FRAMES, DEFAULT_DISCONNECT_TIMEOUT, DEFAULT_FPS
   - `src/network/protocol.rs`: ProtocolState enum

**Key Finding:** All specs and production code are well-aligned. Divergences are intentional and documented.

**Documentation:** See `specs/SPEC_DIVERGENCES.md` for full audit results.

### Phase 7: Runtime Panic Elimination ✅

**Status:** Complete (Session 32, Dec 9). All runtime panic sources in production code have been eliminated or converted to graceful error handling with telemetry.

**Goal:** Eliminate all runtime panics from production code paths. Panics during configuration/construction (init-time) are acceptable, but panics after initialization (runtime) must never occur. All violations must be observable via the telemetry pipeline.

#### 7.1 Audit Results

**Comprehensive grep audit performed for:** `.unwrap()`, `.expect()`, `panic!()`, `unreachable!()`, `unimplemented!()`, `todo!()`

**Categorization:**

| Category | Count | Action |
|----------|-------|--------|
| Test code (`#[cfg(test)]`) | ~40 | Acceptable - tests should panic on failure |
| Init-time (construction) | 4 | Acceptable - fails fast during session setup |
| Loom-specific (`#[cfg(loom)]`) | ~8 | Acceptable - loom testing requires `unwrap()` |
| Debug assertions (`debug_assert!`) | ~5 | Acceptable - only in debug builds |
| Runtime production | 3 | **Fixed** (see below) |

#### 7.2 Remediations Applied

1. **`src/network/protocol.rs:29` - `millis_since_epoch()`**
   - **Issue:** `.expect("Time went backwards")` could panic if system clock adjusted backward
   - **Fix:** Changed to `match` with `report_violation!` on error, returns 0 as fallback
   - **Impact:** Harmless fallback; timing will self-correct on next call
   - **Tests added:** 4 time utility tests

2. **`src/sync_layer.rs:313` - `save_current_state()`**
   - **Issue:** `.expect("Internal error: current_frame should always be valid")` 
   - **Fix:** Added `debug_assert!` + graceful match with `report_violation!` for impossible case
   - **Rationale:** Invariant is construction-enforced but now handles impossible case gracefully
   - **Tests added:** 6 invariant tests

3. **`src/telemetry.rs` - `CollectingObserver`**
   - **Issue:** Used `std::sync::Mutex` which can poison, requiring `.unwrap()` on `lock()`
   - **Fix:** Changed to `parking_lot::Mutex` which doesn't poison (no `unwrap()` needed)
   - **Impact:** Consistent with rest of codebase; eliminates 9 potential panic sources
   - **Tests added:** 4 concurrent access tests

#### 7.3 Additional Telemetry Enhancements

4. **`src/network/protocol.rs` - `on_input()` frame gap detection**
   - **Issue:** Frame gaps could silently cause decode failures
   - **Fix:** Added `report_violation!` before early return when gap is too large
   - **Tests added:** 5 frame gap boundary tests

5. **`src/network/protocol.rs` - `from_inputs()` frame consistency**
   - **Issue:** Inconsistent input frames across players could indicate bugs
   - **Fix:** Added `report_violation!` when frames are inconsistent (with debug_assert)
   - **Tests added:** 3 frame consistency tests

#### 7.4 Acceptable Panic Sources (Not Changed)

- **Init-time panics in session construction:** `set_frame_delay().expect(...)` - fails fast if configuration invalid
- **Init-time panics in protocol setup:** `InputBytes::zeroed()` - fails if Input type can't serialize
- **Loom testing:** Required for loom's `Mutex` API
- **Test code:** Tests should panic on assertion failure
- **Debug assertions:** Only in debug builds, aids development

#### 7.5 Verification

- [x] All 291 library tests pass (22 new)
- [x] All 146+ integration tests pass
- [x] Zero clippy warnings
- [x] grep audit confirms no new runtime panic sources
- [x] All violation paths have telemetry coverage

### Phase 8: Graceful Error Handling Audit ✅

**Status:** Complete (Session 34, Dec 9)

**Goal:** Audit all production code and apply the graceful error handling principles discovered during protocol.rs improvements. Remove redundant `debug_assert!` calls that follow `report_violation!`, ensure all error paths recover gracefully, and downgrade severity levels where appropriate.

**Principles Applied:**
1. `report_violation!` is for **observable telemetry** - it logs but does NOT panic
2. Remove redundant `debug_assert!` after `report_violation!` - they add noise without value
3. **Always recover gracefully** - production code should never panic on recoverable conditions
4. Downgrade severity to `Warning` when the code can continue safely
5. Use `Error` severity only when behavior is degraded but code continues
6. Use `Critical` severity only for true invariant violations (corruption, impossible states)

**Files Audited:**
- [x] `src/sync_layer.rs` - No redundant patterns found, debug_assert before report_violation is correct
- [x] `src/input_queue.rs` - No issues found
- [x] `src/sessions/p2p_session.rs` - **Fixed:** Converted 2 assert! macros to report_violation + graceful handling
- [x] `src/sessions/p2p_spectator_session.rs` - **Fixed:** Converted 3 assert! macros, added frame validation
- [x] `src/sessions/sync_test_session.rs` - Already using report_violation correctly
- [x] `src/time_sync.rs` - **Fixed:** Added NULL/negative frame handling with report_violation
- [x] `src/network/messages.rs` - Already using proper error handling
- [x] `src/network/compression.rs` - **Fixed:** Converted 2 assert! to Result returns with report_violation

**Remediations Applied (Session 34, Dec 9):**

1. **`src/time_sync.rs:advance_frame()`**
   - **Issue:** Negative frame (e.g., `Frame::NULL = -1`) would wrap to large index, causing panic
   - **Fix:** Added early return with `report_violation!` for NULL or negative frames
   - **Tests added:** 2 tests for NULL and negative frame handling

2. **`src/sessions/p2p_session.rs:send_confirmed_inputs_to_spectators()`**
   - **Issue:** Two `assert!` macros could panic in production
   - **Fix:** Converted to `report_violation!` with graceful recovery (skip iteration on mismatch)

3. **`src/sessions/p2p_spectator_session.rs`**
   - **Issue:** `frames_behind_host()` had `assert!(diff >= 0)` that could panic
   - **Fix:** Converted to `report_violation!` returning 0 on invalid state
   - **Issue:** `inputs_at_frame()` could panic on negative frame index
   - **Fix:** Added frame validation with early error return
   - **Issue:** `handle_event()` had `assert!` and potential index panic
   - **Fix:** Added frame and player validation with `report_violation!`

4. **`src/network/compression.rs`**
   - **Issue:** `delta_encode()` had `assert_eq!` that could panic on mismatched lengths
   - **Fix:** Changed to skip mismatched inputs with `report_violation!` warning
   - **Issue:** `delta_decode()` had `assert!` that could panic
   - **Fix:** Changed return type to `Result<>`, returns error with `report_violation!`
   - **Tests added:** 3 tests for error cases

**Success Criteria: ✅ All Met**
- [x] No redundant `debug_assert!` after `report_violation!`
- [x] All recoverable errors use appropriate severity levels
- [x] No runtime panics in any error path (all assert! converted)
- [x] Tests verify graceful degradation (7 new tests added)

### Phase 9: Configurable Constants ✅

**Status:** Complete

**Goal:** Make hardcoded constants configurable at session construction time, enabling users to tune the library for their specific use cases (e.g., longer input history, larger prediction windows).

**Constants Made Configurable:**
- [x] `INPUT_QUEUE_LENGTH` (default 128) - Configurable via `InputQueueConfig`
- [x] `MAX_FRAME_DELAY` (derived as queue_length - 1) - Automatically derived from queue length

**Implementation:**
1. Added `InputQueueConfig` struct with:
   - `queue_length: usize` field (default 128)
   - Presets: `high_latency()` (256), `minimal()` (32), `standard()` (128)
   - `validate()` method to check constraints
   - `validate_frame_delay()` method for delay validation
2. Updated `SessionBuilder` with `with_input_queue_config()` method
3. Updated `InputQueue` with configurable `queue_length` field
4. Updated `SyncLayer`, `P2PSession`, `SyncTestSession` to propagate config
5. Validation at session construction prevents invalid configurations
6. Added 12 new tests for `InputQueueConfig`

**Design Decisions:**
- **Runtime configuration** chosen over const generics for simplicity and flexibility
- **Minimum queue_length: 2** to ensure at least one frame of history
- **Backward compatible** - existing code using defaults continues to work
- **Presets provided** for common use cases (high latency, minimal memory, standard)

**Files Modified:**
- `src/sessions/builder.rs` - Added `InputQueueConfig` struct and builder method
- `src/input_queue.rs` - Made `queue_length` configurable
- `src/sync_layer.rs` - Added `with_queue_length` constructor
- `src/sessions/p2p_session.rs` - Updated to accept queue_length
- `src/sessions/sync_test_session.rs` - Added `with_queue_length` constructor
- `src/lib.rs` - Exported `InputQueueConfig` from public API

### Phase 10: Spec Realignment ✅

**Status:** Complete (Session 35, Dec 9)

**Priority:** HIGH - Completed to ensure formal specs match production behavior after Phase 9 changes

**Goal:** After making constants configurable in Phase 9, revisit all TLA+, Kani, and Z3 specifications to ensure they still accurately model production behavior.

**Tasks Completed:**
- [x] Update TLA+ specs to document configurable parameters
  - `InputQueue.tla`: Added header documentation explaining `QUEUE_LENGTH` maps to configurable `queue_length`
  - `Rollback.tla`: Added header documentation explaining `MAX_PREDICTION` is configurable
  - `NetworkProtocol.tla`: Added header documentation explaining `NUM_SYNC_PACKETS` maps to `SyncConfig`
- [x] Update Kani proofs with documentation
  - Added comprehensive header to `kani_input_queue_proofs` module explaining size-independence
  - Documented that invariants proven for `queue_length=8` hold for any valid queue length (32, 128, 256)
- [x] Update Z3 proofs with documentation
  - Added "Configurable Constants Alignment" section to module documentation
  - Documented that proofs use default values but are size-independent
- [x] Update `specs/SPEC_DIVERGENCES.md`
  - Added "Configurable Constants (Phase 9/10)" section with preset table
  - Documented verification strategy explaining size-independence of proofs
  - Updated revision history to version 1.1
- [x] Update spec-production linkage comments
  - Updated `InputQueueConfig.queue_length` documentation to reference TLA+, Kani, Z3, and SPEC_DIVERGENCES.md
- [x] Run full verification suite - ALL PASSING:
  - TLA+: 4/4 specs pass (NetworkProtocol, InputQueue, Concurrency, Rollback)
  - Z3: 18/18 proofs pass
  - Library tests: 304 pass
  - Clippy: 0 warnings

**Key Finding:** The invariants proven in TLA+, Kani, and Z3 are **size-independent** - they hold for any valid queue length >= 2. This means proofs passing for small queue lengths imply correctness for production sizes (32, 128, 256).

### Phase 11: Formal Verification Gap Analysis ✅

**Status:** Complete (Session 36, Dec 9)

**Priority:** HIGH - Configurable constants introduced new verification surface that may not be fully covered

**Goal:** Investigate whether existing formal proofs adequately cover the new configurable constants, and add new proofs if gaps exist.

#### Investigation Results

1. **Kani Proofs (35 existing → 42 total):**
   - [x] Existing proofs use `INPUT_QUEUE_LENGTH=8` with documented size-independence
   - [x] Invariants (INV-4, INV-5) are truly size-independent - circular buffer arithmetic works identically for any queue size
   - [x] **GAP FOUND & FIXED**: Added 8 new Kani proofs for `InputQueueConfig` validation in `src/sessions/builder.rs`

2. **Z3 Proofs (18 existing):**
   - [x] Proofs use production default values (128) and are size-independent
   - [x] Modulo operations work identically for any valid queue_length
   - [x] No additional Z3 proofs needed - unit tests cover config validation adequately

3. **TLA+ Specs (4 existing):**
   - [x] Already documented with configurable constants mapping (Phase 10)
   - [x] Small constants (QUEUE_LENGTH=3) used for model checking tractability
   - [x] No changes needed - protocol behavior is captured correctly

4. **Missing Coverage Areas (Analyzed):**
   - [x] `TimeSyncConfig.window_size` - enforced via `max(1)` in `TimeSync::with_config`
   - [x] `SyncConfig.num_sync_packets` - no explicit validation needed (production uses reasonable defaults)
   - [x] `SpectatorConfig` - timing configuration, not protocol logic
   - [x] Config presets - now formally verified via Kani

#### New Proofs Added (7 total)

**File:** `src/sessions/builder.rs` - `kani_config_proofs` module

| Proof | What it Verifies |
|-------|------------------|
| `proof_validate_accepts_valid_queue_lengths` | `validate()` accepts queue_length >= 2, rejects < 2 |
| `proof_validate_boundary_at_two` | Boundary condition at queue_length = 2 |
| `proof_validate_frame_delay_constraint` | `validate_frame_delay()` enforces delay < queue_length |
| `proof_max_frame_delay_derivation` | `max_frame_delay() == queue_length.saturating_sub(1)` |
| `proof_max_frame_delay_is_valid_delay` | `max_frame_delay()` always passes `validate_frame_delay()` |
| `proof_all_presets_valid` | `standard()`, `high_latency()`, `minimal()` all pass `validate()` |
| `proof_preset_values` | Preset queue_length values match documentation |

#### Conclusion

The investigation confirmed that existing proofs provide strong coverage through size-independence.
The gap analysis revealed one actionable improvement: formally verifying `InputQueueConfig` validation.
Seven new Kani proofs were added to close this gap, bringing total Kani proofs from 35 to 42.

### Phase 12: Internal Visibility for Production Testing ✅ (Complete)

**Status:** Complete (Dec 9, 2025) - All tests passing

**Priority:** **VERY HIGH** - Enables maximum fuzzing, testing, and verification coverage across production code

**Goal:** Expose internal production components to fuzzers, tests, and formal verification tools to maximize coverage without compromising the public API or creating test-only code paths that diverge from production.

#### Implementation Summary

**12.1 `__internal` Module Created** ✅

Added `pub mod __internal` in `src/lib.rs` with `#[doc(hidden)]` exposing:

| Component | Type | Purpose |
|-----------|------|---------|
| `InputQueue<T>` | Core type | Circular buffer for input history |
| `PlayerInput<T>` | Core type | Input wrapper with frame info |
| `GameState<T>` | Core type | Game state wrapper |
| `SyncLayer<T>` | Core type | Frame synchronization engine |
| `SavedStates<T>` | Core type | State cell management for rollback |
| `TimeSync` | Core type | Time synchronization calculations |
| `encode`, `decode` | Functions | XOR compression |
| `delta_encode`, `delta_decode` | Functions | Delta compression |
| `ConnectionStatus` | Enum | Protocol connection state |
| `Event` | Enum | Protocol events |
| `ProtocolState` | Enum | Protocol state machine |
| `UdpProtocol<T>` | Core type | Network protocol handler |
| `PlayerRegistry<T>` | Core type | Player management |
| `INPUT_QUEUE_LENGTH` | Constant | Default queue length (128) |
| `MAX_FRAME_DELAY` | Constant | Maximum frame delay |

**12.2 New Fuzz Targets Created** ✅

| Target | File | Coverage |
|--------|------|----------|
| `fuzz_input_queue_direct` | `fuzz/fuzz_targets/fuzz_input_queue_direct.rs` | Direct InputQueue operations |
| `fuzz_sync_layer_direct` | `fuzz/fuzz_targets/fuzz_sync_layer_direct.rs` | Direct SavedStates operations |

Total fuzz targets: 4 → 6

**12.3 New Test Files Created** ✅

| File | Tests | Purpose |
|------|-------|---------|
| `tests/test_internal_invariants.rs` | 20+ | InvariantChecker tests for InputQueue, SyncLayer, SavedStates |
| `tests/test_internal_property.rs` | 9 | Proptest property tests for internals (all passing) |
| `loom-tests/tests/loom_saved_states.rs` | 4 | Loom concurrency tests for SavedStates |

**12.4 Z3 Verification Expanded** ✅

Added 8 new Z3 proofs in `tests/test_z3_verification.rs`:
- `z3_proof_input_queue_head_tail_bounds`
- `z3_proof_input_queue_length_calculation`
- `z3_proof_sync_layer_confirmed_frame_invariant`
- `z3_proof_sync_layer_saved_frame_invariant`
- `z3_proof_saved_states_availability`
- `z3_proof_first_incorrect_frame_bound`
- `z3_proof_prediction_window_bounded`
- `z3_proof_frame_discard_safety`

Total Z3 proofs: 17 → 25

**12.5 Property Test Bug Fixed** ✅ (Session 39, Dec 9)

Three proptest tests were failing due to test logic bugs (not production bugs):
- `prop_input_queue_length_bounded`
- `prop_input_queue_sequential_frames`
- `prop_first_incorrect_frame_tracking`

**Root cause**: Tests used hardcoded discard timing (`i > 64 && i % 64 == 0`) that didn't scale with queue size. When `queue_length=32`, the queue would overflow at frame 33 before any discard could occur.

**Fix applied**: Changed discard timing to `i >= queue_length.saturating_sub(4)` which scales dynamically with queue size, ensuring discarding starts before overflow regardless of queue size.

#### Success Criteria ✅ All Met

- [x] All internal components exercisable by fuzzing without going through session APIs
- [x] New fuzz targets created (`fuzz_input_queue_direct`, `fuzz_sync_layer_direct`)
- [x] No increase in public API surface (all new exports are `#[doc(hidden)]`)
- [x] Same code paths for testing and production (no `#[cfg(test)]` divergence)
- [x] Formal verification coverage expanded (8 new Z3 proofs)
- [x] Loom concurrency tests for SavedStates
- [x] **All 9 property tests passing**

#### Design Principles Applied

1. **Production code is test code**: Exposed real implementation, not test doubles
2. **Same code paths always**: No `#[cfg(test)]` conditional compilation
3. **Documentation over restriction**: Used `#[doc(hidden)]` to discourage use
4. **Fuzz real production code**: Fuzzers exercise the same code that ships to users
5. **Verification validates production**: Z3 proofs apply to actual production behavior

---

## Known Issues (Non-Critical)

### Dead Code in InputQueue
The `advance_queue_head` function contains gap-filling code for handling frame delay increases mid-session. This code can never be reached because `add_input` rejects inputs first due to its sequential frame check.

**Test:** `test_frame_delay_change_mid_session_drops_input`

### Remaining Tasks (Optional)
- [ ] Reserve `fortress-rollback` on crates.io and publish initial release
- [ ] Protocol layer panic elimination (lower priority - most panics already removed)
- [ ] Session type pattern for state machine enforcement (optional API improvement)
- [ ] OSS-Fuzz integration for continuous public fuzzing
- [x] ~~**Defensive Programming Patterns Audit**~~ - Complete. Applied techniques from [corrode.dev/blog/defensive-programming](https://corrode.dev/blog/defensive-programming/):
  - [x] ~~Replace vector indexing with slice pattern matching where applicable~~ - Reviewed: protocol.rs peer_connect_status now uses `zip()` for safety
  - [x] ~~Audit `..Default::default()` usage~~ - Reviewed: ChaosConfig Default impl explicitly lists all fields; factory methods using `..Default::default()` are acceptable
  - [x] ~~Use destructuring in trait implementations~~ - Applied to `SessionBuilder`, `PlayerRegistry`, `Input` Debug implementations (Session 44)
  - [x] ~~Verify `From` impls are truly infallible~~ - Reviewed: `From<usize> for Frame` truncation is acceptable for frame numbers
  - [x] ~~Wildcard match arms~~ - Test files have wildcard `_ => unreachable!(...)` for `#[non_exhaustive]` enum support
  - [x] ~~Use named placeholders (`field: _`)~~ - Applied in destructuring patterns where applicable
  - [ ] Apply temporary mutability pattern (shadow to immutable after init) - Optional style improvement
  - [x] ~~Audit constructors for defensive patterns~~ - Added `#[non_exhaustive]` to public enums (FortressError, ViolationKind, FortressEvent, FortressRequest); added Forward Compatibility documentation to all config structs (Session 44)
  - [x] ~~Add `#[must_use]` to important types and methods~~ - Added to `SessionBuilder`, `ChaosConfigBuilder`, `NetworkStats`, `P2PSession` query methods (24 total)
  - [x] ~~Replace boolean parameters with enums for clarity~~ - `SaveMode` enum replaces `sparse_saving: bool` (Session 43); remaining `input_only: bool` internal only
  - [x] ~~Enable clippy lints: `must_use_candidate`, `fallible_impl_from`~~ - Added to Cargo.toml [lints.clippy] section

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
All 1.0 requirements met:
- TLA+ specs (4/4), Kani proofs (42/42), Z3 proofs (25/25) - all CI-validated
- Formal specification complete, deterministic hashing, no known correctness issues
- All multi-process tests pass with checksum validation

---

## Progress Log

### Summary
The project has achieved all primary goals:
- **Test Coverage**: ~92% (target >90%) with 315 library tests and 206 integration tests
- **Formal Verification**: TLA+ (4 specs), Kani (42 proofs), Z3 (25 proofs) - all validated in CI
- **Code Quality**: Zero clippy warnings, no HashMap/HashSet usage, Miri clean, zero doc warnings
- **Flaky Tests**: 1 under verification (`test_terrible_network_preset` - 90 runs passed, need 200+ to confirm)
- **Runtime Safety**: Zero runtime panics in production code paths
- **Graceful Error Handling**: All assert! macros in production code converted to report_violation + recovery
- **Configurable Constants**: InputQueueConfig allows runtime configuration of queue length
- **Spec Realignment**: All formal specs updated to document configurable constants
- **Formal Verification Gap Analysis**: All configurable constant validation formally verified
- **Examples**: Configuration and error handling examples added
- **Metamorphic Testing**: 16 tests covering input permutation, timing invariance, and replay consistency
- **Loom Concurrency Testing**: 5 tests verifying GameStateCell thread safety + 4 tests for SavedStates
- **Spec-Production Alignment**: All formal specs audited and documented in `specs/SPEC_DIVERGENCES.md`
- **Continuous Fuzzing**: 6 fuzz targets (message parsing, session config, input queue, compression, 2 direct internal)
- **Internal Visibility**: `__internal` module exposes internals for testing without `#[cfg(test)]` divergence
- **Property Tests**: All 9 property tests passing (fixed in Session 39)
- **SpectatorSession Tests**: Comprehensive test coverage (22 tests, Session 40)
- **SaveMode Enum**: Boolean `sparse_saving` replaced with self-documenting `SaveMode` enum (Session 43)
- **Defensive Programming Audit**: Complete - `#[non_exhaustive]` on public enums, destructuring in Debug impls, Forward Compatibility docs (Session 44)

Detailed session history archived. Key milestones:
- **Session 45 (Dec 10): Flaky Test Investigation - UNDER VERIFICATION**
  - **Goal**: Investigate `test_terrible_network_preset` flaky test reported in Session 44
  - **Preliminary Results**:
    - Ran 90 consecutive test iterations: **0 failures** (40 quick runs + 50 tracked runs)
    - Hypothesis: Root cause may have been addressed by prior "Major bug fix" commit (e3f6a55, Dec 8)
    - **NOT YET VERIFIED** - No production/test fix made in this session
  - **Next Step**: Run test 200+ times to verify stability before marking as resolved
  - **Fixed documentation warning**: `src/network/udp_socket.rs` URL now uses angle brackets
- **Session 44 (Dec 10): Defensive Programming Patterns Audit - Complete + Flaky Test Discovery**
  - **Goal**: Improve test coverage for `SpectatorSession` which had only 2 tests (~20% coverage)
  - **Added 20 new tests** (2 → 22 total) in `tests/test_p2p_spectator_session.rs`:
    - Basic session tests: `test_start_session`, `test_synchronize_with_host`
    - Session state tests: `test_current_frame_starts_at_null`, `test_frames_behind_host_initially_zero`, `test_num_players_default`, `test_num_players_custom`
    - Network tests: `test_network_stats_not_synchronized`
    - Events tests: `test_events_empty_initially`, `test_events_generated_during_sync`
    - Advance frame tests: `test_advance_frame_before_sync_fails`, `test_advance_frame_after_sync`
    - Violation observer tests: `test_violation_observer_attached`, `test_no_violation_observer_by_default`
    - Configuration tests: `test_spectator_config_buffer_size`, `test_spectator_with_input_queue_config`
    - Poll tests: `test_poll_remote_clients_no_host`
    - Full flow tests: `test_full_spectator_flow`
    - Event handling tests: `test_synchronized_event_generated`, `test_synchronizing_events_generated`
    - Edge case tests: `test_spectator_catchup_speed`, `test_multiple_spectators_same_host`, `test_spectator_disconnect_timeout`
  - Integration tests increased: 178 → 206
  - All 304 library tests + 206 integration tests passing
  - Zero clippy warnings
- **Session 44 (Dec 10): Defensive Programming Patterns Audit - Complete + Flaky Test Discovery**
  - **Goal**: Complete remaining defensive programming patterns from corrode.dev/blog/defensive-programming
  - **Added Forward Compatibility documentation** to all config structs:
    - `SyncConfig`, `ProtocolConfig`, `SpectatorConfig`, `InputQueueConfig` (builder.rs)
    - `TimeSyncConfig` (time_sync.rs)
    - Documentation explains `..Default::default()` pattern for future field additions
  - **Applied destructuring pattern in Debug implementations:**
    - `SessionBuilder<T>` - All fields explicitly listed in Debug impl
    - `PlayerRegistry` - All fields explicitly listed in Debug impl
    - `Input` (network/messages.rs) - All fields explicitly listed in Debug impl
  - **Added `#[non_exhaustive]` to public enums:**
    - `FortressError` (error.rs) - Prevents external exhaustive matching
    - `ViolationKind` (telemetry.rs) - Prevents external exhaustive matching
    - `FortressEvent` (lib.rs) - Prevents external exhaustive matching
    - `FortressRequest` (lib.rs) - Prevents external exhaustive matching
  - **Updated all test and example files with wildcard arms:**
    - `tests/stubs.rs`, `tests/stubs_enum.rs` - Added `_ => unreachable!("Unknown request type")`
    - `tests/bin/network_test_peer.rs` - Added wildcard arm
    - `examples/ex_game/ex_game.rs` - Added wildcard arm
    - `tests/test_metamorphic.rs` - Added wildcard arms to all 5 match statements
    - `tests/test_config.rs` - Updated to use `..Default::default()` pattern
    - `tests/test_p2p_spectator_session.rs` - Updated to use `..Default::default()` pattern
    - `examples/configuration.rs` - Added `#![allow(clippy::needless_update)]`
  - **Fixed corrupted test_metamorphic.rs** - Recreated file after perl regex corruption
  - **🔴 DISCOVERED FLAKY TEST: `test_terrible_network_preset`**
    - Error: `InvalidFrame { frame: Frame(0), reason: "must load frame in the past (frame to load is 0, current frame is 0)" }`
    - Appears to be edge case when rollback is triggered at frame 0
    - Detailed investigation documented in "Flaky Test Investigation" section above
    - Requires root cause analysis before any fix attempt
  - Defensive Programming Audit complete; flaky test investigation pending
- **Session 43 (Dec 10): SaveMode Enum - Boolean→Enum Defensive Pattern**
  - **Goal**: Replace boolean `sparse_saving` parameter with self-documenting `SaveMode` enum
  - **Created `SaveMode` enum** with two variants:
    - `SaveMode::EveryFrame` (default) - Save state every frame for minimal rollback distance
    - `SaveMode::Sparse` - Only save minimum confirmed frame for reduced save overhead
  - **Updated all affected code:**
    - `SessionBuilder`: Added `with_save_mode()`, deprecated `with_sparse_saving_mode()`
    - `P2PSession`: Changed `sparse_saving: bool` to `save_mode: SaveMode`
    - `SyncLayer::set_last_confirmed_frame()`: Parameter changed to `SaveMode`
    - `SyncTestSession`: Updated to use `SaveMode::EveryFrame`
  - **Added comprehensive tests** (11 new SaveMode tests):
    - `test_save_mode_default_is_every_frame`, `test_save_mode_equality`, `test_save_mode_debug_format`
    - `test_save_mode_clone`, `test_save_mode_copy`, `test_save_mode_hash`
    - `test_with_save_mode_every_frame`, `test_with_save_mode_sparse`
    - `test_deprecated_with_sparse_saving_mode_true/false`, `test_builder_default_save_mode`
  - **Updated examples and tests** to use new `SaveMode::Sparse` / `SaveMode::EveryFrame`
  - **Breaking change documented**: Old code using `with_sparse_saving_mode(true/false)` gets deprecation warning
  - Library tests increased: 304 → 315
  - All 315 library tests passing, zero clippy warnings
- **Session 39 (Dec 9): Property Test Bug Fix - Phase 12 Complete**
  - Fixed 3 failing proptest tests in `test_internal_property.rs`:
    - `prop_input_queue_length_bounded`
    - `prop_input_queue_sequential_frames`
    - `prop_first_incorrect_frame_tracking`
  - **Root cause**: Test logic used hardcoded discard timing (`i > 64 && i % 64 == 0`) that didn't scale with queue size. When `queue_length=32`, overflow occurred at frame 33 before any discard.
  - **Fix**: Changed discard timing to `i >= queue_length.saturating_sub(4)` which scales dynamically with queue size
  - Removed unused `frame_delay_strategy` function
  - All 9/9 proptest tests now passing
  - All 304 library tests passing, zero clippy warnings
  - Phase 12 (Internal Visibility) now fully complete
- **Session 42 (Dec 10): Defensive Programming Patterns Audit (Continued)**
  - **Expanded `#[must_use]` coverage from 3 to 24 total attributes:**
    - `InputQueue::new()`, `InputQueue::with_queue_length()` constructors
    - `PlayerRegistry::new()`, `local_player_handles()`, `remote_player_handles()`, `spectator_handles()`, `num_players()`, `num_spectators()`
    - `P2PSession` query methods: `confirmed_frame()`, `current_frame()`, `max_prediction()`, `in_lockstep_mode()`, `current_state()`, `events()`, `num_players()`, `num_spectators()`, `local_player_handles()`, `remote_player_handles()`, `spectator_handles()`, `handles_by_address()`, `frames_ahead()`, `desync_detection()`, `violation_observer()`
    - `ChaosConfigBuilder::build()`
    - Test stubs: `GameStub::new()`, `RandomChecksumGameStub::new()`, `GameStubEnum::new()`
  - **Added defensive clippy lints to Cargo.toml:**
    - `must_use_candidate = "warn"` - catches methods that should have `#[must_use]`
    - `fallible_impl_from = "deny"` - prevents `From` implementations that can fail (should be `TryFrom`)
  - Verified `wildcard_enum_match_arm` lint: library code is clean, only test code has wildcards
  - All 304 library tests passing, zero clippy warnings
- **Session 41 (Dec 10): Defensive Programming Patterns Audit (Partial)**
  - **Added `#[must_use]` attributes to key types:**
    - `SessionBuilder<T>` - "SessionBuilder must be consumed by calling a start_*_session method"
    - `ChaosConfigBuilder` - "ChaosConfigBuilder must be consumed by calling .build()"
    - `NetworkStats` - "NetworkStats should be inspected or used after being queried"
  - **Removed redundant `#[must_use]` from builder methods** - struct-level `#[must_use]` is sufficient
  - **Audited `..Default::default()` usage:**
    - `ChaosConfig::default()` explicitly initializes all fields - good
    - Factory methods using `..Default::default()` are acceptable for presets
  - **Audited `From` implementations:**
    - `From<usize> for Frame` - truncation acceptable for frame numbers (won't exceed i32::MAX in practice)
    - All `From` impls are truly infallible
  - **Reviewed array indexing patterns:**
    - `sync_layer.rs` indexing is construction-safe (num_players matches array length)
    - Fixed `protocol.rs` `on_input()` to use `zip()` instead of raw indexing for peer_connect_status
  - All tests pass, zero clippy warnings
- **Session 40 (Dec 9): SpectatorSession Test Coverage Enhancement**
  - **Goal**: Improve test coverage for `SpectatorSession` which had only 2 tests (~20% coverage)
  - **Added 20 new tests** (2 → 22 total) in `tests/test_p2p_spectator_session.rs`:
    - Basic session tests: `test_start_session`, `test_synchronize_with_host`
    - Session state tests: `test_current_frame_starts_at_null`, `test_frames_behind_host_initially_zero`, `test_num_players_default`, `test_num_players_custom`
    - Network tests: `test_network_stats_not_synchronized`
    - Events tests: `test_events_empty_initially`, `test_events_generated_during_sync`
    - Advance frame tests: `test_advance_frame_before_sync_fails`, `test_advance_frame_after_sync`
    - Violation observer tests: `test_violation_observer_attached`, `test_no_violation_observer_by_default`
    - Configuration tests: `test_spectator_config_buffer_size`, `test_spectator_with_input_queue_config`
    - Poll tests: `test_poll_remote_clients_no_host`
    - Full flow tests: `test_full_spectator_flow`
    - Event handling tests: `test_synchronized_event_generated`, `test_synchronizing_events_generated`
    - Edge case tests: `test_spectator_catchup_speed`, `test_multiple_spectators_same_host`, `test_spectator_disconnect_timeout`
  - Integration tests increased: 178 → 206
  - All 304 library tests + 206 integration tests passing
  - Zero clippy warnings
- **Session 38 (Dec 9): Internal Visibility for Production Testing (Phase 12) - Substantially Complete**
  - Created `__internal` module in `src/lib.rs` with `#[doc(hidden)]` exposing:
    - Core types: `InputQueue`, `PlayerInput`, `GameState`, `SyncLayer`, `SavedStates`, `TimeSync`
    - Network internals: `encode`, `decode`, `delta_encode`, `delta_decode`, `ConnectionStatus`, `Event`, `ProtocolState`, `UdpProtocol`
    - Session internals: `PlayerRegistry`; Constants: `INPUT_QUEUE_LENGTH`, `MAX_FRAME_DELAY`
  - Created 2 new direct fuzz targets: `fuzz_input_queue_direct`, `fuzz_sync_layer_direct`
  - Created new test files: `test_internal_invariants.rs` (20+ tests), `test_internal_property.rs` (9 tests), `loom_saved_states.rs` (4 tests)
  - Added 8 new Z3 proofs for internal invariants (total Z3: 17 → 25)
  - **Known issue**: 3 proptest tests fail due to test logic bugs (queue overflow)
- Session 37 (Dec 9): Continuous Fuzzing complete (Phase 5.2)
  - Installed cargo-fuzz with nightly toolchain
  - Created 4 comprehensive fuzz targets:
    - `fuzz_message_parsing`: Tests bincode deserialization of network Message types (258+ edges)
    - `fuzz_session_config`: Tests SessionBuilder and InputQueueConfig validation (82+ edges)
    - `fuzz_input_queue`: Tests SyncTestSession with arbitrary operations (426+ edges)
    - `fuzz_compression`: Tests XOR encoding and byte patterns (155+ edges)
  - All fuzz targets verified working with no crashes
  - Properly handled init-time panics (input_delay > queue_length) by clamping values
  - Added files: fuzz/Cargo.toml, fuzz/fuzz_targets/*.rs
- Session 36 (Dec 9): Formal Verification Gap Analysis complete (Phase 11)
  - Investigated existing Kani (35), Z3 (18), TLA+ (4) proofs for configurable constant coverage
  - Confirmed size-independence of invariants: proofs for small values imply correctness for any size
  - GAP FOUND: InputQueueConfig.validate() and validate_frame_delay() not formally verified
  - Added 7 new Kani proofs in `src/sessions/builder.rs` (kani_config_proofs module):
    - proof_validate_accepts_valid_queue_lengths
    - proof_validate_boundary_at_two
    - proof_validate_frame_delay_constraint
    - proof_max_frame_delay_derivation
    - proof_max_frame_delay_is_valid_delay
    - proof_all_presets_valid
    - proof_preset_values
  - Total Kani proofs: 35 → 42, all library tests passing (304)
- Session 35 (Dec 9): Spec Realignment complete (Phase 10)
  - Updated TLA+ specs (InputQueue.tla, Rollback.tla, NetworkProtocol.tla) with configurable constants documentation
  - Updated Kani proofs documentation explaining size-independence of invariants
  - Updated Z3 proofs documentation with configurable constants alignment section
  - Updated `specs/SPEC_DIVERGENCES.md` with Configurable Constants section (v1.1)
  - Updated `InputQueueConfig.queue_length` documentation to reference all formal specs
  - Ran full verification suite: TLA+ (4/4), Z3 (18/18), 304 library tests - all passing
  - Key finding: Invariants are size-independent; proofs for small sizes imply correctness for production sizes
- Session 34 (Dec 9): Graceful Error Handling Audit complete (Phase 8)
  - Audited all source files for assert! macros and panic paths in production code
  - Fixed time_sync.rs: Added NULL/negative frame handling with report_violation (2 new tests)
  - Fixed p2p_session.rs: Converted 2 assert! macros to report_violation with graceful recovery
  - Fixed p2p_spectator_session.rs: Converted 3 assert! macros, added frame/player validation
  - Fixed compression.rs: Changed delta_encode to skip mismatched inputs, changed delta_decode to return Result (3 new tests)
  - Total: 7 new tests added, 296 library tests now passing
- Session 33 (Dec 9): Spec-Production Alignment Audit complete
  - Audited all TLA+ specs (4), Kani proofs (35), Z3 proofs (18) against production code
  - Created `specs/SPEC_DIVERGENCES.md` documenting intentional divergences and verified alignments
  - Added formal specification linkage comments to key source files:
    - `src/input_queue.rs`: INPUT_QUEUE_LENGTH, MAX_FRAME_DELAY
    - `src/sync_layer.rs`: SyncLayer struct documentation
    - `src/lib.rs`: Frame type and NULL_FRAME
    - `src/sessions/builder.rs`: Default constants
    - `src/network/protocol.rs`: ProtocolState enum
  - Key finding: All specs and production code well-aligned; divergences are intentional for tractability
- Session 32 (Dec 9): Runtime panic elimination + telemetry enhancement complete
  - Audited all panic sources, fixed 3 runtime panics (millis_since_epoch, save_current_state, CollectingObserver)
  - Enhanced violations to use telemetry pipeline: millis_since_epoch, on_input frame gap, from_inputs frame consistency
  - Added 22 new tests: time utilities (4), save_current_state invariants (6), CollectingObserver concurrent (4), frame gap detection (8)
  - Updated agent files with Breaking Changes Policy
- Session 31 (Dec 9): Loom concurrency testing complete - integrated loom with main library, 5 GameStateCell tests
- Session 30 (Dec 9): Metamorphic testing complete (16 tests including 5 proptest properties)
- Session 29 (Dec 9): Fixed flaky test_asymmetric_packet_loss, added configuration.rs and error_handling.rs examples
- Session 28 (Dec 9): Test coverage goal achieved with comprehensive unit tests
- Session 27 (Dec 8): Z3 CI integration complete
- Session 26 (Dec 8): Z3 SMT verification complete (18 proofs)
- Session 24-25 (Dec 8): Kani CI + config API consistency
- Sessions 17-23 (Dec 7-8): Network resilience tests, multi-process tests, configuration structs
