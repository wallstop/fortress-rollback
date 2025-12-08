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
- **clippy** - Rust linter
- **rustfmt** - Code formatter

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

# Z3
pip3 install z3-solver
```

## Troubleshooting

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
