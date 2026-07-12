#!/usr/bin/env python3
"""Offline invariants for release preparation and publishing workflows."""

from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
PREPARE = REPO_ROOT / ".github" / "workflows" / "release-prepare.yml"
PUBLISH = REPO_ROOT / ".github" / "workflows" / "publish.yml"


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
    assert "gh pr create" in text


def test_publish_workflow_has_one_manual_entrypoint() -> None:
    text = PUBLISH.read_text(encoding="utf-8")
    on_block = text.split("permissions:", maxsplit=1)[0]

    assert "workflow_dispatch:" in on_block
    assert "push:" not in on_block
    assert "concurrency:" in text
    assert "Require default branch dispatch" in text


def test_publish_workflow_packages_and_verifies_release_artifact() -> None:
    text = PUBLISH.read_text(encoding="utf-8")

    assert "cargo package --locked" in text
    assert "sha256sum" in text
    assert "actions/upload-artifact@" in text
    assert "softprops/action-gh-release@" in text
    assert 'package_path="target/package/${CRATE_NAME}-${CARGO_VERSION}.crate"' in text
    assert "${{ steps.package.outputs.path }}.sha256" in text


def test_publish_workflow_is_safe_to_retry_after_crates_io_publish() -> None:
    text = PUBLISH.read_text(encoding="utf-8")

    assert "https://crates.io/api/v1/crates/${CRATE_NAME}/${CARGO_VERSION}" in text
    assert "published_checksum" in text
    assert "local_checksum" in text
    assert "already exists on crates.io with the packaged checksum" in text
