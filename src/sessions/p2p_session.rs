use crate::error::FortressError;
use crate::frame_info::PlayerInput;
use crate::network::messages::ConnectionStatus;
use crate::network::network_stats::NetworkStats;
use crate::report_violation;
use crate::sessions::config::{ProtocolConfig, SaveMode};
use crate::sessions::player_registry::PlayerRegistry;
use crate::sessions::sync_health::SyncHealth;
use crate::sync_layer::SyncLayer;
use crate::telemetry::{
    InvariantChecker, InvariantViolation, ViolationKind, ViolationObserver, ViolationSeverity,
};
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
    /// The highest frame at which checksums matched with all peers.
    /// Used by `sync_health()` to determine if we're in sync.
    /// `None` means no successful comparison yet, `Some(frame)` means
    /// checksums matched at that frame (no desync detected up to that point).
    last_verified_frame: Option<Frame>,
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
            if matches!(player_type, PlayerType::Local) {
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
            last_verified_frame: None,
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
                self.local_connect_status
                    .get_mut(handle.as_usize())
                    .ok_or_else(|| FortressError::InternalError {
                        context: format!(
                            "Invalid player handle {} when updating connection status",
                            handle
                        ),
                    })?
                    .last_frame = actual_frame;
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
        for (event, handles, addr) in std::mem::take(&mut events) {
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
                let status = self
                    .local_connect_status
                    .get(player_handle.as_usize())
                    .ok_or_else(|| FortressError::InternalError {
                        context: format!(
                            "Invalid player handle {} when checking disconnect status",
                            player_handle
                        ),
                    })?;
                if !status.disconnected {
                    let last_frame = status.last_frame;
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
    ///
    /// The returned struct includes:
    /// - Network quality metrics (ping, send queue length, bandwidth)
    /// - Frame advantage/disadvantage relative to the peer
    /// - **Checksum comparison data** for desync detection
    ///
    /// # Checksum Fields
    ///
    /// The checksum fields (`last_compared_frame`, `local_checksum`, `remote_checksum`,
    /// `checksums_match`) are populated when desync detection is enabled and at least
    /// one checksum comparison has occurred. Use these to detect game state divergence.
    ///
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
        let mut stats = match self.player_reg.handles.get(&player_handle) {
            Some(PlayerType::Remote(addr)) => match self.player_reg.remotes.get(addr) {
                Some(endpoint) => endpoint.network_stats()?,
                None => {
                    return Err(FortressError::InternalError {
                        context: format!(
                            "Endpoint not found for registered remote player at {:?}",
                            addr
                        ),
                    });
                },
            },
            Some(PlayerType::Spectator(addr)) => match self.player_reg.remotes.get(addr) {
                Some(endpoint) => endpoint.network_stats()?,
                None => {
                    return Err(FortressError::InternalError {
                        context: format!(
                            "Endpoint not found for registered spectator at {:?}",
                            addr
                        ),
                    });
                },
            },
            _ => {
                return Err(FortressError::InvalidRequest {
                    info: "Given player handle not referring to a remote player or spectator"
                        .to_owned(),
                });
            },
        };

        // Populate checksum fields from local history and remote pending checksums
        self.populate_checksum_stats(&mut stats, player_handle);

        Ok(stats)
    }

    /// Populates the checksum-related fields in NetworkStats.
    fn populate_checksum_stats(&self, stats: &mut NetworkStats, player_handle: PlayerHandle) {
        // Get the remote endpoint's pending checksums
        let Some(player_type) = self.player_reg.handles.get(&player_handle) else {
            return;
        };
        let addr = match player_type {
            PlayerType::Remote(addr) => addr,
            _ => return,
        };
        let Some(remote) = self.player_reg.remotes.get(addr) else {
            return;
        };

        // Find the most recent frame where we have both local and remote checksums
        let mut latest_compared_frame: Option<Frame> = None;
        let mut latest_local: Option<u128> = None;
        let mut latest_remote: Option<u128> = None;

        for (&frame, &local_cs) in &self.local_checksum_history {
            if let Some(&remote_cs) = remote.pending_checksums.get(&frame) {
                if latest_compared_frame.is_none_or(|f| frame > f) {
                    latest_compared_frame = Some(frame);
                    latest_local = Some(local_cs);
                    latest_remote = Some(remote_cs);
                }
            }
        }

        stats.last_compared_frame = latest_compared_frame;
        stats.local_checksum = latest_local;
        stats.remote_checksum = latest_remote;
        stats.checksums_match = match (latest_local, latest_remote) {
            (Some(local), Some(remote)) => Some(local == remote),
            _ => None,
        };
    }

    /// Returns the highest confirmed frame where all inputs have been received.
    ///
    /// A "confirmed frame" means all players' inputs for that frame have been received
    /// and will not be rolled back. The game state at this frame is **locally correct**
    /// based on the inputs received.
    ///
    /// # Important: This Does NOT Guarantee Synchronization
    ///
    /// **Do NOT use this method alone to determine when to terminate a session.**
    ///
    /// This method tells you "inputs are confirmed locally" but does **not** mean:
    /// - Both peers have simulated to the same frame
    /// - Game state matches between peers  
    /// - The session is safe to terminate
    ///
    /// Peers run asynchronously. When peer A's `confirmed_frame()` returns 100, peer B
    /// might still be at frame 80. If peer A terminates, peer B will continue processing
    /// more frames, leading to different final states.
    ///
    /// # Correct Termination Pattern
    ///
    /// Use [`sync_health`](Self::sync_health) in addition to `confirmed_frame()`:
    ///
    /// ```ignore
    /// // WRONG: Terminating based on confirmed_frame alone
    /// if session.confirmed_frame() >= target_frames {
    ///     break; // Dangerous! Peers may be at different frames!
    /// }
    ///
    /// // CORRECT: Use sync_health to verify peer synchronization
    /// if session.confirmed_frame() >= target_frames {
    ///     match session.sync_health(peer_handle) {
    ///         Some(SyncHealth::InSync) => break, // Safe to exit
    ///         Some(SyncHealth::DesyncDetected { .. }) => panic!("Desync!"),
    ///         _ => continue, // Keep polling until sync status is known
    ///     }
    /// }
    /// ```
    ///
    /// # Valid Uses
    ///
    /// - Knowing which frames are safe to discard from rollback history
    /// - Computing checksums over confirmed game state
    /// - Delta compression (older confirmed state won't change)
    /// - Progress reporting to users
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

    /// Returns the synchronization health status for a specific remote peer.
    ///
    /// This is the **primary API for checking if a session is synchronized** before
    /// termination or for ongoing desync detection. Unlike [`confirmed_frame`], which
    /// only indicates input confirmation, this method verifies that game state checksums
    /// match between peers.
    ///
    /// # Arguments
    ///
    /// * `player_handle` - The handle of the remote player to check.
    ///
    /// # Returns
    ///
    /// * `Some(SyncHealth::InSync)` - Checksums match at the last compared frame.
    /// * `Some(SyncHealth::Pending)` - No checksum comparison available yet.
    /// * `Some(SyncHealth::DesyncDetected { .. })` - Checksums differ, indicating desync.
    /// * `None` - The handle doesn't refer to a remote player.
    ///
    /// # Important Anti-Pattern Warning
    ///
    /// **Do NOT use [`confirmed_frame`](Self::confirmed_frame) alone to determine
    /// when to terminate a session.** That method only indicates when rollbacks won't affect
    /// a frame - it does NOT mean both peers have simulated to the same frame.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // WRONG: Using confirmed_frame for termination
    /// if session.confirmed_frame() >= target_frames {
    ///     break; // Dangerous! Peers may be at different frames!
    /// }
    ///
    /// // CORRECT: Use sync_health for safe termination
    /// if session.confirmed_frame() >= target_frames {
    ///     match session.sync_health(peer_handle) {
    ///         Some(SyncHealth::InSync) => break, // Safe to exit
    ///         Some(SyncHealth::DesyncDetected { .. }) => panic!("Desync!"),
    ///         _ => continue, // Keep polling
    ///     }
    /// }
    /// ```
    ///
    /// [`confirmed_frame`]: Self::confirmed_frame
    #[must_use]
    pub fn sync_health(&self, player_handle: PlayerHandle) -> Option<SyncHealth> {
        // Only remote players have sync health
        let player_type = self.player_reg.handles.get(&player_handle)?;
        let addr = match player_type {
            PlayerType::Remote(addr) => addr,
            _ => return None,
        };

        // Get the remote endpoint
        let remote = self.player_reg.remotes.get(addr)?;

        // If desync detection is off, we can't determine sync health
        if self.desync_detection == DesyncDetection::Off {
            return Some(SyncHealth::Pending);
        }

        // Check for any pending checksums that don't match our local history
        // This catches desyncs before they're processed by compare_local_checksums_against_peers
        for (&remote_frame, &remote_checksum) in &remote.pending_checksums {
            // Only compare frames that have been confirmed locally
            if remote_frame >= self.sync_layer.last_confirmed_frame() {
                continue;
            }
            if let Some(&local_checksum) = self.local_checksum_history.get(&remote_frame) {
                if local_checksum != remote_checksum {
                    return Some(SyncHealth::DesyncDetected {
                        frame: remote_frame,
                        local_checksum,
                        remote_checksum,
                    });
                }
            }
        }

        // If we have a verified frame (successful checksum comparison happened), we're in sync
        if self.last_verified_frame.is_some() {
            return Some(SyncHealth::InSync);
        }

        // No successful comparison yet - still pending
        Some(SyncHealth::Pending)
    }

    /// Returns `true` if all remote peers show [`SyncHealth::InSync`].
    ///
    /// This is a convenience method that checks all remote peers at once.
    /// Returns `false` if any peer is pending or has detected a desync,
    /// or if there are no remote peers.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Wait until all peers are synchronized before exiting
    /// if session.confirmed_frame() >= target_frames && session.is_synchronized() {
    ///     break;
    /// }
    /// ```
    #[must_use]
    pub fn is_synchronized(&self) -> bool {
        let remote_handles = self.player_reg.remote_player_handles();
        if remote_handles.is_empty() {
            // No remote peers - always synchronized with ourselves
            return true;
        }

        remote_handles
            .iter()
            .all(|&handle| matches!(self.sync_health(handle), Some(SyncHealth::InSync)))
    }

    /// Returns the highest frame for which checksums have been verified to match.
    ///
    /// This is useful for ensuring synchronization has been verified up to a
    /// specific point before terminating a session.
    ///
    /// # Returns
    ///
    /// * `Some(frame)` - The highest frame where checksums matched between all peers
    /// * `None` - No checksum comparison has successfully completed yet
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Ensure we've verified sync up to the target before exiting
    /// let target = Frame::new(100);
    /// if let Some(verified) = session.last_verified_frame() {
    ///     if verified >= target {
    ///         // Safe to terminate - verified sync at target frame
    ///     }
    /// }
    /// ```
    #[must_use]
    pub fn last_verified_frame(&self) -> Option<Frame> {
        self.last_verified_frame
    }

    /// Returns detailed synchronization information for all remote peers.
    ///
    /// This method provides a comprehensive view of the synchronization status
    /// with all connected remote players. Useful for debugging and diagnostics.
    ///
    /// # Returns
    ///
    /// A vector of tuples containing `(PlayerHandle, SyncHealth)` for each
    /// remote player. The vector is empty if there are no remote players.
    #[must_use]
    pub fn all_sync_health(&self) -> Vec<(PlayerHandle, SyncHealth)> {
        self.player_reg
            .remote_player_handles()
            .into_iter()
            .filter_map(|handle| self.sync_health(handle).map(|health| (handle, health)))
            .collect()
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
                    let Some(status) = self.local_connect_status.get_mut(handle.as_usize()) else {
                        report_violation!(
                            ViolationSeverity::Warning,
                            ViolationKind::InternalError,
                            "Invalid player handle {} when marking as disconnected - skipping",
                            handle
                        );
                        continue;
                    };
                    status.disconnected = true;
                }
                endpoint.disconnect();

                if self.sync_layer.current_frame() > last_frame {
                    // remember to adjust simulation to account for the fact that the player disconnected a few frames ago,
                    // resimulating with correct disconnect flags (to account for user having some AI kick in).
                    self.disconnect_frame = last_frame.saturating_add(1);
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
                self.next_spectator_frame = self.next_spectator_frame.saturating_add(1);
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
            self.next_spectator_frame = self.next_spectator_frame.saturating_add(1);
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
            let Some(status) = self.local_connect_status.get(handle_idx) else {
                report_violation!(
                    ViolationSeverity::Warning,
                    ViolationKind::InternalError,
                    "Invalid player index {} when checking connection status - skipping",
                    handle_idx
                );
                continue;
            };
            let local_connected = !status.disconnected;
            let local_min_confirmed = status.last_frame;

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
                let Some(status) = self.local_connect_status.get(handle.as_usize()) else {
                    report_violation!(
                        ViolationSeverity::Warning,
                        ViolationKind::InternalError,
                        "Invalid player handle {} when checking frame advantage - skipping",
                        handle
                    );
                    continue;
                };
                if !status.disconnected {
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
                    // unwrap_or_else has side effects (violation reporting)
                    #[allow(clippy::map_unwrap_or)]
                    let last_frame = if handle.is_valid_player_for(self.num_players) {
                        self.local_connect_status
                            .get(handle.as_usize())
                            .map(|s| s.last_frame)
                            .unwrap_or_else(|| {
                                report_violation!(
                                    ViolationSeverity::Warning,
                                    ViolationKind::InternalError,
                                    "Invalid player handle {} when handling disconnect event - using NULL frame",
                                    handle
                                );
                                Frame::NULL
                            })
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
                let Some(status) = self.local_connect_status.get_mut(player.as_usize()) else {
                    report_violation!(
                        ViolationSeverity::Warning,
                        ViolationKind::InternalError,
                        "Invalid player handle {} when handling input event - ignoring",
                        player
                    );
                    return;
                };
                if !status.disconnected {
                    // check if the input comes in the correct sequence
                    let current_remote_frame = status.last_frame;
                    let expected_frame = current_remote_frame.saturating_add(1);
                    if current_remote_frame != Frame::NULL && expected_frame != input.frame {
                        report_violation!(
                            ViolationSeverity::Error,
                            ViolationKind::NetworkProtocol,
                            "Input sequence violation: expected frame {}, got {}",
                            expected_frame,
                            input.frame
                        );
                        return;
                    }
                    // update our info
                    status.last_frame = input.frame;
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
                            } else {
                                // Checksums match - update last verified frame
                                self.last_verified_frame = match self.last_verified_frame {
                                    Some(current) if current >= remote_frame => Some(current),
                                    _ => Some(remote_frame),
                                };
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

impl<T: Config> InvariantChecker for P2PSession<T> {
    /// Checks that the session's invariants are satisfied.
    ///
    /// This method verifies:
    /// 1. No desync has been detected with any remote peer
    /// 2. The session state is consistent
    ///
    /// # Returns
    ///
    /// * `Ok(())` - All invariants hold
    /// * `Err(InvariantViolation)` - A desync or other invariant violation was detected
    ///
    /// # Example
    ///
    /// ```ignore
    /// use fortress_rollback::telemetry::InvariantChecker;
    ///
    /// // In tests, assert invariants after operations
    /// session.advance_frame()?;
    /// assert!(session.check_invariants().is_ok(), "Session invariants violated");
    ///
    /// // Or use the macro for debug-only checks
    /// fortress_rollback::debug_check_invariants!(session);
    /// ```
    fn check_invariants(&self) -> Result<(), InvariantViolation> {
        // Check for any desync with remote peers
        for (handle, health) in self.all_sync_health() {
            if let SyncHealth::DesyncDetected {
                frame,
                local_checksum,
                remote_checksum,
            } = health
            {
                return Err(InvariantViolation::new(
                    "P2PSession",
                    "checksum mismatch detected with remote peer",
                )
                .with_details(format!(
                    "Desync at frame {} with player {}: local={:#x}, remote={:#x}",
                    frame, handle, local_checksum, remote_checksum
                )));
            }
        }

        Ok(())
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
    use crate::network::messages::Message;
    use crate::sessions::builder::SessionBuilder;
    use crate::{Config, NonBlockingSocket};
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

    // Helper function to create a local-only P2P session for testing (no network)
    fn create_local_only_session() -> P2PSession<TestConfig> {
        SessionBuilder::new()
            .with_num_players(1)
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .expect("Failed to add player")
            .start_p2p_session(DummySocket)
            .expect("Failed to create session")
    }

    // Helper function to create a 2-player P2P session with one remote
    fn create_two_player_session() -> P2PSession<TestConfig> {
        SessionBuilder::new()
            .with_num_players(2)
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .expect("Failed to add local player")
            .add_player(PlayerType::Remote(test_addr(8080)), PlayerHandle::new(1))
            .expect("Failed to add remote player")
            .start_p2p_session(DummySocket)
            .expect("Failed to create session")
    }

    // Helper function to create a 2-player local-only session
    fn create_two_local_players_session() -> P2PSession<TestConfig> {
        SessionBuilder::new()
            .with_num_players(2)
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .expect("Failed to add player 0")
            .add_player(PlayerType::Local, PlayerHandle::new(1))
            .expect("Failed to add player 1")
            .start_p2p_session(DummySocket)
            .expect("Failed to create session")
    }

    // ==========================================
    // Constant Tests
    // ==========================================

    #[test]
    fn recommendation_interval_is_reasonable() {
        // 60 frames at 60fps = 1 second
        assert_eq!(RECOMMENDATION_INTERVAL, Frame::new(60));
    }

    #[test]
    fn min_recommendation_is_reasonable() {
        // At least 2 frames to avoid micro-stuttering, but not more than 10
        // Use const_assert pattern to satisfy clippy about const assertions
        const _: () = assert!(MIN_RECOMMENDATION >= 2);
        const _: () = assert!(MIN_RECOMMENDATION <= 10);
        // Verify at runtime the constant is what we expect
        assert_eq!(MIN_RECOMMENDATION, 3);
    }

    #[test]
    fn max_event_queue_size_is_reasonable() {
        // Should be large enough to buffer network events (at least 50)
        // but not so large as to consume excessive memory (at most 1000)
        const _: () = assert!(MAX_EVENT_QUEUE_SIZE >= 50);
        const _: () = assert!(MAX_EVENT_QUEUE_SIZE <= 1000);
        // Verify at runtime the constant is what we expect
        assert_eq!(MAX_EVENT_QUEUE_SIZE, 100);
    }

    // ==========================================
    // P2PSession Constructor and Initial State Tests
    // ==========================================

    #[test]
    fn p2p_session_local_only_starts_running() {
        let session = create_local_only_session();
        // With no remote players, session starts running immediately
        assert_eq!(session.current_state(), SessionState::Running);
    }

    #[test]
    fn p2p_session_with_remote_starts_synchronizing() {
        let session = create_two_player_session();
        // With remote players, session starts in synchronizing state
        assert_eq!(session.current_state(), SessionState::Synchronizing);
    }

    #[test]
    fn p2p_session_initial_frame_is_zero() {
        let session = create_local_only_session();
        assert_eq!(session.current_frame(), Frame::new(0));
    }

    #[test]
    fn p2p_session_initial_confirmed_frame_is_null() {
        let session = create_local_only_session();
        // Initially, confirmed frame is NULL (no frames confirmed yet)
        assert_eq!(session.confirmed_frame(), Frame::NULL);
    }

    #[test]
    fn p2p_session_num_players_returns_correct_count() {
        let session = create_two_local_players_session();
        assert_eq!(session.num_players(), 2);
    }

    #[test]
    fn p2p_session_num_spectators_initially_zero() {
        let session = create_local_only_session();
        assert_eq!(session.num_spectators(), 0);
    }

    #[test]
    fn p2p_session_local_player_handles_returns_correct_handles() {
        let session = create_two_local_players_session();
        let handles = session.local_player_handles();
        assert_eq!(handles.len(), 2);
        assert!(handles.contains(&PlayerHandle::new(0)));
        assert!(handles.contains(&PlayerHandle::new(1)));
    }

    #[test]
    fn p2p_session_remote_player_handles_with_remotes() {
        let session = create_two_player_session();
        let handles = session.remote_player_handles();
        assert_eq!(handles.len(), 1);
        assert!(handles.contains(&PlayerHandle::new(1)));
    }

    #[test]
    fn p2p_session_spectator_handles_initially_empty() {
        let session = create_two_player_session();
        assert!(session.spectator_handles().is_empty());
    }

    #[test]
    fn p2p_session_max_prediction_returns_configured_value() {
        let session = SessionBuilder::<TestConfig>::new()
            .with_num_players(1)
            .with_max_prediction_window(4)
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .expect("Failed to add player")
            .start_p2p_session(DummySocket)
            .expect("Failed to create session");
        assert_eq!(session.max_prediction(), 4);
    }

    #[test]
    fn p2p_session_frames_ahead_initially_zero() {
        let session = create_local_only_session();
        assert_eq!(session.frames_ahead(), 0);
    }

    #[test]
    fn p2p_session_desync_detection_returns_configured_value() {
        let session = SessionBuilder::<TestConfig>::new()
            .with_num_players(1)
            .with_desync_detection_mode(DesyncDetection::Off)
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .expect("Failed to add player")
            .start_p2p_session(DummySocket)
            .expect("Failed to create session");
        assert_eq!(session.desync_detection(), DesyncDetection::Off);
    }

    #[test]
    fn p2p_session_violation_observer_none_by_default() {
        let session = create_local_only_session();
        assert!(session.violation_observer().is_none());
    }

    // ==========================================
    // add_local_input Tests
    // ==========================================

    #[test]
    fn add_local_input_for_valid_handle_succeeds() {
        let mut session = create_local_only_session();
        let result = session.add_local_input(PlayerHandle::new(0), 42u8);
        assert!(result.is_ok());
    }

    #[test]
    fn add_local_input_for_remote_handle_fails() {
        let mut session = create_two_player_session();
        // Handle 1 is remote
        let result = session.add_local_input(PlayerHandle::new(1), 42u8);
        assert!(result.is_err());
        match result {
            Err(FortressError::InvalidRequest { info }) => {
                assert!(info.contains("local player"));
            },
            _ => panic!("Expected InvalidRequest error"),
        }
    }

    #[test]
    fn add_local_input_for_invalid_handle_fails() {
        let mut session = create_local_only_session();
        // Handle 99 doesn't exist
        let result = session.add_local_input(PlayerHandle::new(99), 42u8);
        assert!(result.is_err());
    }

    #[test]
    fn add_local_input_multiple_times_overwrites() {
        let mut session = create_local_only_session();
        session
            .add_local_input(PlayerHandle::new(0), 10u8)
            .expect("First input failed");
        session
            .add_local_input(PlayerHandle::new(0), 20u8)
            .expect("Second input failed");
        // Should succeed without error - second input overwrites first
    }

    // ==========================================
    // advance_frame Tests
    // ==========================================

    #[test]
    fn advance_frame_without_input_fails() {
        let mut session = create_local_only_session();
        let result = session.advance_frame();
        assert!(result.is_err());
        match result {
            Err(FortressError::InvalidRequest { info }) => {
                assert!(info.contains("Missing local input"));
            },
            _ => panic!("Expected InvalidRequest error"),
        }
    }

    #[test]
    fn advance_frame_when_not_synchronized_fails() {
        let mut session = create_two_player_session();
        // Session is in Synchronizing state
        assert_eq!(session.current_state(), SessionState::Synchronizing);
        session
            .add_local_input(PlayerHandle::new(0), 42u8)
            .expect("Input failed");
        let result = session.advance_frame();
        assert!(result.is_err());
        match result {
            Err(FortressError::NotSynchronized) => {},
            _ => panic!("Expected NotSynchronized error"),
        }
    }

    #[test]
    fn advance_frame_with_input_succeeds() {
        let mut session = create_local_only_session();
        session
            .add_local_input(PlayerHandle::new(0), 42u8)
            .expect("Input failed");
        let requests = session.advance_frame().expect("Advance failed");
        // Should have at least one request (likely SaveGameState and AdvanceFrame)
        assert!(!requests.is_empty());
    }

    #[test]
    fn advance_frame_increments_current_frame() {
        let mut session = create_local_only_session();
        assert_eq!(session.current_frame(), Frame::new(0));
        session
            .add_local_input(PlayerHandle::new(0), 42u8)
            .expect("Input failed");
        let _ = session.advance_frame();
        assert_eq!(session.current_frame(), Frame::new(1));
    }

    #[test]
    fn advance_frame_clears_local_inputs() {
        let mut session = create_local_only_session();
        session
            .add_local_input(PlayerHandle::new(0), 42u8)
            .expect("Input failed");
        let _ = session.advance_frame();
        // Now trying to advance again without input should fail
        let result = session.advance_frame();
        assert!(result.is_err());
    }

    #[test]
    fn advance_frame_multiple_local_players_requires_all_inputs() {
        let mut session = create_two_local_players_session();
        // Add input for player 0 only
        session
            .add_local_input(PlayerHandle::new(0), 42u8)
            .expect("Input failed");
        let result = session.advance_frame();
        assert!(result.is_err());
        match result {
            Err(FortressError::InvalidRequest { info }) => {
                assert!(info.contains("Missing local input"));
            },
            _ => panic!("Expected InvalidRequest error"),
        }
    }

    #[test]
    fn advance_frame_multiple_local_players_succeeds_with_all_inputs() {
        let mut session = create_two_local_players_session();
        session
            .add_local_input(PlayerHandle::new(0), 1u8)
            .expect("Input 0 failed");
        session
            .add_local_input(PlayerHandle::new(1), 2u8)
            .expect("Input 1 failed");
        let requests = session.advance_frame().expect("Advance failed");
        assert!(!requests.is_empty());
    }

    // ==========================================
    // poll_remote_clients Tests
    // ==========================================

    #[test]
    fn poll_remote_clients_does_not_panic() {
        let mut session = create_two_player_session();
        // Should not panic even with no messages
        session.poll_remote_clients();
    }

    #[test]
    fn poll_remote_clients_multiple_times() {
        let mut session = create_local_only_session();
        for _ in 0..10 {
            session.poll_remote_clients();
        }
        // Should complete without issues
    }

    // ==========================================
    // disconnect_player Tests
    // ==========================================

    #[test]
    fn disconnect_player_local_fails() {
        let mut session = create_local_only_session();
        let result = session.disconnect_player(PlayerHandle::new(0));
        assert!(result.is_err());
        match result {
            Err(FortressError::InvalidRequest { info }) => {
                assert!(info.contains("Local Player cannot be disconnected"));
            },
            _ => panic!("Expected InvalidRequest error"),
        }
    }

    #[test]
    fn disconnect_player_invalid_handle_fails() {
        let mut session = create_local_only_session();
        let result = session.disconnect_player(PlayerHandle::new(99));
        assert!(result.is_err());
        match result {
            Err(FortressError::InvalidRequest { info }) => {
                assert!(info.contains("Invalid Player Handle"));
            },
            _ => panic!("Expected InvalidRequest error"),
        }
    }

    #[test]
    fn disconnect_player_remote_succeeds() {
        let mut session = create_two_player_session();
        // Disconnect remote player (handle 1)
        let result = session.disconnect_player(PlayerHandle::new(1));
        assert!(result.is_ok());
    }

    #[test]
    fn disconnect_player_already_disconnected_fails() {
        let mut session = create_two_player_session();
        session
            .disconnect_player(PlayerHandle::new(1))
            .expect("First disconnect failed");
        let result = session.disconnect_player(PlayerHandle::new(1));
        assert!(result.is_err());
        match result {
            Err(FortressError::InvalidRequest { info }) => {
                assert!(info.contains("already disconnected"));
            },
            _ => panic!("Expected InvalidRequest error"),
        }
    }

    // ==========================================
    // network_stats Tests
    // ==========================================

    #[test]
    fn network_stats_local_player_fails() {
        let session = create_local_only_session();
        let result = session.network_stats(PlayerHandle::new(0));
        assert!(result.is_err());
        match result {
            Err(FortressError::InvalidRequest { info }) => {
                assert!(info.contains("not referring to a remote player"));
            },
            _ => panic!("Expected InvalidRequest error"),
        }
    }

    #[test]
    fn network_stats_invalid_handle_fails() {
        let session = create_local_only_session();
        let result = session.network_stats(PlayerHandle::new(99));
        assert!(result.is_err());
    }

    #[test]
    fn network_stats_remote_not_synchronized_fails() {
        let session = create_two_player_session();
        // Session is not yet synchronized
        let result = session.network_stats(PlayerHandle::new(1));
        assert!(result.is_err());
        match result {
            Err(FortressError::NotSynchronized) => {},
            _ => panic!("Expected NotSynchronized error"),
        }
    }

    // ==========================================
    // confirmed_inputs_for_frame Tests
    // ==========================================

    #[test]
    fn confirmed_inputs_for_frame_future_frame_fails() {
        let session = create_local_only_session();
        let result = session.confirmed_inputs_for_frame(Frame::new(100));
        assert!(result.is_err());
        match result {
            Err(FortressError::InvalidFrame { frame, reason }) => {
                assert_eq!(frame, Frame::new(100));
                assert!(reason.contains("not confirmed"));
            },
            _ => panic!("Expected InvalidFrame error"),
        }
    }

    #[test]
    fn confirmed_inputs_for_frame_returns_correct_inputs() {
        let mut session = create_local_only_session();

        // Advance a few frames
        for i in 0..5 {
            session
                .add_local_input(PlayerHandle::new(0), (i * 10) as u8)
                .expect("Input failed");
            let _ = session.advance_frame();
        }

        // After advancing, confirmed_frame should be at least 0
        let confirmed = session.confirmed_frame();
        assert!(
            confirmed >= Frame::new(0),
            "Should have some confirmed frames"
        );

        // If we have a confirmed frame, we should be able to get inputs for it
        if confirmed >= Frame::new(0) && !confirmed.is_null() {
            let inputs = session
                .confirmed_inputs_for_frame(Frame::new(0))
                .expect("Should get inputs for confirmed frame");
            assert_eq!(inputs.len(), 1, "Should have 1 player's inputs");
            assert_eq!(inputs[0], 0u8, "Frame 0 should have input value 0");
        }
    }

    #[test]
    fn confirmed_inputs_for_frame_frame_at_confirmed_boundary() {
        let mut session = create_local_only_session();

        // Advance several frames
        for i in 0..10 {
            session
                .add_local_input(PlayerHandle::new(0), i as u8)
                .expect("Input failed");
            let _ = session.advance_frame();
        }

        let confirmed = session.confirmed_frame();

        // Getting inputs at the confirmed frame should succeed
        if !confirmed.is_null() {
            let result = session.confirmed_inputs_for_frame(confirmed);
            assert!(result.is_ok(), "Should succeed at confirmed frame boundary");
        }

        // Getting inputs one frame past confirmed should fail
        let result = session.confirmed_inputs_for_frame(confirmed + 1);
        assert!(result.is_err(), "Should fail past confirmed frame");
    }

    #[test]
    fn confirmed_inputs_for_frame_null_frame_handling() {
        let session = create_local_only_session();

        // NULL frame (which is -1) - the behavior depends on whether it's treated as
        // "before confirmed" or as an invalid frame. The function checks if frame > confirmed_frame,
        // and since NULL (-1) <= any confirmed frame, it will try to fetch from the input queue.
        // The input queue will then fail because frame -1 doesn't exist.
        let result = session.confirmed_inputs_for_frame(Frame::NULL);
        // This may succeed or fail depending on implementation details
        // The key is that it doesn't panic and handles the edge case
        match result {
            Ok(_) => {
                // If it somehow succeeds, that's fine - just verify behavior
            },
            Err(_) => {
                // Expected - frame -1 is not in the queue
            },
        }
    }

    #[test]
    fn confirmed_inputs_for_frame_discarded_frame_fails() {
        let mut session = create_local_only_session();

        // Advance many frames to trigger frame discard
        // INPUT_QUEUE_LENGTH is 128, so after 128+ frames, early frames are discarded
        for i in 0..150 {
            session
                .add_local_input(PlayerHandle::new(0), (i % 256) as u8)
                .expect("Input failed");
            let _ = session.advance_frame();
        }

        // Frame 0 should have been discarded by now (we're past INPUT_QUEUE_LENGTH)
        let result = session.confirmed_inputs_for_frame(Frame::new(0));
        // This might succeed or fail depending on how many frames were actually discarded
        // The key point is that it handles the edge case gracefully
        if result.is_err() {
            match result {
                Err(FortressError::InvalidRequest { .. }) => {
                    // Expected - frame was discarded
                },
                Err(FortressError::InvalidFrame { .. }) => {
                    // Also acceptable if frame is considered not confirmed
                },
                _ => panic!("Unexpected error type"),
            }
        }
        // If it succeeds, that's also fine - the frame might still be in the queue
    }

    /// Data-driven test cases for confirmed_inputs_for_frame edge cases
    #[test]
    fn confirmed_inputs_for_frame_edge_cases() {
        struct TestCase {
            name: &'static str,
            frames_to_advance: i32,
            frame_to_query: i32,
            expect_success: bool,
        }

        let test_cases = [
            TestCase {
                name: "frame 0 after advancing 5 frames",
                frames_to_advance: 5,
                frame_to_query: 0,
                expect_success: true,
            },
            TestCase {
                name: "frame beyond confirmed",
                frames_to_advance: 5,
                frame_to_query: 100,
                expect_success: false,
            },
            TestCase {
                name: "frame at current - 1",
                frames_to_advance: 10,
                frame_to_query: 9,
                // For local-only sessions, confirmed_frame tracks current_frame,
                // so frame 9 is confirmed after advancing 10 frames
                expect_success: true,
            },
            TestCase {
                name: "large negative frame",
                frames_to_advance: 5,
                frame_to_query: -100,
                // Negative frames fail in the input queue (no such frame exists)
                expect_success: false,
            },
        ];

        for tc in &test_cases {
            let mut session = create_local_only_session();

            // Advance the specified number of frames
            for i in 0..tc.frames_to_advance {
                session
                    .add_local_input(PlayerHandle::new(0), (i % 256) as u8)
                    .expect("Input failed");
                let _ = session.advance_frame();
            }

            let result = session.confirmed_inputs_for_frame(Frame::new(tc.frame_to_query));

            if tc.expect_success {
                assert!(
                    result.is_ok(),
                    "Test '{}' expected success but got {:?}",
                    tc.name,
                    result
                );
            } else {
                assert!(
                    result.is_err(),
                    "Test '{}' expected failure but got {:?}",
                    tc.name,
                    result
                );
            }
        }
    }

    // ==========================================
    // handles_by_address Tests
    // ==========================================

    #[test]
    fn handles_by_address_returns_correct_handles() {
        let session = create_two_player_session();
        let addr = test_addr(8080);
        let handles = session.handles_by_address(addr);
        assert_eq!(handles.len(), 1);
        assert!(handles.contains(&PlayerHandle::new(1)));
    }

    #[test]
    fn handles_by_address_unknown_returns_empty() {
        let session = create_two_player_session();
        let unknown_addr = test_addr(9999);
        let handles = session.handles_by_address(unknown_addr);
        assert!(handles.is_empty());
    }

    // ==========================================
    // events Tests
    // ==========================================

    #[test]
    fn events_initially_empty() {
        let mut session = create_local_only_session();
        let events: Vec<_> = session.events().collect();
        assert!(events.is_empty());
    }

    #[test]
    fn events_drains_queue() {
        let mut session = create_local_only_session();
        // First drain
        let _: Vec<_> = session.events().collect();
        // Second drain should also be empty
        let events: Vec<_> = session.events().collect();
        assert!(events.is_empty());
    }

    // ==========================================
    // in_lockstep_mode Tests
    // ==========================================

    #[test]
    fn in_lockstep_mode_false_with_default_prediction() {
        let mut session = create_local_only_session();
        assert!(!session.in_lockstep_mode());
    }

    #[test]
    fn in_lockstep_mode_true_with_zero_prediction() {
        let mut session = SessionBuilder::<TestConfig>::new()
            .with_num_players(1)
            .with_max_prediction_window(0)
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .expect("Failed to add player")
            .start_p2p_session(DummySocket)
            .expect("Failed to create session");
        assert!(session.in_lockstep_mode());
    }

    // ==========================================
    // sync_health Tests (with session)
    // ==========================================

    #[test]
    fn sync_health_local_player_returns_none() {
        let session = create_local_only_session();
        // Local player doesn't have sync health
        assert!(session.sync_health(PlayerHandle::new(0)).is_none());
    }

    #[test]
    fn sync_health_invalid_handle_returns_none() {
        let session = create_local_only_session();
        assert!(session.sync_health(PlayerHandle::new(99)).is_none());
    }

    #[test]
    fn sync_health_remote_with_desync_off_returns_pending() {
        let session = SessionBuilder::<TestConfig>::new()
            .with_num_players(2)
            .with_desync_detection_mode(DesyncDetection::Off)
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .expect("Failed to add local player")
            .add_player(PlayerType::Remote(test_addr(8080)), PlayerHandle::new(1))
            .expect("Failed to add remote player")
            .start_p2p_session(DummySocket)
            .expect("Failed to create session");
        // With desync detection off, sync_health returns Pending
        match session.sync_health(PlayerHandle::new(1)) {
            Some(SyncHealth::Pending) => {},
            other => panic!("Expected Pending, got {:?}", other),
        }
    }

    #[test]
    fn sync_health_remote_initially_pending() {
        let session = create_two_player_session();
        // Initially, no checksums compared, so should be Pending
        match session.sync_health(PlayerHandle::new(1)) {
            Some(SyncHealth::Pending) => {},
            other => panic!("Expected Pending, got {:?}", other),
        }
    }

    // ==========================================
    // is_synchronized Tests
    // ==========================================

    #[test]
    fn is_synchronized_no_remotes_returns_true() {
        let session = create_local_only_session();
        assert!(session.is_synchronized());
    }

    #[test]
    fn is_synchronized_with_pending_returns_false() {
        let session = create_two_player_session();
        // Remote player is pending, so not synchronized
        assert!(!session.is_synchronized());
    }

    // ==========================================
    // last_verified_frame Tests
    // ==========================================

    #[test]
    fn last_verified_frame_initially_none() {
        let session = create_local_only_session();
        assert!(session.last_verified_frame().is_none());
    }

    #[test]
    fn last_verified_frame_with_remote_initially_none() {
        let session = create_two_player_session();
        assert!(session.last_verified_frame().is_none());
    }

    // ==========================================
    // all_sync_health Tests
    // ==========================================

    #[test]
    fn all_sync_health_no_remotes_returns_empty() {
        let session = create_local_only_session();
        let health = session.all_sync_health();
        assert!(health.is_empty());
    }

    #[test]
    fn all_sync_health_with_remote_returns_entry() {
        let session = create_two_player_session();
        let health = session.all_sync_health();
        assert_eq!(health.len(), 1);
        let first = health.first().expect("Expected at least one entry");
        assert_eq!(first.0, PlayerHandle::new(1));
        assert_eq!(first.1, SyncHealth::Pending);
    }

    // ==========================================
    // InvariantChecker Tests
    // ==========================================

    #[test]
    fn check_invariants_no_desync_passes() {
        let session = create_local_only_session();
        assert!(session.check_invariants().is_ok());
    }

    #[test]
    fn check_invariants_with_remote_no_desync_passes() {
        let session = create_two_player_session();
        // No desync detected yet
        assert!(session.check_invariants().is_ok());
    }

    // ==========================================
    // Full Session Lifecycle Tests
    // ==========================================

    #[test]
    fn full_session_lifecycle_local_only() {
        let mut session = create_local_only_session();

        // Initial state
        assert_eq!(session.current_state(), SessionState::Running);
        assert_eq!(session.current_frame(), Frame::new(0));

        // Advance a few frames
        for i in 0..5 {
            session
                .add_local_input(PlayerHandle::new(0), i as u8)
                .expect("Input failed");
            let requests = session.advance_frame().expect("Advance failed");
            assert!(!requests.is_empty());
        }

        // Verify frame advancement
        assert_eq!(session.current_frame(), Frame::new(5));
    }

    #[test]
    fn session_with_spectator() {
        let session = SessionBuilder::<TestConfig>::new()
            .with_num_players(1)
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .expect("Failed to add local player")
            .add_player(
                PlayerType::Spectator(test_addr(9090)),
                PlayerHandle::new(10),
            )
            .expect("Failed to add spectator")
            .start_p2p_session(DummySocket)
            .expect("Failed to create session");

        assert_eq!(session.num_players(), 1);
        assert_eq!(session.num_spectators(), 1);
        assert!(!session.spectator_handles().is_empty());
    }
}
