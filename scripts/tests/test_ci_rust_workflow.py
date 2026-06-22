"""Regression tests for the primary Rust CI workflow shape."""

from __future__ import annotations

from pathlib import Path

# check-tomllib-fallback hook enforces this pattern repo-wide.
try:
    import tomllib
except ImportError:  # pragma: no cover - exercised only on Python < 3.11
    try:
        import tomli as tomllib
    except ImportError:  # pragma: no cover - depends on local test deps
        tomllib = None

import pytest
import yaml

REPO_ROOT = Path(__file__).resolve().parents[2]
CI_RUST_WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ci-rust.yml"
CARGO_CONFIG = REPO_ROOT / ".cargo" / "config.toml"
CARGO_HEAVY_WORKFLOWS_WITH_PATH_FILTERS = (
    REPO_ROOT / ".github" / "workflows" / "ci-benchmarks.yml",
    REPO_ROOT / ".github" / "workflows" / "ci-coverage.yml",
    REPO_ROOT / ".github" / "workflows" / "ci-docs.yml",
    REPO_ROOT / ".github" / "workflows" / "ci-network.yml",
    REPO_ROOT / ".github" / "workflows" / "ci-quality.yml",
    REPO_ROOT / ".github" / "workflows" / "ci-rust.yml",
    REPO_ROOT / ".github" / "workflows" / "ci-safety.yml",
    REPO_ROOT / ".github" / "workflows" / "ci-security.yml",
    REPO_ROOT / ".github" / "workflows" / "ci-verification.yml",
)


def _load_ci_rust_workflow() -> dict:
    return yaml.safe_load(CI_RUST_WORKFLOW.read_text(encoding="utf-8"))


def _load_workflow(path: Path) -> dict:
    return yaml.safe_load(path.read_text(encoding="utf-8"))


def _workflow_paths(workflow: dict, event: str) -> set[str]:
    on_block = workflow.get("on")
    if on_block is None:
        on_block = workflow.get(True)
    assert isinstance(on_block, dict), "workflow missing on block"

    event_block = on_block.get(event)
    assert isinstance(event_block, dict), f"workflow missing {event!r} block"

    paths = event_block.get("paths")
    assert isinstance(paths, list), f"workflow {event!r} block missing paths"
    return {str(path) for path in paths}


def _load_cargo_config() -> dict:
    if tomllib is None:
        pytest.skip("tomllib/tomli not available")
    return tomllib.loads(CARGO_CONFIG.read_text(encoding="utf-8"))


@pytest.mark.parametrize(
    ("section", "key", "expected"),
    (
        ("http", "multiplexing", False),
        ("net", "retry", 10),
    ),
)
def test_cargo_network_config_hardens_registry_fetches(
    section: str, key: str, expected: object
) -> None:
    """Cargo registry fetches should survive common CI network flakes."""
    config = _load_cargo_config()

    assert config[section][key] == expected


def test_semver_failure_summary_distinguishes_infra_from_api_breaks() -> None:
    """Semver diagnostics must not misclassify registry flakes as API breaks."""
    workflow = _load_ci_rust_workflow()
    semver_steps = workflow["jobs"]["semver-checks"]["steps"]
    summary_step = next(
        step for step in semver_steps if step.get("name") == "Explain semver failure"
    )
    summary = summary_step["run"]

    assert "setup/dependency-resolution" in summary
    assert "crates.io network error" in summary
    assert "source of truth" in summary
    assert "found a breaking public-API change" not in summary


@pytest.mark.parametrize("workflow_path", CARGO_HEAVY_WORKFLOWS_WITH_PATH_FILTERS)
@pytest.mark.parametrize("event", ("push", "pull_request"))
def test_cargo_heavy_workflows_trigger_on_cargo_config_changes(
    workflow_path: Path, event: str
) -> None:
    """Cargo-heavy CI must run when repo-level Cargo config changes."""
    workflow = _load_workflow(workflow_path)

    assert ".cargo/config.toml" in _workflow_paths(workflow, event)
    assert ".cargo/**" not in _workflow_paths(workflow, event)


def test_miri_job_has_no_cross_target_apt_setup() -> None:
    """Miri PR jobs must not depend on slow uncached cross-target apt installs."""
    workflow = _load_ci_rust_workflow()
    miri_job = workflow["jobs"]["miri"]

    matrix_entries = miri_job["strategy"]["matrix"]["include"]
    for entry in matrix_entries:
        assert "big_endian" not in entry, (
            "Big-endian coverage belongs in fast golden byte tests, not as a "
            f"cross-target Miri matrix flag: {entry!r}"
        )
        assert "s390x" not in entry["name"].lower()

    forbidden_fragments = (
        "apt-get",
        "gcc-s390x-linux-gnu",
        "s390x-unknown-linux-gnu",
    )
    for step in miri_job["steps"]:
        run = step.get("run", "")
        step_name = step.get("name", "<unnamed>")
        for fragment in forbidden_fragments:
            assert fragment not in run, (
                f"Miri step {step_name!r} contains {fragment!r}; keep Miri "
                "jobs host-native and cover wire byte order with unit tests."
            )
