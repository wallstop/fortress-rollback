# Fortress Rollback Development Container
# 
# This container includes all verification tools:
# - TLA+ model checker
# - Kani bounded model checker
# - Miri (undefined behavior detection)
# - Z3 theorem prover
# - Loom (concurrency testing via Cargo)
# - Various cargo tools for testing, coverage, and profiling

FROM mcr.microsoft.com/devcontainers/rust:2-1-trixie

# Avoid prompts from apt
ENV DEBIAN_FRONTEND=noninteractive

# Install system dependencies
RUN apt-get update && apt-get install -y \
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
    # See: https://github.com/not-fl3/macroquad and examples/README.md
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
    # High-performance CLI tools available in apt
    ripgrep \
    fd-find \
    fzf \
    bat \
    htop \
    ncdu \
    # Note: eza, git-delta, dust, duf, procs, sd, etc. are installed via cargo below
    && rm -rf /var/lib/apt/lists/*

# Install linux-perf if available (architecture dependent)
RUN apt-get update && apt-get install -y linux-perf 2>/dev/null || true && rm -rf /var/lib/apt/lists/*

# Set up TLA+ tools
RUN mkdir -p /opt/tla && \
    curl -L https://github.com/tlaplus/tlaplus/releases/download/v1.8.0/tla2tools.jar -o /opt/tla/tla2tools.jar

ENV TLA_TOOLS_JAR=/opt/tla/tla2tools.jar

# Install Z3 via pip
RUN pip3 install --break-system-packages z3-solver

# Switch to vscode user for remaining installations
USER vscode

# Install Rust nightly toolchain (required for Miri and some Kani features)
RUN rustup install nightly && \
    rustup component add rust-src --toolchain nightly && \
    rustup component add llvm-tools-preview --toolchain nightly

# Install Miri (undefined behavior detector)
RUN rustup +nightly component add miri

# Install Kani verifier
# Note: Kani requires its own setup step which downloads CBMC
RUN cargo install --locked kani-verifier && \
    cargo kani setup || echo "Kani setup may need to complete on first use"

# Install cargo tools for testing and verification
RUN cargo install cargo-audit && \
    cargo install cargo-deny && \
    cargo install cargo-tarpaulin && \
    cargo install cargo-llvm-cov && \
    cargo install cargo-mutants && \
    cargo install cargo-fuzz || echo "cargo-fuzz requires nightly" && \
    cargo install flamegraph && \
    cargo install cargo-nextest && \
    cargo install cargo-watch && \
    cargo install sccache && \
    # High-performance CLI tools (Rust-based, not available in Debian apt)
    cargo install eza && \
    cargo install git-delta && \
    cargo install du-dust && \
    cargo install duf && \
    cargo install procs && \
    cargo install sd && \
    cargo install hyperfine && \
    cargo install tokei && \
    cargo install zoxide && \
    cargo install bottom || echo "bottom (btm) optional"

# Install Node.js and markdown tools (for pre-commit hooks)
USER root
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - && \
    apt-get install -y nodejs && \
    npm install -g markdownlint-cli markdown-link-check && \
    pip3 install --break-system-packages pre-commit && \
    rm -rf /var/lib/apt/lists/*
USER vscode

# Set environment variables
ENV RUST_BACKTRACE=1
ENV CARGO_INCREMENTAL=0

# Reset to root for any final setup
USER root

# Create helper script for running TLA+
RUN echo '#!/bin/bash\njava -XX:+UseParallelGC -Xmx4g -jar /opt/tla/tla2tools.jar "$@"' > /usr/local/bin/tlc && \
    chmod +x /usr/local/bin/tlc

# Create helper script for SANY (TLA+ parser)
RUN echo '#!/bin/bash\njava -cp /opt/tla/tla2tools.jar tla2sany.SANY "$@"' > /usr/local/bin/sany && \
    chmod +x /usr/local/bin/sany

# Switch back to vscode user
USER vscode

WORKDIR /workspaces/ggrs

# Set up aliases for high-performance tools
# Note: fd-find installs as 'fdfind' on Debian, bat as 'batcat'
# eza, delta, dust, duf, procs are installed via cargo and have standard names
RUN echo '\n# High-performance tool aliases\nalias fd="fdfind"\nalias bat="batcat"\nalias ls="eza"\nalias ll="eza -la"\nalias la="eza -la"\nalias tree="eza --tree"\nalias cat="batcat --paging=never"\nalias diff="delta"\nalias du="dust"\nalias df="duf"\nalias ps="procs"\nalias top="htop"\nalias sed="sd"\n\n# Initialize zoxide (smart cd)\neval "$(zoxide init bash)"\n\n# Conditionally enable sccache if available\nif command -v sccache &> /dev/null; then\n  export RUSTC_WRAPPER=sccache\nfi\n' >> ~/.bashrc

# Print tool versions on first login (via .bashrc)
RUN echo '\n# Fortress Rollback Development Environment\nif [ -f /workspaces/ggrs/.devcontainer/welcome.sh ]; then\n  source /workspaces/ggrs/.devcontainer/welcome.sh\nfi' >> ~/.bashrc
