#!/usr/bin/env python3
"""Validate JSON files. Cross-platform."""

import json
import sys
from pathlib import Path


def check_file(filepath: str) -> bool:
    """Check if JSON file is valid. Returns True if valid."""
    path = Path(filepath)
    try:
        content = path.read_text(encoding="utf-8")
        json.loads(content)
        return True
    except json.JSONDecodeError as e:
        print(f"JSON error in {filepath}: {e}")
        return False
    except OSError as e:
        print(f"Cannot read {filepath}: {e}")
        return False


def main() -> int:
    if len(sys.argv) < 2:
        return 0
    
    all_valid = True
    for filepath in sys.argv[1:]:
        if not check_file(filepath):
            all_valid = False
    
    return 0 if all_valid else 1


if __name__ == "__main__":
    sys.exit(main())
