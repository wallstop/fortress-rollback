#!/usr/bin/env python3
"""Regression tests for the automated release preparation workflow."""

from __future__ import annotations

import importlib.util
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[2]
PREPARE_SCRIPT = REPO_ROOT / "scripts" / "release" / "prepare_release.py"
sys.path.insert(0, str(PREPARE_SCRIPT.parent))
SPEC = importlib.util.spec_from_file_location("prepare_release", PREPARE_SCRIPT)
assert SPEC is not None and SPEC.loader is not None
prepare_release = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = prepare_release
SPEC.loader.exec_module(prepare_release)


def _write_fixture(
    tmp_path: Path,
    *,
    notes: str = "### Added\n\n- A feature.\n",
    current_version: str = "1.2.3",
) -> Path:
    repo = tmp_path / "repo"
    repo.mkdir()
    (repo / "Cargo.toml").write_text(
        f'[package]\nname = "fortress-rollback"\nversion = "{current_version}"\n',
        encoding="utf-8",
    )
    current_requirement = ".".join(current_version.split(".")[:2])
    (repo / "README.md").write_text(
        f'fortress-rollback = "{current_requirement}"\n', encoding="utf-8"
    )
    sync_script = repo / "scripts" / "sync-version.sh"
    sync_script.parent.mkdir()
    shutil.copy2(REPO_ROOT / "scripts" / "sync-version.sh", sync_script)
    (repo / "src").mkdir()
    (repo / "src" / "lib.rs").write_text(
        "pub fn fixture() {}\n", encoding="utf-8"
    )
    (repo / "CHANGELOG.md").write_text(
        """# Changelog

## [Unreleased]

"""
        + notes
        + """
## [CURRENT_VERSION] - 2026-01-01

- Previous release.

[Unreleased]: https://github.com/wallstop/fortress-rollback/compare/vCURRENT_VERSION...HEAD
[CURRENT_VERSION]: https://github.com/wallstop/fortress-rollback/releases/tag/vCURRENT_VERSION
""",
        encoding="utf-8",
    )
    changelog_path = repo / "CHANGELOG.md"
    changelog_path.write_text(
        changelog_path.read_text(encoding="utf-8").replace(
            "CURRENT_VERSION", current_version
        ),
        encoding="utf-8",
    )
    standalone = {
        "fuzz": ("fixture-fuzz", ".."),
        "loom-tests": ("fixture-loom", ".."),
        "tests/godot-emscripten": ("fixture-godot", "../.."),
    }
    for relative, (name, dependency_path) in standalone.items():
        directory = repo / relative
        (directory / "src").mkdir(parents=True)
        (directory / "Cargo.toml").write_text(
            f"""[package]
name = "{name}"
version = "0.0.0"
edition = "2021"

[workspace]

[dependencies]
fortress-rollback = {{ path = "{dependency_path}" }}
""",
            encoding="utf-8",
        )
        (directory / "src" / "lib.rs").write_text(
            "pub fn fixture() {}\n", encoding="utf-8"
        )
    for manifest in (
        "Cargo.toml",
        "fuzz/Cargo.toml",
        "loom-tests/Cargo.toml",
        "tests/godot-emscripten/Cargo.toml",
    ):
        result = subprocess.run(
            ["cargo", "generate-lockfile", "--manifest-path", manifest],
            cwd=repo,
            capture_output=True,
            text=True,
            check=False,
        )
        assert result.returncode == 0, result.stdout + result.stderr
    subprocess.run(["git", "init", "--quiet"], cwd=repo, check=True)
    subprocess.run(["git", "add", "--all"], cwd=repo, check=True)
    return repo


def _run(repo: Path, bump: str, *extra: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            "python3",
            str(PREPARE_SCRIPT),
            "--repo-root",
            str(repo),
            "--bump",
            bump,
            "--date",
            "2026-07-12",
            *extra,
        ],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )


@pytest.mark.parametrize(
    ("version", "message"),
    [
        (
            f"{'9' * 4_301}.0.0",
            "maximum length for u64 SemVer components",
        ),
        ("18446744073709551616.0.0", "larger than u64"),
        ("0.18446744073709551616.0", "larger than u64"),
        ("0.0.18446744073709551616", "larger than u64"),
    ],
)
def test_parse_version_rejects_unbounded_semver_components(
    version: str, message: str
) -> None:
    with pytest.raises(prepare_release.PreparationError, match=message):
        prepare_release.parse_version(version)


@pytest.mark.parametrize(
    ("version", "bump", "component"),
    [
        ("18446744073709551615.0.0", "major", "major"),
        ("1.18446744073709551615.0", "minor", "minor"),
        ("1.2.18446744073709551615", "patch", "patch"),
    ],
)
def test_bump_version_rejects_u64_component_overflow(
    version: str, bump: str, component: str
) -> None:
    with pytest.raises(
        prepare_release.PreparationError,
        match=rf"cannot bump {component} version component beyond u64",
    ):
        prepare_release.bump_version(version, bump)


@pytest.mark.parametrize(
    ("bump", "notes", "expected", "expected_dependency"),
    [
        ("patch", "### Fixed\n\n- A fix.\n", "1.2.4", "1.2"),
        ("minor", "### Added\n\n- A feature.\n", "1.3.0", "1.3"),
        ("major", "### Removed\n\n- An old API.\n", "2.0.0", "2.0"),
    ],
)
def test_prepare_release_bumps_manifest_all_locks_and_changelog(
    tmp_path: Path,
    bump: str,
    notes: str,
    expected: str,
    expected_dependency: str,
) -> None:
    repo = _write_fixture(tmp_path, notes=notes)

    result = _run(repo, bump)

    assert result.returncode == 0, result.stdout + result.stderr
    assert f'version = "{expected}"' in (repo / "Cargo.toml").read_text()
    for lock_path in (
        repo / "Cargo.lock",
        repo / "fuzz" / "Cargo.lock",
        repo / "loom-tests" / "Cargo.lock",
        repo / "tests" / "godot-emscripten" / "Cargo.lock",
    ):
        lock = lock_path.read_text(encoding="utf-8")
        assert f'name = "fortress-rollback"\nversion = "{expected}"' in lock
    changelog = (repo / "CHANGELOG.md").read_text()
    assert f"## [{expected}] - 2026-07-12\n\n{notes.strip()}" in changelog
    expected_link = (
        f"[{expected}]: https://github.com/wallstop/fortress-rollback/compare/"
        f"v1.2.3...v{expected}"
    )
    assert expected_link in changelog
    expected_unreleased_link = (
        "[Unreleased]: https://github.com/wallstop/fortress-rollback/compare/"
        f"v{expected}...HEAD"
    )
    assert expected_unreleased_link in changelog
    assert (
        repo / "README.md"
    ).read_text() == f'fortress-rollback = "{expected_dependency}"\n'
    assert f"prepared_version={expected}" in result.stdout


def test_prepare_release_dry_run_does_not_write(tmp_path: Path) -> None:
    repo = _write_fixture(tmp_path)
    before = {
        path.relative_to(repo): path.read_bytes()
        for path in repo.rglob("*")
        if path.is_file() and ".git" not in path.parts
    }

    result = _run(repo, "minor", "--dry-run")

    assert result.returncode == 0, result.stdout + result.stderr
    after = {
        path.relative_to(repo): path.read_bytes()
        for path in repo.rglob("*")
        if path.is_file() and ".git" not in path.parts
    }
    assert after == before
    assert "--- Cargo.toml" in result.stdout
    assert "--- fuzz/Cargo.lock" in result.stdout
    assert "--- loom-tests/Cargo.lock" in result.stdout
    assert "--- tests/godot-emscripten/Cargo.lock" in result.stdout
    assert "--- README.md" in result.stdout
    assert "+fortress-rollback = \"1.3\"" in result.stdout
    assert (
        "+[Unreleased]: https://github.com/wallstop/fortress-rollback/compare/"
        "v1.3.0...HEAD"
    ) in result.stdout
    assert "prepared_version=1.3.0" in result.stdout


def test_prepare_release_rejects_empty_unreleased_notes(tmp_path: Path) -> None:
    repo = _write_fixture(tmp_path, notes="")

    result = _run(repo, "patch")

    assert result.returncode != 0
    assert "Unreleased section has no release notes" in result.stderr
    assert 'version = "1.2.3"' in (repo / "Cargo.toml").read_text()


def test_prepare_release_rejects_bump_below_changelog_minimum(
    tmp_path: Path,
) -> None:
    repo = _write_fixture(tmp_path, notes="### Added\n\n- A feature.\n")

    result = _run(repo, "patch")

    assert result.returncode != 0
    assert "requested patch bump is below the minimum minor bump" in result.stderr
    assert 'version = "1.2.3"' in (repo / "Cargo.toml").read_text()


def test_prepare_release_breaking_pre_one_notes_prepare_minor_version(
    tmp_path: Path,
) -> None:
    repo = _write_fixture(
        tmp_path,
        current_version="0.10.0",
        notes="### Changed\n\n- **Breaking:** The wire format changed.\n",
    )

    result = _run(repo, "minor")

    assert result.returncode == 0, result.stdout + result.stderr
    assert "current_version=0.10.0" in result.stdout
    assert "prepared_version=0.11.0" in result.stdout
    assert 'version = "0.11.0"' in (repo / "Cargo.toml").read_text()
    assert "## [0.11.0] - 2026-07-12" in (repo / "CHANGELOG.md").read_text()


def test_prepare_release_rejects_lockfile_version_mismatch(tmp_path: Path) -> None:
    repo = _write_fixture(tmp_path)
    lock_path = repo / "Cargo.lock"
    lock_path.write_text(
        lock_path.read_text().replace(
            'name = "fortress-rollback"\nversion = "1.2.3"',
            'name = "fortress-rollback"\nversion = "1.2.2"',
        )
    )

    result = _run(repo, "minor")

    assert result.returncode != 0
    assert "local package 'fortress-rollback' version '1.2.2'" in result.stderr
    assert "does not match Cargo.toml version '1.2.3'" in result.stderr
    assert 'version = "1.2.3"' in (repo / "Cargo.toml").read_text()


def test_prepare_release_version_sync_failure_leaves_repository_unchanged(
    tmp_path: Path,
) -> None:
    repo = _write_fixture(tmp_path)
    sync_script = repo / "scripts" / "sync-version.sh"
    sync_script.write_text("#!/bin/bash\nexit 7\n", encoding="utf-8")
    before = {
        path.relative_to(repo): path.read_bytes()
        for path in repo.rglob("*")
        if path.is_file() and ".git" not in path.parts
    }

    result = _run(repo, "minor")

    assert result.returncode != 0
    assert "version synchronization update failed with exit code 7" in result.stderr
    after = {
        path.relative_to(repo): path.read_bytes()
        for path in repo.rglob("*")
        if path.is_file() and ".git" not in path.parts
    }
    assert after == before


def test_prepare_release_rejects_missing_tracked_sandbox_input(tmp_path: Path) -> None:
    repo = _write_fixture(tmp_path)
    (repo / "fuzz" / "Cargo.toml").unlink()

    result = _run(repo, "patch", "--dry-run")

    assert result.returncode != 0
    assert "tracked file fuzz/Cargo.toml is missing" in result.stderr
    assert str(repo) not in result.stderr


def test_prepare_release_rejects_tracked_symlink_escaping_sandbox(
    tmp_path: Path,
) -> None:
    repo = _write_fixture(tmp_path)
    outside = tmp_path / "outside.md"
    outside.write_text('fortress-rollback = "1.2"\n', encoding="utf-8")
    (repo / "escape.md").symlink_to(outside)
    subprocess.run(["git", "add", "--all"], cwd=repo, check=True)

    result = _run(repo, "minor", "--dry-run")

    assert result.returncode != 0
    assert "tracked symlink escape.md escapes release sandbox" in result.stderr
    assert outside.read_text(encoding="utf-8") == 'fortress-rollback = "1.2"\n'


def test_prepare_release_preserves_force_tracked_ignored_outputs(tmp_path: Path) -> None:
    repo = _write_fixture(tmp_path)
    (repo / ".gitignore").write_text("progress/\n", encoding="utf-8")
    ignored_output = repo / "progress" / "release-note.md"
    ignored_output.parent.mkdir()
    ignored_output.write_text('fortress-rollback = "1.2"\n', encoding="utf-8")
    subprocess.run(["git", "add", ".gitignore"], cwd=repo, check=True)
    subprocess.run(
        ["git", "add", "--force", "progress/release-note.md"], cwd=repo, check=True
    )

    result = _run(repo, "minor")

    assert result.returncode == 0, result.stdout + result.stderr
    assert ignored_output.read_text(encoding="utf-8") == 'fortress-rollback = "1.3"\n'


def test_prepare_release_rejects_existing_target_section(tmp_path: Path) -> None:
    repo = _write_fixture(tmp_path)
    changelog_path = repo / "CHANGELOG.md"
    changelog_path.write_text(
        changelog_path.read_text().replace(
            "## [1.2.3] - 2026-01-01", "## [1.2.4] - 2026-01-01"
        )
    )

    result = _run(repo, "patch")

    assert result.returncode != 0
    assert "already contains a 1.2.4 release heading" in result.stderr


def test_apply_failure_restores_already_replaced_files(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    first = tmp_path / "first.txt"
    second = tmp_path / "second.txt"
    first.write_text("first-before\n", encoding="utf-8")
    second.write_text("second-before\n", encoding="utf-8")
    prepared = [
        prepare_release.PreparedFile(first, "first-before\n", "first-after\n"),
        prepare_release.PreparedFile(second, "second-before\n", "second-after\n"),
    ]
    real_atomic_write = prepare_release._atomic_write
    calls = 0

    def fail_second_write(path: Path, content: str) -> None:
        nonlocal calls
        calls += 1
        if calls == 2:
            raise OSError("injected late write failure")
        real_atomic_write(path, content)

    monkeypatch.setattr(prepare_release, "_atomic_write", fail_second_write)

    with pytest.raises(prepare_release.PreparationError, match="injected late write failure"):
        prepare_release.apply_prepared(prepared)

    assert first.read_text(encoding="utf-8") == "first-before\n"
    assert second.read_text(encoding="utf-8") == "second-before\n"


def test_apply_rejects_concurrent_change_before_any_write(tmp_path: Path) -> None:
    first = tmp_path / "first.txt"
    second = tmp_path / "second.txt"
    first.write_text("first-before\n", encoding="utf-8")
    second.write_text("second-concurrent-change\n", encoding="utf-8")
    prepared = [
        prepare_release.PreparedFile(first, "first-before\n", "first-after\n"),
        prepare_release.PreparedFile(second, "second-before\n", "second-after\n"),
    ]

    with pytest.raises(prepare_release.PreparationError, match="changed during"):
        prepare_release.apply_prepared(prepared)

    assert first.read_text(encoding="utf-8") == "first-before\n"
    assert second.read_text(encoding="utf-8") == "second-concurrent-change\n"


def test_prepare_dynamic_output_keeps_pre_sandbox_baseline(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    repo = _write_fixture(tmp_path)
    readme = repo / "README.md"
    real_run_version_sync = prepare_release._run_version_sync

    def run_version_sync_with_concurrent_edit(
        sandbox: Path, release_date: str, *, check: bool
    ) -> None:
        real_run_version_sync(sandbox, release_date, check=check)
        if not check:
            readme.write_text("concurrent edit\n", encoding="utf-8")

    monkeypatch.setattr(
        prepare_release, "_run_version_sync", run_version_sync_with_concurrent_edit
    )

    _current, _target, prepared_files, _roots = prepare_release.prepare(
        repo, "minor", "2026-07-12"
    )
    readme_output = next(
        prepared for prepared in prepared_files if prepared.path == readme
    )

    assert readme_output.before == 'fortress-rollback = "1.2"\n'
    assert readme_output.after == 'fortress-rollback = "1.3"\n'
    with pytest.raises(prepare_release.PreparationError, match="changed during"):
        prepare_release.apply_prepared(prepared_files)
    assert readme.read_text(encoding="utf-8") == "concurrent edit\n"
    assert 'version = "1.2.3"' in (repo / "Cargo.toml").read_text()


def test_prepared_release_tree_remains_valid_and_rejects_second_preparation(
    tmp_path: Path,
) -> None:
    repo = _write_fixture(tmp_path, notes="### Added\n\n- A feature.\n")

    first = _run(repo, "minor")

    assert first.returncode == 0, first.stdout + first.stderr
    changelog = (repo / "CHANGELOG.md").read_text(encoding="utf-8")
    assert "## [Unreleased]\n\n## [1.3.0] - 2026-07-12" in changelog
    prepare_release.workspace_locks.check(repo)
    environment = prepare_release.os.environ.copy()
    environment["FORTRESS_PROJECT_ROOT"] = str(repo)
    check = subprocess.run(
        ["bash", str(repo / "scripts" / "sync-version.sh"), "--check"],
        cwd=repo,
        env=environment,
        capture_output=True,
        text=True,
        check=False,
    )
    assert check.returncode == 0, check.stdout + check.stderr
    before_second = {
        path.relative_to(repo): path.read_bytes()
        for path in repo.rglob("*")
        if path.is_file() and ".git" not in path.parts
    }

    second = _run(repo, "minor")

    assert second.returncode != 0
    assert "Unreleased section has no release notes" in second.stderr
    after_second = {
        path.relative_to(repo): path.read_bytes()
        for path in repo.rglob("*")
        if path.is_file() and ".git" not in path.parts
    }
    assert after_second == before_second


def test_real_release_sync_preserves_unrelated_lock_packages(tmp_path: Path) -> None:
    repo = tmp_path / "real-repository"
    repo.mkdir()
    prepare_release._copy_tracked_sandbox(REPO_ROOT, repo)
    changelog_path = repo / "CHANGELOG.md"
    changelog_path.write_text(
        prepare_release.re.sub(
            r"(?ms)(^## \[Unreleased\]\s*\n).*?(?=^## \[)",
            "\\1\n### Fixed\n\n- A controlled regression fixture.\n\n",
            changelog_path.read_text(encoding="utf-8"),
            count=1,
        ),
        encoding="utf-8",
    )
    _current, _target, prepared_files, _roots = prepare_release.prepare(
        repo, "patch", "2026-07-17"
    )

    for prepared in prepared_files:
        if prepared.path.name != "Cargo.lock":
            continue
        before = prepare_release.tomllib.loads(prepared.before)["package"]
        after = prepare_release.tomllib.loads(prepared.after)["package"]
        before_unrelated = [
            package for package in before if package.get("name") != "fortress-rollback"
        ]
        after_unrelated = [
            package for package in after if package.get("name") != "fortress-rollback"
        ]
        assert after_unrelated == before_unrelated
