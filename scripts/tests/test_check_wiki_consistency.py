#!/usr/bin/env python3
"""
Unit tests for check-wiki-consistency.py validation functions.

These tests verify that the wiki consistency checker correctly identifies
problematic patterns that would cause broken links on GitHub Wiki.
"""

from __future__ import annotations

import sys
import tempfile
from pathlib import Path

# Add scripts directory to path for imports
scripts_dir = Path(__file__).parent.parent
sys.path.insert(0, str(scripts_dir))

# Import with proper module name using importlib
# We use dynamic import because the filename contains a hyphen (check-wiki-consistency.py)
# which isn't valid in Python module names (hyphens are interpreted as minus operator).
# This approach loads the module with a valid Python identifier.
import importlib.util
spec = importlib.util.spec_from_file_location(
    "check_wiki_consistency", scripts_dir / "check-wiki-consistency.py"
)
check_wiki_consistency = importlib.util.module_from_spec(spec)
spec.loader.exec_module(check_wiki_consistency)

import pytest

# Import functions from the loaded module
parse_sidebar_wiki_links = check_wiki_consistency.parse_sidebar_wiki_links
validate_wiki_link_display_text = check_wiki_consistency.validate_wiki_link_display_text
validate_sidebar_links = check_wiki_consistency.validate_sidebar_links
WIKI_LINK_PROBLEMATIC_CHARS = check_wiki_consistency.WIKI_LINK_PROBLEMATIC_CHARS


class TestParseSidebarWikiLinks:
    """Tests for parse_sidebar_wiki_links function."""

    def test_simple_wiki_link(self, tmp_path: Path) -> None:
        """Simple wiki link without display text."""
        sidebar = tmp_path / "_Sidebar.md"
        sidebar.write_text("[[Home]]", encoding="utf-8")
        links = parse_sidebar_wiki_links(sidebar)
        assert len(links) == 1
        assert links[0] == ("Home", "Home", 1)

    def test_wiki_link_with_display_text(self, tmp_path: Path) -> None:
        """Wiki link with custom display text."""
        sidebar = tmp_path / "_Sidebar.md"
        sidebar.write_text("[[User-Guide|User Guide]]", encoding="utf-8")
        links = parse_sidebar_wiki_links(sidebar)
        assert len(links) == 1
        assert links[0] == ("User-Guide", "User Guide", 1)

    def test_multiple_wiki_links(self, tmp_path: Path) -> None:
        """Multiple wiki links on different lines."""
        sidebar = tmp_path / "_Sidebar.md"
        sidebar.write_text(
            "[[Home]]\n[[User-Guide|User Guide]]\n[[Architecture]]",
            encoding="utf-8",
        )
        links = parse_sidebar_wiki_links(sidebar)
        assert len(links) == 3
        assert links[0] == ("Home", "Home", 1)
        assert links[1] == ("User-Guide", "User Guide", 2)
        assert links[2] == ("Architecture", "Architecture", 3)

    def test_wiki_link_in_list(self, tmp_path: Path) -> None:
        """Wiki links inside markdown list items."""
        sidebar = tmp_path / "_Sidebar.md"
        sidebar.write_text("- [[Home]]\n- [[User-Guide|User Guide]]", encoding="utf-8")
        links = parse_sidebar_wiki_links(sidebar)
        assert len(links) == 2

    def test_empty_file(self, tmp_path: Path) -> None:
        """Empty sidebar file returns no links."""
        sidebar = tmp_path / "_Sidebar.md"
        sidebar.write_text("", encoding="utf-8")
        links = parse_sidebar_wiki_links(sidebar)
        assert len(links) == 0

    def test_nonexistent_file(self, tmp_path: Path) -> None:
        """Non-existent sidebar file returns no links."""
        sidebar = tmp_path / "_Sidebar.md"
        # Don't create the file
        links = parse_sidebar_wiki_links(sidebar)
        assert len(links) == 0


class TestValidateWikiLinkDisplayText:
    """Tests for validate_wiki_link_display_text function."""

    def test_safe_display_text(self, tmp_path: Path) -> None:
        """Display text without problematic characters passes validation."""
        sidebar = tmp_path / "_Sidebar.md"
        sidebar.write_text(
            "[[Home]]\n[[User-Guide|User Guide]]\n[[TLAplus-Tooling|TLA Plus Tooling]]",
            encoding="utf-8",
        )
        result = validate_wiki_link_display_text(sidebar)
        assert result.errors == 0
        assert result.warnings == 0

    def test_plus_sign_in_display_text(self, tmp_path: Path) -> None:
        """Plus sign in display text is detected as error."""
        sidebar = tmp_path / "_Sidebar.md"
        sidebar.write_text("[[TLAplus-Tooling|TLA+ Tooling]]", encoding="utf-8")
        result = validate_wiki_link_display_text(sidebar)
        assert result.errors == 1

    def test_percent_sign_in_display_text(self, tmp_path: Path) -> None:
        """Percent sign in display text is detected as error."""
        sidebar = tmp_path / "_Sidebar.md"
        sidebar.write_text("[[Page|100% Complete]]", encoding="utf-8")
        result = validate_wiki_link_display_text(sidebar)
        assert result.errors == 1

    def test_hash_in_display_text(self, tmp_path: Path) -> None:
        """Hash in display text is detected as error."""
        sidebar = tmp_path / "_Sidebar.md"
        sidebar.write_text("[[Page|Issue #123]]", encoding="utf-8")
        result = validate_wiki_link_display_text(sidebar)
        assert result.errors == 1

    def test_question_mark_in_display_text(self, tmp_path: Path) -> None:
        """Question mark in display text is detected as error."""
        sidebar = tmp_path / "_Sidebar.md"
        sidebar.write_text("[[FAQ|Questions?]]", encoding="utf-8")
        result = validate_wiki_link_display_text(sidebar)
        assert result.errors == 1

    def test_ampersand_in_display_text(self, tmp_path: Path) -> None:
        """Ampersand in display text is detected as error."""
        sidebar = tmp_path / "_Sidebar.md"
        sidebar.write_text("[[Page|Q&A]]", encoding="utf-8")
        result = validate_wiki_link_display_text(sidebar)
        assert result.errors == 1

    def test_multiple_problematic_links(self, tmp_path: Path) -> None:
        """Multiple links with problematic characters are all detected."""
        sidebar = tmp_path / "_Sidebar.md"
        sidebar.write_text(
            "[[Page1|TLA+ Tools]]\n[[Page2|100% Done]]\n[[Page3|Safe Text]]",
            encoding="utf-8",
        )
        result = validate_wiki_link_display_text(sidebar)
        # Two problematic links
        assert result.errors == 2

    def test_multiple_problematic_chars_in_single_link(self, tmp_path: Path) -> None:
        """Link with multiple problematic characters reports only one error.

        This tests the design decision that we only report the first problematic
        character per link to avoid overwhelming error output. Users fix one issue,
        re-run validation, and see the next issue if any.
        """
        sidebar = tmp_path / "_Sidebar.md"
        # This has +, &, and ? all in one display text
        sidebar.write_text("[[FAQ|Q&A + More?]]", encoding="utf-8")
        result = validate_wiki_link_display_text(sidebar)
        # Only 1 error reported per link (first problematic char found)
        assert result.errors == 1

    def test_equals_sign_in_display_text(self, tmp_path: Path) -> None:
        """Equals sign in display text is detected as error."""
        sidebar = tmp_path / "_Sidebar.md"
        sidebar.write_text("[[Page|Key=Value]]", encoding="utf-8")
        result = validate_wiki_link_display_text(sidebar)
        assert result.errors == 1

    def test_page_name_can_have_special_chars(self, tmp_path: Path) -> None:
        """Special characters in page name (not display text) are allowed.

        The validation only checks display text because that's what causes
        the URL corruption bug in GitHub Wiki.
        """
        sidebar = tmp_path / "_Sidebar.md"
        # Page name has special chars but display text is safe
        sidebar.write_text("[[C++-Guide|CPP Guide]]", encoding="utf-8")
        result = validate_wiki_link_display_text(sidebar)
        # This should pass because display text is safe
        assert result.errors == 0


class TestValidateSidebarLinks:
    """Tests for validate_sidebar_links function."""

    def test_valid_links(self, tmp_path: Path) -> None:
        """All links point to existing pages."""
        sidebar = tmp_path / "_Sidebar.md"
        sidebar.write_text("[[Home]]\n[[Guide]]", encoding="utf-8")
        wiki_pages = {"Home", "Guide"}
        result = validate_sidebar_links(sidebar, wiki_pages)
        assert result.errors == 0

    def test_missing_page(self, tmp_path: Path) -> None:
        """Link to non-existent page is detected as error."""
        sidebar = tmp_path / "_Sidebar.md"
        sidebar.write_text("[[Home]]\n[[Missing-Page]]", encoding="utf-8")
        wiki_pages = {"Home"}
        result = validate_sidebar_links(sidebar, wiki_pages)
        assert result.errors == 1

    def test_multiple_missing_pages(self, tmp_path: Path) -> None:
        """Multiple missing pages are all reported."""
        sidebar = tmp_path / "_Sidebar.md"
        sidebar.write_text("[[Home]]\n[[Missing1]]\n[[Missing2]]", encoding="utf-8")
        wiki_pages = {"Home"}
        result = validate_sidebar_links(sidebar, wiki_pages)
        assert result.errors == 2


class TestProblematicCharsList:
    """Tests for WIKI_LINK_PROBLEMATIC_CHARS constant."""

    def test_plus_sign_documented(self) -> None:
        """Plus sign is in the problematic characters list."""
        assert "+" in WIKI_LINK_PROBLEMATIC_CHARS

    def test_percent_sign_documented(self) -> None:
        """Percent sign is in the problematic characters list."""
        assert "%" in WIKI_LINK_PROBLEMATIC_CHARS

    def test_all_chars_have_explanation(self) -> None:
        """All problematic characters have an explanation."""
        for char, explanation in WIKI_LINK_PROBLEMATIC_CHARS.items():
            assert len(char) == 1, f"'{char}' should be a single character"
            assert len(explanation) > 10, f"'{char}' needs a meaningful explanation"


class TestRealWorldCases:
    """Integration tests with real-world sidebar content."""

    def test_actual_sidebar_format(self, tmp_path: Path) -> None:
        """Test with realistic sidebar content structure."""
        sidebar = tmp_path / "_Sidebar.md"
        sidebar.write_text(
            """# Fortress Rollback

**[[Home]]**

## Documentation

- [[User-Guide|User Guide]]
- [[Architecture]]
- [[Migration]]

## Reference

- [[Changelog]]
- [[TLAplus-Tooling-Research|TLA Plus Tooling Research]]

## Community

- [[Contributing]]
- [[Code-of-Conduct|Code of Conduct]]
""",
            encoding="utf-8",
        )
        links = parse_sidebar_wiki_links(sidebar)
        # Home, User-Guide, Architecture, Migration, Changelog, TLAplus-Tooling-Research,
        # Contributing, Code-of-Conduct = 8 links
        assert len(links) == 8

        # All display text should be safe
        result = validate_wiki_link_display_text(sidebar)
        assert result.errors == 0

    def test_broken_tla_plus_link(self, tmp_path: Path) -> None:
        """Test the exact bug that was reported - TLA+ in display text."""
        sidebar = tmp_path / "_Sidebar.md"
        sidebar.write_text(
            "- [[TLAplus-Tooling-Research|TLA+ Tooling Research]]",
            encoding="utf-8",
        )
        result = validate_wiki_link_display_text(sidebar)
        # This should detect the '+' character
        assert result.errors == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
