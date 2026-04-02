#!/usr/bin/env python3
"""Tests for scripts/sync-version.sh CHANGELOG release updates."""

from __future__ import annotations

import re
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
SYNC_SCRIPT_SOURCE = REPO_ROOT / "scripts" / "sync-version.sh"


def _setup_repo(tmp_path: Path, changelog_content: str, version: str = "0.8.0") -> Path:
    repo = tmp_path / "repo"
    (repo / "scripts").mkdir(parents=True, exist_ok=True)
    shutil.copy2(SYNC_SCRIPT_SOURCE, repo / "scripts" / "sync-version.sh")
    (repo / "scripts" / "sync-version.sh").chmod(0o755)

    (repo / "Cargo.toml").write_text(
        f'[package]\nname = "fortress-rollback"\nversion = "{version}"\n',
        encoding="utf-8",
    )
    (repo / "CHANGELOG.md").write_text(changelog_content, encoding="utf-8")
    return repo


def _run_sync(repo: Path, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["bash", str(repo / "scripts" / "sync-version.sh"), *args],
        cwd=repo,
        capture_output=True,
        text=True,
        check=False,
    )


class TestSyncVersion(unittest.TestCase):
    def test_sync_version_updates_unreleased_release_date_and_missing_links(self) -> None:
        with tempfile.TemporaryDirectory() as tmp_dir:
            tmp_path = Path(tmp_dir)
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
            self.assertEqual(result.returncode, 0, msg=result.stdout + result.stderr)

            updated = (repo / "CHANGELOG.md").read_text(encoding="utf-8")
            self.assertIsNotNone(
                re.search(r"^## \[0\.8\.0\] - \d{4}-\d{2}-\d{2}$", updated, re.MULTILINE)
            )
            self.assertIn(
                "[Unreleased]: https://github.com/wallstop/fortress-rollback/compare/v0.8.0...HEAD",
                updated,
            )
            self.assertIn(
                "[0.8.0]: https://github.com/wallstop/fortress-rollback/compare/v0.7.0...v0.8.0",
                updated,
            )

    def test_sync_version_check_mode_detects_missing_release_updates(self) -> None:
        with tempfile.TemporaryDirectory() as tmp_dir:
            tmp_path = Path(tmp_dir)
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
            self.assertEqual(result.returncode, 1)
            combined = result.stdout + result.stderr
            self.assertIn("CHANGELOG.md (release date)", combined)
            self.assertIn("CHANGELOG.md (link footers)", combined)

    def test_sync_version_fixes_older_missing_link_even_if_current_exists(self) -> None:
        with tempfile.TemporaryDirectory() as tmp_dir:
            tmp_path = Path(tmp_dir)
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
            self.assertEqual(result.returncode, 0, msg=result.stdout + result.stderr)

            updated = (repo / "CHANGELOG.md").read_text(encoding="utf-8")
            self.assertIn(
                "[0.7.0]: https://github.com/wallstop/fortress-rollback/compare/v0.6.0...v0.7.0",
                updated,
            )

    def test_sync_version_normalizes_old_style_unreleased_compare_link(self) -> None:
        with tempfile.TemporaryDirectory() as tmp_dir:
            tmp_path = Path(tmp_dir)
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
            self.assertEqual(result.returncode, 0, msg=result.stdout + result.stderr)

            updated = (repo / "CHANGELOG.md").read_text(encoding="utf-8")
            self.assertIn(
                "[Unreleased]: https://github.com/wallstop/fortress-rollback/compare/v0.8.0...HEAD",
                updated,
            )


if __name__ == "__main__":
    unittest.main()
