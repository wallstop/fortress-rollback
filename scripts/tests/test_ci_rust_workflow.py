"""Regression tests for the primary Rust CI workflow shape."""

from __future__ import annotations

from pathlib import Path

import yaml

REPO_ROOT = Path(__file__).resolve().parents[2]
CI_RUST_WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ci-rust.yml"


def _load_ci_rust_workflow() -> dict:
    return yaml.safe_load(CI_RUST_WORKFLOW.read_text(encoding="utf-8"))


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
