#!/usr/bin/env python3
"""Collect GitHub Actions timing metrics for the CI-duration ratchet."""

from __future__ import annotations

import argparse
import json
import os
import statistics
import sys
import urllib.error
import urllib.parse
import urllib.request
from collections import defaultdict
from datetime import datetime
from typing import Any

API_ROOT = "https://api.github.com"


def _parse_time(value: str | None) -> datetime | None:
    if not value:
        return None
    return datetime.fromisoformat(value.replace("Z", "+00:00"))


def run_duration_seconds(run: dict[str, Any]) -> float | None:
    """Return wall-clock duration for one workflow run."""
    started = _parse_time(run.get("run_started_at"))
    finished = _parse_time(run.get("updated_at"))
    if started is None or finished is None:
        return None
    return (finished - started).total_seconds()


def job_duration_seconds(job: dict[str, Any]) -> float | None:
    """Return wall-clock duration for one job."""
    started = _parse_time(job.get("started_at"))
    finished = _parse_time(job.get("completed_at"))
    if started is None or finished is None:
        return None
    return (finished - started).total_seconds()


def summarize_run_durations(runs: list[dict[str, Any]]) -> dict[str, dict[str, float]]:
    """Group completed workflow-run durations by workflow name."""
    grouped: dict[str, list[float]] = defaultdict(list)
    for run in runs:
        duration = run_duration_seconds(run)
        if duration is None or run.get("conclusion") not in {"success", "failure"}:
            continue
        grouped[str(run.get("name", "unknown"))].append(duration)

    summary: dict[str, dict[str, float]] = {}
    for name, durations in sorted(grouped.items()):
        ordered = sorted(durations)
        p95_index = max(0, min(len(ordered) - 1, round(0.95 * (len(ordered) - 1))))
        summary[name] = {
            "count": float(len(ordered)),
            "median_seconds": round(statistics.median(ordered), 1),
            "p95_seconds": round(ordered[p95_index], 1),
            "max_seconds": round(ordered[-1], 1),
        }
    return summary


def _request_json(url: str, token: str | None) -> Any:
    headers = {
        "Accept": "application/vnd.github+json",
        "User-Agent": "fortress-rollback-ci-metrics",
    }
    if token:
        headers["Authorization"] = f"Bearer {token}"
    request = urllib.request.Request(url, headers=headers)  # noqa: S310
    try:
        with urllib.request.urlopen(request, timeout=30) as response:  # noqa: S310
            return json.loads(response.read().decode("utf-8"))
    except (urllib.error.URLError, TimeoutError) as exc:
        raise RuntimeError(f"GitHub API request failed: {exc}") from exc


def fetch_runs(repo: str, limit: int, token: str | None) -> list[dict[str, Any]]:
    """Fetch recent workflow runs, paging until ``limit`` is reached."""
    runs: list[dict[str, Any]] = []
    page = 1
    while len(runs) < limit:
        per_page = min(100, limit - len(runs))
        query = urllib.parse.urlencode(
            {"per_page": per_page, "page": page, "exclude_pull_requests": "false"}
        )
        payload = _request_json(
            f"{API_ROOT}/repos/{repo}/actions/runs?{query}", token
        )
        batch = payload.get("workflow_runs", [])
        if not batch:
            break
        runs.extend(batch)
        page += 1
    return runs[:limit]


def fetch_jobs(repo: str, run_id: int, token: str | None) -> list[dict[str, Any]]:
    """Fetch jobs for one workflow run."""
    payload = _request_json(
        f"{API_ROOT}/repos/{repo}/actions/runs/{run_id}/jobs?per_page=100", token
    )
    jobs = payload.get("jobs", []) if isinstance(payload, dict) else []
    return jobs if isinstance(jobs, list) else []


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", default="wallstop/fortress-rollback")
    parser.add_argument("--limit", type=int, default=100)
    parser.add_argument("--json", action="store_true")
    parser.add_argument(
        "--jobs-for",
        type=int,
        default=None,
        help="Also summarize job durations for one workflow run ID.",
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    token = os.environ.get("GITHUB_TOKEN")
    runs = fetch_runs(args.repo, args.limit, token)
    summary = summarize_run_durations(runs)

    output: dict[str, Any] = {"repo": args.repo, "runs": summary}
    if args.jobs_for is not None:
        jobs = fetch_jobs(args.repo, args.jobs_for, token)
        job_values = [
            duration
            for job in jobs
            if (duration := job_duration_seconds(job)) is not None
        ]
        output["jobs"] = {
            "run_id": args.jobs_for,
            "count": len(job_values),
            "total_minutes": round(sum(job_values) / 60, 1),
        }

    if args.json:
        print(json.dumps(output, indent=2, sort_keys=True))
        return 0

    print(f"{'workflow':42s} {'n':>4s} {'median':>8s} {'p95':>8s} {'max':>8s}")
    for name, values in summary.items():
        print(
            f"{name:42s} {int(values['count']):4d} "
            f"{values['median_seconds']:8.1f} {values['p95_seconds']:8.1f} "
            f"{values['max_seconds']:8.1f}"
        )
    if "jobs" in output:
        print(
            f"\nrun {output['jobs']['run_id']}: "
            f"{output['jobs']['count']} jobs, "
            f"{output['jobs']['total_minutes']} job-minutes"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
