#!/usr/bin/env python3
"""Tests for the released wire-golden immutability hook."""

from __future__ import annotations

import importlib.util
import os
import subprocess
import sys
from pathlib import Path

import pytest

SCRIPTS = Path(__file__).parent.parent
SPEC = importlib.util.spec_from_file_location(
    "check_wire_golden_immutable",
    SCRIPTS / "hooks" / "check-wire-golden-immutable.py",
)
assert SPEC is not None and SPEC.loader is not None
HOOK = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = HOOK
SPEC.loader.exec_module(HOOK)


def _git(repo: Path, *args: str) -> None:
    subprocess.run(["git", "-C", str(repo), *args], check=True, capture_output=True)


def _write(repo: Path, path: str, text: str) -> None:
    destination = repo / path
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(text, encoding="utf-8")


def _add_v2_suite(repo: Path) -> None:
    _write(
        repo,
        "src/network/wire_golden_v2.rs",
        "const WIRE_GOLDEN_VERSION: u8 = 2;\n"
        "fn expected(body: &MessageBody) -> &'static [u8] {\n"
        "    match body { MessageBody::KeepAlive => &[2] }\n"
        "}\n"
        "fn fixtures() -> Vec<(&'static str, Message)> {\n"
        "    MessageKind::ALL.into_iter().map(fixture_for_kind).collect()\n"
        "}\n"
        "#[test]\n"
        "fn every_protocol_v2_variant_has_immutable_exact_bytes() {\n"
        "    super::assert_wire_golden_suite(WIRE_GOLDEN_VERSION, fixtures(), expected);\n"
        "}\n",
    )
    _write(
        repo,
        "src/network/codec.rs",
        '#[cfg(test)]\n#[path = "wire_golden_v2.rs"]\nmod wire_golden_v2;\n',
    )


@pytest.fixture()
def repo(tmp_path: Path) -> Path:
    _git(tmp_path, "init", "-q")
    _git(tmp_path, "config", "user.email", "wire@example.invalid")
    _git(tmp_path, "config", "user.name", "Wire Test")
    _write(tmp_path, "src/lib.rs", "pub const PROTOCOL_VERSION: u8 = 1;\n")
    _write(tmp_path, "src/network/wire_golden_v1.rs", "const V1: &[u8] = &[1];\n")
    _write(tmp_path, "src/network/wire_golden_legacy_0_9.rs", "const V0: &[u8] = &[0];\n")
    _write(tmp_path, "tests/network/wire_golden_legacy_0_9.rs", "const V0: &[u8] = &[0];\n")
    _write(tmp_path, "src/network/codec.rs", "")
    _git(tmp_path, "add", ".")
    _git(tmp_path, "commit", "-qm", "base")
    return tmp_path


def test_clean_repository_passes(repo: Path) -> None:
    assert HOOK.check_diff(repo)


def test_new_version_fixture_passes_without_bump(repo: Path) -> None:
    _write(repo, "src/network/wire_golden_v2.rs", "const V2: &[u8] = &[2];\n")
    _git(repo, "add", ".")
    assert HOOK.check_diff(repo, cached=True)


@pytest.mark.parametrize(
    "path",
    [
        "src/network/wire_golden_v1.rs",
        "src/network/wire_golden_legacy_0_9.rs",
        "tests/network/wire_golden_legacy_0_9.rs",
    ],
)
def test_existing_fixture_change_fails(repo: Path, path: str, capsys: pytest.CaptureFixture[str]) -> None:
    _write(repo, path, "changed\n")
    assert not HOOK.check_diff(repo)
    assert path in capsys.readouterr().err


def test_deleted_or_renamed_fixture_fails(repo: Path) -> None:
    (repo / "src/network/wire_golden_v1.rs").unlink()
    assert not HOOK.check_diff(repo)
    _git(repo, "restore", "src/network/wire_golden_v1.rs")
    _git(repo, "mv", "src/network/wire_golden_v1.rs", "src/network/old.rs")
    assert not HOOK.check_diff(repo)


def test_real_protocol_bump_allows_fixture_change(repo: Path) -> None:
    _write(repo, "src/network/wire_golden_v1.rs", "changed\n")
    _write(repo, "src/lib.rs", "pub const PROTOCOL_VERSION: u8 = 2;\n")
    _add_v2_suite(repo)
    assert HOOK.check_diff(repo)


def test_downgrade_or_missing_matching_suite_does_not_excuse_change(repo: Path) -> None:
    _write(repo, "src/network/wire_golden_v1.rs", "changed\n")
    _write(repo, "src/lib.rs", "pub const PROTOCOL_VERSION: u8 = 0;\n")
    assert not HOOK.check_diff(repo)
    _write(repo, "src/lib.rs", "pub const PROTOCOL_VERSION: u8 = 2;\n")
    assert not HOOK.check_diff(repo)


def test_empty_or_unwired_successor_does_not_excuse_change(repo: Path) -> None:
    _write(repo, "src/network/wire_golden_v1.rs", "changed\n")
    _write(repo, "src/lib.rs", "pub const PROTOCOL_VERSION: u8 = 2;\n")
    _write(repo, "src/network/wire_golden_v2.rs", "")
    assert not HOOK.check_diff(repo)


def test_wired_but_vacuous_successor_does_not_excuse_change(repo: Path) -> None:
    _write(repo, "src/network/wire_golden_v1.rs", "changed\n")
    _write(repo, "src/lib.rs", "pub const PROTOCOL_VERSION: u8 = 2;\n")
    _write(
        repo,
        "src/network/wire_golden_v2.rs",
        "const WIRE_GOLDEN_VERSION: u8 = 2;\n"
        "#[test]\n"
        "fn every_protocol_v2_variant_has_immutable_exact_bytes() {}\n",
    )
    _write(
        repo,
        "src/network/codec.rs",
        '#[cfg(test)]\n#[path = "wire_golden_v2.rs"]\nmod wire_golden_v2;\n',
    )
    assert not HOOK.check_diff(repo)


@pytest.mark.parametrize(
    "catch_all",
    [
        "_ => &[2]",
        "_\n => &[2]",
        "other => &[2]",
        "MessageBody::KeepAlive => &[2], _ => &[3]",
        "MessageBody::KeepAlive => &[2], other => &[3]",
    ],
)
def test_successor_catch_all_mapping_does_not_excuse_change(
    repo: Path, catch_all: str
) -> None:
    _write(repo, "src/network/wire_golden_v1.rs", "changed\n")
    _write(repo, "src/lib.rs", "pub const PROTOCOL_VERSION: u8 = 2;\n")
    _add_v2_suite(repo)
    suite = repo / "src/network/wire_golden_v2.rs"
    suite.write_text(
        suite.read_text(encoding="utf-8").replace(
            "MessageBody::KeepAlive => &[2]", catch_all
        ),
        encoding="utf-8",
    )
    assert not HOOK.check_diff(repo)


def test_successor_test_body_must_delegate_to_shared_harness(repo: Path) -> None:
    _write(repo, "src/network/wire_golden_v1.rs", "changed\n")
    _write(repo, "src/lib.rs", "pub const PROTOCOL_VERSION: u8 = 2;\n")
    _add_v2_suite(repo)
    suite = repo / "src/network/wire_golden_v2.rs"
    suite.write_text(
        suite.read_text(encoding="utf-8").replace(
            "super::assert_wire_golden_suite(WIRE_GOLDEN_VERSION, fixtures(), expected);",
            "assert_eq!(WIRE_GOLDEN_VERSION, 2);",
        ),
        encoding="utf-8",
    )
    assert not HOOK.check_diff(repo)


def test_local_harness_shadow_does_not_replace_shared_harness(repo: Path) -> None:
    _write(repo, "src/network/wire_golden_v1.rs", "changed\n")
    _write(repo, "src/lib.rs", "pub const PROTOCOL_VERSION: u8 = 2;\n")
    _add_v2_suite(repo)
    suite = repo / "src/network/wire_golden_v2.rs"
    text = suite.read_text(encoding="utf-8").replace(
        "super::assert_wire_golden_suite", "assert_wire_golden_suite"
    )
    suite.write_text(
        "fn assert_wire_golden_suite<A, B, C>(_: A, _: B, _: C) {}\n"
        + text,
        encoding="utf-8",
    )
    assert not HOOK.check_diff(repo)


def test_cfg_disabled_expected_decoy_does_not_hide_active_catch_all(repo: Path) -> None:
    _write(repo, "src/network/wire_golden_v1.rs", "changed\n")
    _write(repo, "src/lib.rs", "pub const PROTOCOL_VERSION: u8 = 2;\n")
    _add_v2_suite(repo)
    suite = repo / "src/network/wire_golden_v2.rs"
    text = suite.read_text(encoding="utf-8")
    decoy = (
        "#[cfg(any())]\n"
        "fn expected(body: &MessageBody) -> &'static [u8] {\n"
        "    match body { MessageBody::KeepAlive => &[2] }\n"
        "}\n"
    )
    suite.write_text(
        decoy + text.replace("MessageBody::KeepAlive => &[2]", "_ => &[2]"),
        encoding="utf-8",
    )
    assert not HOOK.check_diff(repo)


@pytest.mark.parametrize(
    "decoy",
    [
        "#[cfg(any())] let _guard = match body { MessageBody::KeepAlive => &[2] };",
        "discard!(match body { MessageBody::KeepAlive => &[2] });",
    ],
)
def test_discarded_explicit_match_does_not_hide_returned_helper(
    repo: Path, decoy: str
) -> None:
    _write(repo, "src/network/wire_golden_v1.rs", "changed\n")
    _write(repo, "src/lib.rs", "pub const PROTOCOL_VERSION: u8 = 2;\n")
    _add_v2_suite(repo)
    suite = repo / "src/network/wire_golden_v2.rs"
    text = suite.read_text(encoding="utf-8")
    old = "match body { MessageBody::KeepAlive => &[2] }"
    replacement = f"{decoy}\n    actual_expected(body)"
    suite.write_text(
        "fn actual_expected(_: &MessageBody) -> &'static [u8] { &[2] }\n"
        + text.replace(old, replacement),
        encoding="utf-8",
    )
    assert not HOOK.check_diff(repo)


@pytest.mark.parametrize(
    "pattern",
    [
        "_ | MessageBody::KeepAlive",
        "x @ _ | MessageBody::KeepAlive",
    ],
)
def test_successor_or_pattern_catch_all_does_not_excuse_change(
    repo: Path, pattern: str
) -> None:
    _write(repo, "src/network/wire_golden_v1.rs", "changed\n")
    _write(repo, "src/lib.rs", "pub const PROTOCOL_VERSION: u8 = 2;\n")
    _add_v2_suite(repo)
    suite = repo / "src/network/wire_golden_v2.rs"
    suite.write_text(
        suite.read_text(encoding="utf-8").replace("MessageBody::KeepAlive", pattern),
        encoding="utf-8",
    )
    assert not HOOK.check_diff(repo)


def test_unrelated_catch_all_does_not_reject_explicit_expected_mapping(repo: Path) -> None:
    _write(repo, "src/network/wire_golden_v1.rs", "changed\n")
    _write(repo, "src/lib.rs", "pub const PROTOCOL_VERSION: u8 = 2;\n")
    _add_v2_suite(repo)
    suite = repo / "src/network/wire_golden_v2.rs"
    suite.write_text(
        "fn unrelated(value: Option<u8>) { match value { _ => {} } }\n"
        + suite.read_text(encoding="utf-8"),
        encoding="utf-8",
    )
    assert HOOK.check_diff(repo)


def test_comment_only_successor_and_wiring_do_not_excuse_change(repo: Path) -> None:
    _write(repo, "src/network/wire_golden_v1.rs", "changed\n")
    _write(repo, "src/lib.rs", "pub const PROTOCOL_VERSION: u8 = 2;\n")
    _write(
        repo,
        "src/network/wire_golden_v2.rs",
        "/* const WIRE_GOLDEN_VERSION: u8 = 2;\n"
        "#[test]\nfn every_protocol_v2_variant_has_immutable_exact_bytes() {\n"
        "assert_eq!(x, y); match body {} encode(x); decode_message(x); } */\n",
    )
    _write(
        repo,
        "src/network/codec.rs",
        '/* #[path = "wire_golden_v2.rs"]\nmod wire_golden_v2; */\n',
    )
    assert not HOOK.check_diff(repo)


def test_nested_comment_only_successor_and_wiring_do_not_excuse_change(
    repo: Path,
) -> None:
    _write(repo, "src/network/wire_golden_v1.rs", "changed\n")
    _write(repo, "src/lib.rs", "pub const PROTOCOL_VERSION: u8 = 2;\n")
    fake_suite = (
        "/* outer /* inner */\n"
        "const WIRE_GOLDEN_VERSION: u8 = 2;\n"
        "#[test]\nfn every_protocol_v2_variant_has_immutable_exact_bytes() {}\n"
        "*/\n"
    )
    fake_wiring = (
        '/* outer /* inner */\n#[path = "wire_golden_v2.rs"]\n'
        "mod wire_golden_v2;\n*/\n"
    )
    _write(repo, "src/network/wire_golden_v2.rs", fake_suite)
    _write(repo, "src/network/codec.rs", fake_wiring)
    assert not HOOK.check_diff(repo)


def test_markers_inside_rust_strings_do_not_count(repo: Path) -> None:
    _write(repo, "src/network/wire_golden_v1.rs", "changed\n")
    _write(repo, "src/lib.rs", "pub const PROTOCOL_VERSION: u8 = 2;\n")
    _write(
        repo,
        "src/network/wire_golden_v2.rs",
        'const FAKE: &str = r#"\nconst WIRE_GOLDEN_VERSION: u8 = 2;\n'
        '#[test]\nfn every_protocol_v2_variant_has_immutable_exact_bytes() {}\n"#;\n',
    )
    _write(
        repo,
        "src/network/codec.rs",
        'const FAKE: &str = "\\n#[path = \\"wire_golden_v2.rs\\"]\\nmod wire_golden_v2;";\n',
    )
    assert not HOOK.check_diff(repo)


def test_markers_inside_raw_byte_strings_do_not_count(repo: Path) -> None:
    _write(repo, "src/network/wire_golden_v1.rs", "changed\n")
    _write(repo, "src/lib.rs", "pub const PROTOCOL_VERSION: u8 = 2;\n")
    _write(
        repo,
        "src/network/wire_golden_v2.rs",
        'const FAKE: &[u8] = br#"\nconst WIRE_GOLDEN_VERSION: u8 = 2;\n'
        '#[test]\nfn every_protocol_v2_variant_has_immutable_exact_bytes() {\n"#;\n',
    )
    _write(repo, "src/network/codec.rs", 'const FAKE: &str = "mod wire_golden_v2;";\n')
    assert not HOOK.check_diff(repo)


def test_character_literals_before_real_markers_are_accepted(repo: Path) -> None:
    _write(repo, "src/network/wire_golden_v1.rs", "changed\n")
    _write(repo, "src/lib.rs", "pub const PROTOCOL_VERSION: u8 = 2;\n")
    _add_v2_suite(repo)
    suite = repo / "src/network/wire_golden_v2.rs"
    suite.write_text("const QUOTE: u8 = b'\\\"';\n" + suite.read_text(encoding="utf-8"), encoding="utf-8")
    assert HOOK.check_diff(repo)


def test_cfg_disabled_module_or_test_does_not_excuse_change(repo: Path) -> None:
    _write(repo, "src/network/wire_golden_v1.rs", "changed\n")
    _write(repo, "src/lib.rs", "pub const PROTOCOL_VERSION: u8 = 2;\n")
    _add_v2_suite(repo)
    codec = repo / "src/network/codec.rs"
    codec.write_text("#[cfg(any())]\n" + codec.read_text(encoding="utf-8"), encoding="utf-8")
    assert not HOOK.check_diff(repo)


@pytest.mark.parametrize("container", ["#[cfg(any())] mod disabled", "macro_rules! disabled"])
def test_nested_registration_does_not_excuse_change(
    repo: Path, container: str
) -> None:
    _write(repo, "src/network/wire_golden_v1.rs", "changed\n")
    _write(repo, "src/lib.rs", "pub const PROTOCOL_VERSION: u8 = 2;\n")
    _add_v2_suite(repo)
    suite = repo / "src/network/wire_golden_v2.rs"
    suite.write_text(
        f"{container} {{\n" + suite.read_text(encoding="utf-8") + "}\n",
        encoding="utf-8",
    )
    codec = repo / "src/network/codec.rs"
    codec.write_text(
        f"{container} {{\n" + codec.read_text(encoding="utf-8") + "}\n",
        encoding="utf-8",
    )
    assert not HOOK.check_diff(repo)


@pytest.mark.parametrize(("opening", "closing"), [("discard!(", ")"), ("discard![", "]")])
def test_macro_invocation_registration_does_not_excuse_change(
    repo: Path, opening: str, closing: str
) -> None:
    _write(repo, "src/network/wire_golden_v1.rs", "changed\n")
    _write(repo, "src/lib.rs", "pub const PROTOCOL_VERSION: u8 = 2;\n")
    _add_v2_suite(repo)
    suite = repo / "src/network/wire_golden_v2.rs"
    suite.write_text(
        f"{opening}\n" + suite.read_text(encoding="utf-8") + f"{closing};\n",
        encoding="utf-8",
    )
    codec = repo / "src/network/codec.rs"
    codec.write_text(
        f"{opening}\n" + codec.read_text(encoding="utf-8") + f"{closing};\n",
        encoding="utf-8",
    )
    assert not HOOK.check_diff(repo)


@pytest.mark.parametrize("inner", ["#![cfg(any())]", "#![cfg_attr(all(), cfg(any()))]"])
def test_file_level_disabling_cfg_does_not_excuse_change(repo: Path, inner: str) -> None:
    _write(repo, "src/network/wire_golden_v1.rs", "changed\n")
    _write(repo, "src/lib.rs", "pub const PROTOCOL_VERSION: u8 = 2;\n")
    _add_v2_suite(repo)
    suite = repo / "src/network/wire_golden_v2.rs"
    suite.write_text(f"{inner}\n" + suite.read_text(encoding="utf-8"), encoding="utf-8")
    assert not HOOK.check_diff(repo)


def test_multiline_file_or_item_cfg_does_not_excuse_change(repo: Path) -> None:
    _write(repo, "src/network/wire_golden_v1.rs", "changed\n")
    _write(repo, "src/lib.rs", "pub const PROTOCOL_VERSION: u8 = 2;\n")
    _add_v2_suite(repo)
    suite = repo / "src/network/wire_golden_v2.rs"
    suite.write_text(
        "#![\n cfg(any())\n]\n" + suite.read_text(encoding="utf-8"),
        encoding="utf-8",
    )
    assert not HOOK.check_diff(repo)
    _add_v2_suite(repo)
    suite.write_text(
        suite.read_text(encoding="utf-8").replace(
            "#[test]", "#[cfg(\n any()\n)]\n#[test]"
        ),
        encoding="utf-8",
    )
    assert not HOOK.check_diff(repo)
    _add_v2_suite(repo)
    suite = repo / "src/network/wire_golden_v2.rs"
    suite.write_text(
        suite.read_text(encoding="utf-8").replace("#[test]", "#[cfg(any())]\n#[test]"),
        encoding="utf-8",
    )
    assert not HOOK.check_diff(repo)


@pytest.mark.parametrize("cached", [False, True])
def test_fixture_type_change_is_protected(
    repo: Path, cached: bool, capsys: pytest.CaptureFixture[str]
) -> None:
    fixture = repo / "src/network/wire_golden_v1.rs"
    fixture.unlink()
    os.symlink("wire_golden_legacy_0_9.rs", fixture)
    if cached:
        _git(repo, "add", "src/network/wire_golden_v1.rs")
    assert not HOOK.check_diff(repo, cached=cached)
    assert "wire_golden_v1.rs" in capsys.readouterr().err


def test_comment_change_is_not_a_protocol_bump(repo: Path) -> None:
    _write(repo, "src/network/wire_golden_v1.rs", "changed\n")
    _write(repo, "src/lib.rs", "// version 2\npub const PROTOCOL_VERSION: u8 = 1;\n")
    assert not HOOK.check_diff(repo)


def test_malformed_candidate_constant_fails_closed(repo: Path, capsys: pytest.CaptureFixture[str]) -> None:
    _write(repo, "src/network/wire_golden_v1.rs", "changed\n")
    _write(repo, "src/lib.rs", "pub const PROTOCOL_VERSION: u8 = next();\n")
    assert not HOOK.check_diff(repo)
    assert "cannot verify" in capsys.readouterr().err


def test_cached_mode_ignores_unstaged_protocol_bump(repo: Path) -> None:
    _write(repo, "src/network/wire_golden_v1.rs", "changed\n")
    _git(repo, "add", "src/network/wire_golden_v1.rs")
    _write(repo, "src/lib.rs", "pub const PROTOCOL_VERSION: u8 = 2;\n")
    assert not HOOK.check_diff(repo, cached=True)


def test_cached_mode_accepts_staged_protocol_bump(repo: Path) -> None:
    _write(repo, "src/network/wire_golden_v1.rs", "changed\n")
    _write(repo, "src/lib.rs", "pub const PROTOCOL_VERSION: u8 = 2;\n")
    _add_v2_suite(repo)
    _git(repo, "add", "src/network/wire_golden_v1.rs", "src/network/wire_golden_v2.rs", "src/lib.rs")
    _git(repo, "add", "src/network/codec.rs")
    assert HOOK.check_diff(repo, cached=True)


def test_local_mode_catches_staged_rewrite_restored_in_worktree(repo: Path) -> None:
    path = "src/network/wire_golden_v1.rs"
    _write(repo, path, "staged rewrite\n")
    _git(repo, "add", path)
    _git(repo, "restore", "--source=HEAD", "--worktree", path)

    assert HOOK.check_diff(repo), "worktree-only view reproduces the historical blind spot"
    assert not HOOK.check_local(repo)


def test_multiple_failures_are_sorted(repo: Path, capsys: pytest.CaptureFixture[str]) -> None:
    _write(repo, "src/network/wire_golden_v1.rs", "changed\n")
    _write(repo, "src/network/wire_golden_legacy_0_9.rs", "changed\n")
    assert not HOOK.check_diff(repo)
    lines = [line for line in capsys.readouterr().err.splitlines() if line.startswith("src/")]
    assert lines == sorted(lines)


def test_committed_base_ref_mode_catches_pr_rewrite(repo: Path) -> None:
    base = subprocess.run(
        ["git", "-C", str(repo), "rev-parse", "HEAD"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    _write(repo, "src/network/wire_golden_v1.rs", "changed\n")
    _git(repo, "add", ".")
    _git(repo, "commit", "-qm", "rewrite")
    assert not HOOK.check_diff(repo, base_ref=base)


def test_committed_base_ref_mode_accepts_bump_with_matching_suite(repo: Path) -> None:
    base = subprocess.run(
        ["git", "-C", str(repo), "rev-parse", "HEAD"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    _write(repo, "src/network/wire_golden_v1.rs", "changed\n")
    _write(repo, "src/lib.rs", "pub const PROTOCOL_VERSION: u8 = 2;\n")
    _add_v2_suite(repo)
    _git(repo, "add", ".")
    _git(repo, "commit", "-qm", "protocol v2")
    assert HOOK.check_diff(repo, base_ref=base)
