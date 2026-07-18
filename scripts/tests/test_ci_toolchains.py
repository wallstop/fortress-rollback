#!/usr/bin/env python3
"""Regression tests for reproducible nightly toolchains in required CI."""

from __future__ import annotations

import os
import re
import subprocess
from fnmatch import fnmatchcase
from pathlib import Path

import pytest
import yaml

REPO_ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = REPO_ROOT / ".github" / "workflows"
ACTION = REPO_ROOT / ".github" / "actions" / "install-pinned-nightly" / "action.yml"
INSTALLER = (
    REPO_ROOT / ".github" / "actions" / "install-pinned-nightly" / "install.sh"
)
CHECK_TOOLS = REPO_ROOT / "scripts" / "ci" / "check-tools.sh"
PIN = REPO_ROOT / ".github" / "actions" / "install-pinned-nightly" / "toolchain"
MIRI_PIN = (
    REPO_ROOT
    / ".github"
    / "actions"
    / "install-pinned-nightly"
    / "miri-toolchain"
)
RELEASE_ACTION = (
    REPO_ROOT / ".github" / "actions" / "install-pinned-release" / "action.yml"
)
RELEASE_INSTALLER = RELEASE_ACTION.parent / "install.sh"
RELEASE_PIN = RELEASE_ACTION.parent / "toolchain"
LOCAL_INSTALLER = "./.github/actions/install-pinned-nightly"
LOCAL_RELEASE_INSTALLER = "./.github/actions/install-pinned-release"
REQUIRED_NIGHTLY_JOBS = (
    ("ci-safety.yml", "careful", "rust-src", ""),
    ("ci-security.yml", "unused-deps", "", ""),
    ("ci-docs.yml", "doc-coverage", "", ""),
    ("ci-verification.yml", "kani-tier1", "", ""),
    ("ci-verification.yml", "kani-tier2", "", ""),
    ("ci-verification.yml", "kani-tier3", "", ""),
)
DIRECT_SPECIAL_PURPOSE_NIGHTLY_JOBS = {("ci-rust.yml", "godot-emscripten")}
DATED_NIGHTLY = re.compile(
    r"nightly-\d{4}-\d{2}-\d{2}(?:-[A-Za-z0-9._-]+)?"
)
NIGHTLY_TOKEN = re.compile(
    r"(?<![A-Za-z0-9_.-])(nightly(?:-[A-Za-z0-9_.-]+)?)(?![A-Za-z0-9_.-])"
)


def _load_yaml(path: Path) -> dict:
    return yaml.safe_load(path.read_text(encoding="utf-8"))


def _run_release_installer(
    tmp_path: Path, pin_contents: str, succeed_on_attempt: int
) -> tuple[subprocess.CompletedProcess[str], Path, Path, Path, Path]:
    action_path = tmp_path / "release-action"
    action_path.mkdir()
    (action_path / "toolchain").write_text(pin_contents, encoding="utf-8")

    fake_bin = tmp_path / "bin"
    fake_bin.mkdir()
    rustup_log = tmp_path / "rustup.log"
    attempt_count = tmp_path / "attempt-count"
    github_env = tmp_path / "github-env"
    github_output = tmp_path / "github-output"
    bash_env = tmp_path / "bash-env"
    bash_env.write_text(
        """mapfile() { printf '%s\\n' 'forbidden mapfile' >&2; return 97; }
readarray() { printf '%s\\n' 'forbidden readarray' >&2; return 97; }
export -f mapfile readarray
""",
        encoding="utf-8",
    )

    fake_rustup = fake_bin / "rustup"
    fake_rustup.write_text(
        """#!/usr/bin/env bash
set -euo pipefail
printf '%s\\n' "$*" >> "$FAKE_RUSTUP_LOG"
if [ "${1:-}" = "toolchain" ] && [ "${2:-}" = "install" ]; then
  count=0
  if [ -f "$FAKE_ATTEMPT_COUNT" ]; then
    read -r count < "$FAKE_ATTEMPT_COUNT"
  fi
  count=$((count + 1))
  printf '%s\\n' "$count" > "$FAKE_ATTEMPT_COUNT"
  if [ "$FAKE_SUCCEED_ON" -eq 0 ] || [ "$count" -lt "$FAKE_SUCCEED_ON" ]; then
    exit 1
  fi
fi
""",
        encoding="utf-8",
    )
    fake_rustup.chmod(0o755)
    fake_sleep = fake_bin / "sleep"
    fake_sleep.write_text("#!/usr/bin/env bash\nexit 0\n", encoding="utf-8")
    fake_sleep.chmod(0o755)

    env = os.environ.copy()
    env.update(
        {
            "PATH": f"{fake_bin}:{env['PATH']}",
            "GITHUB_ACTION_PATH": str(action_path),
            "GITHUB_ENV": str(github_env),
            "GITHUB_OUTPUT": str(github_output),
            "BASH_COMPAT": "3.2",
            "BASH_ENV": str(bash_env),
            "FAKE_RUSTUP_LOG": str(rustup_log),
            "FAKE_ATTEMPT_COUNT": str(attempt_count),
            "FAKE_SUCCEED_ON": str(succeed_on_attempt),
        }
    )
    result = subprocess.run(
        ["bash", str(RELEASE_INSTALLER)],
        cwd=REPO_ROOT,
        env=env,
        check=False,
        capture_output=True,
        text=True,
    )
    return result, rustup_log, attempt_count, github_env, github_output


def _is_floating_nightly(value: object) -> bool:
    candidate = str(value).strip().strip("'\"")
    return bool(
        re.fullmatch(r"nightly(?:-[A-Za-z0-9_.-]+)?", candidate)
        and not DATED_NIGHTLY.fullmatch(candidate)
    )


def _floating_env_issues(env: object, scope: str) -> list[str]:
    if not isinstance(env, dict):
        return []
    value = env.get("RUSTUP_TOOLCHAIN", "")
    if _is_floating_nightly(value):
        return [f"{scope} sets floating RUSTUP_TOOLCHAIN={value}"]
    return []


def _floating_run_issues(run: str, label: str) -> list[str]:
    issues: list[str] = []

    for match in re.finditer(
        r"\bRUSTUP_TOOLCHAIN\s*=\s*['\"]?"
        r"(nightly(?:-[A-Za-z0-9_.-]+)?)",
        run,
    ):
        if _is_floating_nightly(match.group(1)):
            issues.append(f"{label} sets an inline floating RUSTUP_TOOLCHAIN")

    for line in run.splitlines():
        if re.search(r"\brustup\b", line):
            for match in NIGHTLY_TOKEN.finditer(line):
                if _is_floating_nightly(match.group(1)):
                    issues.append(f"{label} invokes rustup with a floating nightly")
                    break
        if any(
            _is_floating_nightly(match.group(1))
            for match in re.finditer(
                r"\+(nightly(?:-[A-Za-z0-9_.-]+)?)(?![A-Za-z0-9_.-])", line
            )
        ):
            issues.append(f"{label} invokes a floating +nightly")

    return issues


def _floating_nightly_uses(
    job: dict, workflow_env: object | None = None
) -> list[str]:
    issues: list[str] = []
    issues.extend(_floating_env_issues(workflow_env, "workflow env"))
    issues.extend(_floating_env_issues(job.get("env", {}), "job env"))
    for step in job.get("steps", []):
        uses = str(step.get("uses", ""))
        with_block = step.get("with", {})
        toolchain = str(with_block.get("toolchain", ""))
        channel = str(with_block.get("channel", ""))
        run = str(step.get("run", ""))
        action_ref = uses.rpartition("@")[2].split("/", maxsplit=1)[0]
        if _is_floating_nightly(action_ref):
            issues.append(f"{step.get('name', uses)} uses a floating action ref")
        for field_name, field_value in (
            ("toolchain", toolchain),
            ("channel", channel),
        ):
            if _is_floating_nightly(field_value):
                issues.append(
                    f"{step.get('name', uses)} sets floating {field_name}={field_value}"
                )
        label = str(step.get("name", uses or "<run>"))
        issues.extend(_floating_env_issues(step.get("env", {}), f"{label} env"))
        issues.extend(_floating_run_issues(run, label))
    return issues


@pytest.mark.parametrize(
    "step",
    (
        {"uses": "example/toolchain@nightly"},
        {"with": {"toolchain": "nightly"}},
        {"with": {"toolchain": "nightly-x86_64-unknown-linux-gnu"}},
        {"with": {"channel": "nightly"}},
        {"env": {"RUSTUP_TOOLCHAIN": "nightly"}},
        {"run": "cargo +nightly test"},
        {"run": "rustup toolchain install nightly --profile minimal"},
        {"run": "rustup toolchain update nightly"},
        {"run": "rustup update nightly-x86_64-unknown-linux-gnu"},
        {"run": "rustup default nightly-x86_64-unknown-linux-gnu"},
        {"run": "rustup override set nightly"},
        {"run": "rustup run nightly rustc --version"},
        {"run": "rustup +nightly component list"},
        {"run": "rustup component add rust-src --toolchain nightly"},
        {"run": "rustup component add --toolchain=nightly rust-src"},
        {"run": "rustup target add --toolchain nightly wasm32-unknown-unknown"},
        {"run": "RUSTUP_TOOLCHAIN=nightly cargo test"},
        {"run": "env RUSTUP_TOOLCHAIN='nightly-x86_64-unknown-linux-gnu' cargo test"},
        {"run": "export RUSTUP_TOOLCHAIN=nightly; cargo test"},
    ),
)
def test_floating_nightly_detector_rejects_all_supported_forms(step: dict) -> None:
    assert _floating_nightly_uses({"steps": [step]})


@pytest.mark.parametrize(
    ("job", "workflow_env"),
    (
        ({"steps": [], "env": {"RUSTUP_TOOLCHAIN": "nightly"}}, None),
        ({"steps": [{"env": {"RUSTUP_TOOLCHAIN": "nightly"}}]}, None),
        ({"steps": []}, {"RUSTUP_TOOLCHAIN": "nightly"}),
    ),
)
def test_floating_nightly_detector_rejects_env_at_every_scope(
    job: dict, workflow_env: object | None
) -> None:
    assert _floating_nightly_uses(job, workflow_env)


def test_floating_nightly_detector_accepts_dated_forms() -> None:
    dated = "nightly-2026-07-08"
    job = {
        "env": {"RUSTUP_TOOLCHAIN": dated},
        "steps": [
            {"uses": f"example/toolchain@{dated}"},
            {"with": {"toolchain": dated}},
            {"with": {"channel": dated}},
            {"with": {"toolchain": f"{dated}-x86_64-unknown-linux-gnu"}},
            {"env": {"RUSTUP_TOOLCHAIN": dated}},
            {"run": f"cargo +{dated} test"},
            {"run": f"rustup toolchain install {dated} --profile minimal"},
            {"run": f"rustup update {dated}"},
            {"run": f"rustup component add --toolchain {dated} rust-src"},
            {"run": f"rustup run {dated} rustc --version"},
            {"run": f"RUSTUP_TOOLCHAIN={dated} cargo test"},
        ]
    }

    assert _floating_nightly_uses(job, {"RUSTUP_TOOLCHAIN": dated}) == []


def test_generic_nightly_pin_is_dated_and_has_one_source_of_truth() -> None:
    pin = PIN.read_text(encoding="utf-8").strip()

    assert re.fullmatch(r"nightly-\d{4}-\d{2}-\d{2}", pin)
    for workflow_name, *_ in REQUIRED_NIGHTLY_JOBS:
        assert pin not in (WORKFLOWS / workflow_name).read_text(encoding="utf-8")
    assert pin not in ACTION.read_text(encoding="utf-8")


def test_miri_pin_is_dated_and_has_one_source_of_truth() -> None:
    pin = MIRI_PIN.read_text(encoding="utf-8").strip()
    workflow_text = (WORKFLOWS / "ci-rust.yml").read_text(encoding="utf-8")

    assert re.fullmatch(r"nightly-\d{4}-\d{2}-\d{2}", pin)
    assert pin not in workflow_text
    assert "pin-file: miri-toolchain" in workflow_text


def test_release_toolchain_pin_is_exact_u64_semver_and_single_source() -> None:
    pin = RELEASE_PIN.read_text(encoding="utf-8").strip()
    match = re.fullmatch(r"(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)", pin)

    assert match is not None
    assert all(int(component) <= (2**64 - 1) for component in match.groups())
    assert pin not in RELEASE_ACTION.read_text(encoding="utf-8")
    assert pin not in RELEASE_INSTALLER.read_text(encoding="utf-8")
    assert sorted(RELEASE_ACTION.parent.glob("*toolchain*")) == [RELEASE_PIN]


def test_pinned_release_action_defines_fixed_local_installer_contract() -> None:
    action = _load_yaml(RELEASE_ACTION)
    action_script = "\n".join(
        str(step.get("run", "")) for step in action["runs"]["steps"]
    )
    installer = RELEASE_INSTALLER.read_text(encoding="utf-8")

    assert "inputs" not in action
    assert action["outputs"]["toolchain"]["value"] == (
        "${{ steps.install.outputs.toolchain }}"
    )
    assert 'bash "${GITHUB_ACTION_PATH}/install.sh"' in action_script
    assert "for attempt in 1 2 3" in installer
    assert "rustup toolchain install" in installer
    assert "--profile minimal --no-self-update" in installer
    assert "rustc --version --verbose" in installer
    assert "cargo --version --verbose" in installer
    assert "mapfile" not in installer
    assert "readarray" not in installer
    assert "declare -A" not in installer
    assert LOCAL_RELEASE_INSTALLER == "./.github/actions/install-pinned-release"


def test_pinned_release_installer_succeeds_and_exports_exact_toolchain(
    tmp_path: Path,
) -> None:
    pin = RELEASE_PIN.read_text(encoding="utf-8").strip()
    result, rustup_log, attempt_count, github_env, github_output = (
        _run_release_installer(tmp_path, f"{pin}\n", succeed_on_attempt=1)
    )

    assert result.returncode == 0, result.stderr
    assert attempt_count.read_text(encoding="utf-8").strip() == "1"
    assert f"toolchain install {pin} --profile minimal --no-self-update" in (
        rustup_log.read_text(encoding="utf-8").splitlines()
    )
    assert github_env.read_text(encoding="utf-8").strip() == (
        f"RUSTUP_TOOLCHAIN={pin}"
    )
    assert github_output.read_text(encoding="utf-8").strip() == (
        f"toolchain={pin}"
    )


def test_pinned_release_installer_retries_transient_failures(
    tmp_path: Path,
) -> None:
    pin = RELEASE_PIN.read_text(encoding="utf-8").strip()
    result, rustup_log, attempt_count, github_env, github_output = (
        _run_release_installer(tmp_path, f"{pin}\n", succeed_on_attempt=3)
    )

    install_call = f"toolchain install {pin} --profile minimal --no-self-update"
    install_calls = [
        line
        for line in rustup_log.read_text(encoding="utf-8").splitlines()
        if line.startswith("toolchain install ")
    ]
    assert result.returncode == 0, result.stderr
    assert attempt_count.read_text(encoding="utf-8").strip() == "3"
    assert install_calls == [install_call] * 3
    assert github_env.exists()
    assert github_output.exists()


def test_pinned_release_installer_exhausts_bounded_retries(
    tmp_path: Path,
) -> None:
    pin = RELEASE_PIN.read_text(encoding="utf-8").strip()
    result, rustup_log, attempt_count, github_env, github_output = (
        _run_release_installer(tmp_path, f"{pin}\n", succeed_on_attempt=0)
    )

    install_calls = [
        line
        for line in rustup_log.read_text(encoding="utf-8").splitlines()
        if line.startswith("toolchain install ")
    ]
    assert result.returncode != 0
    assert attempt_count.read_text(encoding="utf-8").strip() == "3"
    assert len(install_calls) == 3
    assert "Failed to install" in result.stdout
    assert not github_env.exists()
    assert not github_output.exists()


@pytest.mark.parametrize(
    ("pin_contents", "expected_error"),
    (
        ("", "exactly one line"),
        ("1.96.1\n1.96.2\n", "exactly one line"),
        ("stable\n", "exactly stable X.Y.Z"),
        ("1.96\n", "exactly stable X.Y.Z"),
        ("1.96.1-beta.1\n", "exactly stable X.Y.Z"),
        ("01.96.1\n", "canonical u64 values"),
        ("18446744073709551616.0.0\n", "canonical u64 values"),
        ("0.18446744073709551616.0\n", "canonical u64 values"),
        ("0.0.18446744073709551616\n", "canonical u64 values"),
    ),
)
def test_pinned_release_installer_rejects_malformed_or_unbounded_pin(
    tmp_path: Path, pin_contents: str, expected_error: str
) -> None:
    result, rustup_log, _attempt_count, github_env, github_output = (
        _run_release_installer(tmp_path, pin_contents, succeed_on_attempt=1)
    )

    assert result.returncode != 0
    assert expected_error in result.stdout
    assert not rustup_log.exists()
    assert not github_env.exists()
    assert not github_output.exists()


def test_pinned_release_installer_accepts_u64_component_boundary(
    tmp_path: Path,
) -> None:
    boundary = "18446744073709551615.0.0"
    result, rustup_log, attempt_count, github_env, github_output = (
        _run_release_installer(tmp_path, f"{boundary}\n", succeed_on_attempt=1)
    )

    assert result.returncode == 0, result.stderr
    assert rustup_log.exists()
    assert attempt_count.read_text(encoding="utf-8").strip() == "1"
    assert github_env.read_text(encoding="utf-8").strip() == (
        f"RUSTUP_TOOLCHAIN={boundary}"
    )
    assert github_output.read_text(encoding="utf-8").strip() == (
        f"toolchain={boundary}"
    )


def test_tool_check_reads_exact_pins_from_repo_root_without_dynamic_eval() -> None:
    script = CHECK_TOOLS.read_text(encoding="utf-8")

    assert 'REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"' in script
    assert 'PIN_DIRECTORY="$REPO_ROOT/.github/actions/install-pinned-nightly"' in script
    assert 'read_dated_nightly_pin "$PIN_DIRECTORY/toolchain"' in script
    assert 'read_dated_nightly_pin "$PIN_DIRECTORY/miri-toolchain"' in script
    assert "^nightly-[0-9]{4}-[0-9]{2}-[0-9]{2}$" in script
    assert 'rustup run "$PINNED_NIGHTLY_TOOLCHAIN" rustc --version' in script
    assert 'rustup run "$PINNED_MIRI_TOOLCHAIN" cargo miri --version' in script
    assert 'eval "rustup run' not in script
    assert "rustup +nightly" not in script


def test_pinned_nightly_installer_retries_and_reports_diagnostics() -> None:
    action = _load_yaml(ACTION)
    inputs = action["inputs"]
    action_script = "\n".join(
        str(step.get("run", "")) for step in action["runs"]["steps"]
    )
    installer = INSTALLER.read_text(encoding="utf-8")

    assert inputs["components"]["default"] == ""
    assert inputs["targets"]["default"] == ""
    assert inputs["pin-file"]["default"] == "toolchain"
    assert 'bash "${GITHUB_ACTION_PATH}/install.sh"' in action_script
    assert "for attempt in 1 2 3" in installer
    assert "rustup toolchain install" in installer
    assert "--profile minimal" in installer
    assert "--no-self-update" in installer
    assert "rustc --version --verbose" in installer
    assert "cargo --version --verbose" in installer
    assert "rustup component list" in installer
    assert "rustup target list" in installer
    assert "mapfile" not in installer
    assert "readarray" not in installer
    assert "declare -A" not in installer


@pytest.mark.parametrize(
    ("pin_contents", "components", "targets", "expected_error"),
    (
        ("nightly\nnightly-2026-07-08\n", "", "", "exactly one line"),
        ("nightly\n", "", "", "exactly nightly-YYYY-MM-DD"),
        ("nightly-2026-07-08\n", "rust src", "", "Invalid rustup component"),
        ("nightly-2026-07-08\n", "", "wasm32;uname", "Invalid rustup target"),
    ),
)
def test_pinned_nightly_installer_rejects_invalid_input_before_rustup(
    tmp_path: Path,
    pin_contents: str,
    components: str,
    targets: str,
    expected_error: str,
) -> None:
    action_path = tmp_path / "action"
    action_path.mkdir()
    (action_path / "toolchain").write_text(pin_contents, encoding="utf-8")
    github_env = tmp_path / "github-env"
    github_output = tmp_path / "github-output"
    env = os.environ.copy()
    env.update(
        {
            "GITHUB_ACTION_PATH": str(action_path),
            "GITHUB_ENV": str(github_env),
            "GITHUB_OUTPUT": str(github_output),
            "PIN_FILE_NAME": "toolchain",
            "REQUESTED_COMPONENTS": components,
            "REQUESTED_TARGETS": targets,
        }
    )

    result = subprocess.run(
        ["bash", str(INSTALLER)],
        cwd=REPO_ROOT,
        env=env,
        check=False,
        capture_output=True,
        text=True,
    )

    assert result.returncode != 0
    assert expected_error in result.stdout
    assert not github_env.exists()
    assert not github_output.exists()


def test_pinned_nightly_installer_runs_under_bash_3_2_contract_and_retries(
    tmp_path: Path,
) -> None:
    pin = PIN.read_text(encoding="utf-8").strip()
    fake_bin = tmp_path / "bin"
    fake_bin.mkdir()
    rustup_log = tmp_path / "rustup.log"
    attempt_count = tmp_path / "attempt-count"
    github_env = tmp_path / "github-env"
    github_output = tmp_path / "github-output"
    bash_env = tmp_path / "bash-env"

    # Make execution fail if a Bash-4-only bulk line reader is reintroduced.
    # BASH_COMPAT exercises the compatibility behavior available in newer Bash.
    bash_env.write_text(
        """mapfile() { printf '%s\\n' 'forbidden mapfile' >&2; return 97; }
readarray() { printf '%s\\n' 'forbidden readarray' >&2; return 97; }
export -f mapfile readarray
""",
        encoding="utf-8",
    )

    fake_rustup = fake_bin / "rustup"
    fake_rustup.write_text(
        """#!/usr/bin/env bash
set -euo pipefail
printf '%s\\n' "$*" >> "$FAKE_RUSTUP_LOG"
if [ "${1:-}" = "toolchain" ] && [ "${2:-}" = "install" ]; then
  count=0
  if [ -f "$FAKE_ATTEMPT_COUNT" ]; then
    read -r count < "$FAKE_ATTEMPT_COUNT"
  fi
  count=$((count + 1))
  printf '%s\\n' "$count" > "$FAKE_ATTEMPT_COUNT"
  if [ "$count" -lt 3 ]; then
    exit 1
  fi
fi
""",
        encoding="utf-8",
    )
    fake_rustup.chmod(0o755)
    fake_sleep = fake_bin / "sleep"
    fake_sleep.write_text("#!/usr/bin/env bash\nexit 0\n", encoding="utf-8")
    fake_sleep.chmod(0o755)

    env = os.environ.copy()
    env.update(
        {
            "PATH": f"{fake_bin}:{env['PATH']}",
            "GITHUB_ACTION_PATH": str(ACTION.parent),
            "GITHUB_ENV": str(github_env),
            "GITHUB_OUTPUT": str(github_output),
            "PIN_FILE_NAME": "toolchain",
            "REQUESTED_COMPONENTS": " rust-src,clippy ",
            "REQUESTED_TARGETS": "wasm32-unknown-unknown",
            "BASH_COMPAT": "3.2",
            "BASH_ENV": str(bash_env),
            "FAKE_RUSTUP_LOG": str(rustup_log),
            "FAKE_ATTEMPT_COUNT": str(attempt_count),
        }
    )

    result = subprocess.run(
        ["bash", str(INSTALLER)],
        cwd=REPO_ROOT,
        env=env,
        check=False,
        capture_output=True,
        text=True,
    )

    assert result.returncode == 0, result.stderr
    assert attempt_count.read_text(encoding="utf-8").strip() == "3"
    install_calls = [
        line
        for line in rustup_log.read_text(encoding="utf-8").splitlines()
        if line.startswith("toolchain install ")
    ]
    assert install_calls == [
        f"toolchain install {pin} --profile minimal --no-self-update "
        "--component rust-src --component clippy --target wasm32-unknown-unknown"
    ] * 3
    assert github_env.read_text(encoding="utf-8").strip() == (
        f"RUSTUP_TOOLCHAIN={pin}"
    )
    assert github_output.read_text(encoding="utf-8").strip() == (
        f"toolchain={pin}"
    )


def test_pinned_nightly_installer_exhausts_retries_without_writing_outputs(
    tmp_path: Path,
) -> None:
    fake_bin = tmp_path / "bin"
    fake_bin.mkdir()
    rustup_log = tmp_path / "rustup.log"
    github_env = tmp_path / "github-env"
    github_output = tmp_path / "github-output"

    fake_rustup = fake_bin / "rustup"
    fake_rustup.write_text(
        """#!/usr/bin/env bash
set -euo pipefail
printf '%s\\n' "$*" >> "$FAKE_RUSTUP_LOG"
if [ "${1:-}" = "toolchain" ] && [ "${2:-}" = "install" ]; then
  exit 1
fi
""",
        encoding="utf-8",
    )
    fake_rustup.chmod(0o755)
    fake_sleep = fake_bin / "sleep"
    fake_sleep.write_text("#!/usr/bin/env bash\nexit 0\n", encoding="utf-8")
    fake_sleep.chmod(0o755)

    env = os.environ.copy()
    env.update(
        {
            "PATH": f"{fake_bin}:{env['PATH']}",
            "GITHUB_ACTION_PATH": str(ACTION.parent),
            "GITHUB_ENV": str(github_env),
            "GITHUB_OUTPUT": str(github_output),
            "PIN_FILE_NAME": "toolchain",
            "FAKE_RUSTUP_LOG": str(rustup_log),
        }
    )

    result = subprocess.run(
        ["bash", str(INSTALLER)],
        cwd=REPO_ROOT,
        env=env,
        check=False,
        capture_output=True,
        text=True,
    )

    install_calls = [
        line
        for line in rustup_log.read_text(encoding="utf-8").splitlines()
        if line.startswith("toolchain install ")
    ]
    assert result.returncode != 0
    assert len(install_calls) == 3
    assert "Failed to install" in result.stdout
    assert not github_env.exists()
    assert not github_output.exists()


@pytest.mark.parametrize(
    ("workflow_name", "job_name", "components", "targets"),
    REQUIRED_NIGHTLY_JOBS,
)
def test_required_nightly_job_uses_canonical_installer(
    workflow_name: str, job_name: str, components: str, targets: str
) -> None:
    workflow = _load_yaml(WORKFLOWS / workflow_name)
    job = workflow["jobs"][job_name]
    installer_steps = [
        step for step in job["steps"] if step.get("uses") == LOCAL_INSTALLER
    ]

    assert len(installer_steps) == 1
    assert installer_steps[0]["id"] == "nightly"
    installer_inputs = installer_steps[0].get("with", {})
    assert str(installer_inputs.get("components", "")) == components
    assert str(installer_inputs.get("targets", "")) == targets
    assert _floating_nightly_uses(job) == []

    cache_keys = [
        str(step.get("with", {}).get("key", ""))
        for step in job["steps"]
        if step.get("uses", "").startswith("actions/cache@")
    ]
    for cache_key in cache_keys:
        assert "steps.nightly.outputs.toolchain" in cache_key


def test_workflows_have_no_unqualified_nightly() -> None:
    issues = [
        f"{workflow_path.name}/{job_name}: {issue}"
        for workflow_path in sorted(WORKFLOWS.glob("*.y*ml"))
        for workflow in [_load_yaml(workflow_path)]
        for job_name, job in workflow["jobs"].items()
        for issue in _floating_nightly_uses(job, workflow.get("env", {}))
    ]

    assert issues == []


def test_every_canonical_installer_consumer_observes_action_updates() -> None:
    consumers: list[tuple[Path, str, dict]] = [
        (workflow_path, job_name, step)
        for workflow_path in sorted(WORKFLOWS.glob("*.y*ml"))
        for job_name, job in _load_yaml(workflow_path)["jobs"].items()
        for step in job.get("steps", [])
        if step.get("uses") == LOCAL_INSTALLER
    ]

    assert consumers
    for workflow_path, job_name, step in consumers:
        workflow = _load_yaml(workflow_path)
        on_block = workflow.get("on", workflow.get(True))
        assert step.get("id") == "nightly", f"{workflow_path.name}/{job_name}"
        for event in ("push", "pull_request"):
            event_config = on_block.get(event)
            if isinstance(event_config, dict) and "paths" in event_config:
                assert ".github/actions/install-pinned-nightly/**" in event_config[
                    "paths"
                ], f"{workflow_path.name}/{event}"


def test_miri_job_uses_canonical_pin_and_toolchain_scoped_cache() -> None:
    workflow = _load_yaml(WORKFLOWS / "ci-rust.yml")
    job = workflow["jobs"]["miri"]
    installer_steps = [
        step for step in job["steps"] if step.get("uses") == LOCAL_INSTALLER
    ]

    assert len(installer_steps) == 1
    assert installer_steps[0].get("with", {}).get("pin-file") == "miri-toolchain"
    assert installer_steps[0].get("with", {}).get("components") == "miri"
    cache_keys = [
        str(step.get("with", {}).get("key", ""))
        for step in job["steps"]
        if step.get("uses", "").startswith("actions/cache@")
    ]
    assert cache_keys
    assert all("steps.nightly.outputs.toolchain" in key for key in cache_keys)


def test_generic_nightly_pin_is_not_duplicated_by_consumers() -> None:
    pin = PIN.read_text(encoding="utf-8").strip()
    issues: list[str] = []

    for workflow_path in sorted(WORKFLOWS.glob("*.y*ml")):
        for job_name, job in _load_yaml(workflow_path)["jobs"].items():
            if (
                workflow_path.name,
                job_name,
            ) in DIRECT_SPECIAL_PURPOSE_NIGHTLY_JOBS:
                continue
            for step in job["steps"]:
                toolchain = str(step.get("with", {}).get("toolchain", ""))
                run = str(step.get("run", ""))
                if toolchain == pin or f"+{pin}" in run:
                    issues.append(
                        f"{workflow_path.name}/{job_name} duplicates generic pin {pin}"
                    )

    assert issues == []


def test_direct_dated_nightlies_are_limited_to_declared_special_jobs() -> None:
    issues: list[str] = []
    dated_nightly = re.compile(r"nightly-\d{4}-\d{2}-\d{2}")

    for workflow_path in sorted(WORKFLOWS.glob("*.y*ml")):
        for job_name, job in _load_yaml(workflow_path)["jobs"].items():
            if (
                workflow_path.name,
                job_name,
            ) in DIRECT_SPECIAL_PURPOSE_NIGHTLY_JOBS:
                continue
            for step in job.get("steps", []):
                toolchain = str(step.get("with", {}).get("toolchain", ""))
                run = str(step.get("run", ""))
                if dated_nightly.fullmatch(toolchain) or re.search(
                    r"\+nightly-\d{4}-\d{2}-\d{2}\b", run
                ):
                    issues.append(
                        f"{workflow_path.name}/{job_name} bypasses a canonical pin"
                    )

    assert issues == []


@pytest.mark.parametrize(
    "workflow_name",
    sorted({workflow_name for workflow_name, *_ in REQUIRED_NIGHTLY_JOBS}),
)
def test_nightly_installer_changes_trigger_consuming_workflows(
    workflow_name: str,
) -> None:
    workflow = _load_yaml(WORKFLOWS / workflow_name)
    on_block = workflow.get("on", workflow.get(True))

    for event in ("push", "pull_request"):
        assert ".github/actions/install-pinned-nightly/**" in on_block[event]["paths"]


def test_extensionless_release_pin_triggers_documentation_script_tests() -> None:
    workflow = _load_yaml(WORKFLOWS / "ci-docs.yml")
    on_block = workflow.get("on", workflow.get(True))
    relative_pin = RELEASE_PIN.relative_to(REPO_ROOT).as_posix()

    assert RELEASE_PIN.suffix == ""
    for event in ("push", "pull_request"):
        patterns = on_block[event]["paths"]
        assert ".github/actions/install-pinned-release/**" in patterns
        assert any(fnmatchcase(relative_pin, pattern) for pattern in patterns)
