#!/usr/bin/env python3
"""Keep every TLA+ ``FIX_MODE`` set, its ``.cfg`` files, and the prose in sync.

A spec enumerates its confirmation-rule variants in a single
``ASSUME FIX_MODE \\in {...}`` clause. That clause is the one source of truth for
*that* spec; everything else must agree with it. The checker **discovers** every
spec in ``specs/tla`` that declares such a clause (``DoubleFailureRelay.tla``,
``SpectatorReactivationEpoch.tla``, ...) and validates each against its own set,
so adding a second FIX_MODE spec needs no edit here. For each discovered spec:

* every ``.cfg`` that sets ``FIX_MODE`` must name a value *its own spec* defines
  (no orphaned config, and no config validated against the wrong spec);
* every defined mode must be exercised by at least one of that spec's ``.cfg``
  files (no mode added to the spec but never model-checked);
* every defined mode (across all specs) must be named somewhere in ``README.md``
  (no mode landed without a prose description);
* any prose count of the modes ("nine FIX_MODE modes", "9 modes") must equal the
  number some FIX_MODE spec defines (the drift this check was written to catch).

A ``.cfg`` is paired with its spec by filename: the owning module is the longest
``*.tla`` stem that equals the cfg stem or precedes a ``_`` in it, so
``DoubleFailureRelay_AsyncAckSound_Cold.cfg`` pairs with ``DoubleFailureRelay``
and ``SpectatorReactivationEpoch_EpochBlind.cfg`` with
``SpectatorReactivationEpoch`` -- the same pairing TLC uses for
``tlc -config <Spec>_<variant>.cfg <Spec>.tla``.

This is deliberately narrow: it derives each canonical set from its spec and
compares, rather than guessing intent from English. Run it from anywhere; it
locates the repo relative to its own path.

Usage:
    python scripts/docs/check-tla-config-consistency.py
    python scripts/docs/check-tla-config-consistency.py --verbose

Exit codes:
    0 - specs, configs, and prose agree
    1 - a drift was detected (details printed)
"""

from __future__ import annotations

import argparse
import re
import sys
from collections.abc import Iterable
from dataclasses import dataclass, field
from pathlib import Path

# The directory's README.md must name every discovered mode. The FIX_MODE specs
# themselves are discovered (any *.tla with an `ASSUME FIX_MODE \in {...}`),
# never hardcoded -- see `discover_fix_mode_specs`.
README_FILENAME = "README.md"

# `ASSUME FIX_MODE \in { "Baseline", "Tombstone", ... }` — possibly spanning
# several lines. Captured group is the brace body; mode names are pulled from it.
# TLA+ comments are stripped before this runs (see `strip_tla_comments`), so a
# `}` or a `"quoted"` token inside a comment cannot corrupt the parse.
ASSUME_SET_RE = re.compile(
    r"ASSUME\s+FIX_MODE\s*\\in\s*\{(?P<body>[^}]*)\}",
    re.DOTALL,
)
# A double-quoted mode name inside the ASSUME body or a `.cfg` assignment.
QUOTED_NAME_RE = re.compile(r'"([A-Za-z0-9_]+)"')
# `CONSTANT FIX_MODE = "Mode"` or bare `FIX_MODE = "Mode"` in a `.cfg`
# (comments stripped first, so a commented `\* FIX_MODE = "X"` is not counted).
CFG_FIX_MODE_RE = re.compile(r'\bFIX_MODE\s*=\s*"([A-Za-z0-9_]+)"')
# TLA+ comments: `\* line` and `(* block *)`. Stripped from spec/cfg before any
# token scan so commented-out tokens never count as live code.
TLA_BLOCK_COMMENT_RE = re.compile(r"\(\*.*?\*\)", re.DOTALL)
TLA_LINE_COMMENT_RE = re.compile(r"\\\*[^\n]*")

# English count words this check understands, mapped to their values. Counts of
# nine modes are spelled out in the prose; this stays small on purpose.
NUMBER_WORDS = {
    "zero": 0,
    "one": 1,
    "two": 2,
    "three": 3,
    "four": 4,
    "five": 5,
    "six": 6,
    "seven": 7,
    "eight": 8,
    "nine": 9,
    "ten": 10,
    "eleven": 11,
    "twelve": 12,
    "thirteen": 13,
    "fourteen": 14,
    "fifteen": 15,
    "sixteen": 16,
    "seventeen": 17,
    "eighteen": 18,
    "nineteen": 19,
    "twenty": 20,
}
_NUMBER_WORD_ALT = "|".join(NUMBER_WORDS)
# A count claim: a number (word or 1-2 digits) directly modifying the phrase
# "FIX_MODE mode(s)/value(s)". Requiring FIX_MODE *inside* the matched phrase is
# the whole robustness story: there is no proximity heuristic to misfire on a
# nearby-but-unrelated "12 configs" in a table cell, and no sub-count ("four
# original configs", "three S47 modes") matches because none of them name
# FIX_MODE between the number and the noun. The noun is deliberately NOT
# "config": one mode is exercised by several `.cfg` files (default + witness +
# cold + ...), so a "<N> FIX_MODE configs" claim counts FILES, not modes, and
# must not be checked against the mode count. The authoring convention is
# therefore simple: write a mode count as "<N> FIX_MODE modes". Optional
# `**bold**` markers, backticks around FIX_MODE, and a hyphen separator
# ("nine-mode") are tolerated.
COUNT_CLAIM_RE = re.compile(
    rf"(?P<num>\b(?:{_NUMBER_WORD_ALT})\b|\b\d{{1,2}}\b)"
    r"[\s-]*(?:\*\*)?[\s-]*"
    r"`?FIX_MODE`?[\s-]+"
    r"(?P<noun>mode|value)s?\b",
    re.IGNORECASE,
)


@dataclass
class Report:
    """Accumulated check results."""

    errors: list[str] = field(default_factory=list)
    notes: list[str] = field(default_factory=list)

    def error(self, message: str) -> None:
        self.errors.append(message)

    def note(self, message: str) -> None:
        self.notes.append(message)


def tla_dir() -> Path:
    """Return the repo's ``specs/tla`` directory relative to this script."""
    return Path(__file__).resolve().parents[2] / "specs" / "tla"


def strip_tla_comments(text: str) -> str:
    """Blank TLA+ comments so tokens inside them are never parsed as live code.

    Removes ``\\* line`` and ``(* block *)`` comments, replacing each with blanks
    (newlines preserved) so a ``}`` or ``"Quoted"`` token sitting in a comment
    cannot truncate the ASSUME set or be counted as a configured mode.

    Scope: block comments are treated as non-nesting (the regex stops at the
    first ``*)``). TLA+ block comments technically nest, but neither the spec nor
    the configs use nested blocks, and the FIX_MODE comments that motivated this
    are all ``\\*`` line comments, which are handled exactly. Keeping the stripper
    a single pass avoids a comment parser for a case that does not occur.
    """

    def blank(match: re.Match[str]) -> str:
        return "".join("\n" if ch == "\n" else " " for ch in match.group(0))

    text = TLA_BLOCK_COMMENT_RE.sub(blank, text)
    return TLA_LINE_COMMENT_RE.sub(blank, text)


def parse_number(token: str) -> int | None:
    """Return the integer value of a count token, or None if unparsable."""
    lowered = token.lower()
    if lowered in NUMBER_WORDS:
        return NUMBER_WORDS[lowered]
    if token.isdigit():
        return int(token)
    return None


def canonical_modes(spec_text: str) -> set[str]:
    """Return the FIX_MODE set the spec's ASSUME clause defines."""
    match = ASSUME_SET_RE.search(strip_tla_comments(spec_text))
    if not match:
        return set()
    return set(QUOTED_NAME_RE.findall(match.group("body")))


def discover_fix_mode_specs(cfg_dir: Path) -> dict[str, set[str]]:
    """Map each spec module that declares a FIX_MODE set to that set.

    Every ``*.tla`` whose body contains an ``ASSUME FIX_MODE \\in {...}`` clause
    is the source of truth for its own variants; specs without such a clause are
    not FIX_MODE specs and are skipped. Discovering them -- rather than hardcoding
    a single spec -- is what lets a second FIX_MODE spec be added (with its own
    ``.cfg`` files) without editing this checker.

    A spec whose clause is present but *empty* (``{}``) is still returned, mapped
    to an empty set, so an accidentally-emptied set surfaces as an explicit error
    in `check` instead of the spec silently vanishing from every check.
    """
    specs: dict[str, set[str]] = {}
    for tla in sorted(cfg_dir.glob("*.tla")):
        raw = tla.read_text(encoding="utf-8")
        if ASSUME_SET_RE.search(strip_tla_comments(raw)) is None:
            continue  # no FIX_MODE clause -> not a FIX_MODE spec
        specs[tla.stem] = canonical_modes(raw)
    return specs


def cfg_fix_modes(cfg_path: Path) -> set[str]:
    """Return the FIX_MODE values a single ``.cfg`` sets (comments stripped)."""
    text = strip_tla_comments(cfg_path.read_text(encoding="utf-8"))
    return set(CFG_FIX_MODE_RE.findall(text))


def spec_for_cfg(cfg_stem: str, spec_stems: Iterable[str]) -> str | None:
    """Return the spec module a ``.cfg`` filename pairs with, or ``None``.

    The owning spec is the longest module name that equals ``cfg_stem`` or is a
    prefix of it at a ``_`` boundary, so ``DoubleFailureRelay_AsyncAckSound_Cold``
    pairs with ``DoubleFailureRelay`` and never with a shorter, unrelated stem.
    Matching the longest stem means a future ``DoubleFailureRelay_Async`` spec
    would correctly claim its own variants away from ``DoubleFailureRelay``.
    """
    best: str | None = None
    for stem in spec_stems:
        if (cfg_stem == stem or cfg_stem.startswith(f"{stem}_")) and (
            best is None or len(stem) > len(best)
        ):
            best = stem
    return best


def find_count_claims(readme_text: str) -> list[tuple[int, str, int]]:
    """Return ``(value, snippet, line)`` for each FIX_MODE-count claim in prose.

    Only "<N> FIX_MODE mode(s)/value(s)" phrases match (FIX_MODE is part of the
    matched phrase), so unrelated "two players"/"three rounds" prose, a "<N>
    FIX_MODE configs" *file* count, and nearby-but-unrelated table numbers are
    all ignored.
    """
    claims: list[tuple[int, str, int]] = []
    for match in COUNT_CLAIM_RE.finditer(readme_text):
        value = parse_number(match.group("num"))
        if value is None:
            continue
        line = readme_text.count("\n", 0, match.start()) + 1
        # Strip Markdown emphasis and collapse any interior whitespace (the
        # phrase can wrap across a line) so the diagnostic reads on one line.
        snippet = " ".join(match.group(0).replace("*", "").split())
        claims.append((value, snippet, line))
    return claims


def check(cfg_dir: Path, report: Report) -> None:
    """Run every consistency check, recording findings in ``report``."""
    readme_path = cfg_dir / README_FILENAME
    if not readme_path.is_file():
        report.error(f"README not found: {readme_path}")
        return
    readme_text = readme_path.read_text(encoding="utf-8")

    specs = discover_fix_mode_specs(cfg_dir)
    if not specs:
        report.error(
            f"no spec in {cfg_dir} declares `ASSUME FIX_MODE \\in {{...}}`; "
            "the FIX_MODE set is the source of truth this check needs"
        )
        return
    # A spec whose ASSUME clause is present but empty is reported explicitly: it
    # is kept in `specs` (so its cfgs still resolve to it and read "not in {}"
    # rather than the misleading "no ASSUME set") but must never pass silently.
    for stem in sorted(specs):
        if not specs[stem]:
            report.error(
                f"{stem}.tla declares `ASSUME FIX_MODE \\in {{...}}` but the set is "
                "empty. Add its modes (or remove the clause)."
            )
        else:
            report.note(
                f"{stem}.tla defines {len(specs[stem])} FIX_MODE mode(s): "
                f"{', '.join(sorted(specs[stem]))}"
            )

    # Every `.tla` stem is a candidate owner -- not only FIX_MODE specs -- so a
    # cfg that sets FIX_MODE while pairing with a non-FIX_MODE spec is reported
    # precisely instead of being wrongly attributed to an unrelated spec.
    tla_stems = {tla.stem for tla in cfg_dir.glob("*.tla")}

    # 1. Every configured mode must be defined by the cfg's OWN spec.
    exercised: dict[str, set[str]] = {stem: set() for stem in specs}
    for cfg in sorted(cfg_dir.glob("*.cfg")):
        modes = cfg_fix_modes(cfg)
        if not modes:
            continue
        owner = spec_for_cfg(cfg.stem, tla_stems)
        if owner is None:
            report.error(
                f"{cfg.name}: sets FIX_MODE but no spec .tla pairs with its name. "
                "Name it <Spec>.cfg or <Spec>_<variant>.cfg so it pairs with its "
                "spec."
            )
            continue
        if owner not in specs:
            report.error(
                f"{cfg.name}: sets FIX_MODE but its spec {owner}.tla declares no "
                "`ASSUME FIX_MODE \\in {...}` set. Add the set to the spec or "
                "remove FIX_MODE from the config."
            )
            continue
        for mode in sorted(modes):
            if mode in specs[owner]:
                exercised[owner].add(mode)
            else:
                report.error(
                    f"{cfg.name}: FIX_MODE = \"{mode}\" is not in {owner}.tla's "
                    f"ASSUME set {{{', '.join(sorted(specs[owner]))}}}. "
                    "Add the mode to the spec or fix the config."
                )

    # 2. Every defined mode must be exercised by at least one of its spec's cfgs.
    for stem in sorted(specs):
        for mode in sorted(specs[stem] - exercised[stem]):
            report.error(
                f"FIX_MODE mode \"{mode}\" is defined in {stem}.tla but no "
                f"{cfg_dir.name}/{stem}*.cfg sets it. Add a config that exercises "
                "it (or remove it from the ASSUME set)."
            )

    # 3. Every defined mode (across all specs) must be named in the README prose.
    mode_owner: dict[str, str] = {}
    for stem in sorted(specs):
        for mode in sorted(specs[stem]):
            mode_owner.setdefault(mode, stem)
    for mode in sorted(mode_owner):
        if not re.search(rf"\b{re.escape(mode)}\b", readme_text):
            report.error(
                f"FIX_MODE mode \"{mode}\" is defined in {mode_owner[mode]}.tla but "
                f"never mentioned in {readme_path.name}. Document it so the prose "
                "stays complete."
            )

    # 4. Every prose count of the modes must equal SOME FIX_MODE spec's count.
    # With more than one spec a bare "<N> FIX_MODE modes" is spec-agnostic (the
    # checker deliberately reads no proximity to guess which spec a sentence is
    # about), so a claim is accepted when it matches any spec and flagged only
    # when it matches none. This still catches the stale-count drift this check
    # targets. Accepted limitation: if two specs share a mode count, a claim
    # stale for one is masked by the other -- rare, prose-only, and the safer
    # trade than a fragile "which spec is this sentence about" heuristic.
    counts = {stem: len(modes) for stem, modes in specs.items() if modes}
    valid_counts = set(counts.values())
    summary = ", ".join(f"{stem}={counts[stem]}" for stem in sorted(counts)) or "none"
    for value, snippet, line in find_count_claims(readme_text):
        if value in valid_counts:
            report.note(
                f"{readme_path.name}:{line}: count claim \"{snippet}\" matches a "
                f"spec ({value}) (ok)"
            )
        else:
            report.error(
                f"{readme_path.name}:{line}: prose says \"{snippet}\" but no "
                f"FIX_MODE spec defines that many modes ({summary}). "
                "Update the count to match a spec."
            )


def main(argv: list[str] | None = None) -> int:
    """Entry point."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--verbose", action="store_true", help="print every verified claim, not just failures"
    )
    parser.add_argument(
        "--tla-dir",
        type=Path,
        default=None,
        help="override the specs/tla directory (defaults to the repo's)",
    )
    args = parser.parse_args(argv)

    cfg_dir = args.tla_dir if args.tla_dir is not None else tla_dir()
    report = Report()

    if not cfg_dir.is_dir():
        print(f"ERROR: TLA spec directory not found: {cfg_dir}", file=sys.stderr)
        return 1

    check(cfg_dir, report)

    if args.verbose:
        for note in report.notes:
            print(f"  {note}")

    if report.errors:
        print("")
        for err in report.errors:
            print(f"ERROR: {err}")
        print("")
        print(f"FAILED: {len(report.errors)} TLA FIX_MODE consistency issue(s) detected.")
        return 1

    print("SUCCESS: TLA FIX_MODE set, configs, and prose are consistent.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
