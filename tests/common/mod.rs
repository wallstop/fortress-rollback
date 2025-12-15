//! Common test infrastructure shared across integration tests.
//!
//! This module provides:
//! - `stubs`: Game stub implementations with struct-based inputs
//! - `stubs_enum`: Game stub implementations with enum-based inputs
//!
//! # Usage
//!
//! From any integration test file:
//! ```ignore
//! mod common;
//! use common::stubs::{GameStub, StubConfig, StubInput};
//! ```

pub mod stubs;
pub mod stubs_enum;
