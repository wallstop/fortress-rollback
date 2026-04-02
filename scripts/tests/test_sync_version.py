#!/usr/bin/env python3
"""Tests for scripts/sync-version.sh."""

from __future__ import annotations

import os
import subprocess
from pathlib import Path

# Resolve repository root from scripts/tests/ (two parent levels up).
REPO_ROOT = Path(__file__).resolve().parents[2]
SYNC_VERSION_SCRIPT = REPO_ROOT / "scripts" / "sync-version.sh"


def _write(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")


def _create_workspace(tmp_path: Path) -> Path:
    repo = tmp_path / "repo"
    _write(
        repo / "Cargo.toml",
        '[package]\nname = "fixture"\nversion = "1.2.3"\nedition = "2021"\n',
    )
    return repo


def init_git_repo(repo: Path) -> None:
    subprocess.run(["git", "init"], cwd=repo, check=True, capture_output=True, text=True)
    subprocess.run(
        ["git", "config", "user.email", "tests@example.com"],
        cwd=repo,
        check=True,
        capture_output=True,
        text=True,
    )
    subprocess.run(
        ["git", "config", "user.name", "Sync Version Tests"],
        cwd=repo,
        check=True,
        capture_output=True,
        text=True,
    )
    subprocess.run(["git", "add", "."], cwd=repo, check=True, capture_output=True, text=True)
    subprocess.run(
        ["git", "commit", "-m", "init"],
        cwd=repo,
        check=True,
        capture_output=True,
        text=True,
    )


def _run_sync(repo: Path, *args: str) -> subprocess.CompletedProcess[str]:
    env = os.environ.copy()
    env["FORTRESS_PROJECT_ROOT"] = str(repo)
    return subprocess.run(
        ["bash", str(SYNC_VERSION_SCRIPT), *args],
        cwd=REPO_ROOT,
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )


def test_updates_markdown_dependency_snippet(tmp_path: Path) -> None:
    repo = _create_workspace(tmp_path)
    docs_index = repo / "docs" / "index.md"
    _write(
        docs_index,
        "# Example\n\n```toml\n[dependencies]\nfortress-rollback = \"0.9\"\n```\n",
    )

    result = _run_sync(repo)

    assert result.returncode == 0, result.stdout + result.stderr
    # sync-version.sh normalizes dependency references to major.minor.
    assert docs_index.read_text(encoding="utf-8") == (
        "# Example\n\n```toml\n[dependencies]\nfortress-rollback = \"1.2\"\n```\n"
    )


def test_updates_wiki_home_dependency_snippet(tmp_path: Path) -> None:
    repo = _create_workspace(tmp_path)
    wiki_home = repo / "wiki" / "Home.md"
    _write(wiki_home, 'fortress-rollback = "0.9"\n')

    result = _run_sync(repo)

    assert result.returncode == 0, result.stdout + result.stderr
    assert wiki_home.read_text(encoding="utf-8") == 'fortress-rollback = "1.2"\n'


def test_updates_all_occurrences_in_single_file(tmp_path: Path) -> None:
    repo = _create_workspace(tmp_path)
    docs_index = repo / "docs" / "index.md"
    _write(
        docs_index,
        (
            'fortress-rollback = "0.9"\n'
            'fortress-rollback = { version = "0.9", features = ["tokio"] }\n'
            'fortress-rollback = "0.9"\n'
        ),
    )

    result = _run_sync(repo)

    assert result.returncode == 0, result.stdout + result.stderr
    updated = docs_index.read_text(encoding="utf-8")
    assert updated == (
        'fortress-rollback = "1.2"\n'
        'fortress-rollback = { version = "1.2", features = ["tokio"] }\n'
        'fortress-rollback = "1.2"\n'
    )


def test_check_mode_fails_when_stale_and_passes_when_synced(tmp_path: Path) -> None:
    repo = _create_workspace(tmp_path)
    docs_index = repo / "docs" / "index.md"
    _write(docs_index, 'fortress-rollback = "0.9"\n')

    check_stale = _run_sync(repo, "--check")
    assert check_stale.returncode == 1

    sync_result = _run_sync(repo)
    assert sync_result.returncode == 0, sync_result.stdout + sync_result.stderr

    check_synced = _run_sync(repo, "--check")
    assert check_synced.returncode == 0, check_synced.stdout + check_synced.stderr


def test_dry_run_does_not_modify_files(tmp_path: Path) -> None:
    repo = _create_workspace(tmp_path)
    docs_index = repo / "docs" / "index.md"
    original = 'fortress-rollback = "0.9"\n'
    _write(docs_index, original)

    result = _run_sync(repo, "--dry-run")

    assert result.returncode == 0, result.stdout + result.stderr
    assert docs_index.read_text(encoding="utf-8") == original
    assert "Would update:" in result.stdout


def test_help_documents_fortress_project_root() -> None:
    result = subprocess.run(
        ["bash", str(SYNC_VERSION_SCRIPT), "--help"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )

    assert result.returncode == 0
    assert "FORTRESS_PROJECT_ROOT" in result.stdout
    assert "repository root containing" in result.stdout


def test_check_mode_ignores_gitignored_untracked_outputs(tmp_path: Path) -> None:
    repo = _create_workspace(tmp_path)
    _write(repo / "docs" / "index.md", 'fortress-rollback = "1.2"\n')
    _write(repo / ".gitignore", "site/\n")
    _write(repo / "site" / "index.md", 'fortress-rollback = "0.9"\n')
    init_git_repo(repo)

    result = _run_sync(repo, "--check", "--verbose")

    assert result.returncode == 0, result.stdout + result.stderr
    assert "Discovery mode: git-tracked" in result.stdout
    assert "Skipping untracked and gitignored files via git ls-files." in result.stdout
    assert "site/index.md" not in result.stdout


def test_check_mode_still_fails_for_tracked_stale_files_with_git_discovery(
    tmp_path: Path,
) -> None:
    repo = _create_workspace(tmp_path)
    _write(repo / "docs" / "index.md", 'fortress-rollback = "0.9"\n')
    init_git_repo(repo)

    result = _run_sync(repo, "--check")

    assert result.returncode == 1


def test_check_mode_skips_deleted_tracked_files_in_git_discovery(tmp_path: Path) -> None:
    repo = _create_workspace(tmp_path)
    docs_index = repo / "docs" / "index.md"
    deleted = repo / "docs" / "deleted.md"
    _write(docs_index, 'fortress-rollback = "1.2"\n')
    _write(deleted, 'fortress-rollback = "0.9"\n')
    init_git_repo(repo)
    deleted.unlink()

    result = _run_sync(repo, "--check", "--verbose")

    assert result.returncode == 0, result.stdout + result.stderr
    assert "Discovery mode: git-tracked" in result.stdout
    assert "Files skipped (missing):" in result.stdout


def test_check_mode_uses_filesystem_fallback_when_not_in_git_repo(tmp_path: Path) -> None:
    repo = _create_workspace(tmp_path)
    _write(repo / "docs" / "index.md", 'fortress-rollback = "0.9"\n')

    result = _run_sync(repo, "--check", "--verbose")

    assert result.returncode == 1
    assert "Discovery mode: filesystem-fallback" in result.stdout


def test_filesystem_fallback_ignores_site_directory_content(tmp_path: Path) -> None:
    repo = _create_workspace(tmp_path)
    _write(repo / "docs" / "index.md", 'fortress-rollback = "1.2"\n')
    _write(repo / "site" / "index.md", 'fortress-rollback = "0.9"\n')

    result = _run_sync(repo, "--check", "--verbose")

    assert result.returncode == 0, result.stdout + result.stderr
    assert "Discovery mode: filesystem-fallback" in result.stdout
    assert "site/index.md" not in result.stdout
