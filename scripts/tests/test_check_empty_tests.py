#!/usr/bin/env python3
"""
Unit tests for check-empty-tests.py hook.

Verifies detection of empty test methods/functions and output format compliance.
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
    "check_empty_tests", scripts_dir / "hooks" / "check-empty-tests.py"
)
check_empty_tests = importlib.util.module_from_spec(spec)
spec.loader.exec_module(check_empty_tests)

check_file = check_empty_tests.check_file
main = check_empty_tests.main


def _write(directory: Path, name: str, content: str) -> Path:
    filepath = directory / name
    filepath.parent.mkdir(parents=True, exist_ok=True)
    filepath.write_text(content, encoding="utf-8")
    return filepath


class TestEmptyTestDetection:
    """Tests for detecting empty test functions/methods."""

    def test_empty_test_function_detected(self, tmp_path: Path) -> None:
        f = _write(tmp_path, "test_example.py", "def test_something():\n    pass\n")
        errors = check_file(f)
        assert len(errors) == 1
        assert "test_something" in errors[0]

    def test_empty_test_method_detected(self, tmp_path: Path) -> None:
        content = (
            "class TestFoo:\n"
            "    def test_bar(self):\n"
            "        pass\n"
        )
        f = _write(tmp_path, "test_example.py", content)
        errors = check_file(f)
        assert len(errors) == 1
        assert "test_bar" in errors[0]

    def test_nonempty_test_passes(self, tmp_path: Path) -> None:
        content = (
            "def test_something():\n"
            "    assert True\n"
        )
        f = _write(tmp_path, "test_example.py", content)
        errors = check_file(f)
        assert errors == []

    def test_docstring_only_test_detected(self, tmp_path: Path) -> None:
        content = (
            "def test_something():\n"
            '    """This test does nothing."""\n'
        )
        f = _write(tmp_path, "test_example.py", content)
        errors = check_file(f)
        assert len(errors) == 1

    def test_ellipsis_only_test_detected(self, tmp_path: Path) -> None:
        content = (
            "def test_something():\n"
            "    ...\n"
        )
        f = _write(tmp_path, "test_example.py", content)
        errors = check_file(f)
        assert len(errors) == 1

    def test_non_test_function_ignored(self, tmp_path: Path) -> None:
        f = _write(tmp_path, "test_example.py", "def helper():\n    pass\n")
        errors = check_file(f)
        assert errors == []

    def test_syntax_error_file_returns_empty(self, tmp_path: Path) -> None:
        f = _write(tmp_path, "test_example.py", "def test_foo(\n")
        errors = check_file(f)
        assert errors == []


class TestOutputFormat:
    """Tests that output follows {path}:{line_number}: {message} format."""

    def test_issues_start_with_path_colon_line(self, tmp_path: Path) -> None:
        f = _write(tmp_path, "test_example.py", "def test_foo():\n    pass\n")
        errors = check_file(f)
        assert len(errors) == 1
        assert re.match(r'^.+:\d+: ', errors[0]), f"Bad format: {errors[0]}"
        assert not errors[0].startswith(" "), f"Leading whitespace: {errors[0]}"

    def test_read_error_includes_line_number(self, tmp_path: Path) -> None:
        path = tmp_path / "test_nonexistent.py"
        errors = check_file(path)
        assert len(errors) == 1
        assert ":0:" in errors[0], f"Missing :0: in read error: {errors[0]}"
        assert re.match(r'^.+:\d+: ', errors[0]), f"Bad format: {errors[0]}"

    def test_main_output_no_leading_whitespace(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
        capsys: pytest.CaptureFixture[str],
    ) -> None:
        f = _write(tmp_path, "test_example.py", "def test_foo():\n    pass\n")
        monkeypatch.setattr(sys, "argv", ["check-empty-tests.py", str(f)])
        main()
        captured = capsys.readouterr()
        for line in captured.err.splitlines():
            if line and not line.startswith(("Empty test", "Test methods")):
                assert not line.startswith("  "), f"Leading indent: {line!r}"


class TestFileErrors:
    """Tests for file read error handling."""

    def test_nonexistent_file_no_warning_prefix(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str],
    ) -> None:
        """check_file on a non-existent file does not print a Warning: line.

        Only the formatted {path}:0: message should appear (via main's loop),
        not a duplicate Warning: line from check_file itself.
        """
        path = tmp_path / "test_nonexistent.py"
        errors = check_file(path)
        captured = capsys.readouterr()
        assert len(errors) == 1
        assert ":0:" in errors[0]
        assert "Warning:" not in captured.err


class TestMain:
    """Tests for the main() entry point."""

    def test_main_no_args_returns_zero(self, monkeypatch: pytest.MonkeyPatch) -> None:
        monkeypatch.setattr(sys, "argv", ["check-empty-tests.py"])
        assert main() == 0

    def test_main_clean_file_returns_zero(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        f = _write(tmp_path, "test_good.py", "def test_foo():\n    assert 1 == 1\n")
        monkeypatch.setattr(sys, "argv", ["check-empty-tests.py", str(f)])
        assert main() == 0

    def test_main_violation_returns_one(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        f = _write(tmp_path, "test_bad.py", "def test_foo():\n    pass\n")
        monkeypatch.setattr(sys, "argv", ["check-empty-tests.py", str(f)])
        assert main() == 1

    def test_main_skips_non_test_files(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        f = _write(tmp_path, "helper.py", "def test_foo():\n    pass\n")
        monkeypatch.setattr(sys, "argv", ["check-empty-tests.py", str(f)])
        assert main() == 0

    def test_main_checks_test_suffix_files(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        f = _write(tmp_path, "example_test.py", "def test_foo():\n    pass\n")
        monkeypatch.setattr(sys, "argv", ["check-empty-tests.py", str(f)])
        assert main() == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
