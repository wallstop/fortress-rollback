#!/usr/bin/env python3
"""
Unit tests for end-of-file-fixer.py hook.

Verifies that the end-of-file fixer correctly ensures files end with a single
newline, handles errors, and that output follows conventions.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

# Import the hook module (hyphenated filename requires importlib)
scripts_dir = Path(__file__).parent.parent
spec = importlib.util.spec_from_file_location(
    "end_of_file_fixer", scripts_dir / "hooks" / "end-of-file-fixer.py"
)
end_of_file_fixer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(end_of_file_fixer)

fix_file = end_of_file_fixer.fix_file
main = end_of_file_fixer.main


def _write_bytes(directory: Path, name: str, content: bytes) -> Path:
    """Helper to create a file with given byte content."""
    filepath = directory / name
    filepath.parent.mkdir(parents=True, exist_ok=True)
    filepath.write_bytes(content)
    return filepath


class TestFixFile:
    """Tests for fix_file() functionality."""

    def test_correct_ending_unchanged(self, tmp_path: Path) -> None:
        """A file ending with exactly one newline is not modified."""
        f = _write_bytes(tmp_path, "good.txt", b"hello\nworld\n")
        assert fix_file(str(f)) is False
        assert f.read_bytes() == b"hello\nworld\n"

    def test_missing_newline_added(self, tmp_path: Path) -> None:
        """A file missing a trailing newline gets one added."""
        f = _write_bytes(tmp_path, "no_nl.txt", b"hello\nworld")
        assert fix_file(str(f)) is True
        assert f.read_bytes() == b"hello\nworld\n"

    def test_extra_newlines_removed(self, tmp_path: Path) -> None:
        """Extra trailing newlines are reduced to one."""
        f = _write_bytes(tmp_path, "extra.txt", b"hello\nworld\n\n\n")
        assert fix_file(str(f)) is True
        assert f.read_bytes() == b"hello\nworld\n"

    def test_trailing_spaces_removed(self, tmp_path: Path) -> None:
        """Trailing whitespace (spaces/tabs) at end of file is removed."""
        f = _write_bytes(tmp_path, "ws.txt", b"hello\nworld\n  \t\n")
        assert fix_file(str(f)) is True
        assert f.read_bytes() == b"hello\nworld\n"

    def test_crlf_trailing_removed(self, tmp_path: Path) -> None:
        """Trailing CRLF endings are normalized."""
        f = _write_bytes(tmp_path, "crlf.txt", b"hello\r\n\r\n")
        assert fix_file(str(f)) is True
        assert f.read_bytes() == b"hello\n"

    def test_empty_file_unchanged(self, tmp_path: Path) -> None:
        """An empty file is not modified."""
        f = _write_bytes(tmp_path, "empty.txt", b"")
        assert fix_file(str(f)) is False

    def test_prints_fixed_message_on_modification(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Prints 'Fixed: <path>' to stdout when file is modified."""
        f = _write_bytes(tmp_path, "no_nl.txt", b"hello")
        fix_file(str(f))
        captured = capsys.readouterr()
        assert f"Fixed: {f}" in captured.out

    def test_no_output_when_unchanged(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """No output when file is unchanged."""
        f = _write_bytes(tmp_path, "good.txt", b"hello\n")
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
        f = _write_bytes(tmp_path, "noperm.txt", b"hello")
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
            sys, "argv", ["end-of-file-fixer.py", str(nonexistent)]
        )
        assert main() == 1

    def test_main_error_and_clean_file_returns_one(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        """main() returns 1 when any file has an error, even if others are clean."""
        clean = _write_bytes(tmp_path, "clean.txt", b"hello\n")
        missing = tmp_path / "missing.txt"
        monkeypatch.setattr(
            sys, "argv", ["end-of-file-fixer.py", str(clean), str(missing)]
        )
        assert main() == 1

    def test_main_error_and_modified_file_returns_one(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        """main() returns 1 when one file errors and another is modified."""
        dirty = _write_bytes(tmp_path, "dirty.txt", b"hello")
        missing = tmp_path / "missing.txt"
        monkeypatch.setattr(
            sys, "argv", ["end-of-file-fixer.py", str(dirty), str(missing)]
        )
        assert main() == 1
        # The dirty file should still have been fixed
        assert dirty.read_bytes() == b"hello\n"


class TestMain:
    """Tests for the main() entry point."""

    def test_main_no_args_returns_zero(
        self, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        monkeypatch.setattr(sys, "argv", ["end-of-file-fixer.py"])
        assert main() == 0

    def test_main_clean_file_returns_zero(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        f = _write_bytes(tmp_path, "good.txt", b"hello\n")
        monkeypatch.setattr(
            sys, "argv", ["end-of-file-fixer.py", str(f)]
        )
        assert main() == 0

    def test_main_modified_file_returns_one(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        f = _write_bytes(tmp_path, "no_nl.txt", b"hello")
        monkeypatch.setattr(
            sys, "argv", ["end-of-file-fixer.py", str(f)]
        )
        assert main() == 1

    def test_main_multiple_files(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        """Returns 1 if any file is modified."""
        clean = _write_bytes(tmp_path, "good.txt", b"hello\n")
        dirty = _write_bytes(tmp_path, "no_nl.txt", b"hello")
        monkeypatch.setattr(
            sys, "argv", ["end-of-file-fixer.py", str(clean), str(dirty)]
        )
        assert main() == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
