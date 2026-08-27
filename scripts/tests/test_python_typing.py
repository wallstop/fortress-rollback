"""Contracts for blocking, repository-wide production Python type checking."""

from __future__ import annotations

import configparser
import re
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[2]
MYPY_VERSION = "2.3.1"
TOMLI_VERSION = "2.4.1"
PY_YAML_STUB_VERSION = "6.0.12.20260815"


def _production_python() -> set[Path]:
    return {
        path.relative_to(ROOT)
        for path in (ROOT / "scripts").rglob("*.py")
        if "tests" not in path.relative_to(ROOT / "scripts").parts
    }


def test_strict_config_checks_every_production_script_at_python_310_floor() -> None:
    config = configparser.ConfigParser()
    config.read(ROOT / "mypy.ini", encoding="utf-8")
    options = config["mypy"]

    assert options.getboolean("strict")
    assert options["python_version"] == "3.10"
    assert options["files"] == "scripts"
    assert options["exclude"] == "^scripts/tests/"
    assert "ignore_errors" not in options
    assert Path("scripts/api/public_api_census.py") in _production_python()


def test_ci_type_check_is_pinned_blocking_and_in_the_required_summary() -> None:
    workflow_path = ROOT / ".github" / "workflows" / "ci-quality.yml"
    workflow = yaml.safe_load(workflow_path.read_text(encoding="utf-8"))
    job = workflow["jobs"]["python-type-check"]
    run_blocks = "\n".join(str(step.get("run", "")) for step in job["steps"])

    assert job["timeout-minutes"] == 5
    assert all(not step.get("continue-on-error", False) for step in job["steps"])
    assert 'python-version: "3.10"' in workflow_path.read_text(encoding="utf-8")
    assert f"mypy=={MYPY_VERSION}" in run_blocks
    assert f"tomli=={TOMLI_VERSION}" in run_blocks
    assert f"types-PyYAML=={PY_YAML_STUB_VERSION}" in run_blocks
    assert "mypy --config-file mypy.ini" in run_blocks
    assert "python-type-check" in workflow["jobs"]["quality-summary"]["needs"]


def test_precommit_type_check_uses_the_same_exact_toolchain() -> None:
    precommit = yaml.safe_load(
        (ROOT / ".pre-commit-config.yaml").read_text(encoding="utf-8")
    )
    hooks = {
        hook["id"]: hook
        for repository in precommit["repos"]
        for hook in repository["hooks"]
    }
    hook = hooks["mypy"]

    assert hook["entry"] == "mypy --config-file mypy.ini"
    assert hook["pass_filenames"] is False
    assert hook["additional_dependencies"] == [
        f"mypy=={MYPY_VERSION}",
        f"tomli=={TOMLI_VERSION}",
        f"types-PyYAML=={PY_YAML_STUB_VERSION}",
    ]
    pattern = re.compile(hook["files"])
    assert pattern.search("scripts/release/prepare_release.py")
    assert not pattern.search("scripts/tests/test_prepare_release.py")
    assert pattern.search("mypy.ini")


def test_production_sources_have_no_blanket_type_suppression() -> None:
    for relative_path in _production_python():
        content = (ROOT / relative_path).read_text(encoding="utf-8")
        assert "# mypy: ignore-errors" not in content
        assert not re.search(r"#\s*type:\s*ignore(?!\[)", content)
