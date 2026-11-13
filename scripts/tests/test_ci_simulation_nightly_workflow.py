#!/usr/bin/env python3
"""Structural contracts for the bounded nightly sometimes-state census."""

from __future__ import annotations

from pathlib import Path

import yaml


REPO_ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ci-simulation-nightly.yml"


def workflow_text() -> str:
    return WORKFLOW.read_text(encoding="utf-8")


def workflow() -> dict:
    return yaml.safe_load(workflow_text())


def job_steps(job: dict) -> dict[str, dict]:
    return {step.get("name", ""): step for step in job["steps"]}


def named_step(text: str, name: str) -> str:
    marker = f"      - name: {name}\n"
    start = text.index(marker)
    next_step = text.find("\n      - name: ", start + len(marker))
    return text[start : next_step if next_step != -1 else len(text)]


def test_nightly_workload_and_schedule_remain_fixed() -> None:
    data = workflow()
    fleet_step = job_steps(data["jobs"]["simulation-nightly"])[
        "Run sharded fleet and targeted probes"
    ]
    fleet = fleet_step["run"]
    run_commands = [
        step.get("run", "")
        for job in data["jobs"].values()
        for step in job["steps"]
        if isinstance(step.get("run", ""), str)
    ]

    assert workflow_text().count("cron: '0 4 * * *'") == 1
    assert workflow_text().count("workflow_dispatch:") == 1
    assert data["jobs"]["simulation-nightly"]["timeout-minutes"] == 60
    assert sum(command.count("cargo nextest run") for command in run_commands) == 2
    assert "test(simulation::fleet::nightly_seed_shard_)" in fleet
    for test_name in (
        "h_skew_hour_equivalent_measures_lag_correction_and_cost",
        "h_osc_aggregation_pressure_is_measured_and_decays",
        "h_bloat_scale_fragmentation_interaction_is_bounded_and_recovers",
        "h_pollcap_targeted_release_defers_without_starvation",
    ):
        assert fleet.count(test_name) == 1
    assert "--features hot-join" in fleet
    assert fleet_step["env"]["FORTRESS_SIM_CENSUS_DIR"] == (
        "${{ runner.temp }}/simulation-census"
    )


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


def test_violation_site_census_joins_all_exact_sources() -> None:
    data = workflow()
    jobs = data["jobs"]
    exact = job_steps(jobs["violation-site-observations"])
    census = job_steps(jobs["violation-site-census"])
    upload = job_steps(jobs["simulation-nightly"])["Upload violation-site observations"]

    assert data["permissions"]["actions"] == "read"
    assert upload["with"]["name"] == ("simulation-violation-sites-${{ github.run_id }}")
    assert upload["with"]["path"] == (
        "${{ runner.temp }}/simulation-census/violation-sites/"
    )

    run_step = exact["Run the full suite with the census hook"]
    assert run_step["run"] == (
        "cargo nextest run --profile ci --features tokio,json,hot-join,violation-census"
    )
    assert run_step["env"]["FORTRESS_VIOLATION_CENSUS_DIR"] == (
        "${{ runner.temp }}/violation-observations"
    )
    assert run_step["env"]["FORTRESS_VIOLATION_CENSUS_RUN_ID"] == (
        "${{ github.run_id }}"
    )
    exact_upload = exact["Upload exact observations"]
    assert exact_upload["if"] == "always()"
    assert exact_upload["with"] == {
        "name": "violation-site-observations-${{ github.run_id }}",
        "path": "${{ runner.temp }}/violation-observations/",
        "if-no-files-found": "error",
        "retention-days": 30,
    }

    census_job = jobs["violation-site-census"]
    assert census_job["needs"] == [
        "simulation-nightly",
        "violation-site-observations",
    ]
    assert census_job["if"] == "always()"
    download = census["Download exact test-suite observations"]
    assert download["continue-on-error"] is True
    assert download["with"] == {
        "name": "violation-site-observations-${{ github.run_id }}",
        "path": "${{ runner.temp }}/violation-observations",
    }
    fleet_download = census["Download fleet observations"]
    assert fleet_download["continue-on-error"] is True
    assert fleet_download["with"] == {
        "name": "simulation-violation-sites-${{ github.run_id }}",
        "path": "${{ runner.temp }}/violation-sites",
    }
    report_step = census["Generate violation-site exercise report"]
    report = report_step["run"]
    assert (
        "gh run list --workflow=ci-coverage.yml"
        in census["Download latest main test coverage (best effort)"]["run"]
    )
    assert (
        "gh run download"
        in census["Download latest main test coverage (best effort)"]["run"]
    )
    assert "scripts/simulation/violation_census.py" in report
    assert "--fleet-dir" in report
    assert "--observations-dir" in report
    assert report.count("-name '*.json'") == 2
    assert "--partial" in report
    assert '--expected-run-id "${{ github.run_id }}"' in report
    assert report_step["env"]["CENSUS_PARTIAL"] == (
        "${{ needs.simulation-nightly.result != 'success' || "
        "needs.violation-site-observations.result != 'success' }}"
    )
    assert "--coverage-xml" in report
    assert census_job["steps"][-1]["with"]["name"] == (
        "violation-site-census-${{ github.run_id }}"
    )


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
