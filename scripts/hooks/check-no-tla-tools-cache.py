#!/usr/bin/env python3
"""Reject commits that include anything under `.tla-tools/`.

`.tla-tools/` is the runtime cache directory populated by
`scripts/verification/verify-tla.sh` and `.devcontainer/setup-tla-tools.sh`.
It holds:

- `tla2tools.jar` — a multi-megabyte upstream binary fetched at build time.
- `.version` — a generated sentinel recording which jar revision is cached.

The pinned TLA+ tools version lives in the tracked `.tla-tools-version` file
at the repository root (that is the single source of truth). Committing
anything under `.tla-tools/` desynchronizes the sentinel from the actual
cached jar — pulling a newer committed `.version` would cause the download
guard to incorrectly skip updating an older local jar — and bloats history
with binary blobs that should never live in git.

This hook is a defensive guard around `.gitignore`. It runs against the
candidate file list `pre-commit` passes on stdin (one path per argument) and
exits non-zero with a remediation message if any of them live under
`.tla-tools/`. It also performs a repository-wide sweep via
`git ls-files .tla-tools/` so that even a hook invocation that received no
arguments (`pre-commit run --all-files` against an empty fileset) will catch
already-tracked entries.
"""
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

CACHE_DIR = ".tla-tools/"
REMEDIATION = (
    "Files under `.tla-tools/` must never be tracked. The directory is a\n"
    "runtime cache; the pinned upstream version lives in `.tla-tools-version`.\n"
    "\n"
    "Fix:\n"
    "    git rm --cached -r .tla-tools/\n"
    "    # ensure `.tla-tools/` is listed in `.gitignore`, then re-commit.\n"
)


def _tracked_in_cache_dir() -> list[str]:
    """Return any paths under `.tla-tools/` currently tracked by git."""
    try:
        result = subprocess.run(
            ["git", "ls-files", CACHE_DIR],
            capture_output=True,
            text=True,
            check=False,
        )
    except FileNotFoundError:
        # No git in PATH — skip the repo-wide sweep.
        return []
    if result.returncode != 0:
        return []
    return [line for line in result.stdout.splitlines() if line.strip()]


def main(argv: list[str]) -> int:
    offenders: set[str] = set()

    for arg in argv:
        normalized = Path(arg).as_posix()
        if normalized.startswith(CACHE_DIR) or normalized == CACHE_DIR.rstrip("/"):
            offenders.add(normalized)

    offenders.update(_tracked_in_cache_dir())

    if not offenders:
        return 0

    sys.stderr.write(
        "error: the following paths must not be committed (live under "
        f"`{CACHE_DIR}`):\n"
    )
    for path in sorted(offenders):
        sys.stderr.write(f"  {path}\n")
    sys.stderr.write("\n")
    sys.stderr.write(REMEDIATION)
    return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
