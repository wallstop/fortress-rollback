# Session 187 - Burst consistency evidence

## External reconciliation

- Paginated the open-issue inventory with page size 100: only #365 exists and there is no further
  page. Its bounded Lean pilot remains the P2 item already queued in `PLAN.md`; it does not displace
  the active P0 consistency work. No open pull requests, drafts, or previous-session PRs existed.
- The old `agent/full-consistency-evidence` branch needs no recovery: its tip tree is identical to
  squash commit `ab451d2` (#364), which is an ancestor of current `main`. Current `main` also includes
  the later duplication and reorder evidence.
- Latest `main` at `cab3078685cb9f237a4229512c99aa27f47416f5` completed fully green: 90 successful
  checks, five accepted skips, zero pending, and zero failures. The initial audit observed the final
  Windows Miri and Rust CodeQL jobs still running; neither required intervention.

## Dependency reconciliation

- All four Cargo workspace locks passed `python3 scripts/release/workspace_locks.py check`, and a
  compatible `cargo update --workspace --dry-run` selected zero packages. Root direct upgrades left
  by `cargo outdated` are bincode-next 3.1.1, parking_lot 0.13, proptest 1.12, serial_test 4.0.1,
  and z3 0.22; existing configuration holds them for the Rust 1.86 MSRV. The fuzz, Loom, and Godot
  Emscripten workspaces reported no compatible direct upgrades.
- MkDocs 1.6.1, mkdocs-material 9.8.0, pymdown-extensions 12.3, pytest 9.2.0, packaging 26.3,
  Pygments 2.21.0, iniconfig 2.3.1, and pluggy 1.7.0 already match their latest releases.
- Updated the exactly pinned Playwright browser probe from 1.64.1 to 1.65.0. Both `playwright` and
  `playwright-core` 1.65.0 are now published. `npm ci --ignore-scripts` passed with zero audited
  vulnerabilities, `npx playwright --version` reported 1.65.0, and test discovery listed all three
  browser cases. Playwright's Node 20-or-newer requirement is compatible with the current Node 22
  development environment.

## Burst design and evidence

The active matrix now includes one deterministic directed fixed-burst-loss window at N=2/3/4/8/16.
Only 0→1 changes at step 100, with a seeded 0.05 per-send burst-start probability and burst length
three; `HealAll` restores the clean link at step 180. The expected result is bounded recovery under
`ContinueWithout`, with matched N=2/N=16 state-corruption controls at frame 120.

Saturating per-link and mesh-global burst-start counters participate in trace identity and complete
replay comparison. Zero counts remain omitted from serialized traces. Independent payload reception
reconstructs every missing span, accepts only three-packet bursts or a truncated terminal burst, and
derives the exact event count. Matrix premises require targeted injection, per-link/global agreement,
a real input-loss run of at least three, and drops consistent with complete bursts plus at most one
heal-truncated final burst. Back-to-back bursts may coalesce into longer observed runs.

Adversarial review first identified the missing exact-span oracle. A second review rejected overly
strict matrix assumptions for coalesced and heal-truncated bursts; those valid boundary cases are now
handled. The final adversarial review reported zero findings.

Exact-SHA release evidence at `6bef90ae9b6b5165f9a1808bf38be6459886912b`:

- 45 schema-v4 rows: 35 uninjected bases and ten matched corruption controls.
- 20 bounded recoveries, 15 safe failures, and ten intentional defect controls.
- All seven burst rows observed eight starts and 24 policy drops, exclusively on 0→1, with a maximum
  input-policy loss run of three. Both controls produced typed peer/frame-specific state divergence.
- Every row carries the exact code SHA; replay mismatches and premise violations are both zero.
- `PLAN.md` removes only the completed burst axis. Asymmetry, repeated flaps, poll stalls, exact
  timing, and host resource evidence remain active.

## Validation

- Playwright installation, audit, version, and test-discovery checks passed as described above.
- Focused SimNet/trace/schedule tests and the 13-row PR gate passed.
- Exact-SHA `partition_sweep_full`: 45 rows passed in release mode at the final code commit.
- `cargo fmt` and `cargo clippy --workspace --all-targets --features tokio,json`: passed.
- Full post-change `cargo nextest run --no-capture`: 2,996 passed, 73 skipped in 164.344 seconds.
- `.venv/bin/pytest scripts/tests -q`: 2,191 passed in 87.32 seconds.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: passed. Doc tests passed 160 with 50 ignored.
- Link checking covered 1,445 links with zero errors or warnings; doc claims, wiki consistency,
  Markdownlint, plan hygiene, typos, and all four workspace-lock checks passed.
- Agent preflight passed every selected check without a fallback.

## Delivery

Final PR and hosted-check results are recorded before session closeout.
