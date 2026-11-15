#!/usr/bin/env python3
"""Structural contracts for the nightly fuzz corpus regression."""

from __future__ import annotations

from pathlib import Path

import yaml


REPO_ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ci-fuzz-nightly.yml"


def workflow() -> dict:
    return yaml.safe_load(WORKFLOW.read_text(encoding="utf-8"))


def test_fuzz_targets_build_for_the_pinned_toolchain_host() -> None:
    steps = {
        step.get("name", ""): step
        for step in workflow()["jobs"]["corpus-regression"]["steps"]
    }
    run = steps["Run every fuzz target against its corpus"]["run"]

    assert 'host="$(rustc "+${toolchain}" -vV | sed -n \'s/^host: //p\')"' in run
    assert 'cargo "+${toolchain}" fuzz run "$target" --target "$host" -- -runs=0' in run
    assert 'fuzz run "$target" -- -runs=0' not in run
    assert steps["Install pinned Rust nightly"]["uses"] == (
        "./.github/actions/install-pinned-nightly"
    )
