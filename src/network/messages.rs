use serde::{Deserialize, Serialize};

use crate::metrics::MessageKind;
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
    /// documented framing as the protocol packet-filter connection-ID era counter).
    pub epoch: u16,
}

impl ConnectionStatus {
    /// Exact number of bytes a `ConnectionStatus` serializes to under the crate's
    /// bincode configuration (little-endian, fixed-int): `disconnected` (`bool`, 1) +
    /// `last_frame` ([`Frame`] = `i32`, 4) + `epoch` (`u16`, 2). Kept honest by
    /// [`Message::encoded_len`]'s wire-exactness property test.
    pub(crate) const WIRE_LEN: usize = 1 + 4 + 2;
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
pub(crate) struct SessionConfigBlock {
    pub num_players: u16,
    pub input_bytes_per_player: u16,
    pub fps: u32,
    pub max_prediction: u16,
    /// `0` means [`crate::DesyncDetection::Off`].
    pub desync_interval: u32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) struct SyncRequest {
    pub random_request: u32, // please reply back with this random data
    pub min_compat_version: u8,
    pub features: u32,
    pub config: SessionConfigBlock,
    pub config_digest: u64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) struct SyncReply {
    pub random_reply: u32, // here's your random data back
    pub min_compat_version: u8,
    pub features: u32,
    pub config: SessionConfigBlock,
    pub config_digest: u64,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Input {
    pub peer_connect_status: Vec<ConnectionStatus>,
    pub start_frame: Frame,
    pub ack_frame: Frame,
    pub bytes: Vec<u8>,
}

impl Default for Input {
    fn default() -> Self {
        Self {
            peer_connect_status: Vec::new(),
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
            start_frame,
            ack_frame,
            bytes,
        } = self;

        f.debug_struct("Input")
            .field("peer_connect_status", peer_connect_status)
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
/// [`round_seq`](Self::round_seq) is a monotonic **per-request sequence
/// number** the observer bumps on every outgoing `FloorRequest`. The relay
/// echoes it verbatim in its [`FloorReply`], so the observer can drop a
/// reordered stale (or duplicate) reply — accepting only a `round_seq` strictly
/// newer than the latest accepted, and never one exceeding the latest request
/// it actually issued (an unsolicited/forged seq). Post-prune **freshness** is
/// tracked separately, observer-local: the reply must answer a request issued
/// AFTER the observer's most recent prune. No floor-epoch wire field is needed
/// (S54 machine-shows the seq + dedicated channel replace it).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) struct FloorRequest {
    /// The requesting observer's monotonic per-request sequence number, echoed
    /// verbatim in [`FloorReply`] (see the struct docs).
    pub round_seq: u32,
}

/// Relay → observer: a **floor-round reply** answering a [`FloorRequest`] with
/// the relay's current per-slot pessimistic floors (the double-failure-relay
/// connected-relay reorder fix; see [`FloorRequest`]).
///
/// [`round_seq`](Self::round_seq) echoes the request's value verbatim so the
/// observer can drop reordered stale / duplicate / unsolicited replies (accept
/// only a `round_seq` strictly newer than the latest accepted and not exceeding
/// the latest request issued) and, separately, decide whether the reply
/// postdates its most recent prune (an observer-local freshness check).
/// [`floors`](Self::floors) is the relay's `P2PSession::pessimistic_floors`
/// snapshot at reply time — per slot, the `min` over the relay's own
/// receipt/freeze and the committed DISCONNECTED freezes it still folds (a
/// departed global-min origin's low the observer has already pruned from its own
/// fold). Index = player handle, length `num_players` (the relay always reports
/// every slot). The observer accepts a reply ONLY when `floors` covers every
/// slot; a short vector is malformed and dropped, so an accepted reply fully
/// (re)defines the observer's cached floors with no slot left reading a stale
/// prior round (excess trailing entries are ignored). A reported `Frame::NULL`
/// slot is read as "fall back to `last_frame`" and never confirms higher than
/// the legacy fold.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) struct FloorReply {
    /// The originating [`FloorRequest::round_seq`], echoed verbatim.
    pub round_seq: u32,
    /// The relay's per-slot pessimistic floors at reply time (handle-indexed).
    pub floors: Vec<Frame>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MessageHeader {
    pub sentinel: [u8; 2],
    pub protocol_version: u8,
    pub flags: u8,
    pub conn_id: u32,
}

impl MessageHeader {
    pub(crate) const fn new(conn_id: u32) -> Self {
        Self {
            sentinel: super::WIRE_SENTINEL,
            protocol_version: crate::PROTOCOL_VERSION,
            flags: 0,
            conn_id,
        }
    }
}

impl Default for MessageHeader {
    fn default() -> Self {
        Self::new(1)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct JoinRequest {
    /// The player handle (slot index) the joiner wants to occupy.
    pub player_handle: usize,
}
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
    /// order (S34 fix round 1). Protocol v2 normalizes each epoch to the
    /// coordinator's canonical connected-era membership generation (or that
    /// generation plus one for a carried-disconnected slot); flags and frame
    /// values retain the coordinator's local status captured with the
    /// snapshot. The joiner derives each slot's
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StateSnapshotAck {
    /// The frame the joiner successfully loaded.
    pub frame: Frame,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ReactivateSlot {
    /// The player handle (slot index) the survivor should reopen.
    pub handle: usize,
    /// The activation frame at which the slot becomes live again.
    pub frame: Frame,
}
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct JoinAborted {
    /// The player handle (slot index) whose join aborted.
    pub handle: usize,
    /// The activation frame `F` of the aborted attempt.
    pub frame: Frame,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) struct Goodbye {
    pub reason: u8,
}

/// Stable identity of one coordinated graceful-drop attempt.
#[derive(
    Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub(crate) struct DropOperationId {
    /// Smallest player handle owned by the coordinating endpoint.
    pub coordinator: u16,
    /// Connected-era generation of `coordinator` when the attempt opened.
    pub coordinator_generation: u16,
    /// Coordinator-local monotonic operation sequence.
    pub sequence: u32,
    /// Digest of the canonical sorted [`DropTarget`] list.
    pub target_set_digest: u64,
}

/// One player slot owned by the endpoint being gracefully dropped.
#[derive(
    Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub(crate) struct DropTarget {
    /// Player handle of the target slot.
    pub handle: u16,
    /// Connected-era generation the operation is fenced to.
    pub generation: u16,
}

/// Opens a coordinated graceful-drop attempt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) struct DropPrepare {
    pub operation: DropOperationId,
    /// Canonical sorted target slots belonging to the departing endpoint.
    pub targets: Vec<DropTarget>,
    /// Canonical sorted survivor endpoint representatives.
    pub participants: Vec<u16>,
}

/// Phase represented by a [`DropReport`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) enum DropReportStage {
    /// Initial exposed-confirmation and retained-history inventory.
    #[default]
    Inventory,
    /// The participant has verified the selected cut and its digest.
    Ready,
    /// The participant has applied the commit idempotently.
    Committed,
}

/// Contiguous retained input range for one target slot.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DropReceipt {
    pub target: u16,
    pub available_from: Frame,
    pub contiguous_through: Frame,
}

impl Default for DropReceipt {
    fn default() -> Self {
        Self {
            target: 0,
            available_from: Frame::NULL,
            contiguous_through: Frame::NULL,
        }
    }
}

/// Participant inventory, ready acknowledgement, or committed acknowledgement.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DropReport {
    pub operation: DropOperationId,
    pub participant: u16,
    pub stage: DropReportStage,
    /// Highest frame this participant has exposed as confirmed.
    pub exposed_confirmed: Frame,
    /// Selected non-retracting cut for ready/committed reports.
    pub cut: Frame,
    /// Digest of every target input at `cut` for ready/committed reports.
    pub cut_digest: u64,
    /// Per-target retained ranges for inventory reports.
    pub receipts: Vec<DropReceipt>,
}

/// One bounded chunk of target input history used to close receipt gaps.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DropBackfill {
    pub operation: DropOperationId,
    pub chunk_index: u16,
    pub chunk_count: u16,
    pub start_frame: Frame,
    pub frame_count: u16,
    /// Fixed-width target inputs in frame-major, canonical-target order.
    pub bytes: Vec<u8>,
}

/// Irrevocable coordinated graceful-drop decision.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DropCommit {
    pub operation: DropOperationId,
    pub cut: Frame,
    pub cut_digest: u64,
}

/// Why a coordinated graceful-drop attempt closed without committing.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum DropAbortReason {
    /// A deterministically higher-priority concurrent operation won.
    Superseded,
    /// No participant retained enough target history to form a safe cut.
    MissingHistory,
    /// Two retained copies disagreed on a target input.
    ConflictingHistory,
    /// A declared survivor disappeared before the operation completed.
    ParticipantLost,
    /// The bounded operation deadline elapsed.
    Timeout,
    /// A target or participant generation changed during the operation.
    GenerationChanged,
    /// A configured allocation or protocol bound would be exceeded.
    ResourceLimit,
}

/// Closes an uncommitted coordinated graceful-drop attempt.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DropAbort {
    pub operation: DropOperationId,
    pub reason: DropAbortReason,
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
    // discriminants (0..=7) stay stable. The wire vocabulary below is compiled
    // in every feature build so serde's positional tags never shift; the
    // hand-written `codec::decode_message` mirrors this numbering.
    FloorRequest(FloorRequest),
    FloorReply(FloorReply),
    JoinRequest(JoinRequest),
    StateSnapshot(StateSnapshot),
    StateSnapshotAck(StateSnapshotAck),
    ReactivateSlot(ReactivateSlot),
    ReactivateSlotAck(ReactivateSlotAck),
    JoinCommitted(JoinCommitted),
    JoinAborted(JoinAborted),
    // Protocol-v1 tag 17.
    Goodbye(Goodbye),
    // D14 coordinated graceful-drop barrier, tags 18..=22. These stay in every
    // feature build so the v1 discriminants remain feature-independent.
    DropPrepare(DropPrepare),
    DropReport(DropReport),
    DropBackfill(DropBackfill),
    DropCommit(DropCommit),
    DropAbort(DropAbort),
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

impl MessageBody {
    /// The exact number of bytes this body serializes to under the crate's bincode
    /// configuration (little-endian, fixed-int), computed arithmetically without
    /// allocating or serializing.
    ///
    /// Wire-exactness (`encoded_len == codec::encode(..).len()`) is asserted for
    /// every variant by [`Message::encoded_len`]'s property test, so this stays
    /// honest if the codec configuration or any field ever changes. The `match`
    /// is wildcard-free: adding a `MessageBody` variant fails to compile until it
    /// is accounted here.
    pub(crate) fn encoded_len(&self) -> usize {
        // Fixed bincode widths under `standard().with_little_endian().with_fixed_int_encoding()`.
        const DISCRIMINANT: usize = 4; // enum variant tag (u32)
        const FRAME: usize = 4; // Frame(i32)
        const LEN_PREFIX: usize = 8; // collection length (usize -> u64 with fixed-int)

        let payload = match self {
            Self::SyncRequest(_) | Self::SyncReply(_) => {
                4 // random token: u32
                    + 1 // min_compat_version: u8
                    + 4 // features: u32
                    + 14 // SessionConfigBlock
                    + 8 // config_digest: u64
            },
            Self::Input(input) => {
                LEN_PREFIX
                    + input.peer_connect_status.len() * ConnectionStatus::WIRE_LEN
                    + FRAME // start_frame
                    + FRAME // ack_frame
                    + LEN_PREFIX
                    + input.bytes.len() // bytes: Vec<u8>
            },
            Self::InputAck(_) => FRAME,            // ack_frame
            Self::QualityReport(_) => 2 + 16,      // frame_advantage: i16, ping: u128
            Self::QualityReply(_) => 16,           // pong: u128
            Self::ChecksumReport(_) => 16 + FRAME, // checksum: u128, frame
            Self::KeepAlive => 0,
            Self::FloorRequest(_) => 4, // round_seq: u32
            Self::FloorReply(reply) => {
                4 // round_seq: u32
                    + LEN_PREFIX
                    + reply.floors.len() * FRAME // floors: Vec<Frame>
            },
            Self::JoinRequest(_) => 8, // player_handle: usize
            Self::StateSnapshot(snapshot) => {
                FRAME // frame
                    + 8 // num_players: usize
                    + LEN_PREFIX
                    + snapshot.state_bytes.len()
                    + LEN_PREFIX
                    + snapshot.bridge_inputs.len()
                    + LEN_PREFIX
                    + snapshot.bridge_statuses.len() * ConnectionStatus::WIRE_LEN
                    + 1 // Option tag
                    + snapshot.checksum.map_or(0, |_| 16) // Some(u128)
            },
            Self::StateSnapshotAck(_) => FRAME,      // frame
            Self::ReactivateSlot(_) => 8 + FRAME,    // handle: usize, frame
            Self::ReactivateSlotAck(_) => 8 + FRAME, // handle: usize, frame
            Self::JoinCommitted(_) => 8 + FRAME,     // handle: usize, frame
            Self::JoinAborted(_) => 8 + FRAME,       // handle: usize, frame
            Self::Goodbye(_) => 1,                   // reason: u8
            Self::DropPrepare(prepare) => {
                16 // DropOperationId
                    + LEN_PREFIX
                    + prepare.targets.len() * 4 // DropTarget
                    + LEN_PREFIX
                    + prepare.participants.len() * 2 // u16 handles
            },
            Self::DropReport(report) => {
                16 // DropOperationId
                    + 2 // participant
                    + 4 // DropReportStage discriminant
                    + FRAME // exposed_confirmed
                    + FRAME // cut
                    + 8 // cut_digest
                    + LEN_PREFIX
                    + report.receipts.len() * (2 + FRAME + FRAME)
            },
            Self::DropBackfill(backfill) => {
                16 // DropOperationId
                    + 2 // chunk_index
                    + 2 // chunk_count
                    + FRAME // start_frame
                    + 2 // frame_count
                    + LEN_PREFIX
                    + backfill.bytes.len()
            },
            Self::DropCommit(_) => 16 + FRAME + 8,
            Self::DropAbort(_) => 16 + 4, // operation + DropAbortReason discriminant
        };

        DISCRIMINANT + payload
    }

    /// The payload-independent [`MessageKind`] category of this body.
    ///
    /// The `match` is wildcard-free: adding a `MessageBody` variant fails to
    /// compile until it is categorized here (and, in lockstep, in
    /// [`MessageKind`]).
    pub(crate) fn kind(&self) -> MessageKind {
        match self {
            Self::SyncRequest(_) => MessageKind::SyncRequest,
            Self::SyncReply(_) => MessageKind::SyncReply,
            Self::Input(_) => MessageKind::Input,
            Self::InputAck(_) => MessageKind::InputAck,
            Self::QualityReport(_) => MessageKind::QualityReport,
            Self::QualityReply(_) => MessageKind::QualityReply,
            Self::ChecksumReport(_) => MessageKind::ChecksumReport,
            Self::KeepAlive => MessageKind::KeepAlive,
            Self::FloorRequest(_) => MessageKind::FloorRequest,
            Self::FloorReply(_) => MessageKind::FloorReply,
            Self::JoinRequest(_) => MessageKind::JoinRequest,
            Self::StateSnapshot(_) => MessageKind::StateSnapshot,
            Self::StateSnapshotAck(_) => MessageKind::StateSnapshotAck,
            Self::ReactivateSlot(_) => MessageKind::ReactivateSlot,
            Self::ReactivateSlotAck(_) => MessageKind::ReactivateSlotAck,
            Self::JoinCommitted(_) => MessageKind::JoinCommitted,
            Self::JoinAborted(_) => MessageKind::JoinAborted,
            Self::Goodbye(_) => MessageKind::Goodbye,
            Self::DropPrepare(_) => MessageKind::DropPrepare,
            Self::DropReport(_) => MessageKind::DropReport,
            Self::DropBackfill(_) => MessageKind::DropBackfill,
            Self::DropCommit(_) => MessageKind::DropCommit,
            Self::DropAbort(_) => MessageKind::DropAbort,
        }
    }
}

impl Message {
    /// The exact number of bytes this message serializes to on the wire under the
    /// crate's bincode configuration: the 8-byte [`MessageHeader`] plus the
    /// [`MessageBody`] ([`MessageBody::encoded_len`]).
    ///
    /// This is the true payload size a [`NonBlockingSocket`](crate::NonBlockingSocket)
    /// transmits, used for bandwidth accounting. It is computed arithmetically
    /// (alloc-free) and kept wire-exact by a property test against
    /// [`codec::encode`](crate::network::codec::encode).
    pub(crate) fn encoded_len(&self) -> usize {
        const HEADER: usize = 8;
        HEADER + self.body.encoded_len()
    }

    /// The [`MessageKind`] category of this message's body.
    pub(crate) fn kind(&self) -> MessageKind {
        self.body.kind()
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
        assert_eq!(input.start_frame, Frame::NULL);
        assert_eq!(input.ack_frame, Frame::NULL);
        assert!(input.bytes.is_empty());
    }

    #[test]
    fn test_input_debug() {
        let input = Input {
            peer_connect_status: vec![ConnectionStatus::default()],
            start_frame: Frame::new(10),
            ack_frame: Frame::new(5),
            bytes: vec![0xDE, 0xAD, 0xBE, 0xEF],
        };
        let debug = format!("{:?}", input);
        assert!(debug.contains("Input"));
        assert!(debug.contains("start_frame"));
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
        assert_eq!(header.sentinel, super::super::WIRE_SENTINEL);
        assert_eq!(header.protocol_version, crate::PROTOCOL_VERSION);
        assert_eq!(header.flags, 0);
        assert_eq!(header.conn_id, 1);
    }

    #[test]
    fn test_message_body_variants() {
        // Test each variant can be created and compared
        let sync_req = MessageBody::SyncRequest(SyncRequest {
            random_request: 42,
            ..SyncRequest::default()
        });
        let sync_req2 = MessageBody::SyncRequest(SyncRequest {
            random_request: 42,
            ..SyncRequest::default()
        });
        assert_eq!(sync_req, sync_req2);

        let sync_reply = MessageBody::SyncReply(SyncReply {
            random_reply: 123,
            ..SyncReply::default()
        });
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
            header: MessageHeader::new(0x1234),
            body: MessageBody::FloorRequest(FloorRequest { round_seq: 9 }),
        };
        let serialized = codec::encode(&request).expect("serialization should succeed");
        let (deserialized, _): (Message, _) =
            codec::decode(&serialized).expect("deserialization should succeed");
        assert_eq!(request, deserialized);

        let reply = Message {
            header: MessageHeader::new(0x1234),
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
            header: MessageHeader::new(0x1234),
            body: MessageBody::KeepAlive,
        };
        let cloned = msg.clone();
        assert_eq!(msg, cloned);
    }

    #[test]
    fn test_message_serialization() {
        use crate::network::codec;

        let msg = Message {
            header: MessageHeader::new(0xABCD),
            body: MessageBody::SyncRequest(SyncRequest {
                random_request: 999,
                ..SyncRequest::default()
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
    fn input_wire_layout_has_no_disconnect_flag() {
        let message = Message {
            header: MessageHeader::new(1),
            body: MessageBody::Input(Input::default()),
        };

        assert_eq!(
            message.encoded_len(),
            36,
            "v1 Input is header(8) + tag(4) + status-len(8) + start(4) + ack(4) + bytes-len(8)"
        );
    }

    #[test]
    fn test_bytes_debug_empty() {
        let input = Input {
            peer_connect_status: vec![],
            start_frame: Frame::NULL,
            ack_frame: Frame::NULL,
            bytes: vec![],
        };
        let debug = format!("{:?}", input);
        assert!(debug.contains("0x")); // Empty bytes should still show "0x" prefix
    }

    #[test]
    fn message_body_kind_maps_every_variant() {
        let cases: &[(MessageBody, MessageKind)] = &[
            (
                MessageBody::SyncRequest(SyncRequest::default()),
                MessageKind::SyncRequest,
            ),
            (
                MessageBody::SyncReply(SyncReply::default()),
                MessageKind::SyncReply,
            ),
            (MessageBody::Input(Input::default()), MessageKind::Input),
            (
                MessageBody::InputAck(InputAck::default()),
                MessageKind::InputAck,
            ),
            (
                MessageBody::QualityReport(QualityReport::default()),
                MessageKind::QualityReport,
            ),
            (
                MessageBody::QualityReply(QualityReply::default()),
                MessageKind::QualityReply,
            ),
            (
                MessageBody::ChecksumReport(ChecksumReport::default()),
                MessageKind::ChecksumReport,
            ),
            (MessageBody::KeepAlive, MessageKind::KeepAlive),
            (
                MessageBody::FloorRequest(FloorRequest::default()),
                MessageKind::FloorRequest,
            ),
            (
                MessageBody::FloorReply(FloorReply::default()),
                MessageKind::FloorReply,
            ),
            (
                MessageBody::Goodbye(Goodbye::default()),
                MessageKind::Goodbye,
            ),
            (
                MessageBody::DropPrepare(DropPrepare::default()),
                MessageKind::DropPrepare,
            ),
            (
                MessageBody::DropReport(DropReport {
                    operation: DropOperationId::default(),
                    participant: 0,
                    stage: DropReportStage::Inventory,
                    exposed_confirmed: Frame::NULL,
                    cut: Frame::NULL,
                    cut_digest: 0,
                    receipts: Vec::new(),
                }),
                MessageKind::DropReport,
            ),
            (
                MessageBody::DropBackfill(DropBackfill {
                    operation: DropOperationId::default(),
                    chunk_index: 0,
                    chunk_count: 1,
                    start_frame: Frame::NULL,
                    frame_count: 0,
                    bytes: Vec::new(),
                }),
                MessageKind::DropBackfill,
            ),
            (
                MessageBody::DropCommit(DropCommit {
                    operation: DropOperationId::default(),
                    cut: Frame::NULL,
                    cut_digest: 0,
                }),
                MessageKind::DropCommit,
            ),
            (
                MessageBody::DropAbort(DropAbort {
                    operation: DropOperationId::default(),
                    reason: DropAbortReason::Superseded,
                }),
                MessageKind::DropAbort,
            ),
        ];
        for (body, expected) in cases {
            assert_eq!(body.kind(), *expected, "body.kind() for {body:?}");
            // Message::kind() delegates to its body.
            let msg = Message {
                header: MessageHeader::default(),
                body: body.clone(),
            };
            assert_eq!(msg.kind(), *expected, "Message::kind() for {body:?}");
        }
        {
            let hot_cases: &[(MessageBody, MessageKind)] = &[
                (
                    MessageBody::JoinRequest(JoinRequest { player_handle: 0 }),
                    MessageKind::JoinRequest,
                ),
                (
                    MessageBody::StateSnapshot(StateSnapshot {
                        frame: Frame::NULL,
                        num_players: 2,
                        state_bytes: vec![],
                        bridge_inputs: vec![],
                        bridge_statuses: vec![],
                        checksum: None,
                    }),
                    MessageKind::StateSnapshot,
                ),
                (
                    MessageBody::StateSnapshotAck(StateSnapshotAck { frame: Frame::NULL }),
                    MessageKind::StateSnapshotAck,
                ),
                (
                    MessageBody::ReactivateSlot(ReactivateSlot {
                        handle: 0,
                        frame: Frame::NULL,
                    }),
                    MessageKind::ReactivateSlot,
                ),
                (
                    MessageBody::ReactivateSlotAck(ReactivateSlotAck {
                        handle: 0,
                        frame: Frame::NULL,
                    }),
                    MessageKind::ReactivateSlotAck,
                ),
                (
                    MessageBody::JoinCommitted(JoinCommitted {
                        handle: 0,
                        frame: Frame::NULL,
                    }),
                    MessageKind::JoinCommitted,
                ),
                (
                    MessageBody::JoinAborted(JoinAborted {
                        handle: 0,
                        frame: Frame::NULL,
                    }),
                    MessageKind::JoinAborted,
                ),
            ];
            for (body, expected) in hot_cases {
                assert_eq!(body.kind(), *expected, "body.kind() for {body:?}");
            }
        }
    }
}
