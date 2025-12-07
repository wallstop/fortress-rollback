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
    final_value: i64,
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
        cmd.arg("--packet-loss").arg(config.packet_loss.to_string());
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

    cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()
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
                final_value: 0,
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
///
/// NOTE: Checksum validation is currently disabled due to a known issue with
/// prediction-based rollback causing state divergence between peers. This is
/// tracked for investigation. The test still validates that both peers can
/// successfully complete the session and reach the target frame count.
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

    assert!(result1.success, "Peer 1 failed: {:?}", result1.error);
    assert!(result2.success, "Peer 2 failed: {:?}", result2.error);

    // Both should reach at least target frames
    // Note: final_frame can exceed target due to continued processing while waiting for confirmations
    assert!(
        result1.final_frame >= 100,
        "Peer 1 didn't reach target frames: {}",
        result1.final_frame
    );
    assert!(
        result2.final_frame >= 100,
        "Peer 2 didn't reach target frames: {}",
        result2.final_frame
    );

    // Verify determinism: both peers should reach the same final game state
    assert_eq!(
        result1.final_value, result2.final_value,
        "Desync detected! Final game state values differ: peer1={}, peer2={}",
        result1.final_value, result2.final_value
    );

    // For sessions ≤128 frames, also verify checksum (inputs stay in queue)
    assert_eq!(
        result1.checksum, result2.checksum,
        "Checksum mismatch - possible desync in input history"
    );
}

/// Test longer session (500 frames) to verify stability.
///
/// NOTE: For extended sessions (>128 frames), we verify determinism using final_value
/// instead of the input checksum. The checksum relies on confirmed inputs being available,
/// but older inputs are discarded as the session progresses (the input queue holds only
/// 128 frames). Using final_value is the correct determinism check because:
/// - It reflects the cumulative result of processing all confirmed inputs
/// - Both peers will have the same value after rollbacks complete and all inputs are confirmed
/// - It doesn't depend on which frames happen to still be in the input queue
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
    assert!(
        result1.final_frame >= 500,
        "Peer 1 didn't reach target frames: {}",
        result1.final_frame
    );
    assert!(
        result2.final_frame >= 500,
        "Peer 2 didn't reach target frames: {}",
        result2.final_frame
    );
    // For extended sessions, verify determinism via final game state value.
    // The checksum is unreliable for sessions > 128 frames because old inputs
    // get discarded, and the timing of when they're discarded can vary between peers.
    assert_eq!(
        result1.final_value, result2.final_value,
        "Desync detected! Final game state values differ: peer1={}, peer2={}",
        result1.final_value, result2.final_value
    );
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

    // Verify determinism via final game state
    assert_eq!(
        result1.final_value, result2.final_value,
        "Desync detected! Final values differ: peer1={}, peer2={}",
        result1.final_value, result2.final_value
    );
    // Also verify checksum for short sessions
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

    // Verify determinism via final game state
    assert_eq!(
        result1.final_value, result2.final_value,
        "Desync detected! Final values differ: peer1={}, peer2={}",
        result1.final_value, result2.final_value
    );
    // Also verify checksum for short sessions
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

    // Verify determinism via final game state
    assert_eq!(
        result1.final_value, result2.final_value,
        "Desync detected! Final values differ: peer1={}, peer2={}",
        result1.final_value, result2.final_value
    );
    // Also verify checksum for short sessions
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

    // Verify determinism via final game state
    assert_eq!(
        result1.final_value, result2.final_value,
        "Desync detected! Final values differ: peer1={}, peer2={}",
        result1.final_value, result2.final_value
    );
    // Also verify checksum for short sessions
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

    // Verify determinism via final game state
    assert_eq!(
        result1.final_value, result2.final_value,
        "Desync detected under poor network! Final values differ: peer1={}, peer2={}",
        result1.final_value, result2.final_value
    );
    // Also verify checksum for short sessions
    assert_eq!(
        result1.checksum, result2.checksum,
        "Checksum mismatch under poor network conditions"
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

    // Verify determinism via final game state
    assert_eq!(
        result1.final_value, result2.final_value,
        "Desync detected with asymmetric network! Final values differ: peer1={}, peer2={}",
        result1.final_value, result2.final_value
    );
    // Also verify checksum for short sessions
    assert_eq!(
        result1.checksum, result2.checksum,
        "Checksum mismatch with asymmetric network"
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

    assert!(
        result1.success,
        "Peer 1 failed stress test: {:?}",
        result1.error
    );
    assert!(
        result2.success,
        "Peer 2 failed stress test: {:?}",
        result2.error
    );
    assert!(
        result1.final_frame >= 1000,
        "Peer 1 didn't reach target frames: {}",
        result1.final_frame
    );
    assert!(
        result2.final_frame >= 1000,
        "Peer 2 didn't reach target frames: {}",
        result2.final_frame
    );

    // For long sessions (>128 frames), use final_value as the determinism check.
    // The checksum is unreliable because old inputs get discarded from the queue.
    assert_eq!(
        result1.final_value, result2.final_value,
        "Desync detected in stress test! Final values differ: peer1={}, peer2={}",
        result1.final_value, result2.final_value
    );

    println!(
        "Stress test complete - Peer 1 rollbacks: {}, Peer 2 rollbacks: {}",
        result1.rollbacks, result2.rollbacks
    );
}

// =============================================================================
// Additional Chaos Tests
// =============================================================================

/// Test with high jitter (variable latency).
#[test]
#[serial]
fn test_high_jitter_50ms() {
    let peer1_config = PeerConfig {
        local_port: 10019,
        player_index: 0,
        peer_addr: "127.0.0.1:10020".to_string(),
        frames: 100,
        latency_ms: 30,
        jitter_ms: 50, // High jitter
        seed: Some(42),
        timeout_secs: 90,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10020,
        player_index: 1,
        peer_addr: "127.0.0.1:10019".to_string(),
        frames: 100,
        latency_ms: 30,
        jitter_ms: 50,
        seed: Some(43),
        timeout_secs: 90,
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    assert!(
        result1.success,
        "Peer 1 failed with high jitter: {:?}",
        result1.error
    );
    assert!(
        result2.success,
        "Peer 2 failed with high jitter: {:?}",
        result2.error
    );

    // Verify determinism
    assert_eq!(
        result1.final_value, result2.final_value,
        "Desync with high jitter! peer1={}, peer2={}",
        result1.final_value, result2.final_value
    );
    assert_eq!(result1.checksum, result2.checksum);
}

/// Test with combined loss, latency, and jitter - simulating mobile network.
#[test]
#[serial]
fn test_mobile_network_simulation() {
    let peer1_config = PeerConfig {
        local_port: 10021,
        player_index: 0,
        peer_addr: "127.0.0.1:10022".to_string(),
        frames: 100,
        packet_loss: 0.12, // Mobile often has higher loss
        latency_ms: 60,    // Typical mobile latency
        jitter_ms: 40,     // High jitter on mobile
        seed: Some(42),
        timeout_secs: 120,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10022,
        player_index: 1,
        peer_addr: "127.0.0.1:10021".to_string(),
        frames: 100,
        packet_loss: 0.12,
        latency_ms: 60,
        jitter_ms: 40,
        seed: Some(43),
        timeout_secs: 120,
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    assert!(
        result1.success,
        "Peer 1 failed mobile simulation: {:?}",
        result1.error
    );
    assert!(
        result2.success,
        "Peer 2 failed mobile simulation: {:?}",
        result2.error
    );

    // Verify determinism
    assert_eq!(
        result1.final_value, result2.final_value,
        "Desync in mobile simulation! peer1={}, peer2={}",
        result1.final_value, result2.final_value
    );
    assert_eq!(result1.checksum, result2.checksum);

    println!(
        "Mobile simulation - Rollbacks: peer1={}, peer2={}",
        result1.rollbacks, result2.rollbacks
    );
}

/// Test heavily asymmetric conditions - one peer on great network, one on terrible.
#[test]
#[serial]
fn test_heavily_asymmetric_network() {
    // Peer 1 has terrible network conditions
    let peer1_config = PeerConfig {
        local_port: 10023,
        player_index: 0,
        peer_addr: "127.0.0.1:10024".to_string(),
        frames: 100,
        packet_loss: 0.25, // 25% loss!
        latency_ms: 100,   // 100ms latency
        jitter_ms: 40,
        seed: Some(42),
        timeout_secs: 180,
        ..Default::default()
    };

    // Peer 2 has excellent network
    let peer2_config = PeerConfig {
        local_port: 10024,
        player_index: 1,
        peer_addr: "127.0.0.1:10023".to_string(),
        frames: 100,
        packet_loss: 0.01, // 1% loss
        latency_ms: 5,     // 5ms latency
        jitter_ms: 2,
        seed: Some(43),
        timeout_secs: 180,
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    assert!(
        result1.success,
        "Peer 1 (terrible network) failed: {:?}",
        result1.error
    );
    assert!(
        result2.success,
        "Peer 2 (excellent network) failed: {:?}",
        result2.error
    );

    // Verify determinism despite massive asymmetry
    assert_eq!(
        result1.final_value, result2.final_value,
        "Desync with heavy asymmetry! peer1={}, peer2={}",
        result1.final_value, result2.final_value
    );
    assert_eq!(result1.checksum, result2.checksum);

    println!(
        "Asymmetric test - Rollbacks: bad_peer={}, good_peer={}",
        result1.rollbacks, result2.rollbacks
    );
}

/// Test with higher input delay (4 frames).
#[test]
#[serial]
fn test_higher_input_delay() {
    let peer1_config = PeerConfig {
        local_port: 10025,
        player_index: 0,
        peer_addr: "127.0.0.1:10026".to_string(),
        frames: 100,
        packet_loss: 0.05,
        latency_ms: 20,
        input_delay: 4, // Higher input delay
        seed: Some(42),
        timeout_secs: 60,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10026,
        player_index: 1,
        peer_addr: "127.0.0.1:10025".to_string(),
        frames: 100,
        packet_loss: 0.05,
        latency_ms: 20,
        input_delay: 4,
        seed: Some(43),
        timeout_secs: 60,
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    assert!(
        result1.success,
        "Peer 1 failed with input delay 4: {:?}",
        result1.error
    );
    assert!(
        result2.success,
        "Peer 2 failed with input delay 4: {:?}",
        result2.error
    );

    // Verify determinism
    assert_eq!(
        result1.final_value, result2.final_value,
        "Desync with input delay 4! peer1={}, peer2={}",
        result1.final_value, result2.final_value
    );
    assert_eq!(result1.checksum, result2.checksum);
}

/// Test with zero latency but high packet loss.
#[test]
#[serial]
fn test_zero_latency_high_loss() {
    let peer1_config = PeerConfig {
        local_port: 10027,
        player_index: 0,
        peer_addr: "127.0.0.1:10028".to_string(),
        frames: 100,
        packet_loss: 0.20, // 20% loss but no latency
        latency_ms: 0,
        seed: Some(42),
        timeout_secs: 90,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10028,
        player_index: 1,
        peer_addr: "127.0.0.1:10027".to_string(),
        frames: 100,
        packet_loss: 0.20,
        latency_ms: 0,
        seed: Some(43),
        timeout_secs: 90,
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    assert!(
        result1.success,
        "Peer 1 failed with zero latency + loss: {:?}",
        result1.error
    );
    assert!(
        result2.success,
        "Peer 2 failed with zero latency + loss: {:?}",
        result2.error
    );

    // Verify determinism
    assert_eq!(
        result1.final_value, result2.final_value,
        "Desync with zero latency + loss! peer1={}, peer2={}",
        result1.final_value, result2.final_value
    );
    assert_eq!(result1.checksum, result2.checksum);
}

/// Test medium-length session (300 frames) with moderate conditions.
#[test]
#[serial]
fn test_medium_session_300_frames() {
    let peer1_config = PeerConfig {
        local_port: 10029,
        player_index: 0,
        peer_addr: "127.0.0.1:10030".to_string(),
        frames: 300,
        packet_loss: 0.08,
        latency_ms: 35,
        jitter_ms: 15,
        seed: Some(42),
        timeout_secs: 120,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10030,
        player_index: 1,
        peer_addr: "127.0.0.1:10029".to_string(),
        frames: 300,
        packet_loss: 0.08,
        latency_ms: 35,
        jitter_ms: 15,
        seed: Some(43),
        timeout_secs: 120,
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    assert!(
        result1.success,
        "Peer 1 failed 300 frame session: {:?}",
        result1.error
    );
    assert!(
        result2.success,
        "Peer 2 failed 300 frame session: {:?}",
        result2.error
    );
    assert!(
        result1.final_frame >= 300,
        "Peer 1 didn't reach 300 frames: {}",
        result1.final_frame
    );
    assert!(
        result2.final_frame >= 300,
        "Peer 2 didn't reach 300 frames: {}",
        result2.final_frame
    );

    // For sessions >128 frames, use final_value for determinism check
    assert_eq!(
        result1.final_value, result2.final_value,
        "Desync in 300 frame session! peer1={}, peer2={}",
        result1.final_value, result2.final_value
    );
}

/// Stress test with very long session (2000 frames) - ignored by default.
#[test]
#[serial]
#[ignore] // Takes a long time, run with --ignored
fn test_stress_very_long_session() {
    let peer1_config = PeerConfig {
        local_port: 10031,
        player_index: 0,
        peer_addr: "127.0.0.1:10032".to_string(),
        frames: 2000,
        packet_loss: 0.03,
        latency_ms: 20,
        seed: Some(42),
        timeout_secs: 300, // 5 minutes
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10032,
        player_index: 1,
        peer_addr: "127.0.0.1:10031".to_string(),
        frames: 2000,
        packet_loss: 0.03,
        latency_ms: 20,
        seed: Some(43),
        timeout_secs: 300,
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    assert!(
        result1.success,
        "Peer 1 failed 2000 frame stress: {:?}",
        result1.error
    );
    assert!(
        result2.success,
        "Peer 2 failed 2000 frame stress: {:?}",
        result2.error
    );

    assert_eq!(
        result1.final_value, result2.final_value,
        "Desync in 2000 frame session!"
    );

    println!(
        "2000 frame stress complete - Rollbacks: peer1={}, peer2={}",
        result1.rollbacks, result2.rollbacks
    );
}

/// Test with different random seeds to validate determinism is seed-independent.
#[test]
#[serial]
fn test_determinism_different_seeds() {
    // Same network conditions but different chaos seeds
    let peer1_config = PeerConfig {
        local_port: 10033,
        player_index: 0,
        peer_addr: "127.0.0.1:10034".to_string(),
        frames: 100,
        packet_loss: 0.10,
        latency_ms: 30,
        seed: Some(12345), // Different seed
        timeout_secs: 90,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10034,
        player_index: 1,
        peer_addr: "127.0.0.1:10033".to_string(),
        frames: 100,
        packet_loss: 0.10,
        latency_ms: 30,
        seed: Some(67890), // Different seed
        timeout_secs: 90,
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    assert!(result1.success, "Peer 1 failed: {:?}", result1.error);
    assert!(result2.success, "Peer 2 failed: {:?}", result2.error);

    // Even with different chaos patterns, final state must match
    assert_eq!(
        result1.final_value, result2.final_value,
        "Desync with different seeds! Game logic must be deterministic regardless of network chaos patterns"
    );
    assert_eq!(result1.checksum, result2.checksum);
}
