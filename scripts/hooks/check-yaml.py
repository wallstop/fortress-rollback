#!/usr/bin/env python3
"""Validate YAML files. Cross-platform."""

import sys
from pathlib import Path

try:
    import yaml
    HAS_YAML = True
except ImportError:
    HAS_YAML = False


def check_file(filepath: str) -> bool:
    """Check if YAML file is valid. Returns True if valid."""
    if not HAS_YAML:
        # Skip if PyYAML not installed
        return True

    path = Path(filepath)
    try:
        content = path.read_text(encoding="utf-8")
        yaml.safe_load(content)
        return True
    except yaml.YAMLError as e:
        print(f"YAML error in {filepath}: {e}")
        return False
    except OSError as e:
        print(f"Cannot read {filepath}: {e}")
        return False


def main() -> int:
    if len(sys.argv) < 2:
        return 0

    if not HAS_YAML:
        print("Warning: PyYAML not installed, skipping YAML validation")
        return 0

    all_valid = True
    for filepath in sys.argv[1:]:
        if not check_file(filepath):
            all_valid = False

    return 0 if all_valid else 1


if __name__ == "__main__":
    sys.exit(main())
