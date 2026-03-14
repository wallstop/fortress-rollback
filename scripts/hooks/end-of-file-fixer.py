#!/usr/bin/env python3
"""Ensure files end with a single newline. Cross-platform."""

import sys
from pathlib import Path


def fix_file(filepath: str) -> bool | None:
    """Ensure file ends with exactly one newline.

    Returns True if modified, False if unchanged, None on error.
    """
    path = Path(filepath)
    try:
        content = path.read_bytes()
        if not content:
            return False

        # Remove all trailing whitespace/newlines, then add exactly one \n
        stripped = content.rstrip(b"\r\n \t")
        fixed = stripped + b"\n"

        if fixed != content:
            path.write_bytes(fixed)
            print(f"Fixed: {filepath}")
            return True

        return False
    except OSError as exc:
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
