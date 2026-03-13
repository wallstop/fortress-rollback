#!/usr/bin/env python3
"""
Pre-commit hook: detect #[track_caller] on async fn.

Rust does not support #[track_caller] on async functions (it is a no-op
or a compile error depending on the Rust version and lint configuration).
This fast grep-based check catches the mistake before a full compile.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path


# Matches #[track_caller] (possibly with leading whitespace or other attrs on
# the same line) followed by an async fn within a few lines.
_TRACK_CALLER_RE = re.compile(r"#\[track_caller\]")
_ASYNC_FN_RE = re.compile(r"\basync\s+fn\b")


def check_file(path: Path) -> list[str]:
    """Return a list of error messages for violations in *path*."""
    errors: list[str] = []
    try:
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    except OSError:
        return errors

    for i, line in enumerate(lines):
        stripped = line.strip()
        # Skip comments and doc comments
        if stripped.startswith("//"):
            continue
        if _TRACK_CALLER_RE.search(stripped):
            # Look ahead up to 5 lines for an async fn
            for j in range(i + 1, min(i + 6, len(lines))):
                next_line = lines[j].strip()
                if next_line.startswith("//"):
                    continue
                if _ASYNC_FN_RE.search(next_line):
                    errors.append(
                        f"{path}:{i + 1}: #[track_caller] on async fn "
                        f"(line {j + 1}) is not supported by Rust"
                    )
                    break
                # Stop lookahead if we hit a non-attribute, non-blank line
                # that isn't an async fn
                if next_line and not next_line.startswith("#["):
                    break
    return errors


def main() -> int:
    """Check all .rs files passed as arguments."""
    errors: list[str] = []
    for arg in sys.argv[1:]:
        path = Path(arg)
        if path.suffix == ".rs" and path.is_file():
            errors.extend(check_file(path))

    if errors:
        print("ERROR: #[track_caller] cannot be used on async fn:")
        for err in errors:
            print(f"  {err}")
        print(
            "\nRust ignores or errors on #[track_caller] for async functions."
        )
        print("Remove the attribute or convert to a sync helper function.")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
