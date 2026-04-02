#!/usr/bin/env python3
"""Regression tests for docs sidebar layout styling."""

from __future__ import annotations

from pathlib import Path


def _custom_css_path() -> Path:
    return Path(__file__).parent.parent.parent / "docs" / "stylesheets" / "custom.css"


def test_desktop_primary_sidebar_hidden() -> None:
    """Desktop CSS hides the primary left docs sidebar."""
    content = _custom_css_path().read_text(encoding="utf-8")
    assert "@media screen and (min-width: 76.25em)" in content
    assert ".md-sidebar--primary" in content
    assert "display: none;" in content

