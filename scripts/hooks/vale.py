#!/usr/bin/env python3
"""Run Vale prose linter on documentation files.

This hook validates prose quality in Markdown files using Vale,
checking for:
- Passive voice
- Weasel words
- Cliches and overused phrases
- Clarity and readability issues

Cross-platform: Works on Linux, macOS, and Windows.
Gracefully skips if Vale is not installed.

Note: This hook is advisory-only (always returns 0) since Vale findings
are suggestions for improving prose quality, not blocking errors.
"""
from __future__ import annotations

import shutil
import subprocess
import sys


def main() -> int:
    """Run Vale on the provided documentation files."""
    # Find vale executable
    vale = shutil.which("vale")
    if vale is None:
        print(
            "Warning: Vale not found. "
            "Install from: https://vale.sh/docs/vale-cli/installation/",
            file=sys.stderr,
        )
        print("  - Linux/macOS: brew install vale", file=sys.stderr)
        print("  - Windows: scoop install vale or choco install vale", file=sys.stderr)
        print("Skipping prose linting.", file=sys.stderr)
        return 0  # Don't fail if tool not available

    # Get files from command line args
    files = sys.argv[1:] if len(sys.argv) > 1 else []

    if not files:
        return 0  # No files to check

    # Run Vale on each file
    # Vale can check multiple files at once
    cmd = [vale, "--config", ".vale.ini"]
    cmd.extend(files)

    # Run Vale and capture output
    # We always return 0 since Vale findings are suggestions, not errors
    result = subprocess.run(cmd, check=False)

    if result.returncode != 0:
        print(
            "\nWarning: Vale found prose suggestions above. "
            "These are advisory and won't block your commit.",
            file=sys.stderr,
        )

    # Always return 0 - Vale findings are suggestions, not blocking errors
    return 0


if __name__ == "__main__":
    sys.exit(main())
