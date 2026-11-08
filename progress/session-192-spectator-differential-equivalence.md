# Session 192 — Spectator differential equivalence

## External reconciliation

- Paginated open issues with page size 100: only #365 exists and there is no further page. Its bounded
  Lean pilot remains P2 behind the active correctness work. No open, draft, or dependency PRs existed.
- Latest `main` SHA `b799615d3008e55a901ad288a183cdf9b616b705` completed all 118 checks: 113
  successful, five intentionally skipped, and none pending or failed. The initially running nightly
  network workflow completed successfully on Docker, Linux, macOS, and Windows; no RCA was needed.
- Previous session branches were merged and pruned. The new aggregate branch is
  `agent/spectator-equivalence`.

## Dependency reconciliation

- `cargo outdated --workspace` selected no compatible update. Newer direct versions retain their
  existing Rust 1.86 MSRV holds, including bincode-next 3.1.1, parking_lot 0.13, proptest 1.12,
  serial_test 4.0.1, and z3 0.22.
- The fuzz, Loom, and Godot standalone workspaces each reported all dependencies up to date. All four
  workspace lock checks passed. npm reported no outdated package. `cargo audit` and `npm audit` found
  no vulnerabilities. `cargo deny check` passed with only the existing
  duplicate-version warnings.

## Spectator reference

The two-peer differential oracle now checks every displayed spectator frame against the independent
fixed-history input model instead of relying only on agreement with the mesh. The comparison includes
full typed-input fingerprints and requires `Confirmed` statuses in the supported no-membership-change
scenario. A current-frame leap without its corresponding displayed inputs is a mismatch rather than a
skipped key.

The normalized trajectory retains spectator history for exact replay and cross-drive-order equality.
The bounded matrix covers EveryFrame/Sparse, delayed/held/reordered delivery, forward/reverse/rotating
peer drives, and redundant spectator hosts. A separate single-host row checks the complete runtime
2-to-4 delay gap/resume boundary prefix and post-control rollback evidence.

Mutation controls prove both sides of the spectator check: shared wrong committed inputs produce a
spectator `ReferenceInput` failure even though the mesh and spectator agree, and spectator-only input
corruption is also rejected. Nominal matrix rows provide the matched no-mutation/false-positive
controls.

The claim remains bounded to two peers, Lockstep cadence, static delay zero/two or one strict runtime
increase, positive prediction, and the stub's parity/sum state transition. Runtime membership changes,
hot-join, rebinding, near-overflow frames, and arbitrary game-state semantics remain in the active P0
scope; `PLAN.md` therefore remains unchanged and active.

## Red-green and validation

- Baseline differential selection passed 24 tests. The three new tests initially failed because
  differential validation rejected spectators (`invalid run options: differential mode does not
  support spectators`).
- Main-thread adversarial review (the sub-agent service was quota-limited) found that retained-key-only
  spectator checks could miss a current-frame hole; the comparison now rejects absent required frames.
  Re-review found no remaining correctness, determinism, compatibility, or error-handling issues.
- After implementing the reference comparison, all three focused spectator tests passed. The full
  simulation target passed 394 tests with 26 intentionally skipped. Full Nextest passed 3,024 tests
  with 73 intentionally skipped in 176.196 seconds.
- Strict all-workspace/all-target Clippy with `tokio,json` passed without warnings. Formatting, diff
  checks, plan hygiene, typos, Rustdoc, semantic doc claims, and 1,442 documentation links passed.
- No production or public API code changed, so no changelog or design-decision entry is required.
