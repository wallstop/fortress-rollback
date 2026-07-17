#!/usr/bin/env python3
"""Discover, synchronize, and validate every tracked Cargo workspace lock."""

from __future__ import annotations

import argparse
import os
import stat
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

try:
    import tomllib
except ImportError:
    import tomli as tomllib


class WorkspaceLockError(RuntimeError):
    """The tracked Cargo workspace or lock topology is invalid."""


@dataclass(frozen=True)
class WorkspaceRoot:
    """One Cargo workspace root and the tracked manifests that it owns."""

    manifest: Path
    lock: Path
    members: tuple[Path, ...]


@dataclass(frozen=True)
class LockSnapshot:
    """Recoverable bytes and permissions for one authoritative lock."""

    content: bytes
    mode: int


def _relative(repo_root: Path, path: Path) -> str:
    """Return a stable repository-relative display path."""
    relative = path.resolve().relative_to(repo_root)
    return "." if relative == Path(".") else relative.as_posix()


def _display_manifest(repo_root: Path, path: Path) -> str:
    """Return a repository-relative manifest path."""
    relative = _relative(repo_root, path)
    return "Cargo.toml" if relative == "." else relative


def _sanitize(repo_root: Path, output: str) -> str:
    """Remove absolute repository prefixes from subprocess diagnostics."""
    root = str(repo_root)
    sanitized = output.replace(f"{root}{os.sep}", "")
    return sanitized.replace(root, ".").strip()


def _run(
    repo_root: Path,
    args: list[str],
    *,
    description: str,
) -> subprocess.CompletedProcess[str]:
    """Run a required subprocess and raise a repository-relative failure."""
    try:
        result = subprocess.run(
            args,
            cwd=repo_root,
            capture_output=True,
            text=True,
            check=False,
        )
    except (OSError, UnicodeError) as error:
        raise WorkspaceLockError(f"{description} could not start: {error}") from error
    if result.returncode != 0:
        details = _sanitize(repo_root, result.stderr or result.stdout)
        suffix = f": {details}" if details else ""
        raise WorkspaceLockError(
            f"{description} failed with exit code {result.returncode}{suffix}"
        )
    return result


def _tracked_files(repo_root: Path) -> set[Path]:
    """Return every tracked repository path, failing closed outside Git."""
    result = _run(
        repo_root,
        ["git", "ls-files", "-z", "--"],
        description="git tracked-file discovery",
    )
    tracked: set[Path] = set()
    for raw_path in result.stdout.split("\0"):
        if raw_path:
            tracked.add(Path(raw_path))
    return tracked


def _load_toml(repo_root: Path, path: Path, kind: str) -> dict[str, object]:
    """Load a required UTF-8 TOML document with relative diagnostics."""
    display = (
        _display_manifest(repo_root, path)
        if path.name == "Cargo.toml"
        else _relative(repo_root, path)
    )
    try:
        content = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as error:
        details = _sanitize(repo_root, str(error))
        raise WorkspaceLockError(
            f"{display}:0: cannot read {kind}: {details}"
        ) from error
    try:
        parsed = tomllib.loads(content)
    except tomllib.TOMLDecodeError as error:
        line = getattr(error, "lineno", 1) or 1
        raise WorkspaceLockError(f"{display}:{line}: malformed {kind}: {error}") from error
    if not isinstance(parsed, dict):
        raise WorkspaceLockError(f"{display}: malformed {kind}: expected a TOML table")
    return parsed


def discover(repo_root: Path) -> tuple[WorkspaceRoot, ...]:
    """Discover tracked workspace roots through Cargo's ownership model."""
    repo_root = repo_root.resolve()
    tracked = _tracked_files(repo_root)
    tracked_manifests = sorted(
        (path for path in tracked if path.name == "Cargo.toml"),
        key=lambda path: path.as_posix(),
    )
    if not tracked_manifests:
        raise WorkspaceLockError("no tracked Cargo.toml manifests were found")

    cargo = os.environ.get("CARGO", "cargo")
    ownership: dict[Path, list[Path]] = {}
    tracked_manifest_set = set(tracked_manifests)
    for relative_manifest in tracked_manifests:
        manifest = (repo_root / relative_manifest).resolve()
        try:
            manifest.relative_to(repo_root)
        except ValueError as error:
            raise WorkspaceLockError(
                f"{relative_manifest.as_posix()} resolves outside the repository"
            ) from error
        _load_toml(repo_root, manifest, "Cargo manifest")
        result = _run(
            repo_root,
            [
                cargo,
                "locate-project",
                "--workspace",
                "--manifest-path",
                relative_manifest.as_posix(),
                "--message-format",
                "plain",
            ],
            description=f"Cargo workspace discovery for {relative_manifest.as_posix()}",
        )
        lines = [line.strip() for line in result.stdout.splitlines() if line.strip()]
        if len(lines) != 1:
            raise WorkspaceLockError(
                f"Cargo workspace discovery for {relative_manifest.as_posix()} "
                f"returned {len(lines)} roots"
            )
        root_manifest = Path(lines[0]).resolve()
        try:
            root_relative = root_manifest.relative_to(repo_root)
        except ValueError as error:
            raise WorkspaceLockError(
                f"{relative_manifest.as_posix()} belongs to workspace root outside the repository"
            ) from error
        if root_relative not in tracked_manifest_set:
            raise WorkspaceLockError(
                f"unexpected workspace root {_display_manifest(repo_root, root_manifest)} "
                "is not a tracked manifest"
            )
        ownership.setdefault(root_manifest, []).append(manifest)

    tracked_locks = {path for path in tracked if path.name == "Cargo.lock"}
    roots: list[WorkspaceRoot] = []
    expected_locks: set[Path] = set()
    for manifest, members in ownership.items():
        lock = manifest.with_name("Cargo.lock")
        relative_lock = lock.relative_to(repo_root)
        expected_locks.add(relative_lock)
        if relative_lock not in tracked_locks:
            raise WorkspaceLockError(
                f"{_relative(repo_root, lock)}: missing tracked lock for workspace "
                f"{_display_manifest(repo_root, manifest)}"
            )
        roots.append(
            WorkspaceRoot(
                manifest=manifest,
                lock=lock,
                members=tuple(sorted(members, key=lambda path: _relative(repo_root, path))),
            )
        )

    unexpected_locks = sorted(
        tracked_locks - expected_locks,
        key=lambda path: path.as_posix(),
    )
    if unexpected_locks:
        rendered = ", ".join(path.as_posix() for path in unexpected_locks)
        raise WorkspaceLockError(f"orphan or member-local tracked Cargo.lock: {rendered}")

    return tuple(sorted(roots, key=lambda root: _display_manifest(repo_root, root.manifest)))


def _manifest_version(
    repo_root: Path,
    manifest_path: Path,
    manifest: dict[str, object],
    root_manifest: dict[str, object],
) -> tuple[str, str] | None:
    """Return a local package name/version, resolving workspace inheritance."""
    package = manifest.get("package")
    if package is None:
        return None
    if not isinstance(package, dict):
        raise WorkspaceLockError(
            f"{_display_manifest(repo_root, manifest_path)}: [package] must be a table"
        )
    name = package.get("name")
    if not isinstance(name, str) or not name:
        raise WorkspaceLockError(
            f"{_display_manifest(repo_root, manifest_path)}: [package].name must be a string"
        )
    version = package.get("version")
    if isinstance(version, str):
        return name, version
    if isinstance(version, dict) and version.get("workspace") is True:
        workspace = root_manifest.get("workspace")
        workspace_package = workspace.get("package") if isinstance(workspace, dict) else None
        inherited = (
            workspace_package.get("version")
            if isinstance(workspace_package, dict)
            else None
        )
        if isinstance(inherited, str):
            return name, inherited
    raise WorkspaceLockError(
        f"{_display_manifest(repo_root, manifest_path)}: package {name!r} "
        "has no resolvable version"
    )


def check_structure(repo_root: Path) -> tuple[WorkspaceRoot, ...]:
    """Validate tracked lock ownership and local package versions offline."""
    repo_root = repo_root.resolve()
    roots = discover(repo_root)
    root_manifests = {
        root.manifest: _load_toml(repo_root, root.manifest, "Cargo manifest")
        for root in roots
    }
    local_versions: dict[str, tuple[str, Path]] = {}
    for root in roots:
        for manifest_path in root.members:
            manifest = _load_toml(repo_root, manifest_path, "Cargo manifest")
            local_package = _manifest_version(
                repo_root,
                manifest_path,
                manifest,
                root_manifests[root.manifest],
            )
            if local_package is None:
                continue
            name, version = local_package
            if name in local_versions:
                previous_path = local_versions[name][1]
                raise WorkspaceLockError(
                    f"duplicate tracked package name {name!r} in "
                    f"{_display_manifest(repo_root, previous_path)} and "
                    f"{_display_manifest(repo_root, manifest_path)}"
                )
            local_versions[name] = (version, manifest_path)

    for root in roots:
        lock = _load_toml(repo_root, root.lock, "Cargo lock")
        packages = lock.get("package")
        if not isinstance(packages, list):
            raise WorkspaceLockError(
                f"{_relative(repo_root, root.lock)}: Cargo lock has no package array"
            )
        for manifest_path in root.members:
            manifest = _load_toml(repo_root, manifest_path, "Cargo manifest")
            local_package = _manifest_version(
                repo_root,
                manifest_path,
                manifest,
                root_manifests[root.manifest],
            )
            if local_package is None:
                continue
            name, version = local_package
            matches = [
                package
                for package in packages
                if isinstance(package, dict)
                and package.get("name") == name
                and "source" not in package
            ]
            if len(matches) != 1:
                raise WorkspaceLockError(
                    f"{_relative(repo_root, root.lock)}: expected exactly one local "
                    f"package {name!r}, found {len(matches)}"
                )
            locked_version = matches[0].get("version")
            if locked_version != version:
                raise WorkspaceLockError(
                    f"{_relative(repo_root, root.lock)}: local package {name!r} "
                    f"version {locked_version!r} does not match "
                    f"{_display_manifest(repo_root, manifest_path)} version {version!r}"
                )
        for package in packages:
            if not isinstance(package, dict) or "source" in package:
                continue
            name = package.get("name")
            if not isinstance(name, str) or name not in local_versions:
                continue
            expected_version, manifest_path = local_versions[name]
            locked_version = package.get("version")
            if locked_version != expected_version:
                raise WorkspaceLockError(
                    f"{_relative(repo_root, root.lock)}: local path package {name!r} "
                    f"version {locked_version!r} does not match "
                    f"{_display_manifest(repo_root, manifest_path)} version "
                    f"{expected_version!r}"
                )
    return roots


def check(repo_root: Path) -> tuple[WorkspaceRoot, ...]:
    """Run Cargo's complete locked dependency resolution for every root."""
    repo_root = repo_root.resolve()
    roots = check_structure(repo_root)
    cargo = os.environ.get("CARGO", "cargo")
    for root in roots:
        manifest = _display_manifest(repo_root, root.manifest)
        _run(
            repo_root,
            [
                cargo,
                "metadata",
                "--manifest-path",
                manifest,
                "--locked",
                "--all-features",
                "--format-version",
                "1",
            ],
            description=f"locked Cargo metadata for {manifest}",
        )
    return roots


def _restore_locks(repo_root: Path, snapshots: dict[Path, LockSnapshot]) -> None:
    """Restore lock snapshots after a failed synchronization."""
    errors: list[str] = []
    for lock, snapshot in snapshots.items():
        temporary_name: str | None = None
        try:
            with tempfile.NamedTemporaryFile(
                mode="wb",
                dir=lock.parent,
                prefix=f".{lock.name}.restore-",
                delete=False,
            ) as temporary:
                temporary.write(snapshot.content)
                temporary.flush()
                os.fsync(temporary.fileno())
                temporary_name = temporary.name
            os.chmod(temporary_name, snapshot.mode)
            os.replace(temporary_name, lock)
        except OSError as error:
            if temporary_name is not None:
                try:
                    Path(temporary_name).unlink(missing_ok=True)
                except OSError as cleanup_error:
                    errors.append(
                        f"could not clean temporary restore for "
                        f"{_relative(repo_root, lock)}: "
                        f"{_sanitize(repo_root, str(cleanup_error))}"
                    )
            errors.append(
                f"could not restore {_relative(repo_root, lock)}: "
                f"{_sanitize(repo_root, str(error))}"
            )
    if errors:
        raise WorkspaceLockError("; ".join(errors))


def sync(repo_root: Path) -> tuple[WorkspaceRoot, ...]:
    """Update only workspace packages in every authoritative Cargo lock."""
    repo_root = repo_root.resolve()
    roots = discover(repo_root)
    snapshots: dict[Path, LockSnapshot] = {}
    for root in roots:
        try:
            snapshots[root.lock] = LockSnapshot(
                content=root.lock.read_bytes(),
                mode=stat.S_IMODE(root.lock.stat().st_mode),
            )
        except OSError as error:
            raise WorkspaceLockError(
                f"{_relative(repo_root, root.lock)}: cannot snapshot lock: "
                f"{_sanitize(repo_root, str(error))}"
            ) from error

    cargo = os.environ.get("CARGO", "cargo")
    try:
        for root in roots:
            manifest = _display_manifest(repo_root, root.manifest)
            _run(
                repo_root,
                [
                    cargo,
                    "update",
                    "--manifest-path",
                    manifest,
                    "--workspace",
                ],
                description=f"Cargo workspace lock synchronization for {manifest}",
            )
        return check_structure(repo_root)
    except BaseException as error:
        try:
            _restore_locks(repo_root, snapshots)
        except WorkspaceLockError as restore_error:
            raise WorkspaceLockError(
                f"lock synchronization failed: {error}; lock rollback failed: "
                f"{restore_error}"
            ) from error
        raise


def _print_roots(repo_root: Path, roots: tuple[WorkspaceRoot, ...]) -> None:
    """Print a stable, machine-readable workspace inventory."""
    for root in roots:
        print(f"workspace_root={_relative(repo_root, root.manifest.parent)}")
        print(f"workspace_manifest={_display_manifest(repo_root, root.manifest)}")
        print(f"workspace_lock={_relative(repo_root, root.lock)}")


def main() -> int:
    """CLI entry point."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("list", "sync", "check", "check-structure"))
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=Path(__file__).resolve().parents[2],
    )
    args = parser.parse_args()
    repo_root = args.repo_root.resolve()

    try:
        if args.command in {"list", "check-structure"}:
            roots = check_structure(repo_root)
        elif args.command == "sync":
            roots = sync(repo_root)
        else:
            roots = check(repo_root)
    except WorkspaceLockError as error:
        print(f"workspace-locks: error: {error}", file=sys.stderr)
        return 1

    _print_roots(repo_root, roots)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
