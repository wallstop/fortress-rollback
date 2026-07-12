#!/usr/bin/env python3
"""Prevent released wire-golden rewrites without a protocol-version bump."""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

_VERSION_RE = re.compile(r"^pub const PROTOCOL_VERSION: u8 = (\d+);$", re.MULTILINE)
_VERSIONED_GOLDEN_RE = re.compile(
    r"^(?:src|tests)/network/wire_golden_v\d+\.rs$"
)
_LEGACY_PATHS = {
    "src/network/wire_golden_legacy_0_9.rs",
    "tests/network/wire_golden_legacy_0_9.rs",
}
_VERSION_PATH = "src/lib.rs"


def protected_path(path: str) -> bool:
    """Return whether `path` is a released immutable wire fixture."""
    return path in _LEGACY_PATHS or _VERSIONED_GOLDEN_RE.fullmatch(path) is not None


def parse_protocol_version(text: str) -> int:
    """Read the one canonical numeric `PROTOCOL_VERSION` declaration."""
    matches = _VERSION_RE.findall(text)
    if len(matches) != 1:
        raise ValueError("expected exactly one literal PROTOCOL_VERSION declaration")
    return int(matches[0])


def _git(repo_root: Path, args: list[str]) -> bytes:
    result = subprocess.run(
        ["git", "-C", str(repo_root), *args], capture_output=True, check=False
    )
    if result.returncode != 0:
        detail = result.stderr.decode("utf-8", errors="replace").strip()
        raise RuntimeError(f"git {' '.join(args)} failed: {detail}")
    return result.stdout


def _changed_existing_goldens(
    repo_root: Path, cached: bool, base_ref: str | None
) -> list[str]:
    args = ["diff", "--name-status", "--find-renames", "--find-copies-harder", "-z"]
    if cached:
        args.append("--cached")
    args.extend([base_ref, "HEAD"] if base_ref is not None else ["HEAD"])
    fields = _git(repo_root, args).split(b"\0")
    changed: set[str] = set()
    index = 0
    while index < len(fields) and fields[index]:
        status = fields[index].decode("ascii", errors="strict")
        index += 1
        if status.startswith(("R", "C")):
            old = fields[index].decode("utf-8", errors="strict")
            new = fields[index + 1].decode("utf-8", errors="strict")
            index += 2
            if status.startswith("R") and protected_path(old):
                changed.add(old)
            if protected_path(new) and _base_has_path(repo_root, new, base_ref):
                changed.add(new)
        else:
            path = fields[index].decode("utf-8", errors="strict")
            index += 1
            if status[0] != "A" and protected_path(path):
                changed.add(path)
    return sorted(changed)


def _base_has_path(repo_root: Path, path: str, base_ref: str | None) -> bool:
    """Return whether `path` existed in the selected comparison base."""
    base = base_ref if base_ref is not None else "HEAD"
    return bool(_git(repo_root, ["ls-tree", "-z", "--name-only", base, "--", path]))


def _candidate_version_text(repo_root: Path, cached: bool) -> str:
    if cached:
        return _git(repo_root, ["show", f":{_VERSION_PATH}"]).decode("utf-8")
    return (repo_root / _VERSION_PATH).read_text(encoding="utf-8")


def _candidate_file_text(
    repo_root: Path, path: str, cached: bool, base_ref: str | None
) -> str:
    if base_ref is not None:
        return _git(repo_root, ["show", f"HEAD:{path}"]).decode("utf-8")
    if cached:
        return _git(repo_root, ["show", f":{path}"]).decode("utf-8")
    return (repo_root / path).read_text(encoding="utf-8")


def _rust_code_only(text: str) -> str:
    """Blank comments and literal syntax while preserving code layout/path text."""
    output: list[str] = []
    index = 0
    block_depth = 0
    while index < len(text):
        if block_depth:
            if text.startswith("/*", index):
                block_depth += 1
                output.extend("  ")
                index += 2
            elif text.startswith("*/", index):
                block_depth -= 1
                output.extend("  ")
                index += 2
            else:
                output.append("\n" if text[index] == "\n" else " ")
                index += 1
            continue

        if text.startswith("//", index):
            newline = text.find("\n", index + 2)
            if newline == -1:
                output.extend(" " * (len(text) - index))
                break
            output.extend(" " * (newline - index))
            output.append("\n")
            index = newline + 1
            continue
        if text.startswith("/*", index):
            block_depth = 1
            output.extend("  ")
            index += 2
            continue

        char_prefix = 2 if text.startswith("b'", index) else 1
        if text[index] == "'" or char_prefix == 2:
            end = index + char_prefix
            escaped = False
            closed = False
            while end < len(text) and text[end] != "\n":
                char = text[end]
                end += 1
                if escaped:
                    escaped = False
                elif char == "\\":
                    escaped = True
                elif char == "'":
                    closed = True
                    break
            if closed:
                output.extend(" " * (end - index))
                index = end
                continue

        raw_match = re.match(r"(?:br|r)(?P<hashes>#{0,255})\"", text[index:])
        if raw_match is not None:
            prefix_len = raw_match.end()
            hashes = raw_match.group("hashes")
            closing = f'"{hashes}'
            end = text.find(closing, index + prefix_len)
            end = len(text) if end == -1 else end + len(closing)
            literal = text[index:end]
            output.extend(
                char if char == "\n" or char.isalnum() or char in {'"', "_", "."} else " "
                for char in literal
            )
            index = end
            continue

        prefix_len = 2 if text.startswith('b"', index) else 1
        if text[index] == '"' or prefix_len == 2:
            end = index + prefix_len
            escaped = False
            while end < len(text):
                char = text[end]
                end += 1
                if escaped:
                    escaped = False
                elif char == "\\":
                    escaped = True
                elif char == '"':
                    break
            literal = text[index:end]
            output.extend(
                char if char == "\n" or char.isalnum() or char in {'"', "_", "."} else " "
                for char in literal
            )
            index = end
            continue

        output.append(text[index])
        index += 1
    return "".join(output)


def _top_level_lines(text: str) -> set[int]:
    """Return line indexes whose first token is outside every Rust delimiter."""
    top_level: set[int] = set()
    depths = {"{": 0, "(": 0, "[": 0}
    closing = {"}": "{", ")": "(", "]": "["}
    for index, line in enumerate(text.splitlines()):
        if all(depth == 0 for depth in depths.values()):
            top_level.add(index)
        for char in line:
            if char in depths:
                depths[char] += 1
            elif char in closing and depths[closing[char]] > 0:
                depths[closing[char]] -= 1
    return top_level


def _has_top_level_disabling_inner_attribute(text: str) -> bool:
    return re.search(r"#!\s*\[\s*cfg(?:_attr)?\s*\(", text, flags=re.DOTALL) is not None


def _attributes_before(text: str, item_start: int) -> list[str]:
    """Return complete contiguous Rust attributes before an item, bottom-up."""
    attributes: list[str] = []
    index = item_start - 1
    while index >= 0:
        while index >= 0 and text[index].isspace():
            index -= 1
        if index < 0 or text[index] != "]":
            break
        end = index + 1
        depth = 1
        index -= 1
        while index >= 0 and depth:
            if text[index] == "]":
                depth += 1
            elif text[index] == "[":
                depth -= 1
            index -= 1
        if depth:
            break
        while index >= 0 and text[index].isspace():
            index -= 1
        if index >= 0 and text[index] == "!":
            index -= 1
        if index < 0 or text[index] != "#":
            break
        start = index
        attributes.append(re.sub(r"\s+", "", text[start:end]))
        index = start - 1
    return attributes


def _line_offsets(text: str) -> list[int]:
    offsets: list[int] = []
    offset = 0
    for line in text.splitlines(keepends=True):
        offsets.append(offset)
        offset += len(line)
    return offsets


def _has_registered_test_module(codec: str, version: int) -> bool:
    lines = codec.splitlines()
    offsets = _line_offsets(codec)
    top_level = _top_level_lines(codec)
    declaration = f"mod wire_golden_v{version};"
    expected_attributes = [
        f'#[path="wire_golden_v{version}.rs"]',
        "#[cfg(test)]",
    ]
    return any(
        line.strip() == declaration
        and index in top_level
        and _attributes_before(
            codec, offsets[index] + len(line) - len(line.lstrip())
        )
        == expected_attributes
        for index, line in enumerate(lines)
    )


def _registered_version_modules(codec: str) -> set[int]:
    """Return top-level versioned golden modules compiled by the current codec."""
    top_level = _top_level_lines(codec)
    declaration = re.compile(r"mod wire_golden_v(\d+);")
    return {
        int(match.group(1))
        for index, line in enumerate(codec.splitlines())
        if index in top_level and (match := declaration.fullmatch(line.strip()))
    }


def _golden_test_body(suite: str, version: int) -> str | None:
    """Return the registered top-level golden test body, or None."""
    lines = suite.splitlines()
    offsets = _line_offsets(suite)
    top_level = _top_level_lines(suite)
    declaration = (
        f"fn every_protocol_v{version}_variant_has_immutable_exact_bytes() {{"
    )
    for index, line in enumerate(lines):
        if (
            line.strip() != declaration
            or index not in top_level
            or _attributes_before(
                suite, offsets[index] + len(line) - len(line.lstrip())
            )
            != ["#[test]"]
        ):
            continue
        return _balanced_body_after(suite, offsets[index])
    return None


def _balanced_body_after(text: str, start: int) -> str | None:
    """Return the balanced braced body beginning at or after `start`."""
    opening = text.find("{", start)
    if opening == -1:
        return None
    depth = 1
    cursor = opening + 1
    while cursor < len(text) and depth:
        if text[cursor] == "{":
            depth += 1
        elif text[cursor] == "}":
            depth -= 1
        cursor += 1
    return None if depth else text[opening + 1 : cursor - 1]


def _top_level_segments(text: str) -> list[str]:
    """Split comma-separated Rust syntax without splitting nested delimiters."""
    segments: list[str] = []
    start = 0
    depths = {"{": 0, "(": 0, "[": 0}
    closing = {"}": "{", ")": "(", "]": "["}
    for index, char in enumerate(text):
        if char in depths:
            depths[char] += 1
        elif char in closing and depths[closing[char]] > 0:
            depths[closing[char]] -= 1
        elif char == "," and all(depth == 0 for depth in depths.values()):
            segments.append(text[start:index])
            start = index + 1
    segments.append(text[start:])
    return segments


def _expected_match_is_exhaustive(suite: str) -> bool:
    """Require the expected-byte mapping to use only explicit MessageBody arms."""
    lines = suite.splitlines()
    offsets = _line_offsets(suite)
    top_level = _top_level_lines(suite)
    declaration = re.compile(
        r"^(?:pub(?:\s*\([^)]*\))?\s+)?fn expected\(body: &MessageBody\) -> &'static \[u8\] \{$"
    )
    candidates = [
        (index, line)
        for index, line in enumerate(lines)
        if index in top_level and declaration.fullmatch(line.strip())
    ]
    if len(candidates) != 1:
        return False
    index, line = candidates[0]
    item_start = offsets[index] + len(line) - len(line.lstrip())
    if _attributes_before(suite, item_start):
        return False
    function_body = _balanced_body_after(suite, offsets[index])
    if function_body is None:
        return False
    function_body = function_body.strip()
    match_declaration = re.match(r"match\s+body\s*\{", function_body)
    if match_declaration is None:
        return False
    opening = function_body.find("{", match_declaration.start())
    depth = 1
    cursor = opening + 1
    while cursor < len(function_body) and depth:
        if function_body[cursor] == "{":
            depth += 1
        elif function_body[cursor] == "}":
            depth -= 1
        cursor += 1
    if depth or function_body[cursor:].strip() not in {"", ";"}:
        return False
    match_body = function_body[opening + 1 : cursor - 1]
    explicit_pattern = re.compile(
        r"MessageBody::[A-Z][A-Za-z0-9_]*(?:\([^|]*\)|\{[^|]*\})?"
    )
    arms = [arm.strip() for arm in _top_level_segments(match_body) if arm.strip()]
    if not arms:
        return False
    for arm in arms:
        parts = arm.split("=>")
        if len(parts) != 2 or explicit_pattern.fullmatch(parts[0].strip()) is None:
            return False
    return True


def _suite_has_required_structure(suite: str, codec: str, version: int) -> bool:
    """Require a registered successor suite with load-bearing wire assertions."""
    suite = _rust_code_only(suite)
    codec = _rust_code_only(codec)
    if _has_top_level_disabling_inner_attribute(suite) or _has_top_level_disabling_inner_attribute(
        codec
    ):
        return False
    suite_lines = suite.splitlines()
    suite_top_level = _top_level_lines(suite)
    marker_line = re.compile(
        rf"^(?:pub(?:\s*\([^)]*\))?\s+)?const WIRE_GOLDEN_VERSION: u8 = {version};$"
    )
    marker = any(
        marker_line.fullmatch(line.strip()) is not None and index in suite_top_level
        for index, line in enumerate(suite_lines)
    )
    body = _golden_test_body(suite, version)
    if (
        not marker
        or body is None
        or not _has_registered_test_module(codec, version)
        or _registered_version_modules(codec) != {version}
    ):
        return False
    compact_body = re.sub(r"\s+", "", body)
    required_call = (
        "super::assert_wire_golden_suite(WIRE_GOLDEN_VERSION,fixtures(),expected);"
    )
    return compact_body == required_call and _expected_match_is_exhaustive(suite)


def _candidate_has_version_suite(
    repo_root: Path, version: int, cached: bool, base_ref: str | None
) -> bool:
    path = f"src/network/wire_golden_v{version}.rs"
    try:
        suite = _candidate_file_text(repo_root, path, cached, base_ref)
        codec = _candidate_file_text(
            repo_root, "src/network/codec.rs", cached, base_ref
        )
    except (OSError, UnicodeError, RuntimeError):
        return False
    return _suite_has_required_structure(suite, codec, version)


def check_diff(
    repo_root: Path, cached: bool = False, base_ref: str | None = None
) -> bool:
    """Check a local candidate or committed `base_ref..HEAD` diff."""
    try:
        changed = _changed_existing_goldens(repo_root, cached, base_ref)
        if not changed:
            return True
        base = base_ref if base_ref is not None else "HEAD"
        base_text = _git(repo_root, ["show", f"{base}:{_VERSION_PATH}"]).decode("utf-8")
        base_version = parse_protocol_version(base_text)
        candidate_text = (
            _git(repo_root, ["show", f"HEAD:{_VERSION_PATH}"]).decode("utf-8")
            if base_ref is not None
            else _candidate_version_text(repo_root, cached)
        )
        candidate_version = parse_protocol_version(candidate_text)
    except (OSError, UnicodeError, ValueError, RuntimeError) as error:
        print(f"{_VERSION_PATH}:0: cannot verify wire-golden immutability: {error}", file=sys.stderr)
        return False

    if candidate_version > base_version and _candidate_has_version_suite(
        repo_root, candidate_version, cached, base_ref
    ):
        return True

    for path in changed:
        print(
            f"{path}:0: released wire golden changed without a PROTOCOL_VERSION bump",
            file=sys.stderr,
        )
    print(
        "  remedy: restore the historical fixture, or increase PROTOCOL_VERSION and replace the active versioned registration with its matching wire_golden_vN.rs suite",
        file=sys.stderr,
    )
    return False


def check_local(repo_root: Path) -> bool:
    """Check both worktree and index views against HEAD."""
    worktree_ok = check_diff(repo_root)
    index_ok = check_diff(repo_root, cached=True)
    return worktree_ok and index_ok


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cached", action="store_true", help="compare HEAD with the index")
    parser.add_argument(
        "--local", action="store_true", help="check both worktree and index views"
    )
    parser.add_argument(
        "--base-ref", help="compare a committed base ref with HEAD (CI/PR mode)"
    )
    args = parser.parse_args()
    selected_modes = sum((args.cached, args.local, args.base_ref is not None))
    if selected_modes > 1:
        parser.error("--cached, --local, and --base-ref are mutually exclusive")
    repo_root = Path(__file__).resolve().parents[2]
    passed = (
        check_local(repo_root)
        if args.local
        else check_diff(repo_root, cached=args.cached, base_ref=args.base_ref)
    )
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
