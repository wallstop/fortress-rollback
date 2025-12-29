#!/usr/bin/env python3
"""Run markdownlint on markdown files with autofix.

This hook runs markdownlint-cli to check markdown files for common issues.
It uses the project's .markdownlint.json configuration and ignores
generated/scratch directories.

The --fix flag automatically corrects fixable issues like:
- Trailing whitespace
- Missing blank lines around fenced code blocks
- Missing blank lines around lists
- Inconsistent list markers

Cross-platform: Works on Linux, macOS, and Windows.
"""
from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path


def main() -> int:
    """Run markdownlint on the provided files."""
    # Find markdownlint executable
    markdownlint = shutil.which("markdownlint")
    if markdownlint is None:
        print("markdownlint not found. Install with: npm install -g markdownlint-cli")
        print("Skipping markdown lint check.")
        return 0  # Don't fail if tool not available (Windows compatibility)

    # Build command with project configuration
    repo_root = Path(__file__).parent.parent.parent
    config_file = repo_root / ".markdownlint.json"

    cmd = [
        markdownlint,
        "--config",
        str(config_file),
        "--fix",  # Auto-fix fixable issues
    ]

    # Add files passed by pre-commit (or use all if none provided)
    files = sys.argv[1:] if len(sys.argv) > 1 else []

    # Filter out ignored paths (pre-commit should handle this, but be defensive)
    ignored_patterns = [
        "target/",
        "fuzz/target/",
        "loom-tests/target/",
        "progress/",
        "PLAN.md",
    ]

    filtered_files = []
    for f in files:
        skip = False
        for pattern in ignored_patterns:
            if pattern in f or f.endswith(pattern.rstrip("/")):
                skip = True
                break
        if not skip:
            filtered_files.append(f)

    if not filtered_files:
        return 0  # No files to check

    cmd.extend(filtered_files)

    # markdownlint verified to exist via shutil.which() above.
    # Output flows directly to terminal (no capture needed for linters).
    result = subprocess.run(cmd, check=False)
    return result.returncode


if __name__ == "__main__":
    sys.exit(main())
