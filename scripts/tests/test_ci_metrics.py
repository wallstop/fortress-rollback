#!/usr/bin/env python3
"""Unit tests for the CI timing listener."""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

scripts_dir = Path(__file__).parent.parent
SPEC = importlib.util.spec_from_file_location(
    "ci_metrics", scripts_dir / "ci" / "ci_metrics.py"
)
assert SPEC is not None and SPEC.loader is not None
ci_metrics = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = ci_metrics
SPEC.loader.exec_module(ci_metrics)


def test_run_duration_uses_started_and_updated_times() -> None:
    run = {
        "name": "CI - Rust",
        "conclusion": "success",
        "run_started_at": "2026-11-15T00:00:00Z",
        "updated_at": "2026-11-15T00:10:00Z",
    }

    assert ci_metrics.run_duration_seconds(run) == 600.0


def test_summarize_run_durations_groups_and_reports_p95() -> None:
    runs = [
        {
            "name": "CI - Rust",
            "conclusion": "success",
            "run_started_at": "2026-11-15T00:00:00Z",
            "updated_at": "2026-11-15T00:10:00Z",
        },
        {
            "name": "CI - Rust",
            "conclusion": "failure",
            "run_started_at": "2026-11-15T00:00:00Z",
            "updated_at": "2026-11-15T00:20:00Z",
        },
        {
            "name": "CI - Docs",
            "conclusion": "skipped",
            "run_started_at": "2026-11-15T00:00:00Z",
            "updated_at": "2026-11-15T00:30:00Z",
        },
    ]

    summary = ci_metrics.summarize_run_durations(runs)

    assert summary["CI - Rust"]["count"] == 2.0
    assert summary["CI - Rust"]["median_seconds"] == 900.0
    assert summary["CI - Rust"]["max_seconds"] == 1200.0
    assert "CI - Docs" not in summary
