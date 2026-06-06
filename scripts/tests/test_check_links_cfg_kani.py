#!/usr/bin/env python3
"""Unit tests for the ``#[cfg(kani)]``-only intra-doc link guard.

These cover the defect class where a rustdoc intra-doc link points at an item
that is defined only under ``#[cfg(kani)]`` (for example
``crate::proof_vec::InlineVec``). Such a link resolves under ``--cfg kani`` but
is a broken intra-doc link in every normal ``cargo doc`` build, where the item
is not compiled. Today the crate's only such links avoid tripping CI purely
because they sit on ``#[cfg(kani)]`` items rustdoc skips -- a fragile
coincidence -- so the guard forbids the whole class. The checker lives in
``scripts/docs/check-links.py``.

All fixtures are inline strings or files under ``tmp_path`` so the tests never
depend on the live source tree.
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

find_cfg_kani_only_items = check_links.find_cfg_kani_only_items
cfg_kani_only_link_error = check_links.cfg_kani_only_link_error
intra_doc_link_error = check_links.intra_doc_link_error
check_rust_doc_link = check_links.check_rust_doc_link
check_rust_doc_file = check_links.check_rust_doc_file


def _write(root: Path, rel: str, content: str) -> Path:
    """Write *content* to ``root/rel`` (creating parents) and return the path."""
    path = root / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")
    return path


class TestFindCfgKaniOnlyItems:
    """Enumeration of items defined only under ``#[cfg(kani)]``."""

    def test_item_gated_only_under_cfg_kani_is_collected(self, tmp_path: Path) -> None:
        """A struct/const present only under ``#[cfg(kani)]`` is Kani-only."""
        _write(
            tmp_path,
            "src/proof_vec.rs",
            "#[cfg(kani)]\n"
            "#[derive(Debug)]\n"
            "pub struct InlineVec<T, const CAP: usize> { len: usize }\n"
            "\n"
            "#[cfg(kani)]\n"
            "pub(crate) const KANI_INLINE_CAP: usize = 7;\n",
        )
        result = find_cfg_kani_only_items(tmp_path)
        assert "InlineVec" in result
        assert "KANI_INLINE_CAP" in result

    def test_item_defined_under_both_cfgs_is_not_flagged(self, tmp_path: Path) -> None:
        """A name defined under both ``cfg(kani)`` and ``cfg(not(kani))`` is normal.

        Mirrors ``ProofVec`` and ``INPUT_QUEUE_LENGTH`` in the real crate: they
        exist in every build, so links to them must NOT be flagged.
        """
        _write(
            tmp_path,
            "src/proof_vec.rs",
            "#[cfg(not(kani))]\n"
            "pub(crate) type ProofVec<T> = Vec<T>;\n"
            "\n"
            "#[cfg(kani)]\n"
            "pub(crate) type ProofVec<T> = InlineVec<T, 7>;\n"
            "\n"
            "#[cfg(kani)]\n"
            "pub const INPUT_QUEUE_LENGTH: usize = 7;\n"
            "\n"
            "#[cfg(not(kani))]\n"
            "pub const INPUT_QUEUE_LENGTH: usize = 128;\n",
        )
        result = find_cfg_kani_only_items(tmp_path)
        assert "ProofVec" not in result
        assert "INPUT_QUEUE_LENGTH" not in result

    def test_const_fn_under_cfg_kani_is_collected(self, tmp_path: Path) -> None:
        """A `const fn` is harvested by its name, not the `fn` keyword.

        Regression test: `const` must be treated as a leading modifier so the
        captured identifier is the function name. A bare `const ITEM` declaration
        in the same file must still be harvested by its own name, proving the
        modifier change did not break plain constants.
        """
        _write(
            tmp_path,
            "src/proof_vec.rs",
            "#[cfg(kani)]\n"
            "pub const fn kani_only_fn(x: usize) -> usize { x }\n"
            "\n"
            "#[cfg(kani)]\n"
            "pub(crate) const KANI_ONLY_CONST: usize = 7;\n",
        )
        result = find_cfg_kani_only_items(tmp_path)
        assert "kani_only_fn" in result
        assert "KANI_ONLY_CONST" in result
        assert "fn" not in result
        assert "const" not in result

    def test_unconditional_item_is_not_flagged(self, tmp_path: Path) -> None:
        """An item with no cfg attribute is available in normal builds."""
        _write(tmp_path, "src/lib.rs", "pub struct Widget;\n")
        assert "Widget" not in find_cfg_kani_only_items(tmp_path)

    def test_cfg_not_kani_only_item_is_not_flagged(self, tmp_path: Path) -> None:
        """An item gated only by ``#[cfg(not(kani))]`` exists in normal builds."""
        _write(
            tmp_path,
            "src/lib.rs",
            "#[cfg(not(kani))]\npub struct ProdOnly;\n",
        )
        assert "ProdOnly" not in find_cfg_kani_only_items(tmp_path)

    def test_non_exact_cfg_form_is_treated_conservatively(self, tmp_path: Path) -> None:
        """A combined cfg (e.g. ``any(kani, test)``) is NOT treated as Kani-only.

        Conservatism keeps the guard false-positive-free: only an exact
        ``#[cfg(kani)]`` gate marks an item Kani-only.
        """
        _write(
            tmp_path,
            "src/lib.rs",
            "#[cfg(any(kani, test))]\npub struct Maybe;\n",
        )
        assert "Maybe" not in find_cfg_kani_only_items(tmp_path)

    def test_intervening_doc_and_attrs_do_not_break_gating(self, tmp_path: Path) -> None:
        """Doc comments / other attributes between the cfg and the item are fine."""
        _write(
            tmp_path,
            "src/lib.rs",
            "#[cfg(kani)]\n"
            "/// A documented Kani-only helper.\n"
            "#[allow(dead_code)]\n"
            "\n"
            "pub fn kani_helper() {}\n",
        )
        assert "kani_helper" in find_cfg_kani_only_items(tmp_path)

    def test_cfg_kani_does_not_leak_onto_a_later_item(self, tmp_path: Path) -> None:
        """A non-attribute line between the cfg and an item clears the gate."""
        _write(
            tmp_path,
            "src/lib.rs",
            "#[cfg(kani)]\n"
            "const GATED: usize = 1;\n"
            "pub struct NotGated;\n",
        )
        result = find_cfg_kani_only_items(tmp_path)
        assert "GATED" in result
        assert "NotGated" not in result

    def test_only_src_tree_is_scanned(self, tmp_path: Path) -> None:
        """Items outside ``src/`` (e.g. in ``tests/``) are ignored."""
        _write(tmp_path, "tests/foo.rs", "#[cfg(kani)]\npub struct TestItem;\n")
        assert "TestItem" not in find_cfg_kani_only_items(tmp_path)

    def test_missing_src_dir_returns_empty_set(self, tmp_path: Path) -> None:
        """A project with no ``src/`` yields an empty set without error."""
        assert find_cfg_kani_only_items(tmp_path) == frozenset()


class TestCfgKaniOnlyLinkError:
    """The pure link-target predicate."""

    KANI_ONLY = frozenset({"InlineVec", "KANI_INLINE_CAP"})

    @pytest.mark.parametrize(
        "target",
        [
            "crate::proof_vec::InlineVec",
            "super::proof_vec::InlineVec",
            "self::InlineVec",
            "crate::proof_vec::KANI_INLINE_CAP",
            "crate::proof_vec::InlineVec<T>",
        ],
    )
    def test_links_to_kani_only_items_are_flagged(self, target: str) -> None:
        """Any crate-internal path ending in a Kani-only item is rejected."""
        result = cfg_kani_only_link_error(target, self.KANI_ONLY)
        assert result is not None
        assert result[0] is False
        assert "cfg(kani)" in result[1]

    @pytest.mark.parametrize(
        "target",
        [
            "crate::proof_vec::ProofVec",  # exists in all builds
            "crate::input_queue::INPUT_QUEUE_LENGTH",  # exists in all builds
            "bincode::InlineVec",  # external crate, out of scope
            "https://example.com/InlineVec",  # not an intra-doc path
        ],
    )
    def test_non_kani_only_links_are_allowed(self, target: str) -> None:
        """Links to normal items, external crates, or URLs are not flagged."""
        assert cfg_kani_only_link_error(target, self.KANI_ONLY) is None

    def test_empty_set_disables_the_check(self) -> None:
        """With no Kani-only items known, nothing is flagged (back-compat)."""
        assert cfg_kani_only_link_error("crate::proof_vec::InlineVec", frozenset()) is None


class TestEndToEnd:
    """End-to-end checks through check_rust_doc_link / check_rust_doc_file."""

    KANI_ONLY = frozenset({"InlineVec"})

    def test_copilot_scenario_non_gated_doc_links_kani_item(self, tmp_path: Path) -> None:
        """The exact reported defect: a normal doc comment linking a Kani-only item."""
        rust_file = _write(
            tmp_path,
            "lib.rs",
            "/// See [`InlineVec`](crate::proof_vec::InlineVec) for details.\n"
            "pub fn foo() {}\n",
        )
        result = check_rust_doc_file(rust_file, tmp_path, kani_only_items=self.KANI_ONLY)
        assert result.errors == 1

    def test_kani_item_link_flagged_even_on_kani_gated_item(self, tmp_path: Path) -> None:
        """The guard fires regardless of the enclosing item's own cfg.

        The current code is "safe" only because the link sits on a
        ``#[cfg(kani)]`` item; the guard removes that fragile dependency by
        forbidding the link outright.
        """
        rust_file = _write(
            tmp_path,
            "lib.rs",
            "/// Caps [`InlineVec`](crate::proof_vec::InlineVec).\n"
            "#[cfg(kani)]\n"
            "pub const CAP: usize = 7;\n",
        )
        result = check_rust_doc_file(rust_file, tmp_path, kani_only_items=self.KANI_ONLY)
        assert result.errors == 1

    def test_plain_code_span_is_accepted(self, tmp_path: Path) -> None:
        """The recommended fix -- a plain code span, no link -- passes."""
        rust_file = _write(
            tmp_path,
            "lib.rs",
            "/// Caps the Kani-only `InlineVec` that backs the queue.\n"
            "pub fn foo() {}\n",
        )
        result = check_rust_doc_file(rust_file, tmp_path, kani_only_items=self.KANI_ONLY)
        assert result.errors == 0

    def test_check_rust_doc_link_direct(self, tmp_path: Path) -> None:
        """check_rust_doc_link rejects a Kani-only target via the threaded set."""
        rust_file = _write(tmp_path, "lib.rs", "pub fn foo() {}\n")
        is_valid, message = check_rust_doc_link(
            rust_file,
            "crate::proof_vec::InlineVec",
            tmp_path,
            link_text="`InlineVec`",
            kani_only_items=self.KANI_ONLY,
        )
        assert is_valid is False
        assert "cfg(kani)" in message

    def test_default_empty_set_preserves_backwards_compatibility(
        self, tmp_path: Path
    ) -> None:
        """Callers that omit kani_only_items see no new failures."""
        rust_file = _write(
            tmp_path,
            "lib.rs",
            "/// See [`InlineVec`](crate::proof_vec::InlineVec).\n"
            "pub fn foo() {}\n",
        )
        result = check_rust_doc_file(rust_file, tmp_path)
        assert result.errors == 0


class TestRealCrateInvariants:
    """Guard against regressions in the live source tree."""

    def test_live_crate_has_no_kani_only_intra_doc_links(self) -> None:
        """The real ``src/`` tree must contain zero Kani-only intra-doc links.

        This is the regression backstop for the original Copilot finding: if
        anyone reintroduces a link to a ``#[cfg(kani)]``-only item, this fails.
        """
        project_root = Path(__file__).resolve().parent.parent.parent
        if not (project_root / "src").is_dir():
            pytest.skip("source tree not available")
        kani_only = find_cfg_kani_only_items(project_root)
        assert "InlineVec" in kani_only, "fixture invariant: InlineVec is Kani-only"

        errors = 0
        for rust_file in (project_root / "src").rglob("*.rs"):
            errors += check_rust_doc_file(
                rust_file, project_root, kani_only_items=kani_only
            ).errors
        assert errors == 0
