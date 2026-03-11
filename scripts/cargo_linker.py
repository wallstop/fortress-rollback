#!/usr/bin/env python3
"""
Shared utility for detecting linker availability and providing cargo env overrides.

The project's .cargo/config.toml hardcodes `linker = "clang"` and
`rustflags = ["-C", "link-arg=-fuse-ld=lld"]` for Linux targets. When lld is not
installed, all cargo commands that invoke the linker (clippy, doc, test, build)
fail. This module detects whether lld is available and returns environment variable
overrides that disable the custom linker settings, matching the pattern used in CI.
"""

from __future__ import annotations

import platform
import shutil


def _get_linux_target_triple() -> str:
    """Return the Rust target triple for the current Linux host."""
    machine = platform.machine()
    if machine == "x86_64":
        return "x86_64-unknown-linux-gnu"
    if machine == "aarch64":
        return "aarch64-unknown-linux-gnu"
    # Fallback for other architectures
    return f"{machine}-unknown-linux-gnu"


def _is_lld_available() -> bool:
    """Check if lld is available on the system."""
    return shutil.which("lld") is not None or shutil.which("ld.lld") is not None


def get_cargo_env() -> dict[str, str]:
    """Return environment variable overrides for cargo commands.

    If the current platform is Linux and lld is not available, returns env vars
    that override the linker and rustflags settings from .cargo/config.toml.
    Otherwise returns an empty dict (no overrides needed).

    Returns:
        Dictionary of environment variables to merge into the subprocess env.
    """
    system = platform.system()

    if system != "Linux":
        return {}

    if _is_lld_available():
        return {}

    target_triple = _get_linux_target_triple()
    env_prefix = f"CARGO_TARGET_{target_triple.upper().replace('-', '_')}"

    print(f"NOTE: lld not found, using default system linker for {target_triple}")

    return {
        f"{env_prefix}_LINKER": "cc",
        f"{env_prefix}_RUSTFLAGS": "",
    }
