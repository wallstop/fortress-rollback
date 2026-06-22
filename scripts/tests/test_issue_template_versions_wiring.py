#!/usr/bin/env python3
"""Guard the issue-template version-sync wiring and dropdown invariants.

Background (GitHub issue #168): the bug-report version dropdown drifted because
the standalone ``sync-issue-template.yml`` workflow triggers on
``release: [released]``, but a release created by ``publish.yml`` with the
default ``GITHUB_TOKEN`` does not emit that event. The durable fix syncs the
dropdown *inline* in ``publish.yml``. These offline tests protect both the
wiring (so the inline sync can't silently regress) and the dropdown content
(unique, semver-descending entries between the sentinels).
"""
from __future__ import annotations

import re
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
PUBLISH_WORKFLOW = REPO_ROOT / ".github" / "workflows" / "publish.yml"
BUG_REPORT = REPO_ROOT / ".github" / "ISSUE_TEMPLATE" / "bug_report.yml"
BEGIN_SENTINEL = "# BEGIN_FORTRESS_VERSIONS"
END_SENTINEL = "# END_FORTRESS_VERSIONS"
SYNC_SCRIPT = "scripts/ci/sync-issue-template-versions.py"


def test_publish_workflow_syncs_issue_template_inline() -> None:
    """publish.yml must invoke the issue-template sync (root-cause fix for #168)."""
    text = PUBLISH_WORKFLOW.read_text(encoding="utf-8")
    assert SYNC_SCRIPT in text, (
        "publish.yml must run "
        f"'{SYNC_SCRIPT}' inline so every release updates the bug-report "
        "version dropdown (a GITHUB_TOKEN-created release does not trigger the "
        "standalone release-event workflow)."
    )
    # The sync step pushes the refreshed template back to the default branch.
    assert "bug_report.yml" in text and "git push" in text, (
        "publish.yml's issue-template sync step must commit and push the "
        "updated bug_report.yml."
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


def test_publish_workflow_stamps_release_date() -> None:
    """publish.yml must stamp the real release date (refresh-at-release)."""
    text = PUBLISH_WORKFLOW.read_text(encoding="utf-8")
    assert "--stamp-release-date" in text, (
        "publish.yml must run 'sync-version.sh --stamp-release-date' so the "
        "changelog's placeholder date is refreshed to the real release date."
    )
    assert "RELEASE_VERSION: ${{ steps.get_version.outputs.version }}" in text, (
        "publish.yml must preserve the published version for post-publish "
        "metadata finalization, even if the default branch is bumped before "
        "the finalizer runs."
    )
    assert '--release-version "$RELEASE_VERSION"' in text, (
        "publish.yml must pass the immutable published version into "
        "sync-version.sh instead of letting the default branch Cargo.toml "
        "choose which changelog header is stamped."
    )
    assert '--ensure-version "$RELEASE_TAG"' in text, (
        "publish.yml must inject the just-created release tag into the "
        "issue-template sync so API listing delay cannot omit the published "
        "version from bug_report.yml."
    )
