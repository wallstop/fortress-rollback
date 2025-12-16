#!/bin/bash
# Entrypoint script for Fortress Rollback network test peer
#
# This script configures network conditions using tc/netem before
# starting the test peer binary.
#
# Environment variables for network conditions:
#   NETEM_DELAY     - Latency in ms (e.g., "50ms")
#   NETEM_JITTER    - Jitter in ms (e.g., "20ms")
#   NETEM_LOSS      - Packet loss percentage (e.g., "5%")
#   NETEM_DUPLICATE - Packet duplication percentage (e.g., "1%")
#   NETEM_REORDER   - Packet reorder percentage (e.g., "25%")
#   NETEM_CORRUPT   - Packet corruption percentage (e.g., "0.1%")
#   STARTUP_DELAY_MS - Delay in ms before starting the test peer (for peer synchronization)

set -e

# Apply startup delay if configured (helps ensure both peers are ready)
if [ -n "$STARTUP_DELAY_MS" ] && [ "$STARTUP_DELAY_MS" -gt 0 ]; then
    # Convert ms to seconds using awk (available in base image)
    delay_secs=$(awk "BEGIN {printf \"%.3f\", $STARTUP_DELAY_MS / 1000}")
    echo "Waiting ${delay_secs}s for peer synchronization..."
    sleep "$delay_secs"
fi

# Function to configure tc/netem
configure_netem() {
    local interface="${NETEM_INTERFACE:-eth0}"
    local netem_opts=""

    # Build netem options string
    if [ -n "$NETEM_DELAY" ]; then
        netem_opts="$netem_opts delay $NETEM_DELAY"
        if [ -n "$NETEM_JITTER" ]; then
            netem_opts="$netem_opts $NETEM_JITTER"
        fi
    fi

    if [ -n "$NETEM_LOSS" ]; then
        netem_opts="$netem_opts loss $NETEM_LOSS"
    fi

    if [ -n "$NETEM_DUPLICATE" ]; then
        netem_opts="$netem_opts duplicate $NETEM_DUPLICATE"
    fi

    if [ -n "$NETEM_REORDER" ] && [ -n "$NETEM_DELAY" ]; then
        netem_opts="$netem_opts reorder $NETEM_REORDER"
    fi

    if [ -n "$NETEM_CORRUPT" ]; then
        netem_opts="$netem_opts corrupt $NETEM_CORRUPT"
    fi

    # Apply netem if we have any options
    if [ -n "$netem_opts" ]; then
        echo "Configuring network conditions: tc qdisc add dev $interface root netem $netem_opts"
        tc qdisc add dev "$interface" root netem $netem_opts || {
            echo "Warning: Failed to configure tc/netem (may require --cap-add NET_ADMIN)"
        }
    else
        echo "No network conditions configured (passthrough mode)"
    fi
}

# Function to show network config
show_network_info() {
    echo "=== Network Configuration ==="
    echo "Interface: ${NETEM_INTERFACE:-eth0}"
    ip addr show "${NETEM_INTERFACE:-eth0}" 2>/dev/null || true
    echo ""
    echo "=== TC Configuration ==="
    tc qdisc show dev "${NETEM_INTERFACE:-eth0}" 2>/dev/null || echo "No tc config"
    echo "=========================="
}

# Configure network conditions
configure_netem

# Show network info if debug mode
if [ "$DEBUG" = "1" ]; then
    show_network_info
fi

# Run the test peer with all arguments passed to the container
echo "Starting network_test_peer with args: $@"
exec /usr/local/bin/network_test_peer "$@"
