#!/usr/bin/env python3
"""Validate SYNC headers in docs/ and wiki/ markdown mirrors.

Rules:
- SYNC headers must reference an existing markdown file in docs/ or wiki/.
- SYNC headers must not self-reference.
- wiki/* SYNC headers must point to docs/*.
- docs/* SYNC headers must point to wiki/*.
- The target file must contain a reciprocal SYNC header that points back.

Expected SYNC comment shape is flexible as long as it includes a target path:
  <!-- SYNC: ... docs/...md ... -->
  <!-- SYNC: ... wiki/...md ... -->
"""

from __future__ import annotations

import ast
import re
import sys
from pathlib import Path

SYNC_RE = re.compile(r"^<!--\s*SYNC:\s*(.*?)\s*-->\s*$")
TARGET_RE = re.compile(r"\b((?:docs|wiki)/[^\s>]+\.md)\b")
SYNC_REMEDIATION = "python scripts/docs/sync-wiki.py"


def _display_path(path: Path, repo_root: Path | None = None) -> str:
    """Convert path to a repo-relative path for diagnostics."""
    base = repo_root if repo_root is not None else Path.cwd()
    try:
        return str(path.resolve().relative_to(base.resolve()))
    except ValueError:
        return str(path)


def _find_case_insensitive_match(path: Path, repo_root: Path | None = None) -> str | None:
    """Return a repo-relative filename with different casing, if present."""
    parent = path.parent
    if not parent.exists():
        return None

    expected_name = path.name.lower()
    matches = []
    for candidate in parent.iterdir():
        if candidate.name.lower() != expected_name:
            continue
        if candidate.name == path.name:
            continue
        matches.append(candidate)

    if not matches:
        return None
    matches.sort(key=lambda p: p.name)
    return _display_path(matches[0], repo_root)


def _extract_sync_target(path: Path, repo_root: Path | None = None) -> tuple[int, str] | None:
    """Extract sync target from the first few lines of a markdown file."""
    try:
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    except OSError as exc:
        raise OSError(f"{_display_path(path, repo_root)}:0: cannot read file: {exc}") from exc

    for idx, line in enumerate(lines[:20], start=1):
        match = SYNC_RE.match(line.strip())
        if not match:
            continue
        target_match = TARGET_RE.search(match.group(1))
        if not target_match:
            return (idx, "")
        return (idx, target_match.group(1))
    return None


def _load_wiki_structure(sync_script: Path, repo_root: Path | None = None) -> dict[str, str]:
    """Load WIKI_STRUCTURE mapping from scripts/docs/sync-wiki.py via AST."""
    try:
        content = sync_script.read_text(encoding="utf-8", errors="replace")
    except OSError as exc:
        raise OSError(f"{_display_path(sync_script, repo_root)}:0: cannot read file: {exc}") from exc

    try:
        tree = ast.parse(content, filename=str(sync_script))
    except SyntaxError as exc:
        raise SyntaxError(f"{_display_path(sync_script, repo_root)}:0: cannot parse file: {exc}") from exc

    for node in ast.walk(tree):
        if isinstance(node, ast.Assign):
            for target in node.targets:
                if isinstance(target, ast.Name) and target.id == "WIKI_STRUCTURE":
                    if isinstance(node.value, ast.Dict):
                        mapping: dict[str, str] = {}
                        for key, value in zip(node.value.keys, node.value.values):
                            if isinstance(key, ast.Constant) and isinstance(value, ast.Constant):
                                mapping[str(key.value)] = str(value.value)
                        return mapping

    raise ValueError(f"{_display_path(sync_script, repo_root)}:0: WIKI_STRUCTURE not found")


def _check_file(repo_root: Path, rel_path: Path) -> list[str]:
    """Return violations for a single markdown file."""
    issues: list[str] = []
    if rel_path.suffix != ".md":
        return issues
    if not (rel_path.parts and rel_path.parts[0] in {"docs", "wiki"}):
        return issues

    abs_path = repo_root / rel_path
    try:
        sync_data = _extract_sync_target(abs_path, repo_root)
    except OSError as exc:
        return [str(exc)]
    if sync_data is None:
        return issues

    line_no, target = sync_data
    display_path = _display_path(abs_path, repo_root)

    if not target:
        issues.append(
            f"{display_path}:{line_no}: SYNC header must reference a docs/*.md or wiki/*.md target"
        )
        return issues

    if target == rel_path.as_posix():
        issues.append(f"{display_path}:{line_no}: SYNC header must not self-reference")

    if rel_path.parts[0] == "wiki" and not target.startswith("docs/"):
        issues.append(
            f"{display_path}:{line_no}: wiki SYNC header must point to docs/* source, got {target}"
        )

    if rel_path.parts[0] == "docs" and not target.startswith("wiki/"):
        issues.append(
            f"{display_path}:{line_no}: docs SYNC header must point to wiki/* mirror, got {target}"
        )

    target_abs = repo_root / target
    if not target_abs.exists():
        case_match = _find_case_insensitive_match(target_abs, repo_root)
        if case_match is not None:
            issues.append(
                f"{display_path}:{line_no}: SYNC target does not exist: {target} "
                f"(case mismatch; found {case_match})"
            )
        else:
            issues.append(
                f"{display_path}:{line_no}: SYNC target does not exist: {target} "
                f"(remediation: run {SYNC_REMEDIATION})"
            )
        return issues

    try:
        target_sync_data = _extract_sync_target(target_abs, repo_root)
    except OSError as exc:
        issues.append(str(exc))
        return issues
    if target_sync_data is None:
        issues.append(
            f"{display_path}:{line_no}: SYNC target missing reciprocal SYNC header: {target}"
        )
        return issues

    target_line, target_target = target_sync_data
    if target_target != rel_path.as_posix():
        issues.append(
            f"{_display_path(target_abs, repo_root)}:{target_line}: reciprocal SYNC header must reference {rel_path.as_posix()}"
        )

    return issues


def _check_required_pair(repo_root: Path, docs_rel: str, wiki_name: str) -> list[str]:
    """Validate required SYNC headers for a canonical docs/wiki mirror pair."""
    issues: list[str] = []
    docs_path = repo_root / "docs" / docs_rel
    wiki_rel = f"wiki/{wiki_name}.md"
    docs_expected = f"wiki/{wiki_name}.md"
    wiki_expected = f"docs/{docs_rel}"
    wiki_path = repo_root / wiki_rel

    if not docs_path.exists():
        issues.append(f"docs/{docs_rel}:0: expected docs mirror file is missing")
        return issues
    if not wiki_path.exists():
        case_match = _find_case_insensitive_match(wiki_path, repo_root)
        if case_match is not None:
            issues.append(
                f"wiki/{wiki_name}.md:0: expected wiki mirror file is missing "
                f"(case mismatch; found {case_match})"
            )
        else:
            issues.append(
                f"wiki/{wiki_name}.md:0: expected wiki mirror file is missing "
                f"(remediation: run {SYNC_REMEDIATION})"
            )
        return issues

    docs_read_ok = True
    try:
        docs_sync = _extract_sync_target(docs_path, repo_root)
    except OSError as exc:
        issues.append(str(exc))
        docs_sync = None
        docs_read_ok = False
    if docs_sync is None and docs_read_ok:
        issues.append(
            f"docs/{docs_rel}:1: missing SYNC header; expected target {docs_expected}"
        )
    elif docs_sync is not None:
        docs_line, docs_target = docs_sync
        if not docs_target:
            issues.append(
                f"docs/{docs_rel}:{docs_line}: SYNC header must reference a valid markdown target"
            )
        elif docs_target != docs_expected:
            issues.append(
                f"docs/{docs_rel}:{docs_line}: SYNC header must target {docs_expected}, got {docs_target}"
            )

    wiki_read_ok = True
    try:
        wiki_sync = _extract_sync_target(wiki_path, repo_root)
    except OSError as exc:
        issues.append(str(exc))
        wiki_sync = None
        wiki_read_ok = False
    if wiki_sync is None and wiki_read_ok:
        issues.append(
            f"wiki/{wiki_name}.md:1: missing SYNC header; expected target {wiki_expected}"
        )
    elif wiki_sync is not None:
        wiki_line, wiki_target = wiki_sync
        if not wiki_target:
            issues.append(
                f"wiki/{wiki_name}.md:{wiki_line}: SYNC header must reference a valid markdown target"
            )
        elif wiki_target != wiki_expected:
            issues.append(
                f"wiki/{wiki_name}.md:{wiki_line}: SYNC header must target {wiki_expected}, got {wiki_target}"
            )

    return issues


def main() -> int:
    """Validate SYNC headers for canonical docs/wiki mirror pairs."""
    repo_root = Path.cwd()
    issues: list[str] = []

    sync_script = repo_root / "scripts/docs/sync-wiki.py"
    try:
        wiki_structure = _load_wiki_structure(sync_script, repo_root)
    except (OSError, SyntaxError, ValueError) as exc:
        print(str(exc), file=sys.stderr)
        return 1

    validated: set[str] = set()
    for docs_rel, wiki_name in wiki_structure.items():
        issues.extend(_check_required_pair(repo_root, docs_rel, wiki_name))
        validated.add(f"docs/{docs_rel}")
        validated.add(f"wiki/{wiki_name}.md")

    # Also validate any SYNC headers that exist outside the required pairs.
    for root in ("docs", "wiki"):
        for path in (repo_root / root).rglob("*.md"):
            rel = path.relative_to(repo_root)
            if rel.as_posix() not in validated:
                issues.extend(_check_file(repo_root, rel))

    if issues:
        print("SYNC header validation failed:", file=sys.stderr)
        for issue in issues:
            print(issue, file=sys.stderr)
        print(
            f"hint: regenerate wiki mirrors with `{SYNC_REMEDIATION}` and restage changes",
            file=sys.stderr,
        )
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
