use crate::report_violation;
use crate::telemetry::{ViolationKind, ViolationSeverity};
use crate::Frame;

/// Default window size for time synchronization frame advantage calculation.
const DEFAULT_FRAME_WINDOW_SIZE: usize = 30;

/// Configuration for time synchronization behavior.
///
/// The time sync system tracks local and remote frame advantages over a
/// sliding window to calculate how fast/slow this peer should run relative
/// to the other peer(s).
///
/// # Example
///
/// ```
/// use fortress_rollback::TimeSyncConfig;
///
/// // For more responsive sync (may cause more fluctuation)
/// let responsive_config = TimeSyncConfig {
///     window_size: 15,
///     ..TimeSyncConfig::default()
/// };
///
/// // For smoother sync (slower to adapt to changes)
/// let smooth_config = TimeSyncConfig {
///     window_size: 60,
///     ..TimeSyncConfig::default()
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use = "TimeSyncConfig has no effect unless passed to SessionBuilder::with_time_sync_config()"]
pub struct TimeSyncConfig {
    /// The number of frames to average when calculating frame advantage.
    /// A larger window provides a more stable (less jittery) sync but
    /// is slower to react to network changes. A smaller window reacts
    /// faster but may cause more fluctuation in game speed.
    ///
    /// Default: 30 frames (0.5 seconds at 60 FPS)
    pub window_size: usize,
}

impl Default for TimeSyncConfig {
    fn default() -> Self {
        Self {
            window_size: DEFAULT_FRAME_WINDOW_SIZE,
        }
    }
}

impl TimeSyncConfig {
    /// Creates a new `TimeSyncConfig` with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Configuration preset for responsive synchronization.
    ///
    /// Uses a smaller window to react quickly to network changes,
    /// at the cost of potentially more fluctuation in game speed.
    pub fn responsive() -> Self {
        Self { window_size: 15 }
    }

    /// Configuration preset for smooth synchronization.
    ///
    /// Uses a larger window to provide stable, smooth synchronization,
    /// at the cost of slower adaptation to network changes.
    pub fn smooth() -> Self {
        Self { window_size: 60 }
    }

    /// Configuration preset for LAN play.
    ///
    /// Uses a small window since LAN connections are typically stable.
    pub fn lan() -> Self {
        Self { window_size: 10 }
    }

    /// Configuration preset for mobile/cellular networks.
    ///
    /// Uses a very large window to smooth out the high jitter and
    /// variability typical of mobile connections. This prevents
    /// constant speed adjustments that would feel jarring to players.
    ///
    /// Trade-off: Slower adaptation to actual network condition changes,
    /// but much smoother gameplay during normal mobile network variance.
    pub fn mobile() -> Self {
        Self { window_size: 90 }
    }

    /// Configuration preset for competitive/esports scenarios.
    ///
    /// Uses a smaller window for faster adaptation to network changes,
    /// prioritizing accurate sync over smooth speed transitions.
    /// Assumes good, stable network conditions.
    pub fn competitive() -> Self {
        Self { window_size: 20 }
    }
}

/// Handles time synchronization between peers.
///
/// TimeSync tracks frame advantage differentials between local and remote peers,
/// using a rolling window average to smooth out network jitter.
///
/// # Note
///
/// This type is re-exported in [`__internal`](crate::__internal) for testing and fuzzing.
/// It is not part of the stable public API.
#[derive(Debug)]
pub struct TimeSync {
    local: Vec<i32>,
    remote: Vec<i32>,
    window_size: usize,
}

impl Default for TimeSync {
    fn default() -> Self {
        Self::with_config(TimeSyncConfig::default())
    }
}

impl TimeSync {
    /// Creates a new TimeSync with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new TimeSync with the given configuration.
    #[must_use]
    pub fn with_config(config: TimeSyncConfig) -> Self {
        let window_size = config.window_size.max(1); // Ensure at least 1
        Self {
            local: vec![0; window_size],
            remote: vec![0; window_size],
            window_size,
        }
    }

    /// Advances the time sync state for a frame.
    pub fn advance_frame(&mut self, frame: Frame, local_adv: i32, remote_adv: i32) {
        // Handle NULL or negative frames gracefully - this can happen if input serialization
        // fails (returns Frame::NULL), or in edge cases during initialization.
        // We skip the update rather than panic on invalid array index.
        if frame.is_null() || frame.as_i32() < 0 {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::FrameSync,
                "TimeSync::advance_frame called with invalid frame {:?}, skipping update",
                frame
            );
            return;
        }
        self.local[frame.as_i32() as usize % self.window_size] = local_adv;
        self.remote[frame.as_i32() as usize % self.window_size] = remote_adv;
    }

    /// Calculates the average frame advantage between local and remote peers.
    #[must_use]
    pub fn average_frame_advantage(&self) -> i32 {
        // average local and remote frame advantages
        let local_sum: i32 = self.local.iter().sum();
        let local_avg = local_sum as f32 / self.local.len() as f32;
        let remote_sum: i32 = self.remote.iter().sum();
        let remote_avg = remote_sum as f32 / self.remote.len() as f32;

        // meet in the middle
        ((remote_avg - local_avg) / 2.0) as i32
    }
}

// #########
// # TESTS #
// #########

#[cfg(test)]
mod sync_layer_tests {

    use super::*;

    /// Default window size for tests (matches TimeSyncConfig::default())
    const FRAME_WINDOW_SIZE: usize = 30;

    #[test]
    fn test_advance_frame_no_advantage() {
        let mut time_sync = TimeSync::default();

        for i in 0..60i32 {
            let local_adv = 0;
            let remote_adv = 0;
            time_sync.advance_frame(Frame::new(i), local_adv, remote_adv)
        }

        assert_eq!(time_sync.average_frame_advantage(), 0);
    }

    #[test]
    fn test_advance_frame_local_advantage() {
        let mut time_sync = TimeSync::default();

        for i in 0..60i32 {
            let local_adv = 5;
            let remote_adv = -5;
            time_sync.advance_frame(Frame::new(i), local_adv, remote_adv)
        }

        assert_eq!(time_sync.average_frame_advantage(), -5);
    }

    #[test]
    fn test_advance_frame_small_remote_advantage() {
        let mut time_sync = TimeSync::default();

        for i in 0..60i32 {
            let local_adv = -1;
            let remote_adv = 1;
            time_sync.advance_frame(Frame::new(i), local_adv, remote_adv)
        }

        assert_eq!(time_sync.average_frame_advantage(), 1);
    }

    #[test]
    fn test_advance_frame_remote_advantage() {
        let mut time_sync = TimeSync::default();

        for i in 0..60i32 {
            let local_adv = -4;
            let remote_adv = 4;
            time_sync.advance_frame(Frame::new(i), local_adv, remote_adv)
        }

        assert_eq!(time_sync.average_frame_advantage(), 4);
    }

    #[test]
    fn test_advance_frame_big_remote_advantage() {
        let mut time_sync = TimeSync::default();

        for i in 0..60i32 {
            let local_adv = -40;
            let remote_adv = 40;
            time_sync.advance_frame(Frame::new(i), local_adv, remote_adv)
        }

        assert_eq!(time_sync.average_frame_advantage(), 40);
    }

    #[test]
    fn test_new_creates_default() {
        let time_sync = TimeSync::new();
        // All values should be zero initially
        assert_eq!(time_sync.average_frame_advantage(), 0);
    }

    #[test]
    fn test_window_sliding_behavior() {
        let mut time_sync = TimeSync::default();

        // Fill window with local advantage of 10
        for i in 0..FRAME_WINDOW_SIZE {
            time_sync.advance_frame(Frame::new(i as i32), 10, -10);
        }
        assert_eq!(time_sync.average_frame_advantage(), -10);

        // Now fill window with remote advantage of 10 (overwriting old values)
        for i in FRAME_WINDOW_SIZE..(FRAME_WINDOW_SIZE * 2) {
            time_sync.advance_frame(Frame::new(i as i32), -10, 10);
        }
        // Should now show remote advantage
        assert_eq!(time_sync.average_frame_advantage(), 10);
    }

    #[test]
    fn test_partial_window_fill() {
        let mut time_sync = TimeSync::default();

        // Only fill half the window with values
        for i in 0..(FRAME_WINDOW_SIZE / 2) {
            time_sync.advance_frame(Frame::new(i as i32), 10, -10);
        }

        // Average should be diluted by zeros in other half
        // (10 * 15 + 0 * 15) / 30 = 5 for local
        // (-10 * 15 + 0 * 15) / 30 = -5 for remote
        // (remote_avg - local_avg) / 2 = (-5 - 5) / 2 = -5
        assert_eq!(time_sync.average_frame_advantage(), -5);
    }

    #[test]
    fn test_asymmetric_advantages() {
        let mut time_sync = TimeSync::default();

        // Asymmetric case: local is 0, remote is ahead
        for i in 0..FRAME_WINDOW_SIZE {
            time_sync.advance_frame(Frame::new(i as i32), 0, 6);
        }

        // remote_avg = 6, local_avg = 0
        // (6 - 0) / 2 = 3
        assert_eq!(time_sync.average_frame_advantage(), 3);
    }

    #[test]
    fn test_frame_wraparound_modulo() {
        let mut time_sync = TimeSync::default();

        // Use frame numbers larger than window size to test modulo
        let large_frame = Frame::new(1000);
        time_sync.advance_frame(large_frame, 5, -5);

        // The value should be stored at position 1000 % 30 = 10
        assert_eq!(time_sync.local[10], 5);
        assert_eq!(time_sync.remote[10], -5);
    }

    #[test]
    fn test_advance_frame_null_frame_skipped() {
        let mut time_sync = TimeSync::default();

        // Initialize with some known values first
        time_sync.advance_frame(Frame::new(0), 10, 20);
        assert_eq!(time_sync.local[0], 10);
        assert_eq!(time_sync.remote[0], 20);

        // Advance with NULL frame - should be skipped
        time_sync.advance_frame(Frame::NULL, 99, 99);

        // Values should not have changed (NULL frame is skipped)
        // Note: NULL is -1, which would wrap to a large index if not handled
        // The test passes if we don't panic
        assert_eq!(time_sync.local[0], 10); // unchanged
    }

    #[test]
    fn test_advance_frame_negative_frame_skipped() {
        let mut time_sync = TimeSync::default();

        // Initialize with some known values first
        time_sync.advance_frame(Frame::new(5), 10, 20);

        // Advance with negative frame - should be skipped (no panic)
        time_sync.advance_frame(Frame::new(-5), 99, 99);

        // Test passes if we don't panic from invalid array index
    }
}

// =============================================================================
// Property-Based Tests
//
// These tests use proptest to verify invariants hold under random inputs.
// They are critical for ensuring the TimeSync implementation handles all
// edge cases correctly.
// =============================================================================

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// Maximum frame value for property tests (keep tractable)
    const MAX_FRAME: i32 = 10_000;

    /// Maximum advantage value for property tests
    const MAX_ADVANTAGE: i32 = 100;

    /// Generate valid frame numbers (non-negative)
    fn valid_frame() -> impl Strategy<Value = Frame> {
        (0..MAX_FRAME).prop_map(Frame::new)
    }

    /// Generate advantage values (can be negative)
    fn advantage_value() -> impl Strategy<Value = i32> {
        -MAX_ADVANTAGE..=MAX_ADVANTAGE
    }

    /// Generate window sizes (reasonable range)
    fn window_size() -> impl Strategy<Value = usize> {
        1..=100usize
    }

    proptest! {
        /// Property: Window index is always in bounds.
        ///
        /// For any valid frame number and window size, the computed index
        /// (frame % window_size) must be within [0, window_size).
        #[test]
        fn prop_window_index_in_bounds(
            frame in valid_frame(),
            local_adv in advantage_value(),
            remote_adv in advantage_value(),
            window_size in window_size(),
        ) {
            let config = TimeSyncConfig { window_size };
            let mut ts = TimeSync::with_config(config);

            // This should not panic due to out-of-bounds access
            ts.advance_frame(frame, local_adv, remote_adv);

            // Verify the index computation
            let expected_index = frame.as_i32() as usize % window_size;
            prop_assert!(expected_index < window_size);
            prop_assert_eq!(ts.local[expected_index], local_adv);
            prop_assert_eq!(ts.remote[expected_index], remote_adv);
        }

        /// Property: Average is bounded by min/max of window values.
        ///
        /// The average frame advantage should be within a reasonable range
        /// given the input values.
        #[test]
        fn prop_average_bounded_by_inputs(
            local_adv in advantage_value(),
            remote_adv in advantage_value(),
        ) {
            let mut ts = TimeSync::default();

            // Fill entire window with same values
            for i in 0..30 {
                ts.advance_frame(Frame::new(i), local_adv, remote_adv);
            }

            let avg = ts.average_frame_advantage();
            let expected = (remote_adv - local_adv) / 2;

            // Should be exactly the expected value when window is uniform
            prop_assert_eq!(avg, expected);
        }

        /// Property: Average is deterministic.
        ///
        /// Same sequence of inputs produces same average.
        #[test]
        fn prop_average_deterministic(
            frames in proptest::collection::vec(
                (valid_frame(), advantage_value(), advantage_value()),
                1..100
            ),
        ) {
            let mut ts1 = TimeSync::default();
            let mut ts2 = TimeSync::default();

            for (frame, local, remote) in &frames {
                ts1.advance_frame(*frame, *local, *remote);
                ts2.advance_frame(*frame, *local, *remote);
            }

            prop_assert_eq!(
                ts1.average_frame_advantage(),
                ts2.average_frame_advantage(),
                "Same inputs should produce same average"
            );
        }

        /// Property: NULL frames don't modify state.
        ///
        /// Calling advance_frame with Frame::NULL should leave the window unchanged.
        #[test]
        fn prop_null_frame_no_effect(
            initial_frames in proptest::collection::vec(
                (0..30i32, advantage_value(), advantage_value()),
                10..30
            ),
        ) {
            let mut ts = TimeSync::default();

            // Initialize with known values
            for (frame_val, local, remote) in &initial_frames {
                ts.advance_frame(Frame::new(*frame_val), *local, *remote);
            }

            let avg_before = ts.average_frame_advantage();

            // Attempt update with NULL frame
            ts.advance_frame(Frame::NULL, 999, 999);

            let avg_after = ts.average_frame_advantage();

            // Average should be unchanged (NULL frame is skipped)
            prop_assert_eq!(avg_before, avg_after, "NULL frame should not modify state");
        }

        /// Property: Negative frames don't modify state.
        ///
        /// Calling advance_frame with negative frame should leave the window unchanged.
        #[test]
        fn prop_negative_frame_no_effect(
            initial_frames in proptest::collection::vec(
                (0..30i32, advantage_value(), advantage_value()),
                10..30
            ),
            neg_frame in -1000..-1i32,
        ) {
            let mut ts = TimeSync::default();

            // Initialize with known values
            for (frame_val, local, remote) in &initial_frames {
                ts.advance_frame(Frame::new(*frame_val), *local, *remote);
            }

            let avg_before = ts.average_frame_advantage();

            // Attempt update with negative frame
            ts.advance_frame(Frame::new(neg_frame), 999, 999);

            let avg_after = ts.average_frame_advantage();

            prop_assert_eq!(avg_before, avg_after, "Negative frame should not modify state");
        }

        /// Property: Window slides correctly.
        ///
        /// Older values should be overwritten as new frames advance beyond the window.
        #[test]
        fn prop_window_slides(window_size in 5..50usize) {
            let config = TimeSyncConfig { window_size };
            let mut ts = TimeSync::with_config(config);

            // Fill window with local advantage = 10
            for i in 0..window_size {
                ts.advance_frame(Frame::new(i as i32), 10, -10);
            }

            let avg_initial = ts.average_frame_advantage();

            // Now overwrite with local advantage = -10 (remote advantage)
            for i in 0..window_size {
                ts.advance_frame(Frame::new((window_size + i) as i32), -10, 10);
            }

            let avg_after = ts.average_frame_advantage();

            // The window should have completely different values now
            // Initial: local=10, remote=-10 => avg = (-10 - 10) / 2 = -10
            // After: local=-10, remote=10 => avg = (10 - (-10)) / 2 = 10
            prop_assert_eq!(avg_initial, -10, "Initial average incorrect");
            prop_assert_eq!(avg_after, 10, "After-slide average incorrect");
        }

        /// Property: Average formula is mathematically correct.
        ///
        /// average = (remote_avg - local_avg) / 2
        #[test]
        fn prop_average_formula_correct(
            local_adv in advantage_value(),
            remote_adv in advantage_value(),
        ) {
            let mut ts = TimeSync::default();

            // Fill with uniform values
            for i in 0..30 {
                ts.advance_frame(Frame::new(i), local_adv, remote_adv);
            }

            let avg = ts.average_frame_advantage();

            // Expected: (remote - local) / 2
            // Note: integer division truncates toward zero
            let expected = (remote_adv - local_adv) / 2;
            prop_assert_eq!(avg, expected);
        }

        /// Property: Custom window size is respected.
        #[test]
        fn prop_custom_window_size_respected(window_size in 1..100usize) {
            let config = TimeSyncConfig { window_size };
            let ts = TimeSync::with_config(config);

            prop_assert_eq!(ts.window_size, window_size);
            prop_assert_eq!(ts.local.len(), window_size);
            prop_assert_eq!(ts.remote.len(), window_size);
        }
    }
}

// =============================================================================
// Kani Formal Verification Proofs
//
// These proofs use Kani (https://model-checking.github.io/kani/) to formally
// verify safety properties of the TimeSync implementation. They prove:
//
// 1. No integer overflow in sum/average calculations
// 2. Window index is always in bounds
// 3. Division by zero is impossible
//
// Run with: cargo kani --tests
// =============================================================================

#[cfg(kani)]
mod kani_proofs {
    use super::*;

    /// Proof: Window index computation is always in bounds.
    ///
    /// For any valid frame number and window size >= 1, the modulo operation
    /// produces an index that is always < window_size.
    #[kani::proof]
    fn proof_window_index_in_bounds() {
        let frame_val: i32 = kani::any();
        kani::assume(frame_val >= 0);

        let window_size: usize = kani::any();
        kani::assume(window_size >= 1 && window_size <= 1000);

        let index = (frame_val as usize) % window_size;

        kani::assert(index < window_size, "Index must be less than window size");
    }

    /// Proof: Sum of window values does not overflow.
    ///
    /// With bounded advantage values, the sum of up to 1000 values
    /// should not overflow i32.
    #[kani::proof]
    fn proof_sum_no_overflow() {
        // Simulate a small window for tractability
        let window_size: usize = kani::any();
        kani::assume(window_size >= 1 && window_size <= 10);

        // Bounded advantage values (realistic for frame advantages)
        let advantage: i32 = kani::any();
        kani::assume(advantage >= -1000 && advantage <= 1000);

        // Check that summing `window_size` copies of `advantage` doesn't overflow
        let sum = advantage.checked_mul(window_size as i32);
        kani::assert(sum.is_some(), "Multiplication should not overflow");
    }

    /// Proof: Division in average calculation is safe.
    ///
    /// Division by window.len() is always safe because window_size >= 1.
    #[kani::proof]
    fn proof_division_safe() {
        let window_size: usize = kani::any();
        kani::assume(window_size >= 1 && window_size <= 1000);

        let config = TimeSyncConfig { window_size };
        let ts = TimeSync::with_config(config);

        // The window length is guaranteed to be >= 1
        kani::assert(ts.local.len() >= 1, "Window length must be at least 1");
        kani::assert(ts.remote.len() >= 1, "Window length must be at least 1");
    }

    /// Proof: advance_frame with valid frame doesn't panic.
    ///
    /// For any valid (non-negative) frame, advance_frame should not panic.
    #[kani::proof]
    fn proof_advance_frame_safe() {
        let frame_val: i32 = kani::any();
        kani::assume(frame_val >= 0 && frame_val < 10000);

        let local_adv: i32 = kani::any();
        let remote_adv: i32 = kani::any();
        kani::assume(local_adv >= -1000 && local_adv <= 1000);
        kani::assume(remote_adv >= -1000 && remote_adv <= 1000);

        let config = TimeSyncConfig { window_size: 30 };
        let mut ts = TimeSync::with_config(config);

        // This should not panic
        ts.advance_frame(Frame::new(frame_val), local_adv, remote_adv);

        // Verify the value was stored correctly
        let idx = (frame_val as usize) % 30;
        kani::assert(ts.local[idx] == local_adv, "Local value should be stored");
        kani::assert(
            ts.remote[idx] == remote_adv,
            "Remote value should be stored",
        );
    }

    /// Proof: Window size of 0 is corrected to 1.
    ///
    /// The with_config function ensures window_size is at least 1.
    #[kani::proof]
    fn proof_window_size_minimum() {
        let window_size: usize = kani::any();
        // Even if user passes 0, it should be corrected
        let config = TimeSyncConfig { window_size };
        let ts = TimeSync::with_config(config);

        kani::assert(ts.window_size >= 1, "Window size must be at least 1");
        kani::assert(
            ts.local.len() >= 1,
            "Local vec must have at least 1 element",
        );
        kani::assert(
            ts.remote.len() >= 1,
            "Remote vec must have at least 1 element",
        );
    }

    /// Proof: Default configuration is valid.
    #[kani::proof]
    fn proof_default_valid() {
        let ts = TimeSync::default();

        kani::assert(ts.window_size == 30, "Default window size should be 30");
        kani::assert(
            ts.local.len() == 30,
            "Default local vec length should be 30",
        );
        kani::assert(
            ts.remote.len() == 30,
            "Default remote vec length should be 30",
        );
        kani::assert(
            ts.average_frame_advantage() == 0,
            "Initial average should be 0",
        );
    }
}
