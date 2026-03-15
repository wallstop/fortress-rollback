#!/usr/bin/env python3
"""
Unit tests for mixed-line-ending.py hook.

Verifies that the line ending fixer correctly converts CRLF/CR to LF,
handles errors, and that output follows conventions.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

# Import the hook module (hyphenated filename requires importlib)
scripts_dir = Path(__file__).parent.parent
spec = importlib.util.spec_from_file_location(
    "mixed_line_ending", scripts_dir / "hooks" / "mixed-line-ending.py"
)
mixed_line_ending = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mixed_line_ending)

fix_file = mixed_line_ending.fix_file
main = mixed_line_ending.main


def _write_bytes(directory: Path, name: str, content: bytes) -> Path:
    """Helper to create a file with given byte content."""
    filepath = directory / name
    filepath.parent.mkdir(parents=True, exist_ok=True)
    filepath.write_bytes(content)
    return filepath


class TestFixFile:
    """Tests for fix_file() functionality."""

    def test_lf_only_unchanged(self, tmp_path: Path) -> None:
        """A file with only LF endings is not modified."""
        f = _write_bytes(tmp_path, "lf.txt", b"hello\nworld\n")
        assert fix_file(str(f)) is False
        assert f.read_bytes() == b"hello\nworld\n"

    def test_crlf_converted_to_lf(self, tmp_path: Path) -> None:
        """CRLF line endings are converted to LF."""
        f = _write_bytes(tmp_path, "crlf.txt", b"hello\r\nworld\r\n")
        assert fix_file(str(f)) is True
        assert f.read_bytes() == b"hello\nworld\n"

    def test_cr_converted_to_lf(self, tmp_path: Path) -> None:
        """CR-only line endings are converted to LF."""
        f = _write_bytes(tmp_path, "cr.txt", b"hello\rworld\r")
        assert fix_file(str(f)) is True
        assert f.read_bytes() == b"hello\nworld\n"

    def test_mixed_endings_converted(self, tmp_path: Path) -> None:
        """Mixed line endings are all converted to LF."""
        f = _write_bytes(tmp_path, "mixed.txt", b"hello\r\nworld\rend\n")
        assert fix_file(str(f)) is True
        assert f.read_bytes() == b"hello\nworld\nend\n"

    def test_empty_file_unchanged(self, tmp_path: Path) -> None:
        """An empty file is not modified."""
        f = _write_bytes(tmp_path, "empty.txt", b"")
        assert fix_file(str(f)) is False

    def test_prints_fixed_message_on_modification(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Prints 'Fixed line endings: <path>' to stdout when file is modified."""
        f = _write_bytes(tmp_path, "crlf.txt", b"hello\r\nworld\r\n")
        fix_file(str(f))
        captured = capsys.readouterr()
        assert f"Fixed line endings: {f}" in captured.out

    def test_no_output_when_unchanged(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """No output when file is unchanged."""
        f = _write_bytes(tmp_path, "lf.txt", b"hello\nworld\n")
        fix_file(str(f))
        captured = capsys.readouterr()
        assert captured.out == ""
        assert captured.err == ""


class TestErrorHandling:
    """Tests that read errors cause non-zero exit (fail-closed)."""

    def test_nonexistent_file_returns_none(self, tmp_path: Path) -> None:
        """Nonexistent file returns None (error), not False."""
        result = fix_file(str(tmp_path / "nonexistent.txt"))
        assert result is None

    def test_nonexistent_file_prints_to_stderr(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Nonexistent file prints error to stderr with :0: format."""
        fix_file(str(tmp_path / "nonexistent.txt"))
        captured = capsys.readouterr()
        assert ":0:" in captured.err
        assert "cannot read file" in captured.err

    def test_unreadable_file_returns_none(self, tmp_path: Path) -> None:
        """Unreadable file returns None (error), not False."""
        f = _write_bytes(tmp_path, "noperm.txt", b"hello\r\n")
        f.chmod(0o000)
        try:
            result = fix_file(str(f))
            assert result is None
        finally:
            f.chmod(0o644)

    def test_main_nonexistent_file_returns_one(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        """main() returns 1 when a file cannot be read (fail-closed)."""
        nonexistent = tmp_path / "missing.txt"
        monkeypatch.setattr(
            sys, "argv", ["mixed-line-ending.py", str(nonexistent)]
        )
        assert main() == 1

    def test_main_error_and_clean_file_returns_one(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        """main() returns 1 when any file has an error, even if others are clean."""
        clean = _write_bytes(tmp_path, "clean.txt", b"hello\n")
        missing = tmp_path / "missing.txt"
        monkeypatch.setattr(
            sys, "argv", ["mixed-line-ending.py", str(clean), str(missing)]
        )
        assert main() == 1

    def test_main_error_and_modified_file_returns_one(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        """main() returns 1 when one file errors and another is modified."""
        dirty = _write_bytes(tmp_path, "dirty.txt", b"hello\r\n")
        missing = tmp_path / "missing.txt"
        monkeypatch.setattr(
            sys, "argv", ["mixed-line-ending.py", str(dirty), str(missing)]
        )
        assert main() == 1
        # The dirty file should still have been fixed
        assert dirty.read_bytes() == b"hello\n"


class TestMain:
    """Tests for the main() entry point."""

    def test_main_no_args_returns_zero(
        self, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        monkeypatch.setattr(sys, "argv", ["mixed-line-ending.py"])
        assert main() == 0

    def test_main_clean_file_returns_zero(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        f = _write_bytes(tmp_path, "clean.txt", b"hello\n")
        monkeypatch.setattr(
            sys, "argv", ["mixed-line-ending.py", str(f)]
        )
        assert main() == 0

    def test_main_modified_file_returns_one(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        f = _write_bytes(tmp_path, "crlf.txt", b"hello\r\n")
        monkeypatch.setattr(
            sys, "argv", ["mixed-line-ending.py", str(f)]
        )
        assert main() == 1

    def test_main_multiple_files(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        """Returns 1 if any file is modified."""
        clean = _write_bytes(tmp_path, "clean.txt", b"hello\n")
        dirty = _write_bytes(tmp_path, "dirty.txt", b"hello\r\n")
        monkeypatch.setattr(
            sys, "argv", ["mixed-line-ending.py", str(clean), str(dirty)]
        )
        assert main() == 1


class TestRelativePaths:
    """Tests that _display_path() converts absolute paths to relative."""

    def test_display_path_converts_absolute_to_relative(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        """Absolute path under CWD is converted to relative."""
        monkeypatch.chdir(tmp_path)
        f = tmp_path / "file.txt"
        f.write_text("content\n", encoding="utf-8")
        result = mixed_line_ending._display_path(str(f))
        assert result == "file.txt"

    def test_display_path_fallback_when_outside_cwd(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        """Path outside CWD falls back to original string."""
        other = tmp_path / "other"
        other.mkdir()
        cwd_dir = tmp_path / "cwd_dir"
        cwd_dir.mkdir()
        monkeypatch.chdir(cwd_dir)
        result = mixed_line_ending._display_path(str(other / "file.txt"))
        assert result == str(other / "file.txt")


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
