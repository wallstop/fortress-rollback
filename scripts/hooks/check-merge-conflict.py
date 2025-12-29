#!/usr/bin/env python3
"""Check for merge conflict markers in files. Cross-platform."""

import re
import sys
from pathlib import Path

CONFLICT_PATTERNS = [
    re.compile(rb"^<<<<<<<\s"),
    re.compile(rb"^>>>>>>>\s"),
    re.compile(rb"^=======\s*$"),
]


def check_file(filepath: str) -> bool:
    """Check if file contains merge conflict markers. Returns True if clean."""
    path = Path(filepath)
    try:
        content = path.read_bytes()
        for i, line in enumerate(content.splitlines(), 1):
            for pattern in CONFLICT_PATTERNS:
                if pattern.match(line):
                    print(f"Merge conflict in {filepath}:{i}")
                    return False
        return True
    except OSError:
        return True  # Skip files we can't read


def main() -> int:
    if len(sys.argv) < 2:
        return 0

    all_clean = True
    for filepath in sys.argv[1:]:
        if not check_file(filepath):
            all_clean = False

    return 0 if all_clean else 1


if __name__ == "__main__":
    sys.exit(main())
