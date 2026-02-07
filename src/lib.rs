//! # Fortress Rollback (formerly GGRS)
//!
//! <p align="center">
//!   <img src="https://raw.githubusercontent.com/wallstop/fortress-rollback/main/docs/assets/logo-banner.svg" alt="Fortress Rollback" width="400">
//! </p>
//!
//! Fortress Rollback is a fortified, verified reimagination of the GGPO network SDK written in 100% safe Rust.
//! The callback-style API from the original library has been replaced with a simple request-driven control flow.
//! Instead of registering callback functions, Fortress Rollback (previously GGRS) returns a list of requests for the user to fulfill.

#![forbid(unsafe_code)] // let us try
#![deny(warnings)] // Treat all warnings as errors (matches CI behavior)
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_intra_doc_links)]
#![deny(rustdoc::invalid_codeblock_attributes)]
#![warn(rustdoc::invalid_html_tags)]
#![warn(rustdoc::bare_urls)]
//#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
use std::{fmt::Debug, hash::Hash};

pub use error::{
    DeltaDecodeReason, FortressError, IndexOutOfBounds, InternalErrorKind, InvalidFrameReason,
    InvalidRequestKind, RleDecodeReason, SerializationErrorKind, SocketErrorKind,
};

/// A specialized `Result` type for Fortress Rollback operations.
///
/// This type alias provides a convenient way to write function signatures
/// that return [`FortressError`] as the error type. It supports an optional
/// second type parameter to override the error type if needed.
///
/// # Naming
///
/// This type is named `FortressResult` rather than `Result` to avoid
/// shadowing `std::result::Result` when using glob imports like
/// `use fortress_rollback::*;` or `use fortress_rollback::prelude::*;`.
/// This prevents subtle semver hazards where downstream code might
/// unexpectedly use this alias instead of the standard library's `Result`.
///
/// # Examples
///
/// Using the default error type:
///
/// ```
/// use fortress_rollback::{FortressResult, FortressError};
///
/// fn process_frame() -> FortressResult<()> {
///     // Returns Result<(), FortressError>
///     Ok(())
/// }
/// ```
///
/// Overriding the error type:
///
/// ```
/// use fortress_rollback::FortressResult;
///
/// fn custom_operation() -> FortressResult<String, std::io::Error> {
///     // Returns Result<String, std::io::Error>
///     Ok("success".to_string())
/// }
/// ```
///
/// You can also alias it locally if you prefer a shorter name:
///
/// ```
/// use fortress_rollback::FortressResult as Result;
///
/// fn my_function() -> Result<()> {
///     Ok(())
/// }
/// ```
pub type FortressResult<T, E = FortressError> = std::result::Result<T, E>;

pub use network::chaos_socket::{ChaosConfig, ChaosConfigBuilder, ChaosSocket, ChaosStats};
pub use network::messages::Message;
pub use network::network_stats::NetworkStats;
pub use network::udp_socket::UdpNonBlockingSocket;
use serde::{de::DeserializeOwned, Serialize};
pub use sessions::builder::SessionBuilder;
pub use sessions::config::{
    InputQueueConfig, ProtocolConfig, SaveMode, SpectatorConfig, SyncConfig,
};
pub use sessions::event_drain::EventDrain;
pub use sessions::p2p_session::P2PSession;
pub use sessions::p2p_spectator_session::SpectatorSession;
pub use sessions::player_registry::PlayerRegistry;
pub use sessions::session_trait::Session;
pub use sessions::sync_health::SyncHealth;
pub use sessions::sync_test_session::SyncTestSession;
// Re-export smallvec for users who need to work with SmallVec-backed types directly
pub use smallvec::SmallVec;
pub use sync_layer::{GameStateAccessor, GameStateCell};
pub use time_sync::TimeSyncConfig;

// Re-export prediction strategies
pub use crate::input_queue::{BlankPrediction, PredictionStrategy, RepeatLastConfirmed};

// Re-export checksum utilities for easy access
pub use checksum::{compute_checksum, compute_checksum_fletcher16, fletcher16, hash_bytes_fnv1a};

/// Tokio async runtime integration for Fortress Rollback.
///
/// This module provides [`TokioUdpSocket`], an adapter that wraps a Tokio async UDP socket
/// and implements the [`NonBlockingSocket`] trait for use with Fortress Rollback sessions
/// in async Tokio applications.
///
/// # Feature Flag
///
/// This module requires the `tokio` feature flag:
///
/// ```toml
/// [dependencies]
/// fortress-rollback = { version = "0.6", features = ["tokio"] }
/// ```
///
/// # Example
///
/// ```ignore
/// use fortress_rollback::tokio_socket::TokioUdpSocket;
/// use fortress_rollback::{SessionBuilder, PlayerType, PlayerHandle};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Create and bind a Tokio UDP socket adapter
///     let socket = TokioUdpSocket::bind_to_port(7000).await?;
///
///     // Use with SessionBuilder
///     let session = SessionBuilder::<MyConfig>::new()
///         .with_num_players(2)?
///         .add_player(PlayerType::Local, PlayerHandle::new(0))?
///         .add_player(PlayerType::Remote(remote_addr), PlayerHandle::new(1))?
///         .start_p2p_session(socket)?;
///
///     // Game loop...
///     Ok(())
/// }
/// ```
///
/// [`TokioUdpSocket`]: crate::tokio_socket::TokioUdpSocket
/// [`NonBlockingSocket`]: crate::NonBlockingSocket
#[cfg(feature = "tokio")]
pub mod tokio_socket {
    pub use crate::network::tokio_socket::TokioUdpSocket;
}

/// State checksum utilities for rollback networking.
///
/// Provides deterministic checksum computation for game states, essential for
/// desync detection in peer-to-peer rollback networking.
///
/// # Quick Start
///
/// ```
/// use fortress_rollback::checksum::{compute_checksum, ChecksumError};
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct GameState { frame: u32, x: f32, y: f32 }
///
/// let state = GameState { frame: 100, x: 1.0, y: 2.0 };
/// let checksum = compute_checksum(&state)?;
/// # Ok::<(), ChecksumError>(())
/// ```
///
/// See module documentation for detailed usage and performance considerations.
pub mod checksum;

/// Convenient re-exports for common usage.
///
/// This module provides a "prelude" that re-exports the most commonly used types
/// from Fortress Rollback, allowing you to import them all at once with
/// `use fortress_rollback::prelude::*;`
///
/// See the [`prelude`] module documentation for the full list of included types.
pub mod prelude;

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
/// Shared test configuration for property-based testing.
///
/// This module provides centralized configuration for proptest, including
/// Miri-aware case count reduction for faster testing under the interpreter.
#[cfg(test)]
pub(crate) mod test_config;
#[doc(hidden)]
pub mod time_sync;
#[doc(hidden)]
pub mod sessions {
    #[doc(hidden)]
    pub mod builder;
    /// Configuration types for session behavior.
    #[doc(hidden)]
    pub mod config;
    #[doc(hidden)]
    pub mod event_drain;
    #[doc(hidden)]
    pub mod p2p_session;
    #[doc(hidden)]
    pub mod p2p_spectator_session;
    #[doc(hidden)]
    pub mod player_registry;
    #[doc(hidden)]
    pub mod session_trait;
    #[doc(hidden)]
    pub mod sync_health;
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
    #[cfg(feature = "tokio")]
    pub mod tokio_socket;
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
    pub use crate::sessions::player_registry::PlayerRegistry;
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
    pub const NULL: Self = Self(NULL_FRAME);

    /// Creates a new `Frame` from an `i32` value.
    ///
    /// Note: This does not validate the frame number. Use [`Frame::is_valid()`]
    /// to check if the frame represents a valid (non-negative) frame number.
    #[inline]
    #[must_use]
    pub const fn new(frame: i32) -> Self {
        Self(frame)
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
    pub const fn to_option(self) -> Option<Self> {
        if self.is_valid() {
            Some(self)
        } else {
            None
        }
    }

    /// Creates a Frame from an Option, using NULL for None.
    #[inline]
    #[must_use]
    pub const fn from_option(opt: Option<Self>) -> Self {
        match opt {
            Some(f) => f,
            None => Self::NULL,
        }
    }

    // === Checked Arithmetic Methods ===
    //
    // Design Philosophy: Graceful error handling over panics.
    //
    // These methods are the PREFERRED way to perform Frame arithmetic in production code.
    // They allow the library to handle edge cases gracefully rather than panicking.
    //
    // Guidelines:
    // - Use `checked_*` when you need to detect and handle overflow explicitly
    // - Use `saturating_*` when clamping to bounds is acceptable behavior
    // - Use `abs_diff` when calculating frame distances (order-independent)
    // - Avoid raw `+` and `-` operators except in tests or where overflow is impossible
    //
    // Note: While `overflow-checks = true` in release catches overflow as panics,
    // the goal is zero panics in production - use these methods proactively.

    /// Adds a value to this frame, returning `None` if overflow occurs.
    ///
    /// This is the preferred method for frame arithmetic when overflow must be handled.
    ///
    /// # Examples
    ///
    /// ```
    /// use fortress_rollback::Frame;
    ///
    /// let frame = Frame::new(100);
    /// assert_eq!(frame.checked_add(50), Some(Frame::new(150)));
    /// assert_eq!(Frame::new(i32::MAX).checked_add(1), None);
    /// ```
    #[inline]
    #[must_use]
    pub const fn checked_add(self, rhs: i32) -> Option<Self> {
        match self.0.checked_add(rhs) {
            Some(result) => Some(Self(result)),
            None => None,
        }
    }

    /// Subtracts a value from this frame, returning `None` if overflow occurs.
    ///
    /// This is the preferred method for frame arithmetic when overflow must be handled.
    ///
    /// # Examples
    ///
    /// ```
    /// use fortress_rollback::Frame;
    ///
    /// let frame = Frame::new(100);
    /// assert_eq!(frame.checked_sub(50), Some(Frame::new(50)));
    /// assert_eq!(Frame::new(i32::MIN).checked_sub(1), None);
    /// ```
    #[inline]
    #[must_use]
    pub const fn checked_sub(self, rhs: i32) -> Option<Self> {
        match self.0.checked_sub(rhs) {
            Some(result) => Some(Self(result)),
            None => None,
        }
    }

    /// Adds a value to this frame, saturating at the numeric bounds.
    ///
    /// Use this when clamping to bounds is acceptable (e.g., frame counters that
    /// should never go negative or exceed maximum).
    ///
    /// # Examples
    ///
    /// ```
    /// use fortress_rollback::Frame;
    ///
    /// let frame = Frame::new(100);
    /// assert_eq!(frame.saturating_add(50), Frame::new(150));
    /// assert_eq!(Frame::new(i32::MAX).saturating_add(1), Frame::new(i32::MAX));
    /// ```
    #[inline]
    #[must_use]
    pub const fn saturating_add(self, rhs: i32) -> Self {
        Self(self.0.saturating_add(rhs))
    }

    /// Subtracts a value from this frame, saturating at the numeric bounds.
    ///
    /// Use this when clamping to bounds is acceptable (e.g., ensuring frame
    /// never goes below zero or `i32::MIN`).
    ///
    /// # Examples
    ///
    /// ```
    /// use fortress_rollback::Frame;
    ///
    /// let frame = Frame::new(100);
    /// assert_eq!(frame.saturating_sub(50), Frame::new(50));
    /// assert_eq!(Frame::new(i32::MIN).saturating_sub(1), Frame::new(i32::MIN));
    /// ```
    #[inline]
    #[must_use]
    pub const fn saturating_sub(self, rhs: i32) -> Self {
        Self(self.0.saturating_sub(rhs))
    }

    /// Returns the absolute difference between two frames.
    ///
    /// This is useful for calculating frame distances without worrying about
    /// the order of operands.
    ///
    /// # Examples
    ///
    /// ```
    /// use fortress_rollback::Frame;
    ///
    /// let a = Frame::new(100);
    /// let b = Frame::new(150);
    /// assert_eq!(a.abs_diff(b), 50);
    /// assert_eq!(b.abs_diff(a), 50);
    /// ```
    #[inline]
    #[must_use]
    pub const fn abs_diff(self, other: Self) -> u32 {
        self.0.abs_diff(other.0)
    }

    // === Ergonomic Conversion Methods ===

    /// Returns the frame as a `usize`, or `None` if the frame is negative.
    ///
    /// This is useful for indexing into arrays or vectors where a valid
    /// (non-negative) frame is required.
    ///
    /// # Examples
    ///
    /// ```
    /// use fortress_rollback::Frame;
    ///
    /// assert_eq!(Frame::new(42).as_usize(), Some(42));
    /// assert_eq!(Frame::new(0).as_usize(), Some(0));
    /// assert_eq!(Frame::NULL.as_usize(), None);
    /// assert_eq!(Frame::new(-5).as_usize(), None);
    /// ```
    #[inline]
    #[must_use]
    pub const fn as_usize(self) -> Option<usize> {
        if self.0 >= 0 {
            Some(self.0 as usize)
        } else {
            None
        }
    }

    /// Returns the frame as a `usize`, or a `FortressError` if negative.
    ///
    /// This is the Result-returning version of [`as_usize`](Self::as_usize),
    /// useful when you want to use the `?` operator for error propagation.
    ///
    /// # Errors
    ///
    /// Returns [`FortressError::InvalidFrameStructured`] with reason
    /// [`InvalidFrameReason::MustBeNonNegative`] if the frame is negative.
    ///
    /// # Examples
    ///
    /// ```
    /// use fortress_rollback::{Frame, FortressError, InvalidFrameReason};
    ///
    /// // Successful conversion
    /// let value = Frame::new(42).try_as_usize()?;
    /// assert_eq!(value, 42);
    ///
    /// // Error case - negative frame
    /// let result = Frame::NULL.try_as_usize();
    /// assert!(matches!(
    ///     result,
    ///     Err(FortressError::InvalidFrameStructured {
    ///         frame,
    ///         reason: InvalidFrameReason::MustBeNonNegative,
    ///     }) if frame == Frame::NULL
    /// ));
    /// # Ok::<(), FortressError>(())
    /// ```
    ///
    /// [`InvalidFrameReason::MustBeNonNegative`]: crate::InvalidFrameReason::MustBeNonNegative
    #[inline]
    #[track_caller]
    pub fn try_as_usize(self) -> Result<usize, FortressError> {
        if self.0 >= 0 {
            Ok(self.0 as usize)
        } else {
            Err(FortressError::InvalidFrameStructured {
                frame: self,
                reason: InvalidFrameReason::MustBeNonNegative,
            })
        }
    }

    /// Calculates the buffer index for this frame using modular arithmetic.
    ///
    /// This is a common pattern for ring buffer indexing where you need to map
    /// a frame number to a buffer slot. Returns `None` if the frame is negative
    /// or if `buffer_size` is zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use fortress_rollback::Frame;
    ///
    /// // Frame 7 in a buffer of size 4 -> index 3
    /// assert_eq!(Frame::new(7).buffer_index(4), Some(3));
    ///
    /// // Frame 0 in a buffer of size 4 -> index 0
    /// assert_eq!(Frame::new(0).buffer_index(4), Some(0));
    ///
    /// // Negative frame returns None
    /// assert_eq!(Frame::NULL.buffer_index(4), None);
    ///
    /// // Zero buffer size returns None
    /// assert_eq!(Frame::new(5).buffer_index(0), None);
    /// ```
    #[inline]
    #[must_use]
    pub const fn buffer_index(self, buffer_size: usize) -> Option<usize> {
        if self.0 >= 0 && buffer_size > 0 {
            Some(self.0 as usize % buffer_size)
        } else {
            None
        }
    }

    /// Calculates the buffer index for this frame, returning an error for invalid frames.
    ///
    /// This is the Result-returning version of [`buffer_index()`][Self::buffer_index].
    ///
    /// # Errors
    ///
    /// Returns [`FortressError::InvalidFrameStructured`] if the frame is negative.
    /// Returns [`FortressError::InvalidRequestStructured`] with [`InvalidRequestKind::ZeroBufferSize`]
    /// if `buffer_size` is zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use fortress_rollback::{Frame, FortressError, InvalidRequestKind};
    ///
    /// // Valid frame and buffer size
    /// let index = Frame::new(7).try_buffer_index(4)?;
    /// assert_eq!(index, 3);
    ///
    /// // Negative frame returns error
    /// assert!(Frame::NULL.try_buffer_index(4).is_err());
    ///
    /// // Zero buffer size returns error
    /// let result = Frame::new(5).try_buffer_index(0);
    /// assert!(matches!(
    ///     result,
    ///     Err(FortressError::InvalidRequestStructured {
    ///         kind: InvalidRequestKind::ZeroBufferSize
    ///     })
    /// ));
    /// # Ok::<(), FortressError>(())
    /// ```
    ///
    /// [`InvalidRequestKind::ZeroBufferSize`]: crate::InvalidRequestKind::ZeroBufferSize
    #[inline]
    #[track_caller]
    pub fn try_buffer_index(self, buffer_size: usize) -> Result<usize, FortressError> {
        if buffer_size == 0 {
            return Err(FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::ZeroBufferSize,
            });
        }
        self.try_as_usize().map(|u| u % buffer_size)
    }

    // === Result-Returning Arithmetic ===

    /// Adds a value to this frame, returning an error if overflow occurs.
    ///
    /// This is the Result-returning version of [`checked_add`](Self::checked_add),
    /// useful when you want to use the `?` operator for error propagation.
    ///
    /// # Errors
    ///
    /// Returns [`FortressError::FrameArithmeticOverflow`] if the addition would overflow.
    ///
    /// # Examples
    ///
    /// ```
    /// use fortress_rollback::{Frame, FortressError};
    ///
    /// let frame = Frame::new(100);
    /// let result = frame.try_add(50)?;
    /// assert_eq!(result, Frame::new(150));
    ///
    /// // Overflow returns error
    /// let overflow_result = Frame::new(i32::MAX).try_add(1);
    /// assert!(matches!(overflow_result, Err(FortressError::FrameArithmeticOverflow { .. })));
    /// # Ok::<(), FortressError>(())
    /// ```
    #[inline]
    #[track_caller]
    pub fn try_add(self, rhs: i32) -> Result<Self, FortressError> {
        self.checked_add(rhs)
            .ok_or(FortressError::FrameArithmeticOverflow {
                frame: self,
                operand: rhs,
                operation: "add",
            })
    }

    /// Subtracts a value from this frame, returning an error if overflow occurs.
    ///
    /// This is the Result-returning version of [`checked_sub`](Self::checked_sub),
    /// useful when you want to use the `?` operator for error propagation.
    ///
    /// # Errors
    ///
    /// Returns [`FortressError::FrameArithmeticOverflow`] if the subtraction would overflow.
    ///
    /// # Examples
    ///
    /// ```
    /// use fortress_rollback::{Frame, FortressError};
    ///
    /// let frame = Frame::new(100);
    /// let result = frame.try_sub(50)?;
    /// assert_eq!(result, Frame::new(50));
    ///
    /// // Overflow returns error
    /// let overflow_result = Frame::new(i32::MIN).try_sub(1);
    /// assert!(matches!(overflow_result, Err(FortressError::FrameArithmeticOverflow { .. })));
    /// # Ok::<(), FortressError>(())
    /// ```
    #[inline]
    #[track_caller]
    pub fn try_sub(self, rhs: i32) -> Result<Self, FortressError> {
        self.checked_sub(rhs)
            .ok_or(FortressError::FrameArithmeticOverflow {
                frame: self,
                operand: rhs,
                operation: "sub",
            })
    }

    // === Convenience Increment/Decrement Methods ===

    /// Returns the next frame, or an error if overflow would occur.
    ///
    /// This is equivalent to `try_add(1)`.
    ///
    /// # Errors
    ///
    /// Returns [`FortressError::FrameArithmeticOverflow`] if the frame is `i32::MAX`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fortress_rollback::{Frame, FortressError};
    ///
    /// let next_frame = Frame::new(5).next()?;
    /// assert_eq!(next_frame, Frame::new(6));
    ///
    /// // MAX returns error
    /// assert!(Frame::new(i32::MAX).next().is_err());
    /// # Ok::<(), FortressError>(())
    /// ```
    #[inline]
    #[track_caller]
    pub fn next(self) -> Result<Self, FortressError> {
        self.try_add(1)
    }

    /// Returns the previous frame, or an error if overflow would occur.
    ///
    /// This is equivalent to `try_sub(1)`.
    ///
    /// # Errors
    ///
    /// Returns [`FortressError::FrameArithmeticOverflow`] if the frame is `i32::MIN`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fortress_rollback::{Frame, FortressError};
    ///
    /// let prev_frame = Frame::new(5).prev()?;
    /// assert_eq!(prev_frame, Frame::new(4));
    ///
    /// // MIN returns error
    /// assert!(Frame::new(i32::MIN).prev().is_err());
    /// # Ok::<(), FortressError>(())
    /// ```
    #[inline]
    #[track_caller]
    pub fn prev(self) -> Result<Self, FortressError> {
        self.try_sub(1)
    }

    /// Returns the next frame, saturating at `i32::MAX`.
    ///
    /// This is equivalent to `saturating_add(1)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fortress_rollback::Frame;
    ///
    /// assert_eq!(Frame::new(5).saturating_next(), Frame::new(6));
    /// assert_eq!(Frame::new(i32::MAX).saturating_next(), Frame::new(i32::MAX));
    /// ```
    #[inline]
    #[must_use]
    pub const fn saturating_next(self) -> Self {
        self.saturating_add(1)
    }

    /// Returns the previous frame, saturating at `i32::MIN`.
    ///
    /// This is equivalent to `saturating_sub(1)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fortress_rollback::Frame;
    ///
    /// assert_eq!(Frame::new(5).saturating_prev(), Frame::new(4));
    /// assert_eq!(Frame::new(i32::MIN).saturating_prev(), Frame::new(i32::MIN));
    /// ```
    #[inline]
    #[must_use]
    pub const fn saturating_prev(self) -> Self {
        self.saturating_sub(1)
    }

    // === Safe usize Construction ===

    /// Creates a `Frame` from a `usize`, returning `None` if it exceeds `i32::MAX`.
    ///
    /// This is useful for converting array indices or sizes to frames safely.
    ///
    /// # Examples
    ///
    /// ```
    /// use fortress_rollback::Frame;
    ///
    /// assert_eq!(Frame::from_usize(42), Some(Frame::new(42)));
    /// assert_eq!(Frame::from_usize(0), Some(Frame::new(0)));
    ///
    /// // Values exceeding i32::MAX return None
    /// let too_large = (i32::MAX as usize) + 1;
    /// assert_eq!(Frame::from_usize(too_large), None);
    /// ```
    #[inline]
    #[must_use]
    pub const fn from_usize(value: usize) -> Option<Self> {
        if value <= i32::MAX as usize {
            Some(Self(value as i32))
        } else {
            None
        }
    }

    /// Creates a `Frame` from a `usize`, returning an error if it exceeds `i32::MAX`.
    ///
    /// This is the Result-returning version of [`from_usize`](Self::from_usize),
    /// useful when you want to use the `?` operator for error propagation.
    ///
    /// # Errors
    ///
    /// Returns [`FortressError::FrameValueTooLarge`] if the value exceeds `i32::MAX`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fortress_rollback::{Frame, FortressError};
    ///
    /// let frame = Frame::try_from_usize(42)?;
    /// assert_eq!(frame, Frame::new(42));
    ///
    /// // Values exceeding i32::MAX return error
    /// let too_large = (i32::MAX as usize) + 1;
    /// let result = Frame::try_from_usize(too_large);
    /// assert!(matches!(result, Err(FortressError::FrameValueTooLarge { .. })));
    /// # Ok::<(), FortressError>(())
    /// ```
    #[inline]
    #[track_caller]
    pub fn try_from_usize(value: usize) -> Result<Self, FortressError> {
        Self::from_usize(value).ok_or(FortressError::FrameValueTooLarge { value })
    }

    // === Distance and Range Methods ===

    /// Returns the signed distance from `self` to `other` (`other - self`).
    ///
    /// Returns `None` if the subtraction would overflow.
    ///
    /// # Examples
    ///
    /// ```
    /// use fortress_rollback::Frame;
    ///
    /// let a = Frame::new(100);
    /// let b = Frame::new(150);
    ///
    /// assert_eq!(a.distance_to(b), Some(50));
    /// assert_eq!(b.distance_to(a), Some(-50));
    /// assert_eq!(a.distance_to(a), Some(0));
    ///
    /// // Overflow returns None
    /// assert_eq!(Frame::new(i32::MIN).distance_to(Frame::new(i32::MAX)), None);
    /// ```
    #[inline]
    #[must_use]
    pub const fn distance_to(self, other: Self) -> Option<i32> {
        other.0.checked_sub(self.0)
    }

    /// Returns `true` if `self` is within `window` frames of `reference`.
    ///
    /// This checks if the absolute difference between `self` and `reference`
    /// is less than or equal to `window`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fortress_rollback::Frame;
    ///
    /// let reference = Frame::new(100);
    ///
    /// // Within window
    /// assert!(Frame::new(98).is_within(5, reference));  // diff = 2
    /// assert!(Frame::new(105).is_within(5, reference)); // diff = 5
    ///
    /// // At boundary
    /// assert!(Frame::new(95).is_within(5, reference));  // diff = 5 (exact)
    ///
    /// // Outside window
    /// assert!(!Frame::new(94).is_within(5, reference)); // diff = 6
    /// assert!(!Frame::new(106).is_within(5, reference)); // diff = 6
    /// ```
    #[inline]
    #[must_use]
    pub const fn is_within(self, window: u32, reference: Self) -> bool {
        self.abs_diff(reference) <= window
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
    type Output = Self;

    #[inline]
    fn add(self, rhs: i32) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl std::ops::Add<Self> for Frame {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::AddAssign<i32> for Frame {
    #[inline]
    fn add_assign(&mut self, rhs: i32) {
        self.0 += rhs;
    }
}

impl std::ops::Sub<i32> for Frame {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: i32) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl std::ops::Sub<Self> for Frame {
    type Output = i32;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
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
        Self(value)
    }
}

impl From<Frame> for i32 {
    #[inline]
    fn from(frame: Frame) -> Self {
        frame.0
    }
}

/// Converts a `usize` to a `Frame`.
///
/// # ⚠️ Discouraged
///
/// **Soft-deprecated**: This conversion silently truncates values larger
/// than `i32::MAX`. For safe conversion with overflow detection, use
/// [`Frame::from_usize()`] or [`Frame::try_from_usize()`] instead.
///
/// This impl cannot use `#[deprecated]` because Rust doesn't support that attribute
/// on trait impl blocks — no compiler warning will be emitted. Consider using the
/// safer alternatives listed above.
impl From<usize> for Frame {
    #[inline]
    fn from(value: usize) -> Self {
        Self(value as i32)
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
        Self(handle)
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
        write!(f, "PlayerHandle({})", self.0)
    }
}

// Conversion traits for backwards compatibility

impl From<usize> for PlayerHandle {
    #[inline]
    fn from(value: usize) -> Self {
        Self(value)
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
///
/// Defaults to [`DesyncDetection::On`] with an interval of 60 (once per second at 60hz).
/// This provides reasonable detection frequency while being bandwidth-friendly.
/// For faster detection, you can decrease the interval; for bandwidth-constrained
/// scenarios, you can increase the interval or disable detection entirely.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DesyncDetection {
    /// Desync detection is turned on with a specified interval rate given by the user.
    ///
    /// The interval controls how often checksums are compared. An interval of 1 means
    /// every frame, 10 means every 10th frame (6 times per second at 60hz), etc.
    On {
        /// Interval rate for checksum comparison. At 60hz, an interval of 1 means
        /// checksums are compared every frame, 10 means 6 times per second, etc.
        interval: u32,
    },
    /// Desync detection is turned off.
    ///
    /// **Warning:** Disabling desync detection means state divergence between peers
    /// will go undetected, potentially causing confusing gameplay bugs.
    Off,
}

impl Default for DesyncDetection {
    /// Returns [`DesyncDetection::On`] with `interval: 60` (once per second at 60hz).
    fn default() -> Self {
        Self::On { interval: 60 }
    }
}

impl std::fmt::Display for DesyncDetection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::On { interval } => write!(f, "On(interval={})", interval),
            Self::Off => write!(f, "Off"),
        }
    }
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

impl<A> std::fmt::Display for PlayerType<A>
where
    A: Clone + PartialEq + Eq + PartialOrd + Ord + std::hash::Hash + std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local => write!(f, "Local"),
            Self::Remote(addr) => write!(f, "Remote({})", addr),
            Self::Spectator(addr) => write!(f, "Spectator({})", addr),
        }
    }
}

/// A session is always in one of these states. You can query the current state of a session via [`current_state`].
///
/// [`current_state`]: P2PSession#method.current_state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SessionState {
    /// When synchronizing, the session attempts to establish a connection to the remote clients.
    Synchronizing,
    /// When running, the session has synchronized and is ready to take and transmit player input.
    Running,
}

impl std::fmt::Display for SessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Synchronizing => write!(f, "Synchronizing"),
            Self::Running => write!(f, "Running"),
        }
    }
}

/// [`InputStatus`] will always be given together with player inputs when requested to advance the frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputStatus {
    /// The input of this player for this frame is an actual received input.
    Confirmed,
    /// The input of this player for this frame is predicted.
    Predicted,
    /// The player has disconnected at or prior to this frame, so this input is a dummy.
    Disconnected,
}

impl std::fmt::Display for InputStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Confirmed => write!(f, "Confirmed"),
            Self::Predicted => write!(f, "Predicted"),
            Self::Disconnected => write!(f, "Disconnected"),
        }
    }
}

/// Stack-allocated vector type for player inputs.
///
/// This type uses [`SmallVec`] to avoid heap allocations for the common case of
/// 2-4 players. Games with more than 4 players will spill to the heap automatically.
///
/// # Performance
///
/// For games with 1-4 players, input vectors are stack-allocated, avoiding the
/// overhead of heap allocation and deallocation on every frame. This provides
/// measurable performance improvements in the hot path of `advance_frame()`.
///
/// # Usage
///
/// `InputVec` is used in [`FortressRequest::AdvanceFrame`] and can be iterated
/// like a regular slice:
///
/// ```ignore
/// let FortressRequest::AdvanceFrame { inputs } = request else { return };
/// for (input, status) in inputs.iter() {
///     // Process each player's input
/// }
/// ```
///
/// # Migration from `Vec`
///
/// `InputVec` implements `Deref<Target = [(T::Input, InputStatus)]>`, so most code
/// using `.iter()`, `.len()`, indexing, or other slice methods will work unchanged.
/// If you need a `Vec`, use `.to_vec()`.
pub type InputVec<I> = SmallVec<[(I, InputStatus); 4]>;

/// Stack-allocated vector for player handles.
///
/// This type uses [`SmallVec`] to avoid heap allocations for the common case of
/// up to 8 players. Games with more than 8 players will "spill" to the heap automatically,
/// meaning the data moves from the stack to a heap allocation. This spilling is transparent
/// and the API remains the same — it just incurs the performance cost of heap allocation.
///
/// # Performance
///
/// For games with 1-8 players, handle vectors are stack-allocated, avoiding the
/// overhead of heap allocation and deallocation. This provides measurable performance
/// improvements when querying player handles frequently.
///
/// # Usage
///
/// `HandleVec` is returned by methods like [`P2PSession::local_player_handles()`]:
///
/// ```ignore
/// let handles = session.local_player_handles();
/// for handle in handles {
///     // Process each player handle
/// }
/// ```
///
/// # Migration from `Vec`
///
/// `HandleVec` implements `Deref<Target = [PlayerHandle]>`, so most code
/// using `.iter()`, `.len()`, indexing, or other slice methods will work unchanged.
/// If you need a `Vec`, use `.to_vec()`.
///
/// [`P2PSession::local_player_handles()`]: crate::P2PSession::local_player_handles
pub type HandleVec = SmallVec<[PlayerHandle; 8]>;

/// Stack-allocated vector for frame advance [`FortressRequest`]s.
///
/// Uses [`SmallVec`] with inline capacity of 4 to avoid heap allocations for
/// the common case (1 save + 1 advance = 2 requests per frame). During
/// rollbacks, the count can grow to `2 * max_prediction + 2` (typically ~18),
/// at which point it spills to the heap automatically.
///
/// # Performance
///
/// For the overwhelmingly common non-rollback path, request vectors are
/// fully stack-allocated. Rollback is already an expensive operation
/// (loading state + resimulating N frames), so the marginal cost of a
/// heap allocation during rollback is negligible.
///
/// # Usage
///
/// `RequestVec` is returned by session `advance_frame()` methods and can be
/// iterated like a regular slice:
///
/// ```ignore
/// let requests: RequestVec<MyConfig> = session.advance_frame()?;
/// for request in requests {
///     // Handle each request (save, load, advance)
/// }
/// ```
///
/// # Migration from `Vec`
///
/// `RequestVec` implements `Deref<Target = [FortressRequest<T>]>`, so most
/// code using `.iter()`, `.len()`, indexing, or other slice methods will work
/// unchanged. If you need a `Vec`, use `.to_vec()`. The [`handle_requests!`]
/// macro works unchanged because it uses `for request in $requests`, and
/// `SmallVec` implements `IntoIterator`.
///
/// [`handle_requests!`]: crate::handle_requests
pub type RequestVec<T> = SmallVec<[FortressRequest<T>; 4]>;

/// Notifications that you can receive from the session. Handling them is up to the user.
///
/// # Handling Events
///
/// Events inform you about session state changes. Match on all variants to handle each case:
///
/// ```ignore
/// match event {
///     FortressEvent::Synchronized { addr } => { /* handle */ }
///     FortressEvent::Disconnected { addr } => { /* handle */ }
///     // ... handle all other variants
/// }
/// ```
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

impl<T: Config> std::fmt::Display for FortressEvent<T>
where
    T::Address: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Synchronizing {
                addr,
                total,
                count,
                total_requests_sent,
                elapsed_ms,
            } => write!(
                f,
                "Synchronizing({}/{}, addr={}, requests_sent={}, elapsed={}ms)",
                count, total, addr, total_requests_sent, elapsed_ms
            ),
            Self::Synchronized { addr } => write!(f, "Synchronized(addr={})", addr),
            Self::Disconnected { addr } => write!(f, "Disconnected(addr={})", addr),
            Self::NetworkInterrupted {
                addr,
                disconnect_timeout,
            } => write!(
                f,
                "NetworkInterrupted(addr={}, timeout={}ms)",
                addr, disconnect_timeout
            ),
            Self::NetworkResumed { addr } => write!(f, "NetworkResumed(addr={})", addr),
            Self::WaitRecommendation { skip_frames } => {
                write!(f, "WaitRecommendation(skip_frames={})", skip_frames)
            },
            Self::DesyncDetected {
                frame,
                local_checksum,
                remote_checksum,
                addr,
            } => write!(
                f,
                "DesyncDetected(frame={}, local={:#x}, remote={:#x}, addr={})",
                frame.as_i32(),
                local_checksum,
                remote_checksum,
                addr
            ),
            Self::SyncTimeout { addr, elapsed_ms } => {
                write!(f, "SyncTimeout(addr={}, elapsed={}ms)", addr, elapsed_ms)
            },
        }
    }
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
///     }
/// }
/// ```
#[derive(Debug, Clone)]
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
        ///
        /// This uses [`InputVec`] (a [`SmallVec`]) instead of [`Vec`] for better performance.
        /// For 1-4 players, inputs are stack-allocated (no heap allocation).
        /// The collection implements `Deref<Target = [T]>`, so `.iter()` and indexing work normally.
        inputs: InputVec<T::Input>,
    },
}

impl<T: Config> std::fmt::Display for FortressRequest<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SaveGameState { frame, .. } => {
                write!(f, "SaveGameState(frame={})", frame.as_i32())
            },
            Self::LoadGameState { frame, .. } => {
                write!(f, "LoadGameState(frame={})", frame.as_i32())
            },
            Self::AdvanceFrame { inputs } => {
                write!(f, "AdvanceFrame(inputs={})", inputs.len())
            },
        }
    }
}

/// Macro to simplify handling [`FortressRequest`] variants in a game loop.
///
/// This macro eliminates the boilerplate of matching on request variants, providing
/// a concise way to handle save, load, and advance operations.
///
/// # Usage
///
/// ```
/// # use fortress_rollback::{Config, Frame, FortressRequest, GameStateCell, InputVec, RequestVec, handle_requests};
/// # use serde::{Deserialize, Serialize};
/// # use std::net::SocketAddr;
/// #
/// # #[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
/// # struct MyInput(u8);
/// #
/// # #[derive(Clone, Default)]
/// # struct MyState { frame: i32, data: u64 }
/// #
/// # struct MyConfig;
/// # impl Config for MyConfig {
/// #     type Input = MyInput;
/// #     type State = MyState;
/// #     type Address = SocketAddr;
/// # }
/// #
/// # fn compute_checksum(_: &MyState) -> u128 { 0 }
/// #
/// # fn example(mut state: MyState, requests: RequestVec<MyConfig>) {
/// handle_requests!(
///     requests,
///     save: |cell: GameStateCell<MyState>, frame: Frame| {
///         let checksum = compute_checksum(&state);
///         cell.save(frame, Some(state.clone()), Some(checksum));
///     },
///     load: |cell: GameStateCell<MyState>, _frame: Frame| {
///         // LoadGameState is only requested for previously saved frames.
///         // Handle missing state appropriately for your application.
///         if let Some(loaded) = cell.load() {
///             state = loaded;
///         }
///     },
///     advance: |inputs: InputVec<MyInput>| {
///         state.frame += 1;
///         // Apply inputs...
///     }
/// );
/// # }
/// ```
///
/// # Parameters
///
/// - `requests`: An iterable of [`FortressRequest<T>`] (usually [`RequestVec<T>`])
/// - `save`: Closure taking `(cell: GameStateCell<State>, frame: Frame)` — called for [`FortressRequest::SaveGameState`]
/// - `load`: Closure taking `(cell: GameStateCell<State>, frame: Frame)` — called for [`FortressRequest::LoadGameState`]
/// - `advance`: Closure taking `(inputs: InputVec<Input>)` — called for [`FortressRequest::AdvanceFrame`]
///
/// # Order Preservation
///
/// Requests are processed in iteration order, which matches the order returned by
/// [`P2PSession::advance_frame`]. This order is critical for correctness — do not
/// sort, filter, or reorder the requests.
///
/// # Lockstep Mode
///
/// In lockstep mode (prediction window = 0), you will never receive `SaveGameState`
/// or `LoadGameState` requests. You can provide empty closures:
///
/// ```
/// # use fortress_rollback::{Config, Frame, FortressRequest, GameStateCell, InputVec, RequestVec, handle_requests};
/// # use serde::{Deserialize, Serialize};
/// # use std::net::SocketAddr;
/// #
/// # #[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
/// # struct MyInput(u8);
/// # #[derive(Clone, Default)]
/// # struct MyState { frame: i32 }
/// # struct MyConfig;
/// # impl Config for MyConfig {
/// #     type Input = MyInput;
/// #     type State = MyState;
/// #     type Address = SocketAddr;
/// # }
/// #
/// # fn example(mut state: MyState, requests: RequestVec<MyConfig>) {
/// handle_requests!(
///     requests,
///     save: |_, _| { /* Never called in lockstep */ },
///     load: |_, _| { /* Never called in lockstep */ },
///     advance: |inputs: InputVec<MyInput>| {
///         state.frame += 1;
///     }
/// );
/// # }
/// ```
///
/// # Exhaustive Matching
///
/// `FortressRequest` is exhaustively matchable (not `#[non_exhaustive]`), so this
/// macro handles all variants. If a new variant is added in a future version,
/// the compiler will notify you at compile time.
///
/// [`P2PSession::advance_frame`]: crate::P2PSession::advance_frame
/// [`RequestVec<T>`]: crate::RequestVec
/// [`FortressRequest::SaveGameState`]: crate::FortressRequest::SaveGameState
/// [`FortressRequest::LoadGameState`]: crate::FortressRequest::LoadGameState
/// [`FortressRequest::AdvanceFrame`]: crate::FortressRequest::AdvanceFrame
#[macro_export]
macro_rules! handle_requests {
    (
        $requests:expr,
        save: $save:expr,
        load: $load:expr,
        advance: $advance:expr
        $(,)?
    ) => {{
        for request in $requests {
            match request {
                $crate::FortressRequest::SaveGameState { cell, frame } => {
                    #[allow(clippy::redundant_closure_call)]
                    ($save)(cell, frame);
                },
                $crate::FortressRequest::LoadGameState { cell, frame } => {
                    #[allow(clippy::redundant_closure_call)]
                    ($load)(cell, frame);
                },
                $crate::FortressRequest::AdvanceFrame { inputs } => {
                    #[allow(clippy::redundant_closure_call)]
                    ($advance)(inputs);
                },
            }
        }
    }};
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

/// A trait for integrating custom socket implementations with Fortress Rollback.
///
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
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]
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
    // FortressEvent Display Tests
    // ==========================================

    #[test]
    fn fortress_event_display_synchronizing() {
        let event: FortressEvent<TestConfig> = FortressEvent::Synchronizing {
            addr: test_addr(8080),
            total: 5,
            count: 2,
            total_requests_sent: 3,
            elapsed_ms: 100,
        };
        let display = event.to_string();
        assert!(display.starts_with("Synchronizing("));
        assert!(display.contains("2/5"));
        assert!(display.contains("127.0.0.1:8080"));
        assert!(display.contains("requests_sent=3"));
        assert!(display.contains("elapsed=100ms"));
    }

    #[test]
    fn fortress_event_display_synchronized() {
        let event: FortressEvent<TestConfig> = FortressEvent::Synchronized {
            addr: test_addr(9000),
        };
        assert_eq!(event.to_string(), "Synchronized(addr=127.0.0.1:9000)");
    }

    #[test]
    fn fortress_event_display_disconnected() {
        let event: FortressEvent<TestConfig> = FortressEvent::Disconnected {
            addr: test_addr(7000),
        };
        assert_eq!(event.to_string(), "Disconnected(addr=127.0.0.1:7000)");
    }

    #[test]
    fn fortress_event_display_network_interrupted() {
        let event: FortressEvent<TestConfig> = FortressEvent::NetworkInterrupted {
            addr: test_addr(8080),
            disconnect_timeout: 5000,
        };
        let display = event.to_string();
        assert!(display.starts_with("NetworkInterrupted("));
        assert!(display.contains("127.0.0.1:8080"));
        assert!(display.contains("timeout=5000ms"));
    }

    #[test]
    fn fortress_event_display_network_resumed() {
        let event: FortressEvent<TestConfig> = FortressEvent::NetworkResumed {
            addr: test_addr(8080),
        };
        assert_eq!(event.to_string(), "NetworkResumed(addr=127.0.0.1:8080)");
    }

    #[test]
    fn fortress_event_display_wait_recommendation() {
        let event: FortressEvent<TestConfig> = FortressEvent::WaitRecommendation { skip_frames: 3 };
        assert_eq!(event.to_string(), "WaitRecommendation(skip_frames=3)");
    }

    #[test]
    fn fortress_event_display_desync_detected() {
        let event: FortressEvent<TestConfig> = FortressEvent::DesyncDetected {
            frame: Frame::new(100),
            local_checksum: 0x1234,
            remote_checksum: 0x5678,
            addr: test_addr(8080),
        };
        let display = event.to_string();
        assert!(display.starts_with("DesyncDetected("));
        assert!(display.contains("frame=100"));
        assert!(display.contains("local=0x1234"));
        assert!(display.contains("remote=0x5678"));
        assert!(display.contains("127.0.0.1:8080"));
    }

    #[test]
    fn fortress_event_display_sync_timeout() {
        let event: FortressEvent<TestConfig> = FortressEvent::SyncTimeout {
            addr: test_addr(8080),
            elapsed_ms: 10000,
        };
        let display = event.to_string();
        assert!(display.starts_with("SyncTimeout("));
        assert!(display.contains("127.0.0.1:8080"));
        assert!(display.contains("elapsed=10000ms"));
    }

    // ==========================================
    // SessionState Display Tests
    // ==========================================

    #[test]
    fn session_state_display_synchronizing() {
        assert_eq!(SessionState::Synchronizing.to_string(), "Synchronizing");
    }

    #[test]
    fn session_state_display_running() {
        assert_eq!(SessionState::Running.to_string(), "Running");
    }

    // ==========================================
    // InputStatus Display Tests
    // ==========================================

    #[test]
    fn input_status_display_confirmed() {
        assert_eq!(InputStatus::Confirmed.to_string(), "Confirmed");
    }

    #[test]
    fn input_status_display_predicted() {
        assert_eq!(InputStatus::Predicted.to_string(), "Predicted");
    }

    #[test]
    fn input_status_display_disconnected() {
        assert_eq!(InputStatus::Disconnected.to_string(), "Disconnected");
    }

    // ==========================================
    // Frame Display Tests
    // ==========================================

    #[test]
    fn frame_display_valid() {
        assert_eq!(Frame::new(42).to_string(), "42");
        assert_eq!(Frame::new(0).to_string(), "0");
        assert_eq!(Frame::new(12345).to_string(), "12345");
    }

    #[test]
    fn frame_display_null() {
        assert_eq!(Frame::NULL.to_string(), "NULL_FRAME");
    }

    #[test]
    fn frame_display_negative() {
        // Negative frames other than NULL show as-is
        assert_eq!(Frame::new(-5).to_string(), "-5");
    }

    // ==========================================
    // DesyncDetection Display Tests
    // ==========================================

    #[test]
    fn desync_detection_display_on() {
        let detection = DesyncDetection::On { interval: 60 };
        assert_eq!(detection.to_string(), "On(interval=60)");
    }

    #[test]
    fn desync_detection_display_on_custom_interval() {
        let detection = DesyncDetection::On { interval: 1 };
        assert_eq!(detection.to_string(), "On(interval=1)");
    }

    #[test]
    fn desync_detection_display_off() {
        assert_eq!(DesyncDetection::Off.to_string(), "Off");
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

    #[test]
    fn player_handle_display_format() {
        let handle = PlayerHandle::new(0);
        assert_eq!(format!("{}", handle), "PlayerHandle(0)");

        let handle = PlayerHandle::new(5);
        assert_eq!(format!("{}", handle), "PlayerHandle(5)");

        let handle = PlayerHandle::new(42);
        assert_eq!(format!("{}", handle), "PlayerHandle(42)");
    }

    // ==========================================
    // PlayerType Display Tests
    // ==========================================

    #[test]
    fn player_type_display_local() {
        let player: PlayerType<SocketAddr> = PlayerType::Local;
        assert_eq!(format!("{}", player), "Local");
    }

    #[test]
    fn player_type_display_remote() {
        let addr = test_addr(8080);
        let player: PlayerType<SocketAddr> = PlayerType::Remote(addr);
        let display = format!("{}", player);
        assert!(display.starts_with("Remote("));
        assert!(display.contains("127.0.0.1:8080"));
    }

    #[test]
    fn player_type_display_spectator() {
        let addr = test_addr(9000);
        let player: PlayerType<SocketAddr> = PlayerType::Spectator(addr);
        let display = format!("{}", player);
        assert!(display.starts_with("Spectator("));
        assert!(display.contains("127.0.0.1:9000"));
    }

    // ==========================================
    // FortressRequest Display Tests
    // ==========================================

    #[test]
    fn fortress_request_display_save_game_state() {
        let cell = GameStateCell::<Vec<u8>>::default();
        let request: FortressRequest<TestConfig> = FortressRequest::SaveGameState {
            cell,
            frame: Frame::new(100),
        };
        let display = format!("{}", request);
        assert_eq!(display, "SaveGameState(frame=100)");
    }

    #[test]
    fn fortress_request_display_load_game_state() {
        let cell = GameStateCell::<Vec<u8>>::default();
        let request: FortressRequest<TestConfig> = FortressRequest::LoadGameState {
            cell,
            frame: Frame::new(50),
        };
        let display = format!("{}", request);
        assert_eq!(display, "LoadGameState(frame=50)");
    }

    #[test]
    fn fortress_request_display_advance_frame() {
        use crate::InputVec;
        let inputs: InputVec<u8> = smallvec::smallvec![
            (1_u8, InputStatus::Confirmed),
            (2_u8, InputStatus::Predicted),
        ];
        let request: FortressRequest<TestConfig> = FortressRequest::AdvanceFrame { inputs };
        let display = format!("{}", request);
        assert_eq!(display, "AdvanceFrame(inputs=2)");
    }

    #[test]
    fn fortress_request_display_advance_frame_empty() {
        use crate::InputVec;
        let inputs: InputVec<u8> = smallvec::smallvec![];
        let request: FortressRequest<TestConfig> = FortressRequest::AdvanceFrame { inputs };
        let display = format!("{}", request);
        assert_eq!(display, "AdvanceFrame(inputs=0)");
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

    // ==========================================
    // Frame Checked/Saturating Arithmetic Tests
    // ==========================================

    #[test]
    fn frame_checked_add_normal() {
        let frame = Frame::new(100);
        assert_eq!(frame.checked_add(50), Some(Frame::new(150)));
        assert_eq!(frame.checked_add(-50), Some(Frame::new(50)));
        assert_eq!(frame.checked_add(0), Some(frame));
    }

    #[test]
    fn frame_checked_add_overflow() {
        let frame = Frame::new(i32::MAX);
        assert_eq!(frame.checked_add(1), None);
        assert_eq!(frame.checked_add(100), None);

        // Underflow case
        let frame = Frame::new(i32::MIN);
        assert_eq!(frame.checked_add(-1), None);
    }

    #[test]
    fn frame_checked_sub_normal() {
        let frame = Frame::new(100);
        assert_eq!(frame.checked_sub(50), Some(Frame::new(50)));
        assert_eq!(frame.checked_sub(-50), Some(Frame::new(150)));
        assert_eq!(frame.checked_sub(0), Some(frame));
    }

    #[test]
    fn frame_checked_sub_overflow() {
        let frame = Frame::new(i32::MIN);
        assert_eq!(frame.checked_sub(1), None);

        let frame = Frame::new(i32::MAX);
        assert_eq!(frame.checked_sub(-1), None);
    }

    #[test]
    fn frame_saturating_add_normal() {
        let frame = Frame::new(100);
        assert_eq!(frame.saturating_add(50), Frame::new(150));
        assert_eq!(frame.saturating_add(-50), Frame::new(50));
    }

    #[test]
    fn frame_saturating_add_clamps_at_max() {
        let frame = Frame::new(i32::MAX);
        assert_eq!(frame.saturating_add(1), Frame::new(i32::MAX));
        assert_eq!(frame.saturating_add(100), Frame::new(i32::MAX));
    }

    #[test]
    fn frame_saturating_add_clamps_at_min() {
        let frame = Frame::new(i32::MIN);
        assert_eq!(frame.saturating_add(-1), Frame::new(i32::MIN));
        assert_eq!(frame.saturating_add(-100), Frame::new(i32::MIN));
    }

    #[test]
    fn frame_saturating_sub_normal() {
        let frame = Frame::new(100);
        assert_eq!(frame.saturating_sub(50), Frame::new(50));
        assert_eq!(frame.saturating_sub(-50), Frame::new(150));
    }

    #[test]
    fn frame_saturating_sub_clamps_at_min() {
        let frame = Frame::new(i32::MIN);
        assert_eq!(frame.saturating_sub(1), Frame::new(i32::MIN));
    }

    #[test]
    fn frame_saturating_sub_clamps_at_max() {
        let frame = Frame::new(i32::MAX);
        assert_eq!(frame.saturating_sub(-1), Frame::new(i32::MAX));
    }

    #[test]
    fn frame_abs_diff_basic() {
        let f1 = Frame::new(10);
        let f2 = Frame::new(15);

        // Order-independent
        assert_eq!(f1.abs_diff(f2), 5);
        assert_eq!(f2.abs_diff(f1), 5);

        // Same frame
        assert_eq!(f1.abs_diff(f1), 0);
    }

    #[test]
    fn frame_abs_diff_extremes() {
        // Large positive difference
        let f1 = Frame::new(0);
        let f2 = Frame::new(i32::MAX);
        assert_eq!(f1.abs_diff(f2), i32::MAX as u32);

        // With NULL frame (-1)
        let null = Frame::NULL;
        let zero = Frame::new(0);
        assert_eq!(null.abs_diff(zero), 1);
    }

    // ==========================================
    // Safe Frame Macro Tests
    // ==========================================

    #[test]
    fn safe_frame_add_normal_operation() {
        let frame = Frame::new(100);
        let result = safe_frame_add!(frame, 50, "test_normal_add");
        assert_eq!(result, Frame::new(150));
    }

    #[test]
    fn safe_frame_add_returns_saturated_on_overflow() {
        let frame = Frame::new(i32::MAX);
        let result = safe_frame_add!(frame, 1, "test_overflow_add");
        // Should return saturated value (max) instead of panicking
        assert_eq!(result, Frame::new(i32::MAX));
    }

    #[test]
    fn safe_frame_sub_normal_operation() {
        let frame = Frame::new(100);
        let result = safe_frame_sub!(frame, 50, "test_normal_sub");
        assert_eq!(result, Frame::new(50));
    }

    #[test]
    fn safe_frame_sub_returns_saturated_on_underflow() {
        let frame = Frame::new(i32::MIN);
        let result = safe_frame_sub!(frame, 1, "test_underflow_sub");
        // Should return saturated value (min) instead of panicking
        assert_eq!(result, Frame::new(i32::MIN));
    }

    #[test]
    fn safe_frame_macros_accept_negative_deltas() {
        let frame = Frame::new(100);

        // Negative add = subtract
        let result = safe_frame_add!(frame, -25, "test_negative_add");
        assert_eq!(result, Frame::new(75));

        // Negative sub = add
        let result = safe_frame_sub!(frame, -25, "test_negative_sub");
        assert_eq!(result, Frame::new(125));
    }

    // ==========================================
    // Frame Ergonomic Methods Tests
    // ==========================================

    #[test]
    fn frame_as_usize_positive() {
        assert_eq!(Frame::new(0).as_usize(), Some(0));
        assert_eq!(Frame::new(42).as_usize(), Some(42));
        assert_eq!(Frame::new(i32::MAX).as_usize(), Some(i32::MAX as usize));
    }

    #[test]
    fn frame_as_usize_negative() {
        assert_eq!(Frame::NULL.as_usize(), None);
        assert_eq!(Frame::new(-1).as_usize(), None);
        assert_eq!(Frame::new(-100).as_usize(), None);
        assert_eq!(Frame::new(i32::MIN).as_usize(), None);
    }

    #[test]
    fn frame_try_as_usize_positive() {
        assert_eq!(Frame::new(0).try_as_usize().unwrap(), 0);
        assert_eq!(Frame::new(42).try_as_usize().unwrap(), 42);
    }

    #[test]
    fn frame_try_as_usize_negative_returns_error() {
        let err = Frame::NULL.try_as_usize().unwrap_err();
        assert!(matches!(
            err,
            FortressError::InvalidFrameStructured {
                reason: InvalidFrameReason::MustBeNonNegative,
                ..
            }
        ));
    }

    #[test]
    fn frame_buffer_index_basic() {
        assert_eq!(Frame::new(0).buffer_index(4), Some(0));
        assert_eq!(Frame::new(1).buffer_index(4), Some(1));
        assert_eq!(Frame::new(4).buffer_index(4), Some(0));
        assert_eq!(Frame::new(7).buffer_index(4), Some(3));
        assert_eq!(Frame::new(100).buffer_index(8), Some(4)); // 100 % 8 = 4
    }

    #[test]
    fn frame_buffer_index_negative_frame() {
        assert_eq!(Frame::NULL.buffer_index(4), None);
        assert_eq!(Frame::new(-5).buffer_index(4), None);
    }

    #[test]
    fn frame_buffer_index_zero_size() {
        assert_eq!(Frame::new(5).buffer_index(0), None);
        assert_eq!(Frame::new(0).buffer_index(0), None);
    }

    #[test]
    fn frame_try_buffer_index_success() {
        assert_eq!(Frame::new(0).try_buffer_index(4).unwrap(), 0);
        assert_eq!(Frame::new(1).try_buffer_index(4).unwrap(), 1);
        assert_eq!(Frame::new(4).try_buffer_index(4).unwrap(), 0);
        assert_eq!(Frame::new(7).try_buffer_index(4).unwrap(), 3);
        assert_eq!(Frame::new(100).try_buffer_index(8).unwrap(), 4); // 100 % 8 = 4
    }

    #[test]
    fn frame_try_buffer_index_negative_frame() {
        let err = Frame::NULL.try_buffer_index(4).unwrap_err();
        assert!(matches!(err, FortressError::InvalidFrameStructured { .. }));

        let err = Frame::new(-5).try_buffer_index(4).unwrap_err();
        assert!(matches!(err, FortressError::InvalidFrameStructured { .. }));
    }

    #[test]
    fn frame_try_buffer_index_zero_size() {
        let err = Frame::new(5).try_buffer_index(0).unwrap_err();
        assert!(matches!(
            err,
            FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::ZeroBufferSize
            }
        ));

        // Verify ZeroBufferSize error takes precedence even with Frame::new(0)
        let err = Frame::new(0).try_buffer_index(0).unwrap_err();
        assert!(matches!(
            err,
            FortressError::InvalidRequestStructured {
                kind: InvalidRequestKind::ZeroBufferSize
            }
        ));
    }

    #[test]
    fn frame_try_add_success() {
        let frame = Frame::new(100);
        assert_eq!(frame.try_add(50).unwrap(), Frame::new(150));
        assert_eq!(frame.try_add(-50).unwrap(), Frame::new(50));
        assert_eq!(frame.try_add(0).unwrap(), Frame::new(100));
    }

    #[test]
    fn frame_try_add_overflow() {
        let err = Frame::new(i32::MAX).try_add(1).unwrap_err();
        assert!(matches!(
            err,
            FortressError::FrameArithmeticOverflow {
                operation: "add",
                operand: 1,
                ..
            }
        ));
    }

    #[test]
    fn frame_try_sub_success() {
        let frame = Frame::new(100);
        assert_eq!(frame.try_sub(50).unwrap(), Frame::new(50));
        assert_eq!(frame.try_sub(-50).unwrap(), Frame::new(150));
    }

    #[test]
    fn frame_try_sub_overflow() {
        let err = Frame::new(i32::MIN).try_sub(1).unwrap_err();
        assert!(matches!(
            err,
            FortressError::FrameArithmeticOverflow {
                operation: "sub",
                operand: 1,
                ..
            }
        ));
    }

    #[test]
    fn frame_next_success() {
        assert_eq!(Frame::new(0).next().unwrap(), Frame::new(1));
        assert_eq!(Frame::new(100).next().unwrap(), Frame::new(101));
    }

    #[test]
    fn frame_next_overflow() {
        assert!(Frame::new(i32::MAX).next().is_err());
    }

    #[test]
    fn frame_prev_success() {
        assert_eq!(Frame::new(1).prev().unwrap(), Frame::new(0));
        assert_eq!(Frame::new(100).prev().unwrap(), Frame::new(99));
    }

    #[test]
    fn frame_prev_overflow() {
        assert!(Frame::new(i32::MIN).prev().is_err());
    }

    #[test]
    fn frame_saturating_next() {
        assert_eq!(Frame::new(0).saturating_next(), Frame::new(1));
        assert_eq!(Frame::new(100).saturating_next(), Frame::new(101));
        assert_eq!(Frame::new(i32::MAX).saturating_next(), Frame::new(i32::MAX));
    }

    #[test]
    fn frame_saturating_prev() {
        assert_eq!(Frame::new(1).saturating_prev(), Frame::new(0));
        assert_eq!(Frame::new(100).saturating_prev(), Frame::new(99));
        assert_eq!(Frame::new(i32::MIN).saturating_prev(), Frame::new(i32::MIN));
    }

    #[test]
    fn frame_from_usize_valid() {
        assert_eq!(Frame::from_usize(0), Some(Frame::new(0)));
        assert_eq!(Frame::from_usize(42), Some(Frame::new(42)));
        assert_eq!(
            Frame::from_usize(i32::MAX as usize),
            Some(Frame::new(i32::MAX))
        );
    }

    #[test]
    fn frame_from_usize_too_large() {
        let too_large = (i32::MAX as usize) + 1;
        assert_eq!(Frame::from_usize(too_large), None);
        assert_eq!(Frame::from_usize(usize::MAX), None);
    }

    #[test]
    fn frame_try_from_usize_valid() {
        assert_eq!(Frame::try_from_usize(0).unwrap(), Frame::new(0));
        assert_eq!(Frame::try_from_usize(42).unwrap(), Frame::new(42));
    }

    #[test]
    fn frame_try_from_usize_too_large() {
        let too_large = (i32::MAX as usize) + 1;
        let err = Frame::try_from_usize(too_large).unwrap_err();
        assert!(matches!(
            err,
            FortressError::FrameValueTooLarge { value } if value == too_large
        ));
    }

    #[test]
    fn frame_distance_to_basic() {
        let a = Frame::new(100);
        let b = Frame::new(150);

        assert_eq!(a.distance_to(b), Some(50));
        assert_eq!(b.distance_to(a), Some(-50));
        assert_eq!(a.distance_to(a), Some(0));
    }

    #[test]
    fn frame_distance_to_with_negative_frames() {
        let a = Frame::new(-10);
        let b = Frame::new(10);
        assert_eq!(a.distance_to(b), Some(20));
        assert_eq!(b.distance_to(a), Some(-20));
    }

    #[test]
    fn frame_distance_to_overflow() {
        // This would overflow: i32::MAX - i32::MIN
        assert_eq!(Frame::new(i32::MIN).distance_to(Frame::new(i32::MAX)), None);
        assert_eq!(Frame::new(i32::MAX).distance_to(Frame::new(i32::MIN)), None);
    }

    #[test]
    fn frame_is_within_inside() {
        let reference = Frame::new(100);
        assert!(Frame::new(100).is_within(5, reference)); // diff = 0
        assert!(Frame::new(98).is_within(5, reference)); // diff = 2
        assert!(Frame::new(102).is_within(5, reference)); // diff = 2
        assert!(Frame::new(95).is_within(5, reference)); // diff = 5 (boundary)
        assert!(Frame::new(105).is_within(5, reference)); // diff = 5 (boundary)
    }

    #[test]
    fn frame_is_within_outside() {
        let reference = Frame::new(100);
        assert!(!Frame::new(94).is_within(5, reference)); // diff = 6
        assert!(!Frame::new(106).is_within(5, reference)); // diff = 6
        assert!(!Frame::new(0).is_within(5, reference)); // diff = 100
    }

    #[test]
    fn frame_is_within_zero_window() {
        let reference = Frame::new(100);
        assert!(Frame::new(100).is_within(0, reference)); // exact match
        assert!(!Frame::new(99).is_within(0, reference)); // any diff is outside
        assert!(!Frame::new(101).is_within(0, reference));
    }

    #[test]
    fn frame_is_within_max_window() {
        let reference = Frame::new(0);
        // With max window, everything should be within
        assert!(Frame::new(i32::MAX).is_within(u32::MAX, reference));
        assert!(Frame::new(i32::MIN).is_within(u32::MAX, reference));
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

    /// Proof: Frame::new creates valid frames for non-negative inputs.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Frame construction preserves value and validity
    /// - Related: proof_frame_null_consistency, proof_frame_to_option
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

    /// Proof: Frame::NULL is consistently null.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: NULL frame identity and invariants
    /// - Related: proof_frame_new_valid, proof_frame_to_option
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

    /// Proof: Frame addition with small positive values is safe.
    ///
    /// This proves that for frames in typical game usage (0 to 10,000,000),
    /// adding small increments (0-1000) does not overflow.
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: Frame addition overflow safety (SAFE-6)
    /// - Related: proof_frame_add_assign_consistent, proof_frame_sub_frames_correct
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

    /// Proof: Frame subtraction produces correct differences.
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: Frame subtraction correctness
    /// - Related: proof_frame_add_small_safe, proof_frame_sub_assign_consistent
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

    /// Proof: Frame ordering is consistent with i32 ordering.
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: Frame comparison operators consistency
    /// - Related: proof_frame_ordering
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

    /// Proof: Frame modulo operation is correct for queue indexing.
    ///
    /// This is critical for InputQueue circular buffer indexing (INV-5).
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: Queue index bounds via modulo (INV-5)
    /// - Related: proof_queue_index_calculation, proof_head_wraparound
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

    /// Proof: Frame::to_option correctly handles null and valid frames.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Frame to Option conversion correctness
    /// - Related: proof_frame_from_option, proof_frame_null_consistency
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

    /// Proof: Frame::from_option correctly handles Some and None.
    ///
    /// - Tier: 1 (Fast, <30s)
    /// - Verifies: Option to Frame conversion correctness
    /// - Related: proof_frame_to_option, proof_frame_null_consistency
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

    /// Proof: Frame AddAssign is consistent with Add.
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: AddAssign operator equivalence with Add
    /// - Related: proof_frame_add_small_safe, proof_frame_sub_assign_consistent
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

    /// Proof: Frame SubAssign is consistent with Sub.
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: SubAssign operator equivalence with Sub
    /// - Related: proof_frame_sub_frames_correct, proof_frame_add_assign_consistent
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

    /// Proof: PlayerHandle validity check is correct.
    ///
    /// - Tier: 2 (Medium, 30s-2min)
    /// - Verifies: PlayerHandle player vs spectator classification
    /// - Related: proof_player_handle_preservation, proof_player_handle_equality
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
