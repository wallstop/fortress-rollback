use crate::Frame;

/// The `NetworkStats` struct contains statistics about the current session.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[must_use = "NetworkStats should be inspected or used after being queried"]
pub struct NetworkStats {
    /// The length of the queue containing UDP packets which have not yet been acknowledged by the end client.
    /// The length of the send queue is a rough indication of the quality of the connection. The longer the send queue, the higher the round-trip time between the
    /// clients. The send queue will also be longer than usual during high packet loss situations.
    pub send_queue_len: usize,
    /// The roundtrip packet transmission time as calculated by Fortress Rollback.
    pub ping: u128,
    /// The estimated bandwidth used between the two clients, in kilobits per second.
    pub kbps_sent: usize,

    /// The number of frames Fortress Rollback calculates that the local client is behind the remote client at this instant in time.
    /// For example, if at this instant the current game client is running frame 1002 and the remote game client is running frame 1009,
    /// this value will mostly likely roughly equal 7.
    pub local_frames_behind: i32,
    /// The same as [`local_frames_behind`], but calculated from the perspective of the remote player.
    ///
    /// [`local_frames_behind`]: #structfield.local_frames_behind
    pub remote_frames_behind: i32,

    // === Checksum/Desync Detection Fields ===
    /// The most recent frame for which checksums were compared between peers.
    ///
    /// This is `None` if no checksum comparison has occurred yet (e.g., early
    /// in the session or if desync detection is disabled).
    pub last_compared_frame: Option<Frame>,

    /// The local checksum at [`last_compared_frame`].
    ///
    /// This is the checksum computed locally from the saved game state at that frame.
    /// Compare with [`remote_checksum`] to check for desync.
    ///
    /// [`last_compared_frame`]: #structfield.last_compared_frame
    /// [`remote_checksum`]: #structfield.remote_checksum
    pub local_checksum: Option<u128>,

    /// The remote checksum at [`last_compared_frame`].
    ///
    /// This is the checksum received from the remote peer for that frame.
    /// Compare with [`local_checksum`] to check for desync.
    ///
    /// [`last_compared_frame`]: #structfield.last_compared_frame
    /// [`local_checksum`]: #structfield.local_checksum
    pub remote_checksum: Option<u128>,

    /// Whether checksums matched at the most recently compared frame.
    ///
    /// This is a convenience field derived from comparing [`local_checksum`]
    /// and [`remote_checksum`]. It is `None` if no comparison has occurred.
    ///
    /// * `Some(true)` - Checksums match, peers are synchronized
    /// * `Some(false)` - **DESYNC DETECTED** - game state has diverged
    /// * `None` - No comparison available yet
    ///
    /// [`local_checksum`]: #structfield.local_checksum
    /// [`remote_checksum`]: #structfield.remote_checksum
    pub checksums_match: Option<bool>,
}

impl NetworkStats {
    /// Creates a new `NetworkStats` instance with default values.
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;

    #[test]
    fn test_network_stats_default() {
        let stats = NetworkStats::default();
        assert_eq!(stats.send_queue_len, 0);
        assert_eq!(stats.ping, 0);
        assert_eq!(stats.kbps_sent, 0);
        assert_eq!(stats.local_frames_behind, 0);
        assert_eq!(stats.remote_frames_behind, 0);
        assert_eq!(stats.last_compared_frame, None);
        assert_eq!(stats.local_checksum, None);
        assert_eq!(stats.remote_checksum, None);
        assert_eq!(stats.checksums_match, None);
    }

    #[test]
    fn test_network_stats_new() {
        let stats = NetworkStats::new();
        assert_eq!(stats.send_queue_len, 0);
        assert_eq!(stats.ping, 0);
        assert_eq!(stats.kbps_sent, 0);
        assert_eq!(stats.local_frames_behind, 0);
        assert_eq!(stats.remote_frames_behind, 0);
        assert_eq!(stats.last_compared_frame, None);
        assert_eq!(stats.local_checksum, None);
        assert_eq!(stats.remote_checksum, None);
        assert_eq!(stats.checksums_match, None);
    }

    #[test]
    fn test_network_stats_debug() {
        let stats = NetworkStats {
            send_queue_len: 5,
            ping: 100,
            kbps_sent: 50,
            local_frames_behind: 2,
            remote_frames_behind: -1,
            last_compared_frame: None,
            local_checksum: None,
            remote_checksum: None,
            checksums_match: None,
        };
        let debug = format!("{:?}", stats);
        assert!(debug.contains("NetworkStats"));
        assert!(debug.contains('5'));
        assert!(debug.contains("100"));
        assert!(debug.contains("50"));
    }

    #[test]
    fn test_network_stats_clone() {
        let stats = NetworkStats {
            send_queue_len: 10,
            ping: 50,
            kbps_sent: 100,
            local_frames_behind: 3,
            remote_frames_behind: -2,
            last_compared_frame: Some(Frame::new(42)),
            local_checksum: Some(12345),
            remote_checksum: Some(12345),
            checksums_match: Some(true),
        };
        let cloned = stats;
        assert_eq!(cloned.send_queue_len, 10);
        assert_eq!(cloned.ping, 50);
        assert_eq!(cloned.kbps_sent, 100);
        assert_eq!(cloned.local_frames_behind, 3);
        assert_eq!(cloned.remote_frames_behind, -2);
        assert_eq!(cloned.last_compared_frame, Some(Frame::new(42)));
        assert_eq!(cloned.local_checksum, Some(12345));
        assert_eq!(cloned.remote_checksum, Some(12345));
        assert_eq!(cloned.checksums_match, Some(true));
    }

    #[test]
    fn test_network_stats_negative_frames_behind() {
        let stats = NetworkStats {
            send_queue_len: 0,
            ping: 0,
            kbps_sent: 0,
            local_frames_behind: -5,
            remote_frames_behind: 5,
            last_compared_frame: None,
            local_checksum: None,
            remote_checksum: None,
            checksums_match: None,
        };
        assert_eq!(stats.local_frames_behind, -5);
        assert_eq!(stats.remote_frames_behind, 5);
    }

    #[test]
    fn test_network_stats_checksum_fields() {
        let stats = NetworkStats {
            send_queue_len: 0,
            ping: 0,
            kbps_sent: 0,
            local_frames_behind: 0,
            remote_frames_behind: 0,
            last_compared_frame: Some(Frame::new(100)),
            local_checksum: Some(0xDEAD_BEEF),
            remote_checksum: Some(0xCAFE_BABE),
            checksums_match: Some(false),
        };
        assert_eq!(stats.last_compared_frame, Some(Frame::new(100)));
        assert_eq!(stats.local_checksum, Some(0xDEAD_BEEF));
        assert_eq!(stats.remote_checksum, Some(0xCAFE_BABE));
        assert_eq!(stats.checksums_match, Some(false));
    }
}
