# Session 145: Release and Publish Reliability

## Objective

Make release preparation, publication, and nightly-dependent CI reproducible,
retry-safe, semver-aware, and self-enforcing, then open a fully reviewed green PR.

## Incident Evidence

- Release PR #253 prepared correctly but failed because a repository-derived test
  attempted to prepare another release from the already-rotated, empty
  `Unreleased` section.
- Actions job 88030023204 failed before project code ran while the floating
  nightly channel manifest was being updated and rustup observed a checksum
  mismatch.
- The proposed `0.10.1` patch contains a public API addition and an intentional
  wire-protocol incompatibility, requiring `0.11.0` under the project policy.

## Work Log

- [x] Confirmed the two CI root causes from GitHub job logs.
- [x] Selected `0.11.0`, retained a crate-scoped repository token, and moved all
  release metadata finalization into the reviewed preparation PR.
- [x] Implement release bump classification and generated-tree lifecycle tests.
- [x] Implement immutable prepared-release state, trusted reconstruction,
  checkpointed tag creation, and crates.io/GitHub reconciliation.
- [x] Pin required Rust toolchains and eliminate floating required nightlies.
- [x] Update project guidance, preflight selection, and architectural decisions.
- [x] Complete local verification and adversarial review.
- [x] Open draft PR #254 and trigger Cursor Bugbot and Copilot review on the
  first pushed commit.
- [x] Address Cursor's release-branch trust-gate and annotated-tag review
  findings with reserved-namespace and peeled-ref regressions.
- [x] Address Cursor's historical non-UTF-8 manifest and pinned publish-cache
  findings with strict raw-byte parsing and exact restore-prefix regressions.
- [ ] Resolve CI/reviewer feedback and make every required PR check green.
- [ ] Repository administrator: require the stable `Verify prepared release
  state` check on `main` and enable merge queue (preferred) or strict
  up-to-date required checks; GitHub does not expose this setting to repo code.

## Verification

- Focused integrated release/toolchain/preflight contracts: 381 tests passed.
- Complete Python/script suite: 1,955 tests passed.
- Complete agent preflight passed, including 275 release automation tests, 66
  toolchain tests, 49 Agent Skills, all 5,138 local files and 1,392 links,
  changelog policy, workflow linting, fallback imports, and spelling.
- Toolchain installers have executable Bash 3.2 success, empty-list parsing,
  retry exhaustion, malformed/overflow pin, and workflow/job/step
  floating-nightly regressions.
- Rust formatting, workspace Clippy with `-D warnings`, and the complete
  workspace/all-targets test and benchmark lane passed with `tokio,json`.
- ShellCheck and the repository-wide shell portability scan pass.
- All workflow files currently pass `actionlint`.
- `git diff --check` is clean.
