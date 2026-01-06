//! Shared test utilities for integration tests.
//!
//! This module provides common constants, helper functions, and utilities
//! that are used across multiple test files to avoid duplication.
//!
//! # Port Allocation
//!
//! This module provides a thread-safe port allocation system to prevent port
//! conflicts in parallel tests. Use `PortAllocator` to get unique ports:
//!
//! ```ignore
//! use common::test_utils::PortAllocator;
//!
//! let port1 = PortAllocator::next_port();
//! let port2 = PortAllocator::next_port();
//! ```
//!
//! The allocator uses atomic operations to ensure thread-safety across parallel
//! test execution.

use fortress_rollback::{Config, FortressEvent, P2PSession, SessionState};
use std::hash::Hash;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU16, Ordering};
use std::thread;
use std::time::{Duration, Instant};

// ============================================================================
// Port Allocation System
// ============================================================================

/// Starting port for the atomic port allocator.
///
/// This value is chosen to avoid conflicts with:
/// - Well-known ports (0-1023)
/// - Registered ports commonly used by services (1024-49151)
/// - Ephemeral ports used by the OS (49152-65535)
/// - Other test files that may still use hardcoded ports (9000-9999)
///
/// Port 30000 provides a safe starting point with room for many allocations.
#[allow(dead_code)] // Some integration crates only use subsets of the allocator API.
const PORT_ALLOCATOR_START: u16 = 30000;

/// Ports allocated per test process to avoid conflicts.
/// Each test process gets 20 ports to accommodate data-driven tests
/// that allocate many ports in sequence.
#[allow(dead_code)]
const PORTS_PER_PROCESS: u16 = 20;

/// Global atomic counter for thread-safe port allocation.
///
/// Note: This counter is per-binary, not global across all test binaries.
/// To avoid port conflicts when multiple test binaries run in parallel,
/// we offset the starting port based on the process ID (PID).
/// nextest runs each test in a separate process, so PID provides unique ranges.
/// The #[serial] attribute ensures tests within the same binary don't conflict.
static PORT_COUNTER: AtomicU16 = AtomicU16::new(0);

/// Flag to track if the port counter has been initialized with a binary-specific offset.
static PORT_COUNTER_INITIALIZED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Thread-safe port allocator for integration tests.
///
/// This allocator ensures that each test gets unique ports, preventing
/// "Address already in use" errors when tests run in parallel.
///
/// # Example
///
/// ```ignore
/// use common::test_utils::PortAllocator;
///
/// // Get a single port
/// let port = PortAllocator::next_port();
///
/// // Get multiple ports for a multi-peer test
/// let ports = PortAllocator::next_ports::<4>();
/// ```
///
/// # Thread Safety
///
/// The allocator uses atomic operations, making it safe to use across
/// multiple test threads without locks.
pub struct PortAllocator;

impl PortAllocator {
    /// Initializes the port counter with a process-specific offset.
    /// This is called lazily on the first port allocation.
    ///
    /// nextest runs each test in a separate process, so we use the process ID
    /// to ensure each test process gets a unique port range.
    #[allow(dead_code)]
    fn initialize_counter() {
        // Only initialize once using compare-and-swap
        if PORT_COUNTER_INITIALIZED
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            // Use the process ID to get a unique offset per test process.
            // nextest runs each test in its own process, so this ensures unique ports.
            let pid = std::process::id();

            // Spread across the available range (30000-59999)
            // Use modulo to map PID to a port offset
            // With a range of 30000 ports and PORTS_PER_PROCESS ports per test,
            // we can support many concurrent test processes.
            let max_offsets = (60000 - PORT_ALLOCATOR_START) / PORTS_PER_PROCESS;
            let offset_index = (pid as u16) % max_offsets;
            let start_port = PORT_ALLOCATOR_START + (offset_index * PORTS_PER_PROCESS);

            PORT_COUNTER.store(start_port, Ordering::SeqCst);
        }
    }

    /// Allocates the next available port.
    ///
    /// This method is thread-safe and can be called from parallel tests.
    /// Each call returns a unique port number.
    ///
    /// # Panics
    ///
    /// Panics if the port counter would overflow (after many allocations
    /// from the starting point). This should never happen in practice.
    #[allow(dead_code)]
    #[must_use]
    pub fn next_port() -> u16 {
        // Ensure counter is initialized with a binary-specific offset
        Self::initialize_counter();

        let port = PORT_COUNTER.fetch_add(1, Ordering::SeqCst);
        assert!(
            port < 60000,
            "Port allocator exhausted. This indicates a test suite issue."
        );
        port
    }

    /// Allocates N consecutive ports.
    ///
    /// This is useful for multi-peer tests that need a known set of ports.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // For a 4-player P2P session
    /// let [p1, p2, p3, p4] = PortAllocator::next_ports::<4>();
    /// ```
    #[allow(dead_code)]
    #[must_use]
    pub fn next_ports<const N: usize>() -> [u16; N] {
        let mut ports = [0u16; N];
        for port in &mut ports {
            *port = Self::next_port();
        }
        ports
    }

    /// Allocates a pair of ports (convenience method for 2-player sessions).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let (port1, port2) = PortAllocator::next_pair();
    /// ```
    #[allow(dead_code)]
    #[must_use]
    pub fn next_pair() -> (u16, u16) {
        (Self::next_port(), Self::next_port())
    }

    /// Resets the port counter to the starting value.
    ///
    /// **Warning**: This should only be called in test setup when you're
    /// certain no other tests are running. Using this incorrectly can cause
    /// port conflicts.
    ///
    /// This is primarily useful for:
    /// - Test isolation in single-threaded test scenarios
    /// - Debugging port allocation issues
    #[cfg(test)]
    #[allow(dead_code)] // Provided for test isolation
    pub fn reset() {
        PORT_COUNTER_INITIALIZED.store(false, Ordering::SeqCst);
        PORT_COUNTER.store(0, Ordering::SeqCst);
    }
}

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

use fortress_rollback::{ChaosConfig, ChaosSocket, FortressError, UdpNonBlockingSocket};

/// Maximum number of retry attempts for socket binding on Windows.
///
/// Windows CI (especially GitHub Actions runners) can experience transient
/// WSAEACCES (error 10013) errors when binding sockets. This is often caused
/// by port conflicts or exclusive address use issues that resolve after a
/// short delay.
#[cfg(target_os = "windows")]
const SOCKET_BIND_MAX_RETRIES: u32 = 5;

/// Delay between socket bind retry attempts on Windows.
#[cfg(target_os = "windows")]
const SOCKET_BIND_RETRY_DELAY_MS: u64 = 100;

/// Helper to create a UDP socket wrapped with ChaosSocket for network resilience testing.
///
/// # Windows Retry Logic
///
/// On Windows, this function includes retry logic to handle transient WSAEACCES
/// (error 10013) errors that occur on GitHub Actions Windows runners. These errors
/// are caused by port conflicts or exclusive address use issues that typically
/// resolve after a short delay.
///
/// The retry logic:
/// - Retries up to 5 times on WSAEACCES errors
/// - Waits 100ms between attempts
/// - Only retries on error code 10013 (WSAEACCES), not other errors
#[allow(dead_code)]
#[track_caller]
pub fn create_chaos_socket(
    port: u16,
    config: ChaosConfig,
) -> Result<ChaosSocket<SocketAddr, UdpNonBlockingSocket>, FortressError> {
    let inner = bind_socket_with_retry(port)?;
    Ok(ChaosSocket::new(inner, config))
}

/// Binds a UDP socket with retry logic for Windows CI.
///
/// On non-Windows platforms, this is a simple bind without retries.
/// On Windows, it retries on WSAEACCES (error 10013) errors.
#[track_caller]
fn bind_socket_with_retry(port: u16) -> Result<UdpNonBlockingSocket, FortressError> {
    #[cfg(not(target_os = "windows"))]
    {
        return UdpNonBlockingSocket::bind_to_port(port).map_err(|error| {
            FortressError::SocketError {
                context: format!("Failed to bind chaos socket on port {port}: {error}"),
            }
        });
    }

    #[cfg(target_os = "windows")]
    {
        use std::io::ErrorKind;

        let mut last_error = None;

        for attempt in 0..SOCKET_BIND_MAX_RETRIES {
            match UdpNonBlockingSocket::bind_to_port(port) {
                Ok(socket) => return Ok(socket),
                Err(error) => {
                    // WSAEACCES is error code 10013 on Windows
                    // It manifests as PermissionDenied in Rust's std::io::ErrorKind
                    let is_wsaeacces = error.kind() == ErrorKind::PermissionDenied
                        || error.raw_os_error() == Some(10013);

                    if is_wsaeacces && attempt + 1 < SOCKET_BIND_MAX_RETRIES {
                        tracing::warn!(
                            attempt = attempt + 1,
                            port,
                            delay_ms = SOCKET_BIND_RETRY_DELAY_MS,
                            "Socket bind attempt failed with WSAEACCES, retrying"
                        );
                        std::thread::sleep(std::time::Duration::from_millis(
                            SOCKET_BIND_RETRY_DELAY_MS,
                        ));
                        last_error = Some(error);
                        continue;
                    }
                    // Non-retryable error or max retries exceeded
                    let context = format!(
                        "Failed to bind chaos socket on port {} after {} attempts: {}",
                        port,
                        attempt + 1,
                        error
                    );
                    return Err(FortressError::SocketError { context });
                },
            }
        }

        // This should only be reached if all retries failed with WSAEACCES
        let context = format!(
            "Failed to bind chaos socket on port {} after {} attempts: {:?}",
            port, SOCKET_BIND_MAX_RETRIES, last_error
        );
        Err(FortressError::SocketError { context })
    }
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
#[track_caller]
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
#[track_caller]
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

// ============================================================================
// Spectator Session Synchronization
// ============================================================================

use fortress_rollback::SpectatorSession;

/// Result of a synchronization attempt.
#[derive(Debug)]
pub struct SyncResult {
    /// Number of poll iterations it took to synchronize.
    pub iterations: usize,
    /// Time elapsed during synchronization.
    pub elapsed: Duration,
    /// Whether both sessions are now in Running state.
    pub success: bool,
}

/// Polls a spectator session and host until they reach the Running state or timeout.
///
/// This function is more robust than a fixed number of iterations because:
/// 1. It uses actual time-based timeout instead of iteration count
/// 2. It includes small sleeps between iterations to allow proper message processing
/// 3. It provides diagnostic information on failure
///
/// # Arguments
/// * `spec_sess` - The spectator session to synchronize
/// * `host_sess` - The host P2P session to synchronize
///
/// # Returns
/// `SyncResult` with synchronization outcome and diagnostics.
#[allow(dead_code)]
#[track_caller]
pub fn synchronize_spectator<C: Config>(
    spec_sess: &mut SpectatorSession<C>,
    host_sess: &mut P2PSession<C>,
) -> SyncResult {
    let start = Instant::now();
    let mut iterations = 0;

    while start.elapsed() < SYNC_TIMEOUT && iterations < MAX_SYNC_ITERATIONS {
        spec_sess.poll_remote_clients();
        host_sess.poll_remote_clients();
        iterations += 1;

        // Check if both sessions are synchronized
        if spec_sess.current_state() == SessionState::Running
            && host_sess.current_state() == SessionState::Running
        {
            return SyncResult {
                iterations,
                elapsed: start.elapsed(),
                success: true,
            };
        }

        // Small sleep to allow network layer to process messages
        // This is especially important on fast systems where tight loops
        // may not give the OS enough time to deliver UDP packets
        thread::sleep(POLL_INTERVAL);
    }

    SyncResult {
        iterations,
        elapsed: start.elapsed(),
        success: false,
    }
}

/// Asserts that synchronization completed successfully, with detailed diagnostics on failure.
#[allow(dead_code)]
#[track_caller]
pub fn assert_spectator_synchronized<C: Config>(
    spec_sess: &SpectatorSession<C>,
    host_sess: &P2PSession<C>,
    result: &SyncResult,
) {
    assert!(
        result.success,
        "Synchronization failed after {} iterations ({:?}).\n\
         Spectator state: {:?}\n\
         Host state: {:?}\n\
         This may indicate a timing issue on this platform.",
        result.iterations,
        result.elapsed,
        spec_sess.current_state(),
        host_sess.current_state()
    );
}

// ============================================================================
// Generic P2P Session Test Helpers
// ============================================================================

use fortress_rollback::{FortressRequest, PlayerHandle, PlayerType};

/// Trait for game stubs that can handle fortress requests.
///
/// This allows generic test helpers to work with different stub implementations
/// (e.g., GameStub with struct inputs, GameStubEnum with enum inputs).
pub trait GameStubHandler<C: Config> {
    /// The game state type used by this stub.
    type State;

    /// Creates a new instance of this stub.
    fn new() -> Self;

    /// Handles a list of fortress requests.
    fn handle_requests(&mut self, requests: Vec<FortressRequest<C>>);

    /// Returns the current frame number of the game state.
    fn current_frame(&self) -> i32;
}

/// Generic test for P2P frame advancement.
///
/// This helper runs a complete P2P session test:
/// 1. Creates two P2P sessions with the provided ports
/// 2. Synchronizes them
/// 3. Advances frames using the provided input generator
/// 4. Verifies frames advanced correctly
///
/// # Type Parameters
/// * `C` - The Config type to use
/// * `S` - The game stub type (must implement GameStubHandler<C>)
///
/// # Arguments
/// * `port1`, `port2` - Ports for the two sessions
/// * `input_gen` - Function that generates input for a given frame number
/// * `num_frames` - Number of frames to advance
#[allow(dead_code, clippy::expect_used)]
#[track_caller]
pub fn run_p2p_frame_advancement_test<C, S>(
    port1: u16,
    port2: u16,
    input_gen: impl Fn(u32) -> C::Input,
    num_frames: u32,
) -> Result<(), FortressError>
where
    C: Config<Address = SocketAddr>,
    S: GameStubHandler<C>,
{
    use fortress_rollback::{SessionBuilder, UdpNonBlockingSocket};
    use std::net::{IpAddr, Ipv4Addr};

    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port1);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port2);

    let socket1 = UdpNonBlockingSocket::bind_to_port(port1).expect("Failed to bind socket 1");
    let mut sess1 = SessionBuilder::<C>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let socket2 = UdpNonBlockingSocket::bind_to_port(port2).expect("Failed to bind socket 2");
    let mut sess2 = SessionBuilder::<C>::new()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    assert!(sess1.current_state() == SessionState::Synchronizing);
    assert!(sess2.current_state() == SessionState::Synchronizing);

    // Use robust synchronization with time-based timeout
    synchronize_sessions(&mut sess1, &mut sess2, &SyncConfig::default())
        .expect("Sessions should synchronize");

    assert!(sess1.current_state() == SessionState::Running);
    assert!(sess2.current_state() == SessionState::Running);

    let mut stub1 = S::new();
    let mut stub2 = S::new();

    for i in 0..num_frames {
        // Poll with multiple iterations and sleep to ensure packets are delivered
        poll_with_sleep(&mut sess1, &mut sess2, 3);

        sess1.add_local_input(PlayerHandle::new(0), input_gen(i))?;
        let requests1 = sess1.advance_frame()?;
        stub1.handle_requests(requests1);

        sess2.add_local_input(PlayerHandle::new(1), input_gen(i))?;
        let requests2 = sess2.advance_frame()?;
        stub2.handle_requests(requests2);

        // Gamestate evolves
        assert_eq!(stub1.current_frame(), i as i32 + 1);
        assert_eq!(stub2.current_frame(), i as i32 + 1);
    }

    Ok(())
}

// ============================================================================
// Generic SyncTest Session Test Helpers
// ============================================================================

/// Generic test for SyncTest frame advancement with delayed input.
///
/// This helper runs a complete SyncTest session test:
/// 1. Creates a SyncTest session with the provided configuration
/// 2. Advances frames using the provided input generator
/// 3. Verifies frames advanced correctly
///
/// # Type Parameters
/// * `C` - The Config type to use
/// * `S` - The game stub type (must implement GameStubHandler<C>)
///
/// # Arguments
/// * `check_distance` - The check distance for rollback testing
/// * `input_delay` - Input delay for the session
/// * `input_gen` - Function that generates input for a given frame number
/// * `num_frames` - Number of frames to advance
#[allow(dead_code, clippy::expect_used)]
#[track_caller]
pub fn run_synctest_with_delayed_input<C, S>(
    check_distance: usize,
    input_delay: usize,
    input_gen: impl Fn(u32) -> C::Input,
    num_frames: u32,
) -> Result<(), FortressError>
where
    C: Config,
    S: GameStubHandler<C>,
{
    use fortress_rollback::SessionBuilder;

    let mut stub = S::new();
    let mut sess = SessionBuilder::<C>::new()
        .with_check_distance(check_distance)
        .with_input_delay(input_delay)
        .expect("Valid input delay")
        .start_synctest_session()?;

    for i in 0..num_frames {
        let input = input_gen(i);
        sess.add_local_input(PlayerHandle::new(0), input)?;
        sess.add_local_input(PlayerHandle::new(1), input)?;
        let requests = sess.advance_frame()?;
        stub.handle_requests(requests);
        assert_eq!(
            stub.current_frame(),
            i as i32 + 1,
            "Frame should have advanced"
        );
    }

    Ok(())
}
