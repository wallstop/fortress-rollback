//! Network integration tests.
//!
//! This module contains integration tests for network resilience and multi-process testing:
//! - Network resilience with ChaosSocket (packet loss, latency, jitter)
//! - Multi-process network testing with real UDP sockets

// Shared test infrastructure
#[path = "common/mod.rs"]
mod common;

// Network test modules
mod network {
    pub mod multi_process;
    pub mod resilience;
}
