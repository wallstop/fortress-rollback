#!/usr/bin/env python3
"""Contract tests for the fast PR gate and merge-queue tiering."""

from __future__ import annotations

import json
import os
import re
import subprocess
from pathlib import Path

import pytest
import yaml

REPO_ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = REPO_ROOT / ".github" / "workflows"
PR_GATE = WORKFLOWS / "ci-pr.yml"
RULESET = REPO_ROOT / ".github" / "rulesets" / "main-protection.json"
KANI_CHANGE_SCRIPT = REPO_ROOT / "scripts" / "ci" / "detect-kani-changes.sh"

DEEP_WORKFLOWS = (
    "ci-rust.yml",
    "ci-verification.yml",
    "ci-safety.yml",
    "ci-mutation.yml",
    "ci-coverage.yml",
    "ci-benchmarks.yml",
    "ci-codeql.yml",
)


def _load(path: Path) -> dict:
    return yaml.safe_load(path.read_text(encoding="utf-8"))


def _triggers(workflow: dict) -> dict:
    on_block = workflow.get("on")
    if on_block is None:
        on_block = workflow.get(True)
    assert isinstance(on_block, dict)
    return on_block


def _git(repo: Path, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", *args],
        cwd=repo,
        check=True,
        capture_output=True,
        text=True,
    )


def _run_kani_change_classifier(
    tmp_path: Path, changed_paths: tuple[str, ...], base_sha: str | None = None
) -> str:
    """Run the real classifier script against a scratch repo and base branch."""
    remote = tmp_path / "remote.git"
    repo = tmp_path / "repo"
    subprocess.run(["git", "init", "--bare", str(remote)], check=True, capture_output=True)
    subprocess.run(["git", "init", str(repo)], check=True, capture_output=True)
    _git(repo, "config", "user.email", "kani-gate@example.invalid")
    _git(repo, "config", "user.name", "Kani Gate Test")
    (repo / "README.md").write_text("base\n", encoding="utf-8")
    _git(repo, "add", "-A")
    _git(repo, "commit", "-m", "base")
    base = _git(repo, "rev-parse", "HEAD").stdout.strip()
    _git(repo, "remote", "add", "origin", str(remote))
    _git(repo, "push", "-q", "origin", "HEAD:refs/heads/main")

    for relative in changed_paths:
        path = repo / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("change\n", encoding="utf-8")
    _git(repo, "add", "-A")
    _git(repo, "commit", "-m", "change")

    output = tmp_path / "github-output"
    output.write_text("", encoding="utf-8")
    env = os.environ.copy()
    env["BASE_SHA"] = base if base_sha is None else base_sha
    env["GITHUB_OUTPUT"] = str(output)
    result = subprocess.run(
        ["bash", str(KANI_CHANGE_SCRIPT)],
        cwd=repo,
        env=env,
        check=True,
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0
    lines = [line for line in output.read_text(encoding="utf-8").splitlines()]
    assert len(lines) == 1, lines
    key, _, value = lines[0].partition("=")
    assert key == "kani"
    return value


def test_pr_gate_runs_on_pull_requests_and_merge_groups() -> None:
    triggers = _triggers(_load(PR_GATE))
    assert "pull_request" in triggers
    assert "merge_group" in triggers


def test_pr_gate_has_one_aggregate_required_job() -> None:
    workflow = _load(PR_GATE)
    jobs = workflow["jobs"]
    gate = jobs["gate"]

    assert gate["name"] == "CI Gate"
    assert gate["if"] == "always()"
    assert set(gate["needs"]) == {
        "changes",
        "fast",
        "api-census",
        "wasm-check",
        "kani-compile",
    }

    step = gate["steps"][0]
    assert step["env"]["NEEDS_JSON"] == "${{ toJSON(needs) }}"
    assert 'all(.[];' in step["run"]
    assert '.result == "success"' in step["run"]
    assert '.result == "skipped"' in step["run"]
    assert '.result == "neutral"' in step["run"]


def test_pr_gate_compiles_the_cfg_kani_tree_without_running_proofs() -> None:
    jobs = _load(PR_GATE)["jobs"]
    compile_job = jobs["kani-compile"]

    assert compile_job["needs"] == "changes"
    assert compile_job["if"] == "needs.changes.outputs.kani == 'true'"
    assert "continue-on-error" not in compile_job

    uses = [step.get("uses", "") for step in compile_job["steps"]]
    runs = [step.get("run", "") for step in compile_job["steps"]]
    assert "./.github/actions/setup-kani" in uses
    assert any(run.strip() == "cargo kani --only-codegen" for run in runs)

    # The type-check must be able to fail the job: no `|| true` / `; true`
    # short circuit and no step-level continue-on-error.
    for step in compile_job["steps"]:
        assert "continue-on-error" not in step
        assert "||" not in step.get("run", "")
        assert "; true" not in step.get("run", "")


def test_pr_gate_compiles_the_browser_runtime_test_crate() -> None:
    jobs = _load(PR_GATE)["jobs"]
    runs = [step.get("run", "").strip() for step in jobs["wasm-check"]["steps"]]

    assert (
        "cargo check -p wasm-browser-smoke --target wasm32-unknown-unknown --lib --tests"
        in runs
    ), "the PR gate must compile the browser runtime tests it relies on"


def test_kani_compile_gate_path_detection_fails_closed() -> None:
    jobs = _load(PR_GATE)["jobs"]
    changes = jobs["changes"]

    assert changes["outputs"]["kani"] == "${{ steps.diff.outputs.kani }}"
    diff_step = next(step for step in changes["steps"] if step.get("id") == "diff")
    assert diff_step["run"] == "bash scripts/ci/detect-kani-changes.sh"
    assert "BASE_SHA" in diff_step["env"]


@pytest.mark.parametrize(
    ("changed_paths", "expected"),
    (
        (("src/lib.rs",), "true"),
        (("src/network/protocol/input_bytes.rs",), "true"),
        (("docs/user-guide.md",), "false"),
        (("Cargo.toml",), "true"),
        (("tests/network-peer/Cargo.toml",), "true"),
        ((".github/actions/install-pinned-nightly/action.yml",), "true"),
        ((".github/actions/install-pinned-nightly/toolchain",), "true"),
        ((".github/actions/setup-kani/action.yml",), "true"),
        ((".kani-version",), "true"),
    ),
)
def test_kani_change_classifier_matches_rust_and_build_inputs(
    tmp_path: Path, changed_paths: tuple[str, ...], expected: str
) -> None:
    assert _run_kani_change_classifier(tmp_path, changed_paths) == expected


def test_kani_change_classifier_fails_closed(tmp_path: Path) -> None:
    assert (
        _run_kani_change_classifier(tmp_path / "empty-base", ("docs/x.md",), base_sha="")
        == "true"
    )
    assert (
        _run_kani_change_classifier(
            tmp_path / "bad-base", ("docs/x.md",), base_sha="0" * 40
        )
        == "true"
    )


def test_pr_gate_does_not_run_deep_assurance() -> None:
    text = PR_GATE.read_text(encoding="utf-8")

    for forbidden in (
        "cargo miri",
        "cargo mutants",
        "cargo tarpaulin",
        "TLA+ Model Checking",
        "CodeQL",
        "Godot Emscripten Browser",
        "--harness",
        "--tier",
        "--fail-fast",
    ):
        assert forbidden not in text, forbidden


def test_kani_version_has_one_authority() -> None:
    pin = (REPO_ROOT / ".kani-version").read_text(encoding="utf-8").strip()
    assert re.fullmatch(r"\d+\.\d+\.\d+", pin), pin

    setup = (REPO_ROOT / ".github" / "actions" / "setup-kani" / "action.yml").read_text(
        encoding="utf-8"
    )
    assert ".kani-version" in setup
    assert pin not in setup, "the composite action must not hardcode the version"
    assert setup.count("steps.pin.outputs.version") >= 4
    assert (
        'cargo install --locked kani-verifier --version "=${{ steps.pin.outputs.version }}"'
        in setup
    )
    install_step = next(
        step
        for step in yaml.safe_load(setup)["runs"]["steps"]
        if step.get("name") == "Install Kani with retry"
    )
    install_command = install_step["with"]["command"]
    # Setup must run on cache hits too (the restored binary's toolchain symlink
    # points into ~/.rustup, which the cache does not cover).
    assert "cargo kani setup" in install_command
    assert install_command.index("cargo kani setup") > install_command.index("fi")

    for workflow in WORKFLOWS.glob("*.yml"):
        text = workflow.read_text(encoding="utf-8")
        assert "KANI_VERSION" not in text, workflow.name
        assert pin not in text, f"{workflow.name} hardcodes the Kani version"


def test_deep_workflows_are_merge_queue_or_scheduled_only() -> None:
    for name in DEEP_WORKFLOWS:
        triggers = _triggers(_load(WORKFLOWS / name))
        assert "pull_request" not in triggers, f"{name} must not gate pull requests"
        if name == "ci-codeql.yml":
            assert "schedule" in triggers
        else:
            assert "merge_group" in triggers, f"{name} must validate merge queues"


def test_main_ruleset_requires_the_aggregate_gate() -> None:
    config = json.loads(RULESET.read_text(encoding="utf-8"))
    status_rule = next(
        rule for rule in config["rules"] if rule["type"] == "required_status_checks"
    )
    contexts = {
        check["context"]
        for check in status_rule["parameters"]["required_status_checks"]
    }
    assert contexts == {"Verify prepared release state", "CI Gate"}
