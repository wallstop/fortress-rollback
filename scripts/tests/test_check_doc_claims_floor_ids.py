#!/usr/bin/env python3
"""Tests for the removed-floor-identifier guard in scripts/ci/check-doc-claims.sh.

The floor-round work (S55) removed two production identifiers:
``UdpProtocol::peer_pessimistic_floor`` (now ``round_floor``) and the
``Input::pessimistic_floor`` wire field (now ``FloorReply::floors``). The guard
fails when a tracked doc/comment line names one of them WITHOUT a same-line
historical qualifier, and accepts qualified historical narrative. These tests
exercise the ``find`` fallback path (the temp repo is not a git work tree).
"""

from __future__ import annotations

import shutil
import subprocess
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
CHECKER_SOURCE = REPO_ROOT / "scripts" / "ci" / "check-doc-claims.sh"


def _setup_repo(tmp_path: Path) -> Path:
    """Create a temp tree with the checker plus the stubs it needs to run.

    The script also runs a codec-helper scan and shells out to the Rust
    semantic-claim checker; provide minimal stand-ins so execution reaches the
    floor-identifier guard regardless of those unrelated checks.
    """
    repo = tmp_path / "repo"
    checker = repo / "scripts" / "ci" / CHECKER_SOURCE.name
    checker.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(CHECKER_SOURCE, checker)

    # Minimal codec.rs so the decode-helper discovery has a file to read.
    codec = repo / "src" / "network" / "codec.rs"
    codec.parent.mkdir(parents=True, exist_ok=True)
    codec.write_text("fn decode_bounded_with_consumed() {}\n", encoding="utf-8")

    # Stub semantic-claim checker that always succeeds (it is a separate concern).
    semantic = repo / "scripts" / "hooks" / "check-rust-semantic-claims.py"
    semantic.parent.mkdir(parents=True, exist_ok=True)
    semantic.write_text(
        "#!/usr/bin/env python3\nimport sys\nprint('stub ok')\nsys.exit(0)\n",
        encoding="utf-8",
    )
    return repo


def _write(repo: Path, rel_path: str, content: str) -> Path:
    path = repo / rel_path
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")
    return path


def _run(repo: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["bash", "scripts/ci/check-doc-claims.sh"],
        cwd=repo,
        capture_output=True,
        text=True,
        check=False,
    )


def test_flags_unqualified_peer_pessimistic_floor_current_claim(tmp_path: Path) -> None:
    """An unqualified `peer_pessimistic_floor` current-production claim fails."""
    repo = _setup_repo(tmp_path)
    _write(
        repo,
        "docs/guide.md",
        "The relay caches the floor in `UdpProtocol::peer_pessimistic_floor` today.\n",
    )

    result = _run(repo)

    combined = result.stdout + result.stderr
    assert result.returncode == 1
    assert "removed floor identifier" in combined
    assert "docs/guide.md" in combined
    assert "round_floor" in combined  # names the replacement


def test_flags_unqualified_input_pessimistic_floor_wire_field(tmp_path: Path) -> None:
    """An unqualified `Input::pessimistic_floor` wire-field claim fails."""
    repo = _setup_repo(tmp_path)
    _write(
        repo,
        "specs/tla/Model.tla",
        "\\* The floor rides on `Input::pessimistic_floor` per input packet.\n",
    )

    result = _run(repo)

    combined = result.stdout + result.stderr
    assert result.returncode == 1
    assert "Model.tla" in combined
    assert "FloorReply::floors" in combined  # names the replacement


def test_accepts_qualified_historical_mention(tmp_path: Path) -> None:
    """The same removed token is accepted when qualified historical on its line."""
    repo = _setup_repo(tmp_path)
    _write(
        repo,
        "specs/tla/Model.tla",
        "\\* The now-removed legacy cache (formerly `peer_pessimistic_floor`) is gone.\n",
    )

    result = _run(repo)

    combined = result.stdout + result.stderr
    assert result.returncode == 0
    assert "no removed floor identifiers" in combined


def test_does_not_flag_current_pessimistic_floors_plural(tmp_path: Path) -> None:
    """The CURRENT `P2PSession::pessimistic_floors` plural is not a false hit."""
    repo = _setup_repo(tmp_path)
    _write(
        repo,
        "docs/guide.md",
        "The fold is defined on `P2PSession::pessimistic_floors` (plural, canonical).\n",
    )

    result = _run(repo)

    combined = result.stdout + result.stderr
    assert result.returncode == 0
    assert "no removed floor identifiers" in combined


def test_does_not_flag_current_helper_identifiers(tmp_path: Path) -> None:
    """Current helpers sharing the `pessimistic_floor` stem are not flagged."""
    repo = _setup_repo(tmp_path)
    _write(
        repo,
        "docs/guide.md",
        "The gate is `pessimistic_floor_relay_topology`; the reply is `round_floor`.\n",
    )

    result = _run(repo)

    combined = result.stdout + result.stderr
    assert result.returncode == 0
    assert "no removed floor identifiers" in combined
