# TLA+ Tooling Ecosystem and Development Workflows

This document provides a comprehensive overview of TLA+ tools, IDE support, CI/CD integration patterns, and best practices for integrating TLA+ into development workflows.

## Table of Contents

1. [Available TLA+ Tools](#1-available-tla-tools)
2. [IDE Support and Extensions](#2-ide-support-and-extensions)
3. [CI/CD Integration Patterns](#3-cicd-integration-patterns)
4. [Command-Line Usage for Automation](#4-command-line-usage-for-automation)
5. [Development Workflow Integration Tips](#5-development-workflow-integration-tips)

---

## 1. Available TLA+ Tools

The TLA+ ecosystem includes several tools bundled in `tla2tools.jar`, plus additional standalone tools:

### Core Tools (in tla2tools.jar)

| Tool | Purpose | Command |
|------|---------|---------|
| **SANY** | Syntactic Analyzer - parses TLA+ specifications | `java -cp tla2tools.jar tla2sany.SANY -help` |
| **TLC** | Model Checker - exhaustive state space exploration | `java -cp tla2tools.jar tlc2.TLC -help` |
| **TLC REPL** | Interactive TLA+ expression evaluation | `java -cp tla2tools.jar tlc2.REPL` |
| **PlusCal Translator** | Converts PlusCal algorithms to TLA+ | `java -cp tla2tools.jar pcal.trans -help` |
| **TLATeX** | Converts TLA+ specs to LaTeX/PDF | `java -cp tla2tools.jar tla2tex.TLA -help` |

### Additional Tools

| Tool | Purpose | Source |
|------|---------|--------|
| **TLAPS** (TLA+ Proof System) | Formal proof verification | [proofs.tlapl.us](http://proofs.tlapl.us/) |
| **Apalache** | Symbolic model checker (bounded) | [github.com/apalache-mc/apalache](https://github.com/apalache-mc/apalache) |
| **TLAUC** | Unicode converter for TLA+ specs | [github.com/tlaplus-community/tlauc](https://github.com/tlaplus-community/tlauc) |
| **tree-sitter-tlaplus** | TLA+ grammar for syntax analysis | [github.com/tlaplus-community/tree-sitter-tlaplus](https://github.com/tlaplus-community/tree-sitter-tlaplus) |

### Tool Capabilities Summary

| Tool | Safety Properties | Liveness Properties | Bounded Checking | Exhaustive | Proofs |
|------|-------------------|---------------------|------------------|------------|--------|
| TLC | ✅ | ✅ | ❌ | ✅ | ❌ |
| Apalache | ✅ | ❌ | ✅ | ❌ | ❌ |
| TLAPS | ✅ | ✅ | N/A | N/A | ✅ |

---

## 2. IDE Support and Extensions

### TLA+ Toolbox (Official IDE)

The **TLA+ Toolbox** is the official Eclipse-based IDE maintained by the TLA+ Foundation.

**Features:**

- Integrated SANY parser with syntax highlighting
- TLC model checker integration with visualization
- PlusCal translator support
- LaTeX/PDF generation
- Model configuration via `.cfg` files

**Installation:**

- **macOS (Homebrew):**

  ```bash
  brew tap homebrew/cask-versions
  brew install tla-plus-toolbox-nightly
  ```

- **Debian/Ubuntu:**

  ```bash
  echo "deb https://nightly.tlapl.us/toolboxUpdate/ ./" | sudo tee /etc/apt/sources.list.d/tlaplus.list
  curl -fsSL https://tla.msr-inria.inria.fr/jenkins.pub | sudo apt-key add -
  sudo apt update && sudo apt install tla-toolbox
  ```

- **Releases:** [github.com/tlaplus/tlaplus/releases](https://github.com/tlaplus/tlaplus/releases)
- **Nightly builds:** [nightly.tlapl.us/products/](https://nightly.tlapl.us/products/)

### VS Code Extension (vscode-tlaplus)

The **TLA+ for Visual Studio Code** extension (ID: `alygin.vscode-tlaplus`) provides modern IDE support.

**Features:**

- TLA+ and PlusCal syntax highlighting
- Code snippets and completion
- PlusCal-to-TLA+ translation
- TLC model checker execution with result visualization
- Constant expression evaluation
- LaTeX/PDF generation
- On-type code formatting

**Installation:**

```bash
code --install-extension alygin.vscode-tlaplus
```

**Requirements:** Java 11+ must be installed and available on PATH.

**Documentation:** [docs.tlapl.us/using:vscode:start](https://docs.tlapl.us/using:vscode:start)

### IDE Comparison

| Feature | TLA+ Toolbox | VS Code Extension |
|---------|--------------|-------------------|
| Syntax highlighting | ✅ | ✅ |
| TLC integration | ✅ (native) | ✅ |
| PlusCal translation | ✅ | ✅ |
| Proof support (TLAPS) | ✅ | Limited |
| Model visualization | ✅ (advanced) | ✅ (basic) |
| Cross-platform | ✅ | ✅ |
| Lightweight | ❌ | ✅ |

---

## 3. CI/CD Integration Patterns

### GitHub Actions Workflow Example

Based on the [tlaplus/Examples](https://github.com/tlaplus/Examples) repository's CI configuration:

```yaml
name: TLA+ Spec Validation

on:
  push:
    branches: [master, main]
  pull_request:
    branches: [master, main]

concurrency:
  group: ${{ github.workflow }}-${{ github.event.pull_request.number || github.ref }}
  cancel-in-progress: true

jobs:
  validate:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
      fail-fast: false

    steps:
      - name: Checkout repository
        uses: actions/checkout@v4

      - name: Install Java
        uses: actions/setup-java@v4
        with:
          distribution: adopt
          java-version: '17'

      - name: Download TLA+ tools
        run: |
          mkdir -p deps/tools
          curl -L -o deps/tools/tla2tools.jar \
            https://github.com/tlaplus/tlaplus/releases/download/v1.7.4/tla2tools.jar

      - name: Parse TLA+ modules (SANY)
        run: |
          java -cp deps/tools/tla2tools.jar tla2sany.SANY specs/*.tla

      - name: Run TLC model checker
        run: |
          java -cp deps/tools/tla2tools.jar tlc2.TLC \
            -config specs/MySpec.cfg \
            -workers auto \
            specs/MySpec.tla
```

### CI Validation Script Patterns

The TLA+ Examples repository uses Python scripts for comprehensive CI validation:

| Script | Purpose |
|--------|---------|
| `parse_modules.py` | Runs SANY parser on all `.tla` files |
| `translate_pluscal.py` | Validates PlusCal syntax via translation |
| `check_small_models.py` | Runs TLC on models expected to complete quickly |
| `smoke_test_large_models.py` | Runs TLC briefly on large models to detect crashes |
| `check_proofs.py` | Validates TLAPS proofs |
| `unicode_conversion.py` | Tests Unicode ↔ ASCII equivalence |

### CI Best Practices

1. **Categorize models by size:**
   - Small models (~30s): Run to completion in CI
   - Large models: Smoke test only (5-10s timeout)

2. **Use manifest files** (`manifest.json`) to track:
   - Module paths and dependencies
   - Expected model results (success/failure type)
   - State space size (for regression detection)

3. **Cross-platform testing:** Run on Linux, macOS, and Windows

4. **Parallel execution:** Use matrix strategies for OS and configuration variants

---

## 4. Command-Line Usage for Automation

### Basic TLC Usage

```bash
# Run model checker with default config
java -jar tla2tools.jar MySpec.tla

# Specify configuration file
java -cp tla2tools.jar tlc2.TLC -config MySpec.cfg MySpec.tla

# Use multiple workers (parallel)
java -cp tla2tools.jar tlc2.TLC -workers auto MySpec.tla

# Set deadlock checking behavior
java -cp tla2tools.jar tlc2.TLC -deadlock MySpec.tla

# Simulation mode (random exploration)
java -cp tla2tools.jar tlc2.TLC -simulate MySpec.tla

# Depth-first search with limit
java -cp tla2tools.jar tlc2.TLC -dfid 10 MySpec.tla
```

### Common TLC Options

| Option | Description |
|--------|-------------|
| `-config <file>` | Specify model configuration file |
| `-workers <n\|auto>` | Number of worker threads |
| `-deadlock` | Check for deadlocks |
| `-simulate` | Random simulation mode |
| `-depth <n>` | Maximum depth for simulation |
| `-coverage <n>` | Print coverage info every n minutes |
| `-checkpoint <n>` | Checkpoint interval in minutes |
| `-recover <path>` | Recover from checkpoint |
| `-difftrace` | Show differences between states |
| `-dump <format> <file>` | Dump state graph (dot, json) |

### PlusCal Translation

```bash
# Translate PlusCal to TLA+
java -cp tla2tools.jar pcal.trans MyAlgorithm.tla

# Translate with specific options
java -cp tla2tools.jar pcal.trans -nocfg MyAlgorithm.tla
```

### SANY Parsing

```bash
# Parse and check syntax/semantics
java -cp tla2tools.jar tla2sany.SANY MySpec.tla

# Parse with library path
java -cp tla2tools.jar tla2sany.SANY -I path/to/libs MySpec.tla
```

### TLAPS Proof Checking

```bash
# Check proofs with TLAPS
tlapm --stretch 2 MyProof.tla

# Check specific theorem
tlapm --method zenon MyProof.tla
```

### Exit Codes for Automation

TLC returns meaningful exit codes for CI integration:

| Exit Code | Meaning |
|-----------|---------|
| 0 | Success (no errors found) |
| 10 | Assumption failure |
| 11 | Deadlock detected |
| 12 | Safety violation |
| 13 | Liveness violation |

---

## 5. Development Workflow Integration Tips

### Project Structure Recommendations

```
project/
├── specs/
│   ├── MyProtocol.tla          # Main TLA+ specification
│   ├── MyProtocol.cfg          # TLC model configuration
│   └── MCMyProtocol.tla        # Model-checking constants/overrides
├── proofs/
│   └── MyProtocolProof.tla     # TLAPS proofs
├── pluscal/
│   └── MyAlgorithm.tla         # PlusCal algorithms
├── deps/
│   └── tla2tools.jar           # TLA+ tools
├── manifest.json               # Spec metadata (optional)
└── .github/
    └── workflows/
        └── tla-check.yml       # CI workflow
```

### Configuration File (.cfg) Best Practices

```tla
\* MySpec.cfg

\* Constants
CONSTANTS
    N = 3
    Nodes = {n1, n2, n3}

\* Initial state and next-state relation
INIT Init
NEXT Next

\* Safety properties (invariants)
INVARIANT TypeOK
INVARIANT SafetyProperty

\* Liveness properties
PROPERTY Termination
PROPERTY EventualConsistency

\* State space reduction
SYMMETRY Symmetry
VIEW StateView

\* Constraint to limit state space
CONSTRAINT StateConstraint
```

### Integration with Development Cycle

1. **During Design:**
   - Write high-level TLA+ spec capturing system behavior
   - Use TLC in simulation mode for rapid iteration
   - Validate basic safety properties

2. **Before Implementation:**
   - Add comprehensive invariants
   - Model-check exhaustively (or bounded with Apalache)
   - Document assumptions in spec comments

3. **During Code Review:**
   - Run TLA+ CI checks on spec changes
   - Verify model configuration hasn't regressed

4. **For Critical Systems:**
   - Write TLAPS proofs for key properties
   - Use proofs to verify unbounded correctness

### Environment Setup Script

```bash
#!/bin/bash
# setup-tlaplus.sh

set -euo pipefail

DEPS_DIR="${1:-deps}"
TLA_VERSION="${2:-v1.7.4}"

mkdir -p "$DEPS_DIR/tools"

# Download tla2tools.jar
curl -L -o "$DEPS_DIR/tools/tla2tools.jar" \
  "https://github.com/tlaplus/tlaplus/releases/download/$TLA_VERSION/tla2tools.jar"

# Download community modules (optional)
curl -L -o "$DEPS_DIR/community-modules.jar" \
  "https://github.com/tlaplus/CommunityModules/releases/latest/download/CommunityModules-deps.jar"

echo "TLA+ tools installed to $DEPS_DIR"

# Verify installation
java -cp "$DEPS_DIR/tools/tla2tools.jar" tlc2.TLC -h | head -5
```

### Makefile Integration

```makefile
TLA_JAR := deps/tools/tla2tools.jar
SPECS := $(wildcard specs/*.tla)

.PHONY: parse check simulate clean

parse: $(TLA_JAR)
    java -cp $(TLA_JAR) tla2sany.SANY $(SPECS)

check: $(TLA_JAR)
    java -cp $(TLA_JAR) tlc2.TLC -config specs/MySpec.cfg specs/MySpec.tla

simulate: $(TLA_JAR)
    java -cp $(TLA_JAR) tlc2.TLC -simulate -depth 100 specs/MySpec.tla

$(TLA_JAR):
    ./setup-tlaplus.sh deps

clean:
    rm -rf deps states
```

---

## Additional Resources

### Learning Resources

- **[TLA+ Home Page](http://lamport.azurewebsites.net/tla/tla.html)** - Leslie Lamport's official site
- **[Learn TLA+](https://learntla.com/)** - Hillel Wayne's practical introduction
- **[TLA+ in Practice and Theory](https://pron.github.io/posts/tlaplus_part1)** - Ron Pressler's series
- **[Specifying Systems](http://lamport.azurewebsites.net/tla/book.html)** - The definitive TLA+ book
- **[TLA+ Examples Repository](https://github.com/tlaplus/Examples)** - Comprehensive example collection

### Community

- **[TLA+ Google Group](https://groups.google.com/g/tlaplus)** - Mailing list
- **[TLA+ Foundation](https://foundation.tlapl.us/)** - Organization managing TLA+ development
- **[Official Documentation](https://docs.tlapl.us/)** - Consolidated TLA+ docs

### Tool Downloads

| Resource | URL |
|----------|-----|
| Stable Releases | [github.com/tlaplus/tlaplus/releases](https://github.com/tlaplus/tlaplus/releases) |
| Nightly Tools | [nightly.tlapl.us/dist/](https://nightly.tlapl.us/dist/) |
| Nightly Toolbox | [nightly.tlapl.us/products/](https://nightly.tlapl.us/products/) |
| Maven Artifacts | [oss.sonatype.org/.../tla2tools](https://oss.sonatype.org/content/repositories/snapshots/org/lamport/tla2tools/) |

---

*Last updated: December 2024*
