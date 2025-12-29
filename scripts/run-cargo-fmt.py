#!/usr/bin/env python3
"""
Cross-platform cargo fmt wrapper for pre-commit hooks.

Runs `cargo fmt --check` to verify code formatting.
Works on Windows (PowerShell/cmd), macOS, and Linux.
"""

import subprocess
import sys


def main() -> int:
    """Run cargo fmt --check and return exit code."""
    try:
        # Run cargo fmt --check (fails if formatting needed)
        result = subprocess.run(
            ["cargo", "fmt", "--", "--check"],
            capture_output=False,
        )

        if result.returncode != 0:
            print("\nERROR: Code is not formatted. Run 'cargo fmt' to fix.")

        return result.returncode

    except FileNotFoundError:
        print("ERROR: cargo not found. Is Rust installed?")
        print("  Install from: https://rustup.rs/")
        return 1
    except Exception as e:
        print(f"ERROR: Failed to run cargo fmt: {e}")
        return 1


if __name__ == "__main__":
    sys.exit(main())
