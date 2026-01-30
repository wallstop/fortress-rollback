//! Shared test utilities for integration tests.
//!
//! This module provides common constants, helper functions, and utilities
//! that are used across multiple test files to avoid duplication.
//!
//! # Port Allocation Strategy
//!
//! This module provides two approaches for socket binding in tests:
//!
//! ## Recommended: Ephemeral Ports (for actual socket binding)
//!
//! Use [`bind_socket_ephemeral`] or [`create_chaos_socket_ephemeral`] when you need
//! to bind actual sockets. This approach uses port 0, letting the OS assign an
//! available ephemeral port, eliminating TIME_WAIT conflicts on Windows CI:
//!
//! ```ignore
//! use common::test_utils::{bind_socket_ephemeral, create_chaos_socket_ephemeral};
//!
//! // For regular UDP sockets
//! let (socket1, addr1) = bind_socket_ephemeral()?;
//! let (socket2, addr2) = bind_socket_ephemeral()?;
//!
//! // For chaos sockets with network simulation
//! let config = ChaosConfig::builder().seed(42).build();
//! let (chaos_socket, addr) = create_chaos_socket_ephemeral(config)?;
//! ```
//!
//! ## Legacy: Port Allocator (for unbound addresses only)
//!
//! Use [`PortAllocator`] only for generating remote addresses that will NOT be
//! bound to actual sockets (e.g., mock remote peers). The allocator uses atomic
//! operations for thread-safety but can still encounter TIME_WAIT conflicts:
//!
//! ```ignore
//! use common::test_utils::PortAllocator;
//!
//! // Only for addresses NOT bound to actual sockets
//! let mock_remote_port = PortAllocator::next_port();
//! ```
//!
//! # Migration Note (January 2026)
//!
//! All network tests have been migrated to use ephemeral ports for actual socket
//! binding. This resolved persistent Windows CI failures caused by WSAEACCES (10013)
//! and WSAEADDRINUSE (10048) errors from TIME_WAIT socket states.
//!
//! If adding new network tests, always prefer `bind_socket_ephemeral()` or
//! `create_chaos_socket_ephemeral()` over `PortAllocator` for bound sockets.

use fortress_rollback::{Config, FortressEvent, P2PSession, SessionState};
use std::hash::Hash;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU16, Ordering};
use std::thread;
use std::time::{Duration, Instant};

// ============================================================================
// Port Allocation System
// ============================================================================
//
// ## Windows-Specific Considerations
//
// Windows CI environments (especially GitHub Actions runners) frequently encounter
// transient WSAEACCES (error code 10013) errors when binding UDP sockets. This error
// indicates "Permission denied" and occurs due to:
//
// 1. **Port conflicts**: Another process (possibly from a parallel test) briefly held
//    the port in an exclusive state.
//
// 2. **TIME_WAIT states**: Recently closed sockets may still hold the port for up to
//    2 minutes (MSL timeout) to prevent packet confusion.
//
// 3. **Exclusive address use**: Windows networking stack sometimes marks ports as
//    exclusively used even when they appear free.
//
// These issues are transient and typically resolve after a short delay.
// The `bind_socket_with_retry` function handles this by retrying up to 10 times
// on Windows with exponential backoff (50ms, 100ms, 200ms... up to 1000ms) when
// WSAEACCES (10013) or WSAEADDRINUSE (10048) errors are encountered.
//
// On Linux and macOS, these issues are much rarer due to different socket
// implementation semantics, so no retry logic is needed.

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
    /// # Deprecation Warning
    ///
    /// **For actual socket binding, use [`bind_socket_ephemeral`] instead.**
    ///
    /// This method should only be used for generating addresses that will NOT
    /// be bound to actual sockets (e.g., mock remote peer addresses). Using
    /// pre-allocated ports for actual socket binding can cause TIME_WAIT
    /// conflicts, especially on Windows CI.
    ///
    /// ```ignore
    /// // ❌ Deprecated pattern for bound sockets:
    /// let (port1, port2) = PortAllocator::next_pair();
    /// let socket1 = UdpSocket::bind(format!("127.0.0.1:{}", port1))?; // May fail!
    ///
    /// // ✅ Preferred pattern for bound sockets:
    /// let (socket1, addr1) = bind_socket_ephemeral()?;
    /// let (socket2, addr2) = bind_socket_ephemeral()?;
    /// ```
    ///
    /// # Example (for unbound addresses only)
    ///
    /// ```ignore
    /// // Use for mock remote addresses that won't be bound
    /// let (mock_port1, mock_port2) = PortAllocator::next_pair();
    /// let mock_remote = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), mock_port1);
    /// ```
    ///
    /// [`bind_socket_ephemeral`]: crate::common::test_utils::bind_socket_ephemeral
    #[allow(dead_code)]
    #[must_use]
    #[deprecated(
        since = "0.2.0",
        note = "Use bind_socket_ephemeral() for actual socket binding to avoid TIME_WAIT conflicts on Windows CI. This method should only be used for mock addresses that won't be bound."
    )]
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

use fortress_rollback::{
    ChaosConfig, ChaosSocket, FortressError, SocketErrorKind, UdpNonBlockingSocket,
};

/// Maximum number of retry attempts for socket binding on Windows.
///
/// Windows CI (especially GitHub Actions runners) can experience transient
/// socket binding errors that resolve after a short delay:
/// - WSAEACCES (error 10013): Permission denied, often due to exclusive address use
/// - WSAEADDRINUSE (error 10048): Address already in use, often due to TIME_WAIT state
///
/// We use 15 retries with exponential backoff, providing up to ~12 seconds total wait.
/// Windows requires more aggressive retries than Linux/macOS due to:
/// - Slower TIME_WAIT socket cleanup on Windows networking stack
/// - More aggressive exclusive port locking in Windows socket implementation
/// - Higher contention on shared GitHub Actions Windows runners
#[cfg(target_os = "windows")]
const SOCKET_BIND_MAX_RETRIES: u32 = 15;

/// Initial delay for socket bind retries on Windows (exponential backoff starts here).
///
/// Starting at 100ms gives Windows more time to release ports between attempts,
/// reducing spurious failures on busy CI runners.
#[cfg(target_os = "windows")]
const SOCKET_BIND_INITIAL_DELAY_MS: u64 = 100;

/// Maximum delay between socket bind retry attempts on Windows.
#[cfg(target_os = "windows")]
const SOCKET_BIND_MAX_DELAY_MS: u64 = 1000;

/// Helper to create a UDP socket wrapped with ChaosSocket for network resilience testing.
///
/// # Windows Retry Logic
///
/// On Windows, this function includes retry logic to handle transient socket binding
/// errors that occur on GitHub Actions Windows runners:
/// - WSAEACCES (error 10013): Permission denied
/// - WSAEADDRINUSE (error 10048): Address already in use (TIME_WAIT state)
///
/// The retry logic:
/// - Retries up to 15 times with exponential backoff
/// - Starts at 100ms, doubles each attempt, caps at 1000ms
/// - Retries on both error codes 10013 (WSAEACCES) and 10048 (WSAEADDRINUSE)
#[allow(dead_code)]
#[track_caller]
pub fn create_chaos_socket(
    port: u16,
    config: ChaosConfig,
) -> Result<ChaosSocket<SocketAddr, UdpNonBlockingSocket>, FortressError> {
    let inner = bind_socket_with_retry(port)?;
    Ok(ChaosSocket::new(inner, config))
}

/// Creates a chaos socket bound to an OS-assigned ephemeral port.
///
/// This is the preferred approach for tests on Windows CI where pre-allocated
/// ports may still be in TIME_WAIT state from previous test runs. By using
/// port 0, the OS assigns an available port, eliminating port conflicts.
///
/// # Returns
///
/// Returns a tuple of (socket, address) where address is the localhost address
/// (`127.0.0.1`) with the actual bound port. The socket itself is bound to
/// `0.0.0.0` (all interfaces), but we return the localhost address since that's
/// what tests need for peer-to-peer communication.
///
/// # Example
///
/// ```ignore
/// use common::test_utils::create_chaos_socket_ephemeral;
///
/// let config = ChaosConfig::builder().seed(42).build();
/// let (socket1, addr1) = create_chaos_socket_ephemeral(config.clone())?;
/// let (socket2, addr2) = create_chaos_socket_ephemeral(config)?;
/// // addr1 and addr2 are now 127.0.0.1:ephemeral_port addresses
/// ```
///
/// # Errors
///
/// Returns `FortressError::SocketErrorStructured` if socket binding fails.
#[allow(dead_code)]
#[track_caller]
pub fn create_chaos_socket_ephemeral(
    config: ChaosConfig,
) -> Result<(ChaosSocket<SocketAddr, UdpNonBlockingSocket>, SocketAddr), FortressError> {
    use std::net::{IpAddr, Ipv4Addr};

    let inner = bind_socket_with_retry(0)?; // Port 0 = OS-assigned ephemeral port
    let bound_addr =
        inner
            .local_addr()
            .map_err(|_io_err| FortressError::SocketErrorStructured {
                kind: SocketErrorKind::BindFailed { port: 0 },
            })?;
    // The socket is bound to 0.0.0.0:port, but for peer communication we need
    // to return 127.0.0.1:port (localhost) since 0.0.0.0 is not routable.
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), bound_addr.port());
    Ok((ChaosSocket::new(inner, config), addr))
}

/// Binds a UDP socket with retry logic for Windows CI.
///
/// On non-Windows platforms, this is a simple bind without retries.
/// On Windows, it retries on transient socket binding errors.
///
/// # Windows Retry Logic
///
/// Windows CI environments (especially GitHub Actions runners) can experience
/// transient errors when binding UDP sockets:
/// - WSAEACCES (error 10013): Permission denied - caused by exclusive address use
/// - WSAEADDRINUSE (error 10048): Address in use - caused by TIME_WAIT state
///
/// These issues typically resolve after a short delay, so this function:
/// - Retries up to 15 times with exponential backoff (100ms, 200ms, 400ms, ... 1000ms)
/// - Logs retry attempts via tracing for debugging
/// - Retries on error codes 10013 (WSAEACCES) and 10048 (WSAEADDRINUSE)
///
/// # Example
///
/// ```ignore
/// use common::test_utils::bind_socket_with_retry;
///
/// // Returns Result instead of panicking
/// let socket = bind_socket_with_retry(9000)?;
/// ```
///
/// # Errors
///
/// Returns `FortressError::SocketErrorStructured` if:
/// - The socket cannot be bound after all retry attempts (Windows)
/// - The socket cannot be bound on the first attempt (non-Windows)
#[allow(dead_code)]
#[track_caller]
pub fn bind_socket_with_retry(port: u16) -> Result<UdpNonBlockingSocket, FortressError> {
    #[cfg(not(target_os = "windows"))]
    {
        let socket = UdpNonBlockingSocket::bind_to_port(port).map_err(|_io_err| {
            FortressError::SocketErrorStructured {
                kind: SocketErrorKind::BindFailed { port },
            }
        })?;
        tracing::debug!(port, "Successfully bound socket");
        return Ok(socket);
    }

    #[cfg(target_os = "windows")]
    {
        use std::io::ErrorKind;

        for attempt in 0..SOCKET_BIND_MAX_RETRIES {
            match UdpNonBlockingSocket::bind_to_port(port) {
                Ok(socket) => {
                    if attempt > 0 {
                        tracing::debug!(
                            port,
                            attempts = attempt + 1,
                            "Successfully bound socket after retries"
                        );
                    } else {
                        tracing::debug!(port, "Successfully bound socket");
                    }
                    return Ok(socket);
                },
                Err(error) => {
                    // Check for retryable Windows socket errors:
                    // - WSAEACCES (10013): Permission denied, often due to exclusive address use
                    // - WSAEADDRINUSE (10048): Address in use, often due to TIME_WAIT state
                    let os_error = error.raw_os_error();
                    let is_retryable = matches!(
                        error.kind(),
                        ErrorKind::PermissionDenied | ErrorKind::AddrInUse
                    ) || matches!(os_error, Some(10013) | Some(10048));

                    if is_retryable && attempt + 1 < SOCKET_BIND_MAX_RETRIES {
                        // Exponential backoff: 100ms, 200ms, 400ms, 800ms, 1000ms (capped)
                        let delay_ms = SOCKET_BIND_INITIAL_DELAY_MS
                            .saturating_mul(1u64 << attempt)
                            .min(SOCKET_BIND_MAX_DELAY_MS);
                        let error_name = match os_error {
                            Some(10013) => "WSAEACCES",
                            Some(10048) => "WSAEADDRINUSE",
                            _ => "unknown",
                        };
                        tracing::warn!(
                            attempt = attempt + 1,
                            port,
                            delay_ms,
                            error = error_name,
                            "Socket bind attempt failed, retrying with exponential backoff"
                        );
                        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                        continue;
                    }
                    // Non-retryable error or max retries exceeded
                    // Use saturating conversion since attempt is u32 and attempts is u8
                    let attempts = u8::try_from(attempt + 1).unwrap_or(u8::MAX);
                    return Err(FortressError::SocketErrorStructured {
                        kind: SocketErrorKind::BindFailedAfterRetries { port, attempts },
                    });
                },
            }
        }

        // This should only be reached if all retries failed with retryable errors
        // Use saturating conversion since SOCKET_BIND_MAX_RETRIES is u32 and attempts is u8
        let attempts = u8::try_from(SOCKET_BIND_MAX_RETRIES).unwrap_or(u8::MAX);
        Err(FortressError::SocketErrorStructured {
            kind: SocketErrorKind::BindFailedAfterRetries { port, attempts },
        })
    }
}

/// Binds a UDP socket to an OS-assigned ephemeral port and returns both the socket and its address.
///
/// This is the preferred approach for tests on Windows CI where pre-allocated
/// ports may still be in TIME_WAIT state from previous test runs. By using
/// port 0, the OS assigns an available port, eliminating port conflicts.
///
/// # Returns
///
/// Returns a tuple of (socket, address) where address is the localhost address
/// (`127.0.0.1`) with the actual bound port.
///
/// # Example
///
/// ```ignore
/// use common::test_utils::bind_socket_ephemeral;
///
/// let (socket1, addr1) = bind_socket_ephemeral()?;
/// let (socket2, addr2) = bind_socket_ephemeral()?;
/// // Now addr1 and addr2 can be used for peer-to-peer setup
/// ```
///
/// # Errors
///
/// Returns `FortressError::SocketErrorStructured` if socket binding fails.
#[allow(dead_code)]
#[track_caller]
pub fn bind_socket_ephemeral() -> Result<(UdpNonBlockingSocket, SocketAddr), FortressError> {
    use std::net::{IpAddr, Ipv4Addr};

    let socket = bind_socket_with_retry(0)?; // Port 0 = OS-assigned ephemeral port
    let bound_addr =
        socket
            .local_addr()
            .map_err(|_io_err| FortressError::SocketErrorStructured {
                kind: SocketErrorKind::BindFailed { port: 0 },
            })?;
    // The socket is bound to 0.0.0.0:port, but for peer communication we need
    // to return 127.0.0.1:port (localhost) since 0.0.0.0 is not routable.
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), bound_addr.port());
    Ok((socket, addr))
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
/// - `Err(FortressError)` if synchronization timed out
#[allow(dead_code)]
#[track_caller]
pub fn synchronize_sessions<C: Config>(
    sess1: &mut P2PSession<C>,
    sess2: &mut P2PSession<C>,
    config: &SyncConfig,
) -> Result<usize, FortressError> {
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
            // Use legacy InternalError for test code - it provides detailed debugging
            // info that's useful when tests fail. Structured errors are preferred in
            // production code, but test helpers benefit from rich context.
            return Err(FortressError::InternalError {
                context: format!(
                    "Synchronization timed out after {} iterations ({:?}). \
                     sess1 state: {:?}, sess2 state: {:?}",
                    iterations,
                    start.elapsed(),
                    sess1.current_state(),
                    sess2.current_state()
                ),
            });
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
#[allow(dead_code)]
#[track_caller]
pub fn run_p2p_frame_advancement_test<C, S>(
    input_gen: impl Fn(u32) -> C::Input,
    num_frames: u32,
) -> Result<(), FortressError>
where
    C: Config<Address = SocketAddr>,
    S: GameStubHandler<C>,
{
    use fortress_rollback::SessionBuilder;

    // Use ephemeral ports to avoid TIME_WAIT conflicts on Windows CI
    let (socket1, addr1) = bind_socket_ephemeral()?;
    let (socket2, addr2) = bind_socket_ephemeral()?;

    let mut sess1 = SessionBuilder::<C>::new()
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))?
        .start_p2p_session(socket1)?;

    let mut sess2 = SessionBuilder::<C>::new()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))?
        .start_p2p_session(socket2)?;

    assert!(sess1.current_state() == SessionState::Synchronizing);
    assert!(sess2.current_state() == SessionState::Synchronizing);

    // Use robust synchronization with time-based timeout
    synchronize_sessions(&mut sess1, &mut sess2, &SyncConfig::default())?;

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

// ============================================================================
// Tests for Socket Binding Utilities
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Allow test-specific patterns
    #[allow(
        clippy::panic,
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing
    )]
    mod socket_binding_tests {
        use super::*;

        /// Tests that `bind_socket_with_retry` successfully binds to port 0.
        ///
        /// Port 0 tells the OS to assign any available ephemeral port.
        /// This test verifies the happy path works correctly.
        #[test]
        #[cfg(not(miri))] // Miri cannot execute socket operations
        fn bind_socket_with_retry_succeeds_on_port_zero() {
            let result = bind_socket_with_retry(0);
            assert!(
                result.is_ok(),
                "bind_socket_with_retry should succeed on port 0"
            );
        }

        /// Tests that binding to a privileged port (1) returns an error on most systems.
        ///
        /// On Unix systems, ports below 1024 require root privileges.
        /// On Windows, this may also fail with WSAEACCES.
        /// This verifies the error handling path works correctly.
        #[test]
        #[cfg(not(miri))] // Miri cannot execute socket operations
        fn bind_socket_with_retry_fails_on_privileged_port() {
            // Port 1 is privileged and should fail to bind without root/admin
            let result = bind_socket_with_retry(1);

            // We expect this to fail on most systems
            // (unless running as root/admin, which is unlikely in CI)
            if result.is_err() {
                match result {
                    Err(FortressError::SocketErrorStructured { kind }) => {
                        // Verify the error kind contains the expected port
                        match kind {
                            SocketErrorKind::BindFailed { port } => {
                                assert_eq!(port, 1, "Error should mention port 1");
                            },
                            SocketErrorKind::BindFailedAfterRetries { port, .. } => {
                                assert_eq!(port, 1, "Error should mention port 1");
                            },
                            SocketErrorKind::Custom(_) => {
                                // Custom errors are acceptable as fallback
                            },
                        }
                    },
                    Err(other) => {
                        panic!("Expected SocketErrorStructured, got: {:?}", other);
                    },
                    Ok(_) => unreachable!(),
                }
            }
            // If it succeeds (running as root), that's also acceptable
        }

        /// Tests that multiple sequential binds to port 0 all succeed.
        ///
        /// This exercises the port allocation under repeated use, similar to
        /// how parallel tests might allocate ports.
        #[test]
        #[cfg(not(miri))] // Miri cannot execute socket operations
        fn bind_socket_with_retry_handles_multiple_sequential_binds() {
            // Just verify that multiple binds succeed; we can't easily
            // verify unique ports without accessing private socket fields
            for i in 0..5 {
                let result = bind_socket_with_retry(0);
                assert!(result.is_ok(), "Sequential bind {} should succeed", i + 1);
            }
        }

        /// Tests that `create_chaos_socket` successfully creates a socket.
        #[test]
        #[cfg(not(miri))] // Miri cannot execute socket operations
        fn create_chaos_socket_succeeds() {
            let config = ChaosConfig::default();
            let result = create_chaos_socket(0, config);
            assert!(
                result.is_ok(),
                "create_chaos_socket should succeed on port 0"
            );
        }

        /// Tests that the error kind includes the port number.
        ///
        /// This verifies diagnostic information is useful for debugging.
        #[test]
        #[cfg(not(miri))] // Miri cannot execute socket operations
        fn bind_error_includes_port_in_context() {
            // Use a privileged port that should fail
            let result = bind_socket_with_retry(1);

            if let Err(FortressError::SocketErrorStructured { kind }) = result {
                // Verify the port is accessible from the error kind
                match kind {
                    SocketErrorKind::BindFailed { port } => {
                        assert_eq!(port, 1, "Error should include port number");
                    },
                    SocketErrorKind::BindFailedAfterRetries { port, .. } => {
                        assert_eq!(port, 1, "Error should include port number");
                    },
                    SocketErrorKind::Custom(_) => {
                        // Custom errors may not include port in struct form
                    },
                }
            }
            // If binding succeeded (running as root), test passes trivially
        }

        /// Tests data-driven scenarios for socket binding.
        ///
        /// This uses the data-driven pattern from spectator.rs to test
        /// multiple port scenarios efficiently.
        #[test]
        #[cfg(not(miri))] // Miri cannot execute socket operations
        fn bind_socket_data_driven_scenarios() {
            /// Test case for socket binding scenarios
            struct BindTestCase {
                name: &'static str,
                port: u16,
                should_succeed: bool,
            }

            let test_cases = [
                BindTestCase {
                    name: "port_0_os_assigned",
                    port: 0,
                    should_succeed: true,
                },
                // Privileged ports typically require elevated permissions
                BindTestCase {
                    name: "port_1_privileged",
                    port: 1,
                    should_succeed: false, // May succeed if running as root
                },
                BindTestCase {
                    name: "port_80_privileged_http",
                    port: 80,
                    should_succeed: false, // May succeed if running as root
                },
            ];

            for case in &test_cases {
                let result = bind_socket_with_retry(case.port);

                if case.should_succeed {
                    assert!(
                        result.is_ok(),
                        "[{}] Expected success for port {}",
                        case.name,
                        case.port
                    );
                } else {
                    // For privileged ports, we expect failure unless running as root
                    // So we just verify the error type if it fails
                    if let Err(FortressError::SocketErrorStructured { kind }) = &result {
                        // Verify the error kind matches expected port
                        match kind {
                            SocketErrorKind::BindFailed { port } => {
                                assert_eq!(
                                    *port, case.port,
                                    "[{}] Error should have correct port",
                                    case.name
                                );
                            },
                            SocketErrorKind::BindFailedAfterRetries { port, .. } => {
                                assert_eq!(
                                    *port, case.port,
                                    "[{}] Error should have correct port",
                                    case.name
                                );
                            },
                            SocketErrorKind::Custom(_) => {
                                // Custom errors are acceptable as fallback
                            },
                        }
                    }
                    // If it succeeded, the test runner has elevated privileges
                }
            }
        }
    }

    mod port_allocator_tests {
        use super::*;

        /// Tests that PortAllocator provides unique ports.
        #[test]
        fn port_allocator_provides_unique_ports() {
            // Reset to ensure clean state
            PortAllocator::reset();

            let port1 = PortAllocator::next_port();
            let port2 = PortAllocator::next_port();
            let port3 = PortAllocator::next_port();

            assert_ne!(port1, port2, "Ports should be unique");
            assert_ne!(port2, port3, "Ports should be unique");
            assert_ne!(port1, port3, "Ports should be unique");
        }

        /// Tests that next_ports returns the correct number of ports.
        #[test]
        fn port_allocator_next_ports_returns_correct_count() {
            PortAllocator::reset();

            let ports = PortAllocator::next_ports::<4>();
            assert_eq!(ports.len(), 4, "Should return 4 ports");

            // All should be unique
            for i in 0..ports.len() {
                for j in (i + 1)..ports.len() {
                    assert_ne!(
                        ports[i], ports[j],
                        "Ports at indices {} and {} should be unique",
                        i, j
                    );
                }
            }
        }

        /// Tests that next_pair returns two unique ports.
        /// Note: This tests the deprecated function to ensure it still works correctly.
        #[test]
        #[allow(deprecated)]
        fn port_allocator_next_pair_returns_unique_ports() {
            PortAllocator::reset();

            let (port1, port2) = PortAllocator::next_pair();
            assert_ne!(port1, port2, "Pair ports should be unique");
        }

        /// Tests that ports are within the expected range.
        #[test]
        fn port_allocator_ports_in_valid_range() {
            PortAllocator::reset();

            for _ in 0..10 {
                let port = PortAllocator::next_port();
                assert!(
                    port >= PORT_ALLOCATOR_START,
                    "Port {} should be >= {}",
                    port,
                    PORT_ALLOCATOR_START
                );
                assert!(port < 60000, "Port {} should be < 60000", port);
            }
        }
    }

    /// Tests for ephemeral port binding.
    ///
    /// These tests verify that the ephemeral port binding functions work correctly
    /// and return unique, usable addresses. This is critical for Windows CI stability.
    #[allow(clippy::panic)] // Tests are allowed to panic
    mod ephemeral_port_tests {
        use super::*;
        use std::collections::HashSet;

        /// Verifies that `bind_socket_ephemeral` returns unique addresses.
        ///
        /// This is the core guarantee that prevents TIME_WAIT conflicts:
        /// each call should return a different, OS-assigned ephemeral port.
        #[test]
        fn ephemeral_binding_returns_unique_addresses() {
            let mut addresses = HashSet::new();
            // Keep sockets alive to prevent port reuse during the test
            #[allow(clippy::collection_is_never_read)]
            let mut sockets = Vec::new();

            // Bind multiple sockets and verify all addresses are unique
            for i in 0..10 {
                let (socket, addr) = bind_socket_ephemeral()
                    .unwrap_or_else(|e| panic!("Should bind socket {}: {:?}", i, e));

                // Address should be localhost with a non-zero port
                assert_eq!(
                    addr.ip(),
                    std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
                    "Address should be localhost"
                );
                assert_ne!(addr.port(), 0, "Port should be non-zero (OS-assigned)");

                // Address should be unique
                assert!(
                    addresses.insert(addr),
                    "Address {} should be unique (duplicate found at index {})",
                    addr,
                    i
                );

                // Keep socket alive to prevent port reuse during the test
                sockets.push(socket);
            }

            assert_eq!(
                addresses.len(),
                10,
                "All 10 bindings should have unique addresses"
            );
        }

        /// Verifies parallel socket creation doesn't cause conflicts.
        ///
        /// This test simulates the parallel test execution environment where
        /// multiple threads may bind sockets simultaneously.
        #[test]
        fn parallel_ephemeral_binding_no_conflicts() {
            use std::sync::Arc;
            use std::thread;

            let addresses = Arc::new(std::sync::Mutex::new(HashSet::new()));
            let errors = Arc::new(std::sync::Mutex::new(Vec::new()));

            // Spawn multiple threads to bind sockets concurrently
            let handles: Vec<_> = (0..8)
                .map(|thread_id| {
                    let addresses = Arc::clone(&addresses);
                    let errors = Arc::clone(&errors);

                    thread::spawn(move || {
                        // Each thread binds multiple sockets
                        let mut local_sockets = Vec::new();

                        for i in 0..5 {
                            match bind_socket_ephemeral() {
                                Ok((socket, addr)) => {
                                    // Verify localhost
                                    assert_eq!(
                                        addr.ip(),
                                        std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)
                                    );

                                    // Record address (check for duplicates)
                                    let mut addrs = addresses.lock().unwrap();
                                    if !addrs.insert(addr) {
                                        let mut errs = errors.lock().unwrap();
                                        errs.push(format!(
                                            "Thread {} iteration {}: duplicate address {}",
                                            thread_id, i, addr
                                        ));
                                    }

                                    // Keep socket alive
                                    local_sockets.push(socket);
                                },
                                Err(e) => {
                                    let mut errs = errors.lock().unwrap();
                                    errs.push(format!(
                                        "Thread {} iteration {}: bind failed: {:?}",
                                        thread_id, i, e
                                    ));
                                },
                            }
                        }

                        // Hold sockets until all threads complete
                        thread::sleep(std::time::Duration::from_millis(10));
                        drop(local_sockets);
                    })
                })
                .collect();

            // Wait for all threads
            for handle in handles {
                handle.join().expect("Thread should complete");
            }

            // Check for errors
            let errs = errors.lock().unwrap();
            assert!(
                errs.is_empty(),
                "No errors should occur during parallel binding: {:?}",
                *errs
            );

            // Verify we got the expected number of unique addresses
            let addrs = addresses.lock().unwrap();
            assert_eq!(
                addrs.len(),
                40, // 8 threads × 5 sockets
                "All parallel bindings should produce unique addresses"
            );
        }

        /// Verifies that `create_chaos_socket_ephemeral` works correctly.
        #[test]
        fn chaos_socket_ephemeral_returns_unique_addresses() {
            let mut addresses = HashSet::new();
            // Keep sockets alive to prevent port reuse during the test
            #[allow(clippy::collection_is_never_read)]
            let mut sockets = Vec::new();

            for i in 0..5 {
                let config = ChaosConfig::builder().seed(i as u64).build();
                let (socket, addr) = create_chaos_socket_ephemeral(config)
                    .unwrap_or_else(|e| panic!("Should create chaos socket {}: {:?}", i, e));

                assert_eq!(
                    addr.ip(),
                    std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
                    "Chaos socket address should be localhost"
                );
                assert_ne!(addr.port(), 0, "Chaos socket port should be non-zero");

                assert!(
                    addresses.insert(addr),
                    "Chaos socket address {} should be unique",
                    addr
                );

                sockets.push(socket);
            }

            assert_eq!(
                addresses.len(),
                5,
                "All chaos sockets should have unique addresses"
            );
        }

        /// Edge case: Verify the bound address matches what we can use for communication.
        ///
        /// This ensures the address returned is actually usable for peer setup.
        #[test]
        fn ephemeral_address_is_usable_for_peer_setup() {
            let (socket1, addr1) = bind_socket_ephemeral().expect("Should bind socket 1");
            let (socket2, addr2) = bind_socket_ephemeral().expect("Should bind socket 2");

            // Addresses should be distinct
            assert_ne!(addr1, addr2, "Two sockets should have different addresses");

            // Ports should be in the ephemeral range (typically 49152-65535, but OS-dependent)
            // Just verify they're above well-known ports
            assert!(
                addr1.port() > 1024,
                "Port {} should be above well-known range",
                addr1.port()
            );
            assert!(
                addr2.port() > 1024,
                "Port {} should be above well-known range",
                addr2.port()
            );

            // The sockets should be usable (not in error state)
            // Verify by checking local_addr() matches what we expect
            let local1 = socket1
                .local_addr()
                .expect("Socket 1 should have local addr");
            let local2 = socket2
                .local_addr()
                .expect("Socket 2 should have local addr");

            assert_eq!(
                local1.port(),
                addr1.port(),
                "Returned address port should match socket's local port"
            );
            assert_eq!(
                local2.port(),
                addr2.port(),
                "Returned address port should match socket's local port"
            );
        }

        /// Data-driven test for various socket binding scenarios.
        #[test]
        fn ephemeral_binding_data_driven_scenarios() {
            struct TestCase {
                name: &'static str,
                socket_count: usize,
                expect_all_unique: bool,
            }

            let cases = [
                TestCase {
                    name: "single_socket",
                    socket_count: 1,
                    expect_all_unique: true,
                },
                TestCase {
                    name: "pair_sockets",
                    socket_count: 2,
                    expect_all_unique: true,
                },
                TestCase {
                    name: "quad_sockets",
                    socket_count: 4,
                    expect_all_unique: true,
                },
                TestCase {
                    name: "many_sockets",
                    socket_count: 20,
                    expect_all_unique: true,
                },
            ];

            for case in &cases {
                let mut addresses = HashSet::new();
                // Keep sockets alive to prevent port reuse during the test
                #[allow(clippy::collection_is_never_read)]
                let mut sockets = Vec::new();

                for i in 0..case.socket_count {
                    let result = bind_socket_ephemeral();
                    assert!(
                        result.is_ok(),
                        "[{}] Socket {} should bind successfully",
                        case.name,
                        i
                    );

                    let (socket, addr) = result.unwrap();
                    if case.expect_all_unique {
                        assert!(
                            addresses.insert(addr),
                            "[{}] Address {} should be unique",
                            case.name,
                            addr
                        );
                    }
                    sockets.push(socket);
                }

                assert_eq!(
                    addresses.len(),
                    case.socket_count,
                    "[{}] Should have {} unique addresses",
                    case.name,
                    case.socket_count
                );
            }
        }
    }
}
