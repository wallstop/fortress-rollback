#!/usr/bin/env python3
"""Unit tests for sync-issue-template-versions.py transformations."""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

scripts_dir = Path(__file__).parent.parent
spec = importlib.util.spec_from_file_location(
    "sync_issue_template_versions",
    scripts_dir / "ci" / "sync-issue-template-versions.py",
)
sync_mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sync_mod)

build_version_block = sync_mod.build_version_block
update_template = sync_mod.update_template
BEGIN_SENTINEL = sync_mod.BEGIN_SENTINEL
END_SENTINEL = sync_mod.END_SENTINEL
TEMPLATE_PATH = sync_mod.TEMPLATE_PATH


def _make_template(versions: list[str], indent: str = "        ") -> str:
    """Build a minimal template string with versions between sentinels."""
    block = "\n".join(
        [f"{indent}{BEGIN_SENTINEL}"]
        + [f"{indent}- {v}" for v in versions]
        + [f"{indent}{END_SENTINEL}"]
    )
    return f"options:\n{block}\n"


class TestBuildVersionBlock:
    def test_basic(self) -> None:
        result = build_version_block(["v1.0.0", "v0.9.0"], "  ")
        assert "  # BEGIN_FORTRESS_VERSIONS" in result
        assert "  - v1.0.0" in result
        assert "  - v0.9.0" in result
        assert "  # END_FORTRESS_VERSIONS" in result

    def test_empty_versions(self) -> None:
        result = build_version_block([], "")
        assert BEGIN_SENTINEL in result
        assert END_SENTINEL in result
        lines = result.splitlines()
        assert lines[0] == BEGIN_SENTINEL
        assert lines[-1] == END_SENTINEL
        assert len(lines) == 2

    def test_indent_preserved(self) -> None:
        result = build_version_block(["v1.0.0"], "        ")
        assert result.startswith("        " + BEGIN_SENTINEL)


class TestUpdateTemplate:
    def test_happy_path(self) -> None:
        template = _make_template(["v0.1.0"])
        new_content, changed = update_template(template, ["v1.0.0", "v0.9.0"])
        assert changed
        assert "- v1.0.0" in new_content
        assert "- v0.9.0" in new_content
        assert "- v0.1.0" not in new_content

    def test_already_up_to_date(self) -> None:
        versions = ["v1.0.0", "v0.9.0"]
        template = _make_template(versions)
        new_content, changed = update_template(template, versions)
        assert not changed
        assert new_content == template

    def test_missing_begin_sentinel_raises(self) -> None:
        template = f"options:\n        {END_SENTINEL}\n"
        with pytest.raises(RuntimeError, match="missing.*BEGIN_FORTRESS_VERSIONS"):
            update_template(template, ["v1.0.0"])

    def test_missing_end_sentinel_raises(self) -> None:
        template = f"options:\n        {BEGIN_SENTINEL}\n"
        with pytest.raises(RuntimeError, match="missing.*END_FORTRESS_VERSIONS"):
            update_template(template, ["v1.0.0"])

    def test_sentinels_wrong_order_raises(self) -> None:
        template = (
            f"options:\n"
            f"        {END_SENTINEL}\n"
            f"        {BEGIN_SENTINEL}\n"
        )
        with pytest.raises(RuntimeError, match="must appear before"):
            update_template(template, ["v1.0.0"])

    def test_indentation_preserved(self) -> None:
        indent = "        "
        template = _make_template(["v0.1.0"], indent=indent)
        new_content, changed = update_template(template, ["v2.0.0"])
        assert changed
        lines = new_content.splitlines()
        version_lines = [ln for ln in lines if "- v2.0.0" in ln]
        assert version_lines, "Expected at least one version line"
        assert version_lines[0].startswith(indent)

    def test_sentinel_lines_use_correct_indent(self) -> None:
        indent = "    "
        template = _make_template(["v0.1.0"], indent=indent)
        new_content, _ = update_template(template, ["v1.0.0"])
        assert f"{indent}{BEGIN_SENTINEL}" in new_content
        assert f"{indent}{END_SENTINEL}" in new_content
