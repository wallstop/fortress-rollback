//! Bounded raw events for runtime refinement of `SyncHandshakeV1.tla`.
//!
//! This module is compiled only for the opt-in `trace-validation` feature. The
//! recorder reserves its complete logical capacity before activation and never
//! allocates while the protocol is running. Its records deliberately retain
//! raw request IDs and the complete runtime handshake block; normalization to
//! bounded TLA+ tokens and projection to modeled fields belongs in the driver.

use super::{HandshakeConfig, ProtocolState};
use crate::{FortressError, IncompatibleSessionReason};

/// Complete runtime handshake configuration captured at a trace transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandshakeTraceConfig {
    /// Minimum compatible wire-protocol version.
    pub min_compat_version: u8,
    /// Enabled wire feature bits.
    pub features: u32,
    /// Configured player count.
    pub num_players: u16,
    /// Fixed serialized input width for one player.
    pub input_bytes_per_player: u16,
    /// Simulation rate.
    pub fps: u32,
    /// Maximum prediction window.
    pub max_prediction: u16,
    /// Checksum interval, or zero when disabled.
    pub desync_interval: u32,
    /// Canonical digest over the complete compatibility block.
    pub config_digest: u64,
}

impl From<HandshakeConfig> for HandshakeTraceConfig {
    fn from(value: HandshakeConfig) -> Self {
        Self {
            min_compat_version: value.min_compat_version,
            features: value.features,
            num_players: value.config.num_players,
            input_bytes_per_player: value.config.input_bytes_per_player,
            fps: value.config.fps,
            max_prediction: value.config.max_prediction,
            desync_interval: value.config.desync_interval,
            config_digest: value.config_digest,
        }
    }
}

/// Why a handled synchronization request did or did not update local state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakeRequestDisposition {
    /// The request was answered and its matching configuration was observed.
    Observed,
    /// The request was answered and its incompatible configuration failed the handshake.
    Incompatible,
    /// The request was answered after an earlier incompatibility became terminal.
    AlreadyIncompatible,
    /// The request was answered outside the local synchronizing phase.
    AnsweredOnly,
}

/// Whether an emitted raw request ID was fresh in the live outstanding set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakeRequestIdDisposition {
    /// The ID was newly inserted into the outstanding set.
    Fresh,
    /// The RNG repeated an ID that was already outstanding.
    Collision,
}

/// Why a handled synchronization reply did or did not update local state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakeReplyDisposition {
    /// A live request ID was consumed and synchronization advanced.
    Accepted,
    /// A live request ID was consumed, but the configuration was incompatible.
    Incompatible,
    /// The ID belonged to a reply already accepted by this trace.
    Duplicate,
    /// The ID was never emitted, or is no longer represented in the bounded trace.
    Unknown,
    /// The endpoint was no longer synchronizing.
    NotSynchronizing,
    /// A prior incompatibility had already made the handshake terminal.
    AlreadyIncompatible,
    /// A prior live-send collision makes this raw ID's logical request ambiguous.
    AmbiguousRequestIdCollision,
}

/// One raw protocol transition relevant to `SyncHandshakeV1.tla`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakeTraceAction {
    /// Recorder activation while the endpoint is still `Initializing`.
    Activated,
    /// The endpoint entered `Synchronizing`, before its first request was queued.
    BeginSynchronization,
    /// A request with this raw random ID was queued.
    SendRequest {
        request_id: u32,
        disposition: HandshakeRequestIdDisposition,
    },
    /// An allowed request was answered and optionally observed.
    HandleRequest {
        request_id: u32,
        disposition: HandshakeRequestDisposition,
    },
    /// An allowed reply was classified by the synchronization handler.
    HandleReply {
        request_id: u32,
        disposition: HandshakeReplyDisposition,
    },
    /// The one-shot timeout event was emitted.
    ReportTimeout { elapsed_ms: u128 },
}

/// A fixed-size post-transition snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandshakeTraceEvent {
    /// Transition that produced this post-state.
    pub action: HandshakeTraceAction,
    /// Runtime protocol phase after the transition.
    pub state: ProtocolState,
    /// Complete local compatibility block.
    pub local_config: HandshakeTraceConfig,
    /// Complete remote block observed by a handler, when applicable.
    pub remote_config: Option<HandshakeTraceConfig>,
    /// Successful synchronization roundtrips still required.
    pub sync_remaining_roundtrips: u32,
    /// Raw outstanding-request set cardinality after the transition.
    pub outstanding_request_count: usize,
    /// Bound remote connection ID, or zero before binding.
    pub remote_conn_id: u32,
    /// Whether the one-shot timeout event has been emitted.
    pub timeout_event_sent: bool,
    /// Sticky configuration incompatibility, when one has been observed.
    pub incompatibility: Option<IncompatibleSessionReason>,
}

/// Terminal failure produced when the fixed trace capacity is exhausted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandshakeTraceOverflow {
    /// Configured logical record capacity.
    pub capacity: usize,
    /// Records retained before the first overflow attempt.
    pub recorded: usize,
}

/// Fixed-capacity, allocation-free-after-construction event recorder.
#[derive(Debug)]
pub(super) struct HandshakeTraceRecorder {
    events: Vec<HandshakeTraceEvent>,
    capacity: usize,
    overflowed: bool,
}

impl HandshakeTraceRecorder {
    pub(super) fn try_new(capacity: usize) -> Result<Self, FortressError> {
        let mut events = Vec::new();
        events.try_reserve_exact(capacity).map_err(|_error| {
            crate::error::allocation_failed("protocol.handshake_trace", capacity)
        })?;
        Ok(Self {
            events,
            capacity,
            overflowed: false,
        })
    }

    pub(super) fn record(&mut self, event: HandshakeTraceEvent) {
        if self.overflowed {
            return;
        }
        if self.events.len() >= self.capacity {
            self.overflowed = true;
            return;
        }
        self.events.push(event);
    }

    pub(super) fn result(&self) -> Result<&[HandshakeTraceEvent], HandshakeTraceOverflow> {
        if self.overflowed {
            return Err(HandshakeTraceOverflow {
                capacity: self.capacity,
                recorded: self.events.len(),
            });
        }
        Ok(&self.events)
    }

    pub(super) fn accepted_reply_was_recorded(&self, request_id: u32) -> bool {
        self.events.iter().any(|event| {
            matches!(
                event.action,
                HandshakeTraceAction::HandleReply {
                    request_id: recorded_id,
                    disposition: HandshakeReplyDisposition::Accepted,
                } if recorded_id == request_id
            )
        })
    }

    pub(super) fn request_id_collision_was_recorded(&self, request_id: u32) -> bool {
        self.events.iter().any(|event| {
            matches!(
                event.action,
                HandshakeTraceAction::SendRequest {
                    request_id: recorded_id,
                    disposition: HandshakeRequestIdDisposition::Collision,
                } if recorded_id == request_id
            )
        })
    }
}
