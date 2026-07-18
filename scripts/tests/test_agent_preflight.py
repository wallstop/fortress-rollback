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
    ("workspace-lock-check", "Cargo.toml"),
    ("workspace-lock-check", "fuzz/Cargo.lock"),
    ("workspace-lock-check", "scripts/release/prepare_release.py"),
    ("release-automation-tests", "scripts/release/prepare_release.py"),
    ("release-automation-tests", "scripts/tests/test_release_branch.py"),
    ("release-automation-tests", ".github/workflows/publish.yml"),
    ("release-automation-tests", ".github/workflows/ci-release-state.yml"),
    ("ci-toolchain-contract", ".github/actions/install-pinned-nightly/toolchain"),
    ("ci-toolchain-contract", ".github/actions/install-pinned-release/toolchain"),
    ("ci-toolchain-contract", ".github/workflows/ci-rust.yml"),
    ("ci-toolchain-contract", ".github/workflows/ci-security.yml"),
    ("validate-agent-skills", ".agents/skills/fortress-development/SKILL.md"),
    ("agent-skills-quality", ".agents/skills/dev-pipeline/SKILL.md"),
    ("actionlint", ".github/workflows/ci.yml"),
    ("changelog-unreleased-rule", "CHANGELOG.md"),
    ("wire-golden-immutable", "src/network/wire_golden_v1.rs"),
    ("wire-golden-immutable", "tests/network/wire_golden_legacy_0_9.rs"),
    ("vale-advisory", "docs/index.md"),
    ("link-check", "README.md"),
    ("doc-claims", "tests/common/channel_socket.rs"),
    ("advance-frame-error-handling", "tests/sessions/spectator.rs"),
    ("kani-violation-cost", "src/lib.rs"),
    ("tla-config-consistency", "specs/tla/DoubleFailureRelay.cfg"),
    ("tla-config-consistency", "specs/tla/README.md"),
    ("tla-config-consistency", "specs/tla/DoubleFailureRelay.tla"),
    ("tla-config-consistency", "scripts/docs/check-tla-config-consistency.py"),
    ("network-timing-invariants", ".config/nextest.toml"),
    ("network-timing-invariants", ".github/workflows/ci-network-nightly.yml"),
    ("network-timing-invariants", "scripts/hooks/check-network-timing-invariants.py"),
    ("network-timing-invariants", "src/network/protocol/mod.rs"),
    ("network-timing-invariants", "tests/network/multi_process.rs"),
    ("tomllib-fallback", "scripts/hooks/check-toml.py"),
    ("typos", "src/lib.rs"),
    ("typos", "README.md"),
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
    # sync-version trigger from the docs/ trigger. It is a .md file, so the
    # link-check (which gates on .md/.rs) and the typos spell check (which
    # gates on text extensions) also run.
    checks = plan_checks({"README.md"})
    assert _ids(checks) == ["sync-version-check", "link-check", "typos"]


def test_plan_checks_docs_markdown_triggers_both_sync_and_vale() -> None:
    """A docs/*.md file is a sync-version, vale, link-check, AND spell-check surface."""
    checks = plan_checks({"docs/index.md"})
    assert _ids(checks) == [
        "sync-version-check",
        "vale-advisory",
        "link-check",
        "typos",
    ]


def test_plan_checks_python_file_triggers_tomllib_and_typos_only() -> None:
    """A standalone .py change triggers the tomllib-fallback and spell checks.

    `.py` is deliberately NOT a sync-version surface extension, so neither
    sync-version-check nor the Rust/docs checks fire -- this isolates the two
    Python-relevant gates.
    """
    checks = plan_checks({"scripts/hooks/check-toml.py"})
    assert _ids(checks) == ["tomllib-fallback", "typos"]


def test_plan_checks_runs_agent_skill_checks_for_skill_markdown() -> None:
    checks = plan_checks({".agents/skills/fortress-development/SKILL.md"})
    check_ids = _ids(checks)
    assert "sync-version-check" in check_ids
    assert "validate-agent-skills" in check_ids
    assert "agent-skills-quality" in check_ids


def test_plan_checks_runs_actionlint_for_workflow_files() -> None:
    checks = plan_checks({".github/workflows/ci.yml"})
    check_ids = _ids(checks)
    assert "sync-version-check" in check_ids
    assert "actionlint" in check_ids

    actionlint_check = next(check for check in checks if check.check_id == "actionlint")
    assert actionlint_check.command == [
        PYTHON_EXECUTABLE,
        "scripts/hooks/actionlint.py",
    ]


@pytest.mark.parametrize(
    "changed_file",
    ["Cargo.toml", "loom-tests/Cargo.lock", "scripts/release/workspace_locks.py"],
)
def test_plan_checks_runs_full_workspace_lock_check(changed_file: str) -> None:
    checks = plan_checks({changed_file})
    lock_check = next(check for check in checks if check.check_id == "workspace-lock-check")

    assert lock_check.command == [
        PYTHON_EXECUTABLE,
        "scripts/release/workspace_locks.py",
        "check",
    ]
    assert "--no-deps" not in " ".join(lock_check.command)


def test_plan_checks_runs_release_state_machine_regressions() -> None:
    checks = plan_checks({"scripts/release/release_state.py"})
    release_check = next(
        check for check in checks if check.check_id == "release-automation-tests"
    )

    assert release_check.command == [
        PYTHON_EXECUTABLE,
        "-m",
        "pytest",
        "scripts/tests/test_workspace_locks.py",
        "scripts/tests/test_prepare_release.py",
        "scripts/tests/test_release_policy.py",
        "scripts/tests/test_release_state.py",
        "scripts/tests/test_release_state_ci.py",
        "scripts/tests/test_release_checkpoint.py",
        "scripts/tests/test_publish_state.py",
        "scripts/tests/test_release_branch.py",
        "scripts/tests/test_main_ruleset.py",
        "scripts/tests/test_sync_issue_template_versions.py",
        "scripts/tests/test_issue_template_versions_wiring.py",
        "scripts/tests/test_release_workflows.py",
        "--no-header",
        "-q",
    ]


def test_plan_checks_runs_ci_toolchain_contract_for_pin_changes() -> None:
    checks = plan_checks({".github/actions/install-pinned-nightly/toolchain"})
    toolchain_check = next(
        check for check in checks if check.check_id == "ci-toolchain-contract"
    )

    assert toolchain_check.command == [
        PYTHON_EXECUTABLE,
        "-m",
        "pytest",
        "scripts/tests/test_ci_toolchains.py",
        "--no-header",
        "-q",
    ]


def test_plan_checks_runs_network_timing_invariants_for_timing_surfaces() -> None:
    checks = plan_checks(
        {
            ".config/nextest.toml",
            "src/network/protocol/mod.rs",
            "tests/network/multi_process.rs",
        }
    )
    timing_check = next(c for c in checks if c.check_id == "network-timing-invariants")
    assert timing_check.command == [
        PYTHON_EXECUTABLE,
        "scripts/hooks/check-network-timing-invariants.py",
    ]
    assert timing_check.fix_hint is not None


def test_plan_checks_lints_complete_workflow_set_for_multiple_changes() -> None:
    checks = plan_checks({".github/workflows/ci.yml", ".github/workflows/lint.yaml"})
    actionlint_check = next(check for check in checks if check.check_id == "actionlint")
    assert actionlint_check.command == [
        PYTHON_EXECUTABLE,
        "scripts/hooks/actionlint.py",
    ]


def test_plan_checks_does_not_pass_deleted_workflow_path_to_actionlint() -> None:
    checks = plan_checks(
        {".github/workflows/deleted.yml"},
        existing_files=set(),
    )
    actionlint_check = next(check for check in checks if check.check_id == "actionlint")
    assert actionlint_check.command == [
        PYTHON_EXECUTABLE,
        "scripts/hooks/actionlint.py",
    ]


def test_plan_checks_does_not_pass_deleted_python_paths_to_scanner() -> None:
    checks = plan_checks(
        {"scripts/hooks/deleted.py"},
        existing_files=set(),
    )

    assert "tomllib-fallback" not in _ids(checks)
    assert _ids(checks) == ["typos"]


def test_plan_checks_does_not_pass_deleted_rust_paths_to_scanners() -> None:
    checks = plan_checks(
        {"src/deleted.rs"},
        existing_files=set(),
    )

    advance_check = next(
        check for check in checks if check.check_id == "advance-frame-error-handling"
    )
    unbounded_check = next(
        check for check in checks if check.check_id == "unbounded-alloc"
    )
    assert advance_check.command == [
        PYTHON_EXECUTABLE,
        "scripts/hooks/check-advance-frame-error-handling.py",
    ]
    assert unbounded_check.command == [
        PYTHON_EXECUTABLE,
        "scripts/hooks/check-unbounded-alloc.py",
    ]


def test_plan_checks_validates_collection_when_agent_skill_is_deleted() -> None:
    checks = plan_checks(
        {".agents/skills/deleted/SKILL.md"},
        existing_files=set(),
    )

    assert "validate-agent-skills" in _ids(checks)
    assert "agent-skills-quality" in _ids(checks)


def test_normalize_paths_preserves_leading_dot_segments() -> None:
    normalized = normalize_paths(
        {
            "./docs/index.md",
            ".agents/skills/fortress-development/SKILL.md",
            ".github/workflows/ci.yml",
        }
    )

    assert "docs/index.md" in normalized
    assert ".agents/skills/fortress-development/SKILL.md" in normalized
    assert ".github/workflows/ci.yml" in normalized


def test_plan_checks_returns_empty_for_non_matching_changes() -> None:
    checks = plan_checks({"notes/design.txtx"})
    assert checks == []


def test_plan_checks_skips_tla_consistency_for_non_surface_specs_file() -> None:
    # A non-(.tla/.cfg/README) file under specs/tla/ must NOT trigger the check.
    checks = plan_checks({"specs/tla/notes.txt"})
    assert "tla-config-consistency" not in _ids(checks)


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


def test_plan_checks_excludes_slow_interactive_commands() -> None:
    """Agent preflight must stay changed-file-aware and fast by default."""
    trigger_files = {changed_file for _, changed_file in CHECK_TRIGGER_CASES}
    commands = [
        " ".join(check.command)
        for check in plan_checks(trigger_files)
    ]
    combined = "\n".join(commands)

    forbidden_fragments = [
        "cargo clippy",
        "cargo nextest",
        "cargo doc",
        "cargo hack",
    ]
    for fragment in forbidden_fragments:
        assert fragment not in combined


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


def test_plan_checks_wire_golden_rule_checks_worktree_and_index() -> None:
    checks = plan_checks({"src/network/wire_golden_v1.rs"})
    rule = next(check for check in checks if check.check_id == "wire-golden-immutable")
    assert rule.command == [
        PYTHON_EXECUTABLE,
        "scripts/hooks/check-wire-golden-immutable.py",
        "--local",
    ]


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


def test_plan_checks_runs_link_check_for_markdown_files() -> None:
    """A .md change triggers the whole-tree link check."""
    checks = plan_checks({"docs/user-guide.md"})
    link_check = next(c for c in checks if c.check_id == "link-check")
    # check-links.py scans the whole tree, so it takes no per-file arguments.
    assert link_check.command == [
        PYTHON_EXECUTABLE,
        "scripts/docs/check-links.py",
    ]
    # Detection + reporting only; no safe blind auto-fix.
    assert link_check.fix_command is None
    assert link_check.fix_hint is not None
    assert "private_intra_doc_links" in link_check.fix_hint


def test_plan_checks_runs_link_check_for_rust_files() -> None:
    """A .rs change triggers the link check (rustdoc intra-doc links)."""
    checks = plan_checks({"src/lib.rs"})
    assert "link-check" in _ids(checks)


def test_plan_checks_runs_doc_claims_for_rust_files() -> None:
    """A .rs change triggers Rust semantic-claim checks."""
    checks = plan_checks({"src/lib.rs"})
    doc_claims_check = next(c for c in checks if c.check_id == "doc-claims")
    assert doc_claims_check.command == ["bash", "scripts/ci/check-doc-claims.sh"]


@pytest.mark.parametrize(
    "changed_file",
    [
        "scripts/ci/check-doc-claims.sh",
        "scripts/hooks/check-rust-semantic-claims.py",
    ],
)
def test_plan_checks_runs_doc_claims_for_doc_claims_tooling(
    changed_file: str,
) -> None:
    """Semantic-claim tooling changes trigger the check they affect."""
    checks = plan_checks({changed_file})
    check_ids = _ids(checks)
    doc_claims_check = next(c for c in checks if c.check_id == "doc-claims")
    assert doc_claims_check.command == ["bash", "scripts/ci/check-doc-claims.sh"]
    assert "advance-frame-error-handling" not in check_ids


def test_plan_checks_skips_link_check_for_unrelated_files() -> None:
    """Files that are neither .md nor .rs do not trigger the link check."""
    checks = plan_checks({"Cargo.toml"})
    assert "link-check" not in _ids(checks)


def test_plan_checks_passes_rust_files_to_advance_frame_check() -> None:
    checks = plan_checks({"tests/sessions/spectator.rs", "src/lib.rs"})
    advance_check = next(
        check for check in checks if check.check_id == "advance-frame-error-handling"
    )

    assert advance_check.command == [
        PYTHON_EXECUTABLE,
        "scripts/hooks/check-advance-frame-error-handling.py",
        "src/lib.rs",
        "tests/sessions/spectator.rs",
    ]


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

    def fake_plan(
        _changed_files: set[str],
        run_all: bool = False,
        existing_files: set[str] | None = None,
    ) -> list[PlannedCheck]:
        observed["run_all"] = run_all
        assert existing_files == set()
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
        check_id="validate-agent-skills",
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
