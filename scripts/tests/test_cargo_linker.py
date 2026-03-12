#!/usr/bin/env python3
"""
Unit tests for cargo_linker.py linker detection and env override logic.

Tests cover platform detection, lld availability checks, and correct
environment variable generation for all supported target triples.
"""

from __future__ import annotations

import sys
from pathlib import Path
from unittest.mock import patch

# Add scripts directory to path for imports
scripts_dir = Path(__file__).parent.parent
sys.path.insert(0, str(scripts_dir))

import pytest

from cargo_linker import _get_linux_target_triple, _is_lld_available, get_cargo_env


# ---------------------------------------------------------------------------
# Tests for _get_linux_target_triple
# ---------------------------------------------------------------------------


class TestGetLinuxTargetTriple:
    """Tests for Linux target triple detection."""

    def test_x86_64_returns_correct_triple(self) -> None:
        """x86_64 machine maps to x86_64-unknown-linux-gnu."""
        with patch("cargo_linker.platform") as mock_platform:
            mock_platform.machine.return_value = "x86_64"
            assert _get_linux_target_triple() == "x86_64-unknown-linux-gnu"

    def test_aarch64_returns_correct_triple(self) -> None:
        """aarch64 machine maps to aarch64-unknown-linux-gnu."""
        with patch("cargo_linker.platform") as mock_platform:
            mock_platform.machine.return_value = "aarch64"
            assert _get_linux_target_triple() == "aarch64-unknown-linux-gnu"

    def test_unknown_arch_uses_fallback(self) -> None:
        """Unknown architecture falls back to {arch}-unknown-linux-gnu."""
        with patch("cargo_linker.platform") as mock_platform:
            mock_platform.machine.return_value = "riscv64"
            assert _get_linux_target_triple() == "riscv64-unknown-linux-gnu"


# ---------------------------------------------------------------------------
# Tests for _is_lld_available
# ---------------------------------------------------------------------------


class TestIsLldAvailable:
    """Tests for lld availability detection."""

    def test_lld_found_via_lld(self) -> None:
        """Returns True when 'lld' is on PATH."""
        with patch("cargo_linker.shutil.which") as mock_which:
            mock_which.side_effect = lambda name: "/usr/bin/lld" if name == "lld" else None
            assert _is_lld_available() is True

    def test_lld_found_via_ld_lld(self) -> None:
        """Returns True when only 'ld.lld' is on PATH (not 'lld')."""
        with patch("cargo_linker.shutil.which") as mock_which:
            mock_which.side_effect = lambda name: "/usr/bin/ld.lld" if name == "ld.lld" else None
            assert _is_lld_available() is True

    def test_lld_not_found(self) -> None:
        """Returns False when neither 'lld' nor 'ld.lld' is on PATH."""
        with patch("cargo_linker.shutil.which", return_value=None):
            assert _is_lld_available() is False

    def test_both_lld_and_ld_lld_found(self) -> None:
        """Returns True when both 'lld' and 'ld.lld' are on PATH."""
        with patch("cargo_linker.shutil.which", return_value="/usr/bin/lld"):
            assert _is_lld_available() is True


# ---------------------------------------------------------------------------
# Tests for get_cargo_env
# ---------------------------------------------------------------------------


class TestGetCargoEnv:
    """Tests for get_cargo_env environment override logic."""

    # -- Non-Linux platforms: should always return empty dict --

    def test_macos_returns_empty(self) -> None:
        """macOS needs no overrides regardless of lld availability."""
        with patch("cargo_linker.platform") as mock_platform:
            mock_platform.system.return_value = "Darwin"
            result = get_cargo_env()
            assert result == {}, (
                "macOS should not produce linker overrides"
            )

    def test_windows_returns_empty(self) -> None:
        """Windows needs no overrides regardless of lld availability."""
        with patch("cargo_linker.platform") as mock_platform:
            mock_platform.system.return_value = "Windows"
            result = get_cargo_env()
            assert result == {}, (
                "Windows should not produce linker overrides"
            )

    # -- Linux with lld available: should return empty dict --

    def test_linux_with_lld_returns_empty(self) -> None:
        """Linux with lld installed needs no overrides."""
        with patch("cargo_linker.platform") as mock_platform, \
             patch("cargo_linker.shutil.which", return_value="/usr/bin/lld"):
            mock_platform.system.return_value = "Linux"
            result = get_cargo_env()
            assert result == {}, (
                "Linux with lld available should not produce overrides"
            )

    # -- Linux without lld: should return overrides --

    def test_linux_x86_64_without_lld_returns_overrides(self) -> None:
        """Linux x86_64 without lld produces correct env overrides."""
        with patch("cargo_linker.platform") as mock_platform, \
             patch("cargo_linker.shutil.which", return_value=None):
            mock_platform.system.return_value = "Linux"
            mock_platform.machine.return_value = "x86_64"
            result = get_cargo_env()
            assert result == {
                "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER": "cc",
                "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS": "",
            }, (
                "x86_64 Linux without lld should override linker to 'cc' "
                "and clear rustflags"
            )

    def test_linux_aarch64_without_lld_returns_overrides(self) -> None:
        """Linux aarch64 without lld produces correct env overrides."""
        with patch("cargo_linker.platform") as mock_platform, \
             patch("cargo_linker.shutil.which", return_value=None):
            mock_platform.system.return_value = "Linux"
            mock_platform.machine.return_value = "aarch64"
            result = get_cargo_env()
            assert result == {
                "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER": "cc",
                "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUSTFLAGS": "",
            }, (
                "aarch64 Linux without lld should override linker to 'cc' "
                "and clear rustflags"
            )

    def test_linux_unknown_arch_without_lld_returns_overrides(self) -> None:
        """Linux with unknown arch without lld uses fallback triple."""
        with patch("cargo_linker.platform") as mock_platform, \
             patch("cargo_linker.shutil.which", return_value=None):
            mock_platform.system.return_value = "Linux"
            mock_platform.machine.return_value = "riscv64"
            result = get_cargo_env()
            assert result == {
                "CARGO_TARGET_RISCV64_UNKNOWN_LINUX_GNU_LINKER": "cc",
                "CARGO_TARGET_RISCV64_UNKNOWN_LINUX_GNU_RUSTFLAGS": "",
            }, (
                "Unknown arch on Linux without lld should still produce "
                "overrides using the fallback triple"
            )

    # -- Diagnostic output --

    def test_fallback_prints_diagnostic_message(self, capsys: pytest.CaptureFixture[str]) -> None:
        """A diagnostic message is printed when falling back to system linker."""
        with patch("cargo_linker.platform") as mock_platform, \
             patch("cargo_linker.shutil.which", return_value=None):
            mock_platform.system.return_value = "Linux"
            mock_platform.machine.return_value = "x86_64"
            get_cargo_env()
            captured = capsys.readouterr()
            assert "lld not found" in captured.out, (
                "Expected a diagnostic message about lld not being found"
            )
            assert "default system linker" in captured.out, (
                "Expected the message to mention falling back to default linker"
            )

    def test_no_message_when_lld_available(self, capsys: pytest.CaptureFixture[str]) -> None:
        """No diagnostic message is printed when lld is available."""
        with patch("cargo_linker.platform") as mock_platform, \
             patch("cargo_linker.shutil.which", return_value="/usr/bin/lld"):
            mock_platform.system.return_value = "Linux"
            result = get_cargo_env()
            captured = capsys.readouterr()
            assert captured.out == "", (
                "No output expected when lld is available"
            )
            assert result == {}

    def test_no_message_on_non_linux(self, capsys: pytest.CaptureFixture[str]) -> None:
        """No diagnostic message is printed on non-Linux platforms."""
        with patch("cargo_linker.platform") as mock_platform:
            mock_platform.system.return_value = "Darwin"
            result = get_cargo_env()
            captured = capsys.readouterr()
            assert captured.out == "", (
                "No output expected on non-Linux platforms"
            )
            assert result == {}

    # -- Edge cases --

    def test_overrides_contain_exactly_two_keys(self) -> None:
        """When overrides are returned, they contain exactly LINKER and RUSTFLAGS."""
        with patch("cargo_linker.platform") as mock_platform, \
             patch("cargo_linker.shutil.which", return_value=None):
            mock_platform.system.return_value = "Linux"
            mock_platform.machine.return_value = "x86_64"
            result = get_cargo_env()
            assert len(result) == 2, (
                f"Expected exactly 2 env overrides, got {len(result)}: {result}"
            )
            keys = set(result.keys())
            assert any(k.endswith("_LINKER") for k in keys), (
                "Expected a _LINKER key in overrides"
            )
            assert any(k.endswith("_RUSTFLAGS") for k in keys), (
                "Expected a _RUSTFLAGS key in overrides"
            )

    def test_linker_override_value_is_cc(self) -> None:
        """The linker override uses 'cc' (the standard system compiler driver)."""
        with patch("cargo_linker.platform") as mock_platform, \
             patch("cargo_linker.shutil.which", return_value=None):
            mock_platform.system.return_value = "Linux"
            mock_platform.machine.return_value = "x86_64"
            result = get_cargo_env()
            linker_key = "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER"
            assert result[linker_key] == "cc", (
                f"Linker should be 'cc', got '{result[linker_key]}'"
            )

    def test_rustflags_override_is_empty_string(self) -> None:
        """The rustflags override is an empty string (clears config.toml flags)."""
        with patch("cargo_linker.platform") as mock_platform, \
             patch("cargo_linker.shutil.which", return_value=None):
            mock_platform.system.return_value = "Linux"
            mock_platform.machine.return_value = "x86_64"
            result = get_cargo_env()
            flags_key = "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS"
            assert result[flags_key] == "", (
                f"Rustflags should be empty string, got '{result[flags_key]}'"
            )


# ---------------------------------------------------------------------------
# Parametrized cross-platform matrix
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    "system, machine, lld_path, expected_keys",
    [
        pytest.param(
            "Linux", "x86_64", None,
            {"CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER", "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS"},
            id="linux-x86_64-no-lld",
        ),
        pytest.param(
            "Linux", "aarch64", None,
            {"CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER", "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUSTFLAGS"},
            id="linux-aarch64-no-lld",
        ),
        pytest.param(
            "Linux", "x86_64", "/usr/bin/lld",
            set(),
            id="linux-x86_64-with-lld",
        ),
        pytest.param(
            "Linux", "aarch64", "/usr/bin/lld",
            set(),
            id="linux-aarch64-with-lld",
        ),
        pytest.param(
            "Darwin", "x86_64", None,
            set(),
            id="macos-x86_64-no-lld",
        ),
        pytest.param(
            "Darwin", "arm64", None,
            set(),
            id="macos-arm64-no-lld",
        ),
        pytest.param(
            "Windows", "AMD64", None,
            set(),
            id="windows-x86_64-no-lld",
        ),
        pytest.param(
            "Windows", "AMD64", "C:\\lld.exe",
            set(),
            id="windows-x86_64-with-lld",
        ),
    ],
)
def test_cross_platform_matrix(
    system: str,
    machine: str,
    lld_path: str | None,
    expected_keys: set[str],
) -> None:
    """Verify correct behavior across platform/arch/lld combinations."""
    with patch("cargo_linker.platform") as mock_platform, \
         patch("cargo_linker.shutil.which", return_value=lld_path):
        mock_platform.system.return_value = system
        mock_platform.machine.return_value = machine
        result = get_cargo_env()
        assert set(result.keys()) == expected_keys, (
            f"For {system}/{machine} (lld={lld_path}): "
            f"expected keys {expected_keys}, got {set(result.keys())}"
        )


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
