#!/usr/bin/env python3
"""Tests for immutable reviewed-release state generation and verification."""

from __future__ import annotations

import importlib.util
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

import pytest


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "scripts" / "release" / "release_state.py"
SPEC = importlib.util.spec_from_file_location("release_state", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
release_state = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = release_state
SPEC.loader.exec_module(release_state)


def _fixture(tmp_path: Path) -> Path:
    repo = tmp_path / "repo"
    (repo / ".github" / "ISSUE_TEMPLATE").mkdir(parents=True)
    (repo / "src").mkdir()
    (repo / "Cargo.toml").write_text(
        '[package]\nname = "fortress-rollback"\nversion = "1.3.0"\n',
        encoding="utf-8",
    )
    (repo / "CHANGELOG.md").write_text(
        """# Changelog

## [Unreleased]

## [1.3.0] - 2026-07-18

### Added

- Reliable releases.

## [1.2.3] - 2026-01-01

- Previous release.
""",
        encoding="utf-8",
    )
    (repo / ".github" / "ISSUE_TEMPLATE" / "bug_report.yml").write_text(
        """body:
  - type: dropdown
    attributes:
      options:
        # BEGIN_FORTRESS_VERSIONS
        - v1.3.0
        - v1.2.3
        # END_FORTRESS_VERSIONS
""",
        encoding="utf-8",
    )
    (repo / "src" / "lib.rs").write_text("pub fn fixture() {}\n", encoding="utf-8")
    executable = repo / "release-check.sh"
    executable.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
    executable.chmod(0o755)
    (repo / "source-link").symlink_to("src/lib.rs")
    subprocess.run(["git", "init", "--quiet"], cwd=repo, check=True)
    subprocess.run(["git", "add", "--all"], cwd=repo, check=True)
    return repo


def _generate(repo: Path) -> object:
    state = release_state.generate(
        repo,
        previous_version="1.2.3",
        target_version="1.3.0",
        bump="minor",
        release_date="2026-07-18",
    )
    subprocess.run(["git", "add", release_state.MANIFEST_NAME], cwd=repo, check=True)
    return state


def _trusted_base_fixture(tmp_path: Path) -> tuple[Path, Path]:
    base = tmp_path / "trusted-base"
    candidate = tmp_path / "candidate"
    (base / ".github" / "ISSUE_TEMPLATE").mkdir(parents=True)
    (base / "scripts" / "ci").mkdir(parents=True)
    (base / "scripts" / "release").mkdir(parents=True)
    (base / "src").mkdir()
    (base / "Cargo.toml").write_text(
        '[package]\nname = "fortress-rollback"\nversion = "1.2.3"\nedition = "2021"\n',
        encoding="utf-8",
    )
    (base / "src" / "lib.rs").write_text("pub fn fixture() {}\n", encoding="utf-8")
    (base / "README.md").write_text(
        'fortress-rollback = "1.2"\n', encoding="utf-8"
    )
    (base / "CHANGELOG.md").write_text(
        """# Changelog

## [Unreleased]

### Added

- Reliable releases.

## [1.2.3] - 2026-01-01

- Previous release.

[Unreleased]: https://github.com/wallstop/fortress-rollback/compare/v1.2.3...HEAD
[1.2.3]: https://github.com/wallstop/fortress-rollback/releases/tag/v1.2.3
""",
        encoding="utf-8",
    )
    (base / ".github" / "ISSUE_TEMPLATE" / "bug_report.yml").write_text(
        """body:
  - type: dropdown
    attributes:
      options:
        # BEGIN_FORTRESS_VERSIONS
        - v1.2.3
        # END_FORTRESS_VERSIONS
""",
        encoding="utf-8",
    )
    shutil.copy2(
        REPO_ROOT / "scripts" / "sync-version.sh",
        base / "scripts" / "sync-version.sh",
    )
    shutil.copy2(
        REPO_ROOT / "scripts" / "ci" / "sync-issue-template-versions.py",
        base / "scripts" / "ci" / "sync-issue-template-versions.py",
    )
    lock = subprocess.run(
        ["cargo", "generate-lockfile"],
        cwd=base,
        capture_output=True,
        text=True,
        check=False,
    )
    assert lock.returncode == 0, lock.stdout + lock.stderr
    subprocess.run(["git", "init", "--quiet", "-b", "main"], cwd=base, check=True)
    subprocess.run(
        ["git", "config", "user.name", "Release Test"], cwd=base, check=True
    )
    subprocess.run(
        ["git", "config", "user.email", "release@example.invalid"],
        cwd=base,
        check=True,
    )
    subprocess.run(["git", "add", "--all"], cwd=base, check=True)
    subprocess.run(
        ["git", "commit", "--quiet", "-m", "trusted base"],
        cwd=base,
        check=True,
    )
    subprocess.run(["git", "clone", "--quiet", str(base), str(candidate)], check=True)
    subprocess.run(
        ["git", "config", "user.name", "Release Test"], cwd=candidate, check=True
    )
    subprocess.run(
        ["git", "config", "user.email", "release@example.invalid"],
        cwd=candidate,
        check=True,
    )

    current, target, prepared_files, _roots = release_state.prepare_release.prepare(
        candidate, "minor", "2026-07-18"
    )
    assert (current, target) == ("1.2.3", "1.3.0")
    release_state.prepare_release.apply_prepared(prepared_files)
    release_state._run_trusted_issue_template_sync(base, candidate, target)
    release_state.generate(
        candidate,
        previous_version=current,
        target_version=target,
        bump="minor",
        release_date="2026-07-18",
    )
    subprocess.run(["git", "add", "--all"], cwd=candidate, check=True)
    subprocess.run(
        ["git", "commit", "--quiet", "-m", "Prepare v1.3.0 release"],
        cwd=candidate,
        check=True,
    )
    return base, candidate


def test_generate_and_verify_cover_prepared_release_metadata(tmp_path: Path) -> None:
    repo = _fixture(tmp_path)

    generated = _generate(repo)
    verified = release_state.verify(repo, expected_version="1.3.0")

    assert verified == generated
    document = json.loads((repo / release_state.MANIFEST_NAME).read_text())
    assert document == {
        "schema_version": 1,
        "crate_name": "fortress-rollback",
        "previous_version": "1.2.3",
        "target_version": "1.3.0",
        "bump": "minor",
        "release_date": "2026-07-18",
        "source_sha256": generated.source_sha256,
    }


def test_trusted_reconstruction_accepts_exact_generated_candidate(
    tmp_path: Path,
) -> None:
    base, candidate = _trusted_base_fixture(tmp_path)

    verified = release_state.verify_prepared_candidate(candidate, base)

    assert verified.previous_version == "1.2.3"
    assert verified.target_version == "1.3.0"
    assert verified.release_date == "2026-07-18"


def test_current_prospective_squash_tree_matches_trusted_base(
    tmp_path: Path,
) -> None:
    base, candidate = _trusted_base_fixture(tmp_path)

    verified = release_state.verify_prospective_candidate(candidate, base)

    assert verified.previous_version == "1.2.3"
    assert verified.target_version == "1.3.0"


def test_stale_prospective_squash_tree_fails_after_base_advances(
    tmp_path: Path,
) -> None:
    base, candidate = _trusted_base_fixture(tmp_path)
    old_base = subprocess.run(
        ["git", "-C", str(base), "rev-parse", "HEAD"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()
    release_head = subprocess.run(
        ["git", "-C", str(candidate), "rev-parse", "HEAD"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()
    patch = subprocess.run(
        ["git", "-C", str(candidate), "diff", "--binary", old_base, release_head],
        capture_output=True,
        check=True,
    ).stdout
    (base / "late-main-change.txt").write_text(
        "landed after release preparation\n", encoding="utf-8"
    )
    subprocess.run(["git", "add", "--all"], cwd=base, check=True)
    subprocess.run(
        ["git", "commit", "--quiet", "-m", "Advance main"], cwd=base, check=True
    )
    subprocess.run(
        ["git", "fetch", "--quiet", "origin", "main"], cwd=candidate, check=True
    )
    subprocess.run(
        ["git", "checkout", "--quiet", "--detach", "origin/main"],
        cwd=candidate,
        check=True,
    )
    applied = subprocess.run(
        ["git", "apply", "--index", "--binary", "-"],
        cwd=candidate,
        input=patch,
        capture_output=True,
        check=False,
    )
    assert applied.returncode == 0, applied.stderr.decode(errors="replace")
    subprocess.run(
        ["git", "commit", "--quiet", "-m", "Prospective squash release"],
        cwd=candidate,
        check=True,
    )

    with pytest.raises(
        release_state.ReleaseStateError,
        match="source digest does not match|does not match trusted reconstruction",
    ):
        release_state.verify_prospective_candidate(candidate, base)


def test_trusted_reconstruction_rejects_arbitrary_change_with_regenerated_state(
    tmp_path: Path,
) -> None:
    base, candidate = _trusted_base_fixture(tmp_path)
    (candidate / "backdoor.txt").write_text(
        "This file was not generated from the trusted base.\n", encoding="utf-8"
    )
    subprocess.run(["git", "add", "backdoor.txt"], cwd=candidate, check=True)
    release_state.generate(
        candidate,
        previous_version="1.2.3",
        target_version="1.3.0",
        bump="minor",
        release_date="2026-07-18",
    )
    subprocess.run(["git", "add", "--all"], cwd=candidate, check=True)
    subprocess.run(
        ["git", "commit", "--quiet", "--amend", "--no-edit"],
        cwd=candidate,
        check=True,
    )
    release_state.verify(candidate)

    with pytest.raises(
        release_state.ReleaseStateError,
        match="does not match trusted reconstruction",
    ):
        release_state.verify_prepared_candidate(candidate, base)


def test_trusted_reconstruction_rejects_candidate_not_directly_atop_base(
    tmp_path: Path,
) -> None:
    base, candidate = _trusted_base_fixture(tmp_path)
    (candidate / "second.txt").write_text("second commit\n", encoding="utf-8")
    subprocess.run(["git", "add", "second.txt"], cwd=candidate, check=True)
    subprocess.run(
        ["git", "commit", "--quiet", "-m", "unexpected second commit"],
        cwd=candidate,
        check=True,
    )

    with pytest.raises(
        release_state.ReleaseStateError,
        match="one preparation commit directly atop trusted base",
    ):
        release_state.verify_prepared_candidate(candidate, base)


@pytest.mark.parametrize(
    ("mutate", "message"),
    [
        (
            lambda repo: (repo / "src" / "lib.rs").write_text("changed\n"),
            "source digest does not match",
        ),
        (
            lambda repo: (repo / "release-check.sh").chmod(0o644),
            "source digest does not match",
        ),
        (
            lambda repo: (
                (repo / "source-link").unlink(),
                (repo / "source-link").symlink_to("Cargo.toml"),
            ),
            "source digest does not match",
        ),
    ],
)
def test_verify_rejects_changed_content_mode_or_symlink_target(
    tmp_path: Path, mutate: object, message: str
) -> None:
    repo = _fixture(tmp_path)
    _generate(repo)

    mutate(repo)  # type: ignore[operator]

    with pytest.raises(release_state.ReleaseStateError, match=message):
        release_state.verify(repo)


def test_manifest_content_is_excluded_from_source_digest(tmp_path: Path) -> None:
    repo = _fixture(tmp_path)
    state = _generate(repo)
    manifest = repo / release_state.MANIFEST_NAME
    document = json.loads(manifest.read_text())
    document["source_sha256"] = "0" * 64
    manifest.write_text(json.dumps(document), encoding="utf-8")

    assert release_state.source_digest(repo) == state.source_sha256


@pytest.mark.parametrize(
    ("mutation", "message"),
    [
        (
            lambda repo: (repo / "Cargo.toml").write_text(
                '[package]\nname = "fortress-rollback"\nversion = "1.3.1"\n'
            ),
            "Cargo.toml version",
        ),
        (
            lambda repo: (repo / "CHANGELOG.md").write_text(
                (repo / "CHANGELOG.md").read_text().replace(
                    "## [Unreleased]\n\n", "## [Unreleased]\n\n- Late change.\n\n"
                )
            ),
            "Unreleased section must be empty",
        ),
        (
            lambda repo: (repo / ".github" / "ISSUE_TEMPLATE" / "bug_report.yml").write_text(
                "description: v1.3.0\n"
                "# BEGIN_FORTRESS_VERSIONS\n- v1.2.3\n# END_FORTRESS_VERSIONS\n"
            ),
            "dropdown must contain v1.3.0",
        ),
    ],
)
def test_verify_rejects_inconsistent_release_metadata(
    tmp_path: Path, mutation: object, message: str
) -> None:
    repo = _fixture(tmp_path)
    _generate(repo)

    mutation(repo)  # type: ignore[operator]

    with pytest.raises(release_state.ReleaseStateError, match=message):
        release_state.verify(repo)


def test_generate_rejects_bump_that_does_not_reach_target(tmp_path: Path) -> None:
    repo = _fixture(tmp_path)

    with pytest.raises(release_state.ReleaseStateError, match="does not match the patch bump"):
        release_state.generate(
            repo,
            previous_version="1.2.3",
            target_version="1.3.0",
            bump="patch",
            release_date="2026-07-18",
        )


def test_generate_rejects_breaking_patch_for_zero_major_release(
    tmp_path: Path,
) -> None:
    repo = _fixture(tmp_path)
    (repo / "Cargo.toml").write_text(
        '[package]\nname = "fortress-rollback"\nversion = "0.1.4"\n',
        encoding="utf-8",
    )
    (repo / "CHANGELOG.md").write_text(
        """# Changelog

## [Unreleased]

## [0.1.4] - 2026-07-18

### Changed

- **Breaking:** Reject the previous wire protocol.

## [0.1.3] - 2026-01-01

- Previous release.
""",
        encoding="utf-8",
    )
    (repo / ".github" / "ISSUE_TEMPLATE" / "bug_report.yml").write_text(
        """# BEGIN_FORTRESS_VERSIONS
- v0.1.4
- v0.1.3
# END_FORTRESS_VERSIONS
""",
        encoding="utf-8",
    )

    with pytest.raises(
        release_state.ReleaseStateError,
        match="requested patch bump is below the minimum minor bump",
    ):
        release_state.generate(
            repo,
            previous_version="0.1.3",
            target_version="0.1.4",
            bump="patch",
            release_date="2026-07-18",
        )


def test_generate_rejects_previous_version_not_immediately_after_target(
    tmp_path: Path,
) -> None:
    repo = _fixture(tmp_path)

    with pytest.raises(
        release_state.ReleaseStateError,
        match=(
            "previous_version 1.2.9 does not match the immediately following "
            "CHANGELOG.md release 1.2.3"
        ),
    ):
        release_state.generate(
            repo,
            previous_version="1.2.9",
            target_version="1.3.0",
            bump="minor",
            release_date="2026-07-18",
        )


def test_generate_rejects_undated_immediately_previous_release_heading(
    tmp_path: Path,
) -> None:
    repo = _fixture(tmp_path)
    changelog = repo / "CHANGELOG.md"
    changelog.write_text(
        changelog.read_text(encoding="utf-8").replace(
            "## [1.2.3] - 2026-01-01",
            "## [1.2.3]",
        ),
        encoding="utf-8",
    )

    with pytest.raises(
        release_state.ReleaseStateError,
        match=(
            "heading immediately after the target release must use canonical"
        ),
    ):
        release_state.generate(
            repo,
            previous_version="1.2.3",
            target_version="1.3.0",
            bump="minor",
            release_date="2026-07-18",
        )


def test_verify_rejects_tampered_bump_even_though_manifest_is_not_hashed(
    tmp_path: Path,
) -> None:
    repo = _fixture(tmp_path)
    _generate(repo)
    manifest = repo / release_state.MANIFEST_NAME
    document = json.loads(manifest.read_text(encoding="utf-8"))
    document["bump"] = "patch"
    manifest.write_text(json.dumps(document), encoding="utf-8")

    with pytest.raises(
        release_state.ReleaseStateError,
        match="target_version 1.3.0 does not match the patch bump",
    ):
        release_state.verify(repo)


def test_verify_rechecks_bump_floor_from_finalized_target_notes(
    tmp_path: Path,
) -> None:
    repo = _fixture(tmp_path)
    (repo / "Cargo.toml").write_text(
        '[package]\nname = "fortress-rollback"\nversion = "1.2.4"\n',
        encoding="utf-8",
    )
    changelog = repo / "CHANGELOG.md"
    changelog.write_text(
        """# Changelog

## [Unreleased]

## [1.2.4] - 2026-07-18

### Fixed

- Repair release automation.

## [1.2.3] - 2026-01-01

- Previous release.
""",
        encoding="utf-8",
    )
    (repo / ".github" / "ISSUE_TEMPLATE" / "bug_report.yml").write_text(
        """# BEGIN_FORTRESS_VERSIONS
- v1.2.4
- v1.2.3
# END_FORTRESS_VERSIONS
""",
        encoding="utf-8",
    )
    release_state.generate(
        repo,
        previous_version="1.2.3",
        target_version="1.2.4",
        bump="patch",
        release_date="2026-07-18",
    )
    subprocess.run(["git", "add", release_state.MANIFEST_NAME], cwd=repo, check=True)
    changelog.write_text(
        changelog.read_text(encoding="utf-8").replace("### Fixed", "### Added"),
        encoding="utf-8",
    )

    with pytest.raises(
        release_state.ReleaseStateError,
        match="requested patch bump is below the minimum minor bump",
    ):
        release_state.verify(repo)


def test_source_digest_rejects_missing_tracked_file(tmp_path: Path) -> None:
    repo = _fixture(tmp_path)
    os.unlink(repo / "src" / "lib.rs")

    with pytest.raises(release_state.ReleaseStateError, match="cannot be read"):
        release_state.source_digest(repo)


@pytest.mark.parametrize("invalid_date", ["20260718", "2026-W29-6", "2026-7-18"])
def test_generate_rejects_noncanonical_release_date(
    tmp_path: Path, invalid_date: str
) -> None:
    repo = _fixture(tmp_path)

    with pytest.raises(release_state.ReleaseStateError, match="YYYY-MM-DD"):
        release_state.generate(
            repo,
            previous_version="1.2.3",
            target_version="1.3.0",
            bump="minor",
            release_date=invalid_date,
        )


@pytest.mark.parametrize(
    "version",
    [f"{'9' * 4_301}.0.0", "18446744073709551616.0.0"],
)
def test_version_parser_rejects_unbounded_components_without_traceback(
    version: str,
) -> None:
    with pytest.raises(release_state.ReleaseStateError, match="u64 SemVer|larger than u64"):
        release_state._parse_version(version, "test_version")


def test_verify_validates_expected_version_before_loading_state(tmp_path: Path) -> None:
    with pytest.raises(release_state.ReleaseStateError, match="expected_version"):
        release_state.verify(
            tmp_path,
            expected_version=f"{'9' * 4_301}.0.0",
        )


def test_release_branch_state_is_valid_when_present() -> None:
    """Generated release PRs fail CI on drift without blocking later work."""
    head_ref = os.environ.get("GITHUB_HEAD_REF", "")
    if head_ref.startswith("release/v"):
        release_state.verify(REPO_ROOT)
