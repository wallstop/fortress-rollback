#!/usr/bin/env python3
"""
Unit tests for check-track-caller-async.py hook.

Verifies that the #[track_caller] on async fn checker correctly detects
violations when the attribute and async fn appear on the same line, on
adjacent lines, or within the 5-line lookahead window, and that valid
usage does not produce false positives.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

# Import the hook module (hyphenated filename requires importlib)
scripts_dir = Path(__file__).parent.parent
spec = importlib.util.spec_from_file_location(
    "check_track_caller_async",
    scripts_dir / "hooks" / "check-track-caller-async.py",
)
check_track_caller_async = importlib.util.module_from_spec(spec)
spec.loader.exec_module(check_track_caller_async)

check_file = check_track_caller_async.check_file
main = check_track_caller_async.main


def _write(directory: Path, name: str, content: str) -> Path:
    """Helper to create a file with given content."""
    filepath = directory / name
    filepath.parent.mkdir(parents=True, exist_ok=True)
    filepath.write_text(content, encoding="utf-8")
    return filepath


class TestSameLineDetection:
    """Tests for #[track_caller] and async fn on the same line (bug fix)."""

    def test_track_caller_async_fn_same_line(self, tmp_path: Path) -> None:
        """#[track_caller] async fn on same line is detected."""
        f = _write(tmp_path, "lib.rs", "#[track_caller] async fn foo() {}\n")
        errors = check_file(f)
        assert len(errors) == 1
        assert "line 1" in errors[0]
        assert "#[track_caller] on async fn" in errors[0]

    def test_track_caller_pub_async_fn_same_line(self, tmp_path: Path) -> None:
        """#[track_caller] pub async fn on same line is detected."""
        f = _write(
            tmp_path, "lib.rs", "#[track_caller] pub async fn foo() {}\n"
        )
        errors = check_file(f)
        assert len(errors) == 1
        assert "line 1" in errors[0]

    def test_track_caller_pub_crate_async_fn_same_line(
        self, tmp_path: Path
    ) -> None:
        """#[track_caller] pub(crate) async fn on same line is detected."""
        f = _write(
            tmp_path,
            "lib.rs",
            "#[track_caller] pub(crate) async fn foo() {}\n",
        )
        errors = check_file(f)
        assert len(errors) == 1
        assert "line 1" in errors[0]

    def test_inline_attribute_before_track_caller_async(
        self, tmp_path: Path
    ) -> None:
        """#[inline] #[track_caller] async fn on same line is detected."""
        f = _write(
            tmp_path,
            "lib.rs",
            "#[inline] #[track_caller] async fn foo() {}\n",
        )
        errors = check_file(f)
        assert len(errors) == 1
        assert "line 1" in errors[0]

    def test_track_caller_unsafe_async_fn_same_line(
        self, tmp_path: Path
    ) -> None:
        """#[track_caller] unsafe async fn on same line is detected."""
        f = _write(
            tmp_path,
            "lib.rs",
            "#[track_caller] unsafe async fn foo() {}\n",
        )
        errors = check_file(f)
        assert len(errors) == 1
        assert "line 1" in errors[0]


class TestMultiLineDetection:
    """Tests for existing lookahead behavior across multiple lines."""

    def test_track_caller_next_line_async_fn(self, tmp_path: Path) -> None:
        """Standard 2-line case: #[track_caller] then async fn."""
        content = "#[track_caller]\nasync fn foo() {}\n"
        f = _write(tmp_path, "lib.rs", content)
        errors = check_file(f)
        assert len(errors) == 1
        assert "line 2" in errors[0]

    def test_track_caller_with_other_attrs_then_async(
        self, tmp_path: Path
    ) -> None:
        """#[track_caller] then #[inline] then async fn is detected."""
        content = "#[track_caller]\n#[inline]\nasync fn foo() {}\n"
        f = _write(tmp_path, "lib.rs", content)
        errors = check_file(f)
        assert len(errors) == 1
        assert "line 3" in errors[0]

    def test_track_caller_with_comment_between(self, tmp_path: Path) -> None:
        """Comment between #[track_caller] and async fn doesn't break detection."""
        content = "#[track_caller]\n// some comment\nasync fn foo() {}\n"
        f = _write(tmp_path, "lib.rs", content)
        errors = check_file(f)
        assert len(errors) == 1
        assert "line 3" in errors[0]

    def test_track_caller_with_block_comment_between(
        self, tmp_path: Path
    ) -> None:
        """Single-line block comment between doesn't break detection."""
        content = "#[track_caller]\n/* some comment */\nasync fn foo() {}\n"
        f = _write(tmp_path, "lib.rs", content)
        errors = check_file(f)
        assert len(errors) == 1
        assert "line 3" in errors[0]

    def test_track_caller_stops_at_non_attr_line(
        self, tmp_path: Path
    ) -> None:
        """A sync fn between stops lookahead; async fn after is not flagged."""
        content = (
            "#[track_caller]\n"
            "fn sync_foo() {}\n"
            "async fn bar() {}\n"
        )
        f = _write(tmp_path, "lib.rs", content)
        errors = check_file(f)
        assert len(errors) == 0

    def test_track_caller_five_line_lookahead_limit(
        self, tmp_path: Path
    ) -> None:
        """Async fn beyond the 5-line lookahead window is NOT detected."""
        # #[track_caller] on line 1, then 5 blank lines, async fn on line 7
        # Lookahead checks lines 2-6 (range i+1 to i+6), line 7 is outside
        content = (
            "#[track_caller]\n"
            "\n"
            "\n"
            "\n"
            "\n"
            "\n"
            "async fn foo() {}\n"
        )
        f = _write(tmp_path, "lib.rs", content)
        errors = check_file(f)
        assert len(errors) == 0

    def test_track_caller_unsafe_async_fn_multi_line(
        self, tmp_path: Path
    ) -> None:
        """#[track_caller] then unsafe async fn on next line is detected."""
        content = "#[track_caller]\nunsafe async fn foo() {}\n"
        f = _write(tmp_path, "lib.rs", content)
        errors = check_file(f)
        assert len(errors) == 1
        assert "line 2" in errors[0]

    def test_track_caller_exactly_five_lines_detected(
        self, tmp_path: Path
    ) -> None:
        """Async fn exactly at the 5-line lookahead boundary IS detected."""
        # #[track_caller] on line 1, then 4 blank lines, async fn on line 6
        # Lookahead checks lines 2-6 (range i+1 to i+6), line 6 is included
        content = (
            "#[track_caller]\n"
            "\n"
            "\n"
            "\n"
            "\n"
            "async fn foo() {}\n"
        )
        f = _write(tmp_path, "lib.rs", content)
        errors = check_file(f)
        assert len(errors) == 1
        assert "line 6" in errors[0]


class TestValidUsage:
    """Tests that valid code does not produce false positives."""

    def test_track_caller_on_sync_fn(self, tmp_path: Path) -> None:
        """#[track_caller] fn foo() is valid and should not be flagged."""
        f = _write(tmp_path, "lib.rs", "#[track_caller] fn foo() {}\n")
        errors = check_file(f)
        assert errors == []

    def test_track_caller_on_sync_pub_fn(self, tmp_path: Path) -> None:
        """#[track_caller] then pub fn on next line is valid."""
        content = "#[track_caller]\npub fn foo() {}\n"
        f = _write(tmp_path, "lib.rs", content)
        errors = check_file(f)
        assert errors == []

    def test_async_fn_without_track_caller(self, tmp_path: Path) -> None:
        """async fn without #[track_caller] is valid."""
        f = _write(tmp_path, "lib.rs", "async fn foo() {}\n")
        errors = check_file(f)
        assert errors == []

    def test_empty_file(self, tmp_path: Path) -> None:
        """Empty file produces no errors."""
        f = _write(tmp_path, "lib.rs", "")
        errors = check_file(f)
        assert errors == []

    def test_no_rust_code(self, tmp_path: Path) -> None:
        """Non-Rust content produces no errors."""
        f = _write(tmp_path, "lib.rs", "hello world\nfoo bar baz\n")
        errors = check_file(f)
        assert errors == []

    def test_track_caller_in_comment(self, tmp_path: Path) -> None:
        """// #[track_caller] before async fn is not flagged."""
        content = "// #[track_caller]\nasync fn foo() {}\n"
        f = _write(tmp_path, "lib.rs", content)
        errors = check_file(f)
        assert errors == []

    def test_track_caller_in_single_line_block_comment(
        self, tmp_path: Path
    ) -> None:
        """/* #[track_caller] */ before async fn is not flagged."""
        content = "/* #[track_caller] */\nasync fn foo() {}\n"
        f = _write(tmp_path, "lib.rs", content)
        errors = check_file(f)
        assert errors == []

    def test_indented_track_caller_on_sync_fn(self, tmp_path: Path) -> None:
        """Indented #[track_caller] on sync fn is valid (inside impl block)."""
        content = (
            "impl Foo {\n"
            "    #[track_caller]\n"
            "    fn bar() {}\n"
            "}\n"
        )
        f = _write(tmp_path, "lib.rs", content)
        errors = check_file(f)
        assert errors == []


class TestMultipleViolations:
    """Tests for files with multiple violations."""

    def test_multiple_violations_all_detected(self, tmp_path: Path) -> None:
        """Multiple violations in one file are all detected."""
        content = (
            "#[track_caller]\n"
            "async fn foo() {}\n"
            "\n"
            "#[track_caller]\n"
            "async fn bar() {}\n"
        )
        f = _write(tmp_path, "lib.rs", content)
        errors = check_file(f)
        assert len(errors) == 2

    def test_mix_valid_and_invalid(self, tmp_path: Path) -> None:
        """Only invalid usage is flagged when mixed with valid usage."""
        content = (
            "#[track_caller]\n"
            "fn good() {}\n"
            "\n"
            "#[track_caller]\n"
            "async fn bad() {}\n"
            "\n"
            "async fn also_good() {}\n"
        )
        f = _write(tmp_path, "lib.rs", content)
        errors = check_file(f)
        assert len(errors) == 1
        assert "line 5" in errors[0]

    def test_same_line_and_multi_line_violations(
        self, tmp_path: Path
    ) -> None:
        """Both same-line and multi-line violations are detected."""
        content = (
            "#[track_caller] async fn foo() {}\n"
            "\n"
            "#[track_caller]\n"
            "async fn bar() {}\n"
        )
        f = _write(tmp_path, "lib.rs", content)
        errors = check_file(f)
        assert len(errors) == 2
        # First error: same-line on line 1
        assert "line 1" in errors[0]
        # Second error: multi-line, #[track_caller] on line 3, async fn on line 4
        assert "line 4" in errors[1]


class TestEdgeCases:
    """Tests for edge cases and boundary conditions."""

    def test_nonexistent_file_returns_empty(self, tmp_path: Path) -> None:
        """Nonexistent file returns an empty error list."""
        errors = check_file(tmp_path / "nonexistent.rs")
        assert errors == []

    def test_track_caller_at_end_of_file(self, tmp_path: Path) -> None:
        """#[track_caller] on last line with no async fn after is fine."""
        f = _write(tmp_path, "lib.rs", "fn foo() {}\n#[track_caller]\n")
        errors = check_file(f)
        assert errors == []

    def test_doc_comment_between_track_caller_and_async(
        self, tmp_path: Path
    ) -> None:
        """/// doc comment between should continue scanning."""
        content = (
            "#[track_caller]\n"
            "/// This is a doc comment\n"
            "async fn foo() {}\n"
        )
        f = _write(tmp_path, "lib.rs", content)
        errors = check_file(f)
        assert len(errors) == 1
        assert "line 3" in errors[0]

    def test_indented_track_caller_on_async_fn_detected(
        self, tmp_path: Path
    ) -> None:
        """Indented #[track_caller] on async fn inside impl block is detected."""
        content = (
            "impl Foo {\n"
            "    #[track_caller]\n"
            "    async fn bar() {}\n"
            "}\n"
        )
        f = _write(tmp_path, "lib.rs", content)
        errors = check_file(f)
        assert len(errors) == 1
        assert "line 3" in errors[0]

    def test_track_caller_with_blank_lines(self, tmp_path: Path) -> None:
        """Blank lines between #[track_caller] and async fn are skipped."""
        content = (
            "#[track_caller]\n"
            "\n"
            "\n"
            "async fn foo() {}\n"
        )
        f = _write(tmp_path, "lib.rs", content)
        errors = check_file(f)
        assert len(errors) == 1
        assert "line 4" in errors[0]


class TestMain:
    """Tests for the main() entry point."""

    def test_main_no_args_returns_zero(self, monkeypatch: pytest.MonkeyPatch) -> None:
        """No args means no errors, return 0."""
        monkeypatch.setattr(sys, "argv", ["check-track-caller-async.py"])
        assert main() == 0

    def test_main_clean_file_returns_zero(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        """Clean .rs file returns 0."""
        f = _write(tmp_path, "clean.rs", "#[track_caller]\nfn foo() {}\n")
        monkeypatch.setattr(
            sys, "argv", ["check-track-caller-async.py", str(f)]
        )
        assert main() == 0

    def test_main_violation_returns_one(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        """File with violation returns 1."""
        f = _write(
            tmp_path, "bad.rs", "#[track_caller]\nasync fn foo() {}\n"
        )
        monkeypatch.setattr(
            sys, "argv", ["check-track-caller-async.py", str(f)]
        )
        assert main() == 1

    def test_main_prints_error_details(
        self,
        tmp_path: Path,
        monkeypatch: pytest.MonkeyPatch,
        capsys: pytest.CaptureFixture[str],
    ) -> None:
        """Verify the output format includes error details."""
        f = _write(
            tmp_path, "bad.rs", "#[track_caller]\nasync fn foo() {}\n"
        )
        monkeypatch.setattr(
            sys, "argv", ["check-track-caller-async.py", str(f)]
        )
        main()
        captured = capsys.readouterr()
        assert "ERROR: #[track_caller] cannot be used on async fn:" in captured.out
        assert "#[track_caller] on async fn" in captured.out
        assert "not supported by Rust" in captured.out

    def test_main_skips_non_rs_files(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        """Non-.rs files are ignored by main() even if they contain violations."""
        f = _write(
            tmp_path,
            "notes.txt",
            "#[track_caller]\nasync fn foo() {}\n",
        )
        monkeypatch.setattr(
            sys, "argv", ["check-track-caller-async.py", str(f)]
        )
        assert main() == 0

    def test_main_multiple_files(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        """main() checks all .rs files passed as arguments."""
        good = _write(
            tmp_path, "good.rs", "#[track_caller]\nfn foo() {}\n"
        )
        bad = _write(
            tmp_path, "bad.rs", "#[track_caller]\nasync fn bar() {}\n"
        )
        monkeypatch.setattr(
            sys,
            "argv",
            ["check-track-caller-async.py", str(good), str(bad)],
        )
        assert main() == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
