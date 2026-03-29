#!/usr/bin/env python3
"""Enforce the unreleased code rule in CHANGELOG.md.

The rule: Never add separate 'Fixed' or 'Changed' entries for code that
has not yet been released. Fixes to unreleased features should be folded
into the existing 'Added' entry describing that feature. The changelog
should describe the final shipped state, not intermediate development
history.

Exception: '### Changed' entries that ALL start with '**Breaking:**' are
legitimate (they document new enum variants or API changes affecting
already-released types).

Cross-platform: Works on Linux, macOS, and Windows.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path


def check_file(filepath: Path, repo_root: Path | None = None) -> bool:
    """Check CHANGELOG.md for unreleased code rule violations.

    Returns True if the file passes (no violations found).
    When repo_root is provided, paths in output are relative to it.
    """
    if repo_root is not None:
        try:
            display_path = filepath.relative_to(repo_root)
        except ValueError:
            display_path = filepath
    else:
        display_path = filepath

    try:
        content = filepath.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as e:
        print(f"{display_path}:0: cannot read file: {e}", file=sys.stderr)
        return False

    lines = content.splitlines()

    # Find the [Unreleased] section
    unreleased_start = None
    unreleased_end = None
    for i, line in enumerate(lines):
        if re.match(r"^## \[Unreleased\]", line):
            unreleased_start = i
        elif unreleased_start is not None and re.match(r"^## \[", line):
            unreleased_end = i
            break

    if unreleased_start is None:
        # No [Unreleased] section -- nothing to check
        return True

    if unreleased_end is None:
        unreleased_end = len(lines)

    # Parse subsections within [Unreleased]
    has_added = False
    fixed_line = 0
    changed_line = 0
    has_fixed = False
    has_non_breaking_changed = False

    # Track current subsection for entry analysis
    current_subsection = None
    changed_entries: list[str] = []

    for i in range(unreleased_start + 1, unreleased_end):
        line = lines[i]
        subsection_match = re.match(r"^### (.+)$", line)
        if subsection_match:
            # Before switching subsections, evaluate the previous one
            if current_subsection == "Changed":
                # Check if ALL entries start with **Breaking:**
                non_breaking = [
                    e for e in changed_entries
                    if not e.lstrip("- ").startswith("**Breaking:**")
                ]
                if non_breaking:
                    has_non_breaking_changed = True

            subsection_name = subsection_match.group(1).strip()
            current_subsection = subsection_name
            changed_entries = []

            if subsection_name == "Added":
                has_added = True
            elif subsection_name == "Fixed":
                has_fixed = True
                fixed_line = i + 1
            elif subsection_name == "Changed":
                changed_line = i + 1
        elif current_subsection == "Changed" and line.strip().startswith("- "):
            changed_entries.append(line.strip())

    # Evaluate the last subsection if it was Changed
    if current_subsection == "Changed":
        non_breaking = [
            e for e in changed_entries
            if not e.lstrip("- ").startswith("**Breaking:**")
        ]
        if non_breaking:
            has_non_breaking_changed = True

    # Check for violations
    violations_found = False

    if has_added and has_fixed:
        print(
            f"{display_path}:{fixed_line}: '### Fixed' subsection found "
            f"alongside '### Added' in [Unreleased] -- fixes to unreleased "
            f"features should be folded into the existing Added entry. "
            f"The changelog should describe the final shipped state, not "
            f"intermediate development history.",
            file=sys.stderr,
        )
        violations_found = True

    if has_added and has_non_breaking_changed:
        print(
            f"{display_path}:{changed_line}: '### Changed' subsection with "
            f"non-Breaking entries found alongside '### Added' in "
            f"[Unreleased] -- changes to unreleased features should be "
            f"folded into the existing Added entry. Only **Breaking:** "
            f"entries (for already-released types) belong in Changed.",
            file=sys.stderr,
        )
        violations_found = True

    return not violations_found


def main() -> int:
    repo_root = Path(__file__).resolve().parent.parent.parent
    changelog = repo_root / "CHANGELOG.md"

    if not changelog.is_file():
        # No CHANGELOG.md -- nothing to check
        return 0

    if not check_file(changelog, repo_root):
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
