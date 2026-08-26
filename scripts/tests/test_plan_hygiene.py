#!/usr/bin/env python3
"""Unit tests for scripts/hooks/check-plan-hygiene.py."""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

scripts_dir = Path(__file__).parent.parent
spec = importlib.util.spec_from_file_location(
    "check_plan_hygiene",
    scripts_dir / "hooks" / "check-plan-hygiene.py",
)
check_plan_hygiene = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = check_plan_hygiene
spec.loader.exec_module(check_plan_hygiene)

MAX_LINES = check_plan_hygiene.MAX_LINES
MAX_WORDS = check_plan_hygiene.MAX_WORDS
check_plan = check_plan_hygiene.check_plan
main = check_plan_hygiene.main


def test_active_plan_passes(tmp_path: Path) -> None:
    plan = tmp_path / "PLAN.md"
    plan.write_text(
        "# Project Plan\n\n## In progress\n\n- [ ] Test the active hypothesis.\n",
        encoding="utf-8",
    )

    assert check_plan(plan) == []


def test_line_budget_rejects_bloated_plan(tmp_path: Path) -> None:
    plan = tmp_path / "PLAN.md"
    plan.write_text("\n".join("line" for _ in range(MAX_LINES + 1)), encoding="utf-8")

    issues = check_plan(plan)

    assert len(issues) == 1
    assert f"has {MAX_LINES + 1} lines" in issues[0]


def test_word_budget_rejects_dense_plan(tmp_path: Path) -> None:
    plan = tmp_path / "PLAN.md"
    plan.write_text("word " * (MAX_WORDS + 1), encoding="utf-8")

    issues = check_plan(plan)

    assert len(issues) == 1
    assert f"has {MAX_WORDS + 1} words" in issues[0]


@pytest.mark.parametrize("marker", ["-", "*", "+", "1.", "2)"])
def test_completed_task_is_rejected(tmp_path: Path, marker: str) -> None:
    plan = tmp_path / "PLAN.md"
    plan.write_text(f"# Plan\n\n{marker} [x] Shipped work\n", encoding="utf-8")

    issues = check_plan(plan)

    assert len(issues) == 1
    assert "completed task belongs in a progress/session record" in issues[0]


@pytest.mark.parametrize(
    "heading",
    ["Background", "Completed Work", "Context", "History", "Research", "Status"],
)
def test_non_queue_section_is_rejected(tmp_path: Path, heading: str) -> None:
    plan = tmp_path / "PLAN.md"
    plan.write_text(f"# Plan\n\n## {heading.title()}\n", encoding="utf-8")

    issues = check_plan(plan)

    assert len(issues) == 1
    assert "is not an active queue section" in issues[0]


def test_missing_plan_fails_closed(tmp_path: Path) -> None:
    issues = check_plan(tmp_path / "PLAN.md")

    assert len(issues) == 1
    assert ":0: cannot read file:" in issues[0]


def test_hook_skips_intentionally_absent_local_plan(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setattr(check_plan_hygiene, "PLAN_PATH", tmp_path / "PLAN.md")

    assert main() == 0
