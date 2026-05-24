#!/usr/bin/env python3
"""Reject advance_frame() calls that silently ignore every error.

The scanner blanks comments, string literals, and char literals before applying
the regex patterns so examples in documentation comments, block comments, or
Rust literals do not fail pre-commit.
"""

from __future__ import annotations

import os
import re
import subprocess
import sys
from pathlib import Path


SKIP_DIRS = {"target", ".git", ".tla-tools"}
ADVANCE_FRAME_CALL = r"advance_frame\s*(?:::\s*<[^>]*>)?\s*\("

SWALLOWED_PATTERNS: tuple[tuple[re.Pattern[str], str], ...] = (
    (
        re.compile(
            rf"\b(?:if|while)\s+let\s+Ok(?:\s*<[^>]+>)?\s*\([^)]*\)\s*="
            rf"\s*[^;{{]*{ADVANCE_FRAME_CALL}",
            re.MULTILINE,
        ),
        "advance_frame() error is ignored by if/while let Ok(..)",
    ),
    (
        re.compile(
            rf"\blet\s+_\s*=\s*[^;?]*{ADVANCE_FRAME_CALL}\s*\)\s*;",
            re.MULTILINE,
        ),
        "advance_frame() result is discarded by let _",
    ),
    (
        re.compile(
            rf"{ADVANCE_FRAME_CALL}\s*\)\s*\.\s*(?:ok|is_ok)\s*\(",
            re.MULTILINE,
        ),
        "advance_frame() error detail is discarded by .ok()/.is_ok()",
    ),
)


def _display_path(filepath: str | Path) -> str:
    """Convert a file path to a repository-relative display path."""
    try:
        return str(Path(filepath).resolve().relative_to(Path.cwd().resolve()))
    except ValueError:
        return str(filepath)


def _rust_files_from_repo() -> list[Path]:
    """Return tracked Rust files for all-files/manual runs."""
    result = subprocess.run(
        ["git", "ls-files", "--", "*.rs"],
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode == 0:
        return [Path(line) for line in result.stdout.splitlines() if line.strip()]

    paths: list[Path] = []
    for dirpath, dirnames, filenames in os.walk(Path.cwd()):
        dirnames[:] = [dirname for dirname in dirnames if dirname not in SKIP_DIRS]
        for filename in filenames:
            if filename.endswith(".rs"):
                paths.append(Path(dirpath) / filename)
    return sorted(paths)


def _blank(ch: str) -> str:
    """Blank a source character while preserving newlines."""
    return "\n" if ch == "\n" else " "


def _raw_string_opener(text: str, offset: int) -> tuple[int, str] | None:
    """Return raw-string opener length and closer when `offset` starts one."""
    prefix_len = 0
    if text.startswith("r", offset):
        prefix_len = 1
    elif text.startswith(("br", "cr"), offset):
        prefix_len = 2
    else:
        return None

    cursor = offset + prefix_len
    while cursor < len(text) and text[cursor] == "#":
        cursor += 1
    if cursor >= len(text) or text[cursor] != '"':
        return None

    hashes = cursor - offset - prefix_len
    return cursor - offset + 1, '"' + ("#" * hashes)


def _char_literal_end(text: str, offset: int) -> int | None:
    """Return the exclusive end offset if `offset` starts a Rust char literal."""
    cursor = offset + 1
    if cursor >= len(text) or text[cursor] in "\r\n":
        return None

    if text[cursor] == "\\":
        cursor += 1
        if cursor >= len(text) or text[cursor] in "\r\n":
            return None
        if text[cursor] == "u" and cursor + 1 < len(text) and text[cursor + 1] == "{":
            cursor += 2
            while cursor < len(text) and text[cursor] not in "}\r\n":
                cursor += 1
            if cursor >= len(text) or text[cursor] != "}":
                return None
            cursor += 1
        else:
            cursor += 1
    else:
        cursor += 1

    if cursor < len(text) and text[cursor] == "'":
        return cursor + 1
    return None


def _blank_comments_and_strings(text: str) -> str:
    """Blank Rust comments and string literals while preserving offsets.

    Handles `//` comments, nested `/* ... */` comments, ordinary/byte/C string
    literals, raw/byte-raw/C-raw string literals, and ordinary/byte char
    literals. Non-newline characters in those spans become spaces so match
    offsets and diagnostic line numbers remain stable.
    """
    out: list[str] = []
    cursor = 0
    text_len = len(text)

    while cursor < text_len:
        ch = text[cursor]
        next_ch = text[cursor + 1] if cursor + 1 < text_len else ""

        if ch == "/" and next_ch == "/":
            while cursor < text_len and text[cursor] != "\n":
                out.append(" ")
                cursor += 1
            continue

        if ch == "/" and next_ch == "*":
            depth = 0
            while cursor < text_len:
                if text.startswith("/*", cursor):
                    out.extend((" ", " "))
                    cursor += 2
                    depth += 1
                    continue
                if text.startswith("*/", cursor):
                    out.extend((" ", " "))
                    cursor += 2
                    depth -= 1
                    if depth == 0:
                        break
                    continue
                out.append(_blank(text[cursor]))
                cursor += 1
            continue

        if ch == "b" and next_ch == "'":
            literal_end = _char_literal_end(text, cursor + 1)
            if literal_end is not None:
                while cursor < literal_end:
                    out.append(_blank(text[cursor]))
                    cursor += 1
                continue

        literal_end = _char_literal_end(text, cursor) if ch == "'" else None
        if literal_end is not None:
            while cursor < literal_end:
                out.append(_blank(text[cursor]))
                cursor += 1
            continue

        raw_opener = _raw_string_opener(text, cursor)
        if raw_opener is not None:
            opener_len, closer = raw_opener
            out.extend(" " for _ in range(opener_len))
            cursor += opener_len
            while cursor < text_len:
                if text.startswith(closer, cursor):
                    out.extend(" " for _ in range(len(closer)))
                    cursor += len(closer)
                    break
                out.append(_blank(text[cursor]))
                cursor += 1
            continue

        if ch in ("b", "c") and next_ch == '"':
            out.append(" ")
            cursor += 1
            ch = text[cursor]

        if ch == '"':
            out.append(" ")
            cursor += 1
            while cursor < text_len:
                if text[cursor] == "\\" and cursor + 1 < text_len:
                    out.append(_blank(text[cursor]))
                    out.append(_blank(text[cursor + 1]))
                    cursor += 2
                    continue
                if text[cursor] == '"':
                    out.append(" ")
                    cursor += 1
                    break
                out.append(_blank(text[cursor]))
                cursor += 1
            continue

        out.append(ch)
        cursor += 1

    return "".join(out)


def _line_number(source: str, offset: int) -> int:
    return source.count("\n", 0, offset) + 1


def check_file(path: Path) -> list[str]:
    """Return diagnostics for swallowed advance_frame errors in *path*."""
    try:
        raw_source = path.read_text(encoding="utf-8", errors="replace")
    except OSError as exc:
        return [f"{_display_path(path)}:0: cannot read file: {exc}"]

    source = _blank_comments_and_strings(raw_source)
    errors: list[str] = []
    for pattern, message in SWALLOWED_PATTERNS:
        for match in pattern.finditer(source):
            errors.append(
                f"{_display_path(path)}:{_line_number(source, match.start())}: "
                f"{message}; match the expected error explicitly or use ?"
            )

    return errors


def main() -> int:
    """Check Rust files passed by pre-commit, or all Rust files if none are given."""
    raw_paths = [Path(arg) for arg in sys.argv[1:]]
    paths = raw_paths if raw_paths else _rust_files_from_repo()

    errors: list[str] = []
    for path in paths:
        if path.suffix == ".rs":
            errors.extend(check_file(path))

    if errors:
        print("ERROR: advance_frame() errors must be handled explicitly:", file=sys.stderr)
        for error in errors:
            print(error, file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
