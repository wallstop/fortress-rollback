#!/usr/bin/env python3
"""Compare three paired base/candidate Criterion runs on the same machine."""

from __future__ import annotations

import argparse
import json
import math
import statistics
import sys
from pathlib import Path
from typing import Any

EXPECTED_BENCHMARK_IDS = frozenset(
    {
        "Message serialization/input_deserialize",
        "Message serialization/input_encode_into_buffer",
        "Message serialization/input_serialize",
        "Message serialization/round_trip_input_msg",
        "SyncLayer/256_frame_save_advance",
    }
)


class MeasurementError(ValueError):
    """A Criterion measurement set is incomplete or invalid."""


def _load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise MeasurementError(f"cannot read Criterion JSON {path}: {error}") from error


def load_measurements(root: Path) -> dict[str, float]:
    """Load unique, finite, positive median estimates from one Criterion run."""
    estimates_paths = sorted(root.glob("**/new/estimates.json"))
    if not estimates_paths:
        raise MeasurementError(f"no Criterion estimates found under {root}")

    measurements: dict[str, float] = {}
    for estimates_path in estimates_paths:
        benchmark_path = estimates_path.with_name("benchmark.json")
        benchmark = _load_json(benchmark_path)
        estimates = _load_json(estimates_path)
        benchmark_id = benchmark.get("full_id") if isinstance(benchmark, dict) else None
        if not isinstance(benchmark_id, str) or not benchmark_id:
            raise MeasurementError(f"missing full_id in {benchmark_path}")
        if benchmark_id in measurements:
            raise MeasurementError(f"duplicate Criterion measurement {benchmark_id!r}")

        try:
            estimate = estimates["median"]["point_estimate"]
        except (KeyError, TypeError) as error:
            raise MeasurementError(
                f"missing median point_estimate in {estimates_path}"
            ) from error
        if isinstance(estimate, bool) or not isinstance(estimate, (int, float)):
            raise MeasurementError(f"non-numeric measurement for {benchmark_id!r}")
        value = float(estimate)
        if not math.isfinite(value) or value <= 0.0:
            raise MeasurementError(
                f"measurement for {benchmark_id!r} must be finite and positive"
            )
        measurements[benchmark_id] = value
    return measurements


def compare_runs(
    base_roots: list[Path], candidate_roots: list[Path], threshold: float
) -> bool:
    """Return whether all benchmarks stay below the paired regression gate."""
    if len(base_roots) != 3 or len(candidate_roots) != 3:
        raise MeasurementError("exactly three base and three candidate runs are required")
    if not math.isfinite(threshold) or threshold <= 0.0:
        raise MeasurementError("threshold must be finite and positive")

    base_runs = [load_measurements(root) for root in base_roots]
    candidate_runs = [load_measurements(root) for root in candidate_roots]
    for root, run in zip(
        base_roots + candidate_roots, base_runs + candidate_runs
    ):
        actual = set(run)
        if actual != EXPECTED_BENCHMARK_IDS:
            missing = sorted(EXPECTED_BENCHMARK_IDS - actual)
            extra = sorted(actual - EXPECTED_BENCHMARK_IDS)
            raise MeasurementError(
                f"benchmark set mismatch in {root}: missing={missing}, extra={extra}"
            )

    regressions = []
    print("benchmark | median candidate/base | paired ratios | verdict")
    for benchmark_id in sorted(EXPECTED_BENCHMARK_IDS):
        base_values = [run[benchmark_id] for run in base_runs]
        candidate_values = [run[benchmark_id] for run in candidate_runs]
        paired_ratios = [
            candidate / base for base, candidate in zip(base_values, candidate_values)
        ]
        median_ratio = statistics.median(paired_ratios)
        if not math.isfinite(median_ratio) or not all(
            math.isfinite(ratio) for ratio in paired_ratios
        ):
            raise MeasurementError(f"non-finite ratio for {benchmark_id!r}")
        votes = sum(ratio > threshold for ratio in paired_ratios)
        regressed = median_ratio > threshold and votes >= 2
        verdict = "REGRESSION" if regressed else "ok"
        ratios = ", ".join(f"{ratio:.3f}" for ratio in paired_ratios)
        print(f"{benchmark_id} | {median_ratio:.3f} | {ratios} | {verdict}")
        if regressed:
            regressions.append(benchmark_id)

    if regressions:
        print(
            f"regression threshold {threshold:.2f} exceeded by: "
            + ", ".join(regressions),
            file=sys.stderr,
        )
        return False
    return True


def main() -> int:
    """CLI entry point."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", action="append", required=True, type=Path)
    parser.add_argument("--candidate", action="append", required=True, type=Path)
    parser.add_argument("--threshold", type=float, default=1.5)
    args = parser.parse_args()
    try:
        return 0 if compare_runs(args.base, args.candidate, args.threshold) else 1
    except MeasurementError as error:
        print(f"benchmark gate: error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
