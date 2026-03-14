#!/usr/bin/env python3
"""Trim trailing whitespace from files. Cross-platform."""

import sys
from pathlib import Path


def fix_file(filepath: str) -> bool | None:
    """Remove trailing whitespace from a file.

    Returns True if modified, False if unchanged, None on error.
    """
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
    except (OSError, UnicodeDecodeError) as exc:
        print(f"{filepath}:0: cannot read file: {exc}", file=sys.stderr)
        return None


def main() -> int:
    if len(sys.argv) < 2:
        return 0

    had_error = False
    modified = False
    for filepath in sys.argv[1:]:
        result = fix_file(filepath)
        if result is True:
            modified = True
        elif result is None:
            had_error = True

    return 1 if modified or had_error else 0


if __name__ == "__main__":
    sys.exit(main())
