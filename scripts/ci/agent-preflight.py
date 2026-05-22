#!/usr/bin/env python3
"""Changed-file-aware preflight checks for agentic workflows.

This script runs a small, high-signal set of validations before commit-time
hooks. It is intended for agent workflows that should catch issues early,
before hitting pre-commit failures.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

PYTHON_EXECUTABLE = sys.executable or "python3"

SYNC_VERSION_EXTENSIONS = {
    ".rs",
    ".md",
    ".toml",
    ".yml",
    ".yaml",
    ".sh",
    ".txt",
    ".json",
}


@dataclass(frozen=True)
class PlannedCheck:
    """A check planned for execution."""

    check_id: str
    description: str
    command: list[str]
    fix_hint: str | None = None
    fix_command: list[str] | None = None


def repo_root_from_script() -> Path:
    """Return the repository root based on this script location."""
    return Path(__file__).resolve().parents[2]


def normalize_paths(paths: set[str] | list[str]) -> set[str]:
    """Normalize file paths to repository-relative POSIX style."""
    normalized: set[str] = set()
    for raw_path in paths:
        if not raw_path:
            continue
        path = Path(raw_path)
        normalized_path = path.as_posix()
        if normalized_path.startswith("./"):
            normalized_path = normalized_path[2:]
        normalized.add(normalized_path)
    return normalized


def is_sync_version_surface_file(path: str) -> bool:
    """Return True if this file can trigger sync-version checks."""
    return Path(path).suffix in SYNC_VERSION_EXTENSIONS


def is_llm_markdown_file(path: str) -> bool:
    """Return True for markdown files under .llm/."""
    return path.startswith(".llm/") and path.endswith(".md")


def is_llm_skill_markdown_file(path: str) -> bool:
    """Return True for markdown files under .llm/skills/."""
    return path.startswith(".llm/skills/") and path.endswith(".md")


def is_workflow_file(path: str) -> bool:
    """Return True for GitHub Actions workflow files."""
    if not path.startswith(".github/workflows/"):
        return False
    return Path(path).suffix in {".yml", ".yaml"}


def is_docs_markdown_file(path: str) -> bool:
    """Return True for markdown files under docs/ (vale targets)."""
    return path.startswith("docs/") and path.endswith(".md")


def is_changelog_file(path: str) -> bool:
    """Return True for the top-level CHANGELOG.md file."""
    return path == "CHANGELOG.md"


def is_rust_source_file(path: str) -> bool:
    """Return True for Rust source files under `src/`."""
    return path.startswith("src/") and path.endswith(".rs")


def is_rust_file(path: str) -> bool:
    """Return True for any Rust file."""
    return path.endswith(".rs")


def git_output_lines(repo_root: Path, args: list[str]) -> set[str]:
    """Run git and return non-empty output lines.

    Raises RuntimeError if git exits non-zero.
    """
    result = subprocess.run(
        ["git", "-C", str(repo_root), *args],
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        command = "git " + " ".join(args)
        stderr = (result.stderr or "").strip()
        raise RuntimeError(
            f"{command} failed with exit code {result.returncode}: {stderr}"
        )

    return {line.strip() for line in result.stdout.splitlines() if line.strip()}


def collect_changed_files(repo_root: Path) -> set[str]:
    """Collect staged, unstaged, and untracked file paths."""
    changed: set[str] = set()
    changed |= git_output_lines(repo_root, ["diff", "--name-only"])
    changed |= git_output_lines(repo_root, ["diff", "--cached", "--name-only"])
    changed |= git_output_lines(
        repo_root,
        ["ls-files", "--others", "--exclude-standard"],
    )
    return normalize_paths(changed)


def plan_checks(changed_files: set[str], run_all: bool = False) -> list[PlannedCheck]:
    """Select checks based on changed files."""
    checks: list[PlannedCheck] = []

    llm_files = sorted(path for path in changed_files if is_llm_markdown_file(path))
    llm_skill_files = sorted(
        path for path in changed_files if is_llm_skill_markdown_file(path)
    )
    workflow_files = sorted(path for path in changed_files if is_workflow_file(path))
    docs_files = sorted(path for path in changed_files if is_docs_markdown_file(path))
    rust_files = sorted(path for path in changed_files if is_rust_file(path))
    rust_source_files = sorted(
        path for path in changed_files if is_rust_source_file(path)
    )
    changelog_changed = any(is_changelog_file(path) for path in changed_files)

    if run_all or any(is_sync_version_surface_file(path) for path in changed_files):
        checks.append(
            PlannedCheck(
                check_id="sync-version-check",
                description="validate Cargo/changelog/version reference synchronization",
                command=["bash", "scripts/sync-version.sh", "--check"],
                fix_hint="Run 'bash scripts/sync-version.sh' to synchronize references.",
                fix_command=["bash", "scripts/sync-version.sh"],
            )
        )

    if run_all or llm_files:
        checks.append(
            PlannedCheck(
                check_id="llm-line-limit",
                description="validate .llm markdown line limits",
                command=[PYTHON_EXECUTABLE, "scripts/hooks/check-llm-line-limit.py"],
                fix_hint="Split large .llm files so each stays within the line limit.",
            )
        )
        checks.append(
            PlannedCheck(
                check_id="llm-skills-quality",
                description="validate .llm skills quality constraints",
                command=["bash", "scripts/docs/check-llm-skills.sh"],
                fix_hint="Fix reported .llm formatting or code-sample issues.",
            )
        )

    if run_all or llm_skill_files:
        checks.append(
            PlannedCheck(
                check_id="skills-index-check",
                description="ensure .llm skills index is synchronized",
                command=[
                    PYTHON_EXECUTABLE,
                    "scripts/hooks/regenerate-skills-index.py",
                    "--check",
                ],
                fix_hint="Run 'python scripts/hooks/regenerate-skills-index.py' and commit the regenerated index.",
                fix_command=[PYTHON_EXECUTABLE, "scripts/hooks/regenerate-skills-index.py"],
            )
        )

    if run_all or workflow_files:
        actionlint_command = [PYTHON_EXECUTABLE, "scripts/hooks/actionlint.py"]
        actionlint_command.extend(workflow_files)
        checks.append(
            PlannedCheck(
                check_id="actionlint",
                description="validate modified GitHub Actions workflows",
                command=actionlint_command,
                fix_hint="Fix workflow syntax and actionlint diagnostics.",
            )
        )

    if run_all or changelog_changed:
        checks.append(
            PlannedCheck(
                check_id="changelog-unreleased-rule",
                description=(
                    "enforce the [Unreleased] code rule in CHANGELOG.md "
                    "(see .llm/context.md \"Unreleased code rule\")"
                ),
                command=[
                    PYTHON_EXECUTABLE,
                    "scripts/hooks/check-changelog-unreleased.py",
                ],
                fix_hint=(
                    "non-Breaking entries in '### Changed' or any entries in "
                    "'### Fixed' under [Unreleased] must be folded into the "
                    "matching '### Added' entry. Only '**Breaking:**' entries "
                    "(for already-released types) belong in '### Changed'. "
                    "See .llm/skills/publishing-organization/changelog.md."
                ),
                # Semantic merge -- no safe auto-fix.
                fix_command=None,
            )
        )

    if run_all or docs_files:
        vale_command = [PYTHON_EXECUTABLE, "scripts/hooks/agent-vale-advisory.py"]
        vale_command.extend(docs_files)
        checks.append(
            PlannedCheck(
                check_id="vale-advisory",
                description=(
                    "advisory prose linting for docs/ (always passes; "
                    "see .llm/skills/workflows/user-facing-docs.md "
                    "'Prose Conventions')"
                ),
                command=vale_command,
                fix_hint=(
                    "Vale findings are advisory. Common swaps: "
                    "implement->do, multiple->many, previously->before, "
                    "subsequent->next, additional->extra, maximum->most, "
                    "monitor->watch, terminate->end. Drop weasel words "
                    "(very, extremely, several, usually, significantly)."
                ),
            )
        )

    if run_all or rust_files:
        command = [
            PYTHON_EXECUTABLE,
            "scripts/hooks/check-advance-frame-error-handling.py",
        ]
        command.extend(rust_files)
        checks.append(
            PlannedCheck(
                check_id="advance-frame-error-handling",
                description=(
                    "reject advance_frame() calls whose errors are swallowed by "
                    "if let Ok(..)"
                ),
                command=command,
                fix_hint=(
                    "Use ? when any error should fail, or match only the exact "
                    "expected error such as PredictionThreshold."
                ),
            )
        )

    if run_all or rust_source_files:
        checks.append(
            PlannedCheck(
                check_id="kani-violation-cost",
                description=(
                    "advisory grep for multi-arg report_violation! callsites "
                    "in Kani-reachable files (always passes; see "
                    "src/telemetry.rs report_violation! docs)"
                ),
                command=["bash", "scripts/verification/check-kani-violation-cost.sh"],
                fix_hint=(
                    "report_violation! is a no-op under cfg(kani) but format "
                    "args still surface in CBMC analysis. Reduce format-arg "
                    "count or simplify the message in flagged callsites."
                ),
            )
        )

    return checks


def run_check_command(repo_root: Path, command: list[str]) -> int:
    """Run a check command and return its exit code."""
    return subprocess.run(command, cwd=repo_root, check=False).returncode


def execute_checks(
    repo_root: Path,
    checks: list[PlannedCheck],
    auto_fix: bool,
) -> int:
    """Execute planned checks.

    Returns 0 on success and 1 when any check fails.
    """
    failures: list[PlannedCheck] = []

    for check in checks:
        print(f"[RUN ] {check.check_id}: {check.description}")
        return_code = run_check_command(repo_root, check.command)

        if return_code == 0:
            print(f"[PASS] {check.check_id}")
            continue

        print(f"[FAIL] {check.check_id}", file=sys.stderr)

        if auto_fix and check.fix_command is not None:
            print(
                f"[FIX ] Attempting auto-fix for {check.check_id}...",
                file=sys.stderr,
            )
            fix_code = run_check_command(repo_root, check.fix_command)
            if fix_code == 0:
                retry_code = run_check_command(repo_root, check.command)
                if retry_code == 0:
                    print(f"[PASS] {check.check_id} (auto-fixed)")
                    continue

            print(
                f"[FAIL] {check.check_id} auto-fix did not resolve the issue.",
                file=sys.stderr,
            )

        failures.append(check)

    if not failures:
        print("All agent preflight checks passed.")
        return 0

    print("Agent preflight checks failed:", file=sys.stderr)
    for check in failures:
        print(f"- {check.check_id}: {check.description}", file=sys.stderr)
        if check.fix_hint:
            label = "auto-fix" if check.fix_command is not None else "manual-fix"
            print(f"{label}: {check.fix_hint}", file=sys.stderr)
    return 1


def build_parser() -> argparse.ArgumentParser:
    """Build argument parser."""
    parser = argparse.ArgumentParser(
        description=(
            "Run changed-file-aware preflight checks for agentic workflows."
        )
    )
    parser.add_argument(
        "--all",
        action="store_true",
        help="Run all checks regardless of changed files.",
    )
    parser.add_argument(
        "--auto-fix",
        action="store_true",
        help="Attempt configured auto-fixes for failing checks, then re-run them.",
    )
    parser.add_argument(
        "--files",
        nargs="*",
        help="Optional explicit file list (repository-relative) used for check selection.",
    )
    parser.add_argument(
        "--verbose",
        action="store_true",
        help="Print changed files and selected checks.",
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    """Entry point."""
    parser = build_parser()
    args = parser.parse_args(argv)

    repo_root = repo_root_from_script()

    if args.files:
        changed_files = normalize_paths(args.files)
    elif args.all:
        changed_files = set()
    else:
        try:
            changed_files = collect_changed_files(repo_root)
        except RuntimeError as exc:
            print(str(exc), file=sys.stderr)
            print("Falling back to --all checks.", file=sys.stderr)
            changed_files = set()
            args.all = True

    if not args.all and not changed_files:
        print("No changed files detected; skipping preflight checks.")
        return 0

    planned_checks = plan_checks(changed_files, run_all=args.all)

    if args.verbose:
        if changed_files:
            print("Changed files:")
            for path in sorted(changed_files):
                print(f"  - {path}")
        else:
            print("Changed files: (none provided)")
        print("Planned checks:")
        for check in planned_checks:
            print(f"  - {check.check_id}")

    if not planned_checks:
        print("No relevant preflight checks for the current file set.")
        return 0

    return execute_checks(repo_root, planned_checks, auto_fix=args.auto_fix)


if __name__ == "__main__":
    sys.exit(main())
