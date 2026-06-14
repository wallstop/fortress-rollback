#!/usr/bin/env python3
"""Check Rust doc and test names for semantic claims that drift from code."""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path

SCAN_DIRS = ("src", "tests", "examples", "benches")

FN_RE = re.compile(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)\s*(?:<[^({;]*>)?\s*\(")
INCLUSIVE_RANGE_ASSERT_RE = re.compile(
    r"assert!\s*\(\s*"
    r"\(\s*(?P<lower>[0-9]+)\s*\.\.=\s*(?P<upper>[0-9]+)\s*\)"
    r"\s*\.contains\s*\(\s*&\s*(?P<param>[A-Za-z_][A-Za-z0-9_]*)\s*\)",
    re.DOTALL,
)
CREATE_CHANNEL_MESH_CALL_RE = re.compile(r"\bcreate_channel_mesh\s*\(")
DIVERGENCE_EXPR = r"(?P<var>[A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*)"
NO_DIVERGENCE_ASSERT_RES = (
    re.compile(
        rf"assert!\s*\(\s*{DIVERGENCE_EXPR}\.is_empty\s*\(",
        re.DOTALL,
    ),
    re.compile(
        rf"assert!\s*\(\s*{DIVERGENCE_EXPR}\.is_none\s*\(",
        re.DOTALL,
    ),
    re.compile(
        rf"assert_eq!\s*\(\s*{DIVERGENCE_EXPR}\s*,\s*None\b",
        re.DOTALL,
    ),
    re.compile(
        rf"assert_eq!\s*\(\s*None\s*,\s*{DIVERGENCE_EXPR}\b",
        re.DOTALL,
    ),
)
POSITIVE_DIVERGENCE_ASSERT_RES = (
    re.compile(
        rf"assert!\s*\(\s*!\s*{DIVERGENCE_EXPR}\.is_empty\s*\(",
        re.DOTALL,
    ),
    re.compile(
        rf"assert!\s*\(\s*{DIVERGENCE_EXPR}\.is_some\s*\(",
        re.DOTALL,
    ),
    re.compile(
        rf"assert_ne!\s*\(\s*{DIVERGENCE_EXPR}\s*,\s*None\b",
        re.DOTALL,
    ),
    re.compile(
        rf"assert_eq!\s*\(\s*{DIVERGENCE_EXPR}\s*,\s*Some\s*\(",
        re.DOTALL,
    ),
)


@dataclass(frozen=True)
class RustFunction:
    """A Rust function with the source context needed by this checker."""

    name: str
    line: int
    body: str
    masked_body: str
    doc_text: str
    attrs: tuple[str, ...]


@dataclass(frozen=True)
class Finding:
    """One semantic-claim mismatch."""

    path: Path
    line: int
    message: str
    fix: str


def repo_root_from_script() -> Path:
    """Return the repository root based on this script location."""
    return Path(__file__).resolve().parents[2]


def blank_range(chars: list[str], start: int, end: int) -> None:
    """Replace a source range with spaces while preserving newlines."""
    for index in range(start, min(end, len(chars))):
        if chars[index] != "\n":
            chars[index] = " "


def raw_string_end(text: str, index: int) -> int | None:
    """Return the exclusive end of a raw string that starts at index, if any."""
    if text.startswith("br", index):
        raw_index = index + 1
    elif text.startswith("r", index):
        raw_index = index
    else:
        return None

    cursor = raw_index + 1
    while cursor < len(text) and text[cursor] == "#":
        cursor += 1

    if cursor >= len(text) or text[cursor] != '"':
        return None

    hashes = cursor - raw_index - 1
    terminator = '"' + ("#" * hashes)
    end = text.find(terminator, cursor + 1)
    if end == -1:
        return len(text)
    return end + len(terminator)


def quoted_string_end(text: str, index: int) -> int:
    """Return the exclusive end of a normal quoted string."""
    cursor = index + 1
    while cursor < len(text):
        if text[cursor] == "\\":
            cursor += 2
            continue
        if text[cursor] == '"':
            return cursor + 1
        cursor += 1
    return len(text)


def char_literal_end(text: str, index: int) -> int | None:
    """Return the exclusive end of a char literal, avoiding lifetimes."""
    if index + 1 >= len(text):
        return None
    next_char = text[index + 1]
    if next_char.isalpha() or next_char == "_":
        return None

    cursor = index + 1
    while cursor < len(text):
        if text[cursor] == "\\":
            cursor += 2
            continue
        if text[cursor] == "'":
            return cursor + 1
        cursor += 1
    return None


def mask_non_code(text: str) -> str:
    """Mask comments and string literals so simple Rust scans avoid false hits."""
    chars = list(text)
    cursor = 0
    while cursor < len(text):
        raw_end = raw_string_end(text, cursor)
        if raw_end is not None:
            blank_range(chars, cursor, raw_end)
            cursor = raw_end
            continue

        if text.startswith("//", cursor):
            end = text.find("\n", cursor)
            if end == -1:
                end = len(text)
            blank_range(chars, cursor, end)
            cursor = end
            continue

        if text.startswith("/*", cursor):
            depth = 1
            inner = cursor + 2
            while inner < len(text) and depth > 0:
                if text.startswith("/*", inner):
                    depth += 1
                    inner += 2
                elif text.startswith("*/", inner):
                    depth -= 1
                    inner += 2
                else:
                    inner += 1
            blank_range(chars, cursor, inner)
            cursor = inner
            continue

        if text.startswith('b"', cursor):
            end = quoted_string_end(text, cursor + 1)
            blank_range(chars, cursor, end)
            cursor = end
            continue

        if text[cursor] == '"':
            end = quoted_string_end(text, cursor)
            blank_range(chars, cursor, end)
            cursor = end
            continue

        if text[cursor] == "'":
            end = char_literal_end(text, cursor)
            if end is not None:
                blank_range(chars, cursor, end)
                cursor = end
                continue

        cursor += 1

    return "".join(chars)


def find_matching_brace(masked: str, open_brace: int) -> int | None:
    """Return the matching closing brace in already-masked Rust source."""
    depth = 0
    for cursor in range(open_brace, len(masked)):
        if masked[cursor] == "{":
            depth += 1
        elif masked[cursor] == "}":
            depth -= 1
            if depth == 0:
                return cursor
    return None


def line_number(text: str, offset: int) -> int:
    """Return the 1-based line number for a byte offset."""
    return text.count("\n", 0, offset) + 1


def line_start_offsets(text: str) -> list[int]:
    """Return byte offsets for the start of each source line."""
    offsets = [0]
    for match in re.finditer("\n", text):
        offsets.append(match.end())
    return offsets


def line_index_for_offset(offsets: list[int], offset: int) -> int:
    """Return the 0-based source line index containing offset."""
    low = 0
    high = len(offsets)
    while low + 1 < high:
        mid = (low + high) // 2
        if offsets[mid] <= offset:
            low = mid
        else:
            high = mid
    return low


def strip_doc_marker(line: str) -> str:
    """Remove a Rust outer-doc marker from one source line."""
    return re.sub(r"^[ \t]*/// ?", "", line.rstrip("\n"))


def function_metadata(lines: list[str], fn_line_index: int) -> tuple[str, tuple[str, ...]]:
    """Return attached rustdoc text and attributes for a function line."""
    cursor = fn_line_index - 1
    attrs: list[str] = []
    while cursor >= 0 and lines[cursor].strip().startswith("#["):
        attrs.insert(0, lines[cursor].strip())
        cursor -= 1

    doc_lines: list[str] = []
    while cursor >= 0 and lines[cursor].lstrip().startswith("///"):
        doc_lines.insert(0, strip_doc_marker(lines[cursor]))
        cursor -= 1

    return "\n".join(doc_lines), tuple(attrs)


def iter_functions(text: str) -> list[RustFunction]:
    """Extract Rust functions from source text."""
    masked = mask_non_code(text)
    offsets = line_start_offsets(text)
    lines = text.splitlines(keepends=True)
    functions: list[RustFunction] = []

    for match in FN_RE.finditer(masked):
        open_brace = masked.find("{", match.end())
        semicolon = masked.find(";", match.end())
        if open_brace == -1 or (semicolon != -1 and semicolon < open_brace):
            continue

        close_brace = find_matching_brace(masked, open_brace)
        if close_brace is None:
            continue

        fn_line_index = line_index_for_offset(offsets, match.start())
        doc_text, attrs = function_metadata(lines, fn_line_index)
        functions.append(
            RustFunction(
                name=match.group(1),
                line=line_number(text, match.start()),
                body=text[open_brace : close_brace + 1],
                masked_body=masked[open_brace : close_brace + 1],
                doc_text=doc_text,
                attrs=attrs,
            )
        )

    return functions


def is_test_function(function: RustFunction) -> bool:
    """Return true when a function is marked as a Rust test."""
    return any("test" in attr for attr in function.attrs)


def name_claims_positive_divergence(name: str) -> bool:
    """Return true when a test name says divergence is the expected outcome."""
    tokens = name.lower().split("_")
    if any(token in tokens for token in ("without", "no", "not", "prevents", "avoids")):
        return False
    return any(token in tokens for token in ("diverges", "divergent", "divergence"))


def body_has_divergence_var_assertion(
    body: str,
    regexes: tuple[re.Pattern[str], ...],
) -> bool:
    """Return true when any regex assertion targets a divergence-named value."""
    for regex in regexes:
        for match in regex.finditer(body):
            if "diverg" in match.group("var").lower():
                return True
    return False


def body_asserts_no_divergence(body: str) -> bool:
    """Return true when a body has a direct no-divergence assertion."""
    return body_has_divergence_var_assertion(body, NO_DIVERGENCE_ASSERT_RES)


def body_asserts_positive_divergence(body: str) -> bool:
    """Return true when a body directly asserts a divergence exists."""
    return body_has_divergence_var_assertion(body, POSITIVE_DIVERGENCE_ASSERT_RES)


def number_token_present(text: str, number: str) -> bool:
    """Return true when number appears as a standalone numeric token."""
    return re.search(rf"(?<![0-9]){re.escape(number)}(?![0-9])", text) is not None


def panics_section(doc_text: str) -> str:
    """Extract a rustdoc # Panics section, if present."""
    lines = doc_text.splitlines()
    section: list[str] = []
    in_section = False
    for line in lines:
        if re.match(r"^[ \t]*#+[ \t]+Panics\b", line, flags=re.IGNORECASE):
            in_section = True
            section.append(line)
            continue
        if in_section and re.match(r"^[ \t]*#+[ \t]+[A-Za-z]", line):
            break
        if in_section:
            section.append(line)
    return "\n".join(section)


def check_test_name_claims(path: Path, function: RustFunction) -> list[Finding]:
    """Check test names whose positive divergence claim disagrees with assertions."""
    if not is_test_function(function):
        return []
    if not name_claims_positive_divergence(function.name):
        return []
    if body_asserts_positive_divergence(function.masked_body):
        return []
    if not body_asserts_no_divergence(function.masked_body):
        return []

    return [
        Finding(
            path=path,
            line=function.line,
            message=(
                f"test `{function.name}` says divergence is expected, but its body "
                "asserts a divergence collection/option is empty or absent"
            ),
            fix=(
                "Rename the test to describe the no-divergence/convergence oracle, "
                "or change the assertion if the intended oracle is positive divergence."
            ),
        )
    ]


def check_range_contracts(path: Path, function: RustFunction) -> list[Finding]:
    """Check inclusive-range assertions against attached rustdoc contracts."""
    if not function.doc_text:
        return []

    findings: list[Finding] = []
    for match in INCLUSIVE_RANGE_ASSERT_RE.finditer(function.masked_body):
        lower = match.group("lower")
        upper = match.group("upper")
        param = match.group("param")

        missing_doc_bounds = [
            bound
            for bound in (lower, upper)
            if not number_token_present(function.doc_text, bound)
        ]
        if missing_doc_bounds:
            findings.append(
                Finding(
                    path=path,
                    line=function.line,
                    message=(
                        f"`{function.name}` asserts `{param}` is in `{lower}..={upper}`, "
                        f"but its rustdoc omits bound(s): {', '.join(missing_doc_bounds)}"
                    ),
                    fix=(
                        "Document the full accepted range in the argument contract so "
                        "callers see the same bounds the implementation enforces."
                    ),
                )
            )

        panic_text = panics_section(function.doc_text)
        if panic_text:
            missing_panic_bounds = [
                bound
                for bound in (lower, upper)
                if not number_token_present(panic_text, bound)
            ]
            if missing_panic_bounds:
                findings.append(
                    Finding(
                        path=path,
                        line=function.line,
                        message=(
                            f"`{function.name}` panics outside `{lower}..={upper}`, "
                            "but its # Panics section omits bound(s): "
                            f"{', '.join(missing_panic_bounds)}"
                        ),
                        fix=(
                            "Document every panic condition from the range assertion "
                            "inside # Panics."
                        ),
                    )
                )

    return findings


def check_delegated_mesh_contracts(path: Path, function: RustFunction) -> list[Finding]:
    """Check helpers that inherit `create_channel_mesh`'s panic contract."""
    if function.name == "create_channel_mesh":
        return []
    if not function.doc_text:
        return []
    if not CREATE_CHANNEL_MESH_CALL_RE.search(function.masked_body):
        return []

    panic_text = panics_section(function.doc_text)
    if not panic_text:
        return [
            Finding(
                path=path,
                line=function.line,
                message=(
                    f"`{function.name}` delegates to `create_channel_mesh`, whose "
                    "accepted range is `2..=1000`, but its rustdoc has no # Panics "
                    "section"
                ),
                fix=(
                    "Add a # Panics section that documents the delegated mesh-size "
                    "panic contract, including the `n > 1000` upper-bound case."
                ),
            )
        ]

    missing_panic_bounds = [
        bound for bound in ("2", "1000") if not number_token_present(panic_text, bound)
    ]
    if not missing_panic_bounds:
        return []

    return [
        Finding(
            path=path,
            line=function.line,
            message=(
                f"`{function.name}` delegates to `create_channel_mesh`, whose accepted "
                "range is `2..=1000`, but its # Panics section omits bound(s): "
                f"{', '.join(missing_panic_bounds)}"
            ),
            fix=(
                "Document the delegated mesh-size panic contract, including the "
                "`n > 1000` upper-bound case."
            ),
        )
    ]


def collect_rust_files(repo_root: Path, explicit_files: list[str]) -> list[Path]:
    """Collect Rust files to scan."""
    if explicit_files:
        files = []
        for raw_path in explicit_files:
            path = Path(raw_path)
            if not path.is_absolute():
                path = repo_root / path
            if path.suffix == ".rs" and path.exists():
                files.append(path)
        return sorted(set(files))

    files: list[Path] = []
    for dirname in SCAN_DIRS:
        root = repo_root / dirname
        if root.exists():
            files.extend(root.rglob("*.rs"))
    return sorted(files)


def check_file(path: Path) -> list[Finding]:
    """Check one Rust file."""
    text = path.read_text(encoding="utf-8")
    findings: list[Finding] = []
    for function in iter_functions(text):
        findings.extend(check_test_name_claims(path, function))
        findings.extend(check_range_contracts(path, function))
        findings.extend(check_delegated_mesh_contracts(path, function))
    return findings


def print_findings(repo_root: Path, findings: list[Finding]) -> None:
    """Print findings with stable, repo-relative paths."""
    for finding in findings:
        rel_path = finding.path.relative_to(repo_root)
        print("")
        print(f"ERROR: {rel_path}:{finding.line}")
        print(f"  {finding.message}.")
        print(f"  Fix: {finding.fix}")


def build_parser() -> argparse.ArgumentParser:
    """Build the command-line parser."""
    parser = argparse.ArgumentParser(
        description="Check Rust doc/test semantic claims against implementation clues."
    )
    parser.add_argument(
        "files",
        nargs="*",
        help="Optional Rust files to scan; defaults to src/, tests/, examples/, benches/.",
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    """Run the checker."""
    args = build_parser().parse_args(argv)
    repo_root = repo_root_from_script()
    rust_files = collect_rust_files(repo_root, args.files)

    findings: list[Finding] = []
    for path in rust_files:
        findings.extend(check_file(path))

    if not findings:
        print("SUCCESS: Rust semantic claims match checked implementation clues.")
        return 0

    print_findings(repo_root, findings)
    print("")
    print(f"FAILED: {len(findings)} Rust semantic claim mismatch(es) detected.")
    return 1


if __name__ == "__main__":
    sys.exit(main())
