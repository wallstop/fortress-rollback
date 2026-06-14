<p align="center">
  <img src="../../docs/assets/logo-small.svg" alt="Fortress Rollback" width="64">
</p>

# TLA+ Specifications for Fortress Rollback

This directory contains TLA+ specifications for formally verifying the correctness properties of Fortress Rollback.

## Quick Start

```bash
# Run all TLA+ verification (from project root)
./scripts/verification/verify-tla.sh

# List available specs
./scripts/verification/verify-tla.sh --list

# Verify specific spec
./scripts/verification/verify-tla.sh NetworkProtocol

# Quick verification (smaller bounds)
./scripts/verification/verify-tla.sh --quick
```

## Files

| File | Config | Status | Description |
|------|--------|--------|-------------|
| `NetworkProtocol.tla` | `NetworkProtocol.cfg` | ✓ CI | Sync-handshake + peer-drop state machine (N=3 peers) |
| `InputQueue.tla` | `InputQueue.cfg` | ✓ CI | Circular buffer input queue + graceful-drop freeze (`freeze_at`/`set_frozen_value_at`) |
| `Rollback.tla` | `Rollback.cfg` | ✓ CI | Rollback mechanism |
| `Concurrency.tla` | `Concurrency.cfg` | ✓ CI | GameStateCell thread safety |
| `ChecksumExchange.tla` | `ChecksumExchange.cfg` | ✓ CI | Checksum exchange for desync detection, per-(local,remote)-pair verdicts (N=3 peers) |
| `SpectatorSession.tla` | `SpectatorSession.cfg` | ✓ CI | Single-host spectator session with frame delay and catchup (multi-host failover is modeled separately in `SpectatorFailover.tla`) |
| `TimeSync.tla` | `TimeSync.cfg` | ✓ CI | Per-endpoint rolling-window frame-advantage (pinned N=2; the cross-endpoint aggregation is modeled separately in `FrameAdvantageAggregation.tla`, see cfg) |
| `PeerDrop.tla` | `PeerDrop.cfg` | ✓ CI | Halt vs ContinueWithout peer-drop policy model |
| `NPeerReactivation.tla` | `NPeerReactivation.cfg` | ✓ CI | N-peer mesh reconnection activation-frame agreement (Agreement C) (N=3 survivors) |
| `FreezeConvergence.tla` | `FreezeConvergence.cfg` | ✓ CI | Cross-survivor freeze-value convergence to the global-min agreed frame (the c25fc1f desync fix, N=3 survivors) |
| `FrameAdvantageAggregation.tla` | `FrameAdvantageAggregation.cfg` | ✓ CI | Cross-endpoint `max_frame_advantage` fold over N≥3 remotes — multi-handle idempotence, disconnect-gate exclusion, `i32::MIN→0` fallback (companion to `TimeSync.tla`) |
| `SpectatorFailover.tla` | `SpectatorFailover.cfg` | ✓ CI | Multi-host spectator connect-status merge — converge-down to live global-min freeze + provenance-gated reactivation under host failover (companion to `SpectatorSession.tla`; audit F4 / critic-#1 / critic-#2) |
| `DoubleFailureRelay.tla` | `DoubleFailureRelay.cfg` (+ `_Baseline.cfg`, `_Tombstone.cfg` demos) | ✓ CI | N≥4 "double-failure relay" freeze-barrier residual arbitration: reproduces the residual under the current fold (Baseline → safety violated), proves candidate fix #2 regresses liveness (Tombstone → liveness violated), and proves candidate fix #3 (mesh-acked-floor) sound (MeshAgree → safety+liveness PASS). The audit's last open potential-desync item (S41 REAL/deferred) |

## Properties Verified

### NetworkProtocol.tla

**Safety:**

- Valid state transitions only (Initializing → Synchronizing → Running → etc.)
- Sync remaining counter never negative
- Only Running state processes game inputs

**Liveness:**

- Eventually synchronized (under fair scheduling)
- No deadlock

### InputQueue.tla

**Safety (from formal-spec.md):**

- INV-4: Queue length bounded by `QUEUE_LENGTH` (128)
- INV-5: Head and tail indices always valid
- FIFO ordering preserved
- No frame gaps in queue
- Runtime input delay stays within queue capacity
- Mid-session delay increases preserve contiguous queued frames
- Frozen queues reject later adds and preserve the final confirmed input
- **Frozen-value determinism**: while frozen at a non-NULL agreed freeze frame
  `F` present in the ring, `last_confirmed_input` equals exactly the confirmed
  input at `F` — the frozen value is a deterministic function of `(F, ring)`,
  independent of which survivor froze or the freeze/re-roll order (the
  single-queue heart of the c25fc1f graceful-drop fix; models `freeze_at`,
  `set_frozen_value_at`, `roll_confirmed_input_to`)
- **Freeze-frame honesty**: `freezeFrame` is NULL exactly when no agreed-frame
  value claim is in force, and a recorded non-NULL agreed frame is always
  confirmable in the ring

**Liveness:**

- Predictions eventually confirmed (with rollback)

### FreezeConvergence.tla

Companion to `InputQueue.tla`'s freeze actions, lifting the single-queue
determinism to the **cross-survivor** level the audit flagged as unmodeled.
N survivors freeze a gracefully-dropped slot at possibly-different
locally-received frames (asymmetric loss) and converge their frozen value
**down** to the one global-min agreed frame `F` (via `set_frozen_value_at`).

**Safety:**

- `FrozenValueFaithful`: a survivor's repeated value is always the stream value
  at its current freeze frame (the per-survivor lift of frozen-value determinism)
- `FreezeFrameInRange`: no survivor freezes below the global min `F` or above
  what it actually received
- `ConvergedNoDesync`: once every survivor converges to `F`, the dropped slot's
  reported confirmed stream is byte-identical across all survivors — the
  desync-closing conclusion (the cross-survivor corollary at the fixpoint)

**Liveness:**

- `EventuallyConverged`: every survivor eventually converges to `F` (under weak
  fairness), so the mesh reaches the no-desync fixpoint — also proving
  `ConvergedNoDesync`'s `AllConverged` hypothesis is reachable (non-vacuous)

### FrameAdvantageAggregation.tla

Companion to `TimeSync.tla`. `TimeSync.tla` proves one endpoint's
`average_frame_advantage()` is bounded and deterministic; this spec models the
session-level fold that combines those per-endpoint averages across **all**
remote endpoints into the single `frames_ahead` value driving
`FortressEvent::WaitRecommendation` (`P2PSession::max_frame_advantage` →
`check_wait_recommendation`). It is the cross-peer aggregation the original
audit flagged as unmodeled at N≥3 — a constant bump of `TimeSync.tla` cannot
reach it (Session 27: the window spec has no cross-peer interaction), so the
per-endpoint average is abstracted (the same composition `FreezeConvergence.tla`
uses with `InputQueue.tla`'s ring). Checked at 3 remote endpoints (an N≥4-player
mesh from the local peer's view), one of them a 2-handle couch-co-op endpoint.

**Safety:**

- `FoldMatchesMaxSemantic`: the faithful per-handle / per-endpoint nested fold
  equals the order-**independent** max over connected endpoints — simultaneously
  the fold's correctness and its determinism (the result is invariant to
  `remotes.values()` / `endpoint.handles()` iteration order)
- `MultiHandleIdempotent`: folding a multi-handle endpoint once per handle yields
  the same result as folding it once — `max(x, x) = x`, never the additive `2x`
  (arbitrated finding F15 / completeness-critic #5, verbatim)
- `AggregateIsAContributorOrZero`: the result is always **some connected
  endpoint's** average, or 0 when none is connected — pinning both disconnect-gate
  exclusion (a dropped endpoint's average never wins) and the fallback in one
  statement
- `FallbackZero`: a fully-disconnected mesh aggregates to 0 — the `i32::MIN`
  sentinel never leaks
- `AggregateBounded`: the result stays within the per-endpoint advantage bound
- `RecommendationPositive`: any emitted `WaitRecommendation` carries a
  `skip_frames ≥ MIN_RECOMMENDATION` (never a spurious 0/negative for an in-sync
  or fully-disconnected mesh) — ties the fold to the public event

Each of the six safety properties is mutation-pinned (RED under a targeted
sabotage: additive fold, dropped disconnect gate, dropped `i32::MIN→0` fallback,
threshold-less recommend).

### SpectatorFailover.tla

Companion to `SpectatorSession.tla` (which models a single host). The production
spectator receives input broadcasts from a **vector of redundant hosts** and
**fails over** when the canonical host disconnects; this spec models the
connect-status merge (`merge_connection_status` + `converged_drop_status` +
`converge_latched_drop_status` + `reactivation_provenance`,
`src/sessions/p2p_spectator_session.rs`) that builds one convergent,
reactivation-safe latched view from the hosts' disagreeing (asymmetric-loss)
streams. One droppable player (the merge loops players independently), one drop
cycle with an optional genuine rejoin (the regime in which the provenance gate
is sound), in-order per-host delivery — see the `.tla` header for the scope and
the cross-cycle residuals (which need the future-work host→spectator epoch wire
signal) left out of model.

**Safety:**

- `LatchAtOrBelowLiveMin` (audit F4 / completeness-critic #2): a
  latched-disconnected slot's freeze is never above the global-min freeze across
  **live** hosts that staged a drop — the spectator replays
  `inputs[last_frame]`, so bounding it by the live min guarantees every
  surviving host confirmed that frame (no silent desync). Folds down at commit
  (`converged_drop_status`) and on late arrival (`converge_latched_drop_status`);
  never raised.
- `NoFalseResurrection` (audit critic-#1 / Session 31): once disconnected, the
  latch stays disconnected until the player **genuinely** rejoins — a stale
  lagging host that becomes canonical via failover but never witnessed the drop
  cannot re-open the slot (the `host_drop_witness` provenance gate). This is the
  stale-lagging-canonical (failover) guarantee; the within-cycle reordered-staging
  transient resurrection is a documented single-cycle production fail-open the
  in-order-staging model deliberately excludes (see the `.tla` SCOPE header).
- `GateAcceptsBoundaryWitness` (the availability dual): a genuine current-drop
  witness at **exactly** the converged freeze is classified Witnessed (the
  load-bearing `>=`, not `>`, in `reactivation_provenance`), so a real hot-join
  re-open is not wrongly frozen out.
- `FreezeNeverRaised`: while a slot stays latched-disconnected its freeze frame
  is non-increasing (the min / converge arms only lower it).

**Liveness:**

- `DropEventuallyLatched`: a real drop is eventually reflected in the latch (or
  the player genuinely rejoins) — the spectator does not ignore a drop the hosts
  agree on.

Each safety property is mutation-pinned (RED under a targeted sabotage):
neutralizing **either** convergence fold (commit-time or late-arrival) breaks
`LatchAtOrBelowLiveMin` — proving both are load-bearing; a `min→max` re-commit
breaks `FreezeNeverRaised`; removing the provenance NULL-witness guard (the
pre-Session-31 unconditional follow) breaks `NoFalseResurrection` via a
host-failover stale-resurrection trace; tightening the gate `>=` to `>` breaks
`GateAcceptsBoundaryWitness`. Disabling either WF assumption breaks the liveness.
Reachability probes confirm the interesting states (converge-down, genuine
follow, failover-while-disconnected, latch-disconnected, gate boundary) are
non-vacuously reached.

### DoubleFailureRelay.tla

Arbitrates the N≥4 **"double-failure relay"** freeze-barrier residual — the audit's
last open potential-desync item, arbitrated REAL with the fix deferred-with-spec in
Session 41. The S32 freeze barrier bounds each survivor's confirmed frame for a
dropping slot by the mesh-gossip minimum, and both that **bound**
(`remote_slot_confirmed_bound`) and the freeze **override**
(`update_player_disconnects`) iterate the identical `is_running()` endpoint set — so
confirmation can outrun a later-agreed freeze **only** when the low value's source
endpoint LEAVES the fold (fold-membership asymmetry). The model captures the fold,
the per-endpoint gossip caches (which can go stale under partition), endpoint death
(pruning), the prediction window (irreversible discard of confirmed frames below the
window floor), and the freeze re-roll.

The confirmation rule is a `FIX_MODE` constant exercised by three configs:

- **Baseline** (`DoubleFailureRelay_Baseline.cfg`, expected to FAIL): the current
  production fold. TLC reproduces the residual as a **safety** counterexample
  (`NoConfirmedDivergence` / `LockedRecordMatchesFreeze` violated) — the model-level
  RED mirroring the in-process repro. The global-min source dies and is pruned, the
  victim confirms+discards real inputs on a stale-high cache, and a late relay lowers
  the freeze below the already-discarded window.
- **Tombstone** (`DoubleFailureRelay_Tombstone.cfg`, expected to FAIL on the
  PROPERTY): candidate fix #2 (keep folding a dead survivor's last term). Safety
  holds, but the **liveness** property `ConfirmationProgresses` is violated — a dead
  laggard's retained low term pins a still-live slot's confirmation below the living
  floor forever (a survivor cannot tell a real freeze from ordinary lag at the moment
  of death). The formal proof of the project rule "no partial fix — a partial fix
  regresses liveness."
- **MeshAgree** (`DoubleFailureRelay.cfg`, the default, ✓ CI): candidate fix #3.
  Confirmation of a not-yet-mesh-agreed slot advances only to the **mesh-acked
  floor** (the min over every alive peer reachable on a live link of that peer's
  *current* freeze floor, read via a fresh ack — not the stale cache), holds while
  partitioned, and excludes the slot to MAX only once its freeze has fully converged.
  **Both** the safety invariants and the liveness property hold.

**Safety:** `NoConfirmedDivergence` (no two alive survivors permanently — both
window-locked — disagree on the dropped slot's recorded confirmed value);
`LockedRecordMatchesFreeze` (a mesh-agreed survivor's locked record equals the agreed
freeze value); plus `FreezeNeverBelowGlobalMin` and `RecordedSourceInRange` sanity
invariants. **Liveness:** `ConfirmationProgresses` (every alive survivor eventually
confirms to the living mesh floor; partitions heal monotonically). The three configs
together are the machine-checked arbitration: the residual is real (Baseline), the
obvious non-wire fix regresses liveness (Tombstone), and the mesh-acked-floor design
is sound (MeshAgree). The MeshAgree config is the design a future production
red-green cycle should implement (a per-slot ack/epoch on connect-status gossip).

### PeerDrop.tla

**Safety:**

- Halt transitions to `Synchronizing` after any dropped peer
- ContinueWithout freezes dropped players and keeps survivors independent
- `PeerDropped` events are emitted only by ContinueWithout
- Dropped players are excluded from survivor progress
- Rollback starts no later than every dropped player's `lastFrame + 1`

### NPeerReactivation.tla

Models "Agreement C" of N-peer mesh reconnection (1 coordinator, 3 survivors,
1 joiner) from `progress/session-18-npeer-mesh-reconnection-design.md` (§4.C/§5/§8).

**Safety:**

- `Agreement` (S1): any two peers (coordinator, any survivor, joiner) that both
  committed a frame committed the same committed **value** (the bytes in `val`, *not*
  the `mode`/value-source label) at that frame
- `NoConfirmedRewrite` (S2): committed history never reverts (every frame within `committedUpTo` stays definite)
- `NoSplitBrainOnAbort` (L1): no aborted state has the joiner real-at-`F` while a survivor is frozen-at-`F`

**Liveness (under weak fairness):**

- `EventuallyResolved`: the protocol always reaches a terminal `joined`/`aborted` state

**The cap and the abort lifecycle are modeled non-vacuously (have teeth):**

- Survivors start with `committedUpTo` *below* `L` and **race toward `F`** by committing
  frozen frames (`SurvivorAdvanceFrozen`). The keepalive-preserved cap (design §4.C —
  "confirmed = min over connected incl. K") **holds each survivor at `F-1 = L`**: a
  survivor cannot commit frame `F` frozen while `capHeld` is true. So the cap is an
  exercised, live constraint, not an assertion by construction.
- `CapCollapse` models the cap-collapse hazard (the coordinator dropping out of a
  survivor's connected-min mid-pause). The protocol's keepalive serve (`keepaliveServing`)
  is its guard: while the paused coordinator is serving, `CapCollapse` cannot fire.
- The joiner follows the **late-apply lifecycle** (§5): `JoinerBuffer` buffers the
  snapshot without applying it; `JoinerCommit` makes the joiner real-at-`F` *only* after
  every survivor has reopened (the `JoinCommitted` gate); `JoinerAbortDiscard` discards
  the buffer on abort, so the joiner is never real-at-`F` after an abort.
- Removing either guard (a scratch *naive* variant: `CapCollapse` ungated **and** the
  joiner committing real-at-`F` eagerly on buffer) makes TLC find a `NoSplitBrainOnAbort`
  counterexample — confirming both the cap and the gated commit are load-bearing.

**Precondition P-A is checkable, not assumed:** survivors hold *independent* frozen
values. The default config pins P-A (`AssumePA = TRUE`) and passes. `NPeerReactivation_NoPA.cfg`
(`AssumePA = FALSE`, not registered in CI) drops P-A and TLC reports an `Agreement`
counterexample — demonstrating P-A is necessary.

### Rollback.tla

**Safety (from formal-spec.md):**

- INV-1: Frame monotonicity (except during rollback)
- INV-2: Rollback target is valid frame
- INV-7: Confirmed frame in valid range
- INV-8: Saved frame in valid range
- SAFE-4: Rollback restores correct state (state exists for rollback target)

**Liveness (disabled for CI due to state space):**

- LIVE-4: Rollback completes

### Concurrency.tla

**Safety:**

- Mutual exclusion: At most one thread holds lock at a time
- No data races: Only lock holder can modify cell state
- Frame consistency: After save, cell frame matches saved frame
- Load returns saved: Load operation returns correct data
- Valid frame after save: Save never stores NULL_FRAME
- Wait queue FIFO: Threads acquire lock in request order

**Liveness:**

- No deadlock: Some action always enabled
- Operations complete: Every started operation eventually completes
- Fair lock acquisition: Waiting threads eventually get the lock

**Linearizability:**

- Operations appear atomic (guaranteed by mutex)

### ChecksumExchange.tla

All verdict state (`pendingChecksums`/`syncHealth`/`lastVerifiedFrame`) is keyed by ordered
(local, remote) peer pairs, mirroring the implementation's per-`UdpProtocol` endpoint state
(the F12 fix in `src/sessions/p2p_session.rs`). Checked at `PEERS = {p1, p2, p3}`,
`MAX_FRAME = 3`, `CHECKSUM_INTERVAL = 1` — the smallest model with two comparable checksum
frames per pair. Tractability at N=3 needs both `SYMMETRY Permutations(PEERS)` (sound: the
module's only `CHOOSE` ranges over integer frames, never peers, and liveness is disabled)
and an in-flight network cap of 2 (one broadcast outstanding; sound because
`ReceiveChecksum` commutes with every other action — see the `StateConstraint` comment in
the spec). Measured single-worker: N=3 completes in ~106s with 11,674,741 states generated
/ 1,469,194 distinct (depth 62); N=2 in ~1s with 38,160 generated / 9,167 distinct (depth
32). Without the cap-2 constraint and symmetry the same bounds do NOT terminate in CI
budget (killed at 28.6 min, 233M+ generated / 31.7M+ distinct, queue still growing).

**Safety:**

- Checksums are computed deterministically for each frame
- Checksum reports are only sent for confirmed frames
- No false positives, pair-precise: a `DesyncDetected` verdict for (p,q) requires p or q
  to have actually diverged
- Desync verdicts are terminal per pair (a match against one remote never clears another
  pair's verdict — the Session 27 cross-pair clobbering regression guard)
- `last_verified_frame` is monotonically increasing per pair
- F12 leakage guards: `InSync` for (p,q) requires a matching checksum in pair (p,q) itself;
  the `is_synchronized()` aggregate requires every pair individually verified; the
  `last_verified_frame()` aggregate is the min over pairs. The two aggregate invariants
  (`SynchronizedRequiresAllPairsVerified`, `AggregateVerifiedFrameSound`) are tautological
  given the current aggregate definitions — they are kept as regression tripwires against
  someone redefining the aggregates, not as added verification strength

**Liveness:**

- Defined but disabled (premises are unsound under a late `IntroduceDesync`; same as the
  earlier N=2 model)

## Running the Specifications

### Automated (Recommended)

Use the verification scripts from the project root:

```bash
# Run all enabled TLA+ specs in CI
./scripts/verification/verify-tla.sh

# Run specific spec
./scripts/verification/verify-tla.sh InputQueue

# Run quick verification (smaller state bounds)
./scripts/verification/verify-tla.sh --quick

# List available specs
./scripts/verification/verify-tla.sh --list
```

The script automatically downloads TLC tools if needed.

### Prerequisites for Manual Verification

1. Install TLA+ Toolbox or TLC command-line tools
2. Download from: <https://lamport.azurewebsites.net/tla/tools.html>

### Using TLA+ Toolbox

1. Open TLA+ Toolbox
2. File → Open Spec → Add New Spec
3. Select one of the `.tla` files
4. Create a new model (Model → New Model)
5. Configure constants (see `.cfg` files for values)
6. Run Model Checker

### Configuration Files

Each spec has a `.cfg` file with TLC-compatible settings:

| Config File | Key Constants | State Space |
|-------------|---------------|-------------|
| `NetworkProtocol.cfg` | PEERS={p1,p2,p3}, NUM_SYNC_PACKETS=1 | ~170,000 distinct states (~2.6M generated) |
| `InputQueue.cfg` | QUEUE_LENGTH=3, MAX_FRAME=4, NULL_FRAME=999 | ~56,500 distinct states (~1.07M generated) |
| `Concurrency.cfg` | MAX_FRAME=4 | Small |
| `Rollback.cfg` | MAX_PREDICTION=1, MAX_FRAME=3 | ~1.8M distinct states (~29.2M generated) |
| `ChecksumExchange.cfg` | PEERS={p1,p2,p3}, MAX_FRAME=3, SYMMETRY | ~1.47M distinct states (~11.7M generated), ~106s single worker |
| `FreezeConvergence.cfg` | SURVIVORS={s1,s2,s3}, MAX_FRAME=3, NULL_FRAME=999 (no symmetry — liveness) | ~24,100 distinct states (~79,000 generated) |
| `FrameAdvantageAggregation.cfg` | NUM_ENDPOINTS=3, MAX_ADVANTAGE=4, MULTI_HANDLE_COUNT=2, MIN_RECOMMENDATION=3 (no symmetry) | ~26,200 distinct states (~901,000 generated) |
| `SpectatorFailover.cfg` | HOSTS={1,2,3}, MAX_FRAME=3, NULL_FRAME=999 (no symmetry — canonical=min(live), liveness) | ~96,800 distinct states (~446,000 generated), ~6s single worker |
| `DoubleFailureRelay.cfg` | SURVIVORS={a,b,c}, MAX_FRAME=3, WINDOW=1, RECEIPTS={0,3}, FIX_MODE="MeshAgree" (no symmetry — liveness; links monotone-heal, weak fairness) | ~865,600 distinct states (~3.88M generated), ~2min single worker |

**Note on NULL_FRAME:** TLC config files don't support negative numbers,
so we use `NULL_FRAME = 999` as a sentinel value instead of -1.

### Legacy Manual Configurations

For TLA+ Toolbox or custom model checking:

#### NetworkProtocol.tla

```
CONSTANTS
    NUM_SYNC_PACKETS = 3    \* Reduced from 5 for faster checking
    PEERS = {p1, p2}        \* Two peers

INVARIANT
    TypeInvariant
    SyncRemainingNonNegative

PROPERTY
    ValidStateTransitions
```

#### InputQueue.tla

```
CONSTANTS
    QUEUE_LENGTH = 8        \* Reduced from 128 for faster checking
    MAX_FRAME = 20
    NULL_FRAME = -1

INVARIANT
    SafetyInvariant
```

#### Rollback.tla

```
CONSTANTS
    MAX_PREDICTION = 4      \* Reduced from 8 for faster checking
    MAX_FRAME = 15
    NUM_PLAYERS = 2
    NULL_FRAME = -1

INVARIANT
    SafetyInvariant

PROPERTY
    RollbackCompletes
```

#### Concurrency.tla

```
CONSTANTS
    THREADS = {t1, t2}      \* Two threads for basic checking
    MAX_FRAME = 5           \* Reduced for faster checking
    NULL_FRAME = -1

INVARIANT
    SafetyInvariant

PROPERTY
    OperationsComplete
    FairLockAcquisition
```

### Command Line

```bash
# Check NetworkProtocol
java -jar tla2tools.jar -config NetworkProtocol.cfg NetworkProtocol.tla

# Check InputQueue
java -jar tla2tools.jar -config InputQueue.cfg InputQueue.tla

# Check Rollback
java -jar tla2tools.jar -config Rollback.cfg Rollback.tla

# Check Concurrency
java -jar tla2tools.jar -config Concurrency.cfg Concurrency.tla
```

## Relationship to Implementation

These specifications model the key algorithms from:

| TLA+ Module | Rust Implementation |
|-------------|---------------------|
| `NetworkProtocol` | `src/network/protocol/mod.rs` (UdpProtocol) |
| `InputQueue` | `src/input_queue/mod.rs` (InputQueue; `freeze_at`, `set_frozen_value_at`, `roll_confirmed_input_to`, `confirmed_input`) |
| `Rollback` | `src/sync_layer/mod.rs` (SyncLayer), `src/sessions/p2p_session.rs` |
| `Concurrency` | `src/sync_layer/game_state_cell.rs` (GameStateCell, GameStateAccessor) |
| `ChecksumExchange` | `src/sessions/p2p_session.rs` (sync_health, is_synchronized, last_verified_frame, compare_local_checksums_against_peers, check_checksum_send_interval), `src/network/protocol/mod.rs` (pending_checksums, last_verified_frame, on_checksum_report), `src/network/messages.rs` (ChecksumReport) |
| `FreezeConvergence` | `src/input_queue/mod.rs` (`freeze_at`, `set_frozen_value_at`, `roll_confirmed_input_to`), `src/sessions/p2p_session.rs` (`update_player_disconnects`, `disconnect_player_at_frames`, `remote_disconnect_snapshot`), `src/sync_layer/mod.rs` (frozen-slot bypass in `synchronized_inputs`) |
| `TimeSync` | `src/time_sync.rs` (TimeSync; `advance_frame`, `average_frame_advantage`) |
| `FrameAdvantageAggregation` | `src/sessions/p2p_session.rs` (`max_frame_advantage`, `check_wait_recommendation`, `frames_ahead`), `src/network/protocol/mod.rs` (`average_frame_advantage`, `handles`), `src/lib.rs` (`FortressEvent::WaitRecommendation`) |
| `SpectatorFailover` | `src/sessions/p2p_spectator_session.rs` (`merge_connection_status`, `converged_drop_status`, `converge_latched_drop_status`, `reactivation_provenance`, `witness_host_drop_reports`, `consume_drop_witnesses`, `witness_adopted_drop`, `commit_canonical_snapshot`, `host_drop_witness`, `host_connect_status`) |
| `DoubleFailureRelay` | `src/sessions/p2p_session.rs` (`remote_slot_confirmed_bound`, `update_player_disconnects`, `confirmed_frame`, the freeze-barrier fold and `!endpoint.is_running()` skip), `src/network/protocol/mod.rs` (`merge_peer_connect_status`, `is_running`); models the N≥4 residual the in-process test `tests/sessions/peer_drop.rs::p2p_n4_double_failure_relay_dropped_slot_diverges_across_survivors` characterizes. The MeshAgree fix is a *design* (mesh-acked-floor / per-slot ack-epoch) for a future production red-green cycle — not yet implemented in `src/` |

## Extending the Specifications

### Adding New Properties

1. Define the property in TLA+ temporal logic
2. Add to the PROPERTY section of the model
3. Run model checker to verify

### Modeling New Features

1. Add variables to represent new state
2. Update `Init` for initial values
3. Add actions for state transitions
4. Update `Next` to include new actions
5. Add invariants/properties to verify

## Best Practices for TLA+ Development

> **See also:** [/.llm/skills/formal-verification/verification.md](../../.llm/skills/formal-verification/verification.md) — TLA+ and Z3 verification guide

### Specification Design

| Do | Don't |
|----|-------|
| Model essential behavior | Model implementation details |
| Use small constants for checking | Start with production-sized constants |
| Write type invariants first | Skip type checking |
| Add helper operators | Write monolithic actions |
| Separate safety and liveness | Mix them in one model run |

### Common Patterns

```tla
(* State machines: explicit states *)
States == {"Init", "Running", "Done"}
Trans(from, to) == state = from /\ state' = to

(* Bounded resources *)
ASSUME MAX_VALUE > 0
x' \in 0..MAX_VALUE

(* Nondeterministic choice *)
\/ action_a
\/ action_b

(* Existential: any valid value *)
\E v \in ValidValues: x' = v
```

### When Verification Fails

1. **Read the counterexample trace** — TLC shows exact state sequence
2. **Check if spec matches intent** — Is the model correct?
3. **Check if code matches spec** — Is the implementation correct?
4. **Add regression test** — Capture the bug scenario in Rust tests

### Performance Tips

- Start with tiny constants (MAX_FRAME=3, NUM_PLAYERS=2)
- Run safety checks before liveness
- Use `-workers auto` for parallel checking
- Add state constraints to limit exploration

## References

- [TLA+ Resources](https://lamport.azurewebsites.net/tla/tla.html)
- [Learn TLA+](https://learntla.com/)
- [TLA+ Video Course](https://lamport.azurewebsites.net/video/videos.html)
- [Specifying Systems (book)](https://lamport.azurewebsites.net/tla/book.html)
