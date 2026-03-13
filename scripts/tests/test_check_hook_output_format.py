#!/usr/bin/env python3
"""
Unit tests for check-hook-output-format.py hook.

Verifies that the hook output format checker correctly detects:
- Leading whitespace in print() f-strings (breaks editor hyperlinking)
- Error messages missing line numbers (should use :0: for file-level errors)
"""

from __future__ import annotations

import importlib.util
from pathlib import Path

import pytest

# Import the hook module (hyphenated filename requires importlib)
scripts_dir = Path(__file__).parent.parent
spec = importlib.util.spec_from_file_location(
    "check_hook_output_format",
    scripts_dir / "hooks" / "check-hook-output-format.py",
)
check_hook_output_format = importlib.util.module_from_spec(spec)
spec.loader.exec_module(check_hook_output_format)

check_file = check_hook_output_format.check_file


def _write(directory: Path, name: str, content: str) -> Path:
    """Helper to create a file with given content."""
    filepath = directory / name
    filepath.parent.mkdir(parents=True, exist_ok=True)
    filepath.write_text(content, encoding="utf-8")
    return filepath


class TestIndentedPrinting:
    """Tests for leading whitespace in print() f-string detection."""

    def test_indented_print_double_quote_detected(self, tmp_path: Path) -> None:
        """print(f"  {err}") is flagged."""
        f = _write(
            tmp_path,
            "check-bad.py",
            'def main():\n    for err in errors:\n        print(f"  {err}")\n',
        )
        issues = check_file(f)
        assert len(issues) == 1
        assert "leading whitespace" in issues[0]
        assert ":3:" in issues[0]

    def test_indented_print_single_quote_detected(self, tmp_path: Path) -> None:
        """print(f'  {err}') is flagged."""
        f = _write(
            tmp_path,
            "check-bad.py",
            "def main():\n    for err in errors:\n        print(f'  {err}')\n",
        )
        issues = check_file(f)
        assert len(issues) == 1
        assert "leading whitespace" in issues[0]

    def test_print_without_indent_passes(self, tmp_path: Path) -> None:
        """print(f"{err}") without leading spaces passes."""
        f = _write(
            tmp_path,
            "check-good.py",
            'def main():\n    for err in errors:\n        print(f"{err}")\n',
        )
        issues = check_file(f)
        assert issues == []

    def test_regular_print_string_passes(self, tmp_path: Path) -> None:
        """print("  Some header text") passes (not an f-string with variable)."""
        f = _write(
            tmp_path,
            "check-good.py",
            'def main():\n    print("  Some header text")\n',
        )
        issues = check_file(f)
        assert issues == []

    def test_comment_with_pattern_skipped(self, tmp_path: Path) -> None:
        """Comment lines are not checked."""
        f = _write(
            tmp_path,
            "check-good.py",
            '# print(f"  {err}")\n',
        )
        issues = check_file(f)
        assert issues == []


class TestMissingLineNumber:
    """Tests for error messages missing line numbers."""

    def test_cannot_read_without_line_number_detected(
        self, tmp_path: Path
    ) -> None:
        """f"{filepath}: cannot read" without :0: is flagged."""
        f = _write(
            tmp_path,
            "check-bad.py",
            'def check_file(filepath):\n    return [f"{filepath}: cannot read"]\n',
        )
        issues = check_file(f)
        assert len(issues) == 1
        assert "missing line number" in issues[0]
        assert ":2:" in issues[0]

    def test_cannot_read_with_zero_line_passes(self, tmp_path: Path) -> None:
        """f"{filepath}:0: cannot read" passes."""
        f = _write(
            tmp_path,
            "check-good.py",
            'def check_file(filepath):\n    return [f"{filepath}:0: cannot read file"]\n',
        )
        issues = check_file(f)
        assert issues == []

    def test_cannot_read_with_variable_line_passes(
        self, tmp_path: Path
    ) -> None:
        """f"{filepath}:{line}: cannot read" passes."""
        f = _write(
            tmp_path,
            "check-good.py",
            'def check_file(filepath):\n    return [f"{filepath}:{line_num}: cannot read"]\n',
        )
        issues = check_file(f)
        assert issues == []

    def test_path_var_name_detected(self, tmp_path: Path) -> None:
        """f"{path}: cannot read" is detected regardless of variable name."""
        f = _write(
            tmp_path,
            "check-bad.py",
            'def check_file(path):\n    return [f"{path}: cannot read file"]\n',
        )
        issues = check_file(f)
        assert len(issues) == 1
        assert "missing line number" in issues[0]

    def test_comment_with_pattern_skipped(self, tmp_path: Path) -> None:
        """Comment lines are not checked."""
        f = _write(
            tmp_path,
            "check-good.py",
            '# f"{path}: cannot read"\n',
        )
        issues = check_file(f)
        assert issues == []


class TestFileHandling:
    """Tests for file handling edge cases."""

    def test_clean_file_returns_empty(self, tmp_path: Path) -> None:
        """A clean hook script returns no issues."""
        content = (
            "#!/usr/bin/env python3\n"
            "import sys\n"
            "\n"
            "def check_file(filepath):\n"
            '    return [f"{filepath}:0: cannot read"]\n'
            "\n"
            "def main():\n"
            "    for issue in issues:\n"
            "        print(issue, file=sys.stderr)\n"
            "    return 0\n"
        )
        f = _write(tmp_path, "check-clean.py", content)
        issues = check_file(f)
        assert issues == []

    def test_nonexistent_file_returns_empty(self, tmp_path: Path) -> None:
        """Nonexistent file returns empty list with stderr warning."""
        issues = check_file(tmp_path / "nonexistent.py")
        assert issues == []

    def test_multiple_violations_detected(self, tmp_path: Path) -> None:
        """Multiple violations in one file are all reported."""
        content = (
            'def main():\n'
            '    print(f"  {err}")\n'
            '    print(f"  {issue}")\n'
        )
        f = _write(tmp_path, "check-multi.py", content)
        issues = check_file(f)
        assert len(issues) == 2

    def test_blank_lines_skipped(self, tmp_path: Path) -> None:
        """Blank lines do not cause issues."""
        f = _write(tmp_path, "check-blank.py", "\n\n\n")
        issues = check_file(f)
        assert issues == []

    def test_issue_format_has_path_and_line(self, tmp_path: Path) -> None:
        """Issue output uses {path}:{line}: {message} format."""
        f = _write(
            tmp_path,
            "check-bad.py",
            'print(f"  {err}")\n',
        )
        issues = check_file(f)
        assert len(issues) == 1
        # Verify format: path:line: message
        parts = issues[0].split(":")
        assert len(parts) >= 3
        assert parts[1].strip().isdigit()


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
