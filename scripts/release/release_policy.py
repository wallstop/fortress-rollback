#!/usr/bin/env python3
"""Infer and enforce the minimum release bump from Unreleased notes."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

_STRICT_VERSION = re.compile(
    r"^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$"
)
_UNRELEASED_HEADING = re.compile(r"(?m)^## \[Unreleased\]\s*$")
_UNRELEASED_SECTION = re.compile(
    r"(?ms)^## \[Unreleased\]\s*\n(?P<notes>.*?)(?=^## \[)"
)
_SUBSECTION_HEADING = re.compile(r"^### (?P<name>.+?)\s*$")
_LIST_ENTRY = re.compile(r"^- (?P<text>.*)$")
_BREAKING_PREFIX = "**Breaking:**"

_SECTIONS = ("Added", "Changed", "Deprecated", "Removed", "Fixed", "Security")
_BUMP_RANK = {"patch": 0, "minor": 1, "major": 2}
_MAX_SEMVER_COMPONENT = (1 << 64) - 1
_MAX_SEMVER_COMPONENT_DIGITS = len(str(_MAX_SEMVER_COMPONENT))
_MAX_STABLE_VERSION_LENGTH = 3 * _MAX_SEMVER_COMPONENT_DIGITS + 2


class ReleasePolicyError(ValueError):
    """The changelog cannot support the requested release bump."""


def parse_stable_version(version: str) -> tuple[int, int, int]:
    """Parse strict stable SemVer whose components fit Cargo's u64 domain."""
    if len(version) > _MAX_STABLE_VERSION_LENGTH:
        raise ReleasePolicyError(
            "version exceeds the maximum length for u64 SemVer components"
        )
    match = _STRICT_VERSION.fullmatch(version)
    if match is None:
        raise ReleasePolicyError(f"version {version!r} is not strict X.Y.Z semver")

    parsed: list[int] = []
    for component in match.groups():
        if len(component) > _MAX_SEMVER_COMPONENT_DIGITS:
            raise ReleasePolicyError(
                f"version {version!r} has a component larger than u64"
            )
        value = int(component)
        if value > _MAX_SEMVER_COMPONENT:
            raise ReleasePolicyError(
                f"version {version!r} has a component larger than u64"
            )
        parsed.append(value)
    return parsed[0], parsed[1], parsed[2]


def _extract_unreleased_notes(changelog: str) -> str:
    """Return the single Unreleased section body."""
    if len(_UNRELEASED_HEADING.findall(changelog)) != 1:
        raise ReleasePolicyError(
            "CHANGELOG.md must contain exactly one Unreleased heading"
        )
    match = _UNRELEASED_SECTION.search(changelog)
    if match is None:
        raise ReleasePolicyError(
            "CHANGELOG.md Unreleased section has no following release heading"
        )
    notes = match.group("notes")
    if not notes.strip():
        raise ReleasePolicyError(
            "CHANGELOG.md Unreleased section has no release notes"
        )
    return notes


def minimum_bump(changelog: str, current_version: str) -> str:
    """Infer the minimum permitted bump from Keep a Changelog subsections."""
    major, _minor, _patch = parse_stable_version(current_version)
    notes = _extract_unreleased_notes(changelog)
    entry_counts = {section: 0 for section in _SECTIONS}
    seen_sections: set[str] = set()
    current_section: str | None = None
    current_entry = False
    has_breaking_entry = False

    for line_number, line in enumerate(notes.splitlines(), start=1):
        heading = _SUBSECTION_HEADING.fullmatch(line)
        if heading is not None:
            name = heading.group("name")
            if name not in entry_counts:
                raise ReleasePolicyError(
                    f"CHANGELOG.md Unreleased subsection {name!r} is unsupported; "
                    f"expected one of {', '.join(_SECTIONS)}"
                )
            if name in seen_sections:
                raise ReleasePolicyError(
                    f"CHANGELOG.md Unreleased subsection {name!r} appears more than once"
                )
            seen_sections.add(name)
            current_section = name
            current_entry = False
            continue
        stripped = line.strip()
        if not stripped:
            continue

        if current_section is None:
            raise ReleasePolicyError(
                "CHANGELOG.md Unreleased release notes must be under a supported "
                "Keep a Changelog subsection"
            )
        if "<!--" in stripped or "-->" in stripped:
            raise ReleasePolicyError(
                "CHANGELOG.md Unreleased HTML comments are not release notes "
                f"(line {line_number})"
            )
        if stripped.startswith("#"):
            raise ReleasePolicyError(
                "CHANGELOG.md Unreleased contains an unsupported nested heading "
                f"on line {line_number}"
            )

        entry = _LIST_ENTRY.fullmatch(line)
        if entry is not None:
            text = entry.group("text").strip()
            if not text:
                raise ReleasePolicyError(
                    "CHANGELOG.md Unreleased contains an empty list entry "
                    f"on line {line_number}"
                )
            if _BREAKING_PREFIX in text and not text.startswith(_BREAKING_PREFIX):
                raise ReleasePolicyError(
                    "CHANGELOG.md Unreleased **Breaking:** must begin a Changed "
                    "list entry"
                )
            if text.startswith(_BREAKING_PREFIX):
                if current_section != "Changed":
                    raise ReleasePolicyError(
                        "CHANGELOG.md Unreleased **Breaking:** entries are allowed "
                        "only under Changed"
                    )
                if not text.removeprefix(_BREAKING_PREFIX).strip():
                    raise ReleasePolicyError(
                        "CHANGELOG.md Unreleased **Breaking:** entry must describe "
                        "the breaking change"
                    )
                has_breaking_entry = True
            entry_counts[current_section] += 1
            current_entry = True
            continue

        if line.startswith("  "):
            if not current_entry:
                raise ReleasePolicyError(
                    "CHANGELOG.md Unreleased continuation has no preceding list "
                    f"entry on line {line_number}"
                )
            if stripped.startswith(("- ", "* ", "+ ")):
                raise ReleasePolicyError(
                    "CHANGELOG.md Unreleased nested list entries are unsupported "
                    f"on line {line_number}"
                )
            if _BREAKING_PREFIX in stripped:
                raise ReleasePolicyError(
                    "CHANGELOG.md Unreleased **Breaking:** must begin a Changed "
                    "list entry"
                )
            continue

        raise ReleasePolicyError(
            "CHANGELOG.md Unreleased release notes must be nonempty '- ' list "
            f"entries with two-space-indented continuations (line {line_number})"
        )

    empty_sections = [name for name in seen_sections if entry_counts[name] == 0]
    if empty_sections:
        raise ReleasePolicyError(
            "CHANGELOG.md Unreleased declared subsection(s) have no list entries: "
            + ", ".join(sorted(empty_sections))
        )

    populated = {name for name, count in entry_counts.items() if count > 0}
    if not populated:
        raise ReleasePolicyError(
            "CHANGELOG.md Unreleased section has no release notes"
        )

    if "Removed" in populated or has_breaking_entry:
        return "minor" if major == 0 else "major"
    if populated.intersection(("Added", "Changed", "Deprecated")):
        return "minor"
    return "patch"


def validate_requested_bump(
    changelog: str, current_version: str, requested_bump: str
) -> str:
    """Return the minimum bump, rejecting a requested bump below it."""
    if requested_bump not in _BUMP_RANK:
        raise ReleasePolicyError(f"unsupported bump kind {requested_bump!r}")
    required = minimum_bump(changelog, current_version)
    if _BUMP_RANK[requested_bump] < _BUMP_RANK[required]:
        raise ReleasePolicyError(
            f"requested {requested_bump} bump is below the minimum {required} bump "
            "required by CHANGELOG.md Unreleased notes"
        )
    return required


def main() -> int:
    """CLI entry point for release-policy checks and diagnostics."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--changelog", type=Path, required=True)
    parser.add_argument("--current-version", required=True)
    parser.add_argument("--requested-bump", choices=tuple(_BUMP_RANK))
    args = parser.parse_args()

    try:
        changelog = args.changelog.read_text(encoding="utf-8")
        if args.requested_bump is None:
            required = minimum_bump(changelog, args.current_version)
        else:
            required = validate_requested_bump(
                changelog, args.current_version, args.requested_bump
            )
    except (OSError, UnicodeError, ReleasePolicyError) as error:
        print(f"release-policy: error: {error}", file=sys.stderr)
        return 1

    print(f"minimum_bump={required}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
