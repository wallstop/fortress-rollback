#!/usr/bin/env python3
"""Unit tests for scripts/hooks/agent-vale-advisory.py.

Focuses on the path-bucketing logic in ``_summarize_lines``: the wrapper
must correctly group findings per file across Linux, macOS, and Windows
path conventions, and must not place Windows drive-letter paths in the wrong bucket
(``C:\\foo.md:14:71:...``) where a naive split-on-first-colon would key
the bucket on the drive letter.
"""
from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

# Import the hook module (hyphenated filename requires importlib).
scripts_dir = Path(__file__).parent.parent
spec = importlib.util.spec_from_file_location(
    "agent_vale_advisory",
    scripts_dir / "hooks" / "agent-vale-advisory.py",
)
assert spec is not None and spec.loader is not None
agent_vale_advisory = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = agent_vale_advisory
spec.loader.exec_module(agent_vale_advisory)

_summarize_lines = agent_vale_advisory._summarize_lines
_extract_path = agent_vale_advisory._extract_path


def test_linux_path_bucketing() -> None:
    """POSIX paths (no drive letter) bucket on the path before line:col."""
    stdout = (
        "docs/user-guide.md:12:5:Vale.Avoid:warning:Avoid using 'simply'.\n"
        "docs/user-guide.md:40:1:Vale.Spelling:suggestion:Did you mean ...\n"
        "docs/migration.md:7:9:Vale.Avoid:warning:Avoid using 'just'.\n"
    )
    counts = _summarize_lines(stdout)
    assert counts == {
        "docs/user-guide.md": 2,
        "docs/migration.md": 1,
    }


def test_windows_drive_letter_path_bucketing() -> None:
    """Windows drive-letter paths must not be split at the drive colon."""
    stdout = (
        "C:\\repo\\docs\\user-guide.md:12:5:Vale.Avoid:warning:Avoid 'simply'.\n"
        "C:\\repo\\docs\\user-guide.md:40:1:Vale.Spelling:suggestion:msg\n"
        "D:\\other\\docs\\migration.md:7:9:Vale.Avoid:warning:Avoid 'just'.\n"
    )
    counts = _summarize_lines(stdout)
    # Each unique file path -- not each drive letter -- gets its own bucket.
    assert counts == {
        "C:\\repo\\docs\\user-guide.md": 2,
        "D:\\other\\docs\\migration.md": 1,
    }
    # Belt-and-braces: a naive partition would key on "C" or "D".
    assert "C" not in counts
    assert "D" not in counts


def test_malformed_lines_ignored_gracefully() -> None:
    """Lines that don't match Vale's format are ignored, not incorrectly bucketed."""
    stdout = (
        "docs/user-guide.md:12:5:Vale.Avoid:warning:OK\n"
        "\n"  # blank
        "this is not a vale finding\n"  # no colons
        "::::\n"  # all colons
        "weird:thing:without:digits\n"  # has colons but no line:col digits
        "docs/migration.md:1:1:R:s:m\n"
    )
    counts = _summarize_lines(stdout)
    assert counts == {
        "docs/user-guide.md": 1,
        "docs/migration.md": 1,
    }


def test_extract_path_returns_none_for_non_match() -> None:
    """Single-line helper returns None for malformed input."""
    assert _extract_path("") is None
    assert _extract_path("not a finding") is None
    assert _extract_path("path/without/digits:foo:bar:baz") is None


def test_extract_path_handles_posix_and_windows() -> None:
    """Single-line helper extracts the path correctly for both styles."""
    assert (
        _extract_path("docs/user-guide.md:12:5:Vale.Avoid:warning:msg")
        == "docs/user-guide.md"
    )
    assert (
        _extract_path("C:\\repo\\docs\\guide.md:12:5:Vale.Avoid:warning:msg")
        == "C:\\repo\\docs\\guide.md"
    )
    # macOS-style absolute path (POSIX, but with spaces and dots)
    assert (
        _extract_path("/Users/me/My Docs/guide.md:1:1:R:s:m")
        == "/Users/me/My Docs/guide.md"
    )
