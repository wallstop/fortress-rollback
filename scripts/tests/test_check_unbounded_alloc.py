#!/usr/bin/env python3
"""Unit tests for scripts/hooks/check-unbounded-alloc.py.

These lock in the structural cfg(test)/cfg(kani) exclusion (so production code
appearing AFTER an early test/proof item is still scanned), the detected
constructs (with_capacity/reserve/reserve_exact/resize/resize_with/vec! macro),
the literal/.len()/.count() exemptions, the `// alloc-bound:` marker, and the
comment/raw-string robustness.

Mirrors the runner/structure of test_enable_dependabot_automerge.py and the
hyphenated-hook import pattern of test_check_changelog_unreleased.py.
"""
from __future__ import annotations

import importlib.util
import subprocess
import sys
from pathlib import Path

import pytest

# Import the hook module (hyphenated filename requires importlib).
SCRIPTS_DIR = Path(__file__).parent.parent
HOOK_PATH = SCRIPTS_DIR / "hooks" / "check-unbounded-alloc.py"
REPO_ROOT = SCRIPTS_DIR.parent

spec = importlib.util.spec_from_file_location(
    "check_unbounded_alloc",
    HOOK_PATH,
)
assert spec is not None and spec.loader is not None
check_unbounded_alloc = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = check_unbounded_alloc
spec.loader.exec_module(check_unbounded_alloc)

check_file = check_unbounded_alloc.check_file


def _write_rs(tmp_path: Path, body: str) -> Path:
    """Write `body` to a .rs file and return its path."""
    path = tmp_path / "sample.rs"
    path.write_text(body, encoding="utf-8")
    return path


def _is_flagged(tmp_path: Path, body: str) -> bool:
    """Return True if check_file reports at least one finding for `body`."""
    return bool(check_file(_write_rs(tmp_path, body)))


# ---------------------------------------------------------------------------
# Structural cfg(test)/cfg(kani) exclusion (MAJOR-1)
# ---------------------------------------------------------------------------


def test_early_cfg_test_const_then_production_alloc_is_flagged(tmp_path: Path) -> None:
    """A production alloc AFTER an early `#[cfg(test)] const` must still be flagged.

    This is the core MAJOR-1 regression: a one-shot cutoff would hide everything
    after the early test item; structural exclusion must skip ONLY the const.
    """
    body = (
        "#[cfg(test)]\n"
        "const SAMPLE: usize = 5;\n"
        "\n"
        "fn make(n: usize) -> Vec<u8> {\n"
        "    let mut v = Vec::with_capacity(n);\n"
        "    v\n"
        "}\n"
    )
    findings = check_file(_write_rs(tmp_path, body))
    assert findings, f"production with_capacity(n) must be flagged, got: {findings}"
    assert any("with_capacity" in f for f in findings)


def test_early_cfg_kani_item_then_production_alloc_is_flagged(tmp_path: Path) -> None:
    """A production alloc AFTER an early `#[cfg(kani)]` const must still be flagged."""
    body = (
        "#[cfg(kani)]\n"
        "pub const SMALL: usize = 8;\n"
        "\n"
        "fn make(n: usize) -> Vec<u8> {\n"
        "    vec![0u8; n]\n"
        "}\n"
    )
    findings = check_file(_write_rs(tmp_path, body))
    assert findings, f"production vec![0; n] must be flagged, got: {findings}"


def test_trailing_cfg_test_mod_alloc_is_not_flagged(tmp_path: Path) -> None:
    """A `with_capacity(var)` inside a trailing `#[cfg(test)] mod tests` is excluded."""
    body = (
        "fn prod() {}\n"
        "\n"
        "#[cfg(test)]\n"
        "mod tests {\n"
        "    fn t(n: usize) {\n"
        "        let _ = Vec::<u8>::with_capacity(n);\n"
        "    }\n"
        "}\n"
    )
    assert not _is_flagged(tmp_path, body)


def test_indented_cfg_test_mod_alloc_is_not_flagged(tmp_path: Path) -> None:
    """An indented/nested `#[cfg(test)] mod tests` block is excluded."""
    body = (
        "mod outer {\n"
        "    fn prod() {}\n"
        "\n"
        "    #[cfg(test)]\n"
        "    mod tests {\n"
        "        fn t(n: usize) {\n"
        "            let _ = vec![0u8; n];\n"
        "        }\n"
        "    }\n"
        "}\n"
    )
    assert not _is_flagged(tmp_path, body)


def test_cfg_all_test_feature_gated_item_is_not_flagged(tmp_path: Path) -> None:
    """`#[cfg(all(test, feature = \"x\"))]` gated items are excluded."""
    body = (
        '#[cfg(all(test, feature = "x"))]\n'
        "mod gated {\n"
        "    fn t(n: usize) {\n"
        "        let _ = Vec::<u8>::with_capacity(n);\n"
        "    }\n"
        "}\n"
    )
    assert not _is_flagged(tmp_path, body)


def test_cfg_any_test_gated_item_is_not_flagged(tmp_path: Path) -> None:
    """`#[cfg(any(test, foo))]` gated items are excluded."""
    body = (
        "#[cfg(any(test, foo))]\n"
        "fn t(n: usize) {\n"
        "    let _ = vec![0u8; n];\n"
        "}\n"
    )
    assert not _is_flagged(tmp_path, body)


def test_cfg_kani_gated_item_is_not_flagged(tmp_path: Path) -> None:
    """`#[cfg(kani)]` gated items are excluded."""
    body = (
        "#[cfg(kani)]\n"
        "mod kani_proofs {\n"
        "    fn p(n: usize) {\n"
        "        let _ = vec![0u8; n];\n"
        "    }\n"
        "}\n"
    )
    assert not _is_flagged(tmp_path, body)


def test_multiple_test_regions_with_interleaved_production(tmp_path: Path) -> None:
    """Multiple test regions in one file; production between/after must be scanned."""
    body = (
        "#[cfg(test)]\n"
        "const A: usize = 1;\n"
        "\n"
        "fn prod_one(n: usize) {\n"
        "    let _ = vec![0u8; n];\n"  # flagged
        "}\n"
        "\n"
        "#[cfg(test)]\n"
        "mod tests_a {\n"
        "    fn t(n: usize) {\n"
        "        let _ = Vec::<u8>::with_capacity(n);\n"  # excluded
        "    }\n"
        "}\n"
        "\n"
        "fn prod_two(n: usize) {\n"
        "    let _ = Vec::<u8>::with_capacity(n);\n"  # flagged
        "}\n"
    )
    findings = check_file(_write_rs(tmp_path, body))
    flagged_lines = {int(f.split(":")[1]) for f in findings}
    # vec![0u8; n] in prod_one is line 5; with_capacity in prod_two is line 16.
    assert 5 in flagged_lines, f"prod_one alloc must be flagged: {findings}"
    assert 16 in flagged_lines, f"prod_two alloc must be flagged: {findings}"
    assert len(flagged_lines) == 2, f"only the two production allocs: {findings}"


# ---------------------------------------------------------------------------
# Same-line cfg(test)/cfg(kani) attribute + item (BUG 1)
#
# When the attribute AND the item it gates are on the SAME line, only the gated
# item's span must be excluded -- production code on the NEXT line must still be
# scanned (the old code wrongly excluded the next line).
# ---------------------------------------------------------------------------


def test_same_line_cfg_test_const_then_production_alloc_is_flagged(
    tmp_path: Path,
) -> None:
    """`#[cfg(test)] const X = 5;` then a production alloc on the next line."""
    body = (
        "#[cfg(test)] const X: usize = 5;\n"
        "fn prod(n: usize) { let _ = vec![0u8; n]; }\n"
    )
    findings = check_file(_write_rs(tmp_path, body))
    flagged_lines = {int(f.split(":")[1]) for f in findings}
    assert flagged_lines == {2}, f"only the line-2 production alloc: {findings}"


def test_same_line_cfg_test_fn_then_production_alloc_is_flagged(
    tmp_path: Path,
) -> None:
    """`#[cfg(test)] fn h() {}` then a production alloc on the next line."""
    body = (
        "#[cfg(test)] fn h() {}\n"
        "fn prod(n: usize) { let _ = Vec::<u8>::with_capacity(n); }\n"
    )
    findings = check_file(_write_rs(tmp_path, body))
    flagged_lines = {int(f.split(":")[1]) for f in findings}
    assert flagged_lines == {2}, f"only the line-2 production alloc: {findings}"


def test_two_adjacent_same_line_cfg_consts_then_production_alloc_is_flagged(
    tmp_path: Path,
) -> None:
    """Two adjacent single-line `#[cfg(test)] const` lines then a production alloc."""
    body = (
        "#[cfg(test)] const A: usize = 1;\n"
        "#[cfg(test)] const B: usize = 2;\n"
        "fn prod(n: usize) { let _ = vec![0u8; n]; }\n"
    )
    findings = check_file(_write_rs(tmp_path, body))
    flagged_lines = {int(f.split(":")[1]) for f in findings}
    assert flagged_lines == {3}, f"only the line-3 production alloc: {findings}"


def test_stacked_same_line_cfg_attrs_then_production_alloc_is_flagged(
    tmp_path: Path,
) -> None:
    """Stacked same-line attributes `#[cfg(test)] #[allow(dead_code)] fn f() {}`."""
    body = (
        "#[cfg(test)] #[allow(dead_code)] fn f() {}\n"
        "fn prod(n: usize) { let _ = vec![0u8; n]; }\n"
    )
    findings = check_file(_write_rs(tmp_path, body))
    flagged_lines = {int(f.split(":")[1]) for f in findings}
    assert flagged_lines == {2}, f"only the line-2 production alloc: {findings}"


def test_same_line_cfg_test_mod_block_excludes_inner_allocs(tmp_path: Path) -> None:
    """`#[cfg(test)] mod tests {` (attr + `{` on one line) excludes inner allocs."""
    body = (
        "fn prod() {}\n"
        "#[cfg(test)] mod tests {\n"
        "    fn t(n: usize) {\n"
        "        let _ = vec![0u8; n];\n"
        "    }\n"
        "}\n"
    )
    assert not _is_flagged(tmp_path, body)


# ---------------------------------------------------------------------------
# Marker handling
# ---------------------------------------------------------------------------


def test_vec_macro_var_size_is_flagged(tmp_path: Path) -> None:
    body = "fn f(n: usize) { let _ = vec![0u8; n]; }\n"
    assert _is_flagged(tmp_path, body)


def test_vec_macro_with_same_line_marker_is_not_flagged(tmp_path: Path) -> None:
    body = "fn f(n: usize) { let _ = vec![0u8; n]; } // alloc-bound: n is local\n"
    assert not _is_flagged(tmp_path, body)


def test_vec_macro_with_prior_line_marker_is_not_flagged(tmp_path: Path) -> None:
    body = (
        "fn f(n: usize) {\n"
        "    // alloc-bound: n is a trusted local config value\n"
        "    let _ = vec![0u8; n];\n"
        "}\n"
    )
    assert not _is_flagged(tmp_path, body)


# ---------------------------------------------------------------------------
# Exemptions
# ---------------------------------------------------------------------------


def test_integer_literal_size_is_exempt(tmp_path: Path) -> None:
    body = (
        "fn f() {\n"
        "    let _ = Vec::<u8>::with_capacity(16);\n"
        "    let _ = vec![0u8; 1_024];\n"
        "    let _ = vec![0u8; 0usize];\n"
        "}\n"
    )
    assert not _is_flagged(tmp_path, body)


def test_len_call_size_is_exempt(tmp_path: Path) -> None:
    body = "fn f(s: &[u8]) { let _ = Vec::<u8>::with_capacity(s.len()); }\n"
    assert not _is_flagged(tmp_path, body)


def test_count_call_size_is_exempt(tmp_path: Path) -> None:
    body = "fn f(s: &[u8]) { let _ = Vec::<u8>::with_capacity(s.iter().count()); }\n"
    assert not _is_flagged(tmp_path, body)


# ---------------------------------------------------------------------------
# Arithmetic over a `.len()`/`.count()` term still needs a marker (BUG 2)
# ---------------------------------------------------------------------------


def test_len_arithmetic_addition_is_flagged(tmp_path: Path) -> None:
    """`with_capacity(x + y.len())` is arithmetic over a term -> needs a marker."""
    body = "fn f(x: usize, y: &[u8]) { let _ = Vec::<u8>::with_capacity(x + y.len()); }\n"
    assert _is_flagged(tmp_path, body)


def test_len_arithmetic_multiplication_is_flagged(tmp_path: Path) -> None:
    """`with_capacity(a.len() * 2)` is arithmetic -> needs a marker."""
    body = "fn f(a: &[u8]) { let _ = Vec::<u8>::with_capacity(a.len() * 2); }\n"
    assert _is_flagged(tmp_path, body)


def test_two_len_terms_added_is_flagged(tmp_path: Path) -> None:
    """`with_capacity(a.len() + b.len())` is arithmetic -> needs a marker."""
    body = (
        "fn f(a: &[u8], b: &[u8]) {\n"
        "    let _ = Vec::<u8>::with_capacity(a.len() + b.len());\n"
        "}\n"
    )
    assert _is_flagged(tmp_path, body)


def test_single_len_chain_is_exempt(tmp_path: Path) -> None:
    """A single receiver chain ending in `.len()` is still exempt."""
    body = "fn f(foo: &[u8]) { let _ = Vec::<u8>::with_capacity(foo.len()); }\n"
    assert not _is_flagged(tmp_path, body)


def test_nested_field_len_chain_is_exempt(tmp_path: Path) -> None:
    """`self.bar.baz.len()` is a single chain -> exempt."""
    body = "fn f(&self) { let _ = Vec::<u8>::with_capacity(self.bar.baz.len()); }\n"
    assert not _is_flagged(tmp_path, body)


def test_single_count_chain_is_exempt(tmp_path: Path) -> None:
    """`it.count()` is a single chain -> exempt."""
    body = "fn f(it: impl Iterator) { let _ = Vec::<u8>::with_capacity(it.count()); }\n"
    assert not _is_flagged(tmp_path, body)


# ---------------------------------------------------------------------------
# Fallible reservations (try_reserve / try_reserve_exact) are NOT flagged
# ---------------------------------------------------------------------------


def test_try_reserve_var_is_not_flagged(tmp_path: Path) -> None:
    body = "fn f(v: &mut Vec<u8>, n: usize) { let _ = v.try_reserve(n); }\n"
    assert not _is_flagged(tmp_path, body)


def test_try_reserve_exact_var_is_not_flagged(tmp_path: Path) -> None:
    body = "fn f(v: &mut Vec<u8>, n: usize) { let _ = v.try_reserve_exact(n); }\n"
    assert not _is_flagged(tmp_path, body)


def test_reserve_var_is_flagged(tmp_path: Path) -> None:
    body = "fn f(v: &mut Vec<u8>, n: usize) { v.reserve(n); }\n"
    assert _is_flagged(tmp_path, body)


def test_reserve_exact_var_is_flagged(tmp_path: Path) -> None:
    body = "fn f(v: &mut Vec<u8>, n: usize) { v.reserve_exact(n); }\n"
    assert _is_flagged(tmp_path, body)


# ---------------------------------------------------------------------------
# resize / resize_with detection (size is the FIRST argument)
# ---------------------------------------------------------------------------


def test_resize_var_size_is_flagged(tmp_path: Path) -> None:
    body = "fn f(v: &mut Vec<u8>, n: usize) { v.resize(n, 0); }\n"
    assert _is_flagged(tmp_path, body)


def test_resize_with_var_size_is_flagged(tmp_path: Path) -> None:
    body = "fn f(v: &mut Vec<u8>, n: usize) { v.resize_with(n, || 0); }\n"
    assert _is_flagged(tmp_path, body)


def test_resize_literal_size_is_exempt(tmp_path: Path) -> None:
    body = "fn f(v: &mut Vec<u8>) { v.resize(16, 0); }\n"
    assert not _is_flagged(tmp_path, body)


def test_resize_len_size_is_exempt(tmp_path: Path) -> None:
    body = "fn f(v: &mut Vec<u8>, s: &[u8]) { v.resize(s.len(), 0); }\n"
    assert not _is_flagged(tmp_path, body)


def test_resize_with_marker_is_not_flagged(tmp_path: Path) -> None:
    body = (
        "fn f(v: &mut Vec<u8>, n: usize) {\n"
        "    // alloc-bound: n is trusted local config\n"
        "    v.resize(n, 0);\n"
        "}\n"
    )
    assert not _is_flagged(tmp_path, body)


# ---------------------------------------------------------------------------
# Comment / raw-string robustness
# ---------------------------------------------------------------------------


def test_line_comment_alloc_is_not_flagged(tmp_path: Path) -> None:
    body = "fn f(n: usize) { /* nothing */ let _ = 1; } // let _ = vec![0u8; n];\n"
    assert not _is_flagged(tmp_path, body)


def test_block_comment_single_line_alloc_is_not_flagged(tmp_path: Path) -> None:
    body = "fn f(n: usize) { /* let _ = vec![0u8; n]; */ }\n"
    assert not _is_flagged(tmp_path, body)


def test_block_comment_multi_line_alloc_is_not_flagged(tmp_path: Path) -> None:
    body = (
        "fn f(n: usize) {\n"
        "    /* a multi-line comment\n"
        "       let _ = Vec::<u8>::with_capacity(n);\n"
        "       still inside */\n"
        "    let _ = 1;\n"
        "}\n"
    )
    assert not _is_flagged(tmp_path, body)


def test_raw_string_alloc_is_not_flagged(tmp_path: Path) -> None:
    body = 'fn f(n: usize) { let _ = r"let _ = vec![0u8; n];"; }\n'
    assert not _is_flagged(tmp_path, body)


def test_raw_string_hashed_alloc_is_not_flagged(tmp_path: Path) -> None:
    body = 'fn f(n: usize) { let _ = r#"vec![0u8; n] and "quotes""#; }\n'
    assert not _is_flagged(tmp_path, body)


def test_multi_line_raw_string_alloc_is_not_flagged(tmp_path: Path) -> None:
    body = (
        "fn f(n: usize) {\n"
        '    let _ = r#"\n'
        "        let v = Vec::<u8>::with_capacity(n);\n"
        '    "#;\n'
        "}\n"
    )
    assert not _is_flagged(tmp_path, body)


# ---------------------------------------------------------------------------
# Ordinary / byte string literal contents are stripped (BUG 3)
# ---------------------------------------------------------------------------


def test_ordinary_string_vec_macro_token_is_not_flagged(tmp_path: Path) -> None:
    """A `vec![...; n]` token inside an ordinary string literal is not flagged."""
    body = 'fn f() { let s = "vec![0u8; n]"; let _ = s; }\n'
    assert not _is_flagged(tmp_path, body)


def test_ordinary_string_with_capacity_token_is_not_flagged(tmp_path: Path) -> None:
    """A `with_capacity(n)` token inside an ordinary string literal is not flagged."""
    body = 'fn f() { let s = "with_capacity(n)"; let _ = s; }\n'
    assert not _is_flagged(tmp_path, body)


def test_byte_string_alloc_token_is_not_flagged(tmp_path: Path) -> None:
    """A construct token inside a byte string `b"..."` is not flagged."""
    body = 'fn f() { let s = b"with_capacity(x)"; let _ = s; }\n'
    assert not _is_flagged(tmp_path, body)


def test_string_token_does_not_corrupt_offsets_real_alloc_still_flagged(
    tmp_path: Path,
) -> None:
    """Blanking string contents preserves line offsets: a real alloc still flags."""
    body = (
        "fn f(n: usize) {\n"
        '    let s = "with_capacity(n)";\n'
        "    let _ = Vec::<u8>::with_capacity(n);\n"
        "    let _ = s;\n"
        "}\n"
    )
    findings = check_file(_write_rs(tmp_path, body))
    flagged_lines = {int(f.split(":")[1]) for f in findings}
    assert flagged_lines == {3}, f"only the real line-3 alloc: {findings}"


# ---------------------------------------------------------------------------
# Multi-line / nested-bracket vec! macros
# ---------------------------------------------------------------------------


def test_multi_line_vec_macro_var_size_is_flagged(tmp_path: Path) -> None:
    body = (
        "fn f(n: usize) {\n"
        "    let _ = vec![\n"
        "        0u8;\n"
        "        n\n"
        "    ];\n"
        "}\n"
    )
    assert _is_flagged(tmp_path, body)


def test_vec_macro_nested_bracket_size_is_flagged(tmp_path: Path) -> None:
    body = "fn f(sizes: &[usize]) { let _ = vec![0u8; sizes[0]]; }\n"
    assert _is_flagged(tmp_path, body)


def test_vec_macro_list_form_is_not_flagged(tmp_path: Path) -> None:
    body = "fn f(a: u8, b: u8) { let _ = vec![a, b, a]; }\n"
    assert not _is_flagged(tmp_path, body)


# ---------------------------------------------------------------------------
# End-to-end: subprocess over the real src/ tree and a planted violation
# ---------------------------------------------------------------------------


def _run_hook(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(HOOK_PATH), *args],
        check=False,
        capture_output=True,
        text=True,
        cwd=str(REPO_ROOT),
    )


def test_clean_src_tree_passes() -> None:
    """The real src/ tree must be clean (exit 0) after annotations are added."""
    result = _run_hook()
    assert result.returncode == 0, (
        f"src/ tree should pass; stderr=\n{result.stderr}"
    )


def test_planted_production_violation_is_caught(tmp_path: Path) -> None:
    """A planted unjustified production alloc exits 1 via the CLI path."""
    planted = tmp_path / "planted.rs"
    planted.write_text(
        "fn boom(n: usize) -> Vec<u8> { Vec::with_capacity(n) }\n",
        encoding="utf-8",
    )
    # Pass an explicit path; the hook only scans files under src/, so to exercise
    # the CLI we instead call check_file directly here for the temp file, and
    # exercise the CLI exit code with the src/ tree clean check above.
    findings = check_file(planted)
    assert findings, "planted production with_capacity(n) must be flagged"


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-v"]))
