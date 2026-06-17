#!/usr/bin/env python3
"""Regression tests for external link-checker configuration."""

from __future__ import annotations

import json
import re
import shlex
from pathlib import Path

import pytest
import yaml

# Python 3.11+ ships tomllib in the stdlib; older interpreters fall back to the
# tomli backport so these script tests run on the same Python range as the rest
# of the repo tooling (mirrors scripts/hooks/check-toml.py). The
# check-tomllib-fallback hook enforces this pattern repo-wide.
try:
    import tomllib

    HAS_TOML = True
except ImportError:  # Python < 3.11
    try:
        import tomli as tomllib

        HAS_TOML = True
    except ImportError:
        HAS_TOML = False

REPO_ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS_DIR = REPO_ROOT / ".github" / "workflows"


def _iter_lychee_args() -> list[tuple[Path, str, str, str]]:
    """Return every lychee-action argument from workflow YAML."""
    args: list[tuple[Path, str, str, str]] = []
    for workflow in sorted(WORKFLOWS_DIR.glob("*.yml")):
        data = yaml.safe_load(workflow.read_text(encoding="utf-8")) or {}
        jobs = data.get("jobs", {})
        if not isinstance(jobs, dict):
            continue
        for job_name, job in jobs.items():
            if not isinstance(job, dict):
                continue
            for step in job.get("steps", []):
                if not isinstance(step, dict):
                    continue
                if not str(step.get("uses", "")).startswith("lycheeverse/lychee-action@"):
                    continue
                raw_args = step.get("with", {}).get("args", "")
                if not isinstance(raw_args, str):
                    continue
                step_name = str(step.get("name", "unnamed step"))
                for arg in shlex.split(raw_args):
                    args.append((workflow, str(job_name), step_name, arg))
    return args


def _is_literal_repo_path(arg: str) -> bool:
    """Return True for lychee arguments that should name real repo paths."""
    return (
        not arg.startswith("-")
        and "://" not in arg
        and not any(char in arg for char in "*?[")
    )


LYCHEE_LITERAL_PATH_ARGS = [
    item for item in _iter_lychee_args() if _is_literal_repo_path(item[3])
]


@pytest.mark.parametrize(
    "workflow,job_name,step_name,arg",
    LYCHEE_LITERAL_PATH_ARGS,
    ids=[
        f"{workflow.name}:{job_name}:{step_name}:{arg}"
        for workflow, job_name, step_name, arg in LYCHEE_LITERAL_PATH_ARGS
    ],
)
def test_lychee_literal_path_args_exist(
    workflow: Path,
    job_name: str,
    step_name: str,
    arg: str,
) -> None:
    """Missing path args are interpreted as external URLs and create noise."""
    assert (REPO_ROOT / arg).exists(), (
        f"{workflow.relative_to(REPO_ROOT)} job {job_name!r} step {step_name!r} "
        f"passes missing literal path {arg!r} to lychee. Remove it, make it a "
        "glob, or create the file."
    )


@pytest.mark.parametrize(
    "url",
    [
        "https://github.com/tlaplus/tlaplus/releases/download/v${TLA_TOOLS_VERSION}/tla2tools.jar",
        "https://github.com/tlaplus/tlaplus/releases/download/v%24%7BTLA_TOOLS_VERSION%7D/tla2tools.jar",
    ],
)
def test_lychee_ignores_templated_runtime_urls(url: str) -> None:
    """Runtime-interpolated documentation examples are not fetchable URLs."""
    if not HAS_TOML:
        pytest.skip("tomllib/tomli not available (Python < 3.11 without tomli)")
    config = tomllib.loads((REPO_ROOT / ".lychee.toml").read_text(encoding="utf-8"))
    assert any(re.search(pattern, url) for pattern in config["exclude"])


@pytest.mark.parametrize(
    "url",
    [
        "https://docs.tlapl.us/",
        "https://docs.tlapl.us/using:vscode:start",
        "http://demsky.eecs.uci.edu/publications/c11modelcheck.pdf",
    ],
)
def test_markdown_link_check_ignores_known_external_false_positives(url: str) -> None:
    """Only known external-checker false positives are ignored."""
    config = json.loads(
        (REPO_ROOT / ".markdown-link-check.json").read_text(encoding="utf-8")
    )
    patterns = [entry["pattern"] for entry in config["ignorePatterns"]]
    assert any(re.search(pattern, url) for pattern in patterns)
