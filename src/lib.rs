//! # Fortress Rollback (formerly GGRS)
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
use serde::{Serialize, de::DeserializeOwned};
pub use sessions::builder::SessionBuilder;
pub use sessions::p2p_session::P2PSession;
pub use sessions::p2p_spectator_session::SpectatorSession;
pub use sessions::sync_test_session::SyncTestSession;
pub use sync_layer::{GameStateAccessor, GameStateCell};

pub(crate) mod error;
pub(crate) mod frame_info;
pub(crate) mod input_queue;
pub(crate) mod sync_layer;
pub mod telemetry;
pub(crate) mod time_sync;
pub(crate) mod sessions {
    pub(crate) mod builder;
    pub(crate) mod p2p_session;
    pub(crate) mod p2p_spectator_session;
    pub(crate) mod sync_test_session;
}
pub(crate) mod network {
    pub mod chaos_socket;
    pub(crate) mod compression;
    pub(crate) mod messages;
    pub(crate) mod network_stats;
    pub(crate) mod protocol;
    pub(crate) mod udp_socket;
}

// #############
// # CONSTANTS #
// #############

/// Internally, -1 represents no frame / invalid frame.
pub const NULL_FRAME: i32 = -1;

/// A frame is a single step of game execution.
///
/// Frames are the fundamental unit of time in rollback networking. Each frame
/// represents one discrete step of game simulation. Frame numbers start at 0
/// and increment sequentially.
///
/// The special value [`NULL_FRAME`] (-1) represents "no frame" or "uninitialized".
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
    Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default,
    serde::Serialize, serde::Deserialize,
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
    pub const fn new(frame: i32) -> Self {
        Frame(frame)
    }

    /// Returns the underlying `i32` value.
    #[inline]
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
    pub const fn is_valid(self) -> bool {
        self.0 >= 0
    }

    /// Returns `Some(self)` if the frame is valid, or `None` if it's null or negative.
    ///
    /// This is useful for handling the null/valid frame pattern with Option.
    #[inline]
    pub const fn to_option(self) -> Option<Frame> {
        if self.is_valid() {
            Some(self)
        } else {
            None
        }
    }

    /// Creates a Frame from an Option, using NULL for None.
    #[inline]
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
    Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default,
    serde::Serialize, serde::Deserialize,
)]
pub struct PlayerHandle(usize);

impl PlayerHandle {
    /// Creates a new `PlayerHandle` from a `usize` value.
    ///
    /// Note: This does not validate the handle against a specific session.
    /// Use [`is_valid_player_for()`](Self::is_valid_player_for) or
    /// [`is_spectator_for()`](Self::is_spectator_for) to check validity.
    #[inline]
    pub const fn new(handle: usize) -> Self {
        PlayerHandle(handle)
    }

    /// Returns the underlying `usize` value.
    #[inline]
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
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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
}

/// Requests that you can receive from the session. Handling them is mandatory.
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
// # KANI PROOFS     #
// ###################

/// Kani proofs for Frame arithmetic safety (SAFE-6 from FORMAL_SPEC.md).
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
        kani::assert(frame.is_valid(), "Frame::new with non-negative should be valid");
        kani::assert(!frame.is_null(), "Frame::new with non-negative should not be null");
        kani::assert(frame.as_i32() == value, "Frame::as_i32 should return original value");
    }

    /// Proof: Frame::NULL is consistently null
    #[kani::proof]
    fn proof_frame_null_consistency() {
        let null_frame = Frame::NULL;
        kani::assert(null_frame.is_null(), "NULL frame should be null");
        kani::assert(!null_frame.is_valid(), "NULL frame should not be valid");
        kani::assert(null_frame.as_i32() == NULL_FRAME, "NULL frame should equal NULL_FRAME constant");
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
            "Frame addition should be correct"
        );
        kani::assert(result.is_valid(), "Result should be valid for typical usage");
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
        kani::assert(diff == a - b, "Frame subtraction should produce correct difference");
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
        kani::assert(from_none == Frame::NULL, "from_option(None) should return NULL");
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
            "Handle should be player XOR spectator"
        );

        if handle_val < num_players {
            kani::assert(is_valid_player, "Handle < num_players should be valid player");
            kani::assert(!is_spectator, "Handle < num_players should not be spectator");
        } else {
            kani::assert(!is_valid_player, "Handle >= num_players should not be valid player");
            kani::assert(is_spectator, "Handle >= num_players should be spectator");
        }
    }
}
