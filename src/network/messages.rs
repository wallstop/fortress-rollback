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
    /// Per-slot connection-status **generation**, incremented by the owning peer
    /// on every `connected <-> disconnected` transition (a drop or a
    /// reactivation/hot-join re-open) of this slot in its own
    /// `local_connect_status`. It rides on the connect-status gossip carried by
    /// every input packet so a receiver can order a slot's reports by drop
    /// cycle: a report with a strictly-lower `epoch` is from an earlier cycle
    /// (a reordered or relay-lagged packet) and must not be mistaken for a fresh
    /// one.
    ///
    /// The spectator session is the only consumer today — it uses the epoch to
    /// close the two host->spectator reactivation fail-open corners (a stale
    /// earlier-cycle drop report re-arming consumed provenance; a reordered
    /// pre-drop connected snapshot transiently resurrecting a dropped slot). The
    /// player-mesh confirmed/freeze folds deliberately ignore it (they read only
    /// `disconnected`/`last_frame`), so carrying the epoch is behavior-neutral
    /// there.
    ///
    /// Wraps at [`u16::MAX`]; a single slot toggling 65535 times within one
    /// session is unreachable in practice (the same astronomically-rare,
    /// documented framing as the protocol packet-filter `magic` era counter).
    pub epoch: u16,
}

impl Default for ConnectionStatus {
    fn default() -> Self {
        Self {
            disconnected: false,
            last_frame: Frame::NULL,
            epoch: 0,
        }
    }
}

impl std::fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Destructure to ensure all fields are included when new fields are added.
        let Self {
            disconnected,
            last_frame,
            epoch,
        } = self;

        if *disconnected {
            write!(
                f,
                "Disconnected(last_frame={}, epoch={epoch})",
                last_frame.as_i32()
            )
        } else {
            write!(
                f,
                "Connected(last_frame={}, epoch={epoch})",
                last_frame.as_i32()
            )
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

/// Observer → relay: a **floor-round request** for the double-failure-relay
/// connected-relay reorder fix (the audit's last open player-vs-player desync
/// sub-shape; verified-sound mode `AsyncAckSoundRoundSeq` in
/// `specs/tla/DoubleFailureRelay.tla`, S54).
///
/// While an observer is in the N≥4 relay topology (it has pruned a remote and
/// ≥2 remotes still run), a connected relay's connect-status receipt is its own
/// (possibly high) `last_frame`, not a freeze, so it can hide a departed
/// origin's low the relay still folds — and piggybacking the relay's pessimistic
/// floor on out-of-order `Input` gossip would let a reordered stale-HIGH packet
/// erase a fresh-low one. Instead the observer solicits the relay's CURRENT
/// pessimistic floor through this dedicated request/response round whose
/// [`round_seq`](Self::round_seq) makes the reply reorder-immune.
///
/// [`round_seq`](Self::round_seq) is the observer's **prune generation**: a
/// monotonic counter the observer bumps on every running→pruned remote
/// transition. The relay echoes it verbatim in its [`FloorReply`], so the
/// observer accepts a reply only when its `round_seq` matches the observer's
/// current generation — rejecting both a reordered stale reply (a strictly
/// older generation) and a pre-prune reply (the freshness postdates the most
/// recent prune). No floor-epoch wire field is needed (S54 machine-shows the
/// seq + dedicated channel replace it).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) struct FloorRequest {
    /// The requesting observer's current prune generation (see the struct docs).
    pub round_seq: u32,
}

/// Relay → observer: a **floor-round reply** answering a [`FloorRequest`] with
/// the relay's current per-slot pessimistic floors (the double-failure-relay
/// connected-relay reorder fix; see [`FloorRequest`]).
///
/// [`round_seq`](Self::round_seq) echoes the request's value verbatim so the
/// observer can validate freshness (accept only when it equals the observer's
/// current prune generation). [`floors`](Self::floors) is the relay's
/// `P2PSession::pessimistic_floors` snapshot at reply time — the `min`, per
/// slot, over the relay's own receipt/freeze and every running, non-reserved
/// peer it still folds — surfacing a departed global-min origin's low the
/// observer has already pruned from its own fold. Index = player handle; an
/// observer reads it via `.get(slot)` and falls back to `last_frame` for a
/// missing / `Frame::NULL` slot, so a length mismatch is tolerated (never
/// indexed) and never confirms higher than the legacy fold.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) struct FloorReply {
    /// The originating [`FloorRequest::round_seq`], echoed verbatim.
    pub round_seq: u32,
    /// The relay's per-slot pessimistic floors at reply time (handle-indexed).
    pub floors: Vec<Frame>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) struct MessageHeader {
    pub magic: u16,
}

#[cfg(feature = "hot-join")]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct JoinRequest {
    /// The player handle (slot index) the joiner wants to occupy.
    pub player_handle: usize,
}

#[cfg(feature = "hot-join")]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StateSnapshot {
    /// The frame the serialized state corresponds to. On the 2-peer host path
    /// this is the activation frame `F` (the joiner is real from the frame it
    /// loads); on the N-peer coordinator path this is the snapshot frame
    /// `S = F - 1` (the joiner bridges one frame using
    /// [`bridge_inputs`](Self::bridge_inputs)).
    pub frame: Frame,
    /// Total player count the snapshot was produced with (receiver validates equality).
    pub num_players: usize,
    /// bincode-serialized `Config::State` at `frame`.
    pub state_bytes: Vec<u8>,
    /// N-peer bridge inputs (chunk N4): the **confirmed** inputs at `frame`
    /// (= `S`) for all `num_players` slots, in handle order, each
    /// `codec`-encoded with the crate's fixed-int configuration and
    /// concatenated (`Config::Input` is fixed-width by the network-input-width
    /// rule, so this is exactly `num_players × input_width` bytes). The
    /// joining slot's entry is its agreed frozen value. **Empty on the 2-peer
    /// host path** — emptiness is the explicit wire discriminator between the
    /// two serve shapes, and both joiner roles reject the mismatched shape
    /// fail-closed (an N-peer joiner must never invent bridge inputs).
    /// A zero-width `Config::Input` cannot make a genuine N-peer blob empty:
    /// network sessions reject zero-byte input encodings at endpoint
    /// construction (`validate_default_input_wire_size` →
    /// `SerializationErrorKind::InputSerializedSizeZero`), so the
    /// discriminator is unambiguous for every constructible session.
    pub bridge_inputs: Vec<u8>,
    /// N-peer per-slot connection statuses at `frame` (= `S`), in handle
    /// order (S34 fix round 1): the coordinator's `local_connect_status`
    /// captured together with the snapshot. The joiner derives each slot's
    /// bridge [`InputStatus`] from exactly the predicate every survivor's
    /// simulation of `S` used (`disconnected && last_frame < S` ⇒
    /// `Disconnected`, else `Confirmed` — so the `f0 == S` boundary presents
    /// the slot's real frame-`S` input as `Confirmed` mesh-wide), keeps
    /// carried-disconnected slots frozen at their carried value, and stamps
    /// its own connection-status table from these instead of assuming every
    /// slot live. **Empty on the 2-peer host path** — the same
    /// shape-discriminator contract as [`bridge_inputs`](Self::bridge_inputs)
    /// (both joiner roles reject the mismatched shape fail-closed).
    ///
    /// [`InputStatus`]: crate::InputStatus
    pub bridge_statuses: Vec<ConnectionStatus>,
    /// Optional checksum of the saved state at `frame`.
    pub checksum: Option<u128>,
}

#[cfg(feature = "hot-join")]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StateSnapshotAck {
    /// The frame the joiner successfully loaded.
    pub frame: Frame,
}

#[cfg(feature = "hot-join")]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ReactivateSlot {
    /// The player handle (slot index) the survivor should reopen.
    pub handle: usize,
    /// The activation frame at which the slot becomes live again.
    pub frame: Frame,
}

#[cfg(feature = "hot-join")]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ReactivateSlotAck {
    /// The player handle (slot index) the survivor reopened.
    pub handle: usize,
    /// The activation frame the survivor acknowledged.
    pub frame: Frame,
}

/// Coordinator → joiner + survivors: the N-peer join for `handle` at activation
/// frame `frame` is committed (every survivor reopened and acked, the joiner
/// acked the snapshot, and the coordinator reactivated the slot locally).
///
/// `frame` is the activation frame `F` of the attempt, carried for attempt
/// discrimination: receivers ignore a `JoinCommitted` whose `(handle, frame)`
/// does not match their pending attempt (a stale resend from an earlier serve).
#[cfg(feature = "hot-join")]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct JoinCommitted {
    /// The player handle (slot index) whose join committed.
    pub handle: usize,
    /// The activation frame `F` of the committed attempt.
    pub frame: Frame,
}

/// Coordinator → joiner + survivors: the N-peer join for `handle` at activation
/// frame `frame` is aborted (serve timeout or joiner loss before commit). A
/// survivor that already reopened the slot restores its pre-reopen frozen state;
/// the joiner discards its buffered snapshot and may retry.
///
/// `frame` is the activation frame `F` of the aborted attempt (see
/// [`JoinCommitted`] for the attempt-discrimination contract).
#[cfg(feature = "hot-join")]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct JoinAborted {
    /// The player handle (slot index) whose join aborted.
    pub handle: usize,
    /// The activation frame `F` of the aborted attempt.
    pub frame: Frame,
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
    // Floor-round messages (double-failure-relay connected-relay reorder fix,
    // S55) are appended AFTER the original core variants so the existing core
    // discriminants (0..=7) stay stable; the `#[cfg(feature = "hot-join")]`
    // variants below them are positional and shift accordingly (the
    // hand-written `codec::decode_message` mirrors this numbering).
    FloorRequest(FloorRequest),
    FloorReply(FloorReply),
    #[cfg(feature = "hot-join")]
    JoinRequest(JoinRequest),
    #[cfg(feature = "hot-join")]
    StateSnapshot(StateSnapshot),
    #[cfg(feature = "hot-join")]
    StateSnapshotAck(StateSnapshotAck),
    #[cfg(feature = "hot-join")]
    ReactivateSlot(ReactivateSlot),
    #[cfg(feature = "hot-join")]
    ReactivateSlotAck(ReactivateSlotAck),
    #[cfg(feature = "hot-join")]
    JoinCommitted(JoinCommitted),
    #[cfg(feature = "hot-join")]
    JoinAborted(JoinAborted),
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
            epoch: 0,
        };
        let cloned = status;
        assert!(cloned.disconnected);
        assert_eq!(cloned.last_frame, Frame::new(100));
        let debug = format!("{:?}", status);
        assert!(debug.contains("ConnectionStatus"));
    }

    #[test]
    fn test_connection_status_display_connected() {
        let status = ConnectionStatus {
            disconnected: false,
            last_frame: Frame::new(42),
            epoch: 5,
        };
        let display = format!("{}", status);
        assert_eq!(display, "Connected(last_frame=42, epoch=5)");
    }

    #[test]
    fn test_connection_status_display_disconnected() {
        let status = ConnectionStatus {
            disconnected: true,
            last_frame: Frame::new(100),
            epoch: 3,
        };
        let display = format!("{}", status);
        assert_eq!(display, "Disconnected(last_frame=100, epoch=3)");
    }

    #[test]
    fn test_connection_status_display_null_frame() {
        let status = ConnectionStatus::default();
        let display = format!("{}", status);
        assert_eq!(display, "Connected(last_frame=-1, epoch=0)");
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

        let floor_request = MessageBody::FloorRequest(FloorRequest { round_seq: 7 });
        assert!(matches!(floor_request, MessageBody::FloorRequest(_)));

        let floor_reply = MessageBody::FloorReply(FloorReply {
            round_seq: 7,
            floors: vec![Frame::new(4), Frame::NULL],
        });
        assert!(matches!(floor_reply, MessageBody::FloorReply(_)));

        let keep_alive = MessageBody::KeepAlive;
        assert!(matches!(keep_alive, MessageBody::KeepAlive));
    }

    #[test]
    fn test_floor_request_default() {
        let req = FloorRequest::default();
        assert_eq!(req.round_seq, 0);
    }

    #[test]
    fn test_floor_reply_default() {
        let reply = FloorReply::default();
        assert_eq!(reply.round_seq, 0);
        assert!(reply.floors.is_empty());
    }

    #[test]
    fn test_floor_round_serialization() {
        use crate::network::codec;

        let request = Message {
            header: MessageHeader { magic: 0x1234 },
            body: MessageBody::FloorRequest(FloorRequest { round_seq: 9 }),
        };
        let serialized = codec::encode(&request).expect("serialization should succeed");
        let (deserialized, _): (Message, _) =
            codec::decode(&serialized).expect("deserialization should succeed");
        assert_eq!(request, deserialized);

        let reply = Message {
            header: MessageHeader { magic: 0x1234 },
            body: MessageBody::FloorReply(FloorReply {
                round_seq: 9,
                floors: vec![Frame::new(4), Frame::new(10), Frame::NULL, Frame::new(0)],
            }),
        };
        let serialized = codec::encode(&reply).expect("serialization should succeed");
        let (deserialized, _): (Message, _) =
            codec::decode(&serialized).expect("deserialization should succeed");
        assert_eq!(reply, deserialized);
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
                    epoch: 0,
                },
                ConnectionStatus {
                    disconnected: true,
                    last_frame: Frame::new(20),
                    epoch: 0,
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
