"""Regression contract for the A7/D11 production documentation."""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def _read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def _section(path: str, heading: str, *, level: int = 2) -> str:
    text = _read(path)
    prefix = "#" * level
    marker = f"{prefix} {heading}"
    start = text.index(marker)
    next_heading = re.search(rf"^#{{1,{level}}} ", text[start + len(marker) :], re.MULTILINE)
    if next_heading is None:
        return text[start:]
    end = start + len(marker) + next_heading.start()
    return text[start:end]


def _assert_terms(text: str, *terms: str) -> None:
    lowered = " ".join(text.casefold().replace("///", " ").split())
    for term in terms:
        normalized = " ".join(term.casefold().split())
        assert normalized in lowered, f"missing documentation contract term: {term!r}"


def test_tested_player_tier_is_disclosed_consistently() -> None:
    changelog = _read("CHANGELOG.md")
    release_notes = changelog.split("## [0.10.0]", 1)[1].split("## [0.9.0]", 1)[0]
    for path, text in [
        ("README.md", _section("README.md", "Network Requirements")),
        (
            "docs/production-checklist.md",
            _section("docs/production-checklist.md", "Release evidence"),
        ),
        ("CHANGELOG.md", release_notes),
    ]:
        _assert_terms(
            text,
            "N=16",
            "deterministic simulation",
            "full-mesh correctness and liveness",
            "documented profiles",
            "above eight players",
            "measure",
        )
        assert "production endorsement" in text.casefold(), path


def test_relay_and_asymmetric_delay_tuning_scope_is_explicit() -> None:
    starting = _section("docs/tuning.md", "Starting configurations")
    relay_row = next(
        line for line in starting.splitlines() if "relay fallback" in line.casefold()
    )
    _assert_terms(
        relay_row,
        "relay fallback",
        "+20–80 ms additional RTT",
        "research-derived",
        "not a measured Fortress baseline",
    )
    _assert_terms(starting, "not the future server-star input topology")

    limits = _section("docs/tuning.md", "Frame-advantage measurement limits")
    _assert_terms(
        limits,
        "RTT/2",
        "symmetric one-way delay",
        "10 ms / 200 ms",
        "105 ms / 105 ms",
        "zero `WaitRecommendation`",
        "N>2",
    )


def test_frames_ahead_sign_and_measurement_model_are_unambiguous() -> None:
    rust = _read("src/sessions/p2p_session.rs")
    match = re.search(
        r"(?P<docs>(?:[ ]*///.*\n)+)[ ]*#\[must_use\]\n[ ]*pub fn frames_ahead",
        rust,
    )
    assert match is not None, "frames_ahead rustdoc block not found"
    rustdoc = match.group("docs")
    _assert_terms(
        rustdoc,
        "positive",
        "local session is ahead",
        "negative",
        "local session is behind",
        "maximum across",
        "RTT/2",
        "asymmetric",
        "zero does not prove peer alignment",
        "integer-frame precision",
    )

    guide = _section("docs/user-guide.md", "Frame Pacing", level=3)
    contract = _section(
        "docs/specs/api-contracts.md", "`frames_ahead(&self) -> i32`", level=3
    )
    for text in (guide, contract):
        _assert_terms(
            text,
            "positive",
            "local session is ahead",
            "negative",
            "local session is behind",
            "zero does not prove peer alignment",
            "integer-frame precision",
        )

    architecture = _section("docs/architecture.md", "Time Synchronization", level=3)
    _assert_terms(
        architecture,
        "positive raw value means the local endpoint is behind",
        "(remote_advantage - local_advantage) / 2",
        "positive public value means the local session is ahead",
        "RTT/2",
    )


def test_nonblocking_socket_keeps_best_effort_send_contract_in_both_cfgs() -> None:
    lib = _read("src/lib.rs")
    traits = list(re.finditer(r"pub trait NonBlockingSocket", lib))
    assert len(traits) == 2
    for trait in traits:
        rustdoc = lib[max(0, trait.start() - 2_000) : trait.start()]
        _assert_terms(
            rustdoc,
            "call is best effort",
            "drop or delay a message locally",
            "blocking delivery guarantee",
        )


def test_production_checklist_covers_every_verified_misuse_surface() -> None:
    misuse = _section("docs/production-checklist.md", "Application misuse surface")
    expectations = {
        "U1": ("WaitRecommendation", "poll", "drain"),
        "U2": ("every network poll", "discard", "high-water"),
        "U3": ("DesyncDetected", "quarantine", "evidence"),
        "U4": ("SaveMode::Sparse", "round-trip", "checksum"),
        "U5": ("frames_ahead", "positive", "slow"),
        "U6": (
            "same non-zero width",
            "rejects the transmission",
            "Error violation",
            "send time",
        ),
        "U7": ("every local handle", "MissingLocalInput", "does not name"),
    }
    for code, terms in expectations.items():
        marker = f"- **{code}"
        start = misuse.index(marker)
        next_row = re.search(r"^- \*\*U[1-7]", misuse[start + len(marker) :], re.MULTILINE)
        end = len(misuse) if next_row is None else start + len(marker) + next_row.start()
        _assert_terms(misuse[start:end], *terms)

    game_loop = _section("docs/user-guide.md", "The Game Loop")
    _assert_terms(
        game_loop,
        "recommended_skips",
        "saturating_sub",
        "match_quarantined",
        "continue polling and draining events",
    )
    event_guide = _section("docs/user-guide.md", "Handling Events")
    _assert_terms(
        event_guide,
        "bounded simulation backpressure",
        "preserve_desync_evidence",
        "match_quarantined",
    )
    diagnostic = _section(
        "docs/user-guide.md", "Desync Response Strategies", level=4
    )
    _assert_terms(diagnostic, "isolated diagnostic reproduction", "never resume")

    termination = _section("docs/architecture.md", "The Correct Pattern", level=4)
    _assert_terms(
        termination,
        "preserve_desync_evidence",
        "quarantine_match",
        "stop authoritative simulation",
    )
