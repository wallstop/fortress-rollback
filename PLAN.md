# Fortress Rollback Improvement Plan

**Version:** 1.5
**Last Updated:** December 6, 2025
**Goal:** Transform Fortress Rollback into a production-grade, formally verified rollback networking library with >90% test coverage, absolute determinism guarantees, and exceptional usability.

---

## Current Status Summary

### Metrics
| Metric | Current | Target | Status |
|--------|---------|--------|--------|
| Unit Tests | 152 | 100+ | ✅ Exceeded |
| Total Tests | 194 | 150+ | ✅ Exceeded |
| Est. Coverage | ~87% | >90% | 🔄 Close |
| Clippy Warnings (lib) | 2 | 0 | 🔄 (too_many_arguments) |
| Panics from Public API | 0 | 0 | ✅ |
| HashMap Usage | 0 | 0 | ✅ |
| Miri Clean | 137/137 | All | ✅ |

### What's Complete ✅

- **Phase 1: Foundation & Safety** - Project rebrand, deterministic collections, panic elimination, structured telemetry, session observers, core unit tests, property-based testing (15 tests), runtime invariant checking, paranoid mode, CI/CD pipeline
- **Phase 1.6: Type Safety** - `Frame` newtype with arithmetic ops, `PlayerHandle` newtype with bounds checking
- **Phase 2.1: Miri Testing** - All 137 non-proptest library tests pass under Miri with no undefined behavior detected. Miri CI job added.
- **Phase 3.1: Integration Tests** - Multi-player (3-4 players), rollback scenarios (deep, frequent, with varying input delays), spectator synchronization

### Next Priority Actions
1. **🔴 Network Resilience & Adverse Conditions Testing (HIGH PRIORITY)** - Validate robustness under real-world network conditions:
   - **Latency scenarios**: Normal latency, high latency (200-500ms), extreme latency (1000ms+)
   - **Variable latency (jitter)**: Stable baseline with sudden spikes, oscillating latency patterns
   - **Packet loss**: Sporadic drops, burst loss, asymmetric loss (one direction worse)
   - **Timeouts & missing data**: Temporary disconnects, partial message delivery, reconnection scenarios
   - **Combined conditions**: High latency + packet loss, jitter + occasional timeouts
   - **Edge cases**: Out-of-order packets, duplicate packets, delayed input bursts
   - All correctness work (Miri, property tests, formal verification) should validate these scenarios
   - Requires ChaosSocket implementation as foundation
2. **ChaosSocket Implementation** - Required foundation for network condition simulation
3. **Documentation** - Architecture guide, user guide
4. **Benchmarking** - Set up criterion benchmarks

---

## Remaining Work

### Phase 2: Formal Verification (Continued)

#### 2.2 Kani Formal Verification (Optional)
- [ ] Set up `kani-verifier`
- [ ] Create proofs for InputQueue (buffer overflow, wraparound)
- [ ] Create proofs for SyncLayer (save/load inverses)

#### 2.3 Loom Concurrency Testing (If Needed)
- [ ] Test concurrent GameStateCell operations
- [ ] Verify no deadlocks in Mutex usage

### Phase 3: Comprehensive Test Coverage

#### 3.1 Integration Test Expansion
- [x] Multi-player scenarios (3-4 players) - Added `test_three_player_session` and `test_four_player_session`
- [ ] Network condition simulation (latency, packet loss)
- [x] Rollback scenarios (deep, frequent) - Added `test_deep_rollback_scenario`, `test_frequent_rollback_consistency`, `test_rollback_with_varying_input_delay`
- [x] Spectator scenarios - Basic tests exist (`test_synchronize_with_host`)

#### 3.2 Chaos Engineering & Network Resilience 🔴
- [ ] Implement `ChaosSocket` for fault injection (configurable latency, jitter, loss, reordering)
- [ ] **Latency Tests**
  - [ ] Constant high latency (100ms, 250ms, 500ms, 1000ms)
  - [ ] Latency spikes (baseline 50ms with periodic 500ms spikes)
  - [ ] Asymmetric latency (different delay per direction)
  - [ ] Gradually increasing latency (simulate degrading connection)
- [ ] **Packet Loss Tests**
  - [ ] Constant packet loss (10%, 25%, 50%)
  - [ ] Burst loss (lose 5-10 packets in a row, then normal)
  - [ ] Asymmetric loss (one direction has higher loss)
  - [ ] Correlated loss (loss probability increases after each drop)
- [ ] **Jitter Tests**
  - [ ] High variance latency (50ms ± 40ms)
  - [ ] Bimodal latency (alternating between 20ms and 200ms)
  - [ ] Latency with occasional extreme outliers
- [ ] **Timeout & Disconnect Tests**
  - [ ] Temporary full disconnect (1-5 seconds, then reconnect)
  - [ ] Partial message delivery (truncated packets)
  - [ ] One-way connectivity loss
  - [ ] Slow reconnection with input queue buildup
- [ ] **Packet Ordering Tests**
  - [ ] Out-of-order delivery (random reordering)
  - [ ] Duplicate packets
  - [ ] Delayed input bursts (queue then deliver all at once)
- [ ] **Combined Adverse Conditions**
  - [ ] High latency + packet loss
  - [ ] Jitter + burst loss
  - [ ] Latency spikes + temporary disconnects
- [ ] **Correctness Validation Under Stress**
  - [ ] Verify determinism maintained under all conditions
  - [ ] Verify no panics or UB under adverse conditions
  - [ ] Verify graceful degradation (frame stalls vs crashes)
  - [ ] Verify eventual consistency after conditions normalize

### Phase 4: Enhanced Usability

#### 4.1 Documentation
- [ ] Architecture guide (`docs/ARCHITECTURE.md`)
- [ ] User guide (`docs/USER_GUIDE.md`)
- [ ] Complete rustdoc with examples

#### 4.2 Examples
- [ ] Advanced configuration examples
- [ ] Error handling examples

### Phase 5: Performance

#### 5.1 Benchmarking
- [ ] Set up criterion benchmarks
- [ ] Benchmark core operations
- [ ] Track performance across commits

#### 5.2 Continuous Fuzzing
- [ ] Set up cargo-fuzz
- [ ] Create fuzz targets for message parsing

---

## Known Issues

### Dead Code in InputQueue
The `advance_queue_head` function contains gap-filling code for handling frame delay increases mid-session. This code can never be reached because `add_input` rejects inputs first due to its sequential frame check. Not exploitable via public API (frame delay is only set at construction), but represents dead code.

**Test:** `test_frame_delay_change_mid_session_drops_input`

### Remaining Tasks
- [ ] Reserve `fortress-rollback` on crates.io
- [ ] Protocol layer panic elimination (lower priority)
- [ ] Consider session type pattern for state machine enforcement (optional)

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

---

## Progress Log

### December 6, 2025 (Session 7)
- ✅ **Miri Testing Complete** - Ran all 137 non-proptest library tests under Miri with `-Zmiri-disable-isolation`. No undefined behavior in library code. Added Miri CI job to `.github/workflows/rust.yml`. Property tests excluded due to UB in upstream dependency (`ppv-lite86` via `rand_chacha`/`proptest`).
- ✅ **Multi-player Integration Tests** - Added `test_three_player_session` and `test_four_player_session` for 3-4 player P2P scenarios. Updated `StateStub::advance_frame` to support variable player counts.
- ✅ **Rollback Scenario Tests** - Added `test_deep_rollback_scenario` (check_distance=7, 500 frames), `test_frequent_rollback_consistency` (check_distance=1, 1000 frames), `test_rollback_with_varying_input_delay` (multiple delay values).

### December 6, 2025 (Sessions 5-6)
- ✅ **Type Safety** - `Frame` and `PlayerHandle` newtypes with arithmetic ops and bounds checking

### Earlier Sessions
- ✅ Runtime invariant checking, property-based testing (15 tests), protocol unit tests (32 tests), structured telemetry, panic elimination, HashMap→BTreeMap, rebrand, CI/CD
