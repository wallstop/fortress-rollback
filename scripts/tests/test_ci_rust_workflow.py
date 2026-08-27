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
CI_NETWORK_WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ci-network.yml"
CI_COVERAGE_WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ci-coverage.yml"
CI_SAFETY_WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ci-safety.yml"
CARGO_LOCK = REPO_ROOT / "Cargo.lock"
CARGO_CONFIG = REPO_ROOT / ".cargo" / "config.toml"
WASM_BROWSER_SMOKE_SOURCE = REPO_ROOT / "tests" / "wasm-browser-smoke" / "src" / "lib.rs"
NETWORK_DOCKERFILE = REPO_ROOT / "docker" / "Dockerfile"
NETWORK_DOCKERIGNORE = REPO_ROOT / ".dockerignore"
EMSCRIPTEN_DEPENDENCY_CHECK = "scripts/ci/check-emscripten-dependencies.sh"
EMSCRIPTEN_DEPENDENCY_CHECK_PATH = REPO_ROOT / EMSCRIPTEN_DEPENDENCY_CHECK
GODOT_FIXTURE = "tests/godot-emscripten"
GODOT_FIXTURE_PATH_FILTER = f"{GODOT_FIXTURE}/**"
GODOT_EDITOR_SHA256 = (
    "c7ff14fd28472c8d4f193043de30278dcf7e5241a1dcf7566b02e27addaa33ba"
)
GODOT_TEMPLATES_SHA256 = (
    "86409db6200b6f8fd3230989c2d2002851f3dd18acf11d7bdbafddf5a0dd0f72"
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
    CI_COVERAGE_WORKFLOW,
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


def _locked_package_version(package_name: str) -> str:
    """Return the one locked version for a schema-coupled package."""
    if tomllib is None:
        pytest.skip("tomllib/tomli not available")
    lockfile = tomllib.loads(CARGO_LOCK.read_text(encoding="utf-8"))
    versions = {
        package["version"]
        for package in lockfile["package"]
        if package["name"] == package_name
    }
    assert len(versions) == 1, (
        f"expected exactly one {package_name!r} version in Cargo.lock, got {versions}"
    )
    return versions.pop()


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


def test_network_docker_build_copies_complete_context_and_fails_closed() -> None:
    """Cargo target declarations and sources must enter the image atomically."""
    dockerfile = NETWORK_DOCKERFILE.read_text(encoding="utf-8")
    build_context, separator, build_tail = dockerfile.partition("RUN cargo build")
    assert separator, "Dockerfile is missing its fail-closed Cargo build"
    assert _docker_copy_sources(build_context) == (Path("."),)
    assert dockerfile.count("RUN cargo build") == 1
    assert "prepare-cargo-cache" not in dockerfile
    assert "RUN touch" not in dockerfile
    assert ".cargo/" in NETWORK_DOCKERIGNORE.read_text(encoding="utf-8").splitlines()

    build = ["cargo", "build", *shlex.split(build_tail.splitlines()[0])]
    assert build == [
        "cargo",
        "build",
        "--locked",
        "--release",
        "-p",
        "network-test-peer",
    ]

    workflow_path = REPO_ROOT / ".github" / "workflows" / "ci-network.yml"
    workflow = _load_workflow(workflow_path)
    for event in ("push", "pull_request"):
        assert ".dockerignore" in _workflow_paths(workflow, event)


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


def test_public_api_census_uses_pinned_nightly_and_checked_snapshot() -> None:
    """Callable hidden paths must be rebuilt by the canonical nightly gate."""
    workflow = _load_ci_rust_workflow()
    job = workflow["jobs"]["public-api-census"]
    assert job["runs-on"] == "ubuntu-latest"
    assert job["timeout-minutes"] == 10

    steps = job["steps"]
    nightly = next(
        step for step in steps if step.get("name") == "Install pinned Rust nightly"
    )
    assert nightly["id"] == "nightly"
    assert nightly["uses"] == "./.github/actions/install-pinned-nightly"

    cache = next(step for step in steps if step.get("name") == "Setup Rust cache")
    assert "steps.nightly.outputs.toolchain" in cache["with"]["cache-key"]
    assert "target/public-api-census" in cache["with"]["cache-paths"].splitlines()

    census = next(
        step for step in steps if step.get("name") == "Verify checked public API snapshot"
    )
    assert census["run"] == "python3 scripts/api/public_api_census.py --check"

    expected_paths = {
        "scripts/api/public_api_census.py",
        "scripts/tests/test_public_api_census.py",
        "docs/api/public-api-*.tsv",
    }
    for event in ("push", "pull_request"):
        assert expected_paths <= _workflow_paths(workflow, event)


def test_no_panics_gate_rejects_debug_assertions_and_scan_failures() -> None:
    """Debug-only assertions and failed source scans must both block CI."""
    workflow = _load_workflow(CI_SAFETY_WORKFLOW)
    steps = workflow["jobs"]["no-panics"]["steps"]
    check = next(
        step
        for step in steps
        if step.get("name") == "Check for panic-prone patterns (library only)"
    )
    run = check["run"]

    assert "debug_assert(_eq|_ne)?[[:space:]]*![[:space:]]*\\(" in run
    assert "grep_status=$?" in run
    assert 'if [ "$grep_status" -ne 1 ]' in run
    assert 'exit "$grep_status"' in run


@pytest.mark.parametrize("workflow_path", CARGO_HEAVY_WORKFLOWS_WITH_PATH_FILTERS)
@pytest.mark.parametrize("event", ("push", "pull_request"))
def test_cargo_heavy_workflows_trigger_on_cargo_config_changes(
    workflow_path: Path, event: str
) -> None:
    """Cargo-heavy CI must run when repo-level Cargo config changes."""
    workflow = _load_workflow(workflow_path)

    assert ".cargo/config.toml" in _workflow_paths(workflow, event)
    assert ".cargo/**" not in _workflow_paths(workflow, event)


def test_coverage_allows_instrumented_large_mesh_controls_to_respond() -> None:
    """Tarpaulin must tolerate deterministic N=16 controls under instrumentation."""
    workflow = _load_workflow(CI_COVERAGE_WORKFLOW)
    steps = workflow["jobs"]["coverage"]["steps"]
    coverage_step = next(
        step for step in steps if step.get("name") == "Run coverage (tarpaulin)"
    )
    commands = _shell_commands(coverage_step)

    assert len(commands) == 1
    command = commands[0]
    assert command[:2] == ["cargo", "tarpaulin"]
    timeouts = _option_values(command, "--timeout")
    assert len(timeouts) == 1
    assert int(timeouts[0]) >= 300


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

    check_commands = [
        command
        for command in _cargo_commands(steps, "check")
        if "--no-default-features" in command
    ]
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


def test_hot_join_clippy_covers_supported_feature_composition() -> None:
    """User-facing additive features must compile together across all targets."""
    workflow = _load_ci_rust_workflow()
    steps = workflow["jobs"]["hot-join"]["steps"]
    clippy_step = next(
        step for step in steps if step.get("name") == "Run hot-join clippy"
    )
    commands = _shell_commands(clippy_step)

    assert len(commands) == 1
    assert "--workspace" in commands[0]
    assert "--all-targets" in commands[0]
    assert _feature_set(commands[0]) == frozenset(
        {
            "hot-join",
            "tokio",
            "json",
            "sync-send",
            "paranoid",
            "trace-validation",
        }
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


def test_wasm_job_runs_browser_session_transport_in_browser() -> None:
    """The browser row must execute bounded session traffic with matching tools."""
    workflow = _load_ci_rust_workflow()
    steps = workflow["jobs"]["wasm-check"]["steps"]

    install_step = next(
        step
        for step in steps
        if step.get("name") == "Install wasm-bindgen CLI for browser runtime tests"
    )
    assert install_step["if"] == "matrix.target == 'wasm32-unknown-unknown'"
    assert install_step["uses"] == (
        "taiki-e/install-action@5b4d68e2e660441203ab128a23676f1e4faf1532"
    )
    locked_bindgen = _locked_package_version("wasm-bindgen")
    assert install_step["with"]["tool"] == f"wasm-bindgen-cli@{locked_bindgen}"

    test_step = next(
        step
        for step in steps
        if step.get("name") == "Run browser WASM session runtime test"
    )
    assert test_step["if"] == "matrix.target == 'wasm32-unknown-unknown'"
    assert test_step["id"] == "browser_session_runtime"
    assert test_step["timeout-minutes"] == 5
    assert (
        test_step["env"]["CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER"]
        == "wasm-bindgen-test-runner"
    )
    assert test_step["env"]["WASM_BINDGEN_TEST_ONLY_WEB"] == "1"
    browser_source = WASM_BROWSER_SMOKE_SOURCE.read_text(encoding="utf-8")
    assert "wasm_bindgen_test_configure!(run_in_browser);" in browser_source
    assert 'export CHROMEDRIVER="${CHROMEWEBDRIVER}/chromedriver"' in test_step["run"]
    assert "google-chrome --version" in test_step["run"]
    assert '"${CHROMEDRIVER}" --version' in test_step["run"]

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
    assert "set -euo pipefail" in test_step["run"]
    assert "tee -a test-results/wasm-browser-runtime.log" in test_step["run"]

    upload_step = next(
        step
        for step in steps
        if step.get("name") == "Upload browser runtime failure diagnostics"
    )
    assert upload_step["if"] == (
        "failure() && steps.browser_session_runtime.outcome == 'failure' && "
        "matrix.target == 'wasm32-unknown-unknown'"
    )
    assert upload_step["uses"] == "actions/upload-artifact@v7"
    assert upload_step["with"]["path"] == "test-results/wasm-browser-runtime.log"
    assert upload_step["with"]["retention-days"] == 7


def test_network_job_runs_tokio_owned_sessions_on_every_native_os() -> None:
    """Tokio session ownership must have a bounded, diagnostic runtime oracle."""
    workflow = _load_workflow(CI_NETWORK_WORKFLOW)
    job = workflow["jobs"]["network-tests"]
    assert Counter(job["strategy"]["matrix"]["os"]) == Counter(
        ("ubuntu-latest", "windows-latest", "macos-latest")
    )

    steps = job["steps"]
    test_step = next(
        step
        for step in steps
        if step.get("name") == "Run Tokio-owned session runtime test"
    )
    assert test_step["id"] == "tokio_session_runtime"
    assert test_step["timeout-minutes"] == 10
    assert test_step["shell"] == "bash"
    assert "set -euo pipefail" in test_step["run"]
    assert 'tee "test-results/tokio-session-${{ runner.os }}.log"' in test_step["run"]

    commands = _cargo_commands([test_step], "test")
    assert len(commands) == 1
    command = commands[0]
    assert _option_values(command, "--features") == ["tokio"]
    assert _option_values(command, "--test") == ["tokio_session"]
    assert command.count("--locked") == 1
    test_args = command[command.index("--") + 1 :]
    assert "--nocapture" in test_args

    upload_step = next(
        step
        for step in steps
        if step.get("name") == "Upload Tokio session failure diagnostics"
    )
    assert upload_step["if"] == (
        "failure() && steps.tokio_session_runtime.outcome == 'failure'"
    )
    assert upload_step["uses"] == "actions/upload-artifact@v7"
    assert upload_step["with"]["path"] == (
        "test-results/tokio-session-${{ runner.os }}.log"
    )
    assert upload_step["with"]["retention-days"] == 7


def test_wasm_job_checks_documented_custom_socket_example() -> None:
    """Browser transport guidance must compile for its documented target."""
    workflow = _load_ci_rust_workflow()
    steps = workflow["jobs"]["wasm-check"]["steps"]
    check_step = next(
        step
        for step in steps
        if step.get("name") == "Check documented browser custom-socket example"
    )

    assert check_step["if"] == "matrix.target == 'wasm32-unknown-unknown'"
    assert check_step["run"] == (
        "cargo check --locked --target ${{ matrix.target }} --example custom_socket"
    )


def test_build_job_runs_error_handling_example_contract() -> None:
    """Structured-error example drift must fail ordinary Rust CI."""
    workflow = _load_ci_rust_workflow()
    steps = workflow["jobs"]["build"]["steps"]
    run_step = next(
        step
        for step in steps
        if step.get("name") == "Run error-handling example contract"
    )

    assert run_step["if"] == "runner.os == 'Linux'"
    assert run_step["run"] == "cargo run --locked --example error_handling"


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
        == "dtolnay/rust-toolchain@6c977a6ca4077a0ceb28ffbe03f59d46e9ac8772"
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
    assert str(emsdk_step["with"]["version"]) == "4.0.20"
    assert emsdk_step["with"]["actions-cache-folder"] == "emsdk-cache"

    node_step = next(step for step in steps if step.get("name") == "Install Node.js")
    assert node_step["uses"] == "actions/setup-node@v7"
    assert str(node_step["with"]["node-version"]) == "24"
    assert node_step["with"]["package-manager-cache"] is False


def test_godot_download_cache_contains_only_verified_runtime_files() -> None:
    """Cache the three runtime inputs, never the template or browser archives."""
    workflow = _load_ci_rust_workflow()
    job = workflow["jobs"]["godot-emscripten"]
    steps = job["steps"]

    assert job["env"]["GODOT_VERSION"] == "4.7.1"
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
