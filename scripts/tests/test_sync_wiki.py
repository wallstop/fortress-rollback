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
dedent_mkdocs_tabs = sync_wiki.dedent_mkdocs_tabs
find_code_fence_ranges = sync_wiki.find_code_fence_ranges
find_inline_code_ranges = sync_wiki.find_inline_code_ranges
path_to_wiki_name = sync_wiki.path_to_wiki_name
remove_grid_cards_divs = sync_wiki.remove_grid_cards_divs
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


class TestRemoveGridCardsDivs:
    """Tests for remove_grid_cards_divs function."""

    def test_simple_grid_cards(self) -> None:
        content = 'before<div class="grid cards" markdown>content</div>after'
        result = remove_grid_cards_divs(content)
        assert result == "beforeafter"

    def test_nested_divs(self) -> None:
        content = '<div class="grid cards" markdown><div>nested</div></div>'
        result = remove_grid_cards_divs(content)
        assert result == ""

    def test_preserves_other_content(self) -> None:
        content = "regular content"
        result = remove_grid_cards_divs(content)
        assert result == content


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
