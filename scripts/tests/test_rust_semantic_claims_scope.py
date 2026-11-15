#!/usr/bin/env python3
"""Regression tests for tracked-file scoping in the doc-claim scanner."""

from __future__ import annotations

import importlib.util
import subprocess
import sys
from pathlib import Path

scripts_dir = Path(__file__).parent.parent
SPEC = importlib.util.spec_from_file_location(
    "check_rust_semantic_claims",
    scripts_dir / "hooks" / "check-rust-semantic-claims.py",
)
assert SPEC is not None
assert SPEC.loader is not None
semantic_claims = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = semantic_claims
SPEC.loader.exec_module(semantic_claims)

collect_rust_files = semantic_claims.collect_rust_files


def _write(path: Path, text: str = "") -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def test_collect_rust_files_ignores_untracked_generated_trees(tmp_path: Path) -> None:
    """Local build trees must not change the scan set."""
    subprocess.run(["git", "init"], cwd=tmp_path, check=True, capture_output=True)
    tracked = tmp_path / "src" / "tracked.rs"
    _write(tracked, "fn tracked() {}\n")
    generated = (
        tmp_path
        / "tests"
        / "godot-emscripten"
        / "target"
        / "debug"
        / "build"
        / "generated.rs"
    )
    _write(generated, "fn generated() {}\n")
    subprocess.run(
        ["git", "add", "src/tracked.rs"],
        cwd=tmp_path,
        check=True,
        capture_output=True,
    )

    files = collect_rust_files(tmp_path, [])

    assert files == [tracked]
    assert generated not in files


def test_collect_rust_files_prunes_generated_dirs_without_git(tmp_path: Path) -> None:
    """The non-Git fallback keeps the same generated-tree boundary."""
    tracked = tmp_path / "src" / "tracked.rs"
    _write(tracked, "fn tracked() {}\n")
    generated = tmp_path / "tests" / "godot-emscripten" / "target" / "generated.rs"
    _write(generated, "fn generated() {}\n")

    files = collect_rust_files(tmp_path, [])

    assert files == [tracked]
    assert generated not in files


def test_doc_claims_scanner_uses_semantic_claim_file_list() -> None:
    """The shell gate and the semantic checker share one scan source."""
    script = (scripts_dir / "ci" / "check-doc-claims.sh").read_text(encoding="utf-8")

    assert 'python3 "$semantic_claim_checker" --list-files' in script
    assert 'find "$PROJECT_ROOT/src" "$PROJECT_ROOT/tests"' not in script
