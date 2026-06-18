#!/usr/bin/env python3
"""Keep the TLA+ ``FIX_MODE`` set, its ``.cfg`` files, and the prose in sync.

The ``DoubleFailureRelay`` spec enumerates its confirmation-rule variants in a
single ``ASSUME FIX_MODE \\in {...}`` clause. That clause is the one source of
truth; everything else must agree with it:

* every ``.cfg`` that sets ``FIX_MODE`` must name a value the spec defines
  (no orphaned config referencing a deleted/renamed mode);
* every defined mode must be exercised by at least one ``.cfg``
  (no mode added to the spec but never model-checked);
* every defined mode must be named somewhere in ``README.md``
  (no mode landed without a prose description);
* any prose count of the modes ("nine FIX_MODE modes", "9 modes") must equal
  the number the spec defines (the drift this check was written to catch).

This is deliberately narrow: it derives the canonical set from the spec and
compares, rather than guessing intent from English. Run it from anywhere; it
locates the repo relative to its own path.

Usage:
    python scripts/docs/check-tla-config-consistency.py
    python scripts/docs/check-tla-config-consistency.py --verbose

Exit codes:
    0 - spec, configs, and prose agree
    1 - a drift was detected (details printed)
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

# The spec whose FIX_MODE set is the source of truth, and the directory whose
# *.cfg files and README.md must agree with it. Both are relative to the TLA
# spec directory resolved in `tla_dir()`.
SPEC_FILENAME = "DoubleFailureRelay.tla"
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
# "FIX_MODE mode(s)/value(s)/config(s)". Requiring FIX_MODE *inside* the matched
# phrase is the whole robustness story: there is no proximity heuristic to
# misfire on a nearby-but-unrelated "12 configs" in a table cell, and no
# sub-count ("four original configs", "three S47 modes") matches because none of
# them name FIX_MODE between the number and the noun. The authoring convention
# is therefore simple: write a mode count as "<N> FIX_MODE modes". Optional
# `**bold**` markers, backticks around FIX_MODE, and a hyphen separator
# ("nine-mode") are tolerated.
COUNT_CLAIM_RE = re.compile(
    rf"(?P<num>\b(?:{_NUMBER_WORD_ALT})\b|\b\d{{1,2}}\b)"
    r"[\s-]*(?:\*\*)?[\s-]*"
    r"`?FIX_MODE`?[\s-]+"
    r"(?P<noun>config|mode|value)s?\b",
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


def cfg_modes(cfg_dir: Path) -> dict[str, set[str]]:
    """Map each configured FIX_MODE to the set of ``.cfg`` files that use it."""
    by_mode: dict[str, set[str]] = {}
    for cfg in sorted(cfg_dir.glob("*.cfg")):
        text = strip_tla_comments(cfg.read_text(encoding="utf-8"))
        for mode in CFG_FIX_MODE_RE.findall(text):
            by_mode.setdefault(mode, set()).add(cfg.name)
    return by_mode


def find_count_claims(readme_text: str) -> list[tuple[int, str, int]]:
    """Return ``(value, snippet, line)`` for each FIX_MODE-count claim in prose.

    Only "<N> FIX_MODE mode(s)/value(s)/config(s)" phrases match (FIX_MODE is
    part of the matched phrase), so unrelated "two players"/"three rounds" prose
    and nearby-but-unrelated table numbers are ignored.
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
    spec_path = cfg_dir / SPEC_FILENAME
    readme_path = cfg_dir / README_FILENAME

    if not spec_path.is_file():
        report.error(f"spec not found: {spec_path}")
        return
    if not readme_path.is_file():
        report.error(f"README not found: {readme_path}")
        return

    spec_text = spec_path.read_text(encoding="utf-8")
    readme_text = readme_path.read_text(encoding="utf-8")

    modes = canonical_modes(spec_text)
    if not modes:
        report.error(
            f"could not parse `ASSUME FIX_MODE \\in {{...}}` from {spec_path.name}; "
            "the FIX_MODE set is the source of truth this check needs"
        )
        return
    report.note(f"spec defines {len(modes)} FIX_MODE mode(s): {', '.join(sorted(modes))}")

    configured = cfg_modes(cfg_dir)

    # 1. Every configured mode must be a defined mode.
    for mode in sorted(configured):
        if mode not in modes:
            files = ", ".join(sorted(configured[mode]))
            report.error(
                f"{files}: FIX_MODE = \"{mode}\" is not in {spec_path.name}'s "
                f"ASSUME set {{{', '.join(sorted(modes))}}}. "
                "Add the mode to the spec or fix the config."
            )

    # 2. Every defined mode must be exercised by at least one config.
    for mode in sorted(modes):
        if mode not in configured:
            report.error(
                f"FIX_MODE mode \"{mode}\" is defined in {spec_path.name} but no "
                f"{cfg_dir.name}/*.cfg sets it. Add a config that exercises it "
                "(or remove it from the ASSUME set)."
            )

    # 3. Every defined mode must be named in the README prose.
    for mode in sorted(modes):
        if not re.search(rf"\b{re.escape(mode)}\b", readme_text):
            report.error(
                f"FIX_MODE mode \"{mode}\" is defined in {spec_path.name} but never "
                f"mentioned in {readme_path.name}. Document it so the prose stays "
                "complete."
            )

    # 4. Every prose count of the modes must equal the defined count.
    expected = len(modes)
    claims = find_count_claims(readme_text)
    for value, snippet, line in claims:
        if value != expected:
            report.error(
                f"{readme_path.name}:{line}: prose says \"{snippet}\" but "
                f"{spec_path.name} defines {expected} FIX_MODE mode(s). "
                "Update the count to match the spec."
            )
        else:
            report.note(f"{readme_path.name}:{line}: count claim \"{snippet}\" == {expected} (ok)")


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
