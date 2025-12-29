#!/usr/bin/env python3
"""Run actionlint on GitHub Actions workflow files.

This hook validates GitHub Actions workflow files using actionlint,
catching common errors like:
- Invalid workflow syntax
- Unknown action inputs
- Type mismatches in expressions
- Deprecated features

Cross-platform: Works on Linux, macOS, and Windows.
Gracefully skips if actionlint is not installed.
"""
from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path


def main() -> int:
    """Run actionlint on the provided workflow files."""
    # Find actionlint executable
    actionlint = shutil.which("actionlint")
    if actionlint is None:
        print(
            "Warning: actionlint not found. "
            "Install from: https://github.com/rhysd/actionlint",
            file=sys.stderr,
        )
        print("  - Linux/macOS: brew install actionlint", file=sys.stderr)
        print("  - Windows: scoop install actionlint", file=sys.stderr)
        print(
            "  - Or: go install github.com/rhysd/actionlint/cmd/actionlint@latest",
            file=sys.stderr,
        )
        print("Skipping GitHub Actions workflow validation.", file=sys.stderr)
        return 0  # Don't fail if tool not available (cross-platform compatibility)

    # Get workflow files from command line args
    files = sys.argv[1:] if len(sys.argv) > 1 else []

    if not files:
        # No files provided, check all workflows
        repo_root = Path(__file__).parent.parent.parent
        workflows_dir = repo_root / ".github" / "workflows"
        if workflows_dir.exists():
            files = [
                str(f)
                for f in workflows_dir.iterdir()
                if f.suffix in (".yml", ".yaml")
            ]

    if not files:
        return 0  # No workflow files to check

    # Run actionlint on each file
    # actionlint can check multiple files at once
    cmd = [actionlint, "-color"]
    cmd.extend(files)

    # Use check=True to let subprocess handle errors naturally.
    # Output streams flow directly to terminal (no capture needed for linters).
    # Since shutil.which() already verified the executable exists, we don't
    # need redundant FileNotFoundError handling — any OSError here would
    # indicate a genuine system issue that should propagate as an error.
    result = subprocess.run(cmd, check=False)
    return result.returncode


if __name__ == "__main__":
    sys.exit(main())
