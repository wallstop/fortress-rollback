use crate::Frame;

const FRAME_WINDOW_SIZE: usize = 30;

#[derive(Debug)]
pub(crate) struct TimeSync {
    local: [i32; FRAME_WINDOW_SIZE],
    remote: [i32; FRAME_WINDOW_SIZE],
}

impl Default for TimeSync {
    fn default() -> Self {
        Self {
            local: [0; FRAME_WINDOW_SIZE],
            remote: [0; FRAME_WINDOW_SIZE],
        }
    }
}

impl TimeSync {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn advance_frame(&mut self, frame: Frame, local_adv: i32, remote_adv: i32) {
        self.local[frame.as_i32() as usize % self.local.len()] = local_adv;
        self.remote[frame.as_i32() as usize % self.remote.len()] = remote_adv;
    }

    pub(crate) fn average_frame_advantage(&self) -> i32 {
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
}
