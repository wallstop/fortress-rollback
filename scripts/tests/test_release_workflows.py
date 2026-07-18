#!/usr/bin/env python3
"""Offline invariants for release preparation and publishing workflows."""

import re
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
PREPARE = REPO_ROOT / ".github" / "workflows" / "release-prepare.yml"
PUBLISH = REPO_ROOT / ".github" / "workflows" / "publish.yml"
VERSION_SYNC = REPO_ROOT / ".github" / "workflows" / "ci-version-sync.yml"
VERIFICATION = REPO_ROOT / ".github" / "workflows" / "ci-verification.yml"
RUST_CI = REPO_ROOT / ".github" / "workflows" / "ci-rust.yml"
DOCS_CI = REPO_ROOT / ".github" / "workflows" / "ci-docs.yml"
PUBLISHING_SKILL = REPO_ROOT / ".agents" / "skills" / "publishing" / "SKILL.md"
SYNC_ISSUE_TEMPLATE = REPO_ROOT / ".github" / "workflows" / "sync-issue-template.yml"
RELEASE_STATE_CI = REPO_ROOT / ".github" / "workflows" / "ci-release-state.yml"
CARGO_MANIFEST = REPO_ROOT / "Cargo.toml"
RELEASE_TOOLCHAIN = REPO_ROOT / ".github" / "actions" / "install-pinned-release" / "toolchain"
RELEASE_REQUIREMENTS = REPO_ROOT / "scripts" / "release" / "requirements.txt"


def test_prepare_workflow_opens_ci_capable_release_pr() -> None:
    text = PREPARE.read_text(encoding="utf-8")

    assert "workflow_dispatch:" in text
    assert "type: choice" in text
    for bump in ("patch", "minor", "major"):
        assert f"- {bump}" in text
    assert "actions/create-github-app-token@" in text
    assert "scripts/release/prepare_release.py" in text
    assert 'refs/heads/${DEFAULT_BRANCH}' in text
    assert "scripts/release/release_branch.py resolve" in text
    assert "scripts/release/release_branch.py ensure-pr" in text
    assert "credential.helper" in text
    assert "credential.https://github.com.username" in text
    assert "http.https://github.com/.extraheader" not in text
    assert "--head-sha \"${head_sha}\"" in text


def test_prepare_workflow_uses_canonical_lock_transaction_and_summary() -> None:
    text = PREPARE.read_text(encoding="utf-8")

    tooling_test_command = "python3 -m pytest scripts/tests/test_workspace_locks.py"
    assert text.index(tooling_test_command) < text.index(
        "python3 scripts/release/prepare_release.py"
    )
    for test_file in (
        "test_prepare_release.py",
        "test_release_policy.py",
        "test_release_state.py",
        "test_release_state_ci.py",
        "test_release_checkpoint.py",
        "test_publish_state.py",
        "test_release_branch.py",
        "test_sync_issue_template_versions.py",
        "test_issue_template_versions_wiring.py",
        "test_release_workflows.py",
    ):
        assert text.count(test_file) == 2
    assert "scripts/release/workspace_locks.py sync" in text
    assert "scripts/release/workspace_locks.py check" in text
    assert "git diff --binary -- ." in text
    assert "cmp -s" in text
    assert "Release preparation omitted canonical synchronizer output" in text
    assert "--no-deps" not in text
    assert "git --no-pager diff --cached --binary -- ." in text
    assert 'git --no-pager diff --binary "${BASE_SHA}..HEAD" -- .' in text
    assert "GITHUB_STEP_SUMMARY" in text
    assert "current_version=" in text
    assert "prepared_version=" in text
    assert "workspace_root=" in text
    assert "s/^workspace_root=//p" in text
    assert 'printf -- "- \\`%s\\`\\n" "${workspace_root}"' in text
    assert "--dry-run" in text


def test_publish_workflow_has_one_manual_entrypoint() -> None:
    text = PUBLISH.read_text(encoding="utf-8")
    on_block = text.split("permissions:", maxsplit=1)[0]

    assert "workflow_dispatch:" in on_block
    assert "push:" not in on_block
    assert "concurrency:" in text
    assert "group: release-${{ github.event.inputs.release_version }}" in text
    assert "Require default branch dispatch" in text


def test_release_workflows_pin_external_actions_and_rust_compiler() -> None:
    external_use = re.compile(
        r"(?m)^\s+uses:\s+(?!\./)(?P<action>[^@\s]+)@(?P<ref>[^\s#]+)"
    )

    for workflow in (PREPARE, PUBLISH, SYNC_ISSUE_TEMPLATE, RELEASE_STATE_CI):
        text = workflow.read_text(encoding="utf-8")
        uses = list(external_use.finditer(text))
        assert uses, f"{workflow.name} has no external action references"
        for match in uses:
            assert re.fullmatch(r"[0-9a-f]{40}", match.group("ref")), (
                f"{workflow.name} must pin {match.group('action')} to a full commit SHA"
            )

    prepare = PREPARE.read_text(encoding="utf-8")
    publish = PUBLISH.read_text(encoding="utf-8")
    release_state_ci = RELEASE_STATE_CI.read_text(encoding="utf-8")
    assert "uses: ./.github/actions/install-pinned-release" in prepare
    assert "uses: ./trusted/.github/actions/install-pinned-release" in publish
    assert (
        "uses: ./verifier/.github/actions/install-pinned-release"
        in release_state_ci
    )
    assert RELEASE_TOOLCHAIN.read_text(encoding="utf-8").strip() == "1.96.1"
    for text in (prepare, publish, release_state_ci):
        assert "dtolnay/rust-toolchain@" not in text
        assert "toolchain: stable" not in text
        assert "toolchain: 1.96.1" not in text

    assert "steps.release-rust.outputs.toolchain" in publish


def test_prepare_uses_exact_python_and_hash_locked_test_dependencies() -> None:
    text = PREPARE.read_text(encoding="utf-8")
    requirements = RELEASE_REQUIREMENTS.read_text(encoding="utf-8")

    assert "actions/setup-python@ece7cb06caefa5fff74198d8649806c4678c61a1" in text
    assert 'python-version: "3.13.5"' in text
    assert "--require-hashes" in text
    assert "--only-binary=:all:" in text
    assert "--requirement scripts/release/requirements.txt" in text
    assert "pip install pytest" not in text
    assert "pytest==" in requirements
    requirement_lines = [
        line for line in requirements.splitlines() if line and not line.startswith(("#", " "))
    ]
    assert requirement_lines
    assert requirements.count("--hash=sha256:") == len(requirement_lines)


def test_required_release_workflows_pin_python_runtime() -> None:
    for workflow in (PREPARE, PUBLISH, RELEASE_STATE_CI):
        text = workflow.read_text(encoding="utf-8")
        assert (
            text.count(
                "actions/setup-python@ece7cb06caefa5fff74198d8649806c4678c61a1"
            )
            == 1
        )
        assert text.count('python-version: "3.13.5"') == 1


def test_prepare_dry_run_executes_complete_ephemeral_release() -> None:
    text = PREPARE.read_text(encoding="utf-8")

    # Only credential generation and the branch/PR write step are suppressed.
    assert text.count("if: ${{ !inputs.dry_run }}") == 3
    for required in (
        "scripts/release/workspace_locks.py sync",
        "sync-issue-template-versions.py",
        "--local-only",
        "release_state.py generate",
        "release_state.py verify",
        "Test release tooling after preparation",
        "cargo publish --dry-run --locked --allow-dirty",
        "Stage and show complete prepared diff",
        "git --no-pager diff --cached --binary",
    ):
        assert required in text
    assert text.index("release_branch.py resolve") < text.index("Prepare release files")
    assert text.index("release_state.py generate") < text.index(
        "cargo publish --dry-run --locked --allow-dirty"
    )


def test_prepare_rerun_recovers_branch_and_pull_request() -> None:
    text = PREPARE.read_text(encoding="utf-8")

    assert "Resolve release branch recovery" in text
    assert "steps.recovery.outputs.exists != 'true'" in text
    assert "steps.recovery.outputs.release_date" in text
    assert "steps.recovery.outputs.replace_sha" in text
    assert 'if [ "${BRANCH_EXISTS}" != "true" ]; then' in text
    assert "release_branch.py verify-default" in text
    assert "release_branch.py push" in text
    assert '--expected-base-sha "${BASE_SHA}"' in text
    assert '--expected-branch-sha "${REPLACE_SHA}"' in text
    assert text.index("release_branch.py verify-default") < text.index(
        'git commit -m "Prepare v${VERSION} release"'
    )
    assert text.index('git commit -m "Prepare v${VERSION} release"') < text.index(
        "release_branch.py push"
    )
    assert 'git push origin "HEAD:refs/heads/${BRANCH}"' not in text
    assert "release_branch.py ensure-pr" in text


def test_release_branch_state_check_has_no_path_filter_escape() -> None:
    text = RELEASE_STATE_CI.read_text(encoding="utf-8")
    trigger = text.split("permissions:", maxsplit=1)[0]

    assert "pull_request_target:" in trigger
    assert "merge_group:" in trigger
    assert "types: [checks_requested]" in trigger
    assert "\n  pull_request:\n" not in trigger
    assert "paths:" not in trigger
    assert "paths-ignore:" not in trigger
    assert "Checkout trusted base verifier" in text
    assert "verifier/scripts/release/release_state_ci.py" in text
    assert "github.event.pull_request.head.ref" in text
    assert "github.event.pull_request.head.sha" in text
    assert "github.event.pull_request.base.sha" in text
    assert "github.event.merge_group.head_sha" in text
    assert "github.event.merge_group.base_sha" in text
    assert (
        "repository: ${{ github.event.pull_request.head.repo.full_name || "
        "github.repository }}" in text
    )
    assert "repository: ${{ github.repository }}" in text
    assert "name: Verify prepared release state" in text
    assert text.count("steps.gate.outputs.reconstruction_required == 'true'") == 2
    assert "verifier/scripts/release/release_state.py" in text
    assert "verify-candidate" in text
    assert "verify-prospective" in text
    assert '--candidate-root "${GITHUB_WORKSPACE}/candidate"' in text
    assert '--trusted-base-root "${GITHUB_WORKSPACE}/verifier"' in text
    assert (
        "ref: ${{ github.event.pull_request.head.sha || "
        "github.event.merge_group.head_sha }}" in text
    )
    assert "candidate/scripts/" not in text


def test_issue_template_sync_is_manual_repair_only() -> None:
    text = SYNC_ISSUE_TEMPLATE.read_text(encoding="utf-8")
    trigger_block = text.split("permissions:", maxsplit=1)[0]

    assert "workflow_dispatch:" in trigger_block
    assert "release:" not in trigger_block


def test_publish_workflow_packages_and_verifies_release_artifact() -> None:
    text = PUBLISH.read_text(encoding="utf-8")

    assert "cargo package --locked" in text
    assert "sha256sum" in text
    assert "actions/upload-artifact@" in text
    assert "gh release create" in text
    assert (
        'package_path="${GITHUB_WORKSPACE}/candidate/target/package/'
        '${CRATE_NAME}-${CARGO_VERSION}.crate"'
    ) in text
    assert "${{ steps.package.outputs.path }}.sha256" in text
    assert text.index("trusted/scripts/release/workspace_locks.py") < text.index(
        "cargo package --locked"
    )


def test_publish_workflow_is_safe_to_retry_after_crates_io_publish() -> None:
    text = PUBLISH.read_text(encoding="utf-8")

    assert "scripts/release/publish_state.py" in text
    assert "--checksum \"${LOCAL_CHECKSUM}\"" in text
    assert "CARGO_REGISTRY_TOKEN: ${{ secrets.CRATES_IO_TOKEN }}" in text
    assert text.index("Create immutable release checkpoint") < text.index(
        "scripts/release/publish_state.py"
    )
    assert "trusted/scripts/release/release_checkpoint.py" in text
    assert '--state-file "${RUNNER_TEMP}/release-checkpoint.json"' in text
    assert "--candidate \"${GITHUB_WORKSPACE}/candidate\"" in text


def test_release_state_is_generated_in_pr_and_verified_before_package() -> None:
    prepare = PREPARE.read_text(encoding="utf-8")
    publish = PUBLISH.read_text(encoding="utf-8")

    assert "scripts/release/release_state.py generate" in prepare
    assert "--previous-version \"${CURRENT_VERSION}\"" in prepare
    assert "--target-version \"${PREPARED_VERSION}\"" in prepare
    assert "--date \"${RELEASE_DATE}\"" in prepare
    assert "scripts/release/release_state.py verify" in prepare
    assert "trusted/scripts/release/release_state.py" in publish
    assert '--repo-root "${GITHUB_WORKSPACE}/candidate" verify' in publish
    assert text_order(publish, "Verify immutable reviewed release state", "cargo package --locked")
    assert '"release-state.json"' in CARGO_MANIFEST.read_text(encoding="utf-8")


def text_order(text: str, first: str, second: str) -> bool:
    """Return whether both workflow commands occur in the required order."""
    return text.index(first) < text.index(second)


def test_publish_never_mutates_default_branch_after_publication() -> None:
    text = PUBLISH.read_text(encoding="utf-8")

    assert "git commit" not in text
    assert 'git push origin "HEAD:' not in text
    assert "--stamp-release-date" not in text
    assert "sync-issue-template-versions.py" not in text


def test_target_tag_is_checked_before_irreversible_publish() -> None:
    text = PUBLISH.read_text(encoding="utf-8")

    assert "Resolve trusted candidate source" in text
    assert "Create immutable release checkpoint" in text
    assert text_order(text, "Resolve trusted candidate source", "Verify immutable reviewed release state")
    assert text_order(text, "Verify immutable reviewed release state", "cargo package --locked")
    assert text_order(text, "cargo package --locked", "Create immutable release checkpoint")
    assert text_order(text, "Create immutable release checkpoint", "publish_state.py")


def test_retry_uses_tagged_source_after_default_branch_advances() -> None:
    text = PUBLISH.read_text(encoding="utf-8")

    assert "release_checkpoint.py" in text
    assert "--trusted-sha \"$TRUSTED_DISPATCH_SHA\"" in text
    assert text_order(text, "Resolve trusted candidate source", "Verify immutable reviewed release state")
    assert text_order(text, "Verify immutable reviewed release state", "cargo package --locked")


def test_partial_failure_retry_reuses_pre_publish_checkpoint() -> None:
    text = PUBLISH.read_text(encoding="utf-8")

    assert text_order(text, "cargo package --locked", "Create immutable release checkpoint")
    assert text_order(text, "Create immutable release checkpoint", "publish_state.py")
    assert "resolve" in text
    assert "create" in text


def test_publish_separates_trusted_helpers_from_candidate_source() -> None:
    text = PUBLISH.read_text(encoding="utf-8")

    assert "Checkout trusted dispatch helpers" in text
    assert "ref: ${{ github.sha }}" in text
    assert "path: trusted" in text
    assert "persist-credentials: false" in text
    assert "TRUSTED_DISPATCH_SHA: ${{ github.sha }}" in text
    assert "--candidate \"${GITHUB_WORKSPACE}/candidate\"" in text
    assert "python3 trusted/scripts/release/release_state.py" in text
    assert "python3 trusted/scripts/release/workspace_locks.py" in text
    assert "python3 ../trusted/scripts/release/publish_state.py" in text
    assert "python3 candidate/scripts/release" not in text


def test_publish_revalidates_exact_checkpoint_at_irreversible_boundaries() -> None:
    text = PUBLISH.read_text(encoding="utf-8")

    assert text.count("release_checkpoint.py") == 4
    pre_publish = "Revalidate checkpoint immediately before crates.io"
    pre_release = "Revalidate checkpoint immediately before GitHub Release"
    assert text_order(text, pre_publish, "Publish or reconcile crates.io state")
    assert text_order(text, pre_release, "Create or update GitHub Release")
    assert "verify\n\n      - name: Publish or reconcile crates.io state" in text
    assert "verify\n\n      - name: Create or update GitHub Release" in text


def test_publish_is_explicitly_bound_to_crates_io() -> None:
    manifest = CARGO_MANIFEST.read_text(encoding="utf-8")
    helper = (REPO_ROOT / "scripts" / "release" / "publish_state.py").read_text(
        encoding="utf-8"
    )

    assert 'publish = ["crates-io"]' in manifest
    assert '["cargo", "publish", "--locked", "--registry", "crates-io"]' in helper


def test_publish_workflow_updates_existing_github_release() -> None:
    text = PUBLISH.read_text(encoding="utf-8")

    assert 'releases/tags/${TAG}' in text
    assert "gh release upload" in text
    assert "--clobber" in text
    assert "gh release edit" in text


def test_version_sync_preserves_check_name_and_runs_full_lock_checker() -> None:
    text = VERSION_SYNC.read_text(encoding="utf-8")

    assert "name: Version Sync Check" in text
    assert '"**/Cargo.lock"' in text
    assert '"scripts/release/**"' in text
    assert "dtolnay/rust-toolchain@stable" in text
    assert "scripts/release/workspace_locks.py check" in text
    assert "--no-deps" not in text


def test_loom_ci_enforces_locked_release_test() -> None:
    text = VERIFICATION.read_text(encoding="utf-8")

    assert "run: cargo test --release --locked" in text


def test_godot_ci_retains_locked_clippy_gate() -> None:
    text = RUST_CI.read_text(encoding="utf-8")

    assert (
        "cargo +nightly-2026-07-08 clippy --manifest-path "
        "tests/godot-emscripten/Cargo.toml --locked --all-targets "
        "--all-features -- -D warnings"
    ) in text


def test_release_changes_trigger_existing_script_test_lane() -> None:
    text = DOCS_CI.read_text(encoding="utf-8")

    assert text.count('"scripts/release/**"') == 2
    assert text.count('".github/actions/install-pinned-release/**"') == 2
    for workflow in (
        "release-prepare.yml",
        "ci-release-state.yml",
        "publish.yml",
        "ci-version-sync.yml",
        "ci-verification.yml",
    ):
        assert text.count(f'".github/workflows/{workflow}"') == 2


def test_publishing_skill_matches_reviewed_workflow_contract() -> None:
    skill = PUBLISHING_SKILL.read_text(encoding="utf-8")

    assert "Release - Prepare PR" in skill
    assert "Release - Publish Crate" in skill
    assert "green CI" in skill
    assert "sole manual publication entrypoint" in skill
    assert "workspace_locks.py check" in skill
