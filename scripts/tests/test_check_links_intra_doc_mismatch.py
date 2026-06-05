#!/usr/bin/env python3
"""Unit tests for the intra-doc link text-vs-target mismatch check.

These tests cover the defect class where backticked rustdoc link text names a
specific item but a crate-internal intra-doc target points at a different item
(usually the enclosing module), so the rendered link resolves but lands on the
wrong page. The checker lives in ``scripts/docs/check-links.py``.

All fixtures are inline strings so the tests never depend on the live source
tree (which is already fixed).
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

scripts_dir = Path(__file__).parent.parent
sys.path.insert(0, str(scripts_dir / "docs"))

spec = importlib.util.spec_from_file_location(
    "check_links", scripts_dir / "docs" / "check-links.py"
)
check_links = importlib.util.module_from_spec(spec)
spec.loader.exec_module(check_links)

backticked_item_text = check_links.backticked_item_text
intra_doc_target_item = check_links.intra_doc_target_item
intra_doc_link_text_mismatch = check_links.intra_doc_link_text_mismatch
check_rust_doc_link = check_links.check_rust_doc_link
check_rust_doc_file = check_links.check_rust_doc_file


class TestBacktickedItemText:
    """Tests for extracting the named item from backticked link text."""

    @pytest.mark.parametrize(
        ("text", "expected"),
        [
            ("`Foo`", "Foo"),
            ("`Foo::bar`", "bar"),
            ("`a::b::c::deep`", "deep"),
            ("`decode_message()`", "decode_message"),
            ("`ProofVec<U>`", "ProofVec"),
            ("`Vec<T>::iter`", "iter"),
            ("`MAX_BOUNDED_DECODE_LEN`", "MAX_BOUNDED_DECODE_LEN"),
            ("`_private`", "_private"),
            ("  `Spaced`  ", "Spaced"),
        ],
    )
    def test_single_backticked_identifier_is_extracted(
        self, text: str, expected: str
    ) -> None:
        """A single backticked identifier or path yields its final segment."""
        assert backticked_item_text(text) == expected

    @pytest.mark.parametrize(
        "text",
        [
            "prose link text",
            "see the docs",
            "`Foo` and `Bar`",
            "text with `inline` code",
            "``",
            "` `",
            "`123abc`",
            "`has space`",
            "`weird-name`",
            "`name!`",
            "",
        ],
    )
    def test_non_single_identifier_text_returns_none(self, text: str) -> None:
        """Prose, multi-token, or non-identifier text is not an item name."""
        assert backticked_item_text(text) is None


class TestIntraDocTargetItem:
    """Tests for extracting the final segment of crate-internal targets."""

    @pytest.mark.parametrize(
        ("target", "expected"),
        [
            ("crate::module", "module"),
            ("crate::network::codec", "codec"),
            ("super::other_mod", "other_mod"),
            ("self::sibling", "sibling"),
            ("crate::m::func()", "func"),
            # Generic args are stripped, symmetric with the link-text side.
            ("crate::m::Foo<T>", "Foo"),
            ("crate::collections::Map<K, V>", "Map"),
        ],
    )
    def test_crate_internal_targets_yield_final_segment(
        self, target: str, expected: str
    ) -> None:
        """crate::/super::/self:: targets expose their final path segment."""
        assert intra_doc_target_item(target) == expected

    @pytest.mark.parametrize(
        "target",
        [
            "Self::method",
            "bincode::config::Configuration::with_limit",
            "other_mod::bar",
            "self#anchor",
            "crate::",
            "#anchor",
            "https://example.com",
        ],
    )
    def test_non_crate_internal_targets_return_none(self, target: str) -> None:
        """External, Self::, anchor-only, and relative targets are out of scope."""
        assert intra_doc_target_item(target) is None


class TestIntraDocLinkTextMismatch:
    """Tests for the combined text-vs-target mismatch predicate."""

    # TRUE positives: text names an item the crate-internal target does not.
    @pytest.mark.parametrize(
        ("text", "target", "expected"),
        [
            ("`Foo`", "crate::module", ("Foo", "module")),
            (
                "`MAX_BOUNDED_DECODE_LEN`",
                "crate::network::codec",
                ("MAX_BOUNDED_DECODE_LEN", "codec"),
            ),
            ("`func`", "super::other_mod", ("func", "other_mod")),
            ("`thing`", "self::other", ("thing", "other")),
            (
                "`decode_bounded`",
                "crate::network::codec",
                ("decode_bounded", "codec"),
            ),
            # Generic args on the TARGET are stripped before comparing, so a
            # genuine mismatch is still caught (regression guard for the
            # previously-asymmetric target parser).
            ("`Bar`", "crate::m::Foo<T>", ("Bar", "Foo")),
        ],
    )
    def test_true_positive_mismatches_are_flagged(
        self, text: str, target: str, expected: tuple[str, str]
    ) -> None:
        """Backticked item text pointing at a different page is reported."""
        assert intra_doc_link_text_mismatch(text, target) == expected

    # TRUE negatives: nothing to flag.
    @pytest.mark.parametrize(
        ("text", "target"),
        [
            ("`codec`", "crate::network::codec"),  # text names the module
            ("`Foo`", "crate::module::Foo"),  # correct direct link
            ("`x`", "crate::m::x"),  # text == final target segment
            (
                "`Limit`",
                "bincode::config::Configuration::with_limit",
            ),  # external crate
            ("`bar`", "Self::bar"),  # Self:: excluded
            (
                "`decode_message()`",
                "crate::network::codec::decode_message",
            ),  # call parens stripped, match
            (
                "`ProofVec<U>`",
                "crate::proof_vec::ProofVec",
            ),  # generic args stripped, match
            (
                "`Foo`",
                "crate::m::Foo<T>",
            ),  # generic args on target stripped, match
            ("prose link text", "crate::module"),  # not a backticked identifier
            ("`Foo` and `Bar`", "crate::module"),  # multi-token text
            ("`thing`", "self#anchor"),  # same-page anchor (no :: path)
            ("`bar`", "other_mod::bar"),  # bare relative path, not crate-internal
            (None, "crate::module"),  # link form without inline text
        ],
    )
    def test_true_negatives_are_not_flagged(self, text: str, target: str) -> None:
        """Correct links, external crates, and exempt forms are left alone."""
        assert intra_doc_link_text_mismatch(text, target) is None


class TestCheckRustDocLinkIntegration:
    """End-to-end checks through check_rust_doc_link / check_rust_doc_file."""

    @pytest.mark.parametrize(
        "target",
        [
            "crate::module",
            "super::other_mod",
            "self::other",
        ],
    )
    def test_check_rust_doc_link_rejects_mismatch(
        self, tmp_path: Path, target: str
    ) -> None:
        """A mismatched intra-doc link fails closed with a fix hint."""
        rust_file = tmp_path / "lib.rs"
        rust_file.write_text("struct S {}\n", encoding="utf-8")

        is_valid, error_msg = check_rust_doc_link(
            rust_file, target, tmp_path, link_text="`Widget`"
        )

        assert not is_valid
        assert "Widget" in error_msg
        assert "wrong page" in error_msg
        assert "private_intra_doc_links" in error_msg

    def test_check_rust_doc_link_accepts_matching_target(
        self, tmp_path: Path
    ) -> None:
        """A correct direct link passes (text == final target segment)."""
        rust_file = tmp_path / "lib.rs"
        rust_file.write_text("struct S {}\n", encoding="utf-8")

        is_valid, error_msg = check_rust_doc_link(
            rust_file, "crate::module::Widget", tmp_path, link_text="`Widget`"
        )

        assert is_valid, error_msg

    @pytest.mark.parametrize(
        ("content", "expected_errors"),
        [
            # TRUE positive: constant text, module target.
            (
                "/// See [`MAX_BOUNDED_DECODE_LEN`](crate::network::codec).\n"
                "fn f() {}\n",
                1,
            ),
            # TRUE positive: super:: module mismatch.
            (
                "/// See [`helper`](super::utils).\nfn f() {}\n",
                1,
            ),
            # TRUE positive: self:: mismatch.
            (
                "//! See [`Thing`](self::other_mod).\n",
                1,
            ),
            # Negative: text names the module.
            (
                "/// See [`codec`](crate::network::codec).\nfn f() {}\n",
                0,
            ),
            # Negative: correct direct item link.
            (
                "/// See [`Widget`](crate::ui::Widget).\nfn f() {}\n",
                0,
            ),
            # Negative: external-crate target.
            (
                "/// See "
                "[`Limit`](bincode::config::Configuration::with_limit).\n"
                "fn f() {}\n",
                0,
            ),
            # Negative: relative Self:: link.
            (
                "/// See [`bar`](Self::bar).\nfn f() {}\n",
                0,
            ),
            # Negative: same-page anchor.
            (
                "//! # Section\n/// See [`section`](self#section).\nfn f() {}\n",
                0,
            ),
            # Negative: text == final target segment.
            (
                "/// See [`x`](crate::m::x).\nfn f() {}\n",
                0,
            ),
            # Negative: prose link text (not a backticked identifier).
            (
                "/// See [the codec module](crate::network::codec).\nfn f() {}\n",
                0,
            ),
            # Negative: call-paren and generic args still match the item.
            (
                "/// See [`decode_message()`]"
                "(crate::network::codec::decode_message).\nfn f() {}\n",
                0,
            ),
        ],
    )
    def test_check_rust_doc_file_mismatch_matrix(
        self, tmp_path: Path, content: str, expected_errors: int
    ) -> None:
        """The file-level checker flags only genuine text-vs-target mismatches."""
        rust_file = tmp_path / "lib.rs"
        rust_file.write_text(content, encoding="utf-8")

        result = check_rust_doc_file(rust_file, tmp_path)

        assert result.errors == expected_errors

    def test_mismatch_inside_code_span_is_ignored(self, tmp_path: Path) -> None:
        """A mismatched link inside inline code is not a rendered link."""
        rust_file = tmp_path / "lib.rs"
        rust_file.write_text(
            "/// Inline `[`Foo`](crate::module)` is ignored.\n"
            "/// # Heading\n"
            "fn f() {}\n",
            encoding="utf-8",
        )

        result = check_rust_doc_file(rust_file, tmp_path)

        assert result.errors == 0


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
