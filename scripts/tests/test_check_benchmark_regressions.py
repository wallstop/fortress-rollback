"""Tests for the paired Criterion regression gate and workflow contract."""

from __future__ import annotations

import json
import math
import subprocess
from pathlib import Path

import pytest
import yaml

REPO_ROOT = Path(__file__).resolve().parents[2]
CHECKER = REPO_ROOT / "scripts" / "ci" / "check-benchmark-regressions.py"
WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ci-benchmarks.yml"
EXPECTED_BENCHMARKS = (
    "Message serialization/input_deserialize",
    "Message serialization/input_encode_into_buffer",
    "Message serialization/input_serialize",
    "Message serialization/round_trip_input_msg",
    "SyncLayer/256_frame_save_advance",
)


def _write_measurement(root: Path, benchmark_id: str, value: float, slot: str = "one") -> None:
    result_dir = root / slot / "new"
    result_dir.mkdir(parents=True)
    (result_dir / "benchmark.json").write_text(
        json.dumps({"full_id": benchmark_id}), encoding="utf-8"
    )
    (result_dir / "estimates.json").write_text(
        json.dumps({"median": {"point_estimate": value}}), encoding="utf-8"
    )


def _make_runs(
    tmp_path: Path, base: list[float], candidate: list[float]
) -> tuple[list[Path], list[Path]]:
    base_roots = [tmp_path / f"base-{index}" for index in range(3)]
    candidate_roots = [tmp_path / f"candidate-{index}" for index in range(3)]
    for root, value in zip(base_roots, base):
        for index, benchmark_id in enumerate(EXPECTED_BENCHMARKS):
            _write_measurement(root, benchmark_id, value, slot=str(index))
    for root, value in zip(candidate_roots, candidate):
        for index, benchmark_id in enumerate(EXPECTED_BENCHMARKS):
            _write_measurement(root, benchmark_id, value, slot=str(index))
    return base_roots, candidate_roots


def _run(base: list[Path], candidate: list[Path]) -> subprocess.CompletedProcess[str]:
    command = ["python3", str(CHECKER)]
    for root in base:
        command.extend(("--base", str(root)))
    for root in candidate:
        command.extend(("--candidate", str(root)))
    return subprocess.run(command, cwd=REPO_ROOT, capture_output=True, text=True, check=False)


def test_gate_rejects_confirmed_regression(tmp_path: Path) -> None:
    base, head = _make_runs(tmp_path, [100.0, 100.0, 100.0], [160.0, 140.0, 170.0])

    result = _run(base, head)

    assert result.returncode == 1
    assert "input_deserialize | 1.600 | 1.600, 1.400, 1.700 | REGRESSION" in result.stdout


def test_gate_accepts_single_noisy_pair(tmp_path: Path) -> None:
    base, head = _make_runs(tmp_path, [100.0, 100.0, 100.0], [140.0, 160.0, 140.0])

    result = _run(base, head)

    assert result.returncode == 0, result.stdout + result.stderr
    assert "| ok" in result.stdout


def test_gate_accepts_exact_threshold(tmp_path: Path) -> None:
    base, head = _make_runs(tmp_path, [100.0] * 3, [150.0] * 3)

    result = _run(base, head)

    assert result.returncode == 0, result.stdout + result.stderr


def test_gate_uses_median_of_paired_ratios_for_confirmed_regression(
    tmp_path: Path,
) -> None:
    base, head = _make_runs(
        tmp_path, [1.0, 1000.0, 1000.0], [2.0, 2000.0, 1001.0]
    )

    result = _run(base, head)

    assert result.returncode == 1
    assert "| 2.000 | 2.000, 2.000, 1.001 | REGRESSION" in result.stdout


def test_gate_fails_closed_on_missing_measurement(tmp_path: Path) -> None:
    base, head = _make_runs(tmp_path, [100.0] * 3, [100.0] * 3)
    for root in base + head:
        missing = root / "0" / "new"
        for path in missing.iterdir():
            path.unlink()
        missing.rmdir()

    result = _run(base, head)

    assert result.returncode == 2
    assert "benchmark set mismatch" in result.stderr


def test_gate_fails_closed_on_duplicate_measurement(tmp_path: Path) -> None:
    base, head = _make_runs(tmp_path, [100.0] * 3, [100.0] * 3)
    _write_measurement(base[0], EXPECTED_BENCHMARKS[0], 100.0, slot="duplicate")

    result = _run(base, head)

    assert result.returncode == 2
    assert "duplicate Criterion measurement" in result.stderr


@pytest.mark.parametrize("value", [0.0, math.inf, math.nan])
def test_gate_fails_closed_on_invalid_measurement(tmp_path: Path, value: float) -> None:
    base, head = _make_runs(tmp_path, [100.0] * 3, [100.0] * 3)
    estimates = base[0] / "0" / "new" / "estimates.json"
    estimates.write_text(
        json.dumps({"median": {"point_estimate": value}}), encoding="utf-8"
    )

    result = _run(base, head)

    assert result.returncode == 2
    assert "must be finite and positive" in result.stderr


def test_gate_fails_closed_on_missing_full_id(tmp_path: Path) -> None:
    base, head = _make_runs(tmp_path, [100.0] * 3, [100.0] * 3)
    benchmark = base[0] / "0" / "new" / "benchmark.json"
    benchmark.write_text("{}", encoding="utf-8")

    result = _run(base, head)

    assert result.returncode == 2
    assert "missing full_id" in result.stderr


def test_gate_fails_closed_on_malformed_json(tmp_path: Path) -> None:
    base, head = _make_runs(tmp_path, [100.0] * 3, [100.0] * 3)
    estimates = base[0] / "0" / "new" / "estimates.json"
    estimates.write_text("{", encoding="utf-8")

    result = _run(base, head)

    assert result.returncode == 2
    assert "cannot read Criterion JSON" in result.stderr


def _workflow_jobs() -> dict[str, object]:
    workflow = yaml.safe_load(WORKFLOW.read_text(encoding="utf-8"))
    return workflow["jobs"]


def _step(job: dict[str, object], name: str) -> dict[str, object]:
    return next(step for step in job["steps"] if step.get("name") == name)


def test_workflow_uses_read_only_merge_gate_and_separate_write_history() -> None:
    jobs = _workflow_jobs()
    gate = jobs["benchmark"]
    history = jobs["benchmark-history"]

    assert gate["permissions"] == {"contents": "read"}
    assert gate["if"] == "github.event_name == 'pull_request'"
    checkout = gate["steps"][0]
    assert checkout["with"]["persist-credentials"] is False
    assert history["permissions"] == {"contents": "write", "deployments": "write"}
    assert history["if"] == "github.event_name != 'pull_request'"
    for name in ("Track stable benchmark history", "Track informational benchmark results"):
        assert _step(history, name)["with"]["fail-on-alert"] is False


def test_workflow_has_no_dependabot_benchmark_bypass() -> None:
    text = WORKFLOW.read_text(encoding="utf-8")
    jobs = _workflow_jobs()

    assert "dependabot[bot]" not in text
    assert jobs["benchmark-quick"]["if"] == "github.event_name == 'pull_request'"
    assert jobs["benchmark-quick"]["steps"][0]["with"]["persist-credentials"] is False


def test_workflow_compares_base_with_tested_merge_and_guards_harnesses() -> None:
    gate = _workflow_jobs()["benchmark"]
    paired = _step(gate, "Run paired Criterion regression gate")
    run = paired["run"]

    assert paired["env"]["RAW_HEAD_SHA"] == "${{ github.event.pull_request.head.sha }}"
    assert paired["env"]["TESTED_MERGE_SHA"] == "${{ github.sha }}"
    assert 'original_sha="$(git rev-parse HEAD)"' in run
    assert 'echo "RAW_HEAD_SHA=$RAW_HEAD_SHA"' in run
    assert 'echo "TESTED_MERGE_SHA=$TESTED_MERGE_SHA"' in run
    assert 'measure merge-1 "$TESTED_MERGE_SHA"' in run
    assert 'measure merge-1 "$RAW_HEAD_SHA"' not in run
    assert "benches/p2p_session.rs benches/sync_layer.rs" in run
    assert 'git cat-file -e "$BASE_SHA:$harness"' in run
    assert 'git cat-file -e "$TESTED_MERGE_SHA:$harness"' in run
    assert 'git diff --quiet "$BASE_SHA" "$TESTED_MERGE_SHA"' in run


def test_workflow_preserves_ab_ba_ab_order_and_restores_checkout() -> None:
    gate = _workflow_jobs()["benchmark"]
    run = _step(gate, "Run paired Criterion regression gate")["run"]
    ordered_commands = (
        'measure base-1 "$BASE_SHA"',
        'measure merge-1 "$TESTED_MERGE_SHA"',
        'measure merge-2 "$TESTED_MERGE_SHA"',
        'measure base-2 "$BASE_SHA"',
        'measure base-3 "$BASE_SHA"',
        'measure merge-3 "$TESTED_MERGE_SHA"',
    )

    positions = [run.index(command) for command in ordered_commands]
    assert positions == sorted(positions)
    trap = run.index("trap restore_checkout EXIT")
    assert trap < positions[0]
    restore = run.index("restore_checkout\ntrap - EXIT")
    compare = run.index("python3 scripts/ci/check-benchmark-regressions.py")
    assert restore < compare
