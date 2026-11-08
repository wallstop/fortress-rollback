# Session 193 — Graceful-removal differential equivalence

## External reconciliation

- Paginated open issues with page size 100: only #365 exists and there is no further page. Its bounded
  Lean pilot remains P2 behind the active correctness work. No open, draft, or dependency PRs existed.
- Main SHA `4c102a9aacb16dfe98175fd675f41012ac56b23d` had 91 successful checks, five intentional
  skips, and one cancelled CodeQL Rust analysis. RCA showed the push analysis was replaced by the
  higher-priority scheduled request for the same ref (`codeql-refs/heads/main`). Scheduled run
  42447252073 then completed all three language analyses successfully at the identical SHA. This was
  intentional concurrency replacement, not a source defect; no CI workaround or policy change was made.
- The aggregate branch is `agent/graceful-remove-equivalence`.

## Dependency reconciliation

- `cargo outdated --workspace` selected no compatible update. Newer direct versions retain their
  existing Rust 1.86 MSRV holds, including bincode-next 3.1.1, parking_lot 0.13, proptest 1.12,
  serial_test 4.0.1, and z3 0.22.
- npm reported no outdated package. The fuzz, Loom, and Godot standalone workspaces each reported all
  dependencies up to date. All four workspace lock checks passed. `cargo audit` and `npm audit` found no
  vulnerabilities. `cargo deny check` passed with only the existing duplicate-version warnings.

## Graceful-removal reference

The bounded two-peer differential oracle now supports one explicit `GracefulRemove { by: 0, target: 1 }`
without a simultaneous runtime-delay or spectator scenario. A successful synchronous commit must keep
the survivor Running, disconnect the target, preserve the pre-call direct receipt as certified cut `F`,
and keep the survivor's pre-call confirmed/sampled history at or below `F`.

The independent nominal model leaves input `F` unchanged and repeats that value for the removed target
strictly after `F`; the survivor continues its ordinary delay-derived stream. Thus post-state `F+1`
still consumes nominal input `F`, while `F+2` is the first state that can observe frozen input. The
oracle requires the departed peer's `F+1` state when actually confirmed and simulated, but excludes its
abandoned `F+2+` generation from both reference and legacy cross-peer state comparisons. Target-authored
confirmed inputs above `F` are pruned as canonical authors without erasing their earlier nominal checks.

The matrix covers EveryFrame/delay-0/reordered delivery and Sparse/delay-2/delayed delivery across
forward, reverse, and rotating drive orders. Exact replay retains complete traces and trajectories;
cross-order identity uses the survivor projection because departed peers can legally retain different
trailing prefixes. Matched no-removal controls require distinct input `F+1` and state `F+2` values.
Post-control metrics and load observations prove rollback and resimulation happen after removal.

Negative controls cover a mutation at the ordinal that authors frozen input `F`, wrong and missing
post-freeze survivor states, missing removed-target state `F+1`, removed-target final game/session frame
mismatch, and exact artifact failure/trace/trajectory/control replay. A dedicated reordered row proves
both target-authored inputs above `F` and an abandoned target state at `F+2` or later actually exist.

## Red-green and review evidence

- The initial removal matrix passed 32 focused differential tests. Independent adversarial review then
  found four gaps: optional removed-target `F+1`, an unchecked survivor-history premise, suppressed
  final frame identity, and weak artifact verdict comparison. All were fixed with regressions.
- Tightening the survivor premise initially went red because it sampled confirmation after
  `remove_player`; the API can advance that counter while committing. The guard now uses the pre-call
  value, and the focused suite returned to 32 passed.
- Re-review found and fixed nullable state-control assertions, then missing evidence for abandoned
  target state `F+2`. Final adversarial re-review reported zero issues.
- Full simulation target: 399 passed and 26 intentionally skipped. Full Nextest: 3,029 passed and 73
  intentionally skipped in 152.278 seconds. Strict all-workspace/all-target Clippy with `tokio,json`
  passed without warnings. Rustdoc built successfully and 160 documentation tests passed with 50
  intentionally ignored. Agent preflight, formatting, diff checks, plan hygiene, typos, semantic doc
  claims, and 1,442 documentation links passed.
- No production or public API code changed, so no changelog or design-decision entry is required.
  `PLAN.md` remains active for automatic disconnects/timeouts, spectator membership changes, hot-join,
  and the other explicitly bounded P0 work.
