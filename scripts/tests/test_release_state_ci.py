#!/usr/bin/env python3
"""Offline behavioral tests for the trusted release-state PR gate."""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
from pathlib import Path

import pytest


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "scripts" / "release" / "release_state_ci.py"
SPEC = importlib.util.spec_from_file_location("release_state_ci", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
release_state_ci = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = release_state_ci
SPEC.loader.exec_module(release_state_ci)


def _git(repo: Path, *arguments: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(repo), *arguments],
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, result.stdout + result.stderr
    return result.stdout.strip()


def _repos(tmp_path: Path, *, base_state: dict[str, object] | None = None) -> tuple[Path, Path]:
    base = tmp_path / "base"
    candidate = tmp_path / "candidate"
    base.mkdir()
    _git(base, "init", "--quiet", "-b", "main")
    _git(base, "config", "user.name", "Release CI Test")
    _git(base, "config", "user.email", "release-ci@example.invalid")
    (base / "README.md").write_text("trusted base\n", encoding="utf-8")
    if base_state is not None:
        (base / "release-state.json").write_text(
            json.dumps(base_state, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
    _git(base, "add", "--all")
    _git(base, "commit", "--quiet", "-m", "trusted base")
    result = subprocess.run(
        ["git", "clone", "--quiet", str(base), str(candidate)],
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, result.stdout + result.stderr
    _git(candidate, "config", "user.name", "Release CI Test")
    _git(candidate, "config", "user.email", "release-ci@example.invalid")
    return base, candidate


def _state(target_version: str = "1.3.0") -> dict[str, object]:
    return {
        "schema_version": 1,
        "crate_name": "fortress-rollback",
        "previous_version": "1.2.3",
        "target_version": target_version,
        "bump": "minor",
        "release_date": "2026-07-18",
        "source_sha256": "a" * 64,
    }


def _commit_candidate(candidate: Path, message: str = "candidate") -> None:
    _git(candidate, "add", "--all")
    _git(candidate, "commit", "--quiet", "-m", message)


def _evaluate(base: Path, candidate: Path, head_ref: str):
    return release_state_ci.evaluate(
        candidate,
        base,
        head_ref=head_ref,
        expected_head_sha=_git(candidate, "rev-parse", "HEAD"),
        expected_base_sha=_git(base, "rev-parse", "HEAD"),
    )


def test_ordinary_pr_with_unchanged_state_skips_candidate_execution(
    tmp_path: Path,
) -> None:
    base, candidate = _repos(tmp_path)
    sentinel = tmp_path / "candidate-code-ran"
    candidate_script = candidate / "scripts" / "release" / "release_state_ci.py"
    candidate_script.parent.mkdir(parents=True)
    candidate_script.write_text(
        "from pathlib import Path\n"
        f"Path({str(sentinel)!r}).write_text('unsafe')\n",
        encoding="utf-8",
    )
    (candidate / "ordinary-change.txt").write_text("ordinary PR\n", encoding="utf-8")
    _commit_candidate(candidate)

    decision = _evaluate(base, candidate, "feature/ordinary")

    assert decision == release_state_ci.GateDecision(reconstruction_required=False)
    assert not sentinel.exists()


def test_changed_state_on_matching_release_branch_requires_verification(
    tmp_path: Path,
) -> None:
    base, candidate = _repos(tmp_path)
    (candidate / "release-state.json").write_text(
        json.dumps(_state(), indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    _commit_candidate(candidate)

    decision = _evaluate(base, candidate, "release/v1.3.0")

    assert decision == release_state_ci.GateDecision(
        reconstruction_required=True,
        target_version="1.3.0",
    )


def test_matching_release_branch_with_unchanged_state_requires_verification(
    tmp_path: Path,
) -> None:
    base, candidate = _repos(tmp_path, base_state=_state())
    (candidate / "untrusted-change.txt").write_text(
        "must not bypass reconstruction\n", encoding="utf-8"
    )
    _commit_candidate(candidate)

    decision = _evaluate(base, candidate, "release/v1.3.0")

    assert decision == release_state_ci.GateDecision(
        reconstruction_required=True,
        target_version="1.3.0",
    )


def test_matching_release_branch_cannot_omit_state(tmp_path: Path) -> None:
    base, candidate = _repos(tmp_path)
    (candidate / "untrusted-change.txt").write_text(
        "must not bypass reconstruction\n", encoding="utf-8"
    )
    _commit_candidate(candidate)

    with pytest.raises(
        release_state_ci.ReleaseStateCiError,
        match="cannot delete release-state.json or omit it",
    ):
        _evaluate(base, candidate, "release/v1.3.0")


@pytest.mark.parametrize(
    "head_ref",
    ["release/v01.3.0", "release/v1.3", "release/v1.3.0/extra"],
)
def test_reserved_release_ref_with_unchanged_state_must_be_canonical(
    tmp_path: Path, head_ref: str
) -> None:
    base, candidate = _repos(tmp_path, base_state=_state())
    (candidate / "untrusted-change.txt").write_text(
        "must not bypass reconstruction\n", encoding="utf-8"
    )
    _commit_candidate(candidate)

    with pytest.raises(
        release_state_ci.ReleaseStateCiError,
        match="requires a canonical release/vX.Y.Z head",
    ):
        _evaluate(base, candidate, head_ref)


def test_matching_release_ref_with_unchanged_state_validates_version(
    tmp_path: Path,
) -> None:
    base, candidate = _repos(tmp_path, base_state=_state())
    (candidate / "untrusted-change.txt").write_text(
        "must not bypass reconstruction\n", encoding="utf-8"
    )
    _commit_candidate(candidate)

    with pytest.raises(
        release_state_ci.ReleaseStateCiError,
        match="branch version 1.4.0 does not match candidate target_version 1.3.0",
    ):
        _evaluate(base, candidate, "release/v1.4.0")


def test_merge_group_changed_state_uses_prospective_tree_without_branch_name(
    tmp_path: Path,
) -> None:
    base, candidate = _repos(tmp_path)
    (candidate / "release-state.json").write_text(
        json.dumps(_state(), indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    _commit_candidate(candidate)

    decision = release_state_ci.evaluate(
        candidate,
        base,
        head_ref="refs/heads/gh-readonly-queue/main/pr-253-deadbeef",
        expected_head_sha=_git(candidate, "rev-parse", "HEAD"),
        expected_base_sha=_git(base, "rev-parse", "HEAD"),
        prospective_merge=True,
    )

    assert decision == release_state_ci.GateDecision(True, "1.3.0")


def test_merge_group_rejects_head_that_does_not_descend_from_event_base(
    tmp_path: Path,
) -> None:
    base, candidate = _repos(tmp_path)
    (base / "new-base.txt").write_text("advanced\n", encoding="utf-8")
    _git(base, "add", "--all")
    _git(base, "commit", "--quiet", "-m", "advance base")
    _git(candidate, "fetch", "--quiet", "origin", "main")

    with pytest.raises(
        release_state_ci.ReleaseStateCiError,
        match="does not descend from event base",
    ):
        release_state_ci.evaluate(
            candidate,
            base,
            head_ref="refs/heads/gh-readonly-queue/main/pr-253-deadbeef",
            expected_head_sha=_git(candidate, "rev-parse", "HEAD"),
            expected_base_sha=_git(base, "rev-parse", "HEAD"),
            prospective_merge=True,
        )


@pytest.mark.parametrize(
    "head_ref",
    [
        "feature/ordinary",
        "release/v01.3.0",
        "release/v1.3",
        "release/v1.3.0/extra",
    ],
)
def test_changed_state_rejects_noncanonical_release_branch(
    tmp_path: Path, head_ref: str
) -> None:
    base, candidate = _repos(tmp_path)
    (candidate / "release-state.json").write_text(
        json.dumps(_state(), indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    _commit_candidate(candidate)

    with pytest.raises(
        release_state_ci.ReleaseStateCiError,
        match="requires a canonical release/vX.Y.Z head",
    ):
        _evaluate(base, candidate, head_ref)


def test_changed_state_rejects_branch_manifest_version_mismatch(tmp_path: Path) -> None:
    base, candidate = _repos(tmp_path)
    (candidate / "release-state.json").write_text(
        json.dumps(_state(), indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    _commit_candidate(candidate)

    with pytest.raises(
        release_state_ci.ReleaseStateCiError,
        match="branch version 1.4.0 does not match candidate target_version 1.3.0",
    ):
        _evaluate(base, candidate, "release/v1.4.0")


def test_changed_state_rejects_manifest_deletion(tmp_path: Path) -> None:
    base, candidate = _repos(tmp_path, base_state=_state())
    (candidate / "release-state.json").unlink()
    _commit_candidate(candidate)

    with pytest.raises(
        release_state_ci.ReleaseStateCiError,
        match="cannot delete release-state.json",
    ):
        _evaluate(base, candidate, "release/v1.3.0")


def test_changed_state_rejects_symlink_before_reading_candidate_path(
    tmp_path: Path,
) -> None:
    base, candidate = _repos(tmp_path)
    outside = tmp_path / "outside.json"
    outside.write_text(json.dumps(_state()), encoding="utf-8")
    (candidate / "release-state.json").symlink_to(outside)
    _commit_candidate(candidate)

    with pytest.raises(
        release_state_ci.ReleaseStateCiError,
        match="malformed tree entry",
    ):
        _evaluate(base, candidate, "release/v1.3.0")


def test_gate_rejects_checkout_not_at_event_head(tmp_path: Path) -> None:
    base, candidate = _repos(tmp_path)

    with pytest.raises(
        release_state_ci.ReleaseStateCiError,
        match="expected event head",
    ):
        release_state_ci.evaluate(
            candidate,
            base,
            head_ref="feature/ordinary",
            expected_head_sha="f" * 40,
            expected_base_sha=_git(base, "rev-parse", "HEAD"),
        )


def test_cli_writes_bounded_github_outputs(tmp_path: Path) -> None:
    base, candidate = _repos(tmp_path)
    (candidate / "release-state.json").write_text(
        json.dumps(_state(), indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    _commit_candidate(candidate)
    github_output = tmp_path / "github-output"

    result = subprocess.run(
        [
            sys.executable,
            str(SCRIPT),
            "--candidate-root",
            str(candidate),
            "--trusted-base-root",
            str(base),
            "--head-ref",
            "release/v1.3.0",
            "--expected-head-sha",
            _git(candidate, "rev-parse", "HEAD"),
            "--expected-base-sha",
            _git(base, "rev-parse", "HEAD"),
            "--github-output",
            str(github_output),
        ],
        capture_output=True,
        text=True,
        check=False,
    )

    assert result.returncode == 0, result.stdout + result.stderr
    assert github_output.read_text(encoding="utf-8") == (
        "reconstruction_required=true\ntarget_version=1.3.0\n"
    )
