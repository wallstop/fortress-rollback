use std::collections::{vec_deque::Drain, VecDeque};
use std::sync::Arc;

use crate::{
    frame_info::PlayerInput,
    network::{
        messages::ConnectionStatus,
        protocol::{Event, UdpProtocol},
    },
    report_violation,
    sessions::builder::MAX_EVENT_QUEUE_SIZE,
    telemetry::{ViolationKind, ViolationObserver, ViolationSeverity},
    Config, FortressError, FortressEvent, FortressRequest, Frame, InputStatus, NetworkStats,
    NonBlockingSocket, PlayerHandle, SessionState,
};

/// The number of frames the spectator advances in a single step during normal operation.
///
/// When not catching up to the host, spectators advance one frame at a time to maintain
/// smooth playback. During catchup mode (when far behind), `catchup_speed` is used instead.
const NORMAL_SPEED: usize = 1;

/// [`SpectatorSession`] provides all functionality to connect to a remote host in a peer-to-peer fashion.
/// The host will broadcast all confirmed inputs to this session.
/// This session can be used to spectate a session without contributing to the game input.
pub struct SpectatorSession<T>
where
    T: Config,
{
    state: SessionState,
    num_players: usize,
    buffer_size: usize,
    inputs: Vec<Vec<PlayerInput<T::Input>>>,
    host_connect_status: Vec<ConnectionStatus>,
    socket: Box<dyn NonBlockingSocket<T::Address>>,
    host: UdpProtocol<T>,
    event_queue: VecDeque<FortressEvent<T>>,
    current_frame: Frame,
    last_recv_frame: Frame,
    max_frames_behind: usize,
    catchup_speed: usize,
    /// Optional observer for specification violations.
    violation_observer: Option<Arc<dyn ViolationObserver>>,
}

impl<T: Config> SpectatorSession<T> {
    /// Creates a new [`SpectatorSession`] for a spectator.
    /// The session will receive inputs from all players from the given host directly.
    /// The session will use the provided socket.
    pub(crate) fn new(
        num_players: usize,
        socket: Box<dyn NonBlockingSocket<T::Address>>,
        host: UdpProtocol<T>,
        buffer_size: usize,
        max_frames_behind: usize,
        catchup_speed: usize,
        violation_observer: Option<Arc<dyn ViolationObserver>>,
    ) -> Self {
        // host connection status
        let mut host_connect_status = Vec::new();
        for _ in 0..num_players {
            host_connect_status.push(ConnectionStatus::default());
        }

        // Use at least 1 for buffer size to prevent panics
        let actual_buffer_size = buffer_size.max(1);

        Self {
            state: SessionState::Synchronizing,
            num_players,
            buffer_size: actual_buffer_size,
            inputs: vec![
                vec![PlayerInput::blank_input(Frame::NULL); num_players];
                actual_buffer_size
            ],
            host_connect_status,
            socket,
            host,
            event_queue: VecDeque::new(),
            current_frame: Frame::NULL,
            last_recv_frame: Frame::NULL,
            max_frames_behind,
            catchup_speed,
            violation_observer,
        }
    }

    /// Returns the current [`SessionState`] of a session.
    pub fn current_state(&self) -> SessionState {
        self.state
    }

    /// Returns the number of frames behind the host
    pub fn frames_behind_host(&self) -> usize {
        let diff = self.last_recv_frame - self.current_frame;
        // Gracefully handle the case where current_frame somehow exceeds last_recv_frame.
        // This shouldn't happen in normal operation, but we report it and return 0 rather than panic.
        if diff < 0 {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::FrameSync,
                "frames_behind_host: current_frame {} exceeds last_recv_frame {} - returning 0",
                self.current_frame,
                self.last_recv_frame
            );
            return 0;
        }
        diff as usize
    }

    /// Used to fetch some statistics about the quality of the network connection.
    /// # Errors
    /// - Returns [`NotSynchronized`] if the session is not connected to other clients yet.
    ///
    /// [`NotSynchronized`]: FortressError::NotSynchronized
    pub fn network_stats(&self) -> Result<NetworkStats, FortressError> {
        self.host.network_stats()
    }

    /// Returns all events that happened since last queried for events. If the number of stored events exceeds `MAX_EVENT_QUEUE_SIZE`, the oldest events will be discarded.
    pub fn events(&mut self) -> Drain<'_, FortressEvent<T>> {
        self.event_queue.drain(..)
    }

    /// Returns a reference to the violation observer, if one was configured.
    ///
    /// This allows checking for violations that occurred during session operations
    /// when using a [`CollectingObserver`] or similar.
    ///
    /// [`CollectingObserver`]: crate::telemetry::CollectingObserver
    pub fn violation_observer(&self) -> Option<&Arc<dyn ViolationObserver>> {
        self.violation_observer.as_ref()
    }

    /// You should call this to notify Fortress Rollback that you are ready to advance your gamestate by a single frame.
    /// Returns an order-sensitive [`Vec<FortressRequest>`]. You should fulfill all requests in the exact order they are provided.
    /// Failure to do so will cause panics later.
    ///
    /// # Errors
    /// - Returns [`NotSynchronized`] if the session is not yet ready to accept input.
    ///   In this case, you either need to start the session or wait for synchronization between clients.
    ///
    /// [`Vec<FortressRequest>`]: FortressRequest
    /// [`NotSynchronized`]: FortressError::NotSynchronized
    pub fn advance_frame(&mut self) -> Result<Vec<FortressRequest<T>>, FortressError> {
        // receive info from host, trigger events and send messages
        self.poll_remote_clients();

        if self.state != SessionState::Running {
            return Err(FortressError::NotSynchronized);
        }

        let frames_to_advance = if self.frames_behind_host() > self.max_frames_behind {
            self.catchup_speed
        } else {
            NORMAL_SPEED
        };

        // Pre-allocate for the expected number of frames to advance.
        // In normal operation this is 1, in catchup mode it's catchup_speed.
        let mut requests = Vec::with_capacity(frames_to_advance);

        for _ in 0..frames_to_advance {
            // get inputs for the next frame
            let frame_to_grab = self.current_frame + 1;
            let synced_inputs = self.inputs_at_frame(frame_to_grab)?;

            requests.push(FortressRequest::AdvanceFrame {
                inputs: synced_inputs,
            });

            // advance the frame, but only if grabbing the inputs succeeded
            self.current_frame += 1;
        }

        Ok(requests)
    }

    /// Receive UDP packages, distribute them to corresponding UDP endpoints, handle all occurring events and send all outgoing UDP packages.
    /// Should be called periodically by your application to give Fortress Rollback a chance to do internal work like packet transmissions.
    pub fn poll_remote_clients(&mut self) {
        // Get all udp packets and distribute them to associated endpoints.
        // The endpoints will handle their packets, which will trigger both events and UPD replies.
        for (from, msg) in &self.socket.receive_all_messages() {
            if self.host.is_handling_message(from) {
                self.host.handle_message(msg);
            }
        }

        // run host poll and get events. This will trigger additional UDP packets to be sent.
        let mut events = VecDeque::new();
        let addr = self.host.peer_addr();
        for event in self.host.poll(&self.host_connect_status) {
            events.push_back((event, addr.clone()));
        }

        // handle all events locally
        for (event, addr) in events.drain(..) {
            self.handle_event(event, addr);
        }

        // send out all pending UDP messages
        self.host.send_all_messages(&mut self.socket);
    }

    /// Returns the current frame of a session.
    pub fn current_frame(&self) -> Frame {
        self.current_frame
    }

    /// Returns the number of players this session was constructed with.
    pub fn num_players(&self) -> usize {
        self.num_players
    }

    fn inputs_at_frame(
        &self,
        frame_to_grab: Frame,
    ) -> Result<Vec<(T::Input, InputStatus)>, FortressError> {
        // Validate frame is valid before computing index
        if frame_to_grab.is_null() || frame_to_grab.as_i32() < 0 {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::FrameSync,
                "inputs_at_frame called with invalid frame {:?}",
                frame_to_grab
            );
            return Err(FortressError::InvalidFrame {
                frame: frame_to_grab,
                reason: "Frame is NULL or negative".to_string(),
            });
        }

        let player_inputs = &self.inputs[frame_to_grab.as_i32() as usize % self.buffer_size];

        // We haven't received the input from the host yet. Wait.
        if player_inputs[0].frame < frame_to_grab {
            return Err(FortressError::PredictionThreshold);
        }

        // The host is more than buffer_size frames ahead of the spectator. The input we need is gone forever.
        if player_inputs[0].frame > frame_to_grab {
            return Err(FortressError::SpectatorTooFarBehind);
        }

        Ok(player_inputs
            .iter()
            .enumerate()
            .map(|(handle, player_input)| {
                if self.host_connect_status[handle].disconnected
                    && self.host_connect_status[handle].last_frame < frame_to_grab
                {
                    (player_input.input, InputStatus::Disconnected)
                } else {
                    (player_input.input, InputStatus::Confirmed)
                }
            })
            .collect())
    }

    fn handle_event(&mut self, event: Event<T>, addr: T::Address) {
        match event {
            // forward to user
            Event::Synchronizing {
                total,
                count,
                total_requests_sent,
                elapsed_ms,
            } => {
                self.event_queue.push_back(FortressEvent::Synchronizing {
                    addr,
                    total,
                    count,
                    total_requests_sent,
                    elapsed_ms,
                });
            },
            // forward to user
            Event::NetworkInterrupted { disconnect_timeout } => {
                self.event_queue
                    .push_back(FortressEvent::NetworkInterrupted {
                        addr,
                        disconnect_timeout,
                    });
            },
            // forward to user
            Event::NetworkResumed => {
                self.event_queue
                    .push_back(FortressEvent::NetworkResumed { addr });
            },
            // synced with the host, then forward to user
            Event::Synchronized => {
                self.state = SessionState::Running;
                self.event_queue
                    .push_back(FortressEvent::Synchronized { addr });
            },
            // disconnect the player, then forward to user
            Event::Disconnected => {
                self.event_queue
                    .push_back(FortressEvent::Disconnected { addr });
            },
            // forward sync timeout to user
            Event::SyncTimeout { elapsed_ms } => {
                self.event_queue
                    .push_back(FortressEvent::SyncTimeout { addr, elapsed_ms });
            },
            // add the input and all associated information
            Event::Input { input, player } => {
                // Validate frame before using as index - negative frames would wrap around
                if input.frame.is_null() || input.frame.as_i32() < 0 {
                    report_violation!(
                        ViolationSeverity::Warning,
                        ViolationKind::FrameSync,
                        "Received input with invalid frame {:?} for player {} - ignoring",
                        input.frame,
                        player
                    );
                    return;
                }

                // Validate player handle is in bounds
                if player.as_usize() >= self.num_players {
                    report_violation!(
                        ViolationSeverity::Warning,
                        ViolationKind::InternalError,
                        "Received input for player {} but only {} players configured - ignoring",
                        player,
                        self.num_players
                    );
                    return;
                }

                // save the input
                self.inputs[input.frame.as_i32() as usize % self.buffer_size][player.as_usize()] =
                    input;

                // Validate frame ordering - should receive frames in order
                if input.frame < self.last_recv_frame {
                    report_violation!(
                        ViolationSeverity::Warning,
                        ViolationKind::FrameSync,
                        "Received out-of-order input: frame {} is older than last_recv_frame {}",
                        input.frame,
                        self.last_recv_frame
                    );
                    // Still update if this is a newer frame than what we had
                }
                if input.frame > self.last_recv_frame {
                    self.last_recv_frame = input.frame;
                }

                // update the frame advantage
                self.host.update_local_frame_advantage(input.frame);

                // update the host connection status
                for i in 0..self.num_players {
                    self.host_connect_status[i] =
                        self.host.peer_connect_status(PlayerHandle::new(i));
                }
            },
        }

        // check event queue size and discard oldest events if too big
        while self.event_queue.len() > MAX_EVENT_QUEUE_SIZE {
            self.event_queue.pop_front();
        }
    }
}
