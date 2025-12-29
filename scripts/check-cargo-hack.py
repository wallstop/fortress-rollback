#!/usr/bin/env python3
"""
Cross-platform cargo-hack feature check for pre-commit hooks.

Checks if cargo-hack is installed and runs feature powerset check.
Works on Windows, macOS, and Linux.
"""

import subprocess
import shutil
import sys


def main() -> int:
    """Run cargo hack feature powerset check if available."""
    # Check if cargo-hack is installed
    if not shutil.which("cargo-hack"):
        # cargo-hack not installed, skip silently (it's optional)
        print("Note: cargo-hack not installed, skipping feature check")
        return 0

    try:
        result = subprocess.run(
            [
                "cargo",
                "hack",
                "check",
                "--feature-powerset",
                "--exclude-features",
                "z3-verification,graphical-examples",
            ],
            capture_output=False,
        )
        return result.returncode

    except FileNotFoundError:
        print("ERROR: cargo not found. Is Rust installed?")
        return 1
    except Exception as e:
        print(f"ERROR: Failed to run cargo hack: {e}")
        return 1


if __name__ == "__main__":
    sys.exit(main())
