#!/usr/bin/env python3
"""Fix line endings to LF (Unix-style). Cross-platform."""

import sys
from pathlib import Path


def _display_path(filepath: str | Path) -> str:
    """Convert a file path to a relative display path.

    Pre-commit sets CWD to the repo root, so paths relative to CWD
    are also relative to the project root.
    """
    try:
        return str(Path(filepath).resolve().relative_to(Path.cwd().resolve()))
    except ValueError:
        return str(filepath)


def fix_file(filepath: str) -> bool | None:
    """Convert all line endings to LF.

    Returns True if modified, False if unchanged, None on error.
    """
    path = Path(filepath)
    try:
        content = path.read_bytes()

        # Convert CRLF and CR to LF
        fixed = content.replace(b"\r\n", b"\n").replace(b"\r", b"\n")

        if fixed != content:
            path.write_bytes(fixed)
            print(f"Fixed line endings: {_display_path(filepath)}")
            return True

        return False
    except OSError as exc:
        print(f"{_display_path(filepath)}:0: cannot read file: {exc}", file=sys.stderr)
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
