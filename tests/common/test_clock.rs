//! A manually-advanceable clock for deterministic testing.
//!
//! [`TestClock`] replaces `thread::sleep()` in tests by providing a virtual time source
//! that only advances when explicitly told to. This makes test execution fully deterministic
//! regardless of system load, platform, or CI environment.
//!
//! # How It Works
//!
//! 1. Create a `TestClock` at the start of your test
//! 2. Pass `clock.as_protocol_clock()` to `ProtocolConfig::clock`
//! 3. Pass `clock.as_chaos_clock()` to `ChaosSocket::with_clock()`
//! 4. Replace `thread::sleep(duration)` with `clock.advance(duration)`
//!
//! # Example
//!
//! ```ignore
//! use common::test_clock::TestClock;
//! use fortress_rollback::ProtocolConfig;
//! use std::time::Duration;
//!
//! let clock = TestClock::new();
//!
//! // Configure protocol to use virtual time
//! let protocol_config = ProtocolConfig {
//!     clock: Some(clock.as_protocol_clock()),
//!     ..ProtocolConfig::default()
//! };
//!
//! // Instead of thread::sleep(Duration::from_millis(100)):
//! clock.advance(Duration::from_millis(100));
//! ```

use std::sync::{Arc, Mutex};
use std::time::Duration;
use web_time::Instant;

/// A manually-advanceable clock for deterministic testing.
///
/// All protocol and socket operations that would normally use `Instant::now()`
/// instead read from this clock. Time only advances when [`advance()`](TestClock::advance)
/// is called, making test execution fully deterministic regardless of system load or platform.
///
/// # Thread Safety
///
/// `TestClock` uses `Arc<Mutex<Instant>>` internally, making it safe to share across
/// threads and clone cheaply. All clock functions produced by [`as_protocol_clock()`](TestClock::as_protocol_clock)
/// and [`as_chaos_clock()`](TestClock::as_chaos_clock) share the same underlying time.
pub struct TestClock {
    current: Arc<Mutex<Instant>>,
}

// TestClock is test-only infrastructure. A poisoned mutex indicates a test bug,
// not a recoverable condition, so .expect() is appropriate here.
#[allow(clippy::expect_used)]
impl TestClock {
    /// Creates a new `TestClock` starting at the current wall-clock time.
    ///
    /// The initial time is captured from `Instant::now()` to ensure that
    /// duration calculations (which rely on the monotonic clock epoch) work
    /// correctly.
    pub fn new() -> Self {
        Self {
            current: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// Advances the clock by the given duration.
    ///
    /// This is the key mechanism for deterministic testing: instead of
    /// `thread::sleep(duration)`, tests call `clock.advance(duration)` to
    /// make protocol timers fire without any real waiting.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned (another thread panicked
    /// while holding the lock). This is acceptable in test code.
    pub fn advance(&self, duration: Duration) {
        let mut t = self.current.lock().expect("TestClock mutex poisoned");
        *t += duration;
    }

    /// Returns the current virtual time.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    pub fn now(&self) -> Instant {
        *self.current.lock().expect("TestClock mutex poisoned")
    }

    /// Creates a [`ClockFn`](fortress_rollback::ClockFn) for use with
    /// [`ProtocolConfig::clock`](fortress_rollback::ProtocolConfig::clock).
    ///
    /// The returned closure captures a reference to this clock's shared state,
    /// so advancing the clock affects all protocols using this clock function.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use fortress_rollback::ProtocolConfig;
    ///
    /// let clock = TestClock::new();
    /// let config = ProtocolConfig {
    ///     clock: Some(clock.as_protocol_clock()),
    ///     ..ProtocolConfig::default()
    /// };
    /// ```
    pub fn as_protocol_clock(&self) -> Arc<dyn Fn() -> Instant + Send + Sync> {
        let current = Arc::clone(&self.current);
        Arc::new(move || *current.lock().expect("TestClock mutex poisoned"))
    }

    /// Creates a clock function for [`ChaosSocket::with_clock()`](fortress_rollback::ChaosSocket::with_clock).
    ///
    /// On non-WASM targets, `web_time::Instant` is the same type as
    /// `std::time::Instant`, so this returns the same underlying clock
    /// as [`as_protocol_clock()`](TestClock::as_protocol_clock).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let clock = TestClock::new();
    /// let chaos_socket = ChaosSocket::new(inner, config)
    ///     .with_clock(clock.as_chaos_clock());
    /// ```
    pub fn as_chaos_clock(&self) -> Arc<dyn Fn() -> std::time::Instant + Send + Sync> {
        let current = Arc::clone(&self.current);
        Arc::new(move || *current.lock().expect("TestClock mutex poisoned"))
    }
}

impl Default for TestClock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn new_clock_returns_reasonable_time() {
        let before = Instant::now();
        let clock = TestClock::new();
        let after = Instant::now();

        let clock_time = clock.now();
        assert!(
            clock_time >= before,
            "Clock should be >= time before creation"
        );
        assert!(
            clock_time <= after,
            "Clock should be <= time after creation"
        );
    }

    #[test]
    fn advance_increases_time() {
        let clock = TestClock::new();
        let t0 = clock.now();

        clock.advance(Duration::from_millis(100));
        let t1 = clock.now();

        assert_eq!(t1 - t0, Duration::from_millis(100));
    }

    #[test]
    fn advance_is_cumulative() {
        let clock = TestClock::new();
        let t0 = clock.now();

        clock.advance(Duration::from_millis(50));
        clock.advance(Duration::from_millis(75));
        clock.advance(Duration::from_millis(25));
        let t1 = clock.now();

        assert_eq!(t1 - t0, Duration::from_millis(150));
    }

    #[test]
    fn advance_zero_does_not_change_time() {
        let clock = TestClock::new();
        let t0 = clock.now();

        clock.advance(Duration::ZERO);
        let t1 = clock.now();

        assert_eq!(t0, t1);
    }

    #[test]
    fn time_does_not_advance_without_explicit_call() {
        let clock = TestClock::new();
        let t0 = clock.now();

        // Do nothing — time should not change
        let t1 = clock.now();

        assert_eq!(t0, t1);
    }

    #[test]
    fn protocol_clock_shares_state() {
        let clock = TestClock::new();
        let protocol_clock = clock.as_protocol_clock();

        let t0 = protocol_clock();
        clock.advance(Duration::from_secs(1));
        let t1 = protocol_clock();

        assert_eq!(t1 - t0, Duration::from_secs(1));
    }

    #[test]
    fn chaos_clock_shares_state() {
        let clock = TestClock::new();
        let chaos_clock = clock.as_chaos_clock();

        let t0 = chaos_clock();
        clock.advance(Duration::from_millis(500));
        let t1 = chaos_clock();

        assert_eq!(t1 - t0, Duration::from_millis(500));
    }

    #[test]
    fn protocol_and_chaos_clocks_agree() {
        let clock = TestClock::new();
        let protocol_clock = clock.as_protocol_clock();
        let chaos_clock = clock.as_chaos_clock();

        // On non-WASM, web_time::Instant == std::time::Instant,
        // so both clocks should return the same value.
        let t_protocol = protocol_clock();
        let t_chaos = chaos_clock();
        assert_eq!(t_protocol, t_chaos);

        clock.advance(Duration::from_millis(200));
        let t_protocol2 = protocol_clock();
        let t_chaos2 = chaos_clock();
        assert_eq!(t_protocol2, t_chaos2);
    }

    #[test]
    fn multiple_protocol_clocks_share_same_time() {
        let clock = TestClock::new();
        let clock_a = clock.as_protocol_clock();
        let clock_b = clock.as_protocol_clock();

        clock.advance(Duration::from_millis(300));

        assert_eq!(clock_a(), clock_b());
    }

    #[test]
    fn default_creates_valid_clock() {
        let clock = TestClock::default();
        let t0 = clock.now();
        clock.advance(Duration::from_millis(10));
        let t1 = clock.now();
        assert_eq!(t1 - t0, Duration::from_millis(10));
    }

    #[test]
    fn large_advance_works() {
        let clock = TestClock::new();
        let t0 = clock.now();

        // Advance by 1 hour
        clock.advance(Duration::from_secs(3600));
        let t1 = clock.now();

        assert_eq!(t1 - t0, Duration::from_secs(3600));
    }
}
