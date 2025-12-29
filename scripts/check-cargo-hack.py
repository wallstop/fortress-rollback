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

    # cargo-hack verified to exist via shutil.which() above.
    # Output flows directly to terminal (no capture needed).
    result = subprocess.run(
        [
            "cargo",
            "hack",
            "check",
            "--feature-powerset",
            "--exclude-features",
            "z3-verification,graphical-examples",
        ],
        check=False,
    )
    return result.returncode


if __name__ == "__main__":
    sys.exit(main())
