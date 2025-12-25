use serde::{Deserialize, Serialize};

use crate::Frame;

/// Connection status for a peer in the network protocol.
///
/// # Note
///
/// This type is re-exported in [`__internal`](crate::__internal) for testing and fuzzing.
/// It is not part of the stable public API.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionStatus {
    /// Whether this peer has disconnected.
    pub disconnected: bool,
    /// The last frame received from this peer.
    pub last_frame: Frame,
}

impl Default for ConnectionStatus {
    fn default() -> Self {
        Self {
            disconnected: false,
            last_frame: Frame::NULL,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) struct SyncRequest {
    pub random_request: u32, // please reply back with this random data
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) struct SyncReply {
    pub random_reply: u32, // here's your random data back
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Input {
    pub peer_connect_status: Vec<ConnectionStatus>,
    pub disconnect_requested: bool,
    pub start_frame: Frame,
    pub ack_frame: Frame,
    pub bytes: Vec<u8>,
}

impl Default for Input {
    fn default() -> Self {
        Self {
            peer_connect_status: Vec::new(),
            disconnect_requested: false,
            start_frame: Frame::NULL,
            ack_frame: Frame::NULL,
            bytes: Vec::new(),
        }
    }
}

impl std::fmt::Debug for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Destructure to ensure all fields are included when new fields are added.
        let Self {
            peer_connect_status,
            disconnect_requested,
            start_frame,
            ack_frame,
            bytes,
        } = self;

        f.debug_struct("Input")
            .field("peer_connect_status", peer_connect_status)
            .field("disconnect_requested", disconnect_requested)
            .field("start_frame", start_frame)
            .field("ack_frame", ack_frame)
            .field("bytes", &BytesDebug(bytes))
            .finish()
    }
}
struct BytesDebug<'a>(&'a [u8]);

impl std::fmt::Debug for BytesDebug<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("0x")?;
        for byte in self.0 {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct InputAck {
    pub ack_frame: Frame,
}

impl Default for InputAck {
    fn default() -> Self {
        Self {
            ack_frame: Frame::NULL,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) struct QualityReport {
    /// Frame advantage of other player.
    ///
    /// While on the one hand 2 bytes is overkill for a value that is typically in the range of say
    /// -8 to 8 (for the default prediction window size of 8), on the other hand if we don't get a
    /// chance to read quality reports for a time (due to being paused in a background tab, or
    /// someone stepping through code in a debugger) then it is easy to exceed the range of a signed
    /// 1 byte integer at common FPS values.
    ///
    /// So by using an i16 instead of an i8, we can avoid clamping the value for +/- ~32k frames, or
    /// about +/- 524 seconds of frame advantage - and after 500+ seconds it's a pretty reasonable
    /// assumption that the other player will have been disconnected, or at least that they're so
    /// far ahead/behind that clamping the value to an i16 won't matter for any practical purpose.
    pub frame_advantage: i16,
    pub ping: u128,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) struct QualityReply {
    pub pong: u128,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) struct ChecksumReport {
    pub checksum: u128,
    pub frame: Frame,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) struct MessageHeader {
    pub magic: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum MessageBody {
    SyncRequest(SyncRequest),
    SyncReply(SyncReply),
    Input(Input),
    InputAck(InputAck),
    QualityReport(QualityReport),
    QualityReply(QualityReply),
    ChecksumReport(ChecksumReport),
    KeepAlive,
}

/// A messages that [`NonBlockingSocket`] sends and receives. When implementing [`NonBlockingSocket`],
/// you should deserialize received messages into this `Message` type and pass them.
///
/// [`NonBlockingSocket`]: crate::NonBlockingSocket
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub(crate) header: MessageHeader,
    pub(crate) body: MessageBody,
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
    fn test_connection_status_default() {
        let status = ConnectionStatus::default();
        assert!(!status.disconnected);
        assert_eq!(status.last_frame, Frame::NULL);
    }

    #[test]
    fn test_connection_status_debug_clone() {
        let status = ConnectionStatus {
            disconnected: true,
            last_frame: Frame::new(100),
        };
        let cloned = status;
        assert!(cloned.disconnected);
        assert_eq!(cloned.last_frame, Frame::new(100));
        let debug = format!("{:?}", status);
        assert!(debug.contains("ConnectionStatus"));
    }

    #[test]
    fn test_sync_request_default() {
        let req = SyncRequest::default();
        assert_eq!(req.random_request, 0);
    }

    #[test]
    fn test_sync_reply_default() {
        let reply = SyncReply::default();
        assert_eq!(reply.random_reply, 0);
    }

    #[test]
    fn test_input_default() {
        let input = Input::default();
        assert!(input.peer_connect_status.is_empty());
        assert!(!input.disconnect_requested);
        assert_eq!(input.start_frame, Frame::NULL);
        assert_eq!(input.ack_frame, Frame::NULL);
        assert!(input.bytes.is_empty());
    }

    #[test]
    fn test_input_debug() {
        let input = Input {
            peer_connect_status: vec![ConnectionStatus::default()],
            disconnect_requested: true,
            start_frame: Frame::new(10),
            ack_frame: Frame::new(5),
            bytes: vec![0xDE, 0xAD, 0xBE, 0xEF],
        };
        let debug = format!("{:?}", input);
        assert!(debug.contains("Input"));
        assert!(debug.contains("disconnect_requested"));
        assert!(debug.contains("0xdeadbeef"));
    }

    #[test]
    fn test_input_ack_default() {
        let ack = InputAck::default();
        assert_eq!(ack.ack_frame, Frame::NULL);
    }

    #[test]
    fn test_quality_report_default() {
        let report = QualityReport::default();
        assert_eq!(report.frame_advantage, 0);
        assert_eq!(report.ping, 0);
    }

    #[test]
    fn test_quality_reply_default() {
        let reply = QualityReply::default();
        assert_eq!(reply.pong, 0);
    }

    #[test]
    fn test_checksum_report_default() {
        let report = ChecksumReport::default();
        assert_eq!(report.checksum, 0);
        assert_eq!(report.frame, Frame::default());
    }

    #[test]
    fn test_message_header_default() {
        let header = MessageHeader::default();
        assert_eq!(header.magic, 0);
    }

    #[test]
    fn test_message_body_variants() {
        // Test each variant can be created and compared
        let sync_req = MessageBody::SyncRequest(SyncRequest { random_request: 42 });
        let sync_req2 = MessageBody::SyncRequest(SyncRequest { random_request: 42 });
        assert_eq!(sync_req, sync_req2);

        let sync_reply = MessageBody::SyncReply(SyncReply { random_reply: 123 });
        let debug = format!("{:?}", sync_reply);
        assert!(debug.contains("SyncReply"));

        let input = MessageBody::Input(Input::default());
        assert!(matches!(input, MessageBody::Input(_)));

        let input_ack = MessageBody::InputAck(InputAck::default());
        assert!(matches!(input_ack, MessageBody::InputAck(_)));

        let quality_report = MessageBody::QualityReport(QualityReport::default());
        assert!(matches!(quality_report, MessageBody::QualityReport(_)));

        let quality_reply = MessageBody::QualityReply(QualityReply::default());
        assert!(matches!(quality_reply, MessageBody::QualityReply(_)));

        let checksum_report = MessageBody::ChecksumReport(ChecksumReport::default());
        assert!(matches!(checksum_report, MessageBody::ChecksumReport(_)));

        let keep_alive = MessageBody::KeepAlive;
        assert!(matches!(keep_alive, MessageBody::KeepAlive));
    }

    #[test]
    #[allow(clippy::redundant_clone)] // Testing Clone trait implementation
    fn test_message_clone_eq() {
        let msg = Message {
            header: MessageHeader { magic: 0x1234 },
            body: MessageBody::KeepAlive,
        };
        let cloned = msg.clone();
        assert_eq!(msg, cloned);
    }

    #[test]
    fn test_message_serialization() {
        use crate::network::codec;

        let msg = Message {
            header: MessageHeader { magic: 0xABCD },
            body: MessageBody::SyncRequest(SyncRequest {
                random_request: 999,
            }),
        };

        // Test that serialization/deserialization roundtrips correctly
        let serialized = codec::encode(&msg).expect("serialization should succeed");
        let (deserialized, _): (Message, _) =
            codec::decode(&serialized).expect("deserialization should succeed");
        assert_eq!(msg, deserialized);
    }

    #[test]
    fn test_input_serialization() {
        use crate::network::codec;

        let input = Input {
            peer_connect_status: vec![
                ConnectionStatus {
                    disconnected: false,
                    last_frame: Frame::new(10),
                },
                ConnectionStatus {
                    disconnected: true,
                    last_frame: Frame::new(20),
                },
            ],
            disconnect_requested: false,
            start_frame: Frame::new(100),
            ack_frame: Frame::new(50),
            bytes: vec![1, 2, 3, 4, 5],
        };

        let serialized = codec::encode(&input).expect("serialization should succeed");
        let (deserialized, _): (Input, _) =
            codec::decode(&serialized).expect("deserialization should succeed");
        assert_eq!(input, deserialized);
    }

    #[test]
    fn test_bytes_debug_empty() {
        let input = Input {
            peer_connect_status: vec![],
            disconnect_requested: false,
            start_frame: Frame::NULL,
            ack_frame: Frame::NULL,
            bytes: vec![],
        };
        let debug = format!("{:?}", input);
        assert!(debug.contains("0x")); // Empty bytes should still show "0x" prefix
    }
}
