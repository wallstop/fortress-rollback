#!/usr/bin/env python3
"""
Cross-platform cargo clippy wrapper for pre-commit hooks.

Runs `cargo clippy --all-targets` with warnings as errors.
Works on Windows (PowerShell/cmd), macOS, and Linux.
"""

import subprocess
import sys


def main() -> int:
    """Run cargo clippy and return exit code."""
    try:
        # Run cargo clippy with warnings as errors
        result = subprocess.run(
            ["cargo", "clippy", "--all-targets", "--", "-D", "warnings"],
            capture_output=False,
        )

        if result.returncode != 0:
            print("\nERROR: Clippy found issues. Fix the warnings above.")

        return result.returncode

    except FileNotFoundError:
        print("ERROR: cargo not found. Is Rust installed?")
        print("  Install from: https://rustup.rs/")
        return 1
    except Exception as e:
        print(f"ERROR: Failed to run cargo clippy: {e}")
        return 1


if __name__ == "__main__":
    sys.exit(main())
