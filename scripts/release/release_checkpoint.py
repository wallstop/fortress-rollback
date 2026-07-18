#!/usr/bin/env python3
"""Resolve, create, and revalidate the immutable Git release checkpoint."""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
from dataclasses import dataclass, replace
from pathlib import Path

SCRIPT_DIRECTORY = Path(__file__).resolve().parent
if str(SCRIPT_DIRECTORY) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIRECTORY))

import release_state


SCHEMA_VERSION = 1
MAX_FIRST_PARENT_COMMITS = 256
MAX_RELEASE_STATE_BYTES = 64 * 1024
STRICT_VERSION = re.compile(
    r"^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$"
)
OBJECT_ID = re.compile(r"^[0-9a-f]{40}$")
TAGGER = "github-actions[bot] <41898282+github-actions[bot]@users.noreply.github.com>"


class CheckpointError(RuntimeError):
    """The release checkpoint cannot be trusted or changed safely."""


@dataclass(frozen=True)
class RemoteTag:
    """The exact advertised annotated-tag object and its peeled commit."""

    direct_oid: str
    target_oid: str


@dataclass(frozen=True)
class Checkpoint:
    """Trusted state carried between isolated release workflow steps."""

    version: str
    tag_name: str
    trusted_sha: str
    candidate_sha: str
    direct_oid: str | None

    def to_json(self) -> str:
        """Render the stable checkpoint representation."""
        return json.dumps(
            {
                "schema_version": SCHEMA_VERSION,
                "version": self.version,
                "tag_name": self.tag_name,
                "trusted_sha": self.trusted_sha,
                "candidate_sha": self.candidate_sha,
                "direct_oid": self.direct_oid,
            },
            indent=2,
            sort_keys=True,
        ) + "\n"


def _run(
    repo: Path,
    arguments: list[str],
    *,
    check: bool = True,
    input_text: str | None = None,
) -> subprocess.CompletedProcess[str]:
    try:
        result = subprocess.run(
            ["git", "-C", str(repo), *arguments],
            check=False,
            capture_output=True,
            text=True,
            input=input_text,
        )
    except (OSError, UnicodeError) as error:
        raise CheckpointError(f"could not start git: {error}") from error
    if check and result.returncode != 0:
        details = result.stderr.strip() or result.stdout.strip()
        raise CheckpointError(
            f"git {' '.join(arguments)} failed with exit code "
            f"{result.returncode}: {details}"
        )
    return result


def _validate_oid(value: str, field: str) -> str:
    if OBJECT_ID.fullmatch(value) is None:
        raise CheckpointError(f"{field} must be a lowercase 40-character Git object ID")
    return value


def _validate_version(version: str) -> str:
    if STRICT_VERSION.fullmatch(version) is None:
        raise CheckpointError(
            f"release_version must use strict X.Y.Z semver; got {version!r}"
        )
    return version


def _single_advertisement(output: str, expected_ref: str) -> str | None:
    matches: list[str] = []
    for line in output.splitlines():
        fields = line.split()
        if len(fields) != 2:
            raise CheckpointError(f"malformed ls-remote output: {line!r}")
        oid, ref_name = fields
        if ref_name == expected_ref:
            matches.append(_validate_oid(oid, f"object for {expected_ref}"))
    if len(matches) > 1:
        raise CheckpointError(f"remote advertised {expected_ref} more than once")
    return matches[0] if matches else None


def _remote_tag(repo: Path, remote: str, tag_name: str) -> RemoteTag | None:
    direct_ref = f"refs/tags/{tag_name}"
    peeled_ref = f"{direct_ref}^{{}}"
    result = _run(repo, ["ls-remote", remote, direct_ref, peeled_ref])
    direct = _single_advertisement(result.stdout, direct_ref)
    peeled = _single_advertisement(result.stdout, peeled_ref)
    if direct is None and peeled is None:
        return None
    if direct is None:
        raise CheckpointError(f"remote advertised a peeled {tag_name} without its tag object")
    if peeled is None:
        raise CheckpointError(
            f"tag {tag_name} exists but is not an annotated release checkpoint"
        )
    return RemoteTag(direct_oid=direct, target_oid=peeled)


def _assert_same_remote_tag(
    expected: RemoteTag | None, actual: RemoteTag | None, tag_name: str
) -> None:
    if actual != expected:
        raise CheckpointError(
            f"tag {tag_name} changed during release checkpoint validation: "
            f"expected {expected!r}, found {actual!r}"
        )


def _fetch_tag(repo: Path, remote: str, tag_name: str) -> None:
    destination = f"refs/fortress-release-checkpoints/{tag_name}"
    _run(
        repo,
        ["fetch", "--no-tags", remote, f"+refs/tags/{tag_name}:{destination}"],
    )


def _validate_local_tag(repo: Path, tag_name: str, remote_tag: RemoteTag) -> None:
    object_type = _run(repo, ["cat-file", "-t", remote_tag.direct_oid]).stdout.strip()
    if object_type != "tag":
        raise CheckpointError(
            f"tag {tag_name} direct object {remote_tag.direct_oid} has type "
            f"{object_type!r}, expected 'tag'"
        )
    contents = _run(repo, ["cat-file", "-p", remote_tag.direct_oid]).stdout
    header_text = contents.split("\n\n", maxsplit=1)[0]
    headers: dict[str, str] = {}
    for line in header_text.splitlines():
        key, separator, value = line.partition(" ")
        if not separator or key in headers:
            raise CheckpointError(f"tag {tag_name} has malformed or duplicate headers")
        headers[key] = value
    required = {"object", "type", "tag"}
    if not required.issubset(headers):
        raise CheckpointError(f"tag {tag_name} is missing required annotated-tag headers")
    if headers["type"] != "commit":
        raise CheckpointError(
            f"tag {tag_name} directly targets {headers['type']!r}; nested tags are forbidden"
        )
    direct_target = _validate_oid(headers["object"], f"direct target for {tag_name}")
    if direct_target != remote_tag.target_oid:
        raise CheckpointError(
            f"tag {tag_name} direct target {direct_target} does not match advertised "
            f"peeled commit {remote_tag.target_oid}"
        )
    if headers["tag"] != tag_name:
        raise CheckpointError(
            f"tag object declares {headers['tag']!r}, expected {tag_name!r}"
        )


def _assert_commit(repo: Path, oid: str, field: str) -> None:
    object_type = _run(repo, ["cat-file", "-t", oid]).stdout.strip()
    if object_type != "commit":
        raise CheckpointError(f"{field} {oid} has type {object_type!r}, expected 'commit'")


def _assert_ancestor(repo: Path, candidate: str, trusted: str) -> None:
    result = _run(repo, ["merge-base", "--is-ancestor", candidate, trusted], check=False)
    if result.returncode == 1:
        raise CheckpointError(
            f"candidate tag commit {candidate} is not an ancestor of trusted "
            f"dispatch commit {trusted}"
        )
    if result.returncode != 0:
        details = result.stderr.strip() or result.stdout.strip()
        raise CheckpointError(f"could not validate candidate ancestry: {details}")


def _assert_first_parent_ancestor(
    repo: Path,
    ancestor: str,
    descendant: str,
    *,
    relationship: str,
) -> None:
    """Require ``ancestor`` on ``descendant``'s unbounded first-parent chain."""
    ancestry = _run(
        repo,
        ["merge-base", "--is-ancestor", ancestor, descendant],
        check=False,
    )
    if ancestry.returncode == 1:
        raise CheckpointError(f"{relationship} is not on trusted first-parent history")
    if ancestry.returncode != 0:
        details = ancestry.stderr.strip() or ancestry.stdout.strip()
        raise CheckpointError(
            f"could not validate {relationship} ancestry: {details}"
        )

    distance_text = _run(
        repo,
        ["rev-list", "--first-parent", "--count", f"{ancestor}..{descendant}"],
    ).stdout.strip()
    if not distance_text.isdigit():
        raise CheckpointError(
            f"could not validate {relationship} first-parent distance: "
            f"{distance_text!r}"
        )
    first_parent = _run(
        repo,
        ["rev-parse", f"{descendant}~{distance_text}^{{commit}}"],
        check=False,
    )
    if first_parent.returncode != 0 or first_parent.stdout.strip() != ancestor:
        raise CheckpointError(f"{relationship} is not on trusted first-parent history")


def _assert_trusted_checkout(repo: Path, trusted_sha: str) -> None:
    _assert_commit(repo, trusted_sha, "trusted dispatch SHA")
    head = _run(repo, ["rev-parse", "HEAD^{commit}"]).stdout.strip()
    if head != trusted_sha:
        raise CheckpointError(
            f"trusted helper checkout is at {head}, not dispatch commit {trusted_sha}"
        )


def _commit_release_metadata(
    repo: Path, commit: str, version: str
) -> release_state.ReleaseState | None:
    """Read bounded, strictly shaped manifest metadata without candidate code."""
    object_name = f"{commit}:{release_state.MANIFEST_NAME}"
    size_result = _run(repo, ["cat-file", "-s", object_name], check=False)
    if size_result.returncode != 0:
        return None
    size_text = size_result.stdout.strip()
    if not size_text.isdigit():
        raise CheckpointError(
            f"release-state object at {commit} has invalid size {size_text!r}"
        )
    size = int(size_text)
    if size > MAX_RELEASE_STATE_BYTES:
        return None
    content = _run(repo, ["cat-file", "blob", object_name]).stdout
    try:
        document = json.loads(content)
    except json.JSONDecodeError:
        return None
    expected_fields = {
        "schema_version",
        "crate_name",
        "previous_version",
        "target_version",
        "bump",
        "release_date",
        "source_sha256",
    }
    if not isinstance(document, dict) or set(document) != expected_fields:
        return None
    if document.get("schema_version") != release_state.SCHEMA_VERSION:
        return None
    string_fields = expected_fields - {"schema_version"}
    if any(not isinstance(document.get(field), str) for field in string_fields):
        return None
    metadata = release_state.ReleaseState(
        crate_name=document["crate_name"],
        previous_version=document["previous_version"],
        target_version=document["target_version"],
        bump=document["bump"],
        release_date=document["release_date"],
        source_sha256=document["source_sha256"],
    )
    try:
        release_state._validate_metadata(metadata)  # noqa: SLF001 - trusted parser
    except release_state.ReleaseStateError:
        return None
    return metadata if metadata.target_version == version else None


def _commit_has_valid_release_state(repo: Path, commit: str, version: str) -> bool:
    """Validate one historical tree with the dispatch commit's trusted verifier."""
    with tempfile.TemporaryDirectory(
        prefix="fortress-release-candidate-validation-"
    ) as temporary_directory:
        candidate = Path(temporary_directory) / "candidate"
        _run(repo, ["worktree", "add", "--detach", str(candidate), commit])
        verification_error: release_state.ReleaseStateError | None = None
        try:
            release_state.verify(candidate, expected_version=version)
        except release_state.ReleaseStateError as error:
            verification_error = error
        finally:
            _run(repo, ["worktree", "remove", "--force", str(candidate)])
        return verification_error is None


def _resolve_untagged_candidate(
    repo: Path, remote: str, trusted_sha: str, version: str
) -> str:
    """Find exactly one valid release tree since the prior release checkpoint."""
    history = _run(
        repo,
        [
            "rev-list",
            "--first-parent",
            f"--max-count={MAX_FIRST_PARENT_COMMITS + 1}",
            trusted_sha,
        ],
    ).stdout.splitlines()
    if not history:
        raise CheckpointError("trusted dispatch commit has no first-parent history")
    history = [
        _validate_oid(commit, "first-parent commit") for commit in history
    ]
    searchable_history = history[:MAX_FIRST_PARENT_COMMITS]
    metadata = [
        state
        for commit in searchable_history
        if (state := _commit_release_metadata(repo, commit, version)) is not None
    ]
    previous_versions = {state.previous_version for state in metadata}
    if not previous_versions:
        if len(history) > MAX_FIRST_PARENT_COMMITS:
            raise CheckpointError(
                "no prepared-release metadata found within the bounded first-parent "
                f"search of {MAX_FIRST_PARENT_COMMITS} commits for {version}"
            )
        raise CheckpointError(
            f"no digest-valid prepared-release commit found for {version} on trusted "
            "first-parent history"
        )
    if len(previous_versions) != 1:
        raise CheckpointError(
            f"prepared-release metadata for {version} names multiple previous versions: "
            f"{', '.join(sorted(previous_versions))}"
        )
    previous_version = previous_versions.pop()
    previous_tag_name = f"v{previous_version}"
    previous_tag = _remote_tag(repo, remote, previous_tag_name)
    if previous_tag is None:
        raise CheckpointError(
            f"previous release checkpoint {previous_tag_name} does not exist"
        )
    _fetch_tag(repo, remote, previous_tag_name)
    _assert_same_remote_tag(
        previous_tag,
        _remote_tag(repo, remote, previous_tag_name),
        previous_tag_name,
    )
    _validate_local_tag(repo, previous_tag_name, previous_tag)
    _assert_commit(repo, previous_tag.target_oid, "previous release tag commit")
    _assert_first_parent_ancestor(
        repo,
        previous_tag.target_oid,
        trusted_sha,
        relationship=f"previous release checkpoint {previous_tag_name}",
    )

    candidates: list[str] = []
    for candidate in searchable_history:
        if _commit_release_metadata(repo, candidate, version) is None:
            continue
        if _commit_has_valid_release_state(repo, candidate, version):
            candidates.append(candidate)
    if len(candidates) > 1:
        raise CheckpointError(
            f"multiple digest-valid prepared-release commits found for {version}: "
            f"{', '.join(candidates)}"
        )
    if candidates:
        candidate = candidates[0]
        if candidate == previous_tag.target_oid:
            raise CheckpointError(
                f"prepared release candidate {candidate} does not follow previous "
                f"release checkpoint {previous_tag_name}"
            )
        _assert_first_parent_ancestor(
            repo,
            previous_tag.target_oid,
            candidate,
            relationship=(
                f"prepared release candidate {candidate} after previous release "
                f"checkpoint {previous_tag_name}"
            ),
        )
        _assert_same_remote_tag(
            previous_tag,
            _remote_tag(repo, remote, previous_tag_name),
            previous_tag_name,
        )
        return candidate
    if len(history) > MAX_FIRST_PARENT_COMMITS:
        raise CheckpointError(
            "no digest-valid prepared-release commit found within the bounded "
            f"first-parent search of {MAX_FIRST_PARENT_COMMITS} commits for {version}"
        )
    raise CheckpointError(
        f"no digest-valid prepared-release commit found for {version} on trusted "
        "first-parent history"
    )


def _write_checkpoint(path: Path, checkpoint: Checkpoint) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary_name: str | None = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="w",
            encoding="utf-8",
            newline="",
            dir=path.parent,
            prefix=f".{path.name}.",
            delete=False,
        ) as temporary:
            temporary.write(checkpoint.to_json())
            temporary.flush()
            os.fsync(temporary.fileno())
            temporary_name = temporary.name
        os.replace(temporary_name, path)
        temporary_name = None
    except OSError as error:
        raise CheckpointError(f"cannot write checkpoint state {path}: {error}") from error
    finally:
        if temporary_name is not None:
            try:
                Path(temporary_name).unlink(missing_ok=True)
            except OSError:
                pass


def _load_checkpoint(path: Path) -> Checkpoint:
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise CheckpointError(f"cannot load checkpoint state {path}: {error}") from error
    if not isinstance(document, dict) or document.get("schema_version") != SCHEMA_VERSION:
        raise CheckpointError("checkpoint state has an unsupported schema")
    expected_fields = {
        "schema_version",
        "version",
        "tag_name",
        "trusted_sha",
        "candidate_sha",
        "direct_oid",
    }
    if set(document) != expected_fields:
        raise CheckpointError("checkpoint state has missing or unexpected fields")
    version = document.get("version")
    tag_name = document.get("tag_name")
    trusted_sha = document.get("trusted_sha")
    candidate_sha = document.get("candidate_sha")
    direct_oid = document.get("direct_oid")
    if not isinstance(version, str):
        raise CheckpointError("checkpoint version must be a string")
    if not isinstance(tag_name, str):
        raise CheckpointError("checkpoint tag_name must be a string")
    if not isinstance(trusted_sha, str):
        raise CheckpointError("checkpoint trusted_sha must be a string")
    if not isinstance(candidate_sha, str):
        raise CheckpointError("checkpoint state contains a non-string identity field")
    _validate_version(version)
    if tag_name != f"v{version}":
        raise CheckpointError("checkpoint tag does not match its version")
    _validate_oid(trusted_sha, "trusted_sha")
    _validate_oid(candidate_sha, "candidate_sha")
    if direct_oid is not None:
        if not isinstance(direct_oid, str):
            raise CheckpointError("checkpoint direct_oid must be a string or null")
        _validate_oid(direct_oid, "direct_oid")
    return Checkpoint(version, tag_name, trusted_sha, candidate_sha, direct_oid)


def _prepare_candidate(repo: Path, candidate_path: Path, candidate_sha: str) -> None:
    if candidate_path.exists():
        try:
            if any(candidate_path.iterdir()):
                raise CheckpointError(
                    f"candidate worktree path is not empty: {candidate_path}"
                )
        except OSError as error:
            raise CheckpointError(f"cannot inspect candidate path: {error}") from error
        try:
            candidate_path.rmdir()
        except OSError as error:
            raise CheckpointError(f"cannot prepare candidate path: {error}") from error
    _run(repo, ["worktree", "add", "--detach", str(candidate_path), candidate_sha])


def resolve(
    repo: Path,
    *,
    remote: str,
    version: str,
    trusted_sha: str,
    candidate_path: Path,
    state_file: Path,
) -> Checkpoint:
    """Resolve the candidate without trusting or executing candidate code."""
    repo = repo.resolve()
    version = _validate_version(version)
    trusted_sha = _validate_oid(trusted_sha, "trusted dispatch SHA")
    _assert_trusted_checkout(repo, trusted_sha)
    tag_name = f"v{version}"
    first = _remote_tag(repo, remote, tag_name)
    if first is None:
        _assert_same_remote_tag(None, _remote_tag(repo, remote, tag_name), tag_name)
        candidate_sha = _resolve_untagged_candidate(
            repo, remote, trusted_sha, version
        )
        direct_oid = None
    else:
        _fetch_tag(repo, remote, tag_name)
        _assert_same_remote_tag(first, _remote_tag(repo, remote, tag_name), tag_name)
        _validate_local_tag(repo, tag_name, first)
        _assert_commit(repo, first.target_oid, "candidate tag commit")
        _assert_ancestor(repo, first.target_oid, trusted_sha)
        candidate_sha = first.target_oid
        direct_oid = first.direct_oid
    checkpoint = Checkpoint(
        version=version,
        tag_name=tag_name,
        trusted_sha=trusted_sha,
        candidate_sha=candidate_sha,
        direct_oid=direct_oid,
    )
    _prepare_candidate(repo, candidate_path.resolve(), candidate_sha)
    _write_checkpoint(state_file.resolve(), checkpoint)
    return checkpoint


def _deterministic_tag(repo: Path, checkpoint: Checkpoint) -> str:
    timestamp = _run(
        repo, ["show", "-s", "--format=%ct", checkpoint.candidate_sha]
    ).stdout.strip()
    if not timestamp.isdigit():
        raise CheckpointError(f"candidate commit has invalid timestamp {timestamp!r}")
    tag_document = (
        f"object {checkpoint.candidate_sha}\n"
        "type commit\n"
        f"tag {checkpoint.tag_name}\n"
        f"tagger {TAGGER} {timestamp} +0000\n\n"
        f"Release {checkpoint.tag_name}\n"
    )
    oid = _run(repo, ["mktag"], input_text=tag_document).stdout.strip()
    return _validate_oid(oid, "created annotated tag object")


def _push_tag(repo: Path, remote: str, direct_oid: str, tag_name: str) -> int:
    result = _run(
        repo,
        ["push", remote, f"{direct_oid}:refs/tags/{tag_name}"],
        check=False,
    )
    return result.returncode


def _validate_checkpoint(
    repo: Path,
    remote: str,
    candidate_path: Path,
    checkpoint: Checkpoint,
    *,
    require_tag: bool,
) -> RemoteTag | None:
    _assert_trusted_checkout(repo, checkpoint.trusted_sha)
    candidate_head = _run(candidate_path, ["rev-parse", "HEAD^{commit}"]).stdout.strip()
    if candidate_head != checkpoint.candidate_sha:
        raise CheckpointError(
            f"candidate checkout moved to {candidate_head}, expected {checkpoint.candidate_sha}"
        )
    _assert_ancestor(repo, checkpoint.candidate_sha, checkpoint.trusted_sha)
    remote_tag = _remote_tag(repo, remote, checkpoint.tag_name)
    if checkpoint.direct_oid is None:
        if require_tag:
            raise CheckpointError("release checkpoint has not been created")
        _assert_same_remote_tag(None, remote_tag, checkpoint.tag_name)
        return None
    expected = RemoteTag(checkpoint.direct_oid, checkpoint.candidate_sha)
    _assert_same_remote_tag(expected, remote_tag, checkpoint.tag_name)
    _fetch_tag(repo, remote, checkpoint.tag_name)
    _assert_same_remote_tag(expected, _remote_tag(repo, remote, checkpoint.tag_name), checkpoint.tag_name)
    _validate_local_tag(repo, checkpoint.tag_name, expected)
    return expected


def create(
    repo: Path,
    *,
    remote: str,
    candidate_path: Path,
    state_file: Path,
) -> Checkpoint:
    """Create the deterministic tag or accept only an identical push race."""
    repo = repo.resolve()
    state_file = state_file.resolve()
    checkpoint = _load_checkpoint(state_file)
    candidate_path = candidate_path.resolve()
    if checkpoint.direct_oid is not None:
        _validate_checkpoint(
            repo, remote, candidate_path, checkpoint, require_tag=True
        )
        return checkpoint
    _assert_trusted_checkout(repo, checkpoint.trusted_sha)
    candidate_head = _run(candidate_path, ["rev-parse", "HEAD^{commit}"]).stdout.strip()
    if candidate_head != checkpoint.candidate_sha:
        raise CheckpointError(
            f"candidate checkout moved to {candidate_head}, expected {checkpoint.candidate_sha}"
        )
    _assert_ancestor(repo, checkpoint.candidate_sha, checkpoint.trusted_sha)
    direct_oid = _deterministic_tag(repo, checkpoint)
    expected = RemoteTag(direct_oid, checkpoint.candidate_sha)
    actual = _remote_tag(repo, remote, checkpoint.tag_name)
    if actual is not None and actual != expected:
        raise CheckpointError(
            f"tag {checkpoint.tag_name} push raced to a different checkpoint: "
            f"expected {expected!r}, found {actual!r}"
        )
    if actual is None:
        push_result = _push_tag(repo, remote, direct_oid, checkpoint.tag_name)
        actual = _remote_tag(repo, remote, checkpoint.tag_name)
        if push_result != 0 and actual != expected:
            raise CheckpointError(
                f"tag {checkpoint.tag_name} push raced to a different checkpoint: "
                f"expected {expected!r}, found {actual!r}"
            )
    _assert_same_remote_tag(expected, actual, checkpoint.tag_name)
    updated = replace(checkpoint, direct_oid=direct_oid)
    _write_checkpoint(state_file, updated)
    _validate_checkpoint(
        repo, remote, candidate_path, updated, require_tag=True
    )
    return updated


def verify(
    repo: Path,
    *,
    remote: str,
    candidate_path: Path,
    state_file: Path,
) -> Checkpoint:
    """Re-query and fail closed unless the exact checkpoint remains immutable."""
    checkpoint = _load_checkpoint(state_file.resolve())
    _validate_checkpoint(
        repo.resolve(),
        remote,
        candidate_path.resolve(),
        checkpoint,
        require_tag=True,
    )
    return checkpoint


def _print_outputs(checkpoint: Checkpoint) -> None:
    print(f"version={checkpoint.version}")
    print(f"tag_name={checkpoint.tag_name}")
    print(f"trusted_sha={checkpoint.trusted_sha}")
    print(f"candidate_sha={checkpoint.candidate_sha}")
    print(f"direct_oid={checkpoint.direct_oid or ''}")
    print(f"existing={'true' if checkpoint.direct_oid is not None else 'false'}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--remote", default="origin")
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--state-file", type=Path, required=True)
    subparsers = parser.add_subparsers(dest="command", required=True)
    resolve_parser = subparsers.add_parser("resolve")
    resolve_parser.add_argument("--version", required=True)
    resolve_parser.add_argument("--trusted-sha", required=True)
    subparsers.add_parser("create")
    subparsers.add_parser("verify")
    args = parser.parse_args()
    try:
        if args.command == "resolve":
            checkpoint = resolve(
                args.repo,
                remote=args.remote,
                version=args.version,
                trusted_sha=args.trusted_sha,
                candidate_path=args.candidate,
                state_file=args.state_file,
            )
        elif args.command == "create":
            checkpoint = create(
                args.repo,
                remote=args.remote,
                candidate_path=args.candidate,
                state_file=args.state_file,
            )
        else:
            checkpoint = verify(
                args.repo,
                remote=args.remote,
                candidate_path=args.candidate,
                state_file=args.state_file,
            )
    except CheckpointError as error:
        print(f"release-checkpoint: error: {error}", file=sys.stderr)
        return 1
    _print_outputs(checkpoint)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
