#!/usr/bin/env python3
"""Ensure files end with a single newline. Cross-platform."""

import sys
from pathlib import Path


def fix_file(filepath: str) -> bool:
    """Ensure file ends with exactly one newline. Returns True if modified."""
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
