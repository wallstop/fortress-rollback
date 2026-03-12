# Fortress Rollback Development Container
#
# Optimized for fast builds using cargo-binstall (pre-built binaries).
# Full build: ~3-5 min (vs ~25 min compiling from source).
# With pre-built image from GHCR: ~30 seconds (pull only).
#
# Tools included:
# - TLA+ model checker
# - Kani bounded model checker
# - Miri (undefined behavior detection)
# - Z3 theorem prover
# - Various cargo tools for testing, coverage, and profiling

FROM mcr.microsoft.com/devcontainers/rust:2-1-trixie

# Use bash for all RUN commands to ensure consistent shell behavior
SHELL ["/bin/bash", "-c"]

# Avoid prompts from apt
ENV DEBIAN_FRONTEND=noninteractive

# ============================================================================
# Layer 1: System dependencies (apt)
# ============================================================================
RUN apt-get update && apt-get install -y --no-install-recommends \
    # Build essentials
    build-essential \
    cmake \
    pkg-config \
    libssl-dev \
    # LLVM/Clang (for Kani, Z3 Rust bindings via bindgen, and other tools)
    clang \
    llvm \
    lld \
    libclang-dev \
    # Java (for TLA+)
    default-jdk \
    # macroquad example dependencies (graphics, audio, input)
    libasound2-dev \
    libx11-dev \
    libxi-dev \
    libgl1-mesa-dev \
    # Visualization
    graphviz \
    # Networking tools (for network testing)
    iproute2 \
    iputils-ping \
    netcat-openbsd \
    tcpdump \
    # Debugging and profiling
    valgrind \
    gdb \
    strace \
    ltrace \
    # Python (for Z3 and scripting)
    python3 \
    python3-pip \
    python3-venv \
    # Additional utilities
    curl \
    wget \
    git \
    jq \
    yq \
    # High-performance CLI tools available in apt
    ripgrep \
    fd-find \
    fzf \
    bat \
    htop \
    ncdu \
    silversearcher-ag \
    && rm -rf /var/lib/apt/lists/*

# Install linux-perf if available (architecture dependent)
RUN apt-get update && \
    if apt-get install -y --no-install-recommends linux-perf; then \
        echo "linux-perf: installed"; \
    else \
        echo "linux-perf: skipped (not available for this architecture)"; \
    fi && \
    rm -rf /var/lib/apt/lists/*

# ============================================================================
# Layer 2: Direct downloads (TLA+, actionlint, Vale)
# ============================================================================
RUN mkdir -p /opt/tla && \
    curl --proto '=https' --tlsv1.2 -fsSL --retry 5 --retry-delay 2 --retry-all-errors --max-time 120 https://github.com/tlaplus/tlaplus/releases/download/v1.8.0/tla2tools.jar \
      -o /opt/tla/tla2tools.jar

# Note: devcontainer.json containerEnv overrides this to the workspace copy
ENV TLA_TOOLS_JAR=/opt/tla/tla2tools.jar

# Install Z3 and yamllint via pip
RUN pip3 install --no-cache-dir --break-system-packages z3-solver yamllint

# Install actionlint (GitHub Actions linter)
RUN curl --proto '=https' --tlsv1.2 -fsSL --retry 5 --retry-delay 2 --retry-all-errors --max-time 120 https://raw.githubusercontent.com/rhysd/actionlint/main/scripts/download-actionlint.bash \
      -o /tmp/download-actionlint.bash && \
    bash /tmp/download-actionlint.bash latest /usr/local/bin && \
    rm /tmp/download-actionlint.bash && \
    chmod +x /usr/local/bin/actionlint

# Install Vale (prose linter) - architecture aware
RUN VALE_VERSION="3.9.3" && \
    ARCH="$(uname -m)" && \
    if [ "$ARCH" = "aarch64" ] || [ "$ARCH" = "arm64" ]; then VALE_ARCH="arm64"; else VALE_ARCH="64-bit"; fi && \
        curl --proto '=https' --tlsv1.2 -fsSL --retry 5 --retry-delay 2 --retry-all-errors --max-time 120 "https://github.com/errata-ai/vale/releases/download/v${VALE_VERSION}/vale_${VALE_VERSION}_Linux_${VALE_ARCH}.tar.gz" \
      -o /tmp/vale.tar.gz && \
    tar -xzf /tmp/vale.tar.gz -C /usr/local/bin vale && \
    rm /tmp/vale.tar.gz && \
    chmod +x /usr/local/bin/vale

# ============================================================================
# Layer 3: Node.js and linting tools
# ============================================================================
RUN curl --proto '=https' --tlsv1.2 -fsSL --retry 5 --retry-delay 2 --retry-all-errors --max-time 120 https://deb.nodesource.com/setup_22.x -o /tmp/nodesource_setup.sh && \
    bash /tmp/nodesource_setup.sh && \
    rm /tmp/nodesource_setup.sh && \
    apt-get install -y --no-install-recommends nodejs && \
    npm install -g markdownlint-cli markdown-link-check && \
    pip3 install --no-cache-dir --break-system-packages pre-commit && \
    rm -rf /var/lib/apt/lists/*

# ============================================================================
# Layer 4: Rust toolchains and components (as vscode user)
# ============================================================================
USER vscode

# Install Rust nightly toolchain (required for Miri and some Kani features)
RUN rustup install nightly && \
    rustup component add rust-src --toolchain nightly && \
    rustup component add llvm-tools-preview --toolchain nightly

# Install Miri (undefined behavior detector)
RUN rustup +nightly component add miri

# ============================================================================
# Layer 5: cargo-binstall (enables fast pre-built binary downloads)
# ~10 seconds instead of ~18 minutes compiling from source
# ============================================================================
ENV BINSTALL_DISABLE_TELEMETRY=true
RUN set -e && \
    ARCH="$(uname -m)" && \
    CARGO_BIN_DIR="${CARGO_HOME:-$HOME/.cargo}/bin" && \
    if [ "$ARCH" = "aarch64" ] || [ "$ARCH" = "arm64" ]; then \
        BINSTALL_ARCH="aarch64-unknown-linux-musl"; \
    else \
        BINSTALL_ARCH="x86_64-unknown-linux-musl"; \
    fi && \
    mkdir -p "$CARGO_BIN_DIR" && \
    echo "Installing cargo-binstall for ${BINSTALL_ARCH}" && \
    curl --proto '=https' --tlsv1.2 -fsSL --retry 5 --retry-delay 2 --retry-all-errors --max-time 120 "https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-${BINSTALL_ARCH}.tgz" \
      -o /tmp/cargo-binstall.tgz && \
    tar -xzf /tmp/cargo-binstall.tgz -C "$CARGO_BIN_DIR" && \
    rm /tmp/cargo-binstall.tgz && \
        cargo-binstall -V

# ============================================================================
# Layer 6: Verification & testing tools (via binstall - pre-built binaries)
# These are the most critical tools, installed first for cache stability
# ============================================================================
RUN for tool in cargo-nextest cargo-audit cargo-deny cargo-tarpaulin cargo-llvm-cov cargo-mutants cargo-careful flamegraph; do \
        cargo binstall --no-confirm "$tool" \
            || echo "$tool: failed to install"; \
    done

# ============================================================================
# Layer 7: Development workflow tools (via binstall)
# ============================================================================
RUN for tool in cargo-watch sccache cargo-shear; do \
        cargo binstall --no-confirm "$tool" \
            || echo "$tool: failed to install"; \
    done

# ============================================================================
# Layer 8: High-performance CLI tools (via binstall)
# These are optional -- each tool installs independently so one failure
# does not block the others.
# ============================================================================
RUN for tool in eza git-delta du-dust duf procs sd hyperfine tokei zoxide tealdeer bottom; do \
        cargo binstall --no-confirm "$tool" \
            || echo "$tool: skipped (no prebuilt binary)"; \
    done

# ============================================================================
# Layer 9: Kani verifier (compiled from source - no pre-built binary available)
# This is the heaviest layer (~3-4 min). Cached unless kani version changes.
# ============================================================================
RUN cargo install --locked kani-verifier && \
    (cargo kani setup || echo "Kani setup may need to complete on first use")

# ============================================================================
# Layer 10: Remaining source-compiled tools (each isolated for cache stability)
# ============================================================================
RUN cargo install cargo-fuzz || echo "cargo-fuzz requires nightly"

RUN cargo install cargo-geiger || echo "cargo-geiger optional"

RUN cargo install cargo-spellcheck || echo "cargo-spellcheck optional"

# ============================================================================
# Layer 11: Environment and helper scripts
# ============================================================================
ENV RUST_BACKTRACE=1
ENV CARGO_INCREMENTAL=0

USER root

# Create helper script for running TLA+
RUN echo '#!/bin/bash\njava -XX:+UseParallelGC -Xmx4g -jar /opt/tla/tla2tools.jar "$@"' > /usr/local/bin/tlc && \
    chmod +x /usr/local/bin/tlc

# Create helper script for SANY (TLA+ parser)
RUN echo '#!/bin/bash\njava -cp /opt/tla/tla2tools.jar tla2sany.SANY "$@"' > /usr/local/bin/sany && \
    chmod +x /usr/local/bin/sany

# Switch back to vscode user
USER vscode

WORKDIR /workspaces/fortress-rollback

# Set up aliases for high-performance tools
# Note: fd-find installs as 'fdfind' on Debian, bat as 'batcat'
# eza, delta, dust, duf, procs are installed via binstall and have standard names
# tealdeer installs as 'tldr'
RUN echo '\n# High-performance tool aliases\nalias fd="fdfind"\nalias bat="batcat"\nalias ls="eza"\nalias ll="eza -la"\nalias la="eza -la"\nalias tree="eza --tree"\nalias cat="batcat --paging=never"\nalias diff="delta"\nalias du="dust"\nalias df="duf"\nalias ps="procs"\nalias top="htop"\n\n# Initialize zoxide (smart cd)\neval "$(zoxide init bash)"\n\n# Conditionally enable sccache if available\nif command -v sccache >/dev/null 2>&1; then\n  export RUSTC_WRAPPER=sccache\nfi\n' >> ~/.bashrc

# Print tool versions on first login (via .bashrc)
RUN echo '\n# Fortress Rollback Development Environment\nif [ -f /workspaces/fortress-rollback/.devcontainer/welcome.sh ]; then\n  source /workspaces/fortress-rollback/.devcontainer/welcome.sh\nfi' >> ~/.bashrc
