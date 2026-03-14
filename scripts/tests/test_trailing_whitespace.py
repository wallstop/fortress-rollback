#!/usr/bin/env python3
"""
Unit tests for trailing-whitespace.py hook.

Verifies that the trailing whitespace fixer correctly removes trailing
whitespace, handles errors, and that output follows conventions.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

# Import the hook module (hyphenated filename requires importlib)
scripts_dir = Path(__file__).parent.parent
spec = importlib.util.spec_from_file_location(
    "trailing_whitespace", scripts_dir / "hooks" / "trailing-whitespace.py"
)
trailing_whitespace = importlib.util.module_from_spec(spec)
spec.loader.exec_module(trailing_whitespace)

fix_file = trailing_whitespace.fix_file
main = trailing_whitespace.main


def _write(directory: Path, name: str, content: str) -> Path:
    """Helper to create a file with given content."""
    filepath = directory / name
    filepath.parent.mkdir(parents=True, exist_ok=True)
    filepath.write_text(content, encoding="utf-8")
    return filepath


class TestFixFile:
    """Tests for fix_file() functionality."""

    def test_no_trailing_whitespace_unchanged(self, tmp_path: Path) -> None:
        """A file without trailing whitespace is not modified."""
        f = _write(tmp_path, "clean.txt", "hello\nworld\n")
        assert fix_file(str(f)) is False
        assert f.read_text(encoding="utf-8") == "hello\nworld\n"

    def test_trailing_spaces_removed(self, tmp_path: Path) -> None:
        """Trailing spaces are removed."""
        f = _write(tmp_path, "spaces.txt", "hello   \nworld  \n")
        assert fix_file(str(f)) is True
        assert f.read_text(encoding="utf-8") == "hello\nworld\n"

    def test_trailing_tabs_removed(self, tmp_path: Path) -> None:
        """Trailing tabs are removed."""
        f = _write(tmp_path, "tabs.txt", "hello\t\nworld\t\t\n")
        assert fix_file(str(f)) is True
        assert f.read_text(encoding="utf-8") == "hello\nworld\n"

    def test_preserves_line_endings_lf(self, tmp_path: Path) -> None:
        """LF line endings are preserved."""
        f = _write(tmp_path, "lf.txt", "hello  \nworld  \n")
        fix_file(str(f))
        assert f.read_text(encoding="utf-8") == "hello\nworld\n"

    def test_empty_file_unchanged(self, tmp_path: Path) -> None:
        """An empty file is not modified."""
        f = _write(tmp_path, "empty.txt", "")
        assert fix_file(str(f)) is False

    def test_prints_fixed_message_on_modification(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Prints 'Fixed: <path>' to stdout when file is modified."""
        f = _write(tmp_path, "ws.txt", "hello  \n")
        fix_file(str(f))
        captured = capsys.readouterr()
        assert f"Fixed: {f}" in captured.out

    def test_no_output_when_unchanged(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """No output when file is unchanged."""
        f = _write(tmp_path, "clean.txt", "hello\n")
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
        f = _write(tmp_path, "noperm.txt", "hello  \n")
        f.chmod(0o000)
        try:
            result = fix_file(str(f))
            assert result is None
        finally:
            f.chmod(0o644)

    def test_binary_file_returns_none(self, tmp_path: Path) -> None:
        """Binary (non-UTF-8) file returns None (error), not False."""
        f = tmp_path / "binary.txt"
        f.write_bytes(b"\xff\xfe\x00\x01")
        result = fix_file(str(f))
        assert result is None

    def test_main_nonexistent_file_returns_one(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        """main() returns 1 when a file cannot be read (fail-closed)."""
        nonexistent = tmp_path / "missing.txt"
        monkeypatch.setattr(
            sys, "argv", ["trailing-whitespace.py", str(nonexistent)]
        )
        assert main() == 1

    def test_main_binary_file_returns_one(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        """main() returns 1 when a binary file cannot be decoded."""
        f = tmp_path / "binary.txt"
        f.write_bytes(b"\xff\xfe\x00\x01")
        monkeypatch.setattr(
            sys, "argv", ["trailing-whitespace.py", str(f)]
        )
        assert main() == 1

    def test_main_error_and_clean_file_returns_one(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        """main() returns 1 when any file has an error, even if others are clean."""
        clean = _write(tmp_path, "clean.txt", "hello\n")
        missing = tmp_path / "missing.txt"
        monkeypatch.setattr(
            sys, "argv", ["trailing-whitespace.py", str(clean), str(missing)]
        )
        assert main() == 1

    def test_main_error_and_modified_file_returns_one(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        """main() returns 1 when one file errors and another is modified."""
        dirty = _write(tmp_path, "dirty.txt", "hello  \n")
        missing = tmp_path / "missing.txt"
        monkeypatch.setattr(
            sys, "argv", ["trailing-whitespace.py", str(dirty), str(missing)]
        )
        assert main() == 1
        # The dirty file should still have been fixed
        assert dirty.read_text(encoding="utf-8") == "hello\n"


class TestMain:
    """Tests for the main() entry point."""

    def test_main_no_args_returns_zero(
        self, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        monkeypatch.setattr(sys, "argv", ["trailing-whitespace.py"])
        assert main() == 0

    def test_main_clean_file_returns_zero(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        f = _write(tmp_path, "clean.txt", "hello\n")
        monkeypatch.setattr(
            sys, "argv", ["trailing-whitespace.py", str(f)]
        )
        assert main() == 0

    def test_main_modified_file_returns_one(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        f = _write(tmp_path, "ws.txt", "hello  \n")
        monkeypatch.setattr(
            sys, "argv", ["trailing-whitespace.py", str(f)]
        )
        assert main() == 1

    def test_main_multiple_files(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        """Returns 1 if any file is modified."""
        clean = _write(tmp_path, "clean.txt", "hello\n")
        dirty = _write(tmp_path, "dirty.txt", "hello  \n")
        monkeypatch.setattr(
            sys, "argv", ["trailing-whitespace.py", str(clean), str(dirty)]
        )
        assert main() == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
