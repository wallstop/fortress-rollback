#!/usr/bin/env python3
"""Regression test: every ``./scripts/...sh`` referenced from a workflow has the executable bit set in the git index.

A `.sh` script invoked via ``./script.sh`` (instead of ``bash script.sh``)
fails immediately with ``Permission denied`` if its git-index mode is
``100644`` rather than ``100755`` -- even when the working-tree file mode is
correct, because GitHub Actions' ``actions/checkout`` materialises file modes
from the git index, not from the working tree of whoever last touched it.

This test walks every ``.github/workflows/*.yml``, scans ``run:`` blocks for
``./<path>.sh`` invocations, and asserts ``git ls-files --stage`` reports
``100755`` for each one. It is parametrised so a failure pinpoints the exact
``(workflow, script)`` edge that broke.

The test deliberately uses the **git-index mode** rather than the
filesystem mode because the index is the source of truth for what ships to
CI runners (``chmod +x`` on the developer machine without a corresponding
``git update-index --chmod=+x`` is a silent landmine).
"""

from __future__ import annotations

import re
import subprocess
from pathlib import Path

import pytest

PROJECT_ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS_DIR = PROJECT_ROOT / ".github" / "workflows"

# Regex pattern:
#   (?:^|\s)([^\s]*?)\./([^\s'"]+\.sh)\b
# Matches the typical "./scripts/foo.sh" invocation form, anchored to a
# whitespace boundary so we don't catch path fragments mid-token. The first
# capture group is the (possibly empty) non-whitespace prefix immediately
# preceding "./" -- we use it in ``_is_direct_invocation`` to filter out
# matches like ``bash ./script.sh`` where the executable bit is irrelevant
# because an explicit interpreter is invoked.
#
# We intentionally do NOT try to parse YAML to skip comments -- comments
# referencing real scripts still need those scripts executable for any
# code path that runs them, and false positives have not arisen in
# practice. If a future workflow legitimately documents a non-executable
# script in a comment, prefer a ``# noqa``-style suppression at that site
# over weakening this lint.
_INVOCATION_RE = re.compile(r"(?:^|\s)([^\s]*?)\./([^\s'\"]+\.sh)\b")

# Tokens that, when they appear immediately before ``./script.sh``, mean an
# explicit interpreter is consuming the script -- the executable bit is
# irrelevant in that case, so we skip the match. Compared as a whole token
# (i.e. ``bash`` matches ``bash ./foo.sh`` but not ``mybash ./foo.sh``).
_INTERPRETER_TOKENS: frozenset[str] = frozenset(
    {"bash", "sh", "zsh", "dash", "ksh", "python", "python3", "env", "source", "."}
)


def _is_direct_invocation(prefix: str, line: str, prefix_start: int) -> bool:
    """Return True iff a ``./script.sh`` match represents a direct invocation
    that requires the executable bit.

    Returns False for ``bash ./script.sh``, ``sh ./script.sh``,
    ``python ./script.sh``, ``source ./script.sh``, etc. where an explicit
    interpreter is doing the work.

    ``prefix`` is the (possibly empty) non-whitespace text captured between
    the leading whitespace boundary and ``./``. ``line`` and ``prefix_start``
    let us look one whitespace-separated token to the left when ``prefix``
    is empty (the common case: ``bash ./foo.sh`` has an empty prefix because
    the ``\\s`` boundary sits between ``bash`` and ``./``).
    """
    # Case A: prefix has content (e.g. "/path/to/" or "MYVAR=") -- a real
    # path fragment, not an interpreter. Treat as direct invocation.
    if prefix:
        return True
    # Case B: prefix empty -- look one token to the left of the match start
    # for an interpreter keyword. ``prefix_start`` is the offset where the
    # captured prefix would begin; the character before that is whitespace
    # (the regex's ``\s`` boundary), so scan further back to the previous
    # whitespace to extract the prior token.
    cursor = prefix_start - 1
    while cursor >= 0 and line[cursor].isspace():
        cursor -= 1
    token_end = cursor + 1
    while cursor >= 0 and not line[cursor].isspace():
        cursor -= 1
    token_start = cursor + 1
    prior_token = line[token_start:token_end]
    if not prior_token:
        return True
    return prior_token not in _INTERPRETER_TOKENS


def _discover_workflow_script_pairs() -> list[tuple[Path, str, int]]:
    """Return ``(workflow_path, script_path, line_number)`` triples.

    Only scripts whose executable bit is load-bearing are returned. We
    exclude matches like ``bash ./script.sh`` and ``source ./lib.sh`` where
    an explicit interpreter is invoked -- those don't depend on the
    executable bit.
    """
    pairs: list[tuple[Path, str, int]] = []
    if not WORKFLOWS_DIR.is_dir():
        return pairs
    for workflow in sorted(WORKFLOWS_DIR.glob("*.yml")):
        try:
            text = workflow.read_text(encoding="utf-8")
        except OSError:
            continue
        for line_num, line in enumerate(text.splitlines(), start=1):
            for match in _INVOCATION_RE.finditer(line):
                prefix = match.group(1)
                script_rel = match.group(2)
                if not _is_direct_invocation(prefix, line, match.start(1)):
                    continue
                # Only include scripts that actually exist in the repo.
                # External or generated paths (e.g., paths under target/)
                # would be noise.
                if (PROJECT_ROOT / script_rel).is_file():
                    pairs.append((workflow, script_rel, line_num))
    # Fail loudly if discovery breaks (regex change, workflows moved, etc.)
    # rather than silently skipping the test via the @skipif below. The
    # skipif still guards the genuine "no workflows" edge case.
    if WORKFLOWS_DIR.is_dir() and any(WORKFLOWS_DIR.glob("*.yml")) and not pairs:
        raise RuntimeError(
            "Workflow script discovery returned zero ``./script.sh`` "
            "invocations despite workflows existing under "
            f"{WORKFLOWS_DIR}. The regex or filter has likely regressed."
        )
    return pairs


DISCOVERED_PAIRS: list[tuple[Path, str, int]] = _discover_workflow_script_pairs()


def _pair_id(triple: tuple[Path, str, int]) -> str:
    workflow, script, line = triple
    return f"{workflow.name}:{line}->{script}"


@pytest.mark.skipif(
    not DISCOVERED_PAIRS,
    reason=(
        "No ``./script.sh`` invocations discovered in .github/workflows/*.yml -- "
        "either the regex is broken or the project genuinely has none. "
        "Investigate before assuming the latter."
    ),
)
@pytest.mark.parametrize(
    "workflow_path,script_path,line_number",
    DISCOVERED_PAIRS,
    ids=[_pair_id(t) for t in DISCOVERED_PAIRS],
)
def test_workflow_script_is_executable_in_git_index(
    workflow_path: Path,
    script_path: str,
    line_number: int,
) -> None:
    """Every ``./script.sh`` invocation in a workflow points at a 100755 file.

    The workflow line number is included in the failure message so a user
    can jump straight to the offending invocation.
    """
    result = subprocess.run(
        ["git", "ls-files", "--stage", script_path],
        cwd=PROJECT_ROOT,
        check=True,
        text=True,
        capture_output=True,
    )
    output = result.stdout.strip()
    assert output, (
        f"git ls-files --stage returned no output for {script_path}. "
        f"Referenced from {workflow_path.name}:{line_number}. "
        f"Is the script tracked by git?"
    )
    mode = output.split()[0]
    assert mode == "100755", (
        f"{script_path} (referenced from "
        f"{workflow_path.relative_to(PROJECT_ROOT)}:{line_number} as "
        f"./{script_path}) has git-index mode {mode}, expected 100755. "
        f"Fix with: git update-index --chmod=+x {script_path}"
    )
