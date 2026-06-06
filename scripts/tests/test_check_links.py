#!/usr/bin/env python3
"""
Unit tests for check-links.py validation functions.

These tests verify that the link checker correctly handles code spans
and other edge cases when extracting links from markdown files.
"""

from __future__ import annotations

import sys
from pathlib import Path

# Add scripts/docs directory to path for imports
scripts_dir = Path(__file__).parent.parent
sys.path.insert(0, str(scripts_dir / "docs"))

# Import with proper module name using importlib
import importlib.util

spec = importlib.util.spec_from_file_location(
    "check_links", scripts_dir / "docs" / "check-links.py"
)
check_links = importlib.util.module_from_spec(spec)
spec.loader.exec_module(check_links)

import pytest

# Import functions from the loaded module
find_inline_code_ranges = check_links.find_inline_code_ranges
find_code_fence_ranges = check_links.find_code_fence_ranges
check_rust_doc_link = check_links.check_rust_doc_link
check_rust_doc_file = check_links.check_rust_doc_file
clean_link_target = check_links.clean_link_target
extract_markdown_anchors = check_links.extract_markdown_anchors
extract_rust_doc_blocks = check_links.extract_rust_doc_blocks
extract_rust_doc_markdown = check_links.extract_rust_doc_markdown
is_rustdoc_item_fragment = check_links.is_rustdoc_item_fragment
should_skip_markdown_file = check_links.should_skip_markdown_file


class TestSkippedMarkdownPaths:
    """Tests for repository-wide link-check exclusions."""

    @pytest.mark.parametrize(
        "path",
        [
            "progress/session-139-property-test-conversion.md",
            ".claude/worktrees/agent/wiki/Home.md",
            "target/doc/readme.md",
            "node_modules/package/README.md",
            "PLAN.md",
            "pr-description.md",
        ],
    )
    def test_ignored_markdown_is_skipped(self, path: str) -> None:
        """Ignored or generated markdown must not block local hooks."""
        assert should_skip_markdown_file(Path(path))

    def test_docs_markdown_is_checked(self) -> None:
        """Project documentation remains in scope."""
        assert not should_skip_markdown_file(Path("docs/contributing.md"))

    @pytest.mark.parametrize(
        "path",
        [
            "docs/PLAN.md",
            "wiki/pr-description.md",
        ],
    )
    def test_root_only_ignored_names_are_checked_when_nested(self, path: str) -> None:
        """Root-only generated filenames do not exclude real docs with same names."""
        assert not should_skip_markdown_file(Path(path))


class TestFindInlineCodeRanges:
    """Tests for find_inline_code_ranges function."""

    def test_single_backtick_code_span(self) -> None:
        """Single backtick inline code is detected."""
        content = "Here is `code` text"
        ranges = find_inline_code_ranges(content)
        assert len(ranges) == 1
        assert content[ranges[0][0] : ranges[0][1]] == "`code`"

    def test_double_backtick_code_span(self) -> None:
        """Double backtick inline code is detected.

        Double backtick delimiters allow embedding single backticks in code.
        The span ends at the first matching double backtick delimiter.
        """
        content = "Here is ``code with `backtick`` text"
        ranges = find_inline_code_ranges(content)
        assert len(ranges) == 1
        # Double backtick spans end at the first matching double backticks
        assert content[ranges[0][0] : ranges[0][1]] == "``code with `backtick``"

    def test_multiple_code_spans(self) -> None:
        """Multiple inline code spans are all detected."""
        content = "First `code1` and second `code2` here"
        ranges = find_inline_code_ranges(content)
        assert len(ranges) == 2
        assert content[ranges[0][0] : ranges[0][1]] == "`code1`"
        assert content[ranges[1][0] : ranges[1][1]] == "`code2`"

    def test_unclosed_code_span_not_detected(self) -> None:
        """Unclosed inline code span is not treated as code.

        When opening backticks have no matching closing backticks,
        they are treated as literal text, not as a code span.
        This prevents an unclosed backtick from masking the rest
        of the document.
        """
        content = "Here is `unclosed text"
        ranges = find_inline_code_ranges(content)
        assert len(ranges) == 0

    def test_unclosed_code_span_does_not_mask_valid_links(self) -> None:
        """Valid content after unclosed backticks is still accessible.

        The key behavior: when we have `unclosed followed by [link](url),
        the link should NOT be considered inside a code span.
        """
        content = "Start `unclosed then [link](url) end"
        ranges = find_inline_code_ranges(content)
        # No valid code spans since the first backtick isn't closed
        assert len(ranges) == 0

    def test_code_span_matching_first_available_closing(self) -> None:
        """Code span matches first available closing backtick.

        If we have: `text here `more text`
        The first backtick pairs with the second backtick, creating
        a code span of `text here `. The third backtick then has no
        closing backtick so is treated as literal text.

        This matches standard markdown parser behavior.
        """
        content = "Start `text here `more text` end"
        ranges = find_inline_code_ranges(content)
        # First backtick pairs with second backtick
        assert len(ranges) == 1
        assert content[ranges[0][0] : ranges[0][1]] == "`text here `"

    def test_empty_content(self) -> None:
        """Empty content returns no ranges."""
        ranges = find_inline_code_ranges("")
        assert len(ranges) == 0

    def test_no_backticks(self) -> None:
        """Content without backticks returns no ranges."""
        ranges = find_inline_code_ranges("Just plain text here")
        assert len(ranges) == 0

    def test_fenced_code_block_not_detected_as_inline(self) -> None:
        """Fenced code blocks (3+ backticks at line start) are skipped.

        Fenced code blocks are handled separately by find_code_fence_ranges.
        """
        content = "Before\n```\ncode\n```\nAfter"
        ranges = find_inline_code_ranges(content)
        # Fenced code blocks are not inline code
        assert len(ranges) == 0

    def test_fenced_code_block_with_language(self) -> None:
        """Fenced code blocks with language specifier are skipped."""
        content = "Text\n```python\ncode\n```\nMore"
        ranges = find_inline_code_ranges(content)
        assert len(ranges) == 0

    def test_triple_backticks_inline_not_at_line_start(self) -> None:
        """Triple backticks NOT at line start are treated as inline code.

        Only triple backticks at the start of a line (with optional whitespace)
        are treated as fenced code blocks. Triple backticks in the middle of
        a line are inline code spans.
        """
        content = "text```code```more"
        ranges = find_inline_code_ranges(content)
        assert len(ranges) == 1
        assert content[ranges[0][0] : ranges[0][1]] == "```code```"

    def test_exact_backtick_count_not_prefix(self) -> None:
        """Closing delimiter must be exactly N backticks, not part of longer sequence.

        When we have 2 opening backticks, we need exactly 2 closing backticks.
        A 3-backtick sequence should not match as a closing delimiter for 2 backticks.
        """
        content = "``code ``` more text``"
        ranges = find_inline_code_ranges(content)
        assert len(ranges) == 1
        assert content[ranges[0][0] : ranges[0][1]] == "``code ``` more text``"

    def test_single_backtick_not_matched_by_double(self) -> None:
        """Single backtick opening should not match double backtick closing."""
        content = "`code `` more`"
        ranges = find_inline_code_ranges(content)
        assert len(ranges) == 1
        assert content[ranges[0][0] : ranges[0][1]] == "`code `` more`"

    def test_double_backtick_not_matched_by_triple(self) -> None:
        """Double backtick opening should not match triple backtick closing."""
        content = "start ``code ``` more text`` end"
        ranges = find_inline_code_ranges(content)
        assert len(ranges) == 1
        assert content[ranges[0][0] : ranges[0][1]] == "``code ``` more text``"

    def test_backtick_boundary_check(self) -> None:
        """Closing backticks should not be part of longer sequences."""
        content = "``code```more``text"
        ranges = find_inline_code_ranges(content)
        assert len(ranges) == 1
        assert content[ranges[0][0] : ranges[0][1]] == "``code```more``"

    def test_no_closing_due_to_length_mismatch(self) -> None:
        """Opening delimiter with only longer sequences available should not close."""
        content = "``code ```"
        ranges = find_inline_code_ranges(content)
        assert len(ranges) == 0
        content = "text```code```more"
        ranges = find_inline_code_ranges(content)
        assert len(ranges) == 1
        assert content[ranges[0][0] : ranges[0][1]] == "```code```"

    def test_quadruple_backticks_at_line_start(self) -> None:
        """Quadruple backticks at line start are treated as fenced code block."""
        content = "````\ncode\n````"
        ranges = find_inline_code_ranges(content)
        assert len(ranges) == 0

    def test_quadruple_backticks_inline(self) -> None:
        """Quadruple backticks inline are treated as inline code."""
        content = "text````code````more"
        ranges = find_inline_code_ranges(content)
        assert len(ranges) == 1
        assert content[ranges[0][0] : ranges[0][1]] == "````code````"

    def test_mixed_backtick_counts(self) -> None:
        """Different backtick counts find their matching closers.

        Double backticks can contain single backticks without matching.
        """
        content = "``code with `embedded` backticks``"
        ranges = find_inline_code_ranges(content)
        assert len(ranges) == 1
        assert content[ranges[0][0] : ranges[0][1]] == "``code with `embedded` backticks``"

    def test_empty_double_backtick_span(self) -> None:
        """Empty double backtick span `` `` is detected."""
        content = "text `` more"
        ranges = find_inline_code_ranges(content)
        # `` is matched as the opener, needs another `` as closer
        # In this case, there's no closer, so no range
        assert len(ranges) == 0

    def test_consecutive_backticks_pairing(self) -> None:
        """Consecutive backticks pair correctly.

        When we have ```` (4 backticks), they should pair as 4-backtick
        delimiter, not as two 2-backtick delimiters.
        """
        content = "````code````"  # At start of content = line start
        ranges = find_inline_code_ranges(content)
        # At line start with 4+ backticks = treated as fenced code block
        assert len(ranges) == 0


class TestRustDocLinks:
    """Tests for Rust doc-comment link validation."""

    def test_extract_rust_doc_markdown(self) -> None:
        """Rust doc comments are converted into Markdown before anchor checks."""
        content = """//! Module docs
//! # Module Heading
/// Item docs
/// ## Item Heading
fn f() {}
"""
        markdown = extract_rust_doc_markdown(content)

        assert "Module Heading" in markdown
        assert "Item Heading" in markdown
        assert "fn f" not in markdown

    def test_extract_rust_doc_blocks_preserves_module_and_item_scope(self) -> None:
        """Module docs and item docs are separate Rustdoc pages for fragments."""
        content = """//! Module docs
//! # Module Heading

/// Item docs
/// # Item Heading
fn f() {}
"""
        blocks = extract_rust_doc_blocks(content)

        assert [(block.is_module, block.markdown.splitlines()[0]) for block in blocks] == [
            (True, "Module docs"),
            (False, "Item docs"),
        ]

    @pytest.mark.parametrize(
        "anchor",
        [
            "structfield.defer_input_processing",
            "method.advance_frame",
            "associatedtype.Item",
            "associatedconstant.CAP",
            "variant.Running",
        ],
    )
    def test_rustdoc_item_fragment_is_rejected_as_url_fragment(self, anchor: str) -> None:
        """Rustdoc item fragments are recognized so URL-style links can fail closed."""
        assert is_rustdoc_item_fragment(anchor)

    @pytest.mark.parametrize(
        "anchor",
        [
            "missing",
            "structfield.",
            "structfield.not-a-rust-ident",
            "unknown_kind.Item",
        ],
    )
    def test_non_item_rustdoc_fragment_is_not_delegated(self, anchor: str) -> None:
        """Only known rustdoc item-fragment forms bypass local heading checks."""
        assert not is_rustdoc_item_fragment(anchor)

    @pytest.mark.parametrize(
        ("target", "source"),
        [
            (
                "#bounded-deserialization-allocation-only",
                "//! # Bounded deserialization (allocation only)\n",
            ),
            (
                "self#bounded-deserialization-allocation-only",
                "//! # Bounded deserialization (allocation only)\n",
            ),
        ],
    )
    def test_rust_doc_local_anchor_links_pass(
        self, tmp_path: Path, target: str, source: str
    ) -> None:
        """Local rustdoc anchors are validated instead of treated as files."""
        rust_file = tmp_path / "lib.rs"
        rust_file.write_text(source, encoding="utf-8")

        is_valid, error_msg = check_rust_doc_link(rust_file, target, tmp_path)

        assert is_valid, error_msg

    @pytest.mark.parametrize(
        "target",
        [
            "#missing",
            "self#missing",
        ],
    )
    def test_rust_doc_missing_local_anchor_fails(
        self, tmp_path: Path, target: str
    ) -> None:
        """Missing local rustdoc anchors fail closed with diagnostics."""
        rust_file = tmp_path / "lib.rs"
        rust_file.write_text(
            "//! # Existing\nstruct S { present: bool }\n", encoding="utf-8"
        )

        is_valid, error_msg = check_rust_doc_link(rust_file, target, tmp_path)

        assert not is_valid
        assert "Rustdoc" in error_msg
        assert "anchor" in error_msg

    @pytest.mark.parametrize(
        "target",
        [
            "#structfield.defer_input_processing",
            "Self#structfield.defer_input_processing",
            "P2PSession#method.current_state",
        ],
    )
    def test_rust_doc_item_fragment_urls_fail_closed(
        self, tmp_path: Path, target: str
    ) -> None:
        """Item-fragment URLs must be converted to rustdoc intra-doc paths."""
        rust_file = tmp_path / "lib.rs"
        rust_file.write_text("struct S {}\n", encoding="utf-8")

        is_valid, error_msg = check_rust_doc_link(rust_file, target, tmp_path)

        assert not is_valid
        assert "intra-doc path link" in error_msg

    @pytest.mark.parametrize(
        ("content", "expected_errors"),
        [
            (
                """//! # Module Heading
/// See [module docs](self#module-heading).
fn f() {}
""",
                0,
            ),
            (
                """//! # Module Heading
/// See [wrong page](#module-heading).
fn f() {}
""",
                1,
            ),
            (
                """/// # Item Heading
/// See [same item](#item-heading).
fn f() {}
""",
                0,
            ),
        ],
    )
    def test_rust_doc_heading_fragments_are_page_scoped(
        self, tmp_path: Path, content: str, expected_errors: int
    ) -> None:
        """Bare item fragments are item-local; self# targets module docs."""
        rust_file = tmp_path / "lib.rs"
        rust_file.write_text(content, encoding="utf-8")

        result = check_rust_doc_file(rust_file, tmp_path)

        assert result.errors == expected_errors

    def test_rust_doc_links_inside_code_are_skipped(self, tmp_path: Path) -> None:
        """Rustdoc examples and inline code do not create false link failures."""
        rust_file = tmp_path / "lib.rs"
        rust_file.write_text(
            """//! # Existing
//! Inline code `[bad](missing.md)` is ignored.
//! ```
//! [also ignored](missing.md)
//! ```
//! A real [local link](#existing) is still checked.
""",
            encoding="utf-8",
        )

        result = check_rust_doc_file(rust_file, tmp_path)

        assert result.errors == 0
        assert result.checked == 1

    @pytest.mark.parametrize(
        ("content", "expected_errors"),
        [
            ('/// <img src="target.md" alt="ok">\n', 0),
            ('/// <a href="missing.md">bad</a>\n', 1),
            ("/// [ref]: target.md\n", 0),
            ("/// [ref]: missing.md\n", 1),
            ('/// [inline](target.md "title")\n', 0),
        ],
    )
    def test_rust_doc_markdown_link_forms_are_checked(
        self, tmp_path: Path, content: str, expected_errors: int
    ) -> None:
        """Rustdoc uses the same local link extraction as Markdown files."""
        rust_file = tmp_path / "lib.rs"
        rust_file.write_text(content, encoding="utf-8")
        (tmp_path / "target.md").write_text("# Target\n", encoding="utf-8")

        result = check_rust_doc_file(rust_file, tmp_path)

        assert result.errors == expected_errors


class TestMarkdownLinkExtraction:
    """Tests for Markdown link target extraction beyond inline links."""

    @pytest.mark.parametrize(
        ("raw_target", "expected"),
        [
            ("target.md", "target.md"),
            ("target.md \"title\"", "target.md"),
            ("<target.md> \"title\"", "target.md"),
            (" https://example.com/a b ", "https://example.com/a"),
        ],
    )
    def test_clean_link_target(self, raw_target: str, expected: str) -> None:
        """Optional Markdown titles are removed before local path checks."""
        assert clean_link_target(raw_target) == expected

    @pytest.mark.parametrize(
        ("content", "expected_errors"),
        [
            ("[ref]: target.md\n", 0),
            ("[ref]: <target.md> \"title\"\n", 0),
            ("[ref]: missing.md\n", 1),
            ('<img src="target.md" alt="ok">\n', 0),
            ('<a href="missing.md">bad</a>\n', 1),
            ("[inline](target.md \"title\")\n", 0),
        ],
    )
    def test_markdown_link_forms_are_checked(
        self, tmp_path: Path, content: str, expected_errors: int
    ) -> None:
        """Reference definitions, HTML attributes, and titled links are validated."""
        source = tmp_path / "source.md"
        source.write_text(content, encoding="utf-8")
        (tmp_path / "target.md").write_text("# Target\n", encoding="utf-8")

        result = check_markdown_file(source, tmp_path)

        assert result.errors == expected_errors

    def test_repo_root_absolute_link_is_supported(self, tmp_path: Path) -> None:
        """Root-absolute local links resolve against the repository root."""
        docs_dir = tmp_path / "docs"
        docs_dir.mkdir()
        source = docs_dir / "source.md"
        source.write_text("[root](/target.md)\n", encoding="utf-8")
        (tmp_path / "target.md").write_text("# Target\n", encoding="utf-8")

        result = check_markdown_file(source, tmp_path)

        assert result.errors == 0

    def test_markdown_anchors_ignore_code_fence_headings(self, tmp_path: Path) -> None:
        """Anchors inside fenced code are not treated as rendered headings."""
        content = """# Real
```markdown
# Hidden
{#hidden-explicit}
```
"""
        anchors = extract_markdown_anchors(content)

        assert "real" in anchors
        assert "hidden" not in anchors
        assert "hidden-explicit" not in anchors

        source = tmp_path / "source.md"
        source.write_text("[bad](#hidden)\n" + content, encoding="utf-8")
        result = check_markdown_file(source, tmp_path)

        assert result.errors == 1

    def test_duplicate_markdown_headings_get_github_suffixes(self, tmp_path: Path) -> None:
        """Duplicate headings expose GitHub-style -1/-2 anchor suffixes."""
        content = "# Added\n# Added\n# Added\n"
        anchors = extract_markdown_anchors(content)

        assert {"added", "added-1", "added-2"} <= anchors

        source = tmp_path / "source.md"
        source.write_text("[second](#added-1)\n" + content, encoding="utf-8")
        result = check_markdown_file(source, tmp_path)

        assert result.errors == 0


class TestFindCodeFenceRanges:
    """Tests for find_code_fence_ranges function."""

    def test_basic_fenced_code_block(self) -> None:
        """Basic fenced code block is detected."""
        content = "Before\n```\ncode\n```\nAfter"
        ranges = find_code_fence_ranges(content)
        assert len(ranges) == 1

    def test_fenced_with_language(self) -> None:
        """Fenced code block with language specifier."""
        content = "Text\n```python\ndef foo():\n    pass\n```\nMore"
        ranges = find_code_fence_ranges(content)
        assert len(ranges) == 1

    def test_tilde_fence(self) -> None:
        """Tilde fenced code block is detected."""
        content = "Before\n~~~\ncode\n~~~\nAfter"
        ranges = find_code_fence_ranges(content)
        assert len(ranges) == 1

    def test_multiple_fenced_blocks(self) -> None:
        """Multiple fenced code blocks are all detected."""
        content = "```\nblock1\n```\n\n```\nblock2\n```"
        ranges = find_code_fence_ranges(content)
        assert len(ranges) == 2

    def test_unclosed_fence_extends_to_end(self) -> None:
        """Unclosed fenced code block extends to end of content."""
        content = "Text\n```\nunclosed code\nmore code"
        ranges = find_code_fence_ranges(content)
        assert len(ranges) == 1
        # Should extend to end of content
        assert ranges[0][1] == len(content)


check_markdown_link = check_links.check_markdown_link
check_markdown_file = check_links.check_markdown_file


class TestFailClosedAnchorValidation:
    """Tests that file read errors during anchor validation fail closed."""

    def test_same_file_anchor_unreadable_returns_error(
        self, tmp_path: Path
    ) -> None:
        """Anchor validation on an unreadable file returns an error, not True."""
        f = tmp_path / "test.md"
        f.write_text("# Heading\n[link](#anchor)\n", encoding="utf-8")
        f.chmod(0o000)
        try:
            is_valid, error_msg = check_markdown_link(
                f, "#anchor", tmp_path
            )
            assert not is_valid
            assert "Cannot read file" in error_msg
        finally:
            f.chmod(0o644)

    def test_same_file_anchor_binary_returns_error(
        self, tmp_path: Path
    ) -> None:
        """Anchor validation on a binary file returns an error, not True."""
        f = tmp_path / "test.md"
        f.write_bytes(b"\xff\xfe\x00\x01")
        is_valid, error_msg = check_markdown_link(
            f, "#heading", tmp_path
        )
        assert not is_valid
        assert "Cannot read file" in error_msg

    def test_cross_file_anchor_unreadable_returns_error(
        self, tmp_path: Path
    ) -> None:
        """Cross-file anchor validation on unreadable target returns error."""
        source = tmp_path / "source.md"
        source.write_text("[link](target.md#heading)\n", encoding="utf-8")
        target = tmp_path / "target.md"
        target.write_text("# Heading\n", encoding="utf-8")
        target.chmod(0o000)
        try:
            is_valid, error_msg = check_markdown_link(
                source, "target.md#heading", tmp_path
            )
            assert not is_valid
            assert "Cannot read" in error_msg
            assert "anchor" in error_msg
        finally:
            target.chmod(0o644)

    def test_cross_file_anchor_binary_target_returns_error(
        self, tmp_path: Path
    ) -> None:
        """Cross-file anchor validation on binary target returns error."""
        source = tmp_path / "source.md"
        source.write_text("[link](target.md#heading)\n", encoding="utf-8")
        target = tmp_path / "target.md"
        target.write_bytes(b"\xff\xfe\x00\x01")
        is_valid, error_msg = check_markdown_link(
            source, "target.md#heading", tmp_path
        )
        assert not is_valid
        assert "Cannot read" in error_msg

    def test_check_markdown_file_counts_anchor_read_errors(
        self, tmp_path: Path
    ) -> None:
        """check_markdown_file counts anchor read errors as errors."""
        # Create a source file that links to an unreadable target
        target = tmp_path / "target.md"
        target.write_text("# Heading\n", encoding="utf-8")
        target.chmod(0o000)
        try:
            source = tmp_path / "source.md"
            source.write_text(
                "[link](target.md#heading)\n", encoding="utf-8"
            )
            result = check_markdown_file(source, tmp_path)
            assert result.errors > 0
        finally:
            target.chmod(0o644)

    @pytest.mark.parametrize(
        ("anchor", "expected_errors"),
        [
            ("heading", 0),
            ("missing", 1),
        ],
    )
    def test_cross_file_markdown_anchor_is_validated(
        self, tmp_path: Path, anchor: str, expected_errors: int
    ) -> None:
        """Cross-file markdown fragments fail closed instead of becoming advisory."""
        source = tmp_path / "source.md"
        source.write_text(f"[link](target.md#{anchor})\n", encoding="utf-8")
        target = tmp_path / "target.md"
        target.write_text("# Heading\n", encoding="utf-8")

        result = check_markdown_file(source, tmp_path)

        assert result.errors == expected_errors


class TestRelativePaths:
    """Tests that _rel() converts absolute paths to relative."""

    def test_rel_converts_absolute_to_relative(self, tmp_path: Path) -> None:
        """Absolute path under root is converted to relative."""
        sub = tmp_path / "docs"
        sub.mkdir()
        f = sub / "guide.md"
        f.write_text("# Guide\n", encoding="utf-8")
        result = check_links._rel(f, tmp_path)
        assert result == Path("docs") / "guide.md"

    def test_rel_fallback_when_outside_root(self, tmp_path: Path) -> None:
        """Path outside root falls back to original path."""
        other = tmp_path / "other"
        other.mkdir()
        root = tmp_path / "root"
        root.mkdir()
        target = other / "file.md"
        target.write_text("# File\n", encoding="utf-8")
        result = check_links._rel(target, root)
        assert result == target


class TestErrorsGoToStderr:
    """Tests that ERROR messages go to stderr, not stdout."""

    def test_file_read_error_prints_to_stderr(
        self, tmp_path: Path, capsys
    ) -> None:
        """File read errors produce ERROR on stderr, not stdout."""
        f = tmp_path / "unreadable.md"
        f.write_text("# Heading\n", encoding="utf-8")
        f.chmod(0o000)
        try:
            check_markdown_file(f, tmp_path)
            captured = capsys.readouterr()
            assert "ERROR:" in captured.err
            assert "ERROR:" not in captured.out
        finally:
            f.chmod(0o644)

    def test_broken_link_error_prints_to_stderr(
        self, tmp_path: Path, capsys
    ) -> None:
        """Broken link errors produce ERROR on stderr, not stdout."""
        f = tmp_path / "broken.md"
        f.write_text("[text](nonexistent.md)\n", encoding="utf-8")
        check_markdown_file(f, tmp_path)
        captured = capsys.readouterr()
        assert "ERROR:" in captured.err
        assert "ERROR:" not in captured.out


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
