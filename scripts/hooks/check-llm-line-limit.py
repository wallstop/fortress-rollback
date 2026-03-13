#!/usr/bin/env python3
"""Enforce a hard 300-line limit on all .md files under .llm/.

Cross-platform: Works on Linux, macOS, and Windows.
"""
from __future__ import annotations

import sys
from pathlib import Path

MAX_LINES = 300


def find_llm_md_files(repo_root: Path) -> list[Path]:
    """Find all .md files under the .llm/ directory."""
    llm_dir = repo_root / ".llm"
    if not llm_dir.is_dir():
        return []
    return sorted(llm_dir.rglob("*.md"))


def check_file(filepath: Path, repo_root: Path) -> bool:
    """Check if file exceeds the line limit. Returns True if within limit."""
    try:
        content = filepath.read_text(encoding="utf-8")
        line_count = len(content.splitlines())
        if line_count > MAX_LINES:
            rel = filepath.relative_to(repo_root)
            over = line_count - MAX_LINES
            print(
                f"FAIL: {rel} has {line_count} lines "
                f"({over} over the {MAX_LINES}-line limit)",
                file=sys.stderr,
            )
            return False
        return True
    except OSError as e:
        print(f"Cannot read {filepath}: {e}", file=sys.stderr)
        return False


def main() -> int:
    repo_root = Path(__file__).resolve().parent.parent.parent
    md_files = find_llm_md_files(repo_root)

    if not md_files:
        return 0

    all_ok = True
    for filepath in md_files:
        if not check_file(filepath, repo_root):
            all_ok = False

    if not all_ok:
        print(
            f"\nAll .md files under .llm/ must be {MAX_LINES} lines or fewer.",
            file=sys.stderr,
        )
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
