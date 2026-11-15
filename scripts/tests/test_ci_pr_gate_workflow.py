#!/usr/bin/env python3
"""Contract tests for the fast PR gate and merge-queue tiering."""

from __future__ import annotations

import json
import re
from pathlib import Path

import yaml

REPO_ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = REPO_ROOT / ".github" / "workflows"
PR_GATE = WORKFLOWS / "ci-pr.yml"
RULESET = REPO_ROOT / ".github" / "rulesets" / "main-protection.json"

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

    uses = [step.get("uses", "") for step in compile_job["steps"]]
    runs = [step.get("run", "") for step in compile_job["steps"]]
    assert "./.github/actions/setup-kani" in uses
    assert any("cargo kani --only-codegen" in run for run in runs)

    # Compile-only: proof execution flags must never appear in this job.
    for forbidden in ("--harness", "--tier", "--fail-fast"):
        assert forbidden not in yaml.safe_dump(compile_job), forbidden


def test_kani_compile_gate_path_detection_fails_closed() -> None:
    jobs = _load(PR_GATE)["jobs"]
    changes = jobs["changes"]

    assert changes["outputs"]["kani"] == "${{ steps.diff.outputs.kani }}"
    diff_step = next(step for step in changes["steps"] if step.get("id") == "diff")
    assert "BASE_SHA" in diff_step["env"]
    for path in (
        "'src/**'",
        "'Cargo.toml'",
        "'Cargo.lock'",
        "'.kani-version'",
        "'.github/actions/setup-kani/**'",
    ):
        assert path in diff_step["run"], path
    # Detection failures must fail closed: a fetch error runs the gate.
    assert 'echo "kani=true" >> "$GITHUB_OUTPUT"' in diff_step["run"]


def test_pr_gate_does_not_run_deep_assurance() -> None:
    text = PR_GATE.read_text(encoding="utf-8")

    for forbidden in (
        "cargo miri",
        "cargo mutants",
        "cargo tarpaulin",
        "TLA+ Model Checking",
        "CodeQL",
        "Godot Emscripten Browser",
    ):
        assert forbidden not in text, forbidden


def test_kani_version_has_one_authority() -> None:
    pin = (REPO_ROOT / ".kani-version").read_text(encoding="utf-8").strip()
    assert re.fullmatch(r"\d+\.\d+\.\d+", pin), pin

    setup = (REPO_ROOT / ".github" / "actions" / "setup-kani" / "action.yml").read_text(
        encoding="utf-8"
    )
    assert ".kani-version" in setup
    assert "kani-build-${{ runner.os }}" in setup

    verification = (WORKFLOWS / "ci-verification.yml").read_text(encoding="utf-8")
    assert "KANI_VERSION" not in verification
    assert "./.github/actions/setup-kani" in verification
    assert pin not in verification, "Kani version must live only in .kani-version"


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
