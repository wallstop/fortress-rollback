"""Regression tests for the unified formal-verification driver."""

from __future__ import annotations

import os
import shutil
import subprocess
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[2]
VERIFY_ALL = REPO_ROOT / "scripts" / "verification" / "verify-all.sh"
Z3_COMMAND = "test --test verification --features z3-verification -- z3 --nocapture"


def verifier_workspace(tmp_path: Path) -> tuple[Path, dict[str, str], Path]:
    """Install the real driver with fake sibling verifiers and Cargo."""
    script = tmp_path / "verify-all.sh"
    shutil.copyfile(VERIFY_ALL, script)
    for name in ("verify-tla.sh", "verify-kani.sh"):
        verifier = tmp_path / name
        verifier.write_text("#!/bin/bash\nexit 0\n")
        verifier.chmod(0o755)

    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    cargo = bin_dir / "cargo"
    cargo.write_text(
        "#!/bin/bash\n"
        'printf "%s\\n" "$*" >> "$VERIFY_ALL_CALLS"\n'
        'exit "${VERIFY_ALL_CARGO_EXIT:-0}"\n'
    )
    cargo.chmod(0o755)
    calls = tmp_path / "calls.txt"
    env = os.environ.copy()
    env["PATH"] = f"{bin_dir}{os.pathsep}{env['PATH']}"
    env["VERIFY_ALL_CALLS"] = str(calls)
    return script, env, calls


@pytest.mark.parametrize("args", [[], ["--parallel"]])
def test_default_and_parallel_run_z3(args: list[str], tmp_path: Path) -> None:
    """The all-verifier modes must not silently omit the implemented Z3 gate."""
    script, env, calls = verifier_workspace(tmp_path)

    result = subprocess.run(
        ["bash", str(script), *args],
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )

    assert result.returncode == 0, result.stdout + result.stderr
    assert "Z3 SMT Verification" in result.stdout
    assert "not yet implemented" not in result.stdout
    assert calls.read_text().splitlines() == [Z3_COMMAND]


def test_z3_failure_fails_unified_driver(tmp_path: Path) -> None:
    """A failed Z3 proof must propagate to the aggregate exit status."""
    script, env, calls = verifier_workspace(tmp_path)
    env["VERIFY_ALL_CARGO_EXIT"] = "1"

    result = subprocess.run(
        ["bash", str(script), "--z3"],
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )

    assert result.returncode == 1, result.stdout + result.stderr
    assert "Some Verification Failed" in result.stdout
    assert calls.read_text().splitlines() == [Z3_COMMAND]
