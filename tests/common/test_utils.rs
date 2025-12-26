//! Shared test utilities for integration tests.
//!
//! This module provides common constants, helper functions, and utilities
//! that are used across multiple test files to avoid duplication.

use fortress_rollback::{Config, FortressEvent, P2PSession, SessionState};
use std::hash::Hash;
use std::net::SocketAddr;
use std::thread;
use std::time::{Duration, Instant};

// ============================================================================
// Common Test Constants
// ============================================================================

/// Maximum iterations to wait for synchronization before giving up.
pub const MAX_SYNC_ITERATIONS: usize = 500;

/// Time to sleep between poll iterations to allow for proper timing.
/// This prevents tight loops that may not give the network layer enough time
/// to process messages, especially on systems with different scheduling behavior (e.g., macOS CI).
pub const POLL_INTERVAL: Duration = Duration::from_millis(1);

/// Maximum time to wait for synchronization to complete.
pub const SYNC_TIMEOUT: Duration = Duration::from_secs(5);

// ============================================================================
// Hash Utilities
// ============================================================================

/// Computes FNV-1a hash of any hashable type.
///
/// This is a convenience wrapper around `fortress_rollback::hash::fnv1a_hash`
/// for use in test code where we need deterministic checksums.
#[allow(dead_code)]
pub fn calculate_hash<T: Hash>(t: &T) -> u64 {
    fortress_rollback::hash::fnv1a_hash(t)
}

// ============================================================================
// Network Test Utilities
// ============================================================================

/// Creates a test socket address with localhost IP and the given port.
#[allow(dead_code)]
pub fn test_addr(port: u16) -> SocketAddr {
    use std::net::{IpAddr, Ipv4Addr};
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)
}

use fortress_rollback::{ChaosConfig, ChaosSocket, UdpNonBlockingSocket};

/// Helper to create a UDP socket wrapped with ChaosSocket for network resilience testing.
#[allow(dead_code)]
#[allow(clippy::expect_used)] // expect is acceptable in test utilities
pub fn create_chaos_socket(
    port: u16,
    config: ChaosConfig,
) -> ChaosSocket<SocketAddr, UdpNonBlockingSocket> {
    let inner = UdpNonBlockingSocket::bind_to_port(port).expect("Failed to bind chaos socket");
    ChaosSocket::new(inner, config)
}

// ============================================================================
// Synchronization Helpers
// ============================================================================

/// Synchronization configuration for test sessions.
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Maximum number of poll iterations before timing out.
    pub max_iterations: usize,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            max_iterations: MAX_SYNC_ITERATIONS,
        }
    }
}

/// Synchronizes two P2P sessions and returns the number of iterations taken.
///
/// This helper ensures BOTH sessions are in the `Running` state before returning.
/// The loop uses `||` (OR) in the condition because we want to continue while
/// at least one session is NOT running — this is the correct logic per De Morgan's law.
///
/// # Returns
/// - `Ok(iterations)` if both sessions synchronized successfully
/// - `Err(error message)` if synchronization timed out
#[allow(dead_code)]
pub fn synchronize_sessions<C: Config>(
    sess1: &mut P2PSession<C>,
    sess2: &mut P2PSession<C>,
    config: &SyncConfig,
) -> Result<usize, String> {
    let mut iterations = 0;
    let start = Instant::now();

    // Use || (OR) because we want to continue while EITHER session is not Running.
    // Using && would exit as soon as ONE session is Running, which is incorrect.
    while sess1.current_state() != SessionState::Running
        || sess2.current_state() != SessionState::Running
    {
        // Check both iteration count AND time-based timeout for robustness.
        // Time-based timeout is more reliable across different platforms (especially macOS CI).
        if iterations >= config.max_iterations || start.elapsed() > SYNC_TIMEOUT {
            return Err(format!(
                "Synchronization timed out after {} iterations ({:?}). \
                 sess1 state: {:?}, sess2 state: {:?}",
                iterations,
                start.elapsed(),
                sess1.current_state(),
                sess2.current_state()
            ));
        }

        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        iterations += 1;

        // Small sleep to allow network layer to process messages.
        // This is crucial on fast systems where tight loops may not give
        // the OS enough time to deliver UDP packets.
        thread::sleep(POLL_INTERVAL);
    }

    // Verify both are actually Running
    assert_eq!(
        sess1.current_state(),
        SessionState::Running,
        "Session 1 should be Running after synchronization"
    );
    assert_eq!(
        sess2.current_state(),
        SessionState::Running,
        "Session 2 should be Running after synchronization"
    );

    Ok(iterations)
}

/// Performs robust polling of two sessions with sleep intervals.
///
/// This helper ensures the network layer has adequate time to process packets,
/// which is crucial on systems with different scheduling behavior (e.g., macOS CI).
/// Without proper sleep intervals between polls, tight loops may not give the
/// OS enough time to deliver UDP packets.
///
/// # Arguments
/// * `sess1`, `sess2` - The sessions to poll
/// * `iterations` - Number of poll cycles (each cycle includes a sleep)
#[allow(dead_code)]
pub fn poll_with_sleep<C: Config>(
    sess1: &mut P2PSession<C>,
    sess2: &mut P2PSession<C>,
    iterations: usize,
) {
    for _ in 0..iterations {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        thread::sleep(POLL_INTERVAL);
    }
}

/// Drains synchronization events from sessions and returns them for inspection.
///
/// This should be called after `synchronize_sessions` to clear any accumulated
/// sync events before testing other functionality.
#[allow(dead_code)]
pub fn drain_sync_events<C: Config + std::fmt::Debug>(
    sess1: &mut P2PSession<C>,
    sess2: &mut P2PSession<C>,
) -> (Vec<FortressEvent<C>>, Vec<FortressEvent<C>>) {
    let events1: Vec<_> = sess1.events().collect();
    let events2: Vec<_> = sess2.events().collect();

    // Verify all events are sync-related
    for event in events1.iter().chain(events2.iter()) {
        assert!(
            matches!(
                event,
                FortressEvent::Synchronizing { .. } | FortressEvent::Synchronized { .. }
            ),
            "Expected sync event, got: {:?}",
            event
        );
    }

    (events1, events2)
}
