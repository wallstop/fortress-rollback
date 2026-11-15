#!/usr/bin/env python3
"""Contract tests for single-authority CI checks."""

from __future__ import annotations

from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = REPO_ROOT / ".github" / "workflows"


def _workflow_texts() -> dict[str, str]:
    return {
        path.name: path.read_text(encoding="utf-8")
        for path in WORKFLOWS.glob("*.yml")
    }


def test_actionlint_has_one_ci_authority() -> None:
    owners = [name for name, text in _workflow_texts().items() if "actionlint" in text]
    assert owners == ["ci-lint.yml"]


def test_typos_has_one_ci_authority() -> None:
    owners = [
        name
        for name, text in _workflow_texts().items()
        if "crate-ci/typos@" in text
    ]
    assert owners == ["ci-quality.yml"]


def test_external_link_checks_have_one_authority() -> None:
    texts = _workflow_texts()
    assert not any("lycheeverse/lychee-action@" in text for text in texts.values())
    assert any("markdown-link-check" in text for text in texts.values())


def test_standard_clippy_has_one_ci_authority() -> None:
    needle = "cargo clippy --workspace --all-targets --features tokio,json"
    owners = [name for name, text in _workflow_texts().items() if needle in text]
    # ci-safety owns the distinct strict-nursery policy; ci-rust must not repeat
    # the standard clippy pass now owned by ci-pr.
    assert "ci-pr.yml" in owners
    assert "ci-safety.yml" in owners
    assert "ci-rust.yml" not in owners


def test_standard_rustdoc_build_has_no_duplicate_step() -> None:
    texts = _workflow_texts()
    assert "cargo doc --verbose" not in texts["ci-rust.yml"]
