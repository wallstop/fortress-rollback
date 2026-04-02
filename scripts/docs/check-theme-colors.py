#!/usr/bin/env python3
"""Validate docs theme header colors for mode-awareness and accessibility."""

from __future__ import annotations

import re
import sys
from pathlib import Path
from typing import NamedTuple


class ValidationResult(NamedTuple):
    """Result of theme color validation."""

    errors: int
    warnings: int


ORANGE_HEX_COLORS = {"#f74c00", "#ff6b2c", "#d94000"}
ORANGE_VARIABLES = {
    "--fortress-rust-orange",
    "--fortress-rust-orange-light",
    "--fortress-rust-orange-dark",
}
MAX_VARIABLE_RESOLUTION_DEPTH = 8


def get_project_root() -> Path:
    """Get project root from this script location."""
    script_dir = Path(__file__).parent.resolve()
    return script_dir.parent.parent


def extract_css_block(content: str, selector: str) -> str:
    """Extract selector block body using brace matching."""
    selector_pattern = re.compile(rf"(?m)^\s*{re.escape(selector)}\s*\{{")
    match = selector_pattern.search(content)
    if match is None:
        return ""

    open_index = match.end() - 1
    depth = 1
    in_single_quote = False
    in_double_quote = False
    for index in range(open_index + 1, len(content)):
        char = content[index]
        prev_char = content[index - 1]
        if char == "'" and not in_double_quote and prev_char != "\\":
            in_single_quote = not in_single_quote
            continue
        if char == '"' and not in_single_quote and prev_char != "\\":
            in_double_quote = not in_double_quote
            continue
        if in_single_quote or in_double_quote:
            continue
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return content[open_index + 1 : index]
    return ""


def normalize_hex_color(value: str) -> str | None:
    """Normalize #rgb/#rrggbb to lowercase #rrggbb."""
    value = value.strip().lower()
    if not value.startswith("#"):
        return None
    hex_value = value[1:]
    if len(hex_value) == 3 and all(ch in "0123456789abcdef" for ch in hex_value):
        return "#" + "".join(ch * 2 for ch in hex_value)
    if len(hex_value) == 6 and all(ch in "0123456789abcdef" for ch in hex_value):
        return "#" + hex_value
    return None


def parse_rgb_color(value: str) -> str | None:
    """Parse rgb(...) into lowercase #rrggbb."""
    match = re.fullmatch(
        r"rgb\(\s*([0-9]{1,3})\s*,\s*([0-9]{1,3})\s*,\s*([0-9]{1,3})\s*\)",
        value.strip().lower(),
    )
    if not match:
        return None

    channels = [int(match.group(i)) for i in range(1, 4)]
    if any(channel < 0 or channel > 255 for channel in channels):
        return None
    return "#" + "".join(f"{channel:02x}" for channel in channels)


def parse_color(value: str) -> str | None:
    """Parse a CSS color string into lowercase #rrggbb when possible."""
    return normalize_hex_color(value) or parse_rgb_color(value)


def relative_luminance(hex_color: str) -> float:
    """Compute WCAG relative luminance."""
    red = int(hex_color[1:3], 16) / 255.0
    green = int(hex_color[3:5], 16) / 255.0
    blue = int(hex_color[5:7], 16) / 255.0

    def channel(value: float) -> float:
        if value <= 0.03928:
            return value / 12.92
        return ((value + 0.055) / 1.055) ** 2.4

    return 0.2126 * channel(red) + 0.7152 * channel(green) + 0.0722 * channel(blue)


def contrast_ratio(color_a: str, color_b: str) -> float:
    """Compute WCAG contrast ratio between two #rrggbb colors."""
    lum_a = relative_luminance(color_a)
    lum_b = relative_luminance(color_b)
    lighter = max(lum_a, lum_b)
    darker = min(lum_a, lum_b)
    return (lighter + 0.05) / (darker + 0.05)


def extract_color_variables(block: str) -> dict[str, str]:
    """Extract CSS custom properties in a selector block."""
    properties: dict[str, str] = {}
    for match in re.finditer(r"(--[a-z0-9-]+)\s*:\s*([^;]+);", block):
        properties[match.group(1)] = match.group(2).strip()
    return properties


def resolve_variable_reference(value: str, variables: dict[str, str]) -> str | None:
    """Resolve var(--token) references recursively."""
    current = value.strip()
    for _ in range(MAX_VARIABLE_RESOLUTION_DEPTH):
        match = re.fullmatch(r"var\(\s*(--[a-z0-9-]+)\s*\)", current, re.IGNORECASE)
        if not match:
            return current
        variable = match.group(1).lower()
        mapped_value = variables.get(variable)
        if mapped_value is None:
            return None
        current = mapped_value.strip()
    return None


def normalize_variable_keys(values: dict[str, str]) -> dict[str, str]:
    """Normalize variable dict keys to lowercase for stable lookups."""
    return {key.lower(): value for key, value in values.items()}


def is_forbidden_orange(
    value: str,
    variables: dict[str, str],
) -> bool:
    """Check if a value resolves to the forbidden orange accent palette."""
    lowered = value.strip().lower()
    for variable in ORANGE_VARIABLES:
        if variable in lowered:
            return True

    resolved = resolve_variable_reference(value, variables)
    if resolved is None:
        return False
    normalized_color = parse_color(resolved)
    return normalized_color in ORANGE_HEX_COLORS if normalized_color else False


def parse_required_property(
    *,
    properties: dict[str, str],
    property_name: str,
    css_path: Path,
    errors: int,
    scope: str,
) -> tuple[str | None, int]:
    """Read a required property and emit a standardized error if missing."""
    value = properties.get(property_name)
    if value is None:
        print(
            f"{css_path}:1: error: missing required property {property_name} ({scope})",
            file=sys.stderr,
        )
        return None, errors + 1
    return value, errors


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

    root_variables = normalize_variable_keys(extract_color_variables(extract_css_block(css, ":root")))
    dark_properties = normalize_variable_keys(extract_color_variables(dark_block))
    light_properties = normalize_variable_keys(extract_color_variables(light_block))
    dark_variables = {**root_variables, **dark_properties}
    light_variables = {**root_variables, **light_properties}

    dark_primary, errors = parse_required_property(
        properties=dark_properties,
        property_name="--md-primary-fg-color",
        css_path=css_path,
        errors=errors,
        scope="slate",
    )
    light_primary, errors = parse_required_property(
        properties=light_properties,
        property_name="--md-primary-fg-color",
        css_path=css_path,
        errors=errors,
        scope="default",
    )
    dark_text, errors = parse_required_property(
        properties=dark_properties,
        property_name="--md-primary-bg-color",
        css_path=css_path,
        errors=errors,
        scope="slate",
    )
    light_text, errors = parse_required_property(
        properties=light_properties,
        property_name="--md-primary-bg-color",
        css_path=css_path,
        errors=errors,
        scope="default",
    )

    if errors or dark_primary is None or light_primary is None or dark_text is None or light_text is None:
        return ValidationResult(errors=errors, warnings=warnings)

    for label, value, variables in (
        ("dark primary header background", dark_primary, dark_variables),
        ("light primary header background", light_primary, light_variables),
    ):
        if is_forbidden_orange(value, variables):
            print(
                f"{css_path}:1: error: {label} must not use the orange accent palette ({value})",
                file=sys.stderr,
            )
            errors += 1

    for label, background_value, foreground_value, variables in (
        ("dark header contrast", dark_primary, dark_text, dark_variables),
        ("light header contrast", light_primary, light_text, light_variables),
    ):
        resolved_background = resolve_variable_reference(background_value, variables)
        resolved_foreground = resolve_variable_reference(foreground_value, variables)
        if resolved_background is None or resolved_foreground is None:
            print(
                f"{css_path}:1: error: {label} must resolve to concrete color values",
                file=sys.stderr,
            )
            errors += 1
            continue

        parsed_background = parse_color(resolved_background)
        parsed_foreground = parse_color(resolved_foreground)
        if parsed_background is None or parsed_foreground is None:
            print(
                f"{css_path}:1: error: {label} must use hex or rgb colors (resolved: {resolved_background} vs {resolved_foreground})",
                file=sys.stderr,
            )
            errors += 1
            continue

        ratio = contrast_ratio(parsed_background, parsed_foreground)
        if ratio < 4.5:
            print(
                f"{css_path}:1: error: {label} contrast ratio {ratio:.2f} is below WCAG AA minimum 4.5",
                file=sys.stderr,
            )
            errors += 1

    resolved_dark_primary = resolve_variable_reference(dark_primary, dark_variables)
    resolved_light_primary = resolve_variable_reference(light_primary, light_variables)
    if (
        resolved_dark_primary is not None
        and resolved_light_primary is not None
        and resolved_dark_primary == resolved_light_primary
    ):
        print(
            f"{css_path}:1: error: light/dark --md-primary-fg-color values resolve to the same color; header backgrounds must differ by mode",
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
