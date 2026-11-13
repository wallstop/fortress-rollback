#!/usr/bin/env python3
"""Inventory stable IDs for production ``report_violation!`` sites.

Each site's ID is a truncated SHA-256 of its ``(kind, format literal)`` pair.
Moving a site between lines or files does not change the ID; changing the kind
or message is a deliberate assertion change and requires regenerating the
checked-in baseline. Duplicate assertions with the same kind and literal share
one ID by design.

The scan is intentionally conservative:

- comments, string literals, ``#[cfg(test)]`` blocks, and ``#[cfg(kani)]`` /
  ``#[cfg(loom)]``-only blocks are ignored;
- a production invocation that cannot be parsed (non-literal kind or format)
  is a hard error, so a site can never silently disappear from the inventory;
- forwarding macros whose format literal has no static prefix (for example
  ``"{}"``) are not assertion sites and are skipped.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_SOURCE_ROOT = REPO_ROOT / "src"
DEFAULT_BASELINE = Path(__file__).with_name("violation-site-baseline.tsv")
ID_HEX_LENGTH = 16
_MACRO = "report_violation!"
_CFG_SKIP_ATOMS = ("test", "kani", "loom")
_KIND_RE = re.compile(r"ViolationKind::([A-Za-z_][A-Za-z0-9_]*)")
_CFG_TOKEN_RE = re.compile(
    r'\s*(?:(?P<string>"(?:[^"\\]|\\.)*")|(?P<ident>[A-Za-z_][A-Za-z0-9_]*)'
    r"|(?P<punct>[(),=]))"
)


@dataclass(frozen=True, order=True)
class Site:
    """One production assertion, identified by content rather than location."""

    site_id: str
    kind: str
    message: str


@dataclass(frozen=True, order=True)
class SiteLocation:
    """One source location that can emit a baseline assertion ID."""

    path: str
    line: int
    site_id: str


@dataclass(frozen=True)
class ScanIssue:
    """A production invocation whose identity cannot be derived safely."""

    path: Path
    line: int
    detail: str


def stable_site_id(kind: str, message: str) -> str:
    """Return the line-independent ID for one assertion identity."""
    payload = f"{kind}\0{message}".encode("utf-8", errors="strict")
    return hashlib.sha256(payload).hexdigest()[:ID_HEX_LENGTH]


def _copy_string(text: str, start: int) -> tuple[int, bool]:
    """Return the end offset of the string or char literal at ``start``."""
    quote = text[start]
    index = start + 1
    length = len(text)
    if quote == '"':
        while index < length:
            char = text[index]
            if char == "\\":
                index += 2
            elif char == '"':
                return index + 1, True
            else:
                index += 1
        return length, False
    # Char literal or lifetime. A closing quote within two characters means a
    # char literal; anything else is a lifetime and needs no special handling.
    if index < length and text[index] == "\\":
        index += 2
    else:
        index += 1
    if index < length and text[index] == "'":
        return index + 1, True
    return start + 1, False


def _raw_string_end(text: str, start: int) -> int | None:
    """Return the end offset of a Rust raw string starting at ``start``."""
    if text[start] != "r":
        return None
    index = start + 1
    hashes = 0
    while index < len(text) and text[index] == "#":
        hashes += 1
        index += 1
    if index >= len(text) or text[index] != '"':
        return None
    closing = '"' + "#" * hashes
    end = text.find(closing, index + 1)
    return len(text) if end < 0 else end + len(closing)


def strip_comments(text: str) -> str:
    """Blank comments while preserving byte offsets, newlines, and literals."""
    output: list[str] = []
    index = 0
    length = len(text)
    while index < length:
        char = text[index]
        if char == "/" and index + 1 < length and text[index + 1] == "/":
            end = text.find("\n", index)
            if end < 0:
                end = length
            output.append(" " * (end - index))
            index = end
        elif char == "/" and index + 1 < length and text[index + 1] == "*":
            depth = 1
            end = index + 2
            while end < length and depth > 0:
                if text.startswith("/*", end):
                    depth += 1
                    end += 2
                elif text.startswith("*/", end):
                    depth -= 1
                    end += 2
                else:
                    end += 1
            segment = text[index:end]
            output.append("".join("\n" if item == "\n" else " " for item in segment))
            index = end
        elif char == "r":
            raw_end = _raw_string_end(text, index)
            if raw_end is None:
                output.append(char)
                index += 1
            else:
                output.append(text[index:raw_end])
                index = raw_end
        elif char in {'"', "'"}:
            literal_end, closed = _copy_string(text, index)
            if closed:
                output.append(text[index:literal_end])
                index = literal_end
            else:
                output.append(char)
                index += 1
        else:
            output.append(char)
            index += 1
    return "".join(output)


def _literal_spans(text: str) -> list[tuple[int, int]]:
    """Return spans covered by string and char literals."""
    spans: list[tuple[int, int]] = []
    index = 0
    length = len(text)
    while index < length:
        char = text[index]
        if char == "r":
            raw_end = _raw_string_end(text, index)
            if raw_end is not None:
                spans.append((index, raw_end))
                index = raw_end
                continue
        if char in {'"', "'"}:
            literal_end, closed = _copy_string(text, index)
            if closed:
                spans.append((index, literal_end))
                index = literal_end
                continue
        index += 1
    return spans


def _parse_balanced(
    text: str, start: int, opening: str, closing: str
) -> tuple[str, int] | None:
    """Parse one balanced delimiter pair, respecting string literals."""
    if start >= len(text) or text[start] != opening:
        return None
    depth = 0
    index = start
    while index < len(text):
        char = text[index]
        if char == "r":
            raw_end = _raw_string_end(text, index)
            if raw_end is not None:
                index = raw_end
                continue
        if char in {'"', "'"}:
            literal_end, closed = _copy_string(text, index)
            if closed:
                index = literal_end
                continue
        if char == opening:
            depth += 1
        elif char == closing:
            depth -= 1
            if depth == 0:
                return text[start + 1 : index], index + 1
        index += 1
    return None


def _split_arguments(arguments: str) -> list[str]:
    """Split macro arguments on top-level commas."""
    parts: list[str] = []
    start = 0
    index = 0
    depth = 0
    while index < len(arguments):
        char = arguments[index]
        if char == "r":
            raw_end = _raw_string_end(arguments, index)
            if raw_end is not None:
                index = raw_end
                continue
        if char in {'"', "'"}:
            literal_end, closed = _copy_string(arguments, index)
            if closed:
                index = literal_end
                continue
        if char in "([{":
            depth += 1
        elif char in ")]}":
            depth -= 1
        elif char == "," and depth == 0:
            parts.append(arguments[start:index])
            start = index + 1
        index += 1
    parts.append(arguments[start:])
    return [part.strip() for part in parts]


def _cfg_predicate(attribute: str) -> str | None:
    """Return the predicate of a ``cfg(...)`` attribute body."""
    body = attribute.strip()
    if body.startswith("["):
        body = body[1:].lstrip()
    if body.startswith("!"):
        body = body[1:].lstrip()
    if not body.startswith("cfg("):
        return None
    parsed = _parse_balanced(body, len("cfg"), "(", ")")
    return None if parsed is None else parsed[0]


class _CfgNode:
    """Tiny cfg-predicate expression tree used for required-atom analysis."""

    def __init__(self, operator: str, children: Iterable[_CfgNode] = (), atom: str = "") -> None:
        self.operator = operator
        self.children = tuple(children)
        self.atom = atom

    def atoms(self) -> set[str]:
        if self.operator == "atom":
            return {self.atom}
        result: set[str] = set()
        for child in self.children:
            result.update(child.atoms())
        return result

    def evaluate(self, values: dict[str, bool]) -> bool:
        if self.operator == "atom":
            return values.get(self.atom, False)
        if self.operator == "not":
            return not self.children[0].evaluate(values)
        if self.operator == "all":
            return all(child.evaluate(values) for child in self.children)
        return any(child.evaluate(values) for child in self.children)


class _CfgParser:
    """Recursive-descent parser for the cfg predicate subset Rust accepts."""

    def __init__(self, text: str) -> None:
        self.tokens = self._tokenize(text)
        self.position = 0

    @staticmethod
    def _tokenize(text: str) -> list[str]:
        tokens: list[str] = []
        index = 0
        while index < len(text):
            match = _CFG_TOKEN_RE.match(text, index)
            if match is None:
                raise ValueError(f"invalid cfg predicate near {text[index:]!r}")
            token = match.group("string") or match.group("ident") or match.group("punct")
            if token is None:
                raise ValueError(f"invalid cfg predicate near {text[index:]!r}")
            tokens.append(token)
            index = match.end()
        return tokens

    def _peek(self) -> str | None:
        return self.tokens[self.position] if self.position < len(self.tokens) else None

    def _take(self, expected: str | None = None) -> str:
        token = self._peek()
        if token is None or (expected is not None and token != expected):
            raise ValueError(f"expected cfg token {expected!r}, found {token!r}")
        self.position += 1
        return token

    def parse(self) -> _CfgNode:
        node = self._expression()
        if self.position != len(self.tokens):
            raise ValueError(f"trailing cfg tokens: {self.tokens[self.position:]!r}")
        return node

    def _expression(self) -> _CfgNode:
        cfg_item = self._take()
        if cfg_item in {"all", "any", "not"}:
            self._take("(")
            children = [self._expression()]
            while self._peek() == ",":
                self._take(",")
                children.append(self._expression())
            self._take(")")
            return _CfgNode(cfg_item, children)
        if cfg_item == "(":
            node = self._expression()
            self._take(")")
            return node
        if cfg_item in {"true", "false"}:
            return _CfgNode("atom", atom=cfg_item)
        if self._peek() == "=":
            self._take("=")
            value = self._take()
            return _CfgNode("atom", atom=f"{cfg_item}={value}")
        return _CfgNode("atom", atom=cfg_item)


def _requires_atom(predicate: str, atom: str) -> bool:
    """Return true when the predicate is false under every assignment with
    ``atom = False`` (that is, the predicate requires the atom)."""
    try:
        node = _CfgParser(predicate).parse()
    except ValueError:
        return False
    atoms = node.atoms()
    if atom not in atoms:
        return False
    others = sorted(item for item in atoms if item != atom)
    if len(others) > 12:
        return False
    for mask in range(1 << len(others)):
        values = {atom: False}
        values.update(
            {name: bool(mask & (1 << index)) for index, name in enumerate(others)}
        )
        if node.evaluate(values):
            return False
    return True


def _next_item_marker(text: str, start: int) -> tuple[str, int] | None:
    """Find the next block body or item terminator outside literals."""
    index = start
    while index < len(text):
        char = text[index]
        if char == "r":
            raw_end = _raw_string_end(text, index)
            if raw_end is not None:
                index = raw_end
                continue
        if char in {'"', "'"}:
            literal_end, closed = _copy_string(text, index)
            if closed:
                index = literal_end
                continue
        if char == "{":
            return "block", index
        if char == ";":
            return "semicolon", index
        index += 1
    return None


def _attribute_ranges(text: str) -> list[tuple[int, int]]:
    """Return byte ranges for blocks gated on test, kani, or loom only."""
    ranges: list[tuple[int, int]] = []
    index = 0
    while True:
        start = text.find("#[", index)
        if start < 0:
            break
        parsed = _parse_balanced(text, start + 1, "[", "]")
        if parsed is None:
            index = start + 2
            continue
        attribute, end = parsed
        index = end
        predicate = _cfg_predicate(attribute)
        if predicate is None:
            continue
        if not any(_requires_atom(predicate, atom) for atom in _CFG_SKIP_ATOMS):
            continue
        marker = _next_item_marker(text, end)
        if marker is None or marker[0] != "block":
            continue
        body = _parse_balanced(text, marker[1], "{", "}")
        if body is None:
            continue
        ranges.append((start, body[1]))
    return ranges


def _inside(position: int, ranges: Iterable[tuple[int, int]]) -> bool:
    return any(start <= position < end for start, end in ranges)


def _strip_outer_quotes(literal: str) -> str:
    """Return the source content of a normal or raw Rust string literal."""
    if literal.startswith("r"):
        hashes = 0
        index = 1
        while index < len(literal) and literal[index] == "#":
            hashes += 1
            index += 1
        if index < len(literal) and literal[index] == '"':
            end = len(literal) - 1 - hashes
            return literal[index + 1 : end]
    if len(literal) >= 2 and literal.startswith('"') and literal.endswith('"'):
        return literal[1:-1]
    return literal


def _has_static_prefix(literal: str) -> bool:
    """Return false for forwarding-only formats such as ``"{}"``."""
    content = _strip_outer_quotes(literal)
    prefix = content.split("{", 1)[0]
    return any(char.isalnum() for char in prefix)


def _is_string_literal(expression: str) -> bool:
    stripped = expression.strip()
    if _raw_string_end(stripped, 0) == len(stripped):
        return True
    if not stripped.startswith('"'):
        return False
    end, closed = _copy_string(stripped, 0)
    return closed and end == len(stripped)


def scan_text(text: str, path: Path) -> tuple[list[Site], list[ScanIssue]]:
    """Extract production assertion sites from one Rust source file."""
    entries, issues = _scan_entries(text, path)
    return [site for site, _ in entries], issues


def _scan_entries(
    text: str, path: Path
) -> tuple[list[tuple[Site, int]], list[ScanIssue]]:
    """Extract assertion sites and their invocation lines from one source file."""
    stripped = strip_comments(text)
    skip_ranges = _attribute_ranges(stripped)
    literal_spans = _literal_spans(stripped)
    entries: list[tuple[Site, int]] = []
    issues: list[ScanIssue] = []
    index = 0
    while True:
        start = stripped.find(_MACRO, index)
        if start < 0:
            break
        index = start + len(_MACRO)
        if _inside(start, skip_ranges) or _inside(start, literal_spans):
            continue
        cursor = index
        while cursor < len(stripped) and stripped[cursor].isspace():
            cursor += 1
        parsed = _parse_balanced(stripped, cursor, "(", ")")
        if parsed is None:
            issues.append(ScanIssue(path, stripped.count("\n", 0, start) + 1, "unbalanced macro"))
            continue
        arguments, end = parsed
        parts = _split_arguments(arguments)
        if len(parts) < 3:
            issues.append(
                ScanIssue(path, stripped.count("\n", 0, start) + 1, "fewer than three arguments")
            )
            continue
        kind_expression = parts[1].strip()
        kind_match = _KIND_RE.search(kind_expression)
        if kind_match is None or kind_match.end() != len(kind_expression):
            issues.append(
                ScanIssue(
                    path,
                    stripped.count("\n", 0, start) + 1,
                    f"non-literal kind expression {parts[1].strip()!r}",
                )
            )
            continue
        message = parts[2].strip()
        if not _is_string_literal(message):
            issues.append(
                ScanIssue(
                    path,
                    stripped.count("\n", 0, start) + 1,
                    f"non-literal format expression {message!r}",
                )
            )
            continue
        if not _has_static_prefix(message):
            continue
        kind = kind_match.group(1)
        line = stripped.count("\n", 0, start) + 1
        entries.append((Site(stable_site_id(kind, message), kind, message), line))
        index = end
    return entries, issues


def inventory_locations(
    source_root: Path, repo_root: Path = REPO_ROOT
) -> tuple[list[Site], list[SiteLocation], list[ScanIssue]]:
    """Return the current inventory plus one entry per emitted source location.

    Paths are rendered exactly as ``file!()`` does at the macro call site so
    runtime telemetry locations can be joined back to baseline IDs without
    trusting line-number stability across edits.
    """
    sites: dict[str, Site] = {}
    locations: list[SiteLocation] = []
    issues: list[ScanIssue] = []
    for path in sorted(source_root.rglob("*.rs")):
        entries, file_issues = _scan_entries(path.read_text(encoding="utf-8"), path)
        relative = path.relative_to(repo_root).as_posix()
        for site, line in entries:
            existing = sites.get(site.site_id)
            if existing is not None and existing != site:
                issues.append(
                    ScanIssue(
                        path,
                        line,
                        f"site ID collision with {existing.kind} {existing.message}",
                    )
                )
                continue
            sites[site.site_id] = site
            locations.append(SiteLocation(relative, line, site.site_id))
        issues.extend(file_issues)
    return sorted(sites.values()), sorted(locations), issues


def inventory(source_root: Path) -> tuple[list[Site], list[ScanIssue]]:
    """Scan every Rust source file below ``source_root``."""
    sites: dict[str, Site] = {}
    issues: list[ScanIssue] = []
    for path in sorted(source_root.rglob("*.rs")):
        file_sites, file_issues = scan_text(path.read_text(encoding="utf-8"), path)
        for site in file_sites:
            existing = sites.get(site.site_id)
            if existing is not None and existing != site:
                issues.append(
                    ScanIssue(
                        path,
                        1,
                        f"site ID collision with {existing.kind} {existing.message}",
                    )
                )
                continue
            sites[site.site_id] = site
        issues.extend(file_issues)
    return sorted(sites.values()), issues


def render_baseline(sites: Iterable[Site]) -> str:
    """Render a deterministic baseline document.

    The message column is a JSON string so multi-line Rust literals stay on one
    TSV row without losing their exact source bytes.
    """
    lines = ["site_id\tkind\tmessage_json"]
    for site in sorted(sites):
        lines.append(f"{site.site_id}\t{site.kind}\t{json.dumps(site.message)}")
    return "\n".join(lines) + "\n"


def parse_baseline(text: str) -> list[Site]:
    """Parse a baseline document, rejecting malformed rows and duplicates."""
    sites: list[Site] = []
    seen: set[str] = set()
    for line_number, line in enumerate(text.splitlines(), start=1):
        if not line or line.startswith("#"):
            continue
        columns = line.split("\t")
        if columns == ["site_id", "kind", "message_json"]:
            continue
        if len(columns) != 3:
            raise ValueError(f"baseline line {line_number} does not have three columns")
        site_id, kind, message_json = columns
        if not re.fullmatch(r"[0-9a-f]{16}", site_id):
            raise ValueError(f"baseline line {line_number} has an invalid site ID")
        try:
            message = json.loads(message_json)
        except json.JSONDecodeError as error:
            raise ValueError(
                f"baseline line {line_number} has invalid JSON: {error}"
            ) from error
        if not isinstance(message, str):
            raise ValueError(f"baseline line {line_number} message is not a string")
        if site_id in seen:
            raise ValueError(f"baseline line {line_number} duplicates {site_id}")
        seen.add(site_id)
        sites.append(Site(site_id, kind, message))
    return sites


def print_issue(issue: ScanIssue) -> None:
    print(
        f"{issue.path.relative_to(REPO_ROOT)}:{issue.line}: {issue.detail}",
        file=sys.stderr,
    )


def _load_baseline(path: Path) -> list[Site]:
    if not path.exists():
        raise FileNotFoundError(f"violation-site baseline does not exist: {path}")
    return parse_baseline(path.read_text(encoding="utf-8"))


def check(source_root: Path, baseline_path: Path) -> int:
    """Compare the current inventory with the checked-in baseline."""
    try:
        baseline = _load_baseline(baseline_path)
    except (OSError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2
    sites, issues = inventory(source_root)
    if issues:
        for issue in issues:
            print_issue(issue)
        print(
            f"error: {len(issues)} production {_MACRO} invocation(s) cannot be inventoried",
            file=sys.stderr,
        )
        return 1
    baseline_by_id = {site.site_id: site for site in baseline}
    current_by_id = {site.site_id: site for site in sites}
    removed = sorted(set(baseline_by_id) - set(current_by_id))
    added = sorted(set(current_by_id) - set(baseline_by_id))
    for site_id in removed:
        site = baseline_by_id[site_id]
        print(
            f"removed violation site {site_id}: {site.kind} {site.message}",
            file=sys.stderr,
        )
    for site_id in added:
        site = current_by_id[site_id]
        print(
            f"new violation site {site_id}: {site.kind} {site.message}", file=sys.stderr
        )
    if removed or added:
        print(
            "error: violation-site baseline is stale; run "
            "`python3 scripts/simulation/violation_sites.py --write` "
            "and review the diff",
            file=sys.stderr,
        )
        return 1
    print(f"violation sites: {len(sites)} baseline entries match production source")
    return 0


def write(source_root: Path, baseline_path: Path) -> int:
    """Regenerate the checked-in baseline from current production source."""
    sites, issues = inventory(source_root)
    if issues:
        for issue in issues:
            print_issue(issue)
        print(
            f"error: {len(issues)} production {_MACRO} invocation(s) cannot be inventoried",
            file=sys.stderr,
        )
        return 1
    baseline_path.write_text(render_baseline(sites), encoding="utf-8")
    print(f"wrote {len(sites)} violation-site entries to {baseline_path}")
    return 0


def main(argv: list[str] | None = None) -> int:
    """Run the inventory check or regenerate the baseline."""
    source_root = DEFAULT_SOURCE_ROOT
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument(
        "--check", action="store_true", help="verify the checked-in baseline"
    )
    mode.add_argument("--write", action="store_true", help="regenerate the baseline")
    parser.add_argument("--source-root", type=Path, default=source_root)
    parser.add_argument("--baseline", type=Path, default=DEFAULT_BASELINE)
    arguments = parser.parse_args(argv)
    if arguments.check:
        return check(arguments.source_root, arguments.baseline)
    return write(arguments.source_root, arguments.baseline)


if __name__ == "__main__":
    sys.exit(main())
