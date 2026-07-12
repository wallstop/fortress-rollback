#!/usr/bin/env python3
"""Regression tests for local Git hook policy."""

from __future__ import annotations

import subprocess
from pathlib import Path

import pytest

try:
    import yaml

    HAS_YAML = True
except ImportError:
    HAS_YAML = False


REPO_ROOT = Path(__file__).resolve().parents[2]
CONFIG_PATH = REPO_ROOT / ".pre-commit-config.yaml"
DOCS_CONTRIBUTING = REPO_ROOT / "docs" / "contributing.md"
WIKI_CONTRIBUTING = REPO_ROOT / "wiki" / "Contributing.md"
INSTALL_HOOKS = REPO_ROOT / "scripts" / "install-hooks.sh"
LEGACY_PRE_COMMIT = REPO_ROOT / "scripts" / "pre-commit"
CARGO_HACK_WRAPPER = REPO_ROOT / "scripts" / "build" / "check-cargo-hack.py"
RUSTFMT_WRAPPER = REPO_ROOT / "scripts" / "build" / "run-cargo-fmt.py"
CI_DOCS_WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ci-docs.yml"
CI_VERSION_SYNC_WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ci-version-sync.yml"

SLOW_HOOK_IDS = {
    "cargo-clippy",
    "rustdoc-links",
    "check-links",
    "check-llm-skills",
    "check-code-fence-syntax",
    "check-derive-bounds",
    "check-doc-claims",
    "check-hook-output-format",
    "check-shell-portability",
    "cargo-hack-check",
    "sync-version-check",
    "sync-wiki",
    "wiki-consistency",
}
PRE_PUSH_ALLOWLIST = {"check-issue-template-versions"}
MANUAL_DOC_HOOK_IDS = {
    "cargo-clippy",
    "rustdoc-links",
    "check-links",
    "cargo-hack-check",
    "sync-wiki",
    "check-llm-skills",
    "check-shell-portability",
    "sync-version-check",
    "check-doc-claims",
    "check-derive-bounds",
}
DOCS_WORKFLOW_REQUIRED_PATHS = {
    "**.md",
    "**.rs",
    "Cargo.toml",
    "Cargo.lock",
    ".cargo/config.toml",
    "docs/**",
    "wiki/**",
    "scripts/docs/**",
    "scripts/tests/**",
    "scripts/ci/check-doc-claims.sh",
    "scripts/ci/check-derive-bounds.sh",
    ".markdownlint.json",
    ".markdown-link-check.json",
    ".lychee.toml",
    ".vale.ini",
    ".vale/**",
    ".github/actions/install-cargo-tool/**",
    ".github/workflows/ci-docs.yml",
    ".github/workflows/ci-rust.yml",
}
DOCS_WORKFLOW_FORBIDDEN_PATHS = {
    "scripts/**",
    "**.toml",
    "**.yml",
    "**.yaml",
    "**.sh",
    "**.txt",
    "**.json",
}
VERSION_SYNC_REQUIRED_PATH_GLOBS = {
    "**.rs",
    "**.md",
    "**.toml",
    "**.yml",
    "**.yaml",
    "**.sh",
    "**.txt",
    "**.json",
}
VERSION_SYNC_REQUIRED_PATHS = {
    "scripts/sync-version.sh",
    ".github/workflows/ci-version-sync.yml",
}


@pytest.fixture()
def pre_commit_config() -> dict:
    """Load the pre-commit configuration."""
    if not HAS_YAML:
        pytest.skip("PyYAML not installed")
    return yaml.safe_load(CONFIG_PATH.read_text(encoding="utf-8"))


def _all_hooks(config: dict) -> list[dict]:
    hooks: list[dict] = []
    for repo in config["repos"]:
        hooks.extend(repo.get("hooks", []))
    return hooks


def _hook_by_id(config: dict, hook_id: str) -> dict:
    for hook in _all_hooks(config):
        if hook["id"] == hook_id:
            return hook
    raise AssertionError(f"Hook {hook_id!r} not found")


def _load_yaml(path: Path) -> dict:
    if not HAS_YAML:
        pytest.skip("PyYAML not installed")
    data = yaml.safe_load(path.read_text(encoding="utf-8"))
    assert isinstance(data, dict)
    return data


def _workflow_paths(workflow: dict, event: str) -> set[str]:
    on_block = workflow.get("on")
    if on_block is None:
        on_block = workflow.get(True)
    assert isinstance(on_block, dict), "Workflow missing on block"

    event_block = on_block.get(event)
    assert isinstance(event_block, dict), f"Workflow missing {event!r} block"

    paths = event_block.get("paths")
    assert isinstance(paths, list), f"Workflow {event!r} block missing paths"
    return {str(path) for path in paths}


def test_default_hook_stage_is_fast_pre_commit(pre_commit_config: dict) -> None:
    """Unstaged hooks must not implicitly run during pre-push."""
    assert pre_commit_config.get("default_stages") == ["pre-commit"]


def test_slow_hooks_are_manual_only(pre_commit_config: dict) -> None:
    """Slow full-repository checks must not block commit or push."""
    for hook_id in sorted(MANUAL_DOC_HOOK_IDS):
        hook = _hook_by_id(pre_commit_config, hook_id)
        assert hook.get("stages") == ["manual"], (
            f"{hook_id} must stay manual-only; current stages={hook.get('stages')!r}"
        )


def test_pre_push_hooks_are_explicitly_allowlisted(pre_commit_config: dict) -> None:
    """Pre-push must stay lightweight and opt-in per hook."""
    pre_push_hooks = {
        hook["id"]
        for hook in _all_hooks(pre_commit_config)
        if hook.get("stages") == ["pre-push"]
    }

    assert pre_push_hooks <= PRE_PUSH_ALLOWLIST


def test_wire_golden_hook_checks_the_staged_diff_every_commit(
    pre_commit_config: dict,
) -> None:
    """Deletion, rename, and partial staging must not bypass wire immutability."""
    hook = _hook_by_id(pre_commit_config, "wire-golden-immutable")
    assert hook["entry"] == (
        "python scripts/hooks/check-wire-golden-immutable.py --cached"
    )
    assert hook["pass_filenames"] is False
    assert hook["always_run"] is True


@pytest.mark.parametrize("path", [DOCS_CONTRIBUTING, WIKI_CONTRIBUTING])
def test_docs_install_both_framework_hooks(path: Path) -> None:
    """Contributor docs must install both managed hook types."""
    content = path.read_text(encoding="utf-8")

    assert "pre-commit install --hook-type pre-commit --hook-type pre-push" in content
    assert "pre-commit install\n" not in content


def test_install_script_uses_pre_commit_framework_for_both_hooks() -> None:
    """The helper installer must not reinstall the old standalone hook."""
    content = INSTALL_HOOKS.read_text(encoding="utf-8")

    install_command = "pre-commit install --hook-type pre-commit --hook-type pre-push"
    assert install_command in content
    assert f"{install_command} --overwrite" not in content
    assert 'HOOK_SOURCE="$SCRIPT_DIR/pre-commit"' not in content
    assert 'cp "$HOOK_SOURCE" "$HOOK_DEST"' not in content
    assert "pre-commit install --hook-type pre-commit --hook-type pre-push --overwrite" not in content
    assert "Preserved custom $hook hook" in content
    assert "Removed legacy Fortress Rollback $hook hook" in content
    assert "Pre-commit hook for Fortress Rollback" in content


def test_legacy_pre_commit_fails_cargo_bump_with_unstaged_work(tmp_path: Path) -> None:
    """The legacy hook must not auto-stage unrelated unstaged files."""
    repo = tmp_path / "repo"
    scripts_dir = repo / "scripts"
    scripts_dir.mkdir(parents=True)
    (scripts_dir / "pre-commit").write_text(
        LEGACY_PRE_COMMIT.read_text(encoding="utf-8"),
        encoding="utf-8",
    )
    sync_stub = scripts_dir / "sync-version.sh"
    sync_stub.write_text(
        "#!/bin/sh\n"
        "echo 'sync-version.sh should not run when unstaged changes exist' >&2\n"
        "exit 99\n",
        encoding="utf-8",
    )
    sync_stub.chmod(0o755)
    (repo / "Cargo.toml").write_text(
        '[package]\nname = "fixture"\nversion = "1.2.3"\n',
        encoding="utf-8",
    )
    subprocess.run(["git", "init"], cwd=repo, check=True, capture_output=True, text=True)
    subprocess.run(
        ["git", "config", "user.email", "tests@example.com"],
        cwd=repo,
        check=True,
        capture_output=True,
        text=True,
    )
    subprocess.run(
        ["git", "config", "user.name", "Hook Tests"],
        cwd=repo,
        check=True,
        capture_output=True,
        text=True,
    )
    subprocess.run(["git", "add", "."], cwd=repo, check=True, capture_output=True, text=True)
    subprocess.run(
        ["git", "commit", "-m", "init"],
        cwd=repo,
        check=True,
        capture_output=True,
        text=True,
    )
    (repo / "Cargo.toml").write_text(
        '[package]\nname = "fixture"\nversion = "1.2.4"\n',
        encoding="utf-8",
    )
    subprocess.run(
        ["git", "add", "Cargo.toml"],
        cwd=repo,
        check=True,
        capture_output=True,
        text=True,
    )
    (repo / "README.md").write_text("unstaged work\n", encoding="utf-8")

    result = subprocess.run(
        ["bash", "scripts/pre-commit"],
        cwd=repo,
        capture_output=True,
        text=True,
        check=False,
    )

    combined = result.stdout + result.stderr
    assert result.returncode == 1, combined
    assert (
        "Unstaged or untracked changes are present while Cargo.toml has a staged version bump"
        in combined
    )
    assert "sync-version.sh should not run" not in combined


def test_cargo_hack_local_excludes_match_ci_slow_features() -> None:
    """Local cargo-hack runs must avoid bundled Z3 builds."""
    content = CARGO_HACK_WRAPPER.read_text(encoding="utf-8")

    assert "z3-verification,z3-verification-bundled,graphical-examples" in content


def test_rust_formatter_wrapper_is_file_scoped() -> None:
    """Fast Rust formatting must not mutate unrelated Rust files."""
    content = RUSTFMT_WRAPPER.read_text(encoding="utf-8")

    assert '"rustfmt"' in content
    assert "skip_children=true" in content
    assert '["cargo", "fmt"]' not in content
    assert "Run cargo fmt to auto-fix" not in content
    assert "Failed to run cargo fmt" not in content


@pytest.mark.parametrize("path", [DOCS_CONTRIBUTING, WIKI_CONTRIBUTING])
def test_manual_hook_docs_run_against_all_files(path: Path) -> None:
    """Manual safety-net commands must not silently skip unstaged checks."""
    content = path.read_text(encoding="utf-8")

    for hook_id in sorted(MANUAL_DOC_HOOK_IDS):
        assert (
            f"pre-commit run --hook-stage manual {hook_id} --all-files" in content
        ), f"{path} does not document --all-files for {hook_id}"


def test_ci_docs_enforces_manual_policy_checks() -> None:
    """Checks removed from fast hooks must still have CI coverage."""
    content = CI_DOCS_WORKFLOW.read_text(encoding="utf-8")

    assert "./scripts/ci/check-doc-claims.sh" in content
    assert "./scripts/ci/check-derive-bounds.sh" in content
    assert "bash scripts/sync-version.sh --check" not in content


def test_ci_docs_trigger_paths_are_scoped() -> None:
    """Docs CI must not include broad non-doc globs introduced for version sync."""
    workflow = _load_yaml(CI_DOCS_WORKFLOW)

    for event in ("push", "pull_request"):
        paths = _workflow_paths(workflow, event)
        assert DOCS_WORKFLOW_REQUIRED_PATHS <= paths
        assert DOCS_WORKFLOW_FORBIDDEN_PATHS.isdisjoint(paths)


def test_version_sync_workflow_handles_broad_version_globs() -> None:
    """Broad extension globs belong in the dedicated version-sync workflow."""
    content = CI_VERSION_SYNC_WORKFLOW.read_text(encoding="utf-8")
    workflow = _load_yaml(CI_VERSION_SYNC_WORKFLOW)

    assert "bash scripts/sync-version.sh --check" in content
    for event in ("push", "pull_request"):
        paths = _workflow_paths(workflow, event)
        assert VERSION_SYNC_REQUIRED_PATH_GLOBS <= paths
        assert VERSION_SYNC_REQUIRED_PATHS <= paths


def test_version_sync_workflow_enforces_committed_wire_golden_diff() -> None:
    """PR CI must compare immutable fixtures against the actual base commit."""
    content = CI_VERSION_SYNC_WORKFLOW.read_text(encoding="utf-8")

    assert "fetch-depth: 0" in content
    assert "if: github.event_name == 'pull_request'" in content
    assert "BASE_SHA: ${{ github.event.pull_request.base.sha }}" in content
    assert (
        'python3 scripts/hooks/check-wire-golden-immutable.py --base-ref "$BASE_SHA"'
        in content
    )
