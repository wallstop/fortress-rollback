#!/usr/bin/env python3
"""Offline invariants for release preparation and publishing workflows."""

from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
PREPARE = REPO_ROOT / ".github" / "workflows" / "release-prepare.yml"
PUBLISH = REPO_ROOT / ".github" / "workflows" / "publish.yml"
VERSION_SYNC = REPO_ROOT / ".github" / "workflows" / "ci-version-sync.yml"
VERIFICATION = REPO_ROOT / ".github" / "workflows" / "ci-verification.yml"
RUST_CI = REPO_ROOT / ".github" / "workflows" / "ci-rust.yml"
DOCS_CI = REPO_ROOT / ".github" / "workflows" / "ci-docs.yml"
PUBLISHING_SKILL = REPO_ROOT / ".agents" / "skills" / "publishing" / "SKILL.md"


def test_prepare_workflow_opens_ci_capable_release_pr() -> None:
    text = PREPARE.read_text(encoding="utf-8")

    assert "workflow_dispatch:" in text
    assert "type: choice" in text
    for bump in ("patch", "minor", "major"):
        assert f"- {bump}" in text
    assert "actions/create-github-app-token@" in text
    assert "scripts/release/prepare_release.py" in text
    assert 'refs/heads/${DEFAULT_BRANCH}' in text
    assert 'branch="release/v${VERSION}"' in text
    assert "credential.helper" in text
    assert "credential.https://github.com.username" in text
    assert "http.https://github.com/.extraheader" not in text
    assert "gh pr create" in text


def test_prepare_workflow_uses_canonical_lock_transaction_and_summary() -> None:
    text = PREPARE.read_text(encoding="utf-8")

    tooling_tests = (
        "python3 -m pytest scripts/tests/test_workspace_locks.py "
        "scripts/tests/test_prepare_release.py scripts/tests/test_release_workflows.py"
    )
    assert text.index(tooling_tests) < text.index(
        "python3 scripts/release/prepare_release.py"
    )
    assert "scripts/release/workspace_locks.py sync" in text
    assert "scripts/release/workspace_locks.py check" in text
    assert "git diff --binary -- ." in text
    assert "cmp -s" in text
    assert "Release preparation omitted canonical synchronizer output" in text
    assert "--no-deps" not in text
    assert "git --no-pager diff -- ." in text
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


def test_publish_workflow_packages_and_verifies_release_artifact() -> None:
    text = PUBLISH.read_text(encoding="utf-8")

    assert "cargo package --locked" in text
    assert "sha256sum" in text
    assert "actions/upload-artifact@" in text
    assert "gh release create" in text
    assert 'package_path="target/package/${CRATE_NAME}-${CARGO_VERSION}.crate"' in text
    assert "${{ steps.package.outputs.path }}.sha256" in text
    assert text.index("scripts/release/workspace_locks.py check") < text.index(
        "cargo package --locked"
    )


def test_publish_workflow_is_safe_to_retry_after_crates_io_publish() -> None:
    text = PUBLISH.read_text(encoding="utf-8")

    assert "https://crates.io/api/v1/crates/${CRATE_NAME}/${CARGO_VERSION}" in text
    assert "published_checksum" in text
    assert "local_checksum" in text
    assert "User-Agent: fortress-rollback-release-workflow/${CARGO_VERSION}" in text
    assert "already exists on crates.io with the packaged checksum" in text


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
    for workflow in (
        "release-prepare.yml",
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
