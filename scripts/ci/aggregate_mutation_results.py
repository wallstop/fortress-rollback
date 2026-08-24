#!/usr/bin/env python3
"""Fail-closed aggregation for sharded cargo-mutants CI results."""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any


class AggregateError(Exception):
    """Raised when mutation artifacts are incomplete or violate policy."""


@dataclass(frozen=True)
class Totals:
    """Classified cargo-mutants outcomes across all shards."""

    caught: int = 0
    missed: int = 0
    timeout: int = 0
    unviable: int = 0

    def __add__(self, other: Totals) -> Totals:
        return Totals(
            caught=self.caught + other.caught,
            missed=self.missed + other.missed,
            timeout=self.timeout + other.timeout,
            unviable=self.unviable + other.unviable,
        )

    @property
    def classified(self) -> int:
        return self.caught + self.missed + self.timeout + self.unviable


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("artifacts", type=Path)
    parser.add_argument("--expected-shards", type=int, required=True)
    parser.add_argument("--expected-mutants", type=int, required=True)
    parser.add_argument("--scope", choices=("diff", "filtered", "full"), required=True)
    parser.add_argument("--minimum-score", type=float, default=80.0)
    parser.add_argument("--summary", type=Path)
    args = parser.parse_args(argv)
    if args.expected_shards < 1:
        parser.error("--expected-shards must be positive")
    if args.expected_mutants < 0:
        parser.error("--expected-mutants cannot be negative")
    if not 0.0 <= args.minimum_score <= 100.0:
        parser.error("--minimum-score must be between 0 and 100")
    return args


def load_object(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as err:
        raise AggregateError(f"cannot read valid JSON from {path}: {err}") from err
    if not isinstance(value, dict):
        raise AggregateError(f"expected a JSON object in {path}")
    return value


def integer_field(data: dict[str, Any], key: str, path: Path) -> int:
    value = data.get(key)
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        raise AggregateError(f"{path}: {key} must be a non-negative integer")
    return value


def shard_totals(outcomes: dict[str, Any], path: Path) -> Totals:
    return Totals(
        caught=integer_field(outcomes, "caught", path),
        missed=integer_field(outcomes, "missed", path),
        timeout=integer_field(outcomes, "timeout", path),
        unviable=integer_field(outcomes, "unviable", path),
    )


def validate_command_status(status: int, totals: Totals, path: Path) -> None:
    if status == 0:
        if totals.missed or totals.timeout:
            raise AggregateError(f"{path}: status 0 contradicts missed/timeout outcomes")
    elif status == 2:
        if totals.missed == 0 or totals.timeout:
            raise AggregateError(f"{path}: status 2 requires missed and no timeout outcomes")
    elif status == 3:
        if totals.timeout == 0:
            raise AggregateError(f"{path}: status 3 requires timeout outcomes")
    else:
        raise AggregateError(f"{path}: cargo-mutants failed with status {status}")


def aggregate(
    artifacts: Path,
    expected_shards: int,
    expected_mutants: int,
    scope: str,
    minimum_score: float,
) -> tuple[Totals, float | None]:
    status_paths = sorted(artifacts.rglob("mutation-status.json"))
    if len(status_paths) != expected_shards:
        raise AggregateError(
            f"expected {expected_shards} shard status files, found {len(status_paths)}"
        )

    seen: set[int] = set()
    selected_total = 0
    totals = Totals()
    for status_path in status_paths:
        status = load_object(status_path)
        shard_index = integer_field(status, "shard_index", status_path)
        shard_count = integer_field(status, "shard_count", status_path)
        selected = integer_field(status, "selected_mutants", status_path)
        command_status = integer_field(status, "command_status", status_path)
        if shard_count != expected_shards:
            raise AggregateError(
                f"{status_path}: shard_count {shard_count} != {expected_shards}"
            )
        if shard_index >= expected_shards or shard_index in seen:
            raise AggregateError(f"{status_path}: duplicate/out-of-range shard {shard_index}")
        seen.add(shard_index)
        selected_total += selected

        outcomes_path = status_path.parent / "mutants-out" / "mutants.out" / "outcomes.json"
        outcomes = load_object(outcomes_path)
        outcome_total = integer_field(outcomes, "total_mutants", outcomes_path)
        current = shard_totals(outcomes, outcomes_path)
        if outcome_total != selected or current.classified != outcome_total:
            raise AggregateError(
                f"{outcomes_path}: selected/classified/total counts disagree "
                f"({selected}/{current.classified}/{outcome_total})"
            )
        validate_command_status(command_status, current, status_path)
        totals += current

    if seen != set(range(expected_shards)):
        raise AggregateError("shard index set is incomplete")
    if selected_total != expected_mutants or totals.classified != expected_mutants:
        raise AggregateError(
            "planned/selected/classified mutant counts disagree "
            f"({expected_mutants}/{selected_total}/{totals.classified})"
        )

    score_denominator = totals.caught + totals.missed
    score = (
        100.0 * totals.caught / score_denominator if score_denominator > 0 else None
    )
    if totals.timeout:
        raise AggregateError(f"scope has {totals.timeout} timed-out mutants")
    if scope in {"diff", "filtered"}:
        if totals.missed:
            raise AggregateError(
                f"{scope} scope has {totals.missed} missed mutants"
            )
    elif expected_mutants > 0:
        if score is None:
            raise AggregateError("full scope produced no viable scored mutants")
        if score < minimum_score:
            raise AggregateError(
                f"full-scope mutation score {score:.2f}% is below {minimum_score:.2f}%"
            )
    return totals, score


def markdown(totals: Totals, score: float | None, scope: str) -> str:
    score_text = "N/A" if score is None else f"{score:.2f}%"
    return "\n".join(
        (
            "## Mutation Testing Summary",
            "",
            f"Scope: `{scope}`",
            "",
            "| Outcome | Count |",
            "| --- | ---: |",
            f"| Caught | {totals.caught} |",
            f"| Missed | {totals.missed} |",
            f"| Timeout | {totals.timeout} |",
            f"| Unviable | {totals.unviable} |",
            f"| Mutation score | {score_text} |",
            "",
        )
    )


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        totals, score = aggregate(
            args.artifacts,
            args.expected_shards,
            args.expected_mutants,
            args.scope,
            args.minimum_score,
        )
        report = markdown(totals, score, args.scope)
        print(report)
        if args.summary is not None:
            args.summary.write_text(report, encoding="utf-8")
    except AggregateError as err:
        print(f"error: {err}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
