#!/usr/bin/env python3
"""Cross-platform rustfmt wrapper for pre-commit hooks.

Runs `rustfmt` on the Rust files selected by pre-commit, then stages ONLY the
staged Rust files that rustfmt actually modified. This is careful not to stage other unstaged
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


def normalize_rust_files(files: list[str]) -> list[str]:
    """Return unique existing Rust file paths, preserving order."""
    normalized: list[str] = []
    seen: set[str] = set()
    for file in files:
        path = Path(file)
        if file in seen or path.suffix != ".rs" or not path.exists():
            continue
        normalized.append(file)
        seen.add(file)
    return normalized


def run_rustfmt(files: list[str]) -> bool:
    """Run rustfmt to auto-fix selected files. Returns True on success."""
    if not files:
        return True

    print(f"Running rustfmt on {len(files)} file(s)...")
    result = subprocess.run(
        ["rustfmt", "--edition", "2021", "--config", "skip_children=true", *files],
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
        print("WARNING: Could not stage formatted files", file=sys.stderr)
        return False

    for f in files_to_stage:
        print(f"  staged: {f}")

    return True


def main() -> int:
    """Run cargo fmt to auto-fix and stage only files that were actually modified. Returns exit code."""
    try:
        # Pre-commit passes the selected files. Fallback to staged files when
        # the script is run directly.
        staged_files = get_staged_rust_files()
        files_to_format = normalize_rust_files(sys.argv[1:] or staged_files)
        staged_files_to_track = [file for file in files_to_format if file in staged_files]
        hashes_before = get_file_hashes(staged_files_to_track)

        # Run rustfmt to fix formatting
        if not run_rustfmt(files_to_format):
            print("\nERROR: rustfmt failed.", file=sys.stderr)
            return 1

        # Compare hashes to find tracked staged files that rustfmt modified.
        hashes_after = get_file_hashes(staged_files_to_track)
        files_modified = [
            f for f in staged_files_to_track
            if hashes_before.get(f) != hashes_after.get(f)
        ]

        # Stage only the files that cargo fmt modified
        if not stage_modified_files(files_modified):
            print("\nERROR: Failed to stage formatted files.", file=sys.stderr)
            return 1

        return 0

    except FileNotFoundError as e:
        cmd = str(e).split("'")[1] if "'" in str(e) else "command"
        if "rustfmt" in cmd.lower():
            print("ERROR: rustfmt not found. Is the rustfmt component installed?", file=sys.stderr)
            print("  Install from: https://rustup.rs/", file=sys.stderr)
        elif "git" in cmd.lower():
            print("ERROR: git not found. Is Git installed?", file=sys.stderr)
        else:
            print(f"ERROR: {cmd} not found.", file=sys.stderr)
        return 1
    except Exception as e:
        print(f"ERROR: Failed to run cargo fmt: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
