# Session 190 — Differential rollback reference

## External reconciliation

- Paginated open issues with page size 100: only #365 exists and there is no further page. Its bounded
  Lean pilot remains P2 behind the active correctness work. No open, draft, or dependency PRs existed.
- Latest `main` SHA `6fb81cb421c3016f73289838b983688598180700` completed all checks: 90 successful,
  five intentionally skipped, and none pending or failed. No main-branch RCA or repair was needed.
- Previous session branches were already merged; the new aggregate branch is
  `agent/rollback-equivalence`.

## Dependency reconciliation

- Refreshed `cc` 1.11.0 to 1.11.1 and `smallvec` 1.16.4 to 1.16.5 in all four Cargo workspaces.
- All workspace lock checks passed. `cargo audit` found no vulnerability and `cargo deny check` passed;
  only the existing duplicate-version warnings remained.
- npm reported no outdated package and no vulnerability. Latest documentation dependencies installed
  cleanly and built the site with strict MkDocs. The hash-locked release automation suite passed 200
  tests with the selected latest Python packages.
- Remaining newer Cargo versions retain their MSRV holds. `minicov` 0.3.9 cannot be selected because
  the already-latest `wasm-bindgen-test` 0.3.80 exactly requires 0.3.8.

## Differential oracle

Added an opt-in, two-player fixed-history reference to the deterministic simulation harness. The driver
now uses each session's current frame as its input ordinal only in differential mode, with default input
prefixes for static delay. Confirmed typed inputs and exact `StateStub` post-states are compared against
a nominal reference rather than another peer. Every required post-state through the confirmed frame must
exist; the optional `confirmed + 1` state is checked when present but excluded from normalized history.

The bounded matrix covers EveryFrame/Sparse, static delays zero/two, delayed/held/reordered delivery,
and forward/reverse/rotating complete peer drives. Reports remain peer-indexed, exact schedule/options
replays retain their complete trace, and cross-order comparisons use only normalized confirmed history.
Rollback counts, resimulated frames, load requests, delivery counters, and minimum trajectory lengths
make the green rows non-vacuous.

Mutation controls distinguish the new reference from legacy peer agreement:

- shared wrong committed inputs and padding-only typed input changes produce `ReferenceInput`;
- shared wrong states produce `ReferenceState`;
- interior and whole-history omissions produce `ReferenceStateMissing`;
- checksum-only lies fire the actual checksum detector without a reference mismatch.

Differential options participate in artifact schema v8, exact replay, corpus validation, and shrinking.
Wide-padding artifacts require the 32-byte input representation, so invalid metadata is rejected before
replay. A fixed legacy run pins its trace hash, final confirmed frames, and complete trace-tail digest to
values independently derived at base SHA `6fb81cb421c3016f73289838b983688598180700`.

The claim remains bounded to two peers, Lockstep cadence, static delay zero/two, positive prediction,
and the stub's parity/sum state transition. Runtime membership/delay changes, spectators, hot-join,
rebinding, near-overflow frames, and arbitrary game-state semantics remain in the active P0 scope; the
plan item therefore remains active rather than being presented as complete.

## Review and validation

- Explorer, independent design reviewer, implementation agent, and adversarial re-reviewer examined the
  slice. The first implementation review found missing-state and invalid-wide-artifact gaps plus a weak
  legacy regression; all were corrected. The final adversarial re-review reported zero remaining issues.
- Differential suite: 13 passed. Full simulation target: 379 passed and 26 skipped.
- Dependency-refresh baseline before the differential edits: 2,996 passed and 73 skipped.
- Final full Nextest run: 3,009 passed and 73 skipped. Strict all-workspace/all-target Clippy with
  `tokio,json` passed without warnings. Formatting, diff checks, plan hygiene, typos, and 1,442
  documentation links passed.
- No production or public API code changed, so no changelog or design-decision entry is required.
