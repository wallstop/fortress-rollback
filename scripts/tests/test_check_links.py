#!/usr/bin/env python3
"""
Unit tests for check-links.py validation functions.

These tests verify that the link checker correctly handles code spans
and other edge cases when extracting links from markdown files.
"""

from __future__ import annotations

import sys
from pathlib import Path

# Add scripts directory to path for imports
scripts_dir = Path(__file__).parent.parent
sys.path.insert(0, str(scripts_dir))

# Import with proper module name using importlib
import importlib.util

spec = importlib.util.spec_from_file_location(
    "check_links", scripts_dir / "check-links.py"
)
check_links = importlib.util.module_from_spec(spec)
spec.loader.exec_module(check_links)

import pytest

# Import functions from the loaded module
find_inline_code_ranges = check_links.find_inline_code_ranges
find_code_fence_ranges = check_links.find_code_fence_ranges


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


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
