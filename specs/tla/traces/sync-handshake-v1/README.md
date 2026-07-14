# SyncHandshakeV1 trace contract

This directory contains the hand-authored feasibility fixtures for protocol trace validation. The
same gate now also generates ephemeral traces from real feature-gated `P2PSession` endpoints over
SimNet. It establishes a strict NDJSON schema, constrains every row through a real
`SyncHandshakeV1` action, and proves that both the hand-authored and runtime-derived load-bearing
impossible updates are rejected. Every trace specifies complete deltas for the variables an action
may update; the wrapper carries preceding values forward so TLC constrains the entire post-action
state, including variables expected to remain unchanged.

Run it with:

```bash
python3 scripts/verification/verify-sync-handshake-traces.py
```

`verify-tla.sh SyncHandshakeV1` and the full TLA suite run the same contract automatically.

The runtime files are intentionally ephemeral. The Rust producer emits them into a temporary
directory on each gate run, and the Python wrapper validates their exact four-file manifest before
TLC sees them. This prevents stale checked-in output from standing in for current runtime behavior.

## Trace set

| Trace | Expected | Contract exercised |
| ----- | -------- | ------------------ |
| `matching.ndjson` | Accept | Matching configs; fresh request IDs A, B, and C; acceptance of C before outstanding B; a genuinely duplicated A reply remaining idempotent; and both peers reaching `Synced` |
| `mismatch.ndjson` | Accept | First-field mismatch, terminal failure, exactly-once events, and oriented ours/theirs reasons |
| `timeout.ndjson` | Accept | Event-only timeout followed by another enabled sync request |
| `duplicate-reply-decrement.ndjson` | Reject | Derives from `matching.ndjson` and changes only `syncRemaining.p1` from 1 to 0 for an already accepted reply token |

The reject file contains only mutation metadata. The wrapper materializes it from the accepted
baseline and refuses any undeclared row difference. TLC must accept the complete baseline and then
reject the derived trace specifically because `EventuallyTraceConsumed` cannot advance across the
impossible update. Parse failures, tool failures, timeouts, different properties, or unexpected exit
codes fail the check.

## Runtime-to-spec refinement boundary

The opt-in protocol recorder and two-peer SimNet driver now supply and normalize the runtime
observations in this table. The result is a narrow refinement check for the modeled handshake
projection, not a whole-protocol or whole-session refinement proof.

| Runtime observation | Spec variable/action | Required normalization or known gap |
| ------------------- | -------------------- | ----------------------------------- |
| Endpoint synchronization status | `phase` | Map `Synchronizing` with no `handshake_failed` value to `Syncing`, `Running` to `Synced`, and `handshake_failed.is_some()` to `Failed`; production deliberately retains the `Synchronizing` enum variant after incompatibility |
| Local handshake block | `localConfig` | Trace only `num_players` and `input_bytes_per_player`; `min_compat_version`, features, fps, max prediction, desync interval, and the config digest remain outside this model |
| Decoded request/reply handshake block and source endpoint | `learnedConfig`, `learnedFrom` | Record at handler entry after source/connection checks; the model does not represent connection-ID binding |
| Request/reply enqueue and handler entry | `network`, named send/handle action | Emit ordered protocol-event deltas, not the simulation harness's end-of-step aggregate snapshots |
| `sync_remaining_roundtrips` | `syncRemaining` | Exact integer only when the trace config fixes the same `num_sync_packets` |
| Emitted random request IDs later consumed by valid replies | `acceptedTokens` | Normalize random `u32` IDs to fresh trace-local ordinals from a bounded namespace independent of `NUM_SYNC_PACKETS`; production currently retains outstanding IDs, not the model's consumed-token set |
| Next emitted request ordinal | `nextToken` | Monotonic trace-local ordinal only; the bounded model disables further sends when its namespace is exhausted instead of wrapping and reusing an identity; never expose or compare raw random values as model tokens |
| Elapsed timeout threshold and one-shot event flag | `timeoutTicks`, `timeoutEventCount` | Normalize duration to pre-threshold/threshold states; record the event flag after emission |
| Incompatibility event and reason payload | `incompatibleEventCount`, `reasonField`, `reasonOurs`, `reasonTheirs` | Only the first represented mismatching field is modeled; public event translation remains a Rust-test obligation |

The recorder is opt-in, fixed-capacity, fallibly reserved, and emitted at protocol transition
points; overflow and raw-ID collisions fail closed. The hidden builder caps each endpoint at 64
fixed records. The driver collects newly appended records after each fixed-order peer poll, maps
equal raw IDs to equal trace-local ordinals, reconstructs the one observed network duplication,
and rejects a fourth fresh request beyond the model namespace. Phase, remaining roundtrips,
timeout, incompatibility, and outstanding-cardinality values are projected from each raw post-state
rather than recreated from action labels. `TraceSnapshot`, `trace_hash`, and the final-64 simulation
tail are explicitly not accepted as substitutes for these ordered deltas.

## Measured spike budget

The wrapper prints each observed duration and enforces a 60-second per-case timeout by default.
Development runs normally complete each case within a few seconds, but host load changes the total;
historical sub-second per-case measurements are descriptive, not a strict sub-two-second or
2.2-second acceptance threshold. The hand-authored positives contain 18, 4, and 4 trace rows. The
runtime positives contain 17, 7, and 6 rows; their generated negative retains all 17 matching rows
and changes only the ignored duplicate's `syncRemaining.p1`. All eight cases complete in a few
seconds locally. The gate proves the recorded runtime handshake projection refines these bounded
`SyncHandshakeV1` actions; fields explicitly listed above remain outside that claim.
