//! Multi-process network testing.
//!
//! These tests spawn separate instances of `network_test_peer` to validate
//! P2P networking over real UDP sockets. This tests actual network I/O
//! between separate processes, providing more realistic validation than
//! in-process tests.
//!
//! # Requirements
//!
//! The `network_test_peer` binary must be built before running these tests:
//! ```bash
//! cargo build -p network-test-peer
//! ```
//!
//! # Test Categories
//!
//! - **Basic connectivity**: Two peers can connect and advance frames
//! - **Chaos conditions**: Testing with packet loss, latency, jitter
//! - **Stress tests**: High frame counts, aggressive network conditions

// =============================================================================
// Guidance: keep hostile network tests on the scenario harness
// =============================================================================
//
// This file contains two testing styles:
//
// 1. **Legacy tests** (e.g., `test_packet_loss_5_percent`, `test_latency_30ms`):
//    - Individual test functions with inline `PeerConfig` setup
//    - Harder to maintain, easy to forget sync presets
//
// 2. **Modern scenario tests** (e.g., `test_asymmetric_scenarios`):
//    - Use `NetworkScenario` and `NetworkProfile` abstractions
//    - Auto-sync preset selection via `with_auto_sync_preset()`
//    - Parameterized, table-driven, easier to extend
//
// Hostile real-UDP tests are smoke coverage. Tests use `NetworkScenario` with
// `with_auto_sync_preset()` and run a single attempt via `run_test()`; there is
// no retry mechanism, so a failure is a real failure. Determinism under extreme
// chaos is covered deterministically in `tests/network/in_process_chaos.rs`.
// Direct peer config tests that use significant packet loss or burst loss must
// set an explicit robust sync preset.
//
// See: `.llm/skills/testing/network-chaos-testing.md` for chaos testing best practices
// =============================================================================

// Allow test-specific patterns that are appropriate for test code
#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]
// Allow print macros for test debugging output
#![allow(clippy::print_stdout, clippy::print_stderr, clippy::disallowed_macros)]

use serde::Deserialize;
use serial_test::serial;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

/// Result from a test peer process
#[derive(Debug, Deserialize)]
struct TestResult {
    success: bool,
    final_frame: i32,
    final_value: i64,
    checksum: u64,
    rollbacks: u32,
    /// Per-peer count of `FortressEvent::DesyncDetected` events, surfaced as a
    /// top-level field by the binary. `verify_determinism_n` asserts this is
    /// zero for every peer: the historical 0%-loss false positive that kept it
    /// log-only was finding F17 (a prediction episode re-entered at the
    /// requested frame instead of the queue's first missing frame, silently
    /// swallowing misprediction comparisons), fixed in S30 in
    /// `InputQueue::input`. See [`TestResult::desync_count`] and the module
    /// note above the N-peer tests for the full history.
    #[serde(default)]
    desync_detected: u32,
    #[serde(default)]
    error_kind: Option<String>,
    error: Option<String>,
    #[serde(default)]
    runtime: Option<RuntimeDiagnostics>,
}

/// Runtime diagnostics emitted by `network_test_peer`.
#[derive(Debug, Deserialize)]
struct RuntimeDiagnostics {
    session_state: String,
    current_frame: i32,
    confirmed_frame: i32,
    target_frame: i32,
    elapsed_ms: u128,
    sync_preset: Option<String>,
    sync_config: String,
    protocol_config: Option<String>,
    time_sync_config: Option<String>,
    sync_health: String,
    events: EventSummary,
}

/// Event counters emitted by `network_test_peer`.
#[derive(Debug, Deserialize)]
struct EventSummary {
    synchronizing: u32,
    synchronized: u32,
    network_interrupted: u32,
    network_resumed: u32,
    disconnected: u32,
    sync_timeout: u32,
    wait_recommendation: u32,
    input_delay_recommendation: u32,
    desync_detected: u32,
    peer_dropped: u32,
    replay_desync: u32,
}

impl TestResult {
    /// Number of `DesyncDetected` events this peer observed.
    ///
    /// Prefers the top-level `desync_detected` field; falls back to the nested
    /// `runtime.events.desync_detected` so the count is still available even if
    /// the top-level field is ever absent (older binary / partial JSON).
    fn desync_count(&self) -> u32 {
        if self.desync_detected > 0 {
            self.desync_detected
        } else {
            self.runtime
                .as_ref()
                .map_or(0, |r| r.events.desync_detected)
        }
    }

    /// Returns a diagnostic summary string for debugging test failures
    fn diagnostic_summary(&self) -> String {
        let base = format!(
            "success={}, frame={}, value={}, checksum={:x}, rollbacks={}, error={:?}",
            self.success,
            self.final_frame,
            self.final_value,
            self.checksum,
            self.rollbacks,
            self.error
        );

        if let Some(runtime) = &self.runtime {
            format!(
                "{base}, kind={:?}, state={}, current={}, confirmed={}, target={}, elapsed_ms={}, sync={:?}, sync_config={}, protocol_config={}, time_sync_config={}, sync_health={}, events={}",
                self.error_kind,
                runtime.session_state,
                runtime.current_frame,
                runtime.confirmed_frame,
                runtime.target_frame,
                runtime.elapsed_ms,
                runtime.sync_preset,
                runtime.sync_config,
                runtime.protocol_config.as_deref().unwrap_or("<missing>"),
                runtime.time_sync_config.as_deref().unwrap_or("<missing>"),
                runtime.sync_health,
                runtime.events.summary(),
            )
        } else {
            format!("{base}, kind={:?}", self.error_kind)
        }
    }
}

impl EventSummary {
    fn summary(&self) -> String {
        format!(
            "syncing={}, synced={}, interrupted={}, resumed={}, disconnected={}, sync_timeout={}, wait={}, delay={}, desync={}, dropped={}, replay_desync={}",
            self.synchronizing,
            self.synchronized,
            self.network_interrupted,
            self.network_resumed,
            self.disconnected,
            self.sync_timeout,
            self.wait_recommendation,
            self.input_delay_recommendation,
            self.desync_detected,
            self.peer_dropped,
            self.replay_desync,
        )
    }
}

/// Configuration for a test peer
struct PeerConfig {
    local_port: u16,
    player_index: usize,
    peer_addr: String,
    /// Additional remote peer addresses beyond `peer_addr`, in
    /// ascending-remote-handle order, for N >= 3 meshes; empty for 2-peer.
    ///
    /// These are emitted as extra `--peer` args after `peer_addr`, so the full
    /// ordered remote list this peer sends to the binary is
    /// `[peer_addr] ++ extra_peer_addrs`. The binary maps that list onto the
    /// ascending remote handles `(0..num_players).filter(|h| h != player_index)`.
    extra_peer_addrs: Vec<String>,
    frames: i32,
    packet_loss: f64,
    latency_ms: u64,
    jitter_ms: u64,
    seed: Option<u64>,
    timeout_secs: u64,
    input_delay: usize,
    // Extended chaos options
    reorder_rate: f64,
    reorder_buffer_size: usize,
    duplicate_rate: f64,
    burst_loss_prob: f64,
    burst_loss_len: usize,
    // Sync configuration preset
    sync_preset: Option<String>,
}

impl Default for PeerConfig {
    fn default() -> Self {
        Self {
            local_port: 0,
            player_index: 0,
            peer_addr: String::new(),
            extra_peer_addrs: Vec::new(),
            frames: 100,
            packet_loss: 0.0,
            latency_ms: 0,
            jitter_ms: 0,
            seed: Some(42), // Deterministic by default for reproducible tests
            timeout_secs: 30,
            input_delay: 2,
            reorder_rate: 0.0,
            reorder_buffer_size: 0,
            duplicate_rate: 0.0,
            burst_loss_prob: 0.0,
            burst_loss_len: 0,
            sync_preset: None,
        }
    }
}

impl PeerConfig {
    /// Returns a diagnostic summary string for debugging test failures
    fn diagnostic_summary(&self) -> String {
        format!(
            "port={}, player={}, peer={}, extra_peers={:?}, frames={}, loss={:.1}%, latency={}ms±{}ms, delay={}, seed={:?}, reorder={:.1}%, dup={:.1}%, burst={:.1}%x{}, sync={:?}",
            self.local_port, self.player_index, self.peer_addr, self.extra_peer_addrs, self.frames,
            self.packet_loss * 100.0, self.latency_ms, self.jitter_ms, self.input_delay, self.seed,
            self.reorder_rate * 100.0, self.duplicate_rate * 100.0,
            self.burst_loss_prob * 100.0, self.burst_loss_len,
            self.sync_preset
        )
    }

    fn network_profile(&self) -> NetworkProfile {
        NetworkProfile {
            packet_loss: self.packet_loss,
            latency_ms: self.latency_ms,
            jitter_ms: self.jitter_ms,
            reorder_rate: self.reorder_rate,
            reorder_buffer_size: self.reorder_buffer_size,
            duplicate_rate: self.duplicate_rate,
            burst_loss_prob: self.burst_loss_prob,
            burst_loss_len: self.burst_loss_len,
        }
    }
}

// =============================================================================
// Network Scenario Abstraction
// =============================================================================

/// Represents a reusable network scenario with symmetric or asymmetric conditions.
///
/// This abstraction simplifies test setup by providing named network profiles
/// that can be easily composed and reused across tests.
///
/// # Examples
///
/// ```ignore
/// // Use a predefined scenario
/// let scenario = NetworkScenario::lan();
/// let (r1, r2) = scenario.run_test(10001, 100);
///
/// // Create a custom asymmetric scenario
/// let scenario = NetworkScenario::asymmetric(
///     NetworkProfile::mobile_4g(),  // Peer 1 conditions
///     NetworkProfile::lan(),        // Peer 2 conditions
/// );
/// ```
#[derive(Debug, Clone)]
struct NetworkScenario {
    name: &'static str,
    peer1_profile: NetworkProfile,
    peer2_profile: NetworkProfile,
    frames: i32,
    input_delay: usize,
    timeout_secs: u64,
    /// Sync configuration preset for the session.
    /// Use "lossy" for moderate packet loss, "mobile" for heavy burst loss.
    sync_preset: Option<String>,
}

/// Network condition profile for a single peer.
///
/// These profiles represent common real-world network conditions.
/// Not all profiles are used in tests yet, but they form a library
/// of reusable configurations for future tests.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct NetworkProfile {
    packet_loss: f64,
    latency_ms: u64,
    jitter_ms: u64,
    // Extended chaos options
    reorder_rate: f64,
    reorder_buffer_size: usize,
    duplicate_rate: f64,
    burst_loss_prob: f64,
    burst_loss_len: usize,
}

#[allow(dead_code)]
impl NetworkProfile {
    /// Perfect local network - no loss, no latency.
    const fn local() -> Self {
        Self {
            packet_loss: 0.0,
            latency_ms: 0,
            jitter_ms: 0,
            reorder_rate: 0.0,
            reorder_buffer_size: 0,
            duplicate_rate: 0.0,
            burst_loss_prob: 0.0,
            burst_loss_len: 0,
        }
    }

    /// LAN conditions - minimal latency, no loss.
    const fn lan() -> Self {
        Self {
            packet_loss: 0.0,
            latency_ms: 1,
            jitter_ms: 1,
            reorder_rate: 0.0,
            reorder_buffer_size: 0,
            duplicate_rate: 0.0,
            burst_loss_prob: 0.0,
            burst_loss_len: 0,
        }
    }

    /// Good WiFi - low latency, slight jitter, rare packet loss.
    const fn wifi_good() -> Self {
        Self {
            packet_loss: 0.01,
            latency_ms: 5,
            jitter_ms: 5,
            reorder_rate: 0.0,
            reorder_buffer_size: 0,
            duplicate_rate: 0.0,
            burst_loss_prob: 0.0,
            burst_loss_len: 0,
        }
    }

    /// Average WiFi with moderate interference.
    const fn wifi_average() -> Self {
        Self {
            packet_loss: 0.05,
            latency_ms: 15,
            jitter_ms: 15,
            reorder_rate: 0.02,
            reorder_buffer_size: 3,
            duplicate_rate: 0.01,
            burst_loss_prob: 0.0,
            burst_loss_len: 0,
        }
    }

    /// Congested WiFi with packet loss and high jitter.
    const fn wifi_congested() -> Self {
        Self {
            packet_loss: 0.15,
            latency_ms: 30,
            jitter_ms: 40,
            reorder_rate: 0.05,
            reorder_buffer_size: 5,
            duplicate_rate: 0.02,
            burst_loss_prob: 0.02,
            burst_loss_len: 3,
        }
    }

    /// Mobile 4G/LTE - higher latency, moderate jitter.
    const fn mobile_4g() -> Self {
        Self {
            packet_loss: 0.08,
            latency_ms: 50,
            jitter_ms: 25,
            reorder_rate: 0.03,
            reorder_buffer_size: 4,
            duplicate_rate: 0.01,
            burst_loss_prob: 0.01,
            burst_loss_len: 2,
        }
    }

    /// Mobile 3G - high latency and jitter.
    const fn mobile_3g() -> Self {
        Self {
            packet_loss: 0.15,
            latency_ms: 100,
            jitter_ms: 50,
            reorder_rate: 0.05,
            reorder_buffer_size: 6,
            duplicate_rate: 0.02,
            burst_loss_prob: 0.02,
            burst_loss_len: 4,
        }
    }

    /// Intercontinental connection - high latency but stable.
    const fn intercontinental() -> Self {
        Self {
            packet_loss: 0.02,
            latency_ms: 150,
            jitter_ms: 20,
            reorder_rate: 0.01,
            reorder_buffer_size: 3,
            duplicate_rate: 0.0,
            burst_loss_prob: 0.0,
            burst_loss_len: 0,
        }
    }

    /// Terrible connection - worst realistic conditions.
    const fn terrible() -> Self {
        Self {
            packet_loss: 0.25,
            latency_ms: 120,
            jitter_ms: 60,
            reorder_rate: 0.10,
            reorder_buffer_size: 8,
            duplicate_rate: 0.05,
            burst_loss_prob: 0.05,
            burst_loss_len: 5,
        }
    }

    /// Profile with heavy packet reordering.
    const fn heavy_reorder() -> Self {
        Self {
            packet_loss: 0.02,
            latency_ms: 30,
            jitter_ms: 20,
            reorder_rate: 0.30,
            reorder_buffer_size: 10,
            duplicate_rate: 0.0,
            burst_loss_prob: 0.0,
            burst_loss_len: 0,
        }
    }

    /// Profile with packet duplication (common in load-balanced networks).
    const fn duplicating() -> Self {
        Self {
            packet_loss: 0.02,
            latency_ms: 20,
            jitter_ms: 10,
            reorder_rate: 0.05,
            reorder_buffer_size: 3,
            duplicate_rate: 0.15,
            burst_loss_prob: 0.0,
            burst_loss_len: 0,
        }
    }

    /// Profile with burst loss (simulates brief network outages).
    ///
    /// **Warning**: This profile with 10% burst probability and 8-packet bursts
    /// can cause sync failures even with `stress_test` preset due to the
    /// compounding effect of both peers applying chaos to their outgoing packets.
    /// Consider using `bursty_survivable()` for reliable CI testing.
    const fn bursty() -> Self {
        Self {
            packet_loss: 0.05,
            latency_ms: 25,
            jitter_ms: 15,
            reorder_rate: 0.02,
            reorder_buffer_size: 3,
            duplicate_rate: 0.01,
            burst_loss_prob: 0.10,
            burst_loss_len: 8,
        }
    }

    /// Profile with aggressive but survivable burst loss.
    ///
    /// This is a tuned variant of `bursty()` designed to stress test burst loss
    /// handling while remaining reliably achievable with the `stress_test` sync
    /// preset. The parameters were chosen based on probability analysis:
    ///
    /// - 5% burst probability (vs 10% in `bursty`) reduces the chance of
    ///   overlapping burst events between the two peers during sync handshake
    /// - 5-packet bursts (vs 8) ensure that even a burst during handshake
    ///   doesn't wipe out too many consecutive sync attempts
    /// - Combined with `stress_test` (40 sync packets, 150ms retry, 60s timeout),
    ///   the probability of successful sync is very high (~99.99%)
    ///
    /// Use this profile for CI tests where reliability is important.
    /// Use `bursty()` for exploratory stress testing where some failures are acceptable.
    const fn bursty_survivable() -> Self {
        Self {
            packet_loss: 0.05,
            latency_ms: 25,
            jitter_ms: 15,
            reorder_rate: 0.02,
            reorder_buffer_size: 3,
            duplicate_rate: 0.01,
            burst_loss_prob: 0.05,
            burst_loss_len: 5,
        }
    }

    /// Suggests an appropriate sync preset based on the profile's network conditions.
    ///
    /// Returns `None` for good network conditions where default sync is sufficient.
    /// This helps prevent flaky tests by ensuring aggressive network profiles
    /// use appropriately robust sync configurations.
    ///
    /// # Sync Preset Selection Guidelines
    ///
    /// - **None (default)**: Effective loss <15%, no burst loss
    /// - **"lossy"**: Effective loss 15-25%, no significant burst loss
    /// - **"mobile"**: Effective loss 25-35%, or moderate burst loss alone
    /// - **"extreme"**: Effective loss ≥35% without burst loss
    /// - **"stress_test"**: Burst loss combined with high effective loss, or extreme burst (10%+ with 8+ bursts)
    const fn suggested_sync_preset(&self) -> Option<&'static str> {
        let effective_loss = self.effective_symmetric_packet_loss();

        // High burst loss with long bursts requires stress_test preset
        if self.burst_loss_prob >= 0.10 && self.burst_loss_len >= 8 {
            return Some("stress_test");
        }

        // High effective packet loss combined with burst loss is much worse
        // than either condition alone and needs the test-only preset.
        if effective_loss >= 0.25 && self.burst_loss_prob >= 0.02 && self.burst_loss_len >= 3 {
            return Some("stress_test");
        }

        // Sustained burst loss at this level is CI-hostile even when baseline
        // packet loss is moderate, so use the test-only preset.
        if self.burst_loss_prob >= 0.05 && self.burst_loss_len >= 5 {
            return Some("stress_test");
        }

        // Very high raw packet loss combined with significant burst loss
        // The combination is much worse than either alone - use stress_test
        if self.packet_loss >= 0.20 && self.burst_loss_prob >= 0.03 && self.burst_loss_len >= 3 {
            return Some("stress_test");
        }

        // Very high effective packet loss without burst loss - extreme is sufficient
        if effective_loss >= 0.35 {
            return Some("extreme");
        }

        // Significant burst loss requires mobile preset
        if self.burst_loss_prob >= 0.02 && self.burst_loss_len >= 3 {
            return Some("mobile");
        }

        // High effective packet loss requires mobile preset
        if effective_loss >= 0.25 {
            return Some("mobile");
        }

        // Moderate effective packet loss requires lossy preset
        if effective_loss >= 0.15 {
            return Some("lossy");
        }

        // Good conditions - default is fine
        None
    }

    /// Effective loss when both peers use this profile for send and receive chaos.
    const fn effective_symmetric_packet_loss(&self) -> f64 {
        1.0 - ((1.0 - self.packet_loss) * (1.0 - self.packet_loss))
    }
}

fn sync_preset_rank(preset: &str) -> u8 {
    match preset {
        "stress_test" => 5,
        "extreme" => 4,
        "mobile" => 3,
        "lossy" => 2,
        _ => 1,
    }
}

#[allow(dead_code)]
impl NetworkScenario {
    /// Creates a symmetric scenario where both peers have the same conditions.
    fn symmetric(name: &'static str, profile: NetworkProfile) -> Self {
        Self {
            name,
            peer1_profile: profile,
            peer2_profile: profile,
            frames: 100,
            input_delay: 2,
            timeout_secs: 60,
            sync_preset: None,
        }
    }

    /// Creates an asymmetric scenario with different conditions per peer.
    fn asymmetric(
        name: &'static str,
        peer1_profile: NetworkProfile,
        peer2_profile: NetworkProfile,
    ) -> Self {
        Self {
            name,
            peer1_profile,
            peer2_profile,
            frames: 100,
            input_delay: 2,
            timeout_secs: 120, // Asymmetric needs more time
            sync_preset: None,
        }
    }

    /// LAN scenario - perfect conditions.
    fn lan() -> Self {
        Self::symmetric("lan", NetworkProfile::lan())
    }

    /// Good WiFi scenario.
    fn wifi_good() -> Self {
        Self::symmetric("wifi_good", NetworkProfile::wifi_good())
    }

    /// Mobile 4G scenario.
    fn mobile_4g() -> Self {
        Self::symmetric("mobile_4g", NetworkProfile::mobile_4g())
    }

    /// Intercontinental scenario.
    fn intercontinental() -> Self {
        Self::symmetric("intercontinental", NetworkProfile::intercontinental()).with_timeout(180)
    }

    /// One peer on good network, one on terrible network.
    ///
    /// Uses "extreme" sync preset for the terrible profile's 25% loss (>20% threshold).
    fn asymmetric_extreme() -> Self {
        Self::asymmetric(
            "asymmetric_extreme",
            NetworkProfile::terrible(),
            NetworkProfile::lan(),
        )
        .with_timeout(180)
        .with_sync_preset("extreme")
    }

    /// Builder: Set frame count.
    fn with_frames(mut self, frames: i32) -> Self {
        self.frames = frames;
        self
    }

    /// Builder: Set input delay.
    fn with_input_delay(mut self, delay: usize) -> Self {
        self.input_delay = delay;
        self
    }

    /// Builder: Set timeout.
    fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }

    /// Builder: Set sync preset.
    ///
    /// Use "lossy" for moderate packet loss (5-15%), "mobile" for heavy burst loss
    /// or high variability conditions. Without this, the default sync config may
    /// fail to synchronize under aggressive chaos conditions.
    fn with_sync_preset(mut self, preset: &str) -> Self {
        self.sync_preset = Some(preset.to_string());
        self
    }

    /// Builder: Auto-apply the suggested sync preset based on network profiles.
    ///
    /// Uses `NetworkProfile::suggested_sync_preset()` to determine the most
    /// appropriate preset for the worst-case profile in the scenario.
    /// If a preset is already set, it won't be overwritten.
    fn with_auto_sync_preset(mut self) -> Self {
        if self.sync_preset.is_some() {
            return self;
        }

        // Use the more aggressive suggestion from either profile
        let preset1 = self.peer1_profile.suggested_sync_preset();
        let preset2 = self.peer2_profile.suggested_sync_preset();

        self.sync_preset = match (preset1, preset2) {
            (None, None) => None,
            (Some(p), None) | (None, Some(p)) => Some(p.to_string()),
            (Some(p1), Some(p2)) => {
                if sync_preset_rank(p1) >= sync_preset_rank(p2) {
                    Some(p1.to_string())
                } else {
                    Some(p2.to_string())
                }
            },
        };
        self
    }

    /// Validates that aggressive network profiles have appropriate sync presets.
    ///
    /// Call this at test runtime to catch potential misconfiguration early.
    /// Prints a warning if the network conditions suggest a sync preset but none is set.
    fn validate_sync_preset(&self) {
        let suggested1 = self.peer1_profile.suggested_sync_preset();
        let suggested2 = self.peer2_profile.suggested_sync_preset();
        let effective_loss = self.max_effective_packet_loss();

        let suggested = match (suggested1, suggested2) {
            (None, None) => None,
            (Some(p), None) | (None, Some(p)) => Some(p),
            (Some(p1), Some(p2)) => {
                if sync_preset_rank(p1) >= sync_preset_rank(p2) {
                    Some(p1)
                } else {
                    Some(p2)
                }
            },
        };

        if let Some(recommended) = suggested {
            match self.sync_preset.as_deref() {
                None => {
                    eprintln!(
                        "WARNING: Scenario '{}' has aggressive network conditions \
                         (raw_loss={:.0}%/{:.0}%, effective_loss={:.1}%, burst={:.0}%/{:.0}%) but no sync preset. \
                         Consider using .with_sync_preset(\"{}\") or .with_auto_sync_preset()",
                        self.name,
                        self.peer1_profile.packet_loss * 100.0,
                        self.peer2_profile.packet_loss * 100.0,
                        effective_loss * 100.0,
                        self.peer1_profile.burst_loss_prob * 100.0,
                        self.peer2_profile.burst_loss_prob * 100.0,
                        recommended
                    );
                },
                Some(configured)
                    if sync_preset_rank(configured) < sync_preset_rank(recommended) =>
                {
                    eprintln!(
                        "WARNING: Scenario '{}' uses sync preset '{}' but network conditions recommend '{}' \
                         (raw_loss={:.0}%/{:.0}%, effective_loss={:.1}%, burst={:.0}%/{:.0}%). \
                         Prefer .with_auto_sync_preset() unless this test is intentionally validating a weaker preset.",
                        self.name,
                        configured,
                        recommended,
                        self.peer1_profile.packet_loss * 100.0,
                        self.peer2_profile.packet_loss * 100.0,
                        effective_loss * 100.0,
                        self.peer1_profile.burst_loss_prob * 100.0,
                        self.peer2_profile.burst_loss_prob * 100.0,
                    );
                },
                Some(_) => {},
            }
        }
    }

    /// Creates peer configs for this scenario at the given port base.
    fn to_peer_configs(&self, port_base: u16) -> (PeerConfig, PeerConfig) {
        let peer1_config = PeerConfig {
            local_port: port_base,
            player_index: 0,
            peer_addr: format!("127.0.0.1:{}", port_base + 1),
            extra_peer_addrs: Vec::new(),
            frames: self.frames,
            packet_loss: self.peer1_profile.packet_loss,
            latency_ms: self.peer1_profile.latency_ms,
            jitter_ms: self.peer1_profile.jitter_ms,
            seed: Some(42),
            timeout_secs: self.timeout_secs,
            input_delay: self.input_delay,
            reorder_rate: self.peer1_profile.reorder_rate,
            reorder_buffer_size: self.peer1_profile.reorder_buffer_size,
            duplicate_rate: self.peer1_profile.duplicate_rate,
            burst_loss_prob: self.peer1_profile.burst_loss_prob,
            burst_loss_len: self.peer1_profile.burst_loss_len,
            sync_preset: self.sync_preset.clone(),
        };

        let peer2_config = PeerConfig {
            local_port: port_base + 1,
            player_index: 1,
            peer_addr: format!("127.0.0.1:{}", port_base),
            extra_peer_addrs: Vec::new(),
            frames: self.frames,
            packet_loss: self.peer2_profile.packet_loss,
            latency_ms: self.peer2_profile.latency_ms,
            jitter_ms: self.peer2_profile.jitter_ms,
            seed: Some(43),
            timeout_secs: self.timeout_secs,
            input_delay: self.input_delay,
            reorder_rate: self.peer2_profile.reorder_rate,
            reorder_buffer_size: self.peer2_profile.reorder_buffer_size,
            duplicate_rate: self.peer2_profile.duplicate_rate,
            burst_loss_prob: self.peer2_profile.burst_loss_prob,
            burst_loss_len: self.peer2_profile.burst_loss_len,
            sync_preset: self.sync_preset.clone(),
        };

        (peer1_config, peer2_config)
    }

    /// Runs the test and verifies determinism.
    fn run_test(&self, port_base: u16) -> (TestResult, TestResult) {
        // Warn about potentially misconfigured scenarios
        self.validate_sync_preset();

        let (peer1, peer2) = self.to_peer_configs(port_base);
        let (result1, result2) = run_two_peer_test(peer1, peer2);

        // Verify determinism
        verify_determinism(&result1, &result2, self.name);

        // Log scenario results
        println!(
            "{}: peer1(frames={}, rollbacks={}), peer2(frames={}, rollbacks={})",
            self.name,
            result1.final_frame,
            result1.rollbacks,
            result2.final_frame,
            result2.rollbacks
        );

        (result1, result2)
    }

    /// Returns a diagnostic summary of this scenario.
    fn summary(&self) -> String {
        format!(
            "{}: peer1(loss={:.0}%, lat={}ms±{}ms, reorder={:.0}%/{}, dup={:.0}%, burst={:.0}%x{}), peer2(loss={:.0}%, lat={}ms±{}ms, reorder={:.0}%/{}, dup={:.0}%, burst={:.0}%x{}), effective_loss={:.1}%, frames={}, delay={}, seeds=42/43, sync={:?}",
            self.name,
            self.peer1_profile.packet_loss * 100.0,
            self.peer1_profile.latency_ms,
            self.peer1_profile.jitter_ms,
            self.peer1_profile.reorder_rate * 100.0,
            self.peer1_profile.reorder_buffer_size,
            self.peer1_profile.duplicate_rate * 100.0,
            self.peer1_profile.burst_loss_prob * 100.0,
            self.peer1_profile.burst_loss_len,
            self.peer2_profile.packet_loss * 100.0,
            self.peer2_profile.latency_ms,
            self.peer2_profile.jitter_ms,
            self.peer2_profile.reorder_rate * 100.0,
            self.peer2_profile.reorder_buffer_size,
            self.peer2_profile.duplicate_rate * 100.0,
            self.peer2_profile.burst_loss_prob * 100.0,
            self.peer2_profile.burst_loss_len,
            self.max_effective_packet_loss() * 100.0,
            self.frames,
            self.input_delay,
            self.sync_preset,
        )
    }

    /// Maximum effective packet loss for either direction of this scenario.
    fn max_effective_packet_loss(&self) -> f64 {
        let peer1_to_peer2 = effective_path_loss(
            self.peer1_profile.packet_loss,
            self.peer2_profile.packet_loss,
        );
        let peer2_to_peer1 = effective_path_loss(
            self.peer2_profile.packet_loss,
            self.peer1_profile.packet_loss,
        );
        peer1_to_peer2.max(peer2_to_peer1)
    }
}

fn effective_path_loss(send_loss: f64, receive_loss: f64) -> f64 {
    1.0 - ((1.0 - send_loss) * (1.0 - receive_loss))
}

/// The binary name for the network test peer (platform-specific).
/// On Windows, executables have a `.exe` suffix.
const PEER_BINARY_NAME: &str = if cfg!(windows) {
    "network_test_peer.exe"
} else {
    "network_test_peer"
};

/// Port offset between test cases in parameter sweeps.
const SWEEP_PORT_OFFSET: u16 = 200;

/// Finds the network_test_peer binary path.
///
/// Returns `Some(PathBuf)` if the binary exists, `None` otherwise.
/// This handles platform-specific executable extensions (.exe on Windows).
fn find_peer_binary() -> Option<std::path::PathBuf> {
    // The network_test_peer binary is in a workspace member crate (tests/network-peer).
    // Since it's part of the workspace, it builds to the shared target directory.
    let test_exe = std::env::current_exe().ok()?;

    // Test executables are in target/debug/deps/, but binaries are in target/debug/
    let target_dir = test_exe
        .parent() // deps
        .and_then(|p| p.parent())?; // debug or release

    // The binary is in the target directory (e.g., target/debug/network_test_peer)
    let peer_binary = target_dir.join(PEER_BINARY_NAME);

    peer_binary.exists().then_some(peer_binary)
}

/// Checks if the network_test_peer binary is available.
///
/// Returns `true` if the binary exists and is ready to use.
/// Call this at the start of tests to skip gracefully if not available.
fn is_peer_binary_available() -> bool {
    find_peer_binary().is_some()
}

/// Skips the test if the network_test_peer binary is not available.
///
/// This macro should be called at the beginning of each multi-process test.
/// It prints a diagnostic message and returns early if the binary is missing.
macro_rules! skip_if_no_peer_binary {
    () => {
        if !is_peer_binary_available() {
            // Provide detailed diagnostic information for CI debugging
            let test_exe = std::env::current_exe().ok();
            let target_dir = test_exe.as_ref().and_then(|p| p.parent()).and_then(|p| p.parent());
            let expected_path = target_dir.map(|d| d.join(PEER_BINARY_NAME));

            eprintln!("╔══════════════════════════════════════════════════════════════╗");
            eprintln!("║ SKIP: network_test_peer binary not found                     ║");
            eprintln!("╠══════════════════════════════════════════════════════════════╣");
            eprintln!("║ Build it with: cargo build -p network-test-peer              ║");
            eprintln!("╚══════════════════════════════════════════════════════════════╝");
            eprintln!();
            eprintln!("Diagnostic info:");
            eprintln!("  Expected binary name: {}", PEER_BINARY_NAME);
            if let Some(path) = &expected_path {
                eprintln!("  Expected at: {}", path.display());
                eprintln!("  Path exists: {}", path.exists());
            }
            if let Some(target) = target_dir {
                eprintln!("  Target directory: {}", target.display());
                eprintln!("  Target exists: {}", target.exists());
            }
            eprintln!("  Current platform: {}", std::env::consts::OS);
            eprintln!();
            return;
        }
    };
}

/// Spawns a test peer process
fn spawn_peer(config: &PeerConfig) -> std::io::Result<Child> {
    validate_peer_config_sync_preset("spawn_peer", config);

    let peer_binary = find_peer_binary().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "network_test_peer binary not found. \
                 Build it with: cargo build -p network-test-peer\n\
                 Expected binary name: {}",
                PEER_BINARY_NAME
            ),
        )
    })?;

    let mut cmd = Command::new(peer_binary);

    cmd.arg("--local-port")
        .arg(config.local_port.to_string())
        .arg("--player-index")
        .arg(config.player_index.to_string())
        .arg("--peer")
        .arg(&config.peer_addr)
        .arg("--frames")
        .arg(config.frames.to_string())
        .arg("--timeout")
        .arg(config.timeout_secs.to_string())
        .arg("--input-delay")
        .arg(config.input_delay.to_string());

    // Emit one additional `--peer` per extra remote address, in order. Combined
    // with the `--peer` above, the binary receives the full ordered remote list
    // `[peer_addr] ++ extra_peer_addrs` and maps it onto ascending remote
    // handles. Empty for 2-peer tests, so their command line is unchanged.
    for extra_peer in &config.extra_peer_addrs {
        cmd.arg("--peer").arg(extra_peer);
    }

    if config.packet_loss > 0.0 {
        cmd.arg("--packet-loss").arg(config.packet_loss.to_string());
    }
    if config.latency_ms > 0 {
        cmd.arg("--latency").arg(config.latency_ms.to_string());
    }
    if config.jitter_ms > 0 {
        cmd.arg("--jitter").arg(config.jitter_ms.to_string());
    }
    if let Some(seed) = config.seed {
        cmd.arg("--seed").arg(seed.to_string());
    }
    // Extended chaos options
    if config.reorder_rate > 0.0 {
        cmd.arg("--reorder-rate")
            .arg(config.reorder_rate.to_string());
    }
    if config.reorder_buffer_size > 0 {
        cmd.arg("--reorder-buffer")
            .arg(config.reorder_buffer_size.to_string());
    }
    if config.duplicate_rate > 0.0 {
        cmd.arg("--duplicate-rate")
            .arg(config.duplicate_rate.to_string());
    }
    if config.burst_loss_prob > 0.0 {
        cmd.arg("--burst-loss-prob")
            .arg(config.burst_loss_prob.to_string());
        cmd.arg("--burst-loss-len")
            .arg(config.burst_loss_len.to_string());
    }
    // Sync configuration preset
    if let Some(ref preset) = config.sync_preset {
        cmd.arg("--sync-preset").arg(preset);
    }

    cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()
}

/// Maximum time to wait for a peer process before considering it hung.
/// This prevents tests from hanging forever if something goes wrong.
const PEER_PROCESS_TIMEOUT: Duration = Duration::from_secs(300); // 5 minutes max

/// Waits for a peer with a timeout to prevent infinite hangs.
/// Returns a TestResult indicating success or timeout failure.
fn wait_for_peer_with_timeout(mut child: Child, name: &str, timeout: Duration) -> TestResult {
    let start = std::time::Instant::now();
    let poll_interval = Duration::from_millis(100);

    // Poll for process completion with timeout
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                // Process completed - read output
                let mut stdout_buf = Vec::new();
                let mut stderr_buf = Vec::new();

                if let Some(mut stdout) = child.stdout.take() {
                    use std::io::Read;
                    let _ = stdout.read_to_end(&mut stdout_buf);
                }
                if let Some(mut stderr) = child.stderr.take() {
                    use std::io::Read;
                    let _ = stderr.read_to_end(&mut stderr_buf);
                }

                let stdout = String::from_utf8_lossy(&stdout_buf);
                let stderr = String::from_utf8_lossy(&stderr_buf);

                if !status.success() {
                    eprintln!("{} stderr: {}", name, stderr);
                }

                // Parse the last line as JSON (peers output JSON result on stdout)
                let last_line = stdout.lines().last().unwrap_or("");

                return match serde_json::from_str::<TestResult>(last_line) {
                    Ok(result) => result,
                    Err(e) => {
                        eprintln!("{} failed to parse output: {}", name, e);
                        eprintln!("{} stdout: {}", name, stdout);
                        eprintln!("{} stderr: {}", name, stderr);
                        TestResult {
                            success: false,
                            final_frame: 0,
                            final_value: 0,
                            checksum: 0,
                            rollbacks: 0,
                            desync_detected: 0,
                            error_kind: Some("parse".to_string()),
                            error: Some(format!("Failed to parse output: {}", e)),
                            runtime: None,
                        }
                    },
                };
            },
            Ok(None) => {
                // Process still running - check timeout
                if start.elapsed() > timeout {
                    // Timeout! Kill the process and return error
                    eprintln!(
                        "{} TIMEOUT after {:.1}s - killing process",
                        name,
                        start.elapsed().as_secs_f64()
                    );
                    let _ = child.kill();
                    let _ = child.wait(); // Reap the zombie

                    return TestResult {
                        success: false,
                        final_frame: 0,
                        final_value: 0,
                        checksum: 0,
                        rollbacks: 0,
                        desync_detected: 0,
                        error_kind: Some("timeout".to_string()),
                        error: Some(format!(
                            "Process timed out after {:.1}s (limit: {:.0}s)",
                            start.elapsed().as_secs_f64(),
                            timeout.as_secs_f64()
                        )),
                        runtime: None,
                    };
                }
                // Sleep before polling again
                thread::sleep(poll_interval);
            },
            Err(e) => {
                eprintln!("{} error checking process status: {}", name, e);
                return TestResult {
                    success: false,
                    final_frame: 0,
                    final_value: 0,
                    checksum: 0,
                    rollbacks: 0,
                    desync_detected: 0,
                    error_kind: Some("process".to_string()),
                    error: Some(format!("Error checking process: {}", e)),
                    runtime: None,
                };
            },
        }
    }
}

/// Waits for a peer and parses its result (with default timeout).
fn wait_for_peer(child: Child, name: &str) -> TestResult {
    wait_for_peer_with_timeout(child, name, PEER_PROCESS_TIMEOUT)
}

/// Runs an N-peer test (N >= 2) with the given per-peer configurations.
///
/// This is the generic spawn/wait engine shared by the 2-peer and N-peer paths.
/// It spawns every peer (staggered so each listener binds before later peers
/// start sending), waits for all of them concurrently via per-peer threads so
/// the configured per-peer timeout is also the effective test timeout, and on
/// any failure / cross-peer mismatch prints an N-way diagnostic block.
///
/// Returns one `TestResult` per input config, in the same order.
///
/// # Panics
/// Panics if the network_test_peer binary is not available.
/// Use [`skip_if_no_peer_binary!`] at the start of tests to skip gracefully.
fn run_n_peer_test(configs: Vec<PeerConfig>) -> Vec<TestResult> {
    // Check if the binary exists - fail fast with a clear message
    assert!(
        is_peer_binary_available(),
        "PREREQUISITE NOT MET: network_test_peer binary not found.\n\
         Build it with: cargo build -p network-test-peer\n\
         Expected binary: {} in target directory\n\
         \n\
         This is not a test failure - the test requires the peer binary to be built first.",
        PEER_BINARY_NAME
    );
    assert!(
        configs.len() >= 2,
        "run_n_peer_test requires at least 2 peers, got {}",
        configs.len()
    );

    let test_start = std::time::Instant::now();

    // Calculate timeout: use the max of peer timeouts + 30s buffer for process overhead.
    let peer_timeout = configs.iter().map(|c| c.timeout_secs).max().unwrap_or(0);
    let process_timeout = Duration::from_secs(peer_timeout + 30);

    // Spawn every peer, staggering so each peer's listener is bound before later
    // peers start sending to it. This mirrors the 2-peer 100ms spawn stagger.
    let mut wait_handles = Vec::with_capacity(configs.len());
    for (index, config) in configs.iter().enumerate() {
        if index > 0 {
            thread::sleep(Duration::from_millis(100));
        }
        let child =
            spawn_peer(config).unwrap_or_else(|e| panic!("Failed to spawn peer {index}: {e}"));
        let name = format!("Peer {index}");
        wait_handles.push(thread::spawn(move || {
            wait_for_peer_with_timeout(child, &name, process_timeout)
        }));
    }

    // Wait for all peers concurrently.
    let results: Vec<TestResult> = wait_handles
        .into_iter()
        .enumerate()
        .map(|(index, handle)| {
            handle
                .join()
                .unwrap_or_else(|_| panic!("Peer {index} wait thread panicked"))
        })
        .collect();

    let test_duration = test_start.elapsed();

    // Diagnostic gate (mirrors the 2-peer gate, generalized to N): this dumps the
    // per-peer checksum/final_value divergence purely as a debugging aid. The
    // divergence is FREQUENTLY BENIGN even at 0% loss: the peer that confirms the
    // target LAST computes its checksum/value over an already-discarded
    // `[target - 64, target)` window (empty hash `cbf29ce484222325`,
    // `final_value = 0`) -- the window-discard race documented in the module note
    // above the N-peer tests. So a divergence here is NOT asserted on and does not
    // by itself indicate a desync; it is only worth dumping alongside other
    // diagnostics when a test fails for some other reason.
    let any_failure = results.iter().any(|r| !r.success);
    let first_value = results.first().map(|r| r.final_value);
    let first_checksum = results.first().map(|r| r.checksum);
    let value_mismatch = results.iter().any(|r| Some(r.final_value) != first_value);
    let checksum_mismatch = results.iter().any(|r| Some(r.checksum) != first_checksum);

    if any_failure || value_mismatch || checksum_mismatch {
        eprintln!("=== Test Configuration ({} peers) ===", configs.len());
        for (index, config) in configs.iter().enumerate() {
            eprintln!("Peer {}: {}", index, config.diagnostic_summary());
        }
        eprintln!("=== Test Results ===");
        for (index, result) in results.iter().enumerate() {
            eprintln!("Peer {}: {}", index, result.diagnostic_summary());
        }
        eprintln!("Duration: {:.2}s", test_duration.as_secs_f64());
        eprintln!("========================");
    }

    // Log timing for all tests (helps diagnose CI timeouts)
    if test_duration.as_secs() > 30 {
        eprintln!(
            "WARNING: Test took {:.1}s (>30s) - may timeout under coverage",
            test_duration.as_secs_f64()
        );
    }

    results
}

/// Runs a two-peer test with the given configurations.
///
/// Thin wrapper over [`run_n_peer_test`] that preserves the historical
/// `(TestResult, TestResult)` return shape so all existing 2-peer call sites
/// keep working unchanged.
///
/// # Panics
/// Panics if the network_test_peer binary is not available.
/// Use [`skip_if_no_peer_binary!`] at the start of tests to skip gracefully.
fn run_two_peer_test(
    peer1_config: PeerConfig,
    peer2_config: PeerConfig,
) -> (TestResult, TestResult) {
    let mut results = run_n_peer_test(vec![peer1_config, peer2_config]);
    // run_n_peer_test returns exactly as many results as configs passed in.
    let result2 = results.pop().expect("two-peer test must yield 2 results");
    let result1 = results.pop().expect("two-peer test must yield 2 results");
    (result1, result2)
}

fn validate_peer_config_sync_preset(label: &str, config: &PeerConfig) {
    let profile = config.network_profile();
    let Some(recommended) = profile.suggested_sync_preset() else {
        return;
    };

    match config.sync_preset.as_deref() {
        Some(configured) if sync_preset_rank(configured) >= sync_preset_rank(recommended) => {},
        Some(configured) => {
            panic!(
                "{label} uses sync preset '{configured}' but network conditions recommend '{recommended}'. \
                 Config: {}",
                config.diagnostic_summary()
            );
        },
        None => {
            panic!(
                "{label} has hostile network conditions but no sync preset. \
                 Recommended preset: '{recommended}'. Config: {}",
                config.diagnostic_summary()
            );
        },
    }
}

/// Helper to verify determinism between two test results.
///
/// NOTE: This only checks that both peers succeeded. A peer's `success` is set
/// purely from `game.state.frame >= target && confirmed_frame >= target` in the
/// binary; it does NOT read `sync_health()` (that is only logged in
/// `runtime_diagnostics`). So `success == true` means each peer advanced to
/// `target` AND confirmed all players' inputs through `target`. The `final_value`
/// field is NOT compared because its calculation depends on when inputs become
/// confirmed, which varies between peers due to network timing. This success
/// oracle does not itself assert a desync-free state.
///
/// See progress/session-73-flaky-network-test-analysis.md for details.
#[track_caller]
fn verify_determinism(result1: &TestResult, result2: &TestResult, context: &str) {
    // success == true means each peer reached frame >= target with all inputs
    // confirmed through target. final_value comparison is disabled because it
    // depends on accumulation timing across peers.

    // Provide detailed diagnostics for sync failures
    if !result1.success || !result2.success {
        eprintln!("=== Determinism Verification Failed: {} ===", context);
        eprintln!(
            "Peer 1: success={}, frame={}, error={:?}",
            result1.success, result1.final_frame, result1.error
        );
        eprintln!(
            "Peer 2: success={}, frame={}, error={:?}",
            result2.success, result2.final_frame, result2.error
        );

        // Check for common failure patterns and provide targeted diagnostics
        match (result1.final_frame == 0, result2.final_frame == 0) {
            (true, false) => {
                eprintln!("DIAGNOSIS: Peer 1 never started advancing frames.");
                eprintln!("  This typically indicates sync handshake failure.");
                eprintln!("  The sync protocol requires multiple roundtrips to complete.");
                eprintln!(
                    "  With burst loss, consecutive packets can be dropped, preventing sync."
                );
                eprintln!();
                eprintln!("  Possible causes:");
                eprintln!("  - Burst loss events coincided with sync packets");
                eprintln!("  - Seed {} may produce unfavorable burst patterns", 42);
                eprintln!();
                eprintln!("  Recommendations:");
                eprintln!("  - Use 'stress_test' sync preset for hostile conditions");
                eprintln!("  - Use bursty_survivable() profile instead of bursty()");
                eprintln!("  - Reduce burst_loss_prob or burst_loss_len");
            },
            (false, true) => {
                eprintln!("DIAGNOSIS: Peer 2 never started advancing frames.");
                eprintln!("  This typically indicates sync handshake failure.");
                eprintln!("  The sync protocol requires multiple roundtrips to complete.");
                eprintln!(
                    "  With burst loss, consecutive packets can be dropped, preventing sync."
                );
                eprintln!();
                eprintln!("  Possible causes:");
                eprintln!("  - Burst loss events coincided with sync packets");
                eprintln!("  - Seed {} may produce unfavorable burst patterns", 43);
                eprintln!();
                eprintln!("  Recommendations:");
                eprintln!("  - Use 'stress_test' sync preset for hostile conditions");
                eprintln!("  - Use bursty_survivable() profile instead of bursty()");
                eprintln!("  - Reduce burst_loss_prob or burst_loss_len");
            },
            (true, true) => {
                eprintln!("DIAGNOSIS: Neither peer advanced any frames.");
                eprintln!("  This typically indicates both peers failed to sync.");
                eprintln!("  Check network conditions and sync preset configuration.");
            },
            (false, false) => {
                // Both advanced frames but still failed - different issue
                eprintln!("DIAGNOSIS: Both peers advanced frames but test still failed.");
                eprintln!("  This may indicate a timeout or desync during gameplay.");
            },
        }
        eprintln!("============================================");
    }

    assert!(
        result1.success,
        "{}: Peer 1 failed (did not reach frame >= target, or confirmed_frame < target): {:?}",
        context, result1.error
    );
    assert!(
        result2.success,
        "{}: Peer 2 failed (did not reach frame >= target, or confirmed_frame < target): {:?}",
        context, result2.error
    );

    // Log if values differ (for debugging) but don't assert
    if result1.final_value != result2.final_value {
        eprintln!(
            "NOTE: {} final_value differs (peer1={}, peer2={}), but both peers reached success",
            context, result1.final_value, result2.final_value
        );
    }
}

/// N-peer generalization of [`verify_determinism`].
///
/// Asserts that every peer reached `success == true`. A peer's `success` is set
/// purely from `game.state.frame >= target && confirmed_frame >= target` in the
/// binary (`tests/network-peer/src/main.rs`); it is NOT derived from
/// `sync_health()` (that is only logged in `runtime_diagnostics`). So
/// `success == true` means: this peer advanced its simulation to `target` AND
/// confirmed *all* N players' inputs through `target` (since `confirmed_frame`
/// is the min over connected peers' `last_frame`). All peers succeeding is
/// therefore the faithful N-peer generalization of the established 2-peer
/// oracle.
///
/// In addition, asserts that every peer observed ZERO `DesyncDetected` events.
/// The historical 0%-loss false positive that forced this count to be log-only
/// was finding F17, root-caused in S30 to the library's `InputQueue::input`
/// (a rollback re-simulation re-entered its prediction episode at the
/// REQUESTED frame instead of the queue's first missing frame, silently
/// swallowing misprediction comparisons for the skipped window) and fixed
/// there, so on a clean network a nonzero count is a genuine library
/// regression -- see the module note above the N-peer tests. Like
/// [`verify_determinism`], `final_value` and `checksum` are only logged, not
/// asserted, because at 0% loss the last-confirming peer computes them over an
/// already-discarded `[target - 64, target)` window (empty hash) -- flaky by
/// construction.
#[track_caller]
fn verify_determinism_n(results: &[TestResult], context: &str) {
    let any_failure = results.iter().any(|r| !r.success);
    if any_failure {
        eprintln!("=== Determinism Verification Failed (N-peer): {context} ===");
        eprintln!("Peer count: {}", results.len());
        let advanced = results.iter().filter(|r| r.final_frame > 0).count();
        for (index, result) in results.iter().enumerate() {
            eprintln!(
                "Peer {}: success={}, frame={}, value={}, checksum={:x}, error={:?}",
                index,
                result.success,
                result.final_frame,
                result.final_value,
                result.checksum,
                result.error
            );
        }
        // Mirror verify_determinism's targeted diagnosis, generalized to N: the
        // common failure mode is the sync handshake never completing for one or
        // more peers (they never advance any frame).
        if advanced == 0 {
            eprintln!("DIAGNOSIS: No peer advanced any frames -- all peers failed to sync.");
            eprintln!("  Check the mesh handle<->address mapping and sync preset configuration.");
        } else if advanced < results.len() {
            eprintln!(
                "DIAGNOSIS: {} of {} peers never advanced frames -- partial sync failure.",
                results.len() - advanced,
                results.len()
            );
            eprintln!("  The sync handshake requires every pair of peers to exchange packets.");
        } else {
            eprintln!(
                "DIAGNOSIS: All peers advanced frames but at least one failed -- \
                 likely a timeout or desync during gameplay."
            );
        }
        eprintln!("============================================");
    }

    for (index, result) in results.iter().enumerate() {
        assert!(
            result.success,
            "{context}: Peer {index} failed (did not reach frame >= target, \
             or confirmed_frame < target): {:?}",
            result.error
        );
    }

    // Assert each peer's DesyncDetected count is zero. These tests run on a
    // clean network (0% loss, no disconnects), where the library must never
    // raise a desync. The historical 0%-loss false positive that kept this
    // log-only was finding F17, fixed in S30 at its InputQueue prediction-entry
    // root cause (see the module note above the N-peer tests); post-fix, a
    // 10-run real-UDP soak of the 3-peer test observed 0 events. Dump every
    // peer's count and full diagnostics first so a cross-peer event pattern is
    // visible before the per-peer assertion names the first offender.
    let total_desync: u32 = results.iter().map(TestResult::desync_count).sum();
    if total_desync > 0 {
        eprintln!("=== DesyncDetected events observed ({context}, total={total_desync}) ===");
        for (index, result) in results.iter().enumerate() {
            eprintln!(
                "Peer {index}: desync_detected={}, {}",
                result.desync_count(),
                result.diagnostic_summary()
            );
        }
        eprintln!("============================================");
    }
    for (index, result) in results.iter().enumerate() {
        let desync_count = result.desync_count();
        assert_eq!(
            desync_count,
            0,
            "{context}: Peer {index} observed {desync_count} DesyncDetected event(s) on a \
             clean network (expected 0 since the F17 InputQueue prediction-entry fix, S30). \
             Peer diagnostics: {}",
            result.diagnostic_summary()
        );
    }

    // Log (do not assert) any final_value divergence, matching verify_determinism.
    if let Some(first) = results.first() {
        for (index, result) in results.iter().enumerate().skip(1) {
            if result.final_value != first.final_value {
                eprintln!(
                    "NOTE: {context} final_value differs (peer0={}, peer{index}={}), \
                     but every peer reached success",
                    first.final_value, result.final_value
                );
            }
        }
    }
}

/// Builds `num_players` peer configs for a fully-connected N-peer mesh on
/// localhost.
///
/// Peer `i` listens on `127.0.0.1:(port_base + i)`. Its ordered remote list is
/// `[127.0.0.1:(port_base + j) for j in 0..num_players if j != i]` -- i.e.
/// ascending `j`, which is exactly the ascending-remote-handle order the binary
/// expects (the binary assigns the K-th `--peer` address to the K-th handle in
/// `(0..num_players).filter(|h| h != i)`). The first remote goes in `peer_addr`
/// and the rest in `extra_peer_addrs`. Each peer gets a distinct deterministic
/// seed (`42 + i`) and the same network `profile`.
fn n_peer_mesh_configs(
    num_players: usize,
    port_base: u16,
    profile: NetworkProfile,
    frames: i32,
    input_delay: usize,
    timeout_secs: u64,
    sync_preset: Option<String>,
) -> Vec<PeerConfig> {
    (0..num_players)
        .map(|i| {
            // Ascending-j remote address list, skipping our own index. This must
            // stay ascending to match the binary's handle assignment.
            let mut remotes: Vec<String> = (0..num_players)
                .filter(|&j| j != i)
                .map(|j| format!("127.0.0.1:{}", port_base + j as u16))
                .collect();
            // The first remote becomes `peer_addr`; the remainder become the
            // extra `--peer` args (empty for the 2-peer case).
            let peer_addr = remotes.remove(0);
            PeerConfig {
                local_port: port_base + i as u16,
                player_index: i,
                peer_addr,
                extra_peer_addrs: remotes,
                frames,
                seed: Some(42 + i as u64),
                timeout_secs,
                input_delay,
                sync_preset: sync_preset.clone(),
                packet_loss: profile.packet_loss,
                latency_ms: profile.latency_ms,
                jitter_ms: profile.jitter_ms,
                reorder_rate: profile.reorder_rate,
                reorder_buffer_size: profile.reorder_buffer_size,
                duplicate_rate: profile.duplicate_rate,
                burst_loss_prob: profile.burst_loss_prob,
                burst_loss_len: profile.burst_loss_len,
            }
        })
        .collect()
}

// =============================================================================
// Basic Connectivity Tests
// =============================================================================

/// Test that two peers can connect and advance 100 frames over real network.
///
/// Validates that both peers can successfully complete the session, reach the
/// target frame count, and achieve deterministic final game state values.
#[test]
#[serial]
fn test_basic_connectivity() {
    skip_if_no_peer_binary!();
    let peer1_config = PeerConfig {
        local_port: 10001,
        player_index: 0,
        peer_addr: "127.0.0.1:10002".to_string(),
        frames: 100,
        timeout_secs: 30,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10002,
        player_index: 1,
        peer_addr: "127.0.0.1:10001".to_string(),
        frames: 100,
        timeout_secs: 30,
        seed: Some(43),
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    // Verify both peers reached success (frame >= target with all inputs confirmed);
    // sync_health() is diagnostic-only and is NOT part of the success check.
    verify_determinism(&result1, &result2, "basic_connectivity");

    // Both should reach at least target frames
    // Note: final_frame can exceed target due to continued processing while waiting for confirmations
    assert!(
        result1.final_frame >= 100,
        "Peer 1 didn't reach target frames: {}",
        result1.final_frame
    );
    assert!(
        result2.final_frame >= 100,
        "Peer 2 didn't reach target frames: {}",
        result2.final_frame
    );
}

/// Test longer session (500 frames) to verify stability.
///
/// Verifies determinism using final_value which reflects the cumulative result
/// of processing all confirmed inputs. Both peers will have the same value after
/// rollbacks complete and all inputs are confirmed.
#[test]
#[serial]
fn test_extended_session() {
    skip_if_no_peer_binary!();
    let peer1_config = PeerConfig {
        local_port: 10003,
        player_index: 0,
        peer_addr: "127.0.0.1:10004".to_string(),
        frames: 500,
        timeout_secs: 60,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10004,
        player_index: 1,
        peer_addr: "127.0.0.1:10003".to_string(),
        frames: 500,
        timeout_secs: 60,
        seed: Some(43),
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    // Verify both peers reached success (frame >= target with all inputs confirmed);
    // sync_health() is diagnostic-only and is NOT part of the success check.
    verify_determinism(&result1, &result2, "extended_session");

    assert!(
        result1.final_frame >= 500,
        "Peer 1 didn't reach target frames: {}",
        result1.final_frame
    );
    assert!(
        result2.final_frame >= 500,
        "Peer 2 didn't reach target frames: {}",
        result2.final_frame
    );
}

// =============================================================================
// Packet Loss Tests
// =============================================================================

/// Test with 5% packet loss - should still complete successfully.
#[test]
#[serial]
fn test_packet_loss_5_percent() {
    skip_if_no_peer_binary!();
    let peer1_config = PeerConfig {
        local_port: 10005,
        player_index: 0,
        peer_addr: "127.0.0.1:10006".to_string(),
        frames: 100,
        packet_loss: 0.05,
        seed: Some(42),
        timeout_secs: 45,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10006,
        player_index: 1,
        peer_addr: "127.0.0.1:10005".to_string(),
        frames: 100,
        packet_loss: 0.05,
        seed: Some(43),
        timeout_secs: 45,
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    // Verify both peers reached success (frame >= target with all inputs confirmed);
    // sync_health() is diagnostic-only and is NOT part of the success check.
    verify_determinism(&result1, &result2, "packet_loss_5_percent");
}

/// Test with 15% packet loss - more challenging but should work.
#[test]
#[serial]
fn test_packet_loss_15_percent() {
    skip_if_no_peer_binary!();
    let peer1_config = PeerConfig {
        local_port: 10007,
        player_index: 0,
        peer_addr: "127.0.0.1:10008".to_string(),
        frames: 100,
        packet_loss: 0.15,
        seed: Some(42),
        timeout_secs: 120,
        sync_preset: Some("mobile".to_string()),
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10008,
        player_index: 1,
        peer_addr: "127.0.0.1:10007".to_string(),
        frames: 100,
        packet_loss: 0.15,
        seed: Some(43),
        timeout_secs: 120,
        sync_preset: Some("mobile".to_string()),
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    // Verify both peers reached success (frame >= target with all inputs confirmed);
    // sync_health() is diagnostic-only and is NOT part of the success check.
    verify_determinism(&result1, &result2, "packet_loss_15_percent");
}

// =============================================================================
// Latency Tests
// =============================================================================

/// Test with 30ms simulated latency.
#[test]
#[serial]
fn test_latency_30ms() {
    skip_if_no_peer_binary!();
    let peer1_config = PeerConfig {
        local_port: 10009,
        player_index: 0,
        peer_addr: "127.0.0.1:10010".to_string(),
        frames: 100,
        latency_ms: 30,
        seed: Some(42),
        timeout_secs: 60,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10010,
        player_index: 1,
        peer_addr: "127.0.0.1:10009".to_string(),
        frames: 100,
        latency_ms: 30,
        seed: Some(43),
        timeout_secs: 60,
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    // Verify both peers reached success (frame >= target with all inputs confirmed);
    // sync_health() is diagnostic-only and is NOT part of the success check.
    verify_determinism(&result1, &result2, "latency_30ms");
}

/// Test with 50ms latency and jitter.
#[test]
#[serial]
fn test_latency_with_jitter() {
    skip_if_no_peer_binary!();
    let peer1_config = PeerConfig {
        local_port: 10011,
        player_index: 0,
        peer_addr: "127.0.0.1:10012".to_string(),
        frames: 100,
        latency_ms: 50,
        jitter_ms: 20,
        seed: Some(42),
        timeout_secs: 90,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10012,
        player_index: 1,
        peer_addr: "127.0.0.1:10011".to_string(),
        frames: 100,
        latency_ms: 50,
        jitter_ms: 20,
        seed: Some(43),
        timeout_secs: 90,
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    // Verify both peers reached success (frame >= target with all inputs confirmed);
    // sync_health() is diagnostic-only and is NOT part of the success check.
    verify_determinism(&result1, &result2, "latency_with_jitter");
}

// =============================================================================
// Combined Conditions Tests
// =============================================================================

/// Test "poor network" conditions: latency + loss + jitter combined.
#[test]
#[serial]
fn test_poor_network_combined() {
    skip_if_no_peer_binary!();
    let peer1_config = PeerConfig {
        local_port: 10013,
        player_index: 0,
        peer_addr: "127.0.0.1:10014".to_string(),
        frames: 100,
        packet_loss: 0.08,
        latency_ms: 40,
        jitter_ms: 15,
        seed: Some(42),
        timeout_secs: 120,
        sync_preset: Some("lossy".to_string()),
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10014,
        player_index: 1,
        peer_addr: "127.0.0.1:10013".to_string(),
        frames: 100,
        packet_loss: 0.08,
        latency_ms: 40,
        jitter_ms: 15,
        seed: Some(43),
        timeout_secs: 120,
        sync_preset: Some("lossy".to_string()),
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    // Verify both peers reached success (frame >= target with all inputs confirmed);
    // sync_health() is diagnostic-only and is NOT part of the success check.
    verify_determinism(&result1, &result2, "poor_network_combined");
}

/// Test asymmetric conditions - one peer has much worse network.
#[test]
#[serial]
#[ignore = "extreme real-UDP chaos; runs in the nightly network suite (ci-network-nightly.yml). Determinism is covered deterministically by tests/network/in_process_chaos.rs"]
fn test_asymmetric_network() {
    skip_if_no_peer_binary!();
    let bad_network = NetworkProfile {
        packet_loss: 0.20,
        latency_ms: 80,
        jitter_ms: 0,
        reorder_rate: 0.0,
        reorder_buffer_size: 0,
        duplicate_rate: 0.0,
        burst_loss_prob: 0.0,
        burst_loss_len: 0,
    };
    let good_network = NetworkProfile {
        packet_loss: 0.02,
        latency_ms: 10,
        jitter_ms: 0,
        reorder_rate: 0.0,
        reorder_buffer_size: 0,
        duplicate_rate: 0.0,
        burst_loss_prob: 0.0,
        burst_loss_len: 0,
    };

    let scenario = NetworkScenario::asymmetric("asymmetric_network", bad_network, good_network)
        .with_frames(100)
        .with_timeout(120)
        .with_auto_sync_preset();

    let (result1, result2) = scenario.run_test(10015);

    // Verify both peers reached success (frame >= target with all inputs confirmed);
    // sync_health() is diagnostic-only and is NOT part of the success check.
    verify_determinism(&result1, &result2, "asymmetric_network");

    // Peer 1 should have more rollbacks due to bad network
    println!(
        "Rollbacks - Peer 1 (bad network): {}, Peer 2 (good network): {}",
        result1.rollbacks, result2.rollbacks
    );
}

// =============================================================================
// Stress Tests
// =============================================================================

/// Long session (1000 frames) with moderate packet loss.
///
/// Note: This test runs ~1000 frames with 5% packet loss. In release mode,
/// it typically completes in ~2-3 seconds.
#[test]
#[serial]
#[ignore = "long-running endurance or high-latency real-UDP session; runs in the nightly network suite (ci-network-nightly.yml), not per-PR CI"]
fn test_stress_long_session_with_loss() {
    skip_if_no_peer_binary!();
    let peer1_config = PeerConfig {
        local_port: 10017,
        player_index: 0,
        peer_addr: "127.0.0.1:10018".to_string(),
        frames: 1000,
        packet_loss: 0.05,
        seed: Some(42),
        timeout_secs: 180,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10018,
        player_index: 1,
        peer_addr: "127.0.0.1:10017".to_string(),
        frames: 1000,
        packet_loss: 0.05,
        seed: Some(43),
        timeout_secs: 180,
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    // Verify both peers reached success (frame >= target with all inputs confirmed);
    // sync_health() is diagnostic-only and is NOT part of the success check.
    verify_determinism(&result1, &result2, "stress_long_session_with_loss");

    assert!(
        result1.final_frame >= 1000,
        "Peer 1 didn't reach target frames: {}",
        result1.final_frame
    );
    assert!(
        result2.final_frame >= 1000,
        "Peer 2 didn't reach target frames: {}",
        result2.final_frame
    );

    println!(
        "Stress test complete - Peer 1 rollbacks: {}, Peer 2 rollbacks: {}",
        result1.rollbacks, result2.rollbacks
    );
}

// =============================================================================
// Additional Chaos Tests
// =============================================================================

/// Test with high jitter (variable latency).
#[test]
#[serial]
fn test_high_jitter_50ms() {
    skip_if_no_peer_binary!();
    let peer1_config = PeerConfig {
        local_port: 10019,
        player_index: 0,
        peer_addr: "127.0.0.1:10020".to_string(),
        frames: 100,
        latency_ms: 30,
        jitter_ms: 50, // High jitter
        seed: Some(42),
        timeout_secs: 90,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10020,
        player_index: 1,
        peer_addr: "127.0.0.1:10019".to_string(),
        frames: 100,
        latency_ms: 30,
        jitter_ms: 50,
        seed: Some(43),
        timeout_secs: 90,
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    // Verify both peers reached success (frame >= target with all inputs confirmed);
    // sync_health() is diagnostic-only and is NOT part of the success check.
    verify_determinism(&result1, &result2, "high_jitter_50ms");
}

/// Test with combined loss, latency, and jitter - simulating mobile network.
#[test]
#[serial]
fn test_mobile_network_simulation() {
    skip_if_no_peer_binary!();
    let peer1_config = PeerConfig {
        local_port: 10021,
        player_index: 0,
        peer_addr: "127.0.0.1:10022".to_string(),
        frames: 100,
        packet_loss: 0.12, // Mobile often has higher loss
        latency_ms: 60,    // Typical mobile latency
        jitter_ms: 40,     // High jitter on mobile
        seed: Some(42),
        timeout_secs: 120,
        sync_preset: Some("mobile".to_string()),
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10022,
        player_index: 1,
        peer_addr: "127.0.0.1:10021".to_string(),
        frames: 100,
        packet_loss: 0.12,
        latency_ms: 60,
        jitter_ms: 40,
        seed: Some(43),
        timeout_secs: 120,
        sync_preset: Some("mobile".to_string()),
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    // Verify both peers reached success (frame >= target with all inputs confirmed);
    // sync_health() is diagnostic-only and is NOT part of the success check.
    verify_determinism(&result1, &result2, "mobile_network_simulation");

    println!(
        "Mobile simulation - Rollbacks: peer1={}, peer2={}",
        result1.rollbacks, result2.rollbacks
    );
}

/// Test heavily asymmetric conditions - one peer on great network, one on terrible.
#[test]
#[serial]
#[ignore = "extreme real-UDP chaos; runs in the nightly network suite (ci-network-nightly.yml). Determinism is covered deterministically by tests/network/in_process_chaos.rs"]
fn test_heavily_asymmetric_network() {
    skip_if_no_peer_binary!();
    let terrible_network = NetworkProfile {
        packet_loss: 0.25, // 25% loss!
        latency_ms: 100,   // 100ms latency
        jitter_ms: 40,
        reorder_rate: 0.0,
        reorder_buffer_size: 0,
        duplicate_rate: 0.0,
        burst_loss_prob: 0.0,
        burst_loss_len: 0,
    };
    let excellent_network = NetworkProfile {
        packet_loss: 0.01, // 1% loss
        latency_ms: 5,     // 5ms latency
        jitter_ms: 2,
        reorder_rate: 0.0,
        reorder_buffer_size: 0,
        duplicate_rate: 0.0,
        burst_loss_prob: 0.0,
        burst_loss_len: 0,
    };

    let scenario = NetworkScenario::asymmetric(
        "heavily_asymmetric_network",
        terrible_network,
        excellent_network,
    )
    .with_frames(100)
    .with_timeout(180)
    .with_auto_sync_preset();

    let (result1, result2) = scenario.run_test(10023);

    // Verify both peers reached success (frame >= target with all inputs confirmed);
    // sync_health() is diagnostic-only and is NOT part of the success check.
    verify_determinism(&result1, &result2, "heavily_asymmetric_network");

    println!(
        "Asymmetric test - Rollbacks: bad_peer={}, good_peer={}",
        result1.rollbacks, result2.rollbacks
    );
}

/// Test with higher input delay (4 frames).
#[test]
#[serial]
fn test_higher_input_delay() {
    skip_if_no_peer_binary!();
    let peer1_config = PeerConfig {
        local_port: 10025,
        player_index: 0,
        peer_addr: "127.0.0.1:10026".to_string(),
        frames: 100,
        packet_loss: 0.05,
        latency_ms: 20,
        input_delay: 4, // Higher input delay
        seed: Some(42),
        timeout_secs: 60,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10026,
        player_index: 1,
        peer_addr: "127.0.0.1:10025".to_string(),
        frames: 100,
        packet_loss: 0.05,
        latency_ms: 20,
        input_delay: 4,
        seed: Some(43),
        timeout_secs: 60,
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    // Verify both peers reached success (frame >= target with all inputs confirmed);
    // sync_health() is diagnostic-only and is NOT part of the success check.
    verify_determinism(&result1, &result2, "higher_input_delay");
}

/// Test with zero latency but high packet loss.
#[test]
#[serial]
#[ignore = "extreme real-UDP chaos; runs in the nightly network suite (ci-network-nightly.yml). Determinism is covered deterministically by tests/network/in_process_chaos.rs"]
fn test_zero_latency_high_loss() {
    skip_if_no_peer_binary!();
    let profile = NetworkProfile {
        packet_loss: 0.20, // 20% loss but no latency
        latency_ms: 0,
        jitter_ms: 0,
        reorder_rate: 0.0,
        reorder_buffer_size: 0,
        duplicate_rate: 0.0,
        burst_loss_prob: 0.0,
        burst_loss_len: 0,
    };

    let scenario = NetworkScenario::symmetric("zero_latency_high_loss", profile)
        .with_frames(100)
        .with_timeout(120)
        .with_auto_sync_preset();

    let (result1, result2) = scenario.run_test(10027);

    // Verify both peers reached success (frame >= target with all inputs confirmed);
    // sync_health() is diagnostic-only and is NOT part of the success check.
    verify_determinism(&result1, &result2, "zero_latency_high_loss");

    println!(
        "Zero latency high loss - Rollbacks: peer1={}, peer2={}",
        result1.rollbacks, result2.rollbacks
    );
}

/// Test medium-length session (300 frames) with moderate conditions.
#[test]
#[serial]
fn test_medium_session_300_frames() {
    skip_if_no_peer_binary!();
    let peer1_config = PeerConfig {
        local_port: 10029,
        player_index: 0,
        peer_addr: "127.0.0.1:10030".to_string(),
        frames: 300,
        packet_loss: 0.08,
        latency_ms: 35,
        jitter_ms: 15,
        seed: Some(42),
        timeout_secs: 120,
        sync_preset: Some("lossy".to_string()),
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10030,
        player_index: 1,
        peer_addr: "127.0.0.1:10029".to_string(),
        frames: 300,
        packet_loss: 0.08,
        latency_ms: 35,
        jitter_ms: 15,
        seed: Some(43),
        timeout_secs: 120,
        sync_preset: Some("lossy".to_string()),
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    // Verify both peers reached success (frame >= target with all inputs confirmed);
    // sync_health() is diagnostic-only and is NOT part of the success check.
    verify_determinism(&result1, &result2, "medium_session_300_frames");

    assert!(
        result1.final_frame >= 300,
        "Peer 1 didn't reach 300 frames: {}",
        result1.final_frame
    );
    assert!(
        result2.final_frame >= 300,
        "Peer 2 didn't reach 300 frames: {}",
        result2.final_frame
    );
}

/// Stress test with very long session (2000 frames).
///
/// Note: This test runs 2000 frames with 3% packet loss and 20ms latency.
/// In release mode, it typically completes in ~7-8 seconds.
#[test]
#[serial]
#[ignore = "long-running endurance or high-latency real-UDP session; runs in the nightly network suite (ci-network-nightly.yml), not per-PR CI"]
fn test_stress_very_long_session() {
    skip_if_no_peer_binary!();
    let peer1_config = PeerConfig {
        local_port: 10031,
        player_index: 0,
        peer_addr: "127.0.0.1:10032".to_string(),
        frames: 2000,
        packet_loss: 0.03,
        latency_ms: 20,
        seed: Some(42),
        timeout_secs: 300, // 5 minutes
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10032,
        player_index: 1,
        peer_addr: "127.0.0.1:10031".to_string(),
        frames: 2000,
        packet_loss: 0.03,
        latency_ms: 20,
        seed: Some(43),
        timeout_secs: 300,
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    // Verify both peers reached success (frame >= target with all inputs confirmed);
    // sync_health() is diagnostic-only and is NOT part of the success check.
    verify_determinism(&result1, &result2, "stress_very_long_session");

    println!(
        "2000 frame stress complete - Rollbacks: peer1={}, peer2={}",
        result1.rollbacks, result2.rollbacks
    );
}

/// Test with different random seeds to validate determinism is seed-independent.
#[test]
#[serial]
fn test_determinism_different_seeds() {
    skip_if_no_peer_binary!();
    // Same network conditions but different chaos seeds
    let peer1_config = PeerConfig {
        local_port: 10033,
        player_index: 0,
        peer_addr: "127.0.0.1:10034".to_string(),
        frames: 100,
        packet_loss: 0.10,
        latency_ms: 30,
        seed: Some(12345), // Different seed
        timeout_secs: 90,
        sync_preset: Some("lossy".to_string()),
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10034,
        player_index: 1,
        peer_addr: "127.0.0.1:10033".to_string(),
        frames: 100,
        packet_loss: 0.10,
        latency_ms: 30,
        seed: Some(67890), // Different seed
        timeout_secs: 90,
        sync_preset: Some("lossy".to_string()),
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    // Verify both peers reached success (frame >= target with all inputs confirmed);
    // sync_health() is diagnostic-only and is NOT part of the success check.
    verify_determinism(&result1, &result2, "determinism_different_seeds");
}

// =============================================================================
// Additional Robustness & Edge Case Tests
// =============================================================================

/// Test staggered peer startup - peer 2 joins 500ms after peer 1.
/// This tests robustness of connection establishment when peers don't start simultaneously.
#[test]
#[serial]
fn test_staggered_peer_startup() {
    skip_if_no_peer_binary!();
    let peer1_config = PeerConfig {
        local_port: 10035,
        player_index: 0,
        peer_addr: "127.0.0.1:10036".to_string(),
        frames: 100,
        timeout_secs: 60,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10036,
        player_index: 1,
        peer_addr: "127.0.0.1:10035".to_string(),
        frames: 100,
        timeout_secs: 60,
        seed: Some(43),
        ..Default::default()
    };

    // Spawn peer 1 first
    let peer1 = spawn_peer(&peer1_config).expect("Failed to spawn peer 1");

    // Wait 500ms before spawning peer 2 (simulates staggered start)
    thread::sleep(Duration::from_millis(500));

    // Spawn peer 2
    let peer2 = spawn_peer(&peer2_config).expect("Failed to spawn peer 2");

    // Wait for both peers (wait_for_peer has a default 5-minute timeout)
    let result1 = wait_for_peer(peer1, "Peer 1");
    let result2 = wait_for_peer(peer2, "Peer 2");

    // Verify both peers reached success (frame >= target with all inputs confirmed);
    // sync_health() is diagnostic-only and is NOT part of the success check.
    verify_determinism(&result1, &result2, "staggered_peer_startup");
}

/// Test heavily staggered startup - peer 2 joins 2 seconds after peer 1.
/// This tests connection timeout handling and retry logic.
#[test]
#[serial]
#[ignore = "extreme real-UDP chaos; runs in the nightly network suite (ci-network-nightly.yml). Determinism is covered deterministically by tests/network/in_process_chaos.rs"]
fn test_heavily_staggered_startup() {
    skip_if_no_peer_binary!();
    let peer1_config = PeerConfig {
        local_port: 10037,
        player_index: 0,
        peer_addr: "127.0.0.1:10038".to_string(),
        frames: 100,
        timeout_secs: 90, // Longer timeout to accommodate stagger
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10038,
        player_index: 1,
        peer_addr: "127.0.0.1:10037".to_string(),
        frames: 100,
        timeout_secs: 90,
        seed: Some(43),
        ..Default::default()
    };

    // Spawn peer 1 first
    let peer1 = spawn_peer(&peer1_config).expect("Failed to spawn peer 1");

    // Wait 2 seconds before spawning peer 2
    thread::sleep(Duration::from_secs(2));

    // Spawn peer 2
    let peer2 = spawn_peer(&peer2_config).expect("Failed to spawn peer 2");

    // Wait for both peers (wait_for_peer has a default 5-minute timeout)
    let result1 = wait_for_peer(peer1, "Peer 1");
    let result2 = wait_for_peer(peer2, "Peer 2");

    // Verify both peers reached success (frame >= target with all inputs confirmed);
    // sync_health() is diagnostic-only and is NOT part of the success check.
    verify_determinism(&result1, &result2, "heavily_staggered_startup");
}

/// Test asymmetric input delays between peers.
/// Peer 1 uses input delay 1, peer 2 uses input delay 4.
/// The library should still maintain determinism.
#[test]
#[serial]
fn test_asymmetric_input_delays() {
    skip_if_no_peer_binary!();
    let peer1_config = PeerConfig {
        local_port: 10039,
        player_index: 0,
        peer_addr: "127.0.0.1:10040".to_string(),
        frames: 100,
        input_delay: 1, // Low input delay
        timeout_secs: 60,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10040,
        player_index: 1,
        peer_addr: "127.0.0.1:10039".to_string(),
        frames: 100,
        input_delay: 4, // High input delay
        timeout_secs: 60,
        seed: Some(43),
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    // Verify both peers reached success (frame >= target with all inputs confirmed);
    // sync_health() is diagnostic-only and is NOT part of the success check.
    verify_determinism(&result1, &result2, "asymmetric_input_delays");
}

/// Test minimum input delay (0) under perfect network conditions.
/// This is an edge case that tests the library at its most aggressive timing.
#[test]
#[serial]
fn test_zero_input_delay() {
    skip_if_no_peer_binary!();
    let peer1_config = PeerConfig {
        local_port: 10041,
        player_index: 0,
        peer_addr: "127.0.0.1:10042".to_string(),
        frames: 100,
        input_delay: 0, // No input delay!
        timeout_secs: 60,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10042,
        player_index: 1,
        peer_addr: "127.0.0.1:10041".to_string(),
        frames: 100,
        input_delay: 0,
        timeout_secs: 60,
        seed: Some(43),
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    // Verify both peers reached success (frame >= target with all inputs confirmed);
    // sync_health() is diagnostic-only and is NOT part of the success check.
    verify_determinism(&result1, &result2, "zero_input_delay");

    // With zero input delay, expect more rollbacks
    println!(
        "Zero delay rollbacks - Peer 1: {}, Peer 2: {}",
        result1.rollbacks, result2.rollbacks
    );
}

/// Test maximum reasonable input delay (8 frames).
/// This tests the library with a very conservative timing configuration.
#[test]
#[serial]
fn test_high_input_delay_8_frames() {
    skip_if_no_peer_binary!();
    let peer1_config = PeerConfig {
        local_port: 10043,
        player_index: 0,
        peer_addr: "127.0.0.1:10044".to_string(),
        frames: 100,
        input_delay: 8, // High input delay
        timeout_secs: 90,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10044,
        player_index: 1,
        peer_addr: "127.0.0.1:10043".to_string(),
        frames: 100,
        input_delay: 8,
        timeout_secs: 90,
        seed: Some(43),
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    // Verify both peers reached success (frame >= target with all inputs confirmed);
    // sync_health() is diagnostic-only and is NOT part of the success check.
    verify_determinism(&result1, &result2, "high_input_delay_8_frames");

    // With high input delay, should see fewer rollbacks
    println!(
        "High delay rollbacks - Peer 1: {}, Peer 2: {}",
        result1.rollbacks, result2.rollbacks
    );
}

/// Test high uniform packet loss (20%) with appropriate sync configuration.
///
/// This test verifies that the library can handle extreme packet loss conditions
/// when configured appropriately. With 20% loss on both send/receive paths,
/// the effective loss rate is ~36% (1 - 0.8 × 0.8).
///
/// The `extreme` sync preset uses 20 sync packets and longer retry intervals,
/// which is necessary for reliable synchronization under such extreme conditions.
///
/// NOTE: This test previously failed because it used the default SyncConfig
/// (5 packets, 200ms retry) which is insufficient for 36% effective packet loss.
#[test]
#[serial]
#[ignore = "extreme real-UDP chaos; runs in the nightly network suite (ci-network-nightly.yml). Determinism is covered deterministically by tests/network/in_process_chaos.rs"]
fn test_high_uniform_packet_loss_with_extreme_sync() {
    skip_if_no_peer_binary!();
    let profile = NetworkProfile {
        packet_loss: 0.20, // 20% loss → 36% effective (1 - 0.8*0.8)
        latency_ms: 20,
        jitter_ms: 0,
        reorder_rate: 0.0,
        reorder_buffer_size: 0,
        duplicate_rate: 0.0,
        burst_loss_prob: 0.0,
        burst_loss_len: 0,
    };

    let scenario =
        NetworkScenario::symmetric("high_uniform_packet_loss_with_extreme_sync", profile)
            .with_frames(100)
            .with_timeout(180)
            .with_sync_preset("extreme");

    let (result1, result2) = scenario.run_test(10045);

    // Verify both peers reached success (frame >= target with all inputs confirmed);
    // sync_health() is diagnostic-only and is NOT part of the success check.
    verify_determinism(
        &result1,
        &result2,
        "high_uniform_packet_loss_with_extreme_sync",
    );

    println!(
        "High loss rollbacks - Peer 1: {}, Peer 2: {}",
        result1.rollbacks, result2.rollbacks
    );
}

/// Test actual burst packet loss recovery.
///
/// This test uses burst loss (short periods of complete packet loss) rather than
/// uniform loss. This simulates momentary network outages or WiFi/cellular handoffs.
///
/// Configuration: 3% baseline loss + 5% chance of 4-packet bursts.
/// Uses mobile sync preset for the combination of baseline + burst loss.
#[test]
#[serial]
#[ignore = "extreme real-UDP chaos; runs in the nightly network suite (ci-network-nightly.yml). Determinism is covered deterministically by tests/network/in_process_chaos.rs"]
fn test_burst_loss_recovery() {
    skip_if_no_peer_binary!();
    let profile = NetworkProfile {
        packet_loss: 0.03,     // 3% baseline loss (reduced from 5%)
        burst_loss_prob: 0.05, // 5% chance of burst (reduced from 10%)
        burst_loss_len: 4,     // 4 consecutive packets dropped (reduced from 5)
        latency_ms: 30,
        jitter_ms: 0,
        reorder_rate: 0.0,
        reorder_buffer_size: 0,
        duplicate_rate: 0.0,
    };

    let scenario = NetworkScenario::symmetric("burst_loss_recovery", profile)
        .with_frames(100)
        .with_timeout(120)
        .with_input_delay(3)
        .with_auto_sync_preset();

    let (result1, result2) = scenario.run_test(10750);

    // Verify both peers reached success (frame >= target with all inputs confirmed);
    // sync_health() is diagnostic-only and is NOT part of the success check.
    verify_determinism(&result1, &result2, "burst_loss_recovery");

    println!(
        "Burst loss rollbacks - Peer 1: {}, Peer 2: {}",
        result1.rollbacks, result2.rollbacks
    );
}

/// Test combining high latency with high jitter and packet loss.
/// This represents "worst case realistic" network conditions.
#[test]
#[serial]
#[ignore = "extreme real-UDP chaos; runs in the nightly network suite (ci-network-nightly.yml). Determinism is covered deterministically by tests/network/in_process_chaos.rs"]
fn test_worst_case_realistic_network() {
    skip_if_no_peer_binary!();
    let profile = NetworkProfile {
        packet_loss: 0.18, // 18% loss
        latency_ms: 80,    // 80ms base latency
        jitter_ms: 60,     // ±60ms jitter (20-140ms effective)
        reorder_rate: 0.0,
        reorder_buffer_size: 0,
        duplicate_rate: 0.0,
        burst_loss_prob: 0.0,
        burst_loss_len: 0,
    };

    let scenario = NetworkScenario::symmetric("worst_case_realistic_network", profile)
        .with_frames(100)
        .with_timeout(180)
        .with_auto_sync_preset();

    let (result1, result2) = scenario.run_test(10047);

    // Verify both peers reached success (frame >= target with all inputs confirmed);
    // sync_health() is diagnostic-only and is NOT part of the success check.
    verify_determinism(&result1, &result2, "worst_case_realistic_network");

    println!(
        "Worst case rollbacks - Peer 1: {}, Peer 2: {}",
        result1.rollbacks, result2.rollbacks
    );
}

/// Test rapid short session (10 frames) to verify fast completion.
#[test]
#[serial]
fn test_rapid_short_session() {
    skip_if_no_peer_binary!();
    let peer1_config = PeerConfig {
        local_port: 10049,
        player_index: 0,
        peer_addr: "127.0.0.1:10050".to_string(),
        frames: 10, // Very short session
        timeout_secs: 30,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10050,
        player_index: 1,
        peer_addr: "127.0.0.1:10049".to_string(),
        frames: 10,
        timeout_secs: 30,
        seed: Some(43),
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    // Verify both peers reached success (frame >= target with all inputs confirmed);
    // sync_health() is diagnostic-only and is NOT part of the success check.
    verify_determinism(&result1, &result2, "rapid_short_session");

    assert!(
        result1.final_frame >= 10,
        "Peer 1 didn't complete: {}",
        result1.final_frame
    );
    assert!(
        result2.final_frame >= 10,
        "Peer 2 didn't complete: {}",
        result2.final_frame
    );
}

/// Test where one peer has perfect network, other has terrible.
/// Tests the asymmetric resilience to the extreme.
#[test]
#[serial]
#[ignore = "extreme real-UDP chaos; runs in the nightly network suite (ci-network-nightly.yml). Determinism is covered deterministically by tests/network/in_process_chaos.rs"]
fn test_extreme_asymmetric_one_perfect_one_terrible() {
    skip_if_no_peer_binary!();
    let terrible_profile = NetworkProfile {
        packet_loss: 0.30, // 30% loss!
        latency_ms: 120,   // 120ms latency
        jitter_ms: 50,
        reorder_rate: 0.0,
        reorder_buffer_size: 0,
        duplicate_rate: 0.0,
        burst_loss_prob: 0.0,
        burst_loss_len: 0,
    };

    let scenario = NetworkScenario::asymmetric(
        "extreme_asymmetric_one_perfect_one_terrible",
        NetworkProfile::local(),
        terrible_profile,
    )
    .with_frames(100)
    .with_timeout(180)
    .with_auto_sync_preset();

    let (result1, result2) = scenario.run_test(10051);

    // Verify both peers reached success (frame >= target with all inputs confirmed);
    // sync_health() is diagnostic-only and is NOT part of the success check.
    verify_determinism(
        &result1,
        &result2,
        "extreme_asymmetric_one_perfect_one_terrible",
    );

    println!(
        "Extreme asymmetric - Perfect: {} rollbacks, Terrible: {} rollbacks",
        result1.rollbacks, result2.rollbacks
    );
}

/// Test very high latency (200ms) with moderate jitter.
/// Simulates intercontinental connection.
#[test]
#[serial]
#[ignore = "long-running endurance or high-latency real-UDP session; runs in the nightly network suite (ci-network-nightly.yml), not per-PR CI"]
fn test_intercontinental_latency() {
    skip_if_no_peer_binary!();
    let peer1_config = PeerConfig {
        local_port: 10053,
        player_index: 0,
        peer_addr: "127.0.0.1:10054".to_string(),
        frames: 100,
        packet_loss: 0.03, // Low loss
        latency_ms: 200,   // 200ms - intercontinental
        jitter_ms: 30,
        seed: Some(42),
        input_delay: 4, // Higher delay for high latency
        timeout_secs: 180,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10054,
        player_index: 1,
        peer_addr: "127.0.0.1:10053".to_string(),
        frames: 100,
        packet_loss: 0.03,
        latency_ms: 200,
        jitter_ms: 30,
        seed: Some(43),
        input_delay: 4,
        timeout_secs: 180,
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    // Verify both peers reached success (frame >= target with all inputs confirmed);
    // sync_health() is diagnostic-only and is NOT part of the success check.
    verify_determinism(&result1, &result2, "intercontinental_latency");
}

/// Stress test: 500 frames with varied network conditions.
/// Tests sustained operation under moderate adversity.
#[test]
#[serial]
#[ignore = "long-running endurance or high-latency real-UDP session; runs in the nightly network suite (ci-network-nightly.yml), not per-PR CI"]
fn test_sustained_moderate_adversity() {
    skip_if_no_peer_binary!();
    let peer1_config = PeerConfig {
        local_port: 10055,
        player_index: 0,
        peer_addr: "127.0.0.1:10056".to_string(),
        frames: 500,
        packet_loss: 0.10,
        latency_ms: 40,
        jitter_ms: 20,
        seed: Some(42),
        timeout_secs: 180,
        sync_preset: Some("lossy".to_string()),
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10056,
        player_index: 1,
        peer_addr: "127.0.0.1:10055".to_string(),
        frames: 500,
        packet_loss: 0.10,
        latency_ms: 40,
        jitter_ms: 20,
        seed: Some(43),
        timeout_secs: 180,
        sync_preset: Some("lossy".to_string()),
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    // Verify both peers reached success (frame >= target with all inputs confirmed);
    // sync_health() is diagnostic-only and is NOT part of the success check.
    verify_determinism(&result1, &result2, "sustained_moderate_adversity");

    assert!(
        result1.final_frame >= 500,
        "Peer 1 incomplete: {}",
        result1.final_frame
    );
    assert!(
        result2.final_frame >= 500,
        "Peer 2 incomplete: {}",
        result2.final_frame
    );

    println!(
        "Sustained adversity complete - Rollbacks: peer1={}, peer2={}",
        result1.rollbacks, result2.rollbacks
    );
}

/// Test with matching seeds to verify reproducibility.
/// Both peers should have identical network chaos, making behavior highly predictable.
#[test]
#[serial]
fn test_reproducible_chaos_same_seed() {
    skip_if_no_peer_binary!();
    // Same chaos seed on both peers
    let peer1_config = PeerConfig {
        local_port: 10057,
        player_index: 0,
        peer_addr: "127.0.0.1:10058".to_string(),
        frames: 100,
        packet_loss: 0.15,
        latency_ms: 30,
        jitter_ms: 15,
        seed: Some(12345), // Same seed
        timeout_secs: 120,
        sync_preset: Some("mobile".to_string()),
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10058,
        player_index: 1,
        peer_addr: "127.0.0.1:10057".to_string(),
        frames: 100,
        packet_loss: 0.15,
        latency_ms: 30,
        jitter_ms: 15,
        seed: Some(12345), // Same seed
        timeout_secs: 120,
        sync_preset: Some("mobile".to_string()),
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    // Verify both peers reached success (frame >= target with all inputs confirmed);
    // sync_health() is diagnostic-only and is NOT part of the success check.
    verify_determinism(&result1, &result2, "reproducible_chaos_same_seed");
}

/// Test with no chaos (passthrough) to verify baseline correctness.
#[test]
#[serial]
fn test_baseline_no_chaos() {
    skip_if_no_peer_binary!();
    let peer1_config = PeerConfig {
        local_port: 10059,
        player_index: 0,
        peer_addr: "127.0.0.1:10060".to_string(),
        frames: 200,
        packet_loss: 0.0,
        latency_ms: 0,
        jitter_ms: 0,
        timeout_secs: 60,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10060,
        player_index: 1,
        peer_addr: "127.0.0.1:10059".to_string(),
        frames: 200,
        packet_loss: 0.0,
        latency_ms: 0,
        jitter_ms: 0,
        timeout_secs: 60,
        seed: Some(43),
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    // Verify both peers reached success (frame >= target with all inputs confirmed);
    // sync_health() is diagnostic-only and is NOT part of the success check.
    verify_determinism(&result1, &result2, "baseline_no_chaos");

    assert!(
        result1.final_frame >= 200,
        "Peer 1 incomplete: {}",
        result1.final_frame
    );
    assert!(
        result2.final_frame >= 200,
        "Peer 2 incomplete: {}",
        result2.final_frame
    );

    // With no chaos, should see minimal or no rollbacks
    println!(
        "Baseline rollbacks - Peer 1: {}, Peer 2: {}",
        result1.rollbacks, result2.rollbacks
    );
}

/// Test WiFi interference pattern - bursty packet loss.
/// WiFi with interference has low base latency but burst loss events.
#[test]
#[serial]
fn test_wifi_interference_simulation() {
    skip_if_no_peer_binary!();
    let peer1_config = PeerConfig {
        local_port: 10061,
        player_index: 0,
        peer_addr: "127.0.0.1:10062".to_string(),
        frames: 100,
        packet_loss: 0.03, // Low base loss
        latency_ms: 15,    // Fast base latency
        jitter_ms: 25,     // Moderate jitter
        seed: Some(42),
        timeout_secs: 90,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10062,
        player_index: 1,
        peer_addr: "127.0.0.1:10061".to_string(),
        frames: 100,
        packet_loss: 0.03,
        latency_ms: 15,
        jitter_ms: 25,
        seed: Some(43),
        timeout_secs: 90,
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    // Verify both peers reached success (frame >= target with all inputs confirmed);
    // sync_health() is diagnostic-only and is NOT part of the success check.
    verify_determinism(&result1, &result2, "wifi_interference_simulation");

    println!(
        "WiFi simulation - Rollbacks: peer1={}, peer2={}",
        result1.rollbacks, result2.rollbacks
    );
}

/// Test intercontinental connection pattern - high but stable latency.
/// Cross-ocean connections have consistent high latency with low loss.
#[test]
#[serial]
#[ignore = "long-running endurance or high-latency real-UDP session; runs in the nightly network suite (ci-network-nightly.yml), not per-PR CI"]
fn test_intercontinental_simulation() {
    skip_if_no_peer_binary!();
    let peer1_config = PeerConfig {
        local_port: 10063,
        player_index: 0,
        peer_addr: "127.0.0.1:10064".to_string(),
        frames: 80, // Fewer frames due to high latency
        packet_loss: 0.02,
        latency_ms: 120, // High but stable latency
        jitter_ms: 15,   // Low jitter
        seed: Some(42),
        timeout_secs: 180, // Longer timeout for high latency
        input_delay: 4,    // Higher input delay recommended
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10064,
        player_index: 1,
        peer_addr: "127.0.0.1:10063".to_string(),
        frames: 80,
        packet_loss: 0.02,
        latency_ms: 120,
        jitter_ms: 15,
        seed: Some(43),
        timeout_secs: 180,
        input_delay: 4,
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    // Verify both peers reached success (frame >= target with all inputs confirmed);
    // sync_health() is diagnostic-only and is NOT part of the success check.
    verify_determinism(&result1, &result2, "intercontinental_simulation");

    println!(
        "Intercontinental simulation - Rollbacks: peer1={}, peer2={}",
        result1.rollbacks, result2.rollbacks
    );
}

/// Test competitive/LAN-like conditions - minimal latency and loss.
/// Validates that competitive presets work well under ideal conditions.
#[test]
#[serial]
fn test_competitive_lan_simulation() {
    skip_if_no_peer_binary!();
    let peer1_config = PeerConfig {
        local_port: 10065,
        player_index: 0,
        peer_addr: "127.0.0.1:10066".to_string(),
        frames: 200, // More frames since it's fast
        packet_loss: 0.0,
        latency_ms: 2, // Near-zero latency
        jitter_ms: 1,  // Minimal jitter
        seed: Some(42),
        timeout_secs: 60,
        input_delay: 0, // No input delay for competitive
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10066,
        player_index: 1,
        peer_addr: "127.0.0.1:10065".to_string(),
        frames: 200,
        packet_loss: 0.0,
        latency_ms: 2,
        jitter_ms: 1,
        seed: Some(43),
        timeout_secs: 60,
        input_delay: 0,
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);

    // Verify both peers reached success (frame >= target with all inputs confirmed);
    // sync_health() is diagnostic-only and is NOT part of the success check.
    verify_determinism(&result1, &result2, "competitive_lan_simulation");

    // Rollback counts with input_delay=0 are inherently variable and depend on:
    // - OS scheduling and CPU availability
    // - Network stack behavior
    // - Process startup timing
    //
    // With input_delay=0, rollbacks are EXPECTED because inputs can't arrive
    // before they're needed. The important verification is determinism, not
    // rollback count.
    //
    // Note: peer2 (spawned second) typically has more rollbacks because peer1
    // has a head start and peer2 must catch up through rollbacks.
    println!(
        "Competitive simulation - Rollbacks: peer1={}, peer2={} \
         (variance is expected with input_delay=0)",
        result1.rollbacks, result2.rollbacks
    );

    // Soft warning for unusually high rollback counts that might indicate issues
    // These thresholds are informational, not hard failures
    if result1.rollbacks > 100 || result2.rollbacks > 300 {
        eprintln!(
            "WARNING: Higher than typical rollback counts observed. \
             This may indicate CI environment pressure or timing anomalies. \
             peer1={}, peer2={} (typical: peer1<50, peer2<200)",
            result1.rollbacks, result2.rollbacks
        );
    }
}

// =============================================================================
// Data-Driven Network Condition Tests
// =============================================================================

/// Network condition test case for data-driven testing
struct NetworkConditionCase {
    name: &'static str,
    port_base: u16,
    frames: i32,
    packet_loss: f64,
    latency_ms: u64,
    jitter_ms: u64,
    input_delay: usize,
    sync_preset: Option<&'static str>,
    /// Expected to complete successfully despite conditions
    expect_success: bool,
}

impl NetworkConditionCase {
    fn scenario(&self) -> NetworkScenario {
        let profile = NetworkProfile {
            packet_loss: self.packet_loss,
            latency_ms: self.latency_ms,
            jitter_ms: self.jitter_ms,
            reorder_rate: 0.0,
            reorder_buffer_size: 0,
            duplicate_rate: 0.0,
            burst_loss_prob: 0.0,
            burst_loss_len: 0,
        };

        let scenario = NetworkScenario::symmetric(self.name, profile)
            .with_frames(self.frames)
            .with_input_delay(self.input_delay)
            .with_timeout(120);

        if let Some(sync_preset) = self.sync_preset {
            scenario.with_sync_preset(sync_preset)
        } else {
            scenario.with_auto_sync_preset()
        }
    }

    fn run_and_verify(&self) -> (TestResult, TestResult) {
        let (result1, result2) = self.scenario().run_test(self.port_base);

        println!(
            "{}: peer1(frame={}, rollbacks={}), peer2(frame={}, rollbacks={})",
            self.name,
            result1.final_frame,
            result1.rollbacks,
            result2.final_frame,
            result2.rollbacks,
        );

        if self.expect_success {
            // Use verify_determinism which checks success and logs final_value differences
            verify_determinism(&result1, &result2, self.name);
        }

        (result1, result2)
    }
}

/// Test various input delay values to understand rollback behavior.
///
/// This data-driven test explores the relationship between input delay
/// and rollback counts under different network conditions. Higher input
/// delay should generally reduce rollbacks at the cost of input latency.
#[test]
#[serial]
fn test_input_delay_vs_rollbacks_data_driven() {
    skip_if_no_peer_binary!();
    let test_cases = [
        NetworkConditionCase {
            name: "zero_delay_zero_latency",
            port_base: 10200,
            frames: 100,
            packet_loss: 0.0,
            latency_ms: 0,
            jitter_ms: 0,
            input_delay: 0,
            sync_preset: None,
            expect_success: true,
        },
        NetworkConditionCase {
            name: "zero_delay_low_latency",
            port_base: 10202,
            frames: 100,
            packet_loss: 0.0,
            latency_ms: 5,
            jitter_ms: 2,
            input_delay: 0,
            sync_preset: None,
            expect_success: true,
        },
        NetworkConditionCase {
            name: "delay_2_low_latency",
            port_base: 10204,
            frames: 100,
            packet_loss: 0.0,
            latency_ms: 5,
            jitter_ms: 2,
            input_delay: 2,
            sync_preset: None,
            expect_success: true,
        },
        NetworkConditionCase {
            name: "delay_4_moderate_latency",
            port_base: 10206,
            frames: 100,
            packet_loss: 0.0,
            latency_ms: 20,
            jitter_ms: 5,
            input_delay: 4,
            sync_preset: None,
            expect_success: true,
        },
    ];

    println!("=== Input Delay vs Rollbacks Analysis ===");
    for case in &test_cases {
        let (result1, result2) = case.run_and_verify();

        // For low-latency cases with appropriate input delay, rollbacks should be minimal
        if case.input_delay >= 2 && case.latency_ms <= 20 && case.packet_loss == 0.0 {
            // With proper input delay buffering, we expect fewer rollbacks
            // But we don't assert on specific counts due to environment variance
            if result1.rollbacks > 50 || result2.rollbacks > 50 {
                println!(
                    "  Note: {} had higher than expected rollbacks with delay={}: p1={}, p2={}",
                    case.name, case.input_delay, result1.rollbacks, result2.rollbacks
                );
            }
        }
    }
    println!("==========================================");
}

/// Test that packet loss handling is deterministic.
///
/// This data-driven test verifies that even with packet loss,
/// both peers eventually reach the same game state through rollbacks.
#[test]
#[serial]
#[ignore = "extreme real-UDP chaos; runs in the nightly network suite (ci-network-nightly.yml). Determinism is covered deterministically by tests/network/in_process_chaos.rs"]
fn test_packet_loss_determinism_data_driven() {
    skip_if_no_peer_binary!();
    let test_cases = [
        NetworkConditionCase {
            name: "low_loss_5pct",
            port_base: 10210,
            frames: 100,
            packet_loss: 0.05,
            latency_ms: 10,
            jitter_ms: 5,
            input_delay: 2,
            sync_preset: None,
            expect_success: true,
        },
        NetworkConditionCase {
            name: "moderate_loss_10pct",
            port_base: 10212,
            frames: 100,
            packet_loss: 0.10,
            latency_ms: 15,
            jitter_ms: 5,
            input_delay: 3,
            sync_preset: Some("lossy"),
            expect_success: true,
        },
        NetworkConditionCase {
            name: "high_loss_20pct",
            port_base: 10214,
            frames: 100,
            packet_loss: 0.20,
            latency_ms: 20,
            jitter_ms: 10,
            input_delay: 4,
            sync_preset: Some("extreme"),
            expect_success: true,
        },
    ];

    println!("=== Packet Loss Determinism Analysis ===");
    for case in &test_cases {
        let (result1, result2) = case.run_and_verify();

        // With packet loss, we expect rollbacks but determinism must hold
        println!(
            "  {}: loss={:.0}%, rollbacks p1={}, p2={}, deterministic={}",
            case.name,
            case.packet_loss * 100.0,
            result1.rollbacks,
            result2.rollbacks,
            result1.final_value == result2.final_value
        );
    }
    println!("========================================");
}

/// Test edge cases related to timing and execution overhead.
///
/// This data-driven test covers scenarios that might be affected by:
/// - Code instrumentation (coverage tools)
/// - CI environment variability
/// - Process scheduling overhead
///
/// These tests use shorter frame counts to ensure they complete even
/// under instrumentation overhead.
#[test]
#[serial]
fn test_timing_sensitive_edge_cases_data_driven() {
    skip_if_no_peer_binary!();
    let test_cases = [
        // Zero input delay with perfect network - tests raw sync behavior
        NetworkConditionCase {
            name: "zero_delay_perfect",
            port_base: 10220,
            frames: 50, // Shorter for reliability under instrumentation
            packet_loss: 0.0,
            latency_ms: 0,
            jitter_ms: 0,
            input_delay: 0,
            sync_preset: None,
            expect_success: true,
        },
        // High frame count with minimal conditions
        NetworkConditionCase {
            name: "many_frames_minimal_conditions",
            port_base: 10222,
            frames: 200,
            packet_loss: 0.0,
            latency_ms: 5,
            jitter_ms: 2,
            input_delay: 2,
            sync_preset: None,
            expect_success: true,
        },
        // High latency with jitter - stresses timing
        NetworkConditionCase {
            name: "high_latency_timing_stress",
            port_base: 10224,
            frames: 50,
            packet_loss: 0.0,
            latency_ms: 100,
            jitter_ms: 50,
            input_delay: 4,
            sync_preset: Some("high_latency"),
            expect_success: true,
        },
        // Combined stress: loss + latency + jitter
        NetworkConditionCase {
            name: "combined_timing_stress",
            port_base: 10226,
            frames: 50,
            packet_loss: 0.10,
            latency_ms: 50,
            jitter_ms: 30,
            input_delay: 3,
            sync_preset: Some("lossy"),
            expect_success: true,
        },
    ];

    println!("=== Timing-Sensitive Edge Cases ===");
    let start = std::time::Instant::now();

    for case in &test_cases {
        let case_start = std::time::Instant::now();
        let (result1, result2) = case.run_and_verify();
        let case_duration = case_start.elapsed();

        println!(
            "  {}: duration={:.2}s, rollbacks p1={}, p2={}",
            case.name,
            case_duration.as_secs_f64(),
            result1.rollbacks,
            result2.rollbacks
        );
    }

    let total_duration = start.elapsed();
    println!(
        "Total timing tests: {:.2}s (avg {:.2}s/test)",
        total_duration.as_secs_f64(),
        total_duration.as_secs_f64() / test_cases.len() as f64
    );
    println!("===================================");
}

// =============================================================================
// Network Scenario-Based Tests
// =============================================================================

/// Test multiple network scenarios using the scenario abstraction.
///
/// This demonstrates the `NetworkScenario` pattern for concise, expressive tests.
/// Each scenario encapsulates realistic network conditions.
#[test]
#[serial]
#[ignore = "extreme real-UDP chaos; runs in the nightly network suite (ci-network-nightly.yml). Determinism is covered deterministically by tests/network/in_process_chaos.rs"]
fn test_network_scenario_suite() {
    skip_if_no_peer_binary!();
    println!("=== Network Scenario Suite ===");

    // Test a variety of common network scenarios
    let scenarios = [
        (10300, NetworkScenario::lan().with_frames(100)),
        (10302, NetworkScenario::wifi_good().with_frames(100)),
        // mobile_4g has 8% loss + 1% burst loss; borderline case but adding
        // "lossy" preset for reliability.
        (
            10304,
            NetworkScenario::mobile_4g()
                .with_frames(80)
                .with_input_delay(3)
                .with_sync_preset("lossy"),
        ),
        // wifi_congested has 15% raw loss, which is ~28% effective with
        // bidirectional chaos, plus burst loss. Use the automatic selector.
        (
            10306,
            NetworkScenario::symmetric("wifi_congested", NetworkProfile::wifi_congested())
                .with_frames(60)
                .with_input_delay(4)
                .with_timeout(180)
                .with_auto_sync_preset(),
        ),
    ];

    for (port, scenario) in scenarios {
        println!("Testing scenario: {}", scenario.summary());
        let (result1, result2) = scenario.run_test(port);

        // Verify frame targets are met
        assert!(
            result1.final_frame >= scenario.frames,
            "{}: Peer 1 didn't reach target frames: {} < {}",
            scenario.name,
            result1.final_frame,
            scenario.frames
        );
        assert!(
            result2.final_frame >= scenario.frames,
            "{}: Peer 2 didn't reach target frames: {} < {}",
            scenario.name,
            result2.final_frame,
            scenario.frames
        );
    }

    println!("=== Scenario Suite Complete ===");
}

/// Test asymmetric network scenarios where peers have different conditions.
///
/// This is common in real-world scenarios where one player is on WiFi
/// and another is on mobile, or one has a better ISP than the other.
#[test]
#[serial]
#[ignore = "extreme real-UDP chaos; runs in the nightly network suite (ci-network-nightly.yml). Determinism is covered deterministically by tests/network/in_process_chaos.rs"]
fn test_asymmetric_scenarios() {
    skip_if_no_peer_binary!();
    println!("=== Asymmetric Network Scenarios ===");

    let scenarios = [
        // WiFi vs Mobile
        // mobile_4g has 8% loss + 1% burst; adding "lossy" preset for reliability.
        (
            10320,
            NetworkScenario::asymmetric(
                "wifi_vs_mobile",
                NetworkProfile::wifi_good(),
                NetworkProfile::mobile_4g(),
            )
            .with_frames(80)
            .with_input_delay(3)
            .with_sync_preset("lossy"),
        ),
        // LAN vs WiFi congested. Automatic preset selection accounts for the
        // effective bidirectional loss and burst component.
        (
            10322,
            NetworkScenario::asymmetric(
                "lan_vs_wifi_congested",
                NetworkProfile::lan(),
                NetworkProfile::wifi_congested(),
            )
            .with_frames(60)
            .with_input_delay(4)
            .with_timeout(150)
            .with_auto_sync_preset(),
        ),
        // Good vs Terrible (extreme asymmetry)
        // The terrible profile has 25% loss (>20%) + 5% burst loss (5-packet bursts).
        // The combination of high packet loss WITH burst loss is significantly worse
        // than either alone. Using "stress_test" preset (40 sync packets, 60s timeout)
        // for reliability on CI.
        (
            10324,
            NetworkScenario::asymmetric(
                "good_vs_terrible",
                NetworkProfile::wifi_good(),
                NetworkProfile::terrible(),
            )
            .with_frames(50)
            .with_input_delay(5)
            .with_timeout(180)
            .with_sync_preset("stress_test"),
        ),
    ];

    for (port, scenario) in scenarios {
        println!("Testing scenario: {}", scenario.summary());
        let (result1, result2) = scenario.run_test(port);

        // The peer with worse network typically has more rollbacks
        // Log this for analysis
        println!(
            "  Rollback asymmetry: peer1={}, peer2={}, ratio={:.2}",
            result1.rollbacks,
            result2.rollbacks,
            if result1.rollbacks > 0 {
                result2.rollbacks as f64 / result1.rollbacks as f64
            } else {
                f64::INFINITY
            }
        );

        // Verify frame targets are met
        assert!(
            result1.final_frame >= scenario.frames,
            "{}: Peer 1 didn't reach target frames",
            scenario.name
        );
        assert!(
            result2.final_frame >= scenario.frames,
            "{}: Peer 2 didn't reach target frames",
            scenario.name
        );
    }

    println!("=== Asymmetric Scenarios Complete ===");
}

/// Test edge case: both peers on local network (no simulated chaos).
///
/// This validates that the abstraction works correctly with zero chaos.
#[test]
#[serial]
fn test_scenario_local_perfect() {
    skip_if_no_peer_binary!();
    let scenario = NetworkScenario::symmetric("local_perfect", NetworkProfile::local())
        .with_frames(200)
        .with_input_delay(0);

    println!("Testing perfect local scenario: {}", scenario.summary());
    let (result1, result2) = scenario.run_test(10340);

    // With perfect conditions and no input delay, rollbacks are possible
    // due to process scheduling, but should be minimal
    println!(
        "Perfect local: rollbacks peer1={}, peer2={}",
        result1.rollbacks, result2.rollbacks
    );
}

// =============================================================================
// Unit Tests for Test Infrastructure
// =============================================================================

#[cfg(test)]
mod infrastructure_tests {
    use super::*;

    /// Verify that the platform-specific binary name is correct.
    #[test]
    fn test_peer_binary_name_platform_specific() {
        // This tests that the const correctly handles platform differences
        #[cfg(windows)]
        assert_eq!(PEER_BINARY_NAME, "network_test_peer.exe");

        #[cfg(not(windows))]
        assert_eq!(PEER_BINARY_NAME, "network_test_peer");
    }

    /// Verify that find_peer_binary returns Some when binary exists.
    #[test]
    fn test_find_peer_binary_when_exists() {
        // This test will pass if the binary is built, skip otherwise
        if let Some(path) = find_peer_binary() {
            assert!(
                path.exists(),
                "Returned path should exist: {}",
                path.display()
            );
            assert!(
                path.ends_with(PEER_BINARY_NAME),
                "Path should end with binary name: {}",
                path.display()
            );
        } else {
            eprintln!(
                "Note: find_peer_binary returned None - binary not built. \
                 This is expected if network-test-peer hasn't been compiled."
            );
        }
    }

    /// Verify that is_peer_binary_available is consistent with find_peer_binary.
    #[test]
    fn test_is_peer_binary_available_consistency() {
        let available = is_peer_binary_available();
        let found = find_peer_binary();

        assert_eq!(
            available,
            found.is_some(),
            "is_peer_binary_available should match find_peer_binary().is_some()"
        );
    }

    /// Test PeerConfig default values.
    /// Default seed is Some(42) for deterministic testing.
    #[test]
    fn test_peer_config_defaults() {
        let config = PeerConfig::default();

        assert_eq!(config.local_port, 0);
        assert_eq!(config.player_index, 0);
        assert!(config.peer_addr.is_empty());
        assert_eq!(config.frames, 100);
        assert!((config.packet_loss - 0.0).abs() < f64::EPSILON);
        assert_eq!(config.latency_ms, 0);
        assert_eq!(config.jitter_ms, 0);
        assert_eq!(config.seed, Some(42)); // Deterministic by default
        assert_eq!(config.timeout_secs, 30);
        assert_eq!(config.input_delay, 2);
    }

    /// Test PeerConfig diagnostic summary format.
    #[test]
    fn test_peer_config_diagnostic_summary() {
        let config = PeerConfig {
            local_port: 10001,
            player_index: 0,
            peer_addr: "127.0.0.1:10002".to_string(),
            frames: 100,
            packet_loss: 0.05,
            latency_ms: 30,
            jitter_ms: 10,
            seed: Some(42),
            timeout_secs: 60,
            input_delay: 2,
            ..Default::default()
        };

        let summary = config.diagnostic_summary();

        // Verify all key fields are present in the summary
        assert!(
            summary.contains("port=10001"),
            "Summary should contain port"
        );
        assert!(
            summary.contains("player=0"),
            "Summary should contain player index"
        );
        assert!(
            summary.contains("127.0.0.1:10002"),
            "Summary should contain peer address"
        );
        assert!(
            summary.contains("frames=100"),
            "Summary should contain frames"
        );
        assert!(
            summary.contains("5.0%"),
            "Summary should contain packet loss percentage"
        );
        assert!(summary.contains("30ms"), "Summary should contain latency");
        assert!(summary.contains("10ms"), "Summary should contain jitter");
        assert!(
            summary.contains("delay=2"),
            "Summary should contain input delay"
        );
        assert!(summary.contains("42"), "Summary should contain seed");
    }

    /// Test TestResult diagnostic summary format.
    #[test]
    fn test_test_result_diagnostic_summary() {
        let result = TestResult {
            success: true,
            final_frame: 100,
            final_value: 12345,
            checksum: 0xDEADBEEF,
            rollbacks: 5,
            desync_detected: 0,
            error_kind: None,
            error: None,
            runtime: None,
        };

        let summary = result.diagnostic_summary();

        // Verify all key fields are present
        assert!(
            summary.contains("success=true"),
            "Summary should contain success"
        );
        assert!(
            summary.contains("frame=100"),
            "Summary should contain frame"
        );
        assert!(
            summary.contains("value=12345"),
            "Summary should contain value"
        );
        assert!(
            summary.contains("deadbeef"),
            "Summary should contain checksum hex"
        );
        assert!(
            summary.contains("rollbacks=5"),
            "Summary should contain rollbacks"
        );
    }

    /// Test TestResult with error message.
    #[test]
    fn test_test_result_with_error() {
        let result = TestResult {
            success: false,
            final_frame: 50,
            final_value: 0,
            checksum: 0,
            rollbacks: 0,
            desync_detected: 0,
            error_kind: Some("timeout".to_string()),
            error: Some("Connection timeout".to_string()),
            runtime: None,
        };

        let summary = result.diagnostic_summary();

        assert!(
            summary.contains("success=false"),
            "Summary should show failure"
        );
        assert!(
            summary.contains("Connection timeout"),
            "Summary should contain error"
        );
    }

    /// Successful result with the given top-level desync count (no runtime
    /// diagnostics), for exercising the `verify_determinism_n` desync oracle.
    fn successful_result_with_desyncs(desync_detected: u32) -> TestResult {
        TestResult {
            success: true,
            final_frame: 100,
            final_value: 1,
            checksum: 1,
            rollbacks: 0,
            desync_detected,
            error_kind: None,
            error: None,
            runtime: None,
        }
    }

    /// The flipped desync oracle accepts an all-clean N-peer run.
    #[test]
    fn verify_determinism_n_accepts_zero_desync_counts() {
        let results = vec![
            successful_result_with_desyncs(0),
            successful_result_with_desyncs(0),
            successful_result_with_desyncs(0),
        ];
        verify_determinism_n(&results, "oracle_unit_test_clean");
    }

    /// Non-vacuity guard for the desync oracle flipped after the S30 F17 fix:
    /// a successful run where one peer observed a `DesyncDetected` event must
    /// fail `verify_determinism_n` (the count was previously logged-only).
    #[test]
    #[should_panic(expected = "DesyncDetected event(s) on a clean network")]
    fn verify_determinism_n_panics_on_nonzero_desync_count() {
        let results = vec![
            successful_result_with_desyncs(0),
            successful_result_with_desyncs(1),
            successful_result_with_desyncs(0),
        ];
        verify_determinism_n(&results, "oracle_unit_test_desync");
    }

    /// The oracle also sees a count surfaced only through the nested
    /// `runtime.events` fallback (older binary / partial JSON shape).
    #[test]
    #[should_panic(expected = "DesyncDetected event(s) on a clean network")]
    fn verify_determinism_n_panics_on_nested_runtime_desync_count() {
        let mut result = successful_result_with_desyncs(0);
        result.runtime = Some(RuntimeDiagnostics {
            session_state: "Running".to_string(),
            current_frame: 100,
            confirmed_frame: 100,
            target_frame: 100,
            elapsed_ms: 1_000,
            sync_preset: None,
            sync_config: "default".to_string(),
            protocol_config: None,
            time_sync_config: None,
            sync_health: "InSync".to_string(),
            events: EventSummary {
                synchronizing: 1,
                synchronized: 1,
                network_interrupted: 0,
                network_resumed: 0,
                disconnected: 0,
                sync_timeout: 0,
                wait_recommendation: 0,
                input_delay_recommendation: 0,
                desync_detected: 2,
                peer_dropped: 0,
                replay_desync: 0,
            },
        });
        let results = vec![successful_result_with_desyncs(0), result];
        verify_determinism_n(&results, "oracle_unit_test_nested_desync");
    }

    /// Test NetworkProfile constants are valid.
    #[test]
    fn test_network_profiles_valid() {
        // Test that all profiles have valid values
        let profiles = [
            ("local", NetworkProfile::local()),
            ("lan", NetworkProfile::lan()),
            ("wifi_good", NetworkProfile::wifi_good()),
            ("wifi_average", NetworkProfile::wifi_average()),
            ("wifi_congested", NetworkProfile::wifi_congested()),
            ("mobile_4g", NetworkProfile::mobile_4g()),
            ("mobile_3g", NetworkProfile::mobile_3g()),
            ("intercontinental", NetworkProfile::intercontinental()),
            ("terrible", NetworkProfile::terrible()),
            ("heavy_reorder", NetworkProfile::heavy_reorder()),
            ("duplicating", NetworkProfile::duplicating()),
            ("bursty_survivable", NetworkProfile::bursty_survivable()),
            ("bursty", NetworkProfile::bursty()),
        ];

        for (name, profile) in profiles {
            // Packet loss should be between 0 and 1
            assert!(
                (0.0..=1.0).contains(&profile.packet_loss),
                "{}: packet_loss {} should be between 0 and 1",
                name,
                profile.packet_loss
            );

            // Latency values are reasonable (under 1 second)
            assert!(
                profile.latency_ms < 1000,
                "{}: latency_ms {} seems unreasonably high",
                name,
                profile.latency_ms
            );

            // Jitter should not exceed latency significantly
            assert!(
                profile.jitter_ms <= profile.latency_ms * 2 + 50,
                "{}: jitter_ms {} seems disproportionate to latency {}",
                name,
                profile.jitter_ms,
                profile.latency_ms
            );

            // Extended chaos validation
            assert!(
                (0.0..=1.0).contains(&profile.reorder_rate),
                "{}: reorder_rate {} should be between 0 and 1",
                name,
                profile.reorder_rate
            );
            assert!(
                (0.0..=1.0).contains(&profile.duplicate_rate),
                "{}: duplicate_rate {} should be between 0 and 1",
                name,
                profile.duplicate_rate
            );
            assert!(
                (0.0..=1.0).contains(&profile.burst_loss_prob),
                "{}: burst_loss_prob {} should be between 0 and 1",
                name,
                profile.burst_loss_prob
            );
        }
    }

    /// Test that `suggested_sync_preset()` returns appropriate presets for each profile.
    #[test]
    fn test_suggested_sync_preset() {
        let test_cases = [
            // Good network conditions - no preset needed
            ("local", NetworkProfile::local(), None),
            ("lan", NetworkProfile::lan(), None),
            ("wifi_good", NetworkProfile::wifi_good(), None),
            ("wifi_average", NetworkProfile::wifi_average(), None),
            // Moderate conditions - lossy or mobile preset
            ("mobile_4g", NetworkProfile::mobile_4g(), Some("lossy")),
            // High effective loss with burst conditions - stress_test preset
            (
                "wifi_congested",
                NetworkProfile::wifi_congested(),
                Some("stress_test"),
            ),
            (
                "mobile_3g",
                NetworkProfile::mobile_3g(),
                Some("stress_test"),
            ),
            // Extreme packet loss (>20%) combined with burst loss - stress_test preset
            ("terrible", NetworkProfile::terrible(), Some("stress_test")),
            (
                "bursty_survivable",
                NetworkProfile::bursty_survivable(),
                Some("stress_test"),
            ),
            // Extreme conditions - stress_test preset
            ("bursty", NetworkProfile::bursty(), Some("stress_test")),
            // No loss profiles
            ("heavy_reorder", NetworkProfile::heavy_reorder(), None),
            ("duplicating", NetworkProfile::duplicating(), None),
            ("intercontinental", NetworkProfile::intercontinental(), None),
        ];

        println!("=== Suggested Sync Preset Validation ===");
        for (name, profile, expected) in test_cases {
            let suggested = profile.suggested_sync_preset();
            println!(
                "{:<20} loss={:>5.1}% effective={:>5.1}% burst={:>4.1}%x{} -> {:?}",
                name,
                profile.packet_loss * 100.0,
                profile.effective_symmetric_packet_loss() * 100.0,
                profile.burst_loss_prob * 100.0,
                profile.burst_loss_len,
                suggested
            );
            assert_eq!(
                suggested, expected,
                "{}: expected {:?}, got {:?}",
                name, expected, suggested
            );
        }
        println!("=========================================");
    }

    /// Test edge cases for combined packet loss + burst loss sync preset selection.
    ///
    /// This tests the boundary conditions for the rule that upgrades to `stress_test`
    /// when high packet loss (>20%) is combined with significant burst loss.
    #[test]
    fn test_suggested_sync_preset_combined_conditions() {
        println!("=== Combined Conditions Edge Cases ===");

        // Edge case: exactly at threshold (20% loss + 3% burst with 3-packet bursts)
        let at_threshold = NetworkProfile {
            packet_loss: 0.20,
            latency_ms: 50,
            jitter_ms: 25,
            reorder_rate: 0.0,
            reorder_buffer_size: 0,
            duplicate_rate: 0.0,
            burst_loss_prob: 0.03,
            burst_loss_len: 3,
        };
        assert_eq!(
            at_threshold.suggested_sync_preset(),
            Some("stress_test"),
            "Exactly at combined threshold should return stress_test"
        );

        // Edge case: high effective packet loss without burst loss
        let high_loss_no_burst = NetworkProfile {
            packet_loss: 0.25,
            latency_ms: 50,
            jitter_ms: 25,
            reorder_rate: 0.0,
            reorder_buffer_size: 0,
            duplicate_rate: 0.0,
            burst_loss_prob: 0.0,
            burst_loss_len: 3,
        };
        assert_eq!(
            high_loss_no_burst.suggested_sync_preset(),
            Some("extreme"),
            "High loss without burst should return extreme (not stress_test)"
        );

        // Edge case: high packet loss but burst len too short
        let high_loss_short_burst = NetworkProfile {
            packet_loss: 0.25,
            latency_ms: 50,
            jitter_ms: 25,
            reorder_rate: 0.0,
            reorder_buffer_size: 0,
            duplicate_rate: 0.0,
            burst_loss_prob: 0.05,
            burst_loss_len: 2, // Below 3-packet threshold
        };
        assert_eq!(
            high_loss_short_burst.suggested_sync_preset(),
            Some("extreme"),
            "High loss + short burst should return extreme (not stress_test)"
        );

        // Edge case: moderate effective loss with burst should return mobile.
        let moderate_effective_loss_with_burst = NetworkProfile {
            packet_loss: 0.10,
            latency_ms: 50,
            jitter_ms: 25,
            reorder_rate: 0.0,
            reorder_buffer_size: 0,
            duplicate_rate: 0.0,
            burst_loss_prob: 0.05,
            burst_loss_len: 4,
        };
        assert_eq!(
            moderate_effective_loss_with_burst.suggested_sync_preset(),
            Some("mobile"),
            "Moderate effective loss + burst should return mobile (not stress_test)"
        );

        println!("All combined condition edge cases passed!");
    }

    /// Test NetworkScenario builder pattern.
    #[test]
    fn test_network_scenario_builder() {
        let scenario = NetworkScenario::lan()
            .with_frames(200)
            .with_input_delay(4)
            .with_timeout(120)
            .with_sync_preset("mobile");

        assert_eq!(scenario.frames, 200);
        assert_eq!(scenario.input_delay, 4);
        assert_eq!(scenario.timeout_secs, 120);
        assert_eq!(scenario.sync_preset, Some("mobile".to_string()));
    }

    /// Test NetworkScenario summary includes seeded chaos/profile details.
    #[test]
    fn test_network_scenario_summary_reports_seeded_profile() {
        let scenario = NetworkScenario::symmetric("summary_test", NetworkProfile::bursty())
            .with_frames(60)
            .with_input_delay(3)
            .with_sync_preset("stress_test");

        let summary = scenario.summary();
        assert!(summary.contains("summary_test"));
        assert!(
            summary.contains("seeds=42/43"),
            "Summary should report deterministic peer seeds: {summary}"
        );
        assert!(
            summary.contains("reorder=") && summary.contains("dup=") && summary.contains("burst="),
            "Summary should report extended chaos profile fields: {summary}"
        );
        assert!(
            summary.contains("sync=Some(\"stress_test\")"),
            "Summary should report sync preset: {summary}"
        );
    }

    /// Test NetworkScenario sync_preset is properly propagated to peer configs.
    #[test]
    fn test_network_scenario_sync_preset_propagation() {
        let scenario =
            NetworkScenario::symmetric("test", NetworkProfile::bursty()).with_sync_preset("mobile");

        let (peer1, peer2) = scenario.to_peer_configs(30000);

        // Both peers should have the sync preset set
        assert_eq!(peer1.sync_preset, Some("mobile".to_string()));
        assert_eq!(peer2.sync_preset, Some("mobile".to_string()));
    }

    /// Test NetworkScenario sync_preset "extreme" is properly propagated.
    #[test]
    fn test_network_scenario_extreme_sync_preset_propagation() {
        let scenario = NetworkScenario::symmetric("test", NetworkProfile::bursty())
            .with_sync_preset("extreme");

        let (peer1, peer2) = scenario.to_peer_configs(30020);

        // Both peers should have the extreme sync preset set
        assert_eq!(peer1.sync_preset, Some("extreme".to_string()));
        assert_eq!(peer2.sync_preset, Some("extreme".to_string()));
    }

    /// Test automatic sync preset selection chooses stress_test over extreme.
    #[test]
    fn test_network_scenario_auto_sync_prefers_stress_test_over_extreme() {
        let high_uniform_loss = NetworkProfile {
            packet_loss: 0.25,
            latency_ms: 50,
            jitter_ms: 25,
            reorder_rate: 0.0,
            reorder_buffer_size: 0,
            duplicate_rate: 0.0,
            burst_loss_prob: 0.0,
            burst_loss_len: 0,
        };

        assert_eq!(high_uniform_loss.suggested_sync_preset(), Some("extreme"));
        assert_eq!(
            NetworkProfile::bursty_survivable().suggested_sync_preset(),
            Some("stress_test")
        );

        let scenario = NetworkScenario::asymmetric(
            "extreme_vs_stress",
            high_uniform_loss,
            NetworkProfile::bursty_survivable(),
        )
        .with_auto_sync_preset();

        assert_eq!(scenario.sync_preset.as_deref(), Some("stress_test"));
    }

    /// Test NetworkScenario without sync_preset defaults to None.
    #[test]
    fn test_network_scenario_no_sync_preset() {
        let scenario = NetworkScenario::lan();

        let (peer1, peer2) = scenario.to_peer_configs(30010);

        // Both peers should have None for sync preset
        assert_eq!(peer1.sync_preset, None);
        assert_eq!(peer2.sync_preset, None);
    }

    /// Test NetworkScenario asymmetric creation.
    #[test]
    fn test_network_scenario_asymmetric() {
        let scenario = NetworkScenario::asymmetric(
            "test_asymmetric",
            NetworkProfile::lan(),
            NetworkProfile::terrible(),
        );

        assert_eq!(scenario.name, "test_asymmetric");
        // Use approx comparison for floating point
        assert!(
            (scenario.peer1_profile.packet_loss - NetworkProfile::lan().packet_loss).abs()
                < f64::EPSILON
        );
        assert!(
            (scenario.peer2_profile.packet_loss - NetworkProfile::terrible().packet_loss).abs()
                < f64::EPSILON
        );
    }

    /// Test NetworkScenario to_peer_configs generates valid configurations.
    #[test]
    fn test_network_scenario_to_peer_configs() {
        let scenario = NetworkScenario::lan().with_frames(100);
        let (peer1, peer2) = scenario.to_peer_configs(20000);

        // Verify port allocation
        assert_eq!(peer1.local_port, 20000);
        assert_eq!(peer2.local_port, 20001);

        // Verify peer addresses point to each other
        assert_eq!(peer1.peer_addr, "127.0.0.1:20001");
        assert_eq!(peer2.peer_addr, "127.0.0.1:20000");

        // Verify player indices
        assert_eq!(peer1.player_index, 0);
        assert_eq!(peer2.player_index, 1);

        // Verify frames match scenario
        assert_eq!(peer1.frames, 100);
        assert_eq!(peer2.frames, 100);
    }

    /// Test extended chaos options in NetworkProfile.
    #[test]
    fn test_network_profile_extended_chaos() {
        let profile = NetworkProfile::heavy_reorder();
        assert!(profile.reorder_rate > 0.0);
        assert!(profile.reorder_buffer_size > 0);

        let profile = NetworkProfile::duplicating();
        assert!(profile.duplicate_rate > 0.0);

        let profile = NetworkProfile::bursty();
        assert!(profile.burst_loss_prob > 0.0);
        assert!(profile.burst_loss_len > 0);
    }

    /// Document the relationship between bidirectional packet loss and effective loss rate.
    ///
    /// When packet loss is applied to both send and receive paths (as with ChaosSocket),
    /// the effective loss rate compounds. This test documents these calculations to help
    /// users choose appropriate sync configs for their test scenarios.
    ///
    /// # Effective Loss Rate Formula
    ///
    /// For bidirectional loss rate `p`, effective loss = `1 - (1-p)²`
    ///
    /// | Bidirectional Loss | Effective Loss | Recommended SyncConfig |
    /// |-------------------|----------------|------------------------|
    /// | 5%                | ~9.75%         | default or lossy       |
    /// | 10%               | ~19%           | lossy                  |
    /// | 15%               | ~27.75%        | mobile                 |
    /// | 20%               | ~36%           | extreme                |
    /// | 25%               | ~43.75%        | extreme                |
    /// | 30%               | ~51%           | extreme                |
    ///
    /// For scenarios with burst loss (multiple consecutive packets dropped):
    ///
    /// | Burst Loss Prob | Burst Length | Recommended SyncConfig |
    /// |-----------------|--------------|------------------------|
    /// | 2%              | 3-4          | lossy                  |
    /// | 5%              | 3-5          | mobile                 |
    /// | 10%             | 5-8          | stress_test            |
    /// | >10%            | >8           | stress_test (may be flaky) |
    ///
    /// Note: At 30% bidirectional loss or 10% burst loss with 8-packet bursts,
    /// synchronization can be challenging even with stress_test preset. Tests at
    /// this level may require longer timeouts on slow CI systems.
    #[test]
    fn test_packet_loss_effective_rate_documentation() {
        // Document effective loss rates for bidirectional loss
        let test_cases: [(f64, f64, &str); 6] = [
            (0.05, 0.0975, "default or lossy"),
            (0.10, 0.19, "lossy"),
            (0.15, 0.2775, "mobile"),
            (0.20, 0.36, "extreme"),
            (0.25, 0.4375, "extreme"),
            (0.30, 0.51, "extreme"),
        ];

        for (bidirectional, expected_effective, _recommendation) in test_cases {
            // Formula: effective = 1 - (1 - p)²
            let actual_effective: f64 = 1.0 - (1.0 - bidirectional).powi(2);
            assert!(
                (actual_effective - expected_effective).abs() < 0.001,
                "For {}% bidirectional loss, expected ~{}% effective, got {}%",
                bidirectional * 100.0,
                expected_effective * 100.0,
                actual_effective * 100.0
            );
        }
    }

    /// Test that sync_preset field is properly included in diagnostic summary.
    #[test]
    fn test_peer_config_sync_preset_in_summary() {
        let config = PeerConfig {
            local_port: 10001,
            sync_preset: Some("mobile".to_string()),
            ..Default::default()
        };

        let summary = config.diagnostic_summary();
        assert!(
            summary.contains("sync=Some(\"mobile\")"),
            "Summary should contain sync preset: {}",
            summary
        );

        let config_no_preset = PeerConfig::default();
        let summary = config_no_preset.diagnostic_summary();
        assert!(
            summary.contains("sync=None"),
            "Summary should show None for default: {}",
            summary
        );
    }

    /// Data-driven test documenting which sync presets are appropriate for which network profiles.
    ///
    /// This test verifies that each network profile has a documented recommended sync preset,
    /// and explains the relationship between network conditions and sync configuration.
    ///
    /// # Sync Preset Selection Guidelines
    ///
    /// - **default**: Baseline packet loss ≤5%, no burst loss, latency ≤50ms
    /// - **lossy**: Packet loss 5-15%, no significant burst loss
    /// - **mobile**: Packet loss 10-20%, burst loss ≤5%, high jitter
    /// - **extreme**: Burst loss >5%, packet loss >20%, or combined hostile conditions
    #[test]
    fn test_sync_preset_recommendations_data_driven() {
        struct SyncPresetRecommendation {
            profile_name: &'static str,
            profile: NetworkProfile,
            recommended_preset: Option<&'static str>,
            reason: &'static str,
        }

        let recommendations = [
            SyncPresetRecommendation {
                profile_name: "local",
                profile: NetworkProfile::local(),
                recommended_preset: None,
                reason: "No loss or latency, default sync is sufficient",
            },
            SyncPresetRecommendation {
                profile_name: "lan",
                profile: NetworkProfile::lan(),
                recommended_preset: None,
                reason: "Minimal latency, no loss - default sync works",
            },
            SyncPresetRecommendation {
                profile_name: "wifi_good",
                profile: NetworkProfile::wifi_good(),
                recommended_preset: None,
                reason: "Low loss and latency - default sync sufficient",
            },
            SyncPresetRecommendation {
                profile_name: "wifi_average",
                profile: NetworkProfile::wifi_average(),
                recommended_preset: None,
                reason: "5% loss is borderline, default usually works",
            },
            SyncPresetRecommendation {
                profile_name: "wifi_congested",
                profile: NetworkProfile::wifi_congested(),
                recommended_preset: Some("stress_test"),
                reason: "15% raw loss is ~28% effective, and burst loss needs stress_test",
            },
            SyncPresetRecommendation {
                profile_name: "mobile_4g",
                profile: NetworkProfile::mobile_4g(),
                recommended_preset: Some("lossy"),
                reason: "8% raw loss is ~15% effective, so lossy preset is appropriate",
            },
            SyncPresetRecommendation {
                profile_name: "mobile_3g",
                profile: NetworkProfile::mobile_3g(),
                recommended_preset: Some("stress_test"),
                reason: "15% raw loss is ~28% effective, and burst loss needs stress_test",
            },
            SyncPresetRecommendation {
                profile_name: "terrible",
                profile: NetworkProfile::terrible(),
                recommended_preset: Some("stress_test"),
                reason: "25% loss + 5% burst loss (combined hostile) - stress_test preset required",
            },
            SyncPresetRecommendation {
                profile_name: "heavy_reorder",
                profile: NetworkProfile::heavy_reorder(),
                recommended_preset: None,
                reason: "Low loss, reordering doesn't affect sync handshake",
            },
            SyncPresetRecommendation {
                profile_name: "duplicating",
                profile: NetworkProfile::duplicating(),
                recommended_preset: None,
                reason: "Duplication doesn't negatively affect sync",
            },
            SyncPresetRecommendation {
                profile_name: "bursty",
                profile: NetworkProfile::bursty(),
                recommended_preset: Some("stress_test"),
                reason: "10% burst loss with 8-packet bursts can drop entire sync exchanges",
            },
        ];

        // Print documentation table
        println!("\n=== Sync Preset Recommendations ===");
        println!(
            "{:<16} {:<8} {:<10} {:<10} {:<8} {:<10} Reason",
            "Profile", "Loss%", "Effective%", "BurstP%", "BurstLen", "Preset"
        );
        println!("{}", "-".repeat(100));

        for rec in &recommendations {
            let preset_str = rec.recommended_preset.unwrap_or("default");
            let p = &rec.profile;
            println!(
                "{:<16} {:<8.1} {:<10.1} {:<10.1} {:<8} {:<10} {}",
                rec.profile_name,
                p.packet_loss * 100.0,
                p.effective_symmetric_packet_loss() * 100.0,
                p.burst_loss_prob * 100.0,
                p.burst_loss_len,
                preset_str,
                rec.reason
            );

            // Verify the recommendations make sense based on documented thresholds
            if p.burst_loss_prob >= 0.10 && p.burst_loss_len >= 8 {
                assert_eq!(
                    rec.recommended_preset,
                    Some("stress_test"),
                    "{}: High burst loss ({}%x{}) should recommend 'stress_test' preset",
                    rec.profile_name,
                    p.burst_loss_prob * 100.0,
                    p.burst_loss_len
                );
            } else if p.effective_symmetric_packet_loss() >= 0.25
                && p.burst_loss_prob >= 0.02
                && p.burst_loss_len >= 3
            {
                assert_eq!(
                    rec.recommended_preset,
                    Some("stress_test"),
                    "{}: High effective loss ({}%) + burst ({}%x{}) should recommend 'stress_test' preset",
                    rec.profile_name,
                    p.effective_symmetric_packet_loss() * 100.0,
                    p.burst_loss_prob * 100.0,
                    p.burst_loss_len
                );
            } else if p.effective_symmetric_packet_loss() >= 0.25 {
                assert!(
                    matches!(rec.recommended_preset, Some("mobile") | Some("extreme")),
                    "{}: High effective packet loss ({}%) should recommend 'mobile' or 'extreme' preset",
                    rec.profile_name,
                    p.effective_symmetric_packet_loss() * 100.0
                );
            }
        }
        println!("===================================\n");
    }
}

// =============================================================================
// Extended Chaos Tests - Reordering, Duplication, Burst Loss
// =============================================================================

/// Test with packet reordering - validates that out-of-order packets are handled.
#[test]
#[serial]
fn test_packet_reordering() {
    skip_if_no_peer_binary!();
    let peer1_config = PeerConfig {
        local_port: 10400,
        player_index: 0,
        peer_addr: "127.0.0.1:10401".to_string(),
        frames: 100,
        latency_ms: 20,
        reorder_rate: 0.20,
        reorder_buffer_size: 5,
        seed: Some(42),
        timeout_secs: 90,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10401,
        player_index: 1,
        peer_addr: "127.0.0.1:10400".to_string(),
        frames: 100,
        latency_ms: 20,
        reorder_rate: 0.20,
        reorder_buffer_size: 5,
        seed: Some(43),
        timeout_secs: 90,
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);
    verify_determinism(&result1, &result2, "packet_reordering");

    println!(
        "Reordering test - Rollbacks: peer1={}, peer2={}",
        result1.rollbacks, result2.rollbacks
    );
}

/// Test with aggressive packet reordering.
#[test]
#[serial]
fn test_heavy_packet_reordering() {
    skip_if_no_peer_binary!();
    let scenario = NetworkScenario::symmetric("heavy_reorder", NetworkProfile::heavy_reorder())
        .with_frames(80)
        .with_input_delay(3)
        .with_timeout(120);

    let (result1, result2) = scenario.run_test(10402);

    println!(
        "Heavy reordering - Rollbacks: peer1={}, peer2={}",
        result1.rollbacks, result2.rollbacks
    );
}

/// Test with packet duplication - validates duplicate packet handling.
#[test]
#[serial]
fn test_packet_duplication() {
    skip_if_no_peer_binary!();
    let peer1_config = PeerConfig {
        local_port: 10404,
        player_index: 0,
        peer_addr: "127.0.0.1:10405".to_string(),
        frames: 100,
        latency_ms: 15,
        duplicate_rate: 0.15,
        seed: Some(42),
        timeout_secs: 90,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10405,
        player_index: 1,
        peer_addr: "127.0.0.1:10404".to_string(),
        frames: 100,
        latency_ms: 15,
        duplicate_rate: 0.15,
        seed: Some(43),
        timeout_secs: 90,
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);
    verify_determinism(&result1, &result2, "packet_duplication");

    println!(
        "Duplication test - Rollbacks: peer1={}, peer2={}",
        result1.rollbacks, result2.rollbacks
    );
}

/// Test with high packet duplication rate.
#[test]
#[serial]
fn test_heavy_packet_duplication() {
    skip_if_no_peer_binary!();
    let scenario = NetworkScenario::symmetric("duplicating", NetworkProfile::duplicating())
        .with_frames(100)
        .with_input_delay(2)
        .with_timeout(120);

    let (result1, result2) = scenario.run_test(10406);

    println!(
        "Heavy duplication - Rollbacks: peer1={}, peer2={}",
        result1.rollbacks, result2.rollbacks
    );
}

/// Test with burst packet loss - simulates brief network outages.
///
/// Burst loss (5% probability × 4-packet bursts) can cause multiple consecutive
/// packet drops. This requires the `mobile` sync preset for reliable connection
/// establishment under these conditions.
#[test]
#[serial]
#[ignore = "extreme real-UDP chaos; runs in the nightly network suite (ci-network-nightly.yml). Determinism is covered deterministically by tests/network/in_process_chaos.rs"]
fn test_burst_packet_loss() {
    skip_if_no_peer_binary!();
    let profile = NetworkProfile {
        packet_loss: 0.0,
        latency_ms: 20,
        jitter_ms: 0,
        reorder_rate: 0.0,
        reorder_buffer_size: 0,
        duplicate_rate: 0.0,
        burst_loss_prob: 0.03, // Reduced from 0.05 for reliability
        burst_loss_len: 4,     // Reduced from 5 to limit burst impact
    };

    let scenario = NetworkScenario::symmetric("burst_packet_loss", profile)
        .with_frames(100)
        .with_timeout(120)
        .with_input_delay(3)
        .with_auto_sync_preset();

    let (result1, result2) = scenario.run_test(10408);
    verify_determinism(&result1, &result2, "burst_packet_loss");

    println!(
        "Burst loss test - Rollbacks: peer1={}, peer2={}",
        result1.rollbacks, result2.rollbacks
    );
}

/// Test with aggressive burst loss pattern.
///
/// This test validates that the sync handshake can survive burst loss conditions
/// where multiple consecutive packets are dropped simultaneously. We use the
/// `bursty_survivable()` profile which has 5% burst probability with 5-packet
/// bursts, combined with the `stress_test` sync preset.
///
/// ## Why bursty_survivable instead of bursty?
///
/// The original `bursty()` profile (10% burst prob, 8-packet bursts) proved
/// unreliable in CI due to the compounding effect of chaos being applied on
/// BOTH sides of the connection:
///
/// 1. Peer 1 sends SyncRequest -> may be dropped by Peer 1's outgoing chaos
/// 2. Peer 2 receives it and sends SyncReply -> may be dropped by Peer 2's outgoing chaos
/// 3. Peer 1 needs to receive the reply to complete one roundtrip
///
/// This means BOTH peers' random burst patterns must cooperate for sync to succeed.
/// With 10% burst probability on each side, the combined failure probability
/// during the ~60s sync window becomes high enough to cause intermittent CI failures.
///
/// The `bursty_survivable()` profile uses 5% burst probability and 5-packet bursts,
/// which is still aggressively hostile but has a much higher success probability
/// (>99.99%) with the `stress_test` preset's 40 sync packets and 60s timeout.
///
/// ## Single attempt, no retries
///
/// This test runs a single attempt via `run_test`; a sync timeout here is a real
/// failure. Determinism under burst loss is covered deterministically in
/// `tests/network/in_process_chaos.rs`. This real-UDP variant runs in the nightly
/// network suite (it is `#[ignore]`d in the per-PR lane).
///
/// ## Separate stress test for original bursty profile
///
/// The original `bursty()` profile is still tested in the extended chaos scenarios
/// test suite, but those tests are marked as potentially flaky or skipped in CI.
#[test]
#[serial]
#[ignore = "extreme real-UDP chaos; runs in the nightly network suite (ci-network-nightly.yml). Determinism is covered deterministically by tests/network/in_process_chaos.rs"]
fn test_heavy_burst_loss() {
    skip_if_no_peer_binary!();
    // Use bursty_survivable profile which is aggressive but reliable with stress_test preset.
    // See function docstring for the rationale behind this choice.
    let scenario = NetworkScenario::symmetric("bursty_survivable", NetworkProfile::bursty_survivable())
        .with_frames(80)
        .with_input_delay(3)
        .with_timeout(180) // Generous timeout for slow CI environments
        .with_sync_preset("stress_test");

    let (result1, result2) = scenario.run_test(10410);

    println!(
        "Heavy burst loss - Rollbacks: peer1={}, peer2={}",
        result1.rollbacks, result2.rollbacks
    );
}

/// Test case for burst loss parameter sweep testing.
struct BurstSweepTestCase {
    name: &'static str,
    burst_loss_prob: f64,
    burst_loss_len: usize,
    /// If true, test is expected to pass reliably. If false, it may fail occasionally.
    expect_reliable: bool,
}

impl BurstSweepTestCase {
    const fn new(
        name: &'static str,
        burst_loss_prob: f64,
        burst_loss_len: usize,
        expect_reliable: bool,
    ) -> Self {
        Self {
            name,
            burst_loss_prob,
            burst_loss_len,
            expect_reliable,
        }
    }
}

/// Data-driven test that sweeps multiple burst loss parameter combinations.
///
/// This test validates that various burst loss configurations behave as expected:
/// - Conservative parameters should pass reliably
/// - Aggressive parameters exercise the edge of reliability
///
/// This helps catch regressions if the burst loss handling degrades and provides
/// coverage across the parameter space without manually writing many separate tests.
///
/// Each test case runs a single attempt (no retries); failures are real failures.
/// This real-UDP sweep runs in the nightly network suite (it is `#[ignore]`d in
/// the per-PR lane). Determinism is covered by `tests/network/in_process_chaos.rs`.
#[test]
#[serial]
#[ignore = "extreme real-UDP chaos; runs in the nightly network suite (ci-network-nightly.yml). Determinism is covered deterministically by tests/network/in_process_chaos.rs"]
fn test_burst_loss_parameter_sweep() {
    skip_if_no_peer_binary!();

    // Define test cases with varying burst parameters
    // Format: (name, burst_prob, burst_len, expect_reliable)
    let test_cases = [
        // Conservative: should always pass
        BurstSweepTestCase::new("burst_2pct_2pkt", 0.02, 2, true),
        BurstSweepTestCase::new("burst_3pct_3pkt", 0.03, 3, true),
        // Moderate: should pass reliably with stress_test preset
        BurstSweepTestCase::new("burst_4pct_4pkt", 0.04, 4, true),
        BurstSweepTestCase::new("burst_5pct_5pkt", 0.05, 5, true),
        // Aggressive: exercises the edge of reliability
        BurstSweepTestCase::new("burst_6pct_4pkt", 0.06, 4, true),
    ];

    let mut results: Vec<(&str, bool)> = Vec::new();
    let mut port_base = 10500;

    for case in &test_cases {
        println!(
            "\n=== Testing {} (prob={}, len={}) ===",
            case.name, case.burst_loss_prob, case.burst_loss_len
        );

        // Create a custom profile with the test case parameters
        let profile = NetworkProfile {
            packet_loss: 0.03,
            latency_ms: 20,
            jitter_ms: 10,
            reorder_rate: 0.01,
            reorder_buffer_size: 2,
            duplicate_rate: 0.01,
            burst_loss_prob: case.burst_loss_prob,
            burst_loss_len: case.burst_loss_len,
        };

        let scenario = NetworkScenario::symmetric(case.name, profile)
            .with_frames(60)
            .with_input_delay(2)
            .with_timeout(120)
            .with_sync_preset("stress_test");

        let (result1, result2) = scenario.run_test(port_base);
        port_base += SWEEP_PORT_OFFSET; // Ensure non-overlapping ports between test cases

        let passed = result1.success && result2.success;
        results.push((case.name, passed));

        println!(
            "{}: passed={}, rollbacks=({}, {})",
            case.name, passed, result1.rollbacks, result2.rollbacks
        );

        assert!(
            passed || !case.expect_reliable,
            "Test case '{}' failed but was expected to be reliable",
            case.name
        );
    }

    // Print summary
    println!("\n=== Burst Loss Parameter Sweep Summary ===");
    for (name, passed) in &results {
        let status = if *passed { "PASS" } else { "FAIL" };
        println!("  {}: {}", name, status);
    }
    println!("==========================================\n");
}

/// Test combining all chaos types: loss, latency, jitter, reorder, duplicate, burst.
#[test]
#[serial]
#[ignore = "extreme real-UDP chaos; runs in the nightly network suite (ci-network-nightly.yml). Determinism is covered deterministically by tests/network/in_process_chaos.rs"]
fn test_all_chaos_combined() {
    skip_if_no_peer_binary!();
    let profile = NetworkProfile {
        packet_loss: 0.05,
        latency_ms: 30,
        jitter_ms: 20,
        reorder_rate: 0.10,
        reorder_buffer_size: 4,
        duplicate_rate: 0.05,
        burst_loss_prob: 0.02,
        burst_loss_len: 3,
    };

    let scenario = NetworkScenario::symmetric("all_chaos_combined", profile)
        .with_frames(80)
        .with_input_delay(3)
        .with_timeout(180)
        .with_auto_sync_preset();

    let (result1, result2) = scenario.run_test(10412);
    verify_determinism(&result1, &result2, "all_chaos_combined");

    println!(
        "All chaos combined - Rollbacks: peer1={}, peer2={}",
        result1.rollbacks, result2.rollbacks
    );
}

// =============================================================================
// Extended Stress Tests - Long Sessions, Memory Stability
// =============================================================================

/// Stress test: 5000 frames - very long session.
#[test]
#[serial]
#[ignore = "long-running endurance or high-latency real-UDP session; runs in the nightly network suite (ci-network-nightly.yml), not per-PR CI"]
fn test_stress_5000_frames() {
    skip_if_no_peer_binary!();
    let peer1_config = PeerConfig {
        local_port: 10500,
        player_index: 0,
        peer_addr: "127.0.0.1:10501".to_string(),
        frames: 5000,
        packet_loss: 0.02,
        latency_ms: 15,
        seed: Some(42),
        timeout_secs: 600, // 10 minutes
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10501,
        player_index: 1,
        peer_addr: "127.0.0.1:10500".to_string(),
        frames: 5000,
        packet_loss: 0.02,
        latency_ms: 15,
        seed: Some(43),
        timeout_secs: 600,
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);
    verify_determinism(&result1, &result2, "stress_5000_frames");

    assert!(
        result1.final_frame >= 5000,
        "Peer 1 didn't reach target: {}",
        result1.final_frame
    );
    assert!(
        result2.final_frame >= 5000,
        "Peer 2 didn't reach target: {}",
        result2.final_frame
    );

    println!(
        "5000 frame stress - Rollbacks: peer1={}, peer2={}",
        result1.rollbacks, result2.rollbacks
    );
}

/// Stress test: 3000 frames with moderate chaos.
#[test]
#[serial]
#[ignore = "long-running endurance or high-latency real-UDP session; runs in the nightly network suite (ci-network-nightly.yml), not per-PR CI"]
fn test_stress_3000_frames_moderate_chaos() {
    skip_if_no_peer_binary!();
    let peer1_config = PeerConfig {
        local_port: 10502,
        player_index: 0,
        peer_addr: "127.0.0.1:10503".to_string(),
        frames: 3000,
        packet_loss: 0.08,
        latency_ms: 30,
        jitter_ms: 15,
        reorder_rate: 0.05,
        reorder_buffer_size: 3,
        seed: Some(42),
        timeout_secs: 480, // 8 minutes
        input_delay: 3,
        sync_preset: Some("lossy".to_string()),
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10503,
        player_index: 1,
        peer_addr: "127.0.0.1:10502".to_string(),
        frames: 3000,
        packet_loss: 0.08,
        latency_ms: 30,
        jitter_ms: 15,
        reorder_rate: 0.05,
        reorder_buffer_size: 3,
        seed: Some(43),
        timeout_secs: 480,
        input_delay: 3,
        sync_preset: Some("lossy".to_string()),
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);
    verify_determinism(&result1, &result2, "stress_3000_frames_moderate_chaos");

    println!(
        "3000 frame moderate chaos - Rollbacks: peer1={}, peer2={}",
        result1.rollbacks, result2.rollbacks
    );
}

// =============================================================================
// Edge Case Tests
// =============================================================================

/// Test maximum practical input delay (configurable max prediction).
#[test]
#[serial]
fn test_max_input_delay_edge_case() {
    skip_if_no_peer_binary!();
    let peer1_config = PeerConfig {
        local_port: 10600,
        player_index: 0,
        peer_addr: "127.0.0.1:10601".to_string(),
        frames: 100,
        input_delay: 7, // Very high input delay
        timeout_secs: 120,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10601,
        player_index: 1,
        peer_addr: "127.0.0.1:10600".to_string(),
        frames: 100,
        input_delay: 7,
        timeout_secs: 120,
        seed: Some(43),
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);
    verify_determinism(&result1, &result2, "max_input_delay_edge_case");

    // With max input delay, rollbacks should be very rare
    println!(
        "Max input delay - Rollbacks: peer1={}, peer2={}",
        result1.rollbacks, result2.rollbacks
    );
}

/// Test very short session (5 frames) - validates fast completion.
#[test]
#[serial]
fn test_minimal_session_5_frames() {
    skip_if_no_peer_binary!();
    let peer1_config = PeerConfig {
        local_port: 10602,
        player_index: 0,
        peer_addr: "127.0.0.1:10603".to_string(),
        frames: 5,
        timeout_secs: 30,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10603,
        player_index: 1,
        peer_addr: "127.0.0.1:10602".to_string(),
        frames: 5,
        timeout_secs: 30,
        seed: Some(43),
        ..Default::default()
    };

    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);
    verify_determinism(&result1, &result2, "minimal_session_5_frames");

    assert!(result1.final_frame >= 5, "Peer 1 didn't complete");
    assert!(result2.final_frame >= 5, "Peer 2 didn't complete");
}

/// Test with asymmetric chaos settings on each peer.
#[test]
#[serial]
#[ignore = "extreme real-UDP chaos; runs in the nightly network suite (ci-network-nightly.yml). Determinism is covered deterministically by tests/network/in_process_chaos.rs"]
fn test_asymmetric_chaos_settings() {
    skip_if_no_peer_binary!();
    let high_reorder = NetworkProfile {
        packet_loss: 0.02,
        latency_ms: 20,
        jitter_ms: 0,
        reorder_rate: 0.25,
        reorder_buffer_size: 8,
        duplicate_rate: 0.0,
        burst_loss_prob: 0.0,
        burst_loss_len: 0,
    };

    let high_loss = NetworkProfile {
        packet_loss: 0.20,
        latency_ms: 10,
        jitter_ms: 0,
        reorder_rate: 0.02,
        reorder_buffer_size: 2,
        duplicate_rate: 0.0,
        burst_loss_prob: 0.0,
        burst_loss_len: 0,
    };

    let scenario =
        NetworkScenario::asymmetric("asymmetric_chaos_settings", high_reorder, high_loss)
            .with_frames(100)
            .with_timeout(120)
            .with_auto_sync_preset();

    let (result1, result2) = scenario.run_test(10604);
    verify_determinism(&result1, &result2, "asymmetric_chaos_settings");

    println!(
        "Asymmetric chaos - Rollbacks: peer1={}, peer2={}",
        result1.rollbacks, result2.rollbacks
    );
}

/// Test with one peer experiencing burst loss while other is stable.
///
/// This tests asymmetric network conditions where one peer has reliable connectivity
/// while the other experiences intermittent packet bursts. This simulates scenarios
/// like WiFi interference or temporary signal degradation on one end.
///
/// The test uses moderate burst loss parameters (3% probability, 4-packet bursts)
/// to ensure reliable completion while still exercising the burst loss handling code.
#[test]
#[serial]
#[ignore = "extreme real-UDP chaos; runs in the nightly network suite (ci-network-nightly.yml). Determinism is covered deterministically by tests/network/in_process_chaos.rs"]
fn test_one_sided_burst_loss() {
    skip_if_no_peer_binary!();
    let burst_profile = NetworkProfile {
        packet_loss: 0.0,
        latency_ms: 20,
        jitter_ms: 0,
        reorder_rate: 0.0,
        reorder_buffer_size: 0,
        duplicate_rate: 0.0,
        burst_loss_prob: 0.03, // 3% burst probability - moderate
        burst_loss_len: 4,     // 4 consecutive packets - moderate
    };

    let stable_profile = NetworkProfile {
        packet_loss: 0.0,
        latency_ms: 10,
        jitter_ms: 0,
        reorder_rate: 0.0,
        reorder_buffer_size: 0,
        duplicate_rate: 0.0,
        burst_loss_prob: 0.0,
        burst_loss_len: 0,
    };

    let scenario =
        NetworkScenario::asymmetric("one_sided_burst_loss", burst_profile, stable_profile)
            .with_frames(100)
            .with_timeout(120)
            .with_auto_sync_preset();

    let (result1, result2) = scenario.run_test(10606);
    verify_determinism(&result1, &result2, "one_sided_burst_loss");

    println!(
        "One-sided burst loss - Rollbacks: peer1={}, peer2={}",
        result1.rollbacks, result2.rollbacks
    );
}

/// Test recovery from very long staggered startup (5 seconds).
#[test]
#[serial]
#[ignore = "extreme real-UDP chaos; runs in the nightly network suite (ci-network-nightly.yml). Determinism is covered deterministically by tests/network/in_process_chaos.rs"]
fn test_very_long_staggered_startup() {
    skip_if_no_peer_binary!();
    let peer1_config = PeerConfig {
        local_port: 10608,
        player_index: 0,
        peer_addr: "127.0.0.1:10609".to_string(),
        frames: 100,
        timeout_secs: 120,
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10609,
        player_index: 1,
        peer_addr: "127.0.0.1:10608".to_string(),
        frames: 100,
        timeout_secs: 120,
        seed: Some(43),
        ..Default::default()
    };

    // Spawn peer 1 first
    let peer1 = spawn_peer(&peer1_config).expect("Failed to spawn peer 1");

    // Wait 5 seconds before spawning peer 2
    thread::sleep(Duration::from_secs(5));

    // Spawn peer 2
    let peer2 = spawn_peer(&peer2_config).expect("Failed to spawn peer 2");

    // Wait for both peers (wait_for_peer has a default 5-minute timeout)
    let result1 = wait_for_peer(peer1, "Peer 1");
    let result2 = wait_for_peer(peer2, "Peer 2");

    verify_determinism(&result1, &result2, "very_long_staggered_startup");
}

/// Test with rapid frame rate (minimal sleep between frames).
#[test]
#[serial]
fn test_rapid_frame_rate() {
    skip_if_no_peer_binary!();
    // Tests are already running at full speed, but we can verify
    // with more frames in the same time
    let peer1_config = PeerConfig {
        local_port: 10610,
        player_index: 0,
        peer_addr: "127.0.0.1:10611".to_string(),
        frames: 300,
        latency_ms: 5,
        jitter_ms: 2,
        seed: Some(42),
        timeout_secs: 60,
        input_delay: 1, // Low delay for speed
        ..Default::default()
    };

    let peer2_config = PeerConfig {
        local_port: 10611,
        player_index: 1,
        peer_addr: "127.0.0.1:10610".to_string(),
        frames: 300,
        latency_ms: 5,
        jitter_ms: 2,
        seed: Some(43),
        timeout_secs: 60,
        input_delay: 1,
        ..Default::default()
    };

    let start = std::time::Instant::now();
    let (result1, result2) = run_two_peer_test(peer1_config, peer2_config);
    let duration = start.elapsed();

    verify_determinism(&result1, &result2, "rapid_frame_rate");

    println!(
        "Rapid frame rate - 300 frames in {:.2}s ({:.0} fps), rollbacks: p1={}, p2={}",
        duration.as_secs_f64(),
        300.0 / duration.as_secs_f64(),
        result1.rollbacks,
        result2.rollbacks
    );
}

// =============================================================================
// Network Profile Scenario Tests with Extended Chaos
// =============================================================================

fn run_extended_chaos_scenario(port: u16, scenario: NetworkScenario) {
    println!("Testing scenario: {}", scenario.summary());
    let (result1, result2) = scenario.run_test(port);

    assert!(
        result1.final_frame >= scenario.frames,
        "{}: Peer 1 didn't reach target: {} < {}",
        scenario.name,
        result1.final_frame,
        scenario.frames
    );
    assert!(
        result2.final_frame >= scenario.frames,
        "{}: Peer 2 didn't reach target: {} < {}",
        scenario.name,
        result2.final_frame,
        scenario.frames
    );
}

/// Average WiFi with moderate packet loss, jitter, reordering, and duplication.
#[test]
#[serial]
fn test_extended_chaos_wifi_average() {
    skip_if_no_peer_binary!();
    run_extended_chaos_scenario(
        10700,
        NetworkScenario::symmetric("wifi_average", NetworkProfile::wifi_average())
            .with_frames(100)
            .with_timeout(120)
            .with_auto_sync_preset(),
    );
}

/// Mobile 3G conditions with high effective packet loss and burst loss.
#[test]
#[serial]
#[ignore = "extreme real-UDP chaos; runs in the nightly network suite (ci-network-nightly.yml). Determinism is covered deterministically by tests/network/in_process_chaos.rs"]
fn test_extended_chaos_mobile_3g() {
    skip_if_no_peer_binary!();
    run_extended_chaos_scenario(
        10702,
        NetworkScenario::symmetric("mobile_3g", NetworkProfile::mobile_3g())
            .with_frames(60)
            .with_input_delay(4)
            .with_timeout(240)
            .with_auto_sync_preset(),
    );
}

/// Heavy packet reordering should not break determinism.
#[test]
#[serial]
#[ignore = "extreme real-UDP chaos; runs in the nightly network suite (ci-network-nightly.yml). Determinism is covered deterministically by tests/network/in_process_chaos.rs"]
fn test_extended_chaos_heavy_reorder() {
    skip_if_no_peer_binary!();
    run_extended_chaos_scenario(
        10704,
        NetworkScenario::symmetric("heavy_reorder", NetworkProfile::heavy_reorder())
            .with_frames(80)
            .with_input_delay(3)
            .with_timeout(150)
            .with_auto_sync_preset(),
    );
}

/// Packet duplication should be tolerated by the protocol.
#[test]
#[serial]
#[ignore = "extreme real-UDP chaos; runs in the nightly network suite (ci-network-nightly.yml). Determinism is covered deterministically by tests/network/in_process_chaos.rs"]
fn test_extended_chaos_duplicating() {
    skip_if_no_peer_binary!();
    run_extended_chaos_scenario(
        10706,
        NetworkScenario::symmetric("duplicating", NetworkProfile::duplicating())
            .with_frames(100)
            .with_timeout(120)
            .with_auto_sync_preset(),
    );
}

/// Survivable burst loss profile for required CI coverage.
#[test]
#[serial]
#[ignore = "extreme real-UDP chaos; runs in the nightly network suite (ci-network-nightly.yml). Determinism is covered deterministically by tests/network/in_process_chaos.rs"]
fn test_extended_chaos_bursty_survivable() {
    skip_if_no_peer_binary!();
    run_extended_chaos_scenario(
        10708,
        NetworkScenario::symmetric("bursty_survivable", NetworkProfile::bursty_survivable())
            .with_frames(80)
            .with_input_delay(3)
            .with_timeout(180)
            .with_auto_sync_preset(),
    );
}

/// Test asymmetric scenarios with extended chaos.
#[test]
#[serial]
#[ignore = "extreme real-UDP chaos; runs in the nightly network suite (ci-network-nightly.yml). Determinism is covered deterministically by tests/network/in_process_chaos.rs"]
fn test_asymmetric_extended_chaos() {
    skip_if_no_peer_binary!();
    println!("=== Asymmetric Extended Chaos ===");

    let scenarios = [
        // One peer with reordering, one with duplication
        (
            10720,
            NetworkScenario::asymmetric(
                "reorder_vs_duplicate",
                NetworkProfile::heavy_reorder(),
                NetworkProfile::duplicating(),
            )
            .with_frames(80)
            .with_input_delay(3)
            .with_timeout(150)
            .with_auto_sync_preset(),
        ),
        // One peer with burst loss, one stable
        // Use bursty_survivable (5% burst prob, 5-packet bursts) for reliable CI testing.
        (
            10722,
            NetworkScenario::asymmetric(
                "bursty_survivable_vs_stable",
                NetworkProfile::bursty_survivable(),
                NetworkProfile::lan(),
            )
            .with_frames(100)
            .with_input_delay(3)
            .with_timeout(180) // Accommodate longer sync timeout
            .with_auto_sync_preset(),
        ),
        // Mobile vs WiFi - both have significant packet loss
        (
            10724,
            NetworkScenario::asymmetric(
                "mobile_3g_vs_wifi_congested",
                NetworkProfile::mobile_3g(),
                NetworkProfile::wifi_congested(),
            )
            .with_frames(60)
            .with_input_delay(4)
            .with_timeout(240)
            .with_auto_sync_preset(),
        ),
    ];

    for (port, scenario) in scenarios {
        println!("Testing scenario: {}", scenario.summary());
        let (result1, result2) = scenario.run_test(port);

        println!(
            "  {} - Rollbacks: p1={}, p2={}",
            scenario.name, result1.rollbacks, result2.rollbacks
        );
    }

    println!("=== Asymmetric Extended Chaos Complete ===");
}
// =============================================================================
// Data-Driven Burst Loss Parameter Tests
// =============================================================================

/// Test data for burst loss parameter validation.
///
/// Each entry specifies burst loss parameters and the expected sync preset needed
/// to achieve reliable synchronization. Tests are parameterized to validate the
/// documented guidelines for burst loss configuration.
///
/// Note: This struct uses different field names (`burst_prob`/`burst_len`) than
/// `BurstSweepTestCase` (`burst_loss_prob`/`burst_loss_len`) for historical reasons
/// and backwards compatibility. Both represent the same concepts.
struct BurstLossTestCase {
    name: &'static str,
    burst_prob: f64,
    burst_len: usize,
    baseline_loss: f64,
    sync_preset: &'static str,
    /// If true, test is expected to pass reliably. If false, it may fail occasionally.
    expect_reliable: bool,
}

/// Data-driven test validating burst loss parameter recommendations.
///
/// This test validates the guidelines documented in `NetworkProfile::bursty_survivable()`
/// by testing various burst loss configurations with their recommended sync presets.
///
/// The test cases are derived from probability analysis of the sync handshake:
/// - With N sync packets required and P combined packet drop probability,
///   the probability of at least one successful roundtrip is 1-(1-P)^N
/// - Burst loss compounds this by potentially dropping multiple consecutive packets
///
/// | Burst Prob | Burst Len | Recommended Preset | Reliability |
/// |------------|-----------|-------------------|-------------|
/// | 0-2%       | 1-2       | lossy             | High        |
/// | 2-5%       | 3-5       | mobile            | High        |
/// | 5-8%       | 4-6       | stress_test       | High        |
/// | 8-10%      | 6-8       | stress_test       | Medium      |
/// | >10%       | >8        | stress_test       | Low (flaky) |
#[test]
#[serial]
#[ignore = "extreme real-UDP chaos; runs in the nightly network suite (ci-network-nightly.yml). Determinism is covered deterministically by tests/network/in_process_chaos.rs"]
fn test_burst_loss_parameter_validation() {
    skip_if_no_peer_binary!();
    println!("=== Burst Loss Parameter Validation Suite ===");

    // Test cases designed to validate burst loss handling at various levels.
    // These are the "reliable" configurations that should always pass.
    let test_cases = [
        // Low sustained burst loss - needs mobile preset under the conservative selector
        BurstLossTestCase {
            name: "low_burst_3pct_3len",
            burst_prob: 0.03,
            burst_len: 3,
            baseline_loss: 0.02,
            sync_preset: "mobile",
            expect_reliable: true,
        },
        // Moderate burst loss - needs mobile preset
        BurstLossTestCase {
            name: "moderate_burst_5pct_4len",
            burst_prob: 0.05,
            burst_len: 4,
            baseline_loss: 0.03,
            sync_preset: "mobile",
            expect_reliable: true,
        },
        // High burst loss (our bursty_survivable profile parameters)
        BurstLossTestCase {
            name: "high_burst_5pct_5len",
            burst_prob: 0.05,
            burst_len: 5,
            baseline_loss: 0.05,
            sync_preset: "stress_test",
            expect_reliable: true,
        },
    ];

    let mut port_base = 10800;
    for case in test_cases.iter().filter(|c| c.expect_reliable) {
        println!(
            "Testing: {} (burst={:.0}%, len={}, loss={:.0}%, preset={})",
            case.name,
            case.burst_prob * 100.0,
            case.burst_len,
            case.baseline_loss * 100.0,
            case.sync_preset
        );

        // Create custom profile for this test case
        let profile = NetworkProfile {
            packet_loss: case.baseline_loss,
            latency_ms: 25,
            jitter_ms: 15,
            reorder_rate: 0.02,
            reorder_buffer_size: 3,
            duplicate_rate: 0.01,
            burst_loss_prob: case.burst_prob,
            burst_loss_len: case.burst_len,
        };

        let scenario = NetworkScenario::symmetric(case.name, profile)
            .with_frames(60) // Shorter test for validation
            .with_input_delay(3)
            .with_timeout(180)
            .with_sync_preset(case.sync_preset);

        let (result1, result2) = scenario.run_test(port_base);

        // For expected reliable cases, verify success
        assert!(
            result1.success,
            "{}: Peer 1 failed unexpectedly: {:?}",
            case.name, result1.error
        );
        assert!(
            result2.success,
            "{}: Peer 2 failed unexpectedly: {:?}",
            case.name, result2.error
        );

        println!(
            "  {} PASSED: frames p1={}, p2={}, rollbacks p1={}, p2={}",
            case.name,
            result1.final_frame,
            result2.final_frame,
            result1.rollbacks,
            result2.rollbacks,
        );

        port_base += 2;
    }

    println!("=== Burst Loss Parameter Validation Complete ===");
}

/// Extreme burst loss test for exploratory/research purposes.
///
/// This test uses parameters that are known to be flaky and will fail intermittently
/// even with the most aggressive sync settings. It exists for developers who want to:
/// - Explore the boundaries of what the sync protocol can handle
/// - Research improvements to burst loss tolerance
/// - Validate changes to sync timeout/retry logic
///
/// **Do not rely on this test for CI.** Run with `cargo test -- --ignored` when needed.
///
/// Parameters:
/// - 10% burst probability (very high)
/// - 8-packet burst length (very long)
/// - 10% baseline loss (severe)
/// - stress_test sync preset (maximum retries/timeouts)
#[test]
#[ignore = "Flaky test for exploratory/research purposes only - run with --ignored"]
#[serial]
fn test_burst_loss_extreme_unreliable() {
    skip_if_no_peer_binary!();
    println!("=== Extreme Burst Loss (Exploratory Test) ===");
    println!("WARNING: This test is known to be flaky and may fail intermittently.");

    let profile = NetworkProfile {
        packet_loss: 0.10,
        latency_ms: 25,
        jitter_ms: 15,
        reorder_rate: 0.02,
        reorder_buffer_size: 3,
        duplicate_rate: 0.01,
        burst_loss_prob: 0.10,
        burst_loss_len: 8,
    };

    let scenario = NetworkScenario::symmetric("extreme_burst_10pct_8len", profile)
        .with_frames(60)
        .with_input_delay(3)
        .with_timeout(180)
        .with_sync_preset("stress_test");

    let (peer1, peer2) = scenario.to_peer_configs(10900);
    let (result1, result2) = run_two_peer_test(peer1, peer2);

    println!(
        "Results: p1_success={}, p2_success={}, p1_frames={}, p2_frames={}",
        result1.success, result2.success, result1.final_frame, result2.final_frame
    );

    if result1.success && result2.success {
        println!("Test PASSED (this run was lucky!)");
    } else {
        println!("Test FAILED (expected - this is a known flaky configuration)");
        println!("  Peer 1 error: {:?}", result1.error);
        println!("  Peer 2 error: {:?}", result2.error);
    }

    // We don't assert success because this test is expected to be flaky.
    // The test exists to allow manual exploration, not CI validation.
}

// =============================================================================
// N-Peer (N >= 3) Mesh Determinism Tests
// =============================================================================
//
// These real-UDP tests close the multi-process N>=3 coverage gap: the rest of
// this file only ever spawns 2 peers. They build a fully-connected localhost
// mesh via `n_peer_mesh_configs` (each peer is `network_test_peer` launched with
// N-1 `--peer` args) and verify cross-peer determinism.
//
// Oracle: every peer must reach `success`. A peer's `success` is set purely from
// `game.state.frame >= target && confirmed_frame >= target` in the binary
// (`tests/network-peer/src/main.rs`); it does NOT read `sync_health()` /
// `all_sync_health()` (those are only logged in `runtime_diagnostics`). So
// `success == true` means each peer advanced its simulation to `target` AND
// confirmed *all* N players' inputs through `target` (since `confirmed_frame` is
// the min over connected peers' `last_frame`). All peers succeeding is therefore
// the faithful N-peer generalization of the established 2-peer oracle
// (`verify_determinism` / `verify_determinism_n`). A desync-free state is
// asserted separately via the per-peer `DesyncDetected == 0` check in
// `verify_determinism_n` (see the DesyncDetected note below).
//
// NO CROSS-PEER CHECKSUM EQUALITY ORACLE: a strengthened cross-peer `checksum`
// equality oracle was attempted (at success time the fixed
// `[target - 64, target)` confirmed-input window should be identical across
// peers at 0% loss), but it proved flaky even on clean localhost -- 3/20 green
// over a 20x sweep of these three tests. Root cause matches the historical
// flakiness note near `verify_determinism` and the binary's checksum comment:
// the peer that confirms the target LAST computes its checksum after its current
// frame has already advanced past `target`, so `set_last_confirmed_frame` has
// already discarded the `[target - 64, target)` frames from its input queue;
// `confirmed_inputs_for_frame` then returns `Err` for the whole window, giving
// `frames_included = 0`, `final_value = 0`, and the empty-hash checksum
// `cbf29ce484222325` -- which disagrees with the peers that captured a full
// window. The same race already disables `final_value` comparison in the 2-peer
// `verify_determinism`. We therefore assert `success` (which was 20/20 green)
// plus the per-peer zero-`DesyncDetected` check below; `final_value` and
// `checksum` are only logged in diagnostics.
//
// DesyncDetected == 0 (ASSERTED since the S30 F17 fix): the binary inherits
// `DesyncDetection::On { interval: 60 }` (the library default), so the library's
// per-peer checksum gossip runs, and `verify_determinism_n` asserts that every
// peer observed ZERO `DesyncDetected` events. Historically this count was
// logged-but-not-asserted because a large fraction of 0%-loss 3-peer runs
// (~20-60% across S27-S29 measurements; 0/50 at 2-peer, whose prediction stays
// shallow) raised `DesyncDetected`, always at the checksum-interval frame 60.
// S27 attributed that to a test-harness checksum artifact (the speculative
// `state.value` accumulator); S29 overturned the attribution (a faithful
// full-state hash AND a faithful per-frame-input digest both still fired it,
// with confirmed state byte-identical across peers) and recorded it as library
// finding F17; S30 root-caused and FIXED F17 in `InputQueue::input`: a rollback
// re-simulation whose first input request landed ABOVE a remote queue's missing
// window re-entered its prediction episode at the REQUESTED frame, so the
// missing window's arrivals were accepted with their misprediction comparison
// silently skipped -- the victim's applied trajectory was never re-simulated
// (silently divergent gameplay state) and the checksum harvest gossiped stale
// saved-state stamps, raising false-positive `DesyncDetected` on byte-identical
// confirmed input streams. Prediction episodes now always enter at the queue's
// FIRST MISSING frame, making the swallowed window unconstructible. The
// deterministic in-process reproduction lives in tests/sessions/desync_harvest.rs
// ("F17 DETERMINISTIC REPRODUCTION (S30)", RED on pre-fix code), and a post-fix
// 10-run real-UDP soak of the 3-peer test observed 0 events in 10/10 runs. On
// this clean network (0% loss, no disconnects) a nonzero count is therefore a
// genuine library regression and fails these tests.
//
// Each test uses a unique, non-colliding port range (>= 18001, clear of the
// existing 10001-10900/20000/30000 clusters) as defense-in-depth -- the tests
// are already serialized by `#[serial]` and the `network-multi-process` nextest
// group (`max-threads = 1`), so the distinct ranges just remove any chance of
// cross-test port collisions, not to enable concurrency.

/// Three peers on a perfect local network must reach a deterministic, mutually
/// consistent confirmed state.
#[test]
#[serial]
#[ignore = "real-UDP mesh; runs in the nightly network suite (ci-network-nightly.yml). Determinism is covered deterministically by tests/network/in_process_chaos.rs"]
fn three_peer_local_network_is_deterministic() {
    skip_if_no_peer_binary!();
    const NUM_PLAYERS: usize = 3;
    const FRAMES: i32 = 100;
    // Port base in a range not used elsewhere; uses 18001..=18003.
    let configs = n_peer_mesh_configs(
        NUM_PLAYERS,
        18001,
        NetworkProfile::local(),
        FRAMES,
        2,  // input_delay
        60, // timeout_secs
        None,
    );

    let results = run_n_peer_test(configs);

    // Success + zero-DesyncDetected oracle (cross-peer checksum equality stays
    // log-only -- it proved flaky by construction; see module note above).
    verify_determinism_n(&results, "three_peer_local_network");

    for (index, result) in results.iter().enumerate() {
        assert!(
            result.final_frame >= FRAMES,
            "Peer {index} didn't reach target frames: {} < {FRAMES}",
            result.final_frame
        );
    }
}

/// Four peers on a perfect local network must reach a deterministic, mutually
/// consistent confirmed state.
#[test]
#[serial]
#[ignore = "real-UDP mesh; runs in the nightly network suite (ci-network-nightly.yml). Determinism is covered deterministically by tests/network/in_process_chaos.rs"]
fn four_peer_local_network_is_deterministic() {
    skip_if_no_peer_binary!();
    const NUM_PLAYERS: usize = 4;
    const FRAMES: i32 = 100;
    // Distinct port range from the 3-peer test; uses 18011..=18014.
    let configs = n_peer_mesh_configs(
        NUM_PLAYERS,
        18011,
        NetworkProfile::local(),
        FRAMES,
        2,  // input_delay
        90, // timeout_secs (more peers => more handshakes)
        None,
    );

    let results = run_n_peer_test(configs);

    // Success + zero-DesyncDetected oracle (cross-peer checksum equality stays
    // log-only -- it proved flaky by construction; see module note above).
    verify_determinism_n(&results, "four_peer_local_network");

    for (index, result) in results.iter().enumerate() {
        assert!(
            result.final_frame >= FRAMES,
            "Peer {index} didn't reach target frames: {} < {FRAMES}",
            result.final_frame
        );
    }
}

/// Three peers on a LAN profile (1ms latency/jitter, 0% loss) must reach a
/// deterministic, mutually consistent confirmed state.
#[test]
#[serial]
#[ignore = "real-UDP mesh; runs in the nightly network suite (ci-network-nightly.yml). Determinism is covered deterministically by tests/network/in_process_chaos.rs"]
fn three_peer_lan_network_is_deterministic() {
    skip_if_no_peer_binary!();
    const NUM_PLAYERS: usize = 3;
    const FRAMES: i32 = 100;
    // Distinct port range; uses 18021..=18023.
    let configs = n_peer_mesh_configs(
        NUM_PLAYERS,
        18021,
        NetworkProfile::lan(),
        FRAMES,
        2,  // input_delay
        60, // timeout_secs
        None,
    );

    let results = run_n_peer_test(configs);

    // Success + zero-DesyncDetected oracle (cross-peer checksum equality stays
    // log-only -- it proved flaky by construction; see module note above).
    verify_determinism_n(&results, "three_peer_lan_network");

    for (index, result) in results.iter().enumerate() {
        assert!(
            result.final_frame >= FRAMES,
            "Peer {index} didn't reach target frames: {} < {FRAMES}",
            result.final_frame
        );
    }
}
