#!/usr/bin/env python3
"""Advisory Vale runner for agent-preflight.

Runs Vale (if available) over the supplied docs/*.md files and prints a
compact per-file summary of suggestion counts, plus a one-line hint
pointing at the project's prose-conventions cheat-sheet.

This wrapper is *intentionally distinct* from `scripts/hooks/vale.py`:

- `vale.py` is the pre-commit advisory hook -- it prints the full Vale
  report (every suggestion, line by line). That is the right behavior at
  commit time when the developer is reviewing diffs.
- This wrapper is for the agent-preflight loop, where the goal is
  *visibility without noise*: a one-line-per-file count is enough to
  prompt the agent to look closer, without flooding the preflight log
  with hundreds of individually-fine suggestions.

Always exits 0:
- Vale findings are suggestions, never blocking errors.
- Missing Vale binary is not a failure -- print one-line skip and exit 0.

Cross-platform: Linux, macOS, Windows.
"""
from __future__ import annotations

import re
import shutil
import subprocess
import sys
from pathlib import Path


# Path to the prose-conventions cheat-sheet (relative to repo root).
# Kept as a string here so this script has no dependency on the file
# existing at runtime -- it is only used in the printed hint.
_PROSE_REFERENCE = ".llm/skills/workflows/user-facing-docs.md \"Prose Conventions\""


# Vale `--output=line` format: ``path:line:col:RuleID:Severity:Message``.
# A naive ``partition(":")`` mis-buckets Windows drive-letter paths like
# ``C:\foo\bar.md:14:71:Rule:warning:msg`` (the leading ``C`` would become
# the bucket key). The regex below captures the path up to the first
# ``:<digits>:`` segment (the line:col pair), which is the first colon-pair
# of pure digits in Vale's output -- a drive letter is followed by a path
# separator, not by digits, so it cannot match the path/line boundary.
_VALE_LINE_RE = re.compile(r"^(?P<path>.+?):(?P<line>\d+):(?P<col>\d+):")


def _extract_path(line: str) -> str | None:
    """Return the file path from a single Vale --output=line record.

    Returns None if the line does not look like a Vale finding (e.g. blank,
    malformed, or a stray informational line). Handles Windows drive-letter
    paths (``C:\\foo\\bar.md:14:71:...``) correctly because the regex anchors
    on the ``:<digits>:<digits>:`` line/column pair, which a drive letter
    cannot match.
    """
    match = _VALE_LINE_RE.match(line)
    if match is None:
        return None
    path = match.group("path")
    return path or None


def _summarize_lines(stdout: str) -> dict[str, int]:
    """Count Vale '--output=line' style results per file.

    Vale's `line` output format is one finding per line:
        path:line:col:RuleID:Severity:Message
    so we bucket by the leading path token. Malformed lines are ignored
    rather than mis-bucketed.
    """
    counts: dict[str, int] = {}
    for raw in stdout.splitlines():
        line = raw.strip()
        if not line:
            continue
        path = _extract_path(line)
        if path is None:
            continue
        counts[path] = counts.get(path, 0) + 1
    return counts


def main() -> int:
    """Run Vale and print a compact summary."""
    files = [arg for arg in sys.argv[1:] if arg]
    if not files:
        return 0

    vale = shutil.which("vale")
    if vale is None:
        print(
            "[skip] vale not installed; advisory prose linting skipped. "
            "Install: https://vale.sh/docs/vale-cli/installation/",
            file=sys.stderr,
        )
        return 0

    config = Path(".vale.ini")
    cmd = [vale]
    if config.is_file():
        cmd.extend(["--config", str(config)])
    cmd.extend(["--output=line", "--no-exit"])
    cmd.extend(files)

    try:
        result = subprocess.run(
            cmd,
            check=False,
            capture_output=True,
            text=True,
        )
    except OSError as exc:
        # Tooling failure is not an agent-preflight failure.
        print(f"[skip] could not invoke vale: {exc}", file=sys.stderr)
        return 0

    counts = _summarize_lines(result.stdout)

    if not counts:
        print("vale: 0 suggestions across {} file(s).".format(len(files)))
        return 0

    total = sum(counts.values())
    print(f"vale: {total} suggestion(s) across {len(counts)} file(s) (advisory):")
    for path in sorted(counts):
        print(f"  - {path}: {counts[path]} suggestion(s)")
    print(
        "  hint: see "
        + _PROSE_REFERENCE
        + " for the recurring swap table (implement->do, multiple->many, "
        + "previously->before, ...) and the weasel-word list."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
