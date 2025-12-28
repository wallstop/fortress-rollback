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
            pass
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
            pass

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

    # Find markdown links: [text](url) and [text][ref]
    # Standard links
    link_pattern = re.compile(r"\[([^\]]*)\]\(([^)]+)\)")

    for match in link_pattern.finditer(content):
        link_text = match.group(1)
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
