#!/usr/bin/env python3
"""Decide whether a pull request requires trusted release reconstruction."""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

SCRIPT_DIRECTORY = Path(__file__).resolve().parent
if str(SCRIPT_DIRECTORY) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIRECTORY))

from release_state import MANIFEST_NAME, ReleaseStateError, load


COMMIT_ID = re.compile(r"^[0-9a-f]{40}(?:[0-9a-f]{24})?$")
RELEASE_BRANCH = re.compile(
    r"^release/v(?P<version>(?:0|[1-9][0-9]*)\."
    r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*))$"
)


class ReleaseStateCiError(RuntimeError):
    """The release-state pull-request gate could not make a safe decision."""


@dataclass(frozen=True)
class GateDecision:
    """Whether the pull request needs complete trusted reconstruction."""

    reconstruction_required: bool
    target_version: str | None = None


def _git_bytes(repo_root: Path, args: list[str], description: str) -> bytes:
    try:
        result = subprocess.run(
            ["git", "-C", str(repo_root), *args],
            capture_output=True,
            check=False,
        )
    except OSError as error:
        raise ReleaseStateCiError(f"{description} could not start: {error}") from error
    if result.returncode != 0:
        details = (result.stderr or result.stdout).decode(errors="replace").strip()
        raise ReleaseStateCiError(
            f"{description} failed with exit code {result.returncode}: {details}"
        )
    return result.stdout


def _commit(repo_root: Path, description: str) -> str:
    raw = _git_bytes(repo_root, ["rev-parse", "HEAD^{commit}"], description)
    try:
        commit = raw.decode("ascii").strip()
    except UnicodeDecodeError as error:
        raise ReleaseStateCiError(f"{description} returned non-ASCII data") from error
    if COMMIT_ID.fullmatch(commit) is None:
        raise ReleaseStateCiError(f"{description} returned malformed commit {commit!r}")
    return commit


def _require_commit_id(value: str, label: str) -> None:
    if COMMIT_ID.fullmatch(value) is None:
        raise ReleaseStateCiError(f"{label} must be 40 or 64 lowercase hex digits")


def _require_base_ancestor(candidate_root: Path, base_sha: str) -> None:
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
        raise ReleaseStateCiError(
            f"merge-group ancestry lookup could not start: {error}"
        ) from error
    if result.returncode == 1:
        raise ReleaseStateCiError(
            f"merge-group head does not descend from event base {base_sha}"
        )
    if result.returncode != 0:
        details = (result.stderr or result.stdout).strip()
        raise ReleaseStateCiError(
            "merge-group ancestry lookup failed with exit code "
            f"{result.returncode}: {details}"
        )


def _manifest_tree_entry(repo_root: Path, description: str) -> bytes | None:
    """Return the committed manifest tree entry without reading candidate code."""
    output = _git_bytes(
        repo_root,
        ["ls-tree", "-z", "HEAD", "--", MANIFEST_NAME],
        description,
    )
    if not output:
        return None
    records = [record for record in output.split(b"\0") if record]
    if len(records) != 1:
        raise ReleaseStateCiError(
            f"{description} returned {len(records)} entries; expected exactly one"
        )
    metadata, separator, path = records[0].partition(b"\t")
    fields = metadata.split()
    if (
        not separator
        or path != MANIFEST_NAME.encode("ascii")
        or len(fields) != 3
        or fields[0] != b"100644"
        or fields[1] != b"blob"
        or re.fullmatch(rb"[0-9a-f]{40}(?:[0-9a-f]{24})?", fields[2]) is None
    ):
        raise ReleaseStateCiError(f"{description} returned a malformed tree entry")
    # Include the mode as well as the object ID so an executable-bit change is gated.
    return records[0]


def evaluate(
    candidate_root: Path,
    trusted_base_root: Path,
    *,
    head_ref: str,
    expected_head_sha: str,
    expected_base_sha: str,
    prospective_merge: bool = False,
) -> GateDecision:
    """Classify a PR using only trusted code and committed Git tree metadata."""
    _require_commit_id(expected_head_sha, "expected head SHA")
    _require_commit_id(expected_base_sha, "expected base SHA")
    candidate_root = candidate_root.resolve()
    trusted_base_root = trusted_base_root.resolve()
    actual_head = _commit(candidate_root, "candidate commit lookup")
    actual_base = _commit(trusted_base_root, "trusted base commit lookup")
    if actual_head != expected_head_sha:
        raise ReleaseStateCiError(
            f"candidate checkout is {actual_head}, expected event head {expected_head_sha}"
        )
    if actual_base != expected_base_sha:
        raise ReleaseStateCiError(
            f"trusted base checkout is {actual_base}, expected event base {expected_base_sha}"
        )
    if prospective_merge:
        _require_base_ancestor(candidate_root, actual_base)

    candidate_entry = _manifest_tree_entry(
        candidate_root, "candidate release-state lookup"
    )
    base_entry = _manifest_tree_entry(
        trusted_base_root, "trusted base release-state lookup"
    )
    branch_match = RELEASE_BRANCH.fullmatch(head_ref)
    reserved_release_ref = not prospective_merge and head_ref.startswith("release/v")
    # Every pull-request head in the reserved release/v namespace crosses the
    # full-tree trust boundary. Do not let canonical or malformed release refs
    # bypass reconstruction by retaining the base manifest blob.
    if candidate_entry == base_entry and not reserved_release_ref:
        return GateDecision(reconstruction_required=False)

    if not prospective_merge and branch_match is None:
        raise ReleaseStateCiError(
            f"a {MANIFEST_NAME} change or reserved release/v* head requires a "
            f"canonical release/vX.Y.Z head; got {head_ref!r}"
        )
    if candidate_entry is None:
        raise ReleaseStateCiError(f"a release PR cannot delete {MANIFEST_NAME} or omit it")
    try:
        state = load(candidate_root)
    except ReleaseStateError as error:
        raise ReleaseStateCiError(f"candidate {MANIFEST_NAME} is invalid: {error}") from error
    if not prospective_merge:
        if branch_match is None:
            raise ReleaseStateCiError("release branch classification was lost")
        branch_version = branch_match.group("version")
        if branch_version != state.target_version:
            raise ReleaseStateCiError(
                f"release branch version {branch_version} does not match candidate "
                f"target_version {state.target_version}"
            )
    return GateDecision(reconstruction_required=True, target_version=state.target_version)


def _append_github_output(path: Path, decision: GateDecision) -> None:
    try:
        with path.open("a", encoding="utf-8", newline="") as output:
            output.write(
                "reconstruction_required="
                f"{'true' if decision.reconstruction_required else 'false'}\n"
            )
            if decision.target_version is not None:
                output.write(f"target_version={decision.target_version}\n")
    except OSError as error:
        raise ReleaseStateCiError(f"cannot write GitHub output {path}: {error}") from error


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate-root", type=Path, required=True)
    parser.add_argument("--trusted-base-root", type=Path, required=True)
    parser.add_argument("--head-ref", required=True)
    parser.add_argument("--expected-head-sha", required=True)
    parser.add_argument("--expected-base-sha", required=True)
    parser.add_argument(
        "--event-kind",
        choices=("pull-request", "merge-group"),
        default="pull-request",
    )
    parser.add_argument("--github-output", type=Path, required=True)
    args = parser.parse_args()
    try:
        decision = evaluate(
            args.candidate_root,
            args.trusted_base_root,
            head_ref=args.head_ref,
            expected_head_sha=args.expected_head_sha,
            expected_base_sha=args.expected_base_sha,
            prospective_merge=args.event_kind == "merge-group",
        )
        _append_github_output(args.github_output, decision)
    except ReleaseStateCiError as error:
        print(f"release-state-ci: error: {error}", file=sys.stderr)
        return 1
    if decision.reconstruction_required:
        print(
            f"trusted reconstruction is required for v{decision.target_version}"
        )
    else:
        print(f"{MANIFEST_NAME} is unchanged; no candidate code will run")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
