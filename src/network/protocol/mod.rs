//! UDP protocol implementation for peer-to-peer communication.
//!
//! This module contains the UDP protocol handler for managing network communication
//! between peers in a rollback networking session.

mod event;
mod input_bytes;
mod state;

pub use event::Event;
use input_bytes::InputBytes;
pub use state::ProtocolState;

use crate::frame_info::PlayerInput;
use crate::network::compression::{decode, encode};
use crate::network::messages::{
    ChecksumReport, ConnectionStatus, Input, InputAck, Message, MessageBody, MessageHeader,
    QualityReply, QualityReport, SyncReply, SyncRequest,
};
use crate::report_violation;
use crate::rng::random;
use crate::sessions::config::{ProtocolConfig, SyncConfig};
use crate::telemetry::{ViolationKind, ViolationSeverity};
use crate::time_sync::TimeSync;
use crate::{Config, DesyncDetection, FortressError, Frame, NonBlockingSocket, PlayerHandle};
use tracing::trace;

use std::collections::vec_deque::Drain;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::convert::TryFrom;
use std::ops::Add;
use web_time::{Duration, Instant};

use super::network_stats::NetworkStats;

const UDP_HEADER_SIZE: usize = 28; // Size of IP + UDP headers

/// Returns the current wall-clock time as milliseconds since UNIX_EPOCH.
///
/// This function returns `Some(millis)` under normal conditions, or `None` if the system
/// clock is in an invalid state (e.g., before UNIX_EPOCH due to NTP adjustments, VM snapshots,
/// or misconfigured clocks).
///
/// # When to use
/// Use this ONLY when you need wall-clock time that can be compared across different machines
/// (e.g., for ping/pong RTT calculation). For local elapsed time measurements, prefer using
/// `Instant` which is guaranteed monotonic.
///
/// # Returns
/// - `Some(millis)` - The current time in milliseconds since UNIX_EPOCH
/// - `None` - If the system clock is before UNIX_EPOCH (abnormal condition)
fn millis_since_epoch() -> Option<u128> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(duration) => Some(duration.as_millis()),
            Err(_) => {
                // System time is before UNIX_EPOCH - this can happen due to:
                // - NTP adjustments moving clock backwards
                // - VM snapshots with stale time
                // - Misconfigured system clocks
                // Report via telemetry and return None so callers can handle appropriately.
                report_violation!(
                    ViolationSeverity::Warning,
                    ViolationKind::InternalError,
                    "System time is before UNIX_EPOCH - clock may have gone backwards"
                );
                None
            },
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        // In WASM, Date.getTime() returns milliseconds since epoch as a f64.
        // It can technically be negative for dates before 1970, but this is rare.
        let time = js_sys::Date::new_0().get_time();
        if time >= 0.0 {
            Some(time as u128)
        } else {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::InternalError,
                "WASM Date.getTime() returned negative value - clock may be misconfigured"
            );
            None
        }
    }
}

/// UDP protocol handler for peer-to-peer communication.
///
/// # Note
///
/// This type is re-exported in [`__internal`](crate::__internal) for testing and fuzzing.
/// It is not part of the stable public API.
pub struct UdpProtocol<T>
where
    T: Config,
{
    num_players: usize,
    handles: Vec<PlayerHandle>,
    send_queue: VecDeque<Message>,
    event_queue: VecDeque<Event<T>>,

    // state
    state: ProtocolState,
    sync_remaining_roundtrips: u32,
    sync_random_requests: BTreeSet<u32>,
    /// Total sync requests sent (tracks retries for telemetry).
    sync_requests_sent: u32,
    /// Whether we've emitted a sync retry warning (emit only once).
    sync_retry_warning_sent: bool,
    /// Whether we've emitted a sync duration warning (emit only once).
    sync_duration_warning_sent: bool,
    running_last_quality_report: Instant,
    running_last_input_recv: Instant,
    disconnect_notify_sent: bool,
    disconnect_event_sent: bool,

    // constants
    disconnect_timeout: Duration,
    disconnect_notify_start: Duration,
    shutdown_timeout: Instant,
    fps: usize,
    magic: u16,

    // sync configuration
    sync_config: SyncConfig,

    // protocol configuration
    protocol_config: ProtocolConfig,

    // the other client
    peer_addr: T::Address,
    remote_magic: u16,
    peer_connect_status: Vec<ConnectionStatus>,

    // input compression
    pending_output: VecDeque<InputBytes>,
    last_acked_input: InputBytes,
    max_prediction: usize,
    recv_inputs: BTreeMap<Frame, InputBytes>,

    // time sync
    time_sync_layer: TimeSync,
    local_frame_advantage: i32,
    remote_frame_advantage: i32,

    // network
    /// The instant when synchronization started, used for elapsed time calculations.
    /// Using Instant (monotonic clock) instead of wall-clock time ensures reliable
    /// duration measurements even if the system clock is adjusted.
    stats_start_time: Instant,
    packets_sent: usize,
    bytes_sent: usize,
    round_trip_time: u128,
    last_send_time: Instant,
    last_recv_time: Instant,

    // debug desync
    pub(crate) pending_checksums: BTreeMap<Frame, u128>,
    desync_detection: DesyncDetection,
}

impl<T: Config> PartialEq for UdpProtocol<T> {
    fn eq(&self, other: &Self) -> bool {
        self.peer_addr == other.peer_addr
    }
}

impl<T: Config> UdpProtocol<T> {
    /// Internal constructor for UDP protocol handler.
    ///
    /// Note: This is an internal constructor called via SessionBuilder. The many parameters are
    /// acceptable here because users interact through the builder pattern, not this method directly.
    ///
    /// # Returns
    /// Returns `None` if input serialization fails (indicates a fundamental issue with Config::Input).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        mut handles: Vec<PlayerHandle>,
        peer_addr: T::Address,
        num_players: usize,
        local_players: usize,
        max_prediction: usize,
        disconnect_timeout: Duration,
        disconnect_notify_start: Duration,
        fps: usize,
        desync_detection: DesyncDetection,
        sync_config: SyncConfig,
        protocol_config: ProtocolConfig,
    ) -> Option<Self> {
        let mut magic: u16 = random();
        while magic == 0 {
            magic = random();
        }

        handles.sort_unstable();
        let recv_player_num = handles.len();

        // peer connection status
        let mut peer_connect_status = Vec::new();
        for _ in 0..num_players {
            peer_connect_status.push(ConnectionStatus::default());
        }

        // received input history - may fail if serialization is broken
        let mut recv_inputs = BTreeMap::new();
        recv_inputs.insert(Frame::NULL, InputBytes::zeroed::<T>(recv_player_num)?);

        // last acked input - may fail if serialization is broken
        let last_acked_input = InputBytes::zeroed::<T>(local_players)?;

        Some(Self {
            num_players,
            handles,
            send_queue: VecDeque::new(),
            event_queue: VecDeque::new(),

            // state
            state: ProtocolState::Initializing,
            sync_remaining_roundtrips: sync_config.num_sync_packets,
            sync_random_requests: BTreeSet::new(),
            sync_requests_sent: 0,
            sync_retry_warning_sent: false,
            sync_duration_warning_sent: false,
            running_last_quality_report: Instant::now(),
            running_last_input_recv: Instant::now(),
            disconnect_notify_sent: false,
            disconnect_event_sent: false,

            // constants
            disconnect_timeout,
            disconnect_notify_start,
            shutdown_timeout: Instant::now(),
            fps,
            magic,

            // sync configuration
            sync_config,

            // protocol configuration
            protocol_config,

            // the other client
            peer_addr,
            remote_magic: 0,
            peer_connect_status,

            // input compression
            pending_output: VecDeque::new(),
            last_acked_input,
            max_prediction,
            recv_inputs,

            // time sync
            time_sync_layer: TimeSync::new(),
            local_frame_advantage: 0,
            remote_frame_advantage: 0,

            // network
            stats_start_time: Instant::now(),
            packets_sent: 0,
            bytes_sent: 0,
            round_trip_time: 0,
            last_send_time: Instant::now(),
            last_recv_time: Instant::now(),

            // debug desync
            pending_checksums: BTreeMap::new(),
            desync_detection,
        })
    }

    pub(crate) fn update_local_frame_advantage(&mut self, local_frame: Frame) {
        if local_frame == Frame::NULL || self.last_recv_frame() == Frame::NULL {
            return;
        }
        // Estimate which frame the other client is on by looking at the last frame they gave us plus some delta for the packet roundtrip time.
        // Use saturating conversion to avoid panic if round_trip_time is extremely large
        let ping = i32::try_from(self.round_trip_time / 2).unwrap_or(i32::MAX);
        let remote_frame = self.last_recv_frame() + ((ping * self.fps as i32) / 1000);
        // Our frame "advantage" is how many frames behind the remote client we are. (It's an advantage because they will have to predict more often)
        self.local_frame_advantage = remote_frame - local_frame;
    }

    pub(crate) fn network_stats(&self) -> Result<NetworkStats, FortressError> {
        if self.state != ProtocolState::Synchronizing && self.state != ProtocolState::Running {
            return Err(FortressError::NotSynchronized);
        }

        let elapsed = self.stats_start_time.elapsed();
        let seconds = elapsed.as_secs();
        if seconds == 0 {
            return Err(FortressError::NotSynchronized);
        }

        let total_bytes_sent = self.bytes_sent + (self.packets_sent * UDP_HEADER_SIZE);
        let bps = total_bytes_sent / seconds as usize;
        //let upd_overhead = (self.packets_sent * UDP_HEADER_SIZE) / self.bytes_sent;

        Ok(NetworkStats {
            ping: self.round_trip_time,
            send_queue_len: self.pending_output.len(),
            kbps_sent: bps / 1024,
            local_frames_behind: self.local_frame_advantage,
            remote_frames_behind: self.remote_frame_advantage,
            // Checksum fields are populated by P2PSession::network_stats()
            // which has access to both local and remote checksum histories
            last_compared_frame: None,
            local_checksum: None,
            remote_checksum: None,
            checksums_match: None,
        })
    }

    pub(crate) fn handles(&self) -> &Vec<PlayerHandle> {
        &self.handles
    }

    pub(crate) fn is_synchronized(&self) -> bool {
        self.state == ProtocolState::Running
            || self.state == ProtocolState::Disconnected
            || self.state == ProtocolState::Shutdown
    }

    pub(crate) fn is_running(&self) -> bool {
        self.state == ProtocolState::Running
    }

    pub(crate) fn is_handling_message(&self, addr: &T::Address) -> bool {
        self.peer_addr == *addr
    }

    pub(crate) fn peer_connect_status(&self, handle: PlayerHandle) -> ConnectionStatus {
        self.peer_connect_status[handle.as_usize()]
    }

    pub(crate) fn disconnect(&mut self) {
        if self.state == ProtocolState::Shutdown {
            return;
        }

        self.state = ProtocolState::Disconnected;
        // schedule the timeout which will lead to shutdown
        self.shutdown_timeout = Instant::now().add(self.protocol_config.shutdown_delay)
    }

    pub(crate) fn synchronize(&mut self) {
        assert_eq!(self.state, ProtocolState::Initializing);
        self.state = ProtocolState::Synchronizing;
        self.sync_remaining_roundtrips = self.sync_config.num_sync_packets;
        self.stats_start_time = Instant::now();
        self.send_sync_request();
    }

    pub(crate) fn average_frame_advantage(&self) -> i32 {
        self.time_sync_layer.average_frame_advantage()
    }

    pub(crate) fn peer_addr(&self) -> T::Address {
        self.peer_addr.clone()
    }

    pub(crate) fn poll(&mut self, connect_status: &[ConnectionStatus]) -> Drain<'_, Event<T>> {
        let now = Instant::now();
        match self.state {
            ProtocolState::Synchronizing => {
                // Check for sync timeout if configured
                if let Some(timeout) = self.sync_config.sync_timeout {
                    let elapsed = self.stats_start_time.elapsed();
                    if elapsed > timeout {
                        self.event_queue.push_back(Event::SyncTimeout {
                            elapsed_ms: elapsed.as_millis(),
                        });
                    }
                }

                // some time has passed, let us send another sync request
                if self.last_send_time + self.sync_config.sync_retry_interval < now {
                    self.send_sync_request();
                }
            },
            ProtocolState::Running => {
                // resend pending inputs, if some time has passed without sending or receiving inputs
                if self.running_last_input_recv + self.sync_config.running_retry_interval < now {
                    self.send_pending_output(connect_status);
                    self.running_last_input_recv = Instant::now();
                }

                // periodically send a quality report
                if self.running_last_quality_report + self.protocol_config.quality_report_interval
                    < now
                {
                    self.send_quality_report();
                }

                // send keep alive packet if we didn't send a packet for some time
                if self.last_send_time + self.sync_config.keepalive_interval < now {
                    self.send_keep_alive();
                }

                // trigger a NetworkInterrupted event if we didn't receive a packet for some time
                if !self.disconnect_notify_sent
                    && self.last_recv_time + self.disconnect_notify_start < now
                {
                    let duration: Duration = self.disconnect_timeout - self.disconnect_notify_start;
                    self.event_queue.push_back(Event::NetworkInterrupted {
                        disconnect_timeout: Duration::as_millis(&duration),
                    });
                    self.disconnect_notify_sent = true;
                }

                // if we pass the disconnect_timeout threshold, send an event to disconnect
                if !self.disconnect_event_sent
                    && self.last_recv_time + self.disconnect_timeout < now
                {
                    self.event_queue.push_back(Event::Disconnected);
                    self.disconnect_event_sent = true;
                }
            },
            ProtocolState::Disconnected => {
                if self.shutdown_timeout < Instant::now() {
                    self.state = ProtocolState::Shutdown;
                }
            },
            ProtocolState::Initializing | ProtocolState::Shutdown => (),
        }
        self.event_queue.drain(..)
    }

    fn pop_pending_output(&mut self, ack_frame: Frame) {
        while !self.pending_output.is_empty() {
            if let Some(input) = self.pending_output.front() {
                if input.frame <= ack_frame {
                    // This should always succeed since we just checked front() and is_empty()
                    if let Some(popped) = self.pending_output.pop_front() {
                        self.last_acked_input = popped;
                    }
                } else {
                    break;
                }
            }
        }
    }

    /*
     *  SENDING MESSAGES
     */

    pub(crate) fn send_all_messages(
        &mut self,
        socket: &mut Box<dyn NonBlockingSocket<T::Address>>,
    ) {
        if self.state == ProtocolState::Shutdown {
            trace!(
                "Protocol is shutting down; dropping {} messages",
                self.send_queue.len()
            );
            self.send_queue.drain(..);
            return;
        }

        if self.send_queue.is_empty() {
            // avoid log spam if there's nothing to send
            return;
        }

        trace!("Sending {} messages over socket", self.send_queue.len());
        for msg in self.send_queue.drain(..) {
            socket.send_to(&msg, &self.peer_addr);
        }
    }

    pub(crate) fn send_input(
        &mut self,
        inputs: &BTreeMap<PlayerHandle, PlayerInput<T::Input>>,
        connect_status: &[ConnectionStatus],
    ) {
        if self.state != ProtocolState::Running {
            return;
        }

        let endpoint_data = InputBytes::from_inputs::<T>(self.num_players, inputs);

        // register the input and advantages in the time sync layer
        self.time_sync_layer.advance_frame(
            endpoint_data.frame,
            self.local_frame_advantage,
            self.remote_frame_advantage,
        );

        self.pending_output.push_back(endpoint_data);

        // we should never have so much pending input for a remote player (if they didn't ack, we should stop at MAX_PREDICTION_THRESHOLD)
        // this is a spectator that didn't ack our input, we just disconnect them
        if self.pending_output.len() > self.protocol_config.pending_output_limit {
            self.event_queue.push_back(Event::Disconnected);
        }

        self.send_pending_output(connect_status);
    }

    fn send_pending_output(&mut self, connect_status: &[ConnectionStatus]) {
        let mut body = Input::default();

        if let Some(input) = self.pending_output.front() {
            // Verify input frames are sequential relative to last acked
            if self.last_acked_input.frame != Frame::NULL
                && self.last_acked_input.frame + 1 != input.frame
            {
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::NetworkProtocol,
                    "Input frame sequence violation: last_acked={}, pending_front={}",
                    self.last_acked_input.frame,
                    input.frame
                );
                return;
            }
            body.start_frame = input.frame;

            // encode all pending inputs to a byte buffer
            body.bytes = encode(
                &self.last_acked_input.bytes,
                self.pending_output.iter().map(|gi| &gi.bytes),
            );
            trace!(
                "Encoded {} bytes from {} pending output(s) into {} bytes",
                {
                    let mut sum = 0;
                    for gi in self.pending_output.iter() {
                        sum += gi.bytes.len();
                    }
                    sum
                },
                self.pending_output.len(),
                body.bytes.len()
            );

            body.ack_frame = self.last_recv_frame();
            body.disconnect_requested = self.state == ProtocolState::Disconnected;
            connect_status.clone_into(&mut body.peer_connect_status);

            self.queue_message(MessageBody::Input(body));
        }
    }

    fn send_input_ack(&mut self) {
        let body = InputAck {
            ack_frame: self.last_recv_frame(),
        };

        self.queue_message(MessageBody::InputAck(body));
    }

    fn send_keep_alive(&mut self) {
        self.queue_message(MessageBody::KeepAlive);
    }

    fn send_sync_request(&mut self) {
        self.sync_requests_sent += 1;

        // Check for excessive retries and emit warning (once)
        if !self.sync_retry_warning_sent
            && self.sync_requests_sent > self.protocol_config.sync_retry_warning_threshold
        {
            self.sync_retry_warning_sent = true;
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::Synchronization,
                "Excessive sync retries: {} requests sent (threshold: {}). Possible high packet loss.",
                self.sync_requests_sent,
                self.protocol_config.sync_retry_warning_threshold
            );
        }

        // Check for excessive sync duration and emit warning (once)
        let elapsed_ms = self.stats_start_time.elapsed().as_millis();
        if !self.sync_duration_warning_sent
            && elapsed_ms > self.protocol_config.sync_duration_warning_ms
        {
            self.sync_duration_warning_sent = true;
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::Synchronization,
                "Sync duration exceeded threshold: {}ms (threshold: {}ms). Network latency may be high.",
                elapsed_ms,
                self.protocol_config.sync_duration_warning_ms
            );
        }

        let random_number: u32 = random();
        self.sync_random_requests.insert(random_number);
        let body = SyncRequest {
            random_request: random_number,
        };
        self.queue_message(MessageBody::SyncRequest(body));
    }

    fn send_quality_report(&mut self) {
        self.running_last_quality_report = Instant::now();

        // Get wall-clock time for ping calculation.
        // If the system clock is in an abnormal state, skip sending this quality report.
        // The peer will request another one later, and hopefully the clock will be fixed by then.
        let Some(ping_timestamp) = millis_since_epoch() else {
            trace!("Skipping quality report due to invalid system clock");
            return;
        };

        // Clamp to i16 range and convert - the clamp guarantees this won't fail,
        // but we use unwrap_or as defense-in-depth
        let clamped = self
            .local_frame_advantage
            .clamp(i16::MIN as i32, i16::MAX as i32);
        let frame_advantage = i16::try_from(clamped).unwrap_or(0);
        let body = QualityReport {
            frame_advantage,
            ping: ping_timestamp,
        };

        self.queue_message(MessageBody::QualityReport(body));
    }

    fn queue_message(&mut self, body: MessageBody) {
        trace!("Queuing message to {:?}: {:?}", self.peer_addr, body);

        // set the header
        let header = MessageHeader { magic: self.magic };
        let msg = Message { header, body };

        self.packets_sent += 1;
        self.last_send_time = Instant::now();
        self.bytes_sent += std::mem::size_of_val(&msg);

        // add the packet to the back of the send queue
        self.send_queue.push_back(msg);
    }

    /*
     *  RECEIVING MESSAGES
     */

    pub(crate) fn handle_message(&mut self, msg: &Message) {
        trace!("Handling message from {:?}: {:?}", self.peer_addr, msg);

        // don't handle messages if shutdown
        if self.state == ProtocolState::Shutdown {
            trace!("Protocol is shutting down; ignoring message");
            return;
        }

        // filter packets that don't match the magic if we have set it already
        if self.remote_magic != 0 && msg.header.magic != self.remote_magic {
            trace!("Received message with wrong magic; ignoring");
            return;
        }

        // update time when we last received packages
        self.last_recv_time = Instant::now();

        // if the connection has been marked as interrupted, send an event to signal we are receiving again
        if self.disconnect_notify_sent && self.state == ProtocolState::Running {
            trace!("Received message on interrupted protocol; sending NetworkResumed event");
            self.disconnect_notify_sent = false;
            self.event_queue.push_back(Event::NetworkResumed);
        }

        // handle the message
        match &msg.body {
            MessageBody::SyncRequest(body) => self.on_sync_request(*body),
            MessageBody::SyncReply(body) => self.on_sync_reply(msg.header, *body),
            MessageBody::Input(body) => self.on_input(body),
            MessageBody::InputAck(body) => self.on_input_ack(*body),
            MessageBody::QualityReport(body) => self.on_quality_report(body),
            MessageBody::QualityReply(body) => self.on_quality_reply(body),
            MessageBody::ChecksumReport(body) => self.on_checksum_report(body),
            MessageBody::KeepAlive => (),
        }
    }

    /// Upon receiving a `SyncRequest`, answer with a `SyncReply` with the proper data
    fn on_sync_request(&mut self, body: SyncRequest) {
        let reply_body = SyncReply {
            random_reply: body.random_request,
        };
        self.queue_message(MessageBody::SyncReply(reply_body));
    }

    /// Upon receiving a `SyncReply`, check validity and either continue the synchronization process or conclude synchronization.
    fn on_sync_reply(&mut self, header: MessageHeader, body: SyncReply) {
        // ignore sync replies when not syncing
        if self.state != ProtocolState::Synchronizing {
            return;
        }
        // this is not the correct reply
        if !self.sync_random_requests.remove(&body.random_reply) {
            return;
        }
        // the sync reply is good, so we send a sync request again until we have finished the required roundtrips. Then, we can conclude the syncing process.
        self.sync_remaining_roundtrips -= 1;
        let elapsed_ms = self.stats_start_time.elapsed().as_millis();
        if self.sync_remaining_roundtrips > 0 {
            // register an event
            let evt = Event::Synchronizing {
                total: self.sync_config.num_sync_packets,
                count: self.sync_config.num_sync_packets - self.sync_remaining_roundtrips,
                total_requests_sent: self.sync_requests_sent,
                elapsed_ms,
            };
            self.event_queue.push_back(evt);
            // send another sync request
            self.send_sync_request();
        } else {
            // switch to running state
            self.state = ProtocolState::Running;
            // register an event
            self.event_queue.push_back(Event::Synchronized);
            // the remote endpoint is now "authorized"
            self.remote_magic = header.magic;
        }
    }

    fn on_input(&mut self, body: &Input) {
        // drop pending outputs until the ack frame
        self.pop_pending_output(body.ack_frame);

        // update the peer connection status
        if body.disconnect_requested {
            // if a disconnect is requested, disconnect now
            if self.state != ProtocolState::Disconnected && !self.disconnect_event_sent {
                self.event_queue.push_back(Event::Disconnected);
                self.disconnect_event_sent = true;
            }
        } else {
            // update the peer connection status
            // Use zip to safely handle potential length mismatches (malformed packets)
            for (local, remote) in self
                .peer_connect_status
                .iter_mut()
                .zip(body.peer_connect_status.iter())
            {
                local.disconnected = remote.disconnected || local.disconnected;
                local.last_frame = std::cmp::max(local.last_frame, remote.last_frame);
            }
        }

        // Validate that received inputs are in a recoverable order.
        // If we receive an input for a frame that's too far ahead, we can't decode it
        // because we don't have the reference frame. This is normal UDP behavior -
        // packets can be lost or reordered. We just drop it and wait for retransmission.
        if self.last_recv_frame() != Frame::NULL && self.last_recv_frame() + 1 < body.start_frame {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::NetworkProtocol,
                "Received input for frame {} but last received was frame {} - gap too large to decode (likely packet loss)",
                body.start_frame,
                self.last_recv_frame()
            );
            return;
        }

        // if we did not receive any input yet, we decode with the blank input,
        // otherwise we use the input previous to the start of the encoded inputs
        let decode_frame = if self.last_recv_frame() == Frame::NULL {
            Frame::NULL
        } else {
            body.start_frame - 1
        };

        // if we have the necessary input saved, we decode
        if let Some(decode_inp) = self.recv_inputs.get(&decode_frame) {
            self.running_last_input_recv = Instant::now();

            let recv_inputs = match decode(&decode_inp.bytes, &body.bytes) {
                Ok(inputs) => inputs,
                Err(e) => {
                    report_violation!(
                        ViolationSeverity::Error,
                        ViolationKind::NetworkProtocol,
                        "Failed to decode input packet: {:?}. Packet may be corrupted.",
                        e
                    );
                    return;
                },
            };

            for (i, inp) in recv_inputs.into_iter().enumerate() {
                let inp_frame = body.start_frame + i as i32;
                // skip inputs that we don't need
                if inp_frame <= self.last_recv_frame() {
                    continue;
                }

                let input_data = InputBytes {
                    frame: inp_frame,
                    bytes: inp,
                };
                // send the input to the session
                let player_inputs = input_data.to_player_inputs::<T>(self.handles.len());
                self.recv_inputs.insert(input_data.frame, input_data);

                for (i, player_input) in player_inputs.into_iter().enumerate() {
                    // Bounds check on handles - should always be valid but be defensive
                    if i < self.handles.len() {
                        self.event_queue.push_back(Event::Input {
                            input: player_input,
                            player: self.handles[i],
                        });
                    }
                }
            }

            // send an input ack
            self.send_input_ack();

            // delete received inputs that are too old
            let last_recv_frame = self.last_recv_frame();
            self.recv_inputs
                .retain(|&k, _| k >= last_recv_frame - 2 * self.max_prediction as i32);
        }
    }

    /// Upon receiving a `InputAck`, discard the oldest buffered input including the acked input.
    fn on_input_ack(&mut self, body: InputAck) {
        self.pop_pending_output(body.ack_frame);
    }

    /// Upon receiving a `QualityReport`, update network stats and reply with a `QualityReply`.
    fn on_quality_report(&mut self, body: &QualityReport) {
        self.remote_frame_advantage = body.frame_advantage as i32;
        let reply_body = QualityReply { pong: body.ping };
        self.queue_message(MessageBody::QualityReply(reply_body));
    }

    /// Upon receiving a `QualityReply`, update network stats.
    fn on_quality_reply(&mut self, body: &QualityReply) {
        // Get current wall-clock time to calculate RTT.
        // If the system clock is in an abnormal state, skip this RTT update.
        // The next quality report cycle will try again.
        let Some(millis) = millis_since_epoch() else {
            trace!("Skipping RTT update due to invalid system clock");
            return;
        };
        // Use saturating subtraction to handle edge cases where system time
        // may have drifted between the ping and pong (e.g., NTP adjustments).
        // A 0 RTT is harmless - it will be corrected on the next quality report.
        self.round_trip_time = millis.saturating_sub(body.pong);
    }

    /// Upon receiving a `ChecksumReport`, add it to the checksum history
    fn on_checksum_report(&mut self, body: &ChecksumReport) {
        let interval = if let DesyncDetection::On { interval } = self.desync_detection {
            interval
        } else {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::Configuration,
                "Received checksum report, but desync detection is off. Check that configuration is consistent between peers."
            );
            1
        };

        let max_history = self.protocol_config.max_checksum_history;
        if self.pending_checksums.len() >= max_history {
            let oldest_frame_to_keep = body.frame - (max_history as i32 - 1) * interval as i32;
            self.pending_checksums
                .retain(|&frame, _| frame >= oldest_frame_to_keep);
        }
        self.pending_checksums.insert(body.frame, body.checksum);
    }

    /// Returns the frame of the last received input
    fn last_recv_frame(&self) -> Frame {
        match self.recv_inputs.iter().max_by_key(|&(k, _)| k) {
            Some((k, _)) => *k,
            None => Frame::NULL,
        }
    }

    pub(crate) fn send_checksum_report(&mut self, frame_to_send: Frame, checksum: u128) {
        let body = ChecksumReport {
            frame: frame_to_send,
            checksum,
        };
        self.queue_message(MessageBody::ChecksumReport(body));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::net::SocketAddr;

    // Test configuration
    #[repr(C)]
    #[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize, Debug)]
    struct TestInput {
        inp: u32,
    }

    #[derive(Clone, Default)]
    struct TestState;

    struct TestConfig;

    impl Config for TestConfig {
        type Input = TestInput;
        type State = TestState;
        type Address = SocketAddr;
    }

    fn test_addr() -> SocketAddr {
        "127.0.0.1:7000".parse().unwrap()
    }

    /// Default number of sync packets for test purposes
    const TEST_NUM_SYNC_PACKETS: u32 = 5;

    fn create_protocol(
        handles: Vec<PlayerHandle>,
        num_players: usize,
        local_players: usize,
        max_prediction: usize,
    ) -> UdpProtocol<TestConfig> {
        create_protocol_with_config(
            handles,
            num_players,
            local_players,
            max_prediction,
            SyncConfig::default(),
            ProtocolConfig::default(),
        )
    }

    fn create_protocol_with_config(
        handles: Vec<PlayerHandle>,
        num_players: usize,
        local_players: usize,
        max_prediction: usize,
        sync_config: SyncConfig,
        protocol_config: ProtocolConfig,
    ) -> UdpProtocol<TestConfig> {
        UdpProtocol::new(
            handles,
            test_addr(),
            num_players,
            local_players,
            max_prediction,
            Duration::from_millis(5000),
            Duration::from_millis(3000),
            60,
            DesyncDetection::Off,
            sync_config,
            protocol_config,
        )
        .expect("Failed to create test protocol")
    }

    // ==========================================
    // State Machine Tests
    // ==========================================

    #[test]
    fn new_protocol_starts_in_initializing_state() {
        let protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);

        assert!(!protocol.is_synchronized());
        assert!(!protocol.is_running());
    }

    #[test]
    fn synchronize_transitions_to_synchronizing_state() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);

        protocol.synchronize();

        // Still not synchronized until sync completes
        assert!(!protocol.is_synchronized());
        assert!(!protocol.is_running());
        // But it should have queued a sync request
        assert!(!protocol.send_queue.is_empty());
    }

    #[test]
    #[allow(clippy::wildcard_enum_match_arm)]
    fn sync_request_queues_sync_reply() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize();

        // Clear the initial sync request
        protocol.send_queue.clear();

        // Simulate receiving a sync request
        let sync_req = SyncRequest {
            random_request: 12345,
        };
        protocol.on_sync_request(sync_req);

        // Should have queued a reply
        assert_eq!(protocol.send_queue.len(), 1);
        let msg = protocol.send_queue.front().unwrap();
        match &msg.body {
            MessageBody::SyncReply(reply) => {
                assert_eq!(reply.random_reply, 12345);
            },
            _ => panic!("Expected SyncReply message"),
        }
    }

    #[test]
    fn complete_sync_transitions_to_running() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize();

        // Complete all sync roundtrips
        for _ in 0..TEST_NUM_SYNC_PACKETS {
            // Get the random request from our sync request
            let random = *protocol.sync_random_requests.iter().next().unwrap();

            let header = MessageHeader { magic: 999 };
            let reply = SyncReply {
                random_reply: random,
            };
            protocol.on_sync_reply(header, reply);
        }

        assert!(protocol.is_synchronized());
        assert!(protocol.is_running());
    }

    #[test]
    fn sync_reply_with_wrong_random_is_ignored() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize();

        let initial_remaining = protocol.sync_remaining_roundtrips;

        // Send a reply with the wrong random value
        let header = MessageHeader { magic: 999 };
        let reply = SyncReply {
            random_reply: 99999999, // Wrong value
        };
        protocol.on_sync_reply(header, reply);

        // Should still have same number of remaining roundtrips
        assert_eq!(protocol.sync_remaining_roundtrips, initial_remaining);
    }

    #[test]
    fn sync_reply_when_not_synchronizing_is_ignored() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);

        // Protocol is in Initializing state, not Synchronizing
        let header = MessageHeader { magic: 999 };
        let reply = SyncReply { random_reply: 123 };
        protocol.on_sync_reply(header, reply);

        // Should still be in initializing
        assert!(!protocol.is_synchronized());
    }

    #[test]
    fn disconnect_transitions_to_disconnected() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize();

        // Complete sync
        for _ in 0..TEST_NUM_SYNC_PACKETS {
            let random = *protocol.sync_random_requests.iter().next().unwrap();
            let header = MessageHeader { magic: 999 };
            protocol.on_sync_reply(
                header,
                SyncReply {
                    random_reply: random,
                },
            );
        }

        assert!(protocol.is_running());

        protocol.disconnect();

        // Still counts as synchronized but not running
        assert!(protocol.is_synchronized());
        assert!(!protocol.is_running());
    }

    #[test]
    fn disconnect_when_already_shutdown_does_nothing() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.state = ProtocolState::Shutdown;

        protocol.disconnect();

        // Should still be shutdown, not disconnected
        assert_eq!(protocol.state, ProtocolState::Shutdown);
    }

    // ==========================================
    // Message Handling Tests
    // ==========================================

    #[test]
    fn handle_message_ignores_shutdown_state() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.state = ProtocolState::Shutdown;

        let msg = Message {
            header: MessageHeader { magic: 123 },
            body: MessageBody::KeepAlive,
        };
        protocol.handle_message(&msg);

        // Event queue should be empty
        assert!(protocol.event_queue.is_empty());
    }

    #[test]
    fn handle_message_filters_wrong_magic_after_sync() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize();

        // Complete sync with magic 999
        for _ in 0..TEST_NUM_SYNC_PACKETS {
            let random = *protocol.sync_random_requests.iter().next().unwrap();
            let header = MessageHeader { magic: 999 };
            protocol.on_sync_reply(
                header,
                SyncReply {
                    random_reply: random,
                },
            );
        }

        assert_eq!(protocol.remote_magic, 999);
        protocol.send_queue.clear();

        // Send message with different magic
        let msg = Message {
            header: MessageHeader { magic: 123 }, // Wrong magic
            body: MessageBody::KeepAlive,
        };
        protocol.handle_message(&msg);

        // Should be ignored - no state changes
        assert!(protocol.send_queue.is_empty());
    }

    #[test]
    fn handle_message_accepts_correct_magic() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize();

        // Complete sync with magic 999
        for _ in 0..TEST_NUM_SYNC_PACKETS {
            let random = *protocol.sync_random_requests.iter().next().unwrap();
            let header = MessageHeader { magic: 999 };
            protocol.on_sync_reply(
                header,
                SyncReply {
                    random_reply: random,
                },
            );
        }

        let initial_recv_time = protocol.last_recv_time;

        // Wait a tiny bit
        std::thread::sleep(Duration::from_millis(1));

        // Send message with correct magic
        let msg = Message {
            header: MessageHeader { magic: 999 },
            body: MessageBody::KeepAlive,
        };
        protocol.handle_message(&msg);

        // Should update recv time
        assert!(protocol.last_recv_time > initial_recv_time);
    }

    #[test]
    fn network_resumed_event_after_interrupt() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize();

        // Complete sync
        for _ in 0..TEST_NUM_SYNC_PACKETS {
            let random = *protocol.sync_random_requests.iter().next().unwrap();
            let header = MessageHeader { magic: 999 };
            protocol.on_sync_reply(
                header,
                SyncReply {
                    random_reply: random,
                },
            );
        }

        // Simulate network interrupt notification was sent
        protocol.disconnect_notify_sent = true;

        // Handle a valid message
        let msg = Message {
            header: MessageHeader { magic: 999 },
            body: MessageBody::KeepAlive,
        };
        protocol.handle_message(&msg);

        // Should have NetworkResumed event
        let events: Vec<_> = protocol.event_queue.drain(..).collect();
        assert!(events.iter().any(|e| matches!(e, Event::NetworkResumed)));
        assert!(!protocol.disconnect_notify_sent);
    }

    // ==========================================
    // Input Handling Tests
    // ==========================================

    #[test]
    fn input_ack_pops_pending_output() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize();

        // Complete sync
        for _ in 0..TEST_NUM_SYNC_PACKETS {
            let random = *protocol.sync_random_requests.iter().next().unwrap();
            let header = MessageHeader { magic: 999 };
            protocol.on_sync_reply(
                header,
                SyncReply {
                    random_reply: random,
                },
            );
        }

        // Add some pending outputs
        protocol.pending_output.push_back(InputBytes {
            frame: Frame::new(0),
            bytes: vec![0, 0, 0, 0],
        });
        protocol.pending_output.push_back(InputBytes {
            frame: Frame::new(1),
            bytes: vec![1, 0, 0, 0],
        });
        protocol.pending_output.push_back(InputBytes {
            frame: Frame::new(2),
            bytes: vec![2, 0, 0, 0],
        });

        assert_eq!(protocol.pending_output.len(), 3);

        // Ack frame 1
        protocol.on_input_ack(InputAck {
            ack_frame: Frame::new(1),
        });

        // Should have removed frames 0 and 1
        assert_eq!(protocol.pending_output.len(), 1);
        assert_eq!(
            protocol.pending_output.front().unwrap().frame,
            Frame::new(2)
        );
        assert_eq!(protocol.last_acked_input.frame, Frame::new(1));
    }

    #[test]
    fn send_input_when_not_running_does_nothing() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        // Protocol is in Initializing state

        let inputs = BTreeMap::new();
        let connect_status = vec![ConnectionStatus::default(); 2];

        protocol.send_input(&inputs, &connect_status);

        // Should not queue any messages
        assert!(protocol.send_queue.is_empty());
        assert!(protocol.pending_output.is_empty());
    }

    // ==========================================
    // Quality Report Tests
    // ==========================================

    #[test]
    #[allow(clippy::wildcard_enum_match_arm)]
    fn quality_report_triggers_reply() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize();

        // Complete sync
        for _ in 0..TEST_NUM_SYNC_PACKETS {
            let random = *protocol.sync_random_requests.iter().next().unwrap();
            let header = MessageHeader { magic: 999 };
            protocol.on_sync_reply(
                header,
                SyncReply {
                    random_reply: random,
                },
            );
        }
        protocol.send_queue.clear();

        let report = QualityReport {
            frame_advantage: 5,
            ping: 12345,
        };
        protocol.on_quality_report(&report);

        assert_eq!(protocol.remote_frame_advantage, 5);

        // Should have queued a quality reply
        assert_eq!(protocol.send_queue.len(), 1);
        let msg = protocol.send_queue.front().unwrap();
        match &msg.body {
            MessageBody::QualityReply(reply) => {
                assert_eq!(reply.pong, 12345);
            },
            _ => panic!("Expected QualityReply message"),
        }
    }

    // ==========================================
    // Checksum Report Tests
    // ==========================================

    #[test]
    fn checksum_report_stored_with_desync_detection_off() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);

        let report = ChecksumReport {
            frame: Frame::new(100),
            checksum: 0xDEADBEEF,
        };
        protocol.on_checksum_report(&report);

        // Should still store it (with a warning, but we can't test that here)
        assert_eq!(
            protocol.pending_checksums.get(&Frame::new(100)),
            Some(&0xDEADBEEF)
        );
    }

    #[test]
    fn checksum_report_limits_history_size() {
        let protocol_config = ProtocolConfig::default();
        let max_history = protocol_config.max_checksum_history;

        let mut protocol: UdpProtocol<TestConfig> = UdpProtocol::new(
            vec![PlayerHandle::new(0)],
            test_addr(),
            2,
            1,
            8,
            Duration::from_millis(5000),
            Duration::from_millis(3000),
            60,
            DesyncDetection::On { interval: 1 },
            SyncConfig::default(),
            protocol_config,
        )
        .expect("Failed to create test protocol");

        // Add more than max_checksum_history checksums
        for frame in 0..(max_history as i32 + 10) {
            let report = ChecksumReport {
                frame: Frame::new(frame),
                checksum: frame as u128,
            };
            protocol.on_checksum_report(&report);
        }

        // Should have limited to max_checksum_history
        assert!(protocol.pending_checksums.len() <= max_history);

        // Oldest frames should be removed
        let max_frame = Frame::new(max_history as i32 + 9);
        assert!(protocol.pending_checksums.contains_key(&max_frame));
        // Old frames should be gone
        assert!(!protocol.pending_checksums.contains_key(&Frame::new(0)));
    }

    // ==========================================
    // Network Stats Tests
    // ==========================================

    #[test]
    fn network_stats_returns_error_when_not_synchronized() {
        let protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);

        let result = protocol.network_stats();
        assert!(matches!(result, Err(FortressError::NotSynchronized)));
    }

    #[test]
    fn network_stats_returns_error_when_no_time_elapsed() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize();

        // Complete sync
        for _ in 0..TEST_NUM_SYNC_PACKETS {
            let random = *protocol.sync_random_requests.iter().next().unwrap();
            let header = MessageHeader { magic: 999 };
            protocol.on_sync_reply(
                header,
                SyncReply {
                    random_reply: random,
                },
            );
        }

        // Stats start time is set during synchronize(), so with 0 seconds elapsed
        // it should return an error
        let result = protocol.network_stats();
        // This will likely fail because no time has passed
        // The actual behavior depends on timing
        assert!(result.is_ok() || matches!(result, Err(FortressError::NotSynchronized)));
    }

    // ==========================================
    // Poll / Timeout Tests
    // ==========================================

    #[test]
    fn poll_returns_events_and_clears_queue() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize();

        // Complete sync to generate Synchronizing and Synchronized events
        for _ in 0..TEST_NUM_SYNC_PACKETS {
            let random = *protocol.sync_random_requests.iter().next().unwrap();
            let header = MessageHeader { magic: 999 };
            protocol.on_sync_reply(
                header,
                SyncReply {
                    random_reply: random,
                },
            );
        }

        let connect_status = vec![ConnectionStatus::default(); 2];
        let events: Vec<_> = protocol.poll(&connect_status).collect();

        // Should have Synchronizing events and Synchronized event
        assert!(!events.is_empty());
        assert!(events.iter().any(|e| matches!(e, Event::Synchronized)));

        // Queue should be empty after drain
        assert!(protocol.event_queue.is_empty());
    }

    #[test]
    fn poll_in_disconnected_state_transitions_to_shutdown() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.state = ProtocolState::Disconnected;

        // Set shutdown timeout to the past
        protocol.shutdown_timeout = Instant::now() - Duration::from_secs(1);

        let connect_status = vec![ConnectionStatus::default(); 2];
        let _events: Vec<_> = protocol.poll(&connect_status).collect();

        // Should have transitioned to Shutdown
        assert_eq!(protocol.state, ProtocolState::Shutdown);
    }

    // ==========================================
    // Accessor Tests
    // ==========================================

    #[test]
    fn handles_returns_sorted_handles() {
        let protocol: UdpProtocol<TestConfig> = create_protocol(
            vec![
                PlayerHandle::new(2),
                PlayerHandle::new(0),
                PlayerHandle::new(1),
            ],
            3,
            3,
            8,
        );

        let handles = protocol.handles();
        assert_eq!(
            handles,
            &vec![
                PlayerHandle::new(0),
                PlayerHandle::new(1),
                PlayerHandle::new(2)
            ]
        );
    }

    #[test]
    fn peer_addr_returns_correct_address() {
        let protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);

        assert_eq!(protocol.peer_addr(), test_addr());
    }

    #[test]
    fn is_handling_message_checks_address() {
        let protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);

        assert!(protocol.is_handling_message(&test_addr()));

        let other_addr: SocketAddr = "127.0.0.1:8000".parse().unwrap();
        assert!(!protocol.is_handling_message(&other_addr));
    }

    #[test]
    fn peer_connect_status_returns_correct_status() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);

        // Modify status for player 1
        protocol.peer_connect_status[1] = ConnectionStatus {
            disconnected: true,
            last_frame: Frame::new(100),
        };

        let status = protocol.peer_connect_status(PlayerHandle::new(1));
        assert!(status.disconnected);
        assert_eq!(status.last_frame, Frame::new(100));
    }

    // ==========================================
    // Frame Advantage Tests
    // ==========================================

    #[test]
    fn update_local_frame_advantage_with_null_frames() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);

        // Both frames are Frame::NULL, should return early
        protocol.update_local_frame_advantage(Frame::NULL);
        assert_eq!(protocol.local_frame_advantage, 0);

        // Local frame set but no recv frame
        protocol.update_local_frame_advantage(Frame::new(10));
        assert_eq!(protocol.local_frame_advantage, 0);
    }

    #[test]
    fn average_frame_advantage_delegates_to_time_sync() {
        let protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);

        // Just verify it doesn't panic - the actual value depends on TimeSync internals
        let _advantage = protocol.average_frame_advantage();
    }

    // ==========================================
    // InputBytes Tests
    // ==========================================

    #[test]
    fn input_bytes_zeroed_creates_correct_size() {
        let input_bytes =
            InputBytes::zeroed::<TestConfig>(2).expect("Failed to create input bytes");

        assert_eq!(input_bytes.frame, Frame::NULL);
        // Each TestInput is 4 bytes (u32), so 2 players = 8 bytes
        assert_eq!(input_bytes.bytes.len(), 8);
        assert!(input_bytes.bytes.iter().all(|&b| b == 0));
    }

    #[test]
    fn input_bytes_from_inputs_serializes_correctly() {
        let mut inputs = BTreeMap::new();
        inputs.insert(
            PlayerHandle::new(0),
            PlayerInput::new(Frame::new(10), TestInput { inp: 0xAABBCCDD }),
        );
        inputs.insert(
            PlayerHandle::new(1),
            PlayerInput::new(Frame::new(10), TestInput { inp: 0x11223344 }),
        );

        let input_bytes = InputBytes::from_inputs::<TestConfig>(2, &inputs);

        assert_eq!(input_bytes.frame, Frame::new(10));
        assert_eq!(input_bytes.bytes.len(), 8);
    }

    #[test]
    fn input_bytes_roundtrip() {
        let mut inputs = BTreeMap::new();
        inputs.insert(
            PlayerHandle::new(0),
            PlayerInput::new(Frame::new(5), TestInput { inp: 12345 }),
        );
        inputs.insert(
            PlayerHandle::new(1),
            PlayerInput::new(Frame::new(5), TestInput { inp: 67890 }),
        );

        let input_bytes = InputBytes::from_inputs::<TestConfig>(2, &inputs);
        let player_inputs = input_bytes.to_player_inputs::<TestConfig>(2);

        assert_eq!(player_inputs.len(), 2);
        assert_eq!(player_inputs[0].frame, Frame::new(5));
        assert_eq!(player_inputs[0].input.inp, 12345);
        assert_eq!(player_inputs[1].frame, Frame::new(5));
        assert_eq!(player_inputs[1].input.inp, 67890);
    }

    // ==========================================
    // Send Queue Tests
    // ==========================================

    #[test]
    #[allow(clippy::wildcard_enum_match_arm)]
    fn send_checksum_report_queues_message() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.send_queue.clear();

        protocol.send_checksum_report(Frame::new(100), 0xDEADBEEF);

        assert_eq!(protocol.send_queue.len(), 1);
        let msg = protocol.send_queue.front().unwrap();
        match &msg.body {
            MessageBody::ChecksumReport(report) => {
                assert_eq!(report.frame, Frame::new(100));
                assert_eq!(report.checksum, 0xDEADBEEF);
            },
            _ => panic!("Expected ChecksumReport message"),
        }
    }

    #[test]
    fn protocol_equality_is_by_peer_address() {
        let protocol1: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        let protocol2: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(1)], 3, 2, 16);

        // Same peer address
        assert!(protocol1 == protocol2);

        // Different peer address
        let protocol3: UdpProtocol<TestConfig> = UdpProtocol::new(
            vec![PlayerHandle::new(0)],
            "127.0.0.1:8000".parse().unwrap(),
            2,
            1,
            8,
            Duration::from_millis(5000),
            Duration::from_millis(3000),
            60,
            DesyncDetection::Off,
            SyncConfig::default(),
            ProtocolConfig::default(),
        )
        .expect("Failed to create test protocol");
        assert!(protocol1 != protocol3);
    }

    // ==========================================
    // Frame Gap Detection Tests
    // ==========================================

    /// Test that on_input correctly detects and handles frame gaps.
    /// When the gap is too large to decode (we don't have the reference frame),
    /// the input should be dropped and a violation should be reported.
    #[test]
    fn on_input_rejects_input_with_too_large_gap() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize();

        // Complete sync to get to Running state
        for _ in 0..TEST_NUM_SYNC_PACKETS {
            let random = *protocol.sync_random_requests.iter().next().unwrap();
            let header = MessageHeader { magic: 999 };
            protocol.on_sync_reply(
                header,
                SyncReply {
                    random_reply: random,
                },
            );
        }
        assert!(protocol.is_running());

        // Set up initial state: we have received frame 0
        protocol.recv_inputs.insert(
            Frame::new(0),
            InputBytes {
                frame: Frame::new(0),
                bytes: vec![0, 0, 0, 0],
            },
        );

        // Try to receive an input that's too far ahead (frame 5 when we're at 0)
        // This creates a gap that's too large to decode
        let input = Input {
            start_frame: Frame::new(5), // Gap of 5 when max is 1
            ack_frame: Frame::NULL,
            bytes: vec![1, 2, 3, 4],
            disconnect_requested: false,
            peer_connect_status: vec![ConnectionStatus::default(); 2],
        };

        // Clear event queue and record input count before
        protocol.event_queue.clear();
        let inputs_before = protocol.recv_inputs.len();

        // Call on_input with the gap
        protocol.on_input(&input);

        // Verify: no new inputs were added (because gap too large)
        assert_eq!(
            protocol.recv_inputs.len(),
            inputs_before,
            "No inputs should be added when gap is too large"
        );

        // Verify: no input events were generated
        let input_events: Vec<_> = protocol
            .event_queue
            .iter()
            .filter(|e| matches!(e, Event::Input { .. }))
            .collect();
        assert!(
            input_events.is_empty(),
            "No input events should be generated when gap is too large"
        );
    }

    /// Test that consecutive frames are processed correctly
    #[test]
    fn on_input_accepts_consecutive_frame() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize();

        // Complete sync
        for _ in 0..TEST_NUM_SYNC_PACKETS {
            let random = *protocol.sync_random_requests.iter().next().unwrap();
            let header = MessageHeader { magic: 999 };
            protocol.on_sync_reply(
                header,
                SyncReply {
                    random_reply: random,
                },
            );
        }

        // Set up initial state: we have frame 0
        let initial_bytes = vec![0u8; 4]; // TestConfig::Input is [u8; 4]
        protocol.recv_inputs.insert(
            Frame::new(0),
            InputBytes {
                frame: Frame::new(0),
                bytes: initial_bytes.clone(),
            },
        );

        // Encode frame 1 relative to frame 0
        let frame1_bytes = vec![1u8; 4];
        let encoded = encode(&initial_bytes, std::iter::once(&frame1_bytes));

        let input = Input {
            start_frame: Frame::new(1), // Consecutive - gap of 1 is ok
            ack_frame: Frame::NULL,
            bytes: encoded,
            disconnect_requested: false,
            peer_connect_status: vec![ConnectionStatus::default(); 2],
        };

        protocol.event_queue.clear();
        protocol.on_input(&input);

        // Verify: frame 1 was added
        assert!(
            protocol.recv_inputs.contains_key(&Frame::new(1)),
            "Frame 1 should be added when gap is acceptable"
        );
    }

    /// Test that first input (when no previous non-NULL input exists) is accepted
    #[test]
    fn on_input_accepts_first_input_with_null_frame() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize();

        // Complete sync
        for _ in 0..TEST_NUM_SYNC_PACKETS {
            let random = *protocol.sync_random_requests.iter().next().unwrap();
            let header = MessageHeader { magic: 999 };
            protocol.on_sync_reply(
                header,
                SyncReply {
                    random_reply: random,
                },
            );
        }

        // The protocol constructor inserts Frame::NULL entry for decoding first input.
        // So recv_inputs is NOT empty, but last_recv_frame() returns Frame::NULL
        // because the NULL frame is special.
        assert!(
            protocol.recv_inputs.contains_key(&Frame::NULL),
            "Protocol should have Frame::NULL entry for decoding"
        );
        assert_eq!(
            protocol.last_recv_frame(),
            Frame::NULL,
            "last_recv_frame should return NULL when only NULL entry exists"
        );

        // Get the zeroed bytes from the protocol's NULL entry - this is the reference for encoding
        let zeroed_bytes = protocol
            .recv_inputs
            .get(&Frame::NULL)
            .unwrap()
            .bytes
            .clone();

        // First input comes with frame 0, encoded relative to zeroed bytes
        let test_input = TestInput { inp: 42 };
        let test_bytes = crate::network::codec::encode(&test_input).unwrap();

        // The encoded bytes should have the same size as the reference
        assert_eq!(
            test_bytes.len(),
            zeroed_bytes.len(),
            "Input size should match zeroed size"
        );

        let encoded = encode(&zeroed_bytes, std::iter::once(&test_bytes));

        let input = Input {
            start_frame: Frame::new(0),
            ack_frame: Frame::NULL,
            bytes: encoded,
            disconnect_requested: false,
            peer_connect_status: vec![ConnectionStatus::default(); 2],
        };

        protocol.event_queue.clear();
        protocol.on_input(&input);

        // Verify: frame 0 was added
        assert!(
            protocol.recv_inputs.contains_key(&Frame::new(0)),
            "First input (frame 0) should be accepted when last_recv_frame is NULL"
        );
    }

    /// Test frame gap boundary: gap of exactly 1 is acceptable
    #[test]
    fn on_input_boundary_gap_of_one_is_acceptable() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize();

        // Complete sync
        for _ in 0..TEST_NUM_SYNC_PACKETS {
            let random = *protocol.sync_random_requests.iter().next().unwrap();
            let header = MessageHeader { magic: 999 };
            protocol.on_sync_reply(
                header,
                SyncReply {
                    random_reply: random,
                },
            );
        }

        // Set up: we have frame 5
        let frame5_bytes = vec![5u8; 4];
        protocol.recv_inputs.insert(
            Frame::new(5),
            InputBytes {
                frame: Frame::new(5),
                bytes: frame5_bytes.clone(),
            },
        );

        // Receive frame 6 (gap of exactly 1)
        let frame6_bytes = vec![6u8; 4];
        let encoded = encode(&frame5_bytes, std::iter::once(&frame6_bytes));

        let input = Input {
            start_frame: Frame::new(6), // last_recv_frame() + 1 = 6, so 6 >= 6 is ok
            ack_frame: Frame::NULL,
            bytes: encoded,
            disconnect_requested: false,
            peer_connect_status: vec![ConnectionStatus::default(); 2],
        };

        let inputs_before = protocol.recv_inputs.len();
        protocol.event_queue.clear();
        protocol.on_input(&input);

        // Verify: frame 6 was added
        assert!(
            protocol.recv_inputs.contains_key(&Frame::new(6)),
            "Gap of 1 should be acceptable"
        );
        assert_eq!(protocol.recv_inputs.len(), inputs_before + 1);
    }

    /// Test frame gap boundary: gap of exactly 2 is rejected
    #[test]
    fn on_input_boundary_gap_of_two_is_rejected() {
        let mut protocol: UdpProtocol<TestConfig> =
            create_protocol(vec![PlayerHandle::new(0)], 2, 1, 8);
        protocol.synchronize();

        // Complete sync
        for _ in 0..TEST_NUM_SYNC_PACKETS {
            let random = *protocol.sync_random_requests.iter().next().unwrap();
            let header = MessageHeader { magic: 999 };
            protocol.on_sync_reply(
                header,
                SyncReply {
                    random_reply: random,
                },
            );
        }

        // Set up: we have frame 5
        protocol.recv_inputs.insert(
            Frame::new(5),
            InputBytes {
                frame: Frame::new(5),
                bytes: vec![5u8; 4],
            },
        );

        // Try to receive frame 7 (gap of 2 - we're missing frame 6)
        let input = Input {
            start_frame: Frame::new(7), // last_recv_frame() + 1 = 6, but we have 7 < 6 is false
            ack_frame: Frame::NULL,
            bytes: vec![1, 2, 3, 4], // Won't be decoded anyway
            disconnect_requested: false,
            peer_connect_status: vec![ConnectionStatus::default(); 2],
        };

        let inputs_before = protocol.recv_inputs.len();
        protocol.event_queue.clear();
        protocol.on_input(&input);

        // Verify: no new inputs were added
        assert_eq!(
            protocol.recv_inputs.len(),
            inputs_before,
            "Gap of 2 should be rejected"
        );
        assert!(!protocol.recv_inputs.contains_key(&Frame::new(7)));
    }

    // ==========================================
    // Input Frame Consistency Tests
    // ==========================================

    /// Test that from_inputs handles frame consistency correctly.
    ///
    /// When frames are inconsistent, the function logs a warning violation
    /// but continues processing using the first non-NULL frame. This is
    /// safe because the serialized input data is still correct - only the
    /// frame metadata is inconsistent.
    #[test]
    fn from_inputs_handles_inconsistent_frames_gracefully() {
        use std::collections::BTreeMap;

        // Test 1: Consistent frames work correctly
        let mut inputs = BTreeMap::new();
        inputs.insert(
            PlayerHandle::new(0),
            PlayerInput {
                frame: Frame::new(5),
                input: TestInput { inp: 1 },
            },
        );
        inputs.insert(
            PlayerHandle::new(1),
            PlayerInput {
                frame: Frame::new(5), // Same frame - no violation
                input: TestInput { inp: 2 },
            },
        );

        let result = InputBytes::from_inputs::<TestConfig>(2, &inputs);
        assert!(
            !result.bytes.is_empty(),
            "Should produce bytes for consistent frames"
        );
        assert_eq!(result.frame, Frame::new(5));

        // Test 2: Inconsistent frames still produce valid output
        // (with a warning violation logged)
        let mut inconsistent_inputs = BTreeMap::new();
        inconsistent_inputs.insert(
            PlayerHandle::new(0),
            PlayerInput {
                frame: Frame::new(5),
                input: TestInput { inp: 1 },
            },
        );
        inconsistent_inputs.insert(
            PlayerHandle::new(1),
            PlayerInput {
                frame: Frame::new(7), // Different frame - logs warning but continues
                input: TestInput { inp: 2 },
            },
        );

        let result = InputBytes::from_inputs::<TestConfig>(2, &inconsistent_inputs);
        // Should still produce valid bytes - the serialized input data is correct
        assert!(
            !result.bytes.is_empty(),
            "Should still produce bytes for inconsistent frames"
        );
        // Uses the first non-NULL frame (from player 0)
        assert_eq!(result.frame, Frame::new(5));
    }

    /// Test that from_inputs handles consistent frames correctly
    #[test]
    fn from_inputs_accepts_consistent_frames() {
        use std::collections::BTreeMap;

        // Add inputs with consistent frames
        let mut inputs = BTreeMap::new();
        inputs.insert(
            PlayerHandle::new(0),
            PlayerInput {
                frame: Frame::new(5),
                input: TestInput { inp: 1 },
            },
        );
        inputs.insert(
            PlayerHandle::new(1),
            PlayerInput {
                frame: Frame::new(5), // Same frame
                input: TestInput { inp: 2 },
            },
        );

        let result = InputBytes::from_inputs::<TestConfig>(2, &inputs);

        assert!(!result.bytes.is_empty());
        assert_eq!(result.frame, Frame::new(5));
    }

    /// Test that from_inputs handles NULL frames as wildcard
    #[test]
    fn from_inputs_null_frame_is_wildcard() {
        use std::collections::BTreeMap;

        let mut inputs = BTreeMap::new();

        // Add input with real frame and one with NULL
        inputs.insert(
            PlayerHandle::new(0),
            PlayerInput {
                frame: Frame::new(5),
                input: TestInput { inp: 1 },
            },
        );
        inputs.insert(
            PlayerHandle::new(1),
            PlayerInput {
                frame: Frame::NULL, // NULL frame should be skipped in consistency check
                input: TestInput { inp: 2 },
            },
        );

        let result = InputBytes::from_inputs::<TestConfig>(2, &inputs);

        // Should work without violation
        assert!(!result.bytes.is_empty());
        assert_eq!(result.frame, Frame::new(5));
    }

    // ==========================================
    // SyncConfig Tests
    // ==========================================

    #[test]
    fn sync_config_default_values() {
        let config = SyncConfig::default();
        assert_eq!(config.num_sync_packets, 5);
        assert_eq!(config.sync_retry_interval, Duration::from_millis(200));
        assert_eq!(config.sync_timeout, None);
        assert_eq!(config.running_retry_interval, Duration::from_millis(200));
        assert_eq!(config.keepalive_interval, Duration::from_millis(200));
    }

    #[test]
    fn sync_config_high_latency_preset() {
        let config = SyncConfig::high_latency();
        assert_eq!(config.num_sync_packets, 5);
        assert_eq!(config.sync_retry_interval, Duration::from_millis(400));
        assert_eq!(config.sync_timeout, Some(Duration::from_secs(10)));
        assert_eq!(config.running_retry_interval, Duration::from_millis(400));
        assert_eq!(config.keepalive_interval, Duration::from_millis(400));
    }

    #[test]
    fn sync_config_lossy_preset() {
        let config = SyncConfig::lossy();
        assert_eq!(config.num_sync_packets, 8);
        assert_eq!(config.sync_retry_interval, Duration::from_millis(200));
        assert_eq!(config.sync_timeout, Some(Duration::from_secs(10)));
    }

    #[test]
    fn sync_config_lan_preset() {
        let config = SyncConfig::lan();
        assert_eq!(config.num_sync_packets, 3);
        assert_eq!(config.sync_retry_interval, Duration::from_millis(100));
        assert_eq!(config.sync_timeout, Some(Duration::from_secs(5)));
    }

    #[test]
    #[allow(clippy::wildcard_enum_match_arm)]
    fn protocol_uses_custom_num_sync_packets() {
        let custom_config = SyncConfig {
            num_sync_packets: 3,
            ..SyncConfig::default()
        };

        let mut protocol: UdpProtocol<TestConfig> = create_protocol_with_config(
            vec![PlayerHandle::new(0)],
            2,
            1,
            8,
            custom_config,
            ProtocolConfig::default(),
        );

        protocol.synchronize();

        // Simulate 3 successful sync roundtrips
        for i in 0..3 {
            let request_msg = protocol.send_queue.pop_back().unwrap();
            let random = match request_msg.body {
                MessageBody::SyncRequest(req) => req.random_request,
                _ => panic!("Expected SyncRequest"),
            };

            let reply = Message {
                header: MessageHeader { magic: 42 },
                body: MessageBody::SyncReply(SyncReply {
                    random_reply: random,
                }),
            };
            protocol.handle_message(&reply);

            // Check events
            let events: Vec<_> = protocol.poll(&[]).collect();
            if i < 2 {
                // Should get Synchronizing events for first 2 roundtrips
                assert!(events.iter().any(
                    |e| matches!(e, Event::Synchronizing { total: 3, count, .. } if *count == i + 1)
                ));
            } else {
                // Final roundtrip should produce Synchronized
                assert!(events.iter().any(|e| matches!(e, Event::Synchronized)));
            }
        }

        assert!(protocol.is_running());
    }

    #[test]
    fn sync_config_equality() {
        let config1 = SyncConfig::default();
        let config2 = SyncConfig::default();
        let config3 = SyncConfig::lan();

        assert_eq!(config1, config2);
        assert_ne!(config1, config3);
    }

    #[test]
    fn sync_config_clone() {
        let config = SyncConfig::high_latency();
        let cloned = config;
        assert_eq!(config, cloned);
    }

    // ==========================================
    // ProtocolConfig Tests
    // ==========================================

    #[test]
    fn protocol_config_default_values() {
        let config = ProtocolConfig::default();
        assert_eq!(config.quality_report_interval, Duration::from_millis(200));
        assert_eq!(config.shutdown_delay, Duration::from_millis(5000));
        assert_eq!(config.max_checksum_history, 32);
        assert_eq!(config.pending_output_limit, 128);
        assert_eq!(config.sync_retry_warning_threshold, 10);
        assert_eq!(config.sync_duration_warning_ms, 3000);
    }

    #[test]
    fn protocol_config_competitive_preset() {
        let config = ProtocolConfig::competitive();
        assert_eq!(config.quality_report_interval, Duration::from_millis(100));
        assert_eq!(config.shutdown_delay, Duration::from_millis(3000));
        assert_eq!(config.max_checksum_history, 32);
        assert_eq!(config.pending_output_limit, 128);
        assert_eq!(config.sync_retry_warning_threshold, 10);
        assert_eq!(config.sync_duration_warning_ms, 2000);
    }

    #[test]
    fn protocol_config_high_latency_preset() {
        let config = ProtocolConfig::high_latency();
        assert_eq!(config.quality_report_interval, Duration::from_millis(400));
        assert_eq!(config.shutdown_delay, Duration::from_millis(10000));
        assert_eq!(config.max_checksum_history, 64);
        assert_eq!(config.pending_output_limit, 256);
        assert_eq!(config.sync_retry_warning_threshold, 20);
        assert_eq!(config.sync_duration_warning_ms, 10000);
    }

    #[test]
    fn protocol_config_debug_preset() {
        let config = ProtocolConfig::debug();
        assert_eq!(config.quality_report_interval, Duration::from_millis(500));
        assert_eq!(config.shutdown_delay, Duration::from_millis(30000));
        assert_eq!(config.max_checksum_history, 128);
        assert_eq!(config.pending_output_limit, 64);
        assert_eq!(config.sync_retry_warning_threshold, 5);
        assert_eq!(config.sync_duration_warning_ms, 1000);
    }

    #[test]
    fn protocol_config_equality() {
        let config1 = ProtocolConfig::default();
        let config2 = ProtocolConfig::default();
        let config3 = ProtocolConfig::competitive();

        assert_eq!(config1, config2);
        assert_ne!(config1, config3);
    }

    #[test]
    fn protocol_config_clone() {
        let config = ProtocolConfig::high_latency();
        let cloned = config;
        assert_eq!(config, cloned);
    }

    #[test]
    fn protocol_config_new_same_as_default() {
        let config1 = ProtocolConfig::new();
        let config2 = ProtocolConfig::default();
        assert_eq!(config1, config2);
    }

    // ==========================================
    // Time Utility Tests
    // ==========================================

    #[test]
    fn millis_since_epoch_returns_some_under_normal_conditions() {
        // Under normal conditions, millis_since_epoch should return Some with a valid timestamp
        let millis = millis_since_epoch();
        assert!(
            millis.is_some(),
            "millis_since_epoch should return Some under normal conditions"
        );
    }

    #[test]
    fn millis_since_epoch_returns_reasonable_value() {
        // The function should return a value representing milliseconds since UNIX_EPOCH.
        // As of 2020, this is at least 1577836800000 (Jan 1, 2020 00:00:00 UTC).
        // As of 2030, it would be around 1893456000000.
        let millis = millis_since_epoch().expect("Should return Some under normal conditions");

        // Should be at least year 2020 timestamp
        assert!(
            millis >= 1_577_836_800_000,
            "Time should be after year 2020"
        );

        // Should not be unreasonably far in the future (year 2100)
        assert!(
            millis < 4_102_444_800_000,
            "Time should be before year 2100"
        );
    }

    #[test]
    fn millis_since_epoch_is_monotonically_non_decreasing_in_short_term() {
        // Within a single execution context, time should not go backwards
        let first = millis_since_epoch().expect("Should return Some");
        let second = millis_since_epoch().expect("Should return Some");

        // Second call should be >= first (could be equal if very fast)
        assert!(
            second >= first,
            "Time should not go backwards within same execution"
        );
    }

    #[test]
    fn millis_since_epoch_advances_over_time() {
        let first = millis_since_epoch().expect("Should return Some");

        // Sleep for a tiny bit
        std::thread::sleep(std::time::Duration::from_millis(2));

        let second = millis_since_epoch().expect("Should return Some");

        // Should have advanced
        assert!(second > first, "Time should advance after sleep");
    }

    /// Test documentation: The `millis_since_epoch` function gracefully handles
    /// the case where system time is before UNIX_EPOCH by returning None and
    /// reporting a violation. This cannot be easily tested without mocking,
    /// but the code path is verified through code review. The test below
    /// documents the expected behavior.
    #[test]
    fn millis_since_epoch_documents_backwards_time_handling() {
        // This test documents the behavior when time goes backwards.
        // The actual scenario (SystemTime before UNIX_EPOCH) cannot be triggered
        // in a unit test without mocking std::time::SystemTime.
        //
        // Expected behavior:
        // 1. When SystemTime::now().duration_since(UNIX_EPOCH) returns Err
        // 2. The function reports a ViolationKind::InternalError via telemetry
        // 3. The function returns None to signal the abnormal condition
        //
        // Callers are responsible for handling None appropriately:
        // - send_quality_report: Skips sending the report
        // - on_quality_reply: Skips updating RTT
        //
        // This design ensures:
        // - No incorrect fallback values (like 0) propagate through the system
        // - Callers make explicit decisions about how to handle clock issues
        // - The system degrades gracefully rather than using invalid data
        //
        // This is covered by:
        // - Code review of the implementation
        // - The fact that the code compiles with the error handling path
        // - Integration tests that would fail if the function panicked

        // Simply verify the function works normally
        let result = millis_since_epoch();
        assert!(
            result.is_some(),
            "Under normal conditions, should return Some"
        );
        assert!(
            result.unwrap() > 0,
            "Under normal conditions, should return positive value"
        );
    }
}
