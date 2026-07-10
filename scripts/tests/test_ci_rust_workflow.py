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
ROOT_CARGO_MANIFEST = REPO_ROOT / "Cargo.toml"
NETWORK_DOCKERFILE = REPO_ROOT / "docker" / "Dockerfile"
EMSCRIPTEN_DEPENDENCY_CHECK = "scripts/ci/check-emscripten-dependencies.sh"
EMSCRIPTEN_DEPENDENCY_CHECK_PATH = REPO_ROOT / EMSCRIPTEN_DEPENDENCY_CHECK
GODOT_FIXTURE = "tests/godot-emscripten"
GODOT_FIXTURE_PATH_FILTER = f"{GODOT_FIXTURE}/**"
GODOT_EDITOR_SHA256 = (
    "d0bc2113065e481c9c2c2b2c37daa4e8be3fe9e27f0ab9ab0b6096e9a37907f3"
)
GODOT_TEMPLATES_SHA256 = (
    "3fbe2c0e2dec9d537ab9ec97bcf8da91dcf23357fc51f67092dd068d839290a8"
)
GODOT_CACHE_KEY = (
    "godot-${{ runner.os }}-${{ env.GODOT_EDITOR_SHA256 }}-"
    "${{ env.GODOT_TEMPLATES_SHA256 }}"
)
GODOT_CACHE_PATHS = {
    "${{ env.GODOT4_BIN }}",
    (
        "${{ env.XDG_DATA_HOME }}/godot/export_templates/"
        "${{ env.GODOT_TEMPLATE_VERSION }}/web_dlink_debug.zip"
    ),
    (
        "${{ env.XDG_DATA_HOME }}/godot/export_templates/"
        "${{ env.GODOT_TEMPLATE_VERSION }}/web_dlink_nothreads_debug.zip"
    ),
}
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


def _docker_copy_sources(dockerfile: str) -> tuple[Path, ...]:
    """Return sources from shell-form COPY instructions."""
    sources: list[Path] = []
    for line in dockerfile.splitlines():
        if not line.lstrip().upper().startswith("COPY "):
            continue

        tokens = shlex.split(line, comments=True)
        arguments = [token for token in tokens[1:] if not token.startswith("--")]
        sources.extend(Path(source.rstrip("/")) for source in arguments[:-1])
    return tuple(sources)


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


def test_network_docker_cache_build_covers_workspace_manifests_and_fails_closed() -> None:
    """The cache build must include every member and surface preparation errors."""
    if tomllib is None:
        pytest.skip("tomllib/tomli not available")

    cargo_manifest = tomllib.loads(ROOT_CARGO_MANIFEST.read_text(encoding="utf-8"))
    required_sources = {
        Path("Cargo.toml"),
        Path("Cargo.lock"),
    }
    required_sources.update(
        Path(member) / "Cargo.toml"
        for member in cargo_manifest["workspace"]["members"]
        if member != "."
    )
    dockerfile = NETWORK_DOCKERFILE.read_text(encoding="utf-8")
    cache_context, separator, cache_build_tail = dockerfile.partition("RUN cargo build")
    assert separator, "Dockerfile is missing its dependency-cache Cargo build"
    copy_sources = _docker_copy_sources(cache_context)

    assert set(copy_sources) == required_sources, (
        "Docker cache build must copy only Cargo.lock and workspace manifests; "
        f"expected {sorted(map(str, required_sources))}, "
        f"found {sorted(map(str, copy_sources))}"
    )

    cache_build = ["cargo", "build", *shlex.split(cache_build_tail.splitlines()[0])]
    assert cache_build == [
        "cargo",
        "build",
        "--locked",
        "--release",
        "-p",
        "network-test-peer",
    ]


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


def test_wasm_job_runs_browser_clock_smoke_under_node() -> None:
    """The browser row must execute the clock path with matching bindgen tools."""
    workflow = _load_ci_rust_workflow()
    steps = workflow["jobs"]["wasm-check"]["steps"]

    install_step = next(
        step
        for step in steps
        if step.get("name") == "Install wasm-bindgen CLI for browser runtime tests"
    )
    assert install_step["if"] == "matrix.target == 'wasm32-unknown-unknown'"
    assert install_step["uses"] == "taiki-e/install-action@v2"
    assert install_step["with"]["tool"] == "wasm-bindgen-cli@0.2.106"

    test_step = next(
        step
        for step in steps
        if step.get("name") == "Run browser WASM runtime smoke test"
    )
    assert test_step["if"] == "matrix.target == 'wasm32-unknown-unknown'"
    assert (
        test_step["env"]["CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER"]
        == "wasm-bindgen-test-runner"
    )

    commands = _cargo_commands([test_step], "test")
    assert len(commands) == 1
    command = commands[0]
    assert _option_values(command, "--package") + _option_values(command, "-p") == [
        "wasm-browser-smoke"
    ]
    assert _option_values(command, "--target") == [MATRIX_TARGET_EXPRESSION]
    assert command.count("--lib") == 1
    test_args = command[command.index("--") + 1 :]
    assert "--nocapture" in test_args


def test_godot_fixture_changes_trigger_rust_ci() -> None:
    """Every fixture input must schedule its browser integration gate."""
    workflow = _load_ci_rust_workflow()

    for event in ("push", "pull_request"):
        assert GODOT_FIXTURE_PATH_FILTER in _workflow_paths(workflow, event)


def test_godot_browser_job_pins_its_toolchain() -> None:
    """The integration gate must reproduce the supported Godot toolchain."""
    workflow = _load_ci_rust_workflow()
    job = workflow["jobs"]["godot-emscripten"]
    steps = job["steps"]

    assert job["runs-on"] == "ubuntu-latest"
    assert job["timeout-minutes"] == 60

    rust_step = next(
        step for step in steps if step.get("name") == "Install Godot Rust toolchain"
    )
    assert (
        rust_step["uses"]
        == "dtolnay/rust-toolchain@fa04a1451ff1842e2626ccb99004d0195b455a88"
    )
    assert rust_step["with"]["toolchain"] == "nightly-2026-07-08"
    assert set(str(rust_step["with"]["components"]).split(",")) == {
        "clippy",
        "rust-src",
        "rustfmt",
    }
    assert rust_step["with"]["targets"] == "wasm32-unknown-emscripten"

    emsdk_step = next(
        step for step in steps if step.get("name") == "Install Emscripten SDK"
    )
    assert (
        emsdk_step["uses"]
        == "emscripten-core/setup-emsdk@4528d102f7230f0e7b276855c01ea1159be0e984"
    )
    assert str(emsdk_step["with"]["version"]) == "4.0.11"
    assert emsdk_step["with"]["actions-cache-folder"] == "emsdk-cache"

    node_step = next(step for step in steps if step.get("name") == "Install Node.js")
    assert node_step["uses"] == "actions/setup-node@v6"
    assert str(node_step["with"]["node-version"]) == "24"
    assert node_step["with"]["package-manager-cache"] is False


def test_godot_download_cache_contains_only_verified_runtime_files() -> None:
    """Cache the three runtime inputs, never the template or browser archives."""
    workflow = _load_ci_rust_workflow()
    job = workflow["jobs"]["godot-emscripten"]
    steps = job["steps"]

    assert job["env"]["GODOT_VERSION"] == "4.6.3"
    assert job["env"]["GODOT_EDITOR_SHA256"] == GODOT_EDITOR_SHA256
    assert job["env"]["GODOT_TEMPLATES_SHA256"] == GODOT_TEMPLATES_SHA256

    restore_step = next(
        step for step in steps if step.get("name") == "Restore Godot runtime files"
    )
    save_step = next(
        step for step in steps if step.get("name") == "Save verified Godot runtime files"
    )
    assert restore_step["uses"] == "actions/cache/restore@v6"
    assert save_step["uses"] == "actions/cache/save@v6"
    assert restore_step["continue-on-error"] is True
    assert save_step["continue-on-error"] is True

    for cache_step in (restore_step, save_step):
        cache_entries = {
            line.strip()
            for line in cache_step["with"]["path"].splitlines()
            if line.strip()
        }
        assert cache_entries == GODOT_CACHE_PATHS
        assert cache_step["with"]["key"] == GODOT_CACHE_KEY

    download_step = next(
        step for step in steps if step.get("name") == "Install verified Godot files"
    )
    assert download_step["if"] == "steps.godot-cache.outputs.cache-hit != 'true'"
    assert save_step["if"] == "steps.godot-cache.outputs.cache-hit != 'true'"
    assert steps.index(save_step) == steps.index(download_step) + 1

    script = download_step["run"]
    assert "github.com/godotengine/godot-builds/releases/download/" in script
    checksum_lines = [
        line for line in script.splitlines() if "sha256sum --check" in line
    ]
    assert any(
        "GODOT_EDITOR_SHA256" in line and "editor_archive" in line
        for line in checksum_lines
    )
    assert any(
        "GODOT_TEMPLATES_SHA256" in line and "templates_archive" in line
        for line in checksum_lines
    )
    assert "templates/web_dlink_debug.zip" in script
    assert "templates/web_dlink_nothreads_debug.zip" in script
    assert "GODOT_EDITOR_VERSION" in script
    assert 'unzip -tq "${template_dir}/web_dlink_debug.zip"' in script
    assert 'unzip -tq "${template_dir}/web_dlink_nothreads_debug.zip"' in script


def test_godot_browser_job_lints_runs_and_preserves_failures() -> None:
    """Static checks must precede the two-mode browser runner with diagnostics."""
    workflow = _load_ci_rust_workflow()
    steps = workflow["jobs"]["godot-emscripten"]["steps"]
    step_names = [step.get("name") for step in steps]

    fmt_step = next(
        step for step in steps if step.get("name") == "Check Godot fixture formatting"
    )
    fmt_command = _shell_commands(fmt_step)
    assert len(fmt_command) == 1
    assert fmt_command[0][:3] == ["cargo", "+nightly-2026-07-08", "fmt"]
    assert _option_values(fmt_command[0], "--manifest-path") == [
        f"{GODOT_FIXTURE}/Cargo.toml"
    ]
    assert "--check" in fmt_command[0]

    clippy_step = next(
        step for step in steps if step.get("name") == "Lint Godot fixture"
    )
    clippy_command = _shell_commands(clippy_step)
    assert len(clippy_command) == 1
    assert clippy_command[0][:3] == ["cargo", "+nightly-2026-07-08", "clippy"]
    assert _option_values(clippy_command[0], "--manifest-path") == [
        f"{GODOT_FIXTURE}/Cargo.toml"
    ]
    assert "--all-targets" in clippy_command[0]
    assert "--all-features" in clippy_command[0]
    assert "--locked" in clippy_command[0]
    assert clippy_command[0][-2:] == ["-D", "warnings"]

    rust_index = step_names.index("Install Godot Rust toolchain")
    fmt_index = step_names.index(fmt_step["name"])
    clippy_index = step_names.index(clippy_step["name"])
    assert fmt_index == rust_index + 1
    assert clippy_index == fmt_index + 1
    for expensive_step in (
        "Install Emscripten SDK",
        "Install Node.js",
        "Restore Godot runtime files",
        "Install Xvfb authentication",
    ):
        assert clippy_index < step_names.index(expensive_step)

    xauth_step = next(
        step for step in steps if step.get("name") == "Install Xvfb authentication"
    )
    assert any("xauth" in command for command in _shell_commands(xauth_step))

    run_step = next(
        step for step in steps if step.get("name") == "Run Godot browser fixture"
    )
    assert run_step["run"] == f"./{GODOT_FIXTURE}/run.sh"
    assert str(run_step["env"]["INSTALL_PLAYWRIGHT"]) == "1"
    assert clippy_index < step_names.index(run_step["name"])

    artifact_step = next(
        step for step in steps if step.get("name") == "Upload browser failure results"
    )
    assert artifact_step["if"] == "failure()"
    assert artifact_step["uses"] == "actions/upload-artifact@v7"
    assert artifact_step["with"]["path"] == f"{GODOT_FIXTURE}/test-results/"


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
