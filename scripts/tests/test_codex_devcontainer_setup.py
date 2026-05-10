#!/usr/bin/env python3
"""Regression tests for Codex CLI devcontainer integration."""

from __future__ import annotations

import os
import subprocess
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
DOCKERFILE = REPO_ROOT / ".devcontainer" / "Dockerfile"
DEVCONTAINER_JSON = REPO_ROOT / ".devcontainer" / "devcontainer.json"
BOOTSTRAP_SCRIPT = REPO_ROOT / ".devcontainer" / "codex-bootstrap.sh"
TLA_VERSION_FILE = REPO_ROOT / ".tla-tools-version"


def _write_codex_stub(tmp_path: Path) -> Path:
    """Create a stub codex binary for hook behavior tests."""
    codex_path = tmp_path / "codex"
    codex_path.write_text(
        "#!/bin/bash\n"
        "set -eu\n"
        "if [[ \"${1:-}\" == \"--version\" ]]; then\n"
        "  echo \"codex-cli 0.0.0-test\"\n"
        "  exit 0\n"
        "fi\n"
        "if [[ \"${1:-}\" == \"login\" && \"${2:-}\" == \"status\" ]]; then\n"
        "  if [[ \"${CODEX_STUB_LOGGED_IN:-0}\" == \"1\" ]]; then\n"
        "    echo \"Logged in\"\n"
        "    exit 0\n"
        "  fi\n"
        "  echo \"Not logged in\"\n"
        "  exit 1\n"
        "fi\n"
        "if [[ \"${1:-}\" == \"login\" && \"${2:-}\" == \"--with-api-key\" ]]; then\n"
        "  read -r _ || true\n"
        "  if [[ \"${CODEX_STUB_ACCEPT_API_KEY:-1}\" == \"1\" ]]; then\n"
        "    echo \"Logged in with API key\"\n"
        "    exit 0\n"
        "  fi\n"
        "  echo \"API key rejected\" >&2\n"
        "  exit 1\n"
        "fi\n"
        "echo \"unsupported command: $*\" >&2\n"
        "exit 2\n",
        encoding="utf-8",
    )
    codex_path.chmod(0o755)
    return codex_path


def _write_sudo_stub(tmp_path: Path) -> Path:
    """Create a stub sudo binary that logs calls and fixes write perms."""
    sudo_path = tmp_path / "sudo"
    sudo_path.write_text(
        "#!/bin/bash\n"
        "set -eu\n"
        "echo \"$*\" >> \"${SUDO_STUB_LOG:?}\"\n"
        "target=\"${@: -1}\"\n"
        "chmod u+rwx \"${target}\"\n"
        "exit 0\n",
        encoding="utf-8",
    )
    sudo_path.chmod(0o755)
    return sudo_path


class TestCodexDevcontainerConfiguration:
    """Checks that devcontainer wiring remains stable over time."""

    def test_dockerfile_installs_openai_codex_latest(self) -> None:
        """Dockerfile must install the latest OpenAI Codex CLI."""
        text = DOCKERFILE.read_text(encoding="utf-8")
        assert "@openai/codex@latest" in text

    def test_devcontainer_mounts_persistent_codex_home(self) -> None:
        """devcontainer.json keeps Codex auth cache on a persistent volume."""
        text = DEVCONTAINER_JSON.read_text(encoding="utf-8")
        assert '"target": "/home/vscode/.codex"' in text
        assert '"source": "fortress-rollback-codex-home"' in text

    def test_devcontainer_runs_codex_bootstrap_on_create_and_start(self) -> None:
        """Lifecycle hooks should invoke the Codex bootstrap script."""
        text = DEVCONTAINER_JSON.read_text(encoding="utf-8")
        assert "bash .devcontainer/codex-bootstrap.sh post-create" in text
        assert "bash .devcontainer/codex-bootstrap.sh post-start" in text

    def test_tla_tools_version_is_repo_pinned(self) -> None:
        """Devcontainer setup should derive TLA+ tools from the repo version file."""
        version = TLA_VERSION_FILE.read_text(encoding="utf-8").strip()
        assert version
        dockerfile = DOCKERFILE.read_text(encoding="utf-8")
        devcontainer = DEVCONTAINER_JSON.read_text(encoding="utf-8")
        assert "COPY .tla-tools-version" in dockerfile
        assert ".devcontainer/setup-tla-tools.sh" in devcontainer
        assert f"releases/download/v{version}/tla2tools.jar" not in dockerfile
        assert f"releases/download/v{version}/tla2tools.jar" not in devcontainer


class TestCodexBootstrapScript:
    """Checks for login readiness and shell behavior in the hook script."""

    def test_script_parses_in_bash(self) -> None:
        """Bootstrap script must remain valid Bash syntax."""
        result = subprocess.run(
            ["bash", "-n", str(BOOTSTRAP_SCRIPT)],
            capture_output=True,
            text=True,
            check=False,
        )
        assert result.returncode == 0, result.stderr

    def test_script_contains_headless_login_guidance(self) -> None:
        """Script should advertise browser, device, and API-key login paths."""
        text = BOOTSTRAP_SCRIPT.read_text(encoding="utf-8")
        assert "codex login --device-auth" in text
        assert "codex login --with-api-key" in text
        assert "CODEX_API_KEY" in text

    def test_missing_codex_is_non_blocking(self, tmp_path: Path) -> None:
        """If codex is missing, hook exits successfully with guidance."""
        env = os.environ.copy()
        empty_path = tmp_path / "empty-path"
        empty_path.mkdir()
        env["HOME"] = str(tmp_path)
        env["PATH"] = str(empty_path)

        result = subprocess.run(
            ["/bin/bash", str(BOOTSTRAP_SCRIPT), "post-start"],
            capture_output=True,
            text=True,
            env=env,
            check=False,
        )

        assert result.returncode == 0
        assert "not on PATH" in result.stdout

    def test_unwritable_codex_home_attempts_permission_repair(
        self, tmp_path: Path
    ) -> None:
        """Bootstrap should attempt to repair non-writable CODEX_HOME."""
        bin_dir = tmp_path / "bin"
        bin_dir.mkdir()
        _write_codex_stub(bin_dir)
        _write_sudo_stub(bin_dir)

        codex_home = tmp_path / "codex-home"
        codex_home.mkdir()
        codex_home.chmod(0o500)

        sudo_log = tmp_path / "sudo.log"

        env = os.environ.copy()
        env["HOME"] = str(tmp_path)
        env["CODEX_HOME"] = str(codex_home)
        env["PATH"] = f"{bin_dir}:/usr/bin:/bin"
        env["SUDO_STUB_LOG"] = str(sudo_log)

        result = subprocess.run(
            ["bash", str(BOOTSTRAP_SCRIPT), "post-start"],
            capture_output=True,
            text=True,
            env=env,
            check=False,
        )

        assert result.returncode == 0
        assert "attempting permission repair" in result.stdout
        assert "Repaired Codex home permissions" in result.stdout
        assert sudo_log.exists()
        sudo_invocation = sudo_log.read_text(encoding="utf-8")
        assert "-n chown -R" in sudo_invocation

    def test_unwritable_codex_home_does_not_block_when_sudo_prompts(
        self, tmp_path: Path
    ) -> None:
        """Bootstrap should use non-interactive sudo and continue if repair is unavailable."""
        bin_dir = tmp_path / "bin"
        bin_dir.mkdir()
        _write_codex_stub(bin_dir)
        sudo_path = bin_dir / "sudo"
        sudo_path.write_text(
            "#!/bin/bash\n"
            "set -eu\n"
            "echo \"$*\" >> \"${SUDO_STUB_LOG:?}\"\n"
            "exit 1\n",
            encoding="utf-8",
        )
        sudo_path.chmod(0o755)

        codex_home = tmp_path / "codex-home"
        codex_home.mkdir()
        codex_home.chmod(0o500)

        sudo_log = tmp_path / "sudo.log"

        env = os.environ.copy()
        env["HOME"] = str(tmp_path)
        env["CODEX_HOME"] = str(codex_home)
        env["PATH"] = f"{bin_dir}:/usr/bin:/bin"
        env["SUDO_STUB_LOG"] = str(sudo_log)

        result = subprocess.run(
            ["bash", str(BOOTSTRAP_SCRIPT), "post-start"],
            capture_output=True,
            text=True,
            env=env,
            check=False,
        )

        assert result.returncode == 0
        assert "Non-interactive sudo permission repair was unavailable" in result.stderr
        assert "-n chown -R" in sudo_log.read_text(encoding="utf-8")

    def test_logged_in_status_reports_ready(self, tmp_path: Path) -> None:
        """If codex login status succeeds, script reports auth readiness."""
        bin_dir = tmp_path / "bin"
        bin_dir.mkdir()
        _write_codex_stub(bin_dir)

        env = os.environ.copy()
        env["HOME"] = str(tmp_path)
        env["PATH"] = f"{bin_dir}:/usr/bin:/bin"
        env["CODEX_STUB_LOGGED_IN"] = "1"

        result = subprocess.run(
            ["bash", str(BOOTSTRAP_SCRIPT), "post-start"],
            capture_output=True,
            text=True,
            env=env,
            check=False,
        )

        assert result.returncode == 0
        assert "Authentication is already configured" in result.stdout

    def test_api_key_environment_triggers_login_attempt(self, tmp_path: Path) -> None:
        """OPENAI_API_KEY should be used for non-interactive login."""
        bin_dir = tmp_path / "bin"
        bin_dir.mkdir()
        _write_codex_stub(bin_dir)

        env = os.environ.copy()
        env["HOME"] = str(tmp_path)
        env["PATH"] = f"{bin_dir}:/usr/bin:/bin"
        env["OPENAI_API_KEY"] = "sk-test-key"

        result = subprocess.run(
            ["bash", str(BOOTSTRAP_SCRIPT), "post-create"],
            capture_output=True,
            text=True,
            env=env,
            check=False,
        )

        assert result.returncode == 0
        assert "Logged in using API key from environment" in result.stdout

    def test_logged_out_without_key_prints_guidance(self, tmp_path: Path) -> None:
        """When not logged in and no key is set, script prints next steps."""
        bin_dir = tmp_path / "bin"
        bin_dir.mkdir()
        _write_codex_stub(bin_dir)

        env = os.environ.copy()
        env["HOME"] = str(tmp_path)
        env["PATH"] = f"{bin_dir}:/usr/bin:/bin"

        result = subprocess.run(
            ["bash", str(BOOTSTRAP_SCRIPT), "post-start"],
            capture_output=True,
            text=True,
            env=env,
            check=False,
        )

        assert result.returncode == 0
        assert "Authentication is not configured yet" in result.stdout
        assert "codex login --device-auth" in result.stdout
