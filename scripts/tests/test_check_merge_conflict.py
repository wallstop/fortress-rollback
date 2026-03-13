#!/usr/bin/env python3
"""
Unit tests for check-merge-conflict.py hook.

Verifies that the merge conflict marker checker correctly detects
conflict markers (HEAD, separator, branch) in files, and that output
follows the {path}:{line}: {message} format.
"""

from __future__ import annotations

import importlib.util
import re
import sys
from pathlib import Path

import pytest

# Import the hook module (hyphenated filename requires importlib)
scripts_dir = Path(__file__).parent.parent
spec = importlib.util.spec_from_file_location(
    "check_merge_conflict", scripts_dir / "hooks" / "check-merge-conflict.py"
)
check_merge_conflict = importlib.util.module_from_spec(spec)
spec.loader.exec_module(check_merge_conflict)

check_file = check_merge_conflict.check_file
main = check_merge_conflict.main


def _write(directory: Path, name: str, content: str) -> Path:
    """Helper to create a file with given content."""
    filepath = directory / name
    filepath.parent.mkdir(parents=True, exist_ok=True)
    filepath.write_text(content, encoding="utf-8")
    return filepath


class TestMergeConflictDetection:
    """Tests for detecting merge conflict markers."""

    def test_head_marker_detected(self, tmp_path: Path) -> None:
        """<<<<<<< HEAD marker is detected."""
        f = _write(tmp_path, "file.txt", "<<<<<<< HEAD\nsome content\n")
        assert check_file(str(f)) is False

    def test_separator_marker_detected(self, tmp_path: Path) -> None:
        """======= marker is detected."""
        f = _write(tmp_path, "file.txt", "line one\n=======\nline two\n")
        assert check_file(str(f)) is False

    def test_branch_marker_detected(self, tmp_path: Path) -> None:
        """>>>>>>> branch marker is detected."""
        f = _write(tmp_path, "file.txt", ">>>>>>> feature-branch\n")
        assert check_file(str(f)) is False

    def test_clean_file_passes(self, tmp_path: Path) -> None:
        """A file without conflict markers passes."""
        f = _write(tmp_path, "file.txt", "normal content\nnothing special\n")
        assert check_file(str(f)) is True

    def test_empty_file_passes(self, tmp_path: Path) -> None:
        """An empty file passes."""
        f = _write(tmp_path, "file.txt", "")
        assert check_file(str(f)) is True

    def test_partial_markers_not_detected(self, tmp_path: Path) -> None:
        """Markers without trailing space/newline are not detected."""
        f = _write(tmp_path, "file.txt", "<<<<<<not a marker\n")
        assert check_file(str(f)) is True

    def test_full_conflict_block_detected(self, tmp_path: Path) -> None:
        """Full conflict block is detected (stops at first marker)."""
        content = (
            "before\n"
            "<<<<<<< HEAD\n"
            "our version\n"
            "=======\n"
            "their version\n"
            ">>>>>>> feature\n"
            "after\n"
        )
        f = _write(tmp_path, "file.txt", content)
        assert check_file(str(f)) is False


class TestOutputFormat:
    """Tests that output follows {path}:{line_number}: {message} format."""

    def test_output_starts_with_path_colon_line(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Error output must start with path:line: (no leading whitespace)."""
        f = _write(tmp_path, "file.txt", "<<<<<<< HEAD\n")
        check_file(str(f))
        captured = capsys.readouterr()
        for line in captured.err.splitlines():
            if line:
                assert re.match(r'^.+:\d+: ', line), f"Bad format: {line}"
                assert not line.startswith(" "), f"Leading whitespace: {line}"

    def test_output_contains_merge_conflict_message(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Error output contains 'merge conflict marker found'."""
        f = _write(tmp_path, "file.txt", "<<<<<<< HEAD\n")
        check_file(str(f))
        captured = capsys.readouterr()
        assert "merge conflict marker found" in captured.err

    def test_correct_line_number_reported(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Line number in output matches the line with the marker."""
        f = _write(tmp_path, "file.txt", "clean\nclean\n<<<<<<< HEAD\n")
        check_file(str(f))
        captured = capsys.readouterr()
        assert ":3:" in captured.err

    def test_read_error_uses_zero_line_number(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Read error message must include :0: synthetic line number."""
        path = tmp_path / "nonexistent.txt"
        check_file(str(path))
        captured = capsys.readouterr()
        for line in captured.err.splitlines():
            if line:
                assert ":0:" in line, f"Missing :0: in read error: {line}"
                assert re.match(r'^.+:\d+: ', line), f"Bad format: {line}"


class TestMain:
    """Tests for the main() entry point."""

    def test_main_no_args_returns_zero(
        self, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        monkeypatch.setattr(sys, "argv", ["check-merge-conflict.py"])
        assert main() == 0

    def test_main_clean_file_returns_zero(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        f = _write(tmp_path, "file.txt", "clean content\n")
        monkeypatch.setattr(sys, "argv", ["check-merge-conflict.py", str(f)])
        assert main() == 0

    def test_main_conflict_returns_one(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        f = _write(tmp_path, "file.txt", "<<<<<<< HEAD\n")
        monkeypatch.setattr(sys, "argv", ["check-merge-conflict.py", str(f)])
        assert main() == 1

    def test_main_multiple_files(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        """Returns 1 if any file has conflicts."""
        clean = _write(tmp_path, "clean.txt", "ok\n")
        dirty = _write(tmp_path, "dirty.txt", "<<<<<<< HEAD\n")
        monkeypatch.setattr(
            sys, "argv", ["check-merge-conflict.py", str(clean), str(dirty)]
        )
        assert main() == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
