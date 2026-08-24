#!/usr/bin/env python3
"""Benchmark ``pip`` against ``uv`` for this repository's real Python installs.

Issue #281 asks for a red-green, evidence-backed comparison before deciding
whether to adopt uv. This script measures the two installers against the
repository's actual requirement manifests:

* **cold** -- install into a freshly created throwaway virtual environment.
* **warm** -- repeat the identical install into the same environment, where
  every resolver decision is already satisfied (the common CI-cache case).

Both tools run the same manifests in isolated environments under a temporary
directory that is removed unless ``--keep-venvs`` is given. Results go to
stdout as JSON and optionally to ``--output FILE``.

The benchmark is informational: a slow pip never fails the run. The script
fails closed only when the measurement itself cannot be trusted (venv creation
or an installer command errors).

uv is optional. When no ``uv`` binary is found (via ``UV_BIN`` or ``PATH``),
the pip leg still runs and the uv leg is reported as skipped with a reason,
so the harness is usable on every developer machine.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_REQUIREMENTS = (
    REPO_ROOT / "scripts" / "release" / "requirements.txt",
    REPO_ROOT / "requirements-docs.txt",
)
INSTALL_TIMEOUT_SECONDS = 900
VENV_TIMEOUT_SECONDS = 120


class BenchmarkError(Exception):
    """Raised when the measurement itself fails; the script exits nonzero."""


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Benchmark pip vs uv installation of Python requirements "
            "(issue #281). Prints a JSON report to stdout."
        )
    )
    parser.add_argument(
        "--requirements",
        action="append",
        default=[],
        type=Path,
        help=(
            "Requirements file to install (repeatable). Defaults to the "
            "repository's release and docs manifests."
        ),
    )
    parser.add_argument(
        "--repetitions",
        type=int,
        default=3,
        help="Warm repetitions per tool per manifest (default: 3).",
    )
    parser.add_argument(
        "--output",
        type=Path,
        help="Also write the JSON report to this path.",
    )
    parser.add_argument(
        "--keep-venvs",
        action="store_true",
        help="Keep the temporary environments for inspection instead of deleting them.",
    )
    args = parser.parse_args(argv)
    if args.repetitions < 1:
        parser.error("--repetitions must be at least 1")
    return args


def resolve_requirements(explicit: list[Path]) -> list[Path]:
    candidates = explicit or list(DEFAULT_REQUIREMENTS)
    resolved: list[Path] = []
    for candidate in candidates:
        path = candidate if candidate.is_absolute() else (Path.cwd() / candidate)
        if not path.is_file():
            raise BenchmarkError(f"requirements file not found: {path}")
        resolved.append(path.resolve())
    return resolved


def uv_absence_reason() -> str:
    """Explain why uv is unavailable, for the report's skip record."""
    override = os.environ.get("UV_BIN", "").strip()
    if override and not (Path(override).is_file() or shutil.which(override)):
        return f"UV_BIN={override} does not name an executable file"
    return "no uv executable on PATH and UV_BIN is unset"


def find_uv() -> str | None:
    """Locate uv via ``UV_BIN`` or ``PATH``; ``None`` means legitimately absent."""
    override = os.environ.get("UV_BIN", "").strip()
    if override:
        if Path(override).is_file() or shutil.which(override):
            return override
        return None
    return shutil.which("uv")


def create_venv(python: str, parent: Path, name: str) -> Path:
    venv_dir = parent / name
    result = subprocess.run(
        [python, "-m", "venv", str(venv_dir)],
        capture_output=True,
        text=True,
        timeout=VENV_TIMEOUT_SECONDS,
        check=False,
    )
    if result.returncode != 0:
        raise BenchmarkError(
            f"venv creation failed for {name}: {result.stderr.strip() or result.stdout.strip()}"
        )
    interpreter = venv_dir / "bin" / "python"
    if not interpreter.is_file():
        raise BenchmarkError(f"venv {name} has no interpreter at {interpreter}")
    return interpreter


def run_timed(command: list[str]) -> float:
    start = time.monotonic()
    try:
        result = subprocess.run(
            command,
            capture_output=True,
            text=True,
            timeout=INSTALL_TIMEOUT_SECONDS,
            check=False,
        )
    except subprocess.TimeoutExpired as err:
        raise BenchmarkError(
            f"install timed out after {INSTALL_TIMEOUT_SECONDS}s: {command}"
        ) from err
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip()
        raise BenchmarkError(
            f"install command failed ({result.returncode}): {' '.join(command)}: {detail}"
        )
    return time.monotonic() - start


def pip_install_command(interpreter: Path, requirements: Path) -> list[str]:
    return [
        str(interpreter),
        "-m",
        "pip",
        "install",
        "--quiet",
        "-r",
        str(requirements),
    ]


def uv_install_command(uv_bin: str, interpreter: Path, requirements: Path) -> list[str]:
    return [
        uv_bin,
        "pip",
        "install",
        "--quiet",
        "--python",
        str(interpreter),
        "-r",
        str(requirements),
    ]


def measure_tool(
    build_command: Any,
    requirements: list[Path],
    python: str,
    workspace: Path,
    repetitions: int,
) -> dict[str, Any]:
    """Measure cold+warm installs across every manifest for one installer."""
    per_manifest: dict[str, Any] = {}
    for requirements_path in requirements:
        label = requirements_path.name
        venv_name = f"{label.replace('.', '_')}_venv"
        interpreter = create_venv(python, workspace, venv_name)
        cold_samples = [run_timed(build_command(interpreter, requirements_path))]
        warm_samples = [
            run_timed(build_command(interpreter, requirements_path))
            for _ in range(repetitions)
        ]
        per_manifest[label] = {
            "cold_seconds": round(statistics.median(cold_samples), 4),
            "warm_seconds_median": round(statistics.median(warm_samples), 4),
            "warm_samples": [round(sample, 4) for sample in warm_samples],
        }
    return per_manifest


def build_report(
    requirements: list[Path],
    repetitions: int,
    pip_results: dict[str, Any],
    uv_result: dict[str, Any] | None,
    uv_skip_reason: str | None,
) -> dict[str, Any]:
    return {
        "schema": 1,
        "repetitions": repetitions,
        "requirements": [str(path) for path in requirements],
        "tools": {
            "pip": pip_results,
            "uv": uv_result,
            "uv_skip_reason": uv_skip_reason,
        },
    }


def summarize(report: dict[str, Any]) -> str:
    lines = ["pip vs uv install benchmark (seconds)", ""]
    for tool in ("pip", "uv"):
        results = report["tools"][tool]
        if results is None:
            reason = report["tools"]["uv_skip_reason"] or "not measured"
            lines.append(f"  {tool}: SKIPPED ({reason})")
            continue
        for label, entry in results.items():
            lines.append(
                f"  {tool} [{label}]: cold={entry['cold_seconds']} "
                f"warm_median={entry['warm_seconds_median']}"
            )
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        requirements = resolve_requirements(args.requirements)
        python = sys.executable
        uv_bin = find_uv()
        temp_dir = tempfile.mkdtemp(prefix="fortress-uv-bench-")
        workspace = Path(temp_dir)
        try:
            pip_results = measure_tool(
                pip_install_command,
                requirements,
                python,
                workspace / "pip",
                args.repetitions,
            )
            uv_results: dict[str, Any] | None = None
            uv_skip_reason: str | None = None
            if uv_bin is None:
                uv_skip_reason = uv_absence_reason()
            else:
                uv_results = measure_tool(
                    lambda interp, reqs: uv_install_command(uv_bin, interp, reqs),
                    requirements,
                    python,
                    workspace / "uv",
                    args.repetitions,
                )
        finally:
            if args.keep_venvs:
                print(f"environments retained under {workspace}", file=sys.stderr)
            else:
                shutil.rmtree(workspace, ignore_errors=True)

        report = build_report(
            requirements, args.repetitions, pip_results, uv_results, uv_skip_reason
        )
        text = json.dumps(report, indent=2, sort_keys=True)
        print(text)
        print("", summarize(report), sep="", file=sys.stderr)
        if args.output is not None:
            args.output.write_text(text + "\n", encoding="utf-8")
    except BenchmarkError as err:
        print(f"error: {err}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
