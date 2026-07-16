#!/usr/bin/env python3
"""Unit tests for scripts/hooks/check-changelog-unreleased.py.

Covers the matrix from the rule:
    (a) clean Unreleased with only Added            -> pass
    (b) Added + only-Breaking Changed               -> pass
    (c) Added + non-Breaking Changed                -> fail with line numbers
    (d) no Unreleased section                       -> pass
    (e) Added + unmarked Fixed                      -> fail with line numbers
    (f) Added + only **Pre-existing:** Fixed        -> pass

Plus a couple of robustness checks:
    - mixed Breaking + non-Breaking Changed reports only the non-Breaking lines
    - mixed Pre-existing + unmarked Fixed reports only the unmarked lines
    - accepted **Pre-existing:** entries emit a non-failing stdout note
    - missing CHANGELOG.md is not a failure (hook is a no-op)
"""
from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

# Import the hook module (hyphenated filename requires importlib).
scripts_dir = Path(__file__).parent.parent
spec = importlib.util.spec_from_file_location(
    "check_changelog_unreleased",
    scripts_dir / "hooks" / "check-changelog-unreleased.py",
)
assert spec is not None and spec.loader is not None
check_changelog_unreleased = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = check_changelog_unreleased
spec.loader.exec_module(check_changelog_unreleased)

check_file = check_changelog_unreleased.check_file


def _write_changelog(tmp_path: Path, body: str) -> Path:
    path = tmp_path / "CHANGELOG.md"
    path.write_text(body, encoding="utf-8")
    return path


def test_clean_added_only_passes(tmp_path: Path) -> None:
    """(a) Unreleased with only Added entries passes."""
    body = (
        "# Changelog\n\n"
        "## [Unreleased]\n\n"
        "### Added\n\n"
        "- New `Foo` API for bar\n"
        "- New `Baz` API for qux\n\n"
        "## [0.1.0] - 2026-01-01\n"
    )
    assert check_file(_write_changelog(tmp_path, body)) is True


def test_added_plus_only_breaking_changed_passes(tmp_path: Path) -> None:
    """(b) Added + Changed where every entry is **Breaking:** passes."""
    body = (
        "# Changelog\n\n"
        "## [Unreleased]\n\n"
        "### Added\n\n"
        "- New `Foo` API\n\n"
        "### Changed\n\n"
        "- **Breaking:** `Bar::baz` now returns `Result`\n"
        "- **Breaking:** `Qux::quux` removed\n"
    )
    assert check_file(_write_changelog(tmp_path, body)) is True


def test_added_plus_non_breaking_changed_fails(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    """(c) Added + non-Breaking Changed fails and reports the offending lines."""
    body = (
        "# Changelog\n\n"  # 1-2
        "## [Unreleased]\n\n"  # 3-4
        "### Added\n\n"  # 5-6
        "- New `Foo` API\n\n"  # 7-8
        "### Changed\n\n"  # 9-10
        "- Tweak default for `Foo`\n"  # 11   <- offender
        "- **Breaking:** `Bar` removed\n"  # 12
        "- Adjust telemetry severity for `Foo`\n"  # 13   <- offender
    )
    path = _write_changelog(tmp_path, body)
    assert check_file(path) is False

    captured = capsys.readouterr()
    err = captured.err
    # Header diagnostic points at the '### Changed' header line (line 9).
    assert ":9: '### Changed' subsection" in err
    # Each offending entry is named with its line number and a summary.
    assert ":11: offending Changed entry:" in err
    assert "Tweak default" in err
    assert ":13: offending Changed entry:" in err
    assert "telemetry severity" in err
    # Breaking entries are NOT flagged.
    assert ":12: offending Changed entry:" not in err
    # Remedy and reference are present.
    assert "remedy:" in err
    assert ".agents/skills/fortress-development/SKILL.md" in err


def test_no_unreleased_section_passes(tmp_path: Path) -> None:
    """(d) A changelog with no [Unreleased] section is a no-op pass."""
    body = (
        "# Changelog\n\n"
        "## [0.1.0] - 2026-01-01\n\n"
        "### Added\n\n"
        "- Initial release\n"
    )
    assert check_file(_write_changelog(tmp_path, body)) is True


def test_added_plus_fixed_fails(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    """(e) Added + unmarked Fixed in Unreleased fails with line-number diagnostics."""
    body = (
        "# Changelog\n\n"  # 1-2
        "## [Unreleased]\n\n"  # 3-4
        "### Added\n\n"  # 5-6
        "- New `Foo` API\n\n"  # 7-8
        "### Fixed\n\n"  # 9-10
        "- Fixed bug in unreleased `Foo`\n"  # 11   <- offender
    )
    path = _write_changelog(tmp_path, body)
    assert check_file(path) is False

    captured = capsys.readouterr()
    err = captured.err
    assert ":9: '### Fixed' subsection" in err
    assert ":11: offending Fixed entry:" in err
    assert "Fixed bug in unreleased" in err
    assert "remedy:" in err
    # The remedy advertises the released-behavior escape hatch.
    assert "**Pre-existing:**" in err


def test_added_plus_preexisting_fixed_passes(tmp_path: Path) -> None:
    """Added + Fixed where every entry is **Pre-existing:** passes.

    Fixes to behavior that already shipped in a released version are outside
    the unreleased-code rule's scope; the marker is the author's
    self-declaration, mirroring **Breaking:** for Changed.
    """
    body = (
        "# Changelog\n\n"
        "## [Unreleased]\n\n"
        "### Added\n\n"
        "- New `Foo` API\n\n"
        "### Fixed\n\n"
        "- **Pre-existing:** `Bar` no longer desyncs under reordering\n"
        "## [0.1.0] - 2026-01-01\n"
    )
    assert check_file(_write_changelog(tmp_path, body)) is True


def test_mixed_preexisting_and_unmarked_fixed_reports_only_unmarked(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    """A Fixed block with both kinds reports only the unmarked offenders."""
    body = (
        "# Changelog\n\n"
        "## [Unreleased]\n\n"
        "### Added\n\n"
        "- New `Foo` API\n\n"
        "### Fixed\n\n"
        "- **Pre-existing:** `Bar` no longer desyncs under reordering\n"
        "- Fixed bug in unreleased `Foo`\n"  # offender
    )
    path = _write_changelog(tmp_path, body)
    assert check_file(path) is False

    captured = capsys.readouterr()
    err = captured.err
    assert err.count("offending Fixed entry:") == 1
    assert "Fixed bug in unreleased" in err
    assert "no longer desyncs" not in err
    # The accepted marked entry is still surfaced as a note (on stdout),
    # even though the file fails on the unmarked offender.
    out = captured.out
    assert out.count("note: accepting") == 1
    assert "no longer desyncs" in out


def test_accepted_preexisting_fixed_emits_stdout_note(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    """Accepted **Pre-existing:** entries emit a per-entry note on stdout.

    The note exists for reviewer visibility of the self-declaration; it must
    not affect the pass/fail outcome and must not pollute stderr (which is
    reserved for violations).
    """
    body = (
        "# Changelog\n\n"  # 1-2
        "## [Unreleased]\n\n"  # 3-4
        "### Added\n\n"  # 5-6
        "- New `Foo` API\n\n"  # 7-8
        "### Fixed\n\n"  # 9-10
        "- **Pre-existing:** `Bar` no longer desyncs under reordering\n"  # 11
        "- **Pre-existing:** `Baz` rollback no longer stalls\n"  # 12
        "## [0.1.0] - 2026-01-01\n"
    )
    path = _write_changelog(tmp_path, body)
    assert check_file(path) is True

    captured = capsys.readouterr()
    out = captured.out
    assert out.count("note: accepting") == 2
    assert ":11: note: accepting" in out
    assert ":12: note: accepting" in out
    assert "self-declaration" in out
    assert "no longer desyncs" in out
    assert "rollback no longer stalls" in out
    # Notes are informational only: nothing on stderr.
    assert captured.err == ""


def test_preexisting_fixed_without_added_emits_no_note(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    """Without an Added section the marker is not load-bearing: no note."""
    body = (
        "# Changelog\n\n"
        "## [Unreleased]\n\n"
        "### Fixed\n\n"
        "- **Pre-existing:** `Bar` no longer desyncs under reordering\n"
        "## [0.1.0] - 2026-01-01\n"
    )
    path = _write_changelog(tmp_path, body)
    assert check_file(path) is True

    captured = capsys.readouterr()
    assert "note: accepting" not in captured.out
    assert captured.err == ""


def test_mixed_breaking_and_non_breaking_reports_only_non_breaking(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    """A Changed block with both kinds reports only the non-Breaking offenders."""
    body = (
        "# Changelog\n\n"
        "## [Unreleased]\n\n"
        "### Added\n\n"
        "- `Foo` API\n\n"
        "### Changed\n\n"
        "- **Breaking:** `Bar::baz` now returns `Result`\n"
        "- Tweak unreleased `Foo` default\n"  # offender
        "- **Breaking:** `Qux::quux` removed\n"
    )
    path = _write_changelog(tmp_path, body)
    assert check_file(path) is False

    err = capsys.readouterr().err
    assert "offending Changed entry:" in err
    # Only one offender should be listed.
    assert err.count("offending Changed entry:") == 1
    assert "Tweak unreleased `Foo` default" in err


def test_added_with_only_breaking_changed_no_added_section_passes(
    tmp_path: Path,
) -> None:
    """A Changed-only Unreleased (no Added) with all-Breaking is a real release prep."""
    body = (
        "# Changelog\n\n"
        "## [Unreleased]\n\n"
        "### Changed\n\n"
        "- **Breaking:** `Bar::baz` now returns `Result`\n"
    )
    assert check_file(_write_changelog(tmp_path, body)) is True


def test_changed_only_with_non_breaking_passes_when_no_added(tmp_path: Path) -> None:
    """The rule only fires when Added is also present.

    Without an Added section the rule has no anchor to fold the entries into,
    so we leave them alone (this matches the original behavior).
    """
    body = (
        "# Changelog\n\n"
        "## [Unreleased]\n\n"
        "### Changed\n\n"
        "- Some non-breaking change\n"
    )
    assert check_file(_write_changelog(tmp_path, body)) is True
