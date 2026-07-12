#!/usr/bin/env python3
"""Prepare a reviewed Fortress Rollback release version bump."""

from __future__ import annotations

import argparse
import difflib
import re
import sys
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
    """Rewrite exactly the root package entry in Cargo.lock."""
    try:
        lockfile = tomllib.loads(content)
    except tomllib.TOMLDecodeError as error:
        raise PreparationError(f"invalid Cargo.lock: {error}") from error
    packages = lockfile.get("package")
    matches = [
        package
        for package in packages if isinstance(package, dict) and package.get("name") == PACKAGE_NAME
    ] if isinstance(packages, list) else []
    if len(matches) != 1:
        raise PreparationError(f"Cargo.lock must contain exactly one {PACKAGE_NAME} package")
    lock_version = matches[0].get("version")
    if lock_version != current:
        raise PreparationError(
            f"Cargo.lock package version {lock_version} does not match Cargo.toml {current}"
        )

    blocks = list(re.finditer(r"(?ms)^\[\[package\]\]\s*$.*?(?=^\[\[package\]\]|\Z)", content))
    matching_blocks = [block for block in blocks if re.search(rf'(?m)^name = "{re.escape(PACKAGE_NAME)}"$', block.group(0))]
    if len(matching_blocks) != 1:
        raise PreparationError(f"Cargo.lock text must contain exactly one {PACKAGE_NAME} package")
    block = matching_blocks[0]
    rewritten_block, count = re.subn(
        rf'(?m)^version = "{re.escape(current)}"$', f'version = "{target}"', block.group(0)
    )
    if count != 1:
        raise PreparationError("Cargo.lock root package version could not be rewritten exactly once")
    return content[: block.start()] + rewritten_block + content[block.end() :]


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
    return target, [
        PreparedFile(manifest_path, manifest_before, manifest_after),
        PreparedFile(lockfile_path, lockfile_before, lockfile_after),
        PreparedFile(changelog_path, changelog_before, changelog_after),
    ]


def main() -> int:
    """CLI entry point."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--bump", choices=("major", "minor", "patch"), required=True)
    parser.add_argument("--date", default=datetime.now(timezone.utc).date().isoformat())
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    try:
        target, prepared_files = prepare(args.repo_root.resolve(), args.bump, args.date)
    except (PreparationError, tomllib.TOMLDecodeError) as error:
        print(f"prepare-release: error: {error}", file=sys.stderr)
        return 1

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
        for prepared in prepared_files:
            prepared.path.write_text(prepared.after, encoding="utf-8")
    print(f"prepared_version={target}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
