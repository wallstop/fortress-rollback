# Session 189 - Full consistency evidence matrix

## External reconciliation

- Paginated open issues with page size 100: only #365 exists and there is no further page. Its bounded
  Lean pilot remains P2 behind the correctness work. No open pull requests or dependency PRs existed.
- The previous local consistency branch was already merged through #364 and #367-#370. Rebasing dropped
  every duplicate commit; no production fix was reapplied.
- Latest `main` SHA `4757214205cb8b8f685df777b9c3be49993b0b85` completed all 14 push workflows
  successfully. No main-branch RCA or repair was needed.

## Dependency reconciliation

- Compatible root Cargo updates selected zero packages; all four workspace lock checks passed. Fuzz, Loom,
  Godot Emscripten, and browser-smoke direct dependencies were current. npm reported no outdated package.
- The only newer root direct crates require a newer Rust MSRV and remain under the existing Dependabot
  holds: bincode-next 3.1.1, parking_lot 0.13, proptest 1.12, serial_test 4.0.1, and z3 0.22.
- Documentation/release Python requirements and Playwright were already at their selected latest versions.
- `cargo audit` found no vulnerability. `cargo deny check` passed advisories, bans, licenses, and sources;
  only existing duplicate-version warnings remained.

## Design and red-green evidence

The new schema-v6 rows use only existing deterministic `Block`, `HealAll`, and `PeerStall` primitives.
Repeated flaps use three 20-step symmetric bisection outages at steps 100, 200, and 300, healed at 120,
220, and 320. Poll stalls freeze peer 1 for 20 steps at 100, 140, and 180, with a final heal at 200. Both
axes run at N=2/3/4/8/16 and have matched N=2/N=16 state-corruption controls. Every outage/stall is 320 ms,
below the two-second disconnect timeout.

The runner now records every actual heal's all-running/all-confirmed-progress boundary, every stall's
half-open freeze/resume evidence, and each peer's maximum contiguous post-sync confirmation-stall steps.
These deterministic observations participate in complete replay comparison. Host elapsed time and Linux
VmRSS/VmHWM are row-level process measurements outside that comparison.

Red-green findings were fixed at the appropriate layer:

- The first poll-stall control used the injected survivor as the expected observer. Injection now targets
  survivor peer 0 while the oracle expectation remains peer 1, matching the actual diagnostic contract.
- A 20-step clean interval did not recover every N=4 peer after the second flap. The final schedule retains
  20-step outages but provides 80-step clean intervals, and requires progress before every next cut.
- A schedule-only assertion incorrectly applied the single-outage timeout to the whole repeated-flap run;
  per-outage checks replaced that invalid aggregate premise.
- Main-thread adversarial review added per-cycle and per-stall evidence rather than accepting aggregate
  counters. The delegated adversarial reviewer was unavailable because of a service quota, so the required
  review loop continued on the main thread.

## Exact-SHA evidence

Code commit `ba287019533c01734b8f3a96430812db918a633a` produced
`/tmp/opencode/partition-matrix-ba28701.jsonl`:

- 66 rows: 35 bounded recoveries, 15 safe failures, and 16 intentional defect controls.
- Zero replay mismatches and zero premise violations.
- Every flap row has three per-heal recovery records. Confirmed progress took seven steps after each heal,
  except 21 steps after the latter two N=4 heals; all preceded the next cut.
- Every poll-stall row observed three exact 20-step freezes, unchanged current/confirmed frames, and resume
  at steps 120, 160, and 200. Maximum confirmation stalls were 20 or 21 steps.
- All four new N=2/N=16 controls produced typed `StateDivergence` at peer 1/frame 120, bounded by the
  confirmed prefix at healing.
- All 66 rows contain host elapsed/RSS evidence. Observed executions took 2.16-1,105.74 ms and process-wide
  peak RSS reached 39,542,784 bytes. These are host-specific observations, not portable budgets.
- The pre-change 52-row release matrix passed in 19.475 seconds. The final 66-row matrix passed in 26.504
  seconds before exact-SHA artifact generation, leaving the nightly timeout unchanged.

`PLAN.md` removes the completed matrix and promotes differential rollback equivalence to the active P0.
The tuning guide and wiki mirror publish the schedules, measurements, controls, and exact reproduction
command. No production/public behavior changed, so no changelog entry or design-decision record is needed.

## Validation

- Exact-SHA release `partition_sweep_full`: 66 rows passed with zero replay/premise failures.
- PR matrix plus schedule-materialization test: 2 passed; the matrix contains 19 rows.
- Full `cargo nextest run --no-capture`: 2,996 passed, 73 skipped in 157.463 seconds.
- `cargo fmt` and `cargo clippy --workspace --all-targets --features tokio,json`: passed without warnings.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: passed. Doc tests passed 160 with 50 ignored.
- Link checking covered 1,377 links with zero errors/warnings; semantic doc claims and wiki consistency
  checks passed.
- Markdownlint, plan hygiene, typos, and all four workspace-lock checks passed.
- Agent preflight passed every selected check without fallback. Vale retained seven pre-existing advisory
  suggestions in the tuning guide and no blocking finding.
