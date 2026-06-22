#!/usr/bin/env python3
"""Guard against unaffiliated-community references in user-facing docs (issue #167).

The project must not imply affiliation with communities it is not part of. The
concrete trigger was the *GGPO* Developers Discord linked from the README and
user guide. This test forbids, in user-facing docs:

* the specific GGPO Discord invite URL, and
* the phrase "GGPO Discord" (any casing).

It deliberately does NOT forbid GGPO/GGRS *attribution* (this project is a fork
of GGRS, inspired by GGPO — documented in LICENSE, migration.md, and
fortress-vs-ggrs.md), nor Discord links in general (should the project ever
stand up its own community server).
"""
from __future__ import annotations

import re
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]

# The unaffiliated GGPO Developers Discord invite that prompted issue #167.
GGPO_INVITE = "discord.com/invite/8FKKhCRCCE"
GGPO_DISCORD_PHRASE = re.compile(r"ggpo\s+discord", re.IGNORECASE)

# User-facing surfaces. The generated wiki/ is intentionally included so an
# un-regenerated sync can't reintroduce the reference downstream.
SCAN_TARGETS = [
    REPO_ROOT / "README.md",
    REPO_ROOT / "src" / "lib.rs",
    REPO_ROOT / "docs",
    REPO_ROOT / "wiki",
]


def _iter_files():
    for target in SCAN_TARGETS:
        if target.is_file():
            yield target
        elif target.is_dir():
            yield from sorted(target.rglob("*.md"))


def test_no_ggpo_discord_invite_url() -> None:
    offenders = [
        str(f.relative_to(REPO_ROOT))
        for f in _iter_files()
        if GGPO_INVITE in f.read_text(encoding="utf-8")
    ]
    assert not offenders, (
        f"GGPO Discord invite ({GGPO_INVITE}) found in user-facing docs "
        f"(issue #167 — remove unaffiliated links): {offenders}"
    )


def test_no_ggpo_discord_phrase() -> None:
    offenders = []
    for f in _iter_files():
        for n, line in enumerate(f.read_text(encoding="utf-8").splitlines(), 1):
            if GGPO_DISCORD_PHRASE.search(line):
                offenders.append(f"{f.relative_to(REPO_ROOT)}:{n}")
    assert not offenders, (
        "Reference to the 'GGPO Discord' found in user-facing docs "
        f"(issue #167 — point users to this project's own channels): {offenders}"
    )
