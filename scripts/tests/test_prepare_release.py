#!/usr/bin/env python3
"""Regression tests for the automated release preparation workflow."""

from __future__ import annotations

import importlib.util
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


def _write_fixture(tmp_path: Path, *, notes: str = "### Added\n\n- A feature.\n") -> Path:
    repo = tmp_path / "repo"
    repo.mkdir()
    (repo / "Cargo.toml").write_text(
        '[package]\nname = "fortress-rollback"\nversion = "1.2.3"\n',
        encoding="utf-8",
    )
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
## [1.2.3] - 2026-01-01

- Previous release.

[Unreleased]: https://github.com/wallstop/fortress-rollback/compare/v1.2.3...HEAD
[1.2.3]: https://github.com/wallstop/fortress-rollback/releases/tag/v1.2.3
""",
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
    ("bump", "expected"),
    [("patch", "1.2.4"), ("minor", "1.3.0"), ("major", "2.0.0")],
)
def test_prepare_release_bumps_manifest_all_locks_and_changelog(
    tmp_path: Path, bump: str, expected: str
) -> None:
    repo = _write_fixture(tmp_path)

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
    assert f"## [{expected}] - 2026-07-12\n\n### Added" in changelog
    expected_link = (
        f"[{expected}]: https://github.com/wallstop/fortress-rollback/compare/"
        f"v1.2.3...v{expected}"
    )
    assert expected_link in changelog
    assert f"prepared_version={expected}" in result.stdout


def test_prepare_release_dry_run_does_not_write(tmp_path: Path) -> None:
    repo = _write_fixture(tmp_path)
    before = {
        path.relative_to(repo): path.read_bytes()
        for path in repo.rglob("*")
        if path.is_file() and ".git" not in path.parts
    }

    result = _run(repo, "patch", "--dry-run")

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
    assert "prepared_version=1.2.4" in result.stdout


def test_prepare_release_rejects_empty_unreleased_notes(tmp_path: Path) -> None:
    repo = _write_fixture(tmp_path, notes="")

    result = _run(repo, "patch")

    assert result.returncode != 0
    assert "Unreleased section has no release notes" in result.stderr
    assert 'version = "1.2.3"' in (repo / "Cargo.toml").read_text()


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


def test_prepare_release_rejects_missing_tracked_sandbox_input(tmp_path: Path) -> None:
    repo = _write_fixture(tmp_path)
    (repo / "fuzz" / "Cargo.toml").unlink()

    result = _run(repo, "patch", "--dry-run")

    assert result.returncode != 0
    assert "tracked file fuzz/Cargo.toml is missing" in result.stderr
    assert str(repo) not in result.stderr


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


def test_real_release_sync_preserves_unrelated_lock_packages() -> None:
    _current, _target, prepared_files, _roots = prepare_release.prepare(
        REPO_ROOT, "patch", "2026-07-17"
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
