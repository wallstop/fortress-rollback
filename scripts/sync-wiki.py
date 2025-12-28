#!/usr/bin/env python3
"""
Sync documentation from docs/ to GitHub Wiki format.

This script:
1. Copies markdown files from docs/ to wiki/
2. Converts internal links to wiki format (removes .md, transforms names)
3. Generates a _Sidebar.md for navigation
4. Creates a Home.md landing page from docs/index.md
5. Handles assets/images

Usage:
    python scripts/sync-wiki.py
    python scripts/sync-wiki.py --source docs --dest wiki
    python scripts/sync-wiki.py --dry-run
"""

from __future__ import annotations

import argparse
import logging
import re
import shutil
from pathlib import Path, PurePosixPath
from typing import NamedTuple

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format="%(levelname)s: %(message)s",
)
logger = logging.getLogger(__name__)

# Default directories (relative to script location's parent)
DEFAULT_DOCS_DIR = Path("docs")
DEFAULT_WIKI_DIR = Path("wiki")
DEFAULT_ASSETS_DIR = Path("assets")

# Files/directories to skip during sync
SKIP_PATTERNS = {
    "stylesheets",
    "includes",
    "abbreviations.md",
}

# Files to skip only at docs root level
ROOT_SKIP_FILES = {
    "README.md",  # docs/README.md is redundant with index.md
}

# Mapping of source doc paths to wiki page names
# Keys are relative to docs/, values are wiki page names (without .md)
WIKI_STRUCTURE = {
    # Main pages
    "index.md": "Home",
    "user-guide.md": "User-Guide",
    "architecture.md": "Architecture",
    "migration.md": "Migration",
    "changelog.md": "Changelog",
    "contributing.md": "Contributing",
    "code-of-conduct.md": "Code-of-Conduct",
    "fortress-vs-ggrs.md": "Fortress-vs-GGRS",
    "ggrs-changelog-archive.md": "GGRS-Changelog-Archive",
    "tlaplus-tooling-research.md": "TLAplus-Tooling-Research",
    # Specs directory
    "specs/formal-spec.md": "Formal-Spec",
    "specs/determinism-model.md": "Determinism-Model",
    "specs/api-contracts.md": "API-Contracts",
    "specs/spec-divergences.md": "Spec-Divergences",
    "specs/README.md": "Specs",
}

# Root-level files that may be linked from docs
ROOT_WIKI_NAMES = {
    "README": "Home",
    "CHANGELOG": "Changelog",
    "CONTRIBUTING": "Contributing",
    "LICENSE": "License",
}


class LinkMatch(NamedTuple):
    """Represents a matched markdown link."""

    start: int
    end: int
    text: str
    href: str
    full_match: str


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


def find_inline_code_ranges(content: str) -> list[tuple[int, int]]:
    """Find ranges of inline code (`...`) to skip.

    Handles:
    - Standard inline code: `code`
    - Empty inline code: ``
    - Multi-backtick delimiters: ``code with ` inside``

    Limitation:
    - Does not match inline code containing newlines (rare in practice)
    - Escaped backticks within code are not specially handled,
      but this is uncommon in documentation
    """
    ranges = []
    # Match backtick-delimited code spans
    # Handles: `code`, ``, ``code with ` inside``
    # The pattern matches N backticks, then content (no newlines), then N backticks
    # We handle single and double backtick cases explicitly
    patterns = [
        re.compile(r"``[^`\n]*``"),  # Double backtick (can contain single `)
        re.compile(r"`[^`\n]*`"),    # Single backtick (standard)
    ]
    for pattern in patterns:
        for match in pattern.finditer(content):
            # Avoid overlapping matches
            if not any(start <= match.start() < end for start, end in ranges):
                ranges.append((match.start(), match.end()))
    return ranges


def in_ranges(pos: int, ranges: list[tuple[int, int]]) -> bool:
    """Check if a position is within any of the given ranges."""
    return any(start <= pos < end for start, end in ranges)


def extract_links(content: str) -> list[LinkMatch]:
    """Extract all markdown links from content.

    Known limitations:
    - Nested brackets in link text: [text [nested]](url) won't match correctly
    - Parentheses in URLs: [text](url(with)parens) won't match correctly
    - These edge cases are rare in practice; for full CommonMark compliance,
      a proper markdown parser (like mistune or markdown-it-py) would be needed.
    """
    links = []
    # Match [text](url) pattern
    # Note: This pattern has known limitations with nested brackets/parens
    # (see docstring). A proper parser would use balanced matching.
    pattern = re.compile(r"\[([^\]]*)\]\(([^)]+)\)")
    for match in pattern.finditer(content):
        links.append(
            LinkMatch(
                start=match.start(),
                end=match.end(),
                text=match.group(1),
                href=match.group(2),
                full_match=match.group(0),
            )
        )
    return links


def split_anchor(href: str) -> tuple[str, str]:
    """Split a href into path and anchor parts."""
    if "#" in href:
        path, anchor = href.split("#", 1)
        return path, anchor
    return href, ""


def normalize_path(path: str) -> str:
    """Normalize a path to use forward slashes (POSIX style)."""
    return str(PurePosixPath(Path(path)))


def remove_md_suffix(path: str) -> str:
    """Remove .md extension from path if present."""
    return path.removesuffix(".md")


def resolve_relative_path(source_file: str, link: str) -> str | None:
    """
    Resolve a relative link path from a source file location.

    Returns None if the path escapes the documentation root.
    """
    source_dir = PurePosixPath(source_file).parent
    if str(source_dir) == ".":
        resolved = PurePosixPath(link)
    else:
        resolved = source_dir / link

    # Normalize to handle .. and .
    parts: list[str] = []
    escape_count = 0

    for part in resolved.parts:
        if part == "..":
            if parts:
                parts.pop()
            else:
                escape_count += 1
        elif part != ".":
            parts.append(part)

    if escape_count > 0:
        return None

    return str(PurePosixPath(*parts)) if parts else "."


def path_to_wiki_name(path: str) -> str:
    """
    Convert a path to a wiki page name.

    Examples:
        user-guide.md -> User-Guide
        specs/formal-spec.md -> Formal-Spec
    """
    # Remove .md extension
    name = remove_md_suffix(path)
    # Get just the filename
    name = PurePosixPath(name).name
    # Title-case each segment
    parts = name.split("-")
    parts = [p.capitalize() for p in parts]
    return "-".join(parts)


def convert_links(content: str, source_file: str, wiki_structure: dict[str, str]) -> str:
    """
    Convert relative markdown links to wiki links.

    Transforms:
        [Guide](user-guide.md) -> [Guide](User-Guide)
        [API](architecture.md#section) -> [API](Architecture#section)
        [Code](../src/lib.rs) -> keep as-is (external link)
    """
    # Get code ranges to skip
    code_ranges = find_code_fence_ranges(content)
    inline_code_ranges = find_inline_code_ranges(content)
    skip_ranges = code_ranges + inline_code_ranges

    # Extract links
    links = extract_links(content)

    # Process links in reverse order to maintain correct positions
    result = content

    for link_match in reversed(links):
        # Skip links inside code blocks
        if in_ranges(link_match.start, skip_ranges):
            continue

        href = link_match.href
        link_text = link_match.text

        # Skip external links, anchors, and special protocols
        if href.startswith(("http://", "https://", "#", "mailto:", "tel:")):
            continue

        # Handle anchors in links
        link_path, anchor_text = split_anchor(href)
        anchor = f"#{anchor_text}" if anchor_text else ""

        # Skip if it's just an anchor
        if not link_path:
            continue

        # Skip non-markdown files (images, source code, etc.)
        if not link_path.endswith(".md"):
            # Keep asset links but update paths if needed
            continue

        # Resolve relative path
        resolved = resolve_relative_path(normalize_path(source_file), link_path)

        # Handle invalid paths (escaping root)
        if resolved is None:
            logger.debug(f"  {source_file}: [{href}] - path escapes documentation root")
            continue

        # Remove .md extension for matching
        resolved_without_ext = remove_md_suffix(resolved)

        # Strip docs/ prefix if present
        if resolved_without_ext.startswith("docs/"):
            resolved_without_ext = resolved_without_ext[5:]

        # Look up wiki page name
        wiki_name = None
        resolved_with_ext = resolved_without_ext + ".md"

        # Check direct mapping
        if resolved_with_ext in wiki_structure:
            wiki_name = wiki_structure[resolved_with_ext]

        # Check root files
        if wiki_name is None and resolved_without_ext in ROOT_WIKI_NAMES:
            wiki_name = ROOT_WIKI_NAMES[resolved_without_ext]

        # Auto-generate wiki name if not in structure
        if wiki_name is None:
            wiki_name = path_to_wiki_name(link_path)
            logger.debug(
                f"  Auto-generated wiki name for {link_path}: {wiki_name}"
            )

        # Replace with wiki link format (standard markdown, no .md extension)
        new_link = f"[{link_text}]({wiki_name}{anchor})"
        result = result[: link_match.start] + new_link + result[link_match.end :]
        logger.debug(f"  Converted: {link_match.full_match} -> {new_link}")

    return result


def strip_mkdocs_frontmatter(content: str) -> str:
    """Remove MkDocs-specific frontmatter and admonitions."""
    # Remove YAML frontmatter
    if content.startswith("---"):
        end = content.find("---", 3)
        if end != -1:
            content = content[end + 3 :].lstrip()

    return content


def remove_grid_cards_divs(content: str) -> str:
    """Remove Material grid cards divs with proper nesting support.

    This function properly handles nested divs by tracking div depth,
    rather than using a simple non-greedy regex that would fail on:
        <div class="grid cards" markdown>
            <div>nested content</div>  <!-- regex would stop here -->
            more content
        </div>
    """
    result = []
    i = 0
    n = len(content)

    while i < n:
        # Look for grid cards div opening
        grid_match = re.match(
            r'<div\s+class="grid cards"[^>]*markdown>',
            content[i:],
            re.IGNORECASE,
        )
        if grid_match:
            # Found a grid cards div, now find the matching </div>
            div_depth = 1
            j = i + grid_match.end()

            while j < n and div_depth > 0:
                # Check for opening div
                open_match = re.match(r"<div[^>]*>", content[j:], re.IGNORECASE)
                if open_match:
                    div_depth += 1
                    j += open_match.end()
                    continue

                # Check for closing div
                close_match = re.match(r"</div\s*>", content[j:], re.IGNORECASE)
                if close_match:
                    div_depth -= 1
                    j += close_match.end()
                    continue

                j += 1

            # Skip the entire grid cards div (don't add to result)
            i = j
        else:
            result.append(content[i])
            i += 1

    return "".join(result)


def strip_mkdocs_features(content: str) -> str:
    """Remove MkDocs Material-specific features that don't render in GitHub Wiki."""
    # Remove Material icons like :material-star-four-points:
    content = re.sub(r":material-[a-z-]+:", "", content)
    content = re.sub(r":octicons-[a-z-]+:", "", content)
    content = re.sub(r":fontawesome-[a-z-]+:", "", content)

    # Remove { .lg .middle } and similar Markdown attribute annotations
    # Only matches braces containing class (.) or id (#) selectors to avoid
    # removing legitimate content like {variable} placeholders
    content = re.sub(r"\{\s*[.#][^}]*\}", "", content)

    # Remove Material grid cards divs (with proper nested div handling)
    content = remove_grid_cards_divs(content)

    return content


def read_file_safe(path: Path) -> str | None:
    """Safely read a file, returning None on error."""
    try:
        return path.read_text(encoding="utf-8")
    except OSError as e:
        logger.error(f"Error reading {path}: {e}")
        return None
    except UnicodeDecodeError as e:
        logger.error(f"Error decoding {path}: {e}")
        return None


def write_file_safe(path: Path, content: str, dry_run: bool = False) -> bool:
    """Safely write a file, returning False on error."""
    if dry_run:
        logger.info(f"  [DRY RUN] Would write: {path}")
        return True
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")
        return True
    except OSError as e:
        logger.error(f"Error writing {path}: {e}")
        return False


def process_file(
    src_path: Path,
    wiki_name: str,
    docs_dir: Path,
    wiki_dir: Path,
    wiki_structure: dict[str, str],
    dry_run: bool = False,
) -> bool:
    """Process a markdown file and copy to wiki directory."""
    content = read_file_safe(src_path)
    if content is None:
        return False

    # Get relative path for link resolution
    try:
        relative_path = str(src_path.relative_to(docs_dir))
    except ValueError:
        relative_path = str(src_path)

    relative_path = normalize_path(relative_path)

    # Transform content
    # Note: Order matters - convert links first, then strip MkDocs features
    # to avoid stripping curly braces or divs inside URLs/link text
    content = strip_mkdocs_frontmatter(content)
    content = convert_links(content, relative_path, wiki_structure)
    content = strip_mkdocs_features(content)

    # Write to wiki
    dest_path = wiki_dir / f"{wiki_name}.md"
    if write_file_safe(dest_path, content, dry_run):
        logger.info(f"  {src_path} -> {dest_path}")
        return True
    return False


def should_skip(path: Path, docs_dir: Path) -> bool:
    """Check if a path should be skipped during sync."""
    try:
        rel_path = path.relative_to(docs_dir)
    except ValueError:
        return False

    # Check for root-level files to skip
    if str(rel_path) in ROOT_SKIP_FILES:
        return True

    # Check against skip patterns
    for pattern in SKIP_PATTERNS:
        if pattern in str(rel_path):
            return True
        if rel_path.name == pattern:
            return True
        # Check if any parent matches
        for parent in rel_path.parents:
            if parent.name == pattern:
                return True

    return False


def copy_assets(
    assets_dir: Path,
    wiki_dir: Path,
    dry_run: bool = False,
) -> bool:
    """Copy assets directory to wiki."""
    if not assets_dir.exists():
        logger.warning(f"Assets directory not found: {assets_dir}")
        return True

    dest_assets = wiki_dir / "assets"

    if dry_run:
        logger.info(f"  [DRY RUN] Would copy {assets_dir} -> {dest_assets}")
        return True

    try:
        if dest_assets.exists():
            shutil.rmtree(dest_assets)
        shutil.copytree(assets_dir, dest_assets)
        logger.info(f"  Copied assets: {assets_dir} -> {dest_assets}")
        return True
    except OSError as e:
        logger.error(f"Error copying assets: {e}")
        return False


def generate_sidebar(wiki_structure: dict[str, str]) -> str:
    """Generate the wiki sidebar navigation."""
    sidebar = """# Fortress Rollback

**[[Home]]**

## Documentation

- [[User-Guide|User Guide]]
- [[Architecture]]
- [[Migration]]

## Specifications

- [[Specs|Overview]]
- [[Formal-Spec|Formal Specification]]
- [[Determinism-Model|Determinism Model]]
- [[API-Contracts|API Contracts]]
- [[Spec-Divergences|Spec Divergences]]

## Reference

- [[Changelog]]
- [[GGRS-Changelog-Archive|GGRS Changelog]]
- [[Fortress-vs-GGRS|Fortress vs GGRS]]
- [[TLAplus-Tooling-Research|TLA+ Tooling]]

## Community

- [[Contributing]]
- [[Code-of-Conduct|Code of Conduct]]

---

[View on GitHub](https://github.com/wallstop/fortress-rollback)
"""
    return sidebar


def generate_home(docs_dir: Path, wiki_structure: dict[str, str]) -> str:
    """Generate the Home.md landing page from index.md."""
    index_path = docs_dir / "index.md"
    content = read_file_safe(index_path)

    if content is not None:
        content = strip_mkdocs_frontmatter(content)
        content = strip_mkdocs_features(content)
        content = convert_links(content, "index.md", wiki_structure)
        return content

    # Fallback content if index.md doesn't exist
    return """# Fortress Rollback

**Deterministic Rollback Netcode Built on Correctness**

Fortress Rollback is a correctness-first Rust library for peer-to-peer rollback
networking in deterministic multiplayer games.

## Quick Links

- [[User-Guide|User Guide]] - Get started with Fortress Rollback
- [[Architecture]] - Understand the system design
- [[Migration]] - Migrate from GGRS
- [[Changelog]] - See what's new

## Specifications

- [[Formal-Spec|Formal Specification]] - TLA+ and Z3 verified protocols
- [[Determinism-Model|Determinism Model]] - How determinism is guaranteed
- [[API-Contracts|API Contracts]] - Public API guarantees

## Contributing

- [[Contributing]] - How to contribute
- [[Code-of-Conduct|Code of Conduct]] - Community guidelines

---

This wiki is automatically synced from the
[main repository](https://github.com/wallstop/fortress-rollback).
"""


def clean_wiki_dir(wiki_dir: Path, dry_run: bool = False) -> None:
    """Clean wiki directory, preserving .git folder."""
    if not wiki_dir.exists():
        return

    for item in wiki_dir.iterdir():
        if item.name == ".git":
            continue
        try:
            if dry_run:
                logger.info(f"  [DRY RUN] Would remove: {item}")
            elif item.is_dir():
                shutil.rmtree(item)
                logger.debug(f"  Removed directory: {item}")
            else:
                item.unlink()
                logger.debug(f"  Removed file: {item}")
        except OSError as e:
            logger.warning(f"Could not remove {item}: {e}")


def discover_docs(docs_dir: Path) -> dict[str, str]:
    """
    Discover markdown files in docs directory and create wiki structure mapping.

    Returns a dict mapping relative doc paths to wiki page names.
    """
    structure = dict(WIKI_STRUCTURE)  # Start with predefined structure

    # Find all markdown files
    for md_file in docs_dir.rglob("*.md"):
        if should_skip(md_file, docs_dir):
            continue

        rel_path = str(md_file.relative_to(docs_dir))
        rel_path = normalize_path(rel_path)

        # Skip if already in structure
        if rel_path in structure:
            continue

        # Auto-generate wiki name
        wiki_name = path_to_wiki_name(rel_path)
        structure[rel_path] = wiki_name
        logger.debug(f"  Discovered: {rel_path} -> {wiki_name}")

    return structure


def main() -> int:
    """Main entry point."""
    parser = argparse.ArgumentParser(
        description="Sync documentation to GitHub Wiki format.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
    python scripts/sync-wiki.py
    python scripts/sync-wiki.py --source docs --dest wiki
    python scripts/sync-wiki.py --dry-run --verbose
        """,
    )
    parser.add_argument(
        "--source",
        "-s",
        type=Path,
        default=DEFAULT_DOCS_DIR,
        help=f"Source documentation directory (default: {DEFAULT_DOCS_DIR})",
    )
    parser.add_argument(
        "--dest",
        "-d",
        type=Path,
        default=DEFAULT_WIKI_DIR,
        help=f"Destination wiki directory (default: {DEFAULT_WIKI_DIR})",
    )
    parser.add_argument(
        "--assets",
        "-a",
        type=Path,
        default=DEFAULT_ASSETS_DIR,
        help=f"Assets directory to copy (default: {DEFAULT_ASSETS_DIR})",
    )
    parser.add_argument(
        "--dry-run",
        "-n",
        action="store_true",
        help="Show what would be done without making changes",
    )
    parser.add_argument(
        "--verbose",
        "-v",
        action="store_true",
        help="Enable verbose logging",
    )
    parser.add_argument(
        "--clean",
        action="store_true",
        default=True,
        help="Clean wiki directory before sync (default: True)",
    )
    parser.add_argument(
        "--no-clean",
        action="store_false",
        dest="clean",
        help="Don't clean wiki directory before sync",
    )

    args = parser.parse_args()

    if args.verbose:
        logging.getLogger().setLevel(logging.DEBUG)

    docs_dir = args.source
    wiki_dir = args.dest
    assets_dir = args.assets

    logger.info("Syncing documentation to GitHub Wiki...")
    if args.dry_run:
        logger.info("[DRY RUN MODE - No changes will be made]")

    errors = 0

    # Validate source directory
    if not docs_dir.exists():
        logger.error(f"Source directory not found: {docs_dir}")
        return 1

    # Discover all documentation files
    logger.info("\nDiscovering documentation files...")
    wiki_structure = discover_docs(docs_dir)
    logger.info(f"  Found {len(wiki_structure)} files to sync")

    # Clean wiki directory
    if args.clean:
        logger.info("\nCleaning wiki directory...")
        clean_wiki_dir(wiki_dir, args.dry_run)

    # Ensure wiki directory exists
    if not args.dry_run:
        wiki_dir.mkdir(parents=True, exist_ok=True)

    # Process all mapped files
    logger.info("\nProcessing documentation files:")
    for src_rel, wiki_name in wiki_structure.items():
        src_path = docs_dir / src_rel

        if not src_path.exists():
            logger.warning(f"  File not found: {src_path}")
            continue

        if should_skip(src_path, docs_dir):
            logger.debug(f"  Skipping: {src_path}")
            continue

        # Skip index.md - it becomes Home.md via generate_home
        if src_rel == "index.md":
            continue

        if not process_file(
            src_path, wiki_name, docs_dir, wiki_dir, wiki_structure, args.dry_run
        ):
            errors += 1

    # Generate Home page
    logger.info("\nGenerating Home page...")
    home_content = generate_home(docs_dir, wiki_structure)
    if not write_file_safe(wiki_dir / "Home.md", home_content, args.dry_run):
        errors += 1

    # Generate sidebar
    logger.info("Generating sidebar...")
    sidebar_content = generate_sidebar(wiki_structure)
    if not write_file_safe(wiki_dir / "_Sidebar.md", sidebar_content, args.dry_run):
        errors += 1

    # Copy assets
    logger.info("\nCopying assets...")
    if not copy_assets(assets_dir, wiki_dir, args.dry_run):
        errors += 1

    # Summary
    if errors > 0:
        logger.error(f"\nWiki sync completed with {errors} error(s)!")
        return 1

    logger.info("\nWiki sync complete!")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
