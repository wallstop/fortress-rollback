<!-- SYNC: This source doc syncs to wiki/Desync-Playbook.md. -->

# Desync Incident Playbook

A `DesyncDetected` event means two peers supplied different checksums for the same confirmed
frame. The event is non-terminal and the session may continue, but Fortress does not reconcile
or repair a real confirmed-state divergence. The application should stop treating the affected
match as authoritative until investigators find the cause.

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
6. In a semi-trusted deployment, also preserve per-peer input logs and authenticated transport
   packet records when available. They may expose equivocation that a single merged replay cannot.

With the `json` feature, `SessionMetrics`, `PeerMetrics`, and `SpecViolation` provide
`to_json()` and `to_json_pretty()` helpers. Never include credentials or player secrets in an
incident bundle.

## Triage

| Observation | Likely direction | Next check |
| --- | --- | --- |
| One mismatch, later comparisons match | Transient bad state save/checksum or stale application data | Compare the exact confirmed frame in the replay |
| Mismatch count rises on one peer only | Peer/path-specific nondeterminism, corruption, false report, or on-path modification | Compare platform, build, feature set, input serialization, and authenticated packet evidence |
| Every peer disagrees with one participant | Participant/path-specific anomaly; checksums alone do not localize fault | Corroborate its replay, deterministic state hash, input log, and authenticated packet evidence |
| Two recipients recorded different input bytes for the same sender/frame | Equivocation or on-path modification | Quarantine the match and preserve the authenticated packet records; checksums alone cannot attribute blame |
| Honest replays agree but one remote reports different checksums | False checksum accusation is possible | Corroborate the exact frame across peers; do not auto-eject from the accusation or warning alone |
| A hot joiner diverges immediately after loading | Bad build/configuration, nondeterminism, or poisoned coordinator snapshot | Record coordinator, frame, peer-supplied checksum, and any application-computed state fingerprint; retry only from a separately trusted source |
| Error/Critical violation precedes mismatch | Degraded library or boundary condition | Start from the first violation, not the later checksum symptom |
| Replay agrees but the live match did not | Missing nondeterministic input outside the replay contract | Audit wall time, RNG, collection order, ECS query order, and floating point |

Do not dismiss a one-off mismatch merely because play continued. It proves the reported checksum
evidence differed. In a trusted deployment that is a strong state-divergence signal, but a bad or
dishonest checksum can differ even when states match. Quarantine first, then corroborate state
divergence and attribution from replay, input, build, and authenticated transport evidence.

## Dishonest-peer response

- **B1 equivocation:** stop treating the match as authoritative at the first proven divergent
  frame. A checksum mismatch detects disagreement but cannot identify the liar. Preserve every
  recipient's evidence; a replay assembled from only one recipient cannot show the conflicting
  version delivered elsewhere.
- **B3 false checksum reports:** treat `peer_checksum_mismatch_count` and its one-time persistence
  warning as advisory. Compare the exact frame across other peers, local deterministic replay,
  build identity, and input logs before assigning fault. Never automatically eject or penalize a
  participant from one endpoint's checksum claim alone.
- **B5 flooding:** a flood may starve useful packets without producing a desync first. Keep the
  network poll/event drain alive, capture unknown-source and queue/cap telemetry, and engage
  transport, OS, or network-edge rate limits. Do not respond by increasing bounded receive or
  queue limits without a measured capacity analysis.
- **B6 snapshot poisoning:** quarantine a joiner that disagrees after loading a hot-join snapshot.
  Record the serving coordinator, snapshot frame, peer-supplied optional checksum, any
  application-computed loaded-state fingerprint, and build/configuration identity. The checksum
  and fingerprint preserve evidence but do not attest provenance. Do not retry from the same
  untrusted source; rebuild or select a separately trusted coordinator.

These rules assume at most one dishonest participant. Agreement among several peers is useful
corroboration in that model, not transferable cryptographic proof and not a defense against
collusion. See the [threat model](threat-model.md) for the protocol boundary.

## Reproduction

1. Check and load the captured replay using the APIs in the [replay guide](replay.md).
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
