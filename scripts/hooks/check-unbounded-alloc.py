#!/usr/bin/env python3
"""Flag dynamically-sized allocations in production code lacking a justification.

Rust's default allocator ABORTS the process on allocation failure, so any
allocation whose size comes from an unbounded source (a length read from the
wire, an unvalidated config field, ...) is a denial-of-service / abort vector
(cf. RUSTSEC-2022-0035). This hook forces every dynamically-sized allocation in
production code to declare why its size is bounded via an `// alloc-bound: ...`
comment.

Scope:
- Scans only `src/**/*.rs`.
- Excludes `#[cfg(test)]` / `#[cfg(kani)]`-gated items STRUCTURALLY. An
  attribute whose cfg predicate mentions `test` or `kani` (e.g.
  `#[cfg(test)]`, `#[cfg(kani)]`, `#[cfg(all(test, feature = "x"))]`,
  `#[cfg(any(test, ...))]`, `#[cfg_attr(test, ...)]`) applies to the next
  item; that item's entire span is removed from analysis at ANY indentation.
  This means production code that appears AFTER an early test/proof item (for
  example a `#[cfg(test)] const X = 5;` near the top of a file) is still
  scanned -- only the gated item itself is skipped, not the rest of the file.

Detected constructs:
- `.with_capacity(` / `::with_capacity(`
- `.reserve(` / `::reserve(`
- `.reserve_exact(` / `::reserve_exact(`
- `.resize(` / `::resize(`            (size is the FIRST argument)
- `.resize_with(` / `::resize_with(`  (size is the FIRST argument)
- the `vec![ <expr> ; <size> ]` macro form (the size after the top-level `;`)

Fallible reservations are NOT detected (they return `Result` rather than
aborting): `try_reserve` / `try_reserve_exact` are intentionally excluded
because the `reserve` / `reserve_exact` regex is anchored so the `try_` prefix
does not match.

Exempt (no marker needed):
- a pure integer literal (e.g. `with_capacity(4)`, `vec![0u8; 16]`)
- a size expression that is exactly an `<ident>.len()` / `.len()` / `.count()`
  call (bounded by an existing in-memory collection)

Everything else (a bare identifier, an arithmetic expression, a field access, a
function call, ...) requires an `// alloc-bound: <why>` justification on the
same line OR the line immediately above.

Comment / string handling:
- `//` line comments are stripped (string-aware).
- `/* ... */` block comments are stripped, including multi-line spans.
- Raw strings (`r"..."`, `r#"..."#`, etc.) are stripped so an example
  containing `vec![x; n]` inside a raw string is not flagged.
- Ordinary (`"..."`) and byte (`b"..."`) string literal CONTENTS are blanked
  too, so a construct token inside a normal string (e.g.
  `let s = "vec![0u8; n]";`) is not flagged. Only exotic cases (e.g. a string
  containing an unbalanced quote produced via a char literal `'"'`) remain
  unmodeled.

Accepts a list of file paths as argv (pre-commit passes changed files) and,
when run with no args, scans all of `src/` (agent-preflight `--all` fallback).

Cross-platform: Works on Linux, macOS, and Windows.
"""
from __future__ import annotations

import re
import sys
from dataclasses import dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
SRC_DIR = REPO_ROOT / "src"

# An attribute line whose cfg predicate mentions `test` or `kani` as a token.
# Matches `#[cfg(...)]` and `#[cfg_attr(...)]` at any indentation, requiring a
# whole-word `test` or `kani` somewhere inside the predicate. Examples matched:
#   #[cfg(test)]
#   #[cfg(kani)]
#   #[cfg(all(test, feature = "x"))]
#   #[cfg(any(test, foo))]
#   #[cfg(all(feature = "y", kani))]
#   #[cfg_attr(test, derive(Debug))]
_CFG_TEST_KANI = re.compile(r"^\s*#\[\s*cfg(?:_attr)?\s*\(.*\b(?:test|kani)\b.*\)\s*\]")

# A line that is purely an attribute (e.g. `#[inline]`, `#[allow(...)]`) or a
# doc comment or blank -- skipped while looking for the item an attribute
# applies to.
_ATTRIBUTE_LINE = re.compile(r"^\s*#!?\[")
_DOC_OR_BLANK = re.compile(r"^\s*(?://.*|/\*.*|\*.*)?$")

# The justification marker that exempts a flagged construct.
_ALLOC_BOUND_MARKER = "// alloc-bound:"
_WEAK_ALLOC_BOUND_PHRASES = (
    "no numeric cap",
    "trusted local",
    "trusted/local",
    "trusted config",
    "unvalidated",
    "not validated",
)

# Capacity-reserving / resizing calls (preceded by `.` or `::`). `reserve` and
# `reserve_exact` are anchored on a non-word boundary so `try_reserve` /
# `try_reserve_exact` (fallible, non-aborting) are NOT matched: the `(?:\.|::)`
# prefix means a method name like `try_reserve` is seen as `.try_reserve`,
# whose match attempt for `reserve` would require a `.`/`:` immediately before
# `reserve`, which is absent.
_CAPACITY_CALL = re.compile(
    r"(?:\.|::)(with_capacity|reserve_exact|reserve|resize_with|resize)\s*\("
)

# A size expression that is a pure integer literal, e.g. `4`, `16`, `0u8`,
# `1_024`, `0usize`. Underscores and an optional integer-type suffix allowed.
_INT_LITERAL = re.compile(
    r"^[0-9][0-9_]*(?:u8|u16|u32|u64|u128|usize|i8|i16|i32|i64|i128|isize)?$"
)

# A size expression that is exactly a `.len()` or `.count()` call on something,
# e.g. `foo.len()`, `self.hosts.len()`, `buf.iter().count()`. Bounded by an
# existing in-memory collection, so exempt -- but ONLY when it is a single
# receiver chain with NO top-level binary operator (see `_size_expr_is_exempt`),
# so an arithmetic expression like `wire_len + buf.len()` still needs a marker.
_LEN_COUNT_CALL = re.compile(r"\.(?:len|count)\s*\(\s*\)$")


def _has_top_level_binary_operator(expr: str) -> bool:
    """Return True if `expr` contains a `+ - * / %` at bracket/paren depth 0.

    Tracks `()[]{}` nesting; an operator inside any bracket (a method argument,
    an index, a turbofish, ...) is NOT top-level. A `.len()`/`.count()` chain
    never begins with a unary minus, so any depth-0 operator here is binary.
    """
    depth = 0
    for ch in expr:
        if ch in "([{":
            depth += 1
        elif ch in ")]}":
            depth -= 1
        elif depth == 0 and ch in "+-*/%":
            return True
    return False


@dataclass(frozen=True)
class _Finding:
    """A flagged allocation site."""

    line: int  # 1-indexed line where the construct begins
    construct: str  # human-readable construct name


def _display_path(path: Path) -> str:
    """Return a repository-relative POSIX display path."""
    try:
        return path.resolve().relative_to(REPO_ROOT).as_posix()
    except ValueError:
        return path.as_posix()


def _strip_comments_and_raw_strings(text: str) -> str:
    """Blank out comments and string-literal contents in `text`.

    Blanks `//` line comments, `/* ... */` block comments (including multi-line
    spans), raw strings (`r"..."`, `r#"..."#`, `br"..."`), and the CONTENTS of
    ordinary (`"..."`) and byte (`b"..."`) string literals. Operates on the
    whole-file string so multi-line constructs are handled. Replacement
    preserves newlines (so line numbers are stable) and replaces other
    characters with spaces (so no construct token survives inside them); for
    ordinary/byte strings the opening and closing quotes are kept so the literal
    boundaries are not misread.
    """
    out: list[str] = []
    i = 0
    n = len(text)

    def blank(ch: str) -> str:
        return "\n" if ch == "\n" else " "

    while i < n:
        ch = text[i]
        nxt = text[i + 1] if i + 1 < n else ""

        # Line comment: blank to end of line.
        if ch == "/" and nxt == "/":
            while i < n and text[i] != "\n":
                out.append(" ")
                i += 1
            continue

        # Block comment: blank (preserving newlines) until the closing `*/`.
        if ch == "/" and nxt == "*":
            out.append(" ")
            out.append(" ")
            i += 2
            while i < n and not (text[i] == "*" and i + 1 < n and text[i + 1] == "/"):
                out.append(blank(text[i]))
                i += 1
            if i < n:
                out.append(" ")
                out.append(" ")
                i += 2
            continue

        # Raw string: r"...", r#"..."#, r##"..."##, etc. (also br"...").
        if ch in "rb":
            j = i
            if text[j] == "b" and j + 1 < n and text[j + 1] == "r":
                j += 1
            if text[j] == "r":
                k = j + 1
                hashes = 0
                while k < n and text[k] == "#":
                    hashes += 1
                    k += 1
                if k < n and text[k] == '"':
                    # Confirmed raw-string opener. Blank the prefix.
                    for _ in range(i, k + 1):
                        out.append(" ")
                    i = k + 1
                    closer = '"' + ("#" * hashes)
                    while i < n and text[i : i + len(closer)] != closer:
                        out.append(blank(text[i]))
                        i += 1
                    for _ in range(len(closer)):
                        if i < n:
                            out.append(" ")
                            i += 1
                    continue

        # Byte-string prefix `b"..."` (not `br"..."`, handled above): blank the
        # `b` and fall through to ordinary-string handling for the `"`.
        if ch == "b" and nxt == '"':
            out.append(" ")
            i += 1
            ch = '"'
            nxt = text[i + 1] if i + 1 < n else ""

        # Ordinary string literal: blank out its CONTENTS (preserving the quotes
        # and any embedded newlines so offsets / line counts are unchanged) so
        # construct tokens inside a normal string are not flagged. Handle `\"`
        # escapes so an escaped quote does not prematurely close the literal.
        if ch == '"':
            out.append(ch)  # keep the opening quote
            i += 1
            while i < n:
                if text[i] == "\\" and i + 1 < n:
                    out.append(blank(text[i]))
                    out.append(blank(text[i + 1]))
                    i += 2
                    continue
                if text[i] == '"':
                    out.append(text[i])  # keep the closing quote
                    i += 1
                    break
                out.append(blank(text[i]))
                i += 1
            continue

        out.append(ch)
        i += 1

    return "".join(out)


def _size_expr_is_exempt(expr: str) -> bool:
    """Return True if a size expression needs no justification."""
    expr = expr.strip()
    if not expr:
        # Defensive: an empty/unparsed size is treated as needing a marker.
        return False
    if _INT_LITERAL.match(expr):
        return True
    # A single receiver chain ending in `.len()`/`.count()` is exempt, but only
    # when there is no top-level binary operator: `foo.len()` is bounded, while
    # `wire_len + buf.len()` is arithmetic over a possibly-unbounded term and
    # must carry a marker.
    if _LEN_COUNT_CALL.search(expr) and not _has_top_level_binary_operator(expr):
        return True
    return False


def _marker_text(lines: list[str], line_index: int) -> str | None:
    """Return the `// alloc-bound:` marker on this or the prior line."""
    if 0 <= line_index < len(lines) and _ALLOC_BOUND_MARKER in lines[line_index]:
        return lines[line_index]
    if line_index > 0 and _ALLOC_BOUND_MARKER in lines[line_index - 1]:
        return lines[line_index - 1]
    return None


def _weak_marker_reason(marker: str) -> str | None:
    """Return a diagnostic reason if an alloc-bound marker is too weak."""
    lowered = marker.lower()
    for phrase in _WEAK_ALLOC_BOUND_PHRASES:
        if phrase in lowered:
            return f"marker says '{phrase}' instead of naming a concrete cap or fallible bound"
    return None


def _match_argument(text: str, open_paren: int) -> tuple[str, int] | None:
    """Extract the balanced argument starting just after `open_paren`.

    Returns the argument text and the index just past the matching `)`, or None
    if the parentheses are unbalanced within `text`.
    """
    depth = 0
    i = open_paren
    start = open_paren + 1
    while i < len(text):
        ch = text[i]
        if ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
            if depth == 0:
                return text[start:i], i + 1
        i += 1
    return None


def _first_argument(args: str) -> str:
    """Return the first top-level comma-separated argument from a call body."""
    depth = 0
    for i, ch in enumerate(args):
        if ch in "([{":
            depth += 1
        elif ch in ")]}":
            depth -= 1
        elif ch == "," and depth == 0:
            return args[:i]
    return args


def _top_level_size_after_semicolon(inner: str) -> str | None:
    """Return the size expression after the top-level `;` in a `vec!` body.

    `inner` is the text between `vec![` and its matching `]`. Returns None when
    there is no top-level `;` (a `vec![a, b, c]` list form, which is exempt).
    """
    depth = 0
    for i, ch in enumerate(inner):
        if ch in "([{":
            depth += 1
        elif ch in ")]}":
            depth -= 1
        elif ch == ";" and depth == 0:
            return inner[i + 1 :]
    return None


def _scan_vec_macros(joined: str, line_starts: list[int]) -> list[_Finding]:
    """Find `vec![<expr>; <size>]` constructs with non-exempt sizes."""
    findings: list[_Finding] = []
    for match in re.finditer(r"vec!\[", joined):
        open_bracket = match.end() - 1  # index of '['
        depth = 0
        i = open_bracket
        end = None
        while i < len(joined):
            ch = joined[i]
            if ch in "([{":
                depth += 1
            elif ch in ")]}":
                depth -= 1
                if depth == 0:
                    end = i
                    break
            i += 1
        if end is None:
            continue
        inner = joined[open_bracket + 1 : end]
        size_expr = _top_level_size_after_semicolon(inner)
        if size_expr is None:
            # List form `vec![a, b, c]` -- not a sized allocation.
            continue
        if _size_expr_is_exempt(size_expr):
            continue
        line_no = _offset_to_line(match.start(), line_starts)
        findings.append(_Finding(line=line_no, construct="vec![_; size]"))
    return findings


def _scan_capacity_calls(joined: str, line_starts: list[int]) -> list[_Finding]:
    """Find with_capacity/reserve/reserve_exact/resize/resize_with calls."""
    findings: list[_Finding] = []
    for match in _CAPACITY_CALL.finditer(joined):
        method = match.group(1)
        open_paren = match.end() - 1  # index of '('
        extracted = _match_argument(joined, open_paren)
        if extracted is None:
            continue
        arg, _ = extracted
        # For resize/resize_with the size is the FIRST argument; the others
        # (fill value / closure) are irrelevant to the allocation size.
        size_arg = _first_argument(arg) if method in ("resize", "resize_with") else arg
        if _size_expr_is_exempt(size_arg):
            continue
        line_no = _offset_to_line(match.start(), line_starts)
        findings.append(_Finding(line=line_no, construct=f"{method}()"))
    return findings


def _offset_to_line(offset: int, line_starts: list[int]) -> int:
    """Map a character offset in the joined source to a 1-indexed line number."""
    # line_starts[k] is the offset of the start of line k (0-indexed). Find the
    # greatest k with line_starts[k] <= offset.
    lo, hi = 0, len(line_starts) - 1
    while lo < hi:
        mid = (lo + hi + 1) // 2
        if line_starts[mid] <= offset:
            lo = mid
        else:
            hi = mid - 1
    return lo + 1


def _balanced_block_end(lines: list[str], start: int) -> int:
    """Return the index of the last line of a brace-balanced item.

    Scans from line `start` (which is expected to contain the item's opening
    `{`). Tracks brace depth, ignoring braces inside `//`/`/* */` comments,
    ordinary strings, and raw strings (via `_strip_comments_and_raw_strings`).
    Returns the index of the line containing the matching closing `}`. If no
    open brace is found before a `;` at depth 0, returns the index of that `;`
    line (a statement item). Falls back to `start` if nothing matches.
    """
    depth = 0
    seen_open = False
    for idx in range(start, len(lines)):
        cleaned = _strip_comments_and_raw_strings(lines[idx])
        for ch in cleaned:
            if ch == "{":
                depth += 1
                seen_open = True
            elif ch == "}":
                depth -= 1
                if seen_open and depth == 0:
                    return idx
            elif ch == ";" and depth == 0 and not seen_open:
                return idx
    return start


def _strip_leading_attributes(text: str) -> str:
    """Strip leading whitespace and ALL leading `#[ ... ]` attributes from text.

    Bracket-depth-aware so nested attributes (e.g.
    `#[cfg(all(test, feature = "x"))]`) and stacked same-line attributes (e.g.
    `#[cfg(test)] #[allow(dead_code)] fn f() {}`) are consumed. Returns the
    remaining (item-content) text, stripped of surrounding whitespace; an empty
    result means the line held only attributes (the item is on a later line).
    """
    s = text.lstrip()
    while s.startswith("#"):
        # Accept outer `#[ ... ]` and inner `#![ ... ]` attributes.
        rest = s[1:]
        if rest.startswith("!"):
            rest = rest[1:]
        if not rest.lstrip().startswith("["):
            break
        # Advance past the leading `#`/`#!` to the opening `[`, then consume the
        # bracket-balanced attribute body.
        idx = s.index("[", 1)
        depth = 0
        closed = False
        while idx < len(s):
            ch = s[idx]
            if ch == "[":
                depth += 1
            elif ch == "]":
                depth -= 1
                if depth == 0:
                    idx += 1
                    closed = True
                    break
            idx += 1
        if not closed:
            # Unbalanced attribute (continues on a later line): no same-line
            # item content remains.
            return ""
        s = s[idx:].lstrip()
    return s


def _compute_excluded_lines(lines: list[str]) -> set[int]:
    """Return the set of 0-indexed line numbers belonging to test/kani items.

    For each `#[cfg(test/kani)]`-style attribute, the attribute lines, any
    following attribute / doc / blank lines, and the entire span of the item
    they apply to are excluded. The item may begin on the SAME line as the
    attribute (e.g. `#[cfg(test)] const X: usize = 5;`), in which case only that
    item's span is excluded and production code on later lines is still scanned.
    """
    excluded: set[int] = set()
    i = 0
    n = len(lines)
    while i < n:
        if i in excluded:
            i += 1
            continue
        if _CFG_TEST_KANI.match(lines[i]):
            block_start = i
            # Does the gated item begin ON line `i` (idiomatic same-line form,
            # e.g. `#[cfg(test)] const X = 5;` or `#[cfg(test)] mod tests {`)?
            # Strip the (comment-stripped) line's leading attribute(s); a
            # non-empty remainder means item content follows on this line.
            cleaned_attr_line = _strip_comments_and_raw_strings(lines[i])
            if _strip_leading_attributes(cleaned_attr_line):
                item_start = i
            else:
                # Walk forward over attribute lines / doc comments / blanks to
                # reach the item the cfg attribute applies to (next-line form).
                j = i + 1
                while j < n and (
                    _ATTRIBUTE_LINE.match(lines[j]) or _DOC_OR_BLANK.match(lines[j])
                ):
                    j += 1
                if j >= n:
                    # Dangling attribute at EOF; exclude what we have.
                    for k in range(block_start, n):
                        excluded.add(k)
                    break
                item_start = j
            item_end = _balanced_block_end(lines, item_start)
            for k in range(block_start, item_end + 1):
                excluded.add(k)
            i = item_end + 1
            continue
        i += 1
    return excluded


def check_file(path: Path) -> list[str]:
    """Return diagnostics for unjustified allocations in `path`."""
    display_path = _display_path(path)
    try:
        raw_lines = path.read_text(encoding="utf-8").splitlines()
    except (OSError, UnicodeDecodeError) as exc:
        return [f"{display_path}:0: cannot read file: {exc}"]

    # Structurally exclude cfg(test)/cfg(kani)-gated item spans (any indent).
    excluded = _compute_excluded_lines(raw_lines)

    # Strip comments (line + block) and raw strings file-wide so constructs
    # inside `//`/`/* */`/`r"..."` are not treated as code. Then blank any
    # excluded (test/kani) line entirely so its allocations are not scanned.
    cleaned_text = _strip_comments_and_raw_strings("\n".join(raw_lines))
    cleaned_lines = cleaned_text.split("\n")
    # `splitlines()` and `split("\n")` agree on count for the joined text.
    stripped = [
        "" if idx in excluded else line for idx, line in enumerate(cleaned_lines)
    ]
    joined = "\n".join(stripped)

    # Precompute the start offset of each line within `joined`.
    line_starts: list[int] = []
    offset = 0
    for line in stripped:
        line_starts.append(offset)
        offset += len(line) + 1  # +1 for the '\n' separator

    findings = _scan_vec_macros(joined, line_starts)
    findings.extend(_scan_capacity_calls(joined, line_starts))

    errors: list[str] = []
    for finding in sorted(findings, key=lambda f: (f.line, f.construct)):
        # `_marker_text` reads the ORIGINAL lines so a same-line trailing
        # comment still counts (it was stripped from `joined`).
        marker = _marker_text(raw_lines, finding.line - 1)
        if marker is not None:
            if weak_reason := _weak_marker_reason(marker):
                errors.append(
                    f"{display_path}:{finding.line}: {finding.construct} has a weak "
                    f"'// alloc-bound:' justification: {weak_reason}"
                )
            continue
        errors.append(
            f"{display_path}:{finding.line}: {finding.construct} requires an "
            f"'// alloc-bound: <why>' justification (size is not a literal "
            f"or .len())"
        )
    return errors


def _iter_src_rust_files() -> list[Path]:
    """Return all Rust files under `src/`."""
    return sorted(SRC_DIR.rglob("*.rs"))


def _select_paths(argv: list[str]) -> list[Path]:
    """Select the files to scan from argv, restricted to `src/**/*.rs`."""
    if not argv:
        return _iter_src_rust_files()

    selected: list[Path] = []
    for arg in argv:
        path = Path(arg)
        if path.suffix != ".rs":
            continue
        try:
            resolved = path.resolve()
        except OSError:
            continue
        # Only scan files under src/.
        try:
            resolved.relative_to(SRC_DIR)
        except ValueError:
            continue
        selected.append(path)
    return selected


def main() -> int:
    """Check selected Rust files for unjustified dynamic allocations."""
    paths = _select_paths(sys.argv[1:])

    errors: list[str] = []
    for path in paths:
        errors.extend(check_file(path))

    if errors:
        print(
            "ERROR: dynamically-sized allocations must declare an "
            "'// alloc-bound:' justification:",
            file=sys.stderr,
        )
        for error in errors:
            print(error, file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
