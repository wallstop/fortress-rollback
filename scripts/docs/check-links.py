#!/usr/bin/env python3
"""
Cross-platform link validation script for pre-commit hooks.

Validates:
- Local file references in markdown files
- Relative paths in code comments and documentation
- Anchor links within markdown files

Works on Windows, macOS, and Linux.

This is the CI and pre-commit source of truth for local link validation.
"""

import os
import re
import sys
from pathlib import Path
from typing import NamedTuple

SKIP_DIRS = {".claude", ".git", "node_modules", "progress", "target"}
SKIP_MARKDOWN_FILES = {"PLAN.md", "pr-description.md"}


class LinkCheckResult(NamedTuple):
    """Result of link checking."""

    errors: int
    warnings: int
    checked: int


class RustDocBlock(NamedTuple):
    """One contiguous Rust doc-comment block."""

    markdown: str
    is_module: bool


def get_project_root() -> Path:
    """Get the project root directory."""
    script_dir = Path(__file__).parent.resolve()
    return script_dir.parent.parent


def extract_markdown_anchors(content: str) -> set[str]:
    """Extract anchor IDs from markdown content.

    Uses the same algorithm as markdownlint (GitHub-flavored Markdown):
    1. Convert to lowercase
    2. Replace spaces with hyphens
    3. Remove special characters (keep alphanumeric and hyphens)
    4. Do NOT collapse multiple hyphens (slashes become double hyphens)
    """
    anchors = set()
    heading_counts: dict[str, int] = {}
    code_ranges = find_code_fence_ranges(content)

    # Match headers: # Header, ## Header, etc.
    header_pattern = re.compile(r"^#+\s+(.+)$", re.MULTILINE)
    for match in header_pattern.finditer(content):
        if in_code_block(match.start(), code_ranges):
            continue
        header_text = match.group(1).strip()
        anchor = markdown_anchor_id(header_text)
        if not anchor:
            continue
        suffix = heading_counts.get(anchor, 0)
        heading_counts[anchor] = suffix + 1
        anchors.add(anchor if suffix == 0 else f"{anchor}-{suffix}")

    # Match explicit anchor definitions: {#anchor-id}
    explicit_pattern = re.compile(r"\{#([\w-]+)\}")
    for match in explicit_pattern.finditer(content):
        if in_code_block(match.start(), code_ranges):
            continue
        anchors.add(match.group(1))

    return anchors


def markdown_anchor_id(header_text: str) -> str:
    """Return the GitHub-style anchor ID for one Markdown heading."""
    anchor = header_text.lower()
    anchor = anchor.replace(" ", "-")
    anchor = re.sub(r"[^\w-]", "", anchor)
    return anchor.strip("-")


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
    """Find ranges of inline code spans (backtick-delimited) to skip.

    Uses a state-based parser to properly handle:
    - Standard inline code: `code`
    - Multi-backtick delimiters: ``code with ` inside``
    - Arbitrary backtick counts: ```code``` (if not at line start)

    Fenced code blocks (3+ backticks at line start) are skipped as they
    are handled separately by find_code_fence_ranges().

    Unclosed inline code spans are intentionally not treated as code ranges.
    This prevents an unclosed backtick from incorrectly masking the rest
    of the document.
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

            # Find the closing backticks (exact count, not part of longer sequence)
            closing_pattern = "`" * backtick_count
            search_start = i
            end_pos = -1

            while True:
                candidate = content.find(closing_pattern, search_start)
                if candidate == -1:
                    break  # No more candidates

                # Verify this is exactly backtick_count backticks, not part of a longer sequence
                # Check character before (if exists) is not a backtick
                char_before_ok = candidate == 0 or content[candidate - 1] != "`"
                # Check character after (if exists) is not a backtick
                after_pos = candidate + backtick_count
                char_after_ok = after_pos >= n or content[after_pos] != "`"

                if char_before_ok and char_after_ok:
                    end_pos = candidate
                    break
                else:
                    # This match is part of a longer sequence, keep searching
                    search_start = candidate + 1

            if end_pos != -1:
                # Found closing - range is from first backtick to after closing backticks
                ranges.append((start, end_pos + backtick_count))
                i = end_pos + backtick_count
            # else: No closing found - not a valid inline code span.
            # The opening backticks are treated as literal text.
            # i is already past the opening backticks, so continue scanning.
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


def _rel(path: Path, root: Path) -> Path:
    """Return *path* relative to *root*, falling back to *path* on ValueError."""
    try:
        return path.relative_to(root)
    except ValueError:
        return path


def should_skip_markdown_file(rel_path: Path) -> bool:
    """Return True when a markdown file is outside the checked documentation set."""
    return (
        len(rel_path.parts) == 1
        and rel_path.name in SKIP_MARKDOWN_FILES
        or any(part in SKIP_DIRS for part in rel_path.parts)
    )


def check_markdown_link(
    source_file: Path, link_target: str, project_root: Path, verbose: bool = False
) -> tuple[bool, str]:
    """
    Check if a markdown link target is valid.

    Returns (is_valid, error_message).
    """
    # Skip external links
    if link_target.startswith(
        (
            "http://",
            "https://",
            "mailto:",
            "tel:",
            "ftp://",
            "data:",
            "command:",
            "vscode:",
        )
    ):
        return True, ""

    # Skip special links
    if link_target.startswith("#"):
        # Anchor link within the same file
        anchor = link_target[1:]
        try:
            content = source_file.read_text(encoding="utf-8")
            anchors = extract_markdown_anchors(content)
            if anchor.lower() not in {a.lower() for a in anchors}:
                return False, f"Anchor '{anchor}' not found in {_rel(source_file, project_root)}"
        except (OSError, UnicodeDecodeError) as exc:
            return False, f"Cannot read file to validate anchor '{anchor}': {exc}"
        return True, ""

    # Handle anchor in link: file.md#anchor
    anchor = None
    if "#" in link_target:
        link_target, anchor = link_target.split("#", 1)

    # Resolve relative path
    source_dir = source_file.parent
    if link_target.startswith("/"):
        target_path = (project_root / link_target.lstrip("/")).resolve()
    else:
        target_path = (source_dir / link_target).resolve()

    # Check if target exists
    if not target_path.exists():
        # For wiki files, try adding .md extension (GitHub Wiki uses extensionless links)
        if is_wiki_file(source_file, project_root):
            wiki_target_path = (source_dir / f"{link_target}.md").resolve()
            if wiki_target_path.exists():
                target_path = wiki_target_path
            else:
                return False, f"Link target not found: {link_target} (from {_rel(source_file, project_root)})"
        else:
            return False, f"Link target not found: {link_target} (from {_rel(source_file, project_root)})"

    # If there's an anchor, check it exists in target file
    if anchor and target_path.suffix.lower() == ".md":
        try:
            content = target_path.read_text(encoding="utf-8")
            anchors = extract_markdown_anchors(content)
            if anchor.lower() not in {a.lower() for a in anchors}:
                return (
                    False,
                    f"Anchor '{anchor}' not found in {_rel(target_path, project_root)}",
                )
        except (OSError, UnicodeDecodeError) as exc:
            return (
                False,
                f"Cannot read {_rel(target_path, project_root)} to validate anchor '{anchor}': {exc}",
            )

    return True, ""


def is_probable_local_path(link_path: str) -> bool:
    """Return True when a link path should be treated as a filesystem path."""
    if not link_path:
        return False
    return (
        link_path.startswith((".", "/"))
        or "/" in link_path
        or Path(link_path).suffix != ""
    )


def clean_link_target(link_target: str) -> str:
    """Normalize a Markdown/HTML link target before local validation."""
    link_target = link_target.strip()
    if link_target.startswith("<") and ">" in link_target:
        return link_target[1 : link_target.index(">")].strip()
    return link_target.split(maxsplit=1)[0] if link_target else ""


def iter_markdown_links(content: str):
    """Yield ``(link_text, link_target)`` pairs from Markdown-like content.

    ``link_text`` is ``None`` for link forms that have no inline text (reference
    definitions and HTML ``href``/``src`` attributes). Inline links yield their
    raw bracket text so callers can compare it against the (cleaned) target --
    for example to catch a backticked item name pointing at the wrong page.
    """
    code_ranges = find_code_fence_ranges(content)
    inline_code_ranges = find_inline_code_ranges(content)

    link_pattern = re.compile(r"\[([^\]]*)\]\(([^)]+)\)")
    for match in link_pattern.finditer(content):
        if in_code_block(match.start(), code_ranges):
            continue
        if in_code_span(match.start(), inline_code_ranges):
            continue
        yield match.group(1), clean_link_target(match.group(2))

    ref_link_pattern = re.compile(r"^\s*\[[^\]]+\]:\s*(.+?)\s*$", re.MULTILINE)
    for match in ref_link_pattern.finditer(content):
        if in_code_block(match.start(), code_ranges):
            continue
        yield None, clean_link_target(match.group(1))

    html_link_pattern = re.compile(r"""\b(?:href|src)=["']([^"']+)["']""", re.IGNORECASE)
    for match in html_link_pattern.finditer(content):
        if in_code_block(match.start(), code_ranges):
            continue
        if in_code_span(match.start(), inline_code_ranges):
            continue
        yield None, clean_link_target(match.group(1))


def iter_markdown_link_targets(content: str):
    """Yield local-link candidate targets from Markdown-like content."""
    for _link_text, link_target in iter_markdown_links(content):
        yield link_target


def extract_rust_doc_markdown(content: str) -> str:
    """Extract Markdown text from Rust doc comments in source order."""
    return "\n".join(block.markdown for block in extract_rust_doc_blocks(content))


def extract_rust_doc_blocks(content: str) -> list[RustDocBlock]:
    """Extract contiguous Rust doc-comment blocks in source order."""
    blocks: list[RustDocBlock] = []
    doc_lines: list[str] = []
    current_is_module: bool | None = None

    def flush() -> None:
        nonlocal doc_lines, current_is_module
        if doc_lines and current_is_module is not None:
            blocks.append(RustDocBlock("\n".join(doc_lines), current_is_module))
        doc_lines = []
        current_is_module = None

    for line in content.splitlines():
        stripped = line.lstrip()
        if stripped.startswith(("///", "//!")):
            is_module = stripped.startswith("//!")
            if current_is_module is not None and is_module != current_is_module:
                flush()
            current_is_module = is_module
            text = stripped[3:]
            if text.startswith(" "):
                text = text[1:]
            doc_lines.append(text)
        else:
            flush()
    flush()
    return blocks


def is_rustdoc_item_fragment(anchor: str) -> bool:
    """Return True when *anchor* is a rustdoc-generated item fragment.

    Rustdoc item fragments such as `method.foo` and `structfield.bar` are scoped
    to the documented item. The local checker rejects URL-style links to these
    fragments so authors use rustdoc intra-doc paths that rustdoc can validate.
    """
    try:
        anchor_kind, item_name = anchor.split(".", 1)
    except ValueError:
        return False

    if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", item_name):
        return False

    return anchor_kind in {
        "associatedconstant",
        "associatedtype",
        "constant",
        "method",
        "structfield",
        "variant",
    }


# Crate-internal rustdoc intra-doc path prefixes. Links starting with one of
# these resolve against the current crate, so their final `::` segment must name
# the same item as the backticked link text. External-crate paths (e.g.
# `bincode::config::Configuration`) are deliberately excluded: their text may
# legitimately name a re-exported or differently-scoped item, and a manual audit
# found that flagging them produced false positives.
INTRA_DOC_PATH_PREFIXES = ("crate::", "super::", "self::")


def backticked_item_text(link_text: str) -> str | None:
    """Return the item named by single-backticked rustdoc link text, else None.

    Recognizes link text that is exactly one backticked identifier or qualified
    path, e.g. `` `Foo` `` or `` `Foo::bar` ``. Returns the final `::`-segment
    with any trailing ``()`` call parens and generic arguments stripped, so
    `` `decode_message()` `` -> ``decode_message`` and `` `ProofVec<U>` `` ->
    ``ProofVec``. Returns None for prose text, multi-token text, empty spans, or
    anything that is not a plain identifier path.
    """
    text = link_text.strip()
    if len(text) < 3 or not text.startswith("`") or not text.endswith("`"):
        return None
    inner = text[1:-1].strip()
    if not inner or "`" in inner:
        return None

    # Drop generic arguments anywhere in the path (e.g. `Vec<T>::iter`), then any
    # trailing call parens, before taking the final path segment.
    inner = re.sub(r"<[^`]*>", "", inner)
    last_segment = inner.split("::")[-1].strip()
    if last_segment.endswith("()"):
        last_segment = last_segment[:-2].strip()
    elif last_segment.endswith(")") and "(" in last_segment:
        last_segment = last_segment[: last_segment.index("(")].strip()

    if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", last_segment):
        return None
    return last_segment


def intra_doc_target_item(link_target: str) -> str | None:
    """Return the final `::` segment of a crate-internal intra-doc path, else None.

    Only ``crate::``/``super::``/``self::`` targets qualify; everything else
    (external crates, ``Self::``, bare relative paths, anchors) returns None so
    the mismatch check stays scoped to crate-internal links. Generic arguments
    and a trailing ``()`` are stripped, symmetric with `backticked_item_text`, so
    ``crate::m::Foo<T>`` -> ``Foo``.

    Known, deliberately-unhandled edge cases (near-nil in practice, kept out to
    keep the check simple and false-positive-free): reference-style links
    (``[`Foo`][ref]`` — the text is checked only for inline links, see
    `iter_markdown_links`) and raw identifiers (``r#type``).
    """
    if not link_target.startswith(INTRA_DOC_PATH_PREFIXES):
        return None
    path = link_target.split("#", 1)[0]
    # Strip generic arguments anywhere in the path (mirrors `backticked_item_text`)
    # so a target like ``crate::m::Foo<T>`` compares as ``Foo``.
    path = re.sub(r"<[^`]*>", "", path)
    last_segment = path.split("::")[-1].strip()
    if last_segment.endswith("()"):
        last_segment = last_segment[:-2].strip()
    if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", last_segment):
        return None
    return last_segment


def intra_doc_link_text_mismatch(
    link_text: str | None, link_target: str
) -> tuple[str, str] | None:
    """Return ``(text_item, target_item)`` when a link's text and target disagree.

    Detects the defect class where backticked link text names a specific item but
    a crate-internal intra-doc target points at a different (usually enclosing)
    item, so the rendered link silently lands on the wrong rustdoc page even
    though it resolves. Returns None when there is no such mismatch.
    """
    if link_text is None:
        return None
    text_item = backticked_item_text(link_text)
    if text_item is None:
        return None
    target_item = intra_doc_target_item(link_target)
    if target_item is None:
        return None
    if text_item == target_item:
        return None
    return text_item, target_item


def intra_doc_mismatch_error(
    link_text: str | None, link_target: str
) -> tuple[bool, str] | None:
    """Return a ``(False, message)`` failure for a text/target mismatch, else None.

    Wraps :func:`intra_doc_link_text_mismatch` with the user-facing diagnostic so
    every rustdoc intra-doc rubber-stamp path can share one report. The hint is
    deliberately non-prescriptive about the fix: linking directly to the named
    item is correct only when it is reachable without tripping rustdoc's
    ``private_intra_doc_links`` lint (e.g. a ``pub(crate)`` item inside a ``pub``
    module is not); otherwise de-link to a plain code span or link the module
    with module-named text.
    """
    mismatch = intra_doc_link_text_mismatch(link_text, link_target)
    if mismatch is None:
        return None
    text_item, target_item = mismatch
    return (
        False,
        f"Rustdoc intra-doc link text `{text_item}` points at `{target_item}` "
        f"('{link_target}'); the link resolves but lands on the wrong page. Link "
        f"to the item itself if it is reachable without tripping "
        f"rustdoc::private_intra_doc_links, otherwise use a plain code span (no "
        f"link) or link the module with module-named text.",
    )


def check_rust_doc_link(
    source_file: Path,
    link_target: str,
    project_root: Path,
    verbose: bool = False,
    current_doc_markdown: str | None = None,
    module_doc_markdown: str | None = None,
    link_text: str | None = None,
) -> tuple[bool, str]:
    """Check Markdown URL targets extracted from Rust doc comments."""
    if link_target.startswith(
        (
            "http://",
            "https://",
            "mailto:",
            "tel:",
            "ftp://",
            "data:",
            "command:",
            "vscode:",
        )
    ):
        return True, ""

    link_path, anchor = split_link_target(link_target)

    if anchor and is_rustdoc_item_fragment(anchor):
        return (
            False,
            "Rustdoc item-fragment URL "
            f"'{link_target}' is not linted by rustdoc; use an intra-doc path link instead",
        )

    # The caller (`check_rust_doc_file`) parses the doc blocks once and passes the
    # already-extracted Markdown for every link in a file. Only read (and re-parse)
    # the source as a fallback when a caller omits one of those — never per link in
    # the common path, which would be O(links) redundant I/O on large crates.
    current_markdown = current_doc_markdown
    module_markdown = module_doc_markdown
    if current_markdown is None or module_markdown is None:
        try:
            content = source_file.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError) as exc:
            return False, f"Cannot read Rust source to validate doc link '{link_target}': {exc}"
        fallback_markdown = extract_rust_doc_markdown(content)
        if current_markdown is None:
            current_markdown = fallback_markdown
        if module_markdown is None:
            module_markdown = fallback_markdown

    if anchor and link_path == "self":
        doc_anchors = {a.lower() for a in extract_markdown_anchors(module_markdown)}
        if anchor.lower() in doc_anchors:
            return True, ""
        return (
            False,
            f"Rustdoc module anchor '{anchor}' not found in {_rel(source_file, project_root)}",
        )

    if anchor and link_path in {"Self", "super", "crate"}:
        return (
            False,
            "Rustdoc path URL fragments are not linted by rustdoc; "
            f"use an intra-doc path link instead of '{link_target}'",
        )

    if link_path and not is_probable_local_path(link_path):
        if anchor:
            return (
                False,
                "Rustdoc path URL fragments are not linted by rustdoc; "
                f"use an intra-doc path link instead of '{link_target}'",
            )
        # A clean intra-doc path (e.g. `crate::network::codec`) resolves, so no
        # broken-link lint fires -- but if the backticked link text names an item
        # the path's final segment does not, the link lands on the wrong page.
        mismatch_error = intra_doc_mismatch_error(link_text, link_target)
        if mismatch_error is not None:
            return mismatch_error
        return True, ""

    if link_target.startswith("#"):
        anchor = link_target[1:]
        doc_anchors = {a.lower() for a in extract_markdown_anchors(current_markdown)}
        if anchor.lower() not in doc_anchors:
            return (
                False,
                f"Rustdoc anchor '{anchor}' not found in {_rel(source_file, project_root)}",
            )
        return True, ""

    if link_target.startswith(("crate::", "super::", "self::", "Self::", "`")) or "::" in link_target:
        # Rustdoc resolves these intra-doc paths, so a broken-link lint will not
        # fire. But a crate-internal path whose final segment differs from the
        # backticked item named in the link text resolves to the WRONG page --
        # catch that mismatch here before rubber-stamping the link. (Clean
        # `crate::`/`super::`/`self::` paths are handled in the non-local-path
        # branch above; this also covers path-like intra-doc targets.)
        mismatch_error = intra_doc_mismatch_error(link_text, link_target)
        if mismatch_error is not None:
            return mismatch_error
        return True, ""

    return check_markdown_link(source_file, link_target, project_root, verbose)


def split_link_target(link_target: str) -> tuple[str, str | None]:
    """Split a URL-ish link target into path and optional fragment."""
    if "#" not in link_target:
        return link_target, None
    link_path, anchor = link_target.split("#", 1)
    return link_path, anchor


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
        try:
            rel_path = file_path.relative_to(project_root)
        except ValueError:
            rel_path = file_path
        print(f"ERROR: Could not read {rel_path}: {e}", file=sys.stderr)
        return LinkCheckResult(errors=1, warnings=0, checked=0)

    for link_target in iter_markdown_link_targets(content):
        checked += 1

        # Skip empty links
        if not link_target:
            continue

        is_valid, error_msg = check_markdown_link(
            file_path, link_target, project_root, verbose
        )
        if not is_valid:
            errors += 1
            rel_path = _rel(file_path, project_root)
            print(f"ERROR: {rel_path}: {error_msg}", file=sys.stderr)

    return LinkCheckResult(errors=errors, warnings=warnings, checked=checked)


def check_rust_doc_file(
    file_path: Path, project_root: Path, verbose: bool = False
) -> LinkCheckResult:
    """Check Markdown URL targets in Rust doc comments."""
    errors = 0
    checked = 0

    try:
        content = file_path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as e:
        print(f"ERROR: Could not read {_rel(file_path, project_root)}: {e}", file=sys.stderr)
        return LinkCheckResult(errors=1, warnings=0, checked=0)

    blocks = extract_rust_doc_blocks(content)
    module_doc_markdown = "\n".join(block.markdown for block in blocks if block.is_module)
    for block in blocks:
        for link_text, link_target in iter_markdown_links(block.markdown):
            checked += 1
            if not link_target:
                continue
            is_valid, error_msg = check_rust_doc_link(
                file_path,
                link_target,
                project_root,
                verbose,
                current_doc_markdown=block.markdown,
                module_doc_markdown=module_doc_markdown,
                link_text=link_text,
            )
            if not is_valid:
                errors += 1
                print(f"ERROR: {_rel(file_path, project_root)}: {error_msg}", file=sys.stderr)

    return LinkCheckResult(errors=errors, warnings=0, checked=checked)


def main() -> int:
    """Main entry point."""
    verbose = "--verbose" in sys.argv or "-v" in sys.argv

    project_root = get_project_root()
    os.chdir(project_root)

    total_errors = 0
    total_warnings = 0
    total_checked = 0
    files_checked = 0
    markdown_files_checked = 0
    rust_files_checked = 0

    # Find all markdown files
    for md_file in project_root.rglob("*.md"):
        # Skip files in excluded directories
        rel_path = md_file.relative_to(project_root)
        if should_skip_markdown_file(rel_path):
            continue

        result = check_markdown_file(md_file, project_root, verbose)
        total_errors += result.errors
        total_warnings += result.warnings
        total_checked += result.checked
        files_checked += 1
        markdown_files_checked += 1

    for rust_file in project_root.rglob("*.rs"):
        rel_path = rust_file.relative_to(project_root)
        if any(part in SKIP_DIRS for part in rel_path.parts):
            continue

        result = check_rust_doc_file(rust_file, project_root, verbose)
        total_errors += result.errors
        total_warnings += result.warnings
        total_checked += result.checked
        files_checked += 1
        rust_files_checked += 1

    # Print summary
    print(f"\nLink check complete:")
    print(f"  Files checked: {files_checked}")
    print(f"    Markdown files: {markdown_files_checked}")
    print(f"    Rust files: {rust_files_checked}")
    print(f"  Links checked: {total_checked}")
    print(f"  Errors: {total_errors}")
    print(f"  Warnings: {total_warnings}")

    if total_errors > 0:
        return 1

    print("[OK] All links valid")
    return 0


if __name__ == "__main__":
    sys.exit(main())
