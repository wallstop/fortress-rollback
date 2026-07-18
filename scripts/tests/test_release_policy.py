#!/usr/bin/env python3
"""Tests for changelog-driven release bump policy."""

from __future__ import annotations

import importlib.util
import subprocess
import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[2]
POLICY_SCRIPT = REPO_ROOT / "scripts" / "release" / "release_policy.py"
SPEC = importlib.util.spec_from_file_location("release_policy", POLICY_SCRIPT)
assert SPEC is not None and SPEC.loader is not None
release_policy = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = release_policy
SPEC.loader.exec_module(release_policy)


def _changelog(notes: str) -> str:
    return f"""# Changelog

## [Unreleased]

{notes}
## [0.9.0] - 2026-01-01

- Previous release.
"""


@pytest.mark.parametrize(
    ("current", "notes", "expected"),
    [
        ("1.2.3", "### Fixed\n\n- A fix.\n", "patch"),
        ("1.2.3", "### Security\n\n- A security fix.\n", "patch"),
        ("1.2.3", "### Added\n\n- An API.\n", "minor"),
        ("1.2.3", "### Deprecated\n\n- An old API.\n", "minor"),
        ("1.2.3", "### Changed\n\n- New behavior.\n", "minor"),
        ("0.10.0", "### Removed\n\n- An old API.\n", "minor"),
        ("1.2.3", "### Removed\n\n- An old API.\n", "major"),
        (
            "0.10.0",
            "### Changed\n\n- **Breaking:** The wire format changed.\n",
            "minor",
        ),
        (
            "1.2.3",
            "### Changed\n\n- **Breaking:** The wire format changed.\n",
            "major",
        ),
        (
            "1.2.3",
            "### Added\n\n- An API.\n\n### Fixed\n\n- A fix.\n",
            "minor",
        ),
        (
            "1.2.3",
            "### Changed\n\n- New behavior.\n- **Breaking:** Removed a field.\n",
            "major",
        ),
        (
            "1.2.3",
            "### Added\n\n- A feature with a wrapped\n  description.\n",
            "minor",
        ),
    ],
)
def test_minimum_bump_classifies_unreleased_notes(
    current: str, notes: str, expected: str
) -> None:
    actual = release_policy.minimum_bump(_changelog(notes), current)

    assert actual == expected


@pytest.mark.parametrize(
    ("requested", "expected"),
    [("patch", "patch"), ("minor", "patch"), ("major", "patch")],
)
def test_validate_requested_bump_accepts_minimum_or_higher(
    requested: str, expected: str
) -> None:
    actual = release_policy.validate_requested_bump(
        _changelog("### Fixed\n\n- A fix.\n"), "1.2.3", requested
    )

    assert actual == expected


def test_validate_requested_bump_rejects_bump_below_minimum() -> None:
    with pytest.raises(
        release_policy.ReleasePolicyError,
        match="requested patch bump is below the minimum minor bump",
    ):
        release_policy.validate_requested_bump(
            _changelog("### Added\n\n- An API.\n"), "1.2.3", "patch"
        )


@pytest.mark.parametrize(
    ("notes", "message"),
    [
        ("", "Unreleased section has no release notes"),
        ("### Added\n", "have no list entries: Added"),
        (
            "Release prose without a subsection.\n",
            "release notes must be under a supported",
        ),
        ("### Experimental\n\n- A change.\n", "subsection 'Experimental' is unsupported"),
    ],
)
def test_minimum_bump_rejects_unclassifiable_notes(notes: str, message: str) -> None:
    with pytest.raises(release_policy.ReleasePolicyError, match=message):
        release_policy.minimum_bump(_changelog(notes), "1.2.3")


@pytest.mark.parametrize(
    ("notes", "message"),
    [
        (
            "### Added\n\nRelease prose instead of a list.\n",
            "must be nonempty '- ' list entries",
        ),
        (
            "### Added\n\n- A feature.\n continuation with one space.\n",
            "two-space-indented continuations",
        ),
        (
            "### Added\n\n  continuation before an entry.\n",
            "continuation has no preceding list entry",
        ),
        ("### Added\n\n-\n", "must be nonempty '- ' list entries"),
        ("### Added\n\n-   \n", "contains an empty list entry"),
        (
            "### Added\n\n<!-- future release note -->\n",
            "HTML comments are not release notes",
        ),
        (
            "### Added\n\n- A feature.\n  <!-- hidden text -->\n",
            "HTML comments are not release notes",
        ),
        (
            "### Added\n\n#### Internal\n\n- A feature.\n",
            "unsupported nested heading",
        ),
        (
            "### Added\n\n- A feature.\n  - Hidden nested item.\n",
            "nested list entries are unsupported",
        ),
        (
            "### Added\n\n- A feature.\n\n### Added\n\n- Another feature.\n",
            "subsection 'Added' appears more than once",
        ),
        (
            "### Added\n\n### Fixed\n\n- A fix.\n",
            r"declared subsection\(s\) have no list entries: Added",
        ),
        (
            "### Fixed\n\n- **Breaking:** Not a fix.\n",
            r"\*\*Breaking:\*\* entries are allowed only under Changed",
        ),
        (
            "### Changed\n\n- **Breaking:**\n",
            r"\*\*Breaking:\*\* entry must describe",
        ),
        (
            "### Changed\n\n- A change.\n  **Breaking:** Hidden marker.\n",
            r"\*\*Breaking:\*\* must begin a Changed list entry",
        ),
        (
            "### Changed\n\n- A disguised **Breaking:** marker.\n",
            r"\*\*Breaking:\*\* must begin a Changed list entry",
        ),
    ],
)
def test_minimum_bump_rejects_malformed_release_entries(
    notes: str, message: str
) -> None:
    with pytest.raises(release_policy.ReleasePolicyError, match=message):
        release_policy.minimum_bump(_changelog(notes), "1.2.3")


@pytest.mark.parametrize(
    ("version", "message"),
    [
        (
            f"{'9' * 4_301}.0.0",
            "maximum length for u64 SemVer components",
        ),
        ("18446744073709551616.0.0", "component larger than u64"),
        ("0.18446744073709551616.0", "component larger than u64"),
        ("0.0.18446744073709551616", "component larger than u64"),
    ],
)
def test_parse_stable_version_rejects_components_larger_than_u64(
    version: str, message: str
) -> None:
    with pytest.raises(release_policy.ReleasePolicyError, match=message):
        release_policy.parse_stable_version(version)


def test_parse_stable_version_accepts_largest_u64_components() -> None:
    maximum = 18_446_744_073_709_551_615

    actual = release_policy.parse_stable_version(
        "18446744073709551615.18446744073709551615.18446744073709551615"
    )

    assert actual == (maximum, maximum, maximum)


def test_release_policy_cli_reports_inferred_bump(tmp_path: Path) -> None:
    changelog = tmp_path / "CHANGELOG.md"
    changelog.write_text(
        _changelog("### Changed\n\n- **Breaking:** Removed a field.\n"),
        encoding="utf-8",
    )

    result = subprocess.run(
        [
            "python3",
            str(POLICY_SCRIPT),
            "--changelog",
            str(changelog),
            "--current-version",
            "0.10.0",
        ],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )

    assert result.returncode == 0, result.stdout + result.stderr
    assert result.stdout == "minimum_bump=minor\n"
