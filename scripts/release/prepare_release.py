#!/usr/bin/env python3
"""Prepare a reviewed Fortress Rollback release version bump."""

from __future__ import annotations

import argparse
import difflib
import os
import re
import shutil
import stat
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from datetime import date, datetime, timezone
from pathlib import Path

import workspace_locks
from release_policy import (
    ReleasePolicyError,
    parse_stable_version,
    validate_requested_bump,
)

try:
    import tomllib
except ImportError:
    import tomli as tomllib

PACKAGE_NAME = "fortress-rollback"
MAX_SEMVER_COMPONENT = (1 << 64) - 1


class PreparationError(ValueError):
    """A release input or repository invariant is invalid."""


@dataclass(frozen=True)
class PreparedFile:
    """One fully validated release-file rewrite."""

    path: Path
    before: str
    after: str


def parse_version(value: str) -> tuple[int, int, int]:
    """Parse strict stable SemVer without unbounded integer conversion."""
    try:
        return parse_stable_version(value)
    except ReleasePolicyError as error:
        raise PreparationError(str(error)) from error


def _increment_version_component(value: int, name: str) -> int:
    """Increment one bounded SemVer component or fail before u64 overflow."""
    if value == MAX_SEMVER_COMPONENT:
        raise PreparationError(f"cannot bump {name} version component beyond u64")
    return value + 1


def bump_version(current: str, bump: str) -> str:
    """Return the next stable version for a semver bump kind."""
    major, minor, patch = parse_version(current)
    if bump == "major":
        return f"{_increment_version_component(major, 'major')}.0.0"
    if bump == "minor":
        return f"{major}.{_increment_version_component(minor, 'minor')}.0"
    if bump == "patch":
        return f"{major}.{minor}.{_increment_version_component(patch, 'patch')}"
    raise PreparationError(f"unsupported bump kind {bump!r}")


def load_text(path: Path) -> str:
    """Read a required UTF-8 release file with a concise failure."""
    try:
        return path.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as error:
        raise PreparationError(f"cannot read {path}: {error}") from error


def rewrite_manifest(path: Path, content: str, target: str) -> tuple[str, str]:
    """Rewrite only the root package version and return its old version."""
    try:
        manifest = tomllib.loads(content)
    except tomllib.TOMLDecodeError as error:
        raise PreparationError(f"invalid Cargo.toml: {error}") from error
    package = manifest.get("package")
    if not isinstance(package, dict) or package.get("name") != PACKAGE_NAME:
        raise PreparationError(f"Cargo.toml [package].name must be {PACKAGE_NAME!r}")
    current = package.get("version")
    if not isinstance(current, str):
        raise PreparationError("Cargo.toml [package].version is missing")
    parse_version(current)
    package_block = re.search(r"(?ms)^\[package\]\s*$.*?(?=^\[|\Z)", content)
    if package_block is None:
        raise PreparationError("Cargo.toml has no textual [package] table")
    rewritten_block, count = re.subn(
        r'(?m)^(version\s*=\s*")[^"]+("\s*(?:#.*)?)$',
        rf"\g<1>{target}\g<2>",
        package_block.group(0),
    )
    if count != 1:
        raise PreparationError("Cargo.toml [package] must contain exactly one version assignment")
    rewritten = content[: package_block.start()] + rewritten_block + content[package_block.end() :]
    return current, rewritten


def rewrite_changelog(content: str, current: str, target: str, release_date: str) -> str:
    """Rotate Unreleased notes into a dated release and add its compare link."""
    if len(re.findall(r"(?m)^## \[Unreleased\]\s*$", content)) != 1:
        raise PreparationError("CHANGELOG.md must contain exactly one Unreleased heading")
    target_heading = re.compile(rf"(?m)^## \[{re.escape(target)}\](?:\s|$)")
    if target_heading.search(content):
        raise PreparationError(f"CHANGELOG.md already contains a {target} release heading")
    if re.search(rf"(?m)^\[{re.escape(target)}\]:", content):
        raise PreparationError(f"CHANGELOG.md already contains a {target} link footer")

    unreleased = re.search(
        r"(?ms)^## \[Unreleased\]\s*\n(?P<notes>.*?)(?=^## \[)", content
    )
    if unreleased is None:
        raise PreparationError("CHANGELOG.md Unreleased section has no following release heading")
    if not unreleased.group("notes").strip():
        raise PreparationError("CHANGELOG.md Unreleased section has no release notes")

    rewritten = content.replace(
        "## [Unreleased]", f"## [Unreleased]\n\n## [{target}] - {release_date}", 1
    )
    unreleased_link_pattern = re.compile(r"(?m)^\[Unreleased\]:.*$")
    if len(unreleased_link_pattern.findall(rewritten)) != 1:
        raise PreparationError("CHANGELOG.md must contain exactly one Unreleased link footer")
    unreleased_link = (
        "[Unreleased]: https://github.com/wallstop/fortress-rollback/compare/"
        f"v{target}...HEAD"
    )
    release_link = (
        f"[{target}]: https://github.com/wallstop/fortress-rollback/compare/"
        f"v{current}...v{target}"
    )
    return unreleased_link_pattern.sub(
        lambda _match: f"{unreleased_link}\n{release_link}", rewritten
    )


def _run_git(repo_root: Path, args: list[str], description: str) -> str:
    """Run Git for sandbox construction with concise diagnostics."""
    try:
        result = subprocess.run(
            ["git", "-C", str(repo_root), *args],
            capture_output=True,
            text=True,
            check=False,
        )
    except (OSError, UnicodeError) as error:
        raise PreparationError(f"{description} could not start: {error}") from error
    if result.returncode != 0:
        details = (result.stderr or result.stdout).strip()
        raise PreparationError(
            f"{description} failed with exit code {result.returncode}: {details}"
        )
    return result.stdout


def _copy_tracked_sandbox(repo_root: Path, sandbox: Path) -> tuple[Path, ...]:
    """Copy the current tracked-file state and create an isolated Git index."""
    tracked_output = _run_git(
        repo_root, ["ls-files", "-z", "--"], "tracked-file discovery"
    )
    tracked = [Path(raw) for raw in tracked_output.split("\0") if raw]
    for relative in tracked:
        source = repo_root / relative
        if not source.exists():
            raise PreparationError(
                f"tracked file {relative.as_posix()} is missing from the working tree"
            )
        destination = sandbox / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        try:
            if source.is_symlink():
                destination.symlink_to(source.readlink())
                try:
                    destination.resolve(strict=False).relative_to(sandbox)
                except ValueError as error:
                    raise PreparationError(
                        f"tracked symlink {relative.as_posix()} escapes release sandbox"
                    ) from error
            else:
                shutil.copy2(source, destination)
        except PreparationError:
            raise
        except OSError as error:
            raise PreparationError(
                f"cannot copy {relative.as_posix()} into sandbox: {error}"
            ) from error
    _run_git(sandbox, ["init", "--quiet"], "sandbox Git initialization")
    _run_git(
        sandbox,
        ["add", "--force", "--all", "--", "."],
        "sandbox tracked-file indexing",
    )
    return tuple(tracked)


def _run_version_sync(sandbox: Path, release_date: str, *, check: bool) -> None:
    """Run the canonical version-reference synchronizer inside the sandbox."""
    script = sandbox / "scripts" / "sync-version.sh"
    if not script.is_file():
        raise PreparationError("tracked file scripts/sync-version.sh is missing")
    command = ["bash", str(script)]
    if check:
        command.append("--check")
    environment = os.environ.copy()
    environment["FORTRESS_PROJECT_ROOT"] = str(sandbox)
    environment["FORTRESS_RELEASE_DATE"] = release_date
    try:
        result = subprocess.run(
            command,
            cwd=sandbox,
            env=environment,
            capture_output=True,
            text=True,
            check=False,
        )
    except (OSError, UnicodeError) as error:
        raise PreparationError(f"version synchronization could not start: {error}") from error
    if result.returncode != 0:
        details = (result.stderr or result.stdout).strip().replace(str(sandbox), ".")
        mode = "check" if check else "update"
        raise PreparationError(
            f"version synchronization {mode} failed with exit code "
            f"{result.returncode}: {details}"
        )


def _atomic_write(path: Path, content: str) -> None:
    """Replace one file atomically while preserving its permission mode."""
    mode = stat.S_IMODE(path.stat().st_mode)
    temporary_name: str | None = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="w",
            encoding="utf-8",
            newline="",
            dir=path.parent,
            prefix=f".{path.name}.release-",
            delete=False,
        ) as temporary:
            temporary.write(content)
            temporary.flush()
            os.fsync(temporary.fileno())
            temporary_name = temporary.name
        os.chmod(temporary_name, mode)
        os.replace(temporary_name, path)
    except OSError:
        if temporary_name is not None:
            Path(temporary_name).unlink(missing_ok=True)
        raise


def apply_prepared(prepared_files: list[PreparedFile]) -> None:
    """Apply all prepared files as one rollback-capable transaction."""
    for prepared in prepared_files:
        current = load_text(prepared.path)
        if current != prepared.before:
            raise PreparationError(
                f"{prepared.path}: changed during release preparation; no files applied"
            )

    applied: list[PreparedFile] = []
    try:
        for prepared in prepared_files:
            _atomic_write(prepared.path, prepared.after)
            applied.append(prepared)
    except OSError as error:
        rollback_errors: list[str] = []
        for prepared in reversed(applied):
            try:
                _atomic_write(prepared.path, prepared.before)
            except OSError as rollback_error:
                rollback_errors.append(f"{prepared.path.name}: {rollback_error}")
        suffix = (
            f"; rollback failures: {', '.join(rollback_errors)}"
            if rollback_errors
            else ""
        )
        raise PreparationError(
            f"could not apply prepared release files: {error}{suffix}"
        ) from error


def prepare(
    repo_root: Path, bump: str, release_date: str
) -> tuple[str, str, list[PreparedFile], tuple[Path, ...]]:
    """Calculate and validate the complete release rewrite in a sandbox."""
    try:
        date.fromisoformat(release_date)
    except ValueError as error:
        raise PreparationError(f"date {release_date!r} must be YYYY-MM-DD") from error

    repo_root = repo_root.resolve()
    with tempfile.TemporaryDirectory(prefix="fortress-release-") as temporary_directory:
        sandbox = Path(temporary_directory).resolve()
        tracked_files = set(_copy_tracked_sandbox(repo_root, sandbox))
        try:
            precheck_roots = workspace_locks.check(sandbox)
        except workspace_locks.WorkspaceLockError as error:
            raise PreparationError(f"workspace-lock precheck failed: {error}") from error

        manifest_path = sandbox / "Cargo.toml"
        changelog_path = sandbox / "CHANGELOG.md"
        manifest_before = load_text(manifest_path)
        changelog_before = load_text(changelog_path)
        current_manifest = tomllib.loads(manifest_before)
        package = current_manifest.get("package")
        current = package.get("version") if isinstance(package, dict) else None
        if not isinstance(current, str):
            raise PreparationError("Cargo.toml [package].version is missing")
        target = bump_version(current, bump)
        validated_current, manifest_after = rewrite_manifest(
            manifest_path, manifest_before, target
        )
        changelog_after = rewrite_changelog(
            changelog_before, validated_current, target, release_date
        )
        try:
            validate_requested_bump(changelog_before, validated_current, bump)
        except ReleasePolicyError as error:
            raise PreparationError(str(error)) from error
        try:
            manifest_path.write_text(manifest_after, encoding="utf-8")
            changelog_path.write_text(changelog_after, encoding="utf-8")
        except OSError as error:
            raise PreparationError(f"cannot write release sandbox: {error}") from error

        try:
            roots = workspace_locks.sync(sandbox)
            _run_version_sync(sandbox, release_date, check=False)
            workspace_locks.check(sandbox)
            _run_version_sync(sandbox, release_date, check=True)
        except workspace_locks.WorkspaceLockError as error:
            raise PreparationError(f"workspace-lock validation failed: {error}") from error

        synchronized_roots = {
            root.lock.relative_to(sandbox) for root in roots
        }
        prechecked_roots = {
            root.lock.relative_to(sandbox) for root in precheck_roots
        }
        if synchronized_roots != prechecked_roots:
            raise PreparationError(
                "workspace topology changed during release preparation"
            )
        deleted_output = _run_git(
            sandbox,
            ["diff", "--name-only", "--diff-filter=D", "-z", "--"],
            "release-output deletion check",
        )
        deleted = sorted(Path(raw) for raw in deleted_output.split("\0") if raw)
        if deleted:
            rendered = ", ".join(path.as_posix() for path in deleted)
            raise PreparationError(f"release preparation deleted tracked files: {rendered}")
        untracked_output = _run_git(
            sandbox,
            ["ls-files", "--others", "--exclude-standard", "-z", "--"],
            "release-output untracked-file check",
        )
        untracked = sorted(Path(raw) for raw in untracked_output.split("\0") if raw)
        if untracked:
            rendered = ", ".join(path.as_posix() for path in untracked)
            raise PreparationError(
                f"release preparation created untracked files: {rendered}"
            )
        changed_output = _run_git(
            sandbox,
            ["diff", "--name-only", "-z", "--"],
            "release-output discovery",
        )
        changed = sorted(
            (Path(raw) for raw in changed_output.split("\0") if raw),
            key=lambda path: path.as_posix(),
        )
        unexpected = [relative for relative in changed if relative not in tracked_files]
        if unexpected:
            rendered = ", ".join(path.as_posix() for path in unexpected)
            raise PreparationError(f"release preparation changed untracked files: {rendered}")
        changed_symlinks = [
            relative for relative in changed if (repo_root / relative).is_symlink()
        ]
        if changed_symlinks:
            rendered = ", ".join(path.as_posix() for path in changed_symlinks)
            raise PreparationError(
                f"release preparation cannot replace tracked symlink outputs: {rendered}"
            )
        prepared_files = [
            PreparedFile(
                path=repo_root / relative,
                before=_run_git(
                    sandbox,
                    ["show", f":{relative.as_posix()}"],
                    f"release-output baseline read for {relative.as_posix()}",
                ),
                after=load_text(sandbox / relative),
            )
            for relative in changed
        ]
        workspace_roots = tuple(
            root.manifest.parent.relative_to(sandbox) for root in roots
        )
        return current, target, prepared_files, workspace_roots


def main() -> int:
    """CLI entry point."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--bump", choices=("major", "minor", "patch"), required=True)
    parser.add_argument("--date", default=datetime.now(timezone.utc).date().isoformat())
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    try:
        current, target, prepared_files, workspace_roots = prepare(
            args.repo_root.resolve(), args.bump, args.date
        )
    except (PreparationError, tomllib.TOMLDecodeError) as error:
        print(f"prepare-release: error: {error}", file=sys.stderr)
        return 1

    try:
        if args.dry_run:
            for prepared in prepared_files:
                relative = prepared.path.relative_to(args.repo_root.resolve())
                print(
                    "".join(
                        difflib.unified_diff(
                            prepared.before.splitlines(keepends=True),
                            prepared.after.splitlines(keepends=True),
                            fromfile=str(relative),
                            tofile=str(relative),
                        )
                    ),
                    end="",
                )
        else:
            apply_prepared(prepared_files)
    except PreparationError as error:
        print(f"prepare-release: error: {error}", file=sys.stderr)
        return 1
    print(f"current_version={current}")
    print(f"prepared_version={target}")
    for workspace_root in workspace_roots:
        print(
            "workspace_root="
            f"{workspace_root.as_posix() if workspace_root != Path('.') else '.'}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
