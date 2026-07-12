//! Recorded legacy protocol-0.9.0 packets.
//!
//! Provenance: generated once from annotated tag `v0.9.0`, commit
//! `505d631f9357be3964f3ae24b076043957c7bac0`, with:
//! `cargo test --lib --features hot-join dump_wire_goldens_0_9_0 -- --ignored --nocapture`.
//! The throwaway source was a unit test in `src/network/messages.rs` that built one
//! fully populated `MessageBody` value per positional variant, wrapped each in
//! `Message { header: MessageHeader { magic: 0x1234 }, body }`, serialized it with
//! `crate::network::codec::encode`, and printed the resulting byte slice. The
//! SyncRequest/SyncReply entries are the exact datagrams exchanged by the legacy
//! sync handshake for echoed token `0x10203040`.
//!
//! Exact throwaway generator source (inserted inside the tag's private
//! `messages::tests` module; imports resolve through `use super::*`):
//! ```ignore
//! #[test]
//! #[ignore]
//! fn dump_wire_goldens_0_9_0() {
//!     use crate::network::codec;
//!     let header = MessageHeader { magic: 0x1234 };
//!     let statuses = vec![
//!         ConnectionStatus { disconnected: false, last_frame: Frame::new(10), epoch: 0x0201 },
//!         ConnectionStatus { disconnected: true, last_frame: Frame::new(20), epoch: 7 },
//!     ];
//!     let cases = vec![
//!         MessageBody::SyncRequest(SyncRequest { random_request: 0x1020_3040 }),
//!         MessageBody::SyncReply(SyncReply { random_reply: 0x1020_3040 }),
//!         MessageBody::Input(Input { peer_connect_status: statuses, disconnect_requested: true,
//!             start_frame: Frame::new(100), ack_frame: Frame::new(50),
//!             bytes: vec![0xAA, 0xBB, 0xCC, 0xDD] }),
//!         MessageBody::InputAck(InputAck { ack_frame: Frame::new(77) }),
//!         MessageBody::QualityReport(QualityReport { frame_advantage: -2,
//!             ping: 0x0102_0304_0506_0708_090A_0B0C_0D0E_0F10 }),
//!         MessageBody::QualityReply(QualityReply {
//!             pong: 0x1112_1314_1516_1718_191A_1B1C_1D1E_1F20 }),
//!         MessageBody::ChecksumReport(ChecksumReport {
//!             checksum: 0x2122_2324_2526_2728_292A_2B2C_2D2E_2F30,
//!             frame: Frame::new(88) }),
//!         MessageBody::KeepAlive,
//!         MessageBody::FloorRequest(FloorRequest { round_seq: 42 }),
//!         MessageBody::FloorReply(FloorReply { round_seq: 42,
//!             floors: vec![Frame::new(4), Frame::NULL, Frame::new(10)] }),
//!         MessageBody::JoinRequest(JoinRequest { player_handle: 2 }),
//!         MessageBody::StateSnapshot(StateSnapshot { frame: Frame::new(40), num_players: 3,
//!             state_bytes: vec![1, 2, 3], bridge_inputs: vec![4, 5, 6, 7],
//!             bridge_statuses: vec![ConnectionStatus { disconnected: false,
//!                 last_frame: Frame::new(40), epoch: 9 }],
//!             checksum: Some(0x3132_3334_3536_3738_393A_3B3C_3D3E_3F40) }),
//!         MessageBody::StateSnapshotAck(StateSnapshotAck { frame: Frame::new(40) }),
//!         MessageBody::ReactivateSlot(ReactivateSlot { handle: 2, frame: Frame::new(41) }),
//!         MessageBody::ReactivateSlotAck(ReactivateSlotAck { handle: 2, frame: Frame::new(41) }),
//!         MessageBody::JoinCommitted(JoinCommitted { handle: 2, frame: Frame::new(41) }),
//!         MessageBody::JoinAborted(JoinAborted { handle: 2, frame: Frame::new(41) }),
//!     ];
//!     for body in cases {
//!         let bytes = codec::encode(&Message { header, body }).unwrap();
//!         println!("{bytes:#04X?}");
//!     }
//! }
//! ```

use super::{classify_wire_bytes, decode_message, wire_golden_v1, WireRejectKind};

#[path = "../../tests/network/wire_golden_legacy_0_9.rs"]
mod recorded;
use recorded::LEGACY_SYNC_REQUEST;

const LEGACY_SYNC_REPLY: &[u8] = &[0x34, 0x12, 0x01, 0x00, 0x00, 0x00, 0x40, 0x30, 0x20, 0x10];
const LEGACY_INPUT: &[u8] = &[
    0x34, 0x12, 0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0A,
    0x00, 0x00, 0x00, 0x01, 0x02, 0x01, 0x14, 0x00, 0x00, 0x00, 0x07, 0x00, 0x01, 0x64, 0x00, 0x00,
    0x00, 0x32, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xAA, 0xBB, 0xCC,
    0xDD,
];
const LEGACY_INPUT_ACK: &[u8] = &[0x34, 0x12, 0x03, 0x00, 0x00, 0x00, 0x4D, 0x00, 0x00, 0x00];
const LEGACY_QUALITY_REPORT: &[u8] = &[
    0x34, 0x12, 0x04, 0x00, 0x00, 0x00, 0xFE, 0xFF, 0x10, 0x0F, 0x0E, 0x0D, 0x0C, 0x0B, 0x0A, 0x09,
    0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01,
];
const LEGACY_QUALITY_REPLY: &[u8] = &[
    0x34, 0x12, 0x05, 0x00, 0x00, 0x00, 0x20, 0x1F, 0x1E, 0x1D, 0x1C, 0x1B, 0x1A, 0x19, 0x18, 0x17,
    0x16, 0x15, 0x14, 0x13, 0x12, 0x11,
];
const LEGACY_CHECKSUM_REPORT: &[u8] = &[
    0x34, 0x12, 0x06, 0x00, 0x00, 0x00, 0x30, 0x2F, 0x2E, 0x2D, 0x2C, 0x2B, 0x2A, 0x29, 0x28, 0x27,
    0x26, 0x25, 0x24, 0x23, 0x22, 0x21, 0x58, 0x00, 0x00, 0x00,
];
const LEGACY_KEEP_ALIVE: &[u8] = &[0x34, 0x12, 0x07, 0x00, 0x00, 0x00];
const LEGACY_FLOOR_REQUEST: &[u8] = &[0x34, 0x12, 0x08, 0x00, 0x00, 0x00, 0x2A, 0x00, 0x00, 0x00];
const LEGACY_FLOOR_REPLY: &[u8] = &[
    0x34, 0x12, 0x09, 0x00, 0x00, 0x00, 0x2A, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0x0A, 0x00, 0x00, 0x00,
];
const LEGACY_JOIN_REQUEST: &[u8] = &[
    0x34, 0x12, 0x0A, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];
const LEGACY_STATE_SNAPSHOT: &[u8] = &[
    0x34, 0x12, 0x0B, 0x00, 0x00, 0x00, 0x28, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x05, 0x06, 0x07, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x28, 0x00, 0x00, 0x00, 0x09, 0x00, 0x01, 0x40, 0x3F, 0x3E, 0x3D, 0x3C, 0x3B, 0x3A,
    0x39, 0x38, 0x37, 0x36, 0x35, 0x34, 0x33, 0x32, 0x31,
];
const LEGACY_STATE_SNAPSHOT_ACK: &[u8] =
    &[0x34, 0x12, 0x0C, 0x00, 0x00, 0x00, 0x28, 0x00, 0x00, 0x00];
const LEGACY_REACTIVATE_SLOT: &[u8] = &[
    0x34, 0x12, 0x0D, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x29, 0x00,
    0x00, 0x00,
];
const LEGACY_REACTIVATE_SLOT_ACK: &[u8] = &[
    0x34, 0x12, 0x0E, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x29, 0x00,
    0x00, 0x00,
];
const LEGACY_JOIN_COMMITTED: &[u8] = &[
    0x34, 0x12, 0x0F, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x29, 0x00,
    0x00, 0x00,
];
const LEGACY_JOIN_ABORTED: &[u8] = &[
    0x34, 0x12, 0x10, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x29, 0x00,
    0x00, 0x00,
];

fn fixtures() -> [(&'static str, &'static [u8]); 17] {
    [
        ("SyncRequest", LEGACY_SYNC_REQUEST),
        ("SyncReply", LEGACY_SYNC_REPLY),
        ("Input", LEGACY_INPUT),
        ("InputAck", LEGACY_INPUT_ACK),
        ("QualityReport", LEGACY_QUALITY_REPORT),
        ("QualityReply", LEGACY_QUALITY_REPLY),
        ("ChecksumReport", LEGACY_CHECKSUM_REPORT),
        ("KeepAlive", LEGACY_KEEP_ALIVE),
        ("FloorRequest", LEGACY_FLOOR_REQUEST),
        ("FloorReply", LEGACY_FLOOR_REPLY),
        ("JoinRequest", LEGACY_JOIN_REQUEST),
        ("StateSnapshot", LEGACY_STATE_SNAPSHOT),
        ("StateSnapshotAck", LEGACY_STATE_SNAPSHOT_ACK),
        ("ReactivateSlot", LEGACY_REACTIVATE_SLOT),
        ("ReactivateSlotAck", LEGACY_REACTIVATE_SLOT_ACK),
        ("JoinCommitted", LEGACY_JOIN_COMMITTED),
        ("JoinAborted", LEGACY_JOIN_ABORTED),
    ]
}

fn legacy_header_accepts(bytes: &[u8]) -> bool {
    if bytes.get(..2) != Some(&[0x34, 0x12]) {
        return false;
    }
    let Some(tag_bytes) = bytes.get(2..6) else {
        return false;
    };
    let Ok(tag_bytes) = <[u8; 4]>::try_from(tag_bytes) else {
        return false;
    };
    u32::from_le_bytes(tag_bytes) <= 16
}

#[test]
fn every_legacy_fixture_is_classified_and_rejected_by_v1() {
    for (variant, bytes) in fixtures() {
        assert!(legacy_header_accepts(bytes), "legacy header for {variant}");
        assert_eq!(
            classify_wire_bytes(bytes),
            WireRejectKind::LegacyUnversionedSuspected,
            "classification for {variant}"
        );
        assert!(
            decode_message(bytes).is_err(),
            "v1 decoder must reject legacy {variant}"
        );
    }
}

#[test]
fn legacy_header_rejects_wrong_magic_even_with_a_known_tag() {
    assert!(!legacy_header_accepts(&[
        0x00, 0x00, 0x07, 0x00, 0x00, 0x00
    ]));
}

#[test]
fn legacy_header_rejects_every_v1_golden() {
    for (variant, message) in wire_golden_v1::fixtures() {
        let bytes = wire_golden_v1::expected(&message.body);
        assert!(
            !legacy_header_accepts(bytes),
            "legacy decoder accepted v1 {variant}"
        );
    }
}
