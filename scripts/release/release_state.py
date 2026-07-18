#!/usr/bin/env python3
"""Generate and verify the immutable state of a reviewed release."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import stat
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from datetime import date
from pathlib import Path

SCRIPT_DIRECTORY = Path(__file__).resolve().parent
if str(SCRIPT_DIRECTORY) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIRECTORY))

import prepare_release
from release_policy import ReleasePolicyError, validate_requested_bump

try:
    import tomllib
except ImportError:
    import tomli as tomllib


MANIFEST_NAME = "release-state.json"
SCHEMA_VERSION = 1
STRICT_VERSION = re.compile(
    r"^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$"
)
VALID_BUMPS = frozenset({"major", "minor", "patch"})
CHANGELOG_HEADING = re.compile(r"(?m)^## \[[^\n]*$")
PRIOR_RELEASE_HEADING = re.compile(
    r"^## \[(?P<version>[^]\n]+)\] - "
    r"(?P<date>[0-9]{4}-[0-9]{2}-[0-9]{2})$"
)
MAX_SEMVER_COMPONENT = (1 << 64) - 1
MAX_SEMVER_COMPONENT_DIGITS = len(str(MAX_SEMVER_COMPONENT))
MAX_STABLE_VERSION_LENGTH = 3 * MAX_SEMVER_COMPONENT_DIGITS + 2


class ReleaseStateError(RuntimeError):
    """The prepared release state is missing, malformed, or stale."""


@dataclass(frozen=True)
class ReleaseState:
    """Metadata committed by the reviewed release-preparation PR."""

    crate_name: str
    previous_version: str
    target_version: str
    bump: str
    release_date: str
    source_sha256: str

    def to_json(self) -> str:
        """Render the stable on-disk representation."""
        return json.dumps(
            {
                "schema_version": SCHEMA_VERSION,
                "crate_name": self.crate_name,
                "previous_version": self.previous_version,
                "target_version": self.target_version,
                "bump": self.bump,
                "release_date": self.release_date,
                "source_sha256": self.source_sha256,
            },
            indent=2,
            sort_keys=True,
        ) + "\n"


def _git_tracked_paths(repo_root: Path) -> list[bytes]:
    try:
        result = subprocess.run(
            ["git", "-C", str(repo_root), "ls-files", "-z", "--"],
            capture_output=True,
            check=False,
        )
    except OSError as error:
        raise ReleaseStateError(f"could not start git: {error}") from error
    if result.returncode != 0:
        details = result.stderr.decode(errors="replace").strip()
        raise ReleaseStateError(
            f"tracked-file discovery failed with exit code {result.returncode}: {details}"
        )
    return sorted(path for path in result.stdout.split(b"\0") if path)


def _hash_field(digest: "hashlib._Hash", label: bytes, value: bytes) -> None:
    """Hash one length-delimited field so paths and contents cannot collide."""
    digest.update(label)
    digest.update(len(value).to_bytes(8, byteorder="big"))
    digest.update(value)


def _tracked_tree_digest(repo_root: Path, *, include_manifest: bool) -> str:
    """Hash every tracked path, Git-relevant mode, and file/symlink payload."""
    repo_root = repo_root.resolve()
    manifest_bytes = os.fsencode(MANIFEST_NAME)
    domain = (
        b"fortress-release-complete-tree-v1\0"
        if include_manifest
        else b"fortress-release-state-tree-v1\0"
    )
    digest = hashlib.sha256(domain)
    for relative_bytes in _git_tracked_paths(repo_root):
        if not include_manifest and relative_bytes == manifest_bytes:
            continue
        relative = Path(os.fsdecode(relative_bytes))
        path = repo_root / relative
        try:
            metadata = path.lstat()
        except OSError as error:
            raise ReleaseStateError(
                f"tracked path {os.fsdecode(relative_bytes)!r} cannot be read: {error}"
            ) from error

        if stat.S_ISLNK(metadata.st_mode):
            mode = b"120000"
            try:
                payload = os.fsencode(os.readlink(path))
            except OSError as error:
                raise ReleaseStateError(
                    f"tracked symlink {relative.as_posix()} cannot be read: {error}"
                ) from error
        elif stat.S_ISREG(metadata.st_mode):
            mode = b"100755" if metadata.st_mode & stat.S_IXUSR else b"100644"
            try:
                payload = path.read_bytes()
            except OSError as error:
                raise ReleaseStateError(
                    f"tracked file {relative.as_posix()} cannot be read: {error}"
                ) from error
        else:
            raise ReleaseStateError(
                f"tracked path {relative.as_posix()} is not a regular file or symlink"
            )

        _hash_field(digest, b"path\0", relative_bytes)
        _hash_field(digest, b"mode\0", mode)
        _hash_field(digest, b"payload\0", payload)
    return digest.hexdigest()


def source_digest(repo_root: Path) -> str:
    """Hash the tracked source tree while excluding its release-state manifest."""
    return _tracked_tree_digest(repo_root, include_manifest=False)


def _parse_version(value: object, field: str) -> tuple[int, int, int]:
    if not isinstance(value, str):
        raise ReleaseStateError(f"{field} must be a string")
    if len(value) > MAX_STABLE_VERSION_LENGTH:
        raise ReleaseStateError(
            f"{field} exceeds the maximum length for u64 SemVer components"
        )
    match = STRICT_VERSION.fullmatch(value)
    if match is None:
        raise ReleaseStateError(f"{field} must use strict X.Y.Z semver; got {value!r}")
    parsed: list[int] = []
    for component in match.groups():
        if len(component) > MAX_SEMVER_COMPONENT_DIGITS:
            raise ReleaseStateError(f"{field} has a component larger than u64")
        numeric = int(component)
        if numeric > MAX_SEMVER_COMPONENT:
            raise ReleaseStateError(f"{field} has a component larger than u64")
        parsed.append(numeric)
    return parsed[0], parsed[1], parsed[2]


def _expected_target(previous: str, bump: str) -> str:
    major, minor, patch = _parse_version(previous, "previous_version")
    if bump == "major":
        return f"{major + 1}.0.0"
    if bump == "minor":
        return f"{major}.{minor + 1}.0"
    if bump == "patch":
        return f"{major}.{minor}.{patch + 1}"
    raise ReleaseStateError(f"bump must be one of {sorted(VALID_BUMPS)}; got {bump!r}")


def _load_package(repo_root: Path) -> tuple[str, str]:
    try:
        manifest = tomllib.loads((repo_root / "Cargo.toml").read_text(encoding="utf-8"))
    except (OSError, UnicodeError, tomllib.TOMLDecodeError) as error:
        raise ReleaseStateError(f"cannot load Cargo.toml: {error}") from error
    package = manifest.get("package")
    if not isinstance(package, dict):
        raise ReleaseStateError("Cargo.toml has no [package] table")
    name = package.get("name")
    version = package.get("version")
    if not isinstance(name, str) or not name:
        raise ReleaseStateError("Cargo.toml package name is missing")
    _parse_version(version, "Cargo.toml package version")
    if not isinstance(version, str):
        raise ReleaseStateError("Cargo.toml package version is missing")
    return name, version


def _validate_metadata(state: ReleaseState) -> None:
    _parse_version(state.previous_version, "previous_version")
    _parse_version(state.target_version, "target_version")
    if state.bump not in VALID_BUMPS:
        raise ReleaseStateError(
            f"bump must be one of {sorted(VALID_BUMPS)}; got {state.bump!r}"
        )
    expected = _expected_target(state.previous_version, state.bump)
    if state.target_version != expected:
        raise ReleaseStateError(
            f"target_version {state.target_version} does not match the {state.bump} "
            f"bump from {state.previous_version} ({expected})"
        )
    if re.fullmatch(r"\d{4}-\d{2}-\d{2}", state.release_date) is None:
        raise ReleaseStateError("release_date must use YYYY-MM-DD")
    try:
        parsed_date = date.fromisoformat(state.release_date)
    except ValueError as error:
        raise ReleaseStateError("release_date must use YYYY-MM-DD") from error
    if parsed_date.isoformat() != state.release_date:
        raise ReleaseStateError("release_date must use canonical YYYY-MM-DD")
    if not re.fullmatch(r"[0-9a-f]{64}", state.source_sha256):
        raise ReleaseStateError("source_sha256 must be a lowercase SHA-256 digest")


def generate(
    repo_root: Path,
    *,
    previous_version: str,
    target_version: str,
    bump: str,
    release_date: str,
) -> ReleaseState:
    """Write release-state.json after validating the prepared metadata."""
    repo_root = repo_root.resolve()
    crate_name, cargo_version = _load_package(repo_root)
    state = ReleaseState(
        crate_name=crate_name,
        previous_version=previous_version,
        target_version=target_version,
        bump=bump,
        release_date=release_date,
        source_sha256=source_digest(repo_root),
    )
    _validate_metadata(state)
    if cargo_version != target_version:
        raise ReleaseStateError(
            f"Cargo.toml version {cargo_version} does not match target_version {target_version}"
        )
    _verify_changelog(repo_root, state)
    _verify_issue_template(repo_root, state.target_version)
    destination = repo_root / MANIFEST_NAME
    temporary_name: str | None = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="w",
            encoding="utf-8",
            newline="",
            dir=repo_root,
            prefix=f".{MANIFEST_NAME}.",
            delete=False,
        ) as temporary:
            temporary.write(state.to_json())
            temporary.flush()
            os.fsync(temporary.fileno())
            temporary_name = temporary.name
        os.replace(temporary_name, destination)
    except OSError as error:
        if temporary_name is not None:
            Path(temporary_name).unlink(missing_ok=True)
        raise ReleaseStateError(f"cannot write {MANIFEST_NAME}: {error}") from error
    return state


def load(repo_root: Path) -> ReleaseState:
    """Load a strictly shaped release-state.json."""
    path = repo_root / MANIFEST_NAME
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise ReleaseStateError(f"cannot load {MANIFEST_NAME}: {error}") from error
    expected_keys = {
        "schema_version",
        "crate_name",
        "previous_version",
        "target_version",
        "bump",
        "release_date",
        "source_sha256",
    }
    if not isinstance(raw, dict) or set(raw) != expected_keys:
        raise ReleaseStateError(f"{MANIFEST_NAME} has an unsupported shape")
    if raw.get("schema_version") != SCHEMA_VERSION:
        raise ReleaseStateError(
            f"{MANIFEST_NAME} schema_version must be {SCHEMA_VERSION}"
        )
    string_fields = expected_keys - {"schema_version"}
    if any(not isinstance(raw.get(field), str) for field in string_fields):
        raise ReleaseStateError(f"{MANIFEST_NAME} metadata fields must be strings")
    state = ReleaseState(
        crate_name=raw["crate_name"],
        previous_version=raw["previous_version"],
        target_version=raw["target_version"],
        bump=raw["bump"],
        release_date=raw["release_date"],
        source_sha256=raw["source_sha256"],
    )
    _validate_metadata(state)
    return state


def _finalized_release_notes(changelog: str, state: ReleaseState) -> str:
    """Return target notes after binding them to the preceding release version."""
    heading = f"## [{state.target_version}] - {state.release_date}"
    target_headings = list(re.finditer(rf"(?m)^{re.escape(heading)}$", changelog))
    if len(target_headings) != 1:
        raise ReleaseStateError(f"CHANGELOG.md must contain exactly one {heading!r}")

    target_heading = target_headings[0]
    following_heading = CHANGELOG_HEADING.search(changelog, target_heading.end())
    if following_heading is None:
        raise ReleaseStateError(
            f"CHANGELOG.md release {state.target_version} has no following prior release heading"
        )
    parsed_heading = PRIOR_RELEASE_HEADING.fullmatch(following_heading.group(0))
    if parsed_heading is None:
        raise ReleaseStateError(
            "CHANGELOG.md heading immediately after the target release must use "
            "canonical '## [X.Y.Z] - YYYY-MM-DD' form"
        )
    actual_previous = parsed_heading.group("version")
    try:
        _parse_version(actual_previous, "CHANGELOG.md previous release version")
    except ReleaseStateError as error:
        raise ReleaseStateError(
            "CHANGELOG.md heading immediately after the target release must contain "
            "a strict stable previous version"
        ) from error
    previous_date = parsed_heading.group("date")
    try:
        parsed_previous_date = date.fromisoformat(previous_date)
    except ValueError as error:
        raise ReleaseStateError(
            "CHANGELOG.md heading immediately after the target release must contain "
            "a canonical YYYY-MM-DD date"
        ) from error
    if parsed_previous_date.isoformat() != previous_date:
        raise ReleaseStateError(
            "CHANGELOG.md heading immediately after the target release must contain "
            "a canonical YYYY-MM-DD date"
        )
    if actual_previous != state.previous_version:
        raise ReleaseStateError(
            f"previous_version {state.previous_version} does not match the immediately "
            f"following CHANGELOG.md release {actual_previous}"
        )

    notes = changelog[target_heading.end() : following_heading.start()]
    if not notes.strip():
        raise ReleaseStateError(
            f"CHANGELOG.md release {state.target_version} has no release notes"
        )
    return notes


def _verify_release_policy(notes: str, state: ReleaseState) -> None:
    """Re-derive the bump floor from the exact finalized release notes."""
    synthetic_changelog = (
        "## [Unreleased]\n"
        f"{notes.rstrip()}\n\n"
        f"## [{state.previous_version}] - 1970-01-01\n"
    )
    try:
        validate_requested_bump(
            synthetic_changelog,
            state.previous_version,
            state.bump,
        )
    except ReleasePolicyError as error:
        raise ReleaseStateError(
            f"CHANGELOG.md release {state.target_version} violates release policy: {error}"
        ) from error


def _verify_changelog(repo_root: Path, state: ReleaseState) -> None:
    try:
        changelog = (repo_root / "CHANGELOG.md").read_text(encoding="utf-8")
    except (OSError, UnicodeError) as error:
        raise ReleaseStateError(f"cannot load CHANGELOG.md: {error}") from error
    unreleased = re.search(
        r"(?ms)^## \[Unreleased\][^\n]*\n(?P<body>.*?)(?=^## \[)", changelog
    )
    if unreleased is None:
        raise ReleaseStateError("CHANGELOG.md has no bounded Unreleased section")
    if unreleased.group("body").strip():
        raise ReleaseStateError("CHANGELOG.md Unreleased section must be empty")
    notes = _finalized_release_notes(changelog, state)
    _verify_release_policy(notes, state)


def _verify_issue_template(repo_root: Path, target_version: str) -> None:
    path = repo_root / ".github" / "ISSUE_TEMPLATE" / "bug_report.yml"
    try:
        content = path.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as error:
        raise ReleaseStateError(f"cannot load {path.relative_to(repo_root)}: {error}") from error
    begin = "# BEGIN_FORTRESS_VERSIONS"
    end = "# END_FORTRESS_VERSIONS"
    if content.count(begin) != 1 or content.count(end) != 1:
        raise ReleaseStateError(
            "bug-report version dropdown must contain one managed sentinel pair"
        )
    begin_index = content.index(begin) + len(begin)
    end_index = content.index(end)
    if begin_index >= end_index:
        raise ReleaseStateError("bug-report version dropdown sentinels are out of order")
    managed_versions = content[begin_index:end_index]
    option = re.compile(rf"(?m)^\s*- v{re.escape(target_version)}\s*$")
    if len(option.findall(managed_versions)) != 1:
        raise ReleaseStateError(
            f"bug-report version dropdown must contain v{target_version} exactly once"
        )


def verify(repo_root: Path, *, expected_version: str | None = None) -> ReleaseState:
    """Verify the manifest, prepared metadata, and complete tracked source tree."""
    repo_root = repo_root.resolve()
    if expected_version is not None:
        _parse_version(expected_version, "expected_version")
    state = load(repo_root)
    crate_name, cargo_version = _load_package(repo_root)
    if expected_version is not None and state.target_version != expected_version:
        raise ReleaseStateError(
            f"release input {expected_version} does not match target_version {state.target_version}"
        )
    if crate_name != state.crate_name:
        raise ReleaseStateError(
            f"Cargo.toml crate name {crate_name!r} does not match {state.crate_name!r}"
        )
    if cargo_version != state.target_version:
        raise ReleaseStateError(
            f"Cargo.toml version {cargo_version} does not match {state.target_version}"
        )
    _verify_changelog(repo_root, state)
    _verify_issue_template(repo_root, state.target_version)
    actual_digest = source_digest(repo_root)
    if actual_digest != state.source_sha256:
        raise ReleaseStateError(
            "reviewed release source digest does not match the current tracked tree: "
            f"expected {state.source_sha256}, got {actual_digest}"
        )
    return state


def _git_text(repo_root: Path, args: list[str], description: str) -> str:
    """Run one trusted local Git query with bounded, contextual failure output."""
    try:
        result = subprocess.run(
            ["git", "-C", str(repo_root), *args],
            capture_output=True,
            text=True,
            check=False,
        )
    except (OSError, UnicodeError) as error:
        raise ReleaseStateError(f"{description} could not start: {error}") from error
    if result.returncode != 0:
        details = (result.stderr or result.stdout).strip()
        raise ReleaseStateError(
            f"{description} failed with exit code {result.returncode}: {details}"
        )
    return result.stdout.strip()


def _require_single_preparation_commit(
    candidate_root: Path, trusted_base_root: Path
) -> None:
    """Require the candidate head to be one ordinary commit atop the trusted base."""
    base_sha = _git_text(
        trusted_base_root, ["rev-parse", "HEAD^{commit}"], "trusted base lookup"
    )
    parents = _git_text(
        candidate_root,
        ["rev-list", "--parents", "-n", "1", "HEAD^{commit}"],
        "candidate ancestry lookup",
    ).split()
    if len(parents) != 2 or parents[1] != base_sha:
        actual_parent = parents[1] if len(parents) >= 2 else "<not-one-parent>"
        raise ReleaseStateError(
            "release candidate must be one preparation commit directly atop trusted "
            f"base {base_sha}; parent is {actual_parent}"
        )


def _require_prospective_tree_descends_from_base(
    candidate_root: Path, trusted_base_root: Path
) -> None:
    """Require a merge-queue candidate to contain the exact trusted base."""
    base_sha = _git_text(
        trusted_base_root, ["rev-parse", "HEAD^{commit}"], "trusted base lookup"
    )
    try:
        result = subprocess.run(
            [
                "git",
                "-C",
                str(candidate_root),
                "merge-base",
                "--is-ancestor",
                base_sha,
                "HEAD^{commit}",
            ],
            capture_output=True,
            text=True,
            check=False,
        )
    except (OSError, UnicodeError) as error:
        raise ReleaseStateError(
            f"prospective release ancestry lookup could not start: {error}"
        ) from error
    if result.returncode == 1:
        raise ReleaseStateError(
            f"prospective release tree does not descend from trusted base {base_sha}"
        )
    if result.returncode != 0:
        details = (result.stderr or result.stdout).strip()
        raise ReleaseStateError(
            "prospective release ancestry lookup failed with exit code "
            f"{result.returncode}: {details}"
        )


def _require_clean_checkout(repo_root: Path, label: str) -> None:
    status = _git_text(
        repo_root,
        ["status", "--porcelain=v1", "--untracked-files=all", "--"],
        f"{label} status",
    )
    if status:
        raise ReleaseStateError(f"{label} checkout must be clean; found:\n{status}")


def _run_trusted_issue_template_sync(
    trusted_base_root: Path, expected_root: Path, target_version: str
) -> None:
    """Run the base branch's deterministic issue-template generator."""
    script = trusted_base_root / "scripts" / "ci" / "sync-issue-template-versions.py"
    if not script.is_file():
        raise ReleaseStateError(
            "trusted base is missing scripts/ci/sync-issue-template-versions.py"
        )
    environment = os.environ.copy()
    environment["GITHUB_REPOSITORY"] = "wallstop/fortress-rollback"
    try:
        result = subprocess.run(
            [
                sys.executable,
                str(script),
                "--local-only",
                "--ensure-version",
                f"v{target_version}",
            ],
            cwd=expected_root,
            env=environment,
            capture_output=True,
            text=True,
            check=False,
        )
    except (OSError, UnicodeError) as error:
        raise ReleaseStateError(
            f"trusted issue-template synchronization could not start: {error}"
        ) from error
    if result.returncode != 0:
        details = (result.stderr or result.stdout).strip()
        raise ReleaseStateError(
            "trusted issue-template synchronization failed with exit code "
            f"{result.returncode}: {details}"
        )


def _stage_expected_manifest(expected_root: Path) -> None:
    _git_text(
        expected_root,
        ["add", "--force", "--", MANIFEST_NAME],
        "expected release-state staging",
    )


def _require_candidate_manifest_tracked(candidate_root: Path) -> None:
    tracked = _git_text(
        candidate_root,
        ["ls-files", "--error-unmatch", "--", MANIFEST_NAME],
        "candidate release-state tracking check",
    )
    if tracked != MANIFEST_NAME:
        raise ReleaseStateError(f"candidate must track exactly one {MANIFEST_NAME}")


def _verify_reconstructed_candidate(
    candidate_root: Path,
    trusted_base_root: Path,
    *,
    prospective_merge: bool,
) -> ReleaseState:
    """Rebuild and compare a prepared release without executing candidate code.

    The candidate manifest supplies only bounded release inputs. The expected
    tree is regenerated from the trusted PR base using the base branch's
    generator and synchronizers, then compared byte-for-byte (including modes,
    symlink targets, paths, and the release-state manifest).
    """
    candidate_root = candidate_root.resolve()
    trusted_base_root = trusted_base_root.resolve()
    _require_clean_checkout(candidate_root, "candidate")
    _require_clean_checkout(trusted_base_root, "trusted base")
    if prospective_merge:
        _require_prospective_tree_descends_from_base(candidate_root, trusted_base_root)
    else:
        _require_single_preparation_commit(candidate_root, trusted_base_root)
    _require_candidate_manifest_tracked(candidate_root)
    candidate_state = verify(candidate_root)

    trusted_crate, trusted_version = _load_package(trusted_base_root)
    if trusted_crate != candidate_state.crate_name:
        raise ReleaseStateError(
            f"trusted base crate name {trusted_crate!r} does not match candidate "
            f"{candidate_state.crate_name!r}"
        )
    if trusted_version != candidate_state.previous_version:
        raise ReleaseStateError(
            f"trusted base version {trusted_version} does not match candidate "
            f"previous_version {candidate_state.previous_version}"
        )

    with tempfile.TemporaryDirectory(
        prefix="fortress-expected-release-"
    ) as temporary_directory:
        expected_root = Path(temporary_directory).resolve()
        try:
            prepare_release._copy_tracked_sandbox(  # noqa: SLF001 - canonical copier
                trusted_base_root, expected_root
            )
            current, target, prepared_files, _workspace_roots = (
                prepare_release.prepare(
                    expected_root,
                    candidate_state.bump,
                    candidate_state.release_date,
                )
            )
            if current != candidate_state.previous_version:
                raise ReleaseStateError(
                    f"trusted generator started from {current}, expected "
                    f"{candidate_state.previous_version}"
                )
            if target != candidate_state.target_version:
                raise ReleaseStateError(
                    f"trusted generator produced {target}, expected "
                    f"{candidate_state.target_version}"
                )
            prepare_release.apply_prepared(prepared_files)
        except prepare_release.PreparationError as error:
            raise ReleaseStateError(
                f"trusted release reconstruction failed: {error}"
            ) from error

        _run_trusted_issue_template_sync(
            trusted_base_root, expected_root, candidate_state.target_version
        )
        expected_state = generate(
            expected_root,
            previous_version=candidate_state.previous_version,
            target_version=candidate_state.target_version,
            bump=candidate_state.bump,
            release_date=candidate_state.release_date,
        )
        _stage_expected_manifest(expected_root)
        verify(expected_root, expected_version=candidate_state.target_version)

        if candidate_state != expected_state:
            raise ReleaseStateError(
                "candidate release state does not match trusted reconstruction"
            )
        candidate_digest = _tracked_tree_digest(candidate_root, include_manifest=True)
        expected_digest = _tracked_tree_digest(expected_root, include_manifest=True)
        if candidate_digest != expected_digest:
            raise ReleaseStateError(
                "candidate tracked tree does not match trusted reconstruction: "
                f"expected {expected_digest}, got {candidate_digest}"
            )
    return candidate_state


def verify_prepared_candidate(
    candidate_root: Path, trusted_base_root: Path
) -> ReleaseState:
    """Reconstruct a release PR that must be one commit atop its reviewed base."""
    return _verify_reconstructed_candidate(
        candidate_root,
        trusted_base_root,
        prospective_merge=False,
    )


def verify_prospective_candidate(
    candidate_root: Path, trusted_base_root: Path
) -> ReleaseState:
    """Reconstruct and compare the exact prospective merge-queue release tree."""
    return _verify_reconstructed_candidate(
        candidate_root,
        trusted_base_root,
        prospective_merge=True,
    )


def _print_outputs(state: ReleaseState) -> None:
    print(f"crate_name={state.crate_name}")
    print(f"previous_version={state.previous_version}")
    print(f"target_version={state.target_version}")
    print(f"bump={state.bump}")
    print(f"release_date={state.release_date}")
    print(f"source_sha256={state.source_sha256}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--repo-root", type=Path, default=Path(__file__).resolve().parents[2]
    )
    subparsers = parser.add_subparsers(dest="command", required=True)
    generate_parser = subparsers.add_parser("generate")
    generate_parser.add_argument("--previous-version", required=True)
    generate_parser.add_argument("--target-version", required=True)
    generate_parser.add_argument("--bump", choices=sorted(VALID_BUMPS), required=True)
    generate_parser.add_argument("--date", required=True)
    verify_parser = subparsers.add_parser("verify")
    verify_parser.add_argument("--expected-version")
    candidate_parser = subparsers.add_parser("verify-candidate")
    candidate_parser.add_argument("--candidate-root", type=Path, required=True)
    candidate_parser.add_argument("--trusted-base-root", type=Path, required=True)
    prospective_parser = subparsers.add_parser("verify-prospective")
    prospective_parser.add_argument("--candidate-root", type=Path, required=True)
    prospective_parser.add_argument("--trusted-base-root", type=Path, required=True)
    try:
        if args := parser.parse_args():
            if args.command == "generate":
                state = generate(
                    args.repo_root,
                    previous_version=args.previous_version,
                    target_version=args.target_version,
                    bump=args.bump,
                    release_date=args.date,
                )
            elif args.command == "verify":
                state = verify(args.repo_root, expected_version=args.expected_version)
            elif args.command == "verify-candidate":
                state = verify_prepared_candidate(
                    args.candidate_root,
                    args.trusted_base_root,
                )
            else:
                state = verify_prospective_candidate(
                    args.candidate_root,
                    args.trusted_base_root,
                )
            _print_outputs(state)
            return 0
    except ReleaseStateError as error:
        print(f"release-state: error: {error}", file=sys.stderr)
        return 1
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
