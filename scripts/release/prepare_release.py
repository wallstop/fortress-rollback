#!/usr/bin/env python3
"""Prepare a reviewed Fortress Rollback release version bump."""

from __future__ import annotations

import argparse
import difflib
import fnmatch
import os
import re
import stat
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from datetime import date, datetime, timezone
from pathlib import Path

try:
    import tomllib
except ImportError:
    import tomli as tomllib

STRICT_VERSION = re.compile(r"^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$")
PACKAGE_NAME = "fortress-rollback"


class PreparationError(ValueError):
    """A release input or repository invariant is invalid."""


@dataclass(frozen=True)
class PreparedFile:
    """One fully validated release-file rewrite."""

    path: Path
    before: str
    after: str


def parse_version(value: str) -> tuple[int, int, int]:
    """Parse strict stable semver without accepting ambiguous leading zeroes."""
    match = STRICT_VERSION.fullmatch(value)
    if match is None:
        raise PreparationError(f"version {value!r} is not strict X.Y.Z semver")
    return tuple(int(component) for component in match.groups())  # type: ignore[return-value]


def bump_version(current: str, bump: str) -> str:
    """Return the next stable version for a semver bump kind."""
    major, minor, patch = parse_version(current)
    if bump == "major":
        return f"{major + 1}.0.0"
    if bump == "minor":
        return f"{major}.{minor + 1}.0"
    if bump == "patch":
        return f"{major}.{minor}.{patch + 1}"
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


def rewrite_lockfile(content: str, current: str, target: str) -> str:
    """Rewrite exactly one local package entry in a Cargo lockfile."""
    try:
        lockfile = tomllib.loads(content)
    except tomllib.TOMLDecodeError as error:
        raise PreparationError(f"invalid Cargo.lock: {error}") from error
    packages = lockfile.get("package")
    matches = (
        [
            package
            for package in packages
            if isinstance(package, dict)
            and package.get("name") == PACKAGE_NAME
            and "source" not in package
        ]
        if isinstance(packages, list)
        else []
    )
    if len(matches) != 1:
        raise PreparationError(
            f"Cargo.lock must contain exactly one local {PACKAGE_NAME} package"
        )
    lock_version = matches[0].get("version")
    if lock_version != current:
        raise PreparationError(
            f"Cargo.lock package version {lock_version} does not match Cargo.toml {current}"
        )

    blocks = list(re.finditer(r"(?ms)^\[\[package\]\]\s*$.*?(?=^\[\[package\]\]|\Z)", content))
    matching_blocks = [
        block
        for block in blocks
        if re.search(
            rf'(?m)^name = "{re.escape(PACKAGE_NAME)}"$', block.group(0)
        )
        and not re.search(r"(?m)^source = ", block.group(0))
    ]
    if len(matching_blocks) != 1:
        raise PreparationError(
            f"Cargo.lock text must contain exactly one {PACKAGE_NAME} package"
        )
    block = matching_blocks[0]
    rewritten_block, count = re.subn(
        rf'(?m)^version = "{re.escape(current)}"$', f'version = "{target}"', block.group(0)
    )
    if count != 1:
        raise PreparationError("Cargo.lock root package version could not be rewritten exactly once")
    return content[: block.start()] + rewritten_block + content[block.end() :]


def _tracked_paths(repo_root: Path) -> set[Path]:
    """Return repository-relative paths from Git's tracked-file index."""
    try:
        result = subprocess.run(
            ["git", "ls-files", "-z"],
            cwd=repo_root,
            capture_output=True,
            check=False,
        )
    except OSError as error:
        raise PreparationError(f"cannot enumerate tracked files: {error}") from error
    if result.returncode != 0:
        detail = result.stderr.decode("utf-8", errors="replace").strip()
        raise PreparationError(f"cannot enumerate tracked files: {detail}")
    try:
        names = result.stdout.decode("utf-8").split("\0")
    except UnicodeError as error:
        raise PreparationError(f"tracked file path is not UTF-8: {error}") from error
    return {Path(name) for name in names if name}


def standalone_manifests(repo_root: Path) -> list[Path]:
    """Return tracked standalone-workspace manifests with tracked lockfiles."""
    tracked = _tracked_paths(repo_root)
    root_manifest_path = repo_root / "Cargo.toml"
    try:
        root_manifest = tomllib.loads(load_text(root_manifest_path))
    except tomllib.TOMLDecodeError as error:
        raise PreparationError(f"invalid Cargo.toml: {error}") from error
    workspace = root_manifest.get("workspace")
    members = workspace.get("members", []) if isinstance(workspace, dict) else []
    excludes = workspace.get("exclude", []) if isinstance(workspace, dict) else []
    member_patterns = [member.rstrip("/") for member in members if isinstance(member, str)]
    exclude_patterns = [item.rstrip("/") for item in excludes if isinstance(item, str)]

    manifests = []
    for relative in sorted(tracked):
        if relative.name != "Cargo.toml" or relative == Path("Cargo.toml"):
            continue
        parent = relative.parent.as_posix()
        is_member = any(fnmatch.fnmatchcase(parent, pattern) for pattern in member_patterns)
        is_excluded = any(fnmatch.fnmatchcase(parent, pattern) for pattern in exclude_patterns)
        if is_member and not is_excluded:
            continue
        if relative.with_name("Cargo.lock") not in tracked:
            continue
        manifest_path = repo_root / relative
        try:
            manifest = tomllib.loads(load_text(manifest_path))
        except tomllib.TOMLDecodeError as error:
            raise PreparationError(f"invalid {relative}: {error}") from error
        if "workspace" in manifest:
            manifests.append(manifest_path)
    return manifests


def rewrite_nested_lockfile(content: str, target: str) -> str | None:
    """Rewrite a standalone lockfile's local package, or skip unrelated locks."""
    try:
        lockfile = tomllib.loads(content)
    except tomllib.TOMLDecodeError as error:
        raise PreparationError(f"invalid Cargo.lock: {error}") from error
    packages = lockfile.get("package")
    matches = (
        [
            package
            for package in packages
            if isinstance(package, dict)
            and package.get("name") == PACKAGE_NAME
            and "source" not in package
        ]
        if isinstance(packages, list)
        else []
    )
    if not matches:
        return None
    if len(matches) != 1:
        raise PreparationError(
            f"Cargo.lock must contain at most one local {PACKAGE_NAME} package"
        )
    current = matches[0].get("version")
    if not isinstance(current, str):
        raise PreparationError(
            f"Cargo.lock local {PACKAGE_NAME} package version is missing"
        )
    parse_version(current)
    return rewrite_lockfile(content, current, target)


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
    unreleased_link = re.compile(r"(?m)^\[Unreleased\]:.*$")
    if len(unreleased_link.findall(rewritten)) != 1:
        raise PreparationError("CHANGELOG.md must contain exactly one Unreleased link footer")
    release_link = (
        f"[{target}]: https://github.com/wallstop/fortress-rollback/compare/"
        f"v{current}...v{target}"
    )
    return unreleased_link.sub(lambda match: f"{match.group(0)}\n{release_link}", rewritten)


def prepare(repo_root: Path, bump: str, release_date: str) -> tuple[str, list[PreparedFile]]:
    """Validate every input and calculate the complete release rewrite."""
    try:
        date.fromisoformat(release_date)
    except ValueError as error:
        raise PreparationError(f"date {release_date!r} must be YYYY-MM-DD") from error

    manifest_path = repo_root / "Cargo.toml"
    lockfile_path = repo_root / "Cargo.lock"
    changelog_path = repo_root / "CHANGELOG.md"
    manifest_before = load_text(manifest_path)
    lockfile_before = load_text(lockfile_path)
    changelog_before = load_text(changelog_path)

    current_manifest = tomllib.loads(manifest_before)
    package = current_manifest.get("package")
    current = package.get("version") if isinstance(package, dict) else None
    if not isinstance(current, str):
        raise PreparationError("Cargo.toml [package].version is missing")
    target = bump_version(current, bump)
    validated_current, manifest_after = rewrite_manifest(manifest_path, manifest_before, target)
    lockfile_after = rewrite_lockfile(lockfile_before, validated_current, target)
    changelog_after = rewrite_changelog(
        changelog_before, validated_current, target, release_date
    )
    prepared_files = [
        PreparedFile(manifest_path, manifest_before, manifest_after),
        PreparedFile(lockfile_path, lockfile_before, lockfile_after),
        PreparedFile(changelog_path, changelog_before, changelog_after),
    ]
    for nested_manifest in standalone_manifests(repo_root):
        nested_lockfile = nested_manifest.with_name("Cargo.lock")
        nested_before = load_text(nested_lockfile)
        try:
            nested_after = rewrite_nested_lockfile(nested_before, target)
        except PreparationError as error:
            relative = nested_lockfile.relative_to(repo_root)
            raise PreparationError(f"{relative}: {error}") from error
        if nested_after is not None and nested_after != nested_before:
            prepared_files.append(
                PreparedFile(nested_lockfile, nested_before, nested_after)
            )
    return target, prepared_files


def _stage_content(path: Path, content: str, label: str) -> Path:
    """Write and sync sibling temporary content without changing its destination."""
    try:
        descriptor, temporary_name = tempfile.mkstemp(
            prefix=f".{path.name}.{label}.", dir=path.parent
        )
        temporary = Path(temporary_name)
        with os.fdopen(descriptor, "w", encoding="utf-8", newline="") as handle:
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
        os.chmod(temporary, stat.S_IMODE(path.stat().st_mode))
        return temporary
    except OSError as error:
        if "temporary" in locals():
            _discard_temporary(temporary)
        raise PreparationError(f"cannot stage {path}: {error}") from error


def _discard_temporary(path: Path) -> None:
    """Best-effort cleanup that never masks a preparation or rollback result."""
    try:
        path.unlink(missing_ok=True)
    except OSError:
        pass


def write_prepared_files(prepared_files: list[PreparedFile]) -> None:
    """Stage all rewrites, replace atomically, and restore originals on failure."""
    staged: list[tuple[PreparedFile, Path, Path]] = []
    try:
        for prepared in prepared_files:
            replacement = _stage_content(prepared.path, prepared.after, "new")
            try:
                rollback = _stage_content(prepared.path, prepared.before, "rollback")
            except PreparationError:
                _discard_temporary(replacement)
                raise
            staged.append((prepared, replacement, rollback))
    except PreparationError:
        for _, replacement, rollback in staged:
            _discard_temporary(replacement)
            _discard_temporary(rollback)
        raise

    replaced: list[tuple[PreparedFile, Path]] = []
    preserved_rollbacks: set[Path] = set()
    try:
        for prepared, replacement, rollback in staged:
            os.replace(replacement, prepared.path)
            replaced.append((prepared, rollback))
    except OSError as error:
        rollback_errors = []
        for prepared, rollback in reversed(replaced):
            try:
                os.replace(rollback, prepared.path)
            except OSError as rollback_error:
                preserved_rollbacks.add(rollback)
                rollback_errors.append(f"{prepared.path}: {rollback_error}")
        detail = f"cannot replace {prepared.path}: {error}"
        if rollback_errors:
            detail += "; rollback failed for " + ", ".join(rollback_errors)
        raise PreparationError(detail) from error
    finally:
        for _, replacement, rollback in staged:
            _discard_temporary(replacement)
            if rollback not in preserved_rollbacks:
                _discard_temporary(rollback)


def main() -> int:
    """CLI entry point."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--bump", choices=("major", "minor", "patch"))
    parser.add_argument("--date", default=datetime.now(timezone.utc).date().isoformat())
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--print-standalone-manifests", action="store_true")
    args = parser.parse_args()

    repo_root = args.repo_root.resolve()
    if args.print_standalone_manifests:
        if args.bump is not None or args.dry_run:
            parser.error("--print-standalone-manifests cannot be combined with release options")
        try:
            for manifest in standalone_manifests(repo_root):
                print(manifest.relative_to(repo_root))
        except PreparationError as error:
            print(f"prepare-release: error: {error}", file=sys.stderr)
            return 1
        return 0
    if args.bump is None:
        parser.error("--bump is required unless --print-standalone-manifests is used")

    try:
        target, prepared_files = prepare(repo_root, args.bump, args.date)
    except (PreparationError, tomllib.TOMLDecodeError) as error:
        print(f"prepare-release: error: {error}", file=sys.stderr)
        return 1

    if args.dry_run:
        for prepared in prepared_files:
            relative = prepared.path.relative_to(repo_root)
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
        try:
            write_prepared_files(prepared_files)
        except PreparationError as error:
            print(f"prepare-release: error: {error}", file=sys.stderr)
            return 1
    print(f"prepared_version={target}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
