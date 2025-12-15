/// The `NetworkStats` struct contains statistics about the current session.
#[derive(Debug, Default, Clone, Copy)]
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
}

impl NetworkStats {
    /// Creates a new `NetworkStats` instance with default values.
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
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
    }

    #[test]
    fn test_network_stats_new() {
        let stats = NetworkStats::new();
        assert_eq!(stats.send_queue_len, 0);
        assert_eq!(stats.ping, 0);
        assert_eq!(stats.kbps_sent, 0);
        assert_eq!(stats.local_frames_behind, 0);
        assert_eq!(stats.remote_frames_behind, 0);
    }

    #[test]
    fn test_network_stats_debug() {
        let stats = NetworkStats {
            send_queue_len: 5,
            ping: 100,
            kbps_sent: 50,
            local_frames_behind: 2,
            remote_frames_behind: -1,
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
        };
        let cloned = stats;
        assert_eq!(cloned.send_queue_len, 10);
        assert_eq!(cloned.ping, 50);
        assert_eq!(cloned.kbps_sent, 100);
        assert_eq!(cloned.local_frames_behind, 3);
        assert_eq!(cloned.remote_frames_behind, -2);
    }

    #[test]
    fn test_network_stats_negative_frames_behind() {
        let stats = NetworkStats {
            send_queue_len: 0,
            ping: 0,
            kbps_sent: 0,
            local_frames_behind: -5,
            remote_frames_behind: 5,
        };
        assert_eq!(stats.local_frames_behind, -5);
        assert_eq!(stats.remote_frames_behind, 5);
    }
}
