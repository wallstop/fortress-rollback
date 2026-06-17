"""Regression tests for deterministic, host-independent wire byte order."""

from __future__ import annotations

import re
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[2]
SRC_ROOT = REPO_ROOT / "src"

WIRE_SOURCE_ROOTS = (SRC_ROOT / "network",)
EXTRA_WIRE_SOURCE_PATHS = (
    Path("replay.rs"),
    Path("rng.rs"),
    Path("sessions/hot_join.rs"),
)

FORBIDDEN_NATIVE_ENDIAN_PATTERNS = (
    ("to_ne_bytes", re.compile(r"\bto_ne_bytes\s*\(")),
    ("from_ne_bytes", re.compile(r"\bfrom_ne_bytes\s*\(")),
    ("target_endian cfg", re.compile(r"\btarget_endian\b")),
    ("native endian config", re.compile(r"\bNativeEndian\b|with_native_endian\s*\(")),
)

BINC_CONFIG_CALL = re.compile(
    r"bincode::config::standard\(\)"
    r"(?P<chain>(?:\s*\.\s*[A-Za-z_][A-Za-z0-9_]*"
    r"(?:\s*::\s*<[^>]+>)?\s*\([^)]*\))*)",
    re.MULTILINE,
)


def _source_files() -> list[Path]:
    return sorted(SRC_ROOT.rglob("*.rs"))


def _wire_source_files() -> list[Path]:
    files = {
        path
        for root in WIRE_SOURCE_ROOTS
        if root.exists()
        for path in root.rglob("*.rs")
    }
    files.update(
        SRC_ROOT / relative_path
        for relative_path in EXTRA_WIRE_SOURCE_PATHS
        if (SRC_ROOT / relative_path).exists()
    )
    return sorted(files)


def _line_number(text: str, offset: int) -> int:
    return text.count("\n", 0, offset) + 1


def _strip_rust_comments(text: str) -> str:
    """Blank Rust comments while preserving byte offsets for diagnostics."""
    chars = list(text)
    index = 0
    in_string = False
    in_char = False
    in_block_comment = 0
    escaped = False

    while index < len(chars):
        current = chars[index]
        next_char = chars[index + 1] if index + 1 < len(chars) else ""

        if in_block_comment:
            if current == "/" and next_char == "*":
                chars[index] = " "
                chars[index + 1] = " "
                in_block_comment += 1
                index += 2
                continue
            if current == "*" and next_char == "/":
                chars[index] = " "
                chars[index + 1] = " "
                in_block_comment -= 1
                index += 2
                continue
            if current != "\n":
                chars[index] = " "
            index += 1
            continue

        if in_string:
            escaped = current == "\\" and not escaped
            if current == '"' and not escaped:
                in_string = False
            elif current != "\\":
                escaped = False
            index += 1
            continue

        if in_char:
            escaped = current == "\\" and not escaped
            if current == "'" and not escaped:
                in_char = False
            elif current != "\\":
                escaped = False
            index += 1
            continue

        if current == "/" and next_char == "/":
            chars[index] = " "
            chars[index + 1] = " "
            index += 2
            while index < len(chars) and chars[index] != "\n":
                chars[index] = " "
                index += 1
            continue

        if current == "/" and next_char == "*":
            chars[index] = " "
            chars[index + 1] = " "
            in_block_comment = 1
            index += 2
            continue

        if current == '"':
            in_string = True
            escaped = False
        elif current == "'":
            in_char = True
            escaped = False
        index += 1

    return "".join(chars)


@pytest.mark.parametrize(
    "path",
    _wire_source_files(),
    ids=lambda path: str(path.relative_to(REPO_ROOT)),
)
def test_production_source_avoids_native_endian_wire_apis(path: Path) -> None:
    """Wire-format Rust code must use explicit endian APIs for deterministic bytes."""
    text = path.read_text(encoding="utf-8")
    code = _strip_rust_comments(text)
    for name, pattern in FORBIDDEN_NATIVE_ENDIAN_PATTERNS:
        match = pattern.search(code)
        assert match is None, (
            f"{path.relative_to(REPO_ROOT)}:{_line_number(text, match.start())} "
            f"uses {name}; use an explicit little-endian or big-endian wire format."
        )


@pytest.mark.parametrize(
    "path",
    _source_files(),
    ids=lambda path: str(path.relative_to(REPO_ROOT)),
)
def test_bincode_standard_configs_select_little_endian_explicitly(path: Path) -> None:
    """Bincode's default is little-endian today, but the wire config must say so."""
    text = path.read_text(encoding="utf-8")
    code = _strip_rust_comments(text)
    for match in BINC_CONFIG_CALL.finditer(code):
        chain = match.group("chain")
        assert ".with_little_endian()" in chain, (
            f"{path.relative_to(REPO_ROOT)}:{_line_number(text, match.start())} "
            "uses bincode::config::standard() without .with_little_endian()."
        )


@pytest.mark.parametrize(
    ("source", "expected_chain"),
    [
        ("bincode::config::standard()", ""),
        (
            "bincode::config::standard().with_fixed_int_encoding()",
            ".with_fixed_int_encoding()",
        ),
        (
            "bincode::config::standard()\n    .with_little_endian()\n    .with_fixed_int_encoding()",
            "\n    .with_little_endian()\n    .with_fixed_int_encoding()",
        ),
    ],
)
def test_bincode_standard_config_detector_covers_direct_and_chained_forms(
    source: str, expected_chain: str
) -> None:
    """Regression coverage for the regex backing the source guard."""
    match = BINC_CONFIG_CALL.search(source)
    assert match is not None
    assert match.group("chain") == expected_chain
