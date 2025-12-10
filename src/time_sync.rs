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
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Configuration preset for responsive synchronization.
    ///
    /// Uses a smaller window to react quickly to network changes,
    /// at the cost of potentially more fluctuation in game speed.
    #[must_use]
    pub fn responsive() -> Self {
        Self { window_size: 15 }
    }

    /// Configuration preset for smooth synchronization.
    ///
    /// Uses a larger window to provide stable, smooth synchronization,
    /// at the cost of slower adaptation to network changes.
    #[must_use]
    pub fn smooth() -> Self {
        Self { window_size: 60 }
    }

    /// Configuration preset for LAN play.
    ///
    /// Uses a small window since LAN connections are typically stable.
    #[must_use]
    pub fn lan() -> Self {
        Self { window_size: 10 }
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
