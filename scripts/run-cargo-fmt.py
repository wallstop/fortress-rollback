#!/usr/bin/env python3
"""
Cross-platform cargo fmt wrapper for pre-commit hooks.

Runs `cargo fmt` to auto-fix formatting, then stages ONLY the Rust files that
cargo fmt actually modified. This is careful not to stage other unstaged
changes the developer may have.

Works on Windows (PowerShell/cmd), macOS, and Linux.
"""

import hashlib
import subprocess
import sys
from pathlib import Path


def get_staged_rust_files() -> list[str]:
    """Get list of staged Rust file paths."""
    result = subprocess.run(
        ["git", "diff", "--cached", "--name-only", "--diff-filter=ACMR"],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        return []
    all_files = result.stdout.strip().splitlines() if result.stdout.strip() else []
    return [f for f in all_files if f.endswith(".rs")]


def compute_file_hash(filepath: str) -> str | None:
    """Compute SHA-256 hash of a file's contents. Returns None if file doesn't exist."""
    try:
        content = Path(filepath).read_bytes()
        return hashlib.sha256(content).hexdigest()
    except (FileNotFoundError, PermissionError, OSError):
        return None


def get_file_hashes(files: list[str]) -> dict[str, str | None]:
    """Get content hashes for a list of files."""
    return {f: compute_file_hash(f) for f in files}


def run_cargo_fmt() -> bool:
    """Run cargo fmt to auto-fix formatting. Returns True on success."""
    print("Running cargo fmt...")
    result = subprocess.run(
        ["cargo", "fmt"],
        capture_output=False,
    )
    return result.returncode == 0


def stage_modified_files(files_to_stage: list[str]) -> bool:
    """Stage the specified files.

    Args:
        files_to_stage: List of file paths to stage.

    Returns:
        True on success, False on failure.
    """
    if not files_to_stage:
        return True  # Nothing to stage

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
    """Run cargo fmt to auto-fix and stage only files that were actually modified. Returns exit code."""
    try:
        # Get staged Rust files and their content hashes before running fmt
        staged_files = get_staged_rust_files()
        hashes_before = get_file_hashes(staged_files)

        # Run cargo fmt to fix formatting
        if not run_cargo_fmt():
            print("\nERROR: cargo fmt failed.")
            return 1

        # Compare hashes to find files that cargo fmt actually modified
        hashes_after = get_file_hashes(staged_files)
        files_modified = [
            f for f in staged_files
            if hashes_before.get(f) != hashes_after.get(f)
        ]

        # Stage only the files that cargo fmt modified
        if not stage_modified_files(files_modified):
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
