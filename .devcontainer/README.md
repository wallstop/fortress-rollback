# Fortress Rollback Development Container

This devcontainer provides a complete environment for developing, testing, and formally verifying Fortress Rollback.

## Included Tools

### Formal Verification

- **TLA+ Model Checker (TLC)** - For model checking concurrent protocols
- **Kani** - Bounded model checker for Rust
- **Miri** - Undefined behavior detection for Rust
- **Z3** - SMT solver (via Python bindings)

### Testing

- **cargo-nextest** - Fast parallel test runner
- **cargo-tarpaulin** - Code coverage
- **cargo-llvm-cov** - LLVM-instrumented coverage
- **cargo-mutants** - Mutation testing
- **cargo-fuzz** - Fuzz testing (nightly required)
- **proptest** - Property-based testing (via Cargo.toml)
- **loom** - Concurrency testing (via Cargo.toml)

### Security & Quality

- **cargo-audit** - Vulnerability scanning
- **cargo-deny** - License/dependency checking
- **cargo-geiger** - Unsafe code auditing
- **cargo-shear** - Unused dependency detection
- **cargo-spellcheck** - Documentation spelling
- **cargo-careful** - Extra runtime safety checks
- **clippy** - Rust linter
- **rustfmt** - Code formatter

### CI/CD Linting

- **actionlint** - GitHub Actions workflow linting
- **yamllint** - YAML file linting
- **markdownlint** - Markdown file linting
- **markdown-link-check** - Verify markdown links
- **pre-commit** - Pre-commit hook framework

### AI Coding CLIs

- **Nanocoder** - Local-first coding agent (`@nanocollective/nanocoder`)
- **OpenCode** - Open source coding agent (`opencode-ai`)
- **Codex CLI** - OpenAI coding agent (`@openai/codex`)
- **Claude Code** - Anthropic coding agent (`@anthropic-ai/claude-code`)
- **Copilot CLI** - GitHub coding agent (`@github/copilot`)
- **Z.ai Vision MCP** - Local vision tools (`@z_ai/mcp-server`)

### Profiling

- **flamegraph** - Flame graph generation
- **valgrind** - Memory debugging

### Network Testing

- **iproute2/tc** - Traffic control for network simulation
- **netcat** - Network diagnostics
- **tcpdump** - Packet capture

## Quick Start

After the container starts, verify all tools:

```bash
./scripts/ci/check-tools.sh
```

## AI CLI and npm Setup

The image build installs Nanocoder, OpenCode, Codex CLI, Claude Code, Copilot
CLI, and the Z.ai Vision MCP from each npm package's `latest` tag. The create
hook resolves the tags again, so a cached
image does not freeze their versions. Each package refresh is failure-tolerant:
an unavailable registry does not prevent the container from opening, and the
image-bundled version remains available. Routine start hooks skip registry
access so reopening an existing container stays fast and offline-safe.

```bash
nanocoder --version
opencode --version
codex --version
claude --version
copilot --version
```

The ordinary `codex` and `claude` commands keep their native OpenAI and
Anthropic backends. Create/start hooks also install opt-in `codex-zai` and
`claude-zai` launchers:

```bash
[ -e .env.local ] || cp .env.local.example .env.local
# Add or update ZAI_API_KEY and GITHUB_PERSONAL_ACCESS_TOKEN.
codex-zai
claude-zai
```

Switching is per process; exiting returns immediately to the native commands.
Codex uses a named profile with Z.ai's Responses endpoint. Claude uses Z.ai's
Anthropic endpoint and separate `~/.claude-zai` state. Credentials remain in
the process environment and are not written to the repository or generated
profile. Optional overrides are `CODEX_ZAI_MODEL`,
`CODEX_ZAI_REASONING_EFFORT` (`low`, `high`, or `max`),
`ZAI_API_TIMEOUT_MS` (five minutes by default), and
`CLAUDE_ZAI_CONFIG_DIR`. Claude's normal aliases map to Z.ai's recommended GLM
models; use `CLAUDE_ZAI_HAIKU_MODEL`, `CLAUDE_ZAI_SONNET_MODEL`, or
`CLAUDE_ZAI_OPUS_MODEL` to override them. Ordinary shell and unrelated MCP
children do not inherit the model-server credential. The trusted Z.ai MCP
servers receive only their required credential. For Claude, their wrappers read
the filtered mode-`0600` runtime file through `CLAUDE_PROJECT_DIR`; Codex
forwards only `ZAI_API_KEY` through its MCP credential allowlist for the remote
servers and keeps the direct stdio wrapper for Vision. The wrappers scrub the
canonical key and unrelated tokens before starting the MCP process.

Claude's experimental Linux subprocess scrubber depends on `bubblewrap` and
`socat`. Current Claude Code still requires user and mount namespaces in its
weaker nested mode, which standard Docker confinement rejects. Inside the
devcontainer, `claude-zai` therefore disables that redundant inner scrub sandbox
and relies on the outer container boundary; controlled Bash and stdio MCP probes
verified that Claude still removes the model-server credential from ordinary
shell children while deliberately authenticating the selected Z.ai MCP servers.
Outside a container the scrubber remains enabled. An explicit
`CLAUDE_ZAI_SUBPROCESS_ENV_SCRUB=1` request runs a bubblewrap preflight and fails
before model startup if the runtime cannot support it.

Global npm installs use the `vscode` user's `~/.local` prefix and
`~/.cache/npm` cache. You can install and update packages without `sudo`:

```bash
npm install --global <package>
npm update --global
npm config get prefix
```

The prefix command should print `/home/vscode/.local`.

### Codex authentication

Authentication options:

- ChatGPT login: `codex login`
- Headless-friendly login: `codex login --device-auth`
- API key login (non-interactive): `printenv OPENAI_API_KEY | codex login --with-api-key`
- One-shot non-interactive runs: set `CODEX_API_KEY` and run `codex exec ...`

Notes:

- Codex auth cache is persisted at `~/.codex` using a dedicated devcontainer volume.
- `.devcontainer/devcontainer-bootstrap.sh` refreshes all three AI CLIs on
  create and bounds Cargo artifacts on create/start. `.devcontainer/codex-bootstrap.sh`
  then prints Codex readiness guidance.

### Agent network reliability

OpenCode and Nanocoder use the container's ordinary outbound path. Docker's
default bridge masquerades that traffic through the host, so a failing host
uplink also fails inside the devcontainer.

On Linux, do not keep wired Ethernet and Wi-Fi active together when both have
addresses on the same IPv4 subnet. Each link can work alone while their combined
failure pattern remains consistent with weak-host ARP or return-path ambiguity;
a packet capture is required to identify the exact mechanism. Confirm the
topology, the route carrying the current shell, and each interface independently:

```bash
ip -brief -4 address
ip -4 route show default
ip -4 route show scope link
nmcli device status
model_ip=$(getent ahostsv4 api.z.ai | awk 'NR == 1 {print $1}')
ip route get "$model_ip"
wired_interface=eth0
gateway_ip=192.168.1.1
ping -I "$wired_interface" -c 15 "$gateway_ip"
curl --interface "$wired_interface" --ipv4 --connect-timeout 3 --max-time 10 \
    --show-error \
    --write-out '\ncode=%{http_code} total=%{time_total} error=%{errormsg}\n' \
    https://api.z.ai/api/coding/paas/v4/models
```

An expected HTTP 401 proves only DNS, TCP, TLS, and HTTP reachability, not an
authenticated streaming model call. If the shell is remote, do not disconnect
the interface carrying its route. Verify the alternate link first and keep the
restore commands below available before changing NetworkManager state.

When both links are healthy in isolation, keep only one active on that LAN. To
prefer wired Ethernet persistently with NetworkManager:

```bash
nmcli connection modify "<wifi-profile>" connection.autoconnect no
nmcli connection down "<wifi-profile>"
```

Restore Wi-Fi later with:

```bash
nmcli connection modify "<wifi-profile>" connection.autoconnect yes
nmcli connection up "<wifi-profile>"
```

For automatic wired preference with Wi-Fi failover, this host currently uses
the `prefer-wired-network.service` systemd user service maintained alongside the
NovaSharp devcontainer tooling. Its policy is configurable in
`~/.config/prefer-wired-network/config`; it restores Wi-Fi only if it was the
component that disabled the radio. This is a host repair, so Fortress containers
inherit it without container-specific networking changes.

The repository's `opencode.json` sets a five-minute Z.ai request timeout and a
45-second streamed-chunk timeout.
`.opencode/plugins/bounded-network-retries.js` stops matching network retries
after the third failed attempt or two minutes after retrying begins. These
bounds apply only to OpenCode and turn a silent stall into an actionable abort;
they do not control Nanocoder or repair the host uplink. The single-uplink host
repair covers both clients. After repairing it, repeat interface-bound and
in-container probes, then run authenticated streaming and read-only tool
workloads in both clients. Record counts, latency, and failure class without
recording credentials or response bodies.

### Project MCP servers

Before devcontainer creation, the host preflight copies only the two allowlisted
MCP credentials from the ignored `.env.local` into an ignored mode-`0600`
runtime file. Unrelated project credentials are not injected. The container
exposes GitHub's hosted MCP server plus all four MCPs currently included with
the Z.ai Coding Plan: Vision Understanding, Web Search Prime, Web Reader, and
Zread. Vision is preinstalled as `@z_ai/mcp-server`; the remote servers use
their official HTTPS endpoints. `.mcp.json`, `.codex/config.toml`,
`opencode.json`, and `.vscode/mcp.json` make the same catalog available to
Claude Code, Nanocoder, Copilot CLI, Codex, OpenCode, and VS Code. Nanocoder
receives the catalog's absolute path, so launching it below the repository root
still works.

Claude Code 2.1.260 cannot consume the SSE-framed MCP initialization response
currently returned by the three remote Z.ai endpoints, so `claude-zai` keeps using
the installed `mcp-remote` stdio bridge for those three servers. Codex 0.153.2
completed the response but lost the transport while sending the subsequent
`initialized` notification; after upgrading past that release, the installed
Codex CLI (currently `codex-cli 0.166.0`) accepts the native streamable HTTP
transport, and the repository now declares the Z.ai remote servers with
`url` plus `bearer_token_env_var` instead of a stdio bridge. A stdio bridge
declared alongside a `url` makes Codex fail the whole MCP bootstrap with
`url is not supported for stdio`, which prevented Codex from running at all.
Direct HTTP probes and OpenCode succeed from the same container, so only Claude
keeps the bridge. Vision remains direct stdio and GitHub remains native HTTP.
Re-test native HTTP after either client upgrades before removing a bridge.

In Claude Code 2.1.260, `claude-zai mcp list` ignores this per-session strict
catalog and health-checks `.mcp.json`, so it can falsely report the three native
HTTP failures. The authoritative check is the `system/init` event from an actual
`claude-zai --print --output-format stream-json --verbose` session: its
`mcp_servers` array must show all five entries as `connected`.

Keep model access separate from MCP health. In the 2026-09-04 live probe, all
five MCPs connected before Z.ai's model API returned HTTP 429, code `1310`, for
an exhausted weekly/monthly subscription allowance. Claude retries that response
and can look hung. A rebuild, key rotation, or MCP change cannot repair quota;
interrupt the retries, check the Z.ai plan's usage/reset time, and retry after
capacity is available.

If `.env.local` does not already exist, copy `.env.local.example`; otherwise add
or update the two unquoted MCP keys without overwriting the existing file. Then
rebuild the container. Claude asks once to approve project MCP servers, and
Codex requires the repository to be trusted; those security prompts are
intentionally not bypassed. GitHub is read-only by default through
`X-MCP-Readonly: true`. Remove that header only for intentional mutations and
use a least-privilege token. The two keys enter the trusted container process
environment and remain readable from the mounted workspace, so do not run
untrusted repository code or agents.

The preceding Claude approval statement applies to native `claude`.
`claude-zai` pre-approves only the five audited names in its isolated settings.

Sources: [Z.ai Coding Plan MCP overview](https://docs.z.ai/devpack/overview),
[Claude Code MCP configuration](https://code.claude.com/docs/en/mcp),
[Codex MCP configuration](https://developers.openai.com/codex/mcp/),
[Claude Code native SSE failure](https://github.com/anthropics/claude-code/issues/58759),
[`mcp-remote`](https://github.com/geelen/mcp-remote),
[GitHub MCP server](https://github.com/github/github-mcp-server), and
[Dev Container environment files](https://code.visualstudio.com/remote/advancedcontainers/environment-variables).

## Cargo Artifact Size Policy

The container keeps Rust builds smaller by disabling incremental compilation
and using line-table-only debug information for dev, test, and benchmark
profiles. The create hook runs `cargo fetch --locked` instead of compiling the
workspace, which warms the dependency cache without populating `target/`.

On create and start, the maintenance hook measures `target/`. When it exceeds 8
GiB, the hook runs `cargo clean --target-dir <target>` at most once every seven
days. The cleanup only removes derived Cargo artifacts; source files and Cargo's
download cache remain untouched. Registry failures and cleanup failures never
block container startup.

For safety, automatic cleanup operates only on the exact `target/` directory
under the resolved workspace. A `CARGO_TARGET_DIR` elsewhere is never cleaned
automatically, even when it contains Cargo's standard cache marker.

You can tune or disable the policy through container environment variables:

| Variable                               | Default | Purpose                                       |
|----------------------------------------|---------|-----------------------------------------------|
| `FORTRESS_TARGET_MAX_BYTES`            | 8 GiB   | Cleanup threshold in bytes                    |
| `FORTRESS_TARGET_CLEAN_MIN_AGE_DAYS`   | `7`     | Minimum days between successful cleanups      |
| `FORTRESS_NPM_REFRESH_TIMEOUT_SECONDS` | `120`   | Time limit for each npm package refresh       |
| `DEVCONTAINER_SKIP_TARGET_CLEANUP`     | `0`     | Set to `1` to disable automatic Cargo cleanup |
| `DEVCONTAINER_SKIP_TOOL_REFRESH`       | `0`     | Set to `1` to keep image-bundled CLI versions |

## Build Performance

The Dockerfile uses **cargo-binstall** to download pre-built binaries instead of
compiling tools from source, reducing build time from ~25 minutes to ~3-5 minutes.

| Method                     | Build Time  | When to Use                                    |
| -------------------------- | ----------- | ---------------------------------------------- |
| **Pre-built image (GHCR)** | ~30 seconds | Optional fast path when the image is available |
| **Local Dockerfile build** | ~3-5 min    | When modifying the Dockerfile                  |
| **Without binstall (old)** | ~25-30 min  | N/A (deprecated)                               |

### Using the pre-built image (fastest)

The local Dockerfile build remains the default path until the published image has been validated across the environments this project supports.

In `devcontainer.json`, comment out the `"build"` block and uncomment the `"image"` line:

```jsonc
// "build": { ... },
"image": "ghcr.io/wallstop/fortress-rollback/devcontainer:latest",
```

The image is automatically rebuilt weekly and on changes to `.devcontainer/` files.

> **Note:** The pre-built GHCR image is x86_64 (amd64) only. Apple Silicon (arm64)
> users should use the local Dockerfile build, which includes architecture detection.

### Building locally

The default `devcontainer.json` builds from the Dockerfile. Docker layer caching
means subsequent rebuilds only reprocess changed layers.

## Running Verification

### All Verifiers

```bash
./scripts/verification/verify-all.sh
```

### TLA+ Only

```bash
./scripts/verification/verify-tla.sh
```

### Kani Only

```bash
./scripts/verification/verify-kani.sh
# Or directly:
cargo kani
```

### Miri

```bash
cargo +nightly miri test
```

### Tests with Coverage

```bash
cargo tarpaulin --out Html
# or
cargo llvm-cov --html
```

## Environment Variables

| Variable            | Default                              | Description                               |
| ------------------- | ------------------------------------ | ----------------------------------------- |
| `TLA_TOOLS_JAR`     | `.tla-tools/tla2tools.jar`           | Path to TLA+ tools                        |
| `RUST_BACKTRACE`    | `1`                                  | Enable backtraces                         |
| `CARGO_INCREMENTAL` | `0`                                  | Disable incremental (for reproducibility) |
| `MIRIFLAGS`         | `-Zmiri-symbolic-alignment-check...` | Miri configuration                        |
| `TLA_WORKERS`       | `auto`                               | TLC worker threads                        |
| `TLA_MEMORY`        | `4g`                                 | TLC JVM heap size                         |
| `KANI_TIMEOUT`      | `300`                                | Kani proof timeout (seconds)              |

## Manual Tool Installation

If any tools are missing, install them:

```bash
# Kani
cargo install --locked kani-verifier
cargo kani setup

# Miri
rustup +nightly component add miri

# Other cargo tools
cargo install cargo-nextest cargo-audit cargo-deny cargo-llvm-cov cargo-mutants
cargo install cargo-shear cargo-spellcheck cargo-geiger cargo-careful

# CI/CD linting tools
pip3 install --break-system-packages yamllint pre-commit
# AI tools use the devcontainer's user-owned npm prefix (no sudo)
npm install --global \
  --allow-scripts=opencode-ai,@anthropic-ai/claude-code \
  markdownlint-cli@latest \
  markdown-link-check@latest \
  @nanocollective/nanocoder@latest \
  opencode-ai@latest \
  @openai/codex@latest \
  @anthropic-ai/claude-code@latest

# actionlint (GitHub Actions linter)
# Download the script first, then run with bash (avoids shell issues)
curl --proto '=https' --tlsv1.2 -sSfL https://raw.githubusercontent.com/rhysd/actionlint/main/scripts/download-actionlint.bash -o /tmp/download-actionlint.bash
bash /tmp/download-actionlint.bash latest /tmp
sudo mv /tmp/actionlint /usr/local/bin/
rm /tmp/download-actionlint.bash

# Z3
pip3 install 'z3-solver==5.1.0.0'
```

## Troubleshooting

### Container Build Fails

#### Syntax error "(" unexpected

This error occurs when bash-specific syntax (like process substitution `<(...)`) is used
with `/bin/sh` (dash). The Dockerfile now uses `SHELL ["/bin/bash", "-c"]` to prevent this.

If you see this error in other scripts, ensure they are run with bash explicitly:

```bash
# Instead of piping to sh
curl --proto '=https' --tlsv1.2 -sSfL https://example.com/script.sh | bash

# Download first, then run with bash
curl --proto '=https' --tlsv1.2 -sSfL https://example.com/script.sh -o /tmp/script.sh
bash /tmp/script.sh
rm /tmp/script.sh
```

#### Docker Desktop WSL Integration Issues

If you're using Docker Desktop with WSL2 on Windows:

1. Ensure WSL2 backend is enabled in Docker Desktop settings
2. Enable integration with your WSL distro under **Resources > WSL Integration**
3. Restart Docker Desktop and VS Code if needed

#### Container fails to start after rebuild

Try removing old containers and images:

```bash
# List and remove devcontainer images
docker images | grep fortress-rollback | awk '{print $3}' | xargs -r docker rmi -f

# Or prune all unused images
docker system prune -a
```

### Kani Setup Fails

Kani requires downloading CBMC. Run:

```bash
cargo kani setup
```

### Miri Not Found

Ensure nightly is installed:

```bash
rustup install nightly
rustup +nightly component add miri
```

### TLA+ Tools Missing

Download manually:

```bash
mkdir -p .tla-tools
TLA_TOOLS_VERSION="$(tr -d '[:space:]' < .tla-tools-version)"
curl --proto '=https' --tlsv1.2 -sSfL "https://github.com/tlaplus/tlaplus/releases/download/v${TLA_TOOLS_VERSION}/tla2tools.jar" -o .tla-tools/tla2tools.jar
```

## Files

- `devcontainer.json` - Main configuration
- `Dockerfile` - Custom image with all tools
- `welcome.sh` - Startup information display
