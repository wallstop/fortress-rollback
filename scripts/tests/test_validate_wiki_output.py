#!/usr/bin/env python3
"""Tests for validate-wiki-output.py functionality.

These tests verify that the validation functions correctly detect issues
in wiki markdown content, particularly around index handling with enumerate().
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

# Load the module with dashes in its name
_script_path = Path(__file__).parent.parent / "validate-wiki-output.py"
_spec = importlib.util.spec_from_file_location("validate_wiki_output", _script_path)
_module = importlib.util.module_from_spec(_spec)
sys.modules["validate_wiki_output"] = _module
_spec.loader.exec_module(_module)

from validate_wiki_output import (  # noqa: E402
    check_empty_sections,
    check_indented_code_fences,
    check_orphaned_indented_content,
    check_unconverted_mkdocs_syntax,
)


class TestCheckEmptySections:
    """Tests for check_empty_sections function.

    These tests specifically verify that the 1-indexed enumerate() to 0-indexed
    list access conversion works correctly for looking ahead at subsequent lines.

    Key insight: When i is the 1-indexed line number of lines[i-1], using
    lines[i] correctly accesses the NEXT line because:
    - lines[i-1] is the current line (0-indexed position = i-1)
    - lines[i] is the next line (0-indexed position = i)

    This relationship holds: 1-indexed line number == 0-indexed next line position.
    """

    def test_section_with_content_no_issues(self) -> None:
        """A section with content should not report any issues."""
        content = """## Header

This section has content.
"""
        issues = check_empty_sections(content, "test.md")
        assert len(issues) == 0

    def test_empty_section_reports_issue(self) -> None:
        """An empty section should be reported as an error."""
        content = """## Empty Header

## Next Header

Content here.
"""
        issues = check_empty_sections(content, "test.md")
        assert len(issues) == 1
        assert issues[0].line == 1
        assert "Empty Header" in issues[0].message
        assert issues[0].severity == "error"

    def test_section_at_end_of_file_with_no_content(self) -> None:
        """A section at the end of file with no content should report issue."""
        content = """## Header

Content here.

## Empty Final Section
"""
        issues = check_empty_sections(content, "test.md")
        assert len(issues) == 1
        assert "Empty Final Section" in issues[0].message

    def test_section_with_only_whitespace(self) -> None:
        """Section with only whitespace should report as empty."""
        content = """## Header




## Next Section

Content.
"""
        issues = check_empty_sections(content, "test.md")
        assert len(issues) == 1
        assert issues[0].line == 1
        assert "Header" in issues[0].message

    def test_first_line_is_header_with_content(self) -> None:
        """First line being a header with immediate content should work.

        This tests the edge case where i=1 and we need to access lines[1].
        """
        content = """## First Header
Immediate content on line 2.
"""
        issues = check_empty_sections(content, "test.md")
        assert len(issues) == 0

    def test_single_line_header_file(self) -> None:
        """File with only a header line (no subsequent content) should report issue.

        This tests the boundary where j < len(lines) check prevents IndexError.
        """
        content = "## Only Header"
        issues = check_empty_sections(content, "test.md")
        assert len(issues) == 1
        assert "Only Header" in issues[0].message

    def test_multiple_headers_various_states(self) -> None:
        """Test multiple headers, some empty, some with content."""
        content = """## First Section

Content here.

## Empty Section

## Third Section

More content.

### Nested Empty

## Fourth With Content

Last content.
"""
        issues = check_empty_sections(content, "test.md")
        # Should detect: Empty Section, Nested Empty
        empty_names = [i.message for i in issues]
        assert any("Empty Section" in m for m in empty_names)
        assert any("Nested Empty" in m for m in empty_names)
        assert not any("First Section" in m for m in empty_names)
        assert not any("Third Section" in m for m in empty_names)
        assert not any("Fourth" in m for m in empty_names)

    def test_section_with_horizontal_rule_only(self) -> None:
        """Section with only --- should be reported as empty."""
        content = """## Header

---

## Next
Content.
"""
        issues = check_empty_sections(content, "test.md")
        assert len(issues) == 1
        assert "Header" in issues[0].message

    def test_section_with_html_comment_only(self) -> None:
        """Section with only HTML comments should be reported as empty."""
        content = """## Header

<!-- comment -->

## Next
Content.
"""
        issues = check_empty_sections(content, "test.md")
        assert len(issues) == 1
        assert "Header" in issues[0].message

    def test_subsection_with_content_parent_not_empty(self) -> None:
        """A section with a subsection containing content is not empty."""
        content = """## Parent

### Child

Content in child.
"""
        issues = check_empty_sections(content, "test.md")
        # Child has content, but Parent is followed by Child (same/higher level check)
        # Parent has no direct content before Child subsection
        # The function checks if there's content before hitting a same-or-higher level header
        # ### is lower level than ##, so Parent section continues into Child
        # Child has content, so no issues
        assert len(issues) == 0

    def test_line_number_correctness(self) -> None:
        """Verify reported line numbers match actual header positions."""
        content = """Line 1

## Header On Line 3
Content for line 3 header.

## Empty On Line 6

## Has Content Line 8
Content.
"""
        issues = check_empty_sections(content, "test.md")
        # Should find only "Empty On Line 6" at line 6
        assert len(issues) == 1
        assert issues[0].line == 6
        assert "Empty On Line 6" in issues[0].message


class TestCheckIndentedCodeFences:
    """Tests for check_indented_code_fences function."""

    def test_normal_code_fence_no_issues(self) -> None:
        """Normal code fences should not report issues."""
        content = """```python
code here
```
"""
        issues = check_indented_code_fences(content, "test.md")
        assert len(issues) == 0

    def test_indented_code_fence_reports_issue(self) -> None:
        """Code fences with 4+ spaces should report error."""
        content = """    ```python
    code here
    ```
"""
        issues = check_indented_code_fences(content, "test.md")
        assert len(issues) >= 1
        assert issues[0].severity == "error"


class TestCheckUnconvertedMkdocsSyntax:
    """Tests for check_unconverted_mkdocs_syntax function."""

    def test_clean_content_no_issues(self) -> None:
        """Clean markdown should not report issues."""
        content = """## Header

Normal content here.
"""
        issues = check_unconverted_mkdocs_syntax(content, "test.md")
        assert len(issues) == 0

    def test_tab_marker_reports_issue(self) -> None:
        """Unconverted tab markers should report error."""
        content = """=== "Tab Name"

Content
"""
        issues = check_unconverted_mkdocs_syntax(content, "test.md")
        assert len(issues) >= 1
        assert any("tab marker" in i.message.lower() for i in issues)

    def test_admonition_reports_issue(self) -> None:
        """Unconverted admonitions should report error."""
        content = """!!! note

Content
"""
        issues = check_unconverted_mkdocs_syntax(content, "test.md")
        assert len(issues) >= 1
        assert any("admonition" in i.message.lower() for i in issues)


class TestCheckOrphanedIndentedContent:
    """Tests for check_orphaned_indented_content function."""

    def test_normal_content_no_issues(self) -> None:
        """Normal content should not report issues."""
        content = """## Header

Normal paragraph content.
"""
        issues = check_orphaned_indented_content(content, "test.md")
        assert len(issues) == 0

    def test_code_block_content_ignored(self) -> None:
        """Indented content inside code blocks should be ignored."""
        content = """```python
    indented_code = True
```
"""
        issues = check_orphaned_indented_content(content, "test.md")
        assert len(issues) == 0


if __name__ == "__main__":
    import pytest

    pytest.main([__file__, "-v"])
