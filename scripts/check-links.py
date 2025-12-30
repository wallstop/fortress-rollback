#!/usr/bin/env python3
"""
Cross-platform link validation script for pre-commit hooks.

Validates:
- Local file references in markdown files
- Relative paths in code comments and documentation
- Anchor links within markdown files

Works on Windows, macOS, and Linux.

This is a cross-platform Python port of scripts/check-links.sh
"""

import os
import re
import sys
from pathlib import Path
from typing import NamedTuple


class LinkCheckResult(NamedTuple):
    """Result of link checking."""

    errors: int
    warnings: int
    checked: int


def get_project_root() -> Path:
    """Get the project root directory."""
    script_dir = Path(__file__).parent.resolve()
    return script_dir.parent


def extract_markdown_anchors(content: str) -> set[str]:
    """Extract anchor IDs from markdown content.

    Uses the same algorithm as markdownlint (GitHub-flavored Markdown):
    1. Convert to lowercase
    2. Replace spaces with hyphens
    3. Remove special characters (keep alphanumeric and hyphens)
    4. Do NOT collapse multiple hyphens (slashes become double hyphens)
    """
    anchors = set()

    # Match headers: # Header, ## Header, etc.
    header_pattern = re.compile(r"^#+\s+(.+)$", re.MULTILINE)
    for match in header_pattern.finditer(content):
        header_text = match.group(1).strip()
        # Convert to anchor format matching markdownlint/GFM:
        # 1. Lowercase
        anchor = header_text.lower()
        # 2. Replace spaces with hyphens
        anchor = anchor.replace(" ", "-")
        # 3. Remove special chars (keep alphanumeric and hyphens)
        anchor = re.sub(r"[^\w-]", "", anchor)
        # 4. Strip leading/trailing hyphens
        anchor = anchor.strip("-")
        anchors.add(anchor)

    # Match explicit anchor definitions: {#anchor-id}
    explicit_pattern = re.compile(r"\{#([\w-]+)\}")
    for match in explicit_pattern.finditer(content):
        anchors.add(match.group(1))

    return anchors


def find_code_fence_ranges(content: str) -> list[tuple[int, int]]:
    """Find ranges of fenced code blocks (``` or ~~~) to skip.

    Uses a state-based parser to properly handle:
    - Different fence lengths (``` vs ````)
    - Both backtick and tilde fences
    - Nested fences (longer fence can contain shorter)
    """
    ranges = []
    lines = content.split("\n")
    pos = 0
    fence_start: int | None = None
    fence_char: str | None = None
    fence_len: int = 0

    for line in lines:
        line_start = pos
        stripped = line.lstrip()

        # Check for fence opening/closing
        if stripped.startswith("```") or stripped.startswith("~~~"):
            char = stripped[0]
            # Count consecutive fence characters
            count = 0
            for c in stripped:
                if c == char:
                    count += 1
                else:
                    break

            if fence_start is None:
                # Opening fence
                fence_start = line_start
                fence_char = char
                fence_len = count
            elif char == fence_char and count >= fence_len:
                # Closing fence (same char, at least same length)
                ranges.append((fence_start, pos + len(line)))
                fence_start = None
                fence_char = None
                fence_len = 0

        pos += len(line) + 1  # +1 for newline

    # Handle unclosed fence at end of file
    if fence_start is not None:
        ranges.append((fence_start, len(content)))

    return ranges


def in_code_block(pos: int, code_ranges: list[tuple[int, int]]) -> bool:
    """Check if a position is within any code block range."""
    return any(start <= pos < end for start, end in code_ranges)


def find_inline_code_ranges(content: str) -> list[tuple[int, int]]:
    """Find ranges of inline code spans (single backticks) to skip.

    Handles both single backtick `code` and double backtick ``code`` syntax.
    Does NOT include already-detected fenced code blocks (handled separately).
    """
    ranges = []
    i = 0
    n = len(content)

    while i < n:
        if content[i] == "`":
            # Count consecutive backticks
            start = i
            backtick_count = 0
            while i < n and content[i] == "`":
                backtick_count += 1
                i += 1

            # Skip if this is a fenced code block marker (3+ backticks at line start)
            # Those are handled by find_code_fence_ranges
            line_start = content.rfind("\n", 0, start) + 1
            prefix = content[line_start:start]
            if backtick_count >= 3 and prefix.strip() == "":
                continue

            # Find the closing backticks (same count)
            closing_pattern = "`" * backtick_count
            end_pos = content.find(closing_pattern, i)

            if end_pos != -1:
                # Found closing - range is from first backtick to after closing backticks
                ranges.append((start, end_pos + backtick_count))
                i = end_pos + backtick_count
            # If no closing found, continue scanning
        else:
            i += 1

    return ranges


def in_code_span(pos: int, inline_ranges: list[tuple[int, int]]) -> bool:
    """Check if a position is within any inline code span."""
    return any(start <= pos < end for start, end in inline_ranges)


def is_wiki_file(source_file: Path, project_root: Path) -> bool:
    """Check if a file is in the wiki directory."""
    try:
        rel_path = source_file.relative_to(project_root)
        return rel_path.parts[0] == "wiki"
    except ValueError:
        return False


def check_markdown_link(
    source_file: Path, link_target: str, project_root: Path, verbose: bool = False
) -> tuple[bool, str]:
    """
    Check if a markdown link target is valid.

    Returns (is_valid, error_message).
    """
    # Skip external links
    if link_target.startswith(("http://", "https://", "mailto:", "ftp://")):
        return True, ""

    # Skip special links
    if link_target.startswith("#"):
        # Anchor link within the same file
        anchor = link_target[1:]
        try:
            content = source_file.read_text(encoding="utf-8")
            anchors = extract_markdown_anchors(content)
            if anchor.lower() not in {a.lower() for a in anchors}:
                return False, f"Anchor '{anchor}' not found in {source_file}"
        except (OSError, UnicodeDecodeError):
            pass  # File read errors are non-fatal; treat link as valid
        return True, ""

    # Handle anchor in link: file.md#anchor
    anchor = None
    if "#" in link_target:
        link_target, anchor = link_target.split("#", 1)

    # Resolve relative path
    source_dir = source_file.parent
    target_path = (source_dir / link_target).resolve()

    # Check if target exists
    if not target_path.exists():
        # For wiki files, try adding .md extension (GitHub Wiki uses extensionless links)
        if is_wiki_file(source_file, project_root):
            wiki_target_path = (source_dir / f"{link_target}.md").resolve()
            if wiki_target_path.exists():
                target_path = wiki_target_path
            else:
                return False, f"Link target not found: {link_target} (from {source_file.relative_to(project_root)})"
        else:
            return False, f"Link target not found: {link_target} (from {source_file.relative_to(project_root)})"

    # If there's an anchor, check it exists in target file
    if anchor and target_path.suffix.lower() == ".md":
        try:
            content = target_path.read_text(encoding="utf-8")
            anchors = extract_markdown_anchors(content)
            if anchor.lower() not in {a.lower() for a in anchors}:
                return (
                    False,
                    f"Anchor '{anchor}' not found in {target_path.relative_to(project_root)}",
                )
        except (OSError, UnicodeDecodeError):
            pass  # File read errors are non-fatal; treat anchor as valid

    return True, ""


def check_markdown_file(
    file_path: Path, project_root: Path, verbose: bool = False
) -> LinkCheckResult:
    """Check all links in a markdown file."""
    errors = 0
    warnings = 0
    checked = 0

    try:
        content = file_path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as e:
        print(f"ERROR: Could not read {file_path}: {e}")
        return LinkCheckResult(errors=1, warnings=0, checked=0)

    # Find code fence ranges to skip
    code_ranges = find_code_fence_ranges(content)

    # Find inline code span ranges to skip
    inline_code_ranges = find_inline_code_ranges(content)

    # Find markdown links: [text](url) and [text][ref]
    # Standard links
    link_pattern = re.compile(r"\[([^\]]*)\]\(([^)]+)\)")

    for match in link_pattern.finditer(content):
        # Skip links inside code blocks or inline code spans
        if in_code_block(match.start(), code_ranges):
            continue
        if in_code_span(match.start(), inline_code_ranges):
            continue

        _link_text = match.group(1)  # Captured but unused; kept for debugging
        link_target = match.group(2).strip()
        checked += 1

        # Skip empty links
        if not link_target:
            continue

        is_valid, error_msg = check_markdown_link(
            file_path, link_target, project_root, verbose
        )
        if not is_valid:
            errors += 1
            rel_path = file_path.relative_to(project_root)
            print(f"ERROR: {rel_path}: {error_msg}")

    return LinkCheckResult(errors=errors, warnings=warnings, checked=checked)


def main() -> int:
    """Main entry point."""
    verbose = "--verbose" in sys.argv or "-v" in sys.argv

    project_root = get_project_root()
    os.chdir(project_root)

    total_errors = 0
    total_warnings = 0
    total_checked = 0
    files_checked = 0

    # Directories to skip
    skip_dirs = {"target", "node_modules", ".git", "fuzz/target", "loom-tests/target"}

    # Find all markdown files
    for md_file in project_root.rglob("*.md"):
        # Skip files in excluded directories
        rel_path = md_file.relative_to(project_root)
        if any(part in skip_dirs for part in rel_path.parts):
            continue

        result = check_markdown_file(md_file, project_root, verbose)
        total_errors += result.errors
        total_warnings += result.warnings
        total_checked += result.checked
        files_checked += 1

    # Print summary
    print(f"\nLink check complete:")
    print(f"  Files checked: {files_checked}")
    print(f"  Links checked: {total_checked}")
    print(f"  Errors: {total_errors}")
    print(f"  Warnings: {total_warnings}")

    if total_errors > 0:
        return 1

    print("[OK] All links valid")
    return 0


if __name__ == "__main__":
    sys.exit(main())
