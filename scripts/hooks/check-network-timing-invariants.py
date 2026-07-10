#!/usr/bin/env python3
"""Validate network timing budgets and protocol test clock usage."""

from __future__ import annotations

import re
import sys
from pathlib import Path

NEXTEST_CONFIG = Path(".config/nextest.toml")
MULTI_PROCESS_TEST = Path("tests/network/multi_process.rs")
PROTOCOL_MODULE = Path("src/network/protocol/mod.rs")

NETWORK_FILTER = "test(network::multi_process::)"

U64_CONSTS = (
    "PROCESS_TIMEOUT_OVERHEAD_SECS",
    "PER_PR_MAX_PEER_TIMEOUT_SECS",
    "NIGHTLY_MAX_PEER_TIMEOUT_SECS",
    "MACOS_TIMEOUT_SCALE_NUMERATOR",
    "MACOS_TIMEOUT_SCALE_DENOMINATOR",
)


def _read_text(repo_root: Path, path: Path) -> str:
    return (repo_root / path).read_text(encoding="utf-8")


def _line_number(source: str, offset: int) -> int:
    return source.count("\n", 0, offset) + 1


def _blank(ch: str) -> str:
    return "\n" if ch == "\n" else " "


def _raw_string_opener(text: str, offset: int) -> tuple[int, str] | None:
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
    """Blank Rust comments and strings while preserving offsets."""
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


def _u64_consts(source: str) -> dict[str, int]:
    values: dict[str, int] = {}
    for name in U64_CONSTS:
        match = re.search(rf"\bconst\s+{name}\s*:\s*u64\s*=\s*(\d+)\s*;", source)
        if match is None:
            raise ValueError(f"missing `const {name}: u64 = ...;`")
        values[name] = int(match.group(1))
    return values


def _duration_seconds(value: str) -> int:
    match = re.fullmatch(r"(\d+)s", value.strip())
    if match is None:
        raise ValueError(f"unsupported nextest duration {value!r}; expected '<seconds>s'")
    return int(match.group(1))


def _network_budget_seconds(nextest_source: str, profile: str) -> int:
    override_pattern = re.compile(
        rf"(?ms)^\s*\[\[profile\.{re.escape(profile)}\.overrides\]\]\s*"
        r"(.*?)(?=^\s*\[|\Z)"
    )
    for match in override_pattern.finditer(nextest_source):
        block = match.group(1)
        filter_match = re.search(r"(?m)^\s*filter\s*=\s*'([^']+)'\s*$", block)
        if filter_match is None or filter_match.group(1) != NETWORK_FILTER:
            continue

        slow_timeout_match = re.search(
            r'(?m)^\s*slow-timeout\s*=\s*\{\s*period\s*=\s*"([^"]+)"\s*,\s*'
            r"terminate-after\s*=\s*(\d+)\s*\}",
            block,
        )
        if slow_timeout_match is None:
            raise ValueError(f"profile {profile} network slow-timeout is malformed")
        period = slow_timeout_match.group(1)
        terminate_after = int(slow_timeout_match.group(2))
        return _duration_seconds(period) * terminate_after

    raise ValueError(f"profile {profile} has no {NETWORK_FILTER!r} override")


def _macos_scaled_seconds(base: int, numerator: int, denominator: int) -> int:
    if denominator <= 0:
        raise ValueError("MACOS_TIMEOUT_SCALE_DENOMINATOR must be positive")
    return (base * numerator) // denominator


def _check_nextest_budgets(
    nextest_source: str,
    multi_process_source: str,
) -> list[str]:
    consts = _u64_consts(multi_process_source)

    overhead = consts["PROCESS_TIMEOUT_OVERHEAD_SECS"]
    numerator = consts["MACOS_TIMEOUT_SCALE_NUMERATOR"]
    denominator = consts["MACOS_TIMEOUT_SCALE_DENOMINATOR"]
    smoke_ceiling = (
        _macos_scaled_seconds(
            consts["PER_PR_MAX_PEER_TIMEOUT_SECS"],
            numerator,
            denominator,
        )
        + overhead
    )
    nightly_ceiling = (
        _macos_scaled_seconds(
            consts["NIGHTLY_MAX_PEER_TIMEOUT_SECS"],
            numerator,
            denominator,
        )
        + overhead
    )

    checks = (
        ("profile.default", "default", smoke_ceiling),
        ("profile.ci", "ci", smoke_ceiling),
        ("profile.ci-network-nightly", "ci-network-nightly", nightly_ceiling),
    )

    errors: list[str] = []
    for label, profile, ceiling in checks:
        budget = _network_budget_seconds(nextest_source, profile)
        if budget <= ceiling:
            errors.append(
                f"{NEXTEST_CONFIG}:0: {label} network slow-timeout budget is {budget}s, "
                f"but the macOS-scaled harness ceiling is {ceiling}s; nextest must "
                "exceed the harness ceiling so peer diagnostics are emitted"
            )
    return errors


def _check_multi_process_source(source: str) -> list[str]:
    stripped = _blank_comments_and_strings(source)
    errors: list[str] = []
    for match in re.finditer(r"\bwait_for_peer\s*\(", stripped):
        errors.append(
            f"{MULTI_PROCESS_TEST}:{_line_number(stripped, match.start())}: "
            "direct wait_for_peer() calls bypass scenario-derived process timeouts; "
            "use wait_for_peer_with_timeout() through the shared harness helper"
        )
    return errors


def _check_protocol_source(source: str) -> list[str]:
    stripped = _blank_comments_and_strings(source)
    errors: list[str] = []
    sleep_pattern = re.compile(r"\b(?:std::)?thread::sleep\s*\(", re.MULTILINE)
    for match in sleep_pattern.finditer(stripped):
        errors.append(
            f"{PROTOCOL_MODULE}:{_line_number(stripped, match.start())}: "
            "protocol tests must use ProtocolConfig.clock and virtual time instead of thread::sleep()"
        )
    return errors


def check_repo(repo_root: Path) -> list[str]:
    errors: list[str] = []
    try:
        nextest_source = _read_text(repo_root, NEXTEST_CONFIG)
        multi_process_source = _read_text(repo_root, MULTI_PROCESS_TEST)
        protocol_source = _read_text(repo_root, PROTOCOL_MODULE)
        errors.extend(_check_nextest_budgets(nextest_source, multi_process_source))
        errors.extend(_check_multi_process_source(multi_process_source))
        errors.extend(_check_protocol_source(protocol_source))
    except (OSError, ValueError) as exc:
        errors.append(f"network timing invariant check failed: {exc}")
    return errors


def main() -> int:
    errors = check_repo(Path.cwd())
    if errors:
        print("ERROR: network timing invariants failed:", file=sys.stderr)
        for error in errors:
            print(error, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
