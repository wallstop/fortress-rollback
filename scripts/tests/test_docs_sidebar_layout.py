#!/usr/bin/env python3
"""Regression tests for docs sidebar layout styling."""

from __future__ import annotations

from pathlib import Path
import re

DESKTOP_MEDIA_QUERY = "@media screen and (min-width: 76.25em)"

def _custom_css_path() -> Path:
    return Path(__file__).parent.parent.parent / "docs" / "stylesheets" / "custom.css"


def _extract_desktop_media_block(content: str) -> str | None:
    """Return the body of the desktop media block, or None if not found/malformed.

    This uses explicit brace-depth tracking so nested braces elsewhere in the file
    don't break extraction of the `@media screen and (min-width: 76.25em)` block.
    """
    media_pattern = re.compile(
        r"@media\s+screen\s+and\s+\(min-width:\s*76\.25em\)\s*\{",
        re.DOTALL,
    )
    match = media_pattern.search(content)
    if match is None:
        return None

    block_start = match.end()
    depth = 1
    idx = block_start
    while idx < len(content) and depth > 0:
        char = content[idx]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
        idx += 1

    if depth != 0:
        return None

    return content[block_start : idx - 1]


def test_desktop_primary_sidebar_hidden() -> None:
    """Desktop CSS hides the primary left docs sidebar."""
    content = _custom_css_path().read_text(encoding="utf-8")

    media_block = _extract_desktop_media_block(content)
    assert media_block is not None, (
        f"Expected custom.css to contain {DESKTOP_MEDIA_QUERY}. "
        "Diagnostics: has selector="
        f"{'.md-sidebar--primary' in content}, "
        f"has display none={re.search(r'display\\s*:\\s*none;', content) is not None}."
    )

    sidebar_rule_pattern = re.compile(
        # This intentionally matches flat declarations only (no nested braces),
        # which is correct for `.md-sidebar--primary { ... }` declaration blocks.
        r"\.md-sidebar--primary\s*\{[^{}]*\bdisplay\s*:\s*none\s*;[^{}]*\}"
    )
    assert sidebar_rule_pattern.search(media_block) is not None, (
        "Expected custom.css to hide .md-sidebar--primary inside "
        f"{DESKTOP_MEDIA_QUERY}. "
        f"Diagnostics: media block excerpt={media_block[:200]!r}."
    )
