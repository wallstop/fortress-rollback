#!/usr/bin/env python3
"""Unit tests for scripts/ci/agent-preflight.py."""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

scripts_dir = Path(__file__).parent.parent
spec = importlib.util.spec_from_file_location(
    "agent_preflight",
    scripts_dir / "ci" / "agent-preflight.py",
)
agent_preflight = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = agent_preflight
spec.loader.exec_module(agent_preflight)

PlannedCheck = agent_preflight.PlannedCheck
PYTHON_EXECUTABLE = agent_preflight.PYTHON_EXECUTABLE
execute_checks = agent_preflight.execute_checks
normalize_paths = agent_preflight.normalize_paths
plan_checks = agent_preflight.plan_checks


def _ids(checks: list[PlannedCheck]) -> list[str]:
    return [check.check_id for check in checks]


CHECK_TRIGGER_CASES: list[tuple[str, str]] = [
    ("sync-version-check", "README.md"),
    ("llm-line-limit", ".llm/context.md"),
    ("llm-skills-quality", ".llm/context.md"),
    ("skills-index-check", ".llm/skills/workflows/dev-pipeline.md"),
    ("actionlint", ".github/workflows/ci.yml"),
    ("changelog-unreleased-rule", "CHANGELOG.md"),
    ("vale-advisory", "docs/index.md"),
    ("kani-violation-cost", "src/lib.rs"),
]


@pytest.mark.parametrize(("expected_check_id", "changed_file"), CHECK_TRIGGER_CASES)
def test_plan_checks_trigger_matrix_includes_expected_check(
    expected_check_id: str,
    changed_file: str,
) -> None:
    checks = plan_checks({changed_file})
    check_ids = _ids(checks)
    assert expected_check_id in check_ids, (
        f"Expected {expected_check_id!r} for changed file {changed_file!r}; "
        f"planned checks were: {check_ids!r}"
    )


def test_plan_checks_runs_sync_version_for_version_surface_files() -> None:
    # README.md is a sync-version surface file but is NOT under docs/, so the
    # vale-advisory check should not trigger -- this isolates the
    # sync-version trigger from the docs/ trigger.
    checks = plan_checks({"README.md"})
    assert _ids(checks) == ["sync-version-check"]


def test_plan_checks_docs_markdown_triggers_both_sync_and_vale() -> None:
    """A docs/*.md file is both a sync-version surface AND a vale target."""
    checks = plan_checks({"docs/index.md"})
    assert _ids(checks) == ["sync-version-check", "vale-advisory"]


def test_plan_checks_runs_llm_checks_for_llm_markdown() -> None:
    checks = plan_checks({".llm/context.md"})
    check_ids = _ids(checks)
    assert "sync-version-check" in check_ids
    assert "llm-line-limit" in check_ids
    assert "llm-skills-quality" in check_ids
    assert "skills-index-check" not in check_ids


def test_plan_checks_runs_skills_index_check_for_skill_files() -> None:
    checks = plan_checks({".llm/skills/workflows/dev-pipeline.md"})
    check_ids = _ids(checks)
    assert "sync-version-check" in check_ids
    assert "llm-line-limit" in check_ids
    assert "llm-skills-quality" in check_ids
    assert "skills-index-check" in check_ids


def test_plan_checks_runs_actionlint_for_workflow_files() -> None:
    checks = plan_checks({".github/workflows/ci.yml"})
    check_ids = _ids(checks)
    assert "sync-version-check" in check_ids
    assert "actionlint" in check_ids

    actionlint_check = next(check for check in checks if check.check_id == "actionlint")
    assert actionlint_check.command == [
        PYTHON_EXECUTABLE,
        "scripts/hooks/actionlint.py",
        ".github/workflows/ci.yml",
    ]


def test_plan_checks_passes_multiple_workflow_files_to_actionlint() -> None:
    checks = plan_checks({".github/workflows/ci.yml", ".github/workflows/lint.yaml"})
    actionlint_check = next(check for check in checks if check.check_id == "actionlint")
    assert actionlint_check.command == [
        PYTHON_EXECUTABLE,
        "scripts/hooks/actionlint.py",
        ".github/workflows/ci.yml",
        ".github/workflows/lint.yaml",
    ]


def test_normalize_paths_preserves_leading_dot_segments() -> None:
    normalized = normalize_paths(
        {
            "./docs/index.md",
            ".llm/context.md",
            ".github/workflows/ci.yml",
        }
    )

    assert "docs/index.md" in normalized
    assert ".llm/context.md" in normalized
    assert ".github/workflows/ci.yml" in normalized


def test_plan_checks_returns_empty_for_non_matching_changes() -> None:
    checks = plan_checks({"notes/design.txtx"})
    assert checks == []


def test_plan_checks_run_all_forces_all_checks() -> None:
    run_all_ids = _ids(plan_checks(set(), run_all=True))
    trigger_files = {changed_file for _, changed_file in CHECK_TRIGGER_CASES}
    matrix_ids = _ids(plan_checks(trigger_files))

    missing_from_run_all = sorted(set(matrix_ids) - set(run_all_ids))
    extra_in_run_all = sorted(set(run_all_ids) - set(matrix_ids))
    assert run_all_ids == matrix_ids, (
        "run_all check selection drifted from matrix-driven expectations. "
        f"missing={missing_from_run_all}, extra={extra_in_run_all}, "
        f"run_all={run_all_ids}, matrix={matrix_ids}"
    )
    assert len(run_all_ids) == len(set(run_all_ids)), (
        "run_all produced duplicate check IDs: "
        f"{run_all_ids}"
    )


def test_plan_checks_runs_changelog_rule_when_changelog_changed() -> None:
    checks = plan_checks({"CHANGELOG.md"})
    check_ids = _ids(checks)
    assert "changelog-unreleased-rule" in check_ids
    assert "sync-version-check" in check_ids  # CHANGELOG.md is also a sync-version surface
    # No auto-fix for the changelog rule (semantic merge).
    rule_check = next(c for c in checks if c.check_id == "changelog-unreleased-rule")
    assert rule_check.fix_command is None
    assert rule_check.fix_hint is not None
    assert "Breaking" in rule_check.fix_hint


def test_plan_checks_skips_changelog_rule_for_unrelated_files() -> None:
    checks = plan_checks({"docs/index.md"})
    assert "changelog-unreleased-rule" not in _ids(checks)


def test_plan_checks_runs_vale_advisory_for_docs_files() -> None:
    checks = plan_checks({"docs/user-guide.md", "docs/migration.md"})
    check_ids = _ids(checks)
    assert "vale-advisory" in check_ids
    vale_check = next(c for c in checks if c.check_id == "vale-advisory")
    # The two docs files must be passed to the wrapper script.
    assert "docs/user-guide.md" in vale_check.command
    assert "docs/migration.md" in vale_check.command
    assert vale_check.command[0] == PYTHON_EXECUTABLE
    assert vale_check.command[1] == "scripts/hooks/agent-vale-advisory.py"


def test_plan_checks_skips_vale_advisory_when_no_docs_files() -> None:
    checks = plan_checks({"src/lib.rs"})
    assert "vale-advisory" not in _ids(checks)


def test_collect_changed_files_merges_all_git_sources(monkeypatch) -> None:
    def fake_git(_repo_root: Path, args: list[str]) -> set[str]:
        if args == ["diff", "--name-only"]:
            return {"unstaged.md"}
        if args == ["diff", "--cached", "--name-only"]:
            return {"staged.md"}
        if args == ["ls-files", "--others", "--exclude-standard"]:
            return {"untracked.md"}
        raise AssertionError(f"Unexpected git args: {args}")

    monkeypatch.setattr(agent_preflight, "git_output_lines", fake_git)

    changed = agent_preflight.collect_changed_files(Path("/repo"))

    assert changed == {"unstaged.md", "staged.md", "untracked.md"}


def test_main_falls_back_to_all_checks_when_git_detection_fails(
    monkeypatch,
    capsys,
) -> None:
    def raise_detection_error(_repo_root: Path) -> set[str]:
        raise RuntimeError("git detection failed")

    observed: dict[str, bool] = {}

    def fake_plan(_changed_files: set[str], run_all: bool = False) -> list[PlannedCheck]:
        observed["run_all"] = run_all
        return []

    monkeypatch.setattr(agent_preflight, "collect_changed_files", raise_detection_error)
    monkeypatch.setattr(agent_preflight, "plan_checks", fake_plan)

    code = agent_preflight.main([])

    assert code == 0
    assert observed == {"run_all": True}
    captured = capsys.readouterr()
    assert "git detection failed" in captured.err
    assert "Falling back to --all checks." in captured.err


def test_execute_checks_auto_fix_retries_and_passes(monkeypatch) -> None:
    check = PlannedCheck(
        check_id="sync-version-check",
        description="version sync",
        command=["bash", "scripts/sync-version.sh", "--check"],
        fix_command=["bash", "scripts/sync-version.sh"],
        fix_hint="run sync",
    )

    calls: list[tuple[str, ...]] = []
    outcomes = {
        tuple(check.command): [1, 0],
        tuple(check.fix_command): [0],
    }

    def fake_run(_repo_root: Path, command: list[str]) -> int:
        key = tuple(command)
        calls.append(key)
        values = outcomes[key]
        return values.pop(0)

    monkeypatch.setattr(agent_preflight, "run_check_command", fake_run)

    code = execute_checks(Path("/repo"), [check], auto_fix=True)

    assert code == 0
    assert calls == [
        tuple(check.command),
        tuple(check.fix_command),
        tuple(check.command),
    ]


def test_execute_checks_auto_fix_retry_failure_returns_error(monkeypatch) -> None:
    check = PlannedCheck(
        check_id="sync-version-check",
        description="version sync",
        command=["bash", "scripts/sync-version.sh", "--check"],
        fix_command=["bash", "scripts/sync-version.sh"],
        fix_hint="run sync",
    )

    outcomes = {
        tuple(check.command): [1, 1],
        tuple(check.fix_command): [0],
    }

    def fake_run(_repo_root: Path, command: list[str]) -> int:
        key = tuple(command)
        values = outcomes[key]
        return values.pop(0)

    monkeypatch.setattr(agent_preflight, "run_check_command", fake_run)

    code = execute_checks(Path("/repo"), [check], auto_fix=True)

    assert code == 1


def test_execute_checks_manual_fix_label_for_non_autofixable_checks(
    monkeypatch,
    capsys,
) -> None:
    check = PlannedCheck(
        check_id="llm-line-limit",
        description="llm limit",
        command=["tool", "check"],
        fix_hint="split long file",
        fix_command=None,
    )

    def fake_run(_repo_root: Path, _command: list[str]) -> int:
        return 1

    monkeypatch.setattr(agent_preflight, "run_check_command", fake_run)

    code = execute_checks(Path("/repo"), [check], auto_fix=True)

    assert code == 1
    captured = capsys.readouterr()
    assert "manual-fix: split long file" in captured.err


def test_execute_checks_without_auto_fix_fails(monkeypatch) -> None:
    check = PlannedCheck(
        check_id="sync-version-check",
        description="version sync",
        command=["bash", "scripts/sync-version.sh", "--check"],
        fix_hint="run sync",
    )

    def fake_run(_repo_root: Path, _command: list[str]) -> int:
        return 1

    monkeypatch.setattr(agent_preflight, "run_check_command", fake_run)

    code = execute_checks(Path("/repo"), [check], auto_fix=False)

    assert code == 1
