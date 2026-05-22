#!/usr/bin/env python3
"""Reject advance_frame() calls that silently ignore every error."""

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


def _strip_line_comment(line: str) -> str:
    """Remove simple Rust line comments while preserving line numbering."""
    comment_index = line.find("//")
    if comment_index == -1:
        return line
    return line[:comment_index]


def _line_number(source: str, offset: int) -> int:
    return source.count("\n", 0, offset) + 1


def check_file(path: Path) -> list[str]:
    """Return diagnostics for swallowed advance_frame errors in *path*."""
    try:
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    except OSError as exc:
        return [f"{_display_path(path)}:0: cannot read file: {exc}"]

    source = "\n".join(_strip_line_comment(line) for line in lines)
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
