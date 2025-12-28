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
./scripts/check-tools.sh
```

## Running Verification

### All Verifiers

```bash
./scripts/verify-all.sh
```

### TLA+ Only

```bash
./scripts/verify-tla.sh
```

### Kani Only

```bash
./scripts/verify-kani.sh
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

| Variable | Default | Description |
|----------|---------|-------------|
| `TLA_TOOLS_JAR` | `.tla-tools/tla2tools.jar` | Path to TLA+ tools |
| `RUST_BACKTRACE` | `1` | Enable backtraces |
| `CARGO_INCREMENTAL` | `0` | Disable incremental (for reproducibility) |
| `MIRIFLAGS` | `-Zmiri-symbolic-alignment-check...` | Miri configuration |
| `TLA_WORKERS` | `auto` | TLC worker threads |
| `TLA_MEMORY` | `4g` | TLC JVM heap size |
| `KANI_TIMEOUT` | `300` | Kani proof timeout (seconds) |

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
npm install -g markdownlint-cli markdown-link-check

# actionlint (GitHub Actions linter)
# Download the script first, then run with bash (avoids shell issues)
curl -sL https://raw.githubusercontent.com/rhysd/actionlint/main/scripts/download-actionlint.bash -o /tmp/download-actionlint.bash
bash /tmp/download-actionlint.bash latest /tmp
sudo mv /tmp/actionlint /usr/local/bin/
rm /tmp/download-actionlint.bash

# Z3
pip3 install z3-solver
```

## Troubleshooting

### Container Build Fails

#### Syntax error "(" unexpected

This error occurs when bash-specific syntax (like process substitution `<(...)`) is used
with `/bin/sh` (dash). The Dockerfile now uses `SHELL ["/bin/bash", "-c"]` to prevent this.

If you see this error in other scripts, ensure they are run with bash explicitly:

```bash
# Instead of piping to sh
curl -sL https://example.com/script.sh | bash

# Download first, then run with bash
curl -sL https://example.com/script.sh -o /tmp/script.sh
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
docker images | grep vsc-ggrs | awk '{print $3}' | xargs -r docker rmi -f

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
curl -L https://github.com/tlaplus/tlaplus/releases/download/v1.8.0/tla2tools.jar -o .tla-tools/tla2tools.jar
```

## Files

- `devcontainer.json` - Main configuration
- `Dockerfile` - Custom image with all tools
- `welcome.sh` - Startup information display
