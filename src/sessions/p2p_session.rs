use crate::error::FortressError;
use crate::frame_info::PlayerInput;
use crate::network::messages::ConnectionStatus;
use crate::network::network_stats::NetworkStats;
use crate::network::protocol::UdpProtocol;
use crate::report_violation;
use crate::sessions::builder::{ProtocolConfig, SaveMode};
use crate::sync_layer::SyncLayer;
use crate::telemetry::{ViolationKind, ViolationObserver, ViolationSeverity};
use crate::DesyncDetection;
use crate::{
    network::protocol::Event, Config, FortressEvent, FortressRequest, Frame, NonBlockingSocket,
    PlayerHandle, PlayerType, SessionState,
};
use tracing::{debug, trace};

use std::collections::vec_deque::Drain;
use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::convert::TryInto;
use std::sync::Arc;

/// Minimum frames between [`FortressEvent::WaitRecommendation`] events.
///
/// Set to 60 (1 second at 60fps) to avoid spamming the user with frequent
/// wait suggestions. This prevents the event queue from being overwhelmed
/// with wait recommendations during network instability.
const RECOMMENDATION_INTERVAL: Frame = Frame::new(60);

/// Minimum recommended frames to wait when behind.
///
/// When the session calculates that the local player should wait for
/// remote players to catch up, this ensures the recommendation is at
/// least 3 frames. This avoids micro-stuttering from very small waits
/// and provides enough time for network conditions to improve.
const MIN_RECOMMENDATION: u32 = 3;

/// Maximum number of events to queue before oldest are dropped.
///
/// This prevents unbounded memory growth if events aren't being consumed.
/// At 100 events, there's ample buffer for typical network jitter while
/// providing backpressure if the application isn't processing events.
const MAX_EVENT_QUEUE_SIZE: usize = 100;

/// Registry tracking all players and their connection states.
///
/// # Note
///
/// This type is re-exported in [`__internal`](crate::__internal) for testing and fuzzing.
/// It is not part of the stable public API.
pub struct PlayerRegistry<T>
where
    T: Config,
{
    /// Map from player handles to their types.
    pub handles: BTreeMap<PlayerHandle, PlayerType<T::Address>>,
    /// Map from addresses to protocol handlers for remote players.
    pub remotes: BTreeMap<T::Address, UdpProtocol<T>>,
    /// Map from addresses to protocol handlers for spectators.
    pub spectators: BTreeMap<T::Address, UdpProtocol<T>>,
}

impl<T> std::fmt::Debug for PlayerRegistry<T>
where
    T: Config,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Destructure to ensure all fields are included when new fields are added.
        let Self {
            handles,
            remotes,
            spectators,
        } = self;

        f.debug_struct("PlayerRegistry")
            .field("handles", handles)
            .field("remotes", &remotes.keys())
            .field("spectators", &spectators.keys())
            .finish()
    }
}

impl<T: Config> PlayerRegistry<T> {
    /// Creates a new empty player registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            handles: BTreeMap::new(),
            remotes: BTreeMap::new(),
            spectators: BTreeMap::new(),
        }
    }

    /// Returns handles for all local players.
    #[must_use]
    pub fn local_player_handles(&self) -> Vec<PlayerHandle> {
        self.handles
            .iter()
            .filter_map(|(k, v)| match v {
                PlayerType::Local => Some(*k),
                PlayerType::Remote(_) => None,
                PlayerType::Spectator(_) => None,
            })
            .collect()
    }

    /// Returns handles for all remote players.
    #[must_use]
    pub fn remote_player_handles(&self) -> Vec<PlayerHandle> {
        self.handles
            .iter()
            .filter_map(|(k, v)| match v {
                PlayerType::Local => None,
                PlayerType::Remote(_) => Some(*k),
                PlayerType::Spectator(_) => None,
            })
            .collect()
    }

    /// Returns handles for all spectators.
    #[must_use]
    pub fn spectator_handles(&self) -> Vec<PlayerHandle> {
        self.handles
            .iter()
            .filter_map(|(k, v)| match v {
                PlayerType::Local => Some(*k),
                PlayerType::Remote(_) => None,
                PlayerType::Spectator(_) => Some(*k),
            })
            .collect()
    }

    /// Returns the number of players (local + remote, excluding spectators).
    #[must_use]
    pub fn num_players(&self) -> usize {
        self.handles
            .iter()
            .filter(|(_, v)| matches!(v, PlayerType::Local | PlayerType::Remote(_)))
            .count()
    }

    /// Returns the number of spectators.
    #[must_use]
    pub fn num_spectators(&self) -> usize {
        self.handles
            .iter()
            .filter(|(_, v)| matches!(v, PlayerType::Spectator(_)))
            .count()
    }

    /// Returns all handles associated with a given address.
    pub fn handles_by_address(&self, addr: T::Address) -> Vec<PlayerHandle> {
        let handles: Vec<PlayerHandle> = self
            .handles
            .iter()
            .filter_map(|(h, player_type)| match player_type {
                PlayerType::Local => None,
                PlayerType::Remote(a) => Some((h, a)),
                PlayerType::Spectator(a) => Some((h, a)),
            })
            .filter_map(|(h, a)| if addr == *a { Some(*h) } else { None })
            .collect();
        handles
    }
}

impl<T: Config> Default for PlayerRegistry<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// A [`P2PSession`] provides all functionality to connect to remote clients in a peer-to-peer fashion, exchange inputs and handle the gamestate by saving, loading and advancing.
pub struct P2PSession<T>
where
    T: Config,
{
    /// The number of players of the session.
    num_players: usize,
    /// The maximum number of frames Fortress Rollback will roll back. Every gamestate older than this is guaranteed to be correct.
    max_prediction: usize,
    /// The sync layer handles player input queues and provides predictions.
    sync_layer: SyncLayer<T>,
    /// Controls how game states are saved for rollback.
    save_mode: SaveMode,

    /// If we receive a disconnect from another client, we have to rollback from that frame on in order to prevent wrong predictions
    disconnect_frame: Frame,

    /// Internal State of the Session.
    state: SessionState,

    /// The [`P2PSession`] uses this socket to send and receive all messages for remote players.
    socket: Box<dyn NonBlockingSocket<T::Address>>,
    /// Handles players and their endpoints
    player_reg: PlayerRegistry<T>,
    /// This struct contains information about remote players, like connection status and the frame of last received input.
    local_connect_status: Vec<ConnectionStatus>,

    /// notes which inputs have already been sent to the spectators
    next_spectator_frame: Frame,
    /// The soonest frame on which the session can send a [`FortressEvent::WaitRecommendation`] again.
    next_recommended_sleep: Frame,
    /// How many frames we estimate we are ahead of every remote client
    frames_ahead: i32,

    /// Contains all events to be forwarded to the user.
    event_queue: VecDeque<FortressEvent<T>>,
    /// Contains all local inputs not yet sent into the system. This should have inputs for every local player before calling advance_frame
    local_inputs: BTreeMap<PlayerHandle, PlayerInput<T::Input>>,

    /// With desync detection, the session will compare checksums for all peers to detect discrepancies / desyncs between peers
    desync_detection: DesyncDetection,
    /// Desync detection over the network
    local_checksum_history: BTreeMap<Frame, u128>,
    /// The last frame we sent a checksum for
    last_sent_checksum_frame: Frame,
    /// Optional observer for specification violations.
    violation_observer: Option<Arc<dyn ViolationObserver>>,
    /// Protocol configuration for network behavior.
    protocol_config: ProtocolConfig,
}

impl<T: Config> P2PSession<T> {
    /// Creates a new [`P2PSession`] for players who participate on the game input. After creating the session, add local and remote players,
    /// set input delay for local players and then start the session. The session will use the provided socket.
    ///
    /// Note: This is an internal constructor called via SessionBuilder. The many parameters are
    /// acceptable here because users interact through the builder pattern, not this method directly.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        num_players: usize,
        max_prediction: usize,
        socket: Box<dyn NonBlockingSocket<T::Address>>,
        players: PlayerRegistry<T>,
        save_mode: SaveMode,
        desync_detection: DesyncDetection,
        input_delay: usize,
        violation_observer: Option<Arc<dyn ViolationObserver>>,
        protocol_config: ProtocolConfig,
        queue_length: usize,
    ) -> Self {
        // local connection status
        let mut local_connect_status = Vec::new();
        for _ in 0..num_players {
            local_connect_status.push(ConnectionStatus::default());
        }

        // sync layer & set input delay
        let mut sync_layer =
            SyncLayer::with_queue_length(num_players, max_prediction, queue_length);
        for (player_handle, player_type) in players.handles.iter() {
            if let PlayerType::Local = player_type {
                // This should never fail during construction as player handles are validated
                if let Err(e) = sync_layer.set_frame_delay(*player_handle, input_delay) {
                    report_violation!(
                        ViolationSeverity::Critical,
                        ViolationKind::InternalError,
                        "Failed to set frame delay for player {:?} during session construction: {}",
                        player_handle,
                        e
                    );
                }
            }
        }

        // initial session state - if there are no endpoints, we don't need a synchronization phase
        let state = if players.remotes.len() + players.spectators.len() == 0 {
            SessionState::Running
        } else {
            SessionState::Synchronizing
        };

        let save_mode = if max_prediction == 0 && save_mode == SaveMode::Sparse {
            // in lockstep mode, saving will never happen, but we use the last saved frame to mark
            // control marking frames confirmed, so we need to turn off sparse saving to ensure that
            // frames are marked as confirmed - otherwise we will never advance the game state.
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::Configuration,
                "Sparse saving setting is ignored because lockstep mode is on (max_prediction set to 0), so no saving will take place"
            );
            SaveMode::EveryFrame
        } else {
            save_mode
        };

        Self {
            state,
            num_players,
            max_prediction,
            save_mode,
            socket,
            local_connect_status,
            next_recommended_sleep: Frame::new(0),
            next_spectator_frame: Frame::new(0),
            frames_ahead: 0,
            sync_layer,
            disconnect_frame: Frame::NULL,
            player_reg: players,
            event_queue: VecDeque::new(),
            local_inputs: BTreeMap::new(),
            desync_detection,
            local_checksum_history: BTreeMap::new(),
            last_sent_checksum_frame: Frame::NULL,
            violation_observer,
            protocol_config,
        }
    }

    /// Registers local input for a player for the current frame. This should be successfully called for every local player before calling [`advance_frame()`].
    /// If this is called multiple times for the same player before advancing the frame, older given inputs will be overwritten.
    ///
    /// # Errors
    /// - Returns [`InvalidRequest`] when the given handle does not refer to a local player.
    ///
    /// [`advance_frame()`]: Self#method.advance_frame
    /// [`InvalidRequest`]: FortressError::InvalidRequest
    pub fn add_local_input(
        &mut self,
        player_handle: PlayerHandle,
        input: T::Input,
    ) -> Result<(), FortressError> {
        // make sure the input is for a registered local player
        if !self
            .player_reg
            .local_player_handles()
            .contains(&player_handle)
        {
            return Err(FortressError::InvalidRequest {
                info: "The player handle you provided is not referring to a local player."
                    .to_owned(),
            });
        }
        let player_input = PlayerInput::<T::Input>::new(self.sync_layer.current_frame(), input);
        self.local_inputs.insert(player_handle, player_input);
        Ok(())
    }

    /// You should call this to notify Fortress Rollback that you are ready to advance your gamestate by a single frame.
    /// Returns an order-sensitive [`Vec<FortressRequest>`]. You should fulfill all requests in the exact order they are provided.
    /// Failure to do so will cause panics later.
    ///
    /// # Errors
    /// - Returns [`InvalidRequest`] if the provided player handle refers to a remote player.
    /// - Returns [`NotSynchronized`] if the session is not yet ready to accept input. In this case, you either need to start the session or wait for synchronization between clients.
    ///
    /// [`Vec<FortressRequest>`]: FortressRequest
    /// [`InvalidRequest`]: FortressError::InvalidRequest
    /// [`NotSynchronized`]: FortressError::NotSynchronized
    pub fn advance_frame(&mut self) -> Result<Vec<FortressRequest<T>>, FortressError> {
        // receive info from remote players, trigger events and send messages
        self.poll_remote_clients();

        // session is not running and synchronized
        if self.state != SessionState::Running {
            trace!("Session not synchronized; returning error");
            return Err(FortressError::NotSynchronized);
        }

        // check if input for all local players is queued
        for handle in self.player_reg.local_player_handles() {
            if !self.local_inputs.contains_key(&handle) {
                return Err(FortressError::InvalidRequest {
                    info: format!(
                        "Missing local input for handle {handle} while calling advance_frame()."
                    ),
                });
            }
        }

        /*
         *  DESYNC DETECTION
         */
        // Collect, send, compare and check the last checksums against the other peers. The timing
        // of this is important: since the checksum comparison looks at the current confirmed frame,
        // (and the sync layer will happily mark a frame as confirmed after requesting rollback and
        // resimulation of it, at which point that frame's new checksum will not be stored yet), we
        // must examine our checksum state *before* the sync layer is able to mark any frames as
        // confirmed.
        if self.desync_detection != DesyncDetection::Off {
            self.check_checksum_send_interval();
            self.compare_local_checksums_against_peers();
        }

        // This list of requests will be returned to the user.
        // Pre-allocate with capacity for typical case: 1 save + 1 advance = 2 requests.
        // During rollback, more requests will be added but Vec will grow as needed.
        let mut requests = Vec::with_capacity(2);

        /*
         * ROLLBACKS AND GAME STATE MANAGEMENT
         */

        // if in lockstep mode, we will only ever request to advance the frame when all inputs for
        // the current frame have been confirmed; therefore there's no need to roll back, and hence
        // no need to ever save the game state either.
        let lockstep = self.in_lockstep_mode();

        // if we are in the first frame, we have to save the state
        if self.sync_layer.current_frame() == 0 && !lockstep {
            trace!("Saving state of first frame");
            requests.push(self.sync_layer.save_current_state());
        }

        // propagate disconnects to multiple players
        self.update_player_disconnects();

        // find the confirmed frame for which we received all inputs
        let confirmed_frame = self.confirmed_frame();

        // check game consistency and roll back, if necessary
        if !lockstep {
            // the disconnect frame indicates if a rollback is necessary due to a previously
            // disconnected player (whose input would have been incorrectly predicted).
            let first_incorrect = self
                .sync_layer
                .check_simulation_consistency(self.disconnect_frame);
            // if we have an incorrect frame, then we need to rollback
            if first_incorrect != Frame::NULL {
                self.adjust_gamestate(first_incorrect, confirmed_frame, &mut requests)?;
                self.disconnect_frame = Frame::NULL;
            }

            // request gamestate save of current frame
            let last_saved = self.sync_layer.last_saved_frame();
            if self.save_mode == SaveMode::Sparse {
                self.check_last_saved_state(last_saved, confirmed_frame, &mut requests)?;
            } else {
                // without sparse saving, always save the current frame after correcting and rollbacking
                requests.push(self.sync_layer.save_current_state());
            }
        }

        /*
         *  SEND OFF AND THROW AWAY INPUTS BEFORE THE CONFIRMED FRAME
         */

        // send confirmed inputs to spectators before throwing them away
        self.send_confirmed_inputs_to_spectators(confirmed_frame)?;

        // set the last confirmed frame and discard all saved inputs before that frame
        self.sync_layer
            .set_last_confirmed_frame(confirmed_frame, self.save_mode);

        /*
         *  WAIT RECOMMENDATION
         */

        // check time sync between clients and send wait recommendation, if appropriate
        self.check_wait_recommendation();

        /*
         *  INPUTS
         */

        // register local inputs in the system and send them
        for handle in self.player_reg.local_player_handles() {
            // we have checked that these all exist above, but return error for safety
            let player_input =
                self.local_inputs
                    .get_mut(&handle)
                    .ok_or_else(|| FortressError::MissingInput {
                        player_handle: handle,
                        frame: self.sync_layer.current_frame(),
                    })?;
            // send the input into the sync layer
            let actual_frame = self.sync_layer.add_local_input(handle, *player_input);
            player_input.frame = actual_frame;
            // if the input has not been dropped
            if actual_frame != Frame::NULL {
                self.local_connect_status[handle.as_usize()].last_frame = actual_frame;
            }
        }

        // if the local inputs have not been dropped by the sync layer, send to all remote clients
        if !self.local_inputs.values().any(|&i| i.frame == Frame::NULL) {
            for endpoint in self.player_reg.remotes.values_mut() {
                endpoint.send_input(&self.local_inputs, &self.local_connect_status);
                endpoint.send_all_messages(&mut self.socket);
            }
        }

        /*
         * ADVANCE THE STATE
         */

        let can_advance = if lockstep {
            // lockstep mode: only advance if the current frame has inputs confirmed from all other
            // players.
            self.sync_layer.last_confirmed_frame() == self.sync_layer.current_frame()
        } else {
            // rollback mode: advance as long as we aren't past our prediction window
            let frames_ahead = if self.sync_layer.last_confirmed_frame().is_null() {
                // we haven't had any frames confirmed, so all frames we've advanced are "ahead"
                self.sync_layer.current_frame().as_i32()
            } else {
                // we're not at the first frame, so we have to subtract the last confirmed frame
                self.sync_layer.current_frame() - self.sync_layer.last_confirmed_frame()
            };
            frames_ahead < self.max_prediction as i32
        };
        if can_advance {
            // get correct inputs for the current frame
            let inputs = match self
                .sync_layer
                .synchronized_inputs(&self.local_connect_status)
            {
                Some(inputs) => inputs,
                None => {
                    report_violation!(
                        ViolationSeverity::Critical,
                        ViolationKind::InternalError,
                        "Failed to get synchronized inputs for frame {}",
                        self.sync_layer.current_frame()
                    );
                    return Err(FortressError::InternalError {
                        context: "Failed to get synchronized inputs".to_owned(),
                    });
                },
            };
            // advance the frame count
            self.sync_layer.advance_frame();
            // clear the local inputs after advancing the frame to allow new inputs to be ingested
            self.local_inputs.clear();
            requests.push(FortressRequest::AdvanceFrame { inputs });
        } else {
            debug!(
                "Prediction Threshold reached. Skipping on frame {}",
                self.sync_layer.current_frame()
            );
        }

        Ok(requests)
    }

    /// Should be called periodically by your application to give Fortress Rollback a chance to do internal work.
    /// Fortress Rollback will receive packets, distribute them to corresponding endpoints, handle all occurring events and send all outgoing packets.
    pub fn poll_remote_clients(&mut self) {
        // Get all packets and distribute them to associated endpoints.
        // The endpoints will handle their packets, which will trigger both events and UPD replies.
        for (from_addr, msg) in &self.socket.receive_all_messages() {
            if let Some(endpoint) = self.player_reg.remotes.get_mut(from_addr) {
                endpoint.handle_message(msg);
            }
            if let Some(endpoint) = self.player_reg.spectators.get_mut(from_addr) {
                endpoint.handle_message(msg);
            }
        }

        // update frame information between remote players
        for remote_endpoint in self.player_reg.remotes.values_mut() {
            if remote_endpoint.is_running() {
                remote_endpoint.update_local_frame_advantage(self.sync_layer.current_frame());
            }
        }

        // run endpoint poll and get events from players and spectators. This will trigger additional packets to be sent.
        let mut events = VecDeque::new();
        for endpoint in self.player_reg.remotes.values_mut() {
            let handles = endpoint.handles().clone();
            let addr = endpoint.peer_addr();
            for event in endpoint.poll(&self.local_connect_status) {
                events.push_back((event, handles.clone(), addr.clone()))
            }
        }
        for endpoint in self.player_reg.spectators.values_mut() {
            let handles = endpoint.handles().clone();
            let addr = endpoint.peer_addr();
            for event in endpoint.poll(&self.local_connect_status) {
                events.push_back((event, handles.clone(), addr.clone()))
            }
        }

        // handle all events locally
        for (event, handles, addr) in events.drain(..) {
            self.handle_event(event, handles, addr);
        }

        // send all queued packets
        for endpoint in self.player_reg.remotes.values_mut() {
            endpoint.send_all_messages(&mut self.socket);
        }
        for endpoint in self.player_reg.spectators.values_mut() {
            endpoint.send_all_messages(&mut self.socket);
        }
    }

    /// Disconnects a remote player and all other remote players with the same address from the session.
    /// # Errors
    /// - Returns [`InvalidRequest`] if you try to disconnect a local player or the provided handle is invalid.
    ///
    /// [`InvalidRequest`]: FortressError::InvalidRequest
    pub fn disconnect_player(&mut self, player_handle: PlayerHandle) -> Result<(), FortressError> {
        match self.player_reg.handles.get(&player_handle) {
            // the local player cannot be disconnected
            None => Err(FortressError::InvalidRequest {
                info: "Invalid Player Handle.".to_owned(),
            }),
            Some(PlayerType::Local) => Err(FortressError::InvalidRequest {
                info: "Local Player cannot be disconnected.".to_owned(),
            }),
            // a remote player can only be disconnected if not already disconnected, since there is some additional logic attached
            Some(PlayerType::Remote(_)) => {
                if !self.local_connect_status[player_handle.as_usize()].disconnected {
                    let last_frame = self.local_connect_status[player_handle.as_usize()].last_frame;
                    self.disconnect_player_at_frame(player_handle, last_frame);
                    return Ok(());
                }
                Err(FortressError::InvalidRequest {
                    info: "Player already disconnected.".to_owned(),
                })
            },
            // disconnecting spectators is simpler
            Some(PlayerType::Spectator(_)) => {
                self.disconnect_player_at_frame(player_handle, Frame::NULL);
                Ok(())
            },
        }
    }

    /// Returns a [`NetworkStats`] struct that gives information about the quality of the network connection.
    /// # Errors
    /// - Returns [`InvalidRequest`] if the handle not referring to a remote player or spectator.
    /// - Returns [`NotSynchronized`] if the session is not connected to other clients yet.
    ///
    /// [`InvalidRequest`]: FortressError::InvalidRequest
    /// [`NotSynchronized`]: FortressError::NotSynchronized
    pub fn network_stats(
        &self,
        player_handle: PlayerHandle,
    ) -> Result<NetworkStats, FortressError> {
        match self.player_reg.handles.get(&player_handle) {
            Some(PlayerType::Remote(addr)) => match self.player_reg.remotes.get(addr) {
                Some(endpoint) => endpoint.network_stats(),
                None => Err(FortressError::InternalError {
                    context: format!(
                        "Endpoint not found for registered remote player at {:?}",
                        addr
                    ),
                }),
            },
            Some(PlayerType::Spectator(addr)) => match self.player_reg.remotes.get(addr) {
                Some(endpoint) => endpoint.network_stats(),
                None => Err(FortressError::InternalError {
                    context: format!("Endpoint not found for registered spectator at {:?}", addr),
                }),
            },
            _ => Err(FortressError::InvalidRequest {
                info: "Given player handle not referring to a remote player or spectator"
                    .to_owned(),
            }),
        }
    }

    /// Returns the highest confirmed frame. We have received all input for this frame and it is thus correct.
    #[must_use]
    pub fn confirmed_frame(&self) -> Frame {
        let mut confirmed_frame = Frame::new(i32::MAX);

        for con_stat in &self.local_connect_status {
            if !con_stat.disconnected {
                confirmed_frame = std::cmp::min(confirmed_frame, con_stat.last_frame);
            }
        }

        // If all players are disconnected, this should not happen in a running session
        if confirmed_frame.as_i32() == i32::MAX {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::FrameSync,
                "No connected players found when computing confirmed_frame - returning 0 as fallback"
            );
            return Frame::new(0);
        }
        confirmed_frame
    }

    /// Returns the current frame of a session.
    #[must_use]
    pub fn current_frame(&self) -> Frame {
        self.sync_layer.current_frame()
    }

    /// Returns the maximum prediction window of a session.
    #[must_use]
    pub fn max_prediction(&self) -> usize {
        self.max_prediction
    }

    /// Returns true if the session is running in lockstep mode.
    ///
    /// In lockstep mode, a session will only advance if the current frame has inputs confirmed from
    /// all other players.
    #[must_use]
    pub fn in_lockstep_mode(&mut self) -> bool {
        self.max_prediction == 0
    }

    /// Returns the current [`SessionState`] of a session.
    #[must_use]
    pub fn current_state(&self) -> SessionState {
        self.state
    }

    /// Returns all events that happened since last queried for events. If the number of stored events exceeds `MAX_EVENT_QUEUE_SIZE`, the oldest events will be discarded.
    #[must_use]
    pub fn events(&mut self) -> Drain<'_, FortressEvent<T>> {
        self.event_queue.drain(..)
    }

    /// Returns the confirmed inputs for all players at a specific frame.
    ///
    /// This is useful for computing deterministic checksums over confirmed game state.
    /// The returned inputs are guaranteed to be the same across all peers for the same frame,
    /// making them suitable for desync detection and verification.
    ///
    /// # Arguments
    ///
    /// * `frame` - The frame to get confirmed inputs for. Must be <= `confirmed_frame()`.
    ///
    /// # Returns
    ///
    /// A vector of inputs for each player, in player handle order (0, 1, 2, ...).
    /// Returns an error if the frame is not confirmed yet or has been discarded.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let confirmed = session.confirmed_frame();
    /// if confirmed.as_i32() >= 100 {
    ///     let inputs = session.confirmed_inputs_for_frame(Frame::new(100))?;
    ///     // These inputs are deterministic across all peers
    ///     let checksum = compute_checksum(&inputs);
    /// }
    /// ```
    pub fn confirmed_inputs_for_frame(&self, frame: Frame) -> Result<Vec<T::Input>, FortressError> {
        if frame > self.confirmed_frame() {
            return Err(FortressError::InvalidFrame {
                frame,
                reason: format!(
                    "Frame {} is not confirmed yet (confirmed_frame = {})",
                    frame,
                    self.confirmed_frame()
                ),
            });
        }
        self.sync_layer
            .confirmed_inputs(frame, &self.local_connect_status)
            .map(|inputs| inputs.into_iter().map(|pi| pi.input).collect())
    }

    /// Returns the number of players added to this session
    #[must_use]
    pub fn num_players(&self) -> usize {
        self.player_reg.num_players()
    }

    /// Return the number of spectators currently registered
    #[must_use]
    pub fn num_spectators(&self) -> usize {
        self.player_reg.num_spectators()
    }

    /// Returns the handles of local players that have been added
    #[must_use]
    pub fn local_player_handles(&self) -> Vec<PlayerHandle> {
        self.player_reg.local_player_handles()
    }

    /// Returns the handles of remote players that have been added
    #[must_use]
    pub fn remote_player_handles(&self) -> Vec<PlayerHandle> {
        self.player_reg.remote_player_handles()
    }

    /// Returns the handles of spectators that have been added
    #[must_use]
    pub fn spectator_handles(&self) -> Vec<PlayerHandle> {
        self.player_reg.spectator_handles()
    }

    /// Returns all handles associated to a certain address
    #[must_use]
    pub fn handles_by_address(&self, addr: T::Address) -> Vec<PlayerHandle> {
        self.player_reg.handles_by_address(addr)
    }

    /// Returns the number of frames this session is estimated to be ahead of other sessions
    #[must_use]
    pub fn frames_ahead(&self) -> i32 {
        self.frames_ahead
    }

    /// Returns the [`DesyncDetection`] mode set for this session at creation time.
    #[must_use]
    pub fn desync_detection(&self) -> DesyncDetection {
        self.desync_detection
    }

    /// Returns a reference to the violation observer, if one was configured.
    ///
    /// This allows checking for violations that occurred during session operations
    /// when using a [`CollectingObserver`] or similar.
    ///
    /// [`CollectingObserver`]: crate::telemetry::CollectingObserver
    #[must_use]
    pub fn violation_observer(&self) -> Option<&Arc<dyn ViolationObserver>> {
        self.violation_observer.as_ref()
    }

    fn disconnect_player_at_frame(&mut self, player_handle: PlayerHandle, last_frame: Frame) {
        // disconnect the remote player
        let Some(player_type) = self.player_reg.handles.get(&player_handle) else {
            report_violation!(
                ViolationSeverity::Warning,
                ViolationKind::InternalError,
                "Invalid player handle {} in disconnect_player_at_frame - ignoring",
                player_handle
            );
            return;
        };

        match player_type {
            PlayerType::Remote(addr) => {
                let Some(endpoint) = self.player_reg.remotes.get_mut(addr) else {
                    report_violation!(
                        ViolationSeverity::Warning,
                        ViolationKind::InternalError,
                        "No endpoint found for remote address in disconnect_player_at_frame - ignoring"
                    );
                    return;
                };

                // mark the affected players as disconnected
                for &handle in endpoint.handles() {
                    self.local_connect_status[handle.as_usize()].disconnected = true;
                }
                endpoint.disconnect();

                if self.sync_layer.current_frame() > last_frame {
                    // remember to adjust simulation to account for the fact that the player disconnected a few frames ago,
                    // resimulating with correct disconnect flags (to account for user having some AI kick in).
                    self.disconnect_frame = last_frame + 1;
                }
            },
            PlayerType::Spectator(addr) => {
                let Some(endpoint) = self.player_reg.spectators.get_mut(addr) else {
                    report_violation!(
                        ViolationSeverity::Warning,
                        ViolationKind::InternalError,
                        "No endpoint found for spectator address in disconnect_player_at_frame - ignoring"
                    );
                    return;
                };
                endpoint.disconnect();
            },
            PlayerType::Local => (),
        }

        // check if all remotes are synchronized now
        self.check_initial_sync();
    }

    /// Change the session state to [`SessionState::Running`] if all UDP endpoints are synchronized.
    fn check_initial_sync(&mut self) {
        // if we are not synchronizing, we don't need to do anything
        if self.state != SessionState::Synchronizing {
            return;
        }

        // if any endpoint is not synchronized, we continue synchronizing
        for endpoint in self.player_reg.remotes.values_mut() {
            if !endpoint.is_synchronized() {
                return;
            }
        }
        for endpoint in self.player_reg.spectators.values_mut() {
            if !endpoint.is_synchronized() {
                return;
            }
        }

        // everyone is synchronized, so we can change state and accept input
        self.state = SessionState::Running;
    }

    /// Roll back to `min_confirmed` frame and resimulate the game with most up-to-date input data.
    ///
    /// # Errors
    /// Returns `FortressError::InvalidFrame` if the frame to load is invalid.
    fn adjust_gamestate(
        &mut self,
        first_incorrect: Frame,
        min_confirmed: Frame,
        requests: &mut Vec<FortressRequest<T>>,
    ) -> Result<(), FortressError> {
        let current_frame = self.sync_layer.current_frame();
        // determine the frame to load
        let frame_to_load = if self.save_mode == SaveMode::Sparse {
            // if sparse saving is turned on, we will rollback to the last saved state
            self.sync_layer.last_saved_frame()
        } else {
            // otherwise, we will rollback to first_incorrect
            first_incorrect
        };

        // we should always load a frame that is before or exactly the first incorrect frame
        if frame_to_load > first_incorrect {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::FrameSync,
                "frame_to_load {} > first_incorrect {} - this indicates a bug",
                frame_to_load,
                first_incorrect
            );
        }

        // If frame_to_load >= current_frame, there's nothing to roll back to.
        // This can happen when a misprediction is detected at the current frame
        // (e.g., at frame 0 when we haven't advanced yet). In this case, we just
        // need to reset predictions - the next frame advance will use the correct inputs.
        if frame_to_load >= current_frame {
            debug!(
                "Skipping rollback: frame_to_load {} >= current_frame {} - resetting predictions only",
                frame_to_load, current_frame
            );
            self.sync_layer.reset_prediction();
            return Ok(());
        }

        let count = current_frame - frame_to_load;

        // request to load that frame
        debug!(
            "Pushing request to load frame {} (current frame {})",
            frame_to_load, current_frame
        );
        requests.push(self.sync_layer.load_frame(frame_to_load)?);

        // we are now at the desired frame
        let actual_frame = self.sync_layer.current_frame();
        if actual_frame != frame_to_load {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::FrameSync,
                "current frame mismatch after load: expected={}, actual={}",
                frame_to_load,
                actual_frame
            );
        }
        self.sync_layer.reset_prediction();

        // step forward to the previous current state, but with updated inputs
        for i in 0..count {
            let inputs = match self
                .sync_layer
                .synchronized_inputs(&self.local_connect_status)
            {
                Some(inputs) => inputs,
                None => {
                    report_violation!(
                        ViolationSeverity::Critical,
                        ViolationKind::InternalError,
                        "Failed to get synchronized inputs during resimulation at frame {}",
                        self.sync_layer.current_frame()
                    );
                    return Err(FortressError::InternalError {
                        context: "Failed to get synchronized inputs during resimulation".to_owned(),
                    });
                },
            };

            // decide whether to request a state save
            if self.save_mode == SaveMode::Sparse {
                // with sparse saving, we only save exactly the min_confirmed frame
                if self.sync_layer.current_frame() == min_confirmed {
                    requests.push(self.sync_layer.save_current_state());
                }
            } else {
                // without sparse saving, we save every state except the very first (just loaded that))
                if i > 0 {
                    requests.push(self.sync_layer.save_current_state());
                }
            }

            // advance the frame
            self.sync_layer.advance_frame();
            requests.push(FortressRequest::AdvanceFrame { inputs });
        }
        // after all this, we should have arrived at the same frame where we started
        let final_frame = self.sync_layer.current_frame();
        if final_frame != current_frame {
            report_violation!(
                ViolationSeverity::Error,
                ViolationKind::FrameSync,
                "current frame mismatch after resimulation: expected={}, actual={}",
                current_frame,
                final_frame
            );
        }
        Ok(())
    }

    /// For each spectator, send all confirmed input up until the minimum confirmed frame.
    fn send_confirmed_inputs_to_spectators(
        &mut self,
        confirmed_frame: Frame,
    ) -> Result<(), FortressError> {
        if self.num_spectators() == 0 {
            return Ok(());
        }

        while self.next_spectator_frame <= confirmed_frame {
            let mut inputs = self
                .sync_layer
                .confirmed_inputs(self.next_spectator_frame, &self.local_connect_status)?;

            // Validate input count matches num_players - this should always hold due to construction
            // but we recover gracefully rather than panic if somehow violated
            if inputs.len() != self.num_players {
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::InternalError,
                    "confirmed_inputs returned {} inputs but expected {} - skipping spectator send for frame {}",
                    inputs.len(),
                    self.num_players,
                    self.next_spectator_frame
                );
                self.next_spectator_frame += 1;
                continue;
            }

            let mut input_map = BTreeMap::new();
            for (handle, input) in inputs.iter_mut().enumerate() {
                // Validate frame consistency - should be NULL or match expected frame
                if input.frame != Frame::NULL && input.frame != self.next_spectator_frame {
                    report_violation!(
                        ViolationSeverity::Warning,
                        ViolationKind::FrameSync,
                        "Input frame {} doesn't match expected spectator frame {} for handle {}",
                        input.frame,
                        self.next_spectator_frame,
                        handle
                    );
                }
                input_map.insert(PlayerHandle::new(handle), *input);
            }

            // send it to all spectators
            for endpoint in self.player_reg.spectators.values_mut() {
                if endpoint.is_running() {
                    endpoint.send_input(&input_map, &self.local_connect_status);
                }
            }

            // onto the next frame
            self.next_spectator_frame += 1;
        }

        Ok(())
    }

    /// Check if players are registered as disconnected for earlier frames on other remote players in comparison to our local assumption.
    /// Disconnect players that are disconnected for other players and update the frame they disconnected
    fn update_player_disconnects(&mut self) {
        for handle_idx in 0..self.num_players {
            let handle = PlayerHandle::new(handle_idx);
            let mut queue_connected = true;
            let mut queue_min_confirmed = Frame::new(i32::MAX);

            // check all player connection status for every remote player
            for endpoint in self.player_reg.remotes.values() {
                if !endpoint.is_running() {
                    continue;
                }
                let con_status = endpoint.peer_connect_status(handle);
                let connected = !con_status.disconnected;
                let min_confirmed = con_status.last_frame;

                queue_connected = queue_connected && connected;
                queue_min_confirmed = std::cmp::min(queue_min_confirmed, min_confirmed);
            }

            // check our local info for that player
            let local_connected = !self.local_connect_status[handle_idx].disconnected;
            let local_min_confirmed = self.local_connect_status[handle_idx].last_frame;

            if local_connected {
                queue_min_confirmed = std::cmp::min(queue_min_confirmed, local_min_confirmed);
            }

            if !queue_connected {
                // check to see if the remote disconnect is further back than we have disconnected that player.
                // If so, we need to re-adjust. This can happen when we e.g. detect our own disconnect at frame n
                // and later receive a disconnect notification for frame n-1.
                if local_connected || local_min_confirmed > queue_min_confirmed {
                    self.disconnect_player_at_frame(handle, queue_min_confirmed);
                }
            }
        }
    }

    /// Gather average frame advantage from each remote player endpoint and return the maximum.
    fn max_frame_advantage(&self) -> i32 {
        let mut interval = i32::MIN;
        for endpoint in self.player_reg.remotes.values() {
            for &handle in endpoint.handles() {
                if !self.local_connect_status[handle.as_usize()].disconnected {
                    interval = std::cmp::max(interval, endpoint.average_frame_advantage());
                }
            }
        }

        // if no remote player is connected
        if interval == i32::MIN {
            interval = 0;
        }

        interval
    }

    fn check_wait_recommendation(&mut self) {
        self.frames_ahead = self.max_frame_advantage();
        if self.sync_layer.current_frame() > self.next_recommended_sleep
            && self.frames_ahead >= MIN_RECOMMENDATION as i32
        {
            self.next_recommended_sleep = self.sync_layer.current_frame() + RECOMMENDATION_INTERVAL;
            // frames_ahead is guaranteed to be >= MIN_RECOMMENDATION (positive), so try_into should succeed.
            // Using unwrap_or(0) as defense-in-depth; 0 effectively skips the recommendation.
            let skip_frames = self.frames_ahead.try_into().unwrap_or(0);
            self.event_queue
                .push_back(FortressEvent::WaitRecommendation { skip_frames });
        }
    }

    fn check_last_saved_state(
        &mut self,
        last_saved: Frame,
        confirmed_frame: Frame,
        requests: &mut Vec<FortressRequest<T>>,
    ) -> Result<(), FortressError> {
        // in sparse saving mode, we need to make sure not to lose the last saved frame
        if self.sync_layer.current_frame() - last_saved >= self.max_prediction as i32 {
            // check if the current frame is confirmed, otherwise we need to roll back
            if confirmed_frame >= self.sync_layer.current_frame() {
                // the current frame is confirmed, save it
                requests.push(self.sync_layer.save_current_state());
            } else {
                // roll back to the last saved state, resimulate and save on the way
                self.adjust_gamestate(last_saved, confirmed_frame, requests)?;
            }

            // after all this, we should have saved the confirmed state
            let expected_saved_frame =
                std::cmp::min(confirmed_frame, self.sync_layer.current_frame());
            if confirmed_frame != Frame::NULL
                && self.sync_layer.last_saved_frame() != expected_saved_frame
            {
                report_violation!(
                    ViolationSeverity::Error,
                    ViolationKind::StateManagement,
                    "last_saved_frame mismatch after check_last_saved_state: expected={}, actual={}",
                    expected_saved_frame,
                    self.sync_layer.last_saved_frame()
                );
            }
        }
        Ok(())
    }

    /// Handle events received from the UDP endpoints. Most events are being forwarded to the user for notification, but some require action.
    fn handle_event(
        &mut self,
        event: Event<T>,
        player_handles: Vec<PlayerHandle>,
        addr: T::Address,
    ) {
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
            // check if all remotes are synced, then forward to user
            Event::Synchronized => {
                self.check_initial_sync();
                self.event_queue
                    .push_back(FortressEvent::Synchronized { addr });
            },
            // disconnect the player, then forward to user
            Event::Disconnected => {
                for handle in player_handles {
                    let last_frame = if handle.is_valid_player_for(self.num_players) {
                        self.local_connect_status[handle.as_usize()].last_frame
                    } else {
                        Frame::NULL // spectator
                    };

                    self.disconnect_player_at_frame(handle, last_frame);
                }

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
                // input only comes from remote players, not spectators
                if !player.is_valid_player_for(self.num_players) {
                    report_violation!(
                        ViolationSeverity::Error,
                        ViolationKind::NetworkProtocol,
                        "Received input from invalid player handle {} (max={})",
                        player,
                        self.num_players - 1
                    );
                    return;
                }
                if !self.local_connect_status[player.as_usize()].disconnected {
                    // check if the input comes in the correct sequence
                    let current_remote_frame =
                        self.local_connect_status[player.as_usize()].last_frame;
                    if current_remote_frame != Frame::NULL
                        && current_remote_frame + 1 != input.frame
                    {
                        report_violation!(
                            ViolationSeverity::Error,
                            ViolationKind::NetworkProtocol,
                            "Input sequence violation: expected frame {}, got {}",
                            current_remote_frame + 1,
                            input.frame
                        );
                        return;
                    }
                    // update our info
                    self.local_connect_status[player.as_usize()].last_frame = input.frame;
                    // add the remote input
                    self.sync_layer.add_remote_input(player, input);
                }
            },
        }

        // check event queue size and discard oldest events if too big
        while self.event_queue.len() > MAX_EVENT_QUEUE_SIZE {
            self.event_queue.pop_front();
        }
    }

    fn compare_local_checksums_against_peers(&mut self) {
        match self.desync_detection {
            DesyncDetection::On { .. } => {
                for remote in self.player_reg.remotes.values_mut() {
                    let mut checked_frames = Vec::new();

                    for (&remote_frame, &remote_checksum) in &remote.pending_checksums {
                        if remote_frame >= self.sync_layer.last_confirmed_frame() {
                            // we're still waiting for inputs for this frame
                            continue;
                        }
                        if let Some(&local_checksum) =
                            self.local_checksum_history.get(&remote_frame)
                        {
                            if local_checksum != remote_checksum {
                                self.event_queue.push_back(FortressEvent::DesyncDetected {
                                    frame: remote_frame,
                                    local_checksum,
                                    remote_checksum,
                                    addr: remote.peer_addr(),
                                });
                            }
                            checked_frames.push(remote_frame);
                        }
                    }

                    for frame in checked_frames {
                        remote.pending_checksums.remove_entry(&frame);
                    }
                }
            },
            DesyncDetection::Off => (),
        }
    }

    fn check_checksum_send_interval(&mut self) {
        match self.desync_detection {
            DesyncDetection::On { interval } => {
                let frame_to_send = if self.last_sent_checksum_frame.is_null() {
                    Frame::new(interval as i32)
                } else {
                    self.last_sent_checksum_frame + interval as i32
                };

                if frame_to_send <= self.sync_layer.last_confirmed_frame()
                    && frame_to_send <= self.sync_layer.last_saved_frame()
                {
                    let Some(cell) = self.sync_layer.saved_state_by_frame(frame_to_send) else {
                        // This shouldn't happen if frame is within confirmed and saved range
                        report_violation!(
                            ViolationSeverity::Warning,
                            ViolationKind::StateManagement,
                            "Cell not found for frame {} in check_checksum_send_interval (confirmed={}, saved={}) - skipping checksum",
                            frame_to_send,
                            self.sync_layer.last_confirmed_frame(),
                            self.sync_layer.last_saved_frame()
                        );
                        return;
                    };

                    if let Some(checksum) = cell.checksum() {
                        for remote in self.player_reg.remotes.values_mut() {
                            remote.send_checksum_report(frame_to_send, checksum);
                        }
                        self.last_sent_checksum_frame = frame_to_send;
                        // collect locally for later comparison
                        self.local_checksum_history.insert(frame_to_send, checksum);
                    }

                    let max_history = self.protocol_config.max_checksum_history;
                    if self.local_checksum_history.len() > max_history {
                        let oldest_frame_to_keep =
                            frame_to_send - (max_history as i32 - 1) * interval as i32;
                        self.local_checksum_history
                            .retain(|&frame, _| frame >= oldest_frame_to_keep);
                    }
                }
            },
            DesyncDetection::Off => (),
        }
    }
}
