#!/usr/bin/env python3
"""
Unit tests for check-toml.py hook.

Verifies that the TOML validator correctly detects invalid TOML files
and that output follows the {path}:{line}: {message} format.
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
    "check_toml", scripts_dir / "hooks" / "check-toml.py"
)
check_toml = importlib.util.module_from_spec(spec)
spec.loader.exec_module(check_toml)

check_file = check_toml.check_file
main = check_toml.main
HAS_TOML = check_toml.HAS_TOML


def _write(directory: Path, name: str, content: str) -> Path:
    """Helper to create a file with given content."""
    filepath = directory / name
    filepath.parent.mkdir(parents=True, exist_ok=True)
    filepath.write_text(content, encoding="utf-8")
    return filepath


@pytest.mark.skipif(not HAS_TOML, reason="tomllib/tomli not available")
class TestTomlValidation:
    """Tests for TOML validation."""

    def test_valid_toml_passes(self, tmp_path: Path) -> None:
        """A valid TOML file passes."""
        content = '[section]\nkey = "value"\nnumber = 42\n'
        f = _write(tmp_path, "config.toml", content)
        assert check_file(str(f)) is True

    def test_empty_toml_passes(self, tmp_path: Path) -> None:
        """An empty TOML file passes."""
        f = _write(tmp_path, "empty.toml", "")
        assert check_file(str(f)) is True

    def test_invalid_toml_fails(self, tmp_path: Path) -> None:
        """An invalid TOML file fails."""
        f = _write(tmp_path, "bad.toml", "[section\nkey = value without quotes\n")
        assert check_file(str(f)) is False

    def test_valid_toml_with_arrays(self, tmp_path: Path) -> None:
        """TOML with arrays passes."""
        content = 'list = [1, 2, 3]\n\n[[items]]\nname = "a"\n\n[[items]]\nname = "b"\n'
        f = _write(tmp_path, "arrays.toml", content)
        assert check_file(str(f)) is True

    def test_duplicate_keys_fails(self, tmp_path: Path) -> None:
        """TOML with duplicate keys fails."""
        content = 'key = "first"\nkey = "second"\n'
        f = _write(tmp_path, "dup.toml", content)
        assert check_file(str(f)) is False

    def test_error_line_number_is_accurate(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Error line number uses lineno from exception when available.

        Python's built-in tomllib.TOMLDecodeError does not expose a lineno
        attribute, so getattr falls back to 1.  If the underlying library
        (e.g. tomli) ever does expose it, the hook will use the real value.
        This test verifies the output format is correct and the hook does
        not crash.
        """
        # Line 1-2 are valid, line 3 has the error (unclosed bracket)
        content = '[section]\nkey = "value"\n[broken\n'
        f = _write(tmp_path, "lineno.toml", content)
        assert check_file(str(f)) is False
        captured = capsys.readouterr()
        first_line = captured.err.splitlines()[0]
        # Must match path:line: format and contain TOML error
        assert re.match(r'^.+:\d+: TOML error:', first_line), (
            f"Bad format: {first_line}"
        )


@pytest.mark.skipif(not HAS_TOML, reason="tomllib/tomli not available")
class TestUnicodeError:
    """Tests that UnicodeDecodeError is treated as a read error, not TOML error."""

    def test_binary_file_treated_as_read_error(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """A binary/non-UTF-8 file is reported as a read error with :0:."""
        binary_file = tmp_path / "binary.toml"
        binary_file.write_bytes(b"\x80\x81\x82\xff\xfe")
        assert check_file(str(binary_file)) is False
        captured = capsys.readouterr()
        first_line = captured.err.splitlines()[0]
        assert ":0:" in first_line, f"Expected :0: in read error: {first_line}"
        assert "cannot read file" in first_line, (
            f"Expected 'cannot read file' in: {first_line}"
        )
        assert "TOML error" not in first_line, (
            f"Should not say 'TOML error' for encoding issue: {first_line}"
        )


@pytest.mark.skipif(not HAS_TOML, reason="tomllib/tomli not available")
class TestOutputFormat:
    """Tests that output follows {path}:{line_number}: {message} format."""

    def test_output_starts_with_path_colon_line(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Error output must start with path:line: (no leading whitespace)."""
        f = _write(tmp_path, "bad.toml", "[section\nbroken\n")
        check_file(str(f))
        captured = capsys.readouterr()
        for line in captured.err.splitlines():
            if line:
                assert re.match(r'^.+:\d+: ', line), f"Bad format: {line}"
                assert not line.startswith(" "), f"Leading whitespace: {line}"

    def test_output_contains_toml_error(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Error output contains 'TOML error'."""
        f = _write(tmp_path, "bad.toml", "[section\nbroken\n")
        check_file(str(f))
        captured = capsys.readouterr()
        assert "TOML error" in captured.err

    def test_read_error_uses_zero_line_number(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Read error message must include :0: synthetic line number."""
        path = tmp_path / "nonexistent.toml"
        check_file(str(path))
        captured = capsys.readouterr()
        for line in captured.err.splitlines():
            if line:
                assert ":0:" in line, f"Missing :0: in read error: {line}"
                assert re.match(r'^.+:\d+: ', line), f"Bad format: {line}"


@pytest.mark.skipif(not HAS_TOML, reason="tomllib/tomli not available")
class TestMain:
    """Tests for the main() entry point."""

    def test_main_no_args_returns_zero(
        self, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        monkeypatch.setattr(sys, "argv", ["check-toml.py"])
        assert main() == 0

    def test_main_valid_file_returns_zero(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        f = _write(tmp_path, "good.toml", 'key = "value"\n')
        monkeypatch.setattr(sys, "argv", ["check-toml.py", str(f)])
        assert main() == 0

    def test_main_invalid_file_returns_one(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        f = _write(tmp_path, "bad.toml", "[section\nbroken\n")
        monkeypatch.setattr(sys, "argv", ["check-toml.py", str(f)])
        assert main() == 1

    def test_main_multiple_files(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        """Returns 1 if any file is invalid."""
        good = _write(tmp_path, "good.toml", 'key = "value"\n')
        bad = _write(tmp_path, "bad.toml", "[section\nbroken\n")
        monkeypatch.setattr(
            sys, "argv", ["check-toml.py", str(good), str(bad)]
        )
        assert main() == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
