#!/usr/bin/env python3
"""Enforce the warmed N>=8 full-mesh ``advance_frame`` absolute budgets.

Issue #443 measured the per-frame mesh folds that the allocation contracts
cannot observe (they allocate nothing): ``update_player_disconnects`` and
``max_frame_advantage`` scan the player x remote matrix. The Criterion row in
``benches/h16p_mesh.rs`` reports one full mesh pass; this gate fails an
order-of-magnitude regression on the shared CI runner.

Budgets are nanosecond medians with roughly 10x headroom over the measured
values (N=8 ~33 us, N=16 ~158 us locally). They are guardrails, not
hardware-independent performance claims.
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

BUDGETS_NS = {
    "H-16P advance_frame/warmed_mesh/N=8": 300_000.0,
    "H-16P advance_frame/warmed_mesh/N=16": 1_500_000.0,
}


def load_medians(root: Path) -> dict[str, float]:
    """Return ``full_id -> median ns`` for every Criterion benchmark under root."""
    medians: dict[str, float] = {}
    for benchmark_path in sorted(root.glob("**/new/benchmark.json")):
        benchmark = json.loads(benchmark_path.read_text(encoding="utf-8"))
        benchmark_id = benchmark.get("full_id")
        if not isinstance(benchmark_id, str) or not benchmark_id:
            continue
        estimates = json.loads(
            benchmark_path.with_name("estimates.json").read_text(encoding="utf-8")
        )
        median = estimates["median"]["point_estimate"]
        if isinstance(median, bool) or not isinstance(median, (int, float)):
            raise ValueError(f"non-numeric median for {benchmark_id!r}")
        value = float(median)
        if not math.isfinite(value) or value <= 0.0:
            raise ValueError(f"invalid median for {benchmark_id!r}: {value}")
        medians[benchmark_id] = value
    return medians


def check_budgets(root: Path) -> bool:
    """Print each warmed-mesh median and return whether all are within budget."""
    medians = load_medians(root)
    missing = sorted(set(BUDGETS_NS) - set(medians))
    if missing:
        print(
            f"mesh benchmark budget: missing measurements: {', '.join(missing)}",
            file=sys.stderr,
        )
        return False
    over: list[str] = []
    for benchmark_id, budget in BUDGETS_NS.items():
        measured = medians[benchmark_id]
        verdict = "over" if measured > budget else "ok"
        print(f"{benchmark_id} | {measured:.0f} ns | budget {budget:.0f} ns | {verdict}")
        if measured > budget:
            over.append(benchmark_id)
    if over:
        print(
            "mesh benchmark budget exceeded by: " + ", ".join(over),
            file=sys.stderr,
        )
        return False
    return True


def main() -> int:
    """CLI entry point."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "root",
        nargs="?",
        type=Path,
        default=Path("target/criterion"),
        help="Criterion output root (default: target/criterion)",
    )
    args = parser.parse_args()
    try:
        return 0 if check_budgets(args.root) else 1
    except (OSError, UnicodeError, json.JSONDecodeError, KeyError, ValueError) as error:
        print(f"mesh benchmark budget: error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
