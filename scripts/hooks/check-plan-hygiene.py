#!/usr/bin/env python3
"""Keep PLAN.md limited to active and ordered future work."""

from __future__ import annotations

import re
import sys
from pathlib import Path

PLAN_PATH = Path("PLAN.md")
MAX_LINES = 120
MAX_WORDS = 1_000
ALLOWED_SECTIONS = frozenset({"future", "in progress"})
COMPLETED_TASK = re.compile(r"^\s*(?:[-*+]|\d+[.)])\s+\[[xX]\]")
HEADING = re.compile(r"^(#{2,6})\s+(.+?)\s*$")


def _display_path(path: Path) -> str:
    """Return a repository-relative path when possible."""
    try:
        return str(path.resolve().relative_to(Path.cwd().resolve()))
    except ValueError:
        return str(path)


def check_plan(path: Path = PLAN_PATH) -> list[str]:
    """Return actionable plan-hygiene violations."""
    display_path = _display_path(path)
    try:
        content = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as exc:
        return [f"{display_path}:0: cannot read file: {exc}"]

    lines = content.splitlines()
    issues: list[str] = []

    if len(lines) > MAX_LINES:
        issues.append(
            f"{display_path}:{MAX_LINES + 1}: PLAN.md has {len(lines)} lines; "
            f"limit active planning to {MAX_LINES} and move detail to its owner"
        )

    word_count = len(content.split())
    if word_count > MAX_WORDS:
        issues.append(
            f"{display_path}:1: PLAN.md has {word_count} words; limit active planning "
            f"to {MAX_WORDS} and move detail to its owner"
        )

    for line_number, line in enumerate(lines, start=1):
        if COMPLETED_TASK.match(line):
            issues.append(
                f"{display_path}:{line_number}: completed task belongs in a progress/session "
                "record, not PLAN.md"
            )

        heading_match = HEADING.match(line)
        if heading_match is None:
            continue
        heading_level = len(heading_match.group(1))
        heading = heading_match.group(2).strip().casefold()
        if heading_level == 2 and heading not in ALLOWED_SECTIONS:
            issues.append(
                f"{display_path}:{line_number}: '{heading_match.group(2)}' is not an active "
                "queue section; use only 'In progress' or 'Future' and move the content to "
                "progress/, an issue, documentation, or an Agent Skill"
            )

    return issues


def main() -> int:
    """Validate the repository plan."""
    # PLAN.md is an intentionally ignored local agent artifact. Fresh clones and
    # release packages do not contain it, so the always-run hook is a no-op there.
    if not PLAN_PATH.exists():
        return 0
    issues = check_plan()
    for issue in issues:
        print(issue, file=sys.stderr)
    return 1 if issues else 0


if __name__ == "__main__":
    sys.exit(main())
