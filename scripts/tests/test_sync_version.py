#!/usr/bin/env python3
"""Tests for scripts/sync-version.sh."""

from __future__ import annotations

import os
import re
import subprocess
from datetime import datetime, timezone
from pathlib import Path

# Resolve repository root from scripts/tests/ (two parent levels up).
REPO_ROOT = Path(__file__).resolve().parents[2]
SYNC_SCRIPT_SOURCE = REPO_ROOT / "scripts" / "sync-version.sh"


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


def _setup_repo(tmp_path: Path, changelog_content: str, version: str = "0.8.0") -> Path:
    repo = tmp_path / "repo"
    repo.mkdir(parents=True, exist_ok=True)

    (repo / "Cargo.toml").write_text(
        f'[package]\nname = "fortress-rollback"\nversion = "{version}"\n',
        encoding="utf-8",
    )
    (repo / "CHANGELOG.md").write_text(changelog_content, encoding="utf-8")
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
        ["bash", str(SYNC_SCRIPT_SOURCE), *args],
        cwd=REPO_ROOT,
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )


def _assert_no_manual_link_footer_intervention(result: subprocess.CompletedProcess[str]) -> None:
    combined = result.stdout + result.stderr
    assert "require manual intervention" not in combined
    assert "CHANGELOG.md (link footers)" not in combined


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
        ["bash", str(SYNC_SCRIPT_SOURCE), "--help"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )

    assert result.returncode == 0
    assert "FORTRESS_PROJECT_ROOT" in result.stdout
    assert "FORTRESS_RELEASE_DATE" in result.stdout
    assert "repository root containing" in result.stdout


def test_missing_cargo_version_reports_diagnostic(tmp_path: Path) -> None:
    repo = tmp_path / "repo"
    repo.mkdir(parents=True)
    _write(repo / "Cargo.toml", '[package]\nname = "fixture"\n')
    _write(repo / "CHANGELOG.md", "# Changelog\n")

    result = _run_sync(repo)

    assert result.returncode == 1
    assert "Could not extract version from Cargo.toml" in (
        result.stdout + result.stderr
    )


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


def test_sync_version_updates_unreleased_release_date_and_missing_links(tmp_path: Path) -> None:
    changelog = """# Changelog

## [Unreleased]

## [0.8.0]

### Added
- Thing

## [0.7.0] - 2026-01-01

### Added
- Prior thing

[Unreleased]: https://github.com/wallstop/fortress-rollback/compare/v0.7.0...HEAD
[0.7.0]: https://github.com/wallstop/fortress-rollback/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/wallstop/fortress-rollback/compare/v0.5.0...v0.6.0
"""
    repo = _setup_repo(tmp_path, changelog)
    result = _run_sync(repo, "--changelog-only")
    assert result.returncode == 0, result.stdout + result.stderr

    updated = (repo / "CHANGELOG.md").read_text(encoding="utf-8")
    assert re.search(r"^## \[0\.8\.0\] - \d{4}-\d{2}-\d{2}$", updated, re.MULTILINE)
    assert (
        "[Unreleased]: https://github.com/wallstop/fortress-rollback/compare/v0.8.0...HEAD"
        in updated
    )
    assert "[0.8.0]: https://github.com/wallstop/fortress-rollback/compare/v0.7.0...v0.8.0" in updated


def test_sync_version_check_mode_detects_missing_release_updates(tmp_path: Path) -> None:
    changelog = """# Changelog

## [Unreleased]

## [0.8.0]

### Added
- Thing

## [0.7.0] - 2026-01-01

[Unreleased]: https://github.com/wallstop/fortress-rollback/compare/v0.7.0...HEAD
[0.7.0]: https://github.com/wallstop/fortress-rollback/compare/v0.6.0...v0.7.0
"""
    repo = _setup_repo(tmp_path, changelog)
    result = _run_sync(repo, "--changelog-only", "--check")
    assert result.returncode == 1
    combined = result.stdout + result.stderr
    assert "CHANGELOG.md (release date)" in combined
    assert "CHANGELOG.md (link footers)" in combined


def test_sync_version_fixes_older_missing_link_even_if_current_exists(tmp_path: Path) -> None:
    changelog = """# Changelog

## [Unreleased]

## [0.8.0] - 2026-02-01

### Added
- Thing

## [0.7.0] - 2026-01-01

### Added
- Prior thing

## [0.6.0] - 2025-12-01

### Added
- Old thing

[Unreleased]: https://github.com/wallstop/fortress-rollback/compare/v0.8.0...HEAD
[0.8.0]: https://github.com/wallstop/fortress-rollback/compare/v0.7.0...v0.8.0
[0.6.0]: https://github.com/wallstop/fortress-rollback/compare/v0.5.0...v0.6.0
"""
    repo = _setup_repo(tmp_path, changelog)
    result = _run_sync(repo, "--changelog-only")
    assert result.returncode == 0, result.stdout + result.stderr
    _assert_no_manual_link_footer_intervention(result)

    updated = (repo / "CHANGELOG.md").read_text(encoding="utf-8")
    assert "[0.7.0]: https://github.com/wallstop/fortress-rollback/compare/v0.6.0...v0.7.0" in updated


def test_sync_version_normalizes_old_style_unreleased_compare_link(tmp_path: Path) -> None:
    changelog = """# Changelog

## [Unreleased]

## [0.8.0]

### Added
- Thing

## [0.7.0] - 2026-01-01

[Unreleased]: https://github.com/wallstop/fortress-rollback/compare/0.7.0...HEAD
[0.7.0]: https://github.com/wallstop/fortress-rollback/compare/v0.6.0...v0.7.0
"""
    repo = _setup_repo(tmp_path, changelog)
    result = _run_sync(repo, "--changelog-only")
    assert result.returncode == 0, result.stdout + result.stderr

    updated = (repo / "CHANGELOG.md").read_text(encoding="utf-8")
    assert (
        "[Unreleased]: https://github.com/wallstop/fortress-rollback/compare/v0.8.0...HEAD"
        in updated
    )


def test_sync_version_normalizes_old_style_unreleased_link_when_already_current(
    tmp_path: Path,
) -> None:
    changelog = """# Changelog

## [Unreleased]

## [0.8.0] - 2026-02-01

### Added
- Thing

## [0.7.0] - 2026-01-01

[Unreleased]: https://github.com/wallstop/fortress-rollback/compare/0.8.0...HEAD
[0.8.0]: https://github.com/wallstop/fortress-rollback/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/wallstop/fortress-rollback/compare/v0.6.0...v0.7.0
"""
    repo = _setup_repo(tmp_path, changelog)
    result = _run_sync(repo, "--changelog-only")
    assert result.returncode == 0, result.stdout + result.stderr
    _assert_no_manual_link_footer_intervention(result)

    updated = (repo / "CHANGELOG.md").read_text(encoding="utf-8")
    assert (
        "[Unreleased]: https://github.com/wallstop/fortress-rollback/compare/v0.8.0...HEAD"
        in updated
    )
    assert "[Unreleased]: https://github.com/wallstop/fortress-rollback/compare/0.8.0...HEAD" not in updated


def test_sync_version_normalizes_unreleased_with_extra_whitespace(tmp_path: Path) -> None:
    changelog = """# Changelog

## [Unreleased]

## [0.8.0]

### Added
- Thing

## [0.7.0] - 2026-01-01

[Unreleased]:   https://github.com/wallstop/fortress-rollback/compare/v0.7.0...HEAD    
[0.7.0]:\thttps://github.com/wallstop/fortress-rollback/compare/v0.6.0...v0.7.0
"""
    repo = _setup_repo(tmp_path, changelog)
    result = _run_sync(repo, "--changelog-only")
    assert result.returncode == 0, result.stdout + result.stderr

    updated = (repo / "CHANGELOG.md").read_text(encoding="utf-8")
    assert (
        "[Unreleased]: https://github.com/wallstop/fortress-rollback/compare/v0.8.0...HEAD"
        in updated
    )
    assert "[0.7.0]:\thttps://github.com/wallstop/fortress-rollback/compare/v0.6.0...v0.7.0" in updated


def test_sync_version_reports_anchor_missing_when_unreleased_footer_absent(tmp_path: Path) -> None:
    changelog = """# Changelog

## [Unreleased]

## [0.8.0] - 2026-02-01

### Added
- Thing

## [0.7.0] - 2026-01-01

### Added
- Prior thing

## [0.6.0] - 2025-12-01

### Added
- Old thing

[0.8.0]: https://github.com/wallstop/fortress-rollback/compare/v0.7.0...v0.8.0
[0.6.0]: https://github.com/wallstop/fortress-rollback/compare/v0.5.0...v0.6.0
"""
    repo = _setup_repo(tmp_path, changelog)
    result = _run_sync(repo, "--changelog-only")
    assert result.returncode == 1

    combined = result.stdout + result.stderr
    assert "Could not insert generated link footers" in combined
    assert "require manual intervention" in combined
    assert "✓ Added: CHANGELOG.md [0.7.0] link" not in combined

    updated = (repo / "CHANGELOG.md").read_text(encoding="utf-8")
    assert "[0.7.0]: https://github.com/wallstop/fortress-rollback/compare/v0.6.0...v0.7.0" not in updated


# --------------------------------------------------------------------------- #
# --stamp-release-date (refresh-at-release date stamping)
# --------------------------------------------------------------------------- #

_STAMP_CHANGELOG = """# Changelog

## [Unreleased]

## [0.9.0] - 2026-06-04

### Added
- A shipped feature

## [0.8.1] - 2026-05-16

### Added
- Older thing

[Unreleased]: https://github.com/wallstop/fortress-rollback/compare/v0.9.0...HEAD
[0.9.0]: https://github.com/wallstop/fortress-rollback/compare/v0.8.1...v0.9.0
[0.8.1]: https://github.com/wallstop/fortress-rollback/compare/v0.8.0...v0.8.1
"""


def _utc_today_candidates() -> set[str]:
    """UTC dates around now, tolerating a midnight rollover during the test."""
    return {datetime.now(timezone.utc).date().isoformat()}


def _current_version_header(changelog_text: str, version: str) -> str:
    esc = re.escape(version)
    m = re.search(rf"^## \[{esc}\].*$", changelog_text, re.MULTILINE)
    assert m, f"no '## [{version}]' header found"
    return m.group(0).rstrip()


def test_stamp_release_date_rewrites_existing_dated_header(tmp_path: Path) -> None:
    repo = _setup_repo(tmp_path, _STAMP_CHANGELOG, version="0.9.0")
    before = _utc_today_candidates()
    result = _run_sync(repo, "--stamp-release-date")
    after = _utc_today_candidates()
    assert result.returncode == 0, result.stdout + result.stderr

    text = (repo / "CHANGELOG.md").read_text(encoding="utf-8")
    header = _current_version_header(text, "0.9.0")
    valid_headers = {f"## [0.9.0] - {d}" for d in (before | after)}
    assert header in valid_headers, f"header was {header!r}, expected one of {valid_headers}"
    # The placeholder date must have been replaced.
    assert "## [0.9.0] - 2026-06-04" not in text
    # Other version headers are untouched.
    assert "## [0.8.1] - 2026-05-16" in text


def test_stamp_release_date_is_idempotent(tmp_path: Path) -> None:
    repo = _setup_repo(tmp_path, _STAMP_CHANGELOG, version="0.9.0")
    candidates = _utc_today_candidates()
    first = _run_sync(repo, "--stamp-release-date")
    candidates |= _utc_today_candidates()
    assert first.returncode == 0, first.stdout + first.stderr
    after_first = (repo / "CHANGELOG.md").read_text(encoding="utf-8")
    # The first run must have actually rewritten the placeholder date to today
    # (this is what fails if the --stamp-release-date feature is reverted).
    header_first = _current_version_header(after_first, "0.9.0")
    assert header_first in {f"## [0.9.0] - {d}" for d in candidates}, header_first
    assert "## [0.9.0] - 2026-06-04" not in after_first
    assert "Stamped:" in (first.stdout + first.stderr)

    second = _run_sync(repo, "--stamp-release-date")
    assert second.returncode == 0, second.stdout + second.stderr
    after_second = (repo / "CHANGELOG.md").read_text(encoding="utf-8")
    assert after_first == after_second
    # The second run must not re-stamp (no-op when already dated today).
    assert "Stamped:" not in (second.stdout + second.stderr)


def test_stamp_release_date_preserves_check_invariants(tmp_path: Path) -> None:
    """After stamping, the routine --check gate still passes."""
    repo = _setup_repo(tmp_path, _STAMP_CHANGELOG, version="0.9.0")
    assert _run_sync(repo, "--stamp-release-date").returncode == 0
    check = _run_sync(repo, "--check")
    assert check.returncode == 0, check.stdout + check.stderr


def test_stamp_release_date_targets_released_version_after_default_branch_bump(
    tmp_path: Path,
) -> None:
    """Post-publish stamping must not follow a concurrently bumped Cargo.toml."""
    changelog = """# Changelog

## [Unreleased]

## [0.10.0] - 2026-06-20

### Added
- Next in-flight feature

## [0.9.0] - 2026-06-04

### Added
- Shipped feature

## [0.8.1] - 2026-05-16

### Added
- Older thing

[Unreleased]: https://github.com/wallstop/fortress-rollback/compare/v0.10.0...HEAD
[0.10.0]: https://github.com/wallstop/fortress-rollback/compare/v0.9.0...v0.10.0
[0.9.0]: https://github.com/wallstop/fortress-rollback/compare/v0.8.1...v0.9.0
[0.8.1]: https://github.com/wallstop/fortress-rollback/compare/v0.8.0...v0.8.1
"""
    repo = _setup_repo(tmp_path, changelog, version="0.10.0")
    before = _utc_today_candidates()
    result = _run_sync(repo, "--stamp-release-date", "--release-version", "0.9.0")
    after = _utc_today_candidates()
    assert result.returncode == 0, result.stdout + result.stderr

    text = (repo / "CHANGELOG.md").read_text(encoding="utf-8")
    released_header = _current_version_header(text, "0.9.0")
    valid_released_headers = {f"## [0.9.0] - {d}" for d in (before | after)}
    assert released_header in valid_released_headers
    assert "## [0.10.0] - 2026-06-20" in text
    assert (
        "[Unreleased]: https://github.com/wallstop/fortress-rollback/compare/v0.10.0...HEAD"
        in text
    )


def test_stamp_release_date_missing_release_header_fails_with_diagnostic(
    tmp_path: Path,
) -> None:
    changelog = """# Changelog

## [Unreleased]

## [0.8.1] - 2026-05-16

### Added
- Older thing

[Unreleased]: https://github.com/wallstop/fortress-rollback/compare/v0.9.0...HEAD
[0.9.0]: https://github.com/wallstop/fortress-rollback/compare/v0.8.1...v0.9.0
[0.8.1]: https://github.com/wallstop/fortress-rollback/compare/v0.8.0...v0.8.1
"""
    repo = _setup_repo(tmp_path, changelog, version="0.9.0")
    result = _run_sync(repo, "--stamp-release-date")

    assert result.returncode == 1
    combined = result.stdout + result.stderr
    assert "Cannot stamp release date: no '## [0.9.0]' header found" in combined
    assert "CHANGELOG.md (release date)" in combined


def test_release_version_requires_stamp_release_date(tmp_path: Path) -> None:
    repo = _setup_repo(tmp_path, _STAMP_CHANGELOG, version="0.9.0")
    result = _run_sync(repo, "--release-version", "0.9.0")

    assert result.returncode == 1
    assert "--release-version requires --stamp-release-date" in (
        result.stdout + result.stderr
    )


def test_stamp_release_date_documented_in_help() -> None:
    # --help does not need a repo; run it directly against the script.
    result = subprocess.run(
        ["bash", str(SYNC_SCRIPT_SOURCE), "--help"],
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0
    assert "--stamp-release-date" in result.stdout
    assert "--release-version" in result.stdout
