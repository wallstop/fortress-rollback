//! Convenient re-exports for common usage.
//!
//! This module provides a "prelude" that re-exports the most commonly used types
//! from Fortress Rollback, allowing you to import them all at once.
//!
//! # Usage
//!
//! ```rust
//! use fortress_rollback::prelude::*;
//! ```
//!
//! # What's Included
//!
//! The prelude includes:
//!
//! - **Session types**: [`P2PSession`], [`SpectatorSession`], [`SyncTestSession`], [`SessionBuilder`]
//! - **Core traits**: [`Config`], [`NonBlockingSocket`]
//! - **Socket implementations**: [`UdpNonBlockingSocket`]
//! - **Fundamental types**: [`Frame`], [`PlayerHandle`], [`PlayerType`], [`NULL_FRAME`]
//! - **Session state**: [`SessionState`], [`InputStatus`], [`DesyncDetection`]
//! - **Request/Event handling**: [`FortressRequest`], [`FortressEvent`], [`handle_requests`]
//! - **Error handling**: [`FortressError`], [`FortressResult`]
//! - **Game state**: [`GameStateCell`], [`GameStateAccessor`], [`InputVec`]
//! - **Network monitoring**: [`NetworkStats`]
//! - **Configuration**: [`SyncConfig`]
//!
//! # Example
//!
//! ```rust
//! use fortress_rollback::prelude::*;
//! use serde::{Deserialize, Serialize};
//! use std::net::SocketAddr;
//!
//! // Define your input type
//! #[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
//! struct MyInput {
//!     buttons: u8,
//! }
//!
//! // Define your game state
//! #[derive(Clone, Default)]
//! struct MyGameState {
//!     frame: i32,
//!     player_positions: [(i32, i32); 2],
//! }
//!
//! // Create the config marker struct
//! struct MyConfig;
//!
//! impl Config for MyConfig {
//!     type Input = MyInput;
//!     type State = MyGameState;
//!     type Address = SocketAddr;
//! }
//! ```

// Core session types
pub use crate::sessions::builder::SessionBuilder;
pub use crate::sessions::p2p_session::P2PSession;
pub use crate::sessions::p2p_spectator_session::SpectatorSession;
pub use crate::sessions::sync_test_session::SyncTestSession;

// Core traits
pub use crate::{Config, NonBlockingSocket};

// Standard socket implementation
pub use crate::UdpNonBlockingSocket;

// Fundamental types and constants
pub use crate::{Frame, PlayerHandle, PlayerType, NULL_FRAME};

// Session state types
pub use crate::{DesyncDetection, InputStatus, SessionState};

// Request and event handling
pub use crate::{handle_requests, FortressEvent, FortressRequest};

// Error handling
pub use crate::{FortressError, FortressResult};

// Game state management
pub use crate::sync_layer::{GameStateAccessor, GameStateCell};

// Input vector type for advance frame
pub use crate::InputVec;

// Network monitoring
pub use crate::NetworkStats;

// Common configuration types
pub use crate::sessions::config::SyncConfig;
