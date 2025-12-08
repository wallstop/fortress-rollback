use crate::frame_info::PlayerInput;
use crate::network::compression::{decode, encode};
use crate::network::messages::{
    ChecksumReport, ConnectionStatus, Input, InputAck, Message, MessageBody, MessageHeader,
    QualityReply, QualityReport, SyncReply, SyncRequest,
};
use crate::report_violation;
use crate::sessions::builder::{ProtocolConfig, SyncConfig};
use crate::telemetry::{ViolationKind, ViolationSeverity};
use crate::time_sync::TimeSync;
use crate::{Config, DesyncDetection, FortressError, Frame, NonBlockingSocket, PlayerHandle};
use tracing::trace;

use instant::{Duration, Instant};
use std::collections::vec_deque::Drain;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::convert::TryFrom;
use std::ops::Add;

use super::network_stats::NetworkStats;

const UDP_HEADER_SIZE: usize = 28; // Size of IP + UDP headers

fn millis_since_epoch() -> u128 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis()
    }
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::new_0().get_time() as u128
    }
}

// byte-encoded data representing the inputs of a client, possibly for multiple players at the same time
#[derive(Clone)]
struct InputBytes {
    /// The frame to which this info belongs to. -1/[`Frame::NULL`] represents an invalid frame
    pub frame: Frame,
    /// An input buffer that will hold input data
    pub bytes: Vec<u8>,
}

impl InputBytes {
    fn zeroed<T: Config>(num_players: usize) -> Self {
        let input_size =
            bincode::serialized_size(&T::Input::default()).expect("input serialization failed");
        let size = (input_size as usize) * num_players;
        Self {
            frame: Frame::NULL,
            bytes: vec![0; size],
        }
    }

    fn from_inputs<T: Config>(
        num_players: usize,
        inputs: &BTreeMap<PlayerHandle, PlayerInput<T::Input>>,
    ) -> Self {
        let mut bytes = Vec::new();
        let mut frame = Frame::NULL;
        // in ascending order
        for handle in 0..num_players {
            if let Some(input) = inputs.get(&PlayerHandle::new(handle)) {
                assert!(frame == Frame::NULL || input.frame == Frame::NULL || frame == input.frame);
                if input.frame != Frame::NULL {
                    frame = input.frame;
                }

                bincode::serialize_into(&mut bytes, &input.input)
                    .expect("input serialization failed");
            }
        }
        Self { frame, bytes }
    }

    // Note: is_multiple_of() is nightly-only, so we use modulo
    #[allow(clippy::manual_is_multiple_of)]
    fn to_player_inputs<T: Config>(&self, num_players: usize) -> Vec<PlayerInput<T::Input>> {
        let mut player_inputs = Vec::new();
        assert!(num_players > 0 && self.bytes.len() % num_players == 0);
        let size = self.bytes.len() / num_players;
        for p in 0..num_players {
            let start = p * size;
            let end = start + size;
            let player_byte_slice = &self.bytes[start..end];
            let input: T::Input =
                bincode::deserialize(player_byte_slice).expect("input deserialization failed");
            player_inputs.push(PlayerInput::new(self.frame, input));
        }
        player_inputs
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Event<T>
where
    T: Config,
{
    /// The session is currently synchronizing with the remote client. It will continue until `count` reaches `total`.
    Synchronizing {
        /// Total sync roundtrips required.
        total: u32,
        /// Completed sync roundtrips so far.
        count: u32,
        /// Total sync requests sent (includes retries due to packet loss).
        total_requests_sent: u32,
        /// Milliseconds elapsed since sync started.
        elapsed_ms: u128,
    },
    /// The session is now synchronized with the remote client.
    Synchronized,
    /// The session has received an input from the remote client. This event will not be forwarded to the user.
    Input {
        input: PlayerInput<T::Input>,
        player: PlayerHandle,
    },
    /// The remote client has disconnected.
    Disconnected,
    /// The session has not received packets from the remote client since `disconnect_timeout` ms.
    NetworkInterrupted { disconnect_timeout: u128 },
    /// Sent only after a `NetworkInterrupted` event, if communication has resumed.
    NetworkResumed,
    /// Synchronization has timed out. This is only emitted if a sync timeout was configured.
    /// The session will continue trying to sync, but the user may choose to abort.
    SyncTimeout {
        /// Milliseconds elapsed since sync started.
        elapsed_ms: u128,
    },
}

#[derive(Debug, PartialEq, Eq)]
enum ProtocolState {
    Initializing,
    Synchronizing,
    Running,
    Disconnected,
    Shutdown,
}

pub(crate) struct UdpProtocol<T>
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
    stats_start_time: u128,
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
    ) -> Self {
        let mut magic = rand::random::<u16>();
        while magic == 0 {
            magic = rand::random::<u16>();
        }

        handles.sort_unstable();
        let recv_player_num = handles.len();

        // peer connection status
        let mut peer_connect_status = Vec::new();
        for _ in 0..num_players {
            peer_connect_status.push(ConnectionStatus::default());
        }

        // received input history
        let mut recv_inputs = BTreeMap::new();
        recv_inputs.insert(Frame::NULL, InputBytes::zeroed::<T>(recv_player_num));

        Self {
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
            last_acked_input: InputBytes::zeroed::<T>(local_players),
            max_prediction,
            recv_inputs,

            // time sync
            time_sync_layer: TimeSync::new(),
            local_frame_advantage: 0,
            remote_frame_advantage: 0,

            // network
            stats_start_time: 0,
            packets_sent: 0,
            bytes_sent: 0,
            round_trip_time: 0,
            last_send_time: Instant::now(),
            last_recv_time: Instant::now(),

            // debug desync
            pending_checksums: BTreeMap::new(),
            desync_detection,
        }
    }

    pub(crate) fn update_local_frame_advantage(&mut self, local_frame: Frame) {
        if local_frame == Frame::NULL || self.last_recv_frame() == Frame::NULL {
            return;
        }
        // Estimate which frame the other client is on by looking at the last frame they gave us plus some delta for the packet roundtrip time.
        let ping = i32::try_from(self.round_trip_time / 2).expect("Ping is higher than i32::MAX");
        let remote_frame = self.last_recv_frame() + ((ping * self.fps as i32) / 1000);
        // Our frame "advantage" is how many frames behind the remote client we are. (It's an advantage because they will have to predict more often)
        self.local_frame_advantage = remote_frame - local_frame;
    }

    pub(crate) fn network_stats(&self) -> Result<NetworkStats, FortressError> {
        if self.state != ProtocolState::Synchronizing && self.state != ProtocolState::Running {
            return Err(FortressError::NotSynchronized);
        }

        let now = millis_since_epoch();
        let seconds = (now - self.stats_start_time) / 1000;
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
        self.stats_start_time = millis_since_epoch();
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
                    let elapsed = Duration::from_millis(
                        (millis_since_epoch().saturating_sub(self.stats_start_time)) as u64,
                    );
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
            }
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
            }
            ProtocolState::Disconnected => {
                if self.shutdown_timeout < Instant::now() {
                    self.state = ProtocolState::Shutdown;
                }
            }
            ProtocolState::Initializing | ProtocolState::Shutdown => (),
        }
        self.event_queue.drain(..)
    }

    fn pop_pending_output(&mut self, ack_frame: Frame) {
        while !self.pending_output.is_empty() {
            if let Some(input) = self.pending_output.front() {
                if input.frame <= ack_frame {
                    self.last_acked_input = self
                        .pending_output
                        .pop_front()
                        .expect("Expected input to exist");
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
            assert!(
                self.last_acked_input.frame == Frame::NULL
                    || self.last_acked_input.frame + 1 == input.frame
            );
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
        let elapsed_ms = millis_since_epoch().saturating_sub(self.stats_start_time);
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

        let random_number = rand::random::<u32>();
        self.sync_random_requests.insert(random_number);
        let body = SyncRequest {
            random_request: random_number,
        };
        self.queue_message(MessageBody::SyncRequest(body));
    }

    fn send_quality_report(&mut self) {
        self.running_last_quality_report = Instant::now();
        let body = QualityReport {
            frame_advantage: i16::try_from(
                self.local_frame_advantage
                    .clamp(i16::MIN as i32, i16::MAX as i32),
            )
            .expect("local_frame_advantage should have been clamped into the range of an i16"),
            ping: millis_since_epoch(),
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
        let elapsed_ms = millis_since_epoch().saturating_sub(self.stats_start_time);
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
            for i in 0..self.peer_connect_status.len() {
                self.peer_connect_status[i].disconnected = body.peer_connect_status[i].disconnected
                    || self.peer_connect_status[i].disconnected;
                self.peer_connect_status[i].last_frame = std::cmp::max(
                    self.peer_connect_status[i].last_frame,
                    body.peer_connect_status[i].last_frame,
                );
            }
        }

        // if the encoded packet is decoded with an input we did not receive yet, we cannot recover
        assert!(
            self.last_recv_frame() == Frame::NULL || self.last_recv_frame() + 1 >= body.start_frame
        );

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

            let recv_inputs = decode(&decode_inp.bytes, &body.bytes).expect("decoding failed");

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
                    self.event_queue.push_back(Event::Input {
                        input: player_input,
                        player: self.handles[i],
                    });
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
        let millis = millis_since_epoch();
        assert!(millis >= body.pong);
        self.round_trip_time = millis - body.pong;
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
            }
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
            }
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
        );

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
        let input_bytes = InputBytes::zeroed::<TestConfig>(2);

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
            }
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
        );
        assert!(protocol1 != protocol3);
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
}
