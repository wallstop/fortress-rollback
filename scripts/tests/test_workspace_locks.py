#!/usr/bin/env python3
"""Regression tests for Cargo workspace-lock discovery and validation."""

from __future__ import annotations

import os
import shutil
import subprocess
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[2]
LOCK_SCRIPT = REPO_ROOT / "scripts" / "release" / "workspace_locks.py"


def _run_command(
    args: list[str], cwd: Path, *, env: dict[str, str] | None = None
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        args,
        cwd=cwd,
        capture_output=True,
        text=True,
        check=False,
        env=env,
    )


def _write_package(
    directory: Path,
    name: str,
    version: str,
    *,
    dependency_path: str | None = None,
    standalone: bool = False,
) -> None:
    directory.mkdir(parents=True, exist_ok=True)
    manifest = [
        "[package]",
        f'name = "{name}"',
        f'version = "{version}"',
        'edition = "2021"',
    ]
    if standalone:
        manifest.extend(["", "[workspace]"])
    if dependency_path is not None:
        manifest.extend(
            [
                "",
                "[dependencies]",
                f'fortress-rollback = {{ path = "{dependency_path}" }}',
            ]
        )
    (directory / "Cargo.toml").write_text("\n".join(manifest) + "\n", encoding="utf-8")
    source = directory / "src"
    source.mkdir()
    (source / "lib.rs").write_text("pub fn fixture() {}\n", encoding="utf-8")


def _generate_lock(repo: Path, manifest: str) -> None:
    result = _run_command(
        ["cargo", "generate-lockfile", "--manifest-path", manifest], repo
    )
    assert result.returncode == 0, result.stdout + result.stderr


def _git_add(repo: Path) -> None:
    result = _run_command(["git", "add", "--all"], repo)
    assert result.returncode == 0, result.stdout + result.stderr


def _write_fixture(tmp_path: Path) -> Path:
    repo = tmp_path / "repo"
    repo.mkdir()
    _write_package(repo, "fortress-rollback", "1.2.3")
    root_manifest = (repo / "Cargo.toml").read_text(encoding="utf-8")
    (repo / "Cargo.toml").write_text(
        '[workspace]\nmembers = ["member"]\n\n' + root_manifest,
        encoding="utf-8",
    )
    _write_package(repo / "member", "root-member", "0.1.0")
    _write_package(
        repo / "fuzz",
        "fixture-fuzz",
        "0.0.0",
        dependency_path="..",
        standalone=True,
    )
    _write_package(
        repo / "loom-tests",
        "fixture-loom",
        "0.1.0",
        dependency_path="..",
        standalone=True,
    )
    _write_package(
        repo / "tests" / "godot-emscripten",
        "fixture-godot",
        "0.0.0",
        dependency_path="../..",
        standalone=True,
    )
    for manifest in (
        "Cargo.toml",
        "fuzz/Cargo.toml",
        "loom-tests/Cargo.toml",
        "tests/godot-emscripten/Cargo.toml",
    ):
        _generate_lock(repo, manifest)
    init = _run_command(["git", "init", "--quiet"], repo)
    assert init.returncode == 0, init.stderr
    _git_add(repo)
    return repo


def _run(
    repo: Path, command: str, *, env: dict[str, str] | None = None
) -> subprocess.CompletedProcess[str]:
    return _run_command(
        ["python3", str(LOCK_SCRIPT), command, "--repo-root", str(repo)],
        REPO_ROOT,
        env=env,
    )


def _lock_snapshots(repo: Path) -> dict[str, bytes]:
    return {
        path.relative_to(repo).as_posix(): path.read_bytes()
        for path in repo.rglob("Cargo.lock")
    }


def test_list_discovers_current_authoritative_inventory() -> None:
    result = _run(REPO_ROOT, "list")

    assert result.returncode == 0, result.stdout + result.stderr
    assert result.stdout.splitlines() == [
        "workspace_root=.",
        "workspace_manifest=Cargo.toml",
        "workspace_lock=Cargo.lock",
        "workspace_root=fuzz",
        "workspace_manifest=fuzz/Cargo.toml",
        "workspace_lock=fuzz/Cargo.lock",
        "workspace_root=loom-tests",
        "workspace_manifest=loom-tests/Cargo.toml",
        "workspace_lock=loom-tests/Cargo.lock",
        "workspace_root=tests/godot-emscripten",
        "workspace_manifest=tests/godot-emscripten/Cargo.toml",
        "workspace_lock=tests/godot-emscripten/Cargo.lock",
    ]


def test_new_standalone_workspace_is_discovered_without_allowlist(tmp_path: Path) -> None:
    repo = _write_fixture(tmp_path)
    _write_package(
        repo / "new-checks",
        "new-checks",
        "0.2.0",
        dependency_path="..",
        standalone=True,
    )
    _generate_lock(repo, "new-checks/Cargo.toml")
    _git_add(repo)

    result = _run(repo, "list")

    assert result.returncode == 0, result.stdout + result.stderr
    assert "workspace_root=new-checks" in result.stdout
    assert "workspace_lock=new-checks/Cargo.lock" in result.stdout


def test_root_member_shares_root_lock_and_member_lock_is_rejected(tmp_path: Path) -> None:
    repo = _write_fixture(tmp_path)
    shutil.copy2(repo / "Cargo.lock", repo / "member" / "Cargo.lock")
    _git_add(repo)

    result = _run(repo, "check-structure")

    assert result.returncode != 0
    assert "orphan or member-local tracked Cargo.lock: member/Cargo.lock" in result.stderr


def test_stale_standalone_versions_fail_structure_after_root_only_update(
    tmp_path: Path,
) -> None:
    repo = _write_fixture(tmp_path)
    manifest = repo / "Cargo.toml"
    manifest.write_text(
        manifest.read_text(encoding="utf-8").replace(
            'version = "1.2.3"', 'version = "1.2.4"'
        ),
        encoding="utf-8",
    )
    root_only_update = _run_command(
        [
            "cargo",
            "update",
            "--manifest-path",
            "Cargo.toml",
            "--workspace",
        ],
        repo,
    )
    assert root_only_update.returncode == 0, (
        root_only_update.stdout + root_only_update.stderr
    )

    structure_result = _run(repo, "check-structure")
    assert structure_result.returncode != 0
    assert "fuzz/Cargo.lock" in structure_result.stderr
    assert "Cargo.toml version '1.2.4'" in structure_result.stderr


def test_dependency_staleness_requires_full_locked_metadata(tmp_path: Path) -> None:
    repo = _write_fixture(tmp_path)
    fuzz_manifest = repo / "fuzz" / "Cargo.toml"
    fuzz_manifest.write_text(
        fuzz_manifest.read_text(encoding="utf-8")
        + 'root-member = { path = "../member" }\n',
        encoding="utf-8",
    )

    structure_result = _run(repo, "check-structure")
    assert structure_result.returncode == 0, (
        structure_result.stdout + structure_result.stderr
    )

    result = _run(repo, "check")

    assert result.returncode != 0
    assert "locked Cargo metadata for fuzz/Cargo.toml" in result.stderr
    assert "cannot update the lock file" in result.stderr
    assert "--locked" in result.stderr


@pytest.mark.parametrize("failure", ("missing", "malformed"))
def test_missing_or_malformed_lock_fails_closed(tmp_path: Path, failure: str) -> None:
    repo = _write_fixture(tmp_path)
    lock = repo / "fuzz" / "Cargo.lock"
    if failure == "missing":
        lock.unlink()
    else:
        lock.write_text("not = [valid\n", encoding="utf-8")

    result = _run(repo, "check-structure")

    assert result.returncode != 0
    assert "fuzz/Cargo.lock" in result.stderr


def test_malformed_manifest_fails_with_relative_diagnostic(tmp_path: Path) -> None:
    repo = _write_fixture(tmp_path)
    (repo / "loom-tests" / "Cargo.toml").write_text(
        "[package\nname = 'broken'\n", encoding="utf-8"
    )

    result = _run(repo, "check-structure")

    assert result.returncode != 0
    assert "loom-tests/Cargo.toml" in result.stderr
    assert "malformed Cargo manifest" in result.stderr
    assert str(repo) not in result.stderr


def test_missing_tracked_manifest_cannot_hide_workspace(tmp_path: Path) -> None:
    repo = _write_fixture(tmp_path)
    (repo / "fuzz" / "Cargo.toml").unlink()
    (repo / "fuzz" / "Cargo.lock").unlink()

    result = _run(repo, "check-structure")

    assert result.returncode != 0
    assert "fuzz/Cargo.toml:0: cannot read Cargo manifest" in result.stderr
    assert str(repo) not in result.stderr


def test_manifest_resolving_outside_repository_is_rejected(tmp_path: Path) -> None:
    repo = _write_fixture(tmp_path)
    outside = tmp_path / "outside.toml"
    outside.write_text(
        '[package]\nname = "outside"\nversion = "1.0.0"\n', encoding="utf-8"
    )
    manifest = repo / "fuzz" / "Cargo.toml"
    manifest.unlink()
    manifest.symlink_to(outside)
    _git_add(repo)

    result = _run(repo, "check-structure")

    assert result.returncode != 0
    assert "fuzz/Cargo.toml resolves outside the repository" in result.stderr


def _write_failing_cargo_wrapper(tmp_path: Path) -> Path:
    wrapper = tmp_path / "cargo-wrapper.py"
    wrapper.write_text(
        """#!/usr/bin/env python3
import os
import subprocess
import sys
from pathlib import Path

args = sys.argv[1:]
counter = Path(os.environ["CARGO_WRAPPER_COUNTER"])
if args and args[0] == "update":
    count = int(counter.read_text() if counter.exists() else "0") + 1
    counter.write_text(str(count))
    if count == 2:
        Path(os.environ["CARGO_WRAPPER_DELETE_LOCK"]).unlink()
        print("injected update failure", file=sys.stderr)
        raise SystemExit(42)
raise SystemExit(subprocess.run([os.environ["REAL_CARGO"], *args], check=False).returncode)
""",
        encoding="utf-8",
    )
    wrapper.chmod(0o755)
    return wrapper


def test_sync_restores_every_lock_after_subprocess_failure(tmp_path: Path) -> None:
    repo = _write_fixture(tmp_path)
    manifest = repo / "Cargo.toml"
    manifest.write_text(
        manifest.read_text(encoding="utf-8").replace(
            'version = "1.2.3"', 'version = "1.2.4"'
        ),
        encoding="utf-8",
    )
    before = _lock_snapshots(repo)
    wrapper = _write_failing_cargo_wrapper(tmp_path)
    env = os.environ.copy()
    env.update(
        {
            "CARGO": str(wrapper),
            "REAL_CARGO": shutil.which("cargo") or "cargo",
            "CARGO_WRAPPER_COUNTER": str(tmp_path / "counter"),
            "CARGO_WRAPPER_DELETE_LOCK": str(
                repo / "tests" / "godot-emscripten" / "Cargo.lock"
            ),
        }
    )

    result = _run(repo, "sync", env=env)

    assert result.returncode != 0
    assert "injected update failure" in result.stderr
    assert _lock_snapshots(repo) == before


def test_sync_updates_all_stale_locks_and_check_passes(tmp_path: Path) -> None:
    repo = _write_fixture(tmp_path)
    manifest = repo / "Cargo.toml"
    manifest.write_text(
        manifest.read_text(encoding="utf-8").replace(
            'version = "1.2.3"', 'version = "1.2.4"'
        ),
        encoding="utf-8",
    )

    sync_result = _run(repo, "sync")
    check_result = _run(repo, "check")

    assert sync_result.returncode == 0, sync_result.stdout + sync_result.stderr
    assert check_result.returncode == 0, check_result.stdout + check_result.stderr
    for lock in _lock_snapshots(repo).values():
        lock_text = lock.decode("utf-8")
        if 'name = "fortress-rollback"' in lock_text:
            assert 'name = "fortress-rollback"\nversion = "1.2.4"' in lock_text
