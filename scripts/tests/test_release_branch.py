#!/usr/bin/env python3
"""Controlled Git and GitHub recovery tests for release preparation reruns."""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[2]
HELPER = REPO_ROOT / "scripts" / "release" / "release_branch.py"
RELEASE_STATE = REPO_ROOT / "scripts" / "release" / "release_state.py"
sys.path.insert(0, str(HELPER.parent))


def _load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


release_state = _load_module("release_state_fixture", RELEASE_STATE)


def _git(repo: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(repo), *args],
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, result.stdout + result.stderr
    return result.stdout.strip()


def _write_base(repo: Path) -> None:
    (repo / ".github" / "ISSUE_TEMPLATE").mkdir(parents=True)
    (repo / "Cargo.toml").write_text(
        '[package]\nname = "fortress-rollback"\nversion = "1.2.3"\n',
        encoding="utf-8",
    )
    (repo / "CHANGELOG.md").write_text(
        """# Changelog

## [Unreleased]

### Added

- A feature.

## [1.2.3] - 2026-01-01

- Previous release.
""",
        encoding="utf-8",
    )
    (repo / ".github" / "ISSUE_TEMPLATE" / "bug_report.yml").write_text(
        """body:
  - type: dropdown
    attributes:
      options:
        # BEGIN_FORTRESS_VERSIONS
        - v1.2.3
        # END_FORTRESS_VERSIONS
""",
        encoding="utf-8",
    )


def _write_prepared(repo: Path, release_date: str = "2026-07-18") -> None:
    (repo / "Cargo.toml").write_text(
        '[package]\nname = "fortress-rollback"\nversion = "1.3.0"\n',
        encoding="utf-8",
    )
    (repo / "CHANGELOG.md").write_text(
        f"""# Changelog

## [Unreleased]

## [1.3.0] - {release_date}

### Added

- A feature.

## [1.2.3] - 2026-01-01

- Previous release.
""",
        encoding="utf-8",
    )
    template = repo / ".github" / "ISSUE_TEMPLATE" / "bug_report.yml"
    template.write_text(
        template.read_text(encoding="utf-8").replace(
            "        - v1.2.3\n",
            "        - v1.3.0\n        - v1.2.3\n",
        ),
        encoding="utf-8",
    )
    release_state.generate(
        repo,
        previous_version="1.2.3",
        target_version="1.3.0",
        bump="minor",
        release_date=release_date,
    )


@pytest.fixture
def branch_fixture(tmp_path: Path):
    remote = tmp_path / "remote.git"
    work = tmp_path / "work"
    subprocess.run(["git", "init", "--bare", "--quiet", str(remote)], check=True)
    subprocess.run(["git", "init", "--quiet", "-b", "main", str(work)], check=True)
    _git(work, "config", "user.name", "Release Test")
    _git(work, "config", "user.email", "release@example.invalid")
    _git(work, "remote", "add", "origin", str(remote))
    _write_base(work)
    _git(work, "add", "--all")
    _git(work, "commit", "-m", "base")
    base_sha = _git(work, "rev-parse", "HEAD")
    _git(work, "push", "origin", "main")

    def publish_release(*, corrupt: bool = False) -> str:
        _git(work, "checkout", "-b", "release/v1.3.0", base_sha)
        _write_prepared(work)
        if corrupt:
            state_path = work / "release-state.json"
            state = json.loads(state_path.read_text(encoding="utf-8"))
            state["target_version"] = "1.4.0"
            state_path.write_text(json.dumps(state, sort_keys=True) + "\n", encoding="utf-8")
        _git(work, "add", "--all")
        _git(work, "commit", "-m", "Prepare v1.3.0 release")
        branch_sha = _git(work, "rev-parse", "HEAD")
        _git(work, "push", "origin", "release/v1.3.0")
        _git(work, "checkout", "--detach", base_sha)
        return branch_sha

    return work, base_sha, publish_release


def _resolve(module, repo: Path):
    return module.resolve_release_branch(
        repo,
        remote="origin",
        default_branch="main",
        previous_version="1.2.3",
        target_version="1.3.0",
        bump="minor",
        requested_date="2026-07-19",
    )


def test_resolve_absent_branch_preserves_base_checkout(branch_fixture) -> None:
    work, base_sha, _publish_release = branch_fixture
    module = _load_module("release_branch_absent", HELPER)

    resolution = _resolve(module, work)

    assert resolution.exists is False
    assert resolution.branch == "release/v1.3.0"
    assert resolution.base_sha == base_sha
    assert resolution.branch_sha is None
    assert resolution.replace_sha is None
    assert resolution.release_date == "2026-07-19"
    assert _git(work, "rev-parse", "HEAD") == base_sha


def test_resolve_matching_branch_reuses_immutable_date(
    branch_fixture, monkeypatch: pytest.MonkeyPatch
) -> None:
    work, base_sha, publish_release = branch_fixture
    branch_sha = publish_release()
    module = _load_module("release_branch_matching", HELPER)
    monkeypatch.setattr(
        module,
        "verify_prepared_candidate",
        lambda candidate, _base: release_state.verify(candidate),
    )

    resolution = _resolve(module, work)

    assert resolution.exists is True
    assert resolution.base_sha == base_sha
    assert resolution.branch_sha == branch_sha
    assert resolution.replace_sha is None
    assert resolution.release_date == "2026-07-18"
    assert _git(work, "rev-parse", "HEAD") == branch_sha


def test_resolve_conflicting_branch_fails_closed(branch_fixture) -> None:
    work, _base_sha, publish_release = branch_fixture
    publish_release(corrupt=True)
    module = _load_module("release_branch_conflict", HELPER)

    with pytest.raises(module.ReleaseBranchError, match="target_version"):
        _resolve(module, work)


def test_resolve_rejects_advanced_default_branch(branch_fixture) -> None:
    work, base_sha, _publish_release = branch_fixture
    _git(work, "checkout", "main")
    (work / "advanced.txt").write_text("new main\n", encoding="utf-8")
    _git(work, "add", "advanced.txt")
    _git(work, "commit", "-m", "advance main")
    _git(work, "push", "origin", "main")
    _git(work, "checkout", "--detach", base_sha)
    module = _load_module("release_branch_advanced_main", HELPER)

    with pytest.raises(module.ReleaseBranchError, match="is not current origin/main"):
        _resolve(module, work)


def test_resolve_rejects_existing_release_tag(branch_fixture) -> None:
    work, _base_sha, _publish_release = branch_fixture
    _git(work, "tag", "v1.3.0")
    _git(work, "push", "origin", "v1.3.0")
    module = _load_module("release_branch_existing_tag", HELPER)

    with pytest.raises(module.ReleaseBranchError, match="tag v1.3.0 already exists"):
        _resolve(module, work)


def test_resolve_rejects_release_branch_with_wrong_ancestry(branch_fixture) -> None:
    work, base_sha, publish_release = branch_fixture
    publish_release()
    _git(work, "checkout", "release/v1.3.0")
    (work / "extra.txt").write_text("second commit\n", encoding="utf-8")
    _git(work, "add", "extra.txt")
    _git(work, "commit", "-m", "unexpected second commit")
    _git(work, "push", "--force", "origin", "release/v1.3.0")
    _git(work, "checkout", "--detach", base_sha)
    module = _load_module("release_branch_wrong_ancestry", HELPER)

    with pytest.raises(module.ReleaseBranchError, match="not an ancestor"):
        _resolve(module, work)


def _prepare_main_update(work: Path, tmp_path: Path, marker: str) -> tuple[Path, str]:
    updater = tmp_path / f"updater-{marker}"
    remote = _git(work, "remote", "get-url", "origin")
    subprocess.run(["git", "clone", "--quiet", remote, str(updater)], check=True)
    _git(updater, "checkout", "main")
    _git(updater, "config", "user.name", "Main Update Test")
    _git(updater, "config", "user.email", "main-update@example.invalid")
    (updater / f"{marker}.txt").write_text("main advanced\n", encoding="utf-8")
    _git(updater, "add", "--all")
    _git(updater, "commit", "-m", f"advance main for {marker}")
    advanced_sha = _git(updater, "rev-parse", "HEAD")
    return updater, advanced_sha


def _advance_main(work: Path, tmp_path: Path, marker: str) -> tuple[Path, str]:
    updater, advanced_sha = _prepare_main_update(work, tmp_path, marker)
    _git(updater, "push", "origin", "main")
    _git(work, "fetch", "origin", "main")
    return updater, advanced_sha


def test_push_rejects_main_advancing_after_resolve(
    branch_fixture, tmp_path: Path
) -> None:
    work, base_sha, _publish_release = branch_fixture
    module = _load_module("release_branch_push_advanced_main", HELPER)
    resolution = _resolve(module, work)
    (work / "prepared.txt").write_text("prepared\n", encoding="utf-8")
    _git(work, "add", "--all")
    _git(work, "commit", "-m", "prepare release candidate")
    _advance_main(work, tmp_path, "before-push")

    with pytest.raises(module.ReleaseBranchError, match="is not current origin/main"):
        module.push_release_branch(
            work,
            remote="origin",
            default_branch="main",
            expected_base_sha=base_sha,
            branch=resolution.branch,
            expected_branch_sha=None,
        )

    assert (
        module._remote_ref(
            work, "origin", "heads", "refs/heads/release/v1.3.0"
        )
        is None
    )


def test_new_branch_push_lease_rejects_branch_creation_race(
    branch_fixture, monkeypatch: pytest.MonkeyPatch
) -> None:
    work, base_sha, _publish_release = branch_fixture
    module = _load_module("release_branch_creation_race", HELPER)
    resolution = _resolve(module, work)
    (work / "prepared.txt").write_text("prepared\n", encoding="utf-8")
    _git(work, "add", "--all")
    _git(work, "commit", "-m", "prepare release candidate")
    original_run_git_result = module._run_git_result
    raced = False

    def race_before_push(repo: Path, args: list[str]):
        nonlocal raced
        if not raced and args and args[0] == "push":
            raced = True
            _git(
                work,
                "push",
                "origin",
                f"{base_sha}:refs/heads/release/v1.3.0",
            )
        return original_run_git_result(repo, args)

    monkeypatch.setattr(module, "_run_git_result", race_before_push)

    with pytest.raises(module.ReleaseBranchError, match="compare-and-swap push"):
        module.push_release_branch(
            work,
            remote="origin",
            default_branch="main",
            expected_base_sha=base_sha,
            branch=resolution.branch,
            expected_branch_sha=None,
        )

    assert raced is True
    assert _git(work, "ls-remote", "origin", "refs/heads/release/v1.3.0").startswith(
        base_sha
    )


def test_atomic_push_rejects_main_race_without_creating_branch(
    branch_fixture, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    work, base_sha, _publish_release = branch_fixture
    module = _load_module("release_branch_atomic_main_race", HELPER)
    resolution = _resolve(module, work)
    (work / "prepared.txt").write_text("prepared\n", encoding="utf-8")
    _git(work, "add", "--all")
    _git(work, "commit", "-m", "prepare release candidate")
    updater, advanced_sha = _prepare_main_update(work, tmp_path, "atomic-create")
    original_run_git_result = module._run_git_result
    raced = False

    def race_before_atomic_push(repo: Path, args: list[str]):
        nonlocal raced
        if not raced and args and args[0] == "push":
            raced = True
            _git(updater, "push", "origin", "main")
        return original_run_git_result(repo, args)

    monkeypatch.setattr(module, "_run_git_result", race_before_atomic_push)

    with pytest.raises(module.ReleaseBranchError, match="compare-and-swap push"):
        module.push_release_branch(
            work,
            remote="origin",
            default_branch="main",
            expected_base_sha=base_sha,
            branch=resolution.branch,
            expected_branch_sha=None,
        )

    assert raced is True
    assert _git(work, "ls-remote", "origin", "refs/heads/main").startswith(
        advanced_sha
    )
    assert (
        module._remote_ref(
            work, "origin", "heads", "refs/heads/release/v1.3.0"
        )
        is None
    )


def test_resolve_recovers_generated_branch_after_main_advances(
    branch_fixture, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    work, _base_sha, publish_release = branch_fixture
    old_branch_sha = publish_release()
    _updater, advanced_sha = _advance_main(work, tmp_path, "recover")
    _git(work, "fetch", "origin", "main")
    _git(work, "checkout", "--detach", advanced_sha)
    module = _load_module("release_branch_stale_recovery", HELPER)
    monkeypatch.setattr(
        module,
        "verify_prepared_candidate",
        lambda candidate, _base: release_state.verify(candidate),
    )

    resolution = _resolve(module, work)

    assert resolution.exists is False
    assert resolution.base_sha == advanced_sha
    assert resolution.branch_sha == old_branch_sha
    assert resolution.replace_sha == old_branch_sha
    assert resolution.release_date == "2026-07-18"
    assert _git(work, "rev-parse", "HEAD") == advanced_sha

    _write_prepared(work, release_date=resolution.release_date)
    _git(work, "add", "--all")
    _git(work, "commit", "-m", "regenerate release on current main")
    new_branch_sha = module.push_release_branch(
        work,
        remote="origin",
        default_branch="main",
        expected_base_sha=advanced_sha,
        branch=resolution.branch,
        expected_branch_sha=resolution.replace_sha,
    )

    assert new_branch_sha != old_branch_sha
    assert _git(work, "ls-remote", "origin", "refs/heads/release/v1.3.0").startswith(
        new_branch_sha
    )


def test_stale_hostile_branch_fails_closed(branch_fixture, tmp_path: Path) -> None:
    work, base_sha, _publish_release = branch_fixture
    _git(work, "checkout", "-b", "release/v1.3.0", base_sha)
    (work / "hostile.txt").write_text("not generator output\n", encoding="utf-8")
    _git(work, "add", "--all")
    _git(work, "commit", "-m", "hostile release branch")
    _git(work, "push", "origin", "release/v1.3.0")
    _updater, advanced_sha = _advance_main(work, tmp_path, "hostile")
    _git(work, "checkout", "--detach", advanced_sha)
    module = _load_module("release_branch_stale_hostile", HELPER)

    with pytest.raises(module.ReleaseBranchError, match="invalid state"):
        _resolve(module, work)


def test_recovery_push_lease_rejects_branch_race(
    branch_fixture, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    work, _base_sha, publish_release = branch_fixture
    old_branch_sha = publish_release()
    _updater, advanced_sha = _advance_main(work, tmp_path, "lease")
    _git(work, "checkout", "--detach", advanced_sha)
    module = _load_module("release_branch_lease_race", HELPER)
    monkeypatch.setattr(
        module,
        "verify_prepared_candidate",
        lambda candidate, _base: release_state.verify(candidate),
    )
    resolution = _resolve(module, work)
    _write_prepared(work, release_date=resolution.release_date)
    _git(work, "add", "--all")
    _git(work, "commit", "-m", "regenerate release on current main")

    remote = _git(work, "remote", "get-url", "origin")
    racer = tmp_path / "branch-racer"
    subprocess.run(["git", "clone", "--quiet", remote, str(racer)], check=True)
    _git(racer, "checkout", "release/v1.3.0")
    _git(racer, "config", "user.name", "Branch Race Test")
    _git(racer, "config", "user.email", "branch-race@example.invalid")
    (racer / "race.txt").write_text("concurrent update\n", encoding="utf-8")
    _git(racer, "add", "--all")
    _git(racer, "commit", "-m", "race release recovery")
    racing_sha = _git(racer, "rev-parse", "HEAD")

    original_run_git_result = module._run_git_result
    raced = False

    def race_before_push(repo: Path, args: list[str]):
        nonlocal raced
        if not raced and args and args[0] == "push":
            raced = True
            _git(racer, "push", "--force", "origin", "HEAD:release/v1.3.0")
        return original_run_git_result(repo, args)

    monkeypatch.setattr(module, "_run_git_result", race_before_push)

    with pytest.raises(module.ReleaseBranchError, match="compare-and-swap push"):
        module.push_release_branch(
            work,
            remote="origin",
            default_branch="main",
            expected_base_sha=advanced_sha,
            branch=resolution.branch,
            expected_branch_sha=old_branch_sha,
        )

    assert raced is True
    assert _git(work, "ls-remote", "origin", "refs/heads/release/v1.3.0").startswith(
        racing_sha
    )


def test_atomic_recovery_rejects_main_race_without_replacing_branch(
    branch_fixture, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    work, _base_sha, publish_release = branch_fixture
    old_branch_sha = publish_release()
    _updater, advanced_sha = _advance_main(work, tmp_path, "atomic-recovery-base")
    _git(work, "checkout", "--detach", advanced_sha)
    module = _load_module("release_branch_atomic_recovery_race", HELPER)
    monkeypatch.setattr(
        module,
        "verify_prepared_candidate",
        lambda candidate, _base: release_state.verify(candidate),
    )
    resolution = _resolve(module, work)
    _write_prepared(work, release_date=resolution.release_date)
    _git(work, "add", "--all")
    _git(work, "commit", "-m", "regenerate release on current main")
    updater, raced_main_sha = _prepare_main_update(
        work, tmp_path, "atomic-recovery-race"
    )
    original_run_git_result = module._run_git_result
    raced = False

    def race_before_atomic_push(repo: Path, args: list[str]):
        nonlocal raced
        if not raced and args and args[0] == "push":
            raced = True
            _git(updater, "push", "origin", "main")
        return original_run_git_result(repo, args)

    monkeypatch.setattr(module, "_run_git_result", race_before_atomic_push)

    with pytest.raises(module.ReleaseBranchError, match="compare-and-swap push"):
        module.push_release_branch(
            work,
            remote="origin",
            default_branch="main",
            expected_base_sha=advanced_sha,
            branch=resolution.branch,
            expected_branch_sha=old_branch_sha,
        )

    assert raced is True
    assert _git(work, "ls-remote", "origin", "refs/heads/main").startswith(
        raced_main_sha
    )
    assert _git(work, "ls-remote", "origin", "refs/heads/release/v1.3.0").startswith(
        old_branch_sha
    )


def _rest_pull(
    head_sha: str,
    *,
    state: str = "open",
    head_repository: str = "wallstop/fortress-rollback",
    head_ref: str = "release/v1.3.0",
    base_ref: str = "main",
    merged_at: str | None = None,
) -> dict[str, object]:
    return {
        "number": 253,
        "state": state,
        "html_url": "https://github.com/wallstop/fortress-rollback/pull/253",
        "head": {
            "ref": head_ref,
            "sha": head_sha,
            "repo": {"full_name": head_repository},
        },
        "base": {"ref": base_ref},
        "merged_at": merged_at,
    }


def test_branch_without_pr_creates_pr(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    module = _load_module("release_branch_create_pr", HELPER)
    calls: list[list[str]] = []
    body_file = tmp_path / "release-pr.md"
    body_file.write_text("Release body.\n", encoding="utf-8")

    def fake_gh(args: list[str], description: str) -> str:
        calls.append(args)
        if description == "release pull request lookup":
            return "[]"
        assert description == "release pull request creation"
        return json.dumps(
            {
                "html_url": "https://github.com/wallstop/fortress-rollback/pull/999",
                "number": 999,
            }
        )

    monkeypatch.setattr(module, "_run_gh", fake_gh)

    result = module.ensure_release_pr(
        repository="wallstop/fortress-rollback",
        base="main",
        head="release/v1.3.0",
        head_sha="a" * 40,
        title="Prepare v1.3.0 release",
        body_file=body_file,
    )

    assert result.created is True
    assert result.number == 999
    assert result.url.endswith("/pull/999")
    assert [call[:3] for call in calls] == [
        ["api", "--method", "GET"],
        ["api", "--method", "POST"],
    ]
    assert "head=wallstop:release/v1.3.0" in calls[0]
    assert "head=wallstop:release/v1.3.0" in calls[1]


def test_matching_branch_and_pr_reuses_open_pr(monkeypatch: pytest.MonkeyPatch) -> None:
    module = _load_module("release_branch_existing_pr", HELPER)
    head_sha = "b" * 40
    existing = [_rest_pull(head_sha)]

    monkeypatch.setattr(
        module,
        "_run_gh",
        lambda args, _description: json.dumps(existing),
    )

    result = module.ensure_release_pr(
        repository="wallstop/fortress-rollback",
        base="main",
        head="release/v1.3.0",
        head_sha=head_sha,
        title="Prepare v1.3.0 release",
        body_file=Path("release-pr.md"),
    )

    assert result.created is False
    assert result.number == 253
    assert result.url.endswith("/pull/253")


def test_closed_current_canonical_pr_is_reopened(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    module = _load_module("release_branch_reopen_pr", HELPER)
    head_sha = "c" * 40
    calls: list[str] = []

    def fake_gh(_args: list[str], description: str) -> str:
        calls.append(description)
        if description == "release pull request lookup":
            return json.dumps([_rest_pull(head_sha, state="closed")])
        return json.dumps(_rest_pull(head_sha, state="open"))

    monkeypatch.setattr(module, "_run_gh", fake_gh)
    result = module.ensure_release_pr(
        repository="wallstop/fortress-rollback",
        base="main",
        head="release/v1.3.0",
        head_sha=head_sha,
        title="Prepare v1.3.0 release",
        body_file=Path("release-pr.md"),
    )

    assert result.created is False
    assert result.number == 253
    assert calls == ["release pull request lookup", "release pull request reopen"]


def test_stale_merged_canonical_pr_allows_new_current_pr(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    module = _load_module("release_branch_after_merged_pr", HELPER)
    calls: list[str] = []
    body_file = tmp_path / "release-pr.md"
    body_file.write_text("Release body.\n", encoding="utf-8")

    def fake_gh(_args: list[str], description: str) -> str:
        calls.append(description)
        if description == "release pull request lookup":
            return json.dumps(
                [_rest_pull("a" * 40, state="closed", merged_at="2026-07-17T00:00:00Z")]
            )
        return json.dumps(
            {
                "html_url": "https://github.com/wallstop/fortress-rollback/pull/999",
                "number": 999,
            }
        )

    monkeypatch.setattr(module, "_run_gh", fake_gh)
    result = module.ensure_release_pr(
        repository="wallstop/fortress-rollback",
        base="main",
        head="release/v1.3.0",
        head_sha="b" * 40,
        title="Prepare v1.3.0 release",
        body_file=body_file,
    )

    assert result.created is True
    assert calls == ["release pull request lookup", "release pull request creation"]


@pytest.mark.parametrize(
    ("pulls", "message"),
    [
        ([_rest_pull("d" * 40)], "does not match"),
        (
            [_rest_pull("e" * 40, head_repository="attacker/fortress-rollback")],
            "mismatched",
        ),
        ([_rest_pull("e" * 40, head_ref="release/v9.9.9")], "mismatched"),
        ([_rest_pull("e" * 40, base_ref="develop")], "mismatched"),
        ([_rest_pull("e" * 40), _rest_pull("e" * 40)], "found 2 open pull requests"),
    ],
)
def test_closed_mismatched_multiple_or_fork_pull_request_fails_closed(
    pulls: list[dict[str, object]],
    message: str,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    module = _load_module(f"release_branch_pr_conflict_{len(message)}", HELPER)
    monkeypatch.setattr(
        module,
        "_run_gh",
        lambda _args, _description: json.dumps(pulls),
    )

    with pytest.raises(module.ReleaseBranchError, match=message):
        module.ensure_release_pr(
            repository="wallstop/fortress-rollback",
            base="main",
            head="release/v1.3.0",
            head_sha="e" * 40,
            title="Prepare v1.3.0 release",
            body_file=Path("release-pr.md"),
        )
