#!/usr/bin/env python3
"""Regression tests for docs sidebar layout styling."""

from __future__ import annotations

from pathlib import Path
import re


def _custom_css_path() -> Path:
    return Path(__file__).parent.parent.parent / "docs" / "stylesheets" / "custom.css"


def test_desktop_primary_sidebar_hidden() -> None:
    """Desktop CSS hides the primary left docs sidebar."""
    content = _custom_css_path().read_text(encoding="utf-8")
    pattern = re.compile(
        r"@media\s+screen\s+and\s+\(min-width:\s*76\.25em\)\s*\{"
        r".*?\.md-sidebar--primary\s*\{.*?display:\s*none;.*?\}"
        r".*?\}",
        re.DOTALL,
    )
    assert pattern.search(content) is not None, (
        "Expected custom.css to hide .md-sidebar--primary inside "
        "@media screen and (min-width: 76.25em). "
        f"Diagnostics: has media query="
        f"{'@media screen and (min-width: 76.25em)' in content}, "
        f"has selector={'.md-sidebar--primary' in content}, "
        f"has display none={'display: none;' in content}."
    )
