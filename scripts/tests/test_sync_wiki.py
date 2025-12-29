#!/usr/bin/env python3
"""
Unit tests for sync-wiki.py transformations.

These tests verify that the wiki sync script correctly converts MkDocs
Material-specific syntax to GitHub Wiki-compatible markdown.
"""

from __future__ import annotations

import sys
from pathlib import Path

# Add scripts directory to path for imports
scripts_dir = Path(__file__).parent.parent
sys.path.insert(0, str(scripts_dir))

# Import with proper module name (hyphen replaced by underscore for import)
import importlib.util
spec = importlib.util.spec_from_file_location("sync_wiki", scripts_dir / "sync-wiki.py")
sync_wiki = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sync_wiki)

import pytest

# Import functions from the loaded module
convert_admonitions = sync_wiki.convert_admonitions
convert_grid_cards_to_list = sync_wiki.convert_grid_cards_to_list
dedent_mkdocs_tabs = sync_wiki.dedent_mkdocs_tabs
find_code_fence_ranges = sync_wiki.find_code_fence_ranges
find_inline_code_ranges = sync_wiki.find_inline_code_ranges
path_to_wiki_name = sync_wiki.path_to_wiki_name
strip_mkdocs_attributes = sync_wiki.strip_mkdocs_attributes
strip_mkdocs_features = sync_wiki.strip_mkdocs_features
strip_mkdocs_icons = sync_wiki.strip_mkdocs_icons
transform_outside_code_blocks = sync_wiki.transform_outside_code_blocks


class TestFindCodeFenceRanges:
    """Tests for find_code_fence_ranges function."""

    def test_simple_code_block(self) -> None:
        content = "before\n```python\ncode\n```\nafter"
        ranges = find_code_fence_ranges(content)
        assert len(ranges) == 1
        # The range should span from ``` to closing ```
        assert content[ranges[0][0] : ranges[0][1]].startswith("```python")
        assert content[ranges[0][0] : ranges[0][1]].endswith("```")

    def test_multiple_code_blocks(self) -> None:
        content = "```\na\n```\ntext\n```\nb\n```"
        ranges = find_code_fence_ranges(content)
        assert len(ranges) == 2

    def test_tilde_fence(self) -> None:
        content = "~~~\ncode\n~~~"
        ranges = find_code_fence_ranges(content)
        assert len(ranges) == 1

    def test_longer_fence_contains_shorter(self) -> None:
        content = "````\n```\nnested\n```\n````"
        ranges = find_code_fence_ranges(content)
        # Should be one range for the outer fence
        assert len(ranges) == 1

    def test_unclosed_fence(self) -> None:
        content = "```python\ncode without closing"
        ranges = find_code_fence_ranges(content)
        assert len(ranges) == 1
        # Range should extend to end of content
        assert ranges[0][1] == len(content)


class TestFindInlineCodeRanges:
    """Tests for find_inline_code_ranges function."""

    def test_single_backtick(self) -> None:
        content = "text `code` text"
        ranges = find_inline_code_ranges(content)
        assert len(ranges) == 1
        assert content[ranges[0][0] : ranges[0][1]] == "`code`"

    def test_double_backtick(self) -> None:
        content = "text ``code with backtick`` text"
        ranges = find_inline_code_ranges(content)
        assert len(ranges) == 1
        assert "code with backtick" in content[ranges[0][0] : ranges[0][1]]

    def test_multiple_inline_codes(self) -> None:
        content = "`a` and `b` and `c`"
        ranges = find_inline_code_ranges(content)
        assert len(ranges) == 3


class TestDedentMkdocsTabs:
    """Tests for dedent_mkdocs_tabs function."""

    def test_simple_tab(self) -> None:
        content = '=== "Tab Name"\n\n    content here'
        result = dedent_mkdocs_tabs(content)
        assert "### Tab Name" in result
        assert "    content here" not in result
        assert "content here" in result

    def test_tab_with_code_block(self) -> None:
        content = '''=== "Cargo.toml"

    ```toml
    [dependencies]
    fortress = "0.11"
    ```'''
        result = dedent_mkdocs_tabs(content)
        assert "### Cargo.toml" in result
        # Code block should not have 4-space prefix
        assert "    ```toml" not in result
        assert "```toml" in result
        assert "[dependencies]" in result

    def test_multiple_tabs(self) -> None:
        content = '''=== "Tab 1"

    content 1

=== "Tab 2"

    content 2'''
        result = dedent_mkdocs_tabs(content)
        assert "### Tab 1" in result
        assert "### Tab 2" in result
        assert "content 1" in result
        assert "content 2" in result
        # Neither should have 4-space indentation
        assert "    content 1" not in result
        assert "    content 2" not in result

    def test_tab_preserves_inner_indentation(self) -> None:
        content = '''=== "Code"

    ```python
    def foo():
        return 42
    ```'''
        result = dedent_mkdocs_tabs(content)
        # The function body should still be indented relative to def
        # After dedenting by 4 spaces, "        return 42" becomes "    return 42"
        assert "    return 42" in result

    def test_non_tab_content_unchanged(self) -> None:
        content = "regular content\nwith no tabs"
        result = dedent_mkdocs_tabs(content)
        assert result == content


class TestConvertAdmonitions:
    """Tests for convert_admonitions function."""

    def test_simple_admonition(self) -> None:
        content = '!!! note "Important"\n    This is a note.'
        result = convert_admonitions(content)
        assert "> **Important**" in result
        assert "> This is a note." in result

    def test_admonition_without_title(self) -> None:
        content = "!!! warning\n    Be careful!"
        result = convert_admonitions(content)
        assert "> **Warning**" in result
        assert "> Be careful!" in result

    def test_admonition_multiline_content(self) -> None:
        content = '!!! note "Title"\n    Line 1\n    Line 2\n    Line 3'
        result = convert_admonitions(content)
        assert "> **Title**" in result
        assert "> Line 1" in result
        assert "> Line 2" in result
        assert "> Line 3" in result

    def test_admonition_with_empty_line(self) -> None:
        content = '!!! note "Title"\n    Para 1\n\n    Para 2'
        result = convert_admonitions(content)
        # Empty line should become just ">"
        lines = result.split("\n")
        assert ">" in lines  # Standalone > for empty line

    def test_content_after_admonition(self) -> None:
        content = '!!! note "Title"\n    Content\n\nRegular text'
        result = convert_admonitions(content)
        assert "Regular text" in result
        # Regular text should NOT be in blockquote
        assert "> Regular text" not in result


class TestStripMkdocsIcons:
    """Tests for strip_mkdocs_icons function."""

    def test_material_icon(self) -> None:
        content = ":material-star: Star rating"
        result = strip_mkdocs_icons(content)
        assert result == "Star rating"

    def test_octicons_icon(self) -> None:
        content = ":octicons-arrow-right-24: Continue"
        result = strip_mkdocs_icons(content)
        assert result == "Continue"

    def test_fontawesome_icon(self) -> None:
        content = ":fontawesome-solid-check: Done"
        result = strip_mkdocs_icons(content)
        assert result == "Done"

    def test_multiple_icons(self) -> None:
        content = ":material-star: :material-heart: Love"
        result = strip_mkdocs_icons(content)
        assert result == "Love"

    def test_icon_in_link(self) -> None:
        content = "[:material-github: GitHub](https://github.com)"
        result = strip_mkdocs_icons(content)
        assert result == "[GitHub](https://github.com)"


class TestStripMkdocsAttributes:
    """Tests for strip_mkdocs_attributes function."""

    def test_class_attribute(self) -> None:
        content = "{ .lg .middle }"
        result = strip_mkdocs_attributes(content)
        assert result == ""

    def test_id_attribute(self) -> None:
        content = "{ #my-id }"
        result = strip_mkdocs_attributes(content)
        assert result == ""

    def test_mixed_attributes(self) -> None:
        content = "text { .class #id } more text"
        result = strip_mkdocs_attributes(content)
        assert result == "text  more text"

    def test_preserves_non_attribute_braces(self) -> None:
        # Should NOT remove braces that don't start with . or #
        content = "{variable}"
        result = strip_mkdocs_attributes(content)
        assert result == "{variable}"


class TestConvertGridCardsToList:
    """Tests for convert_grid_cards_to_list function."""

    def test_simple_grid_cards(self) -> None:
        """Grid cards with single card converts to markdown list."""
        content = '''<div class="grid cards" markdown>

-   :material-star:{ .lg .middle } **Feature One**

    ---

    Description of feature one.

    [:octicons-arrow-right-24: Learn more](user-guide.md)

</div>'''
        result = convert_grid_cards_to_list(content)
        assert "**Feature One**" in result
        assert "Description of feature one." in result
        assert "[Learn more](user-guide.md)" in result
        # Should be a list item
        assert result.strip().startswith("-")

    def test_multiple_cards(self) -> None:
        """Multiple cards in grid convert to multiple list items."""
        content = '''<div class="grid cards" markdown>

-   :material-one: **Card One**

    ---

    First card description.

    [:octicons-arrow-right-24: Link One](page1.md)

-   :material-two: **Card Two**

    ---

    Second card description.

    [:octicons-arrow-right-24: Link Two](page2.md)

</div>'''
        result = convert_grid_cards_to_list(content)
        # Both cards should be present
        assert "**Card One**" in result
        assert "**Card Two**" in result
        assert "First card description." in result
        assert "Second card description." in result
        assert "[Link One](page1.md)" in result
        assert "[Link Two](page2.md)" in result
        # Should have two list items
        lines = [l for l in result.strip().split('\n') if l.strip().startswith('-')]
        assert len(lines) == 2

    def test_nested_divs(self) -> None:
        """Grid cards with nested divs are handled correctly."""
        content = '<div class="grid cards" markdown><div>nested</div></div>'
        result = convert_grid_cards_to_list(content)
        # Content may be empty or converted - just verify no crash
        assert "<div" not in result

    def test_preserves_surrounding_content(self) -> None:
        """Content before and after grid cards is preserved."""
        content = '''before

<div class="grid cards" markdown>

-   :icon: **Title**

    ---

    Desc.

    [:octicons-arrow: Link](url)

</div>

after'''
        result = convert_grid_cards_to_list(content)
        assert "before" in result
        assert "after" in result
        assert "**Title**" in result

    def test_card_without_link(self) -> None:
        """Cards without link lines are handled."""
        content = '''<div class="grid cards" markdown>

-   :icon: **No Link Card**

    ---

    Just a description.

</div>'''
        result = convert_grid_cards_to_list(content)
        assert "**No Link Card**" in result
        assert "Just a description." in result

    def test_empty_grid_cards(self) -> None:
        """Empty grid cards produce no output (no blank lines)."""
        content = '<div class="grid cards" markdown></div>'
        result = convert_grid_cards_to_list(content)
        # Empty grid cards should be completely removed (no blank lines)
        assert result == ""
        assert "<div" not in result

    def test_empty_grid_cards_surrounded_by_content(self) -> None:
        """Empty grid cards don't add extra blank lines beyond surrounding whitespace."""
        # The surrounding newlines are preserved, but empty grid produces no extra output
        content = 'Before\n<div class="grid cards" markdown></div>\nAfter'
        result = convert_grid_cards_to_list(content)
        # One newline before div, one after - no extra added by empty conversion
        assert result == "Before\n\nAfter"
        # Verify the div was actually removed
        assert "<div" not in result
        # Before the fix, empty grids added an extra "\n" resulting in triple newlines
        assert "\n\n\n" not in result

    def test_preserves_regular_content(self) -> None:
        """Content without grid cards is unchanged."""
        content = "regular content\nwith no grids"
        result = convert_grid_cards_to_list(content)
        assert result == content

    def test_multiple_grid_sections(self) -> None:
        """Multiple grid card sections in same document are all converted."""
        content = '''## Features

<div class="grid cards" markdown>

-   :icon: **Feature 1**

    ---

    Description 1.

    [:octicons-arrow-right-24: Link 1](page1.md)

</div>

## Navigation

<div class="grid cards" markdown>

-   :icon: **Nav Item**

    ---

    Nav description.

    [:octicons-arrow-right-24: Go](nav.md)

</div>'''
        result = convert_grid_cards_to_list(content)
        # Both sections should have content
        assert "**Feature 1**" in result
        assert "**Nav Item**" in result
        assert "[Link 1](page1.md)" in result
        assert "[Go](nav.md)" in result
        # Headers should be preserved
        assert "## Features" in result
        assert "## Navigation" in result

    def test_card_with_only_title(self) -> None:
        """Card with only a title (no description, no link) is handled."""
        content = '''<div class="grid cards" markdown>

-   :icon: **Minimal Card**

</div>'''
        result = convert_grid_cards_to_list(content)
        assert "**Minimal Card**" in result
        # Should create a list item
        assert result.strip().startswith("-")

    def test_closing_div_with_whitespace(self) -> None:
        """Closing div with whitespace before > is handled."""
        content = '<div class="grid cards" markdown>-   :i: **Test**\n\n    ---\n\n    Desc.\n\n</div >'
        result = convert_grid_cards_to_list(content)
        # Should not crash, should extract content
        assert "**Test**" in result

    def test_unclosed_grid_cards_div(self) -> None:
        """Unclosed grid cards div (no matching </div>) is handled gracefully.

        This is a regression test for a bug where an unclosed div would cause
        incorrect content truncation. Now closing_tag_len is initialized to 0,
        so unclosed divs naturally don't subtract anything from the end position.
        """
        content = '''<div class="grid cards" markdown>

-   :icon: **Unclosed Card**

    ---

    This div has no closing tag.

    [:octicons-arrow-right-24: Link](url.md)

Some content after that should be preserved'''
        result = convert_grid_cards_to_list(content)
        # Should not crash
        # Should extract the card content
        assert "**Unclosed Card**" in result
        assert "This div has no closing tag." in result
        # The content at the end should NOT be truncated
        # Before the fix, "preserved" would be cut to "pres" (6 chars removed)
        # With the unclosed div, everything after opening tag becomes div_content
        # Critical: "preserved" should NOT be truncated to "pres" or similar
        assert "preserved" in result

    def test_external_url_in_link(self) -> None:
        """External URLs in card links are preserved."""
        content = '''<div class="grid cards" markdown>

-   :icon: **External**

    ---

    External link test.

    [:octicons-arrow-right-24: Visit](https://example.com)

</div>'''
        result = convert_grid_cards_to_list(content)
        assert "https://example.com" in result
        assert "[Visit](https://example.com)" in result


class TestTransformOutsideCodeBlocks:
    """Tests for transform_outside_code_blocks function."""

    def test_transforms_outside_code(self) -> None:
        content = "text :material-star: text"
        result = transform_outside_code_blocks(content, strip_mkdocs_icons)
        # Icon stripped, space consumed, so "text text" not "text  text"
        assert result == "text text"

    def test_preserves_inside_code_block(self) -> None:
        content = "text\n```\n:material-star:\n```\ntext"
        result = transform_outside_code_blocks(content, strip_mkdocs_icons)
        assert ":material-star:" in result  # Preserved inside code

    def test_preserves_inside_inline_code(self) -> None:
        content = "text `:material-star:` text"
        result = transform_outside_code_blocks(content, strip_mkdocs_icons)
        assert ":material-star:" in result  # Preserved inside inline code

    def test_transforms_mixed_content(self) -> None:
        content = ":material-a: `code` :material-b:"
        result = transform_outside_code_blocks(content, strip_mkdocs_icons)
        # Outside code should be stripped
        assert ":material-a:" not in result
        assert ":material-b:" not in result
        # Inline code should be preserved
        assert "`code`" in result


class TestStripMkdocsFeatures:
    """Integration tests for strip_mkdocs_features function."""

    def test_tabs_with_code_blocks_render_correctly(self) -> None:
        """Verify the main issue is fixed: tabs with code blocks."""
        content = '''=== "Cargo.toml"

    ```toml
    [dependencies]
    fortress = "0.11"
    ```

=== "Code"

    ```rust
    fn main() {
        println!("Hello");
    }
    ```'''
        result = strip_mkdocs_features(content)

        # Should have headers
        assert "### Cargo.toml" in result
        assert "### Code" in result

        # Code blocks should NOT have leading 4-space indent
        # (which would prevent them from rendering)
        lines = result.split("\n")
        for i, line in enumerate(lines):
            if line.strip().startswith("```"):
                # The ``` should not be indented by 4 spaces
                assert not line.startswith("    "), f"Line {i} has unwanted indent: {repr(line)}"

    def test_admonition_content_in_blockquote(self) -> None:
        """Verify admonitions convert properly with content."""
        content = '''!!! note "Important"
    This is important info.
    With multiple lines.'''
        result = strip_mkdocs_features(content)

        assert "> **Important**" in result
        assert "> This is important info." in result
        assert "> With multiple lines." in result

    def test_icons_stripped_outside_code_only(self) -> None:
        """Verify icons are only stripped outside code blocks."""
        content = ''':material-star: Feature

```python
# Document :material-star: usage
pattern = r":material-[a-z]+:"
```'''
        result = strip_mkdocs_features(content)

        # Icon outside code should be stripped
        lines = result.split("\n")
        assert lines[0] == "Feature"

        # Icon inside code should be preserved
        assert ":material-star:" in result


class TestPathToWikiName:
    """Tests for path_to_wiki_name function."""

    def test_simple_path(self) -> None:
        assert path_to_wiki_name("user-guide.md") == "User-Guide"

    def test_nested_path(self) -> None:
        # Takes just the filename
        assert path_to_wiki_name("specs/formal-spec.md") == "Formal-Spec"

    def test_already_capitalized(self) -> None:
        assert path_to_wiki_name("README.md") == "Readme"


class TestWikiValidation:
    """Tests that validate wiki output quality."""

    def test_no_indented_code_fences(self) -> None:
        """Code fences should never be preceded by 4+ spaces after processing."""
        # This is a regression test for the main bug
        content = '''=== "Example"

    ```rust
    fn foo() {}
    ```'''
        result = strip_mkdocs_features(content)

        lines = result.split("\n")
        for i, line in enumerate(lines):
            stripped = line.strip()
            if stripped.startswith("```"):
                # Line should not start with 4+ spaces
                leading_spaces = len(line) - len(line.lstrip())
                assert leading_spaces < 4, (
                    f"Code fence at line {i} has {leading_spaces} leading spaces. "
                    f"This will break GitHub Wiki rendering. Line: {repr(line)}"
                )

    def test_no_orphaned_admonition_content(self) -> None:
        """All admonition content lines should be prefixed with >."""
        content = '''!!! warning "Caution"
    This is a warning.
    Be careful.'''
        result = strip_mkdocs_features(content)

        lines = result.split("\n")
        in_blockquote = False
        for line in lines:
            if line.startswith("> **"):
                in_blockquote = True
            elif in_blockquote:
                if line.strip() and not line.startswith(">"):
                    # Non-empty line that doesn't start with > while in blockquote
                    # (unless it's a new paragraph after empty line)
                    if line.startswith("    "):
                        pytest.fail(
                            f"Orphaned admonition content found: {repr(line)}. "
                            "Should be prefixed with '> ' for blockquote."
                        )


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
