#!/usr/bin/env python3
"""
Cross-platform cargo fmt wrapper for pre-commit hooks.

Runs `cargo fmt` to auto-fix formatting, then stages ONLY the Rust files that
cargo fmt actually modified. This is careful not to stage other unstaged
changes the developer may have.

Works on Windows (PowerShell/cmd), macOS, and Linux.
"""

import subprocess
import sys


def get_unstaged_rust_files() -> set[str]:
    """Get set of currently unstaged Rust file paths."""
    result = subprocess.run(
        ["git", "diff", "--name-only"],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        return set()
    all_files = result.stdout.strip().splitlines() if result.stdout.strip() else []
    # Filter to only .rs files
    return {f for f in all_files if f.endswith(".rs")}


def run_cargo_fmt() -> bool:
    """Run cargo fmt to auto-fix formatting. Returns True on success."""
    print("Running cargo fmt...")
    result = subprocess.run(
        ["cargo", "fmt"],
        capture_output=False,
    )
    return result.returncode == 0


def stage_formatted_files(rust_files_before: set[str]) -> bool:
    """Stage only Rust files that cargo fmt modified (not other unstaged changes).

    Args:
        rust_files_before: Set of unstaged .rs files before cargo fmt ran.

    Returns:
        True on success, False on failure.
    """
    rust_files_after = get_unstaged_rust_files()

    # Files that became newly unstaged = files that cargo fmt touched
    # (These were either staged files that fmt modified, or new modifications)
    newly_unstaged = rust_files_after - rust_files_before

    if not newly_unstaged:
        return True  # Nothing new to stage

    files_to_stage = sorted(newly_unstaged)
    print(f"Staging {len(files_to_stage)} formatted file(s)...")

    stage_result = subprocess.run(
        ["git", "add", "--"] + files_to_stage,
        capture_output=False,
    )
    if stage_result.returncode != 0:
        print("WARNING: Could not stage formatted files")
        return False

    for f in files_to_stage:
        print(f"  staged: {f}")

    return True


def main() -> int:
    """Run cargo fmt to auto-fix and stage only newly formatted files. Returns exit code."""
    try:
        # Record unstaged Rust files before running fmt
        rust_files_before = get_unstaged_rust_files()

        # Run cargo fmt to fix formatting
        if not run_cargo_fmt():
            print("\nERROR: cargo fmt failed.")
            return 1

        # Stage only Rust files that cargo fmt modified
        if not stage_formatted_files(rust_files_before):
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
