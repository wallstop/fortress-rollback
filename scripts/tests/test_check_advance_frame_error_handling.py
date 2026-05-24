#!/usr/bin/env python3
"""Unit tests for scripts/hooks/check-advance-frame-error-handling.py."""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

scripts_dir = Path(__file__).parent.parent
spec = importlib.util.spec_from_file_location(
    "check_advance_frame_error_handling",
    scripts_dir / "hooks" / "check-advance-frame-error-handling.py",
)
check_advance_frame_error_handling = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = check_advance_frame_error_handling
spec.loader.exec_module(check_advance_frame_error_handling)

check_file = check_advance_frame_error_handling.check_file


def test_flags_if_let_ok_advance_frame(tmp_path: Path) -> None:
    path = tmp_path / "spectator.rs"
    path.write_text(
        "fn test() {\n"
        "    if let Ok(requests) = session.advance_frame() {\n"
        "        game.handle_requests(requests);\n"
        "    }\n"
        "}\n",
        encoding="utf-8",
    )

    issues = check_file(path)

    assert len(issues) == 1
    assert "advance_frame() error is ignored" in issues[0]


def test_flags_split_if_let_ok_advance_frame(tmp_path: Path) -> None:
    path = tmp_path / "spectator.rs"
    path.write_text(
        "fn test() {\n"
        "    if let Ok(requests) =\n"
        "        session.advance_frame()\n"
        "    {\n"
        "        game.handle_requests(requests);\n"
        "    }\n"
        "}\n",
        encoding="utf-8",
    )

    issues = check_file(path)

    assert len(issues) == 1


def test_flags_while_let_ok_advance_frame(tmp_path: Path) -> None:
    path = tmp_path / "spectator.rs"
    path.write_text(
        "fn test() {\n"
        "    while let Ok(requests) = session.advance_frame() {\n"
        "        game.handle_requests(requests);\n"
        "    }\n"
        "}\n",
        encoding="utf-8",
    )

    issues = check_file(path)

    assert len(issues) == 1


def test_flags_discarded_advance_frame_result(tmp_path: Path) -> None:
    path = tmp_path / "spectator.rs"
    path.write_text(
        "fn test() {\n"
        "    let _ = session.advance_frame();\n"
        "}\n",
        encoding="utf-8",
    )

    issues = check_file(path)

    assert len(issues) == 1
    assert "result is discarded" in issues[0]


def test_flags_advance_frame_ok_and_is_ok(tmp_path: Path) -> None:
    path = tmp_path / "spectator.rs"
    path.write_text(
        "fn test() {\n"
        "    let _maybe = session.advance_frame().ok();\n"
        "    assert!(session.advance_frame().is_ok());\n"
        "}\n",
        encoding="utf-8",
    )

    issues = check_file(path)

    assert len(issues) == 2
    assert all("error detail is discarded" in issue for issue in issues)


def test_allows_explicit_match_on_advance_frame(tmp_path: Path) -> None:
    path = tmp_path / "spectator.rs"
    path.write_text(
        "fn test() {\n"
        "    match session.advance_frame() {\n"
        "        Ok(requests) => game.handle_requests(requests),\n"
        "        Err(FortressError::PredictionThreshold) => {},\n"
        "        Err(err) => panic!(\"unexpected: {err:?}\"),\n"
        "    }\n"
        "}\n",
        encoding="utf-8",
    )

    issues = check_file(path)

    assert issues == []


def test_allows_unrelated_if_let_ok_before_advance_frame(tmp_path: Path) -> None:
    path = tmp_path / "spectator.rs"
    path.write_text(
        "fn test() -> Result<(), FortressError> {\n"
        "    if let Ok(value) = value.parse::<u32>() {\n"
        "        session.advance_frame()?;\n"
        "        assert_eq!(value, 1);\n"
        "    }\n"
        "    Ok(())\n"
        "}\n",
        encoding="utf-8",
    )

    issues = check_file(path)

    assert issues == []


def test_allows_error_handled_discard_of_request_value(tmp_path: Path) -> None:
    path = tmp_path / "spectator.rs"
    path.write_text(
        "fn test() -> Result<(), FortressError> {\n"
        "    let _ = session.advance_frame()?;\n"
        "    let _ = session.advance_frame().expect(\"advance\");\n"
        "    Ok(())\n"
        "}\n",
        encoding="utf-8",
    )

    issues = check_file(path)

    assert issues == []


def test_ignores_comments(tmp_path: Path) -> None:
    path = tmp_path / "spectator.rs"
    path.write_text(
        "// if let Ok(requests) = session.advance_frame() {}\n"
        "/// if let Ok(requests) = session.advance_frame() {}\n",
        encoding="utf-8",
    )

    issues = check_file(path)

    assert issues == []


def test_ignores_block_comments(tmp_path: Path) -> None:
    path = tmp_path / "spectator.rs"
    path.write_text(
        "/*\n"
        "if let Ok(requests) = session.advance_frame() {\n"
        "    game.handle_requests(requests);\n"
        "}\n"
        "*/\n"
        "/*! assert!(session.advance_frame().is_ok()); */\n",
        encoding="utf-8",
    )

    issues = check_file(path)

    assert issues == []


def test_ignores_nested_block_comments(tmp_path: Path) -> None:
    path = tmp_path / "spectator.rs"
    path.write_text(
        "/* outer\n"
        "   /* inner */\n"
        "   let _ = session.advance_frame();\n"
        "*/\n",
        encoding="utf-8",
    )

    issues = check_file(path)

    assert issues == []


def test_ignores_ordinary_string_literals(tmp_path: Path) -> None:
    path = tmp_path / "spectator.rs"
    path.write_text(
        'const EXAMPLE: &str = "if let Ok(requests) = session.advance_frame() {}";\n'
        'const ESCAPED: &str = "quote: \\"; let _ = session.advance_frame();";\n'
        'const MULTILINE: &str = "assert!(session.advance_frame().is_ok());\n'
        'still inside the string";\n',
        encoding="utf-8",
    )

    issues = check_file(path)

    assert issues == []


def test_ignores_raw_and_prefixed_string_literals(tmp_path: Path) -> None:
    path = tmp_path / "spectator.rs"
    path.write_text(
        'const RAW: &str = r#"if let Ok(requests) = session.advance_frame() {}"#;\n'
        'const BYTE_RAW: &[u8] = br##"assert!(session.advance_frame().is_ok());"##;\n'
        'const BYTE: &[u8] = b"let _ = session.advance_frame();";\n'
        'const C_STRING: &CStr = c"session.advance_frame().ok()";\n'
        'const C_RAW: &CStr = cr#"while let Ok(x) = session.advance_frame() {}"#;\n',
        encoding="utf-8",
    )

    issues = check_file(path)

    assert issues == []


def test_ignores_char_and_byte_char_literals_without_masking_lifetimes(
    tmp_path: Path,
) -> None:
    path = tmp_path / "spectator.rs"
    path.write_text(
        "fn test<'a>(value: &'a str) -> &'a str {\n"
        "    let quote = '\\'';\n"
        "    let unicode = '\\u{27}';\n"
        "    let byte = b'(';\n"
        "    let label = 'retry: loop { break 'retry; };\n"
        "    session.advance_frame()?;\n"
        "    value\n"
        "}\n",
        encoding="utf-8",
    )

    issues = check_file(path)

    assert issues == []


def test_still_flags_code_after_ignored_spans(tmp_path: Path) -> None:
    path = tmp_path / "spectator.rs"
    path.write_text(
        "/* let _ = session.advance_frame(); */\n"
        'const EXAMPLE: &str = "if let Ok(requests) = session.advance_frame() {}";\n'
        "fn test() {\n"
        "    let _ = session.advance_frame();\n"
        "}\n",
        encoding="utf-8",
    )

    issues = check_file(path)

    assert len(issues) == 1
    assert ":4:" in issues[0]
    assert "result is discarded" in issues[0]
