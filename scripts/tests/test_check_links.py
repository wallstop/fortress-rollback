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
        """Double backtick inline code is detected."""
        content = "Here is ``code with `backtick``` text"
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
