#!/usr/bin/env python3
"""Guard the issue-template version-sync wiring and dropdown invariants.

Background (GitHub issue #168): the bug-report version dropdown drifted because
the standalone release event is not emitted by a release created with the
default ``GITHUB_TOKEN``. The durable fix finalizes the dropdown in the reviewed
release-preparation PR, before the immutable source digest is recorded.
"""
from __future__ import annotations

import re
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
PUBLISH_WORKFLOW = REPO_ROOT / ".github" / "workflows" / "publish.yml"
PREPARE_WORKFLOW = REPO_ROOT / ".github" / "workflows" / "release-prepare.yml"
BUG_REPORT = REPO_ROOT / ".github" / "ISSUE_TEMPLATE" / "bug_report.yml"
BEGIN_SENTINEL = "# BEGIN_FORTRESS_VERSIONS"
END_SENTINEL = "# END_FORTRESS_VERSIONS"
SYNC_SCRIPT = "scripts/ci/sync-issue-template-versions.py"
SYNC_WORKFLOW = REPO_ROOT / ".github" / "workflows" / "sync-issue-template.yml"
DOCS_WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ci-docs.yml"


def test_prepare_workflow_syncs_issue_template_before_digest() -> None:
    """The reviewed PR must contain the target version before it is hashed."""
    prepare = PREPARE_WORKFLOW.read_text(encoding="utf-8")
    assert SYNC_SCRIPT in prepare
    assert "--local-only" in prepare
    assert '--ensure-version "v${PREPARED_VERSION}"' in prepare
    assert prepare.index(SYNC_SCRIPT) < prepare.index("release_state.py generate")


def test_publish_workflow_does_not_push_issue_template_fixup() -> None:
    """Published, tagged, and default-branch sources must stay identical."""
    publish = PUBLISH_WORKFLOW.read_text(encoding="utf-8")
    assert SYNC_SCRIPT not in publish
    assert "bug_report.yml" not in publish
    assert "git commit" not in publish


def test_sync_repair_workflow_is_not_release_triggered() -> None:
    """Creating a release must never cause an unreviewed source mutation."""
    workflow = SYNC_WORKFLOW.read_text(encoding="utf-8")
    on_block = workflow.split("permissions:", maxsplit=1)[0]
    assert "workflow_dispatch:" in on_block
    assert "release:" not in on_block


def test_issue_template_sync_surfaces_trigger_docs_ci_for_push_and_pr() -> None:
    """Every source of dropdown drift must exercise its offline CI checks."""
    docs = DOCS_WORKFLOW.read_text(encoding="utf-8")
    for path in (
        SYNC_SCRIPT,
        ".github/workflows/sync-issue-template.yml",
        ".github/ISSUE_TEMPLATE/bug_report.yml",
    ):
        assert docs.count(f'      - "{path}"') == 2, (
            f"ci-docs.yml must trigger for {path} on both push and pull_request"
        )


def _dropdown_versions() -> list[str]:
    lines = BUG_REPORT.read_text(encoding="utf-8").splitlines()
    begin = end = None
    for i, line in enumerate(lines):
        if line.strip() == BEGIN_SENTINEL:
            begin = i
        elif line.strip() == END_SENTINEL:
            end = i
            break
    assert begin is not None and end is not None and begin < end, (
        "bug_report.yml is missing well-formed "
        f"{BEGIN_SENTINEL}/{END_SENTINEL} sentinels"
    )
    versions = []
    for line in lines[begin + 1 : end]:
        m = re.match(r"\s*-\s+(v\d+\.\d+\.\d+)\s*$", line)
        assert m, f"unexpected entry between version sentinels: {line!r}"
        versions.append(m.group(1))
    return versions


def _semver_key(tag: str) -> tuple[int, int, int]:
    major, minor, patch = (int(p) for p in tag.lstrip("v").split("."))
    return major, minor, patch


def test_dropdown_versions_unique_and_semver_descending() -> None:
    """Dropdown entries must be unique and strictly newest-first (semver order)."""
    versions = _dropdown_versions()
    assert versions, "version dropdown is unexpectedly empty"
    assert len(versions) == len(set(versions)), (
        f"duplicate versions in dropdown: {versions}"
    )
    keys = [_semver_key(v) for v in versions]
    assert keys == sorted(keys, reverse=True), (
        f"dropdown versions must be in strictly descending semver order: {versions}"
    )


def test_dropdown_includes_v0_8_1() -> None:
    """Regression for #168: the latest published release must be selectable."""
    assert "v0.8.1" in _dropdown_versions(), (
        "v0.8.1 (the latest published release at the time of the fix) must be "
        "present in the bug-report version dropdown"
    )


def test_prepare_workflow_chooses_one_release_date_before_generation() -> None:
    """New releases share one UTC date while reruns reuse committed metadata."""
    prepare = PREPARE_WORKFLOW.read_text(encoding="utf-8")
    assert 'release_date="$(date -u +%Y-%m-%d)"' in prepare
    assert '--date "${release_date}"' in prepare
    assert 'echo "requested_date=${release_date}"' in prepare
    assert "steps.recovery.outputs.release_date" in prepare
    assert '--date "${RELEASE_DATE}"' in prepare
    publish = PUBLISH_WORKFLOW.read_text(encoding="utf-8")
    assert "--stamp-release-date" not in publish
