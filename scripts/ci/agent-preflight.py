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

# Text source extensions that `typos` (the CI spell-check gate) scans. Used only
# to decide WHETHER to run the whole-repo spell check, not to limit its scope --
# a change to any of these triggers the same repo-wide scan CI performs.
SPELLCHECK_EXTENSIONS = {
    ".rs",
    ".md",
    ".toml",
    ".yml",
    ".yaml",
    ".sh",
    ".py",
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


def is_agent_skill_markdown_file(path: str) -> bool:
    """Return True for Markdown files in the project Agent Skills tree."""
    return path.startswith(".agents/skills/") and path.endswith(".md")


def is_workflow_file(path: str) -> bool:
    """Return True for GitHub Actions workflow files."""
    if not path.startswith(".github/workflows/"):
        return False
    return Path(path).suffix in {".yml", ".yaml"}


def is_workspace_lock_surface_file(path: str) -> bool:
    """Return True for Cargo topology, locks, and canonical release tooling."""
    return Path(path).name in {"Cargo.toml", "Cargo.lock"} or path.startswith(
        "scripts/release/"
    )


def is_release_automation_surface_file(path: str) -> bool:
    """Return True for reviewed-release state-machine inputs and contracts."""
    return (
        path.startswith("scripts/release/")
        or path
        in {
            ".github/ISSUE_TEMPLATE/bug_report.yml",
            ".github/workflows/release-prepare.yml",
            ".github/workflows/ci-release-state.yml",
            ".github/workflows/publish.yml",
            ".agents/skills/changelog/SKILL.md",
            ".agents/skills/fortress-development/SKILL.md",
            ".agents/skills/publishing/SKILL.md",
            ".agents/skills/design-decisions/references/release.txt",
            "CHANGELOG.md",
            "scripts/tests/test_prepare_release.py",
            "scripts/tests/test_release_checkpoint.py",
            "scripts/tests/test_publish_state.py",
            "scripts/tests/test_release_branch.py",
            "scripts/tests/test_release_policy.py",
            "scripts/tests/test_release_state.py",
            "scripts/tests/test_release_state_ci.py",
            "scripts/tests/test_release_workflows.py",
            "scripts/ci/sync-issue-template-versions.py",
            "scripts/tests/test_issue_template_versions_wiring.py",
            "scripts/tests/test_sync_issue_template_versions.py",
        }
    )


def is_ci_toolchain_surface_file(path: str) -> bool:
    """Return True for canonical nightly pin consumers and their contract."""
    return path.startswith(
        (
            ".github/actions/install-pinned-nightly/",
            ".github/actions/install-pinned-release/",
        )
    ) or path in {
        ".agents/skills/github-actions/SKILL.md",
        ".github/workflows/ci-docs.yml",
        ".github/workflows/ci-rust.yml",
        ".github/workflows/ci-safety.yml",
        ".github/workflows/ci-security.yml",
        ".github/workflows/ci-verification.yml",
        "scripts/ci/check-tools.sh",
        "scripts/tests/test_ci_toolchains.py",
    }


def is_docs_markdown_file(path: str) -> bool:
    """Return True for markdown files under docs/ (vale targets)."""
    return path.startswith("docs/") and path.endswith(".md")


def is_changelog_file(path: str) -> bool:
    """Return True for the top-level CHANGELOG.md file."""
    return path == "CHANGELOG.md"


def is_wire_golden_surface_file(path: str) -> bool:
    """Return True for immutable wire fixtures and their enforcing hook."""
    return path == "scripts/hooks/check-wire-golden-immutable.py" or (
        path.startswith(("src/network/wire_golden_", "tests/network/wire_golden_"))
        and path.endswith(".rs")
    )


def is_tla_consistency_surface_file(path: str) -> bool:
    """Return True for files that can affect the TLA FIX_MODE consistency check.

    The check discovers every FIX_MODE spec in ``specs/tla/`` and compares each
    against its own ``.cfg`` files and the shared ``README.md``, so any ``.tla``,
    ``.cfg``, the README (or the checker itself) can change its result.
    """
    if path == "scripts/docs/check-tla-config-consistency.py":
        return True
    if not path.startswith("specs/tla/"):
        return False
    return path.endswith((".tla", ".cfg")) or path == "specs/tla/README.md"


def is_network_timing_surface_file(path: str) -> bool:
    """Return True for files that affect network timing invariant checks."""
    return path in {
        ".config/nextest.toml",
        ".github/workflows/ci-network-nightly.yml",
        "scripts/hooks/check-network-timing-invariants.py",
        "src/network/protocol/mod.rs",
        "tests/network/multi_process.rs",
    }


def is_rust_source_file(path: str) -> bool:
    """Return True for Rust source files under `src/`."""
    return path.startswith("src/") and path.endswith(".rs")


def is_rust_file(path: str) -> bool:
    """Return True for any Rust file."""
    return path.endswith(".rs")


def is_markdown_file(path: str) -> bool:
    """Return True for any Markdown file."""
    return path.endswith(".md")


def is_link_check_surface_file(path: str) -> bool:
    """Return True for files whose links check-links.py validates (.md or .rs).

    Mirrors the ``check-links`` pre-commit hook's ``\\.(md|rs)$`` file filter so
    agent preflight runs the same validation when those files change.
    """
    return is_markdown_file(path) or is_rust_file(path)


def is_doc_claims_surface_file(path: str) -> bool:
    """Return True for files that can affect check-doc-claims output."""
    return is_rust_file(path) or path in {
        "scripts/ci/check-doc-claims.sh",
        "scripts/hooks/check-rust-semantic-claims.py",
    }


def is_python_file(path: str) -> bool:
    """Return True for any Python file (tomllib-fallback surface)."""
    return path.endswith(".py")


def is_spellcheck_surface_file(path: str) -> bool:
    """Return True for text files whose change should trigger the spell check."""
    return Path(path).suffix in SPELLCHECK_EXTENSIONS


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


def plan_checks(
    changed_files: set[str],
    run_all: bool = False,
    existing_files: set[str] | None = None,
) -> list[PlannedCheck]:
    """Select checks, passing only extant changed files to path-based tools."""
    checks: list[PlannedCheck] = []
    extant_files = changed_files if existing_files is None else existing_files

    agent_skill_changed = any(
        is_agent_skill_markdown_file(path) for path in changed_files
    )
    workflow_changed = any(is_workflow_file(path) for path in changed_files)
    workspace_lock_changed = any(
        is_workspace_lock_surface_file(path) for path in changed_files
    )
    release_automation_changed = any(
        is_release_automation_surface_file(path) for path in changed_files
    )
    ci_toolchain_changed = any(
        is_ci_toolchain_surface_file(path) for path in changed_files
    )
    docs_files = sorted(path for path in extant_files if is_docs_markdown_file(path))
    rust_changed = any(is_rust_file(path) for path in changed_files)
    rust_files = sorted(path for path in extant_files if is_rust_file(path))
    rust_source_changed = any(
        is_rust_source_file(path) for path in changed_files
    )
    rust_source_files = sorted(
        path for path in extant_files if is_rust_source_file(path)
    )
    link_check_changed = any(
        is_link_check_surface_file(path) for path in changed_files
    )
    doc_claims_changed = any(
        is_doc_claims_surface_file(path) for path in changed_files
    )
    tla_consistency_changed = any(
        is_tla_consistency_surface_file(path) for path in changed_files
    )
    network_timing_changed = any(
        is_network_timing_surface_file(path) for path in changed_files
    )
    changelog_changed = any(is_changelog_file(path) for path in changed_files)
    python_files = sorted(path for path in extant_files if is_python_file(path))
    spellcheck_changed = any(
        is_spellcheck_surface_file(path) for path in changed_files
    )

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

    if run_all or workspace_lock_changed:
        checks.append(
            PlannedCheck(
                check_id="workspace-lock-check",
                description="validate every Cargo workspace lock with full dependency metadata",
                command=[
                    PYTHON_EXECUTABLE,
                    "scripts/release/workspace_locks.py",
                    "check",
                ],
                fix_hint=(
                    "Run 'python3 scripts/release/workspace_locks.py sync', then "
                    "rerun the full check; never bypass --locked or use --no-deps "
                    "as a lock-freshness oracle."
                ),
            )
        )

    if run_all or release_automation_changed:
        checks.append(
            PlannedCheck(
                check_id="release-automation-tests",
                description="exercise release policy, prepared state, and publish retries",
                command=[
                    PYTHON_EXECUTABLE,
                    "-m",
                    "pytest",
                    "scripts/tests/test_workspace_locks.py",
                    "scripts/tests/test_prepare_release.py",
                    "scripts/tests/test_release_policy.py",
                    "scripts/tests/test_release_state.py",
                    "scripts/tests/test_release_state_ci.py",
                    "scripts/tests/test_release_checkpoint.py",
                    "scripts/tests/test_publish_state.py",
                    "scripts/tests/test_release_branch.py",
                    "scripts/tests/test_sync_issue_template_versions.py",
                    "scripts/tests/test_issue_template_versions_wiring.py",
                    "scripts/tests/test_release_workflows.py",
                    "--no-header",
                    "-q",
                ],
                fix_hint=(
                    "Fix the release state-machine regression; do not bypass a "
                    "semantic-version, digest, lock, or checksum invariant."
                ),
            )
        )

    if run_all or ci_toolchain_changed:
        checks.append(
            PlannedCheck(
                check_id="ci-toolchain-contract",
                description="enforce dated Rust toolchains and canonical retry installation",
                command=[
                    PYTHON_EXECUTABLE,
                    "-m",
                    "pytest",
                    "scripts/tests/test_ci_toolchains.py",
                    "--no-header",
                    "-q",
                ],
                fix_hint=(
                    "Use the canonical pinned-nightly installer; required workflows "
                    "must not use floating nightly channels or duplicate the pin."
                ),
            )
        )

    if run_all or agent_skill_changed:
        checks.append(
            PlannedCheck(
                check_id="validate-agent-skills",
                description="validate the Agent Skills open-format contract",
                command=[PYTHON_EXECUTABLE, "scripts/hooks/validate-agent-skills.py"],
                fix_hint="Fix the reported SKILL.md metadata or layout violation.",
            )
        )
        checks.append(
            PlannedCheck(
                check_id="agent-skills-quality",
                description="validate Agent Skills code-example quality constraints",
                command=["bash", "scripts/docs/check-agent-skills.sh"],
                fix_hint="Fix the reported Agent Skills code-sample issue.",
            )
        )

    if run_all or workflow_changed:
        checks.append(
            PlannedCheck(
                check_id="actionlint",
                description="validate GitHub Actions workflows",
                command=[PYTHON_EXECUTABLE, "scripts/hooks/actionlint.py"],
                fix_hint="Fix workflow syntax and actionlint diagnostics.",
            )
        )

    if run_all or changelog_changed:
        checks.append(
            PlannedCheck(
                check_id="changelog-unreleased-rule",
                description=(
                    "enforce the [Unreleased] code rule in CHANGELOG.md "
                    "(see .agents/skills/fortress-development/SKILL.md \"Unreleased code rule\")"
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
                    "See .agents/skills/changelog/SKILL.md."
                ),
                # Semantic merge -- no safe auto-fix.
                fix_command=None,
            )
        )

    if run_all or any(is_wire_golden_surface_file(path) for path in changed_files):
        checks.append(
            PlannedCheck(
                check_id="wire-golden-immutable",
                description="prevent released wire fixture changes without a protocol bump",
                command=[
                    PYTHON_EXECUTABLE,
                    "scripts/hooks/check-wire-golden-immutable.py",
                    "--local",
                ],
                fix_hint=(
                    "Restore released wire fixtures, or bump PROTOCOL_VERSION and add the next "
                    "versioned golden suite as the active registration."
                ),
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
                    "see .agents/skills/user-facing-docs/SKILL.md "
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

    if run_all or link_check_changed:
        checks.append(
            PlannedCheck(
                check_id="link-check",
                description=(
                    "validate local file/anchor links and rustdoc intra-doc "
                    "links, including text-vs-target mismatches where backticked "
                    "link text names an item the crate-internal path does not "
                    "(see .agents/skills/doc-code-sync/SKILL.md)"
                ),
                # check-links.py scans the whole tree (no per-file args), matching
                # the `pass_filenames: false` `check-links` pre-commit hook.
                command=[PYTHON_EXECUTABLE, "scripts/docs/check-links.py"],
                fix_hint=(
                    "Fix the reported broken link or anchor. For a flagged "
                    "intra-doc text/target mismatch, link to the named item if it "
                    "is reachable without tripping "
                    "rustdoc::private_intra_doc_links; otherwise use a plain code "
                    "span (no link) or link the module with module-named text."
                ),
                # Detection + reporting only -- the correct fix depends on item
                # visibility, so there is no safe blind auto-fix.
                fix_command=None,
            )
        )

    if run_all or doc_claims_changed:
        checks.append(
            PlannedCheck(
                check_id="doc-claims",
                description=(
                    "validate Rust doc comments and test names against "
                    "implementation clues"
                ),
                command=["bash", "scripts/ci/check-doc-claims.sh"],
                fix_hint=(
                    "Update stale rustdoc/test names, or adjust the implementation "
                    "if the documented contract is the intended behavior."
                ),
            )
        )

    if run_all or tla_consistency_changed:
        checks.append(
            PlannedCheck(
                check_id="tla-config-consistency",
                description=(
                    "validate the TLA FIX_MODE set, its .cfg files, and the "
                    "specs/tla/README.md prose stay in sync"
                ),
                command=[
                    PYTHON_EXECUTABLE,
                    "scripts/docs/check-tla-config-consistency.py",
                ],
                fix_hint=(
                    "Align the count/mode names in specs/tla/README.md with the "
                    "spec's `ASSUME FIX_MODE \\in {...}` set, add a .cfg that "
                    "exercises any new mode, and document it."
                ),
                # The right fix depends on intent (spec vs prose vs config), so
                # there is no safe blind auto-fix.
                fix_command=None,
            )
        )

    if run_all or network_timing_changed:
        checks.append(
            PlannedCheck(
                check_id="network-timing-invariants",
                description=(
                    "validate multi-process nextest budgets, peer wait helpers, "
                    "and protocol virtual-time test usage"
                ),
                command=[
                    PYTHON_EXECUTABLE,
                    "scripts/hooks/check-network-timing-invariants.py",
                ],
                fix_hint=(
                    "Keep nextest network budgets above the macOS-scaled harness "
                    "ceilings, route direct peer waits through scenario-derived "
                    "timeouts, and use ProtocolConfig.clock for protocol timer tests."
                ),
            )
        )

    if run_all or rust_changed:
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

    if run_all or rust_source_changed:
        unbounded_alloc_command = [
            PYTHON_EXECUTABLE,
            "scripts/hooks/check-unbounded-alloc.py",
        ]
        unbounded_alloc_command.extend(rust_source_files)
        checks.append(
            PlannedCheck(
                check_id="unbounded-alloc",
                description=(
                    "require an '// alloc-bound:' justification on "
                    "dynamically-sized allocations and a '// reserve-in-loop:' "
                    "justification on per-iteration fallible reserves in src/ "
                    "(see .agents/skills/defensive-programming/SKILL.md)"
                ),
                command=unbounded_alloc_command,
                fix_hint=(
                    "Add an '// alloc-bound: <why>' comment (same line or the "
                    "line above) stating why the size is bounded, or bound the "
                    "size if it is genuinely unbounded (e.g. a length read from "
                    "the wire or an unvalidated config field). For a "
                    "try_reserve inside a loop, prefer a single bulk "
                    "pre-reservation before the loop (e.g. error::try_reserve_hint "
                    "from a size_hint), or mark the deliberate per-iteration "
                    "reserve with '// reserve-in-loop: <why>'."
                ),
            )
        )
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

    if run_all or python_files:
        tomllib_command = [
            PYTHON_EXECUTABLE,
            "scripts/hooks/check-tomllib-fallback.py",
        ]
        tomllib_command.extend(python_files)
        checks.append(
            PlannedCheck(
                check_id="tomllib-fallback",
                description=(
                    "require a tomli fallback wherever tomllib is imported "
                    "(tomllib is stdlib only on Python 3.11+)"
                ),
                command=tomllib_command,
                fix_hint=(
                    "Wrap `import tomllib` with a tomli fallback (mirrors "
                    "scripts/hooks/check-toml.py): try: import tomllib / "
                    "except ImportError: import tomli as tomllib."
                ),
            )
        )

    if run_all or spellcheck_changed:
        checks.append(
            PlannedCheck(
                check_id="typos",
                description=(
                    "spell check the repo via `typos` (mirrors the CI "
                    "spell-check gate; uses .typos.toml)"
                ),
                command=[PYTHON_EXECUTABLE, "scripts/hooks/agent-typos.py"],
                fix_hint=(
                    "Fix the flagged spelling, or add a legitimate project term "
                    "to .typos.toml [default.extend-words]. No auto-fix: typos' "
                    "suggestions are not always the intended word, so review "
                    "each before applying it."
                ),
                # No fix_command: `typos --write-changes` can pick the wrong
                # correction for hyphenation/compound cases.
                fix_command=None,
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

    existing_files = {
        path for path in changed_files if (repo_root / path).is_file()
    }
    planned_checks = plan_checks(
        changed_files,
        run_all=args.all,
        existing_files=existing_files,
    )

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
