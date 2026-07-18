#!/usr/bin/env python3
"""Behavioral tests for trusted release-tag checkpoint handling."""

from __future__ import annotations

import importlib.util
import subprocess
import sys
from pathlib import Path
from types import ModuleType

import pytest

REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "scripts" / "release" / "release_checkpoint.py"


def _load_module() -> ModuleType:
    spec = importlib.util.spec_from_file_location("release_checkpoint", SCRIPT)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


release_checkpoint = _load_module()


def _git(repo: Path, *arguments: str, check: bool = True) -> str:
    result = subprocess.run(
        ["git", "-C", str(repo), *arguments],
        check=False,
        capture_output=True,
        text=True,
    )
    if check and result.returncode != 0:
        raise AssertionError(
            f"git {' '.join(arguments)} failed ({result.returncode}): {result.stderr}"
        )
    return result.stdout.strip()


@pytest.fixture
def repository(tmp_path: Path) -> tuple[Path, Path]:
    remote = tmp_path / "origin.git"
    trusted = tmp_path / "trusted"
    _git(tmp_path, "init", "--bare", str(remote))
    _git(tmp_path, "init", "-b", "main", str(trusted))
    _git(trusted, "config", "user.name", "Release Test")
    _git(trusted, "config", "user.email", "release-test@example.com")
    (trusted / "tracked.txt").write_text("first\n", encoding="utf-8")
    (trusted / "Cargo.toml").write_text(
        '[package]\nname = "checkpoint-fixture"\nversion = "1.2.2"\n',
        encoding="utf-8",
    )
    (trusted / "CHANGELOG.md").write_text(
        "# Changelog\n\n"
        "## [Unreleased]\n\n"
        "### Fixed\n\n"
        "- **Pre-existing:** Fix the release fixture.\n\n"
        "## [1.2.2] - 2026-01-01\n\n"
        "### Fixed\n\n"
        "- Previous release.\n",
        encoding="utf-8",
    )
    issue_template = trusted / ".github" / "ISSUE_TEMPLATE" / "bug_report.yml"
    issue_template.parent.mkdir(parents=True)
    issue_template.write_text(
        "body:\n"
        "  - type: dropdown\n"
        "    attributes:\n"
        "      options:\n"
        "        # BEGIN_FORTRESS_VERSIONS\n"
        "        - v1.2.2\n"
        "        # END_FORTRESS_VERSIONS\n",
        encoding="utf-8",
    )
    _git(trusted, "add", ".")
    _git(trusted, "commit", "-m", "previous release")
    _git(trusted, "tag", "-a", "v1.2.2", "-m", "previous release")

    (trusted / "Cargo.toml").write_text(
        '[package]\nname = "checkpoint-fixture"\nversion = "1.2.3"\n',
        encoding="utf-8",
    )
    (trusted / "CHANGELOG.md").write_text(
        "# Changelog\n\n"
        "## [Unreleased]\n\n"
        "## [1.2.3] - 2026-07-18\n\n"
        "### Fixed\n\n"
        "- **Pre-existing:** Fix the release fixture.\n\n"
        "## [1.2.2] - 2026-01-01\n\n"
        "### Fixed\n\n"
        "- Previous release.\n",
        encoding="utf-8",
    )
    issue_template.write_text(
        "body:\n"
        "  - type: dropdown\n"
        "    attributes:\n"
        "      options:\n"
        "        # BEGIN_FORTRESS_VERSIONS\n"
        "        - v1.2.3\n"
        "        - v1.2.2\n"
        "        # END_FORTRESS_VERSIONS\n",
        encoding="utf-8",
    )
    release_checkpoint.release_state.generate(
        trusted,
        previous_version="1.2.2",
        target_version="1.2.3",
        bump="patch",
        release_date="2026-07-18",
    )
    _git(trusted, "add", ".")
    _git(trusted, "commit", "-m", "prepare release 1.2.3")
    _git(trusted, "remote", "add", "origin", str(remote))
    _git(trusted, "push", "-u", "origin", "main")
    _git(trusted, "push", "origin", "refs/tags/v1.2.2")
    return trusted, remote


def _paths(tmp_path: Path) -> tuple[Path, Path]:
    return tmp_path / "candidate", tmp_path / "checkpoint.json"


def _resolve(
    repository: Path, tmp_path: Path, version: str = "1.2.3"
) -> object:
    candidate, state = _paths(tmp_path)
    return release_checkpoint.resolve(
        repository,
        remote="origin",
        version=version,
        trusted_sha=_git(repository, "rev-parse", "HEAD"),
        candidate_path=candidate,
        state_file=state,
    )


def _annotated_tag(repo: Path, tag_name: str, target: str, message: str) -> str:
    _git(repo, "tag", "-f", "-a", tag_name, "-m", message, target)
    return _git(repo, "rev-parse", tag_name)


def _force_remote_tag(repo: Path, tag_name: str, direct_oid: str) -> None:
    _git(repo, "push", "--force", "origin", f"{direct_oid}:refs/tags/{tag_name}")


def _delete_local_and_remote_tag(repo: Path, tag_name: str) -> None:
    _git(repo, "tag", "-d", tag_name, check=False)
    _git(repo, "push", "origin", f":refs/tags/{tag_name}", check=False)


def test_absent_tag_uses_unique_prepared_release_commit(
    repository: tuple[Path, Path], tmp_path: Path
) -> None:
    trusted, _remote = repository
    checkpoint = _resolve(trusted, tmp_path)

    assert checkpoint.candidate_sha == _git(trusted, "rev-parse", "HEAD")
    assert checkpoint.direct_oid is None
    assert _git(tmp_path / "candidate", "rev-parse", "HEAD") == checkpoint.candidate_sha


def test_absent_tag_uses_prepared_commit_after_main_advances(
    repository: tuple[Path, Path], tmp_path: Path
) -> None:
    trusted, _remote = repository
    prepared = _git(trusted, "rev-parse", "HEAD")
    (trusted / "tracked.txt").write_text("unrelated advancement\n", encoding="utf-8")
    _git(trusted, "add", "tracked.txt")
    _git(trusted, "commit", "-m", "unrelated main advancement")
    _git(trusted, "push", "origin", "main")

    checkpoint = _resolve(trusted, tmp_path)

    assert checkpoint.candidate_sha == prepared
    assert checkpoint.trusted_sha == _git(trusted, "rev-parse", "HEAD")
    assert checkpoint.direct_oid is None


def test_absent_tag_skips_historical_non_utf8_manifest(
    repository: tuple[Path, Path], tmp_path: Path
) -> None:
    trusted, _remote = repository
    prepared = _git(trusted, "rev-parse", "HEAD")
    (trusted / release_checkpoint.release_state.MANIFEST_NAME).write_bytes(
        b"{\"target_version\":\"1.2.3\",\"invalid\":\xff}\n"
    )
    _git(trusted, "add", release_checkpoint.release_state.MANIFEST_NAME)
    _git(trusted, "commit", "-m", "historical invalid release metadata")
    _git(trusted, "push", "origin", "main")

    checkpoint = _resolve(trusted, tmp_path)

    assert checkpoint.candidate_sha == prepared
    assert checkpoint.trusted_sha == _git(trusted, "rev-parse", "HEAD")
    assert checkpoint.direct_oid is None


def test_absent_tag_rejects_multiple_digest_valid_prepared_commits(
    repository: tuple[Path, Path], tmp_path: Path
) -> None:
    trusted, _remote = repository
    _git(trusted, "commit", "--allow-empty", "-m", "duplicate prepared tree")

    with pytest.raises(release_checkpoint.CheckpointError, match="multiple"):
        _resolve(trusted, tmp_path)


def test_absent_tag_rejects_when_prepared_commit_is_beyond_history_bound(
    repository: tuple[Path, Path],
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    trusted, _remote = repository
    (trusted / "tracked.txt").write_text("first advancement\n", encoding="utf-8")
    _git(trusted, "add", "tracked.txt")
    _git(trusted, "commit", "-m", "first advancement")
    (trusted / "tracked.txt").write_text("second advancement\n", encoding="utf-8")
    _git(trusted, "add", "tracked.txt")
    _git(trusted, "commit", "-m", "second advancement")
    monkeypatch.setattr(release_checkpoint, "MAX_FIRST_PARENT_COMMITS", 2)

    with pytest.raises(release_checkpoint.CheckpointError, match="bounded.*2"):
        _resolve(trusted, tmp_path)


def test_absent_tag_accepts_previous_checkpoint_older_than_search_bound(
    repository: tuple[Path, Path], tmp_path: Path
) -> None:
    trusted, _remote = repository
    prepared = _git(trusted, "rev-parse", "HEAD")
    previous = _git(trusted, "rev-parse", "HEAD~1")
    _git(trusted, "reset", "--hard", previous)
    for index in range(release_checkpoint.MAX_FIRST_PARENT_COMMITS + 1):
        _git(
            trusted,
            "commit",
            "--allow-empty",
            "-m",
            f"intervening main commit {index}",
        )
    _git(trusted, "cherry-pick", prepared)
    _git(trusted, "push", "--force", "origin", "main")

    checkpoint = _resolve(trusted, tmp_path)

    assert checkpoint.candidate_sha == _git(trusted, "rev-parse", "HEAD")
    assert checkpoint.direct_oid is None
    assert (
        int(_git(trusted, "rev-list", "--first-parent", "--count", "v1.2.2..HEAD"))
        > release_checkpoint.MAX_FIRST_PARENT_COMMITS
    )


def test_absent_tag_rejects_zero_matching_prepared_commits(
    repository: tuple[Path, Path], tmp_path: Path
) -> None:
    trusted, _remote = repository

    with pytest.raises(release_checkpoint.CheckpointError, match="no digest-valid"):
        _resolve(trusted, tmp_path, version="9.9.9")


def test_absent_tag_rejects_missing_previous_release_checkpoint(
    repository: tuple[Path, Path], tmp_path: Path
) -> None:
    trusted, _remote = repository
    _delete_local_and_remote_tag(trusted, "v1.2.2")

    with pytest.raises(release_checkpoint.CheckpointError, match="does not exist"):
        _resolve(trusted, tmp_path)


def test_absent_tag_rejects_previous_tag_off_first_parent_history(
    repository: tuple[Path, Path], tmp_path: Path
) -> None:
    trusted, _remote = repository
    base = _git(trusted, "rev-parse", "HEAD~1")
    _delete_local_and_remote_tag(trusted, "v1.2.2")
    _git(trusted, "checkout", "-b", "side-previous-release", base)
    _git(trusted, "commit", "--allow-empty", "-m", "side previous release")
    _annotated_tag(trusted, "v1.2.2", "HEAD", "side previous release")
    _git(trusted, "push", "origin", "refs/tags/v1.2.2")
    _git(trusted, "checkout", "main")
    _git(
        trusted,
        "merge",
        "--no-ff",
        "side-previous-release",
        "-m",
        "merge side previous release",
    )
    _git(trusted, "push", "origin", "main")

    assert _git(trusted, "merge-base", "v1.2.2^{}", "HEAD") == _git(
        trusted, "rev-parse", "v1.2.2^{}"
    )

    with pytest.raises(release_checkpoint.CheckpointError, match="not on.*first-parent"):
        _resolve(trusted, tmp_path)


def test_absent_tag_rejects_candidate_before_previous_checkpoint(
    repository: tuple[Path, Path], tmp_path: Path
) -> None:
    trusted, _remote = repository
    candidate = _git(trusted, "rev-parse", "HEAD")
    (trusted / "tracked.txt").write_text("main advanced after candidate\n", encoding="utf-8")
    _git(trusted, "add", "tracked.txt")
    _git(trusted, "commit", "-m", "advance beyond prepared candidate")
    _delete_local_and_remote_tag(trusted, "v1.2.2")
    _annotated_tag(trusted, "v1.2.2", "HEAD", "misordered previous release")
    _git(trusted, "push", "origin", "refs/tags/v1.2.2")

    with pytest.raises(
        release_checkpoint.CheckpointError,
        match=rf"candidate {candidate}.*previous release checkpoint.*first-parent",
    ):
        _resolve(trusted, tmp_path)


def test_annotated_tag_preserves_direct_object_and_peeled_commit(
    repository: tuple[Path, Path], tmp_path: Path
) -> None:
    trusted, _remote = repository
    target = _git(trusted, "rev-parse", "HEAD")
    direct = _annotated_tag(trusted, "v1.2.3", target, "reviewed release")
    _git(trusted, "push", "origin", "refs/tags/v1.2.3")

    checkpoint = _resolve(trusted, tmp_path)

    assert checkpoint.direct_oid == direct
    assert checkpoint.candidate_sha == target


def test_lightweight_tag_is_rejected(
    repository: tuple[Path, Path], tmp_path: Path
) -> None:
    trusted, _remote = repository
    _git(trusted, "tag", "v1.2.3")
    _git(trusted, "push", "origin", "refs/tags/v1.2.3")

    with pytest.raises(release_checkpoint.CheckpointError, match="not an annotated"):
        _resolve(trusted, tmp_path)


def test_nested_annotated_tag_is_rejected(
    repository: tuple[Path, Path], tmp_path: Path
) -> None:
    trusted, _remote = repository
    _annotated_tag(trusted, "inner", "HEAD", "inner")
    _annotated_tag(trusted, "v1.2.3", "inner", "outer")
    _git(trusted, "push", "origin", "refs/tags/v1.2.3")

    with pytest.raises(release_checkpoint.CheckpointError, match="nested tags"):
        _resolve(trusted, tmp_path)


def test_tagged_side_commit_not_ancestor_of_dispatch_is_rejected(
    repository: tuple[Path, Path], tmp_path: Path
) -> None:
    trusted, _remote = repository
    base = _git(trusted, "rev-parse", "HEAD")
    _git(trusted, "checkout", "--detach", base)
    (trusted / "side.txt").write_text("side\n", encoding="utf-8")
    _git(trusted, "add", "side.txt")
    _git(trusted, "commit", "-m", "side")
    side = _git(trusted, "rev-parse", "HEAD")
    _annotated_tag(trusted, "v1.2.3", side, "side release")
    _git(trusted, "push", "origin", "refs/tags/v1.2.3")
    _git(trusted, "checkout", "main")

    with pytest.raises(release_checkpoint.CheckpointError, match="not an ancestor"):
        _resolve(trusted, tmp_path)


def test_existing_tag_is_valid_after_main_advances_with_empty_commit(
    repository: tuple[Path, Path], tmp_path: Path
) -> None:
    trusted, _remote = repository
    tagged = _git(trusted, "rev-parse", "HEAD")
    direct = _annotated_tag(trusted, "v1.2.3", tagged, "release")
    _git(trusted, "push", "origin", "refs/tags/v1.2.3")
    _git(trusted, "commit", "--allow-empty", "-m", "main advanced")
    _git(trusted, "push", "origin", "main")

    checkpoint = _resolve(trusted, tmp_path)

    assert checkpoint.direct_oid == direct
    assert checkpoint.candidate_sha == tagged
    assert checkpoint.trusted_sha != tagged


def test_lookup_fetch_mutation_is_rejected(
    repository: tuple[Path, Path], tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    trusted, _remote = repository
    target = _git(trusted, "rev-parse", "HEAD")
    _annotated_tag(trusted, "v1.2.3", target, "first")
    _git(trusted, "push", "origin", "refs/tags/v1.2.3")
    original_fetch = release_checkpoint._fetch_tag

    def mutate_then_fetch(repo: Path, remote: str, tag_name: str) -> None:
        replacement = _annotated_tag(trusted, tag_name, target, "changed during fetch")
        _force_remote_tag(trusted, tag_name, replacement)
        original_fetch(repo, remote, tag_name)

    monkeypatch.setattr(release_checkpoint, "_fetch_tag", mutate_then_fetch)

    with pytest.raises(release_checkpoint.CheckpointError, match="changed during"):
        _resolve(trusted, tmp_path)


def test_revalidation_rejects_moved_tag_object_with_same_target(
    repository: tuple[Path, Path], tmp_path: Path
) -> None:
    trusted, _remote = repository
    target = _git(trusted, "rev-parse", "HEAD")
    first = _annotated_tag(trusted, "v1.2.3", target, "first")
    _git(trusted, "push", "origin", "refs/tags/v1.2.3")
    checkpoint = _resolve(trusted, tmp_path)
    assert checkpoint.direct_oid == first
    replacement = _annotated_tag(trusted, "v1.2.3", target, "different object")
    _force_remote_tag(trusted, "v1.2.3", replacement)

    with pytest.raises(release_checkpoint.CheckpointError, match="changed during"):
        release_checkpoint.verify(
            trusted,
            remote="origin",
            candidate_path=tmp_path / "candidate",
            state_file=tmp_path / "checkpoint.json",
        )


def test_revalidation_rejects_moved_tag_target(
    repository: tuple[Path, Path], tmp_path: Path
) -> None:
    trusted, _remote = repository
    original = _git(trusted, "rev-parse", "HEAD")
    _annotated_tag(trusted, "v1.2.3", original, "first")
    _git(trusted, "push", "origin", "refs/tags/v1.2.3")
    _git(trusted, "commit", "--allow-empty", "-m", "new target")
    _resolve(trusted, tmp_path)
    replacement = _annotated_tag(trusted, "v1.2.3", "HEAD", "moved target")
    _force_remote_tag(trusted, "v1.2.3", replacement)

    with pytest.raises(release_checkpoint.CheckpointError, match="changed during"):
        release_checkpoint.verify(
            trusted,
            remote="origin",
            candidate_path=tmp_path / "candidate",
            state_file=tmp_path / "checkpoint.json",
        )


def test_same_deterministic_push_race_is_accepted(
    repository: tuple[Path, Path], tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    trusted, _remote = repository
    checkpoint = _resolve(trusted, tmp_path)

    def concurrent_identical_push(
        repo: Path, remote: str, direct_oid: str, tag_name: str
    ) -> int:
        _git(repo, "push", remote, f"{direct_oid}:refs/tags/{tag_name}")
        return 1

    monkeypatch.setattr(release_checkpoint, "_push_tag", concurrent_identical_push)
    created = release_checkpoint.create(
        trusted,
        remote="origin",
        candidate_path=tmp_path / "candidate",
        state_file=tmp_path / "checkpoint.json",
    )

    assert created.candidate_sha == checkpoint.candidate_sha
    assert created.direct_oid is not None


def test_same_checkpoint_created_before_create_step_is_accepted(
    repository: tuple[Path, Path], tmp_path: Path
) -> None:
    trusted, _remote = repository
    checkpoint = _resolve(trusted, tmp_path)
    direct = release_checkpoint._deterministic_tag(trusted, checkpoint)
    _git(trusted, "push", "origin", f"{direct}:refs/tags/{checkpoint.tag_name}")

    created = release_checkpoint.create(
        trusted,
        remote="origin",
        candidate_path=tmp_path / "candidate",
        state_file=tmp_path / "checkpoint.json",
    )

    assert created.direct_oid == direct


def test_different_push_race_is_rejected(
    repository: tuple[Path, Path], tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    trusted, _remote = repository
    checkpoint = _resolve(trusted, tmp_path)

    def concurrent_different_push(
        repo: Path, remote: str, _direct_oid: str, tag_name: str
    ) -> int:
        competing = _annotated_tag(repo, tag_name, checkpoint.candidate_sha, "competitor")
        _git(repo, "push", remote, f"{competing}:refs/tags/{tag_name}")
        return 1

    monkeypatch.setattr(release_checkpoint, "_push_tag", concurrent_different_push)

    with pytest.raises(release_checkpoint.CheckpointError, match="different checkpoint"):
        release_checkpoint.create(
            trusted,
            remote="origin",
            candidate_path=tmp_path / "candidate",
            state_file=tmp_path / "checkpoint.json",
        )


@pytest.mark.parametrize("phase", ["pre-publish", "pre-release"])
def test_irreversible_phase_recheck_detects_remote_mutation(
    repository: tuple[Path, Path],
    tmp_path: Path,
    phase: str,
) -> None:
    trusted, _remote = repository
    checkpoint = _resolve(trusted, tmp_path)
    created = release_checkpoint.create(
        trusted,
        remote="origin",
        candidate_path=tmp_path / "candidate",
        state_file=tmp_path / "checkpoint.json",
    )
    assert created.direct_oid is not None
    _delete_local_and_remote_tag(trusted, checkpoint.tag_name)
    moved = _annotated_tag(trusted, checkpoint.tag_name, "HEAD", f"moved {phase}")
    _force_remote_tag(trusted, checkpoint.tag_name, moved)

    with pytest.raises(release_checkpoint.CheckpointError, match="changed during"):
        release_checkpoint.verify(
            trusted,
            remote="origin",
            candidate_path=tmp_path / "candidate",
            state_file=tmp_path / "checkpoint.json",
        )
