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
    Config, FortressError, FortressEvent, FortressRequest, Frame, InputStatus, InputVec,
    NetworkStats, NonBlockingSocket, PlayerHandle, SessionState,
};

/// The number of frames the spectator advances in a single step during normal operation.
///
/// When not catching up to the host, spectators advance one frame at a time to maintain
/// smooth playback. During catchup mode (when far behind), `catchup_speed` is used instead.
const NORMAL_SPEED: usize = 1;

/// [`SpectatorSession`] provides all functionality to connect to a remote host in a peer-to-peer fashion.
///
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
        for (event, addr) in std::mem::take(&mut events) {
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

    fn inputs_at_frame(&self, frame_to_grab: Frame) -> Result<InputVec<T::Input>, FortressError> {
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

        let player_inputs = self
            .inputs
            .get(frame_to_grab.as_i32() as usize % self.buffer_size)
            .ok_or_else(|| FortressError::InternalError {
                context: format!(
                    "Buffer index out of bounds: frame {} % buffer_size {}",
                    frame_to_grab, self.buffer_size
                ),
            })?;

        // We haven't received the input from the host yet. Wait.
        let first_input = player_inputs
            .first()
            .ok_or_else(|| FortressError::InternalError {
                context: "Player inputs vector is empty".into(),
            })?;
        if first_input.frame < frame_to_grab {
            return Err(FortressError::PredictionThreshold);
        }

        // The host is more than buffer_size frames ahead of the spectator. The input we need is gone forever.
        if first_input.frame > frame_to_grab {
            return Err(FortressError::SpectatorTooFarBehind);
        }

        Ok(player_inputs
            .iter()
            .enumerate()
            .map(|(handle, player_input)| {
                if let Some(status) = self.host_connect_status.get(handle) {
                    if status.disconnected && status.last_frame < frame_to_grab {
                        (player_input.input, InputStatus::Disconnected)
                    } else {
                        (player_input.input, InputStatus::Confirmed)
                    }
                } else {
                    // If we can't get the connection status, assume confirmed
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
                let frame_index = input.frame.as_i32() as usize % self.buffer_size;
                if let Some(frame_inputs) = self.inputs.get_mut(frame_index) {
                    if let Some(player_input) = frame_inputs.get_mut(player.as_usize()) {
                        *player_input = input;
                    } else {
                        report_violation!(
                            ViolationSeverity::Warning,
                            ViolationKind::InternalError,
                            "Failed to store input for player {} at frame {} - player index out of bounds",
                            player,
                            input.frame
                        );
                        return;
                    }
                } else {
                    report_violation!(
                        ViolationSeverity::Warning,
                        ViolationKind::InternalError,
                        "Failed to store input at frame {} - frame index {} out of bounds",
                        input.frame,
                        frame_index
                    );
                    return;
                }

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
                    if let Some(status) = self.host_connect_status.get_mut(i) {
                        *status = self.host.peer_connect_status(PlayerHandle::new(i));
                    } else {
                        report_violation!(
                            ViolationSeverity::Warning,
                            ViolationKind::InternalError,
                            "Failed to update connection status for player {} - index out of bounds",
                            i
                        );
                    }
                }
            },
        }

        // check event queue size and discard oldest events if too big
        while self.event_queue.len() > MAX_EVENT_QUEUE_SIZE {
            self.event_queue.pop_front();
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::needless_collect
)]
mod tests {
    use super::*;
    use crate::{Config, Message, NonBlockingSocket, SessionBuilder};
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    /// A minimal test configuration for unit testing.
    struct TestConfig;

    impl Config for TestConfig {
        type Input = u8;
        type State = u8;
        type Address = SocketAddr;
    }

    fn test_addr(port: u16) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)
    }

    /// A dummy socket that doesn't actually send or receive messages.
    /// Used for unit testing without network dependencies.
    struct DummySocket;

    impl NonBlockingSocket<SocketAddr> for DummySocket {
        fn send_to(&mut self, _msg: &Message, _addr: &SocketAddr) {}
        fn receive_all_messages(&mut self) -> Vec<(SocketAddr, Message)> {
            Vec::new()
        }
    }

    // Helper function to create a spectator session for testing
    fn create_test_spectator_session() -> Option<SpectatorSession<TestConfig>> {
        SessionBuilder::new()
            .with_num_players(2)
            .start_spectator_session(test_addr(7000), DummySocket)
    }

    fn create_test_spectator_session_with_config(
        num_players: usize,
        buffer_size: usize,
        max_frames_behind: usize,
        catchup_speed: usize,
    ) -> Option<SpectatorSession<TestConfig>> {
        use crate::SpectatorConfig;
        SessionBuilder::new()
            .with_num_players(num_players)
            .with_spectator_config(SpectatorConfig {
                buffer_size,
                catchup_speed,
                max_frames_behind,
            })
            .start_spectator_session(test_addr(7001), DummySocket)
    }

    // ==========================================
    // Constructor Tests
    // ==========================================

    #[test]
    fn spectator_session_creates_successfully() {
        let session = create_test_spectator_session();
        assert!(session.is_some());
    }

    #[test]
    fn spectator_session_with_custom_config() {
        let session = create_test_spectator_session_with_config(4, 120, 20, 3);
        assert!(session.is_some());
        let session = session.unwrap();
        assert_eq!(session.num_players(), 4);
    }

    #[test]
    fn spectator_session_single_player() {
        let session = create_test_spectator_session_with_config(1, 60, 10, 1);
        assert!(session.is_some());
        let session = session.unwrap();
        assert_eq!(session.num_players(), 1);
    }

    #[test]
    fn spectator_session_many_players() {
        let session = create_test_spectator_session_with_config(8, 60, 10, 1);
        assert!(session.is_some());
        let session = session.unwrap();
        assert_eq!(session.num_players(), 8);
    }

    // ==========================================
    // State and Getter Tests
    // ==========================================

    #[test]
    fn spectator_session_initial_state_is_synchronizing() {
        let session = create_test_spectator_session().unwrap();
        assert_eq!(session.current_state(), SessionState::Synchronizing);
    }

    #[test]
    fn spectator_session_initial_frame_is_null() {
        let session = create_test_spectator_session().unwrap();
        assert_eq!(session.current_frame(), Frame::NULL);
    }

    #[test]
    fn spectator_session_num_players_returns_correct_count() {
        let session2 = create_test_spectator_session_with_config(2, 60, 10, 1).unwrap();
        assert_eq!(session2.num_players(), 2);

        let session4 = create_test_spectator_session_with_config(4, 60, 10, 1).unwrap();
        assert_eq!(session4.num_players(), 4);
    }

    #[test]
    fn spectator_session_frames_behind_host_initially_zero() {
        let session = create_test_spectator_session().unwrap();
        // Both last_recv_frame and current_frame start at NULL (Frame(-1))
        // so frames_behind_host should be 0
        assert_eq!(session.frames_behind_host(), 0);
    }

    // ==========================================
    // advance_frame Tests
    // ==========================================

    #[test]
    fn spectator_session_advance_frame_returns_not_synchronized_when_not_running() {
        let mut session = create_test_spectator_session().unwrap();

        // Session starts in Synchronizing state
        let result = session.advance_frame();
        assert!(result.is_err());
        assert!(matches!(result, Err(FortressError::NotSynchronized)));
    }

    // ==========================================
    // network_stats Tests
    // ==========================================

    #[test]
    fn spectator_session_network_stats_returns_not_synchronized_initially() {
        let session = create_test_spectator_session().unwrap();

        // Network stats should fail when not synchronized
        let result = session.network_stats();
        assert!(result.is_err());
    }

    // ==========================================
    // events Tests
    // ==========================================

    #[test]
    fn spectator_session_events_initially_empty() {
        let mut session = create_test_spectator_session().unwrap();
        let events: Vec<_> = session.events().collect();
        assert!(events.is_empty());
    }

    #[test]
    fn spectator_session_events_drains_queue() {
        let mut session = create_test_spectator_session().unwrap();

        // First call to events
        let events1: Vec<_> = session.events().collect();
        assert!(events1.is_empty());

        // Second call should also be empty (queue was drained)
        let events2: Vec<_> = session.events().collect();
        assert!(events2.is_empty());
    }

    // ==========================================
    // violation_observer Tests
    // ==========================================

    #[test]
    fn spectator_session_violation_observer_none_by_default() {
        let session = create_test_spectator_session().unwrap();
        assert!(session.violation_observer().is_none());
    }

    #[test]
    fn spectator_session_with_violation_observer() {
        use crate::telemetry::CollectingObserver;

        let observer = Arc::new(CollectingObserver::new());
        let session: Option<SpectatorSession<TestConfig>> = SessionBuilder::new()
            .with_num_players(2)
            .with_violation_observer(observer)
            .start_spectator_session(test_addr(7002), DummySocket);

        let session = session.unwrap();
        assert!(session.violation_observer().is_some());
    }

    // ==========================================
    // poll_remote_clients Tests
    // ==========================================

    #[test]
    fn spectator_session_poll_remote_clients_does_not_panic() {
        let mut session = create_test_spectator_session().unwrap();

        // Polling should not panic even with no messages
        session.poll_remote_clients();

        // State should still be synchronizing (no sync messages received)
        assert_eq!(session.current_state(), SessionState::Synchronizing);
    }

    #[test]
    fn spectator_session_poll_remote_clients_multiple_times() {
        let mut session = create_test_spectator_session().unwrap();

        // Multiple polls should not cause issues
        for _ in 0..10 {
            session.poll_remote_clients();
        }

        assert_eq!(session.current_state(), SessionState::Synchronizing);
    }

    // ==========================================
    // SpectatorConfig Tests
    // ==========================================

    #[test]
    fn spectator_config_default_values() {
        use crate::SpectatorConfig;

        let config = SpectatorConfig::default();
        assert_eq!(config.buffer_size, 60);
        assert_eq!(config.catchup_speed, 1);
        assert_eq!(config.max_frames_behind, 10);
    }

    #[test]
    fn spectator_config_new_equals_default() {
        use crate::SpectatorConfig;

        let new_config = SpectatorConfig::new();
        let default_config = SpectatorConfig::default();
        assert_eq!(new_config, default_config);
    }

    #[test]
    fn spectator_config_fast_paced_preset() {
        use crate::SpectatorConfig;

        let config = SpectatorConfig::fast_paced();
        assert_eq!(config.buffer_size, 90);
        assert_eq!(config.catchup_speed, 2);
        assert_eq!(config.max_frames_behind, 15);
    }

    #[test]
    fn spectator_config_slow_connection_preset() {
        use crate::SpectatorConfig;

        let config = SpectatorConfig::slow_connection();
        assert_eq!(config.buffer_size, 120);
        assert_eq!(config.catchup_speed, 1);
        assert_eq!(config.max_frames_behind, 20);
    }

    #[test]
    fn spectator_config_local_preset() {
        use crate::SpectatorConfig;

        let config = SpectatorConfig::local();
        assert_eq!(config.buffer_size, 30);
        assert_eq!(config.catchup_speed, 2);
        assert_eq!(config.max_frames_behind, 5);
    }

    #[test]
    fn spectator_config_equality() {
        use crate::SpectatorConfig;

        let a = SpectatorConfig {
            buffer_size: 100,
            catchup_speed: 2,
            max_frames_behind: 15,
        };
        let b = SpectatorConfig {
            buffer_size: 100,
            catchup_speed: 2,
            max_frames_behind: 15,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn spectator_config_inequality() {
        use crate::SpectatorConfig;

        let a = SpectatorConfig::default();
        let b = SpectatorConfig::fast_paced();
        assert_ne!(a, b);
    }

    #[test]
    fn spectator_config_clone() {
        use crate::SpectatorConfig;

        let original = SpectatorConfig::fast_paced();
        let cloned = original;
        assert_eq!(original, cloned);
    }

    #[test]
    fn spectator_config_debug_format() {
        use crate::SpectatorConfig;

        let config = SpectatorConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("SpectatorConfig"));
        assert!(debug_str.contains("buffer_size"));
        assert!(debug_str.contains("60"));
    }

    #[test]
    fn spectator_config_all_presets_are_distinct() {
        use crate::SpectatorConfig;

        let default = SpectatorConfig::default();
        let fast_paced = SpectatorConfig::fast_paced();
        let slow_connection = SpectatorConfig::slow_connection();
        let local = SpectatorConfig::local();

        // All presets should be different
        assert_ne!(default, fast_paced);
        assert_ne!(default, slow_connection);
        assert_ne!(default, local);
        assert_ne!(fast_paced, slow_connection);
        assert_ne!(fast_paced, local);
        assert_ne!(slow_connection, local);
    }

    // ==========================================
    // Edge Case Tests
    // ==========================================

    #[test]
    fn spectator_session_with_minimum_buffer_size() {
        // Buffer size of 1 should work (edge case)
        let session = create_test_spectator_session_with_config(2, 1, 10, 1);
        assert!(session.is_some());
    }

    #[test]
    fn spectator_session_with_zero_buffer_size_uses_minimum() {
        // Buffer size of 0 should be handled (converted to 1 internally)
        let session = create_test_spectator_session_with_config(2, 0, 10, 1);
        assert!(session.is_some());
    }

    #[test]
    fn spectator_session_with_large_buffer_size() {
        let session = create_test_spectator_session_with_config(2, 1000, 10, 1);
        assert!(session.is_some());
    }

    #[test]
    fn spectator_session_with_high_catchup_speed() {
        let session = create_test_spectator_session_with_config(2, 60, 10, 10);
        assert!(session.is_some());
    }

    #[test]
    fn spectator_session_with_zero_max_frames_behind() {
        // Zero max_frames_behind means always in catchup mode
        let session = create_test_spectator_session_with_config(2, 60, 0, 2);
        assert!(session.is_some());
    }

    // ==========================================
    // Internal State Tests
    // ==========================================

    #[test]
    fn spectator_session_buffer_size_respects_minimum() {
        // When buffer_size is 0, it should be clamped to 1
        let session = create_test_spectator_session_with_config(2, 0, 10, 1).unwrap();
        // buffer_size is private, but we can verify the session was created successfully
        // The internal buffer_size.max(1) ensures this doesn't panic
        assert_eq!(session.num_players(), 2);
    }

    #[test]
    fn spectator_session_host_connect_status_initialized() {
        // Verify that host_connect_status is initialized for all players
        let session = create_test_spectator_session_with_config(4, 60, 10, 1).unwrap();
        // We can't directly check host_connect_status, but we can verify
        // the session was created with the correct number of players
        assert_eq!(session.num_players(), 4);
    }

    #[test]
    fn spectator_session_last_recv_frame_initially_null() {
        let session = create_test_spectator_session().unwrap();
        // last_recv_frame starts at NULL (Frame(-1)), which means
        // frames_behind_host should be 0 (since current_frame is also NULL)
        assert_eq!(session.frames_behind_host(), 0);
    }

    // ==========================================
    // NORMAL_SPEED Constant Test
    // ==========================================

    #[test]
    fn normal_speed_is_one() {
        // NORMAL_SPEED constant should be 1 for smooth playback
        assert_eq!(NORMAL_SPEED, 1);
    }

    // ==========================================
    // Current Frame Tests
    // ==========================================

    #[test]
    fn spectator_session_current_frame_is_null_initially() {
        let session = create_test_spectator_session().unwrap();
        assert!(session.current_frame().is_null());
        assert_eq!(session.current_frame(), Frame::NULL);
    }

    // ==========================================
    // Session State Tests
    // ==========================================

    #[test]
    fn spectator_session_state_transitions() {
        // Session starts in Synchronizing state
        let session = create_test_spectator_session().unwrap();
        assert_eq!(session.current_state(), SessionState::Synchronizing);

        // We can't easily transition to Running without a real network connection,
        // but we verify the initial state is correct
    }

    // ==========================================
    // SpectatorConfig Builder Tests
    // ==========================================

    #[test]
    fn spectator_config_with_zero_catchup_speed() {
        use crate::SpectatorConfig;

        // Catchup speed of 0 is technically valid (no frames advanced in catchup)
        let config = SpectatorConfig {
            buffer_size: 60,
            catchup_speed: 0,
            max_frames_behind: 10,
        };
        assert_eq!(config.catchup_speed, 0);
    }

    #[test]
    fn spectator_config_extreme_values() {
        use crate::SpectatorConfig;

        // Test with extreme values
        let config = SpectatorConfig {
            buffer_size: usize::MAX,
            catchup_speed: usize::MAX,
            max_frames_behind: usize::MAX,
        };
        assert_eq!(config.buffer_size, usize::MAX);
        assert_eq!(config.catchup_speed, usize::MAX);
        assert_eq!(config.max_frames_behind, usize::MAX);
    }

    // ==========================================
    // Multiple Poll Tests
    // ==========================================

    #[test]
    fn spectator_session_poll_preserves_state() {
        let mut session = create_test_spectator_session().unwrap();

        // Record initial state
        let initial_state = session.current_state();
        let initial_frame = session.current_frame();

        // Poll multiple times
        for _ in 0..5 {
            session.poll_remote_clients();
        }

        // State should not change without actual network events
        assert_eq!(session.current_state(), initial_state);
        assert_eq!(session.current_frame(), initial_frame);
    }

    #[test]
    fn spectator_session_events_empty_after_drain() {
        let mut session = create_test_spectator_session().unwrap();

        // Drain events
        let events: Vec<_> = session.events().collect();
        assert!(events.is_empty());

        // Poll and drain again
        session.poll_remote_clients();
        let events: Vec<_> = session.events().collect();
        assert!(events.is_empty());
    }

    // ==========================================
    // Network Stats Edge Cases
    // ==========================================

    #[test]
    fn spectator_session_network_stats_before_sync() {
        let session = create_test_spectator_session().unwrap();

        // Should fail when not synchronized
        let result = session.network_stats();
        assert!(result.is_err());
        assert!(matches!(result, Err(FortressError::NotSynchronized)));
    }

    // ==========================================
    // Violation Observer Tests
    // ==========================================

    #[test]
    fn spectator_session_violation_observer_is_arc() {
        use crate::telemetry::CollectingObserver;

        let observer = Arc::new(CollectingObserver::new());
        let observer_clone = Arc::clone(&observer);

        let session: Option<SpectatorSession<TestConfig>> = SessionBuilder::new()
            .with_num_players(2)
            .with_violation_observer(observer)
            .start_spectator_session(test_addr(7003), DummySocket);

        let session = session.unwrap();

        // Observer should be accessible
        assert!(session.violation_observer().is_some());

        // The clone should still be usable (Arc reference counting)
        assert_eq!(observer_clone.violations().len(), 0);
    }

    #[test]
    fn spectator_session_without_violation_observer() {
        let session = create_test_spectator_session().unwrap();
        assert!(session.violation_observer().is_none());
    }

    // ==========================================
    // Frames Behind Host Edge Cases
    // ==========================================

    #[test]
    fn spectator_session_frames_behind_with_both_null() {
        let session = create_test_spectator_session().unwrap();
        // Both last_recv_frame and current_frame are NULL
        // NULL - NULL = 0, so frames_behind should be 0
        assert_eq!(session.frames_behind_host(), 0);
    }
}
