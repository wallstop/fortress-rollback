# Session 177: Z3 0.21 Verification Refresh

## Objective

Reconcile `GOAL.md`, `PLAN.md`, GitHub, and dependency state; take the highest-impact compatible
upgrade; and keep the formal-verification environment reproducible across local development and
hosted CI.

## External State

- Local `main`, remote `main`, and the GitHub default branch all resolve to
  `b621ffbd6f42147511ec36e6195f96f91f0f9c0e`.
- The exhaustive two-page GitHub census contains no open issues and no open or draft pull requests.
- All 18 workflows associated with the current `main` commit completed successfully, including the
  nightly simulation workflow.
- `PLAN.md` has no in-progress or future milestones, so the dependency audit supplied the next
  actionable unit of work.

## Dependency Decision

- All four Cargo workspaces selected zero compatible lockfile updates before this change. npm also
  reported no outdated packages.
- `z3` 0.21.0 supports the repository's Rust 1.86 MSRV and was therefore upgraded. Its transitive
  solver crates moved to `z3-sys` 0.13.0 and `z3-src` 501.0.0.
- `bincode-next` 3.1.1 remains excluded because it requires Rust 1.90. `serial_test` 4.0.1 remains
  excluded because it requires Rust 1.93.1. The post-update compatible dependency dry run selected
  zero additional updates.
- The upstream `z3` 0.21 release requires a native solver at least as new as Z3 4.13.3 and ships
  generated definitions for Z3 5.x. CI and the devcontainer now pin Z3 5.1.0 so compile-time
  bindings and the loaded runtime library cannot drift with runner package repositories.

## Implementation

- Upgraded the optional Rust dependency from `z3` 0.20 to 0.21 and changed the existing
  `z3-verification-bundled` compatibility feature to upstream's supported `vendored` path.
- Replaced the verification runner's unpinned Ubuntu package with the pinned `z3-solver` 5.1.0.0
  wheel. The job verifies the runtime version, discovers the wheel's native library, exports the
  build/runtime paths, and runs Nextest with the committed lockfile.
- Pinned the same solver in the devcontainer and registered its native library with the dynamic
  linker. The Codex npm package and bootstrap hooks remain unchanged.
- Added a static CI regression test covering the solver pin, runtime assertion, library overrides,
  blocking failure behavior, locked test invocation, and devcontainer linker setup.
- Documented the new solver floor and recommended version in the changelog, migration guide, user
  guide, and synchronized wiki copies.

## Green Evidence

- The targeted static test failed before the workflow change and passed afterward. The complete
  CI static-analysis module passes 31 tests; the focused devcontainer suites pass 70 tests.
- The native Z3 5.1.0 proof run passes all 65 selected verification tests, including 54 Z3 proofs.
- Stable formatting and strict workspace/all-target Clippy with `tokio,json` pass. The matching
  Nextest suite passes 3,008 tests with 72 skipped; the review-readiness `cargo c` and `cargo t`
  aliases pass, with `cargo t` completing 2,975 tests and 72 accepted skips.
- All 210 documentation tests pass with 160 executions and 50 accepted ignores. Rustdoc with
  warnings denied, Actionlint, doc-claim checks, wiki consistency, Markdown linting, link checking,
  spell checking, version synchronization, workspace lock checks, Cargo audit, and Cargo deny pass.
- The repository agent preflight passes all 286 general checks and 66 additional checks.

## Adversarial Review

The first review pass found a low-severity duplicated Z3 runtime-version literal in the workflow;
the assertion now reads the single workflow environment pin. The final diff pass found two
documentation inconsistencies: an overstated 5.x minimum in the manifest comment and a shell
command labeled as TOML. Both are corrected. The change does not touch production Rust, network or
replay formats, deterministic state, allocation-sensitive paths, or public runtime APIs.
