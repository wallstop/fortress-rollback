<!-- SYNC: This wiki page is generated from docs/desync-playbook.md. Edit docs source. -->

# Desync Incident Playbook

A `DesyncDetected` event means two peers supplied different checksums for the same confirmed
frame. The library can recover from the event, but the application should stop treating the
affected match as authoritative until investigators find the cause.

## Immediate response

1. Keep polling long enough to drain pending events and capture final metrics; do not advance
   gameplay after the application's chosen quarantine boundary.
2. Record the event's frame, local checksum, remote checksum, and peer address.
3. For every remote handle, capture `sync_health(handle)`,
   `peer_checksum_mismatch_count(handle)`, and `peer_metrics(handle)`.
4. Capture `metrics()`, all collected specification violations, the full session configuration,
   game/build version, platform, player mapping, and recent application logs.
5. If the session records replays, persist `take_replay()` before destroying it. Mark a
   missing or partial replay explicitly instead of silently omitting it.

With the `json` feature, `SessionMetrics`, `PeerMetrics`, and `SpecViolation` provide
`to_json()` and `to_json_pretty()` helpers. Never include credentials or player secrets in an
incident bundle.

## Triage

| Observation | Likely direction | Next check |
| --- | --- | --- |
| One mismatch, later comparisons match | Transient bad state save/checksum or stale application data | Compare the exact confirmed frame in the replay |
| Mismatch count rises on one peer only | Peer-specific nondeterminism or corruption | Compare platform, build, feature set, and input serialization |
| Every peer disagrees with one participant | Fault localized to that participant or its game build | Run its replay and deterministic state hash locally |
| Error/Critical violation precedes mismatch | Degraded library or boundary condition | Start from the first violation, not the later checksum symptom |
| Replay agrees but the live match did not | Missing nondeterministic input outside the replay contract | Audit wall time, RNG, collection order, ECS query order, and floating point |

Do not dismiss a one-off mismatch merely because play continued. A single disagreement proves
the peers did not share the same deterministic state at that frame.

## Reproduction

1. Check and load the captured replay using the APIs in the [replay guide](Replay).
2. Run the same input stream through `SyncTestSession` with the original save mode and checksum
   function.
3. Reproduce on the original target, then on a second architecture or OS.
4. Reduce the input prefix to the first mismatching confirmed frame. Preserve the original
   session configuration while shrinking.
5. Add a regression that fails at the earliest divergent state transition, not only at the final
   checksum.

## Issue template

Include the following in a private report when the bundle may contain user data:

```text
Fortress Rollback version / commit:
Application build and platform:
Session config and enabled Cargo features:
Desync frame and both checksums:
Affected peer/address mapping (redacted if needed):
SyncHealth and mismatch count per peer:
SessionMetrics and PeerMetrics JSON:
Violation JSON and logs before the first mismatch:
Replay attached: yes / no / partial:
Smallest known reproduction:
```

After removing private data, file a public issue with the smallest replay or test case that still
reproduces the divergence.
