#!/usr/bin/env python3
"""Contracts for fail-closed mutation-result aggregation."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[2]
AGGREGATOR = REPO_ROOT / "scripts" / "ci" / "aggregate_mutation_results.py"


def write_shard(
    root: Path,
    index: int,
    shard_count: int,
    *,
    caught: int = 0,
    missed: int = 0,
    timeout: int = 0,
    unviable: int = 0,
    command_status: int | None = None,
) -> None:
    selected = caught + missed + timeout + unviable
    if command_status is None:
        command_status = 3 if timeout else (2 if missed else 0)
    shard = root / f"mutation-report-{index}-of-{shard_count}"
    outcomes = shard / "mutants-out" / "mutants.out"
    outcomes.mkdir(parents=True)
    (shard / "mutation-status.json").write_text(
        json.dumps(
            {
                "shard_index": index,
                "shard_count": shard_count,
                "selected_mutants": selected,
                "command_status": command_status,
            }
        ),
        encoding="utf-8",
    )
    (outcomes / "outcomes.json").write_text(
        json.dumps(
            {
                "total_mutants": selected,
                "caught": caught,
                "missed": missed,
                "timeout": timeout,
                "unviable": unviable,
            }
        ),
        encoding="utf-8",
    )


def run_aggregator(
    artifacts: Path, expected_shards: int, expected_mutants: int, scope: str
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            sys.executable,
            str(AGGREGATOR),
            str(artifacts),
            "--expected-shards",
            str(expected_shards),
            "--expected-mutants",
            str(expected_mutants),
            "--scope",
            scope,
        ],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        timeout=30,
        check=False,
    )


def test_complete_diff_without_survivors_passes(tmp_path: Path) -> None:
    write_shard(tmp_path, 0, 2, caught=3, unviable=1)
    write_shard(tmp_path, 1, 2, caught=2)

    result = run_aggregator(tmp_path, 2, 6, "diff")

    assert result.returncode == 0, result.stderr
    assert "| Caught | 5 |" in result.stdout
    assert "100.00%" in result.stdout


@pytest.mark.parametrize(
    ("outcomes", "message"),
    [
        ({"caught": 1, "missed": 1}, "missed"),
        ({"caught": 1, "timeout": 1}, "timed-out"),
    ],
)
def test_diff_survivor_or_timeout_fails(
    tmp_path: Path, outcomes: dict[str, int], message: str
) -> None:
    write_shard(tmp_path, 0, 1, **outcomes)

    result = run_aggregator(tmp_path, 1, 2, "diff")

    assert result.returncode != 0
    assert message in result.stderr


def test_full_scope_enforces_global_score_across_shards(tmp_path: Path) -> None:
    write_shard(tmp_path, 0, 2, caught=4, missed=1)
    write_shard(tmp_path, 1, 2, caught=3, missed=2)

    result = run_aggregator(tmp_path, 2, 10, "full")

    assert result.returncode != 0
    assert "70.00% is below 80.00%" in result.stderr


def test_full_scope_rejects_timeout_even_with_passing_score(tmp_path: Path) -> None:
    write_shard(tmp_path, 0, 1, caught=9, timeout=1)

    result = run_aggregator(tmp_path, 1, 10, "full")

    assert result.returncode != 0
    assert "1 timed-out mutants" in result.stderr


def test_missing_shard_fails(tmp_path: Path) -> None:
    write_shard(tmp_path, 0, 2, caught=1)

    result = run_aggregator(tmp_path, 2, 1, "full")

    assert result.returncode != 0
    assert "expected 2 shard status files, found 1" in result.stderr


def test_unexpected_cargo_mutants_status_fails(tmp_path: Path) -> None:
    write_shard(tmp_path, 0, 1, caught=1, command_status=70)

    result = run_aggregator(tmp_path, 1, 1, "full")

    assert result.returncode != 0
    assert "failed with status 70" in result.stderr


def test_count_mismatch_fails(tmp_path: Path) -> None:
    write_shard(tmp_path, 0, 1, caught=1)

    result = run_aggregator(tmp_path, 1, 2, "full")

    assert result.returncode != 0
    assert "planned/selected/classified mutant counts disagree" in result.stderr
