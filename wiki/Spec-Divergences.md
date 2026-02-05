<p align="center">
  <img src="assets/logo-small.svg" alt="Fortress Rollback" width="64">
</p>

# Spec-Production Divergences

**Version:** 1.3
**Date:** December 15, 2025
**Status:** Documented

This document lists intentional divergences between formal specifications and production code, with justification for each.

---

## Intentional Divergences

### 1. TLA+ Model Checking Constants

| Constant | TLA+ Value | Production Value | Justification |
|----------|------------|------------------|---------------|
| `QUEUE_LENGTH` | 3 | 128 | Small values keep state space tractable for exhaustive model checking. The invariants (INV-4, INV-5) hold regardless of queue size. |
| `MAX_PREDICTION` | 1-3 | 8 (default) | Small values allow TLC to explore all states. The rollback boundedness property (INV-2) is size-independent. |
| `MAX_FRAME` | 3-4 | ~2 billion | Model checking bounds. Frame arithmetic correctness is verified by Z3 and Kani. |
| `NULL_FRAME` | 999 | -1 | TLA+ uses natural numbers; 999 serves as sentinel outside valid frame range. Production uses -1 as the standard null sentinel. |
| `NUM_PLAYERS` | 2 | configurable | Model checking focuses on 2-player case. Properties generalize to N players. |

**Verification Strategy:** TLA+ proves invariant structure holds; Z3 and Kani verify with production-scale values where tractable.

### 2. Kani Verification Constants

| Constant | Kani Value | Production Value | Justification |
|----------|------------|------------------|---------------|
| `INPUT_QUEUE_LENGTH` | 8 | 128 | Kani's symbolic execution is exponential in array size. 8 elements is sufficient to prove circular buffer invariants (wrap-around, bounds checking). |

**Verification Strategy:** The invariants are size-independent; proving them for 8 elements implies they hold for 128.

### 3. Z3 SMT Constants

Z3 proofs use production values (128, -1, 8) where possible, providing direct verification of production behavior.

---

## Verified Alignment

The following aspects are verified to be perfectly aligned:

### State Machine Alignment

| Component | TLA+ Spec | Production Code | Status |
|-----------|-----------|-----------------|--------|
| `ProtocolState` enum | `NetworkProtocol.tla` | `src/network/protocol.rs:220` | ✅ Aligned |
| State transitions | `NetworkProtocol.tla` | `protocol.rs` various methods | ✅ Aligned |
| `SyncLayer` fields | `Rollback.tla` | `src/sync_layer.rs:263` | ✅ Aligned |
| `InputQueue` fields | `InputQueue.tla` | `src/input_queue.rs:100` | ✅ Aligned |

### Prediction Strategy Alignment (Fixed Dec 15, 2025)

| Aspect | TLA+ Spec | Production Code | Status |
|--------|-----------|-----------------|--------|
| Prediction source | `lastConfirmedInput` | `last_confirmed_input` | ✅ Aligned |
| Update timing | Updated on every `AddInput`/`AddRemoteInput` | Updated on every `add_input_by_frame` | ✅ Aligned |

**IMPORTANT**: Fortress Rollback uses `last_confirmed_input` (updated when any input is added)
as the source for predictions, NOT a separate `prediction` variable like original GGPO.
This is intentional and ensures determinism across peers because:

1. Confirmed inputs are synchronized via the network protocol
2. Both local and remote inputs are "confirmed" by the time they reach the queue
3. The `RepeatLastConfirmed` strategy uses this synchronized value

The TLA+ spec was updated (Dec 15, 2025) to use `lastConfirmedInput` variable to match
production naming and document this design decision.

### Invariant Implementation

| Invariant | Spec Location | Code Location | Status |
|-----------|---------------|---------------|--------|
| INV-1 (Frame Monotonicity) | `Rollback.tla:FrameMonotonicity` | `sync_layer.rs:advance_frame`, `load_frame` | ✅ Verified |
| INV-2 (Rollback Bounds) | `Rollback.tla:RollbackBounded` | `sync_layer.rs:394` | ✅ Verified |
| INV-4 (Queue Length) | `InputQueue.tla:QueueLengthBounded` | `input_queue.rs:check_invariants` | ✅ Verified |
| INV-5 (Queue Index) | `InputQueue.tla:QueueIndexValid` | `input_queue.rs:check_invariants` | ✅ Verified |
| INV-7 (Confirmed Frame) | `Rollback.tla:ConfirmedFrameConsistency` | `sync_layer.rs:check_invariants` | ✅ Verified |
| INV-8 (Saved Frame) | `Rollback.tla:SavedFrameConsistency` | `sync_layer.rs:check_invariants` | ✅ Verified |

### Default Values

| Parameter | Spec Value | Production Value | Status |
|-----------|------------|------------------|--------|
| `DEFAULT_MAX_PREDICTION` | 8 | 8 | ✅ Aligned |
| `DEFAULT_DISCONNECT_TIMEOUT` | 2000ms | 2000ms | ✅ Aligned |
| `DEFAULT_FPS` | 60 | 60 | ✅ Aligned |
| `NUM_SYNC_PACKETS` | 5 | 5 | ✅ Aligned |

---

## Audit Methodology

1. **Grep for constants** - Compared all named constants across TLA+, Z3, Kani, and production code
2. **State machine review** - Verified state enum values and transitions match
3. **Invariant trace** - Linked each formal-spec.md invariant to both spec and code implementation
4. **Documentation linkage** - Added `# Formal Specification Alignment` sections to key production code

---

## Configurable Constants (Phase 9/10)

Several constants are now configurable via builder config structs. This section documents
how TLA+ constants map to production configuration.

### TLA+ Constants → Production Config Mapping

| TLA+ Constant | Config Struct | Field | Default | Notes |
|---------------|---------------|-------|---------|-------|
| `QUEUE_LENGTH` | `InputQueueConfig` | `queue_length` | 128 | TLA+ uses 3 for tractability |
| `MAX_PREDICTION` | `SessionBuilder` | `max_prediction` | 8 | TLA+ uses 1-3 for tractability |
| `NUM_SYNC_PACKETS` | `SyncConfig` | `num_sync_packets` | 5 | Same in TLA+ and production |
| `NUM_PLAYERS` | `SessionBuilder` | (player count) | 2 | TLA+ focuses on 2-player case |

### Production Config Structs Not Modeled in TLA+

The following config structs control **timing/thresholds** rather than **protocol behavior**,
so they don't have TLA+ counterparts:

| Config Struct | Purpose | Why Not in TLA+ |
|---------------|---------|-----------------|
| `ProtocolConfig` | Quality reports, shutdown delay, thresholds | Timing details, not state machine behavior |
| `SpectatorConfig` | Buffer size, catchup speed | Spectator protocol not separately modeled |
| `TimeSyncConfig` | Averaging window size | Time sync is implementation detail |

### All Configurable Constants

| Constant | Config Struct | Field | Default | Range |
|----------|---------------|-------|---------|-------|
| `INPUT_QUEUE_LENGTH` | `InputQueueConfig` | `queue_length` | 128 | 2+ |
| `MAX_FRAME_DELAY` | (derived) | `queue_length - 1` | 127 | - |
| `NUM_SYNC_PACKETS` | `SyncConfig` | `num_sync_packets` | 5 | 1+ |
| `MAX_PREDICTION` | `SessionBuilder` | `max_prediction` | 8 | 1+ |
| `WINDOW_SIZE` | `TimeSyncConfig` | `window_size` | 30 | 1+ |
| `SPECTATOR_BUFFER` | `SpectatorConfig` | `buffer_size` | 60 | 1+ |

### Presets Available

**InputQueueConfig:**

| Preset | Queue Length | Use Case |
|--------|--------------|----------|
| `standard()` | 128 | Default, ~2.1s at 60 FPS |
| `high_latency()` | 256 | High latency networks, ~4.3s at 60 FPS |
| `minimal()` | 32 | Memory-constrained, ~0.5s at 60 FPS |

**SyncConfig:**

| Preset | Sync Packets | Retry Interval | Use Case |
|--------|--------------|----------------|----------|
| `default()` | 5 | 200ms | Standard networks |
| `high_latency()` | 5 | 400ms | High latency (100-200ms RTT) |
| `lossy()` | 8 | 200ms | Lossy networks (5-15% loss) |
| `lan()` | 3 | 100ms | LAN play |

### Verification Strategy

The invariants proven in TLA+, Kani, and Z3 are **size-independent**:

1. **TLA+**: Uses small constants (e.g., `QUEUE_LENGTH=3`) for model checking tractability.
   The invariants (INV-4: length bounded, INV-5: valid indices) hold for ANY `QUEUE_LENGTH >= 2`.

2. **Kani**: Uses `INPUT_QUEUE_LENGTH=8` for symbolic execution tractability.
   Circular buffer arithmetic and bounds checking are independent of actual size.

3. **Z3**: Uses default values (128) but proves properties that hold for any valid size.
   Frame arithmetic and modulo operations work identically for any queue length.

Therefore, proofs passing for small queue lengths imply correctness for production sizes (32, 128, 256).

---

## Future Maintenance

When modifying specs or production code:

1. **Check this document** - Ensure changes don't introduce unintended divergences
2. **Update linkage comments** - If code moves, update the spec linkage comments
3. **Run all verification** - `./scripts/verify-all.sh` validates TLA+, Kani, and Z3

---

## Revision History

| Version | Date | Changes |
|---------|------|---------|
| 1.3 | 2025-12-15 | Fixed InputQueue.tla to use `lastConfirmedInput` matching production's `last_confirmed_input` (was incorrectly named `prediction`, which modeled original GGPO behavior) |
| 1.2 | 2025-12-09 | Comprehensive config struct documentation: SyncConfig, ProtocolConfig, SpectatorConfig, TimeSyncConfig |
| 1.1 | 2025-12-09 | Phase 10: Documented configurable constants (InputQueueConfig) |
| 1.0 | 2025-12-09 | Initial audit documenting all divergences and alignments |
