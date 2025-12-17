//! # Fortress Rollback (formerly GGRS)
//!
//! <p align="center">
//!   <img src="https://raw.githubusercontent.com/wallstop/fortress-rollback/main/assets/logo-banner.svg" alt="Fortress Rollback" width="400">
//! </p>
//!
//! Fortress Rollback is a fortified, verified reimagination of the GGPO network SDK written in 100% safe Rust.
//! The callback-style API from the original library has been replaced with a simple request-driven control flow.
//! Instead of registering callback functions, Fortress Rollback (previously GGRS) returns a list of requests for the user to fulfill.

#![forbid(unsafe_code)] // let us try
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
//#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
use std::{fmt::Debug, hash::Hash};

pub use error::FortressError;
pub use network::chaos_socket::{ChaosConfig, ChaosConfigBuilder, ChaosSocket, ChaosStats};
pub use network::messages::Message;
pub use network::network_stats::NetworkStats;
pub use network::udp_socket::UdpNonBlockingSocket;
use serde::{de::DeserializeOwned, Serialize};
pub use sessions::builder::{
    InputQueueConfig, ProtocolConfig, SaveMode, SessionBuilder, SpectatorConfig, SyncConfig,
};
pub use sessions::p2p_session::{P2PSession, SyncHealth};
pub use sessions::p2p_spectator_session::SpectatorSession;
pub use sessions::sync_test_session::SyncTestSession;
pub use sync_layer::{GameStateAccessor, GameStateCell};
pub use time_sync::TimeSyncConfig;

// Re-export prediction strategies
pub use crate::input_queue::{BlankPrediction, PredictionStrategy, RepeatLastConfirmed};

// Internal modules - made pub for re-export in __internal, but doc(hidden) for API cleanliness
#[doc(hidden)]
pub mod error;
#[doc(hidden)]
pub mod frame_info;
pub mod hash;
#[doc(hidden)]
pub mod input_queue;
/// Internal run-length encoding module for network compression.
///
/// Provides RLE encoding/decoding that replaces the `bitfield-rle` crate dependency.
/// See the module documentation for usage details.
pub mod rle;
/// Internal random number generator module based on PCG32.
///
/// Provides a minimal, high-quality PRNG that replaces the `rand` crate dependency.
/// See the module documentation for usage details.
pub mod rng;
#[doc(hidden)]
pub mod sync;
#[doc(hidden)]
pub mod sync_layer;
pub mod telemetry;
#[doc(hidden)]
pub mod time_sync;
#[doc(hidden)]
pub mod sessions {
    #[doc(hidden)]
    pub mod builder;
    #[doc(hidden)]
    pub mod p2p_session;
    #[doc(hidden)]
    pub mod p2p_spectator_session;
    #[doc(hidden)]
    pub mod sync_test_session;
}
#[doc(hidden)]
pub mod network {
    pub mod chaos_socket;
    /// Binary codec for network message serialization.
    ///
    /// Provides centralized, zero-allocation-where-possible encoding and decoding
    /// of network messages using bincode.
    pub mod codec;
    #[doc(hidden)]
    pub mod compression;
    #[doc(hidden)]
    pub mod messages;
    #[doc(hidden)]
    pub mod network_stats;
    #[doc(hidden)]
    pub mod protocol;
    #[doc(hidden)]
    pub mod udp_socket;
}

/// Internal module exposing implementation details for testing, fuzzing, and formal verification.
///
/// # ⚠️ WARNING: No Stability Guarantees
///
/// **This module is NOT part of the public API.** Everything here is:
/// - Subject to change without notice
/// - Not covered by semver compatibility guarantees
/// - Intended ONLY for:
///   - Fuzzing (cargo-fuzz, libFuzzer, AFL)
///   - Property-based testing (proptest)
///   - Formal verification (Kani, Z3)
///   - Integration testing in the same workspace
///
/// **DO NOT** depend on anything in this module for production code.
/// **DO NOT** import these types in your game/application code.
///
/// # Rationale
///
/// Rollback networking has complex invariants that benefit from direct testing
/// of internal components:
/// - **InputQueue**: Circular buffer with prediction, frame delay, rollback semantics
/// - **SyncLayer**: Frame synchronization, state management, rollback coordination
/// - **TimeSync**: Time synchronization calculations and averaging
/// - **Compression**: Delta encoding for network efficiency
/// - **Protocol**: State machine for peer connections
///
/// By exposing these internals (with clear warnings), we enable:
/// 1. Higher fuzz coverage (direct component testing vs. through session APIs)
/// 2. Better fault isolation (pinpoint which component failed)
/// 3. Direct invariant testing (test component contracts directly)
/// 4. Same code paths for testing and production (no `#[cfg(test)]` divergence)
///
/// # Example: Fuzz Target
///
/// ```ignore
/// use fortress_rollback::__internal::{InputQueue, PlayerInput};
/// use fortress_rollback::Frame;
///
/// // Direct fuzzing of InputQueue (not possible without this module)
/// fuzz_target!(|ops: Vec<QueueOp>| {
///     let mut queue = InputQueue::<TestConfig>::with_queue_length(0, 32);
///     for op in ops {
///         match op {
///             QueueOp::Add(frame, input) => queue.add_input(PlayerInput::new(frame, input)),
///             QueueOp::Get(frame) => queue.input(frame),
///             // ...
///         }
///     }
/// });
/// ```
#[doc(hidden)]
pub mod __internal {

    // Core types
    pub use crate::frame_info::{GameState, PlayerInput};
    pub use crate::input_queue::{InputQueue, INPUT_QUEUE_LENGTH, MAX_FRAME_DELAY};
    pub use crate::sync_layer::{GameStateCell, SavedStates, SyncLayer};
    pub use crate::time_sync::TimeSync;

    // Network internals
    pub use crate::network::compression::{decode, delta_decode, delta_encode, encode};
    pub use crate::network::messages::ConnectionStatus;
    pub use crate::network::protocol::{Event, ProtocolState, UdpProtocol};

    // RLE compression (internal implementation)
    pub use crate::rle::{decode as rle_decode, encode as rle_encode};

    // Session internals
    pub use crate::sessions::p2p_session::PlayerRegistry;
}

// #############
// # CONSTANTS #
// #############

/// Internally, -1 represents no frame / invalid frame.
///
/// # Formal Specification Alignment
/// - **TLA+**: `NULL_FRAME = 999` in `specs/tla/*.cfg` (uses 999 to stay in Nat domain)
/// - **Z3**: `NULL_FRAME = -1` in `tests/test_z3_verification.rs`
/// - **formal-spec.md**: `NULL_FRAME = -1`, with `VALID_FRAME(f) ↔ f ≥ 0`
pub const NULL_FRAME: i32 = -1;

/// A frame is a single step of game execution.
///
/// Frames are the fundamental unit of time in rollback networking. Each frame
/// represents one discrete step of game simulation. Frame numbers start at 0
/// and increment sequentially.
///
/// The special value [`NULL_FRAME`] (-1) represents "no frame" or "uninitialized".
///
/// # Formal Specification Alignment
/// - **TLA+**: `Frame == {NULL_FRAME} ∪ (0..MAX_FRAME)` in `specs/tla/Rollback.tla`
/// - **Z3**: Frame arithmetic proofs in `tests/test_z3_verification.rs`
/// - **formal-spec.md**: Core type definition with operations `frame_add`, `frame_sub`, `frame_valid`
/// - **Kani**: `kani_frame_*` proofs verify overflow safety and arithmetic correctness
///
/// # Type Safety
///
/// `Frame` is a newtype wrapper around `i32` that provides:
/// - Clear semantic meaning (frames vs arbitrary integers)
/// - Helper methods like [`is_null()`](Frame::is_null) and [`is_valid()`](Frame::is_valid)
/// - Arithmetic operations for frame calculations
/// - Compile-time prevention of accidentally mixing frames with other integers
///
/// # Examples
///
/// ```
/// use fortress_rollback::{Frame, NULL_FRAME};
///
/// // Creating frames
/// let frame = Frame::new(0);
/// let null_frame = Frame::NULL;
///
/// // Checking validity
/// assert!(frame.is_valid());
/// assert!(null_frame.is_null());
///
/// // Frame arithmetic
/// let next_frame = frame + 1;
/// assert_eq!(next_frame.as_i32(), 1);
///
/// // Comparison
/// assert!(next_frame > frame);
/// ```
#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct Frame(i32);

impl Frame {
    /// The null frame constant, representing "no frame" or "uninitialized".
    ///
    /// This is equivalent to [`NULL_FRAME`] (-1).
    pub const NULL: Frame = Frame(NULL_FRAME);

    /// Creates a new `Frame` from an `i32` value.
    ///
    /// Note: This does not validate the frame number. Use [`Frame::is_valid()`]
    /// to check if the frame represents a valid (non-negative) frame number.
    #[inline]
    #[must_use]
    pub const fn new(frame: i32) -> Self {
        Frame(frame)
    }

    /// Returns the underlying `i32` value.
    #[inline]
    #[must_use]
    pub const fn as_i32(self) -> i32 {
        self.0
    }

    /// Returns `true` if this frame is the null frame (equivalent to [`NULL_FRAME`]).
    ///
    /// # Examples
    ///
    /// ```
    /// use fortress_rollback::Frame;
    ///
    /// assert!(Frame::NULL.is_null());
    /// assert!(!Frame::new(0).is_null());
    /// ```
    #[inline]
    #[must_use]
    pub const fn is_null(self) -> bool {
        self.0 == NULL_FRAME
    }

    /// Returns `true` if this frame is valid (non-negative).
    ///
    /// # Examples
    ///
    /// ```
    /// use fortress_rollback::Frame;
    ///
    /// assert!(Frame::new(0).is_valid());
    /// assert!(Frame::new(100).is_valid());
    /// assert!(!Frame::NULL.is_valid());
    /// assert!(!Frame::new(-5).is_valid());
    /// ```
    #[inline]
    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.0 >= 0
    }

    /// Returns `Some(self)` if the frame is valid, or `None` if it's null or negative.
    ///
    /// This is useful for handling the null/valid frame pattern with Option.
    #[inline]
    #[must_use]
    pub const fn to_option(self) -> Option<Frame> {
        if self.is_valid() {
            Some(self)
        } else {
            None
        }
    }

    /// Creates a Frame from an Option, using NULL for None.
    #[inline]
    #[must_use]
    pub const fn from_option(opt: Option<Frame>) -> Frame {
        match opt {
            Some(f) => f,
            None => Frame::NULL,
        }
    }
}

impl std::fmt::Display for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_null() {
            write!(f, "NULL_FRAME")
        } else {
            write!(f, "{}", self.0)
        }
    }
}

// Arithmetic operations

impl std::ops::Add<i32> for Frame {
    type Output = Frame;

    #[inline]
    fn add(self, rhs: i32) -> Self::Output {
        Frame(self.0 + rhs)
    }
}

impl std::ops::Add<Frame> for Frame {
    type Output = Frame;

    #[inline]
    fn add(self, rhs: Frame) -> Self::Output {
        Frame(self.0 + rhs.0)
    }
}

impl std::ops::AddAssign<i32> for Frame {
    #[inline]
    fn add_assign(&mut self, rhs: i32) {
        self.0 += rhs;
    }
}

impl std::ops::Sub<i32> for Frame {
    type Output = Frame;

    #[inline]
    fn sub(self, rhs: i32) -> Self::Output {
        Frame(self.0 - rhs)
    }
}

impl std::ops::Sub<Frame> for Frame {
    type Output = i32;

    #[inline]
    fn sub(self, rhs: Frame) -> Self::Output {
        self.0 - rhs.0
    }
}

impl std::ops::SubAssign<i32> for Frame {
    #[inline]
    fn sub_assign(&mut self, rhs: i32) {
        self.0 -= rhs;
    }
}

impl std::ops::Rem<i32> for Frame {
    type Output = i32;

    #[inline]
    fn rem(self, rhs: i32) -> Self::Output {
        self.0 % rhs
    }
}

// Conversion traits for backwards compatibility

impl From<i32> for Frame {
    #[inline]
    fn from(value: i32) -> Self {
        Frame(value)
    }
}

impl From<Frame> for i32 {
    #[inline]
    fn from(frame: Frame) -> Self {
        frame.0
    }
}

impl From<usize> for Frame {
    #[inline]
    fn from(value: usize) -> Self {
        Frame(value as i32)
    }
}

// Comparison with i32 for convenience

impl PartialEq<i32> for Frame {
    #[inline]
    fn eq(&self, other: &i32) -> bool {
        self.0 == *other
    }
}

impl PartialOrd<i32> for Frame {
    #[inline]
    fn partial_cmp(&self, other: &i32) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}

/// A unique identifier for a player or spectator in a session.
///
/// Player handles are the primary way to reference participants in a Fortress Rollback
/// session. Each player or spectator is assigned a unique handle when added to the session.
///
/// # Handle Ranges
///
/// - **Players**: Handles `0` through `num_players - 1` are reserved for active players
/// - **Spectators**: Handles `num_players` and above are used for spectators
///
/// # Type Safety
///
/// `PlayerHandle` is a newtype wrapper around `usize` that provides:
/// - Clear semantic meaning (player identifiers vs arbitrary integers)
/// - Helper methods like [`is_spectator_for()`](PlayerHandle::is_spectator_for)
/// - Compile-time prevention of accidentally mixing handles with other integers
///
/// # Examples
///
/// ```
/// use fortress_rollback::PlayerHandle;
///
/// // Creating handles
/// let player = PlayerHandle::new(0);
/// let spectator = PlayerHandle::new(2); // In a 2-player game
///
/// // Checking if a handle is for a spectator
/// assert!(!player.is_spectator_for(2));
/// assert!(spectator.is_spectator_for(2));
///
/// // Getting the raw value
/// assert_eq!(player.as_usize(), 0);
/// ```
#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct PlayerHandle(usize);

impl PlayerHandle {
    /// Creates a new `PlayerHandle` from a `usize` value.
    ///
    /// Note: This does not validate the handle against a specific session.
    /// Use [`is_valid_player_for()`](Self::is_valid_player_for) or
    /// [`is_spectator_for()`](Self::is_spectator_for) to check validity.
    #[inline]
    #[must_use]
    pub const fn new(handle: usize) -> Self {
        PlayerHandle(handle)
    }

    /// Returns the underlying `usize` value.
    #[inline]
    #[must_use]
    pub const fn as_usize(self) -> usize {
        self.0
    }

    /// Returns `true` if this handle refers to a valid player (not spectator)
    /// for a session with the given number of players.
    ///
    /// # Examples
    ///
    /// ```
    /// use fortress_rollback::PlayerHandle;
    ///
    /// let handle = PlayerHandle::new(1);
    /// assert!(handle.is_valid_player_for(2));  // Valid for 2-player session
    /// assert!(!handle.is_valid_player_for(1)); // Invalid for 1-player session
    /// ```
    #[inline]
    #[must_use]
    pub const fn is_valid_player_for(self, num_players: usize) -> bool {
        self.0 < num_players
    }

    /// Returns `true` if this handle refers to a spectator
    /// for a session with the given number of players.
    ///
    /// # Examples
    ///
    /// ```
    /// use fortress_rollback::PlayerHandle;
    ///
    /// let handle = PlayerHandle::new(2);
    /// assert!(handle.is_spectator_for(2));  // Spectator in 2-player session
    /// assert!(!handle.is_spectator_for(3)); // Player in 3-player session
    /// ```
    #[inline]
    #[must_use]
    pub const fn is_spectator_for(self, num_players: usize) -> bool {
        self.0 >= num_players
    }
}

impl std::fmt::Display for PlayerHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Conversion traits for backwards compatibility

impl From<usize> for PlayerHandle {
    #[inline]
    fn from(value: usize) -> Self {
        PlayerHandle(value)
    }
}

impl From<PlayerHandle> for usize {
    #[inline]
    fn from(handle: PlayerHandle) -> Self {
        handle.0
    }
}

// #############
// #   ENUMS   #
// #############

/// Desync detection by comparing checksums between peers.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DesyncDetection {
    /// Desync detection is turned on with a specified interval rate given by the user.
    On {
        /// interval rate given by the user. e.g. at 60hz an interval of 10 results to 6 reports a second.
        interval: u32,
    },
    /// Desync detection is turned off
    Off,
}

/// Defines the three types of players that Fortress Rollback considers:
/// - local players, who play on the local device,
/// - remote players, who play on other devices and
/// - spectators, who are remote players that do not contribute to the game input.
///
/// Both [`PlayerType::Remote`] and [`PlayerType::Spectator`] have a socket address associated with them.
#[derive(Debug, Default, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum PlayerType<A>
where
    A: Clone + PartialEq + Eq + PartialOrd + Ord + Hash,
{
    /// This player plays on the local device.
    #[default]
    Local,
    /// This player plays on a remote device identified by the socket address.
    Remote(A),
    /// This player spectates on a remote device identified by the socket address. They do not contribute to the game input.
    Spectator(A),
}

/// A session is always in one of these states. You can query the current state of a session via [`current_state`].
///
/// [`current_state`]: P2PSession#method.current_state
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SessionState {
    /// When synchronizing, the session attempts to establish a connection to the remote clients.
    Synchronizing,
    /// When running, the session has synchronized and is ready to take and transmit player input.
    Running,
}

/// [`InputStatus`] will always be given together with player inputs when requested to advance the frame.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum InputStatus {
    /// The input of this player for this frame is an actual received input.
    Confirmed,
    /// The input of this player for this frame is predicted.
    Predicted,
    /// The player has disconnected at or prior to this frame, so this input is a dummy.
    Disconnected,
}

/// Notifications that you can receive from the session. Handling them is up to the user.
///
/// # Forward Compatibility
///
/// This enum is marked `#[non_exhaustive]` because new event types may be
/// added in future versions. Always include a wildcard arm when matching:
///
/// ```ignore
/// match event {
///     FortressEvent::Synchronized { addr } => { /* handle */ }
///     FortressEvent::Disconnected { addr } => { /* handle */ }
///     _ => { /* handle unknown events */ }
/// }
/// ```
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FortressEvent<T>
where
    T: Config,
{
    /// The session made progress in synchronizing. After `total` roundtrips, the session are synchronized.
    Synchronizing {
        /// The address of the endpoint.
        addr: T::Address,
        /// Total number of required successful synchronization steps.
        total: u32,
        /// Current number of successful synchronization steps.
        count: u32,
        /// Total sync requests sent (includes retries due to packet loss).
        /// Higher values indicate network issues during synchronization.
        total_requests_sent: u32,
        /// Milliseconds elapsed since synchronization started.
        /// Useful for detecting slow sync due to high latency or packet loss.
        elapsed_ms: u128,
    },
    /// The session is now synchronized with the remote client.
    Synchronized {
        /// The address of the endpoint.
        addr: T::Address,
    },
    /// The remote client has disconnected.
    Disconnected {
        /// The address of the endpoint.
        addr: T::Address,
    },
    /// The session has not received packets from the remote client for some time and will disconnect the remote in `disconnect_timeout` ms.
    NetworkInterrupted {
        /// The address of the endpoint.
        addr: T::Address,
        /// The client will be disconnected in this amount of ms.
        disconnect_timeout: u128,
    },
    /// Sent only after a [`FortressEvent::NetworkInterrupted`] event, if communication with that player has resumed.
    NetworkResumed {
        /// The address of the endpoint.
        addr: T::Address,
    },
    /// Sent out if Fortress Rollback recommends skipping a few frames to let clients catch up. If you receive this, consider waiting `skip_frames` number of frames.
    WaitRecommendation {
        /// Amount of frames recommended to be skipped in order to let other clients catch up.
        skip_frames: u32,
    },
    /// Sent whenever Fortress Rollback locally detected a discrepancy between local and remote checksums
    DesyncDetected {
        /// Frame of the checksums
        frame: Frame,
        /// local checksum for the given frame
        local_checksum: u128,
        /// remote checksum for the given frame
        remote_checksum: u128,
        /// remote address of the endpoint.
        addr: T::Address,
    },
    /// Synchronization has timed out. This is only emitted if a sync timeout was configured
    /// via [`SyncConfig`]. The session will continue trying to sync, but the user may choose
    /// to abort and disconnect.
    SyncTimeout {
        /// The address of the endpoint that timed out.
        addr: T::Address,
        /// Milliseconds elapsed since synchronization started.
        elapsed_ms: u128,
    },
}

/// Requests that you can receive from the session. Handling them is mandatory.
///
/// # ⚠️ CRITICAL: Request Ordering
///
/// **Requests MUST be fulfilled in the exact order they are returned.** The session
/// returns requests in a specific sequence that ensures correct simulation:
///
/// ```text
/// ┌──────────────────────────────────────────────────────────────┐
/// │                    Request Flow                               │
/// ├──────────────────────────────────────────────────────────────┤
/// │ 1. SaveGameState  ─► Save current state before advancing     │
/// │         ↓                                                     │
/// │ 2. LoadGameState  ─► (During rollback) Load earlier state    │
/// │         ↓                                                     │
/// │ 3. AdvanceFrame   ─► Apply inputs and advance simulation     │
/// └──────────────────────────────────────────────────────────────┘
/// ```
///
/// # Why Order Matters
///
/// - **`SaveGameState` before `AdvanceFrame`**: Ensures the state can be rolled
///   back if a misprediction is detected later.
/// - **`LoadGameState` resets simulation**: When rollback occurs, loading
///   restores an earlier known-correct state.
/// - **`AdvanceFrame` uses loaded state**: After a load, advance applies
///   corrected inputs to the restored state.
///
/// # Consequences of Wrong Ordering
///
/// Processing requests out of order will cause:
/// - **Desyncs**: Wrong state saved/loaded, causing peers to diverge
/// - **Incorrect simulation**: Inputs applied to wrong state
/// - **Assertion failures**: Internal invariants violated
///
/// # Example
///
/// ```ignore
/// let requests = session.advance_frame()?;
/// // Process in order - DO NOT reorder!
/// for request in requests {
///     match request {
///         FortressRequest::SaveGameState { cell, frame } => {
///             let checksum = compute_checksum(&game_state);
///             cell.save(frame, Some(game_state.clone()), Some(checksum));
///         }
///         FortressRequest::LoadGameState { cell, frame } => {
///             if let Some(state) = cell.load() {
///                 game_state = state;
///             }
///         }
///         FortressRequest::AdvanceFrame { inputs } => {
///             game_state.update(&inputs);
///         }
///         _ => panic!("Unknown request type"),
///     }
/// }
/// ```
///
/// # Forward Compatibility
///
/// This enum is marked `#[non_exhaustive]` because new request types may be
/// added in future versions. Always include a wildcard arm when matching.
#[non_exhaustive]
pub enum FortressRequest<T>
where
    T: Config,
{
    /// You should save the current gamestate in the `cell` provided to you. The given `frame` is a sanity check: The gamestate you save should be from that frame.
    SaveGameState {
        /// Use `cell.save(...)` to save your state.
        cell: GameStateCell<T::State>,
        /// The given `frame` is a sanity check: The gamestate you save should be from that frame.
        frame: Frame,
    },
    /// You should load the gamestate in the `cell` provided to you. The given `frame` is a sanity check: The gamestate you load should be from that frame.
    LoadGameState {
        /// Use `cell.load()` to load your state.
        cell: GameStateCell<T::State>,
        /// The given `frame` is a sanity check: The gamestate you load is from that frame.
        frame: Frame,
    },
    /// You should advance the gamestate with the `inputs` provided to you.
    /// Disconnected players are indicated by having [`NULL_FRAME`] instead of the correct current frame in their input.
    AdvanceFrame {
        /// Contains inputs and input status for each player.
        inputs: Vec<(T::Input, InputStatus)>,
    },
}

// #############
// #  TRAITS   #
// #############

//  special thanks to james7132 for the idea of a config trait that bundles all generics

/// Compile time parameterization for sessions.
///
/// This trait bundles the generic types needed for a session. Implement this on
/// a marker struct to configure your session types.
///
/// # Example
///
/// ```
/// use fortress_rollback::Config;
/// use serde::{Deserialize, Serialize};
/// use std::net::SocketAddr;
///
/// // Your game's input type
/// #[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
/// struct GameInput {
///     buttons: u8,
///     stick_x: i8,
///     stick_y: i8,
/// }
///
/// // Your game's state (for save/load)
/// #[derive(Clone)]
/// struct GameState {
///     frame: i32,
///     // ... game-specific state
/// }
///
/// // Marker struct for Config
/// struct GameConfig;
///
/// impl Config for GameConfig {
///     type Input = GameInput;
///     type State = GameState;
///     type Address = SocketAddr; // Most common choice for UDP games
/// }
/// ```
///
/// # Common Patterns
///
/// - **UDP Games**: Use `std::net::SocketAddr` for `Address`
/// - **WebRTC/Browser**: Use a custom address type from your WebRTC library
/// - **Local Testing**: Any `Clone + PartialEq + Eq + Ord + Hash + Debug` type works
#[cfg(feature = "sync-send")]
pub trait Config: 'static + Send + Sync {
    /// The input type for a session. This is the only game-related data
    /// transmitted over the network.
    ///
    /// The implementation of [Default] is used for representing "no input" for
    /// a player, including when a player is disconnected.
    type Input: Copy + Clone + PartialEq + Default + Serialize + DeserializeOwned + Send + Sync;

    /// The save state type for the session.
    type State: Clone + Send + Sync;

    /// The address type which identifies the remote clients
    type Address: Clone + PartialEq + Eq + PartialOrd + Ord + Hash + Send + Sync + Debug;
}

/// This [`NonBlockingSocket`] trait is used when you want to use Fortress Rollback with your own socket.
/// However you wish to send and receive messages, it should be implemented through these two methods.
/// Messages should be sent in an UDP-like fashion, unordered and unreliable.
/// Fortress Rollback has an internal protocol on top of this to make sure all important information is sent and received.
#[cfg(feature = "sync-send")]
pub trait NonBlockingSocket<A>: Send + Sync
where
    A: Clone + PartialEq + Eq + Hash + Send + Sync,
{
    /// Takes a [`Message`] and sends it to the given address.
    fn send_to(&mut self, msg: &Message, addr: &A);

    /// This method should return all messages received since the last time this method was called.
    /// The pairs `(A, Message)` indicate from which address each packet was received.
    fn receive_all_messages(&mut self) -> Vec<(A, Message)>;
}

/// Compile time parameterization for sessions.
#[cfg(not(feature = "sync-send"))]
pub trait Config: 'static {
    /// The input type for a session. This is the only game-related data
    /// transmitted over the network.
    ///
    /// The implementation of [Default] is used for representing "no input" for
    /// a player, including when a player is disconnected.
    type Input: Copy + Clone + PartialEq + Default + Serialize + DeserializeOwned;

    /// The save state type for the session.
    type State;

    /// The address type which identifies the remote clients
    type Address: Clone + PartialEq + Eq + PartialOrd + Ord + Hash + Debug;
}

/// This [`NonBlockingSocket`] trait is used when you want to use Fortress Rollback with your own socket.
/// However you wish to send and receive messages, it should be implemented through these two methods.
/// Messages should be sent in an UDP-like fashion, unordered and unreliable.
/// Fortress Rollback has an internal protocol on top of this to make sure all important information is sent and received.
#[cfg(not(feature = "sync-send"))]
pub trait NonBlockingSocket<A>
where
    A: Clone + PartialEq + Eq + Hash,
{
    /// Takes a [`Message`] and sends it to the given address.
    fn send_to(&mut self, msg: &Message, addr: &A);

    /// This method should return all messages received since the last time this method was called.
    /// The pairs `(A, Message)` indicate from which address each packet was received.
    fn receive_all_messages(&mut self) -> Vec<(A, Message)>;
}

// ###################
// # UNIT TESTS      #
// ###################

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;

    /// A minimal test configuration for unit testing.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct TestConfig;

    impl Config for TestConfig {
        type Input = u8;
        type State = Vec<u8>;
        type Address = SocketAddr;
    }

    fn test_addr(port: u16) -> SocketAddr {
        use std::net::{IpAddr, Ipv4Addr};
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)
    }

    // ==========================================
    // SessionState Tests
    // ==========================================

    #[test]
    fn session_state_default_values_exist() {
        // Verify both variants are constructible
        assert!(matches!(
            SessionState::Synchronizing,
            SessionState::Synchronizing
        ));
        assert!(matches!(SessionState::Running, SessionState::Running));
    }

    #[test]
    fn session_state_equality() {
        assert_eq!(SessionState::Synchronizing, SessionState::Synchronizing);
        assert_eq!(SessionState::Running, SessionState::Running);
        assert_ne!(SessionState::Synchronizing, SessionState::Running);
    }

    #[test]
    fn session_state_clone() {
        let state = SessionState::Running;
        let cloned = state;
        assert_eq!(state, cloned);
    }

    #[test]
    fn session_state_copy() {
        let state = SessionState::Synchronizing;
        let copied: SessionState = state;
        assert_eq!(state, copied);
    }

    #[test]
    fn session_state_debug_format() {
        let sync = SessionState::Synchronizing;
        let running = SessionState::Running;
        assert_eq!(format!("{:?}", sync), "Synchronizing");
        assert_eq!(format!("{:?}", running), "Running");
    }

    // ==========================================
    // InputStatus Tests
    // ==========================================

    #[test]
    fn input_status_variants_exist() {
        // Verify all variants are constructible
        assert!(matches!(InputStatus::Confirmed, InputStatus::Confirmed));
        assert!(matches!(InputStatus::Predicted, InputStatus::Predicted));
        assert!(matches!(
            InputStatus::Disconnected,
            InputStatus::Disconnected
        ));
    }

    #[test]
    fn input_status_equality() {
        assert_eq!(InputStatus::Confirmed, InputStatus::Confirmed);
        assert_eq!(InputStatus::Predicted, InputStatus::Predicted);
        assert_eq!(InputStatus::Disconnected, InputStatus::Disconnected);
        assert_ne!(InputStatus::Confirmed, InputStatus::Predicted);
        assert_ne!(InputStatus::Confirmed, InputStatus::Disconnected);
        assert_ne!(InputStatus::Predicted, InputStatus::Disconnected);
    }

    #[test]
    fn input_status_clone() {
        let status = InputStatus::Predicted;
        let cloned = status;
        assert_eq!(status, cloned);
    }

    #[test]
    fn input_status_copy() {
        let status = InputStatus::Confirmed;
        let copied: InputStatus = status;
        assert_eq!(status, copied);
    }

    #[test]
    fn input_status_debug_format() {
        assert_eq!(format!("{:?}", InputStatus::Confirmed), "Confirmed");
        assert_eq!(format!("{:?}", InputStatus::Predicted), "Predicted");
        assert_eq!(format!("{:?}", InputStatus::Disconnected), "Disconnected");
    }

    // ==========================================
    // FortressEvent Tests
    // ==========================================

    #[test]
    fn fortress_event_synchronizing() {
        let event: FortressEvent<TestConfig> = FortressEvent::Synchronizing {
            addr: test_addr(8080),
            total: 5,
            count: 2,
            total_requests_sent: 3,
            elapsed_ms: 100,
        };

        if let FortressEvent::Synchronizing {
            total,
            count,
            total_requests_sent,
            elapsed_ms,
            ..
        } = event
        {
            assert_eq!(total, 5);
            assert_eq!(count, 2);
            assert_eq!(total_requests_sent, 3);
            assert_eq!(elapsed_ms, 100);
        } else {
            panic!("Expected Synchronizing event");
        }
    }

    #[test]
    fn fortress_event_synchronized() {
        let addr = test_addr(8080);
        let event: FortressEvent<TestConfig> = FortressEvent::Synchronized { addr };

        if let FortressEvent::Synchronized { addr: received } = event {
            assert_eq!(received, addr);
        } else {
            panic!("Expected Synchronized event");
        }
    }

    #[test]
    fn fortress_event_disconnected() {
        let addr = test_addr(9000);
        let event: FortressEvent<TestConfig> = FortressEvent::Disconnected { addr };

        if let FortressEvent::Disconnected { addr: received } = event {
            assert_eq!(received, addr);
        } else {
            panic!("Expected Disconnected event");
        }
    }

    #[test]
    fn fortress_event_network_interrupted() {
        let event: FortressEvent<TestConfig> = FortressEvent::NetworkInterrupted {
            addr: test_addr(8080),
            disconnect_timeout: 5000,
        };

        if let FortressEvent::NetworkInterrupted {
            disconnect_timeout, ..
        } = event
        {
            assert_eq!(disconnect_timeout, 5000);
        } else {
            panic!("Expected NetworkInterrupted event");
        }
    }

    #[test]
    fn fortress_event_network_resumed() {
        let addr = test_addr(8080);
        let event: FortressEvent<TestConfig> = FortressEvent::NetworkResumed { addr };

        if let FortressEvent::NetworkResumed { addr: received } = event {
            assert_eq!(received, addr);
        } else {
            panic!("Expected NetworkResumed event");
        }
    }

    #[test]
    fn fortress_event_wait_recommendation() {
        let event: FortressEvent<TestConfig> = FortressEvent::WaitRecommendation { skip_frames: 3 };

        if let FortressEvent::WaitRecommendation { skip_frames } = event {
            assert_eq!(skip_frames, 3);
        } else {
            panic!("Expected WaitRecommendation event");
        }
    }

    #[test]
    fn fortress_event_desync_detected() {
        let event: FortressEvent<TestConfig> = FortressEvent::DesyncDetected {
            frame: Frame::new(100),
            local_checksum: 0x1234,
            remote_checksum: 0x5678,
            addr: test_addr(8080),
        };

        if let FortressEvent::DesyncDetected {
            frame,
            local_checksum,
            remote_checksum,
            ..
        } = event
        {
            assert_eq!(frame, Frame::new(100));
            assert_eq!(local_checksum, 0x1234);
            assert_eq!(remote_checksum, 0x5678);
        } else {
            panic!("Expected DesyncDetected event");
        }
    }

    #[test]
    fn fortress_event_sync_timeout() {
        let event: FortressEvent<TestConfig> = FortressEvent::SyncTimeout {
            addr: test_addr(8080),
            elapsed_ms: 10000,
        };

        if let FortressEvent::SyncTimeout { elapsed_ms, .. } = event {
            assert_eq!(elapsed_ms, 10000);
        } else {
            panic!("Expected SyncTimeout event");
        }
    }

    #[test]
    fn fortress_event_equality() {
        let event1: FortressEvent<TestConfig> =
            FortressEvent::WaitRecommendation { skip_frames: 5 };
        let event2: FortressEvent<TestConfig> =
            FortressEvent::WaitRecommendation { skip_frames: 5 };
        let event3: FortressEvent<TestConfig> =
            FortressEvent::WaitRecommendation { skip_frames: 10 };

        assert_eq!(event1, event2);
        assert_ne!(event1, event3);
    }

    #[test]
    fn fortress_event_clone() {
        let event: FortressEvent<TestConfig> = FortressEvent::Synchronized {
            addr: test_addr(8080),
        };
        let cloned = event;
        assert_eq!(event, cloned);
    }

    #[test]
    fn fortress_event_debug_format() {
        let event: FortressEvent<TestConfig> = FortressEvent::WaitRecommendation { skip_frames: 3 };
        let debug = format!("{:?}", event);
        assert!(debug.contains("WaitRecommendation"));
        assert!(debug.contains('3'));
    }

    // ==========================================
    // PlayerType Tests
    // ==========================================

    #[test]
    fn player_type_local() {
        let player_type: PlayerType<SocketAddr> = PlayerType::Local;
        assert!(matches!(player_type, PlayerType::Local));
    }

    #[test]
    fn player_type_remote() {
        let addr = test_addr(8080);
        let player_type: PlayerType<SocketAddr> = PlayerType::Remote(addr);

        if let PlayerType::Remote(received) = player_type {
            assert_eq!(received, addr);
        } else {
            panic!("Expected Remote player type");
        }
    }

    #[test]
    fn player_type_spectator() {
        let addr = test_addr(9000);
        let player_type: PlayerType<SocketAddr> = PlayerType::Spectator(addr);

        if let PlayerType::Spectator(received) = player_type {
            assert_eq!(received, addr);
        } else {
            panic!("Expected Spectator player type");
        }
    }

    #[test]
    fn player_type_equality() {
        let addr1 = test_addr(8080);
        let addr2 = test_addr(9000);

        assert_eq!(
            PlayerType::<SocketAddr>::Local,
            PlayerType::<SocketAddr>::Local
        );
        assert_eq!(PlayerType::Remote(addr1), PlayerType::Remote(addr1));
        assert_ne!(PlayerType::Remote(addr1), PlayerType::Remote(addr2));
        assert_ne!(PlayerType::<SocketAddr>::Local, PlayerType::Remote(addr1));
    }

    #[test]
    fn player_type_clone() {
        let player_type: PlayerType<SocketAddr> = PlayerType::Remote(test_addr(8080));
        let cloned = player_type; // PlayerType is Copy
        assert_eq!(player_type, cloned);
    }

    #[test]
    fn player_type_debug_format() {
        let local: PlayerType<SocketAddr> = PlayerType::Local;
        assert_eq!(format!("{:?}", local), "Local");

        let remote: PlayerType<SocketAddr> = PlayerType::Remote(test_addr(8080));
        let debug = format!("{:?}", remote);
        assert!(debug.contains("Remote"));
    }

    // ==========================================
    // PlayerHandle Tests
    // ==========================================

    #[test]
    fn player_handle_new() {
        let handle = PlayerHandle::new(0);
        assert_eq!(handle.as_usize(), 0);

        let handle = PlayerHandle::new(5);
        assert_eq!(handle.as_usize(), 5);
    }

    #[test]
    fn player_handle_is_valid_player_for() {
        let handle = PlayerHandle::new(0);
        assert!(handle.is_valid_player_for(2));
        assert!(handle.is_valid_player_for(1));
        assert!(!handle.is_valid_player_for(0));

        let handle = PlayerHandle::new(5);
        assert!(handle.is_valid_player_for(6));
        assert!(!handle.is_valid_player_for(5));
        assert!(!handle.is_valid_player_for(4));
    }

    #[test]
    fn player_handle_is_spectator_for() {
        let handle = PlayerHandle::new(0);
        assert!(!handle.is_spectator_for(2));

        let handle = PlayerHandle::new(2);
        assert!(handle.is_spectator_for(2));
        assert!(!handle.is_spectator_for(3));
    }

    #[test]
    fn player_handle_equality() {
        let handle1 = PlayerHandle::new(1);
        let handle2 = PlayerHandle::new(1);
        let handle3 = PlayerHandle::new(2);

        assert_eq!(handle1, handle2);
        assert_ne!(handle1, handle3);
    }

    #[test]
    fn player_handle_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(PlayerHandle::new(0));
        set.insert(PlayerHandle::new(1));
        set.insert(PlayerHandle::new(0)); // duplicate

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn player_handle_ordering() {
        let h0 = PlayerHandle::new(0);
        let h1 = PlayerHandle::new(1);
        let h2 = PlayerHandle::new(2);

        assert!(h0 < h1);
        assert!(h1 < h2);
        assert!(h0 < h2);
    }

    #[test]
    fn player_handle_debug_format() {
        let handle = PlayerHandle::new(3);
        let debug = format!("{:?}", handle);
        assert!(debug.contains('3'));
    }

    // ==========================================
    // Frame Tests (additional tests beyond kani)
    // ==========================================

    #[test]
    fn frame_null_constant() {
        assert_eq!(Frame::NULL.as_i32(), -1);
        assert!(Frame::NULL.is_null());
        assert!(!Frame::NULL.is_valid());
    }

    #[test]
    fn frame_new() {
        let frame = Frame::new(0);
        assert_eq!(frame.as_i32(), 0);
        assert!(!frame.is_null());
        assert!(frame.is_valid());

        let frame = Frame::new(100);
        assert_eq!(frame.as_i32(), 100);
    }

    #[test]
    fn frame_arithmetic() {
        let frame = Frame::new(10);

        // Addition with i32
        assert_eq!((frame + 5).as_i32(), 15);

        // Subtraction with i32
        assert_eq!((frame - 3).as_i32(), 7);

        // Subtraction between frames
        assert_eq!(Frame::new(10) - Frame::new(5), 5);
    }

    #[test]
    fn frame_add_assign() {
        let mut frame = Frame::new(10);
        frame += 5;
        assert_eq!(frame.as_i32(), 15);
    }

    #[test]
    fn frame_sub_assign() {
        let mut frame = Frame::new(10);
        frame -= 3;
        assert_eq!(frame.as_i32(), 7);
    }

    #[test]
    fn frame_comparison() {
        let f1 = Frame::new(5);
        let f2 = Frame::new(10);
        let f3 = Frame::new(5);

        assert!(f1 < f2);
        assert!(f2 > f1);
        assert!(f1 <= f3);
        assert!(f1 >= f3);
        assert_eq!(f1, f3);
    }

    #[test]
    fn frame_modulo() {
        let frame = Frame::new(135);
        let remainder = frame % 128;
        assert_eq!(remainder, 7);
    }

    #[test]
    fn frame_to_option() {
        assert!(Frame::NULL.to_option().is_none());
        assert_eq!(Frame::new(5).to_option(), Some(Frame::new(5)));
    }

    #[test]
    fn frame_from_option() {
        assert_eq!(Frame::from_option(None), Frame::NULL);
        assert_eq!(Frame::from_option(Some(Frame::new(5))), Frame::new(5));
    }

    #[test]
    fn frame_debug_format() {
        let frame = Frame::new(42);
        let debug = format!("{:?}", frame);
        // Use multi-char string to avoid single_char_pattern lint
        assert!(debug.contains("42"));
    }
}

// ###################
// # KANI PROOFS     #
// ###################

/// Kani proofs for Frame arithmetic safety (SAFE-6 from formal-spec.md).
///
/// These proofs verify:
/// - Frame addition does not overflow in typical usage
/// - Frame subtraction produces correct results
/// - Frame comparisons are consistent
/// - NULL_FRAME (-1) is handled correctly
///
/// Note: Requires Kani verifier. Install with:
///   cargo install --locked kani-verifier
///   cargo kani setup
///
/// Run proofs with:
///   cargo kani --tests
#[cfg(kani)]
mod kani_proofs {
    use super::*;

    /// Proof: Frame::new creates valid frames for non-negative inputs
    #[kani::proof]
    fn proof_frame_new_valid() {
        let value: i32 = kani::any();
        kani::assume(value >= 0);

        let frame = Frame::new(value);
        kani::assert(
            frame.is_valid(),
            "Frame::new with non-negative should be valid",
        );
        kani::assert(
            !frame.is_null(),
            "Frame::new with non-negative should not be null",
        );
        kani::assert(
            frame.as_i32() == value,
            "Frame::as_i32 should return original value",
        );
    }

    /// Proof: Frame::NULL is consistently null
    #[kani::proof]
    fn proof_frame_null_consistency() {
        let null_frame = Frame::NULL;
        kani::assert(null_frame.is_null(), "NULL frame should be null");
        kani::assert(!null_frame.is_valid(), "NULL frame should not be valid");
        kani::assert(
            null_frame.as_i32() == NULL_FRAME,
            "NULL frame should equal NULL_FRAME constant",
        );
    }

    /// Proof: Frame addition with small positive values is safe
    ///
    /// This proves that for frames in typical game usage (0 to 10,000,000),
    /// adding small increments (0-1000) does not overflow.
    #[kani::proof]
    fn proof_frame_add_small_safe() {
        let frame_val: i32 = kani::any();
        let increment: i32 = kani::any();

        // Typical game usage: frames 0 to 10 million, increments 0 to 1000
        kani::assume(frame_val >= 0 && frame_val <= 10_000_000);
        kani::assume(increment >= 0 && increment <= 1000);

        let frame = Frame::new(frame_val);
        let result = frame + increment;

        kani::assert(
            result.as_i32() == frame_val + increment,
            "Frame addition should be correct",
        );
        kani::assert(
            result.is_valid(),
            "Result should be valid for typical usage",
        );
    }

    /// Proof: Frame subtraction produces correct differences
    #[kani::proof]
    fn proof_frame_sub_frames_correct() {
        let a: i32 = kani::any();
        let b: i32 = kani::any();

        kani::assume(a >= 0 && a <= 1_000_000);
        kani::assume(b >= 0 && b <= 1_000_000);

        let frame_a = Frame::new(a);
        let frame_b = Frame::new(b);

        let diff: i32 = frame_a - frame_b;
        kani::assert(
            diff == a - b,
            "Frame subtraction should produce correct difference",
        );
    }

    /// Proof: Frame ordering is consistent with i32 ordering
    #[kani::proof]
    fn proof_frame_ordering_consistent() {
        let a: i32 = kani::any();
        let b: i32 = kani::any();

        kani::assume(a >= -1 && a <= 1_000_000);
        kani::assume(b >= -1 && b <= 1_000_000);

        let frame_a = Frame::new(a);
        let frame_b = Frame::new(b);

        // Verify ordering consistency
        if a < b {
            kani::assert(frame_a < frame_b, "Frame < should be consistent");
        }
        if a > b {
            kani::assert(frame_a > frame_b, "Frame > should be consistent");
        }
        if a == b {
            kani::assert(frame_a == frame_b, "Frame == should be consistent");
        }
    }

    /// Proof: Frame modulo operation is correct for queue indexing
    ///
    /// This is critical for InputQueue circular buffer indexing (INV-5).
    #[kani::proof]
    fn proof_frame_modulo_for_queue() {
        let frame_val: i32 = kani::any();

        // Valid frames for queue indexing
        kani::assume(frame_val >= 0 && frame_val <= 10_000_000);

        let frame = Frame::new(frame_val);
        let queue_len: i32 = 128; // INPUT_QUEUE_LENGTH

        let index = frame % queue_len;

        kani::assert(index >= 0, "Queue index should be non-negative");
        kani::assert(index < queue_len, "Queue index should be within bounds");
        kani::assert(index == frame_val % queue_len, "Modulo should be correct");
    }

    /// Proof: Frame::to_option correctly handles null and valid frames
    #[kani::proof]
    fn proof_frame_to_option() {
        let frame_val: i32 = kani::any();
        kani::assume(frame_val >= -1 && frame_val <= 1_000_000);

        let frame = Frame::new(frame_val);
        let opt = frame.to_option();

        if frame.is_valid() {
            kani::assert(opt.is_some(), "Valid frame should produce Some");
            kani::assert(opt.unwrap() == frame, "Option should contain same frame");
        } else {
            kani::assert(opt.is_none(), "Invalid frame should produce None");
        }
    }

    /// Proof: Frame::from_option correctly handles Some and None
    #[kani::proof]
    fn proof_frame_from_option() {
        let frame_val: i32 = kani::any();
        kani::assume(frame_val >= 0 && frame_val <= 1_000_000);

        let frame = Frame::new(frame_val);

        // Test with Some
        let from_some = Frame::from_option(Some(frame));
        kani::assert(from_some == frame, "from_option(Some) should return frame");

        // Test with None
        let from_none = Frame::from_option(None);
        kani::assert(
            from_none == Frame::NULL,
            "from_option(None) should return NULL",
        );
    }

    /// Proof: Frame AddAssign is consistent with Add
    #[kani::proof]
    fn proof_frame_add_assign_consistent() {
        let frame_val: i32 = kani::any();
        let increment: i32 = kani::any();

        kani::assume(frame_val >= 0 && frame_val <= 1_000_000);
        kani::assume(increment >= 0 && increment <= 1000);

        let frame1 = Frame::new(frame_val);
        let mut frame2 = Frame::new(frame_val);

        let result1 = frame1 + increment;
        frame2 += increment;

        kani::assert(result1 == frame2, "AddAssign should be consistent with Add");
    }

    /// Proof: Frame SubAssign is consistent with Sub
    #[kani::proof]
    fn proof_frame_sub_assign_consistent() {
        let frame_val: i32 = kani::any();
        let decrement: i32 = kani::any();

        kani::assume(frame_val >= 100 && frame_val <= 1_000_000);
        kani::assume(decrement >= 0 && decrement <= 100);

        let frame1 = Frame::new(frame_val);
        let mut frame2 = Frame::new(frame_val);

        let result1 = frame1 - decrement;
        frame2 -= decrement;

        kani::assert(result1 == frame2, "SubAssign should be consistent with Sub");
    }

    /// Proof: PlayerHandle validity check is correct
    #[kani::proof]
    fn proof_player_handle_validity() {
        let handle_val: usize = kani::any();
        let num_players: usize = kani::any();

        kani::assume(handle_val < 100);
        kani::assume(num_players > 0 && num_players <= 16);

        let handle = PlayerHandle::new(handle_val);

        let is_valid_player = handle.is_valid_player_for(num_players);
        let is_spectator = handle.is_spectator_for(num_players);

        // A handle is either a valid player OR a spectator, never both
        kani::assert(
            is_valid_player != is_spectator || handle_val >= num_players,
            "Handle should be player XOR spectator",
        );

        if handle_val < num_players {
            kani::assert(
                is_valid_player,
                "Handle < num_players should be valid player",
            );
            kani::assert(
                !is_spectator,
                "Handle < num_players should not be spectator",
            );
        } else {
            kani::assert(
                !is_valid_player,
                "Handle >= num_players should not be valid player",
            );
            kani::assert(is_spectator, "Handle >= num_players should be spectator");
        }
    }
}
