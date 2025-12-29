#!/usr/bin/env python3
"""
Validate generated wiki content for common formatting issues.

This script checks that the generated wiki files don't have issues that would
cause GitHub Wiki to render them incorrectly, such as:

1. Indented code fences (4+ spaces before ```) which become preformatted blocks
2. Orphaned admonition content (lines that should be blockquotes but aren't)
3. Raw markdown syntax that wasn't converted properly
4. Missing or broken wiki links

Usage:
    python scripts/validate-wiki-output.py
    python scripts/validate-wiki-output.py --wiki-dir wiki
    python scripts/validate-wiki-output.py --strict  # Fail on warnings too
"""

from __future__ import annotations

import argparse
import io
import os
import re
import sys
from pathlib import Path
from typing import NamedTuple

# Ensure stdout uses UTF-8 encoding for Unicode symbols
if sys.stdout.encoding != "utf-8":
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")


class Issue(NamedTuple):
    """Represents a validation issue."""

    file: str
    line: int
    severity: str  # "error" or "warning"
    message: str


def supports_color() -> bool:
    """Check if the terminal supports color output."""
    if not hasattr(sys.stdout, "isatty") or not sys.stdout.isatty():
        return False
    if os.name == "nt":
        try:
            import ctypes

            kernel32 = ctypes.windll.kernel32
            kernel32.SetConsoleMode(kernel32.GetStdHandle(-11), 7)
            return True
        except (AttributeError, OSError):
            return False
    return True


_USE_COLOR = supports_color()


def _color(text: str, code: str) -> str:
    if _USE_COLOR:
        return f"\033[{code}m{text}\033[0m"
    return text


def red(text: str) -> str:
    return _color(text, "31")


def yellow(text: str) -> str:
    return _color(text, "33")


def green(text: str) -> str:
    return _color(text, "32")


def bold(text: str) -> str:
    return _color(text, "1")


def check_indented_code_fences(content: str, filename: str) -> list[Issue]:
    """Check for code fences preceded by 4+ spaces (won't render as code blocks)."""
    issues = []
    lines = content.split("\n")

    for i, line in enumerate(lines, 1):
        # Check if line starts with 4+ spaces followed by ```
        if re.match(r"^    +```", line):
            issues.append(
                Issue(
                    file=filename,
                    line=i,
                    severity="error",
                    message=f"Indented code fence (4+ spaces) won't render as code block: {line[:50]}...",
                )
            )

    return issues


def check_unconverted_mkdocs_syntax(content: str, filename: str) -> list[Issue]:
    """Check for MkDocs-specific syntax that wasn't converted."""
    issues = []
    lines = content.split("\n")

    for i, line in enumerate(lines, 1):
        # Check for unconverted tab markers
        if re.match(r'^=== "[^"]+"', line):
            issues.append(
                Issue(
                    file=filename,
                    line=i,
                    severity="error",
                    message=f"Unconverted MkDocs tab marker: {line}",
                )
            )

        # Check for unconverted admonitions
        if re.match(r"^!!! \w+", line):
            issues.append(
                Issue(
                    file=filename,
                    line=i,
                    severity="error",
                    message=f"Unconverted MkDocs admonition: {line}",
                )
            )

        # Check for Material icons (only outside code blocks)
        # Simple heuristic: line doesn't start with > or `
        if not line.strip().startswith((">", "`", "#")):
            if re.search(r":(material|octicons|fontawesome)-[a-z0-9-]+:", line):
                # Check if it's inside inline code
                if "`" not in line or not re.search(r"`[^`]*:(material|octicons|fontawesome)-", line):
                    issues.append(
                        Issue(
                            file=filename,
                            line=i,
                            severity="warning",
                            message=f"Possible unconverted Material icon: {line[:60]}...",
                        )
                    )

    return issues


def check_orphaned_indented_content(content: str, filename: str) -> list[Issue]:
    """Check for 4-space indented content that looks like orphaned tab/admonition content."""
    issues = []
    lines = content.split("\n")
    in_code_block = False

    for i, line in enumerate(lines, 1):
        # Track code blocks
        if line.strip().startswith("```"):
            in_code_block = not in_code_block
            continue

        if in_code_block:
            continue

        # Check for 4-space indented lines that aren't in code blocks
        # and don't look like deliberate indentation
        if re.match(r"^    [^\s]", line):
            # Skip if it's part of a list or looks intentional
            prev_line = lines[i - 2] if i > 1 else ""
            if prev_line.strip().startswith(("-", "*", "1.", ">")):
                continue

            # Check if previous line is a header (which would indicate orphaned tab content)
            if prev_line.strip().startswith("###"):
                issues.append(
                    Issue(
                        file=filename,
                        line=i,
                        severity="warning",
                        message=f"Possibly orphaned indented content after header: {line[:50]}...",
                    )
                )

    return issues


def check_empty_sections(content: str, filename: str) -> list[Issue]:
    """Check for empty content sections (headers followed by only whitespace/comments).

    This catches issues like grid cards content being removed instead of converted,
    leaving empty sections.
    """
    issues = []
    lines = content.split("\n")

    for i, line in enumerate(lines, 1):
        # Check for section headers (##, ###, etc.)
        header_match = re.match(r"^(#{2,6})\s+(.+)$", line)
        if header_match:
            header_level = header_match.group(1)
            header_text = header_match.group(2).strip()

            # Look ahead to see if section has content
            section_has_content = False
            # i is 1-indexed (line number), but lines[] is 0-indexed.
            # Conveniently, the 1-indexed line number of current line equals
            # the 0-indexed position of the NEXT line (i.e., lines[i] is next line).
            j = i  # 0-indexed position of next line after header

            while j < len(lines):
                next_line = lines[j]

                # Check if we've hit another header of same or higher level
                next_header_match = re.match(r"^(#{2,6})\s+", next_line)
                if next_header_match:
                    next_level = next_header_match.group(1)
                    # If same or higher level, section ends
                    if len(next_level) <= len(header_level):
                        break

                # Skip empty lines, horizontal rules, and HTML comments
                stripped = next_line.strip()
                if stripped and stripped != "---" and not stripped.startswith("<!--"):
                    section_has_content = True
                    break

                j += 1

            # If section has no content, report it
            if not section_has_content:
                issues.append(
                    Issue(
                        file=filename,
                        line=i,
                        severity="error",
                        message=f"Empty section: '{header_text}' has no content (possible conversion issue)",
                    )
                )

    return issues


def check_broken_wiki_links(content: str, filename: str, wiki_pages: set[str]) -> list[Issue]:
    """Check for wiki links that point to non-existent pages."""
    issues = []

    # Check [[Page]] and [[Page|Text]] style links
    for match in re.finditer(r"\[\[([^\]|]+)(?:\|[^\]]+)?\]\]", content):
        page_name = match.group(1)
        if page_name not in wiki_pages and page_name != "Home":
            # Find line number
            line_num = content[: match.start()].count("\n") + 1
            issues.append(
                Issue(
                    file=filename,
                    line=line_num,
                    severity="error",
                    message=f"Broken wiki link to non-existent page: [[{page_name}]]",
                )
            )

    # Check standard markdown links to wiki pages [text](Page-Name)
    for match in re.finditer(r"\[([^\]]+)\]\(([^)]+)\)", content):
        link_target = match.group(2)
        # Skip external links and anchors
        if link_target.startswith(("http://", "https://", "#", "mailto:")):
            continue
        # Skip asset links
        if link_target.startswith("assets/"):
            continue
        # Extract page name (remove anchor)
        page_name = link_target.split("#")[0]
        if page_name and page_name not in wiki_pages:
            line_num = content[: match.start()].count("\n") + 1
            issues.append(
                Issue(
                    file=filename,
                    line=line_num,
                    severity="warning",
                    message=f"Link may be broken - page not found: {page_name}",
                )
            )

    return issues


def validate_wiki(wiki_dir: Path, strict: bool = False) -> int:
    """Validate all wiki files and return exit code."""
    if not wiki_dir.exists():
        print(red(f"ERROR: Wiki directory not found: {wiki_dir}"))
        return 1

    # Get all wiki pages
    wiki_pages = {f.stem for f in wiki_dir.glob("*.md") if not f.name.startswith("_")}

    all_issues: list[Issue] = []

    # Validate each wiki file
    for wiki_file in sorted(wiki_dir.glob("*.md")):
        if wiki_file.name.startswith("_"):
            continue  # Skip _Sidebar.md etc.

        content = wiki_file.read_text(encoding="utf-8")
        filename = wiki_file.name

        # Run all checks
        all_issues.extend(check_indented_code_fences(content, filename))
        all_issues.extend(check_unconverted_mkdocs_syntax(content, filename))
        all_issues.extend(check_orphaned_indented_content(content, filename))
        all_issues.extend(check_empty_sections(content, filename))
        all_issues.extend(check_broken_wiki_links(content, filename, wiki_pages))

    # Also validate sidebar
    sidebar_file = wiki_dir / "_Sidebar.md"
    if sidebar_file.exists():
        content = sidebar_file.read_text(encoding="utf-8")
        all_issues.extend(check_broken_wiki_links(content, "_Sidebar.md", wiki_pages))

    # Report results
    errors = [i for i in all_issues if i.severity == "error"]
    warnings = [i for i in all_issues if i.severity == "warning"]

    if all_issues:
        print(bold("\nWiki Validation Issues:\n"))

        for issue in sorted(all_issues, key=lambda x: (x.file, x.line)):
            if issue.severity == "error":
                prefix = red("ERROR")
            else:
                prefix = yellow("WARNING")

            print(f"  {prefix} {issue.file}:{issue.line}: {issue.message}")

        print()

    # Summary
    if errors:
        print(red(f"✗ Found {len(errors)} error(s) and {len(warnings)} warning(s)"))
    elif warnings:
        print(yellow(f"⚠ Found {len(warnings)} warning(s)"))
    else:
        print(green("✓ Wiki validation passed"))

    # Determine exit code
    if errors:
        return 1
    if strict and warnings:
        return 1
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Validate generated wiki content for formatting issues."
    )
    parser.add_argument(
        "--wiki-dir",
        type=Path,
        default=Path("wiki"),
        help="Path to wiki directory (default: wiki)",
    )
    parser.add_argument(
        "--strict",
        action="store_true",
        help="Treat warnings as errors",
    )

    args = parser.parse_args()

    # Resolve paths relative to project root
    script_dir = Path(__file__).parent.resolve()
    project_root = script_dir.parent

    wiki_dir = args.wiki_dir
    if not wiki_dir.is_absolute():
        wiki_dir = project_root / wiki_dir

    print(bold(f"Validating wiki at: {wiki_dir}\n"))

    return validate_wiki(wiki_dir, strict=args.strict)


if __name__ == "__main__":
    sys.exit(main())
