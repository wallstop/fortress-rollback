#!/usr/bin/env python3
"""
Unit tests for check-hook-output-format.py hook.

Verifies that the hook output format checker correctly detects:
- Leading whitespace in print() f-strings (breaks editor hyperlinking)
- Error messages missing line numbers (should use :0: for file-level errors)
- Warning: prints that bypass {path}:{line}: format convention
- print() followed by return-in-list (causes duplicate output)
- ERROR:/WARNING: diagnostic prints going to stdout (must use file=sys.stderr)
"""

from __future__ import annotations

import importlib.util
import sys
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

    def test_indented_print_rf_prefix_detected(self, tmp_path: Path) -> None:
        """print(rf"  {err}") with rf prefix is flagged."""
        f = _write(
            tmp_path,
            "check-bad.py",
            'def main():\n    print(rf"  {err}")\n',
        )
        issues = check_file(f)
        assert len(issues) == 1
        assert "leading whitespace" in issues[0]

    def test_indented_print_fr_prefix_detected(self, tmp_path: Path) -> None:
        """print(fr"  {err}") with fr prefix is flagged."""
        f = _write(
            tmp_path,
            "check-bad.py",
            "def main():\n    print(fr\"  {err}\")\n",
        )
        issues = check_file(f)
        assert len(issues) == 1
        assert "leading whitespace" in issues[0]

    def test_concat_leading_whitespace_double_quote_detected(
        self, tmp_path: Path
    ) -> None:
        """print("  " + f"...") is flagged (Check 1b)."""
        f = _write(
            tmp_path,
            "check-bad.py",
            'def main():\n    print("  " + f"{prefix} {issue}")\n',
        )
        issues = check_file(f)
        assert any("leading" in i and "whitespace" in i for i in issues), (
            f"Expected leading-whitespace warning, got: {issues}"
        )

    def test_concat_leading_whitespace_single_quote_detected(
        self, tmp_path: Path
    ) -> None:
        """print('  ' + ...) is flagged (Check 1b)."""
        f = _write(
            tmp_path,
            "check-bad.py",
            "def main():\n    print('  ' + f'{prefix} {issue}')\n",
        )
        issues = check_file(f)
        assert any("leading" in i and "whitespace" in i for i in issues), (
            f"Expected leading-whitespace warning, got: {issues}"
        )

    def test_concat_no_leading_whitespace_passes(self, tmp_path: Path) -> None:
        """print("prefix" + ...) without leading spaces passes (Check 1b)."""
        f = _write(
            tmp_path,
            "check-good.py",
            'def main():\n    print("prefix" + f" {issue}")\n',
        )
        issues = check_file(f)
        concat_issues = [i for i in issues if "concatenated" in i and "leading" in i]
        assert not concat_issues

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

    def test_cannot_read_single_quote_detected(self, tmp_path: Path) -> None:
        """f'{filepath}: cannot read' with single quotes is flagged."""
        f = _write(
            tmp_path,
            "check-bad.py",
            "def check_file(filepath):\n    return [f'{filepath}: cannot read']\n",
        )
        issues = check_file(f)
        assert len(issues) == 1
        assert "missing line number" in issues[0]

    def test_cannot_read_rf_prefix_detected(self, tmp_path: Path) -> None:
        """rf"{filepath}: cannot read" with rf prefix is flagged."""
        f = _write(
            tmp_path,
            "check-bad.py",
            'def check_file(filepath):\n    return [rf"{filepath}: cannot read"]\n',
        )
        issues = check_file(f)
        assert len(issues) == 1
        assert "missing line number" in issues[0]

    def test_cannot_read_fr_prefix_detected(self, tmp_path: Path) -> None:
        """fr"{filepath}: cannot read" with fr prefix is flagged."""
        f = _write(
            tmp_path,
            "check-bad.py",
            'def check_file(filepath):\n    return [fr"{filepath}: cannot read"]\n',
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


class TestWarningPrefix:
    """Tests for Warning: print detection."""

    def test_warning_print_double_quote_detected(self, tmp_path: Path) -> None:
        """print(f"Warning: ...") is flagged."""
        f = _write(
            tmp_path,
            "check-bad.py",
            'def check_file(path):\n    print(f"Warning: cannot read {path}")\n',
        )
        issues = check_file(f)
        assert len(issues) == 1
        assert "Warning:" in issues[0]
        assert ":2:" in issues[0]

    def test_warning_print_single_quote_detected(self, tmp_path: Path) -> None:
        """print(f'Warning: ...') is flagged."""
        f = _write(
            tmp_path,
            "check-bad.py",
            "def check_file(path):\n    print(f'Warning: cannot read {path}')\n",
        )
        issues = check_file(f)
        assert len(issues) == 1
        assert "Warning:" in issues[0]

    def test_warning_without_f_prefix_passes(self, tmp_path: Path) -> None:
        """print("Warning: ...") without f-prefix passes (static text is OK)."""
        f = _write(
            tmp_path,
            "check-good.py",
            'def main():\n    print("Warning: tomllib not available")\n',
        )
        issues = check_file(f)
        assert issues == []

    def test_formatted_error_passes(self, tmp_path: Path) -> None:
        """print(f"{path}:0: cannot read") passes (correct format)."""
        f = _write(
            tmp_path,
            "check-good.py",
            'def check_file(path):\n    print(f"{path}:0: cannot read file")\n',
        )
        issues = check_file(f)
        assert issues == []

    def test_comment_with_warning_pattern_skipped(self, tmp_path: Path) -> None:
        """Comment lines are not checked."""
        f = _write(
            tmp_path,
            "check-good.py",
            '# print(f"Warning: cannot read {path}")\n',
        )
        issues = check_file(f)
        assert issues == []


class TestDualOutputDetection:
    """Tests for Check 4: print() followed by return-in-list detection."""

    def test_print_then_return_list_detected(self, tmp_path: Path) -> None:
        """print(file=sys.stderr) followed by return [...] is flagged."""
        content = (
            "def check_file(filepath):\n"
            "    except OSError as exc:\n"
            "        print(f'{filepath}:0: error: {exc}', file=sys.stderr)\n"
            "        return [f'{filepath}:0: error: {exc}']\n"
        )
        f = _write(tmp_path, "check-bad.py", content)
        issues = check_file(f)
        assert any("duplicate output" in i for i in issues), f"Expected dual-output warning, got: {issues}"

    def test_print_then_return_list_with_gap_detected(
        self, tmp_path: Path
    ) -> None:
        """print(file=sys.stderr) with 1-2 blank lines before return [...] is flagged."""
        content = (
            "def check_file(filepath):\n"
            "    except OSError as exc:\n"
            "        msg = f'{filepath}:0: error'\n"
            "        print(msg, file=sys.stderr)\n"
            "\n"
            "        return [msg]\n"
        )
        f = _write(tmp_path, "check-bad.py", content)
        issues = check_file(f)
        assert any("duplicate output" in i for i in issues), f"Expected dual-output warning, got: {issues}"

    def test_print_then_return_bool_passes(self, tmp_path: Path) -> None:
        """print(file=sys.stderr) followed by return False passes (no list)."""
        content = (
            "def check_file(filepath):\n"
            "    except OSError as exc:\n"
            "        print(f'{filepath}:0: error: {exc}', file=sys.stderr)\n"
            "        return False\n"
        )
        f = _write(tmp_path, "check-good.py", content)
        issues = check_file(f)
        assert not any("duplicate output" in i for i in issues)

    def test_print_in_main_loop_passes(self, tmp_path: Path) -> None:
        """print() in main() loop that iterates issues passes (no return-list nearby)."""
        content = (
            "def main():\n"
            "    for issue in issues:\n"
            "        print(issue, file=sys.stderr)\n"
            "    return 1\n"
        )
        f = _write(tmp_path, "check-good.py", content)
        issues = check_file(f)
        assert not any("duplicate output" in i for i in issues)

    def test_print_then_return_empty_list_passes(self, tmp_path: Path) -> None:
        """print(file=sys.stderr) followed by return [] passes (empty list is not dual-output)."""
        content = (
            "def check_file(filepath):\n"
            "    except OSError as exc:\n"
            "        print(f'{filepath}:0: error: {exc}', file=sys.stderr)\n"
            "        return []\n"
        )
        f = _write(tmp_path, "check-good.py", content)
        issues = check_file(f)
        assert not any("duplicate output" in i for i in issues)

    def test_print_far_from_return_list_passes(self, tmp_path: Path) -> None:
        """print(file=sys.stderr) more than 3 lines before return [...] passes."""
        content = (
            "def check_file(filepath):\n"
            "    print(f'{filepath}:0: error', file=sys.stderr)\n"
            "    do_something()\n"
            "    do_more()\n"
            "    yet_more()\n"
            "    even_more()\n"
            "    return [f'{filepath}:0: error']\n"
        )
        f = _write(tmp_path, "check-good.py", content)
        issues = check_file(f)
        assert not any("duplicate output" in i for i in issues)


class TestNoDuplicateOutput:
    """Tests that check_file() read errors don't produce duplicate stderr output."""

    def test_nonexistent_file_no_stderr_from_check_file(
        self,
        tmp_path: Path,
        capsys: pytest.CaptureFixture[str],
    ) -> None:
        """check_file() on a nonexistent file does not print to stderr itself."""
        issues = check_file(tmp_path / "nonexistent.py")
        captured = capsys.readouterr()
        assert len(issues) == 1
        assert "cannot read file" in issues[0]
        # check_file must NOT print -- the caller (main) prints
        assert captured.err == ""

    def test_main_prints_read_error_exactly_once(
        self,
        tmp_path: Path,
        monkeypatch: pytest.MonkeyPatch,
        capsys: pytest.CaptureFixture[str],
    ) -> None:
        """main() prints the read-error message exactly once (no duplicates)."""
        nonexistent = tmp_path / "check-missing.py"
        monkeypatch.setattr(
            sys, "argv", ["check-hook-output-format.py", str(nonexistent)]
        )
        check_hook_output_format.main()
        captured = capsys.readouterr()
        error_lines = [
            line for line in captured.err.splitlines()
            if "cannot read file" in line
        ]
        assert len(error_lines) == 1


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

    def test_nonexistent_file_fails_closed(self, tmp_path: Path) -> None:
        """Nonexistent file returns error issue (fail-closed)."""
        issues = check_file(tmp_path / "nonexistent.py")
        assert len(issues) == 1
        assert "cannot read file" in issues[0]
        assert ":0:" in issues[0]

    def test_unreadable_file_fails_closed(self, tmp_path: Path) -> None:
        """Unreadable file returns error issue (fail-closed)."""
        f = _write(tmp_path, "check-unreadable.py", "content")
        f.chmod(0o000)
        try:
            issues = check_file(f)
            assert len(issues) == 1
            assert "cannot read file" in issues[0]
            assert ":0:" in issues[0]
        finally:
            f.chmod(0o644)  # Restore for cleanup

    def test_binary_file_fails_closed(self, tmp_path: Path) -> None:
        """Binary (non-UTF-8) file returns error issue (fail-closed)."""
        f = tmp_path / "check-binary.py"
        f.write_bytes(b"\xff\xfe\x00\x01")
        issues = check_file(f)
        assert len(issues) == 1
        assert "cannot read file" in issues[0]
        assert ":0:" in issues[0]

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


class TestFailOpenDetection:
    """Tests for Check 5: except-pass/return-fallback fail-open detection."""

    def test_except_oserror_pass_detected(self, tmp_path: Path) -> None:
        """except OSError followed by pass is flagged."""
        content = (
            "def check_file(filepath):\n"
            "    try:\n"
            "        content = filepath.read_text()\n"
            "    except OSError:\n"
            "        pass\n"
        )
        f = _write(tmp_path, "check-bad.py", content)
        issues = check_file(f)
        assert any("swallows" in i and "pass" in i for i in issues), (
            f"Expected fail-open warning, got: {issues}"
        )

    def test_except_unicode_error_pass_detected(self, tmp_path: Path) -> None:
        """except UnicodeDecodeError followed by pass is flagged."""
        content = (
            "def check_file(filepath):\n"
            "    try:\n"
            "        content = filepath.read_text()\n"
            "    except UnicodeDecodeError:\n"
            "        pass\n"
        )
        f = _write(tmp_path, "check-bad.py", content)
        issues = check_file(f)
        assert any("swallows" in i and "pass" in i for i in issues)

    def test_except_combined_errors_pass_detected(self, tmp_path: Path) -> None:
        """except (OSError, UnicodeDecodeError) followed by pass is flagged."""
        content = (
            "def check_file(filepath):\n"
            "    try:\n"
            "        content = filepath.read_text()\n"
            "    except (OSError, UnicodeDecodeError):\n"
            "        pass\n"
        )
        f = _write(tmp_path, "check-bad.py", content)
        issues = check_file(f)
        assert any("swallows" in i for i in issues)

    def test_except_oserror_return_true_detected(self, tmp_path: Path) -> None:
        """except OSError followed by return True is flagged."""
        content = (
            "def check_file(filepath):\n"
            "    try:\n"
            "        content = filepath.read_text()\n"
            "    except OSError:\n"
            "        return True\n"
        )
        f = _write(tmp_path, "check-bad.py", content)
        issues = check_file(f)
        assert any("returns True" in i for i in issues), (
            f"Expected fail-open warning, got: {issues}"
        )

    def test_except_oserror_return_false_passes(self, tmp_path: Path) -> None:
        """except OSError followed by return False passes (fail-closed)."""
        content = (
            "def check_file(filepath):\n"
            "    try:\n"
            "        content = filepath.read_text()\n"
            "    except OSError:\n"
            "        return False\n"
        )
        f = _write(tmp_path, "check-good.py", content)
        issues = check_file(f)
        assert not any("swallows" in i or "returns True" in i for i in issues)

    def test_except_oserror_return_none_passes(self, tmp_path: Path) -> None:
        """except OSError followed by return None passes (fail-closed fixer)."""
        content = (
            "def fix_file(filepath):\n"
            "    try:\n"
            "        content = filepath.read_text()\n"
            "    except OSError as exc:\n"
            '        print(f"{filepath}:0: cannot read file: {exc}", file=sys.stderr)\n'
            "        return None\n"
        )
        f = _write(tmp_path, "check-good.py", content)
        issues = check_file(f)
        assert not any("swallows" in i or "returns True" in i for i in issues)

    def test_except_with_print_and_return_false_passes(
        self, tmp_path: Path
    ) -> None:
        """except with print then return False passes (proper error handling)."""
        content = (
            "def check_file(filepath):\n"
            "    try:\n"
            "        content = filepath.read_text()\n"
            "    except (OSError, UnicodeDecodeError) as e:\n"
            '        print(f"{filepath}:0: cannot read file: {e}", file=sys.stderr)\n'
            "        return False\n"
        )
        f = _write(tmp_path, "check-good.py", content)
        issues = check_file(f)
        assert not any("swallows" in i or "returns True" in i for i in issues)

    def test_except_with_named_var_pass_detected(self, tmp_path: Path) -> None:
        """except OSError as exc followed by pass is flagged."""
        content = (
            "def check_file(filepath):\n"
            "    try:\n"
            "        content = filepath.read_text()\n"
            "    except OSError as exc:\n"
            "        pass\n"
        )
        f = _write(tmp_path, "check-bad.py", content)
        issues = check_file(f)
        assert any("swallows" in i for i in issues)

    def test_except_valueerror_not_detected(self, tmp_path: Path) -> None:
        """except ValueError with pass is not flagged (not an I/O error)."""
        content = (
            "def check_file(filepath):\n"
            "    try:\n"
            "        int(value)\n"
            "    except ValueError:\n"
            "        pass\n"
        )
        f = _write(tmp_path, "check-ok.py", content)
        issues = check_file(f)
        assert not any("swallows" in i or "returns True" in i for i in issues)

    def test_pass_with_comment_still_detected(self, tmp_path: Path) -> None:
        """pass with an explanatory comment is still flagged."""
        content = (
            "def check_file(filepath):\n"
            "    try:\n"
            "        content = filepath.read_text()\n"
            "    except (OSError, UnicodeDecodeError):\n"
            "        pass  # File read errors are non-fatal\n"
        )
        f = _write(tmp_path, "check-bad.py", content)
        issues = check_file(f)
        assert any("swallows" in i for i in issues)

    def test_except_ioerror_pass_detected(self, tmp_path: Path) -> None:
        """except IOError followed by pass is flagged."""
        content = (
            "def check_file(filepath):\n"
            "    try:\n"
            "        content = filepath.read_text()\n"
            "    except IOError:\n"
            "        pass\n"
        )
        f = _write(tmp_path, "check-bad.py", content)
        issues = check_file(f)
        assert any("swallows" in i for i in issues)

    def test_except_oserror_return_empty_list_detected(
        self, tmp_path: Path
    ) -> None:
        """except OSError followed by return [] is flagged (fail-open)."""
        content = (
            "def check_file(filepath):\n"
            "    try:\n"
            "        content = filepath.read_text()\n"
            "    except OSError:\n"
            "        return []\n"
        )
        f = _write(tmp_path, "check-bad.py", content)
        issues = check_file(f)
        assert any("empty value" in i for i in issues), (
            f"Expected empty-value warning, got: {issues}"
        )

    def test_except_oserror_return_empty_string_detected(
        self, tmp_path: Path
    ) -> None:
        """except OSError followed by return '' is flagged (fail-open)."""
        content = (
            "def check_file(filepath):\n"
            "    try:\n"
            "        content = filepath.read_text()\n"
            "    except OSError:\n"
            "        return ''\n"
        )
        f = _write(tmp_path, "check-bad.py", content)
        issues = check_file(f)
        assert any("empty value" in i for i in issues)

    def test_except_oserror_return_empty_dquote_string_detected(
        self, tmp_path: Path
    ) -> None:
        """except OSError followed by return "" is flagged (fail-open)."""
        content = (
            "def check_file(filepath):\n"
            "    try:\n"
            "        content = filepath.read_text()\n"
            "    except OSError:\n"
            '        return ""\n'
        )
        f = _write(tmp_path, "check-bad.py", content)
        issues = check_file(f)
        assert any("empty value" in i for i in issues)

    def test_except_oserror_return_nonempty_list_passes(
        self, tmp_path: Path
    ) -> None:
        """except OSError followed by return [error_msg] passes (fail-closed)."""
        content = (
            "def check_file(filepath):\n"
            "    try:\n"
            "        content = filepath.read_text()\n"
            "    except OSError as exc:\n"
            '        return [f"{filepath}:0: cannot read: {exc}"]\n'
        )
        f = _write(tmp_path, "check-good.py", content)
        issues = check_file(f)
        fail_open_issues = [
            i for i in issues
            if "swallows" in i or "returns True" in i or "empty value" in i
        ]
        assert not fail_open_issues


class TestRelativePaths:
    """Tests that check_file() uses relative paths when repo_root is provided."""

    def test_issues_use_relative_path_when_repo_root_provided(
        self, tmp_path: Path
    ) -> None:
        """Issue paths are relative to repo_root, not absolute."""
        f = _write(
            tmp_path,
            "check-bad.py",
            'def main():\n    print(f"  {err}")\n',
        )
        issues = check_file(f, repo_root=tmp_path)
        assert len(issues) == 1
        assert str(tmp_path) not in issues[0]
        assert "check-bad.py:2:" in issues[0]

    def test_read_error_uses_relative_path(self, tmp_path: Path) -> None:
        """Read error path prefix is relative to repo_root."""
        nonexistent = tmp_path / "check-missing.py"
        issues = check_file(nonexistent, repo_root=tmp_path)
        assert len(issues) == 1
        assert "check-missing.py:0:" in issues[0]
        # The path prefix (before ": cannot read") should be relative;
        # the exception message itself may still contain the absolute path.
        prefix = issues[0].split(": cannot read")[0]
        assert str(tmp_path) not in prefix


class TestRawPathInGlobScript:
    """Tests for Check 6: raw path variables in error output when file uses glob/rglob/iterdir."""

    def test_raw_filepath_with_glob_detected(self, tmp_path: Path) -> None:
        """f"{filepath}:0:" in a script using .glob( is flagged."""
        content = (
            "def check_all():\n"
            "    for p in path.glob('*.py'):\n"
            '        issues.append(f"{filepath}:0: error")\n'
        )
        f = _write(tmp_path, "check-bad.py", content)
        issues = check_file(f)
        assert any("may be absolute" in i for i in issues), (
            f"Expected raw-path warning, got: {issues}"
        )
        assert any("{filepath}" in i for i in issues)

    def test_raw_filepath_with_rglob_detected(self, tmp_path: Path) -> None:
        """f"{filepath}:0:" in a script using .rglob( is flagged."""
        content = (
            "def check_all():\n"
            "    for p in path.rglob('*.py'):\n"
            '        issues.append(f"{filepath}:0: error")\n'
        )
        f = _write(tmp_path, "check-bad.py", content)
        issues = check_file(f)
        assert any("may be absolute" in i for i in issues), (
            f"Expected raw-path warning, got: {issues}"
        )

    def test_raw_path_with_iterdir_detected(self, tmp_path: Path) -> None:
        """f"{filepath}:0:" in a script using .iterdir() is flagged."""
        content = (
            "def check_all():\n"
            "    for p in path.iterdir():\n"
            '        issues.append(f"{filepath}:0: error")\n'
        )
        f = _write(tmp_path, "check-bad.py", content)
        issues = check_file(f)
        assert any("may be absolute" in i for i in issues), (
            f"Expected raw-path warning, got: {issues}"
        )

    def test_display_path_with_glob_passes(self, tmp_path: Path) -> None:
        """f"{display_path}:0:" in a script using .glob( passes (safe variable)."""
        content = (
            "def check_all():\n"
            "    for p in path.glob('*.py'):\n"
            "        display_path = p.relative_to(root)\n"
            '        issues.append(f"{display_path}:0: error")\n'
        )
        f = _write(tmp_path, "check-good.py", content)
        issues = check_file(f)
        assert not any("may be absolute" in i for i in issues)

    def test_rel_var_with_glob_passes(self, tmp_path: Path) -> None:
        """f"{rel}:0:" in a script using .glob( passes (safe variable)."""
        content = (
            "def check_all():\n"
            "    for p in path.glob('*.py'):\n"
            "        rel = p.relative_to(root)\n"
            '        issues.append(f"{rel}:0: error")\n'
        )
        f = _write(tmp_path, "check-good.py", content)
        issues = check_file(f)
        assert not any("may be absolute" in i for i in issues)

    def test_raw_filepath_without_glob_passes(self, tmp_path: Path) -> None:
        """f"{filepath}:0:" without glob/rglob/iterdir passes (argv paths are relative)."""
        content = (
            "def check_file(filepath):\n"
            '    issues.append(f"{filepath}:0: error")\n'
        )
        f = _write(tmp_path, "check-good.py", content)
        issues = check_file(f)
        assert not any("may be absolute" in i for i in issues)

    def test_comment_glob_not_detected(self, tmp_path: Path) -> None:
        """A commented-out .glob( line does not trigger Check 6.

        The hook skips comment lines when scanning for .glob(/.rglob(/.iterdir(
        to avoid false positives on scripts that only mention glob in comments.
        """
        content = (
            "def check_file(filepath):\n"
            "    # for p in path.glob('*.py'):\n"
            '    issues.append(f"{filepath}:0: error")\n'
        )
        f = _write(tmp_path, "check-good.py", content)
        issues = check_file(f)
        assert not any("may be absolute" in i for i in issues)


class TestStderrErrorPrints:
    """Tests for Check 7: ERROR/WARNING prints going to stdout instead of stderr."""

    def test_error_print_without_stderr_flagged(self, tmp_path: Path) -> None:
        """print("ERROR: something") without file=sys.stderr is flagged."""
        f = _write(
            tmp_path,
            "check-bad.py",
            'def main():\n    print("ERROR: something went wrong")\n',
        )
        issues = check_file(f)
        assert len(issues) == 1
        assert "stdout" in issues[0]
        assert ":2:" in issues[0]

    def test_error_print_with_stderr_ok(self, tmp_path: Path) -> None:
        """print("ERROR: something", file=sys.stderr) is not flagged."""
        f = _write(
            tmp_path,
            "check-good.py",
            'def main():\n    print("ERROR: something went wrong", file=sys.stderr)\n',
        )
        issues = check_file(f)
        assert not any("stdout" in i for i in issues)

    def test_fstring_error_without_stderr_flagged(self, tmp_path: Path) -> None:
        """print(f"ERROR: {var}") without file=sys.stderr is flagged."""
        f = _write(
            tmp_path,
            "check-bad.py",
            'def main():\n    print(f"ERROR: {detail}")\n',
        )
        issues = check_file(f)
        assert any("stdout" in i for i in issues), (
            f"Expected stdout warning, got: {issues}"
        )

    def test_red_error_without_stderr_flagged(self, tmp_path: Path) -> None:
        """print(red("ERROR:") + " msg") without file=sys.stderr is flagged."""
        f = _write(
            tmp_path,
            "check-bad.py",
            'def main():\n    print(red("ERROR:") + " something failed")\n',
        )
        issues = check_file(f)
        assert any("stdout" in i for i in issues), (
            f"Expected stdout warning, got: {issues}"
        )

    def test_red_error_with_stderr_ok(self, tmp_path: Path) -> None:
        """print(red("ERROR:") + " msg", file=sys.stderr) is not flagged."""
        f = _write(
            tmp_path,
            "check-good.py",
            'def main():\n    print(red("ERROR:") + " something failed", file=sys.stderr)\n',
        )
        issues = check_file(f)
        assert not any("stdout" in i for i in issues)

    def test_warning_print_without_stderr_flagged(self, tmp_path: Path) -> None:
        """print("WARNING: something") without file=sys.stderr is flagged."""
        f = _write(
            tmp_path,
            "check-bad.py",
            'def main():\n    print("WARNING: check skipped")\n',
        )
        issues = check_file(f)
        assert any("stdout" in i for i in issues), (
            f"Expected stdout warning, got: {issues}"
        )

    def test_warning_print_with_stderr_ok(self, tmp_path: Path) -> None:
        """print("WARNING: something", file=sys.stderr) is not flagged."""
        f = _write(
            tmp_path,
            "check-good.py",
            'def main():\n    print("WARNING: check skipped", file=sys.stderr)\n',
        )
        issues = check_file(f)
        assert not any("stdout" in i for i in issues)

    def test_comment_line_not_flagged(self, tmp_path: Path) -> None:
        """# print("ERROR: something") is not flagged (comment line)."""
        f = _write(
            tmp_path,
            "check-good.py",
            '# print("ERROR: something went wrong")\n',
        )
        issues = check_file(f)
        assert issues == []

    def test_ok_summary_print_not_flagged(self, tmp_path: Path) -> None:
        """print("[OK] All good") is not flagged (not an error print)."""
        f = _write(
            tmp_path,
            "check-good.py",
            'def main():\n    print("[OK] All good")\n',
        )
        issues = check_file(f)
        assert not any("stdout" in i for i in issues)

    def test_single_quoted_error_without_stderr_flagged(
        self, tmp_path: Path
    ) -> None:
        """print('ERROR: something') with single quotes is flagged."""
        f = _write(
            tmp_path,
            "check-bad.py",
            "def main():\n    print('ERROR: something went wrong')\n",
        )
        issues = check_file(f)
        assert any("stdout" in i for i in issues), (
            f"Expected stdout warning, got: {issues}"
        )

    def test_single_quoted_fstring_error_without_stderr_flagged(
        self, tmp_path: Path
    ) -> None:
        """print(f'ERROR: {var}') with single-quoted f-string is flagged."""
        f = _write(
            tmp_path,
            "check-bad.py",
            "def main():\n    print(f'ERROR: {detail}')\n",
        )
        issues = check_file(f)
        assert any("stdout" in i for i in issues), (
            f"Expected stdout warning, got: {issues}"
        )

    def test_single_quoted_warning_without_stderr_flagged(
        self, tmp_path: Path
    ) -> None:
        """print('WARNING: something') with single quotes is flagged."""
        f = _write(
            tmp_path,
            "check-bad.py",
            "def main():\n    print('WARNING: check skipped')\n",
        )
        issues = check_file(f)
        assert any("stdout" in i for i in issues), (
            f"Expected stdout warning, got: {issues}"
        )

    def test_single_quoted_fstring_warning_without_stderr_flagged(
        self, tmp_path: Path
    ) -> None:
        """print(f'WARNING: {var}') with single-quoted f-string is flagged."""
        f = _write(
            tmp_path,
            "check-bad.py",
            "def main():\n    print(f'WARNING: {detail}')\n",
        )
        issues = check_file(f)
        assert any("stdout" in i for i in issues), (
            f"Expected stdout warning, got: {issues}"
        )

    def test_informational_print_not_flagged(self, tmp_path: Path) -> None:
        """print("Running cargo fmt...") is not flagged (informational)."""
        f = _write(
            tmp_path,
            "check-good.py",
            'def main():\n    print("Running cargo fmt...")\n',
        )
        issues = check_file(f)
        assert not any("stdout" in i for i in issues)


class TestMultiLineStderrPrints:
    """Tests for Check 7b: multi-line print() with ERROR:/WARNING: on continuation lines."""

    def test_multiline_warning_without_stderr_flagged(self, tmp_path: Path) -> None:
        """Multi-line print with WARNING: on a continuation line is flagged."""
        f = _write(
            tmp_path,
            "check-bad.py",
            (
                "def main():\n"
                "    print(\n"
                '        f"  WARNING: proof missing unwind"\n'
                "    )\n"
            ),
        )
        issues = check_file(f)
        assert any("multi-line" in i and "stdout" in i for i in issues), (
            f"Expected multi-line stdout warning, got: {issues}"
        )

    def test_multiline_error_without_stderr_flagged(self, tmp_path: Path) -> None:
        """Multi-line print with ERROR: on a continuation line is flagged."""
        f = _write(
            tmp_path,
            "check-bad.py",
            (
                "def main():\n"
                "    print(\n"
                '        red("ERROR:")\n'
                '        + f" something failed"\n'
                "    )\n"
            ),
        )
        issues = check_file(f)
        assert any("multi-line" in i and "stdout" in i for i in issues), (
            f"Expected multi-line stdout warning, got: {issues}"
        )

    def test_multiline_with_stderr_ok(self, tmp_path: Path) -> None:
        """Multi-line print with file=sys.stderr is not flagged."""
        f = _write(
            tmp_path,
            "check-good.py",
            (
                "def main():\n"
                "    print(\n"
                '        f"  WARNING: proof missing unwind",\n'
                "        file=sys.stderr,\n"
                "    )\n"
            ),
        )
        issues = check_file(f)
        assert not any("multi-line" in i for i in issues)

    def test_multiline_informational_not_flagged(self, tmp_path: Path) -> None:
        """Multi-line informational print is not flagged."""
        f = _write(
            tmp_path,
            "check-good.py",
            (
                "def main():\n"
                "    print(\n"
                '        f"Found {count} proofs"\n'
                "    )\n"
            ),
        )
        issues = check_file(f)
        assert not any("multi-line" in i for i in issues)


class TestSelfCompliance:
    """Verify that check-hook-output-format.py passes its own checks."""

    def test_self_passes_check6(self) -> None:
        """check-hook-output-format.py itself passes Check 6 (uses display_path, not filepath)."""
        hook_path = scripts_dir / "hooks" / "check-hook-output-format.py"
        issues = check_file(hook_path)
        check6_issues = [i for i in issues if "may be absolute" in i]
        assert not check6_issues, (
            f"check-hook-output-format.py fails its own Check 6: {check6_issues}"
        )


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
