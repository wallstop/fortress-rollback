//! Deterministic whole-mesh simulation tests (DST).
//!
//! FoundationDB/TigerBeetle-style simulation testing for Fortress Rollback:
//! N real P2P sessions run in one process over a seeded, virtual-time
//! simulated network ([`common::sim_net::SimNet`]), driven by materialized
//! fault schedules and checked by a global invariant oracle (confirmed-prefix
//! agreement, state agreement, in-band desync cross-check, liveness).
//!
//! Every failure reproduces from `(seed, SimConfig)` — see the
//! `FORTRESS_SIM_REPRO` line printed on failure.

// Allow test-specific patterns that are appropriate for test code
#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::ip_constant
)]

// Shared test infrastructure
#[path = "common/mod.rs"]
mod common;

// Simulation test modules
mod simulation {
    pub mod baseline_sweep;
    pub mod census;
    pub mod corpus_replay;
    pub mod fleet;
    pub mod harness;
}
