#!/usr/bin/env python3
"""Validate docs theme color mappings for light/dark accessibility consistency."""

from __future__ import annotations

import re
import sys
from pathlib import Path
from typing import NamedTuple


class ValidationResult(NamedTuple):
    """Result of theme color validation."""

    errors: int
    warnings: int


def get_project_root() -> Path:
    """Get project root from this script location."""
    script_dir = Path(__file__).parent.resolve()
    return script_dir.parent.parent


def extract_css_block(content: str, selector: str) -> str:
    """Extract a simple selector block body."""
    pattern = re.compile(rf"{re.escape(selector)}\s*\{{([^}}]+)\}}", re.MULTILINE)
    match = pattern.search(content)
    return match.group(1) if match else ""


def extract_custom_property(block: str, name: str) -> str | None:
    """Extract CSS custom property value from a block."""
    pattern = re.compile(rf"{re.escape(name)}\s*:\s*([^;]+);")
    match = pattern.search(block)
    return match.group(1).strip() if match else None


def validate_theme_colors(css_path: Path) -> ValidationResult:
    """Validate that header color variables are mode-aware and non-orange."""
    errors = 0
    warnings = 0

    try:
        css = css_path.read_text(encoding="utf-8")
    except OSError as error:
        print(f"{css_path}:1: error: could not read file: {error}", file=sys.stderr)
        return ValidationResult(errors=1, warnings=0)

    dark_block = extract_css_block(css, '[data-md-color-scheme="slate"]')
    light_block = extract_css_block(css, '[data-md-color-scheme="default"]')

    if not dark_block:
        print(
            f"{css_path}:1: error: missing [data-md-color-scheme=\"slate\"] block",
            file=sys.stderr,
        )
        errors += 1
    if not light_block:
        print(
            f"{css_path}:1: error: missing [data-md-color-scheme=\"default\"] block",
            file=sys.stderr,
        )
        errors += 1
    if errors:
        return ValidationResult(errors=errors, warnings=warnings)

    dark_primary = extract_custom_property(dark_block, "--md-primary-fg-color")
    light_primary = extract_custom_property(light_block, "--md-primary-fg-color")
    dark_text = extract_custom_property(dark_block, "--md-primary-bg-color")
    light_text = extract_custom_property(light_block, "--md-primary-bg-color")

    required = {
        "--md-primary-fg-color (slate)": dark_primary,
        "--md-primary-fg-color (default)": light_primary,
        "--md-primary-bg-color (slate)": dark_text,
        "--md-primary-bg-color (default)": light_text,
    }
    for label, value in required.items():
        if value is None:
            print(
                f"{css_path}:1: error: missing required property {label}",
                file=sys.stderr,
            )
            errors += 1

    if errors:
        return ValidationResult(errors=errors, warnings=warnings)

    forbidden_orange_tokens = (
        "var(--fortress-rust-orange)",
        "var(--fortress-rust-orange-light)",
        "var(--fortress-rust-orange-dark)",
        "#F74C00",
        "#FF6B2C",
        "#D94000",
    )

    for label, value in (
        ("dark primary header background", dark_primary),
        ("light primary header background", light_primary),
    ):
        if value in forbidden_orange_tokens:
            print(
                f"{css_path}:1: error: {label} must not use the orange accent palette ({value})",
                file=sys.stderr,
            )
            errors += 1

    if dark_primary == light_primary:
        print(
            f"{css_path}:1: error: light/dark --md-primary-fg-color values must differ for mode-aware header backgrounds",
            file=sys.stderr,
        )
        errors += 1

    return ValidationResult(errors=errors, warnings=warnings)


def main() -> int:
    """Run theme color validation."""
    root = get_project_root()
    css_path = root / "docs" / "stylesheets" / "custom.css"

    result = validate_theme_colors(css_path)
    if result.errors:
        print(
            f"✗ Theme color validation failed with {result.errors} error(s)",
            file=sys.stderr,
        )
        return 1

    print("✓ Theme color validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
