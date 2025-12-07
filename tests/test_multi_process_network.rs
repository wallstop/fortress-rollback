//! Multi-process network testing.
//!
//! These tests spawn separate instances of `network_test_peer` to validate
//! P2P networking over real UDP sockets. This tests actual network I/O
//! between separate processes, providing more realistic validation than
//! in-process tests.
//!
//! # Requirements
//!
//! The `network_test_peer` binary must be built before running these tests:
//! ```bash
//! cargo build --bin network_test_peer
//! ```
//!
//! # Test Categories
//!
//! - **Basic connectivity**: Two peers can connect and advance frames
//! - **Chaos conditions**: Testing with packet loss, latency, jitter
//! - **Stress tests**: High frame counts, aggressive network conditions

use serde::Deserialize;
use serial_test::serial;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

/// Result from a test peer process
#[derive(Debug, Deserialize)]
struct TestResult {
    success: bool,
    final_frame: i32,
    checksum: u64,
    rollbacks: u32,
    error: Option<String>,
}

/// Configuration for a test peer
struct PeerConfig {
    local_port: u16,
    player_index: usize,
    peer_addr: String,
    frames: i32,
    packet_loss: f64,
    latency_ms: u64,
    jitter_ms: u64,
    seed: Option<u64>,
    timeout_secs: u64,
    input_delay: usize,
}

impl Default for PeerConfig {
    fn default() -> Self {
        Self {
            local_port: 0,
            player_index: 0,
            peer_addr: String::new(),
            frames: 100,
            packet_loss: 0.0,
            latency_ms: 0,
            jitter_ms: 0,
            seed: None,
            timeout_secs: 30,
            input_delay: 2,
        }
    }
}

/// Spawns a test peer process
fn spawn_peer(config: &PeerConfig) -> std::io::Result<Child> {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_network_test_peer"));

    cmd.arg("--local-port")
        .arg(config.local_port.to_string())
        .arg("--player-index")
        .arg(config.player_index.to_string())
        .arg("--peer")
        .arg(&config.peer_addr)
        .arg("--frames")
        .arg(config.frames.to_string())
        .arg("--timeout")
        .arg(config.timeout_secs.to_string())
        .arg("--input-delay")
        .arg(config.input_delay.to_string());

    if config.packet_loss > 0.0 {
        cmd.arg("--packet-loss")
            .arg(config.packet_loss.to_string());
    }
    if config.latency_ms > 0 {
        cmd.arg("--latency").arg(config.latency_ms.to_string());
    }
    if config.jitter_ms > 0 {
        cmd.arg("--jitter").arg(config.jitter_ms.to_string());
    }
    if let Some(seed) = config.seed {
        cmd.arg("--seed").arg(seed.to_string());
    }

    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
}

/// Waits for a peer and parses its result
fn wait_for_peer(child: Child, name: &str) -> TestResult {
    let output = child.wait_with_output().expect("Failed to wait for peer");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        eprintln!("{} stderr: {}", name, stderr);
    }

    // Parse the last line as JSON (peers output JSON result on stdout)
    let last_line = stdout.lines().last().unwrap_or("");

    match serde_json::from_str::<TestResult>(last_line) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("{} failed to parse output: {}", name, e);
            eprintln!("{} stdout: {}", name, stdout);
            eprintln!("{} stderr: {}", name, stderr);
            TestResult {
                success: false,
                final_frame: 0,
                checksum: 0,
                rollbacks: 0,
                error: Some(format!("Failed to parse output: {}", e)),
            }
        }
    }
}

/// Runs a two-peer test with the given configurations
fn run_two_peer_test(
    peer1_config: PeerConfig,
    peer2_config: PeerConfig,
) -> (TestResult, TestResult) {
    // Spawn peer 1
    let peer1 = spawn_peer(&peer1_config).expect("Failed to spawn peer 1");

    // Small delay to ensure peer 1 is listening
    thread::sleep(Duration::from_millis(100));

    // Spawn peer 2
    let peer2 = spawn_peer(&peer2_config).expect("Failed to spawn peer 2");

    // Wait for both peers
    let result1 = wait_for_peer(peer1, "Peer 1");
    let result2 = wait_for_peer(peer2, "Peer 2");

    (result1, result2)
}

// =============================================================================
// Basic Connectivity Tests
// =============================================================================

/// Test that two peers can connect and advance 100 frames over real network.
#[test]
#[serial]
fn test_basic_connectivity() {
    let peer1_config = PeerConfig {
        local_port: 10001,
        player_index: 0,
        peer_addr: "127.0.0.1:10002".to_string(),
        frames: 100,
        timeout_secs: 30,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10002,
        player_index: 1,
        peer_addr: "127.0.0.1:10001".to_string(),
        frames: 100,
        timeout_secs: 30,
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    assert!(
        result1.success,
        "Peer 1 failed: {:?}",
        result1.error
    );
    assert!(
        result2.success,
        "Peer 2 failed: {:?}",
        result2.error
    );

    // Both should reach target frames
    assert_eq!(result1.final_frame, 100);
    assert_eq!(result2.final_frame, 100);

    // Checksums should match (deterministic simulation)
    assert_eq!(
        result1.checksum, result2.checksum,
        "Checksums don't match - possible desync!"
    );
}

/// Test longer session (500 frames) to verify stability.
#[test]
#[serial]
fn test_extended_session() {
    let peer1_config = PeerConfig {
        local_port: 10003,
        player_index: 0,
        peer_addr: "127.0.0.1:10004".to_string(),
        frames: 500,
        timeout_secs: 60,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10004,
        player_index: 1,
        peer_addr: "127.0.0.1:10003".to_string(),
        frames: 500,
        timeout_secs: 60,
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    assert!(result1.success, "Peer 1 failed: {:?}", result1.error);
    assert!(result2.success, "Peer 2 failed: {:?}", result2.error);
    assert_eq!(result1.final_frame, 500);
    assert_eq!(result2.final_frame, 500);
    assert_eq!(result1.checksum, result2.checksum);
}

// =============================================================================
// Packet Loss Tests
// =============================================================================

/// Test with 5% packet loss - should still complete successfully.
#[test]
#[serial]
fn test_packet_loss_5_percent() {
    let peer1_config = PeerConfig {
        local_port: 10005,
        player_index: 0,
        peer_addr: "127.0.0.1:10006".to_string(),
        frames: 100,
        packet_loss: 0.05,
        seed: Some(42),
        timeout_secs: 45,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10006,
        player_index: 1,
        peer_addr: "127.0.0.1:10005".to_string(),
        frames: 100,
        packet_loss: 0.05,
        seed: Some(43),
        timeout_secs: 45,
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    assert!(
        result1.success,
        "Peer 1 failed with 5% loss: {:?}",
        result1.error
    );
    assert!(
        result2.success,
        "Peer 2 failed with 5% loss: {:?}",
        result2.error
    );
    assert_eq!(result1.checksum, result2.checksum);
}

/// Test with 15% packet loss - more challenging but should work.
#[test]
#[serial]
fn test_packet_loss_15_percent() {
    let peer1_config = PeerConfig {
        local_port: 10007,
        player_index: 0,
        peer_addr: "127.0.0.1:10008".to_string(),
        frames: 100,
        packet_loss: 0.15,
        seed: Some(42),
        timeout_secs: 60,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10008,
        player_index: 1,
        peer_addr: "127.0.0.1:10007".to_string(),
        frames: 100,
        packet_loss: 0.15,
        seed: Some(43),
        timeout_secs: 60,
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    assert!(
        result1.success,
        "Peer 1 failed with 15% loss: {:?}",
        result1.error
    );
    assert!(
        result2.success,
        "Peer 2 failed with 15% loss: {:?}",
        result2.error
    );
    assert_eq!(result1.checksum, result2.checksum);
}

// =============================================================================
// Latency Tests
// =============================================================================

/// Test with 30ms simulated latency.
#[test]
#[serial]
fn test_latency_30ms() {
    let peer1_config = PeerConfig {
        local_port: 10009,
        player_index: 0,
        peer_addr: "127.0.0.1:10010".to_string(),
        frames: 100,
        latency_ms: 30,
        seed: Some(42),
        timeout_secs: 60,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10010,
        player_index: 1,
        peer_addr: "127.0.0.1:10009".to_string(),
        frames: 100,
        latency_ms: 30,
        seed: Some(43),
        timeout_secs: 60,
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    assert!(
        result1.success,
        "Peer 1 failed with 30ms latency: {:?}",
        result1.error
    );
    assert!(
        result2.success,
        "Peer 2 failed with 30ms latency: {:?}",
        result2.error
    );
    assert_eq!(result1.checksum, result2.checksum);
}

/// Test with 50ms latency and jitter.
#[test]
#[serial]
fn test_latency_with_jitter() {
    let peer1_config = PeerConfig {
        local_port: 10011,
        player_index: 0,
        peer_addr: "127.0.0.1:10012".to_string(),
        frames: 100,
        latency_ms: 50,
        jitter_ms: 20,
        seed: Some(42),
        timeout_secs: 90,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10012,
        player_index: 1,
        peer_addr: "127.0.0.1:10011".to_string(),
        frames: 100,
        latency_ms: 50,
        jitter_ms: 20,
        seed: Some(43),
        timeout_secs: 90,
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    assert!(
        result1.success,
        "Peer 1 failed with jitter: {:?}",
        result1.error
    );
    assert!(
        result2.success,
        "Peer 2 failed with jitter: {:?}",
        result2.error
    );
    assert_eq!(result1.checksum, result2.checksum);
}

// =============================================================================
// Combined Conditions Tests
// =============================================================================

/// Test "poor network" conditions: latency + loss + jitter combined.
#[test]
#[serial]
fn test_poor_network_combined() {
    let peer1_config = PeerConfig {
        local_port: 10013,
        player_index: 0,
        peer_addr: "127.0.0.1:10014".to_string(),
        frames: 100,
        packet_loss: 0.08,
        latency_ms: 40,
        jitter_ms: 15,
        seed: Some(42),
        timeout_secs: 120,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10014,
        player_index: 1,
        peer_addr: "127.0.0.1:10013".to_string(),
        frames: 100,
        packet_loss: 0.08,
        latency_ms: 40,
        jitter_ms: 15,
        seed: Some(43),
        timeout_secs: 120,
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    assert!(
        result1.success,
        "Peer 1 failed under poor network: {:?}",
        result1.error
    );
    assert!(
        result2.success,
        "Peer 2 failed under poor network: {:?}",
        result2.error
    );
    assert_eq!(
        result1.checksum, result2.checksum,
        "Desync detected under poor network conditions"
    );
}

/// Test asymmetric conditions - one peer has much worse network.
#[test]
#[serial]
fn test_asymmetric_network() {
    // Peer 1 has bad network
    let peer1_config = PeerConfig {
        local_port: 10015,
        player_index: 0,
        peer_addr: "127.0.0.1:10016".to_string(),
        frames: 100,
        packet_loss: 0.20,
        latency_ms: 80,
        seed: Some(42),
        timeout_secs: 120,
        ..Default::default()
    };

    // Peer 2 has good network
    let peer2_config = PeerConfig {
        local_port: 10016,
        player_index: 1,
        peer_addr: "127.0.0.1:10015".to_string(),
        frames: 100,
        packet_loss: 0.02,
        latency_ms: 10,
        seed: Some(43),
        timeout_secs: 120,
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    assert!(
        result1.success,
        "Peer 1 (bad network) failed: {:?}",
        result1.error
    );
    assert!(
        result2.success,
        "Peer 2 (good network) failed: {:?}",
        result2.error
    );
    assert_eq!(
        result1.checksum, result2.checksum,
        "Desync detected with asymmetric network"
    );

    // Peer 1 should have more rollbacks due to bad network
    println!(
        "Rollbacks - Peer 1 (bad network): {}, Peer 2 (good network): {}",
        result1.rollbacks, result2.rollbacks
    );
}

// =============================================================================
// Stress Tests
// =============================================================================

/// Long session (1000 frames) with moderate packet loss.
#[test]
#[serial]
#[ignore] // This test takes a while, run with --ignored
fn test_stress_long_session_with_loss() {
    let peer1_config = PeerConfig {
        local_port: 10017,
        player_index: 0,
        peer_addr: "127.0.0.1:10018".to_string(),
        frames: 1000,
        packet_loss: 0.05,
        seed: Some(42),
        timeout_secs: 180,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10018,
        player_index: 1,
        peer_addr: "127.0.0.1:10017".to_string(),
        frames: 1000,
        packet_loss: 0.05,
        seed: Some(43),
        timeout_secs: 180,
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    assert!(result1.success, "Peer 1 failed stress test: {:?}", result1.error);
    assert!(result2.success, "Peer 2 failed stress test: {:?}", result2.error);
    assert_eq!(result1.final_frame, 1000);
    assert_eq!(result2.final_frame, 1000);
    assert_eq!(result1.checksum, result2.checksum);

    println!(
        "Stress test complete - Peer 1 rollbacks: {}, Peer 2 rollbacks: {}",
        result1.rollbacks, result2.rollbacks
    );
}
