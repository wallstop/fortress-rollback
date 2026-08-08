"""Contract tests for blocking static-analysis workflows."""

from __future__ import annotations

from pathlib import Path
import subprocess
from typing import Any

import pytest
import yaml

try:
    import tomllib
except ImportError:  # pragma: no cover - Python < 3.11
    import tomli as tomllib


REPO_ROOT = Path(__file__).resolve().parents[2]
QUALITY_WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ci-quality.yml"
LINT_WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ci-lint.yml"
CODEQL_WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ci-codeql.yml"
DEPENDABOT_CONFIG = REPO_ROOT / ".github" / "dependabot.yml"
RUFF_CONFIG = REPO_ROOT / "ruff.toml"
RUFF_ACTION = "astral-sh/ruff-action@278981a28ce3188b1e39527901f38254bf3aac89"
SHELLCHECK_SHA256 = (
    "8c3be12b05d5c177a04c29e3c78ce89ac86f1595681cab149b65b97c4e227198"
)
CARGO_SHEAR_SHA256 = (
    "21ba04662e0eaa6059ca7e1adf48f3bdf1c6656b613b926f3bda0dc6f5b38f82"
)


def _load_workflow(path: Path) -> dict[str, Any]:
    document = yaml.safe_load(path.read_text(encoding="utf-8"))
    assert isinstance(document, dict)
    return document


def _steps(workflow: dict[str, Any], job_name: str) -> list[dict[str, Any]]:
    steps = workflow["jobs"][job_name]["steps"]
    assert isinstance(steps, list)
    return steps


def _triggers(workflow: dict[str, Any]) -> dict[str, Any]:
    triggers = workflow.get("on", workflow.get(True))
    assert isinstance(triggers, dict)
    return triggers


def _step_by_name(steps: list[dict[str, Any]], name: str) -> dict[str, Any]:
    return next(step for step in steps if step.get("name") == name)


def _codeql_jq_predicate() -> str:
    workflow = _load_workflow(CODEQL_WORKFLOW)
    run = _step_by_name(
        _steps(workflow, "analyze"), "Fail on CodeQL findings"
    )["run"]
    prefix = "jq -s -e '\n"
    suffix = '\n\' "${reports[@]}"'
    assert prefix in run and suffix in run
    return run.split(prefix, 1)[1].split(suffix, 1)[0]


@pytest.mark.parametrize(
    ("job_name", "expected_fragment"),
    [
        ("python-static-analysis", "astral-sh/ruff-action@"),
        ("unused-deps-fast", "cargo shear --locked --deny-warnings"),
    ],
)
def test_quality_analyzers_are_blocking(
    job_name: str,
    expected_fragment: str,
) -> None:
    workflow = _load_workflow(QUALITY_WORKFLOW)
    job = workflow["jobs"][job_name]
    assert job.get("continue-on-error") is not True

    serialized_steps = yaml.safe_dump(job["steps"])
    assert expected_fragment in serialized_steps
    assert "|| true" not in serialized_steps
    assert "or true" not in serialized_steps


def test_every_blocking_static_step_rejects_advisory_bypasses() -> None:
    blocking_jobs = (
        (_load_workflow(LINT_WORKFLOW), "validate-yaml"),
        (_load_workflow(QUALITY_WORKFLOW), "python-static-analysis"),
        (_load_workflow(QUALITY_WORKFLOW), "unused-deps-fast"),
        (_load_workflow(CODEQL_WORKFLOW), "analyze"),
    )
    for workflow, job_name in blocking_jobs:
        job = workflow["jobs"][job_name]
        assert job.get("continue-on-error") is not True
        assert "if" not in job
        for step in _steps(workflow, job_name):
            assert step.get("continue-on-error") is not True
            command = step.get("run", "")
            assert "|| true" not in command
            assert "or true" not in command
            if step.get("name") != "Upload shear report":
                assert "if" not in step


def test_static_workflows_have_read_only_contents_permission() -> None:
    for path in (LINT_WORKFLOW, QUALITY_WORKFLOW):
        workflow = _load_workflow(path)
        assert workflow["permissions"] == {"contents": "read"}


def test_quality_tools_are_immutable_and_version_pinned() -> None:
    workflow = _load_workflow(QUALITY_WORKFLOW)

    ruff = _step_by_name(
        _steps(workflow, "python-static-analysis"), "Check Python scripts"
    )
    assert ruff["uses"] == RUFF_ACTION
    assert ruff["with"]["version"] == "0.16.0"
    assert ruff.get("continue-on-error") is not True

    shear = _step_by_name(
        _steps(workflow, "unused-deps-fast"), "Install cargo-shear"
    )
    shear_install = shear["run"]
    executable_lines = [
        line.strip()
        for line in shear_install.splitlines()
        if line.strip() and not line.lstrip().startswith("#")
    ]
    executable_install = "\n".join(executable_lines)
    assert executable_lines[0] == "set -euo pipefail"
    assert 'cargo_shear_version="1.13.3"' in shear_install
    assert (
        '"https://github.com/Boshen/cargo-shear/releases/download/'
        'v${cargo_shear_version}/cargo-shear-x86_64-unknown-linux-musl.tar.gz"'
        in executable_install
    )
    assert CARGO_SHEAR_SHA256 in executable_install
    checksum_input = f'echo "{CARGO_SHEAR_SHA256}  $cargo_shear_archive" \\'
    checksum_input_index = executable_lines.index(checksum_input)
    assert executable_lines[checksum_input_index + 1] == "| sha256sum --check"
    ordered_install_markers = [
        "curl --fail --location --retry 3",
        "| sha256sum --check",
        'tar -xzf "$cargo_shear_archive"',
        'sudo install -m 0755 "$RUNNER_TEMP/cargo-shear"',
    ]
    marker_positions = [executable_install.index(marker) for marker in ordered_install_markers]
    assert marker_positions == sorted(marker_positions)
    assert shear.get("continue-on-error") is not True
    command = _step_by_name(
        _steps(workflow, "unused-deps-fast"), "Check for unused dependencies"
    )
    assert command.get("continue-on-error") is not True
    assert "scripts/release/workspace_locks.py list" in command["run"]
    assert 'for workspace_root in "${workspace_roots[@]}"' in command["run"]
    assert '"$workspace_root"' in command["run"]


def test_shellcheck_is_warning_denied_and_blocking() -> None:
    workflow = _load_workflow(LINT_WORKFLOW)
    job = workflow["jobs"]["validate-yaml"]
    assert job.get("continue-on-error") is not True

    steps = _steps(workflow, "validate-yaml")
    install = _step_by_name(steps, "Install ShellCheck")
    assert "shellcheck-v${shellcheck_version}.linux.x86_64.tar.xz" in install["run"]
    assert 'shellcheck_version="0.11.0"' in install["run"]
    assert SHELLCHECK_SHA256 in install["run"]
    assert "sha256sum --check" in install["run"]
    assert install.get("continue-on-error") is not True

    check = _step_by_name(steps, "Check shell scripts")
    assert "git ls-files -z -- '*.sh'" in check["run"]
    assert 'shellcheck --severity=warning "${shell_scripts[@]}"' in check["run"]
    assert check.get("continue-on-error") is not True


def test_static_workflow_paths_cover_all_relevant_sources() -> None:
    lint = _triggers(_load_workflow(LINT_WORKFLOW))
    codeql = _triggers(_load_workflow(CODEQL_WORKFLOW))
    quality = _triggers(_load_workflow(QUALITY_WORKFLOW))

    for event in ("push", "pull_request"):
        assert "**/*.sh" in lint[event]["paths"]
        assert "**/*.rs" in codeql[event]["paths"]
        assert "scripts/**/*.py" in quality[event]["paths"]
        assert ".github/**/*.py" in quality[event]["paths"]
        assert {"Cargo.toml", "tests/**", "fuzz/**", "loom-tests/**"}.issubset(
            quality[event]["paths"]
        )


def test_codeql_covers_all_repository_languages_and_fails_on_findings() -> None:
    workflow = _load_workflow(CODEQL_WORKFLOW)
    job = workflow["jobs"]["analyze"]
    assert job.get("continue-on-error") is not True
    assert set(job["strategy"]["matrix"]["language"]) == {
        "actions",
        "python",
        "rust",
    }

    steps = _steps(workflow, "analyze")
    initialize = _step_by_name(steps, "Initialize CodeQL")
    assert initialize["with"]["build-mode"] == "none"
    assert initialize["with"]["queries"] == "security-extended"
    assert initialize.get("continue-on-error") is not True

    analyze = _step_by_name(steps, "Analyze")
    assert analyze["id"] == "codeql"
    assert analyze["with"]["output"] == "codeql-results"
    assert analyze.get("continue-on-error") is not True

    enforcement = _step_by_name(steps, "Fail on CodeQL findings")
    assert enforcement["env"]["SARIF_DIRECTORY"] == (
        "${{ steps.codeql.outputs.sarif-output }}"
    )
    run = enforcement["run"]
    assert "(.results // []) | length" in run
    assert "all(.[];" in run
    assert '.runs | type == "array"' in run
    assert "(.runs | length) > 0" in run
    assert 'type == "object"' in run
    assert '.results | type == "array"' in run
    assert "== 0" in run
    assert enforcement.get("continue-on-error") is not True


@pytest.mark.parametrize(
    ("documents", "should_pass"),
    [
        ((r'{"runs":[{"results":[]}]}',), True),
        ((r'{"runs":[{}]}',), True),
        ((r'{"runs":[{"results":[{"ruleId":"finding"}]}]}',), False),
        ((r'{"runs":[]}',), False),
        (("{malformed",), False),
        (("null",), False),
        ((r'{"runs":{}}',), False),
        ((r'{"runs":[{"results":{}}]}',), False),
        ((r'{"runs":[{"results":null}]}',), False),
        ((r'{"runs":[null]}',), False),
        (
            (
                r'{"runs":[{"results":[]}]}',
                r'{"runs":[]}',
            ),
            False,
        ),
    ],
    ids=(
        "clean",
        "clean-default-results",
        "finding",
        "empty",
        "malformed",
        "null-report",
        "runs-object",
        "results-object",
        "null-results",
        "null-run",
        "mixed",
    ),
)
def test_codeql_jq_predicate(
    tmp_path: Path,
    documents: tuple[str, ...],
    should_pass: bool,
) -> None:
    reports = []
    for index, document in enumerate(documents):
        report = tmp_path / f"report-{index}.sarif"
        report.write_text(document, encoding="utf-8")
        reports.append(report)

    result = subprocess.run(
        ["jq", "-s", "-e", _codeql_jq_predicate(), *map(str, reports)],
        check=False,
        capture_output=True,
        text=True,
    )
    assert (result.returncode == 0) is should_pass, result.stderr


def test_dependabot_tracks_devcontainer_images_and_features() -> None:
    config = _load_workflow(DEPENDABOT_CONFIG)
    entries = [
        update
        for update in config["updates"]
        if update.get("directory") == "/.devcontainer"
    ]
    assert {entry["package-ecosystem"] for entry in entries} == {
        "devcontainers",
        "docker",
    }
    assert all(entry["schedule"]["interval"] == "weekly" for entry in entries)


def test_ruff_subprocess_exceptions_are_scoped() -> None:
    with RUFF_CONFIG.open("rb") as config_file:
        config = tomllib.load(config_file)

    global_ignores = set(config["lint"]["ignore"])
    assert global_ignores.isdisjoint({"S603", "S607"})

    per_file = config["lint"]["per-file-ignores"]
    assert {"S603", "S607"}.issubset(per_file["scripts/tests/**/*.py"])
    for pattern, ignored_rules in per_file.items():
        scoped_rules = {"S603", "S607"}.intersection(ignored_rules)
        if not scoped_rules or pattern.startswith("scripts/tests/"):
            continue
        assert "*" not in pattern
        assert (REPO_ROOT / pattern).is_file()
