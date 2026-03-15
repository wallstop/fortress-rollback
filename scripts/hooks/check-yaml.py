#!/usr/bin/env python3
"""Validate YAML files. Cross-platform."""

import sys
from pathlib import Path

try:
    import yaml
    HAS_YAML = True
except ImportError:
    HAS_YAML = False


def _display_path(filepath: str | Path) -> str:
    """Convert a file path to a relative display path.

    Pre-commit sets CWD to the repo root, so paths relative to CWD
    are also relative to the project root.
    """
    try:
        return str(Path(filepath).resolve().relative_to(Path.cwd().resolve()))
    except ValueError:
        return str(filepath)


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
        line = (e.problem_mark.line + 1) if hasattr(e, 'problem_mark') and e.problem_mark else 1
        print(f"{_display_path(filepath)}:{line}: YAML error: {e}", file=sys.stderr)
        return False
    except (OSError, UnicodeDecodeError) as e:
        print(f"{_display_path(filepath)}:0: cannot read file: {e}", file=sys.stderr)
        return False


def main() -> int:
    if len(sys.argv) < 2:
        return 0

    if not HAS_YAML:
        print("Skipping YAML validation: PyYAML not installed", file=sys.stderr)
        return 0

    all_valid = True
    for filepath in sys.argv[1:]:
        if not check_file(filepath):
            all_valid = False

    return 0 if all_valid else 1


if __name__ == "__main__":
    sys.exit(main())
