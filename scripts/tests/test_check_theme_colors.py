#!/usr/bin/env python3
"""Unit tests for check-theme-colors.py."""

from __future__ import annotations

import importlib.util
from pathlib import Path

scripts_dir = Path(__file__).parent.parent
spec = importlib.util.spec_from_file_location(
    "check_theme_colors",
    scripts_dir / "docs" / "check-theme-colors.py",
)
check_theme_colors = importlib.util.module_from_spec(spec)
spec.loader.exec_module(check_theme_colors)

validate_theme_colors = check_theme_colors.validate_theme_colors


def _write(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")


def test_validate_theme_colors_accepts_mode_aware_non_orange_header(
    tmp_path: Path,
) -> None:
    css_path = tmp_path / "docs" / "stylesheets" / "custom.css"
    _write(
        css_path,
        """
[data-md-color-scheme="slate"] {
  --md-primary-fg-color: var(--fortress-bg-dark-secondary);
  --md-primary-bg-color: var(--fortress-text-dark);
}
[data-md-color-scheme="default"] {
  --md-primary-fg-color: var(--fortress-bg-light-secondary);
  --md-primary-bg-color: var(--fortress-text-light);
}
""",
    )

    result = validate_theme_colors(css_path)
    assert result.errors == 0


def test_validate_theme_colors_rejects_orange_header_background(tmp_path: Path) -> None:
    css_path = tmp_path / "docs" / "stylesheets" / "custom.css"
    _write(
        css_path,
        """
[data-md-color-scheme="slate"] {
  --md-primary-fg-color: var(--fortress-rust-orange);
  --md-primary-bg-color: var(--fortress-text-dark);
}
[data-md-color-scheme="default"] {
  --md-primary-fg-color: var(--fortress-bg-light-secondary);
  --md-primary-bg-color: var(--fortress-text-light);
}
""",
    )

    result = validate_theme_colors(css_path)
    assert result.errors == 1


def test_validate_theme_colors_requires_different_light_dark_primary(tmp_path: Path) -> None:
    css_path = tmp_path / "docs" / "stylesheets" / "custom.css"
    _write(
        css_path,
        """
[data-md-color-scheme="slate"] {
  --md-primary-fg-color: var(--fortress-bg-dark-secondary);
  --md-primary-bg-color: var(--fortress-text-dark);
}
[data-md-color-scheme="default"] {
  --md-primary-fg-color: var(--fortress-bg-dark-secondary);
  --md-primary-bg-color: var(--fortress-text-light);
}
""",
    )

    result = validate_theme_colors(css_path)
    assert result.errors == 1
