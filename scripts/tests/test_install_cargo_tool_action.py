#!/usr/bin/env python3
"""Regression tests for the install-cargo-tool action version matching helper."""

from __future__ import annotations

import os
import subprocess
from pathlib import Path

import pytest

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


def _run_helper(
    tmp_path: Path,
    command: str,
    install_list: str,
    env_overrides: dict[str, str] | None = None,
    helper_path: Path = VERSION_HELPER,
) -> subprocess.CompletedProcess[str]:
    bin_dir = _write_cargo_stub(tmp_path, install_list)
    env = os.environ.copy()
    env["PATH"] = f"{bin_dir}:{env['PATH']}"
    env["VERSION_HELPER_PATH"] = str(helper_path)
    if env_overrides:
        env.update(env_overrides)
    return subprocess.run(
        ["bash", "-c", 'source "$VERSION_HELPER_PATH"; eval "$TEST_COMMAND"'],
        cwd=REPO_ROOT,
        env={**env, "TEST_COMMAND": command},
        capture_output=True,
        text=True,
        check=False,
    )


def _bash_single_quote(value: str) -> str:
    return "'" + value.replace("'", "'\"'\"'") + "'"


@pytest.mark.parametrize("value", ["it's", "a'b'c", "''"])
def test_bash_single_quote_round_trip(tmp_path: Path, value: str) -> None:
    """Shell-quoted helper output should round-trip through bash unchanged."""
    result = _run_helper(tmp_path, f"printf %s {_bash_single_quote(value)}", "")
    assert result.returncode == 0
    assert result.stdout == value


@pytest.mark.parametrize(
    ("command", "install_list", "expected_returncode"),
    [
        (
            "installed_version_matches cargo-nextest 'cargo-nextest 0.9.99' 0.9.100",
            "cargo-llvm-cov v0.9.100:\n    cargo-llvm-cov",
            1,
        ),
        (
            "installed_version_matches cargo-nextest 'cargo-nextest 0.9.99 nextest-runner 0.9.100,' 0.9.100",
            "",
            1,
        ),
        (
            "installed_version_matches cargo-nextest 'cargo-nextest 0.9.100 nextest-runner 0.9.99' 0.9.100",
            "",
            0,
        ),
        (
            "installed_version_matches cargo-nextest '' 0.9.100",
            "cargo-nextest v0.9.100:\n    cargo-nextest",
            0,
        ),
        (
            "installed_version_matches cargo-nextest '' 0.9.100",
            "cargo-llvm-cov v0.9.100:\n    cargo-llvm-cov",
            1,
        ),
    ],
    ids=[
        "reject_unrelated_cargo_install_version",
        "reject_dependency_version_token",
        "accept_primary_tool_version",
        "accept_matching_cargo_install_entry",
        "reject_unrelated_cargo_install_entry",
    ],
)
def test_installed_version_matches_matrix(
    tmp_path: Path,
    command: str,
    install_list: str,
    expected_returncode: int,
) -> None:
    """Version checks remain strict across stale and valid version-reporting patterns."""
    result = _run_helper(tmp_path, command, install_list)
    assert result.returncode == expected_returncode


@pytest.mark.parametrize(
    ("output", "tool_name", "required", "expected_returncode"),
    [
        ("cargo-nextest 0.9.100", "cargo-nextest", "0.9.100", 0),
        ("cargo-nextest.exe v0.9.100:", "cargo-nextest", "0.9.100", 0),
        (
            "cargo-nextest 0.9.99 nextest-runner 0.9.100,",
            "cargo-nextest",
            "0.9.100",
            1,
        ),
        ("", "cargo-nextest", "0.9.100", 1),
        ("nextest version 0.9.100", "cargo-nextest", "0.9.100", 1),
    ],
    ids=[
        "exact_match",
        "windows_exe_prefix_and_v",
        "dependency_token_must_not_match",
        "empty_output_fails",
        "unexpected_format_fails",
    ],
)
def test_primary_version_output_matches_required_matrix(
    tmp_path: Path,
    output: str,
    tool_name: str,
    required: str,
    expected_returncode: int,
) -> None:
    """Primary parser should only accept the requested tool's own version token."""
    command = (
        "primary_version_output_matches_required "
        f"{_bash_single_quote(output)} "
        f"{_bash_single_quote(tool_name)} "
        f"{_bash_single_quote(required)}"
    )
    result = _run_helper(tmp_path, command, "")
    assert result.returncode == expected_returncode


@pytest.mark.parametrize(
    ("env_overrides", "expected_cargo_home"),
    [
        ({"CARGO_HOME": "/tmp/custom-cargo", "HOME": "/tmp/home"}, "/tmp/custom-cargo"),
        ({"CARGO_HOME": "", "HOME": "/tmp/home"}, "/tmp/home/.cargo"),
        (
            {"CARGO_HOME": "C:\\Users\\runneradmin\\.cargo", "HOME": "/tmp/home"},
            "C:/Users/runneradmin/.cargo",
        ),
    ],
    ids=[
        "prefer_cargo_home",
        "fallback_to_home",
        "normalize_windows_backslashes",
    ],
)
def test_resolve_cargo_home_matrix(
    tmp_path: Path,
    env_overrides: dict[str, str],
    expected_cargo_home: str,
) -> None:
    """Cargo home resolution must be deterministic across OS-specific path formats."""
    result = _run_helper(tmp_path, "resolve_cargo_home", "", env_overrides)
    assert result.returncode == 0
    assert result.stdout.strip() == expected_cargo_home


def test_resolve_cargo_home_requires_environment(tmp_path: Path) -> None:
    """A missing HOME and CARGO_HOME should fail with a precise diagnostic."""
    result = _run_helper(
        tmp_path,
        "resolve_cargo_home",
        "",
        {"CARGO_HOME": "", "HOME": ""},
    )

    assert result.returncode == 1
    assert "neither CARGO_HOME nor HOME is set" in result.stderr


def test_run_helper_supports_shell_metacharacters_in_helper_path(tmp_path: Path) -> None:
    """Helper sourcing should work when the helper path contains shell metacharacters."""
    helper_dir = tmp_path / "helper path's"
    helper_dir.mkdir()
    helper_copy = helper_dir / "version-check.sh"
    helper_copy.write_text(VERSION_HELPER.read_text(encoding="utf-8"), encoding="utf-8")
    helper_copy.chmod(0o755)

    result = _run_helper(
        tmp_path,
        "resolve_cargo_home",
        "",
        {"CARGO_HOME": "", "HOME": "/tmp/home"},
        helper_path=helper_copy,
    )
    assert result.returncode == 0
    assert result.stdout.strip() == "/tmp/home/.cargo"


@pytest.mark.parametrize(
    ("cargo_home", "expected_returncode"),
    [
        ("/tmp/.cargo", 0),
        ("//server/share/.cargo", 0),
        ("/c/Users/runneradmin/.cargo", 0),
        ("C:/Users/runneradmin/.cargo", 0),
        ("C:\\Users\\runneradmin\\.cargo", 0),
        ("", 1),
        ("/", 1),
        ("C:/", 1),
        ("C:\\", 1),
        (".", 1),
        ("..", 1),
        ("cargo", 1),
        ("./cargo", 1),
        ("../cargo", 1),
        ("~/.cargo", 1),
        ("C:relative", 1),
    ],
    ids=[
        "accept_unix_absolute",
        "accept_unc_absolute",
        "accept_msys_absolute",
        "accept_windows_absolute",
        "accept_windows_backslashes",
        "reject_empty",
        "reject_unix_root",
        "reject_windows_root",
        "reject_windows_root_backslashes",
        "reject_relative_current",
        "reject_relative_parent",
        "reject_relative_bare",
        "reject_relative_dot_prefix",
        "reject_relative_parent_prefix",
        "reject_tilde",
        "reject_windows_drive_relative",
    ],
)
def test_cargo_home_is_safe_matrix(
    tmp_path: Path,
    cargo_home: str,
    expected_returncode: int,
) -> None:
    """Cargo home safety checks must reject unsafe paths before destructive actions."""
    command = f"cargo_home_is_safe {_bash_single_quote(cargo_home)}"
    result = _run_helper(tmp_path, command, "")
    assert result.returncode == expected_returncode


@pytest.mark.parametrize(
    ("tool_name", "cargo_home", "expected_glob"),
    [
        ("cargo-nextest", "/tmp/.cargo", "/tmp/.cargo/bin/cargo-nextest*"),
        ("cargo-hack", "/tmp/.cargo/", "/tmp/.cargo/bin/cargo-hack*"),
        (
            "cargo-nextest",
            "C:\\Users\\runneradmin\\.cargo",
            "C:/Users/runneradmin/.cargo/bin/cargo-nextest*",
        ),
    ],
    ids=[
        "unix_path",
        "trailing_slash",
        "windows_path_normalized",
    ],
)
def test_cargo_tool_cache_glob_matrix(
    tmp_path: Path,
    tool_name: str,
    cargo_home: str,
    expected_glob: str,
) -> None:
    """Cache glob generation should be stable for both Unix and Windows path styles."""
    command = f"cargo_tool_cache_glob {tool_name} '{cargo_home}'"
    result = _run_helper(tmp_path, command, "")
    assert result.returncode == 0
    assert result.stdout.strip() == expected_glob


def test_cargo_tool_cache_glob_requires_tool_name(tmp_path: Path) -> None:
    """An empty tool name should fail fast with a useful error."""
    result = _run_helper(tmp_path, "cargo_tool_cache_glob '' '/tmp/.cargo'", "")
    assert result.returncode == 1
    assert "tool name is required" in result.stderr
