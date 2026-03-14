#!/usr/bin/env python3
"""
Pre-commit hook: detect #[track_caller] on async fn.

Rust does not support #[track_caller] on async functions (it is a no-op
or a compile error depending on the Rust version and lint configuration).
This fast grep-based check catches the mistake before a full compile.

Known limitations (acceptable for a fast grep-based hook):
  - ``#[cfg_attr(test, track_caller)]`` is not detected (rare in practice).
  - ``#[track_caller]`` inside string literals may produce a false positive.
  - Multi-line block comments are not tracked; a single-line block comment
    containing the attribute (``/* #[track_caller] */``) is skipped, but
    multi-line block comments are not fully parsed.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path


# Matches #[track_caller] (possibly with leading whitespace or other attrs on
# the same line) followed by an async fn within a few lines.
_TRACK_CALLER_RE = re.compile(r"#\[track_caller\]")
_ASYNC_FN_RE = re.compile(r"\basync\s+(?:unsafe\s+)?fn\b")


def check_file(path: Path) -> list[str]:
    """Return a list of error messages for violations in *path*."""
    errors: list[str] = []
    try:
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    except OSError as exc:
        return [f"{path}:0: cannot read file: {exc}"]

    for i, line in enumerate(lines):
        stripped = line.strip()
        # Skip comments and doc comments
        if stripped.startswith("//"):
            continue
        # Skip single-line block comments
        if stripped.startswith("/*") and stripped.endswith("*/"):
            continue
        if _TRACK_CALLER_RE.search(stripped):
            # Check if async fn is on the same line as #[track_caller]
            if _ASYNC_FN_RE.search(stripped):
                errors.append(
                    f"{path}:{i + 1}: #[track_caller] on async fn "
                    f"is not supported by Rust"
                )
                continue
            # Look ahead up to 5 lines for an async fn
            for j in range(i + 1, min(i + 6, len(lines))):
                next_line = lines[j].strip()
                if next_line.startswith("//"):
                    continue
                # Skip single-line block comments in lookahead
                if next_line.startswith("/*") and next_line.endswith("*/"):
                    continue
                if _ASYNC_FN_RE.search(next_line):
                    errors.append(
                        f"{path}:{j + 1}: #[track_caller] (line {i + 1}) "
                        f"on async fn is not supported by Rust"
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
        if path.suffix == ".rs":
            errors.extend(check_file(path))

    if errors:
        print("ERROR: #[track_caller] cannot be used on async fn:", file=sys.stderr)
        for err in errors:
            print(err, file=sys.stderr)
        print(
            "\nRust ignores or errors on #[track_caller] for async functions.",
            file=sys.stderr,
        )
        print("Remove the attribute or convert to a sync helper function.", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
