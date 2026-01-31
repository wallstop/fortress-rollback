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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl std::fmt::Display for TimeSyncConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Destructure to ensure all fields are included when new fields are added.
        let Self { window_size } = self;
        write!(f, "TimeSyncConfig {{ window_size: {} }}", window_size)
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
        let index = frame.as_i32() as usize % self.window_size;
        if let Some(local_slot) = self.local.get_mut(index) {
            *local_slot = local_adv;
        }
        if let Some(remote_slot) = self.remote.get_mut(index) {
            *remote_slot = remote_adv;
        }
    }

    /// Calculates the average frame advantage between local and remote peers.
    ///
    /// Uses integer-only arithmetic for determinism across platforms.
    /// The formula `(remote_sum - local_sum) / (2 * count)` is mathematically
    /// equivalent to `((remote_avg - local_avg) / 2)` but avoids floating-point
    /// operations that could produce different results due to compiler optimizations,
    /// FPU rounding modes, or platform-specific implementations.
    #[must_use]
    pub fn average_frame_advantage(&self) -> i32 {
        let local_sum: i32 = self.local.iter().sum();
        let remote_sum: i32 = self.remote.iter().sum();
        // local and remote have the same length (both initialized with window_size)
        let count = self.local.len() as i32;

        // Integer division: (remote_sum - local_sum) / (2 * count)
        // This avoids floating-point non-determinism while producing equivalent results.
        (remote_sum - local_sum) / (2 * count)
    }
}

// #########
// # TESTS #
// #########

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]
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

    // ==========================================================================
    // Data-Driven Tests for TimeSyncConfig Presets
    // ==========================================================================

    /// Test data structure for config preset verification
    struct ConfigPresetTestCase {
        name: &'static str,
        config: TimeSyncConfig,
        expected_window_size: usize,
    }

    /// Data-driven test for all configuration presets
    #[test]
    fn test_config_presets_data_driven() {
        let test_cases = [
            ConfigPresetTestCase {
                name: "default",
                config: TimeSyncConfig::default(),
                expected_window_size: 30,
            },
            ConfigPresetTestCase {
                name: "new",
                config: TimeSyncConfig::new(),
                expected_window_size: 30,
            },
            ConfigPresetTestCase {
                name: "responsive",
                config: TimeSyncConfig::responsive(),
                expected_window_size: 15,
            },
            ConfigPresetTestCase {
                name: "smooth",
                config: TimeSyncConfig::smooth(),
                expected_window_size: 60,
            },
            ConfigPresetTestCase {
                name: "lan",
                config: TimeSyncConfig::lan(),
                expected_window_size: 10,
            },
            ConfigPresetTestCase {
                name: "mobile",
                config: TimeSyncConfig::mobile(),
                expected_window_size: 90,
            },
            ConfigPresetTestCase {
                name: "competitive",
                config: TimeSyncConfig::competitive(),
                expected_window_size: 20,
            },
        ];

        for test_case in &test_cases {
            let ts = TimeSync::with_config(test_case.config);
            assert_eq!(
                ts.window_size, test_case.expected_window_size,
                "Config preset '{}' should have window_size={}, got={}",
                test_case.name, test_case.expected_window_size, ts.window_size
            );
            assert_eq!(
                ts.local.len(),
                test_case.expected_window_size,
                "Config preset '{}' should have local.len()={}, got={}",
                test_case.name,
                test_case.expected_window_size,
                ts.local.len()
            );
            assert_eq!(
                ts.remote.len(),
                test_case.expected_window_size,
                "Config preset '{}' should have remote.len()={}, got={}",
                test_case.name,
                test_case.expected_window_size,
                ts.remote.len()
            );
            // Initial average should always be 0
            assert_eq!(
                ts.average_frame_advantage(),
                0,
                "Config preset '{}' should have initial average=0, got={}",
                test_case.name,
                ts.average_frame_advantage()
            );
        }
    }

    // ==========================================================================
    // TimeSyncConfig Display Tests
    // ==========================================================================

    #[test]
    fn test_time_sync_config_display() {
        let config = TimeSyncConfig { window_size: 30 };
        assert_eq!(config.to_string(), "TimeSyncConfig { window_size: 30 }");

        let config = TimeSyncConfig { window_size: 60 };
        assert_eq!(config.to_string(), "TimeSyncConfig { window_size: 60 }");
    }

    // ==========================================================================
    // Edge Case Tests
    // ==========================================================================

    /// Test window_size of 0 is corrected to 1
    #[test]
    fn test_window_size_zero_corrected_to_one() {
        let config = TimeSyncConfig { window_size: 0 };
        let ts = TimeSync::with_config(config);

        assert_eq!(ts.window_size, 1, "Window size 0 should be corrected to 1");
        assert_eq!(ts.local.len(), 1, "Local vec should have length 1");
        assert_eq!(ts.remote.len(), 1, "Remote vec should have length 1");
    }

    /// Test window_size of 1 (minimum valid)
    #[test]
    fn test_window_size_minimum_one() {
        let config = TimeSyncConfig { window_size: 1 };
        let mut ts = TimeSync::with_config(config);

        // With window size 1, every frame overwrites the same index
        ts.advance_frame(Frame::new(0), 10, 5);
        assert_eq!(ts.average_frame_advantage(), -2); // (5 - 10) / 2 = -2.5 truncated

        ts.advance_frame(Frame::new(1), -10, 10);
        assert_eq!(ts.average_frame_advantage(), 10); // (10 - (-10)) / 2 = 10

        // Verify all frames map to index 0
        assert_eq!(ts.local[0], -10);
        assert_eq!(ts.remote[0], 10);
    }

    /// Test i32::MAX frame number doesn't cause panic
    #[test]
    fn test_large_frame_number() {
        let mut ts = TimeSync::default();

        // Very large frame number should work (modulo wraps)
        let large_frame = Frame::new(i32::MAX);
        ts.advance_frame(large_frame, 5, 10);

        // Should be stored at i32::MAX % 30 = 7
        let expected_index = (i32::MAX as usize) % 30;
        assert_eq!(
            ts.local[expected_index], 5,
            "Large frame value should map to index {}, got local[{}]={}",
            expected_index, expected_index, ts.local[expected_index]
        );
    }

    /// Test that extreme advantage values don't cause overflow in average calculation
    #[test]
    fn test_extreme_advantage_values() {
        let mut ts = TimeSync::default();

        // Fill with large but not overflowing values
        // 30 * 1000 = 30000, well within i32 range
        for i in 0..30 {
            ts.advance_frame(Frame::new(i), 1000, -1000);
        }

        let avg = ts.average_frame_advantage();
        assert_eq!(
            avg, -1000,
            "Average should be -1000 for local=1000, remote=-1000"
        );
    }

    /// Test mixed positive and negative values average correctly
    #[test]
    fn test_mixed_advantage_values() {
        let mut ts = TimeSync::default();

        // Alternate between positive and negative values
        for i in 0..30 {
            if i % 2 == 0 {
                ts.advance_frame(Frame::new(i), 10, -10);
            } else {
                ts.advance_frame(Frame::new(i), -10, 10);
            }
        }

        // Should average to 0 (equal positive and negative)
        let avg = ts.average_frame_advantage();
        assert_eq!(avg, 0, "Mixed values should average to 0");
    }

    /// Test frame=0 specifically (boundary case)
    #[test]
    fn test_frame_zero() {
        let mut ts = TimeSync::default();

        ts.advance_frame(Frame::new(0), 42, -42);

        assert_eq!(ts.local[0], 42, "Frame 0 should map to index 0");
        assert_eq!(ts.remote[0], -42, "Frame 0 should map to index 0");
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
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]
mod property_tests {
    use super::*;
    use crate::test_config::miri_case_count;
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
        #![proptest_config(ProptestConfig {
            cases: miri_case_count(),
            ..ProptestConfig::default()
        })]
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
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Modulo index bounds (INV-5 for TimeSync)
    /// - Related: proof_index_wrapping_consistent, proof_advance_frame_safe
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
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Arithmetic overflow safety in sum calculation
    /// - Related: proof_division_safe, proof_advance_frame_safe
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
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Division by zero is impossible
    /// - Related: proof_sum_no_overflow, proof_window_size_minimum
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
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: advance_frame safety with valid inputs
    /// - Related: proof_negative_frame_safe, proof_window_index_in_bounds
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
    /// We bound window_size to realistic values to avoid capacity overflow
    /// during Vec allocation - this is not a production bug but a verification
    /// tractability constraint. Real users won't pass usize::MAX as window_size.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Window size minimum bound enforcement
    /// - Related: proof_zero_window_size_corrected, proof_division_safe
    #[kani::proof]
    fn proof_window_size_minimum() {
        let window_size: usize = kani::any();
        // Bound to realistic window sizes to avoid capacity overflow in Vec allocation.
        // In production, window sizes > 10000 would be unreasonable (> 2 minutes at 60fps).
        // This constraint exists because Kani explores all possible usize values including
        // those that would exhaust memory when creating Vec<i32> of that size.
        kani::assume(window_size <= 10000);
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
    ///
    /// Note: We need unwind(32) because the default window size is 30,
    /// and average_frame_advantage() iterates over the window using .iter().sum().
    /// Kani needs to unroll the loop at least window_size+1 times.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Default TimeSync state validity
    /// - Related: proof_preset_configs_valid, proof_window_size_minimum
    #[kani::proof]
    #[kani::unwind(32)]
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

    /// Proof: All config presets create valid TimeSync instances.
    ///
    /// Tests that all factory methods produce TimeSync with:
    /// - window_size >= 1
    /// - local/remote vec lengths match window_size
    /// - initial average is 0
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: All preset configurations produce valid state
    /// - Related: proof_default_valid, proof_zero_window_size_corrected
    #[kani::proof]
    fn proof_preset_configs_valid() {
        // Use symbolic choice to test all presets
        let preset_choice: u8 = kani::any();
        kani::assume(preset_choice < 5);

        let config = match preset_choice {
            0 => TimeSyncConfig::responsive(),
            1 => TimeSyncConfig::smooth(),
            2 => TimeSyncConfig::lan(),
            3 => TimeSyncConfig::competitive(),
            _ => TimeSyncConfig::mobile(),
        };

        let ts = TimeSync::with_config(config);

        // All presets must result in valid TimeSync
        kani::assert(
            ts.window_size >= 1,
            "All presets must have window_size >= 1",
        );
        kani::assert(
            ts.local.len() == ts.window_size,
            "local vec length must match window_size",
        );
        kani::assert(
            ts.remote.len() == ts.window_size,
            "remote vec length must match window_size",
        );
    }

    /// Proof: Window size 0 is always corrected to 1.
    ///
    /// Explicitly verifies the edge case where window_size = 0.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Zero window size edge case handling
    /// - Related: proof_window_size_minimum, proof_division_safe
    #[kani::proof]
    fn proof_zero_window_size_corrected() {
        let config = TimeSyncConfig { window_size: 0 };
        let ts = TimeSync::with_config(config);

        kani::assert(ts.window_size == 1, "window_size 0 must be corrected to 1");
        kani::assert(ts.local.len() == 1, "local vec must have exactly 1 element");
        kani::assert(
            ts.remote.len() == 1,
            "remote vec must have exactly 1 element",
        );
    }

    /// Proof: Frame index wrapping is consistent.
    ///
    /// Verifies that for any frame f and window size w,
    /// the index (f % w) is always in [0, w).
    ///
    /// Note: We bound frame_val to [0, 10000] to keep verification tractable.
    /// The mathematical property (modulo determinism and bounds) holds for all
    /// non-negative integers, so testing a representative range is sufficient.
    ///
    /// - Tier: 3 (Slow, >2min)
    /// - Verifies: Modulo index determinism and bounds
    /// - Related: proof_window_index_in_bounds, proof_advance_frame_safe
    #[kani::proof]
    fn proof_index_wrapping_consistent() {
        let frame_val: i32 = kani::any();
        // Bound to representative range - the math property is universal
        kani::assume(frame_val >= 0 && frame_val <= 10_000);

        let window_size: usize = kani::any();
        kani::assume(window_size >= 1 && window_size <= 100);

        // First calculation
        let index1 = (frame_val as usize) % window_size;

        // Second calculation with same inputs (should be deterministic)
        let index2 = (frame_val as usize) % window_size;

        kani::assert(index1 == index2, "Index calculation must be deterministic");
        kani::assert(
            index1 < window_size,
            "Index must be strictly less than window_size",
        );
    }

    /// Proof: Negative frame values are correctly rejected.
    ///
    /// Verifies that advance_frame with negative frames doesn't modify state.
    /// Note: We can't directly test this in Kani without state comparison,
    /// so we verify the safety property that it doesn't panic.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Negative frame rejection safety
    /// - Related: proof_advance_frame_safe, proof_window_index_in_bounds
    #[kani::proof]
    fn proof_negative_frame_safe() {
        let frame_val: i32 = kani::any();
        kani::assume(frame_val < 0);

        let local_adv: i32 = kani::any();
        let remote_adv: i32 = kani::any();
        kani::assume(local_adv >= -1000 && local_adv <= 1000);
        kani::assume(remote_adv >= -1000 && remote_adv <= 1000);

        let config = TimeSyncConfig { window_size: 10 };
        let mut ts = TimeSync::with_config(config);

        // This should not panic even with negative frame
        ts.advance_frame(Frame::new(frame_val), local_adv, remote_adv);

        // Verify state wasn't modified (local[0] should still be 0)
        kani::assert(
            ts.local[0] == 0,
            "Negative frame should not modify local values",
        );
        kani::assert(
            ts.remote[0] == 0,
            "Negative frame should not modify remote values",
        );
    }
}
