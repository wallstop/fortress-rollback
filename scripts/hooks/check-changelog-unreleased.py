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

Exception: '### Fixed' entries that start with '**Pre-existing:**' are
legitimate (they document fixes to behavior that already shipped in a
released version, which the unreleased-code rule does not cover — the
marker is the author's self-declaration, mirroring '**Breaking:**').
Each accepted entry is surfaced with a non-failing 'note:' line on stdout
for reviewer visibility; the exit code is unaffected.

See `.agents/skills/fortress-development/SKILL.md` (section "Changelog Policy", "Unreleased code rule")
and `.agents/skills/changelog/SKILL.md` for the canonical
specification.

Assumes a single `## [Unreleased]` section; multiple Unreleased headers in
one file are not supported and would not be valid Keep-a-Changelog format
anyway.

Cross-platform: Works on Linux, macOS, and Windows.
"""
from __future__ import annotations

import re
import sys
from dataclasses import dataclass
from pathlib import Path

# Truncation width for entry summaries in diagnostics. Picked so a line
# fits comfortably in a terminal alongside the path:line: prefix.
_SUMMARY_WIDTH = 80

_RULE_REFERENCE = (
    ".agents/skills/fortress-development/SKILL.md \"Unreleased code rule\" / "
    ".agents/skills/changelog/SKILL.md"
)


@dataclass(frozen=True)
class _Entry:
    """A single bulleted entry inside a CHANGELOG subsection."""

    line_number: int  # 1-indexed file line number
    text: str  # Trimmed entry text including the leading dash


def _summarize(entry_text: str, width: int = _SUMMARY_WIDTH) -> str:
    """Return a single-line truncated summary of an entry for diagnostics."""
    # Collapse internal whitespace so multi-line wraps don't blow up the summary.
    summary = " ".join(entry_text.split())
    if len(summary) <= width:
        return summary
    return summary[: max(0, width - 1)].rstrip() + "..."


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
    fixed_entries: list[_Entry] = []
    changed_entries: list[_Entry] = []

    current_subsection: str | None = None
    current_entries: list[_Entry] = []

    for i in range(unreleased_start + 1, unreleased_end):
        line = lines[i]
        subsection_match = re.match(r"^### (.+)$", line)
        if subsection_match:
            # Save the entries that just finished
            if current_subsection == "Changed":
                changed_entries = current_entries
            elif current_subsection == "Fixed":
                fixed_entries = current_entries

            subsection_name = subsection_match.group(1).strip()
            current_subsection = subsection_name
            current_entries = []

            if subsection_name == "Added":
                has_added = True
            elif subsection_name == "Fixed":
                fixed_line = i + 1
            elif subsection_name == "Changed":
                changed_line = i + 1
        elif current_subsection in ("Changed", "Fixed") and line.lstrip().startswith(
            "- "
        ):
            current_entries.append(_Entry(line_number=i + 1, text=line.strip()))

    # Save the last subsection if it was a tracked one
    if current_subsection == "Changed":
        changed_entries = current_entries
    elif current_subsection == "Fixed":
        fixed_entries = current_entries

    non_breaking_changed = [
        entry
        for entry in changed_entries
        if not entry.text.lstrip("- ").startswith("**Breaking:**")
    ]

    # Fixed entries self-declared as fixing already-released behavior are
    # outside the unreleased-code rule's scope (mirrors the '**Breaking:**'
    # self-declaration for Changed).
    unmarked_fixed = [
        entry
        for entry in fixed_entries
        if not entry.text.lstrip("- ").startswith("**Pre-existing:**")
    ]
    preexisting_fixed = [
        entry
        for entry in fixed_entries
        if entry.text.lstrip("- ").startswith("**Pre-existing:**")
    ]

    # The marker is load-bearing only when an Added section is present
    # (without one, Fixed entries pass regardless). Surface each accepted
    # self-declaration as a non-failing note so reviewers can verify the
    # claim; this never affects the exit code.
    if has_added and preexisting_fixed:
        for entry in preexisting_fixed:
            print(
                f"{display_path}:{entry.line_number}: note: accepting "
                f"'**Pre-existing:**' Fixed entry on the author's "
                f"self-declaration (verify it fixes behavior that shipped "
                f"in a released version): {_summarize(entry.text)}"
            )

    violations_found = False

    if has_added and unmarked_fixed:
        print(
            f"{display_path}:{fixed_line}: '### Fixed' subsection found "
            f"alongside '### Added' in [Unreleased] -- fixes to unreleased "
            f"features should be folded into the existing Added entry. "
            f"The changelog should describe the final shipped state, not "
            f"intermediate development history.",
            file=sys.stderr,
        )
        for entry in unmarked_fixed:
            print(
                f"{display_path}:{entry.line_number}: offending Fixed entry: "
                f"{_summarize(entry.text)}",
                file=sys.stderr,
            )
        print(
            f"  remedy: fold each entry into the matching '### Added' entry "
            f"that introduces the affected feature and delete the '### Fixed' "
            f"subsection, OR prefix the entry with '**Pre-existing:**' if it "
            f"fixes behavior that already shipped in a released version.",
            file=sys.stderr,
        )
        print(f"  see: {_RULE_REFERENCE}", file=sys.stderr)
        violations_found = True

    if has_added and non_breaking_changed:
        print(
            f"{display_path}:{changed_line}: '### Changed' subsection with "
            f"non-Breaking entries found alongside '### Added' in "
            f"[Unreleased] -- changes to unreleased features should be "
            f"folded into the existing Added entry. Only **Breaking:** "
            f"entries (for already-released types) belong in Changed.",
            file=sys.stderr,
        )
        for entry in non_breaking_changed:
            print(
                f"{display_path}:{entry.line_number}: offending Changed "
                f"entry: {_summarize(entry.text)}",
                file=sys.stderr,
            )
        print(
            f"  remedy: fold each entry into the existing '### Added' entry "
            f"for the same feature, OR prefix the entry with '**Breaking:**' "
            f"if it is a breaking change to an already-released type.",
            file=sys.stderr,
        )
        print(f"  see: {_RULE_REFERENCE}", file=sys.stderr)
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
