#!/usr/bin/env python3
"""Structural contracts for the bounded nightly sometimes-state census."""

from __future__ import annotations

from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ci-simulation-nightly.yml"


def workflow_text() -> str:
    return WORKFLOW.read_text(encoding="utf-8")


def named_step(text: str, name: str) -> str:
    marker = f"      - name: {name}\n"
    start = text.index(marker)
    next_step = text.find("\n      - name: ", start + len(marker))
    return text[start : next_step if next_step != -1 else len(text)]


def test_nightly_workload_and_schedule_remain_fixed() -> None:
    text = workflow_text()
    fleet = named_step(text, "Run sharded fleet and targeted probes")

    assert text.count("cron: '0 4 * * *'") == 1
    assert text.count("workflow_dispatch:") == 1
    assert "timeout-minutes: 60" in text
    assert text.count("cargo nextest run") == 1
    assert "test(simulation::fleet::nightly_seed_shard_)" in fleet
    for test_name in (
        "h_skew_hour_equivalent_measures_lag_correction_and_cost",
        "h_osc_aggregation_pressure_is_measured_and_decays",
        "h_bloat_scale_fragmentation_interaction_is_bounded_and_recovers",
        "h_pollcap_targeted_release_defers_without_starvation",
    ):
        assert fleet.count(test_name) == 1
    assert "--features hot-join" in fleet
    assert "FORTRESS_SIM_CENSUS_DIR: ${{ runner.temp }}/simulation-census" in fleet


def test_merge_is_sequential_and_does_not_replay_fleet_schedules() -> None:
    text = workflow_text()
    prepare = named_step(text, "Prepare sometimes-state census directory")
    fleet = named_step(text, "Run sharded fleet and targeted probes")
    merge = named_step(text, "Merge and validate sometimes-state census")

    assert 'census_dir="$RUNNER_TEMP/simulation-census"' in prepare
    assert 'rm -rf -- "$census_dir"' in prepare
    assert 'mkdir -p "$census_dir"' in prepare
    assert text.index(prepare) < text.index(fleet)
    assert text.index(fleet) < text.index(merge)
    assert "if: always()" in merge
    assert "cargo test" in merge
    assert "simulation::sometimes_state::merge_nightly_sometimes_state_census" in merge
    assert "--ignored" in merge
    assert "--exact" in merge
    assert "--nocapture" in merge
    assert "cargo nextest" not in merge
    assert "nightly_seed_shard" not in merge


def test_census_upload_is_always_retained_and_separate_from_failures() -> None:
    text = workflow_text()
    upload = named_step(text, "Upload sometimes-state census")
    failures = named_step(text, "Upload simulation failure artifacts")

    assert "if: always()" in upload
    assert "uses: actions/upload-artifact@v7" in upload
    assert "path: ${{ runner.temp }}/simulation-census/published/" in upload
    assert "if-no-files-found: error" in upload
    assert "retention-days: 30" in upload
    assert "target/sim-artifacts" not in upload

    assert "if: failure()" in failures
    assert "path: target/sim-artifacts/" in failures
    assert "retention-days: 7" in failures
    assert "simulation-census" not in failures
    assert text.index(upload) > text.index(
        named_step(text, "Merge and validate sometimes-state census")
    )
