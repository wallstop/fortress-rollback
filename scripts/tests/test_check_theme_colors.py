#!/usr/bin/env python3
"""Unit tests for check-theme-colors.py."""

from __future__ import annotations

import importlib.util
from pathlib import Path

import pytest

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
:root {
  --fortress-bg-dark-secondary: #161b22;
  --fortress-bg-light-secondary: #f6f8fa;
  --fortress-text-dark: #c9d1d9;
  --fortress-text-light: #24292f;
}
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
:root {
  --fortress-rust-orange: #F74C00;
  --fortress-bg-light-secondary: #f6f8fa;
  --fortress-text-dark: #c9d1d9;
  --fortress-text-light: #24292f;
}
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
    assert result.errors == 2


def test_validate_theme_colors_requires_different_light_dark_primary(tmp_path: Path) -> None:
    css_path = tmp_path / "docs" / "stylesheets" / "custom.css"
    _write(
        css_path,
        """
:root {
  --fortress-bg-dark-secondary: #161b22;
  --fortress-text-dark: #c9d1d9;
  --fortress-text-light: #24292f;
}
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
    assert result.errors == 2


def test_validate_theme_colors_rejects_lowercase_orange_hex(tmp_path: Path) -> None:
    css_path = tmp_path / "docs" / "stylesheets" / "custom.css"
    _write(
        css_path,
        """
[data-md-color-scheme="slate"] {
  --md-primary-fg-color: #f74c00;
  --md-primary-bg-color: #c9d1d9;
}
[data-md-color-scheme="default"] {
  --md-primary-fg-color: #f6f8fa;
  --md-primary-bg-color: #24292f;
}
""",
    )

    result = validate_theme_colors(css_path)
    assert result.errors == 2


def test_validate_theme_colors_rejects_rgb_orange(tmp_path: Path) -> None:
    css_path = tmp_path / "docs" / "stylesheets" / "custom.css"
    _write(
        css_path,
        """
[data-md-color-scheme="slate"] {
  --md-primary-fg-color: rgb(247, 76, 0);
  --md-primary-bg-color: #c9d1d9;
}
[data-md-color-scheme="default"] {
  --md-primary-fg-color: #f6f8fa;
  --md-primary-bg-color: #24292f;
}
""",
    )

    result = validate_theme_colors(css_path)
    assert result.errors == 2


def test_validate_theme_colors_requires_minimum_contrast(tmp_path: Path) -> None:
    css_path = tmp_path / "docs" / "stylesheets" / "custom.css"
    _write(
        css_path,
        """
[data-md-color-scheme="slate"] {
  --md-primary-fg-color: #161b22;
  --md-primary-bg-color: #1f2933;
}
[data-md-color-scheme="default"] {
  --md-primary-fg-color: #f6f8fa;
  --md-primary-bg-color: #24292f;
}
""",
    )

    result = validate_theme_colors(css_path)
    assert result.errors == 1


def test_validate_theme_colors_requires_text_color_properties(tmp_path: Path) -> None:
    css_path = tmp_path / "docs" / "stylesheets" / "custom.css"
    _write(
        css_path,
        """
[data-md-color-scheme="slate"] {
  --md-primary-fg-color: #161b22;
}
[data-md-color-scheme="default"] {
  --md-primary-fg-color: #f6f8fa;
  --md-primary-bg-color: #24292f;
}
""",
    )

    result = validate_theme_colors(css_path)
    assert result.errors == 1


def test_extract_css_block_handles_braces_inside_values() -> None:
    content = """
[data-md-color-scheme="slate"] {
  --md-primary-fg-color: #161b22;
  --note: "{ok}";
  --md-primary-bg-color: #c9d1d9;
}
"""
    block = check_theme_colors.extract_css_block(
        content,
        '[data-md-color-scheme="slate"]',
    )
    assert "--md-primary-bg-color: #c9d1d9;" in block


def test_extract_css_block_ignores_selector_inside_comment() -> None:
    content = """
/* [data-md-color-scheme="slate"] { --md-primary-fg-color: #f74c00; } */
[data-md-color-scheme="slate"] {
  --md-primary-fg-color: #161b22;
  --md-primary-bg-color: #c9d1d9;
}
"""
    block = check_theme_colors.extract_css_block(
        content,
        '[data-md-color-scheme="slate"]',
    )
    assert "--md-primary-fg-color: #161b22;" in block
    assert "#f74c00" not in block


def test_extract_css_block_ignores_commented_selector_before_real_block() -> None:
    content = """
/* [data-md-color-scheme="slate"] {
  --md-primary-fg-color: #f74c00;
  --md-primary-bg-color: #c9d1d9;
} */
[data-md-color-scheme="slate"] {
  --md-primary-fg-color: #161b22;
  --md-primary-bg-color: #c9d1d9;
}
"""
    block = check_theme_colors.extract_css_block(
        content,
        '[data-md-color-scheme="slate"]',
    )
    assert "--md-primary-fg-color: #161b22;" in block
    assert "#f74c00" not in block


def test_extract_css_block_handles_braces_inside_comments() -> None:
    content = """
[data-md-color-scheme="slate"] {
  --md-primary-fg-color: #161b22;
  /* comment with braces { } should not affect depth */
  --md-primary-bg-color: #c9d1d9;
}
"""
    block = check_theme_colors.extract_css_block(
        content,
        '[data-md-color-scheme="slate"]',
    )
    assert "--md-primary-bg-color: #c9d1d9;" in block


def test_extract_color_variables_ignores_commented_properties() -> None:
    block = """
--md-primary-fg-color: #161b22;
/* --md-primary-fg-color: #f74c00; */
--md-primary-bg-color: #c9d1d9;
"""
    properties = check_theme_colors.extract_color_variables(block)
    assert properties["--md-primary-fg-color"] == "#161b22"
    assert properties["--md-primary-bg-color"] == "#c9d1d9"
    assert all("#f74c00" not in value for value in properties.values())


def test_extract_color_variables_ignores_comment_like_text_inside_string() -> None:
    block = """
--note: "/* keep this text */";
--md-primary-fg-color: #161b22;
--md-primary-bg-color: #c9d1d9;
"""
    properties = check_theme_colors.extract_color_variables(block)
    assert properties["--note"] == '"/* keep this text */"'
    assert properties["--md-primary-fg-color"] == "#161b22"


def test_extract_color_variables_preserves_url_with_double_slash() -> None:
    block = """
--banner-url: url(https://example.com/assets/icon.svg);
--md-primary-fg-color: #161b22;
"""
    properties = check_theme_colors.extract_color_variables(block)
    assert properties["--banner-url"] == "url(https://example.com/assets/icon.svg)"
    assert properties["--md-primary-fg-color"] == "#161b22"


def test_extract_color_variables_ignores_double_dash_inside_string_literal() -> None:
    block = """
color: "--fake: red;";
--md-primary-fg-color: #161b22;
"""
    properties = check_theme_colors.extract_color_variables(block)
    assert "--fake" not in properties
    assert properties["--md-primary-fg-color"] == "#161b22"


def test_validate_theme_colors_rejects_resolved_equal_primary_values(
    tmp_path: Path,
) -> None:
    css_path = tmp_path / "docs" / "stylesheets" / "custom.css"
    _write(
        css_path,
        """
:root {
  --bg1: #161b22;
  --bg2: #161b22;
  --text-dark: #c9d1d9;
  --text-light: #f6f8fa;
}
[data-md-color-scheme="slate"] {
  --md-primary-fg-color: var(--bg1);
  --md-primary-bg-color: var(--text-dark);
}
[data-md-color-scheme="default"] {
  --md-primary-fg-color: var(--bg2);
  --md-primary-bg-color: var(--text-light);
}
""",
    )

    result = validate_theme_colors(css_path)
    assert result.errors == 1


def test_validate_theme_colors_rejects_equivalent_primary_hex_variants(
    tmp_path: Path,
) -> None:
    css_path = tmp_path / "docs" / "stylesheets" / "custom.css"
    _write(
        css_path,
        """
:root {
  --bg1: #fff;
  --bg2: #ffffff;
  --text-dark: #161b22;
  --text-light: #24292f;
}
[data-md-color-scheme="slate"] {
  --md-primary-fg-color: var(--bg1);
  --md-primary-bg-color: var(--text-dark);
}
[data-md-color-scheme="default"] {
  --md-primary-fg-color: var(--bg2);
  --md-primary-bg-color: var(--text-light);
}
""",
    )

    result = validate_theme_colors(css_path)
    assert result.errors == 1


def test_validate_theme_colors_rejects_equivalent_primary_rgb_vs_hex(
    tmp_path: Path,
) -> None:
    css_path = tmp_path / "docs" / "stylesheets" / "custom.css"
    _write(
        css_path,
        """
:root {
  --bg1: #161b22;
  --bg2: rgb(22, 27, 34);
  --text-dark: #c9d1d9;
  --text-light: #f6f8fa;
}
[data-md-color-scheme="slate"] {
  --md-primary-fg-color: var(--bg1);
  --md-primary-bg-color: var(--text-dark);
}
[data-md-color-scheme="default"] {
  --md-primary-fg-color: var(--bg2);
  --md-primary-bg-color: var(--text-light);
}
""",
    )

    result = validate_theme_colors(css_path)
    assert result.errors == 1


def test_validate_theme_colors_uses_relative_display_path_and_file_level_line_zero(
    tmp_path: Path,
    capsys,
) -> None:
    css_path = tmp_path / "docs" / "stylesheets" / "custom.css"
    _write(
        css_path,
        """
[data-md-color-scheme="slate"] {
  --md-primary-fg-color: #161b22;
}
[data-md-color-scheme="default"] {
  --md-primary-fg-color: #f6f8fa;
  --md-primary-bg-color: #24292f;
}
""",
    )

    result = validate_theme_colors(css_path)
    assert result.errors == 1
    stderr = capsys.readouterr().err
    assert "custom.css:0: error:" in stderr
    assert ":1: error:" not in stderr


def test_validate_theme_colors_uses_repo_relative_path_when_cwd_is_subdir(
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
    capsys,
) -> None:
    mock_root = tmp_path / "repo"
    scripts_dir = mock_root / "scripts"
    scripts_dir.mkdir(parents=True)
    monkeypatch.chdir(scripts_dir)
    monkeypatch.setattr(check_theme_colors, "get_project_root", lambda: mock_root)
    css_path = mock_root / "docs" / "stylesheets" / "custom.css"
    _write(
        css_path,
        """
[data-md-color-scheme="slate"] {
  --md-primary-fg-color: #161b22;
}
[data-md-color-scheme="default"] {
  --md-primary-fg-color: #f6f8fa;
  --md-primary-bg-color: #24292f;
}
""",
    )

    result = validate_theme_colors(css_path)
    assert result.errors == 1
    stderr = capsys.readouterr().err
    assert "docs/stylesheets/custom.css:0: error:" in stderr


def test_main_outputs_ascii_only_success(
    monkeypatch: pytest.MonkeyPatch,
    capsys,
) -> None:
    monkeypatch.setattr(
        check_theme_colors,
        "validate_theme_colors",
        lambda _: check_theme_colors.ValidationResult(errors=0, warnings=0),
    )

    assert check_theme_colors.main() == 0
    output = capsys.readouterr().out
    assert "Theme color validation passed" in output
    assert "✓" not in output
    assert "✗" not in output


def test_main_outputs_ascii_only_failure(
    monkeypatch: pytest.MonkeyPatch,
    capsys,
) -> None:
    monkeypatch.setattr(
        check_theme_colors,
        "validate_theme_colors",
        lambda _: check_theme_colors.ValidationResult(errors=2, warnings=0),
    )

    assert check_theme_colors.main() == 1
    stderr = capsys.readouterr().err
    assert "Theme color validation failed with 2 error(s)" in stderr
    assert "✓" not in stderr
    assert "✗" not in stderr
