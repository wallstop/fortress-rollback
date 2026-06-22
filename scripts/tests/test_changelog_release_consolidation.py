#!/usr/bin/env python3
"""Guard the release-consolidation invariant in CHANGELOG.md.

This repo bumps ``Cargo.toml`` to the *in-flight* (next) version and creates a
dated ``## [<version>]`` section for it at the same time (``sync-version.sh``
requires that dated header to exist and stay consistent). The corollary —
enforced here — is that the ``## [Unreleased]`` section is a placeholder only:
every entry destined for the next release belongs under the current version's
section, never stranded above it under ``[Unreleased]``.

Why it matters: ``publish.yml`` extracts *only* the ``## [<version>]`` section
for the GitHub release notes (the awk block at publish.yml). Anything left under
``[Unreleased]`` while ``<version>`` is the unreleased target would be silently
omitted from that release (this is exactly the bug that stranded the Hot Join
feature before 0.9.0 was consolidated).

The two assertions are durable across release cycles: when the maintainer next
bumps ``Cargo.toml`` they create the new dated version section and in-flight work
goes there, so ``[Unreleased]`` remains empty.
"""
from __future__ import annotations

import re
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
CHANGELOG = REPO_ROOT / "CHANGELOG.md"
CARGO_TOML = REPO_ROOT / "Cargo.toml"


def _cargo_version() -> str:
    for line in CARGO_TOML.read_text(encoding="utf-8").splitlines():
        m = re.match(r'^version = "([0-9]+\.[0-9]+(?:\.[0-9]+)?)"', line)
        if m:
            return m.group(1)
    raise AssertionError("could not find version in Cargo.toml")


def _unreleased_body() -> list[str]:
    """Lines between the ``## [Unreleased]`` header and the next ``## [`` header."""
    lines = CHANGELOG.read_text(encoding="utf-8").splitlines()
    start = None
    for i, line in enumerate(lines):
        if re.match(r"^## \[Unreleased\]", line):
            start = i
            break
    assert start is not None, "CHANGELOG.md is missing the [Unreleased] section"
    body = []
    for line in lines[start + 1 :]:
        if re.match(r"^## \[", line):
            break
        body.append(line)
    return body


def test_unreleased_section_has_no_entries() -> None:
    """[Unreleased] must be an empty placeholder (no bullet entries)."""
    body = _unreleased_body()
    bullets = [ln for ln in body if ln.lstrip().startswith("- ")]
    assert not bullets, (
        "CHANGELOG.md [Unreleased] must be empty while the current version's "
        "section is the in-flight release; fold these entries into the "
        f"## [{_cargo_version()}] section instead:\n"
        + "\n".join(f"  {b}" for b in bullets)
    )
    # No stray subsection headers (### Added/Fixed/...) either.
    subsections = [ln for ln in body if ln.startswith("### ")]
    assert not subsections, (
        "CHANGELOG.md [Unreleased] must not carry subsection headers while it "
        "is an empty placeholder:\n" + "\n".join(f"  {s}" for s in subsections)
    )


def test_cargo_version_has_dated_changelog_section() -> None:
    """The Cargo.toml version must have a dated ``## [<version>] - YYYY-MM-DD`` header."""
    version = _cargo_version()
    text = CHANGELOG.read_text(encoding="utf-8")
    dated = re.search(
        rf"^## \[{re.escape(version)}\] - \d{{4}}-\d{{2}}-\d{{2}}$",
        text,
        re.MULTILINE,
    )
    bare = re.search(rf"^## \[{re.escape(version)}\]$", text, re.MULTILINE)
    assert dated is not None and bare is None, (
        f"CHANGELOG.md must have a dated '## [{version}] - YYYY-MM-DD' header "
        f"matching Cargo.toml (a bare '## [{version}]' fails sync-version.sh --check)."
    )
