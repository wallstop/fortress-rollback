#!/usr/bin/env python3
"""Contract tests for the fast PR gate and merge-queue tiering."""

from __future__ import annotations

import json
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
    assert set(gate["needs"]) == {"fast", "api-census", "wasm-check", "semver"}

    step = gate["steps"][0]
    assert step["env"]["NEEDS_JSON"] == "${{ toJSON(needs) }}"
    assert 'all(.[];' in step["run"]
    assert '.result == "success"' in step["run"]
    assert '.result == "skipped"' in step["run"]
    assert '.result == "neutral"' in step["run"]


def test_pr_gate_does_not_run_deep_assurance() -> None:
    text = PR_GATE.read_text(encoding="utf-8")

    for forbidden in (
        "cargo miri",
        "cargo kani",
        "cargo mutants",
        "cargo tarpaulin",
        "TLA+ Model Checking",
        "CodeQL",
        "Godot Emscripten Browser",
    ):
        assert forbidden not in text, forbidden


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
