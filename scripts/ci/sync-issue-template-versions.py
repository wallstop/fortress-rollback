#!/usr/bin/env python3
"""Sync fortress-rollback release versions into GitHub issue templates.

Fetches all releases from the GitHub API and updates the dropdown option list
between the BEGIN_FORTRESS_VERSIONS / END_FORTRESS_VERSIONS sentinel comments
in the bug report issue template.

Usage:
    python scripts/ci/sync-issue-template-versions.py
    python scripts/ci/sync-issue-template-versions.py --dry-run
    python scripts/ci/sync-issue-template-versions.py --check
"""
from __future__ import annotations

import argparse
import json
import os
import re
import sys
import urllib.error
import urllib.request
from pathlib import Path

TEMPLATE_PATH = ".github/ISSUE_TEMPLATE/bug_report.yml"
BEGIN_SENTINEL = "# BEGIN_FORTRESS_VERSIONS"
END_SENTINEL = "# END_FORTRESS_VERSIONS"
GITHUB_REPO = os.environ.get("GITHUB_REPOSITORY", "wallstop/fortress-rollback")
GITHUB_API = f"https://api.github.com/repos/{GITHUB_REPO}/releases"
REQUEST_TIMEOUT = 30


class NetworkError(RuntimeError):
    """Raised when a GitHub API network request fails.

    Treated as a non-fatal skip in --check mode so that offline pushes are
    not blocked by an inability to reach the GitHub API.
    """


def fetch_versions() -> list[str]:
    """Fetch all release tag names from GitHub API, newest first.

    Tags are returned exactly as provided by the GitHub API and may not
    conform to the expected ``vX.Y.Z`` format.  Callers should pass the
    result through :func:`validate_version_tags` before use.
    """
    headers = {
        "Accept": "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
    }
    token = os.environ.get("GITHUB_TOKEN")
    if token:
        headers["Authorization"] = f"Bearer {token}"

    versions: list[str] = []
    page = 1
    while True:
        url = f"{GITHUB_API}?per_page=100&page={page}"
        req = urllib.request.Request(url, headers=headers)
        try:
            with urllib.request.urlopen(req, timeout=REQUEST_TIMEOUT) as resp:
                data = json.loads(resp.read().decode())
        except urllib.error.HTTPError as exc:
            body = exc.read().decode(errors="replace")[:200]
            raise NetworkError(f"HTTP {exc.code} fetching releases from {url}: {body}")
        except urllib.error.URLError as exc:
            raise NetworkError(f"network error fetching releases from {url}: {exc}")

        if not data:
            break
        for release in data:
            if release.get("prerelease") or release.get("draft"):
                continue
            tag = release.get("tag_name", "")
            if tag:
                versions.append(tag)
        if len(data) < 100:
            break
        page += 1

    return versions


def validate_version_tags(versions: list[str]) -> list[str]:
    """Filter *versions* to those matching the ``vX.Y.Z`` semver pattern.

    Any tag that does not start with ``v`` followed by three dot-separated
    integers is logged as a warning to stderr and excluded from the returned
    list.  The sync can still proceed with the remaining valid tags.

    Tags with pre-release or build-metadata suffixes (e.g. ``v1.2.3-hotfix``)
    are accepted because the pattern is a prefix match; only the leading
    ``vMAJOR.MINOR.PATCH`` portion is required.
    """
    valid: list[str] = []
    pattern = re.compile(r"^v\d+\.\d+\.\d+")
    for tag in versions:
        if pattern.match(tag):
            valid.append(tag)
        else:
            print(
                f"warning: skipping tag {tag!r} — does not match vX.Y.Z format",
                file=sys.stderr,
            )
    return valid


def build_version_block(versions: list[str], indent: str) -> str:
    """Build the YAML list lines for versions between the sentinels."""
    lines = [f"{indent}{BEGIN_SENTINEL}"]
    for version in versions:
        lines.append(f"{indent}- {version}")
    lines.append(f"{indent}{END_SENTINEL}")
    return "\n".join(lines)


def update_template(content: str, versions: list[str]) -> tuple[str, bool]:
    """Replace the version list between sentinels. Returns (new_content, changed)."""
    lines = content.splitlines(keepends=True)

    begin_idx = None
    end_idx = None
    sentinel_indent = ""

    for i, line in enumerate(lines):
        stripped = line.strip()
        if stripped == BEGIN_SENTINEL:
            begin_idx = i
            sentinel_indent = line[: len(line) - len(line.lstrip())]
        elif stripped == END_SENTINEL:
            end_idx = i

    if begin_idx is None:
        raise RuntimeError(f"{TEMPLATE_PATH}:0: missing {BEGIN_SENTINEL} sentinel")
    if end_idx is None:
        raise RuntimeError(f"{TEMPLATE_PATH}:0: missing {END_SENTINEL} sentinel")
    if begin_idx >= end_idx:
        raise RuntimeError(
            f"{TEMPLATE_PATH}:0: {BEGIN_SENTINEL} must appear before {END_SENTINEL}"
        )

    new_block = build_version_block(versions, sentinel_indent) + "\n"
    # Replace lines from begin to end (inclusive)
    new_lines = lines[:begin_idx] + [new_block] + lines[end_idx + 1 :]
    new_content = "".join(new_lines)
    return new_content, new_content != content


def main() -> int:
    """Entry point."""
    parser = argparse.ArgumentParser(
        description="Sync release versions into GitHub issue template."
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print what would change without writing.",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Exit 1 if the template would change (for CI validation).",
    )
    args = parser.parse_args()

    template = Path(TEMPLATE_PATH)
    try:
        original = template.read_text(encoding="utf-8")
    except OSError as exc:
        print(f"{TEMPLATE_PATH}:0: cannot read file: {exc}", file=sys.stderr)
        return 1

    versions: list[str] = []
    try:
        versions = validate_version_tags(fetch_versions())
    except NetworkError as exc:
        if args.check:
            print(f"Skipping issue template version check: {exc}", file=sys.stderr)
            return 0
        print(f"{TEMPLATE_PATH}:0: {exc}", file=sys.stderr)
        return 1
    except RuntimeError as exc:
        print(f"{TEMPLATE_PATH}:0: {exc}", file=sys.stderr)
        return 1
    if not versions:
        print(f"{TEMPLATE_PATH}:0: no releases found", file=sys.stderr)
        return 1

    try:
        updated, changed = update_template(original, versions)
    except RuntimeError as exc:
        print(str(exc), file=sys.stderr)
        return 1

    if not changed:
        print("Template is already up to date.")
        return 0

    if args.dry_run:
        print("Would update version list:")
        for version in versions:
            print(f"- {version}")
        return 0

    if args.check:
        print("Template is out of date (run script to update).", file=sys.stderr)
        return 1

    try:
        template.write_text(updated, encoding="utf-8")
    except OSError as exc:
        print(f"{TEMPLATE_PATH}:0: cannot write file: {exc}", file=sys.stderr)
        return 1

    print(f"Updated {TEMPLATE_PATH} with {len(versions)} version(s).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
