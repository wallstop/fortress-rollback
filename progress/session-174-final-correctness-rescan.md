# Session 174: Final Correctness Rescan

## Scope

Complete issue #297's requested final correctness census after PR #314 merged. Re-scan the public
runtime boundaries for panic, undefined behavior, timer overflow, and rollback ordering faults;
then advance the result to reviewed, exact-head green PR #315.

## Discovery

- Full GitHub pagination found issue #297 as the only open issue, with no open or draft pull
  requests and no dependency pull requests.
- All 17 workflows on merged `main` commit `f610fbd` completed successfully.
- Dry-run updates selected zero changes in the root, fuzz, Loom, and Godot/Emscripten lockfiles.
  `cargo audit` found no vulnerabilities and `cargo deny check` passed. The only newer direct
  releases, bincode-next 3.1.1 and serial_test 4.0.1, require Rust 1.90 and 1.93.1 respectively,
  above the project's Rust 1.86 MSRV.
- Red regressions reproduced invalid zero/overflowing synchronization timers, disconnect-notify
  subtraction underflow, `Instant` deadline overflow, floor-round wraparound ordering faults,
  chaos-delay overflow, and modulo by zero after an empty saved-state container.
- A production-source census found ten `debug_assert!` calls that could still panic in paranoid
  builds despite the project's runtime zero-panic contract.

## Implementation

- `SyncConfig::validate` now rejects zero packets and sub-millisecond intervals, and all session
  builders plus direct protocol construction enforce it. Disconnect notification timing must also
  fit inside the configured timeout.
- Protocol and chaos scheduling compare saturating elapsed durations rather than constructing
  fallible absolute deadlines. Counters and duration arithmetic saturate at their public bounds,
  and injected regressing clocks cannot move protocol time backward.
- Floor round trips use reserved-zero serial-number ordering across `u32` wrap. Bootstrap replies
  retain numeric solicitation ordering so a forged pre-bootstrap maximum cannot bypass the first
  request, and zero requests are rejected.
- Empty saved-state lookup now returns its structured error instead of dividing by zero. Remaining
  production `debug_assert!` sites became ordinary fail-closed checks with invariant reporting;
  Safety CI rejects future debug-assert invocations under `src/`.
- The public API census, changelog, API/formal specifications, user guide, and generated wiki
  mirrors describe the validation and wraparound contracts.

## Adversarial Review

The first main-thread adversarial pass found the bootstrap maximum-round bypass and converted it
into a failing regression before fixing the serial rule. The second pass found that Safety CI's
initial source scan could miss whitespace variants and accept a failed scanner pipeline; the
workflow and its executable contract test now cover both paths. The final frozen diff produced no
additional concrete correctness findings.

After PR publication, a local diff-scoped mutation slice found that changing the input gap-fill
failure guard from OR to AND survived the existing full-queue test. A terminal-frame regression now
isolates the rejected-insert branch when saturating addition leaves `last_added_frame` equal to the
candidate. The formerly missed mutant is caught.

## Local Evidence

- Strict production Clippy passed with panic, unwrap, expect, unreachable, todo, unimplemented,
  and indexing lints denied under the Safety workflow feature set.
- Baseline Nextest passed 2,938/2,938 with 71 skipped; hot-join Nextest passed 3,196/3,196 with 72
  skipped; rustdoc passed 169 cases with 54 ignored.
- Focused regressions passed for configuration/timing, floor wrap, chaos `Duration::MAX`, empty
  saved states, input-queue rollback, synchronization invariants, and coordinated peer drop.
- Pinned Miri passed the maximum-timer, floor-wrap, chaos-delay, and empty-state regressions for
  seeds 0 through 2. Kani proved the synchronization counter and bounds harnesses.
- The complete agent preflight, 38 Safety workflow tests, actionlint, API snapshot regeneration,
  Markdown/link/wiki checks, `cargo audit`, `cargo deny check`, and `git diff --check` passed.
- The configuration mutation slice reports 11 caught, three unviable, zero missed or timed out; the
  formerly surviving input gap-fill guard mutant is caught in its focused rerun.

## Commits

- `003d2b8` harden timing and serial correctness
