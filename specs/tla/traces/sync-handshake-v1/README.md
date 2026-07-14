# SyncHandshakeV1 trace contract

This directory is the no-Rust-instrumentation feasibility spike for protocol trace validation. It
does not consume production logs and is not a runtime refinement proof. It establishes a strict
NDJSON schema, constrains every row through a real `SyncHandshakeV1` action, and proves that one
load-bearing impossible update is rejected. Fixtures specify complete deltas for the variables an
action may update; the wrapper carries the preceding values forward so TLC constrains the entire
post-action state, including variables that the fixture expects to remain unchanged.

Run it with:

```bash
python3 scripts/verification/verify-sync-handshake-traces.py
```

`verify-tla.sh SyncHandshakeV1` and the full TLA suite run the same contract automatically.

## Trace set

| Trace | Expected | Contract exercised |
| ----- | -------- | ------------------ |
| `matching.ndjson` | Accept | Matching configs, reordered replies, duplicate-token idempotence, and both peers reaching `Synced` |
| `mismatch.ndjson` | Accept | First-field mismatch, terminal failure, exactly-once events, and oriented ours/theirs reasons |
| `timeout.ndjson` | Accept | Event-only timeout followed by another enabled sync request |
| `duplicate-reply-decrement.ndjson` | Reject | Derives from `matching.ndjson` and changes only `syncRemaining.p1` from 1 to 0 for an already accepted reply token |

The reject file contains only mutation metadata. The wrapper materializes it from the accepted
baseline and refuses any undeclared row difference. TLC must accept the complete baseline and then
reject the derived trace specifically because `EventuallyTraceConsumed` cannot advance across the
impossible update. Parse failures, tool failures, timeouts, different properties, or unexpected exit
codes fail the check.

## Candidate runtime-to-spec refinement

This table is a contract for a later opt-in instrumentation patch, not a claim that the link exists
today.

| Runtime observation | Spec variable/action | Required normalization or known gap |
| ------------------- | -------------------- | ----------------------------------- |
| Endpoint synchronization status | `phase` | Map `Synchronizing` with no `handshake_failed` value to `Syncing`, `Running` to `Synced`, and `handshake_failed.is_some()` to `Failed`; production deliberately retains the `Synchronizing` enum variant after incompatibility |
| Local handshake block | `localConfig` | Trace only `num_players` and `input_bytes_per_player`; `min_compat_version`, features, fps, max prediction, desync interval, and the config digest remain outside this model |
| Decoded request/reply handshake block and source endpoint | `learnedConfig`, `learnedFrom` | Record at handler entry after source/connection checks; the model does not represent connection-ID binding |
| Request/reply enqueue and handler entry | `network`, named send/handle action | Emit ordered protocol-event deltas, not the simulation harness's end-of-step aggregate snapshots |
| `sync_remaining_roundtrips` | `syncRemaining` | Exact integer only when the trace config fixes the same `num_sync_packets` |
| Emitted random request IDs later consumed by valid replies | `acceptedTokens` | Normalize random `u32` IDs to trace-local ordinals; production currently retains outstanding IDs, not the model's consumed-token set |
| Next emitted request ordinal | `nextToken` | Trace-local ordinal only; never expose or compare raw random values as model tokens |
| Elapsed timeout threshold and one-shot event flag | `timeoutTicks`, `timeoutEventCount` | Normalize duration to pre-threshold/threshold states; record the event flag after emission |
| Incompatibility event and reason payload | `incompatibleEventCount`, `reasonField`, `reasonOurs`, `reasonTheirs` | Only the first represented mismatching field is modeled; public event translation remains a Rust-test obligation |

Any later logger must be opt-in, bounded, allocation-aware, and emitted at protocol transition
points. `TraceSnapshot`, `trace_hash`, and the final-64 simulation tail are explicitly not accepted
as substitutes for these ordered deltas.

## Measured spike budget

On the Session 129 development machine, the three positive traces and the expected-reject mutation
each explore 4–16 distinct states in about 0.4–0.6 seconds, under two seconds combined. These bounds
authorize design of opt-in protocol trace points; they do not authorize claiming runtime refinement
until real emitted traces pass this same contract and a runtime mutation remains rejected.
