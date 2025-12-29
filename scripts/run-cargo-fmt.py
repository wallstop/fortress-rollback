#!/usr/bin/env python3
"""
Cross-platform cargo fmt wrapper for pre-commit hooks.

Runs `cargo fmt` to auto-fix formatting, then stages modified files.
Works on Windows (PowerShell/cmd), macOS, and Linux.
"""

import subprocess
import sys


def run_cargo_fmt() -> bool:
    """Run cargo fmt to auto-fix formatting. Returns True on success."""
    print("Running cargo fmt...")
    result = subprocess.run(
        ["cargo", "fmt"],
        capture_output=False,
    )
    return result.returncode == 0


def stage_modified_files() -> bool:
    """Stage any files modified by cargo fmt. Returns True on success."""
    # Get list of modified (unstaged) files
    result = subprocess.run(
        ["git", "diff", "--name-only"],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print("WARNING: Could not check for modified files")
        return True  # Continue anyway

    modified_files = result.stdout.strip()
    if modified_files:
        print(f"Staging formatted files...")
        # Stage all modified files
        stage_result = subprocess.run(
            ["git", "add", "-u"],
            capture_output=False,
        )
        if stage_result.returncode != 0:
            print("WARNING: Could not stage modified files")
            return False
        print("Formatted files staged for commit.")

    return True


def main() -> int:
    """Run cargo fmt to auto-fix and stage changes. Returns exit code."""
    try:
        # Run cargo fmt to fix formatting
        if not run_cargo_fmt():
            print("\nERROR: cargo fmt failed.")
            return 1

        # Stage any modified files so they're included in the commit
        if not stage_modified_files():
            print("\nERROR: Failed to stage formatted files.")
            return 1

        return 0

    except FileNotFoundError as e:
        cmd = str(e).split("'")[1] if "'" in str(e) else "command"
        if "cargo" in cmd.lower():
            print("ERROR: cargo not found. Is Rust installed?")
            print("  Install from: https://rustup.rs/")
        elif "git" in cmd.lower():
            print("ERROR: git not found. Is Git installed?")
        else:
            print(f"ERROR: {cmd} not found.")
        return 1
    except Exception as e:
        print(f"ERROR: Failed to run cargo fmt: {e}")
        return 1


if __name__ == "__main__":
    sys.exit(main())
