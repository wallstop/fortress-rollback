"""Regression contracts for the user-first documentation path."""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def _read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def test_readme_stays_a_short_route_map() -> None:
    readme = _read("README.md")
    words = re.findall(r"[A-Za-z0-9][A-Za-z0-9'_-]*", readme)
    assert len(words) <= 1_000

    headings = set(re.findall(r"^#{1,6} (.+)$", readme, re.MULTILINE))
    assert "Feature Flags" not in headings
    assert "Web / WASM Support" not in headings
    assert "Custom Sockets" not in headings
    assert "Contributing" not in headings

    for target in (
        "docs/getting-started.md",
        "docs/user-guide.md",
        "examples/README.md",
        "docs/tuning.md",
        "docs/production-checklist.md",
    ):
        assert f"]({target}" in readme


def test_docs_home_routes_to_one_first_session_program() -> None:
    home = _read("docs/index.md")
    first_session = _read("docs/getting-started.md")
    navigation = _read("mkdocs.yml")

    assert "```rust" not in home
    assert home.count("](getting-started.md)") >= 2
    assert first_session.count("```rust") == 1
    assert "fn main() -> Result<(), Box<dyn std::error::Error>>" in first_session
    assert "cargo run --example sync_test" in first_session
    assert "status != InputStatus::Disconnected" in first_session
    assert ".unwrap()" not in first_session
    assert ".expect(" not in first_session

    first_nav = navigation.index("First Session: getting-started.md")
    guide_nav = navigation.index("User Guide: user-guide.md")
    assert first_nav < guide_nav
