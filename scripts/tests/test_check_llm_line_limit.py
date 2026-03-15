#!/usr/bin/env python3
"""
Unit tests for check-llm-line-limit.py hook.

Verifies that the line-limit checker correctly enforces the 300-line
maximum on .md files under the .llm/ directory.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

# Add scripts directory to path for imports
scripts_dir = Path(__file__).parent.parent
sys.path.insert(0, str(scripts_dir))

# Import with proper module name using importlib (hyphenated filename)
import importlib.util

spec = importlib.util.spec_from_file_location(
    "check_llm_line_limit", scripts_dir / "hooks" / "check-llm-line-limit.py"
)
check_llm_line_limit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(check_llm_line_limit)

import pytest

# Import functions and constants from the loaded module
find_llm_md_files = check_llm_line_limit.find_llm_md_files
check_file = check_llm_line_limit.check_file
MAX_LINES = check_llm_line_limit.MAX_LINES


def _make_md_file(directory: Path, name: str, num_lines: int) -> Path:
    """Helper to create a .md file with the given number of lines."""
    filepath = directory / name
    filepath.parent.mkdir(parents=True, exist_ok=True)
    if num_lines == 0:
        filepath.write_text("", encoding="utf-8")
    else:
        filepath.write_text(
            "\n".join(f"Line {i + 1}" for i in range(num_lines)) + "\n",
            encoding="utf-8",
        )
    return filepath


class TestFindLlmMdFiles:
    """Tests for find_llm_md_files function."""

    def test_no_llm_directory(self, tmp_path: Path) -> None:
        """Returns empty list when .llm/ directory does not exist."""
        result = find_llm_md_files(tmp_path)
        assert result == []

    def test_empty_llm_directory(self, tmp_path: Path) -> None:
        """Returns empty list when .llm/ exists but has no .md files."""
        (tmp_path / ".llm").mkdir()
        result = find_llm_md_files(tmp_path)
        assert result == []

    def test_finds_md_files(self, tmp_path: Path) -> None:
        """Finds .md files directly under .llm/."""
        llm_dir = tmp_path / ".llm"
        llm_dir.mkdir()
        _make_md_file(llm_dir, "context.md", 10)
        _make_md_file(llm_dir, "notes.md", 5)

        result = find_llm_md_files(tmp_path)
        assert len(result) == 2
        names = [f.name for f in result]
        assert "context.md" in names
        assert "notes.md" in names

    def test_ignores_non_md_files(self, tmp_path: Path) -> None:
        """Non-.md files in .llm/ are not returned."""
        llm_dir = tmp_path / ".llm"
        llm_dir.mkdir()
        (llm_dir / "config.toml").write_text("key = 'value'\n", encoding="utf-8")
        (llm_dir / "data.txt").write_text("hello\n", encoding="utf-8")
        (llm_dir / "script.py").write_text("print('hi')\n", encoding="utf-8")

        result = find_llm_md_files(tmp_path)
        assert result == []

    def test_finds_nested_subdirectory_files(self, tmp_path: Path) -> None:
        """Finds .md files in nested subdirectories under .llm/."""
        llm_dir = tmp_path / ".llm"
        _make_md_file(llm_dir / "skills", "kani.md", 20)
        _make_md_file(llm_dir / "deep" / "nested", "guide.md", 15)
        _make_md_file(llm_dir, "context.md", 10)

        result = find_llm_md_files(tmp_path)
        assert len(result) == 3
        names = sorted(f.name for f in result)
        assert names == ["context.md", "guide.md", "kani.md"]

    def test_results_are_sorted(self, tmp_path: Path) -> None:
        """Returned file list is sorted."""
        llm_dir = tmp_path / ".llm"
        _make_md_file(llm_dir, "zebra.md", 1)
        _make_md_file(llm_dir, "alpha.md", 1)
        _make_md_file(llm_dir, "middle.md", 1)

        result = find_llm_md_files(tmp_path)
        assert result == sorted(result)


class TestCheckFile:
    """Tests for check_file function."""

    @pytest.mark.parametrize(
        "num_lines, expected",
        [
            (0, True),
            (1, True),
            (150, True),
            (299, True),
            (300, True),
            (301, False),
            (500, False),
        ],
        ids=[
            "empty_file",
            "single_line",
            "midrange",
            "one_below_limit",
            "exactly_at_limit",
            "one_over_limit",
            "well_over_limit",
        ],
    )
    def test_line_count_boundary(
        self, tmp_path: Path, num_lines: int, expected: bool
    ) -> None:
        """Files at or below MAX_LINES pass; files above fail."""
        filepath = _make_md_file(tmp_path, "test.md", num_lines)
        assert check_file(filepath, tmp_path) is expected

    def test_fail_prints_message(self, tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
        """Failing file prints a FAIL message with line count details."""
        filepath = _make_md_file(tmp_path, "big.md", 305)
        result = check_file(filepath, tmp_path)

        assert result is False
        captured = capsys.readouterr()
        assert "big.md:0:" in captured.err
        assert "305" in captured.err

    def test_pass_prints_nothing(self, tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
        """Passing file produces no output."""
        filepath = _make_md_file(tmp_path, "ok.md", 100)
        result = check_file(filepath, tmp_path)

        assert result is True
        captured = capsys.readouterr()
        assert captured.out == ""

    def test_oserror_returns_false(self, tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
        """Unreadable file triggers OSError branch and returns False."""
        nonexistent = tmp_path / "does_not_exist.md"
        result = check_file(nonexistent, tmp_path)

        assert result is False
        captured = capsys.readouterr()
        assert "cannot read file" in captured.err

    def test_unicode_decode_error_returns_false(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Invalid UTF-8 file triggers UnicodeDecodeError and returns False."""
        filepath = tmp_path / "bad_encoding.md"
        filepath.write_bytes(b"\x80\x81\x82 invalid utf8")
        result = check_file(filepath, tmp_path)

        assert result is False
        captured = capsys.readouterr()
        assert "cannot read file" in captured.err


class TestMain:
    """Tests for the main() entry point and return codes.

    Uses _run_main_with_root to exercise the same logic as main() but with
    a configurable repo root (main() derives the root from __file__).
    """

    def test_returns_zero_when_all_pass(self, tmp_path: Path) -> None:
        """Returns 0 when all files are within the limit."""
        llm_dir = tmp_path / ".llm"
        _make_md_file(llm_dir, "a.md", 100)
        _make_md_file(llm_dir, "b.md", 300)

        assert _run_main_with_root(tmp_path) == 0

    def test_returns_one_when_any_fail(self, tmp_path: Path) -> None:
        """Returns 1 when any file exceeds the limit."""
        llm_dir = tmp_path / ".llm"
        _make_md_file(llm_dir, "ok.md", 100)
        _make_md_file(llm_dir, "bad.md", 301)

        assert _run_main_with_root(tmp_path) == 1

    def test_returns_zero_when_no_llm_dir(self, tmp_path: Path) -> None:
        """Returns 0 when there is no .llm/ directory."""
        assert _run_main_with_root(tmp_path) == 0

    def test_failure_prints_summary_message(self, tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
        """When a file fails, the summary message is printed."""
        llm_dir = tmp_path / ".llm"
        _make_md_file(llm_dir, "bad.md", 301)

        _run_main_with_root(tmp_path, print_summary=True)
        captured = capsys.readouterr()
        assert "must be 300 lines or fewer" in captured.err


def _run_main_with_root(repo_root: Path, *, print_summary: bool = False) -> int:
    """Exercise the same logic as main() with a configurable repo root.

    The real main() derives repo_root from __file__; this helper allows
    testing with tmp_path instead.
    """
    md_files = find_llm_md_files(repo_root)
    if not md_files:
        return 0
    all_ok = True
    for filepath in md_files:
        if not check_file(filepath, repo_root):
            all_ok = False
    if not all_ok:
        if print_summary:
            print(f"\nAll .md files under .llm/ must be {MAX_LINES} lines or fewer.", file=sys.stderr)
        return 1
    return 0


class TestIntegration:
    """End-to-end tests combining find and check."""

    def test_multiple_files_all_within_limit(self, tmp_path: Path) -> None:
        """All files within limit means all checks pass."""
        llm_dir = tmp_path / ".llm"
        _make_md_file(llm_dir, "a.md", 100)
        _make_md_file(llm_dir, "b.md", 200)
        _make_md_file(llm_dir, "c.md", 300)

        md_files = find_llm_md_files(tmp_path)
        assert len(md_files) == 3
        assert all(check_file(f, tmp_path) for f in md_files)

    def test_multiple_files_one_exceeds_limit(self, tmp_path: Path) -> None:
        """If any file exceeds the limit, that check fails."""
        llm_dir = tmp_path / ".llm"
        _make_md_file(llm_dir, "ok.md", 100)
        _make_md_file(llm_dir, "bad.md", 301)
        _make_md_file(llm_dir, "also_ok.md", 250)

        md_files = find_llm_md_files(tmp_path)
        results = [check_file(f, tmp_path) for f in md_files]
        assert not all(results)
        # Exactly one file should fail
        assert results.count(False) == 1

    def test_no_llm_directory_returns_zero(self, tmp_path: Path) -> None:
        """No .llm directory means no files to check and nothing fails."""
        md_files = find_llm_md_files(tmp_path)
        assert md_files == []

    def test_non_md_files_ignored_even_if_long(self, tmp_path: Path) -> None:
        """Non-.md files are never checked regardless of line count."""
        llm_dir = tmp_path / ".llm"
        llm_dir.mkdir(parents=True)
        # Create a very long .txt file - should not appear in results
        long_txt = llm_dir / "huge.txt"
        long_txt.write_text("\n".join(["x"] * 1000) + "\n", encoding="utf-8")
        # Create a valid .md file
        _make_md_file(llm_dir, "ok.md", 50)

        md_files = find_llm_md_files(tmp_path)
        assert len(md_files) == 1
        assert md_files[0].name == "ok.md"

    def test_nested_files_checked(self, tmp_path: Path) -> None:
        """Nested .md files under .llm/ subdirectories are checked."""
        llm_dir = tmp_path / ".llm"
        _make_md_file(llm_dir / "skills", "deep.md", 301)

        md_files = find_llm_md_files(tmp_path)
        assert len(md_files) == 1
        assert check_file(md_files[0], tmp_path) is False


class TestOutputFormat:
    """Tests that output follows {path}:{line_number}: {message} format."""

    def test_fail_output_starts_with_path_colon_line(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Failure message must start with path:line: (no leading whitespace)."""
        filepath = _make_md_file(tmp_path, "big.md", 305)
        check_file(filepath, tmp_path)
        captured = capsys.readouterr()
        for line in captured.err.splitlines():
            if line:
                assert re.match(r'^.+:\d+: ', line), f"Bad format: {line}"
                assert not line.startswith(" "), f"Leading whitespace: {line}"

    def test_oserror_output_includes_line_number(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Read error message must include :0: synthetic line number."""
        nonexistent = tmp_path / "does_not_exist.md"
        check_file(nonexistent, tmp_path)
        captured = capsys.readouterr()
        for line in captured.err.splitlines():
            if line:
                assert ":0:" in line, f"Missing :0: in read error: {line}"
                assert re.match(r'^.+:\d+: ', line), f"Bad format: {line}"

    def test_oserror_output_uses_relative_path(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """OSError output must use a relative path, not an absolute path."""
        nonexistent = tmp_path / "does_not_exist.md"
        check_file(nonexistent, tmp_path)
        captured = capsys.readouterr()
        err = captured.err.strip()
        assert not err.startswith(str(tmp_path)), (
            f"Error output should not start with absolute path: {err}"
        )
        assert err.startswith("does_not_exist.md"), (
            f"Error output should start with relative filename: {err}"
        )

    def test_over_limit_output_uses_relative_path(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Over-limit output must use a relative path, not an absolute path."""
        llm_dir = tmp_path / ".llm"
        filepath = _make_md_file(llm_dir, "big.md", 305)
        check_file(filepath, tmp_path)
        captured = capsys.readouterr()
        err = captured.err.strip()
        assert str(tmp_path) not in err, (
            f"Error output should not contain absolute path: {err}"
        )
        assert err.startswith(".llm/"), (
            f"Error output should start with relative .llm/ path: {err}"
        )

    def test_unicode_error_output_uses_relative_path(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """UnicodeDecodeError output must use a relative path, not absolute."""
        filepath = tmp_path / "bad_encoding.md"
        filepath.write_bytes(b"\x80\x81\x82 invalid utf8")
        check_file(filepath, tmp_path)
        captured = capsys.readouterr()
        err = captured.err.strip()
        assert not err.startswith(str(tmp_path)), (
            f"Error output should not start with absolute path: {err}"
        )
        assert err.startswith("bad_encoding.md"), (
            f"Error output should start with relative filename: {err}"
        )

    def test_main_output_no_leading_whitespace(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """main()-equivalent prints issue lines without leading whitespace."""
        llm_dir = tmp_path / ".llm"
        _make_md_file(llm_dir, "bad.md", 301)
        rc = _run_main_with_root(tmp_path, print_summary=True)
        assert rc == 1
        captured = capsys.readouterr()
        for line in captured.err.splitlines():
            if line:
                assert not line.startswith("  "), f"Leading indent: {line!r}"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
