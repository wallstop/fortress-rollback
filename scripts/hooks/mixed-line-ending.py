#!/usr/bin/env python3
"""Fix line endings to LF (Unix-style). Cross-platform."""

import sys
from pathlib import Path


def fix_file(filepath: str) -> bool:
    """Convert all line endings to LF. Returns True if modified."""
    path = Path(filepath)
    try:
        content = path.read_bytes()

        # Convert CRLF and CR to LF
        fixed = content.replace(b"\r\n", b"\n").replace(b"\r", b"\n")

        if fixed != content:
            path.write_bytes(fixed)
            print(f"Fixed line endings: {filepath}")
            return True

        return False
    except OSError:
        return False


def main() -> int:
    if len(sys.argv) < 2:
        return 0

    modified = False
    for filepath in sys.argv[1:]:
        if fix_file(filepath):
            modified = True

    return 1 if modified else 0


if __name__ == "__main__":
    sys.exit(main())
