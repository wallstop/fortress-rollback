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
import sys
import urllib.request
from pathlib import Path

TEMPLATE_PATH = ".github/ISSUE_TEMPLATE/bug_report.yml"
BEGIN_SENTINEL = "# BEGIN_FORTRESS_VERSIONS"
END_SENTINEL = "# END_FORTRESS_VERSIONS"
GITHUB_API = "https://api.github.com/repos/wallstop/fortress-rollback/releases"


def fetch_versions() -> list[str]:
    """Fetch all release tag names from GitHub API, newest first."""
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
            with urllib.request.urlopen(req) as resp:
                data = json.loads(resp.read().decode())
        except urllib.error.HTTPError as exc:
            body = exc.read().decode(errors="replace")[:200]
            print(
                f"error: HTTP {exc.code} fetching releases from {url}: {body}",
                file=sys.stderr,
            )
            sys.exit(1)
        except urllib.error.URLError as exc:
            print(f"error: network error fetching releases from {url}: {exc}", file=sys.stderr)
            sys.exit(1)

        if not data:
            break
        for release in data:
            tag = release.get("tag_name", "")
            if tag:
                versions.append(tag)
        if len(data) < 100:
            break
        page += 1

    return versions


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
            break

    if begin_idx is None:
        print(f"{TEMPLATE_PATH}:0: missing {BEGIN_SENTINEL} sentinel", file=sys.stderr)
        sys.exit(1)
    if end_idx is None:
        print(f"{TEMPLATE_PATH}:0: missing {END_SENTINEL} sentinel", file=sys.stderr)
        sys.exit(1)

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

    versions = fetch_versions()
    if not versions:
        print(f"{TEMPLATE_PATH}:0: no releases found", file=sys.stderr)
        return 1

    updated, changed = update_template(original, versions)

    if not changed:
        print("Template is already up to date.")
        return 0

    if args.dry_run:
        print("Would update version list:")
        for version in versions:
            print(f"  - {version}")
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
