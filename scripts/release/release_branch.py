#!/usr/bin/env python3
"""Recover release-preparation branches and their pull requests safely."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

from release_state import ReleaseStateError, load, verify_prepared_candidate

STRICT_VERSION = re.compile(r"^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$")
STRICT_DATE = re.compile(r"^[0-9]{4}-[0-9]{2}-[0-9]{2}$")
STRICT_SHA = re.compile(r"^[0-9a-f]{40}$")
STRICT_RELEASE_BRANCH = re.compile(
    r"^release/v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$"
)


class ReleaseBranchError(RuntimeError):
    """A remote release branch or pull request conflicts with the request."""


@dataclass(frozen=True)
class BranchResolution:
    """Resolved release branch state for one preparation dispatch."""

    exists: bool
    branch: str
    base_sha: str
    branch_sha: str | None
    replace_sha: str | None
    release_date: str


@dataclass(frozen=True)
class PullRequestResolution:
    """Located or created release pull request."""

    created: bool
    url: str
    number: int | None = None


@dataclass(frozen=True)
class _PullRecord:
    """Validated canonical pull-request state returned by GitHub."""

    number: int
    state: str
    url: str
    head_sha: str
    merged: bool


def _run_git_result(repo_root: Path, args: list[str]) -> subprocess.CompletedProcess[str]:
    try:
        return subprocess.run(
            ["git", "-C", str(repo_root), *args],
            capture_output=True,
            text=True,
            check=False,
        )
    except (OSError, UnicodeError) as error:
        raise ReleaseBranchError(f"could not start git: {error}") from error


def _run_git(repo_root: Path, args: list[str], description: str) -> str:
    result = _run_git_result(repo_root, args)
    if result.returncode != 0:
        details = (result.stderr or result.stdout).strip()
        raise ReleaseBranchError(
            f"{description} failed with exit code {result.returncode}: {details}"
        )
    return result.stdout.strip()


def _require_clean(repo_root: Path) -> None:
    status = _run_git(
        repo_root,
        ["status", "--porcelain=v1", "--untracked-files=all", "--"],
        "working-tree status",
    )
    if status:
        raise ReleaseBranchError(
            "release branch recovery requires a clean working tree; found:\n" + status
        )


def _remote_ref(
    repo_root: Path, remote: str, kind: str, ref: str
) -> str | None:
    result = _run_git_result(
        repo_root,
        ["ls-remote", "--exit-code", f"--{kind}", remote, ref],
    )
    if result.returncode == 2:
        return None
    if result.returncode != 0:
        details = (result.stderr or result.stdout).strip()
        raise ReleaseBranchError(
            f"remote {kind.removesuffix('s')} lookup for {ref} failed with "
            f"exit code {result.returncode}: {details}"
        )
    lines = [line for line in result.stdout.splitlines() if line.strip()]
    if len(lines) != 1:
        raise ReleaseBranchError(
            f"remote {kind.removesuffix('s')} lookup for {ref} returned "
            f"{len(lines)} refs; expected exactly one"
        )
    fields = lines[0].split()
    if len(fields) != 2 or fields[1] != ref or re.fullmatch(r"[0-9a-f]{40}", fields[0]) is None:
        raise ReleaseBranchError(f"remote returned malformed data for {ref!r}")
    return fields[0]


def _require_valid_default_branch(repo_root: Path, default_branch: str) -> None:
    if not default_branch or "\n" in default_branch or "\r" in default_branch:
        raise ReleaseBranchError("default branch must be a valid branch name")
    result = _run_git_result(
        repo_root, ["check-ref-format", "--branch", default_branch]
    )
    if result.returncode != 0:
        raise ReleaseBranchError("default branch must be a valid branch name")


def require_remote_default(
    repo_root: Path,
    *,
    remote: str,
    default_branch: str,
    expected_sha: str,
) -> None:
    """Require the remote default branch to remain at the dispatch commit."""
    if STRICT_SHA.fullmatch(expected_sha) is None:
        raise ReleaseBranchError("expected default branch SHA must be 40 lowercase hex digits")
    repo_root = repo_root.resolve()
    _require_valid_default_branch(repo_root, default_branch)
    default_ref = f"refs/heads/{default_branch}"
    remote_default_sha = _remote_ref(repo_root, remote, "heads", default_ref)
    if remote_default_sha != expected_sha:
        raise ReleaseBranchError(
            f"dispatch base {expected_sha} is not current {remote}/{default_branch} "
            f"({remote_default_sha or '<missing>'}); rerun from the latest default branch"
        )


def _is_ancestor(repo_root: Path, ancestor: str, descendant: str) -> bool:
    result = _run_git_result(
        repo_root, ["merge-base", "--is-ancestor", ancestor, descendant]
    )
    if result.returncode == 0:
        return True
    if result.returncode == 1:
        return False
    details = (result.stderr or result.stdout).strip()
    raise ReleaseBranchError(
        "release branch ancestry lookup failed with exit code "
        f"{result.returncode}: {details}"
    )


def push_release_branch(
    repo_root: Path,
    *,
    remote: str,
    default_branch: str,
    expected_base_sha: str,
    branch: str,
    expected_branch_sha: str | None,
) -> str:
    """Push one prepared commit with exact default-branch and branch leases."""
    if STRICT_SHA.fullmatch(expected_base_sha) is None:
        raise ReleaseBranchError(
            "expected default branch SHA must be 40 lowercase hex digits"
        )
    if STRICT_RELEASE_BRANCH.fullmatch(branch) is None:
        raise ReleaseBranchError("release branch must use release/vX.Y.Z form")
    if expected_branch_sha is not None and STRICT_SHA.fullmatch(expected_branch_sha) is None:
        raise ReleaseBranchError(
            "expected release branch SHA must be 40 lowercase hex digits"
        )
    repo_root = repo_root.resolve()
    _require_clean(repo_root)
    head_sha = _run_git(repo_root, ["rev-parse", "HEAD^{commit}"], "prepared commit lookup")
    parents = _run_git(
        repo_root,
        ["rev-list", "--parents", "-n", "1", head_sha],
        "prepared commit ancestry check",
    ).split()
    if len(parents) != 2 or parents[1] != expected_base_sha:
        actual_parent = parents[1] if len(parents) >= 2 else "<not-one-parent>"
        raise ReleaseBranchError(
            f"prepared commit is not a single commit on dispatch base "
            f"{expected_base_sha}; parent is {actual_parent}"
        )

    branch_ref = f"refs/heads/{branch}"
    remote_branch_sha = _remote_ref(repo_root, remote, "heads", branch_ref)
    if remote_branch_sha != expected_branch_sha:
        raise ReleaseBranchError(
            f"release branch {branch} changed before push: expected "
            f"{expected_branch_sha or '<missing>'}, got "
            f"{remote_branch_sha or '<missing>'}"
        )

    # Keep this as the final remote read before the push. The exact lease below
    # protects both states: an empty expectation requires the ref to remain
    # absent, while a stale-branch recovery requires the inspected object ID.
    require_remote_default(
        repo_root,
        remote=remote,
        default_branch=default_branch,
        expected_sha=expected_base_sha,
    )
    lease_sha = expected_branch_sha or ""
    default_ref = f"refs/heads/{default_branch}"
    push_args = [
        "push",
        "--atomic",
        f"--force-with-lease={branch_ref}:{lease_sha}",
        remote,
        f"{expected_base_sha}:{default_ref}",
        f"HEAD:{branch_ref}",
    ]
    _run_git(repo_root, push_args, "release branch compare-and-swap push")

    pushed_sha = _remote_ref(repo_root, remote, "heads", branch_ref)
    if pushed_sha != head_sha:
        raise ReleaseBranchError(
            f"release branch {branch} did not resolve to pushed commit {head_sha}; "
            f"got {pushed_sha or '<missing>'}"
        )
    return head_sha


def resolve_release_branch(
    repo_root: Path,
    *,
    remote: str,
    default_branch: str,
    previous_version: str,
    target_version: str,
    bump: str,
    requested_date: str,
) -> BranchResolution:
    """Reuse a matching prepared branch or report that a new one is needed."""
    if STRICT_VERSION.fullmatch(previous_version) is None:
        raise ReleaseBranchError("previous_version must use strict X.Y.Z semver")
    if STRICT_VERSION.fullmatch(target_version) is None:
        raise ReleaseBranchError("target_version must use strict X.Y.Z semver")
    if bump not in {"major", "minor", "patch"}:
        raise ReleaseBranchError("bump must be major, minor, or patch")
    if STRICT_DATE.fullmatch(requested_date) is None:
        raise ReleaseBranchError("requested_date must use YYYY-MM-DD")

    repo_root = repo_root.resolve()
    _require_clean(repo_root)
    _require_valid_default_branch(repo_root, default_branch)
    base_sha = _run_git(repo_root, ["rev-parse", "HEAD^{commit}"], "base commit lookup")
    require_remote_default(
        repo_root,
        remote=remote,
        default_branch=default_branch,
        expected_sha=base_sha,
    )
    branch = f"release/v{target_version}"
    branch_ref = f"refs/heads/{branch}"
    tag_ref = f"refs/tags/v{target_version}"

    if _remote_ref(repo_root, remote, "tags", tag_ref) is not None:
        raise ReleaseBranchError(
            f"release tag v{target_version} already exists; preparation cannot continue"
        )
    branch_sha = _remote_ref(repo_root, remote, "heads", branch_ref)
    if branch_sha is None:
        return BranchResolution(False, branch, base_sha, None, None, requested_date)

    candidate_ref = "refs/release-prepare/candidate"
    _run_git(
        repo_root,
        ["fetch", "--no-tags", "--force", remote, f"{branch_ref}:{candidate_ref}"],
        "existing release branch fetch",
    )
    fetched_sha = _run_git(
        repo_root, ["rev-parse", f"{candidate_ref}^{{commit}}"], "release branch commit lookup"
    )
    if fetched_sha != branch_sha:
        raise ReleaseBranchError(
            f"release branch {branch} changed during recovery; retry the dispatch"
        )
    parents = _run_git(
        repo_root,
        ["rev-list", "--parents", "-n", "1", fetched_sha],
        "release branch ancestry check",
    ).split()
    if len(parents) != 2:
        raise ReleaseBranchError(
            f"release branch {branch} is not a single preparation commit"
        )
    candidate_parent = parents[1]
    if not _is_ancestor(repo_root, candidate_parent, base_sha):
        raise ReleaseBranchError(
            f"release branch {branch} parent {candidate_parent} is not an ancestor "
            f"of dispatch base {base_sha}"
        )

    with tempfile.TemporaryDirectory(
        prefix="fortress-release-base-"
    ) as temporary_directory:
        trusted_base_root = Path(temporary_directory) / "base"
        _run_git(
            repo_root,
            ["worktree", "add", "--detach", str(trusted_base_root), candidate_parent],
            "trusted base worktree creation",
        )
        try:
            _run_git(
                repo_root,
                ["checkout", "--detach", fetched_sha],
                "release branch checkout",
            )
            try:
                state = load(repo_root)
            except ReleaseStateError as error:
                raise ReleaseBranchError(
                    f"release branch {branch} has invalid state: {error}"
                ) from error
            expected_fields = {
                "previous_version": (state.previous_version, previous_version),
                "target_version": (state.target_version, target_version),
                "bump": (state.bump, bump),
            }
            conflicts = [
                f"{field} is {actual!r}, expected {expected!r}"
                for field, (actual, expected) in expected_fields.items()
                if actual != expected
            ]
            if conflicts:
                raise ReleaseBranchError(
                    f"release branch {branch} conflicts with this dispatch: "
                    + "; ".join(conflicts)
                )
            try:
                verify_prepared_candidate(repo_root, trusted_base_root)
            except ReleaseStateError as error:
                raise ReleaseBranchError(
                    f"release branch {branch} failed trusted reconstruction: {error}"
                ) from error
        finally:
            _run_git(
                repo_root,
                ["worktree", "remove", "--force", str(trusted_base_root)],
                "trusted base worktree cleanup",
            )
    _require_clean(repo_root)
    _run_git(repo_root, ["update-ref", "-d", candidate_ref], "temporary ref cleanup")
    if candidate_parent == base_sha:
        return BranchResolution(
            True, branch, base_sha, fetched_sha, None, state.release_date
        )

    # The old branch is exactly the trusted generator output from an ancestor
    # of today's default branch. Return to the dispatch base and regenerate;
    # the caller may replace only the inspected SHA with force-with-lease.
    _run_git(repo_root, ["checkout", "--detach", base_sha], "dispatch base checkout")
    return BranchResolution(
        False, branch, base_sha, fetched_sha, fetched_sha, state.release_date
    )


def _run_gh(args: list[str], description: str) -> str:
    try:
        result = subprocess.run(
            ["gh", *args], capture_output=True, text=True, check=False
        )
    except (OSError, UnicodeError) as error:
        raise ReleaseBranchError(f"{description} could not start: {error}") from error
    if result.returncode != 0:
        details = (result.stderr or result.stdout).strip()
        raise ReleaseBranchError(
            f"{description} failed with exit code {result.returncode}: {details}"
        )
    return result.stdout


def ensure_release_pr(
    *,
    repository: str,
    base: str,
    head: str,
    head_sha: str,
    title: str,
    body_file: Path,
) -> PullRequestResolution:
    """Reuse the unique matching open PR or create it when none exists."""
    repository_parts = repository.split("/")
    if (
        len(repository_parts) != 2
        or any(not part for part in repository_parts)
        or any(part in {".", ".."} for part in repository_parts)
    ):
        raise ReleaseBranchError("repository must use canonical owner/name form")
    owner = repository_parts[0]
    canonical_head = f"{owner}:{head}"
    raw = _run_gh(
        [
            "api",
            "--method",
            "GET",
            f"repos/{repository}/pulls",
            "--raw-field",
            "state=all",
            "--raw-field",
            f"base={base}",
            "--raw-field",
            f"head={canonical_head}",
        ],
        "release pull request lookup",
    )
    try:
        pulls = json.loads(raw)
    except json.JSONDecodeError as error:
        raise ReleaseBranchError(f"release pull request lookup returned invalid JSON: {error}") from error
    if not isinstance(pulls, list) or any(not isinstance(item, dict) for item in pulls):
        raise ReleaseBranchError("release pull request lookup returned an unsupported shape")
    records: list[_PullRecord] = []
    for pull in pulls:
        number = pull.get("number")
        state = pull.get("state")
        url = pull.get("html_url")
        pull_head = pull.get("head")
        pull_base = pull.get("base")
        if not isinstance(pull_head, dict) or not isinstance(pull_base, dict):
            raise ReleaseBranchError(
                f"release branch {head} already has a malformed pull request"
            )
        head_repo = pull_head.get("repo")
        actual_repo = head_repo.get("full_name") if isinstance(head_repo, dict) else None
        actual_ref = pull_head.get("ref")
        actual_sha = pull_head.get("sha")
        actual_base = pull_base.get("ref")
        if (
            state not in {"open", "closed"}
            or not isinstance(number, int)
            or not isinstance(url, str)
            or not url.startswith("https://")
            or actual_repo != repository
            or actual_ref != head
            or actual_base != base
            or not isinstance(actual_sha, str)
            or STRICT_SHA.fullmatch(actual_sha) is None
        ):
            raise ReleaseBranchError(
                f"release branch {head} already has a mismatched or malformed "
                "pull request"
            )
        records.append(
            _PullRecord(
                number=number,
                state=state,
                url=url,
                head_sha=actual_sha,
                merged=pull.get("merged_at") is not None,
            )
        )

    open_records = [record for record in records if record.state == "open"]
    if len(open_records) > 1:
        raise ReleaseBranchError(
            f"found {len(open_records)} open pull requests for {head}; expected at most one"
        )
    if open_records:
        record = open_records[0]
        if record.head_sha != head_sha:
            raise ReleaseBranchError(
                f"release pull request #{record.number} head {record.head_sha!r} "
                f"does not match {head_sha}"
            )
        return PullRequestResolution(False, record.url, record.number)

    exact_closed = [record for record in records if record.head_sha == head_sha]
    if len(exact_closed) > 1:
        raise ReleaseBranchError(
            f"found {len(exact_closed)} closed pull requests for current {head}; "
            "refusing ambiguous recovery"
        )
    if exact_closed:
        record = exact_closed[0]
        if record.merged:
            raise ReleaseBranchError(
                f"release pull request #{record.number} already merged at current "
                f"head {head_sha}; no new release diff exists"
            )
        reopened_raw = _run_gh(
            [
                "api",
                "--method",
                "PATCH",
                f"repos/{repository}/pulls/{record.number}",
                "--raw-field",
                "state=open",
            ],
            "release pull request reopen",
        )
        try:
            reopened = json.loads(reopened_raw)
        except json.JSONDecodeError as error:
            raise ReleaseBranchError(
                f"release pull request reopen returned invalid JSON: {error}"
            ) from error
        if not isinstance(reopened, dict):
            raise ReleaseBranchError(
                "release pull request reopen returned an unsupported shape"
            )
        reopened_head = reopened.get("head")
        reopened_base = reopened.get("base")
        reopened_repo = (
            reopened_head.get("repo") if isinstance(reopened_head, dict) else None
        )
        if (
            reopened.get("number") != record.number
            or reopened.get("state") != "open"
            or reopened.get("html_url") != record.url
            or not isinstance(reopened_head, dict)
            or reopened_head.get("ref") != head
            or reopened_head.get("sha") != head_sha
            or not isinstance(reopened_repo, dict)
            or reopened_repo.get("full_name") != repository
            or not isinstance(reopened_base, dict)
            or reopened_base.get("ref") != base
        ):
            raise ReleaseBranchError(
                "release pull request reopen did not return the expected open PR"
            )
        return PullRequestResolution(False, record.url, record.number)

    try:
        body = body_file.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as error:
        raise ReleaseBranchError(
            f"cannot read release pull request body: {error}"
        ) from error
    created_raw = _run_gh(
        [
            "api",
            "--method",
            "POST",
            f"repos/{repository}/pulls",
            "--raw-field",
            f"base={base}",
            "--raw-field",
            f"head={canonical_head}",
            "--raw-field",
            f"title={title}",
            "--raw-field",
            f"body={body}",
        ],
        "release pull request creation",
    )
    try:
        created = json.loads(created_raw)
    except json.JSONDecodeError as error:
        raise ReleaseBranchError(
            f"release pull request creation returned invalid JSON: {error}"
        ) from error
    if not isinstance(created, dict):
        raise ReleaseBranchError(
            "release pull request creation returned an unsupported shape"
        )
    url = created.get("html_url")
    number = created.get("number")
    if not isinstance(url, str) or not url.startswith("https://"):
        raise ReleaseBranchError("release pull request creation returned no HTTPS URL")
    if not isinstance(number, int):
        raise ReleaseBranchError(
            "release pull request creation returned no pull request number"
        )
    return PullRequestResolution(True, url, number)


def _write_outputs(path: Path, values: dict[str, str]) -> None:
    try:
        with path.open("a", encoding="utf-8", newline="") as output:
            for key, value in values.items():
                if "\n" in value or "\r" in value:
                    raise ReleaseBranchError(f"output {key} contains a newline")
                output.write(f"{key}={value}\n")
    except OSError as error:
        raise ReleaseBranchError(f"cannot write workflow outputs: {error}") from error


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    resolve = subparsers.add_parser("resolve")
    resolve.add_argument("--repo-root", type=Path, default=Path.cwd())
    resolve.add_argument("--remote", default="origin")
    resolve.add_argument("--default-branch", required=True)
    resolve.add_argument("--previous-version", required=True)
    resolve.add_argument("--target-version", required=True)
    resolve.add_argument("--bump", required=True)
    resolve.add_argument("--requested-date", required=True)
    resolve.add_argument("--github-output", type=Path, required=True)

    verify_default = subparsers.add_parser("verify-default")
    verify_default.add_argument("--repo-root", type=Path, default=Path.cwd())
    verify_default.add_argument("--remote", default="origin")
    verify_default.add_argument("--default-branch", required=True)
    verify_default.add_argument("--expected-sha", required=True)

    push = subparsers.add_parser("push")
    push.add_argument("--repo-root", type=Path, default=Path.cwd())
    push.add_argument("--remote", default="origin")
    push.add_argument("--default-branch", required=True)
    push.add_argument("--expected-base-sha", required=True)
    push.add_argument("--branch", required=True)
    push.add_argument("--expected-branch-sha")

    ensure = subparsers.add_parser("ensure-pr")
    ensure.add_argument("--repository", required=True)
    ensure.add_argument("--base", required=True)
    ensure.add_argument("--head", required=True)
    ensure.add_argument("--head-sha", required=True)
    ensure.add_argument("--title", required=True)
    ensure.add_argument("--body-file", type=Path, required=True)
    ensure.add_argument("--github-output", type=Path, required=True)
    args = parser.parse_args()
    try:
        if args.command == "resolve":
            result = resolve_release_branch(
                args.repo_root,
                remote=args.remote,
                default_branch=args.default_branch,
                previous_version=args.previous_version,
                target_version=args.target_version,
                bump=args.bump,
                requested_date=args.requested_date,
            )
            _write_outputs(
                args.github_output,
                {
                    "exists": str(result.exists).lower(),
                    "branch": result.branch,
                    "base_sha": result.base_sha,
                    "branch_sha": result.branch_sha or "",
                    "replace_sha": result.replace_sha or "",
                    "release_date": result.release_date,
                },
            )
        elif args.command == "verify-default":
            require_remote_default(
                args.repo_root,
                remote=args.remote,
                default_branch=args.default_branch,
                expected_sha=args.expected_sha,
            )
        elif args.command == "push":
            push_release_branch(
                args.repo_root,
                remote=args.remote,
                default_branch=args.default_branch,
                expected_base_sha=args.expected_base_sha,
                branch=args.branch,
                expected_branch_sha=args.expected_branch_sha,
            )
        else:
            result = ensure_release_pr(
                repository=args.repository,
                base=args.base,
                head=args.head,
                head_sha=args.head_sha,
                title=args.title,
                body_file=args.body_file,
            )
            _write_outputs(
                args.github_output,
                {
                    "created": str(result.created).lower(),
                    "url": result.url,
                    "number": str(result.number or ""),
                },
            )
    except ReleaseBranchError as error:
        print(f"release-branch: error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
