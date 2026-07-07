# Session 001: Spectator Host Kill

## Scope

Advance M3 by completing the `SpectatorHostKill` lifecycle vocabulary item for
the deterministic simulation harness. The change is test-infra only: no
production Rust API or wire behavior changed.

## Changes

- Added schema-v7 `ScheduleEvent::SpectatorHostKill { host }`.
- Reused the lifecycle retire/detach path for configured spectator hosts.
- Added fail-loud validation for out-of-range, non-spectator-host, and
  already-retired host-kill schedules.
- Exposed `RunReport::spectator_final_hosts` so fleet tests can assert host
  compaction directly.
- Added a planted partition+host-kill fleet test that proves:
  - partition traffic was actually dropped (`dropped_blocked > 0`);
  - partition-only fails closed through spectator divergence while retaining all
    three hosts;
  - `SpectatorHostKill` changes execution beyond a partition-only control;
  - the spectator removes the crashed host (`Some(2)` final hosts from `3`);
  - live survivors keep confirming after the configured host crash.

## Review Loop

- Explorer recommended `SpectatorHostKill` as the smallest coherent unfinished
  M3 task and suggested final host-count plus blocked-traffic premise checks.
- First adversarial pass found the no-op check did not isolate host-kill from
  partition-only effects, asked for direct failover proof, and flagged
  already-retired host no-ops.
- Follow-up changes added the partition-only control, `spectator_final_hosts`
  assertion, and fail-loud already-retired validation.
- Final validation showed the partition-only control intentionally fails closed
  with spectator divergence; the test now asserts that failure shape rather than
  treating the stale-host control as a passing schedule.

## Validation

```bash
cargo fmt
cargo nextest run spectator_failover_survives_configured_host_kill_under_partition \
  run_rejects_malformed_spectator_host_kill_events \
  lifecycle_events_round_trip_through_json --no-capture
cargo clippy --workspace --all-targets --features tokio,json
python3 scripts/ci/agent-preflight.py --auto-fix
git diff --check
```

All commands passed locally.
