#!/usr/bin/env python3
"""Regression tests for the install-cargo-tool action version matching helper."""

from __future__ import annotations

import os
import subprocess
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
VERSION_HELPER = REPO_ROOT / ".github" / "actions" / "install-cargo-tool" / "version-check.sh"


def _write_cargo_stub(tmp_path: Path, install_list: str) -> Path:
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    cargo = bin_dir / "cargo"
    cargo.write_text(
        "#!/bin/bash\n"
        "set -eu\n"
        "if [[ \"${1:-}\" == \"install\" && \"${2:-}\" == \"--list\" ]]; then\n"
        f"  cat <<'EOF'\n{install_list}\nEOF\n"
        "  exit 0\n"
        "fi\n"
        "exit 2\n",
        encoding="utf-8",
    )
    cargo.chmod(0o755)
    return bin_dir


def _run_helper(tmp_path: Path, command: str, install_list: str) -> subprocess.CompletedProcess[str]:
    bin_dir = _write_cargo_stub(tmp_path, install_list)
    env = os.environ.copy()
    env["PATH"] = f"{bin_dir}:{env['PATH']}"
    return subprocess.run(
        ["bash", "-c", f"source {VERSION_HELPER}; {command}"],
        cwd=REPO_ROOT,
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )


def test_installed_version_rejects_unrelated_cargo_install_version(
    tmp_path: Path,
) -> None:
    """A stale target binary must not pass because another package has the version."""
    result = _run_helper(
        tmp_path,
        "installed_version_matches cargo-nextest 'cargo-nextest 0.9.99' 0.9.100",
        "cargo-llvm-cov v0.9.100:\n    cargo-llvm-cov",
    )

    assert result.returncode == 1


def test_installed_version_rejects_matching_dependency_version_in_tool_output(
    tmp_path: Path,
) -> None:
    """Only the target tool's primary version token may satisfy the requirement."""
    result = _run_helper(
        tmp_path,
        "installed_version_matches cargo-nextest 'cargo-nextest 0.9.99 nextest-runner 0.9.100,' 0.9.100",
        "",
    )

    assert result.returncode == 1


def test_installed_version_accepts_primary_tool_version_output(
    tmp_path: Path,
) -> None:
    """The target tool's own version remains the preferred successful path."""
    result = _run_helper(
        tmp_path,
        "installed_version_matches cargo-nextest 'cargo-nextest 0.9.100 nextest-runner 0.9.99' 0.9.100",
        "",
    )

    assert result.returncode == 0


def test_installed_version_accepts_target_cargo_install_entry_when_output_missing(
    tmp_path: Path,
) -> None:
    """The cargo-install fallback is allowed only for the requested tool entry."""
    result = _run_helper(
        tmp_path,
        "installed_version_matches cargo-nextest '' 0.9.100",
        "cargo-nextest v0.9.100:\n    cargo-nextest",
    )

    assert result.returncode == 0


def test_installed_version_rejects_unrelated_entry_when_output_missing(
    tmp_path: Path,
) -> None:
    """The fallback must stay scoped even when the target has no --version output."""
    result = _run_helper(
        tmp_path,
        "installed_version_matches cargo-nextest '' 0.9.100",
        "cargo-llvm-cov v0.9.100:\n    cargo-llvm-cov",
    )

    assert result.returncode == 1
