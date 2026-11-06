# Session 188 - Asymmetric-delay consistency evidence

## External reconciliation

- Paginated the open-issue inventory with page size 100: only #365 exists and there is no further
  page. Its bounded Lean pilot remains the P2 item already queued in `PLAN.md`; it does not displace
  the active P0 consistency work. No open pull requests, drafts, or previous-session PRs existed.
- Latest `main` at `b960525f4c687de7522b10835f45567f95d0eb94` has 13 successful push workflows and
  no failures. The final Rust CodeQL workflow remained in progress during local verification; no RCA
  or repair was indicated.

## Dependency reconciliation

- All four Cargo workspace locks passed `python3 scripts/release/workspace_locks.py check`, and
  compatible `cargo update --workspace --dry-run` selected zero packages.
- Root direct upgrades left by `cargo outdated` are bincode-next 3.1.1, parking_lot 0.13, proptest
  1.12, serial_test 4.0.1, and z3 0.22; existing configuration holds them for the Rust 1.86 MSRV.
  The fuzz, Loom, Godot Emscripten, and browser-smoke workspaces reported no compatible direct
  upgrades. No open dependency PR required incorporation.

## Red-green design and review

The active matrix now includes one deterministic 200 ms one-way delay on directed link 0→1 at
N=2/3/4/8/16. The window runs from step 100 through step 180, followed by `HealAll`; every other
link remains clean. The expected result is bounded recovery under `ContinueWithout`, with matched
N=2/N=16 state-corruption controls at frame 120.

A focused SimNet test first failed to compile because no base-delay counters existed. It now proves
that only the targeted direction is delayed, the reverse direction delivers immediately, and delivery
occurs exactly at the virtual-time boundary. Per-copy global/per-link counters use saturating
accounting, merge across source generations, participate in complete replay comparison and trace
identity, and remain absent from serialized traces when zero. Delayed duplication and counter-merge
tests prove two-copy accounting and saturation.

The first matrix run correctly rejected an initially incomplete premise: after `HealAll`, clean sends
can overtake still-in-flight delayed packets. The oracle now expects that directed reorder evidence
instead of misclassifying a valid boundary as a defect. Three adversarial reviews closed the
copy-accounting, saturation, documentation, and progress-record findings; no production code changed.

## Exact-SHA matrix evidence

Code commit `726b25f47fb51213c71a8ebcbb47b07dfa7757f1` passed the release-mode full matrix:

- 52 schema-v5 rows: 40 uninjected bases and 12 matched corruption controls.
- 25 bounded recoveries, 15 safe failures, and 12 intentional defect controls.
- Asymmetric delay observed 170 delayed copies and 26 post-heal reorder deliveries at N=2; every
  larger size observed 132 delayed copies and 22 reorder deliveries. All injection occurred only on
  0→1, and per-link sums matched global counters.
- Both controls produced typed state divergence at peer 1/frame 120, bounded by the confirmed prefix
  at healing.
- Every row carries the exact code SHA; replay mismatches and premise violations are both zero.
- `PLAN.md` removes only the completed asymmetry axis. Repeated flaps, poll stalls, exact timing, and
  host resource evidence remain active.

## Validation

- Focused SimNet/trace/schedule tests and the 15-row PR gate passed.
- Exact-SHA `partition_sweep_full`: 52 rows passed in release mode.
- `cargo fmt` and `cargo clippy --workspace --all-targets --features tokio,json`: passed.
- Full post-change `cargo nextest run --no-capture`: 2,996 passed, 73 skipped in 159.228 seconds.
- `.venv/bin/pytest scripts/tests -q`: 2,191 passed in 122.68 seconds.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: passed; doc tests passed 160 with 50 ignored.
- Link checking covered 1,445 links with zero errors or warnings; doc claims, wiki consistency,
  Markdownlint, plan hygiene, typos, and all four workspace-lock checks passed.
- Agent preflight passed every selected check without a fallback. Vale reported seven advisory
  suggestions in the existing tuning guide and no blocking finding.
