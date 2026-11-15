#!/usr/bin/env python3
"""Validate Agent Skills markdown code examples without line-by-line shells."""

from __future__ import annotations

import argparse
import re
import sys
from collections.abc import Iterator
from pathlib import Path

MAX_LINES = 500
JUSTIFICATION_REACH = 8

RED = "\033[0;31m"
GREEN = "\033[0;32m"
YELLOW = "\033[0;33m"
CYAN = "\033[0;36m"
BOLD = "\033[1m"
NC = "\033[0m"

RUST_FENCE_RE = re.compile(r"^```rust")
ANY_FENCE_RE = re.compile(r"^```")
SHELL_FENCE_RE = re.compile(r"^```(?:bash|shell|sh)(?:\s|$)")
FORBIDDEN_CONTEXT_RE = re.compile(
    r"(forbidden|wrong|bad example|what not to do|never use|do not use)",
    re.IGNORECASE,
)
JUSTIFICATION_RE = re.compile(
    r"//\s*(build\.rs:|test:|Loom test:|Fuzz target|proptest:|allowed:|SAFETY:|In tests:)"
)
ATTRIBUTE_RE = re.compile(r"^\s*#\[(?:allow\(|test|fixture|cfg\(test\))")
UNWRAP_RE = re.compile(r"\.(?:unwrap|expect)\(")
ALLOW_RE = re.compile(r"#\[allow\(")
AMBIGUOUS_RESULT_RE = re.compile(r"collect::<Result<[^,>]+>>")
COMMENT_RE = re.compile(r"^\s*//")


class Violations:
    """Collect and print violations in the established format."""

    def __init__(self, fix_mode: bool) -> None:
        self.count = 0
        self.fix_mode = fix_mode

    def report(self, file: Path, line: int, check: str, message: str) -> None:
        self.count += 1
        print(f"  {RED}[{check}]{NC} {file}:{line}: {message}")

    def suggest(self, suggestion: str) -> None:
        if self.fix_mode:
            print(f"    {CYAN}FIX:{NC} {suggestion}")


def _trimmed(line: str) -> str:
    return line.strip()[:100]


def _iter_blocks(
    lines: list[str], fence_re: re.Pattern[str]
) -> Iterator[tuple[int, list[str]]]:
    """Yield (start_index, block_lines) for fenced blocks matching fence_re."""
    index = 0
    while index < len(lines):
        if not fence_re.match(lines[index]):
            index += 1
            continue
        start = index
        index += 1
        block: list[str] = []
        while index < len(lines) and not ANY_FENCE_RE.match(lines[index]):
            block.append(lines[index])
            index += 1
        yield start, block
        index += 1


def check_unguarded_unwrap(path: Path, lines: list[str], out: Violations) -> None:
    in_rust = False
    forbidden_block = False
    pre_fence = ""
    last_justification = 0

    for index, line in enumerate(lines, start=1):
        if RUST_FENCE_RE.match(line):
            in_rust = True
            forbidden_block = bool(FORBIDDEN_CONTEXT_RE.search(pre_fence))
            last_justification = 0
            continue
        if not in_rust:
            stripped = line.strip()
            if stripped and not stripped.startswith("```"):
                pre_fence = line
            continue
        if ANY_FENCE_RE.match(line):
            in_rust = False
            continue
        if forbidden_block:
            continue
        if COMMENT_RE.match(line):
            if JUSTIFICATION_RE.search(line):
                last_justification = index
            continue
        if ATTRIBUTE_RE.match(line):
            last_justification = index
        if not UNWRAP_RE.search(line):
            continue

        justified = bool(JUSTIFICATION_RE.search(line) or ALLOW_RE.search(line))
        if (
            not justified
            and last_justification
            and index - last_justification <= JUSTIFICATION_REACH
        ):
            justified = True
        if not justified:
            out.report(
                path,
                index,
                "unwrap",
                f".unwrap()/.expect() without justifying comment: {_trimmed(line)}",
            )
            out.suggest("Add a comment like: // test: or // allowed: <reason>")


def check_ambiguous_collect(path: Path, lines: list[str], out: Violations) -> None:
    for start, block in _iter_blocks(lines, re.compile(r"^```rust")):
        for offset, line in enumerate(block, start=start + 2):
            if AMBIGUOUS_RESULT_RE.search(line):
                out.report(
                    path,
                    offset,
                    "ambiguous-result",
                    f"collect::<Result<...>> missing explicit error type: {_trimmed(line)}",
                )
                out.suggest("Specify the error type: collect::<Result<T, E>>()")


def check_catch_unwind(path: Path, lines: list[str], out: Violations) -> None:
    for start, block in _iter_blocks(lines, re.compile(r"^```rust")):
        if not any("catch_unwind" in line for line in block):
            continue
        if any("AssertUnwindSafe" in line for line in block):
            continue
        for offset, line in enumerate(block, start=start + 2):
            if "catch_unwind" in line:
                out.report(
                    path,
                    offset,
                    "catch-unwind",
                    f"catch_unwind without AssertUnwindSafe: {_trimmed(line)}",
                )
                out.suggest(
                    "Wrap the closure: catch_unwind(AssertUnwindSafe(|| { ... }))"
                )


def check_line_count(path: Path, lines: list[str], out: Violations) -> None:
    count = len(lines)
    if count > MAX_LINES:
        over = count - MAX_LINES
        out.report(
            path,
            count,
            "line-limit",
            f"File has {count} lines ({over} over the {MAX_LINES}-line limit)",
        )
        out.suggest("Split into smaller focused documents or trim content")


def check_dev_null(path: Path, lines: list[str], out: Violations) -> None:
    for start, block in _iter_blocks(lines, SHELL_FENCE_RE):
        for offset, line in enumerate(block, start=start + 2):
            if "2>/dev/null" in line:
                out.report(
                    path,
                    offset,
                    "dev-null",
                    f"2>/dev/null in shell code block (banned): {_trimmed(line)}",
                )
                out.suggest(
                    "Remove 2>/dev/null; handle errors explicitly or use '|| true'"
                )


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Validate Agent Skills markdown code examples."
    )
    parser.add_argument(
        "--fix",
        action="store_true",
        help="Print fix suggestions for each violation.",
    )
    parser.add_argument(
        "--dir",
        type=Path,
        default=None,
        help="Directory to scan (default: .agents/skills/).",
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    repo_root = Path(__file__).resolve().parents[2]
    search_dir = args.dir if args.dir is not None else repo_root / ".agents" / "skills"

    if not search_dir.is_dir():
        print(f"{YELLOW}Warning: Directory '{search_dir}' does not exist, skipping{NC}")
        return 0

    files = sorted(path for path in search_dir.rglob("*.md") if path.is_file())
    if not files:
        print(f"{YELLOW}No .md files found under {search_dir}{NC}")
        return 0

    print(f"{BOLD}Checking .agents/skills/ markdown files for code quality issues...{NC}")
    print("")
    print(f"Scanning {len(files)} markdown file(s)...")
    print("")

    out = Violations(args.fix)
    for path in files:
        lines = path.read_text(encoding="utf-8").splitlines()
        check_unguarded_unwrap(path, lines, out)
        check_ambiguous_collect(path, lines, out)
        check_catch_unwind(path, lines, out)
        check_line_count(path, lines, out)
        check_dev_null(path, lines, out)

    print("")
    if out.count:
        print(f"{RED}{BOLD}Found {out.count} violation(s){NC}")
        if not args.fix:
            print(f"Run with {CYAN}--fix{NC} to see fix suggestions")
        return 1
    print(f"{GREEN}All checks passed{NC}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
