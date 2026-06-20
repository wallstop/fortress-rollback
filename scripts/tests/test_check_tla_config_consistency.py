#!/usr/bin/env python3
"""Unit tests for check-tla-config-consistency.py.

These build tiny synthetic ``specs/tla`` directories (one or more specs, some
``.cfg`` files, a README) and assert the checker accepts a consistent set and
rejects each kind of drift it is meant to catch -- including a config validated
against the wrong spec, the production regression that motivated the multi-spec
rewrite (``SpectatorReactivationEpoch.cfg`` checked against
``DoubleFailureRelay.tla``).
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

scripts_dir = Path(__file__).parent.parent
spec = importlib.util.spec_from_file_location(
    "check_tla_config_consistency",
    scripts_dir / "docs" / "check-tla-config-consistency.py",
)
checker = importlib.util.module_from_spec(spec)
sys.modules["check_tla_config_consistency"] = checker
spec.loader.exec_module(checker)


# --- builders --------------------------------------------------------------
# Real cfgs pair with their spec by filename: ``<Spec>.cfg`` and
# ``<Spec>_<variant>.cfg`` both belong to ``<Spec>.tla``. The builders follow
# that convention so the synthetic dirs exercise the real pairing.


def write_spec(directory: Path, stem: str, modes: list[str]) -> None:
    """Write a minimal ``<stem>.tla`` whose ASSUME clause defines ``modes``."""
    quoted = ", ".join(f'"{m}"' for m in modes)
    (directory / f"{stem}.tla").write_text(
        f"---- MODULE {stem} ----\nASSUME FIX_MODE \\in {{{quoted}}}\n====\n",
        encoding="utf-8",
    )


def write_plain_spec(directory: Path, stem: str) -> None:
    """Write a ``<stem>.tla`` with no FIX_MODE clause (not a FIX_MODE spec)."""
    (directory / f"{stem}.tla").write_text(
        f"---- MODULE {stem} ----\nVARIABLE x\n====\n", encoding="utf-8"
    )


def write_cfg(directory: Path, name: str, mode: str) -> None:
    """Write ``<name>.cfg`` (a spec name or ``<Spec>_<variant>``) setting ``mode``."""
    (directory / f"{name}.cfg").write_text(
        f'CONSTANT FIX_MODE = "{mode}"\n', encoding="utf-8"
    )


def write_readme(directory: Path, body: str) -> None:
    """Write the README prose."""
    (directory / checker.README_FILENAME).write_text(body, encoding="utf-8")


def consistent_dir(tmp_path: Path) -> Path:
    """Build a fully consistent single-spec directory (two modes)."""
    write_spec(tmp_path, "DoubleFailureRelay", ["Baseline", "MeshAgree"])
    write_cfg(tmp_path, "DoubleFailureRelay", "Baseline")
    write_cfg(tmp_path, "DoubleFailureRelay_Mesh", "MeshAgree")
    write_readme(
        tmp_path,
        "FIX_MODE has two modes: Baseline (the residual) and MeshAgree (the fix).\n",
    )
    return tmp_path


def two_spec_dir(tmp_path: Path) -> Path:
    """Build a consistent directory with two independent FIX_MODE specs."""
    write_spec(tmp_path, "DoubleFailureRelay", ["Baseline", "MeshAgree"])
    write_cfg(tmp_path, "DoubleFailureRelay", "Baseline")
    write_cfg(tmp_path, "DoubleFailureRelay_Mesh", "MeshAgree")
    write_spec(tmp_path, "SpectatorReactivationEpoch", ["Epoch", "EpochBlind"])
    write_cfg(tmp_path, "SpectatorReactivationEpoch", "Epoch")
    write_cfg(tmp_path, "SpectatorReactivationEpoch_EpochBlind", "EpochBlind")
    write_readme(
        tmp_path,
        "DoubleFailureRelay: Baseline, MeshAgree.\n"
        "SpectatorReactivationEpoch: Epoch, EpochBlind.\n",
    )
    return tmp_path


def run(directory: Path) -> checker.Report:
    """Run the checker against ``directory`` and return its Report."""
    report = checker.Report()
    checker.check(directory, report)
    return report


# --- consistent inputs pass ------------------------------------------------


def test_consistent_directory_passes(tmp_path: Path) -> None:
    assert run(consistent_dir(tmp_path)).errors == []


def test_two_fix_mode_specs_each_validate_against_their_own_spec(tmp_path: Path) -> None:
    # The whole point of the rewrite: two specs, each with its own modes/cfgs,
    # all consistent -> no error (the old single-spec checker false-failed here).
    assert run(two_spec_dir(tmp_path)).errors == []


# --- spec discovery and cfg<->spec pairing ---------------------------------


def test_discover_fix_mode_specs_skips_non_fix_mode_specs(tmp_path: Path) -> None:
    write_spec(tmp_path, "DoubleFailureRelay", ["Baseline"])
    write_plain_spec(tmp_path, "Rollback")  # no ASSUME FIX_MODE -> not discovered
    discovered = checker.discover_fix_mode_specs(tmp_path)
    assert discovered == {"DoubleFailureRelay": {"Baseline"}}


@pytest.mark.parametrize(
    ("cfg_stem", "expected"),
    [
        ("DoubleFailureRelay", "DoubleFailureRelay"),
        ("DoubleFailureRelay_Baseline", "DoubleFailureRelay"),
        ("DoubleFailureRelay_AsyncAckSound_Cold", "DoubleFailureRelay"),
        ("SpectatorReactivationEpoch", "SpectatorReactivationEpoch"),
        ("SpectatorReactivationEpoch_EpochBlind", "SpectatorReactivationEpoch"),
        ("Unrelated", None),
        ("Unrelated_Variant", None),
    ],
)
def test_spec_for_cfg_pairs_by_longest_boundary_prefix(
    cfg_stem: str, expected: str | None
) -> None:
    stems = {"DoubleFailureRelay", "SpectatorReactivationEpoch"}
    assert checker.spec_for_cfg(cfg_stem, stems) == expected


def test_spec_for_cfg_prefers_the_longest_matching_stem() -> None:
    # When a spec and a longer sub-spec BOTH match at a `_` boundary, the longer
    # one must win (e.g. a future `DoubleFailureRelay_Async` claiming its variants
    # away from `DoubleFailureRelay`). Assert both list orders so a "return first
    # match" regression is caught regardless of iteration order (a set would let
    # hash order hide it).
    assert checker.spec_for_cfg("Foo_Bar_x", ["Foo", "Foo_Bar"]) == "Foo_Bar"
    assert checker.spec_for_cfg("Foo_Bar_x", ["Foo_Bar", "Foo"]) == "Foo_Bar"


def test_spec_for_cfg_requires_an_underscore_boundary() -> None:
    # "Foo" is a string-prefix of "FooBar" but not at a `_` boundary, so a
    # FooBar.cfg must NOT be wrongly paired with the Foo spec.
    assert checker.spec_for_cfg("FooBar", {"Foo"}) is None


# --- the production regression: wrong-spec validation -----------------------


def test_mode_is_validated_against_its_own_spec_not_a_sibling(tmp_path: Path) -> None:
    # Mirrors the live failure: SpectatorReactivationEpoch.cfg's "Epoch" must be
    # checked against SpectatorReactivationEpoch.tla, NOT DoubleFailureRelay.tla.
    directory = two_spec_dir(tmp_path)
    report = run(directory)
    assert report.errors == []
    # Cross-pollinating a Spectator mode into a DoubleFailureRelay cfg IS drift.
    write_cfg(directory, "DoubleFailureRelay_Bad", "Epoch")
    report = run(directory)
    assert any(
        '"Epoch"' in e and "DoubleFailureRelay.tla" in e for e in report.errors
    ), report.errors
    # ...and the well-placed SpectatorReactivationEpoch.cfg is NOT blamed.
    assert not any("SpectatorReactivationEpoch.cfg:" in e for e in report.errors), (
        report.errors
    )


def test_cfg_for_non_fix_mode_spec_is_rejected(tmp_path: Path) -> None:
    # A cfg that sets FIX_MODE but pairs with a spec that has no FIX_MODE set is
    # a clear, precisely attributed error (not blamed on an unrelated spec).
    directory = consistent_dir(tmp_path)
    write_plain_spec(directory, "Rollback")
    write_cfg(directory, "Rollback_X", "Baseline")
    report = run(directory)
    assert any(
        "Rollback_X.cfg" in e and "Rollback.tla" in e and "no" in e.lower()
        for e in report.errors
    ), report.errors


def test_cfg_with_no_matching_spec_is_rejected(tmp_path: Path) -> None:
    directory = consistent_dir(tmp_path)
    write_cfg(directory, "Orphan", "Baseline")  # no Orphan*.tla exists
    report = run(directory)
    assert any(
        "Orphan.cfg" in e and "no spec .tla pairs" in e for e in report.errors
    ), report.errors


# --- the four drift checks (data-driven) -----------------------------------


def _orphan_mode(directory: Path) -> None:
    write_cfg(directory, "DoubleFailureRelay_Ghost", "GhostMode")


def _unexercised_mode(directory: Path) -> None:
    write_spec(directory, "DoubleFailureRelay", ["Baseline", "MeshAgree", "Tombstone"])
    write_readme(directory, "FIX_MODE modes: Baseline, MeshAgree, Tombstone.\n")


def _undocumented_mode(directory: Path) -> None:
    write_spec(directory, "DoubleFailureRelay", ["Baseline", "MeshAgree", "Tombstone"])
    write_cfg(directory, "DoubleFailureRelay_Tomb", "Tombstone")
    write_readme(directory, "FIX_MODE modes: Baseline and MeshAgree.\n")


def _wrong_count_word(directory: Path) -> None:
    write_readme(directory, "Baseline and MeshAgree. The seven FIX_MODE modes.\n")


def _wrong_count_digit(directory: Path) -> None:
    write_readme(directory, "Baseline and MeshAgree. There are 9 FIX_MODE modes.\n")


@pytest.mark.parametrize(
    ("mutate", "needle"),
    [
        (_orphan_mode, "GhostMode"),
        (_unexercised_mode, "Tombstone"),
        (_undocumented_mode, "Tombstone"),
        (_wrong_count_word, "seven"),
        (_wrong_count_digit, "9 FIX_MODE mode"),
    ],
    ids=["orphan", "unexercised", "undocumented", "count-word", "count-digit"],
)
def test_drift_is_rejected(tmp_path: Path, mutate, needle: str) -> None:
    directory = consistent_dir(tmp_path)
    mutate(directory)
    report = run(directory)
    assert any(needle in e for e in report.errors), report.errors


# --- count-claim parsing (kept from the original suite) --------------------


def test_correct_count_passes(tmp_path: Path) -> None:
    directory = consistent_dir(tmp_path)
    write_readme(
        directory,
        "Baseline and MeshAgree. The two FIX_MODE modes are both checked.\n",
    )
    assert run(directory).errors == []


def test_count_claim_matching_any_spec_passes(tmp_path: Path) -> None:
    # With two specs (2 and 3 modes) a count claim is OK if it matches EITHER.
    write_spec(tmp_path, "DoubleFailureRelay", ["A", "B", "C"])
    write_cfg(tmp_path, "DoubleFailureRelay", "A")
    write_cfg(tmp_path, "DoubleFailureRelay_B", "B")
    write_cfg(tmp_path, "DoubleFailureRelay_C", "C")
    write_spec(tmp_path, "SpectatorReactivationEpoch", ["E", "F"])
    write_cfg(tmp_path, "SpectatorReactivationEpoch", "E")
    write_cfg(tmp_path, "SpectatorReactivationEpoch_F", "F")
    write_readme(
        tmp_path,
        "Modes A, B, C, E, F. The three FIX_MODE modes of one spec and the "
        "two FIX_MODE modes of the other.\n",
    )
    assert run(tmp_path).errors == []


def test_count_claim_matching_no_spec_is_rejected(tmp_path: Path) -> None:
    directory = two_spec_dir(tmp_path)  # specs of 2 and 2 modes
    write_readme(
        directory,
        "Baseline, MeshAgree, Epoch, EpochBlind. The nine FIX_MODE modes.\n",
    )
    report = run(directory)
    assert any("nine" in e for e in report.errors), report.errors


def test_subcount_near_fix_mode_is_not_flagged(tmp_path: Path) -> None:
    # "four original configs" / "three S47 modes" are sub-counts: a word sits
    # between the number and the noun, so they must NOT be read as the total.
    directory = consistent_dir(tmp_path)
    write_readme(
        directory,
        "Baseline and MeshAgree. The two FIX_MODE modes: the four original configs "
        "stay byte-identical, and the three S47 modes discharge idealizations.\n",
    )
    assert run(directory).errors == []


def test_count_noun_without_fix_mode_is_ignored(tmp_path: Path) -> None:
    # A bare "three modes" far from any FIX_MODE mention is unrelated prose.
    directory = consistent_dir(tmp_path)
    write_readme(
        directory,
        "Baseline and MeshAgree are the two FIX_MODE modes.\n\n"
        "Elsewhere, the time-sync layer cycles through three modes of drift "
        "estimation, which has nothing to do with the spec's variants.\n",
    )
    assert run(directory).errors == []


def test_table_cell_number_near_fix_mode_is_not_flagged(tmp_path: Path) -> None:
    directory = consistent_dir(tmp_path)
    write_readme(
        directory,
        "Baseline and MeshAgree are the two FIX_MODE modes.\n\n"
        '| `FIX_MODE="Baseline"` | 12 configs generated, 865,558 distinct |\n',
    )
    assert run(directory).errors == []


def test_config_count_is_not_validated_as_a_mode_count(tmp_path: Path) -> None:
    # One mode is exercised by several `.cfg` files, so "<N> FIX_MODE configs"
    # counts FILES, not modes, and must NOT be checked against the mode count
    # (the README really does say "N configs generated").
    directory = consistent_dir(tmp_path)
    write_readme(
        directory,
        "Baseline and MeshAgree are the two FIX_MODE modes; 17 FIX_MODE configs "
        "were generated across the sweep.\n",
    )
    assert run(directory).errors == []


def test_count_snippet_is_single_line(tmp_path: Path) -> None:
    directory = consistent_dir(tmp_path)
    write_readme(directory, "Baseline and MeshAgree. The seven FIX_MODE\nmodes.\n")
    report = run(directory)
    assert report.errors, "a wrong count must be reported"
    assert all("\n" not in err for err in report.errors), report.errors
    assert any("seven FIX_MODE modes" in err for err in report.errors), report.errors


# --- comment / whitespace robustness in the parsers ------------------------


def test_canonical_modes_parses_multiline_assume() -> None:
    text = (
        "---- MODULE DoubleFailureRelay ----\n"
        'ASSUME FIX_MODE \\in {"Baseline", "Tombstone",\n   "MeshAgree"}\n===='
    )
    assert checker.canonical_modes(text) == {"Baseline", "Tombstone", "MeshAgree"}


def test_assume_with_brace_in_comment_parses_all_modes() -> None:
    # A `\* comment` containing a `}` inside the braces must NOT truncate the set.
    text = (
        "---- MODULE DoubleFailureRelay ----\n"
        'ASSUME FIX_MODE \\in {"Baseline",  \\* note: a set like {x} is fine\n'
        '                     "Tombstone", "MeshAgree"}\n===='
    )
    assert checker.canonical_modes(text) == {"Baseline", "Tombstone", "MeshAgree"}


def test_block_comment_in_assume_is_stripped() -> None:
    text = (
        "---- MODULE DoubleFailureRelay ----\n"
        'ASSUME FIX_MODE \\in {"Baseline", (* "Ghost" }} *) "MeshAgree"}\n===='
    )
    assert checker.canonical_modes(text) == {"Baseline", "MeshAgree"}


def test_commented_cfg_fix_mode_is_ignored(tmp_path: Path) -> None:
    # A `\* FIX_MODE = "X"` comment line in a .cfg must NOT count as configured.
    (tmp_path / "DoubleFailureRelay.cfg").write_text(
        '\\* historical: FIX_MODE = "Ghost" was the old default\n'
        'CONSTANT FIX_MODE = "Baseline"\n',
        encoding="utf-8",
    )
    modes = checker.cfg_fix_modes(tmp_path / "DoubleFailureRelay.cfg")
    assert modes == {"Baseline"}


# --- structural / IO errors ------------------------------------------------


def test_missing_readme_is_rejected(tmp_path: Path) -> None:
    directory = consistent_dir(tmp_path)
    (directory / checker.README_FILENAME).unlink()
    assert any("README" in e for e in run(directory).errors)


def test_no_fix_mode_spec_is_rejected(tmp_path: Path) -> None:
    # A dir whose only spec has no ASSUME FIX_MODE clause: nothing to check
    # against, so the checker must report the missing source of truth.
    write_plain_spec(tmp_path, "Rollback")
    write_readme(tmp_path, "no modes here\n")
    report = run(tmp_path)
    assert any("ASSUME" in e and "FIX_MODE" in e for e in report.errors), report.errors


def test_empty_assume_set_is_rejected(tmp_path: Path) -> None:
    # A spec whose ASSUME clause was emptied must NOT silently vanish from the
    # checks alongside a healthy sibling spec -- it is an explicit error.
    write_spec(tmp_path, "DoubleFailureRelay", ["Baseline"])
    write_cfg(tmp_path, "DoubleFailureRelay", "Baseline")
    write_spec(tmp_path, "SpectatorReactivationEpoch", [])  # emptied set
    write_readme(tmp_path, "Baseline.\n")
    report = run(tmp_path)
    assert any(
        "SpectatorReactivationEpoch.tla" in e and "empty" in e for e in report.errors
    ), report.errors


def test_main_returns_zero_on_consistent_dir(tmp_path: Path) -> None:
    assert checker.main(["--tla-dir", str(consistent_dir(tmp_path))]) == 0


def test_main_returns_one_on_drift(tmp_path: Path) -> None:
    directory = consistent_dir(tmp_path)
    write_readme(directory, "Baseline and MeshAgree. The seven FIX_MODE modes.\n")
    assert checker.main(["--tla-dir", str(directory)]) == 1


def test_main_returns_one_on_missing_dir(tmp_path: Path) -> None:
    assert checker.main(["--tla-dir", str(tmp_path / "nope")]) == 1


def test_real_repo_is_consistent() -> None:
    """The checked-in specs/tla must pass (guards against regressions)."""
    report = run(checker.tla_dir())
    assert report.errors == [], report.errors


if __name__ == "__main__":
    sys.exit(pytest.main([__file__, "-v"]))
