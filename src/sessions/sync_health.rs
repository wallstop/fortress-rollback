//! Synchronization health status for peer-to-peer sessions.
//!
//! This module provides the [`SyncHealth`] enum for tracking synchronization
//! status between peers in a P2P rollback networking session.

use crate::Frame;

/// Health status of synchronization with a remote peer.
///
/// This enum represents the current synchronization status between the local
/// session and a remote peer, based on checksum comparison.
///
/// # Important
///
/// This should be the primary API for checking session synchronization status
/// before termination. Using [`P2PSession::confirmed_frame`](crate::P2PSession::confirmed_frame) alone is
/// **not sufficient** to determine safe session termination.
///
/// # Example
///
/// ```ignore
/// // Check sync status before terminating
/// match session.sync_health(peer_handle) {
///     Some(SyncHealth::InSync) => {
///         // Safe to proceed - checksums match
///     }
///     Some(SyncHealth::DesyncDetected { frame, .. }) => {
///         // Desync occurred - game state has diverged
///         panic!("Desync at frame {}", frame);
///     }
///     Some(SyncHealth::Pending) | None => {
///         // Still waiting for checksum data
///     }
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncHealth {
    /// Checksums match at the most recently compared frame.
    ///
    /// This indicates that at the last checksum comparison, both peers
    /// had identical game state. Note that this does not guarantee
    /// synchronization at the current frame - only at the last compared frame.
    InSync,

    /// Waiting for checksum data from peer (no comparison possible yet).
    ///
    /// This typically occurs:
    /// - Early in the session before enough frames have been confirmed
    /// - When desync detection is disabled
    /// - When the peer hasn't sent checksum data yet
    Pending,

    /// Checksums differ - game state has diverged.
    ///
    /// This is a critical error indicating non-determinism or a bug.
    /// The session should typically be terminated when this occurs.
    DesyncDetected {
        /// The frame at which the desync was detected.
        frame: Frame,
        /// The checksum computed locally for this frame.
        local_checksum: u128,
        /// The checksum received from the remote peer for this frame.
        remote_checksum: u128,
    },
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
    fn sync_health_in_sync_equality() {
        let a = SyncHealth::InSync;
        let b = SyncHealth::InSync;
        assert_eq!(a, b);
    }

    #[test]
    fn sync_health_pending_equality() {
        let a = SyncHealth::Pending;
        let b = SyncHealth::Pending;
        assert_eq!(a, b);
    }

    #[test]
    fn sync_health_desync_equality() {
        let a = SyncHealth::DesyncDetected {
            frame: Frame::new(10),
            local_checksum: 0x1234,
            remote_checksum: 0x5678,
        };
        let b = SyncHealth::DesyncDetected {
            frame: Frame::new(10),
            local_checksum: 0x1234,
            remote_checksum: 0x5678,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn sync_health_desync_inequality() {
        let a = SyncHealth::DesyncDetected {
            frame: Frame::new(10),
            local_checksum: 0x1234,
            remote_checksum: 0x5678,
        };
        let b = SyncHealth::DesyncDetected {
            frame: Frame::new(10),
            local_checksum: 0x1234,
            remote_checksum: 0x9999, // Different checksum
        };
        assert_ne!(a, b);
    }

    #[test]
    fn sync_health_different_variants_not_equal() {
        let in_sync = SyncHealth::InSync;
        let pending = SyncHealth::Pending;
        let desync = SyncHealth::DesyncDetected {
            frame: Frame::new(10),
            local_checksum: 0x1234,
            remote_checksum: 0x5678,
        };

        assert_ne!(in_sync, pending);
        assert_ne!(in_sync, desync);
        assert_ne!(pending, desync);
    }

    #[test]
    fn sync_health_clone() {
        let original = SyncHealth::DesyncDetected {
            frame: Frame::new(10),
            local_checksum: 0x1234,
            remote_checksum: 0x5678,
        };
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn sync_health_debug_format() {
        let in_sync = SyncHealth::InSync;
        let debug_str = format!("{:?}", in_sync);
        assert!(debug_str.contains("InSync"));

        let pending = SyncHealth::Pending;
        let debug_str = format!("{:?}", pending);
        assert!(debug_str.contains("Pending"));

        let desync = SyncHealth::DesyncDetected {
            frame: Frame::new(42),
            local_checksum: 0x1234,
            remote_checksum: 0x5678,
        };
        let debug_str = format!("{:?}", desync);
        assert!(debug_str.contains("DesyncDetected"));
        assert!(debug_str.contains("42"));
    }
}
