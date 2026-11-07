# Session 191 — Runtime-delay differential equivalence

## External reconciliation

- Paginated open issues with page size 100: only #365 exists and there is no further page. Its bounded
  Lean pilot remains P2 behind the active correctness work. No open, draft, or dependency PRs existed.
- Latest `main` SHA `a6615805df71fb845de580e55a30a1b19ea10c08` completed all 94 checks: 89 successful,
  five intentionally skipped, and none pending or failed. No main-branch RCA or repair was needed.
- The new aggregate branch is `agent/runtime-delay-equivalence`.

## Dependency reconciliation

- `cargo outdated --workspace` selected no compatible update. Newer direct versions retain their existing
  Rust 1.86 MSRV holds, including bincode-next 3.1.1, parking_lot 0.13, proptest 1.12, serial_test 4.0.1,
  and z3 0.22.
- All four workspace lock checks passed. npm reported no outdated package. `cargo audit` found no
  vulnerability and `cargo deny check` passed; only existing duplicate-version warnings remained.

## Runtime-delay reference

The two-peer differential oracle now supports one successful, strict, post-synchronization runtime
input-delay increase. It captures the actual session frame `C` at the control boundary and retains
`(peer, C, old delay, new delay)` in the normalized trajectory. The independent nominal model repeats
ordinal `C - 1` throughout `C + old .. C + new`, then resumes fresh ordinals at `C + new`; it never reads
actual inputs or queue contents.

The bounded matrix covers EveryFrame/Sparse, delayed/held/reordered delivery, and forward/reverse/rotating
peer drive order for a 0-to-2 increase, plus cross-order 2-to-4 coverage. Hard-coded known answers pin both
zero and nonzero old-delay boundaries. Exact replay, matched no-change controls, changed-player mutation
boundaries, state/checksum controls, artifact replay, post-control rollback evidence, and strict option
validation make the new rows non-vacuous.

Final state validation uses the independent session-current frame as its simulated ceiling, rather than
allowing a bad game-state frame to reduce its own checks. A final game/session frame mismatch produces a
reference failure. Legal confirmed-but-unsimulated tails remain optional. Differential shrinking now uses
the strict-increase floor while legacy non-differential shrinking retains its zero-delay simplification.

## Red-green and review evidence

- Before the nominal gap model, `runtime_delay_increase_matches_nominal_gap_and_resume` failed with
  `ReferenceInput` at frame 80: the actual input repeated ordinal 79 while the static reference advanced.
  After the model change, the focused differential and shrinker gate passed 25 tests.
- Explorer, implementation, and independent adversarial review examined the slice. The first code review
  found a game-controlled state-validation ceiling and an unintended legacy shrinker change. Both were
  fixed with regressions; the final adversarial re-review reported zero remaining issues.
- Full simulation target: 391 passed and 26 ignored. Full Nextest: 3,021 passed and 73 skipped in 149.513
  seconds. Formatting and strict all-workspace/all-target Clippy with `tokio,json` passed without warnings.
- No production or public API code changed, so no changelog or design-decision entry is required. The
  active P0 plan remains open for membership, spectators, hot-join, broader interleavings, and other
  explicitly bounded oracle work.
