#!/bin/bash
# Docker-based network resilience tests for Fortress Rollback
#
# This script runs a suite of network tests using Docker containers
# with Linux tc/netem for realistic network condition simulation.
#
# Usage:
#   ./scripts/docker-network-tests.sh           # Run all tests
#   ./scripts/docker-network-tests.sh basic     # Run only basic test
#   ./scripts/docker-network-tests.sh --build   # Force rebuild before tests
#
# Requirements:
#   - Docker with compose plugin
#   - Linux (for tc/netem support in containers)
#
# Environment:
#   DOCKER_COMPOSE_FILE  - Path to docker-compose.yml (default: docker/docker-compose.yml)
#   SKIP_BUILD           - Set to 1 to skip Docker build
#   VERBOSE              - Set to 1 for verbose output

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
COMPOSE_FILE="${DOCKER_COMPOSE_FILE:-$PROJECT_ROOT/docker/docker-compose.yml}"
RESULTS_DIR="$PROJECT_ROOT/test-results"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test results tracking
TESTS_PASSED=0
TESTS_FAILED=0
FAILED_TESTS=""

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
}

log_error() {
    echo -e "${RED}[FAIL]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."

    if ! command -v docker &> /dev/null; then
        log_error "Docker is not installed"
        exit 1
    fi

    if ! docker compose version &> /dev/null; then
        log_error "Docker Compose plugin is not installed"
        exit 1
    fi

    if [ ! -f "$COMPOSE_FILE" ]; then
        log_error "Docker Compose file not found: $COMPOSE_FILE"
        exit 1
    fi

    log_success "Prerequisites OK"
}

# Build Docker images
build_images() {
    log_info "Building Docker images..."

    cd "$PROJECT_ROOT"
    if docker compose -f "$COMPOSE_FILE" build; then
        log_success "Docker images built successfully"
    else
        log_error "Failed to build Docker images"
        exit 1
    fi
}

# Run a single test scenario
# Arguments:
#   $1 - Test name
#   $2+ - Environment variables (VAR=value format)
run_test() {
    local test_name="$1"
    shift
    local env_vars=("$@")

    log_info "Running test: $test_name"

    # Create results directory
    mkdir -p "$RESULTS_DIR"
    local log_file="$RESULTS_DIR/${test_name}.log"

    # Build environment string
    local env_string=""
    for var in "${env_vars[@]}"; do
        env_string="$env_string $var"
    done

    # Run the test
    cd "$PROJECT_ROOT"
    local start_time=$(date +%s)

    if env $env_string docker compose -f "$COMPOSE_FILE" up \
        --abort-on-container-exit \
        --exit-code-from peer1 \
        --timeout 180 \
        > "$log_file" 2>&1; then

        local end_time=$(date +%s)
        local duration=$((end_time - start_time))

        # Parse results in a single pass using awk (much faster than multiple greps)
        # Uses POSIX-compatible awk (no gawk extensions)
        local result
        result=$(awk '
            /"success": true/ { success_count++ }
            /"checksum":/ { 
                # Extract checksum value using gsub to strip non-digits
                gsub(/.*"checksum": */, "")
                gsub(/[^0-9].*/, "")
                if ($0 != "") checksums[++checksum_count] = $0
            }
            END {
                printf "%d %s %s", success_count+0, checksums[1], checksums[2]
            }
        ' "$log_file")

        local success_count checksum1 checksum2
        read -r success_count checksum1 checksum2 <<< "$result"

        # Both peers must report success
        if [ "$success_count" -ge 2 ]; then
            log_success "$test_name passed (${duration}s)"

            if [ -n "$checksum1" ] && [ -n "$checksum2" ]; then
                if [ "$checksum1" = "$checksum2" ]; then
                    log_success "  Checksums match: $checksum1"
                else
                    log_warning "  Checksums differ: $checksum1 vs $checksum2 (possible desync)"
                fi
            fi

            TESTS_PASSED=$((TESTS_PASSED + 1))
        else
            log_error "$test_name failed (only $success_count peers succeeded) - check $log_file"
            TESTS_FAILED=$((TESTS_FAILED + 1))
            FAILED_TESTS="$FAILED_TESTS $test_name"
        fi
    else
        log_error "$test_name failed (exit code: $?) - check $log_file"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        FAILED_TESTS="$FAILED_TESTS $test_name"
    fi

    # Clean up containers
    docker compose -f "$COMPOSE_FILE" down --timeout 5 > /dev/null 2>&1 || true
}

# Test scenarios
# Note: Timeouts are set generously to accommodate slow CI VMs
test_basic() {
    run_test "basic_connectivity" \
        "TEST_FRAMES=100" \
        "TEST_TIMEOUT=90"
}

test_extended() {
    run_test "extended_session" \
        "TEST_FRAMES=500" \
        "TEST_TIMEOUT=180"
}

test_latency_50ms() {
    run_test "latency_50ms" \
        "TEST_FRAMES=100" \
        "TEST_TIMEOUT=120" \
        "NETEM_DELAY=50ms"
}

test_latency_100ms() {
    run_test "latency_100ms" \
        "TEST_FRAMES=100" \
        "TEST_TIMEOUT=90" \
        "NETEM_DELAY=100ms"
}

test_latency_with_jitter() {
    run_test "latency_with_jitter" \
        "TEST_FRAMES=100" \
        "TEST_TIMEOUT=90" \
        "NETEM_DELAY=50ms" \
        "NETEM_JITTER=25ms"
}

test_packet_loss_5() {
    run_test "packet_loss_5_percent" \
        "TEST_FRAMES=100" \
        "TEST_TIMEOUT=120" \
        "NETEM_LOSS=5%"
}

test_packet_loss_10() {
    run_test "packet_loss_10_percent" \
        "TEST_FRAMES=100" \
        "TEST_TIMEOUT=90" \
        "NETEM_LOSS=10%"
}

test_packet_loss_20() {
    run_test "packet_loss_20_percent" \
        "TEST_FRAMES=100" \
        "TEST_TIMEOUT=120" \
        "NETEM_LOSS=20%"
}

test_poor_network() {
    run_test "poor_network" \
        "TEST_FRAMES=100" \
        "TEST_TIMEOUT=180" \
        "NETEM_DELAY=80ms" \
        "NETEM_JITTER=30ms" \
        "NETEM_LOSS=5%"
}

test_terrible_network() {
    run_test "terrible_network" \
        "TEST_FRAMES=100" \
        "TEST_TIMEOUT=180" \
        "NETEM_DELAY=150ms" \
        "NETEM_JITTER=50ms" \
        "NETEM_LOSS=15%"
}

test_high_latency() {
    run_test "high_latency_200ms" \
        "TEST_FRAMES=100" \
        "TEST_TIMEOUT=120" \
        "NETEM_DELAY=200ms"
}

test_extreme_latency() {
    run_test "extreme_latency_500ms" \
        "TEST_FRAMES=50" \
        "TEST_TIMEOUT=180" \
        "NETEM_DELAY=500ms"
}

test_packet_duplication() {
    run_test "packet_duplication" \
        "TEST_FRAMES=100" \
        "TEST_TIMEOUT=120" \
        "NETEM_DUPLICATE=10%"
}

test_packet_reorder() {
    run_test "packet_reorder" \
        "TEST_FRAMES=100" \
        "TEST_TIMEOUT=90" \
        "NETEM_DELAY=20ms" \
        "NETEM_REORDER=25%"
}

test_stress_long_session() {
    run_test "stress_long_session" \
        "TEST_FRAMES=1000" \
        "TEST_TIMEOUT=300" \
        "NETEM_DELAY=30ms" \
        "NETEM_LOSS=3%"
}

# Run all tests
run_all_tests() {
    log_info "Running all Docker network tests..."
    echo ""

    # Basic tests
    test_basic
    test_extended

    # Latency tests
    test_latency_50ms
    test_latency_100ms
    test_latency_with_jitter
    test_high_latency

    # Packet loss tests
    test_packet_loss_5
    test_packet_loss_10
    test_packet_loss_20

    # Combined conditions
    test_poor_network
    test_terrible_network

    # Other network issues
    test_packet_duplication
    test_packet_reorder
}

# Run quick tests (subset for CI)
run_quick_tests() {
    log_info "Running quick Docker network tests (CI mode)..."
    echo ""

    test_basic
    test_latency_50ms
    test_packet_loss_5
    test_poor_network
}

# Print summary
print_summary() {
    echo ""
    echo "=============================================="
    echo "           TEST SUMMARY"
    echo "=============================================="
    echo -e "Passed: ${GREEN}$TESTS_PASSED${NC}"
    echo -e "Failed: ${RED}$TESTS_FAILED${NC}"

    if [ $TESTS_FAILED -gt 0 ]; then
        echo ""
        echo -e "Failed tests:${RED}$FAILED_TESTS${NC}"
        echo ""
        echo "Check logs in: $RESULTS_DIR"
    fi

    echo "=============================================="

    if [ $TESTS_FAILED -gt 0 ]; then
        return 1
    fi
    return 0
}

# Main entry point
main() {
    local do_build=0
    local test_to_run=""

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --build)
                do_build=1
                shift
                ;;
            --quick|quick)
                test_to_run="quick"
                shift
                ;;
            --all|all)
                test_to_run="all"
                shift
                ;;
            --stress|stress)
                test_to_run="stress"
                shift
                ;;
            basic|extended|latency_*|packet_*|poor_network|terrible_network|high_latency|extreme_latency)
                test_to_run="$1"
                shift
                ;;
            --help|-h)
                echo "Usage: $0 [OPTIONS] [TEST_NAME]"
                echo ""
                echo "Options:"
                echo "  --build     Force rebuild Docker images"
                echo "  --quick     Run quick test subset (for CI)"
                echo "  --all       Run all tests"
                echo "  --stress    Run stress test (long session)"
                echo "  --help      Show this help"
                echo ""
                echo "Test names:"
                echo "  basic, extended, latency_50ms, latency_100ms,"
                echo "  latency_with_jitter, high_latency, extreme_latency,"
                echo "  packet_loss_5, packet_loss_10, packet_loss_20,"
                echo "  poor_network, terrible_network,"
                echo "  packet_duplication, packet_reorder"
                exit 0
                ;;
            *)
                log_error "Unknown argument: $1"
                exit 1
                ;;
        esac
    done

    # Default to quick tests
    if [ -z "$test_to_run" ]; then
        test_to_run="quick"
    fi

    echo "=============================================="
    echo "  Fortress Rollback Docker Network Tests"
    echo "=============================================="
    echo ""

    check_prerequisites

    # Build if requested or if SKIP_BUILD is not set
    if [ $do_build -eq 1 ] || [ "${SKIP_BUILD:-0}" != "1" ]; then
        build_images
    fi

    echo ""

    # Run requested tests
    case $test_to_run in
        quick)
            run_quick_tests
            ;;
        all)
            run_all_tests
            ;;
        stress)
            test_stress_long_session
            ;;
        basic)
            test_basic
            ;;
        extended)
            test_extended
            ;;
        latency_50ms)
            test_latency_50ms
            ;;
        latency_100ms)
            test_latency_100ms
            ;;
        latency_with_jitter)
            test_latency_with_jitter
            ;;
        high_latency)
            test_high_latency
            ;;
        extreme_latency)
            test_extreme_latency
            ;;
        packet_loss_5)
            test_packet_loss_5
            ;;
        packet_loss_10)
            test_packet_loss_10
            ;;
        packet_loss_20)
            test_packet_loss_20
            ;;
        poor_network)
            test_poor_network
            ;;
        terrible_network)
            test_terrible_network
            ;;
        packet_duplication)
            test_packet_duplication
            ;;
        packet_reorder)
            test_packet_reorder
            ;;
        *)
            log_error "Unknown test: $test_to_run"
            exit 1
            ;;
    esac

    print_summary
}

main "$@"
