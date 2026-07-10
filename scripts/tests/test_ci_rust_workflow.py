"""Regression tests for the primary Rust CI workflow shape."""

from __future__ import annotations

import shlex
import subprocess
from collections import Counter
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
EMSCRIPTEN_DEPENDENCY_CHECK = "scripts/ci/check-emscripten-dependencies.sh"
EMSCRIPTEN_DEPENDENCY_CHECK_PATH = REPO_ROOT / EMSCRIPTEN_DEPENDENCY_CHECK
MATRIX_TARGET_EXPRESSION = "${{ matrix.target }}"
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


def _shell_commands(step: dict) -> list[list[str]]:
    """Tokenize non-comment command lines without depending on shell whitespace."""
    run = step.get("run")
    if not isinstance(run, str):
        return []

    placeholder = "__MATRIX_TARGET_EXPRESSION__"
    return [
        [
            token.replace(placeholder, MATRIX_TARGET_EXPRESSION)
            for token in shlex.split(line.replace(MATRIX_TARGET_EXPRESSION, placeholder))
        ]
        for line in run.splitlines()
        if line.strip() and not line.lstrip().startswith("#")
    ]


def _cargo_commands(steps: list[dict], subcommand: str) -> list[list[str]]:
    """Return direct Cargo invocations for one subcommand."""
    return [
        command
        for step in steps
        for command in _shell_commands(step)
        if command[:2] == ["cargo", subcommand]
    ]


def _option_values(command: list[str], option: str) -> list[str]:
    """Read both ``--option value`` and ``--option=value`` forms."""
    values: list[str] = []
    for index, token in enumerate(command):
        if token == option:
            assert index + 1 < len(command), f"{option} is missing its value: {command}"
            values.append(command[index + 1])
        elif token.startswith(f"{option}="):
            values.append(token.partition("=")[2])
    return values


def _feature_set(command: list[str]) -> frozenset[str]:
    """Normalize Cargo feature values separated by commas or spaces."""
    return frozenset(
        feature
        for value in _option_values(command, "--features")
        for feature in value.replace(",", " ").split()
    )


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


def test_wasm_job_covers_browser_and_emscripten_targets() -> None:
    """WASM CI must compile both supported target families independently."""
    workflow = _load_ci_rust_workflow()
    wasm_job = workflow["jobs"]["wasm-check"]

    assert wasm_job["strategy"]["fail-fast"] is False
    assert Counter(wasm_job["strategy"]["matrix"]["target"]) == Counter(
        ("wasm32-unknown-unknown", "wasm32-unknown-emscripten")
    )

    steps = wasm_job["steps"]
    rust_step = next(
        step
        for step in steps
        if step.get("name") == "Install Rust toolchain with WASM target"
    )
    assert rust_step["with"]["targets"] == MATRIX_TARGET_EXPRESSION

    check_commands = _cargo_commands(steps, "check")
    assert len(check_commands) == 5
    for command in check_commands:
        assert _option_values(command, "--target") == [MATRIX_TARGET_EXPRESSION]
        assert command.count("--no-default-features") == 1

    assert Counter(_feature_set(command) for command in check_commands) == Counter(
        (
            frozenset(),
            frozenset({"sync-send"}),
            frozenset({"paranoid"}),
            frozenset({"json"}),
            frozenset({"hot-join"}),
        )
    )

    clippy_commands = _cargo_commands(steps, "clippy")
    assert len(clippy_commands) == 1
    clippy_command = clippy_commands[0]
    assert _option_values(clippy_command, "--target") == [MATRIX_TARGET_EXPRESSION]
    assert clippy_command.count("--no-default-features") == 1
    assert _feature_set(clippy_command) == frozenset()
    rustc_args = clippy_command[clippy_command.index("--") + 1 :]
    assert "-Dwarnings" in rustc_args or "--deny=warnings" in rustc_args or any(
        token in {"-D", "--deny"}
        and index + 1 < len(rustc_args)
        and rustc_args[index + 1] == "warnings"
        for index, token in enumerate(rustc_args)
    )


def test_wasm_job_rejects_browser_dependencies_only_on_emscripten() -> None:
    """The JS bridge ban must not reject legitimate browser dependencies."""
    workflow = _load_ci_rust_workflow()
    steps = workflow["jobs"]["wasm-check"]["steps"]
    check_step = next(
        step
        for step in steps
        if step.get("name") == "Reject browser-only dependencies on Emscripten"
    )

    assert check_step["if"] == "matrix.target == 'wasm32-unknown-emscripten'"
    assert check_step["run"] == f"./{EMSCRIPTEN_DEPENDENCY_CHECK}"

    for event in ("push", "pull_request"):
        assert EMSCRIPTEN_DEPENDENCY_CHECK in _workflow_paths(workflow, event)


def test_emscripten_dependency_check_quotes_configured_cargo_binary(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """A configured Cargo executable path must remain one shell argument."""
    cargo_wrapper = tmp_path / "fake cargo"
    cargo_wrapper.write_text(
        "#!/usr/bin/env bash\nprintf 'fortress-rollback 0.10.0\\n'\n",
        encoding="utf-8",
    )
    cargo_wrapper.chmod(0o755)
    manifest_path = tmp_path / "Cargo.toml"
    manifest_path.write_text("[package]\nname = 'fixture'\n", encoding="utf-8")
    monkeypatch.setenv("CARGO", str(cargo_wrapper))

    result = subprocess.run(
        [str(EMSCRIPTEN_DEPENDENCY_CHECK_PATH), str(manifest_path)],
        cwd=REPO_ROOT,
        check=False,
        capture_output=True,
        text=True,
    )

    assert result.returncode == 0, result.stderr
    assert "free of browser-only JS bridge crates" in result.stdout
