#!/usr/bin/env python3
"""Validate JSON files. Cross-platform. Supports JSONC (JSON with Comments)."""

import json
import re
import sys
from pathlib import Path


def strip_jsonc_comments(content: str) -> str:
    """
    Strip comments from JSONC content.

    Handles:
    - Single-line comments: // comment
    - Multi-line comments: /* comment */
    - Preserves strings that contain // or /*
    """
    result = []
    i = 0
    in_string = False
    escape_next = False

    while i < len(content):
        char = content[i]

        if escape_next:
            result.append(char)
            escape_next = False
            i += 1
            continue

        if char == "\\" and in_string:
            result.append(char)
            escape_next = True
            i += 1
            continue

        if char == '"' and not escape_next:
            in_string = not in_string
            result.append(char)
            i += 1
            continue

        if not in_string:
            # Check for single-line comment
            if content[i : i + 2] == "//":
                # Skip to end of line
                newline = content.find("\n", i)
                if newline == -1:
                    break
                i = newline
                continue

            # Check for multi-line comment
            if content[i : i + 2] == "/*":
                # Skip to end of comment
                end = content.find("*/", i + 2)
                if end == -1:
                    break
                i = end + 2
                continue

        result.append(char)
        i += 1

    return "".join(result)


def check_file(filepath: str) -> bool:
    """Check if JSON/JSONC file is valid. Returns True if valid."""
    path = Path(filepath)
    try:
        content = path.read_text(encoding="utf-8")

        # Strip BOM if present
        if content.startswith("\ufeff"):
            content = content[1:]

        # Strip JSONC comments for .json files (many tools use JSONC)
        content = strip_jsonc_comments(content)

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
