#!/usr/bin/env python3
"""Regression tests for AI CLI and storage devcontainer integration."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
from pathlib import Path

import pytest
import yaml

try:
    import tomllib
except ImportError:  # pragma: no cover - Python < 3.11
    import tomli as tomllib

REPO_ROOT = Path(__file__).resolve().parents[2]
DOCKERFILE = REPO_ROOT / ".devcontainer" / "Dockerfile"
DEVCONTAINER_JSON = REPO_ROOT / ".devcontainer" / "devcontainer.json"
BOOTSTRAP_SCRIPT = REPO_ROOT / ".devcontainer" / "codex-bootstrap.sh"
DEVCONTAINER_BOOTSTRAP_SCRIPT = (
    REPO_ROOT / ".devcontainer" / "devcontainer-bootstrap.sh"
)
CHECK_TOOLS_SCRIPT = REPO_ROOT / "scripts" / "ci" / "check-tools.sh"
DEVCONTAINER_BUILD_WORKFLOW = (
    REPO_ROOT / ".github" / "workflows" / "devcontainer-build.yml"
)
TLA_VERSION_FILE = REPO_ROOT / ".tla-tools-version"
CODEX_CONFIG = REPO_ROOT / ".codex" / "config.toml"
OPENCODE_PLUGIN = REPO_ROOT / ".opencode" / "plugins" / "bounded-network-retries.js"
DEPENDABOT_CONFIG = REPO_ROOT / ".github" / "dependabot.yml"
GITHUB_TOOL_GUIDANCE_FILES = (
    REPO_ROOT / ".agents" / "skills" / "fortress-development" / "SKILL.md",
    REPO_ROOT / "AGENTS.md",
    REPO_ROOT / ".github" / "copilot-instructions.md",
    REPO_ROOT / "CLAUDE.md",
    REPO_ROOT / ".cursorrules",
)


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

    def test_dockerfile_installs_all_ai_clis_at_latest_tag(self) -> None:
        """Every requested AI CLI must resolve latest during image build."""
        text = DOCKERFILE.read_text(encoding="utf-8")
        assert "@nanocollective/nanocoder@latest" in text
        assert "@opencode/cli@latest" in text
        assert "opencode-ai@latest" not in text
        assert "@openai/codex@latest" in text
        assert "@anthropic-ai/claude-code@latest" in text
        assert "@github/copilot@latest" in text
        assert "mcp-remote@latest" in text
        assert "@z_ai/mcp-server@latest" in text

    def test_dockerfile_installs_claude_subprocess_isolation_dependencies(self) -> None:
        """Claude credential scrubbing requires Linux sandbox dependencies."""
        text = DOCKERFILE.read_text(encoding="utf-8")
        assert "bubblewrap \\" in text
        assert "socat \\" in text

    def test_dockerfile_uses_user_owned_npm_prefix(self) -> None:
        """Global npm commands must install below the vscode user's home."""
        text = DOCKERFILE.read_text(encoding="utf-8")
        assert "ENV NPM_CONFIG_PREFIX=/home/vscode/.local" in text
        assert "ENV NPM_CONFIG_CACHE=/home/vscode/.cache/npm" in text
        assert "ENV PATH=/home/vscode/.local/bin:${PATH}" in text

        user_install = text.index("@nanocollective/nanocoder@latest")
        assert text.rfind("USER vscode", 0, user_install) != -1

    def test_devcontainer_mounts_persistent_codex_home(self) -> None:
        """devcontainer.json keeps Codex auth cache on a persistent volume."""
        text = DEVCONTAINER_JSON.read_text(encoding="utf-8")
        assert '"target": "/home/vscode/.codex"' in text
        assert '"source": "fortress-rollback-codex-home"' in text

    def test_codex_zai_mcp_servers_use_native_streamable_http(self) -> None:
        """Z.ai remote MCP servers must not be declared as Codex stdio bridges.

        A stdio bridge (``command`` + ``args``) merged with a ``url`` makes Codex
        fail the MCP bootstrap with ``url is not supported for stdio``, which
        prevents Codex from running at all.
        """
        with CODEX_CONFIG.open("rb") as stream:
            codex_config = tomllib.load(stream)
        codex = codex_config["mcp_servers"]
        assert codex["zai-vision"]["command"] == "zai-mcp-vision"
        # Environment variable *name*, never a credential value.
        expected_env_var = "ZAI_API_KEY"  # noqa: S105
        for name in ("zai-web-search", "zai-web-reader", "zai-zread"):
            server = codex[name]
            assert server["url"].startswith("https://api.z.ai/api/mcp/")
            assert server["bearer_token_env_var"] == expected_env_var
            assert "command" not in server
            assert "args" not in server

    def test_devcontainer_runs_bootstrap_on_create_and_start(self) -> None:
        """Lifecycle hooks should refresh tools and enforce storage bounds."""
        text = DEVCONTAINER_JSON.read_text(encoding="utf-8")
        assert "bash .devcontainer/devcontainer-bootstrap.sh post-create" in text
        assert "bash .devcontainer/devcontainer-bootstrap.sh post-start" in text
        assert "bash .devcontainer/codex-bootstrap.sh post-create" in text
        assert "bash .devcontainer/codex-bootstrap.sh post-start" in text

    def test_devcontainer_builds_with_compact_debug_information(self) -> None:
        """Dev and test artifacts should retain line tables without full debug data."""
        text = DEVCONTAINER_JSON.read_text(encoding="utf-8")
        assert '"CARGO_PROFILE_DEV_DEBUG": "1"' in text
        assert '"CARGO_PROFILE_TEST_DEBUG": "1"' in text

    def test_devcontainer_puts_user_npm_binaries_first_on_path(self) -> None:
        """Terminals should resolve refreshed user-owned CLIs before image copies."""
        text = DEVCONTAINER_JSON.read_text(encoding="utf-8")
        assert '"PATH": "/home/vscode/.local/bin:${containerEnv:PATH}"' in text

    def test_tool_checker_covers_all_ai_clis(self) -> None:
        """The post-open verifier should report every requested AI CLI."""
        text = CHECK_TOOLS_SCRIPT.read_text(encoding="utf-8")
        assert 'check_tool "nanocoder" "nanocoder --version"' in text
        assert 'check_tool "opencode (v2)"' in text
        assert 'check_tool "codex" "codex --version"' in text
        assert 'check_tool "claude" "claude --version"' in text
        assert 'check_tool "copilot" "copilot --version"' in text
        assert 'check_tool "mcp-remote" "mcp-remote --version"' in text
        assert 'check_tool "bubblewrap" "bwrap --version"' in text
        assert 'check_tool "socat" "socat -V"' in text

    def test_tool_checker_does_not_accept_opencode_v1(self) -> None:
        """A stale V1 binary must not count as the installed V2 CLI."""
        text = CHECK_TOOLS_SCRIPT.read_text(encoding="utf-8")
        assert 'check_tool "opencode (v2)"' in text
        assert "opencode --version | grep" in text

    def test_dependabot_applies_msrv_holds_to_every_cargo_workspace(self) -> None:
        """Path-dependent workspaces must not bypass the root manifest's MSRV holds."""
        config = yaml.safe_load(DEPENDABOT_CONFIG.read_text(encoding="utf-8"))
        expected_holds = {
            "bincode-next": {"version-update:semver-major"},
            "serial_test": {"version-update:semver-major"},
            "parking_lot": {
                "version-update:semver-minor",
                "version-update:semver-major",
            },
            "proptest": {
                "version-update:semver-minor",
                "version-update:semver-major",
            },
            "z3": {
                "version-update:semver-minor",
                "version-update:semver-major",
            },
        }
        cargo_updates = [
            update
            for update in config["updates"]
            if update["package-ecosystem"] == "cargo"
        ]
        assert {update["directory"] for update in cargo_updates} == {
            "/",
            "/fuzz",
            "/loom-tests",
            "/tests/godot-emscripten",
        }
        root_ignore = next(
            update for update in cargo_updates if update["directory"] == "/"
        ).get("ignore", [])
        for update in cargo_updates:
            rules = {
                rule["dependency-name"]: set(rule.get("update-types", []))
                for rule in update.get("ignore", [])
            }
            for dependency, update_types in expected_holds.items():
                assert update_types <= rules.get(dependency, set()), (
                    f"{update['directory']} must hold {dependency} updates that "
                    "exceed the Rust 1.86 MSRV"
                )
            if update["directory"] != "/":
                assert all(
                    rule in update.get("ignore", []) for rule in root_ignore
                ), f"{update['directory']} must include every root Cargo ignore rule"

    def test_pull_requests_build_devcontainer_without_publishing(self) -> None:
        """Image changes must build before merge without writing to GHCR."""
        text = DEVCONTAINER_BUILD_WORKFLOW.read_text(encoding="utf-8")
        assert "pull_request:\n    branches: [main]\n    paths:" in text
        assert "if: github.event_name != 'pull_request'" in text
        assert "push: ${{ github.event_name != 'pull_request' }}" in text

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

    def test_agent_guidance_makes_gh_an_absolute_fallback(self) -> None:
        """Every agent entrypoint must prefer VS Code connectors and Git."""
        for guidance_file in GITHUB_TOOL_GUIDANCE_FILES:
            text = guidance_file.read_text(encoding="utf-8")
            normalized = " ".join(text.split())
            assert "callable VS Code" in normalized, guidance_file
            assert "GitHub Pull Requests and Issues" in normalized, guidance_file
            assert "local `git`" in normalized, guidance_file
            assert "`gh` is an absolute fallback" in normalized, guidance_file
            assert "`gh auth status` is not a prerequisite" in normalized, guidance_file
            assert normalized.index("callable VS Code") < normalized.index(
                "`gh` is an absolute fallback"
            ), guidance_file


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
        env.pop("OPENAI_API_KEY", None)
        env.pop("CODEX_API_KEY", None)
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
        env.pop("OPENAI_API_KEY", None)
        env.pop("CODEX_API_KEY", None)

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


def _write_npm_stub(tmp_path: Path) -> Path:
    """Create an npm stub that records user-prefix installs."""
    npm_path = tmp_path / "npm"
    npm_path.write_text(
        "#!/bin/bash\n"
        "set -eu\n"
        "printf '%s|%s|%s\\n' \"${NPM_CONFIG_PREFIX:-}\" "
        "\"${NPM_CONFIG_CACHE:-}\" \"$*\" >> \"${NPM_STUB_LOG:?}\"\n"
        "case \"$*\" in\n"
        "  view*) printf '%s\\n' \"${NPM_STUB_VIEW_VERSION:-}\" ;;\n"
        "esac\n"
        "case \"$*\" in\n"
        "  *\"${NPM_STUB_FAIL_PACKAGE:-__never__}\"*) exit 1 ;;\n"
        "esac\n"
        "if [[ \"$*\" == *\"install --global\"* ]] "
        "&& [[ \"$*\" == *\"${NPM_STUB_FAIL_INSTALL_PACKAGE:-__never__}\"* ]]; then\n"
        "  exit 1\n"
        "fi\n"
        "if [[ \"$*\" == *\"uninstall --global --no-audit --no-fund opencode-ai\"* ]] "
        "&& [[ \"${NPM_STUB_MODEL_OPENCODE_RELINK:-0}\" == \"1\" ]]; then\n"
        "  rm -f \"${NPM_CONFIG_PREFIX}/bin/opencode\"\n"
        "fi\n"
        "if [[ \"$*\" == *\"install --global\"*\"@opencode/cli@latest\"* ]] "
        "&& [[ \"${NPM_STUB_MODEL_OPENCODE_RELINK:-0}\" == \"1\" ]]; then\n"
        "  printf '#!/bin/sh\\necho 2.0.0-test\\n' "
        "> \"${NPM_CONFIG_PREFIX}/bin/opencode\"\n"
        "  chmod 755 \"${NPM_CONFIG_PREFIX}/bin/opencode\"\n"
        "fi\n"
        "exit 0\n",
        encoding="utf-8",
    )
    npm_path.chmod(0o755)
    return npm_path


def _write_node_stub(tmp_path: Path) -> Path:
    """Create the minimal Node stub used to read an installed package version."""
    node_path = tmp_path / "node"
    node_path.write_text(
        "#!/bin/bash\n"
        "set -eu\n"
        "package_json=\"${3:?}\"\n"
        "sed -n 's/.*\"version\"[[:space:]]*:[[:space:]]*\"\\([^\"]*\\)\".*/\\1/p' "
        "\"${package_json}\"\n",
        encoding="utf-8",
    )
    node_path.chmod(0o755)
    return node_path


def _write_opencode_v2_package(npm_prefix: Path, version: str) -> None:
    """Model an installed V2 package and its shared opencode bin symlink."""
    package_dir = npm_prefix / "lib" / "node_modules" / "@opencode" / "cli"
    package_dir.mkdir(parents=True)
    (package_dir / "package.json").write_text(
        f'{{"version":"{version}"}}\n', encoding="utf-8"
    )
    executable = package_dir / "bin" / "opencode.exe"
    executable.parent.mkdir()
    executable.write_text("#!/bin/sh\necho 2.0.0-test\n", encoding="utf-8")
    executable.chmod(0o755)
    bin_dir = npm_prefix / "bin"
    bin_dir.mkdir(parents=True, exist_ok=True)
    (bin_dir / "opencode").symlink_to(
        "../lib/node_modules/@opencode/cli/bin/opencode.exe"
    )


def _write_cargo_stub(tmp_path: Path) -> Path:
    """Create a cargo stub that records cleanup requests."""
    cargo_path = tmp_path / "cargo"
    cargo_path.write_text(
        "#!/bin/bash\n"
        "set -eu\n"
        "printf '%s\\n' \"$*\" >> \"${CARGO_STUB_LOG:?}\"\n",
        encoding="utf-8",
    )
    cargo_path.chmod(0o755)
    return cargo_path


def _mark_cargo_target(target_dir: Path) -> None:
    """Write Cargo's standard cache marker into a synthetic target directory."""
    (target_dir / "CACHEDIR.TAG").write_text(
        "Signature: 8a477f597d28d172789f06886806bc55\n",
        encoding="utf-8",
    )


def _bootstrap_env(tmp_path: Path, **overrides: str) -> dict[str, str]:
    """Return a subprocess environment isolated from host npm/cache settings."""
    env = os.environ.copy()
    for variable in (
        "NPM_CONFIG_PREFIX",
        "NPM_CONFIG_CACHE",
        "XDG_CACHE_HOME",
        "GITHUB_PERSONAL_ACCESS_TOKEN",
        "ZAI_API_KEY",
    ):
        env.pop(variable, None)
    env["HOME"] = str(tmp_path)
    env["PATH"] = f"{tmp_path / 'bin'}:/usr/bin:/bin"
    env.update(overrides)
    return env


class TestDevcontainerBootstrapScript:
    """Checks latest-at-run npm installs and bounded Cargo cleanup."""

    def test_script_parses_in_bash(self) -> None:
        """Lifecycle bootstrap must remain valid Bash syntax."""
        result = subprocess.run(
            ["bash", "-n", str(DEVCONTAINER_BOOTSTRAP_SCRIPT)],
            capture_output=True,
            text=True,
            check=False,
        )
        assert result.returncode == 0, result.stderr

    def test_refreshes_each_ai_cli_at_latest_without_sudo(self, tmp_path: Path) -> None:
        """A lifecycle run resolves every CLI into a writable user prefix."""
        bin_dir = tmp_path / "bin"
        bin_dir.mkdir()
        _write_npm_stub(bin_dir)
        npm_log = tmp_path / "npm.log"

        env = _bootstrap_env(
            tmp_path,
            NPM_STUB_LOG=str(npm_log),
            DEVCONTAINER_SKIP_TARGET_CLEANUP="1",
            DEVCONTAINER_SKIP_CODEX_BOOTSTRAP="1",
        )

        result = subprocess.run(
            ["/bin/bash", str(DEVCONTAINER_BOOTSTRAP_SCRIPT), "post-create"],
            capture_output=True,
            text=True,
            env=env,
            check=False,
        )

        assert result.returncode == 0, result.stderr
        log = npm_log.read_text(encoding="utf-8")
        expected_prefix = f"{tmp_path}/.local"
        expected_cache = f"{tmp_path}/.cache/npm"
        for package in (
            "@nanocollective/nanocoder@latest",
            "@opencode/cli@latest",
            "@openai/codex@latest",
            "@anthropic-ai/claude-code@latest",
            "@github/copilot@latest",
            "mcp-remote@latest",
            "@z_ai/mcp-server@latest",
        ):
            assert (
                f"{expected_prefix}|{expected_cache}|install --global --no-audit "
                "--no-fund --prefer-online "
                "--allow-scripts=@opencode/cli,@anthropic-ai/claude-code "
                f"{package}"
            ) in log
        assert "sudo" not in log
        assert (tmp_path / ".local" / "bin").is_dir()
        assert (tmp_path / ".cache" / "npm").is_dir()

    def test_legacy_opencode_v1_removal_restores_shared_v2_executable(
        self, tmp_path: Path
    ) -> None:
        """Uninstalling V1 must relink V2 even when its package is already latest."""
        bin_dir = tmp_path / "bin"
        bin_dir.mkdir()
        _write_npm_stub(bin_dir)
        _write_node_stub(bin_dir)
        npm_log = tmp_path / "npm.log"
        npm_prefix = tmp_path / ".local"
        legacy_package = npm_prefix / "lib" / "node_modules" / "opencode-ai"
        legacy_package.mkdir(parents=True)
        (legacy_package / "package.json").write_text(
            '{"version":"1.19.5"}\n', encoding="utf-8"
        )
        _write_opencode_v2_package(npm_prefix, "1.2.3")

        env = _bootstrap_env(
            tmp_path,
            NPM_STUB_LOG=str(npm_log),
            NPM_STUB_VIEW_VERSION="1.2.3",
            NPM_STUB_MODEL_OPENCODE_RELINK="1",
            DEVCONTAINER_SKIP_TARGET_CLEANUP="1",
            DEVCONTAINER_SKIP_CODEX_BOOTSTRAP="1",
        )

        result = subprocess.run(
            ["/bin/bash", str(DEVCONTAINER_BOOTSTRAP_SCRIPT), "post-create"],
            capture_output=True,
            text=True,
            env=env,
            check=False,
        )

        assert result.returncode == 0, result.stderr
        assert "Removing legacy OpenCode 1 package" in result.stdout
        assert "OpenCode 2 executable verified" in result.stdout
        assert (npm_prefix / "bin" / "opencode").is_file()
        lines = npm_log.read_text(encoding="utf-8").splitlines()
        uninstall_indexes = [
            index
            for index, line in enumerate(lines)
            if line.endswith("|uninstall --global --no-audit --no-fund opencode-ai")
        ]
        v2_install_indexes = [
            index
            for index, line in enumerate(lines)
            if line.endswith(
                "|install --global --no-audit --no-fund --prefer-online "
                "--allow-scripts=@opencode/cli,@anthropic-ai/claude-code @opencode/cli@latest"
            )
        ]
        assert len(uninstall_indexes) == 1
        assert not v2_install_indexes, "an already-current V2 package needs no npm reinstall"
        assert "opencode-ai@latest" not in "\n".join(lines)

    def test_legacy_removal_restores_installed_v2_without_registry(
        self, tmp_path: Path
    ) -> None:
        """Unlink followed by lookup/install failure must leave a usable V2 CLI."""
        for failure in ("view", "install"):
            case_root = tmp_path / failure
            bin_dir = case_root / "bin"
            bin_dir.mkdir(parents=True)
            _write_npm_stub(bin_dir)
            _write_node_stub(bin_dir)
            npm_log = case_root / "npm.log"
            npm_prefix = case_root / ".local"
            legacy_package = npm_prefix / "lib" / "node_modules" / "opencode-ai"
            legacy_package.mkdir(parents=True)
            (legacy_package / "package.json").write_text(
                '{"version":"1.19.5"}\n', encoding="utf-8"
            )
            _write_opencode_v2_package(npm_prefix, "1.0.0")
            failure_override = {
                "NPM_STUB_FAIL_PACKAGE"
                if failure == "view"
                else "NPM_STUB_FAIL_INSTALL_PACKAGE": "@opencode/cli"
            }

            env = _bootstrap_env(
                case_root,
                NPM_STUB_LOG=str(npm_log),
                NPM_STUB_VIEW_VERSION="1.2.3",
                NPM_STUB_MODEL_OPENCODE_RELINK="1",
                DEVCONTAINER_SKIP_TARGET_CLEANUP="1",
                DEVCONTAINER_SKIP_CODEX_BOOTSTRAP="1",
                **failure_override,
            )
            result = subprocess.run(
                ["/bin/bash", str(DEVCONTAINER_BOOTSTRAP_SCRIPT), "post-create"],
                capture_output=True,
                text=True,
                env=env,
                check=False,
                timeout=10,
            )

            assert result.returncode == 0, failure
            assert "OpenCode 2 executable verified" in result.stdout, failure
            executable = npm_prefix / "bin" / "opencode"
            assert executable.is_file(), failure
            version = subprocess.run(
                [str(executable), "--version"],
                capture_output=True,
                text=True,
                check=False,
                timeout=10,
            )
            assert version.stdout.strip() == "2.0.0-test", failure

    def test_one_npm_failure_is_non_blocking_and_other_tools_continue(
        self, tmp_path: Path
    ) -> None:
        """A registry/package failure must not prevent later tool refreshes."""
        bin_dir = tmp_path / "bin"
        bin_dir.mkdir()
        _write_npm_stub(bin_dir)
        npm_log = tmp_path / "npm.log"

        env = _bootstrap_env(
            tmp_path,
            NPM_STUB_LOG=str(npm_log),
            NPM_STUB_FAIL_PACKAGE="@opencode/cli",
            DEVCONTAINER_SKIP_TARGET_CLEANUP="1",
            DEVCONTAINER_SKIP_CODEX_BOOTSTRAP="1",
        )

        result = subprocess.run(
            ["/bin/bash", str(DEVCONTAINER_BOOTSTRAP_SCRIPT), "post-create"],
            capture_output=True,
            text=True,
            env=env,
            check=False,
        )

        assert result.returncode == 0
        assert "@opencode/cli@latest refresh failed" in result.stderr
        assert "@openai/codex@latest" in npm_log.read_text(encoding="utf-8")

    def test_current_versions_skip_expensive_reinstallation(
        self, tmp_path: Path
    ) -> None:
        """A warm container should resolve latest tags without replacing packages."""
        bin_dir = tmp_path / "bin"
        bin_dir.mkdir()
        _write_npm_stub(bin_dir)
        _write_node_stub(bin_dir)
        npm_log = tmp_path / "npm.log"
        npm_prefix = tmp_path / ".local"
        for package_path in (
            "@nanocollective/nanocoder",
            "@opencode/cli",
            "@openai/codex",
            "@anthropic-ai/claude-code",
            "@github/copilot",
            "mcp-remote",
            "@z_ai/mcp-server",
        ):
            package_dir = npm_prefix / "lib" / "node_modules" / package_path
            package_dir.mkdir(parents=True)
            (package_dir / "package.json").write_text(
                '{"version":"1.2.3"}\n', encoding="utf-8"
            )

        env = _bootstrap_env(
            tmp_path,
            NPM_STUB_LOG=str(npm_log),
            NPM_STUB_VIEW_VERSION="1.2.3",
            DEVCONTAINER_SKIP_TARGET_CLEANUP="1",
            DEVCONTAINER_SKIP_CODEX_BOOTSTRAP="1",
        )

        result = subprocess.run(
            ["/bin/bash", str(DEVCONTAINER_BOOTSTRAP_SCRIPT), "post-create"],
            capture_output=True,
            text=True,
            env=env,
            check=False,
        )

        assert result.returncode == 0, result.stderr
        log = npm_log.read_text(encoding="utf-8")
        assert log.count("|view ") == 7
        assert "|install " not in log
        assert result.stdout.count("1.2.3 is current") == 7

    def test_unwritable_npm_prefix_skips_refresh_without_sudo(
        self, tmp_path: Path
    ) -> None:
        """A stale permission problem should warn without prompting or blocking."""
        bin_dir = tmp_path / "bin"
        bin_dir.mkdir()
        _write_npm_stub(bin_dir)
        npm_log = tmp_path / "npm.log"
        npm_prefix = tmp_path / "read-only-prefix"
        (npm_prefix / "bin").mkdir(parents=True)
        (npm_prefix / "lib" / "node_modules").mkdir(parents=True)
        npm_prefix.chmod(0o500)

        env = _bootstrap_env(
            tmp_path,
            NPM_CONFIG_PREFIX=str(npm_prefix),
            NPM_STUB_LOG=str(npm_log),
            DEVCONTAINER_SKIP_TARGET_CLEANUP="1",
            DEVCONTAINER_SKIP_CODEX_BOOTSTRAP="1",
        )

        result = subprocess.run(
            ["/bin/bash", str(DEVCONTAINER_BOOTSTRAP_SCRIPT), "post-create"],
            capture_output=True,
            text=True,
            env=env,
            check=False,
        )

        assert result.returncode == 0
        assert "skipping refresh without sudo" in result.stderr
        assert not npm_log.exists()

    def test_post_start_skips_registry_refresh(self, tmp_path: Path) -> None:
        """Routine starts must not wait for registry access."""
        bin_dir = tmp_path / "bin"
        bin_dir.mkdir()
        _write_npm_stub(bin_dir)
        npm_log = tmp_path / "npm.log"

        env = _bootstrap_env(
            tmp_path,
            NPM_STUB_LOG=str(npm_log),
            DEVCONTAINER_SKIP_TARGET_CLEANUP="1",
            DEVCONTAINER_SKIP_CODEX_BOOTSTRAP="1",
        )

        result = subprocess.run(
            ["/bin/bash", str(DEVCONTAINER_BOOTSTRAP_SCRIPT), "post-start"],
            capture_output=True,
            text=True,
            env=env,
            check=False,
        )

        assert result.returncode == 0, result.stderr
        assert "Skipping npm CLI refresh outside post-create." in result.stdout
        assert not npm_log.exists()

    def test_target_over_limit_runs_cargo_clean(self, tmp_path: Path) -> None:
        """Oversized derived artifacts should be removed through Cargo."""
        bin_dir = tmp_path / "bin"
        bin_dir.mkdir()
        _write_cargo_stub(bin_dir)
        cargo_log = tmp_path / "cargo.log"
        target_dir = tmp_path / "workspace" / "target"
        target_dir.mkdir(parents=True)
        (target_dir / "artifact").write_bytes(b"oversized")
        _mark_cargo_target(target_dir)

        env = _bootstrap_env(
            tmp_path,
            CARGO_STUB_LOG=str(cargo_log),
            DEVCONTAINER_WORKSPACE_DIR=str(tmp_path / "workspace"),
            FORTRESS_TARGET_DIR=str(target_dir),
            FORTRESS_TARGET_MAX_BYTES="1",
            FORTRESS_TARGET_CLEAN_MIN_AGE_DAYS="0",
            DEVCONTAINER_SKIP_TOOL_REFRESH="1",
            DEVCONTAINER_SKIP_CODEX_BOOTSTRAP="1",
        )

        result = subprocess.run(
            ["/bin/bash", str(DEVCONTAINER_BOOTSTRAP_SCRIPT), "post-start"],
            capture_output=True,
            text=True,
            env=env,
            check=False,
        )

        assert result.returncode == 0, result.stderr
        assert f"clean --target-dir {target_dir}" in cargo_log.read_text(
            encoding="utf-8"
        )
        assert "exceeds" in result.stdout

    def test_target_below_limit_preserves_artifacts(self, tmp_path: Path) -> None:
        """Small target directories should remain warm between starts."""
        bin_dir = tmp_path / "bin"
        bin_dir.mkdir()
        _write_cargo_stub(bin_dir)
        cargo_log = tmp_path / "cargo.log"
        target_dir = tmp_path / "workspace" / "target"
        target_dir.mkdir(parents=True)
        (target_dir / "artifact").write_bytes(b"small")

        env = _bootstrap_env(
            tmp_path,
            CARGO_STUB_LOG=str(cargo_log),
            DEVCONTAINER_WORKSPACE_DIR=str(tmp_path / "workspace"),
            FORTRESS_TARGET_DIR=str(target_dir),
            FORTRESS_TARGET_MAX_BYTES=str(1024 * 1024),
            DEVCONTAINER_SKIP_TOOL_REFRESH="1",
            DEVCONTAINER_SKIP_CODEX_BOOTSTRAP="1",
        )

        result = subprocess.run(
            ["/bin/bash", str(DEVCONTAINER_BOOTSTRAP_SCRIPT), "post-start"],
            capture_output=True,
            text=True,
            env=env,
            check=False,
        )

        assert result.returncode == 0, result.stderr
        assert not cargo_log.exists()
        assert (target_dir / "artifact").exists()

    def test_recent_cleanup_prevents_repeated_cache_thrashing(
        self, tmp_path: Path
    ) -> None:
        """An oversized target should not be purged more than once per interval."""
        bin_dir = tmp_path / "bin"
        bin_dir.mkdir()
        _write_cargo_stub(bin_dir)
        cargo_log = tmp_path / "cargo.log"
        target_dir = tmp_path / "workspace" / "target"
        target_dir.mkdir(parents=True)
        (target_dir / "artifact").write_bytes(b"oversized")
        _mark_cargo_target(target_dir)
        stamp_dir = tmp_path / ".cache" / "fortress-rollback"
        stamp_dir.mkdir(parents=True)
        (stamp_dir / "last-target-clean").touch()

        env = _bootstrap_env(
            tmp_path,
            CARGO_STUB_LOG=str(cargo_log),
            DEVCONTAINER_WORKSPACE_DIR=str(tmp_path / "workspace"),
            FORTRESS_TARGET_DIR=str(target_dir),
            FORTRESS_TARGET_MAX_BYTES="1",
            FORTRESS_TARGET_CLEAN_MIN_AGE_DAYS="7",
            DEVCONTAINER_SKIP_TOOL_REFRESH="1",
            DEVCONTAINER_SKIP_CODEX_BOOTSTRAP="1",
        )

        result = subprocess.run(
            ["/bin/bash", str(DEVCONTAINER_BOOTSTRAP_SCRIPT), "post-start"],
            capture_output=True,
            text=True,
            env=env,
            check=False,
        )

        assert result.returncode == 0, result.stderr
        assert "cleanup ran within 7 days" in result.stdout
        assert not cargo_log.exists()

    def test_cleanup_interval_check_failure_skips_cleanup(self, tmp_path: Path) -> None:
        """A failed age check must not fall through to destructive cleanup."""
        bin_dir = tmp_path / "bin"
        bin_dir.mkdir()
        _write_cargo_stub(bin_dir)
        find_stub = bin_dir / "find"
        find_stub.write_text("#!/bin/sh\nexit 1\n", encoding="utf-8")
        find_stub.chmod(0o755)
        cargo_log = tmp_path / "cargo.log"
        target_dir = tmp_path / "workspace" / "target"
        target_dir.mkdir(parents=True)
        artifact = target_dir / "artifact"
        artifact.write_bytes(b"preserve")
        _mark_cargo_target(target_dir)
        stamp_dir = tmp_path / ".cache" / "fortress-rollback"
        stamp_dir.mkdir(parents=True)
        (stamp_dir / "last-target-clean").touch()

        env = _bootstrap_env(
            tmp_path,
            CARGO_STUB_LOG=str(cargo_log),
            DEVCONTAINER_WORKSPACE_DIR=str(tmp_path / "workspace"),
            FORTRESS_TARGET_DIR=str(target_dir),
            FORTRESS_TARGET_MAX_BYTES="1",
            FORTRESS_TARGET_CLEAN_MIN_AGE_DAYS="7",
            DEVCONTAINER_SKIP_TOOL_REFRESH="1",
            DEVCONTAINER_SKIP_CODEX_BOOTSTRAP="1",
        )

        result = subprocess.run(
            ["/bin/bash", str(DEVCONTAINER_BOOTSTRAP_SCRIPT), "post-start"],
            capture_output=True,
            text=True,
            env=env,
            check=False,
        )

        assert result.returncode == 0
        assert "Could not validate the target cleanup interval" in result.stderr
        assert not cargo_log.exists()
        assert artifact.read_bytes() == b"preserve"

    def test_refuses_broad_cleanup_target(self, tmp_path: Path) -> None:
        """Cleanup must never accept a filesystem root as its target."""
        bin_dir = tmp_path / "bin"
        bin_dir.mkdir()
        _write_cargo_stub(bin_dir)
        cargo_log = tmp_path / "cargo.log"

        env = _bootstrap_env(
            tmp_path,
            CARGO_STUB_LOG=str(cargo_log),
            FORTRESS_TARGET_DIR="/",
            FORTRESS_TARGET_MAX_BYTES="1",
            DEVCONTAINER_SKIP_TOOL_REFRESH="1",
            DEVCONTAINER_SKIP_CODEX_BOOTSTRAP="1",
        )

        result = subprocess.run(
            ["/bin/bash", str(DEVCONTAINER_BOOTSTRAP_SCRIPT), "post-start"],
            capture_output=True,
            text=True,
            env=env,
            check=False,
        )

        assert result.returncode == 0
        assert "Refusing unmanaged target directory" in result.stderr
        assert not cargo_log.exists()

    def test_refuses_unmarked_cleanup_target(self, tmp_path: Path) -> None:
        """Cleanup must not trust an arbitrary directory supplied through the environment."""
        bin_dir = tmp_path / "bin"
        bin_dir.mkdir()
        _write_cargo_stub(bin_dir)
        cargo_log = tmp_path / "cargo.log"
        target_dir = tmp_path / "workspace" / "target"
        target_dir.mkdir(parents=True)
        (target_dir / "unrelated-data").write_bytes(b"oversized")

        env = _bootstrap_env(
            tmp_path,
            CARGO_STUB_LOG=str(cargo_log),
            DEVCONTAINER_WORKSPACE_DIR=str(tmp_path / "workspace"),
            FORTRESS_TARGET_DIR=str(target_dir),
            FORTRESS_TARGET_MAX_BYTES="1",
            DEVCONTAINER_SKIP_TOOL_REFRESH="1",
            DEVCONTAINER_SKIP_CODEX_BOOTSTRAP="1",
        )

        result = subprocess.run(
            ["/bin/bash", str(DEVCONTAINER_BOOTSTRAP_SCRIPT), "post-start"],
            capture_output=True,
            text=True,
            env=env,
            check=False,
        )

        assert result.returncode == 0
        assert "missing Cargo cache marker" in result.stderr
        assert not cargo_log.exists()
        assert (target_dir / "unrelated-data").exists()

    def test_refuses_marked_workspace_parent_as_cleanup_target(
        self, tmp_path: Path
    ) -> None:
        """A generic cache marker must not authorize cleaning the workspace itself."""
        bin_dir = tmp_path / "bin"
        bin_dir.mkdir()
        _write_cargo_stub(bin_dir)
        cargo_log = tmp_path / "cargo.log"
        workspace_dir = tmp_path / "projects" / "workspace"
        workspace_dir.mkdir(parents=True)
        parent_dir = workspace_dir.parent
        (parent_dir / "source.rs").write_bytes(b"source")
        _mark_cargo_target(parent_dir)

        env = _bootstrap_env(
            tmp_path,
            CARGO_STUB_LOG=str(cargo_log),
            DEVCONTAINER_WORKSPACE_DIR=str(workspace_dir),
            FORTRESS_TARGET_DIR=str(parent_dir),
            FORTRESS_TARGET_MAX_BYTES="1",
            DEVCONTAINER_SKIP_TOOL_REFRESH="1",
            DEVCONTAINER_SKIP_CODEX_BOOTSTRAP="1",
        )

        result = subprocess.run(
            ["/bin/bash", str(DEVCONTAINER_BOOTSTRAP_SCRIPT), "post-start"],
            capture_output=True,
            text=True,
            env=env,
            check=False,
        )

        assert result.returncode == 0
        assert "Refusing unmanaged target directory" in result.stderr
        assert not cargo_log.exists()
        assert (parent_dir / "source.rs").exists()

    def test_refuses_marked_arbitrary_cleanup_target(self, tmp_path: Path) -> None:
        """A generic cache marker must not authorize an unrelated directory."""
        bin_dir = tmp_path / "bin"
        bin_dir.mkdir()
        _write_cargo_stub(bin_dir)
        cargo_log = tmp_path / "cargo.log"
        workspace_dir = tmp_path / "workspace"
        workspace_dir.mkdir()
        target_dir = tmp_path / "shared-cache"
        target_dir.mkdir()
        (target_dir / "unrelated-data").write_bytes(b"oversized")
        _mark_cargo_target(target_dir)

        env = _bootstrap_env(
            tmp_path,
            CARGO_STUB_LOG=str(cargo_log),
            DEVCONTAINER_WORKSPACE_DIR=str(workspace_dir),
            FORTRESS_TARGET_DIR=str(target_dir),
            FORTRESS_TARGET_MAX_BYTES="1",
            DEVCONTAINER_SKIP_TOOL_REFRESH="1",
            DEVCONTAINER_SKIP_CODEX_BOOTSTRAP="1",
        )

        result = subprocess.run(
            ["/bin/bash", str(DEVCONTAINER_BOOTSTRAP_SCRIPT), "post-start"],
            capture_output=True,
            text=True,
            env=env,
            check=False,
        )

        assert result.returncode == 0
        assert "Refusing unmanaged target directory" in result.stderr
        assert not cargo_log.exists()
        assert (target_dir / "unrelated-data").exists()

    def test_refuses_marked_external_symlink_target(self, tmp_path: Path) -> None:
        """The managed target path must not redirect cleanup through a symlink."""
        bin_dir = tmp_path / "bin"
        bin_dir.mkdir()
        _write_cargo_stub(bin_dir)
        cargo_log = tmp_path / "cargo.log"
        workspace_dir = tmp_path / "workspace"
        workspace_dir.mkdir()
        external_target = tmp_path / "external-target"
        external_target.mkdir()
        (external_target / "unrelated-data").write_bytes(b"oversized")
        _mark_cargo_target(external_target)
        target_link = workspace_dir / "target"
        target_link.symlink_to(external_target, target_is_directory=True)

        env = _bootstrap_env(
            tmp_path,
            CARGO_STUB_LOG=str(cargo_log),
            DEVCONTAINER_WORKSPACE_DIR=str(workspace_dir),
            FORTRESS_TARGET_DIR=str(target_link),
            FORTRESS_TARGET_MAX_BYTES="1",
            DEVCONTAINER_SKIP_TOOL_REFRESH="1",
            DEVCONTAINER_SKIP_CODEX_BOOTSTRAP="1",
        )

        result = subprocess.run(
            ["/bin/bash", str(DEVCONTAINER_BOOTSTRAP_SCRIPT), "post-start"],
            capture_output=True,
            text=True,
            env=env,
            check=False,
        )

        assert result.returncode == 0
        assert "Refusing symlink target directory" in result.stderr
        assert not cargo_log.exists()
        assert (external_target / "unrelated-data").exists()

    def test_huge_target_max_bytes_skips_cleanup(self, tmp_path: Path) -> None:
        """An oversized digit string must not reach Bash arithmetic or Cargo."""
        bin_dir = tmp_path / "bin"
        bin_dir.mkdir()
        _write_cargo_stub(bin_dir)
        cargo_log = tmp_path / "cargo.log"
        target_dir = tmp_path / "workspace" / "target"
        target_dir.mkdir(parents=True)
        artifact = target_dir / "artifact"
        artifact.write_bytes(b"preserve")
        _mark_cargo_target(target_dir)

        env = _bootstrap_env(
            tmp_path,
            CARGO_STUB_LOG=str(cargo_log),
            DEVCONTAINER_WORKSPACE_DIR=str(tmp_path / "workspace"),
            FORTRESS_TARGET_DIR=str(target_dir),
            FORTRESS_TARGET_MAX_BYTES="9" * 1000,
            FORTRESS_TARGET_CLEAN_MIN_AGE_DAYS="0",
            DEVCONTAINER_SKIP_TOOL_REFRESH="1",
            DEVCONTAINER_SKIP_CODEX_BOOTSTRAP="1",
        )

        result = subprocess.run(
            ["/bin/bash", str(DEVCONTAINER_BOOTSTRAP_SCRIPT), "post-start"],
            capture_output=True,
            text=True,
            env=env,
            check=False,
        )

        assert result.returncode == 0
        assert "FORTRESS_TARGET_MAX_BYTES" in result.stderr
        assert not cargo_log.exists()
        assert artifact.read_bytes() == b"preserve"

    def test_huge_target_clean_min_age_skips_cleanup(self, tmp_path: Path) -> None:
        """An oversized age must not reach Bash arithmetic, find, or Cargo."""
        bin_dir = tmp_path / "bin"
        bin_dir.mkdir()
        _write_cargo_stub(bin_dir)
        cargo_log = tmp_path / "cargo.log"
        target_dir = tmp_path / "workspace" / "target"
        target_dir.mkdir(parents=True)
        artifact = target_dir / "artifact"
        artifact.write_bytes(b"preserve")
        _mark_cargo_target(target_dir)

        env = _bootstrap_env(
            tmp_path,
            CARGO_STUB_LOG=str(cargo_log),
            DEVCONTAINER_WORKSPACE_DIR=str(tmp_path / "workspace"),
            FORTRESS_TARGET_DIR=str(target_dir),
            FORTRESS_TARGET_MAX_BYTES="1",
            FORTRESS_TARGET_CLEAN_MIN_AGE_DAYS="9" * 1000,
            DEVCONTAINER_SKIP_TOOL_REFRESH="1",
            DEVCONTAINER_SKIP_CODEX_BOOTSTRAP="1",
        )

        result = subprocess.run(
            ["/bin/bash", str(DEVCONTAINER_BOOTSTRAP_SCRIPT), "post-start"],
            capture_output=True,
            text=True,
            env=env,
            check=False,
        )

        assert result.returncode == 0
        assert "FORTRESS_TARGET_CLEAN_MIN_AGE_DAYS" in result.stderr
        assert not cargo_log.exists()
        assert artifact.read_bytes() == b"preserve"

    def test_reports_unconfigured_project_mcp_servers(self, tmp_path: Path) -> None:
        """A missing runtime MCP environment must be reported, not silent."""
        workspace = tmp_path / "workspace"
        (workspace / ".devcontainer").mkdir(parents=True)

        env = _bootstrap_env(
            tmp_path,
            DEVCONTAINER_WORKSPACE_DIR=str(workspace),
            DEVCONTAINER_SKIP_TOOL_REFRESH="1",
            DEVCONTAINER_SKIP_TARGET_CLEANUP="1",
        )

        result = subprocess.run(
            ["/bin/bash", str(DEVCONTAINER_BOOTSTRAP_SCRIPT), "post-start"],
            capture_output=True,
            text=True,
            env=env,
            check=False,
        )

        assert result.returncode == 0
        assert "Project MCP servers are disabled" in result.stderr
        assert ".env.local.example" in result.stderr
        assert "restart is not enough" in result.stderr

    def test_reports_partially_configured_project_mcp_servers(
        self, tmp_path: Path
    ) -> None:
        """A GitHub-only configuration must name the servers it cannot enable."""
        workspace = tmp_path / "workspace"
        runtime_env = workspace / ".devcontainer" / ".mcp.env"
        runtime_env.parent.mkdir(parents=True)
        runtime_env.write_text(
            "GITHUB_PERSONAL_ACCESS_TOKEN=test-value\n", encoding="utf-8"
        )

        credentials = {"GITHUB_PERSONAL_ACCESS_TOKEN": "test-value"}
        env = _bootstrap_env(
            tmp_path,
            DEVCONTAINER_WORKSPACE_DIR=str(workspace),
            DEVCONTAINER_SKIP_TOOL_REFRESH="1",
            DEVCONTAINER_SKIP_TARGET_CLEANUP="1",
            **credentials,
        )

        result = subprocess.run(
            ["/bin/bash", str(DEVCONTAINER_BOOTSTRAP_SCRIPT), "post-start"],
            capture_output=True,
            text=True,
            env=env,
            check=False,
        )

        assert result.returncode == 0
        assert "partially configured" in result.stderr
        assert "zai-*" in result.stderr
        assert "test-value" not in result.stderr

    def test_mcp_file_process_mismatch_requires_rebuild(self, tmp_path: Path) -> None:
        """File-only, env-only, and differing credentials must not look injected."""
        scenarios = (
            ("file-only", "GITHUB_PERSONAL_ACCESS_TOKEN=file-test\n", {}),
            ("env-only", "", {"GITHUB_PERSONAL_ACCESS_TOKEN": "env-test"}),
            (
                "differing",
                "GITHUB_PERSONAL_ACCESS_TOKEN=file-test\n",
                {"GITHUB_PERSONAL_ACCESS_TOKEN": "process-test"},
            ),
        )
        for label, file_contents, credentials in scenarios:
            workspace = tmp_path / label
            runtime_env = workspace / ".devcontainer" / ".mcp.env"
            runtime_env.parent.mkdir(parents=True)
            runtime_env.write_text(file_contents, encoding="utf-8")
            env = _bootstrap_env(
                tmp_path,
                DEVCONTAINER_WORKSPACE_DIR=str(workspace),
                DEVCONTAINER_SKIP_TOOL_REFRESH="1",
                DEVCONTAINER_SKIP_TARGET_CLEANUP="1",
                **credentials,
            )

            result = subprocess.run(
                ["/bin/bash", str(DEVCONTAINER_BOOTSTRAP_SCRIPT), "post-start"],
                capture_output=True,
                text=True,
                env=env,
                check=False,
                timeout=10,
            )

            assert result.returncode == 0, label
            assert "does not match" in result.stderr, label
            assert "rebuild required" in result.stderr, label
            assert "file-test" not in result.stderr, label
            assert "process-test" not in result.stderr, label

    def test_silent_when_project_mcp_servers_are_configured(
        self, tmp_path: Path
    ) -> None:
        """A complete runtime environment must not produce MCP noise."""
        workspace = tmp_path / "workspace"
        runtime_env = workspace / ".devcontainer" / ".mcp.env"
        runtime_env.parent.mkdir(parents=True)
        runtime_env.write_text(
            "GITHUB_PERSONAL_ACCESS_TOKEN=test-value\nZAI_API_KEY=test-value\n",
            encoding="utf-8",
        )

        credentials = {
            "GITHUB_PERSONAL_ACCESS_TOKEN": "test-value",
            "ZAI_API_KEY": "test-value",
        }
        env = _bootstrap_env(
            tmp_path,
            DEVCONTAINER_WORKSPACE_DIR=str(workspace),
            DEVCONTAINER_SKIP_TOOL_REFRESH="1",
            DEVCONTAINER_SKIP_TARGET_CLEANUP="1",
            **credentials,
        )

        result = subprocess.run(
            ["/bin/bash", str(DEVCONTAINER_BOOTSTRAP_SCRIPT), "post-start"],
            capture_output=True,
            text=True,
            env=env,
            check=False,
        )

        assert result.returncode == 0
        assert "Project MCP servers" not in result.stderr


def _write_opencode_plugin_stub(root: Path) -> None:
    """Provide the V2 plugin SDK surface that a local plugin imports."""
    package_dir = root / "node_modules" / "@opencode" / "plugin"
    package_dir.mkdir(parents=True)
    (package_dir / "package.json").write_text(
        json.dumps(
            {
                "name": "@opencode/plugin",
                "version": "0.0.0-test",
                "type": "module",
                "exports": "./index.js",
            }
        )
        + "\n",
        encoding="utf-8",
    )
    (package_dir / "index.js").write_text(
        "export const Plugin = { define: (definition) => definition }\n",
        encoding="utf-8",
    )


OPENCODE_PLUGIN_HARNESS = """
import assert from "node:assert/strict"

const timers = new Map()
const clearedTimers = []
let nextTimerID = 0
globalThis.setTimeout = (callback, delay) => {
  const timer = { id: ++nextTimerID, callback, delay, unref() {} }
  timers.set(timer.id, timer)
  return timer
}
globalThis.clearTimeout = (timer) => {
  clearedTimers.push(timer.id)
  timers.delete(timer.id)
}

const { default: plugin } = await import(new URL("plugin.mjs", import.meta.url).href)

const interrupts = []
const failOnce = new Set()
const hungOnce = new Set()
const deferOnce = new Set()
let deferredReject = null
const queue = []
let wake = null

const subscribe = ({ signal } = {}) => ({
  async *[Symbol.asyncIterator]() {
    while (!signal?.aborted) {
      if (queue.length === 0) {
        await new Promise((resolve) => {
          wake = resolve
        })
        continue
      }
      yield queue.shift()
    }
  },
})

const cleanup = plugin.setup({
  event: { subscribe },
  session: {
    async interrupt(input) {
      interrupts.push(input)
      if (failOnce.delete(input.sessionID)) throw new Error("fake interrupt failure")
      if (hungOnce.delete(input.sessionID)) return new Promise(() => {})
      if (deferOnce.delete(input.sessionID)) {
        return new Promise((_, reject) => {
          deferredReject = reject
        })
      }
    },
  },
})

const flush = async () => {
  await new Promise((resolve) => setImmediate(resolve))
  await new Promise((resolve) => setImmediate(resolve))
}

const emit = async (sessionID, status) => {
  queue.push({ type: "session.status", data: { sessionID, status } })
  const resume = wake
  wake = null
  resume?.()
  await flush()
}

const fireTimer = async (delay) => {
  const entry = [...timers.entries()].find(([, timer]) => timer.delay === delay)
  assert.ok(entry, `expected a pending ${delay}ms timer`)
  const [timerID, timer] = entry
  timers.delete(timerID)
  timer.callback()
  await flush()
}

// Idle must really cancel the first timer and allow a later retry to arm another.
await emit("a", { type: "retry", attempt: 1, message: "fetch failed" })
assert.equal(timers.size, 1)
assert.equal(timers.get(1).delay, 120_000)
await emit("a", { type: "idle" })
assert.equal(timers.size, 0)
assert.deepEqual(clearedTimers, [1])
await emit("a", { type: "retry", attempt: 1, message: "fetch failed" })
assert.equal(timers.size, 1)
await fireTimer(120_000)
assert.deepEqual(interrupts, [{ sessionID: "a", resume: false }])
await emit("a", { type: "retry", attempt: 3, message: "socket hang up" })
assert.equal(interrupts.length, 1, "a successful interrupt remains in flight until idle")
await emit("a", { type: "idle" })

// A failed attempt-limit interrupt must be retryable by the very next event.
failOnce.add("b")
await emit("b", { type: "retry", attempt: 3, message: "socket hang up" })
assert.equal(interrupts.length, 2)
assert.equal(timers.size, 1, "failed interrupt should retain bounded recovery")
await emit("b", { type: "retry", attempt: 3, message: "socket hang up" })
assert.equal(interrupts.length, 3, "failed interrupt marker must be cleared")
assert.equal(timers.size, 0)
await emit("b", { type: "idle" })

// Timer-triggered failure must recover without requiring another status event.
failOnce.add("c")
await emit("c", { type: "retry", attempt: 1, message: "connection reset" })
await fireTimer(120_000)
assert.equal(interrupts.length, 4)
assert.equal(timers.size, 1, "failed timer interrupt should schedule one replacement")
await fireTimer(120_000)
assert.equal(interrupts.length, 5)
assert.equal(timers.size, 0)
await emit("c", { type: "idle" })

await emit("d", { type: "retry", attempt: 1, message: "model overloaded" })
assert.equal(interrupts.length, 5)
assert.equal(timers.size, 0)

// A hung interrupt must not block unrelated events and must time out into recovery.
hungOnce.add("e")
await emit("e", { type: "retry", attempt: 3, message: "socket hang up" })
assert.equal(interrupts.length, 6)
assert.ok([...timers.values()].some((timer) => timer.delay === 10_000))
await emit("f", { type: "retry", attempt: 1, message: "fetch failed" })
assert.ok([...timers.values()].some((timer) => timer.delay === 120_000))
await emit("f", { type: "idle" })
await fireTimer(10_000)
assert.ok([...timers.values()].some((timer) => timer.delay === 120_000))
await emit("e", { type: "idle" })
assert.equal(timers.size, 0)

// An old interrupt settling after idle must not disarm or rearm a newer attempt.
hungOnce.add("g")
await emit("g", { type: "retry", attempt: 3, message: "socket hang up" })
await emit("g", { type: "idle" })
await emit("g", { type: "retry", attempt: 3, message: "socket hang up" })
assert.equal(interrupts.length, 8, "the newer interrupt must not be blocked by the old one")
await fireTimer(10_000)
assert.equal(timers.size, 0, "the stale failure must not arm recovery after newer success")
await emit("g", { type: "idle" })

// Cleanup must cancel pending timeouts and late rejection must not revive timers.
deferOnce.add("h")
await emit("h", { type: "retry", attempt: 3, message: "socket hang up" })
assert.equal(interrupts.length, 9)
assert.equal(typeof deferredReject, "function")
cleanup()
assert.equal(timers.size, 0)
deferredReject(new Error("late interrupt failure"))
await flush()
assert.equal(timers.size, 0, "disposal must prevent late timer revival")
console.log("ok")
"""


class TestOpenCodeBoundedNetworkRetriesPlugin:
    """Checks that the V1 plugin implementation is ported to the V2 plugin API."""

    def test_plugin_uses_v2_definition_and_event_api(self) -> None:
        """V2 does not run V1 plugin functions, so the file must use the V2 entrypoint."""
        text = OPENCODE_PLUGIN.read_text(encoding="utf-8")
        assert 'from "@opencode/plugin"' in text
        assert "Plugin.define(" in text
        assert 'id: "fortress.bounded-network-retries"' in text
        assert "ctx.event.subscribe(" in text
        assert 'types: ["session.status"]' in text
        assert "ctx.session.interrupt(" in text
        assert "client.session.abort" not in text
        assert "export default async (" not in text

    def test_plugin_interrupts_retries_after_the_attempt_limit(
        self, tmp_path: Path
    ) -> None:
        """The ported plugin must keep the retry limit, idle reset, and message filter."""
        node = shutil.which("node")
        if node is None:
            pytest.skip("node is required to exercise the OpenCode plugin")

        _write_opencode_plugin_stub(tmp_path)
        (tmp_path / "plugin.mjs").write_text(
            OPENCODE_PLUGIN.read_text(encoding="utf-8"), encoding="utf-8"
        )
        (tmp_path / "harness.mjs").write_text(OPENCODE_PLUGIN_HARNESS, encoding="utf-8")
        (tmp_path / "package.json").write_text('{"type":"module"}\n', encoding="utf-8")

        result = subprocess.run(
            [node, str(tmp_path / "harness.mjs")],
            capture_output=True,
            text=True,
            cwd=tmp_path,
            check=False,
            timeout=10,
        )

        assert result.returncode == 0, result.stderr
        assert "ok" in result.stdout
