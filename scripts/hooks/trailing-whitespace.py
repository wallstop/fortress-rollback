#!/usr/bin/env python3
"""Trim trailing whitespace from files. Cross-platform."""

import sys
from pathlib import Path


def fix_file(filepath: str) -> bool:
    """Remove trailing whitespace from a file. Returns True if modified."""
    path = Path(filepath)
    try:
        content = path.read_text(encoding="utf-8")
        lines = content.splitlines(keepends=True)

        fixed_lines = []
        modified = False
        for line in lines:
            # Preserve line ending
            ending = ""
            if line.endswith("\r\n"):
                ending = "\r\n"
                line = line[:-2]
            elif line.endswith("\n"):
                ending = "\n"
                line = line[:-1]
            elif line.endswith("\r"):
                ending = "\r"
                line = line[:-1]

            stripped = line.rstrip()
            if stripped != line:
                modified = True
            fixed_lines.append(stripped + ending)

        if modified:
            path.write_text("".join(fixed_lines), encoding="utf-8")
            print(f"Fixed: {filepath}")

        return modified
    except (OSError, UnicodeDecodeError):
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
