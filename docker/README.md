<p align="center">
  <img src="../assets/logo-small.svg" alt="Fortress Rollback" width="64">
</p>

# Docker Network Testing

This directory contains Docker infrastructure for realistic network testing of Fortress Rollback using Linux `tc/netem` for kernel-level network simulation.

## Quick Start

```bash
# Run quick tests (basic + latency + loss + combined)
./scripts/docker-network-tests.sh --quick

# Run all tests
./scripts/docker-network-tests.sh --all

# Run a specific test
./scripts/docker-network-tests.sh latency_50ms
```

## Prerequisites

- Docker with Compose plugin
- Linux (required for tc/netem in containers)

On macOS/Windows, the Docker tests will run but without kernel-level network simulation (falls back to application-level chaos via ChaosSocket).

## Test Scenarios

| Test | Conditions | Description |
|------|------------|-------------|
| `basic` | None | Basic connectivity check |
| `extended` | None | 500-frame stability test |
| `latency_50ms` | 50ms delay | Moderate latency |
| `latency_100ms` | 100ms delay | High latency |
| `latency_with_jitter` | 50ms ± 25ms | Variable latency |
| `high_latency` | 200ms delay | Very high latency |
| `extreme_latency` | 500ms delay | Extreme latency stress |
| `packet_loss_5` | 5% loss | Light packet loss |
| `packet_loss_10` | 10% loss | Moderate packet loss |
| `packet_loss_20` | 20% loss | Heavy packet loss |
| `poor_network` | 80ms + 30ms jitter + 5% loss | Typical poor connection |
| `terrible_network` | 150ms + 50ms jitter + 15% loss | Very bad connection |
| `packet_duplication` | 10% duplicate | Duplicate packet handling |
| `packet_reorder` | 25% reorder | Out-of-order delivery |
| `stress_long_session` | 30ms + 3% loss, 1000 frames | Long-running stress test |

## Manual Usage

### Run containers manually

```bash
# Build the image
docker compose -f docker/docker-compose.yml build

# Run with no network chaos
docker compose -f docker/docker-compose.yml up

# Run with specific conditions
NETEM_DELAY=100ms NETEM_LOSS=10% docker compose -f docker/docker-compose.yml up

# Run with asymmetric conditions
PEER1_NETEM_LOSS=20% PEER2_NETEM_LOSS=5% docker compose -f docker/docker-compose.yml up
```

### Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `NETEM_DELAY` | Base latency | `50ms` |
| `NETEM_JITTER` | Latency variation | `20ms` |
| `NETEM_LOSS` | Packet loss rate | `10%` |
| `NETEM_DUPLICATE` | Packet duplication rate | `5%` |
| `NETEM_REORDER` | Packet reorder rate | `25%` |
| `NETEM_CORRUPT` | Packet corruption rate | `0.1%` |
| `TEST_FRAMES` | Number of frames to run | `100` |
| `TEST_TIMEOUT` | Timeout in seconds | `60` |
| `DEBUG` | Enable debug output | `1` |

## How It Works

1. **Docker Compose** creates a custom bridge network (`172.28.0.0/16`)
2. Two peer containers are started with fixed IPs
3. Each container runs the **entrypoint script** which:
   - Configures `tc qdisc` with `netem` for network conditions
   - Starts the `network_test_peer` binary
4. Peers connect over the Docker network and run the test
5. Results are output as JSON and validated by the test script

## tc/netem Reference

The entrypoint script uses Linux Traffic Control (`tc`) with the Network Emulator (`netem`) qdisc:

```bash
# Add 50ms latency with 20ms jitter
tc qdisc add dev eth0 root netem delay 50ms 20ms

# Add 10% packet loss
tc qdisc add dev eth0 root netem loss 10%

# Combined conditions
tc qdisc add dev eth0 root netem delay 100ms 30ms loss 5% duplicate 1%
```

## CI/CD Integration

The Docker tests run automatically in GitHub Actions:

- **Quick tests** run on every push/PR (`--quick` mode)
- Test results are uploaded as artifacts
- Full test suite can be triggered manually

## Troubleshooting

### "tc: command not found"

The container needs `iproute2` installed (included in the Docker image).

### "RTNETLINK answers: Operation not permitted"

The container needs `NET_ADMIN` capability:
```bash
docker run --cap-add NET_ADMIN ...
```

### Tests timing out

Increase the timeout:
```bash
TEST_TIMEOUT=120 docker compose -f docker/docker-compose.yml up
```

### Viewing container logs

```bash
docker logs fortress-peer1
docker logs fortress-peer2
```
