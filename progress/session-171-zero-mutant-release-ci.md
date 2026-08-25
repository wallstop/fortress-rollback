# Session 171: Zero-Mutant Release CI

## Objective

Close the post-merge audit, repair the only failing v0.13.0 release check, and preserve the
release process's reconstructible-candidate trust boundary.

## Incident evidence

- PR #304 squash-merged as `0370e8a`; all 16 push workflow groups and the subsequent network and
  simulation nightlies passed on that exact `main` head.
- Complete paginated searches found zero open issues, zero dependency-update pull requests, and no
  compatible dependency update omitted from Session 170.
- Automated release preparation opened PR #305. Thirteen workflow groups passed; mutation run
  32810731825 alone failed.
- The release changes contain only Rust documentation edits. Cargo-mutants 27.1.0 therefore exited
  successfully with `INFO No mutants to filter` and wrote zero bytes for `--list --json`.
- The planner and shard both assumed successful list output was nonempty JSON. Empty shell values
  then reached arithmetic and `jq --argjson`, cascading into shard and summary failures.

## Implementation

- Normalize a successful zero-byte mutant list to the canonical empty JSON array in both corpus
  planning and shard execution.
- Require the normalized value to be a JSON array before deriving its length. Malformed JSON,
  scalars, objects, and producer failures remain fail-closed.
- Exercise the actual shell fragments extracted from both workflow steps with a fake cargo
  producer covering empty output, a valid two-item array, malformed JSON, non-array JSON, and an
  exact producer exit status of 42.
- Close generated release PR #305 before repair publication. The release-state gate requires a
  release candidate to remain one byte-for-byte reconstructible preparation commit, so the CI fix
  must land on `main` before v0.13.0 is regenerated.

## Review

- The first adversarial pass found no implementation, parsing, portability, or fail-open defect.
  It requested behavioral coverage rather than a structural-only workflow assertion.
- A second independent pass implemented the table-driven extraction test without adding a
  one-off production helper. Exact successful output and every rejection path are pinned.

## Local verification

- Exact cargo-mutants 27.1.0 release-diff reproduction: zero-byte output normalizes to `[]` and
  count `0`.
- Focused mutation workflow and aggregation contracts: 15 passed.
- `actionlint .github/workflows/ci-mutation.yml`: passed.
- Repository shell portability check: passed.
- `cargo fmt --check`: passed.
- Strict workspace/all-target Clippy with `tokio,json`: passed.
- `git diff --check`: passed.

## Remaining gates

- Publish the focused repair PR and require every exact-head workflow and reviewer pass.
- Merge the repair to `main`, verify post-merge CI, and regenerate v0.13.0 through the supported
  release-preparation workflow.
