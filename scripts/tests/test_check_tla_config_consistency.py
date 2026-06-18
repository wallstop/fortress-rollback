#!/usr/bin/env python3
"""Unit tests for check-tla-config-consistency.py.

These build tiny synthetic ``specs/tla`` directories (a spec, some ``.cfg``
files, a README) and assert the checker accepts a consistent set and rejects
each kind of drift it is meant to catch.
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


SPEC_HEADER = "---- MODULE DoubleFailureRelay ----\n"


def write_spec(directory: Path, modes: list[str]) -> None:
    """Write a minimal spec whose ASSUME clause defines ``modes``."""
    quoted = ", ".join(f'"{m}"' for m in modes)
    (directory / checker.SPEC_FILENAME).write_text(
        f"{SPEC_HEADER}\nASSUME FIX_MODE \\in {{{quoted}}}\n====\n",
        encoding="utf-8",
    )


def write_cfg(directory: Path, name: str, mode: str) -> None:
    """Write a ``.cfg`` file that sets FIX_MODE to ``mode``."""
    (directory / f"{name}.cfg").write_text(
        f'CONSTANT FIX_MODE = "{mode}"\n', encoding="utf-8"
    )


def write_readme(directory: Path, body: str) -> None:
    """Write the README prose."""
    (directory / checker.README_FILENAME).write_text(body, encoding="utf-8")


def consistent_dir(tmp_path: Path) -> Path:
    """Build a fully consistent two-mode spec/config/README directory."""
    modes = ["Baseline", "MeshAgree"]
    write_spec(tmp_path, modes)
    write_cfg(tmp_path, "Base", "Baseline")
    write_cfg(tmp_path, "Mesh", "MeshAgree")
    write_readme(
        tmp_path,
        "FIX_MODE has two modes: Baseline (the residual) and MeshAgree (the fix).\n",
    )
    return tmp_path


def run(directory: Path) -> checker.Report:
    """Run the checker against ``directory`` and return its Report."""
    report = checker.Report()
    checker.check(directory, report)
    return report


def test_consistent_directory_passes(tmp_path: Path) -> None:
    report = run(consistent_dir(tmp_path))
    assert report.errors == [], report.errors


def test_canonical_modes_parses_multiline_assume(tmp_path: Path) -> None:
    quoted = '"Baseline", "Tombstone",\n   "MeshAgree"'
    text = f"{SPEC_HEADER}\nASSUME FIX_MODE \\in {{{quoted}}}\n===="
    assert checker.canonical_modes(text) == {"Baseline", "Tombstone", "MeshAgree"}


def test_orphan_cfg_mode_is_rejected(tmp_path: Path) -> None:
    directory = consistent_dir(tmp_path)
    # A config naming a mode the spec does not define.
    write_cfg(directory, "Ghost", "AsyncAckSound")
    report = run(directory)
    assert any("AsyncAckSound" in e and "not in" in e for e in report.errors), report.errors


def test_unexercised_mode_is_rejected(tmp_path: Path) -> None:
    directory = consistent_dir(tmp_path)
    # Add a third defined mode with no config exercising it.
    write_spec(directory, ["Baseline", "MeshAgree", "Tombstone"])
    write_readme(
        directory,
        "FIX_MODE has three modes: Baseline, MeshAgree, Tombstone.\n",
    )
    report = run(directory)
    assert any('"Tombstone"' in e and "no" in e.lower() for e in report.errors), report.errors


def test_undocumented_mode_is_rejected(tmp_path: Path) -> None:
    directory = consistent_dir(tmp_path)
    write_spec(directory, ["Baseline", "MeshAgree", "Tombstone"])
    write_cfg(directory, "Tomb", "Tombstone")
    # README omits "Tombstone" entirely and miscounts.
    write_readme(directory, "FIX_MODE has two modes: Baseline and MeshAgree.\n")
    report = run(directory)
    assert any("Tombstone" in e and "mentioned" in e for e in report.errors), report.errors


def test_wrong_count_word_is_rejected(tmp_path: Path) -> None:
    directory = consistent_dir(tmp_path)
    # Spec defines two modes; prose claims "seven FIX_MODE modes".
    write_readme(
        directory,
        "Baseline and MeshAgree. The seven FIX_MODE modes are checked.\n",
    )
    report = run(directory)
    assert any("seven" in e for e in report.errors), report.errors


def test_wrong_count_digit_is_rejected(tmp_path: Path) -> None:
    directory = consistent_dir(tmp_path)
    write_readme(
        directory,
        "Baseline and MeshAgree. There are 9 FIX_MODE configs in total.\n",
    )
    report = run(directory)
    assert any("9 FIX_MODE config" in e for e in report.errors), report.errors


def test_correct_count_passes(tmp_path: Path) -> None:
    directory = consistent_dir(tmp_path)
    write_readme(
        directory,
        "Baseline and MeshAgree. The two FIX_MODE modes are both checked.\n",
    )
    report = run(directory)
    assert report.errors == [], report.errors


def test_subcount_near_fix_mode_is_not_flagged(tmp_path: Path) -> None:
    # "four original configs" / "three S47 modes" are sub-counts: a word sits
    # between the number and the noun, so they must NOT be read as the total.
    directory = consistent_dir(tmp_path)
    write_readme(
        directory,
        "Baseline and MeshAgree. The two FIX_MODE modes: the four original configs "
        "stay byte-identical, and the three S47 modes discharge idealizations.\n",
    )
    report = run(directory)
    assert report.errors == [], report.errors


def test_count_noun_without_fix_mode_is_ignored(tmp_path: Path) -> None:
    # A bare "three modes" far from any FIX_MODE mention is unrelated prose and
    # must not be read as a FIX_MODE count.
    directory = consistent_dir(tmp_path)
    write_readme(
        directory,
        "Baseline and MeshAgree are the two FIX_MODE modes.\n\n"
        "Elsewhere, in a section with no relation to the confirmation rule, the "
        "time-sync layer is described as cycling through three modes of drift "
        "estimation, which has nothing to do with the spec's variants.\n",
    )
    report = run(directory)
    assert report.errors == [], report.errors


def test_assume_with_brace_in_comment_parses_all_modes() -> None:
    # A `\* comment` containing a `}` inside the braces must NOT truncate the
    # set (regression: the body matcher used to stop at the first `}`).
    text = (
        f"{SPEC_HEADER}\n"
        'ASSUME FIX_MODE \\in {"Baseline",  \\* note: a set like {x} is fine\n'
        '                     "Tombstone", "MeshAgree"}\n===='
    )
    assert checker.canonical_modes(text) == {"Baseline", "Tombstone", "MeshAgree"}


def test_block_comment_in_assume_is_stripped() -> None:
    text = (
        f"{SPEC_HEADER}\n"
        'ASSUME FIX_MODE \\in {"Baseline", (* "Ghost" }} *) "MeshAgree"}\n===='
    )
    assert checker.canonical_modes(text) == {"Baseline", "MeshAgree"}


def test_commented_cfg_fix_mode_is_ignored(tmp_path: Path) -> None:
    # A `\* FIX_MODE = "X"` comment line in a .cfg (real cfgs have these) must
    # NOT be counted as a configured mode.
    directory = consistent_dir(tmp_path)
    (directory / "Base.cfg").write_text(
        '\\* historical: FIX_MODE = "Ghost" was the old default\n'
        'CONSTANT FIX_MODE = "Baseline"\n',
        encoding="utf-8",
    )
    report = run(directory)
    assert report.errors == [], report.errors
    # And the live assignment is still picked up.
    assert "Baseline" in checker.cfg_modes(directory)
    assert "Ghost" not in checker.cfg_modes(directory)


def test_table_cell_number_near_fix_mode_is_not_flagged(tmp_path: Path) -> None:
    # A bare "12 configs" sitting next to a `FIX_MODE="X"` table cell must NOT be
    # read as a mode count (regression: the old proximity heuristic flagged it).
    directory = consistent_dir(tmp_path)
    write_readme(
        directory,
        "Baseline and MeshAgree are the two FIX_MODE modes.\n\n"
        '| `FIX_MODE="Baseline"` | 12 configs generated, 865,558 distinct |\n',
    )
    report = run(directory)
    assert report.errors == [], report.errors


def test_count_snippet_is_single_line(tmp_path: Path) -> None:
    # When the claim phrase wraps across a newline, the diagnostic must collapse
    # it onto one line.
    directory = consistent_dir(tmp_path)
    write_readme(
        directory,
        "Baseline and MeshAgree. The seven FIX_MODE\nmodes are wrong.\n",
    )
    report = run(directory)
    assert report.errors, "a wrong count must be reported"
    assert all("\n" not in err for err in report.errors), report.errors
    assert any("seven FIX_MODE modes" in err for err in report.errors), report.errors


def test_missing_readme_is_rejected(tmp_path: Path) -> None:
    directory = consistent_dir(tmp_path)
    (directory / checker.README_FILENAME).unlink()
    report = run(directory)
    assert any("README" in e for e in report.errors), report.errors


def test_main_returns_zero_on_consistent_dir(tmp_path: Path) -> None:
    directory = consistent_dir(tmp_path)
    assert checker.main(["--tla-dir", str(directory)]) == 0


def test_main_returns_one_on_drift(tmp_path: Path) -> None:
    directory = consistent_dir(tmp_path)
    write_readme(directory, "Baseline and MeshAgree. The seven FIX_MODE modes.\n")
    assert checker.main(["--tla-dir", str(directory)]) == 1


def test_main_returns_one_on_missing_dir(tmp_path: Path) -> None:
    assert checker.main(["--tla-dir", str(tmp_path / "nope")]) == 1


def test_missing_assume_clause_is_rejected(tmp_path: Path) -> None:
    directory = consistent_dir(tmp_path)
    (directory / checker.SPEC_FILENAME).write_text(
        f"{SPEC_HEADER}\n(* no ASSUME here *)\n====\n", encoding="utf-8"
    )
    report = run(directory)
    assert any("ASSUME" in e for e in report.errors), report.errors


def test_real_repo_is_consistent() -> None:
    """The checked-in specs/tla must pass (guards against regressions)."""
    report = run(checker.tla_dir())
    assert report.errors == [], report.errors


if __name__ == "__main__":
    sys.exit(pytest.main([__file__, "-v"]))
