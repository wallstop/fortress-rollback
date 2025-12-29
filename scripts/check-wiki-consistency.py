#!/usr/bin/env python3
"""
Cross-platform wiki consistency validation script.

Validates:
- Wiki-style links in _Sidebar.md point to existing pages
- All docs/ source files are mapped in WIKI_STRUCTURE
- All wiki pages have corresponding sidebar entries

Works on Windows, macOS, and Linux.

Usage:
    python scripts/check-wiki-consistency.py
    python scripts/check-wiki-consistency.py --verbose
"""

from __future__ import annotations

import ast
import io
import os
import re
import sys
from pathlib import Path
from typing import NamedTuple

# Ensure stdout uses UTF-8 encoding for Unicode symbols (✓, ✗, ⚠)
# This fixes UnicodeEncodeError on Windows with CP1252 default encoding
if sys.stdout.encoding != "utf-8":
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")


class ValidationResult(NamedTuple):
    """Result of validation."""

    errors: int
    warnings: int


# ANSI color codes (disabled on Windows without colorama or if not a TTY)
def supports_color() -> bool:
    """Check if the terminal supports color output."""
    if not hasattr(sys.stdout, "isatty") or not sys.stdout.isatty():
        return False
    if os.name == "nt":
        # Windows: check for newer Windows Terminal or enable VT processing
        try:
            import ctypes

            kernel32 = ctypes.windll.kernel32
            # Enable ANSI escape sequences on Windows 10+
            kernel32.SetConsoleMode(kernel32.GetStdHandle(-11), 7)
            return True
        except (AttributeError, OSError):
            return False
    return True


# Color output helpers
_USE_COLOR = supports_color()


def _color(text: str, code: str) -> str:
    """Apply ANSI color code to text if supported."""
    if _USE_COLOR:
        return f"\033[{code}m{text}\033[0m"
    return text


def red(text: str) -> str:
    """Red text for errors."""
    return _color(text, "31")


def yellow(text: str) -> str:
    """Yellow text for warnings."""
    return _color(text, "33")


def green(text: str) -> str:
    """Green text for success."""
    return _color(text, "32")


def bold(text: str) -> str:
    """Bold text."""
    return _color(text, "1")


def get_project_root() -> Path:
    """Get the project root directory."""
    script_dir = Path(__file__).parent.resolve()
    return script_dir.parent


def parse_wiki_structure_from_sync_script(sync_script_path: Path) -> dict[str, str]:
    """
    Parse WIKI_STRUCTURE dict from sync-wiki.py using AST.

    This safely extracts the dictionary without executing the script.
    """
    try:
        content = sync_script_path.read_text(encoding="utf-8")
        tree = ast.parse(content, filename=str(sync_script_path))

        for node in ast.walk(tree):
            if isinstance(node, ast.Assign):
                for target in node.targets:
                    if isinstance(target, ast.Name) and target.id == "WIKI_STRUCTURE":
                        if isinstance(node.value, ast.Dict):
                            result = {}
                            for key, value in zip(node.value.keys, node.value.values):
                                if isinstance(key, ast.Constant) and isinstance(
                                    value, ast.Constant
                                ):
                                    result[str(key.value)] = str(value.value)
                            return result
    except (OSError, SyntaxError) as e:
        print(red(f"ERROR: Could not parse {sync_script_path}: {e}"))

    return {}


def get_wiki_pages(wiki_dir: Path) -> set[str]:
    """Get all wiki page names (without .md extension)."""
    pages = set()
    if wiki_dir.exists():
        for md_file in wiki_dir.glob("*.md"):
            # Skip special files like _Sidebar.md
            if not md_file.name.startswith("_"):
                pages.add(md_file.stem)
    return pages


def get_docs_source_files(docs_dir: Path) -> set[str]:
    """
    Get all source markdown files in docs/ that should be mapped.

    Returns paths relative to docs/ directory.
    """
    skip_patterns = {
        "stylesheets",
        "includes",
        "abbreviations.md",
    }
    root_skip_files = {
        "README.md",  # docs/README.md is redundant with index.md
    }

    source_files = set()

    if not docs_dir.exists():
        return source_files

    for md_file in docs_dir.rglob("*.md"):
        rel_path = md_file.relative_to(docs_dir)

        # Skip files in excluded directories
        if any(part in skip_patterns for part in rel_path.parts):
            continue

        # Skip root-level files that should be skipped
        if len(rel_path.parts) == 1 and rel_path.name in root_skip_files:
            continue

        source_files.add(str(rel_path))

    return source_files


def parse_sidebar_wiki_links(sidebar_path: Path) -> list[tuple[str, str, int]]:
    """
    Parse wiki-style links from _Sidebar.md.

    Returns list of (page_name, display_text, line_number) tuples.
    Wiki links have format: [[PageName|Display Text]] or [[PageName]]
    """
    wiki_link_pattern = re.compile(r"\[\[([^\]|]+)(?:\|([^\]]+))?\]\]")

    links = []

    if not sidebar_path.exists():
        return links

    try:
        content = sidebar_path.read_text(encoding="utf-8")
        for line_num, line in enumerate(content.splitlines(), start=1):
            for match in wiki_link_pattern.finditer(line):
                page_name = match.group(1).strip()
                display_text = match.group(2).strip() if match.group(2) else page_name
                links.append((page_name, display_text, line_num))
    except (OSError, UnicodeDecodeError) as e:
        print(red(f"ERROR: Could not read {sidebar_path}: {e}"))

    return links


# Characters that cause URL encoding issues in GitHub Wiki display text.
# The '+' character is URL-decoded as a space, causing broken links.
# Example: [[Page|TLA+ Tools]] generates URL "/wiki/TLA--Tools" instead of "/wiki/Page"
#
# GitHub Wiki's wiki-link syntax [[PageName|Display Text]] has a quirk where
# certain characters in the display text can corrupt the generated URL.
# This is a known limitation of GitHub Wiki's markdown processor.
WIKI_LINK_PROBLEMATIC_CHARS = {
    "+": "plus sign (decoded as space, causing double-dashes in URL)",
    "%": "percent sign (interferes with URL encoding)",
    "#": "hash (interpreted as anchor)",
    "?": "question mark (interpreted as query string)",
    "&": "ampersand (interpreted as URL parameter separator)",
    "=": "equals sign (interpreted as URL parameter value separator)",
}


def validate_wiki_link_display_text(
    sidebar_path: Path, verbose: bool = False
) -> ValidationResult:
    """
    Validate that wiki-link display text doesn't contain problematic characters.

    GitHub Wiki's [[Page|Display]] syntax can break when the display text
    contains certain characters that interfere with URL generation.

    For example, [[TLAplus-Tooling|TLA+ Tooling]] generates a broken URL
    because '+' is decoded as a space, creating "/wiki/TLA--Tooling" instead
    of "/wiki/TLAplus-Tooling".
    """
    errors = 0
    warnings = 0

    links = parse_sidebar_wiki_links(sidebar_path)

    if verbose:
        print("\nChecking wiki-link display text for problematic characters...")

    for page_name, display_text, line_num in links:
        for char, reason in WIKI_LINK_PROBLEMATIC_CHARS.items():
            if char in display_text:
                errors += 1
                print(
                    red("ERROR:")
                    + f" _Sidebar.md:{line_num}: Wiki link [[{page_name}|{display_text}]] "
                    + f"contains '{char}' in display text ({reason}). "
                    + f"This will generate a broken URL."
                )
                break  # Only report first problematic character per link

    return ValidationResult(errors=errors, warnings=warnings)


def validate_sidebar_links(
    sidebar_path: Path, wiki_pages: set[str], verbose: bool = False
) -> ValidationResult:
    """
    Validate that all wiki-style links in _Sidebar.md point to existing pages.
    """
    errors = 0
    warnings = 0

    links = parse_sidebar_wiki_links(sidebar_path)

    if verbose:
        print(f"\nChecking {len(links)} wiki links in _Sidebar.md...")

    for page_name, display_text, line_num in links:
        if page_name not in wiki_pages:
            errors += 1
            print(
                red("ERROR:")
                + f" _Sidebar.md:{line_num}: Wiki link [[{page_name}]] "
                + f"points to non-existent page '{page_name}.md'"
            )
        elif verbose:
            print(f"  ✓ [[{page_name}|{display_text}]] -> {page_name}.md")

    return ValidationResult(errors=errors, warnings=warnings)


def validate_wiki_structure_completeness(
    wiki_structure: dict[str, str], docs_files: set[str], verbose: bool = False
) -> ValidationResult:
    """
    Validate that all docs/ source files have a mapping in WIKI_STRUCTURE.
    """
    errors = 0
    warnings = 0

    mapped_sources = set(wiki_structure.keys())
    unmapped_files = docs_files - mapped_sources

    if verbose:
        print(f"\nChecking WIKI_STRUCTURE completeness...")
        print(f"  Found {len(docs_files)} docs source files")
        print(f"  Found {len(mapped_sources)} mappings in WIKI_STRUCTURE")

    if unmapped_files:
        for unmapped in sorted(unmapped_files):
            warnings += 1
            print(
                yellow("WARNING:")
                + f" docs/{unmapped} has no mapping in WIKI_STRUCTURE "
                + "(scripts/sync-wiki.py)"
            )

    # Also check for stale mappings (mapped files that no longer exist)
    stale_mappings = mapped_sources - docs_files

    for stale in sorted(stale_mappings):
        warnings += 1
        print(
            yellow("WARNING:")
            + f" WIKI_STRUCTURE contains mapping for '{stale}' "
            + "which does not exist in docs/"
        )

    return ValidationResult(errors=errors, warnings=warnings)


def validate_sidebar_completeness(
    sidebar_path: Path,
    wiki_pages: set[str],
    wiki_structure: dict[str, str],
    verbose: bool = False,
) -> ValidationResult:
    """
    Validate that all wiki pages have a corresponding sidebar entry.
    """
    errors = 0
    warnings = 0

    sidebar_links = parse_sidebar_wiki_links(sidebar_path)
    linked_pages = {page_name for page_name, _, _ in sidebar_links}

    # Get all wiki pages that should be in sidebar
    # (pages generated from WIKI_STRUCTURE)
    expected_pages = set(wiki_structure.values())

    if verbose:
        print(f"\nChecking sidebar completeness...")
        print(f"  Found {len(wiki_pages)} wiki pages")
        print(f"  Found {len(linked_pages)} sidebar entries")

    # Find pages that exist but aren't in sidebar
    # Only warn about pages that are in WIKI_STRUCTURE (expected wiki pages)
    missing_from_sidebar = expected_pages - linked_pages

    for missing in sorted(missing_from_sidebar):
        # Verify the page actually exists
        if missing in wiki_pages:
            warnings += 1
            print(
                yellow("WARNING:")
                + f" Wiki page '{missing}.md' has no entry in _Sidebar.md"
            )

    return ValidationResult(errors=errors, warnings=warnings)


def validate_markdown_link_syntax(wiki_dir: Path, verbose: bool = False) -> ValidationResult:
    """
    Validate that markdown links in wiki pages have correct syntax.

    Checks for common malformed patterns:
    - Space after opening bracket: [ Text](url) should be [Text](url)
    - Space before closing bracket: [Text ](url) should be [Text](url)
    - Empty link text: [](url) is suspicious
    """
    errors = 0
    warnings = 0

    # Pattern for malformed links with space after opening bracket
    space_after_open = re.compile(r"\[\s+[^\]]+\]\([^)]+\)")
    # Pattern for malformed links with space before closing bracket
    space_before_close = re.compile(r"\[[^\]]+\s+\]\([^)]+\)")
    # Pattern for empty link text
    empty_text = re.compile(r"\[\s*\]\([^)]+\)")

    if verbose:
        print("\nChecking markdown link syntax in wiki pages...")

    for md_file in sorted(wiki_dir.glob("*.md")):
        try:
            content = md_file.read_text(encoding="utf-8")
            lines = content.splitlines()

            for line_num, line in enumerate(lines, start=1):
                # Check for space after opening bracket
                for match in space_after_open.finditer(line):
                    errors += 1
                    print(
                        red("ERROR:")
                        + f" {md_file.name}:{line_num}: Malformed link with space "
                        + f"after '[': {match.group()}"
                    )

                # Check for space before closing bracket
                for match in space_before_close.finditer(line):
                    warnings += 1
                    print(
                        yellow("WARNING:")
                        + f" {md_file.name}:{line_num}: Link has trailing space "
                        + f"before ']': {match.group()}"
                    )

                # Check for empty link text
                for match in empty_text.finditer(line):
                    warnings += 1
                    print(
                        yellow("WARNING:")
                        + f" {md_file.name}:{line_num}: Empty link text: "
                        + f"{match.group()}"
                    )

        except (OSError, UnicodeDecodeError) as e:
            print(red(f"ERROR: Could not read {md_file}: {e}"))
            errors += 1

    return ValidationResult(errors=errors, warnings=warnings)


def main() -> int:
    """Main entry point."""
    verbose = "--verbose" in sys.argv or "-v" in sys.argv

    project_root = get_project_root()
    os.chdir(project_root)

    wiki_dir = project_root / "wiki"
    docs_dir = project_root / "docs"
    sidebar_path = wiki_dir / "_Sidebar.md"
    sync_script_path = project_root / "scripts" / "sync-wiki.py"

    total_errors = 0
    total_warnings = 0

    print(bold("Wiki Consistency Check"))
    print("=" * 40)

    # Check required files/directories exist
    if not wiki_dir.exists():
        print(red("ERROR:") + f" Wiki directory not found: {wiki_dir}")
        return 1

    if not sidebar_path.exists():
        print(red("ERROR:") + f" Sidebar not found: {sidebar_path}")
        return 1

    if not sync_script_path.exists():
        print(red("ERROR:") + f" Sync script not found: {sync_script_path}")
        return 1

    # Get wiki pages and parse WIKI_STRUCTURE
    wiki_pages = get_wiki_pages(wiki_dir)
    wiki_structure = parse_wiki_structure_from_sync_script(sync_script_path)
    docs_files = get_docs_source_files(docs_dir)

    if not wiki_structure:
        print(red("ERROR:") + " Could not parse WIKI_STRUCTURE from sync-wiki.py")
        return 1

    if verbose:
        print(f"\nFound {len(wiki_pages)} wiki pages:")
        for page in sorted(wiki_pages):
            print(f"  - {page}.md")

    # 1. Validate sidebar links point to existing pages
    print(f"\n{bold('1. Validating _Sidebar.md wiki links...')}")
    result = validate_sidebar_links(sidebar_path, wiki_pages, verbose)
    total_errors += result.errors
    total_warnings += result.warnings
    if result.errors == 0:
        print(green("   ✓ All sidebar links are valid"))

    # 2. Validate wiki-link display text for problematic characters
    print(f"\n{bold('2. Validating wiki-link display text...')}")
    result = validate_wiki_link_display_text(sidebar_path, verbose)
    total_errors += result.errors
    total_warnings += result.warnings
    if result.errors == 0:
        print(green("   ✓ All display text is safe for URL generation"))

    # 3. Validate WIKI_STRUCTURE completeness
    print(f"\n{bold('3. Validating WIKI_STRUCTURE completeness...')}")
    result = validate_wiki_structure_completeness(wiki_structure, docs_files, verbose)
    total_errors += result.errors
    total_warnings += result.warnings
    if result.errors == 0 and result.warnings == 0:
        print(green("   ✓ All docs files are mapped"))

    # 4. Validate sidebar completeness
    print(f"\n{bold('4. Validating sidebar completeness...')}")
    result = validate_sidebar_completeness(
        sidebar_path, wiki_pages, wiki_structure, verbose
    )
    total_errors += result.errors
    total_warnings += result.warnings
    if result.errors == 0 and result.warnings == 0:
        print(green("   ✓ All wiki pages have sidebar entries"))

    # 5. Validate markdown link syntax
    print(f"\n{bold('5. Validating markdown link syntax...')}")
    result = validate_markdown_link_syntax(wiki_dir, verbose)
    total_errors += result.errors
    total_warnings += result.warnings
    if result.errors == 0 and result.warnings == 0:
        print(green("   ✓ All markdown links have correct syntax"))

    # Print summary
    print("\n" + "=" * 40)
    print(bold("Summary:"))
    print(f"  Errors:   {total_errors}")
    print(f"  Warnings: {total_warnings}")

    if total_errors > 0:
        print(red("\n✗ Wiki consistency check FAILED"))
        return 1

    if total_warnings > 0:
        print(yellow("\n⚠ Wiki consistency check passed with warnings"))
        return 0

    print(green("\n✓ Wiki consistency check PASSED"))
    return 0


if __name__ == "__main__":
    sys.exit(main())
