//! Immutable protocol-v1 wire fixtures.
//!
//! Changing any literal in this released-version file requires a protocol-version
//! bump. `scripts/hooks/check-wire-golden-immutable.py` enforces that rule.

use super::{decode_message, decode_value, encode};
use crate::network::messages::{
    ChecksumReport, ConnectionStatus, DropAbort, DropAbortReason, DropBackfill, DropCommit,
    DropOperationId, DropPrepare, DropReceipt, DropReport, DropReportStage, DropTarget, FloorReply,
    FloorRequest, Goodbye, Input, InputAck, JoinAborted, JoinCommitted, JoinRequest, Message,
    MessageBody, MessageHeader, QualityReply, QualityReport, ReactivateSlot, ReactivateSlotAck,
    SessionConfigBlock, StateSnapshot, StateSnapshotAck, SyncReply, SyncRequest,
};
use crate::Frame;

pub(super) const WIRE_GOLDEN_VERSION: u8 = 1;

const SYNC_REQUEST: &[u8] = &[
    0xF5, 0x52, 0x01, 0x00, 0x34, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x30, 0x20, 0x10,
    0x01, 0x01, 0x00, 0x00, 0x00, 0x03, 0x00, 0x04, 0x00, 0x3C, 0x00, 0x00, 0x00, 0x08, 0x00, 0x78,
    0x00, 0x00, 0x00, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01,
];
const SYNC_REPLY: &[u8] = &[
    0xF5, 0x52, 0x01, 0x00, 0x34, 0x12, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x80, 0x70, 0x60, 0x50,
    0x01, 0x01, 0x00, 0x00, 0x00, 0x03, 0x00, 0x04, 0x00, 0x3C, 0x00, 0x00, 0x00, 0x08, 0x00, 0x78,
    0x00, 0x00, 0x00, 0x18, 0x17, 0x16, 0x15, 0x14, 0x13, 0x12, 0x11,
];
const INPUT: &[u8] = &[
    0xF5, 0x52, 0x01, 0x00, 0x34, 0x12, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x0A, 0x00, 0x00, 0x00, 0x01, 0x02, 0x01, 0x14, 0x00, 0x00, 0x00,
    0x07, 0x00, 0x64, 0x00, 0x00, 0x00, 0x32, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0xAA, 0xBB, 0xCC, 0xDD,
];
const INPUT_ACK: &[u8] = &[
    0xF5, 0x52, 0x01, 0x00, 0x34, 0x12, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x4D, 0x00, 0x00, 0x00,
];
const QUALITY_REPORT: &[u8] = &[
    0xF5, 0x52, 0x01, 0x00, 0x34, 0x12, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0xFE, 0xFF, 0x10, 0x0F,
    0x0E, 0x0D, 0x0C, 0x0B, 0x0A, 0x09, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01,
];
const QUALITY_REPLY: &[u8] = &[
    0xF5, 0x52, 0x01, 0x00, 0x34, 0x12, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x20, 0x1F, 0x1E, 0x1D,
    0x1C, 0x1B, 0x1A, 0x19, 0x18, 0x17, 0x16, 0x15, 0x14, 0x13, 0x12, 0x11,
];
const CHECKSUM_REPORT: &[u8] = &[
    0xF5, 0x52, 0x01, 0x00, 0x34, 0x12, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x30, 0x2F, 0x2E, 0x2D,
    0x2C, 0x2B, 0x2A, 0x29, 0x28, 0x27, 0x26, 0x25, 0x24, 0x23, 0x22, 0x21, 0x58, 0x00, 0x00, 0x00,
];
const KEEP_ALIVE: &[u8] = &[
    0xF5, 0x52, 0x01, 0x00, 0x34, 0x12, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00,
];
const FLOOR_REQUEST: &[u8] = &[
    0xF5, 0x52, 0x01, 0x00, 0x34, 0x12, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x2A, 0x00, 0x00, 0x00,
];
const FLOOR_REPLY: &[u8] = &[
    0xF5, 0x52, 0x01, 0x00, 0x34, 0x12, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x2A, 0x00, 0x00, 0x00,
    0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF,
    0x0A, 0x00, 0x00, 0x00,
];
const JOIN_REQUEST: &[u8] = &[
    0xF5, 0x52, 0x01, 0x00, 0x34, 0x12, 0x00, 0x00, 0x0A, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00,
];
const STATE_SNAPSHOT: &[u8] = &[
    0xF5, 0x52, 0x01, 0x00, 0x34, 0x12, 0x00, 0x00, 0x0B, 0x00, 0x00, 0x00, 0x28, 0x00, 0x00, 0x00,
    0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x01, 0x02, 0x03, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x05, 0x06, 0x07, 0x01,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x28, 0x00, 0x00, 0x00, 0x09, 0x00, 0x01, 0x40,
    0x3F, 0x3E, 0x3D, 0x3C, 0x3B, 0x3A, 0x39, 0x38, 0x37, 0x36, 0x35, 0x34, 0x33, 0x32, 0x31,
];
const STATE_SNAPSHOT_ACK: &[u8] = &[
    0xF5, 0x52, 0x01, 0x00, 0x34, 0x12, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00, 0x28, 0x00, 0x00, 0x00,
];
const REACTIVATE_SLOT: &[u8] = &[
    0xF5, 0x52, 0x01, 0x00, 0x34, 0x12, 0x00, 0x00, 0x0D, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x29, 0x00, 0x00, 0x00,
];
const REACTIVATE_SLOT_ACK: &[u8] = &[
    0xF5, 0x52, 0x01, 0x00, 0x34, 0x12, 0x00, 0x00, 0x0E, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x29, 0x00, 0x00, 0x00,
];
const JOIN_COMMITTED: &[u8] = &[
    0xF5, 0x52, 0x01, 0x00, 0x34, 0x12, 0x00, 0x00, 0x0F, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x29, 0x00, 0x00, 0x00,
];
const JOIN_ABORTED: &[u8] = &[
    0xF5, 0x52, 0x01, 0x00, 0x34, 0x12, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x29, 0x00, 0x00, 0x00,
];
const GOODBYE: &[u8] = &[
    0xF5, 0x52, 0x01, 0x00, 0x34, 0x12, 0x00, 0x00, 0x11, 0x00, 0x00, 0x00, 0x03,
];
const DROP_PREPARE: &[u8] = &[
    0xF5, 0x52, 0x01, 0x00, 0x34, 0x12, 0x00, 0x00, 0x12, 0x00, 0x00, 0x00, 0x02, 0x00, 0x07, 0x00,
    0x40, 0x30, 0x20, 0x10, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01, 0x02, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x09, 0x00, 0x05, 0x00, 0x09, 0x00, 0x04, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x03, 0x00,
];
const DROP_REPORT: &[u8] = &[
    0xF5, 0x52, 0x01, 0x00, 0x34, 0x12, 0x00, 0x00, 0x13, 0x00, 0x00, 0x00, 0x02, 0x00, 0x07, 0x00,
    0x40, 0x30, 0x20, 0x10, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01, 0x01, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x1E, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x0A, 0x00, 0x00, 0x00,
    0x1F, 0x00, 0x00, 0x00, 0x05, 0x00, 0x0B, 0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00,
];
const DROP_BACKFILL: &[u8] = &[
    0xF5, 0x52, 0x01, 0x00, 0x34, 0x12, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00, 0x02, 0x00, 0x07, 0x00,
    0x40, 0x30, 0x20, 0x10, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01, 0x01, 0x00, 0x03, 0x00,
    0x18, 0x00, 0x00, 0x00, 0x02, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xAA, 0xBB,
    0xCC, 0xDD,
];
const DROP_COMMIT: &[u8] = &[
    0xF5, 0x52, 0x01, 0x00, 0x34, 0x12, 0x00, 0x00, 0x15, 0x00, 0x00, 0x00, 0x02, 0x00, 0x07, 0x00,
    0x40, 0x30, 0x20, 0x10, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01, 0x1F, 0x00, 0x00, 0x00,
    0x18, 0x17, 0x16, 0x15, 0x14, 0x13, 0x12, 0x11,
];
const DROP_ABORT: &[u8] = &[
    0xF5, 0x52, 0x01, 0x00, 0x34, 0x12, 0x00, 0x00, 0x16, 0x00, 0x00, 0x00, 0x02, 0x00, 0x07, 0x00,
    0x40, 0x30, 0x20, 0x10, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01, 0x02, 0x00, 0x00, 0x00,
];

fn operation() -> DropOperationId {
    DropOperationId {
        coordinator: 2,
        coordinator_generation: 7,
        sequence: 0x1020_3040,
        target_set_digest: 0x0102_0304_0506_0708,
    }
}

pub(super) fn fixtures() -> Vec<(&'static str, Message)> {
    let config = SessionConfigBlock {
        num_players: 3,
        input_bytes_per_player: 4,
        fps: 60,
        max_prediction: 8,
        desync_interval: 120,
    };
    let bodies = vec![
        MessageBody::SyncRequest(SyncRequest {
            random_request: 0x1020_3040,
            min_compat_version: 1,
            features: 1,
            config,
            config_digest: 0x0102_0304_0506_0708,
        }),
        MessageBody::SyncReply(SyncReply {
            random_reply: 0x5060_7080,
            min_compat_version: 1,
            features: 1,
            config,
            config_digest: 0x1112_1314_1516_1718,
        }),
        MessageBody::Input(Input {
            peer_connect_status: vec![
                ConnectionStatus {
                    disconnected: false,
                    last_frame: Frame::new(10),
                    epoch: 0x0201,
                },
                ConnectionStatus {
                    disconnected: true,
                    last_frame: Frame::new(20),
                    epoch: 7,
                },
            ],
            start_frame: Frame::new(100),
            ack_frame: Frame::new(50),
            bytes: vec![0xAA, 0xBB, 0xCC, 0xDD],
        }),
        MessageBody::InputAck(InputAck {
            ack_frame: Frame::new(77),
        }),
        MessageBody::QualityReport(QualityReport {
            frame_advantage: -2,
            ping: 0x0102_0304_0506_0708_090A_0B0C_0D0E_0F10,
        }),
        MessageBody::QualityReply(QualityReply {
            pong: 0x1112_1314_1516_1718_191A_1B1C_1D1E_1F20,
        }),
        MessageBody::ChecksumReport(ChecksumReport {
            checksum: 0x2122_2324_2526_2728_292A_2B2C_2D2E_2F30,
            frame: Frame::new(88),
        }),
        MessageBody::KeepAlive,
        MessageBody::FloorRequest(FloorRequest { round_seq: 42 }),
        MessageBody::FloorReply(FloorReply {
            round_seq: 42,
            floors: vec![Frame::new(4), Frame::NULL, Frame::new(10)],
        }),
        MessageBody::JoinRequest(JoinRequest { player_handle: 2 }),
        MessageBody::StateSnapshot(StateSnapshot {
            frame: Frame::new(40),
            num_players: 3,
            state_bytes: vec![1, 2, 3],
            bridge_inputs: vec![4, 5, 6, 7],
            bridge_statuses: vec![ConnectionStatus {
                disconnected: false,
                last_frame: Frame::new(40),
                epoch: 9,
            }],
            checksum: Some(0x3132_3334_3536_3738_393A_3B3C_3D3E_3F40),
        }),
        MessageBody::StateSnapshotAck(StateSnapshotAck {
            frame: Frame::new(40),
        }),
        MessageBody::ReactivateSlot(ReactivateSlot {
            handle: 2,
            frame: Frame::new(41),
        }),
        MessageBody::ReactivateSlotAck(ReactivateSlotAck {
            handle: 2,
            frame: Frame::new(41),
        }),
        MessageBody::JoinCommitted(JoinCommitted {
            handle: 2,
            frame: Frame::new(41),
        }),
        MessageBody::JoinAborted(JoinAborted {
            handle: 2,
            frame: Frame::new(41),
        }),
        MessageBody::Goodbye(Goodbye { reason: 3 }),
        MessageBody::DropPrepare(DropPrepare {
            operation: operation(),
            targets: vec![
                DropTarget {
                    handle: 4,
                    generation: 9,
                },
                DropTarget {
                    handle: 5,
                    generation: 9,
                },
            ],
            participants: vec![0, 1, 2, 3],
        }),
        MessageBody::DropReport(DropReport {
            operation: operation(),
            participant: 1,
            stage: DropReportStage::Inventory,
            exposed_confirmed: Frame::new(30),
            cut: Frame::NULL,
            cut_digest: 0,
            receipts: vec![
                DropReceipt {
                    target: 4,
                    available_from: Frame::new(10),
                    contiguous_through: Frame::new(31),
                },
                DropReceipt {
                    target: 5,
                    available_from: Frame::new(11),
                    contiguous_through: Frame::new(31),
                },
            ],
        }),
        MessageBody::DropBackfill(DropBackfill {
            operation: operation(),
            chunk_index: 1,
            chunk_count: 3,
            start_frame: Frame::new(24),
            frame_count: 2,
            bytes: vec![0xAA, 0xBB, 0xCC, 0xDD],
        }),
        MessageBody::DropCommit(DropCommit {
            operation: operation(),
            cut: Frame::new(31),
            cut_digest: 0x1112_1314_1516_1718,
        }),
        MessageBody::DropAbort(DropAbort {
            operation: operation(),
            reason: DropAbortReason::ConflictingHistory,
        }),
    ];
    bodies
        .into_iter()
        .map(|body| {
            (
                name(&body),
                Message {
                    header: MessageHeader::new(0x1234),
                    body,
                },
            )
        })
        .collect()
}

fn name(body: &MessageBody) -> &'static str {
    match body {
        MessageBody::SyncRequest(_) => "SyncRequest",
        MessageBody::SyncReply(_) => "SyncReply",
        MessageBody::Input(_) => "Input",
        MessageBody::InputAck(_) => "InputAck",
        MessageBody::QualityReport(_) => "QualityReport",
        MessageBody::QualityReply(_) => "QualityReply",
        MessageBody::ChecksumReport(_) => "ChecksumReport",
        MessageBody::KeepAlive => "KeepAlive",
        MessageBody::FloorRequest(_) => "FloorRequest",
        MessageBody::FloorReply(_) => "FloorReply",
        MessageBody::JoinRequest(_) => "JoinRequest",
        MessageBody::StateSnapshot(_) => "StateSnapshot",
        MessageBody::StateSnapshotAck(_) => "StateSnapshotAck",
        MessageBody::ReactivateSlot(_) => "ReactivateSlot",
        MessageBody::ReactivateSlotAck(_) => "ReactivateSlotAck",
        MessageBody::JoinCommitted(_) => "JoinCommitted",
        MessageBody::JoinAborted(_) => "JoinAborted",
        MessageBody::Goodbye(_) => "Goodbye",
        MessageBody::DropPrepare(_) => "DropPrepare",
        MessageBody::DropReport(_) => "DropReport",
        MessageBody::DropBackfill(_) => "DropBackfill",
        MessageBody::DropCommit(_) => "DropCommit",
        MessageBody::DropAbort(_) => "DropAbort",
    }
}

pub(super) fn expected(body: &MessageBody) -> &'static [u8] {
    match body {
        MessageBody::SyncRequest(_) => SYNC_REQUEST,
        MessageBody::SyncReply(_) => SYNC_REPLY,
        MessageBody::Input(_) => INPUT,
        MessageBody::InputAck(_) => INPUT_ACK,
        MessageBody::QualityReport(_) => QUALITY_REPORT,
        MessageBody::QualityReply(_) => QUALITY_REPLY,
        MessageBody::ChecksumReport(_) => CHECKSUM_REPORT,
        MessageBody::KeepAlive => KEEP_ALIVE,
        MessageBody::FloorRequest(_) => FLOOR_REQUEST,
        MessageBody::FloorReply(_) => FLOOR_REPLY,
        MessageBody::JoinRequest(_) => JOIN_REQUEST,
        MessageBody::StateSnapshot(_) => STATE_SNAPSHOT,
        MessageBody::StateSnapshotAck(_) => STATE_SNAPSHOT_ACK,
        MessageBody::ReactivateSlot(_) => REACTIVATE_SLOT,
        MessageBody::ReactivateSlotAck(_) => REACTIVATE_SLOT_ACK,
        MessageBody::JoinCommitted(_) => JOIN_COMMITTED,
        MessageBody::JoinAborted(_) => JOIN_ABORTED,
        MessageBody::Goodbye(_) => GOODBYE,
        MessageBody::DropPrepare(_) => DROP_PREPARE,
        MessageBody::DropReport(_) => DROP_REPORT,
        MessageBody::DropBackfill(_) => DROP_BACKFILL,
        MessageBody::DropCommit(_) => DROP_COMMIT,
        MessageBody::DropAbort(_) => DROP_ABORT,
    }
}

#[test]
fn every_protocol_v1_variant_has_immutable_exact_bytes() {
    assert_eq!(
        crate::PROTOCOL_VERSION,
        WIRE_GOLDEN_VERSION,
        "wire bytes changed? create wire_golden_v2 and bump"
    );
    let fixtures = fixtures();
    for (variant, message) in fixtures {
        let expected = expected(&message.body);
        let encoded = encode(&message).expect("fixture must encode");
        assert_eq!(encoded, expected, "encoded bytes for {variant}");
        assert_eq!(
            message.encoded_len(),
            expected.len(),
            "encoded length for {variant}"
        );
        let generic: Message = decode_value(expected).expect("fixture must generically decode");
        assert_eq!(generic, message, "generic decode for {variant}");
        if !matches!(
            &message.body,
            MessageBody::JoinRequest(_)
                | MessageBody::StateSnapshot(_)
                | MessageBody::StateSnapshotAck(_)
                | MessageBody::ReactivateSlot(_)
                | MessageBody::ReactivateSlotAck(_)
                | MessageBody::JoinCommitted(_)
                | MessageBody::JoinAborted(_)
        ) || cfg!(feature = "hot-join")
        {
            let (manual, consumed) =
                decode_message(expected).expect("enabled fixture must manually decode");
            assert_eq!(manual, message, "manual decode for {variant}");
            assert_eq!(consumed, expected.len(), "consumed bytes for {variant}");
        }
    }
}

#[cfg(not(feature = "hot-join"))]
#[test]
fn hot_join_v1_goldens_are_recognized_when_feature_is_disabled() {
    for (_, message) in fixtures().into_iter().filter(|(_, message)| {
        matches!(
            &message.body,
            MessageBody::JoinRequest(_)
                | MessageBody::StateSnapshot(_)
                | MessageBody::StateSnapshotAck(_)
                | MessageBody::ReactivateSlot(_)
                | MessageBody::ReactivateSlotAck(_)
                | MessageBody::JoinCommitted(_)
                | MessageBody::JoinAborted(_)
        )
    }) {
        let error = decode_message(expected(&message.body))
            .expect_err("disabled hot-join fixture must reject");
        assert!(error
            .to_string()
            .contains("requires the disabled hot-join feature"));
    }
}
