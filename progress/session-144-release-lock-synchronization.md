# Session 144 -- Release Lock Synchronization

## Objective

Eliminate the release generator's root-only lock update failure mode, enforce every Cargo
workspace lock at local, CI, preparation, and publication boundaries, then deliver a fully green
reviewed hardening pull request before repairing the blocked v0.10.1 release.

## Root cause and red evidence

- Cargo owns four repository workspace locks: root, `fuzz`, `loom-tests`, and
  `tests/godot-emscripten`.
- The prior release generator rewrote only the root `Cargo.lock` textually.
- The prepare workflow's `cargo metadata --locked --no-deps` check was vacuous for dependency
  freshness, allowing standalone path-dependency locks to remain stale.
- The new structural checker initially rejected the obsolete tracked
  `tests/network-peer/Cargo.lock`, proving the repository's former lock inventory was invalid.
- A realistic fixture updates only the root lock after a root version bump: the three standalone
  locks remain stale and are rejected. A dependency-only stale fixture passes structural checks
  but fails full `cargo metadata --locked`, proving `--no-deps` cannot replace full resolution.

## Implementation

- Added `scripts/release/workspace_locks.py` with dynamic tracked-manifest discovery through
  `cargo locate-project --workspace`, structural ownership/version checks, Cargo-authoritative
  synchronization, full locked metadata validation, relative diagnostics, and lock rollback on
  failure.
- Refactored `scripts/release/prepare_release.py` to prepare manifest, changelog, and all Cargo
  locks in a tracked-file sandbox; validate before and after the bump; atomically apply validated
  outputs; and make dry-runs byte-for-byte immutable.
- Removed the obsolete member-local network-peer lock and corrected its ownership comment.
- Enforced the canonical checker in release preparation, version-sync CI, publication, hooks, and
  changed-file-aware agent preflight. Loom now tests with `--locked`; Godot retains its locked
  Clippy gate.
- Updated publishing, workspace, fortress-development, and release decision guidance so future
  agents cannot strand generator fixes on generated branches or use `--no-deps` as a lock oracle.

## Adversarial review

- Post-incident scope: malformed manifests/locks, missing locks, orphan/member locks, newly added
  standalone roots, roots outside the repository, stale path-package versions, dependency-only
  staleness, subprocess failure, late transactional failure, and dry-run immutability.
- All diagnostics fail closed and remain repository-relative.
- The adversarial pass found that reading live files as the transaction's “before” state after
  sandbox validation could overwrite a concurrent edit. Preparation now snapshots every output
  before mutation, requires topology stability, and validates all live inputs before writing any
  output; a late-change regression proves zero files are replaced.
- Rollback snapshots now include permissions and atomically recreate deleted locks. Restoration
  attempts every lock and aggregates failures instead of abandoning later snapshots after one
  error; the subprocess-failure regression deletes a lock before failing and proves exact recovery.
- Tracked-file discovery no longer drops paths merely because their working-tree files are
  missing, and release sandbox construction rejects missing tracked inputs instead of rebuilding
  a falsely smaller topology. Regressions cover both fail-closed paths with repository-relative
  diagnostics.
- The first PR CI pass exposed a ShellCheck-only summary-rendering defect that local actionlint
  could not see without ShellCheck installed. The corrected workflow strips the key before safely
  formatting each discovered root, and its semantic contract now asserts that exact behavior.
- Cursor Bugbot found that the workflow ran a live-repository release preservation test after the
  real bump had emptied `Unreleased`, making successful non-dry preparation fail in its validation
  step. Release-tool tests now gate the workflow before mutation, while post-bump canonical lock,
  changelog, and package validation remains in place; a workflow-order contract guards the fix.
- A post-merge recovery simulation exposed a second false-green contract: dry-run omitted the
  updated `[Unreleased]` comparison link and any minor/major dependency-reference changes because
  `sync-version.sh` still ran outside the sandbox transaction. Release preparation now runs and
  validates that canonical synchronizer inside the sandbox, discovers all changed tracked outputs,
  and makes the workflow prove subsequent canonical synchronization is idempotent. Patch, minor,
  and major regressions cover the comparison link and dependency-reference behavior; a subprocess
  failure regression proves the live repository remains byte-for-byte unchanged.
- Cursor Bugbot then found that ordinary `git add --all` in the fresh sandbox index omitted
  force-tracked ignored files. Sandbox indexing now force-adds the already-vetted copied tracked
  set, and a regression proves ignored `progress/**` outputs participate in minor-version
  synchronization.
- No Rust production path, public API, wire behavior, or deterministic simulation behavior changes.
- No high- or critical-severity finding remains in the current main-thread adversarial pass.

## Local verification

- Initial full `scripts/tests` Python suite: 1,698 passed.
- Focused release/hook/workflow/preflight suite: 115 passed.
- Follow-up full `scripts/tests` Python suite after the dry-run completeness fix: 1,703 passed;
  the final focused release suite passed 38 tests. Actionlint, Python compilation, shell
  portability, markdownlint, and agent preflight passed.
- Canonical `workspace_locks.py check`: root, Fuzz, Loom, and Godot roots passed.
- Release dry-run: unchanged worktree and complete diffs for all four authoritative locks.
- Loom exact gate: 19 passed with `--release --locked` and `RUSTFLAGS="--cfg loom"`.
- Godot exact locked Clippy gate: passed on nightly 2026-07-08.
- `cargo fmt --check` and workspace Clippy with `tokio,json`: passed.
- `cargo nextest run --no-capture`: 2,875 passed, 74 skipped.
- `cargo doc --no-deps`, actionlint, shell portability, Agent Skill validation, link checks,
  markdownlint, `git diff --check`, and agent preflight: passed.

## Changelog

No changelog entry: this is internal release, CI, test, and contributor-tooling hardening with no
public API or user-observable runtime behavior change.

## Publication and review convergence

PR #252 was opened as a focused hardening change. Its first three commits reached 14/14 successful
workflow groups, a clean exact-head Cursor Bugbot review, no unresolved threads, and a Copilot
quota response with no actionable feedback. The post-merge recovery simulation then found the
dry-run completeness defect described above; that follow-up must repeat the push, review, and CI
convergence loop before merge.
