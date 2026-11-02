#!/usr/bin/env bash
# Regression coverage for isolated native/Z.ai CLI backend launchers.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
launcher_script="${repo_root}/.devcontainer/ai-backends.sh"
test_root="$(mktemp -d)"
trap 'rm -rf "${test_root}"' EXIT

test_home="${test_root}/home"
test_bin="${test_root}/bin"
launcher_bin="${test_root}/launchers"
codex_home="${test_root}/codex-home"
mkdir -p "${test_home}" "${test_bin}" "${launcher_bin}"

fail() {
    printf 'FAIL: %s\n' "$*" >&2
    exit 1
}

grep -Eq '^[[:space:]]*bubblewrap[[:space:]]' "${repo_root}/.devcontainer/Dockerfile" \
    || fail "devcontainer does not install bubblewrap for Claude subprocess isolation"
grep -Eq '^[[:space:]]*socat[[:space:]]' "${repo_root}/.devcontainer/Dockerfile" \
    || fail "devcontainer does not install socat for Claude sandbox networking"

cat >"${test_bin}/codex" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
{
    printf 'zai_key=%s\n' "${ZAI_API_KEY:+set}"
    printf 'lowercase_zai_key=%s\n' "${zai_key-unset}"
    printf 'arg=%s\n' "$@"
} >"${STUB_LOG:?}"
STUB

cat >"${test_bin}/claude" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
{
    printf 'auth_token=%s\n' "${ANTHROPIC_AUTH_TOKEN:+set}"
    printf 'zai_key=%s\n' "${ZAI_API_KEY-unset}"
    printf 'zai_key_alias=%s\n' "${Z_AI_API_KEY-unset}"
    printf 'zai_mcp_key=%s\n' "${ZAI_MCP_API_KEY-unset}"
    printf 'native_key=%s\n' "${ANTHROPIC_API_KEY-unset}"
    printf 'native_oauth=%s\n' "${CLAUDE_CODE_OAUTH_TOKEN-unset}"
    printf 'lowercase_zai_key=%s\n' "${zai_key-unset}"
    printf 'base_url=%s\n' "${ANTHROPIC_BASE_URL-unset}"
    printf 'timeout=%s\n' "${API_TIMEOUT_MS-unset}"
    printf 'config_dir=%s\n' "${CLAUDE_CONFIG_DIR-unset}"
    printf 'bedrock=%s\n' "${CLAUDE_CODE_USE_BEDROCK-unset}"
    printf 'vertex=%s\n' "${CLAUDE_CODE_USE_VERTEX-unset}"
    printf 'foundry=%s\n' "${CLAUDE_CODE_USE_FOUNDRY-unset}"
    printf 'gateway=%s\n' "${CLAUDE_CODE_USE_GATEWAY-unset}"
    printf 'mantle=%s\n' "${CLAUDE_CODE_USE_MANTLE-unset}"
    printf 'anthropic_aws=%s\n' "${CLAUDE_CODE_USE_ANTHROPIC_AWS-unset}"
    printf 'managed_provider=%s\n' "${CLAUDE_CODE_PROVIDER_MANAGED_BY_HOST-unset}"
    printf 'subprocess_scrub=%s\n' "${CLAUDE_CODE_SUBPROCESS_ENV_SCRUB-unset}"
    printf 'haiku_model=%s\n' "${ANTHROPIC_DEFAULT_HAIKU_MODEL-unset}"
    printf 'sonnet_model=%s\n' "${ANTHROPIC_DEFAULT_SONNET_MODEL-unset}"
    printf 'opus_model=%s\n' "${ANTHROPIC_DEFAULT_OPUS_MODEL-unset}"
    printf 'arg=%s\n' "$@"
} >"${STUB_LOG:?}"
STUB
cat >"${test_bin}/bwrap" <<'STUB'
#!/usr/bin/env bash
if [ "${STUB_BWRAP_FAIL:-0}" = "1" ]; then
    printf 'simulated namespace denial\n' >&2
    exit 1
fi
exit 0
STUB
cat >"${test_bin}/socat" <<'STUB'
#!/usr/bin/env bash
exit 0
STUB
chmod 755 "${test_bin}/codex" "${test_bin}/claude" \
    "${test_bin}/bwrap" "${test_bin}/socat"

HOME="${test_home}" \
CODEX_HOME="${codex_home}" \
AI_BACKENDS_BIN_DIR="${launcher_bin}" \
    bash "${launcher_script}" install

[ -L "${launcher_bin}/codex-zai" ] || fail "codex-zai symlink was not installed"
[ -L "${launcher_bin}/claude-zai" ] || fail "claude-zai symlink was not installed"

# Containers may not put ~/.local/bin on PATH. In that case install beside the
# writable Codex binary so the launchers are immediately callable.
PATH="${test_bin}:/usr/bin:/bin" \
HOME="${test_home}" \
CODEX_HOME="${codex_home}" \
    bash "${launcher_script}" install
[ -L "${test_bin}/codex-zai" ] || fail "codex-adjacent launcher was not installed"
[ -L "${test_bin}/claude-zai" ] || fail "claude launcher was not installed beside Codex"

[ "$(stat -c '%a' "${codex_home}/devcontainer-zai.config.toml")" = "600" ] \
    || fail "Codex profile permissions are not 600"
jq -e '.models[0].slug == "glm-5.3"' \
    "${codex_home}/devcontainer-zai-models.json" >/dev/null \
    || fail "Codex model catalog is invalid"
grep -Eq '^env_key = "ZAI_API_KEY"$' "${codex_home}/devcontainer-zai.config.toml" \
    || fail "Codex profile does not use the environment key"
grep -Eq '^model_reasoning_effort = "max"$' "${codex_home}/devcontainer-zai.config.toml" \
    || fail "Codex profile does not enable Z.ai reasoning"
grep -Fq 'filters = { ZAI_API_KEY = "exclude", Z_AI_API_KEY = "exclude" }' \
    "${codex_home}/devcontainer-zai.config.toml" \
    || fail "Codex profile exposes Z.ai keys to tool subprocesses"
grep -Fq "model_catalog_json = \"${codex_home}/devcontainer-zai-models.json\"" \
    "${codex_home}/devcontainer-zai.config.toml" \
    || fail "Codex profile ignored the custom CODEX_HOME catalog path"
if grep -Eq 'experimental_bearer_token|test-secret' "${codex_home}/devcontainer-zai.config.toml"; then
    fail "Codex profile persisted a bearer token"
fi

codex_log="${test_root}/codex.log"
PATH="${test_bin}:${launcher_bin}:/usr/bin:/bin" \
HOME="${test_home}" \
CODEX_HOME="${codex_home}" \
STUB_LOG="${codex_log}" \
ZAI_API_KEY="test-secret" \
zai_key="ambient-lowercase-secret" \
CODEX_ZAI_MODEL="glm-test" \
    "${launcher_bin}/codex-zai" exec "hello world"
grep -Eq '^zai_key=set$' "${codex_log}" || fail "Codex did not receive the Z.ai key"
grep -Eq '^lowercase_zai_key=unset$' "${codex_log}" || fail "Codex leaked its internal Z.ai key variable"
grep -Eq '^arg=--profile$' "${codex_log}" || fail "Codex profile was not selected"
grep -Eq '^arg=devcontainer-zai$' "${codex_log}" || fail "Codex profile name was not forwarded"
grep -Eq '^arg=glm-test$' "${codex_log}" || fail "Codex model override was not forwarded"
grep -Eq '^arg=model_reasoning_effort="max"$' "${codex_log}" \
    || fail "Codex reasoning effort was not forwarded"
grep -Eq '^arg=exec$' "${codex_log}" || fail "Codex subcommand was not forwarded"
if grep -Eq 'test-secret' "${codex_log}"; then
    fail "Codex key leaked into arguments"
fi

claude_log="${test_root}/claude.log"
PATH="${test_bin}:${launcher_bin}:/usr/bin:/bin" \
HOME="${test_home}" \
STUB_LOG="${claude_log}" \
Z_AI_API_KEY="test-secret" \
ZAI_MCP_API_KEY="stale-mcp-secret" \
ANTHROPIC_API_KEY="native-secret" \
CLAUDE_CODE_OAUTH_TOKEN="native-oauth-secret" \
zai_key="ambient-lowercase-secret" \
CLAUDE_CODE_USE_BEDROCK=1 \
CLAUDE_CODE_USE_VERTEX=1 \
CLAUDE_CODE_USE_FOUNDRY=1 \
CLAUDE_CODE_USE_GATEWAY=1 \
CLAUDE_CODE_USE_MANTLE=1 \
CLAUDE_CODE_USE_ANTHROPIC_AWS=1 \
CLAUDE_CODE_PROVIDER_MANAGED_BY_HOST=ambient \
ANTHROPIC_MODEL=native-model \
ANTHROPIC_DEFAULT_HAIKU_MODEL=native-haiku \
ANTHROPIC_DEFAULT_SONNET_MODEL=native-sonnet \
ANTHROPIC_DEFAULT_OPUS_MODEL=native-opus \
AI_BACKENDS_CONTAINER_MODE=yes \
STUB_BWRAP_FAIL=1 \
ZAI_API_TIMEOUT_MS="123456" \
    "${launcher_bin}/claude-zai" --print "hello world"
grep -Eq '^auth_token=set$' "${claude_log}" || fail "Claude did not receive the Z.ai token"
grep -Eq '^zai_key=unset$' "${claude_log}" || fail "Claude retained the canonical Z.ai key"
grep -Eq '^zai_key_alias=unset$' "${claude_log}" || fail "Claude retained the Z.ai key alias"
grep -Eq '^zai_mcp_key=unset$' "${claude_log}" || fail "Claude retained the ambient MCP key alias"
grep -Eq '^native_key=unset$' "${claude_log}" || fail "Claude native key was not isolated"
grep -Eq '^native_oauth=unset$' "${claude_log}" || fail "Claude native OAuth token was not isolated"
grep -Eq '^lowercase_zai_key=unset$' "${claude_log}" || fail "Claude leaked its internal Z.ai key variable"
grep -Eq '^base_url=https://api.z.ai/api/anthropic$' "${claude_log}" \
    || fail "Claude Z.ai endpoint is incorrect"
grep -Eq '^timeout=123456$' "${claude_log}" || fail "Claude timeout override was not forwarded"
grep -Eq "^config_dir=${test_home}/.claude-zai$" "${claude_log}" \
    || fail "Claude Z.ai state was not isolated"
grep -Eq '^bedrock=unset$' "${claude_log}" || fail "Claude Bedrock routing was not isolated"
grep -Eq '^vertex=unset$' "${claude_log}" || fail "Claude Vertex routing was not isolated"
grep -Eq '^foundry=unset$' "${claude_log}" || fail "Claude Foundry routing was not isolated"
grep -Eq '^gateway=unset$' "${claude_log}" || fail "Claude gateway routing was not isolated"
grep -Eq '^mantle=unset$' "${claude_log}" || fail "Claude Mantle routing was not isolated"
grep -Eq '^anthropic_aws=unset$' "${claude_log}" \
    || fail "Claude Platform on AWS routing was not isolated"
grep -Eq '^managed_provider=1$' "${claude_log}" \
    || fail "Claude project settings can override the managed Z.ai provider"
grep -Eq '^subprocess_scrub=0$' "${claude_log}" \
    || fail "Claude enables namespace-dependent subprocess isolation inside a devcontainer"
grep -Eq '^haiku_model=glm-5.3-flash\[1m\]$' "${claude_log}" \
    || fail "Claude Haiku alias was not mapped to Z.ai"
grep -Eq '^sonnet_model=glm-5.3\[1m\]$' "${claude_log}" \
    || fail "Claude Sonnet alias was not mapped to Z.ai"
grep -Eq '^opus_model=glm-5.3\[1m\]$' "${claude_log}" \
    || fail "Claude Opus alias was not mapped to Z.ai"
grep -Eq '^arg=--print$' "${claude_log}" || fail "Claude arguments were not forwarded"
grep -Eq '^arg=--settings$' "${claude_log}" \
    || fail "Claude nested-container settings were not forwarded"
grep -Eq '^arg=--mcp-config$' "${claude_log}" \
    || fail "Claude Z.ai MCP catalog was not selected"
grep -Eq '^arg=--strict-mcp-config$' "${claude_log}" \
    || fail "Claude Z.ai did not isolate its MCP catalog"
grep -Fq 'arg={"enabledMcpjsonServers":["github-mcp-server","zai-vision","zai-web-search","zai-web-reader","zai-zread"],"sandbox":{"enabled":false,"enableWeakerNestedSandbox":true}}' \
    "${claude_log}" \
    || fail "Claude Z.ai did not approve only its known project MCP servers"
if grep -Eq 'test-secret|native-secret' "${claude_log}"; then
    fail "Claude credential leaked into arguments"
fi
if grep -Eq 'exec env' "${launcher_script}"; then
    fail "Claude launcher exposes assignments to an intermediate env process argv"
fi

# Outside an existing container, retain Claude's stronger subprocess isolation.
PATH="${test_bin}:${launcher_bin}:/usr/bin:/bin" \
HOME="${test_home}" \
STUB_LOG="${claude_log}" \
ZAI_API_KEY="test-secret" \
AI_BACKENDS_CONTAINER_MODE=no \
    "${launcher_bin}/claude-zai" --print "host mode"
grep -Eq '^subprocess_scrub=1$' "${claude_log}" \
    || fail "Claude subprocess isolation was not retained outside a container"

invalid_scrub_output="${test_root}/invalid-scrub.log"
if PATH="${test_bin}:${launcher_bin}:/usr/bin:/bin" \
    HOME="${test_home}" \
    STUB_LOG="${claude_log}" \
    ZAI_API_KEY="test-secret" \
    AI_BACKENDS_CONTAINER_MODE=yes \
    CLAUDE_ZAI_SUBPROCESS_ENV_SCRUB=maybe \
    "${launcher_bin}/claude-zai" --print "invalid" >"${invalid_scrub_output}" 2>&1; then
    fail "Claude Z.ai launcher accepted an invalid subprocess isolation mode"
fi
grep -Eq 'CLAUDE_ZAI_SUBPROCESS_ENV_SCRUB must be auto, 0, or 1' \
    "${invalid_scrub_output}" \
    || fail "Invalid subprocess isolation mode did not provide actionable guidance"

nested_sandbox_output="${test_root}/nested-sandbox.log"
nested_sandbox_claude_log="${test_root}/nested-sandbox-claude.log"
if PATH="${test_bin}:${launcher_bin}:/usr/bin:/bin" \
    HOME="${test_home}" \
    STUB_LOG="${nested_sandbox_claude_log}" \
    STUB_BWRAP_FAIL=1 \
    ZAI_API_KEY="test-secret" \
    AI_BACKENDS_CONTAINER_MODE=yes \
    CLAUDE_ZAI_SUBPROCESS_ENV_SCRUB=1 \
    "${launcher_bin}/claude-zai" --print "nested" >"${nested_sandbox_output}" 2>&1; then
    fail "Claude launcher ignored a broken explicitly requested nested sandbox"
fi
grep -Eq 'simulated namespace denial' "${nested_sandbox_output}" \
    || fail "Nested sandbox probe discarded bubblewrap diagnostics"
grep -Eq 'cannot create its nested sandbox' "${nested_sandbox_output}" \
    || fail "Nested sandbox failure did not provide actionable guidance"
[ ! -e "${nested_sandbox_claude_log}" ] \
    || fail "Claude was launched after the nested sandbox preflight failed"

missing_key_output="${test_root}/missing-key.log"
if PATH="${test_bin}:${launcher_bin}:/usr/bin:/bin" \
    HOME="${test_home}" \
    CODEX_HOME="${codex_home}" \
    STUB_LOG="${test_root}/unused.log" \
    env -u ZAI_API_KEY -u Z_AI_API_KEY \
    "${launcher_bin}/codex-zai" --version >"${missing_key_output}" 2>&1; then
    fail "Codex Z.ai launcher accepted a missing key"
fi
grep -Eq 'Set ZAI_API_KEY' "${missing_key_output}" \
    || fail "Missing-key failure did not provide setup guidance"

# Every supported frontend must discover the same project-scoped MCP catalog,
# while credentials remain in the ignored host-side .env.local file.
mcp_env_check="${repo_root}/.devcontainer/check-mcp-env.sh"
vision_launcher="${repo_root}/.devcontainer/zai-mcp-vision.sh"
remote_launcher="${repo_root}/.devcontainer/zai-mcp-remote.sh"
for required_file in \
    "${repo_root}/.mcp.json" \
    "${repo_root}/.vscode/mcp.json" \
    "${repo_root}/.codex/config.toml" \
    "${repo_root}/opencode.json" \
    "${repo_root}/.devcontainer/claude-zai.mcp.json" \
    "${repo_root}/.env.local.example" \
    "${mcp_env_check}" \
    "${vision_launcher}" \
    "${remote_launcher}"; do
    [ -f "${required_file}" ] || fail "missing MCP configuration: ${required_file#"${repo_root}/"}"
done
for executable_file in "${mcp_env_check}" "${vision_launcher}" "${remote_launcher}"; do
    [ -x "${executable_file}" ] \
        || fail "MCP helper is not executable: ${executable_file#"${repo_root}/"}"
done

python3 - "${repo_root}" <<'PY' || fail "project MCP catalogs are inconsistent"
import json
from pathlib import Path
import sys
import tomllib

root = Path(sys.argv[1])
names = {
    "github-mcp-server",
    "zai-vision",
    "zai-web-search",
    "zai-web-reader",
    "zai-zread",
}
urls = {
    "github-mcp-server": "https://api.githubcopilot.com/mcp/",
    "zai-web-search": "https://api.z.ai/api/mcp/web_search_prime/mcp",
    "zai-web-reader": "https://api.z.ai/api/mcp/web_reader/mcp",
    "zai-zread": "https://api.z.ai/api/mcp/zread/mcp",
}

portable = json.loads((root / ".mcp.json").read_text())
assert set(portable["mcpServers"]) == names
assert portable["mcpServers"]["zai-vision"]["command"] == "zai-mcp-vision"
assert portable["mcpServers"]["zai-vision"]["args"] == []
assert portable["mcpServers"]["zai-vision"]["env"] == {"ZAI_API_KEY": "${ZAI_API_KEY}"}
for name, url in urls.items():
    server = portable["mcpServers"][name]
    assert server["type"] == "http" and server["transport"] == "http"
    assert server["url"] == url
    token = "GITHUB_PERSONAL_ACCESS_TOKEN" if name == "github-mcp-server" else "ZAI_API_KEY"
    assert server["headers"]["Authorization"] == f"Bearer ${{{token}}}"
assert portable["mcpServers"]["github-mcp-server"]["headers"]["X-MCP-Readonly"] == "true"

vscode = json.loads((root / ".vscode/mcp.json").read_text())
assert set(vscode["servers"]) == names
assert vscode["servers"]["zai-vision"]["command"] == "zai-mcp-vision"
assert vscode["servers"]["zai-vision"]["env"] == {"ZAI_API_KEY": "${env:ZAI_API_KEY}"}
for name, url in urls.items():
    server = vscode["servers"][name]
    assert server["type"] == "http" and server["url"] == url
    token = "GITHUB_PERSONAL_ACCESS_TOKEN" if name == "github-mcp-server" else "ZAI_API_KEY"
    assert server["headers"]["Authorization"] == f"Bearer ${{env:{token}}}"
assert vscode["servers"]["github-mcp-server"]["headers"]["X-MCP-Readonly"] == "true"

opencode = json.loads((root / "opencode.json").read_text())
assert set(opencode["mcp"]) == names
assert opencode["mcp"]["zai-vision"]["type"] == "local"
assert opencode["mcp"]["zai-vision"]["command"] == ["zai-mcp-vision"]
assert opencode["mcp"]["zai-vision"]["environment"] == {"ZAI_API_KEY": "{env:ZAI_API_KEY}"}
for name, url in urls.items():
    server = opencode["mcp"][name]
    assert server["type"] == "remote" and server["url"] == url
    assert server["enabled"] is True and server["oauth"] is False
    token = "GITHUB_PERSONAL_ACCESS_TOKEN" if name == "github-mcp-server" else "ZAI_API_KEY"
    assert server["headers"]["Authorization"] == f"Bearer {{env:{token}}}"
assert opencode["mcp"]["github-mcp-server"]["headers"]["X-MCP-Readonly"] == "true"

with (root / ".codex/config.toml").open("rb") as stream:
    codex_config = tomllib.load(stream)
codex = codex_config["mcp_servers"]
assert set(codex) == names
assert codex["zai-vision"]["command"] == "zai-mcp-vision"
assert codex["zai-vision"]["args"] == []
assert codex["zai-vision"]["env_vars"] == ["ZAI_API_KEY"]
github = codex["github-mcp-server"]
assert github["url"] == urls["github-mcp-server"]
assert github["bearer_token_env_var"] == "GITHUB_PERSONAL_ACCESS_TOKEN"
assert codex["github-mcp-server"]["http_headers"]["X-MCP-Readonly"] == "true"
for name, url in urls.items():
    if name == "github-mcp-server":
        continue
    server = codex[name]
    assert server["url"] == url
    assert server["bearer_token_env_var"] == "ZAI_API_KEY"
    # Remote Z.ai servers must use the native streamable HTTP transport. A
    # stdio bridge (command + args) alongside a url is rejected by Codex with
    # "url is not supported for stdio", which blocks the whole MCP bootstrap.
    assert "command" not in server
    assert "args" not in server
assert codex_config["shell_environment_policy"]["filters"] == {
    "GITHUB_PERSONAL_ACCESS_TOKEN": "exclude",
    "ZAI_API_KEY": "exclude",
    "Z_AI_API_KEY": "exclude",
}

claude_zai = json.loads((root / ".devcontainer/claude-zai.mcp.json").read_text())["mcpServers"]
assert set(claude_zai) == names
assert claude_zai["zai-vision"]["command"] == "zai-mcp-vision"
assert "env" not in claude_zai["zai-vision"]
github = claude_zai["github-mcp-server"]
assert github["type"] == "http" and github["url"] == urls["github-mcp-server"]
assert github["headers"]["Authorization"] == "Bearer ${GITHUB_PERSONAL_ACCESS_TOKEN}"
assert claude_zai["github-mcp-server"]["headers"]["X-MCP-Readonly"] == "true"
for name, alias in {
    "zai-web-search": "web-search",
    "zai-web-reader": "web-reader",
    "zai-zread": "zread",
}.items():
    server = claude_zai[name]
    assert server["type"] == "stdio"
    assert server["command"] == "zai-mcp-remote"
    assert server["args"] == [alias]
    assert "env" not in server
PY

grep -R -Fq '@z_ai/mcp-server@latest' "${repo_root}/.devcontainer" \
    || fail "devcontainer does not install the Z.ai Vision MCP package"
grep -R -Fq '@github/copilot@latest' "${repo_root}/.devcontainer" \
    || fail "devcontainer does not install GitHub Copilot CLI"
grep -R -Fq 'mcp-remote@latest' "${repo_root}/.devcontainer" \
    || fail "devcontainer does not install the Claude/Z.ai transport bridge"
grep -Fq '/usr/local/bin/zai-mcp-vision' "${repo_root}/.devcontainer/Dockerfile" \
    || fail "devcontainer does not install the cwd-independent Vision wrapper"
grep -Fq '/usr/local/bin/zai-mcp-remote' "${repo_root}/.devcontainer/Dockerfile" \
    || fail "devcontainer does not install the cwd-independent remote MCP wrapper"
dockerignore_has_mcp_env=0
for dockerignore in "${repo_root}/.dockerignore" "${repo_root}/.devcontainer/.dockerignore"; do
    if [ -f "${dockerignore}" ] && grep -Fq '.mcp.env' "${dockerignore}"; then
        dockerignore_has_mcp_env=1
    fi
done
[ "${dockerignore_has_mcp_env}" = "1" ] \
    || fail "filtered MCP credentials can enter the Docker build context"
if grep -Eq '"context"[[:space:]]*:[[:space:]]*"\.\."' \
    "${repo_root}/.devcontainer/devcontainer.json"; then
    grep -Eq '^\.env\.(local\*|\*)$' "${repo_root}/.dockerignore" \
        || fail ".env.local backup files can enter the Docker build context"
fi
grep -Fq '"--env-file"' "${repo_root}/.devcontainer/devcontainer.json" \
    || fail "devcontainer does not load an environment file"
grep -Fq '".devcontainer/.mcp.env"' "${repo_root}/.devcontainer/devcontainer.json" \
    || fail "devcontainer does not load the filtered MCP environment"
grep -Fq '"NANOCODER_MCPSERVERS_FILE"' "${repo_root}/.devcontainer/devcontainer.json" \
    || fail "Nanocoder MCP discovery depends on the frontend launch directory"
grep -R -Fq 'check-mcp-env.sh' "${repo_root}/.devcontainer" \
    || fail "devcontainer initialization does not validate MCP credentials"
git -C "${repo_root}" check-ignore -q .env.local \
    || fail ".env.local is not ignored by Git"
git -C "${repo_root}" check-ignore -q .env.local.bak \
    || fail ".env.local backup files are not ignored by Git"
git -C "${repo_root}" check-ignore -q .devcontainer/.mcp.env \
    || fail "generated MCP environment is not ignored by Git"
git -C "${repo_root}" check-ignore -q .devcontainer/.mcp.env.interrupted \
    || fail "temporary MCP environment files are not ignored by Git"

preflight_workspace="${test_root}/preflight-workspace"
mkdir -p "${preflight_workspace}/.devcontainer"
parent_symlink_workspace="${test_root}/parent-symlink-workspace"
parent_symlink_target="${test_root}/outside-devcontainer"
mkdir -p "${parent_symlink_workspace}" "${parent_symlink_target}"
printf 'sentinel\n' >"${parent_symlink_target}/.mcp.env"
ln -s "${parent_symlink_target}" "${parent_symlink_workspace}/.devcontainer"
if bash "${mcp_env_check}" "${parent_symlink_workspace}" \
    >"${test_root}/preflight-parent-symlink.log" 2>&1; then
    fail "MCP credential preflight accepted a symlinked .devcontainer directory"
fi
grep -Fxq sentinel "${parent_symlink_target}/.mcp.env" \
    || fail "MCP credential preflight cleanup escaped through a symlinked parent directory"
if bash "${mcp_env_check}" "${preflight_workspace}" >"${test_root}/preflight-missing.log" 2>&1; then
    fail "MCP credential preflight accepted a missing .env.local"
fi
grep -Fq '.env.local.example' "${test_root}/preflight-missing.log" \
    || fail "missing credential file error is not actionable"
source_symlink_target="${test_root}/source-credentials"
printf 'GITHUB_PERSONAL_ACCESS_TOKEN=github-test\nZAI_API_KEY=zai-test\n' \
    >"${source_symlink_target}"
chmod 644 "${source_symlink_target}"
ln -s "${source_symlink_target}" "${preflight_workspace}/.env.local"
if bash "${mcp_env_check}" "${preflight_workspace}" >"${test_root}/preflight-source-symlink.log" 2>&1; then
    fail "MCP credential preflight accepted a symlinked .env.local"
fi
[ "$(stat -c '%a' "${source_symlink_target}")" = "644" ] \
    || fail "MCP credential preflight chmodded a symlink target"
[ ! -e "${preflight_workspace}/.devcontainer/.mcp.env" ] \
    || fail "symlinked source generated MCP credentials"
rm -f -- "${preflight_workspace}/.env.local"
printf 'GITHUB_PERSONAL_ACCESS_TOKEN=github-test\r\nZAI_API_KEY=zai-test\r\nUNRELATED_SECRET=must-not-propagate\r\n' \
    >"${preflight_workspace}/.env.local"
chmod 644 "${preflight_workspace}/.env.local"
credential_probe_bin="${test_root}/credential_probe_bin"
credential_probe_bin="${test_root}/credential-probe-bin"
mkdir -p "${credential_probe_bin}"
real_sed="$(command -v sed)"
cat >"${credential_probe_bin}/sed" <<EOF
#!/usr/bin/env bash
set -euo pipefail
[ -z "\${github_token:-}" ] && [ -z "\${zai_token:-}" ] && [ -z "\${value:-}" ] || exit 97
exec "${real_sed}" "\$@"
EOF
chmod 755 "${credential_probe_bin}/sed"
PATH="${credential_probe_bin}:${PATH}" \
github_token=ambient zai_token=ambient value=ambient \
    bash "${mcp_env_check}" "${preflight_workspace}"
[ "$(stat -c '%a' "${preflight_workspace}/.env.local")" = "600" ] \
    || fail "MCP credential preflight did not restrict .env.local permissions"
[ "$(stat -c '%a' "${preflight_workspace}/.devcontainer/.mcp.env")" = "600" ] \
    || fail "generated MCP environment permissions are not 600"
[ "$(wc -l <"${preflight_workspace}/.devcontainer/.mcp.env")" = "2" ] \
    || fail "generated MCP environment contains unexpected assignments"
grep -Fxq 'GITHUB_PERSONAL_ACCESS_TOKEN=github-test' \
    "${preflight_workspace}/.devcontainer/.mcp.env" \
    || fail "generated MCP environment omitted the GitHub credential"
grep -Fxq 'ZAI_API_KEY=zai-test' "${preflight_workspace}/.devcontainer/.mcp.env" \
    || fail "generated MCP environment omitted the Z.ai credential"
if grep -Fq 'must-not-propagate' "${preflight_workspace}/.devcontainer/.mcp.env"; then
    fail "generated MCP environment propagated an unrelated secret"
fi
printf 'ZAI_API_KEY=\n' >>"${preflight_workspace}/.env.local"
if bash "${mcp_env_check}" "${preflight_workspace}" >"${test_root}/preflight-duplicate.log" 2>&1; then
    fail "MCP credential preflight accepted a duplicate key"
fi
[ "$(stat -c '%a' "${preflight_workspace}/.env.local")" = "600" ] \
    || fail "MCP credential preflight rejected content before restricting source permissions"
if grep -Fq 'zai-test' "${test_root}/preflight-duplicate.log"; then
    fail "MCP credential preflight leaked a secret value"
fi
[ ! -e "${preflight_workspace}/.devcontainer/.mcp.env" ] \
    || fail "invalid source left stale generated MCP credentials available"
printf 'GITHUB_PERSONAL_ACCESS_TOKEN=github-test\nZAI_API_KEY=zai-test\n' \
    >"${preflight_workspace}/.env.local"
preflight_bin="${test_root}/preflight-bin"
mkdir -p "${preflight_bin}"
cat >"${preflight_bin}/mv" <<'STUB'
#!/usr/bin/env bash
exit 73
STUB
chmod 755 "${preflight_bin}/mv"
if PATH="${preflight_bin}:${PATH}" \
    bash "${mcp_env_check}" "${preflight_workspace}" >"${test_root}/preflight-move.log" 2>&1; then
    fail "MCP credential preflight ignored an atomic replacement failure"
fi
if find "${preflight_workspace}/.devcontainer" -maxdepth 1 -name '.mcp.env.*' -print -quit \
    | grep -q .; then
    fail "MCP credential preflight left a secret-bearing temporary file"
fi
[ ! -e "${preflight_workspace}/.devcontainer/.mcp.env" ] \
    || fail "atomic replacement failure left generated MCP credentials available"

cat >"${preflight_bin}/mv" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
/bin/mv "$@"
kill -TERM "${PPID}"
exit 0
STUB
chmod 755 "${preflight_bin}/mv"
if PATH="${preflight_bin}:${PATH}" \
    bash "${mcp_env_check}" "${preflight_workspace}" >"${test_root}/preflight-post-move-signal.log" 2>&1; then
    fail "MCP credential preflight reported success after post-rename termination"
fi
[ ! -e "${preflight_workspace}/.devcontainer/.mcp.env" ] \
    || fail "post-rename termination left generated MCP credentials available"

cat >"${preflight_bin}/mv" <<'STUB'
#!/usr/bin/env bash
kill -TERM "${PPID}"
exit 0
STUB
chmod 755 "${preflight_bin}/mv"
if PATH="${preflight_bin}:${PATH}" \
    bash "${mcp_env_check}" "${preflight_workspace}" >"${test_root}/preflight-signal.log" 2>&1; then
    fail "MCP credential preflight reported success after a termination signal"
fi
[ ! -e "${preflight_workspace}/.devcontainer/.mcp.env" ] \
    || fail "terminated preflight left generated MCP credentials available"
if find "${preflight_workspace}/.devcontainer" -maxdepth 1 -name '.mcp.env.*' -print -quit \
    | grep -q .; then
    fail "terminated preflight left a secret-bearing temporary file"
fi

symlink_target="${test_root}/must-not-overwrite"
printf 'sentinel\n' >"${symlink_target}"
ln -s "${symlink_target}" "${preflight_workspace}/.devcontainer/.mcp.env"
if bash "${mcp_env_check}" "${preflight_workspace}" >"${test_root}/preflight-symlink.log" 2>&1; then
    fail "MCP credential preflight accepted a symlinked generated file"
fi
[ -L "${preflight_workspace}/.devcontainer/.mcp.env" ] \
    || fail "MCP credential preflight followed or replaced a generated-file symlink"
grep -Fxq sentinel "${symlink_target}" \
    || fail "MCP credential preflight overwrote a symlink target"
rm -f -- "${preflight_workspace}/.devcontainer/.mcp.env"

cat >"${test_bin}/zai-mcp-server" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
{
    printf 'alias=%s\n' "${Z_AI_API_KEY:+set}"
    printf 'canonical=%s\n' "${ZAI_API_KEY-unset}"
    printf 'github=%s\n' "${GITHUB_PERSONAL_ACCESS_TOKEN-unset}"
    printf 'bridge=%s\n' "${ZAI_MCP_API_KEY-unset}"
    printf 'anthropic_auth=%s\n' "${ANTHROPIC_AUTH_TOKEN-unset}"
    printf 'anthropic_api=%s\n' "${ANTHROPIC_API_KEY-unset}"
    printf 'claude_oauth=%s\n' "${CLAUDE_CODE_OAUTH_TOKEN-unset}"
    printf 'github_pat=%s\n' "${GITHUB_PAT-unset}"
    printf 'github_token=%s\n' "${GITHUB_TOKEN-unset}"
    printf 'gh_token=%s\n' "${GH_TOKEN-unset}"
    printf 'openai=%s\n' "${OPENAI_API_KEY-unset}"
    printf 'codex=%s\n' "${CODEX_API_KEY-unset}"
    printf 'lowercase_vision_key=%s\n' "${zai_api_key-unset}"
    printf 'mode=%s\n' "${Z_AI_MODE-unset}"
    printf 'arg=%s\n' "$@"
} >"${STUB_LOG:?}"
STUB
chmod 755 "${test_bin}/zai-mcp-server"
vision_log="${test_root}/vision.log"
PATH="${test_bin}:/usr/bin:/bin" \
STUB_LOG="${vision_log}" \
ZAI_API_KEY="vision-test-secret" \
GITHUB_PERSONAL_ACCESS_TOKEN="github-test-secret" \
ZAI_MCP_API_KEY="bridge-test-secret" \
ANTHROPIC_AUTH_TOKEN="anthropic-auth-test-secret" \
ANTHROPIC_API_KEY="anthropic-api-test-secret" \
CLAUDE_CODE_OAUTH_TOKEN="claude-oauth-test-secret" \
GITHUB_PAT="github-pat-test-secret" \
GITHUB_TOKEN="github-token-test-secret" \
GH_TOKEN="gh-token-test-secret" \
OPENAI_API_KEY="openai-test-secret" \
CODEX_API_KEY="codex-test-secret" \
zai_api_key="ambient-lowercase-secret" \
    bash "${vision_launcher}" --probe
grep -Eq '^alias=set$' "${vision_log}" || fail "Vision MCP did not receive the Z.ai key alias"
grep -Eq '^canonical=unset$' "${vision_log}" || fail "Vision MCP retained the canonical Z.ai key"
grep -Eq '^github=unset$' "${vision_log}" || fail "Vision MCP received the unrelated GitHub credential"
for scrubbed in bridge anthropic_auth anthropic_api claude_oauth github_pat github_token gh_token openai codex lowercase_vision_key; do
    grep -Fxq "${scrubbed}=unset" "${vision_log}" \
        || fail "Vision MCP exposed ${scrubbed} to the server"
done
grep -Eq '^mode=ZAI$' "${vision_log}" || fail "Vision MCP did not receive Z.ai mode"
grep -Eq '^arg=--probe$' "${vision_log}" || fail "Vision MCP wrapper did not forward arguments"
if grep -Eq 'vision-test-secret|github-test-secret' "${vision_log}"; then
    fail "Vision MCP wrapper leaked the secret"
fi

runtime_workspace="${test_root}/runtime-workspace"
mkdir -p "${runtime_workspace}/.devcontainer"
printf 'GITHUB_PERSONAL_ACCESS_TOKEN=unused\nZAI_API_KEY=bridge-test-secret\n' \
    >"${runtime_workspace}/.devcontainer/.mcp.env"
chmod 600 "${runtime_workspace}/.devcontainer/.mcp.env"
PATH="${test_bin}:/usr/bin:/bin" \
STUB_LOG="${vision_log}" \
CLAUDE_PROJECT_DIR="${runtime_workspace}" \
    env -u ZAI_API_KEY -u Z_AI_API_KEY bash "${vision_launcher}" --runtime-probe
grep -Eq '^alias=set$' "${vision_log}" \
    || fail "Vision MCP did not load the filtered runtime credential"

cat >"${test_bin}/mcp-remote" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
endpoint="${1:?}"
[ "${2:-}" = "--header-file" ] || exit 91
header_file="${3:?}"
if [ "$(sed -n '1p' "${header_file}")" = 'Authorization: Bearer bridge-test-secret' ]; then
    header_status=valid
else
    header_status=invalid
fi
{
    printf 'endpoint=%s\n' "${endpoint}"
    printf 'header=%s\n' "${header_status}"
    printf 'bridge_key=%s\n' "${ZAI_MCP_API_KEY-unset}"
    printf 'auth=%s\n' "${ANTHROPIC_AUTH_TOKEN-unset}"
    printf 'canonical=%s\n' "${ZAI_API_KEY-unset}"
    printf 'alias=%s\n' "${Z_AI_API_KEY-unset}"
    printf 'github=%s\n' "${GITHUB_PERSONAL_ACCESS_TOKEN-unset}"
    printf 'claude_oauth=%s\n' "${CLAUDE_CODE_OAUTH_TOKEN-unset}"
    printf 'github_pat=%s\n' "${GITHUB_PAT-unset}"
    printf 'github_token=%s\n' "${GITHUB_TOKEN-unset}"
    printf 'gh_token=%s\n' "${GH_TOKEN-unset}"
    printf 'openai=%s\n' "${OPENAI_API_KEY-unset}"
    printf 'codex=%s\n' "${CODEX_API_KEY-unset}"
    printf 'lowercase_token=%s\n' "${token-unset}"
    printf 'internal_token=%s\n' "${zai_mcp_token-unset}"
    printf 'header_arg=%s\n' "${header_file}"
} >"${STUB_LOG:?}"
STUB
chmod 755 "${test_bin}/mcp-remote"
for mapping in \
    'web-search|https://api.z.ai/api/mcp/web_search_prime/mcp' \
    'web-reader|https://api.z.ai/api/mcp/web_reader/mcp' \
    'zread|https://api.z.ai/api/mcp/zread/mcp'; do
    alias_name="${mapping%%|*}"
    expected_endpoint="${mapping#*|}"
    remote_log="${test_root}/remote-${alias_name}.log"
    PATH="${test_bin}:/usr/bin:/bin" \
    STUB_LOG="${remote_log}" \
    CLAUDE_PROJECT_DIR="${runtime_workspace}" \
    ZAI_MCP_API_KEY="hostile-bridge-secret" \
    ANTHROPIC_AUTH_TOKEN="model-test-secret" \
    ZAI_API_KEY="canonical-test-secret" \
    Z_AI_API_KEY="alias-test-secret" \
    GITHUB_PERSONAL_ACCESS_TOKEN="github-test-secret" \
    CLAUDE_CODE_OAUTH_TOKEN="claude-oauth-test-secret" \
    GITHUB_PAT="github-pat-test-secret" \
    GITHUB_TOKEN="github-token-test-secret" \
    GH_TOKEN="gh-token-test-secret" \
    OPENAI_API_KEY="openai-test-secret" \
    CODEX_API_KEY="codex-test-secret" \
    token="ambient-lowercase-secret" \
    zai_mcp_token="ambient-internal-secret" \
        bash "${remote_launcher}" "${alias_name}"
    grep -Fxq "endpoint=${expected_endpoint}" "${remote_log}" \
        || fail "Remote MCP wrapper mapped ${alias_name} to the wrong endpoint"
    grep -Fxq 'header=valid' "${remote_log}" \
        || fail "Remote MCP wrapper did not provide its authorization header"
    for scrubbed in bridge_key auth canonical alias github claude_oauth github_pat github_token gh_token openai codex lowercase_token internal_token; do
        grep -Fxq "${scrubbed}=unset" "${remote_log}" \
            || fail "Remote MCP wrapper exposed ${scrubbed} to the bridge"
    done
    if grep -Eq 'bridge-test-secret|model-test-secret|canonical-test-secret|alias-test-secret|github-test-secret' \
        "${remote_log}"; then
        fail "Remote MCP wrapper leaked a credential"
    fi
done

runtime_remote_log="${test_root}/remote-runtime.log"
PATH="${test_bin}:/usr/bin:/bin" \
STUB_LOG="${runtime_remote_log}" \
CLAUDE_PROJECT_DIR="${runtime_workspace}" \
    env -u ZAI_MCP_API_KEY -u ZAI_API_KEY -u Z_AI_API_KEY \
    bash "${remote_launcher}" web-search
grep -Fxq 'header=valid' "${runtime_remote_log}" \
    || fail "Remote MCP wrapper did not load the filtered runtime credential"

codex_remote_log="${test_root}/remote-codex-env.log"
PATH="${test_bin}:/usr/bin:/bin" \
STUB_LOG="${codex_remote_log}" \
ZAI_API_KEY="bridge-test-secret" \
    env -u CLAUDE_PROJECT_DIR -u Z_AI_API_KEY -u ZAI_MCP_API_KEY \
    bash "${remote_launcher}" web-search
grep -Fxq 'header=valid' "${codex_remote_log}" \
    || fail "Remote MCP wrapper did not accept Codex's explicitly forwarded credential"
grep -Fxq 'canonical=unset' "${codex_remote_log}" \
    || fail "Remote MCP wrapper exposed Codex's credential to the bridge environment"
if PATH="${test_bin}:/usr/bin:/bin" STUB_LOG="${codex_remote_log}" \
    ZAI_API_KEY='quoted secret' \
    env -u CLAUDE_PROJECT_DIR -u Z_AI_API_KEY -u ZAI_MCP_API_KEY \
    bash "${remote_launcher}" web-search >"${test_root}/remote-codex-malformed.log" 2>&1; then
    fail "Remote MCP wrapper accepted a malformed Codex-forwarded credential"
fi

chmod 644 "${runtime_workspace}/.devcontainer/.mcp.env"
if PATH="${test_bin}:/usr/bin:/bin" STUB_LOG="${vision_log}" \
    CLAUDE_PROJECT_DIR="${runtime_workspace}" env -u ZAI_API_KEY -u Z_AI_API_KEY \
    bash "${vision_launcher}" >"${test_root}/vision-mode.log" 2>&1; then
    fail "Vision MCP accepted an over-readable runtime credential file"
fi
if PATH="${test_bin}:/usr/bin:/bin" STUB_LOG="${runtime_remote_log}" \
    CLAUDE_PROJECT_DIR="${runtime_workspace}" \
    bash "${remote_launcher}" web-search >"${test_root}/remote-mode.log" 2>&1; then
    fail "Remote MCP accepted an over-readable runtime credential file"
fi
chmod 600 "${runtime_workspace}/.devcontainer/.mcp.env"
printf 'GITHUB_PERSONAL_ACCESS_TOKEN=unused\nZAI_API_KEY="quoted secret"\n' \
    >"${runtime_workspace}/.devcontainer/.mcp.env"
if PATH="${test_bin}:/usr/bin:/bin" STUB_LOG="${vision_log}" \
    CLAUDE_PROJECT_DIR="${runtime_workspace}" env -u ZAI_API_KEY -u Z_AI_API_KEY \
    bash "${vision_launcher}" >"${test_root}/vision-malformed.log" 2>&1; then
    fail "Vision MCP accepted a malformed runtime credential"
fi
if PATH="${test_bin}:/usr/bin:/bin" STUB_LOG="${runtime_remote_log}" \
    CLAUDE_PROJECT_DIR="${runtime_workspace}" \
    bash "${remote_launcher}" web-search >"${test_root}/remote-malformed.log" 2>&1; then
    fail "Remote MCP accepted a malformed runtime credential"
fi

if PATH="${test_bin}:/usr/bin:/bin" STUB_LOG="${runtime_remote_log}" \
    env -u CLAUDE_PROJECT_DIR -u ZAI_MCP_API_KEY -u ZAI_API_KEY -u Z_AI_API_KEY \
    bash "${remote_launcher}" web-search >"${test_root}/remote-missing.log" 2>&1; then
    fail "Remote MCP wrapper accepted a missing key"
fi
grep -Fq 'A filtered Claude runtime credential or Codex-forwarded ZAI_API_KEY is required.' \
    "${test_root}/remote-missing.log" \
    || fail "Remote MCP missing-key failure did not come from credential validation"
missing_runtime_workspace="${test_root}/missing-runtime-workspace"
mkdir -p "${missing_runtime_workspace}"
if PATH="${test_bin}:/usr/bin:/bin" STUB_LOG="${runtime_remote_log}" \
    CLAUDE_PROJECT_DIR="${missing_runtime_workspace}" \
    ZAI_API_KEY="bridge-test-secret" \
    bash "${remote_launcher}" web-search >"${test_root}/remote-claude-no-fallback.log" 2>&1; then
    fail "Remote MCP wrapper fell back to an ambient key for missing Claude runtime credentials"
fi
grep -Fq 'Filtered MCP credentials are missing' "${test_root}/remote-claude-no-fallback.log" \
    || fail "Remote MCP Claude source-precedence failure was not actionable"
if PATH="${test_bin}:/usr/bin:/bin" ZAI_MCP_API_KEY=placeholder \
    bash "${remote_launcher}" unknown >"${test_root}/remote-unknown.log" 2>&1; then
    fail "Remote MCP wrapper accepted an unknown endpoint alias"
fi

printf 'PASS: isolated AI backends and project MCP catalogs\n'
