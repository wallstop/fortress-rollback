#!/usr/bin/env python3
"""Validate TOML files. Cross-platform."""

import sys
from pathlib import Path

# Python 3.11+ has tomllib built-in
try:
    import tomllib
    HAS_TOML = True
except ImportError:
    try:
        import tomli as tomllib
        HAS_TOML = True
    except ImportError:
        HAS_TOML = False


def check_file(filepath: str) -> bool:
    """Check if TOML file is valid. Returns True if valid."""
    if not HAS_TOML:
        return True

    path = Path(filepath)
    try:
        content = path.read_bytes()
        tomllib.loads(content.decode("utf-8"))
        return True
    except (ValueError, UnicodeDecodeError) as e:
        line = getattr(e, "lineno", 1) or 1
        print(f"{filepath}:{line}: TOML error: {e}", file=sys.stderr)
        return False
    except OSError as e:
        print(f"{filepath}:0: cannot read file: {e}", file=sys.stderr)
        return False


def main() -> int:
    if len(sys.argv) < 2:
        return 0

    if not HAS_TOML:
        print("Warning: tomllib/tomli not available, skipping TOML validation", file=sys.stderr)
        return 0

    all_valid = True
    for filepath in sys.argv[1:]:
        if not check_file(filepath):
            all_valid = False

    return 0 if all_valid else 1


if __name__ == "__main__":
    sys.exit(main())
