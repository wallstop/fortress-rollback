#!/usr/bin/env python3
"""Check Dockerfiles and devcontainer.json for common anti-patterns.

Detects issues such as:
- pip install without --no-cache-dir (leaves cache in image)
- command -v output redirected to stderr instead of /dev/null

Cross-platform: Works on Linux, macOS, and Windows.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path


def check_file(filepath: Path) -> list[str]:
    """Check a single file for Dockerfile anti-patterns.

    Returns a list of issue descriptions (empty if no issues).
    """
    issues: list[str] = []

    try:
        lines = filepath.read_text(encoding="utf-8").splitlines()
    except (OSError, UnicodeDecodeError) as exc:
        print(f"Warning: skipping {filepath}: {exc}", file=sys.stderr)
        return []

    is_dockerfile = filepath.name.startswith("Dockerfile")

    for line_num, line in enumerate(lines, start=1):
        stripped = line.strip()

        # Skip comment lines in Dockerfiles
        if is_dockerfile and stripped.startswith("#"):
            continue

        # Check 1: pip install without --no-cache-dir
        if re.search(r"\bpip3?\s+install\b", stripped) and "--no-cache-dir" not in stripped:
            issues.append(
                f"{filepath}:{line_num}: pip install without --no-cache-dir "
                f"(leaves pip cache in the image)"
            )

        # Check 2: command -v with stderr redirect instead of /dev/null
        if re.search(r"\bcommand\s+-v\b", stripped) and re.search(r">&2", stripped):
            issues.append(
                f"{filepath}:{line_num}: command -v output redirected to stderr "
                f"instead of /dev/null (use >/dev/null 2>&1)"
            )

    return issues


def main() -> int:
    """Check Dockerfiles and devcontainer.json for anti-patterns."""
    files = sys.argv[1:] if len(sys.argv) > 1 else []

    if not files:
        # No files provided, scan for all Dockerfiles and devcontainer.json
        repo_root = Path(__file__).parent.parent.parent
        for path in repo_root.rglob("Dockerfile*"):
            files.append(str(path))
        for path in repo_root.rglob("devcontainer.json"):
            files.append(str(path))

    if not files:
        return 0

    all_issues: list[str] = []

    for arg in files:
        filepath = Path(arg)

        # Only check Dockerfiles and devcontainer.json
        if not (
            filepath.name.startswith("Dockerfile")
            or filepath.name == "devcontainer.json"
        ):
            continue

        issues = check_file(filepath)
        all_issues.extend(issues)

    if all_issues:
        print("Dockerfile anti-patterns detected:")
        for issue in all_issues:
            print(f"  {issue}")
        print(f"\n{len(all_issues)} issue(s) found.")
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
