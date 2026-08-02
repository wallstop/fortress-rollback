#!/usr/bin/env python3
"""Fail closed when a cargo-deny advisory exception is stale or undocumented."""

from __future__ import annotations

import datetime as dt
import importlib.util
import re
import sys
from pathlib import Path
from typing import Any

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - Python < 3.11
    import tomli as tomllib  # type: ignore[no-redef]


ADVISORY_RE = re.compile(r"RUSTSEC-\d{4}-\d{4}")
TOML_ERROR_LINE_RE = re.compile(r"\(at line (?P<line>\d+), column \d+\)")
DISPOSITIONS_PATH = Path("supply-chain/advisory-dispositions.toml")
CRATES_IO_SOURCE = "registry+https://github.com/rust-lang/crates.io-index"
SOURCE_SELECTOR_KEYS = {"branch", "git", "package", "path", "registry", "rev", "tag"}
WORKSPACE_LOCKS_PATH = Path(__file__).resolve().parents[1] / "release" / "workspace_locks.py"

WORKSPACE_LOCKS_SPEC = importlib.util.spec_from_file_location(
    "advisory_workspace_locks", WORKSPACE_LOCKS_PATH
)
if WORKSPACE_LOCKS_SPEC is None or WORKSPACE_LOCKS_SPEC.loader is None:
    raise RuntimeError(
        "scripts/release/workspace_locks.py:0: cannot load workspace discovery"
    )
WORKSPACE_LOCKS = importlib.util.module_from_spec(WORKSPACE_LOCKS_SPEC)
sys.modules[WORKSPACE_LOCKS_SPEC.name] = WORKSPACE_LOCKS
WORKSPACE_LOCKS_SPEC.loader.exec_module(WORKSPACE_LOCKS)


def _display_path(repo_root: Path, path: Path) -> str:
    try:
        return path.resolve().relative_to(repo_root.resolve()).as_posix()
    except ValueError:
        return path.as_posix()


def _load_toml(repo_root: Path, path: Path) -> dict[str, Any]:
    display = _display_path(repo_root, path)
    try:
        content = path.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as error:
        detail = error.strerror if isinstance(error, OSError) else str(error)
        raise RuntimeError(f"{display}:0: cannot read TOML: {detail}") from error
    try:
        return tomllib.loads(content)
    except tomllib.TOMLDecodeError as error:
        line = getattr(error, "lineno", None)
        if not isinstance(line, int) or line < 1:
            match = TOML_ERROR_LINE_RE.search(str(error))
            line = (
                int(match.group("line"))
                if match is not None
                else content.count("\n") + 1
            )
        raise RuntimeError(f"{display}:{line}: invalid TOML: {error}") from error


def _error(path: str | Path, message: str) -> str:
    return f"{path}:0: {message}"


def _review_date(value: object) -> dt.date | None:
    if isinstance(value, dt.datetime):
        return value.date()
    if isinstance(value, dt.date):
        return value
    if isinstance(value, str):
        try:
            return dt.date.fromisoformat(value)
        except ValueError:
            return None
    return None


def _exact_dependency_version(manifest: dict[str, Any], package: str) -> str | None:
    dependencies = manifest.get("dependencies", {})
    if not isinstance(dependencies, dict):
        return None
    dependency = dependencies.get(package)
    if isinstance(dependency, str):
        return dependency
    if isinstance(dependency, dict):
        version = dependency.get("version")
        return version if isinstance(version, str) else None
    return None


def _has_dependency(manifest: dict[str, Any], package: str) -> bool:
    dependencies = manifest.get("dependencies", {})
    return isinstance(dependencies, dict) and package in dependencies


def _dependency_source_selectors(
    manifest: dict[str, Any], package: str
) -> list[str]:
    dependencies = manifest.get("dependencies", {})
    if not isinstance(dependencies, dict):
        return []
    dependency = dependencies.get(package)
    if not isinstance(dependency, dict):
        return []
    return sorted(SOURCE_SELECTOR_KEYS.intersection(dependency))


def _package_name(manifest: dict[str, Any]) -> str | None:
    package = manifest.get("package")
    if not isinstance(package, dict):
        return None
    name = package.get("name")
    return name if isinstance(name, str) and name else None


def _dependency_package_names(manifest: dict[str, Any]) -> set[str]:
    """Return dependency keys and explicit package-renaming targets."""
    dependencies = manifest.get("dependencies", {})
    if not isinstance(dependencies, dict):
        return set()
    names = set(dependencies)
    for dependency in dependencies.values():
        if not isinstance(dependency, dict):
            continue
        renamed_package = dependency.get("package")
        if isinstance(renamed_package, str) and renamed_package:
            names.add(renamed_package)
    return names


def check_repository(repo_root: Path, *, today: dt.date | None = None) -> list[str]:
    """Return deterministic policy errors for `repo_root`."""
    repo_root = repo_root.resolve()
    today = today or dt.datetime.now(dt.timezone.utc).date()
    deny = _load_toml(repo_root, repo_root / "deny.toml")
    disposition_file = _load_toml(repo_root, repo_root / DISPOSITIONS_PATH)
    try:
        workspace_roots = WORKSPACE_LOCKS.discover(repo_root)
    except RuntimeError as error:
        raise RuntimeError(
            f"Cargo.toml:0: cannot discover tracked Cargo workspaces: {error}"
        ) from error
    manifests = {
        manifest_path: _load_toml(repo_root, manifest_path)
        for workspace in workspace_roots
        for manifest_path in workspace.members
    }
    locks = {
        workspace.lock: _load_toml(repo_root, workspace.lock)
        for workspace in workspace_roots
    }

    errors: list[str] = []
    disposition_format = disposition_file.get("format")
    if type(disposition_format) is not int or disposition_format != 1:
        errors.append(_error(DISPOSITIONS_PATH, "format must be the integer 1"))

    ignored = deny.get("advisories", {}).get("ignore", [])
    if not isinstance(ignored, list) or not all(isinstance(item, str) for item in ignored):
        return [_error("deny.toml", "advisories.ignore must be a string list")]

    raw_dispositions = disposition_file.get("dispositions", [])
    if not isinstance(raw_dispositions, list) or not all(
        isinstance(item, dict) for item in raw_dispositions
    ):
        return [_error(DISPOSITIONS_PATH, "dispositions must be an array of tables")]

    by_advisory: dict[str, list[dict[str, Any]]] = {}
    for item in raw_dispositions:
        advisory = item.get("advisory")
        if not isinstance(advisory, str) or ADVISORY_RE.fullmatch(advisory) is None:
            errors.append(
                _error(DISPOSITIONS_PATH, f"invalid advisory identifier {advisory!r}")
            )
            continue
        by_advisory.setdefault(advisory, []).append(item)

    ignored_set = set(ignored)
    for advisory in sorted(ignored_set | set(by_advisory)):
        entries = by_advisory.get(advisory, [])
        if advisory not in ignored_set:
            errors.append(
                _error(DISPOSITIONS_PATH, f"{advisory} is documented but not ignored")
            )
            continue
        if len(entries) != 1:
            errors.append(
                _error(
                    DISPOSITIONS_PATH,
                    f"{advisory} requires exactly one disposition, found {len(entries)}",
                )
            )
            continue

        entry = entries[0]
        package = entry.get("package")
        version = entry.get("version")
        source = entry.get("source")
        checksum = entry.get("checksum")
        owner = entry.get("owner")
        issue = entry.get("issue")
        reason = entry.get("reason")
        exit_criteria = entry.get("exit_criteria")
        review_by = _review_date(entry.get("review_by"))

        if not isinstance(package, str) or not package:
            errors.append(_error(DISPOSITIONS_PATH, f"{advisory} package is required"))
            continue
        if not isinstance(version, str) or not version:
            errors.append(_error(DISPOSITIONS_PATH, f"{advisory} version is required"))
            continue
        if source != CRATES_IO_SOURCE:
            errors.append(
                _error(
                    DISPOSITIONS_PATH,
                    f"{advisory} source must be {CRATES_IO_SOURCE}",
                )
            )
        if not isinstance(checksum, str) or re.fullmatch(r"[0-9a-f]{64}", checksum) is None:
            errors.append(
                _error(
                    DISPOSITIONS_PATH,
                    f"{advisory} checksum must be 64 lowercase hex characters",
                )
            )
        for field, value in (("owner", owner), ("issue", issue), ("reason", reason)):
            if not isinstance(value, str) or not value.strip():
                errors.append(
                    _error(DISPOSITIONS_PATH, f"{advisory} {field} is required")
                )
        if not isinstance(issue, str) or re.fullmatch(
            r"https://github\.com/[^/]+/[^/]+/issues/\d+", issue
        ) is None:
            errors.append(
                _error(DISPOSITIONS_PATH, f"{advisory} issue must be a GitHub issue URL")
            )
        if not isinstance(exit_criteria, list) or not exit_criteria or not all(
            isinstance(item, str) and item.strip() for item in exit_criteria
        ):
            errors.append(
                _error(
                    DISPOSITIONS_PATH,
                    f"{advisory} exit_criteria must be non-empty strings",
                )
            )
        if review_by is None:
            errors.append(
                _error(DISPOSITIONS_PATH, f"{advisory} review_by must be an ISO date")
            )
        elif today > review_by:
            errors.append(
                _error(
                    DISPOSITIONS_PATH,
                    f"{advisory} review expired on {review_by.isoformat()}",
                )
            )

        direct_manifests = [
            (path, manifest)
            for path, manifest in manifests.items()
            if _has_dependency(manifest, package)
        ]
        if not direct_manifests:
            errors.append(
                _error(
                    "Cargo.toml",
                    f"{package} must be a direct exact dependency while {advisory} is ignored",
                )
            )
        for manifest_path, manifest in direct_manifests:
            display_manifest = _display_path(repo_root, manifest_path)
            exact = _exact_dependency_version(manifest, package)
            if exact != f"={version}":
                errors.append(
                    _error(
                        display_manifest,
                        f"{package} must be pinned to ={version} while {advisory} is ignored",
                    )
                )
            selectors = _dependency_source_selectors(manifest, package)
            if selectors:
                errors.append(
                    _error(
                        display_manifest,
                        f"{package} must not select an alternate source ({', '.join(selectors)}) while {advisory} is ignored",
                    )
                )

        direct_manifest_paths = {path for path, _manifest in direct_manifests}
        required_by_packages = {
            name
            for _path, manifest in direct_manifests
            if (name := _package_name(manifest)) is not None
        }
        for workspace in workspace_roots:
            lock_path = workspace.lock
            lock = locks[lock_path]
            packages = lock.get("package")
            display_lock = _display_path(repo_root, lock_path)
            if not isinstance(packages, list) or not all(
                isinstance(item, dict) for item in packages
            ):
                errors.append(
                    _error(display_lock, "package must be an array of tables")
                )
                continue
            workspace_requires_package = any(
                manifest_path in direct_manifest_paths
                or bool(
                    required_by_packages.intersection(
                        _dependency_package_names(manifests[manifest_path])
                    )
                )
                for manifest_path in workspace.members
            ) or any(
                isinstance(item, dict)
                and item.get("name") in required_by_packages
                for item in packages
            )
            if not workspace_requires_package:
                continue
            resolved = [
                item
                for item in packages
                if isinstance(item, dict) and item.get("name") == package
            ]
            if len(resolved) != 1:
                errors.append(
                    _error(
                        display_lock,
                        f"{package} must resolve exactly once while {advisory} is ignored; found {len(resolved)} packages",
                    )
                )
                continue
            locked = resolved[0]
            if locked.get("version") != version:
                errors.append(
                    _error(
                        display_lock,
                        f"{package} must resolve version {version} while {advisory} is ignored",
                    )
                )
            if locked.get("source") != source:
                errors.append(
                    _error(
                        display_lock,
                        f"{package} {version} source does not match the {advisory} disposition",
                    )
                )
            if locked.get("checksum") != checksum:
                errors.append(
                    _error(
                        display_lock,
                        f"{package} {version} checksum does not match the {advisory} disposition",
                    )
                )

    return sorted(set(errors))


def main() -> int:
    repo_root = Path(__file__).resolve().parents[2]
    try:
        errors = check_repository(repo_root)
    except RuntimeError as error:
        print(error, file=sys.stderr)
        return 1
    for error in errors:
        print(error, file=sys.stderr)
    if errors:
        return 1
    print("advisory dispositions are current and complete")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
