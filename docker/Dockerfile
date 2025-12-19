# Dockerfile for Fortress Rollback network testing
#
# This image builds the network_test_peer binary and includes
# tools for network condition simulation (tc/netem).
#
# Build:
#   docker build -t fortress-test-peer -f docker/Dockerfile .
#
# Run manually:
#   docker run --rm --cap-add NET_ADMIN fortress-test-peer \
#     --local-port 9001 --player-index 0 --peer 172.20.0.3:9001 --frames 100
#
# NOTE: The Rust version must be >= 1.86 due to criterion 0.8.1 dependency.
# Update this when dependencies change their MSRV requirements.

FROM rust:1.92-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifests first for better caching
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir -p src tests/bin && \
    echo "fn main() {}" > src/lib.rs && \
    echo "fn main() {}" > tests/bin/network_test_peer.rs

# Build dependencies only (this layer will be cached)
RUN cargo build --release --bin network_test_peer 2>/dev/null || true

# Now copy the real source code (this overwrites the dummy files)
COPY src ./src
COPY tests ./tests
COPY benches ./benches

# Touch source files to ensure they're newer than cached dependencies
# (the COPY above already overwrote the dummy files with real content)
RUN touch src/lib.rs tests/bin/network_test_peer.rs

# Build the actual binary
RUN cargo build --release --bin network_test_peer

# Runtime image
FROM debian:bookworm-slim

# Install networking tools for tc/netem
RUN apt-get update && apt-get install -y \
    iproute2 \
    iputils-ping \
    net-tools \
    && rm -rf /var/lib/apt/lists/*

# Copy the built binary
COPY --from=builder /app/target/release/network_test_peer /usr/local/bin/

# Copy the entrypoint script
COPY docker/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
