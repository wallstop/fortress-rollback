#!/usr/bin/env python3
"""Regression tests for scripts/verification/verify-kani.sh diagnostics."""

from __future__ import annotations

import os
import shutil
import subprocess
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[2]
VERIFY_KANI_SOURCE = REPO_ROOT / "scripts" / "verification" / "verify-kani.sh"


def _setup_repo(tmp_path: Path) -> tuple[Path, Path, Path]:
    """Create a temporary repo with verify-kani.sh and fake cargo tools."""
    repo = tmp_path / "repo"
    script_path = repo / "scripts" / "verification" / "verify-kani.sh"
    script_path.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(VERIFY_KANI_SOURCE, script_path)

    fake_bin = repo / "fake-bin"
    fake_bin.mkdir()
    cargo = fake_bin / "cargo"
    cargo.write_text(
        "#!/bin/bash\n"
        "set -euo pipefail\n"
        "if [[ \"${1:-}\" == \"kani\" && \"${2:-}\" == \"--version\" ]]; then\n"
        "  echo 'cargo-kani 0.67.0'\n"
        "  exit 0\n"
        "fi\n"
        "echo 'Checking harness fake::proof_timeout_like_exit...'\n"
        "echo 'last diagnostic line from fake Kani'\n"
        "exit \"${FAKE_KANI_EXIT:-1}\"\n",
        encoding="utf-8",
    )
    cargo.chmod(0o755)

    cargo_kani = fake_bin / "cargo-kani"
    cargo_kani.write_text("#!/bin/bash\nexit 0\n", encoding="utf-8")
    cargo_kani.chmod(0o755)

    return repo, script_path, fake_bin


def test_per_proof_timeout_exit_names_harness_and_command(tmp_path: Path) -> None:
    """GNU timeout exits emit a clear per-proof timeout diagnostic."""
    repo, script_path, fake_bin = _setup_repo(tmp_path)
    env = {
        **os.environ,
        "PATH": f"{fake_bin}{os.pathsep}{os.environ['PATH']}",
        "FAKE_KANI_EXIT": "124",
        "KANI_TIMEOUT": "42",
    }

    result = subprocess.run(
        [
            "bash",
            str(script_path),
            "--quick",
            "--harness",
            "proof_minimal_sync_layer_initial_state_valid",
        ],
        cwd=repo,
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )

    combined = result.stdout + result.stderr
    assert result.returncode == 1
    assert (
        "TIMEOUT: proof 'proof_minimal_sync_layer_initial_state_valid' exceeded "
        "the per-proof timeout of 42s (exit code 124)."
    ) in combined
    assert (
        "Command: cargo kani --harness "
        "proof_minimal_sync_layer_initial_state_valid --default-unwind 8"
    ) in combined
    assert "last diagnostic line from fake Kani" in combined


@pytest.mark.parametrize("exit_code", [137, 143])
def test_termination_like_kani_exits_name_harness_and_command(
    tmp_path: Path, exit_code: int
) -> None:
    """Signal-style exits do not claim the per-proof timeout elapsed."""
    repo, script_path, fake_bin = _setup_repo(tmp_path)
    env = {
        **os.environ,
        "PATH": f"{fake_bin}{os.pathsep}{os.environ['PATH']}",
        "FAKE_KANI_EXIT": str(exit_code),
        "KANI_TIMEOUT": "42",
    }

    result = subprocess.run(
        [
            "bash",
            str(script_path),
            "--quick",
            "--harness",
            "proof_minimal_sync_layer_initial_state_valid",
        ],
        cwd=repo,
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )

    combined = result.stdout + result.stderr
    assert result.returncode == 1
    assert (
        "TERMINATED: proof 'proof_minimal_sync_layer_initial_state_valid' ended "
        f"with timeout/cancellation/termination-like exit code {exit_code}."
    ) in combined
    assert "after 42s" not in combined
    assert (
        "Command: cargo kani --harness "
        "proof_minimal_sync_layer_initial_state_valid --default-unwind 8"
    ) in combined
    assert "last diagnostic line from fake Kani" in combined
