"""Tests for the warmed full-mesh advance_frame absolute budget gate."""

from __future__ import annotations

import json
import subprocess
from pathlib import Path

import yaml

REPO_ROOT = Path(__file__).resolve().parents[2]
CHECKER = REPO_ROOT / "scripts" / "ci" / "check-mesh-advance-budget.py"
WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ci-benchmarks.yml"

N8 = "H-16P advance_frame/warmed_mesh/N=8"
N16 = "H-16P advance_frame/warmed_mesh/N=16"


def _write_measurement(root: Path, benchmark_id: str, median_ns: float) -> None:
    result_dir = root / benchmark_id.replace(" ", "_").replace("/", "_") / "new"
    result_dir.mkdir(parents=True)
    (result_dir / "benchmark.json").write_text(
        json.dumps({"full_id": benchmark_id}), encoding="utf-8"
    )
    (result_dir / "estimates.json").write_text(
        json.dumps({"median": {"point_estimate": median_ns}}), encoding="utf-8"
    )


def _run(root: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["python3", str(CHECKER), str(root)],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )


def test_budget_accepts_warmed_mesh_within_limits(tmp_path: Path) -> None:
    _write_measurement(tmp_path, N8, 33_000.0)
    _write_measurement(tmp_path, N16, 158_000.0)

    result = _run(tmp_path)

    assert result.returncode == 0, result.stdout + result.stderr
    assert "| ok" in result.stdout


def test_budget_rejects_order_of_magnitude_regression(tmp_path: Path) -> None:
    _write_measurement(tmp_path, N8, 33_000.0)
    _write_measurement(tmp_path, N16, 5_000_000.0)

    result = _run(tmp_path)

    assert result.returncode == 1
    assert "warmed_mesh/N=16 | 5000000 ns" in result.stdout
    assert "| over" in result.stdout
    assert "budget exceeded" in result.stderr


def test_budget_requires_both_rows(tmp_path: Path) -> None:
    _write_measurement(tmp_path, N8, 33_000.0)

    result = _run(tmp_path)

    assert result.returncode == 1
    assert "missing measurements" in result.stderr


def test_workflow_runs_the_mesh_budget_gate() -> None:
    workflow = yaml.safe_load(WORKFLOW.read_text(encoding="utf-8"))
    steps = workflow["jobs"]["benchmark"]["steps"]
    commands = "\n".join(
        step.get("run", "") for step in steps if isinstance(step, dict)
    )

    assert "cargo bench --bench h16p_mesh -- 'H-16P advance_frame'" in commands
    assert "scripts/ci/check-mesh-advance-budget.py" in commands
