#!/usr/bin/env python3
"""Spell-check runner for agent-preflight: run `typos` and surface findings.

CI enforces spelling with the crate-ci/typos action
(.github/workflows/ci-quality.yml) using `.typos.toml`, but `typos` is absent
from the local pre-commit hooks -- it is a Rust binary, and the pre-commit hooks
are kept all-Python for Windows portability. That gap let a spelling error
reach CI. This wrapper closes the gap for the agent loop: agent-preflight runs
`typos` so an agent catches spelling BEFORE pushing, mirroring the exact CI gate
(same `.typos.toml`).

Behavior:
- typos found issues  -> stream them to the terminal, exit non-zero (BLOCKING --
  it is a required CI gate, so preflight should fail the same way CI will).
- typos clean         -> exit 0.
- typos not installed -> print an actionable install hint, exit 0 (soft-skip:
  CI still enforces, so a missing LOCAL tool must not block the agent).

The issue output streams straight through (not captured) so the agent sees the
full `path:line:col` report, which is what makes it actionable.

Cross-platform: Linux, macOS, Windows.
"""

from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

# typos exit codes: 0 = clean, 2 = typos found. Any other non-zero is a tool
# error (bad config, I/O). We propagate all non-zero codes so preflight fails
# whenever CI would.
_TYPOS_INSTALL_HINT = (
    "[skip] typos not installed; spell check skipped (CI still enforces). "
    "Install: https://github.com/crate-ci/typos#install "
    "(or `cargo install typos-cli`)."
)


def main() -> int:
    """Run typos over the repo with the project config; return its exit code."""
    typos = shutil.which("typos")
    if typos is None:
        print(_TYPOS_INSTALL_HINT, file=sys.stderr)
        return 0

    cmd = [typos]
    config = Path(".typos.toml")
    if config.is_file():
        cmd.extend(["--config", str(config)])

    try:
        result = subprocess.run(cmd, check=False)
    except OSError as exc:
        # A tooling failure (e.g. the binary vanished mid-run) must not block
        # the agent; CI remains the source of truth.
        print(f"[skip] could not invoke typos: {exc}", file=sys.stderr)
        return 0

    return result.returncode


if __name__ == "__main__":
    sys.exit(main())
