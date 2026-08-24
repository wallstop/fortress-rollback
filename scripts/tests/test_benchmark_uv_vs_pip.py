#!/usr/bin/env python3
"""Contract tests for scripts/ci/benchmark_uv_vs_pip.py (issue #281).

Every test is hermetic: installers are fake executables that sleep briefly and
exit zero, and requirements files install nothing, so no network access or real
package installation occurs.
"""

from __future__ import annotations

import json
import os
import stat
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
BENCH = REPO_ROOT / "scripts" / "ci" / "benchmark_uv_vs_pip.py"


def write_fake_installer(path: Path, sleep_seconds: float) -> None:
    """Create an executable shim accepting any arguments and exiting zero."""
    path.write_text(f"#!/bin/sh\nsleep {sleep_seconds}\nexit 0\n", encoding="utf-8")
    mode = path.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH
    path.chmod(mode)


def write_failing_installer(path: Path) -> None:
    path.write_text(
        "#!/bin/sh\necho 'simulated resolver failure' >&2\nexit 3\n", encoding="utf-8"
    )
    mode = path.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH
    path.chmod(mode)


def run_bench(
    args: list[str], env_overrides: dict[str, str] | None = None
) -> subprocess.CompletedProcess[str]:
    env = os.environ.copy()
    env.pop("UV_BIN", None)
    if env_overrides:
        env.update(env_overrides)
    return subprocess.run(
        [sys.executable, str(BENCH), *args],
        capture_output=True,
        text=True,
        cwd=REPO_ROOT,
        env=env,
        timeout=600,
        check=False,
    )


def comment_only_requirements(tmp_path: Path) -> Path:
    """A requirements file pip/uv accept without installing anything."""
    target = tmp_path / "comment-only-requirements.txt"
    target.write_text(
        "# nothing to install; used for hermetic timing runs\n", encoding="utf-8"
    )
    return target


def test_report_measures_both_tools_with_stable_schema(tmp_path: Path) -> None:
    uv_bin = tmp_path / "uv-shim"
    write_fake_installer(uv_bin, 0.01)
    requirements = comment_only_requirements(tmp_path)
    output = tmp_path / "report.json"

    result = run_bench(
        [
            "--requirements",
            str(requirements),
            "--repetitions",
            "2",
            "--output",
            str(output),
        ],
        env_overrides={"UV_BIN": str(uv_bin)},
    )

    assert result.returncode == 0, result.stderr
    report = json.loads(result.stdout)
    assert report["schema"] == 1
    assert report["repetitions"] == 2
    assert str(requirements) in report["requirements"]

    pip_results = report["tools"]["pip"]
    assert set(pip_results) == {requirements.name}
    entry = pip_results[requirements.name]
    assert isinstance(entry["cold_seconds"], float)
    assert len(entry["warm_samples"]) == 2

    uv_results = report["tools"]["uv"]
    assert uv_results is not None, "uv leg must run when UV_BIN names a working binary"
    assert set(uv_results) == {requirements.name}
    assert report["tools"]["uv_skip_reason"] is None

    persisted = json.loads(output.read_text(encoding="utf-8"))
    assert persisted == report, "file output must match stdout"


def test_missing_uv_records_skip_reason_and_keeps_pip_leg(tmp_path: Path) -> None:
    requirements = comment_only_requirements(tmp_path)

    result = run_bench(
        ["--requirements", str(requirements), "--repetitions", "1"],
        env_overrides={"PATH": "/nonexistent-fortress-bench/bin", "UV_BIN": ""},
    )

    assert result.returncode == 0, result.stderr
    report = json.loads(result.stdout)
    assert report["tools"]["pip"], "the pip leg must still be measured without uv"
    assert report["tools"]["uv"] is None
    reason = report["tools"]["uv_skip_reason"]
    assert isinstance(reason, str) and reason, "a missing uv must carry a skip reason"


def test_uv_leg_skipped_when_uv_bin_override_is_invalid(tmp_path: Path) -> None:
    requirements = comment_only_requirements(tmp_path)

    result = run_bench(
        ["--requirements", str(requirements), "--repetitions", "1"],
        env_overrides={"UV_BIN": str(tmp_path / "does-not-exist")},
    )

    assert result.returncode == 0, result.stderr
    report = json.loads(result.stdout)
    assert report["tools"]["uv"] is None
    assert str(tmp_path / "does-not-exist") in report["tools"]["uv_skip_reason"]


def test_failing_uv_install_fails_closed(tmp_path: Path) -> None:
    failing = tmp_path / "failing-uv-shim"
    write_failing_installer(failing)
    requirements = comment_only_requirements(tmp_path)

    result = run_bench(
        ["--requirements", str(requirements), "--repetitions", "1"],
        env_overrides={"UV_BIN": str(failing)},
    )

    assert result.returncode != 0, "an installer failure must fail closed"
    combined = result.stdout + result.stderr
    assert "install command failed" in combined


def test_missing_requirements_file_fails_closed(tmp_path: Path) -> None:
    result = run_bench(["--requirements", str(tmp_path / "absent.txt")])
    assert result.returncode != 0
    assert "requirements file not found" in (result.stderr + result.stdout)


def test_repetitions_must_be_positive(tmp_path: Path) -> None:
    requirements = comment_only_requirements(tmp_path)
    result = run_bench(["--requirements", str(requirements), "--repetitions", "0"])
    assert result.returncode != 0
