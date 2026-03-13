#!/usr/bin/env python3
"""
Unit tests for check-json.py hook.

Verifies that the JSON/JSONC validator correctly detects invalid JSON files,
handles JSONC comments, and that output follows the {path}:{line}: {message} format.
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
    "check_json", scripts_dir / "hooks" / "check-json.py"
)
check_json = importlib.util.module_from_spec(spec)
spec.loader.exec_module(check_json)

check_file = check_json.check_file
main = check_json.main
strip_jsonc_comments = check_json.strip_jsonc_comments


def _write(directory: Path, name: str, content: str) -> Path:
    """Helper to create a file with given content."""
    filepath = directory / name
    filepath.parent.mkdir(parents=True, exist_ok=True)
    filepath.write_text(content, encoding="utf-8")
    return filepath


class TestJsonValidation:
    """Tests for JSON validation."""

    def test_valid_json_passes(self, tmp_path: Path) -> None:
        """A valid JSON file passes."""
        f = _write(tmp_path, "data.json", '{"key": "value", "num": 42}\n')
        assert check_file(str(f)) is True

    def test_empty_object_passes(self, tmp_path: Path) -> None:
        """An empty JSON object passes."""
        f = _write(tmp_path, "empty.json", "{}\n")
        assert check_file(str(f)) is True

    def test_empty_array_passes(self, tmp_path: Path) -> None:
        """An empty JSON array passes."""
        f = _write(tmp_path, "array.json", "[]\n")
        assert check_file(str(f)) is True

    def test_invalid_json_fails(self, tmp_path: Path) -> None:
        """An invalid JSON file fails."""
        f = _write(tmp_path, "bad.json", '{"key": value}\n')
        assert check_file(str(f)) is False

    def test_trailing_comma_fails(self, tmp_path: Path) -> None:
        """JSON with trailing comma fails (not valid JSON, only JSONC comments are stripped)."""
        f = _write(tmp_path, "trailing.json", '{"key": "value",}\n')
        assert check_file(str(f)) is False

    def test_jsonc_single_line_comment_passes(self, tmp_path: Path) -> None:
        """JSONC with single-line comments passes after stripping."""
        content = '// comment\n{"key": "value"}\n'
        f = _write(tmp_path, "config.json", content)
        assert check_file(str(f)) is True

    def test_jsonc_multi_line_comment_passes(self, tmp_path: Path) -> None:
        """JSONC with multi-line comments passes after stripping."""
        content = '/* comment */\n{"key": "value"}\n'
        f = _write(tmp_path, "config.json", content)
        assert check_file(str(f)) is True

    def test_bom_stripped(self, tmp_path: Path) -> None:
        """BOM at start of file is stripped."""
        content = '\ufeff{"key": "value"}\n'
        f = _write(tmp_path, "bom.json", content)
        assert check_file(str(f)) is True


class TestStripJsoncComments:
    """Tests for the strip_jsonc_comments function."""

    def test_no_comments_unchanged(self) -> None:
        """Content without comments is unchanged."""
        content = '{"key": "value"}'
        assert strip_jsonc_comments(content) == content

    def test_single_line_comment_stripped(self) -> None:
        """Single-line comments are removed."""
        content = '// comment\n{"key": "value"}'
        result = strip_jsonc_comments(content)
        assert "//" not in result
        assert '"key"' in result

    def test_multi_line_comment_stripped(self) -> None:
        """Multi-line comments are removed."""
        content = '/* comment */{"key": "value"}'
        result = strip_jsonc_comments(content)
        assert "/*" not in result
        assert '"key"' in result

    def test_comment_inside_string_preserved(self) -> None:
        """// and /* inside strings are preserved."""
        content = '{"url": "https://example.com"}'
        assert strip_jsonc_comments(content) == content

    def test_escaped_quote_in_string(self) -> None:
        """Escaped quotes inside strings are handled correctly."""
        content = '{"key": "value with \\"quote\\""}'
        assert strip_jsonc_comments(content) == content

    def test_inline_comment_stripped(self) -> None:
        """Inline single-line comment after value is stripped."""
        content = '{"key": "value"} // inline comment\n'
        result = strip_jsonc_comments(content)
        assert "inline comment" not in result
        assert '"key"' in result


class TestOutputFormat:
    """Tests that output follows {path}:{line_number}: {message} format."""

    def test_output_starts_with_path_colon_line(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Error output must start with path:line: (no leading whitespace)."""
        f = _write(tmp_path, "bad.json", '{"key": value}\n')
        check_file(str(f))
        captured = capsys.readouterr()
        for line in captured.err.splitlines():
            if line:
                assert re.match(r'^.+:\d+: ', line), f"Bad format: {line}"
                assert not line.startswith(" "), f"Leading whitespace: {line}"

    def test_output_contains_json_error(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Error output contains 'JSON error'."""
        f = _write(tmp_path, "bad.json", '{"key": value}\n')
        check_file(str(f))
        captured = capsys.readouterr()
        assert "JSON error" in captured.err

    def test_correct_line_number_reported(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Line number from JSONDecodeError is reported."""
        f = _write(tmp_path, "bad.json", '{\n  "key": value\n}\n')
        check_file(str(f))
        captured = capsys.readouterr()
        # The error should be on line 2 where 'value' is unquoted
        assert ":2:" in captured.err

    def test_read_error_uses_zero_line_number(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Read error message must include :0: synthetic line number."""
        path = tmp_path / "nonexistent.json"
        check_file(str(path))
        captured = capsys.readouterr()
        for line in captured.err.splitlines():
            if line:
                assert ":0:" in line, f"Missing :0: in read error: {line}"
                assert re.match(r'^.+:\d+: ', line), f"Bad format: {line}"

    def test_unicode_decode_error_handled(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Invalid UTF-8 file returns False and outputs :0: format."""
        path = tmp_path / "bad_encoding.json"
        path.write_bytes(b"\x80\x81\x82 invalid utf8")
        assert check_file(str(path)) is False
        captured = capsys.readouterr()
        assert ":0:" in captured.err


class TestMain:
    """Tests for the main() entry point."""

    def test_main_no_args_returns_zero(
        self, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        monkeypatch.setattr(sys, "argv", ["check-json.py"])
        assert main() == 0

    def test_main_valid_file_returns_zero(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        f = _write(tmp_path, "good.json", '{"key": "value"}\n')
        monkeypatch.setattr(sys, "argv", ["check-json.py", str(f)])
        assert main() == 0

    def test_main_invalid_file_returns_one(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        f = _write(tmp_path, "bad.json", '{"key": value}\n')
        monkeypatch.setattr(sys, "argv", ["check-json.py", str(f)])
        assert main() == 1

    def test_main_multiple_files(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        """Returns 1 if any file is invalid."""
        good = _write(tmp_path, "good.json", '{"key": "value"}\n')
        bad = _write(tmp_path, "bad.json", '{"key": value}\n')
        monkeypatch.setattr(
            sys, "argv", ["check-json.py", str(good), str(bad)]
        )
        assert main() == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
