//! Deterministic, data-driven in-process network-chaos tests.
//!
//! This module replaces the historically flaky real-UDP "extended chaos"
//! coverage with a **fully deterministic, fast, seeded, in-process
//! simulation** that cannot time out or flake. It is the primary chaos
//! coverage going forward.
//!
//! # Why this is deterministic
//!
//! Like [`resilience`](super::resilience), this module uses
//! [`ChannelSocket`](crate::common::ChannelSocket) (in-memory sockets),
//! [`TestClock`](crate::common::TestClock) (virtual time) and
//! [`ChaosSocket`] (seeded chaos) so that there is **no** real UDP I/O, **no**
//! `thread::sleep`, **no** `Instant::now()`, and **no** `#[serial]`. Every run
//! is bit-for-bit reproducible.
//!
//! # Seed Correlation Warning
//!
//! As documented at length in [`resilience`](super::resilience), the two peers
//! **must** use different chaos seeds. Identical seeds produce correlated drop
//! sequences that systematically block synchronization and deadlock. Every
//! scenario below uses `seed` for peer 1 and `seed + 1` for peer 2.
//!
//! # The core value: the confirmed-input determinism + checksum assertion
//!
//! The ground truth for determinism is the **confirmed input stream**, which
//! must be identical on both peers regardless of any network chaos. For each
//! scenario we:
//!
//! 1. Synchronize both sessions (bounded, virtual-time iteration cap).
//! 2. Advance both until they reach a target confirmed frame.
//! 3. For every confirmed frame `f`, assert
//!    `sess1.confirmed_inputs_for_frame(f) == sess2.confirmed_inputs_for_frame(f)`,
//!    folding the inputs into a per-peer FNV-1a checksum and asserting the two
//!    peers' checksums are equal.
//! 4. Assert each peer's `GameStub` state actually advanced.
//! 5. **Reproducibility**: re-run the whole scenario from scratch and assert
//!    the resulting checksum is bit-identical to the first run.

// Allow test-specific patterns (mirrors resilience.rs).
#![allow(
    clippy::print_stderr,
    clippy::disallowed_macros,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::ip_constant,
    clippy::too_many_lines,
    clippy::use_self
)]

use crate::common::stubs::{GameStub, StubConfig, StubInput};
use crate::common::{create_chaos_channel_mesh, create_chaos_channel_pair, TestClock};
use fortress_rollback::hash::fnv1a_hash;
use fortress_rollback::{
    ChaosConfig, FortressError, Frame, P2PSession, PlayerHandle, PlayerType, ProtocolConfig,
    RequestVec, SessionBuilder, SessionState, SyncConfig, TimeSyncConfig,
};
use std::time::Duration;

/// Folds an `advance_frame()` result into the stub, panicking on ANY error.
///
/// `P2PSession::advance_frame` surfaces the prediction-window throttle (a peer
/// racing ahead under chaos) as `Ok(<empty requests>)` — a no-op — **not** as an
/// `Err`, so it needs no special-casing here. The chaos harness also disables
/// disconnect detection (a disconnect timeout provably larger than the whole
/// virtual-time budget), so under correct operation `advance_frame` cannot
/// legitimately error. Any `Err` is therefore a genuine failure (a spurious
/// disconnect, a missing local input, an internal invariant break) and must
/// turn the test RED rather than be silently absorbed.
///
/// This is strictly stronger than the historical `if let Ok(..)` swallow-all
/// (which would hide such an error as silent non-progress) and satisfies the
/// agent-preflight `advance-frame-error-handling` rule (mirroring
/// `handle_advance_frame_allowing` in `resilience.rs`, which likewise panics on
/// an unexpected error).
fn fold_advance_frame(result: Result<RequestVec<StubConfig>, FortressError>, stub: &mut GameStub) {
    match result {
        Ok(requests) => stub.handle_requests(requests),
        Err(err) => panic!("unexpected advance_frame error under chaos: {err:?}"),
    }
}

/// Helper: creates a `ProtocolConfig` with the given test clock.
///
/// Mirrors the `protocol_config` helper in `resilience.rs`, but also lets the
/// caller select a preset analogous to the peer binary's
/// `protocol_config_for_preset`.
fn protocol_config(clock: &TestClock, preset: SyncPreset) -> ProtocolConfig {
    let base = match preset {
        SyncPreset::Mobile | SyncPreset::StressTest => ProtocolConfig::mobile(),
        SyncPreset::HighLatency => ProtocolConfig::high_latency(),
        SyncPreset::Default | SyncPreset::Lan | SyncPreset::Lossy | SyncPreset::Competitive => {
            ProtocolConfig::default()
        },
    };
    ProtocolConfig {
        clock: Some(clock.as_protocol_clock()),
        ..base
    }
}

/// Maps a preset to the matching [`TimeSyncConfig`], mirroring the peer
/// binary's `time_sync_config_for_preset`.
fn time_sync_config(preset: SyncPreset) -> TimeSyncConfig {
    match preset {
        SyncPreset::Lan => TimeSyncConfig::lan(),
        SyncPreset::Competitive => TimeSyncConfig::competitive(),
        SyncPreset::Mobile | SyncPreset::StressTest | SyncPreset::HighLatency => {
            TimeSyncConfig::mobile()
        },
        SyncPreset::Default | SyncPreset::Lossy => TimeSyncConfig::default(),
    }
}

/// Sync configuration preset, matching the peer binary's preset names and the
/// `suggested_sync_preset` logic in `multi_process.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SyncPreset {
    Default,
    Lan,
    Lossy,
    Mobile,
    HighLatency,
    Competitive,
    StressTest,
}

impl SyncPreset {
    fn sync_config(self) -> SyncConfig {
        match self {
            SyncPreset::Default => SyncConfig::default(),
            SyncPreset::Lan => SyncConfig::lan(),
            SyncPreset::Lossy => SyncConfig::lossy(),
            SyncPreset::Mobile => SyncConfig::mobile(),
            SyncPreset::HighLatency => SyncConfig::high_latency(),
            SyncPreset::Competitive => SyncConfig::competitive(),
            SyncPreset::StressTest => SyncConfig::stress_test(),
        }
    }
}

/// A single set of chaos parameters applied to one peer's socket.
///
/// Field semantics match the `NetworkProfile` in `multi_process.rs`.
#[derive(Debug, Clone, Copy)]
struct ChaosProfile {
    packet_loss: f64,
    latency_ms: u64,
    jitter_ms: u64,
    reorder_rate: f64,
    reorder_buffer_size: usize,
    duplicate_rate: f64,
    burst_loss_prob: f64,
    burst_loss_len: usize,
}

impl ChaosProfile {
    /// Builds a deterministic [`ChaosConfig`] for this profile with the given
    /// seed. (Callers must use different seeds for the two peers.)
    fn to_chaos_config(self, seed: u64) -> ChaosConfig {
        let mut builder = ChaosConfig::builder()
            .packet_loss_rate(self.packet_loss)
            .latency_ms(self.latency_ms)
            .jitter_ms(self.jitter_ms)
            .duplication_rate(self.duplicate_rate)
            .seed(seed);
        if self.reorder_buffer_size > 0 {
            builder = builder
                .reorder_buffer_size(self.reorder_buffer_size)
                .reorder_rate(self.reorder_rate);
        }
        if self.burst_loss_prob > 0.0 && self.burst_loss_len > 0 {
            builder = builder.burst_loss(self.burst_loss_prob, self.burst_loss_len);
        }
        builder.build()
    }

    // --- Profiles ported from multi_process.rs NetworkProfile ----------------

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
}

/// One row of the data-driven scenario table.
#[derive(Debug, Clone, Copy)]
struct ChaosScenario {
    /// Human-readable scenario name (used in diagnostics).
    name: &'static str,
    /// Chaos applied to peer 1's socket.
    peer1: ChaosProfile,
    /// Chaos applied to peer 2's socket.
    peer2: ChaosProfile,
    /// Base seed; peer 1 uses `seed`, peer 2 uses `seed + 1` (never identical).
    seed: u64,
    /// Target confirmed frame both peers must reach.
    target_confirmed: i32,
    /// Per-player input delay frames.
    input_delay: usize,
    /// Sync configuration preset to use for both peers.
    preset: SyncPreset,
}

/// The full data-driven scenario table.
///
/// Symmetric rows cover every `NetworkProfile` from `multi_process.rs`
/// (including the extreme `mobile_3g`, `terrible`, and `bursty` profiles).
/// Asymmetric rows cover mixed-quality combinations. Sync presets are chosen
/// the same way `multi_process.rs::suggested_sync_preset` does.
fn scenarios() -> Vec<ChaosScenario> {
    use SyncPreset::{Competitive, Default, Lan, Lossy, Mobile, StressTest};
    vec![
        // --- Symmetric profiles (both peers identical conditions) -----------
        ChaosScenario {
            name: "local",
            peer1: ChaosProfile::local(),
            peer2: ChaosProfile::local(),
            seed: 100,
            target_confirmed: 40,
            input_delay: 2,
            preset: Lan,
        },
        ChaosScenario {
            name: "lan",
            peer1: ChaosProfile::lan(),
            peer2: ChaosProfile::lan(),
            seed: 200,
            target_confirmed: 40,
            input_delay: 2,
            preset: Lan,
        },
        ChaosScenario {
            name: "wifi_good",
            peer1: ChaosProfile::wifi_good(),
            peer2: ChaosProfile::wifi_good(),
            seed: 300,
            target_confirmed: 40,
            input_delay: 2,
            // Low-latency, low-loss link: exercise the competitive preset.
            preset: Competitive,
        },
        ChaosScenario {
            name: "wifi_average",
            peer1: ChaosProfile::wifi_average(),
            peer2: ChaosProfile::wifi_average(),
            seed: 400,
            target_confirmed: 40,
            input_delay: 2,
            preset: Default,
        },
        ChaosScenario {
            // effective loss ~27.8% + burst -> mobile
            name: "wifi_congested",
            peer1: ChaosProfile::wifi_congested(),
            peer2: ChaosProfile::wifi_congested(),
            seed: 500,
            target_confirmed: 35,
            input_delay: 2,
            preset: Mobile,
        },
        ChaosScenario {
            name: "mobile_4g",
            peer1: ChaosProfile::mobile_4g(),
            peer2: ChaosProfile::mobile_4g(),
            seed: 600,
            target_confirmed: 35,
            input_delay: 2,
            preset: Mobile,
        },
        ChaosScenario {
            // effective loss ~27.8% + burst -> mobile (extreme profile)
            name: "mobile_3g",
            peer1: ChaosProfile::mobile_3g(),
            peer2: ChaosProfile::mobile_3g(),
            seed: 700,
            target_confirmed: 30,
            input_delay: 2,
            preset: Mobile,
        },
        ChaosScenario {
            name: "intercontinental",
            peer1: ChaosProfile::intercontinental(),
            peer2: ChaosProfile::intercontinental(),
            seed: 800,
            target_confirmed: 35,
            input_delay: 2,
            preset: Default,
        },
        // NOTE: the symmetric ~43.75%-loss `terrible` profile (and the
        // `mobile_4g_vs_terrible` pairing below) are intentionally NOT in this
        // deterministic table. At ~67% effective two-way loss the protocol
        // correctly near-deadlocks (confirms only 2-3 frames in the entire
        // virtual budget), so it cannot serve as a meaningful determinism test
        // (the checksum would cover a near-vacuous range). "Does the protocol
        // survive at all under catastrophic loss" is exercised by the nightly
        // real-UDP suite instead; this table covers the meaningful determinism
        // range up to ~27.8% effective loss (mobile_3g / wifi_congested).
        ChaosScenario {
            name: "heavy_reorder",
            peer1: ChaosProfile::heavy_reorder(),
            peer2: ChaosProfile::heavy_reorder(),
            seed: 1000,
            target_confirmed: 35,
            input_delay: 2,
            preset: Lossy,
        },
        ChaosScenario {
            name: "duplicating",
            peer1: ChaosProfile::duplicating(),
            peer2: ChaosProfile::duplicating(),
            seed: 1100,
            target_confirmed: 40,
            input_delay: 2,
            preset: Default,
        },
        ChaosScenario {
            // 10% burst prob, 8-packet bursts -> stress_test
            name: "bursty",
            peer1: ChaosProfile::bursty(),
            peer2: ChaosProfile::bursty(),
            seed: 1200,
            target_confirmed: 25,
            input_delay: 2,
            preset: StressTest,
        },
        ChaosScenario {
            // 5% burst prob, 5-packet bursts -> stress_test
            name: "bursty_survivable",
            peer1: ChaosProfile::bursty_survivable(),
            peer2: ChaosProfile::bursty_survivable(),
            seed: 1300,
            target_confirmed: 30,
            input_delay: 2,
            preset: StressTest,
        },
        // --- Asymmetric profiles (one direction worse than the other) -------
        ChaosScenario {
            name: "asymmetric_mobile_3g_vs_wifi_congested",
            peer1: ChaosProfile::mobile_3g(),
            peer2: ChaosProfile::wifi_congested(),
            seed: 1400,
            target_confirmed: 30,
            input_delay: 2,
            preset: Mobile,
        },
        ChaosScenario {
            name: "asymmetric_one_perfect_one_terrible",
            peer1: ChaosProfile::local(),
            peer2: ChaosProfile::terrible(),
            seed: 1500,
            target_confirmed: 25,
            input_delay: 2,
            preset: StressTest,
        },
        ChaosScenario {
            name: "asymmetric_lan_vs_extreme_latency",
            peer1: ChaosProfile::lan(),
            peer2: ChaosProfile::intercontinental(),
            seed: 1600,
            target_confirmed: 30,
            input_delay: 4,
            preset: SyncPreset::HighLatency,
        },
        // (asymmetric_mobile_4g_vs_terrible removed — see the `terrible` note
        // above: ~catastrophic loss near-deadlocks and is covered by the
        // nightly real-UDP suite, not this deterministic determinism table.)
    ]
}

/// The result of running a single scenario once.
struct ScenarioRun {
    /// `true` if both peers reached the target confirmed frame.
    reached_target: bool,
    /// Confirmed frame reached by peer 1.
    confirmed1: i32,
    /// Confirmed frame reached by peer 2.
    confirmed2: i32,
    /// Current frame on peer 1.
    current1: i32,
    /// Current frame on peer 2.
    current2: i32,
    /// Final `GameStub` frame on peer 1.
    stub_frame1: i32,
    /// Final `GameStub` frame on peer 2.
    stub_frame2: i32,
    /// Per-peer checksum over the shared confirmed-input range.
    checksum1: u64,
    /// Per-peer checksum over the shared confirmed-input range.
    checksum2: u64,
    /// First frame index at which confirmed inputs diverged, if any.
    first_divergence: Option<i32>,
    /// Final session states for diagnostics.
    state1: SessionState,
    state2: SessionState,
}

/// Builds two synced-or-not P2P sessions for the scenario, runs the simulation
/// to the target confirmed frame, and computes the determinism checksum.
///
/// This is the single source of truth used by both the initial run and the
/// reproducibility re-run.
fn execute_scenario_once(s: &ChaosScenario) -> ScenarioRun {
    let clock = TestClock::new();

    // Always different seeds for the two peers (see seed-correlation warning).
    let config1 = s.peer1.to_chaos_config(s.seed);
    let config2 = s.peer2.to_chaos_config(s.seed + 1);

    let (socket1, socket2, addr1, addr2) = create_chaos_channel_pair(config1, config2, &clock);

    // The simulation advances virtual time well past 200s (the sync loop alone
    // can advance 6000 * 20ms = 120s, the advance loop 6000 * 16ms = 96s). The
    // real disconnect detector has no analog here — there is no peer that died,
    // only a chaos socket dropping packets — so we set the disconnect timeout
    // PROVABLY larger than the whole virtual-time budget. Otherwise an extreme
    // inter-delivery gap (in virtual time) would trip a spurious disconnect,
    // downgrade the session to Synchronizing and break the confirmed-input
    // invariant. Virtual time is free, so a huge timeout costs nothing.
    //
    // NOTE: this deliberately disables disconnect detection in this suite;
    // real disconnect/reconnect behavior is covered by
    // `test_temporary_disconnect_reconnect` and friends in `resilience.rs`. The
    // `confirmed >= 0` (no spurious downgrade) invariant asserted in `check_run`
    // would still catch a disconnect bug that fired despite this timeout.
    let disconnect_timeout = Duration::from_secs(100_000);
    let disconnect_notify = Duration::from_secs(50_000);

    let mut sess1 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock, s.preset))
        .with_sync_config(s.preset.sync_config())
        .with_time_sync_config(time_sync_config(s.preset))
        .with_disconnect_timeout(disconnect_timeout)
        .with_disconnect_notify_delay(disconnect_notify)
        .with_input_delay(s.input_delay)
        .expect("valid input delay")
        .add_player(PlayerType::Local, PlayerHandle::new(0))
        .expect("add local player 0")
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(1))
        .expect("add remote player 1")
        .start_p2p_session(socket1)
        .expect("start p2p session 1");

    let mut sess2 = SessionBuilder::<StubConfig>::new()
        .with_protocol_config(protocol_config(&clock, s.preset))
        .with_sync_config(s.preset.sync_config())
        .with_time_sync_config(time_sync_config(s.preset))
        .with_disconnect_timeout(disconnect_timeout)
        .with_disconnect_notify_delay(disconnect_notify)
        .with_input_delay(s.input_delay)
        .expect("valid input delay")
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))
        .expect("add remote player 0")
        .add_player(PlayerType::Local, PlayerHandle::new(1))
        .expect("add local player 1")
        .start_p2p_session(socket2)
        .expect("start p2p session 2");

    // --- Synchronize (bounded, virtual-time iteration cap) ------------------
    // Virtual time is free, so the caps below are generous on purpose. Poll
    // several times per clock tick to give the chaos buffers (reorder /
    // duplicate / burst) repeated chances to deliver under extreme loss.
    for _ in 0..6000 {
        for _ in 0..4 {
            sess1.poll_remote_clients();
            sess2.poll_remote_clients();
        }
        if sess1.current_state() == SessionState::Running
            && sess2.current_state() == SessionState::Running
        {
            break;
        }
        clock.advance(Duration::from_millis(20));
    }

    let mut stub1 = GameStub::new();
    let mut stub2 = GameStub::new();

    // --- Advance until both reach the target confirmed frame ----------------
    if sess1.current_state() == SessionState::Running
        && sess2.current_state() == SessionState::Running
    {
        let mut frame_input: u32 = 0;
        for _ in 0..6000 {
            if sess1.confirmed_frame().as_i32() >= s.target_confirmed
                && sess2.confirmed_frame().as_i32() >= s.target_confirmed
            {
                break;
            }

            // Poll a few times per advance to help drain chaos buffers.
            for _ in 0..10 {
                sess1.poll_remote_clients();
                sess2.poll_remote_clients();
            }
            clock.advance(Duration::from_millis(16));

            // Deterministic, frame-derived inputs (independent of wall clock).
            let input1 = StubInput {
                inp: frame_input.wrapping_mul(7).wrapping_add(1),
            };
            let input2 = StubInput {
                inp: frame_input.wrapping_mul(11).wrapping_add(3),
            };

            // add_local_input only rejects a non-local handle (the prediction
            // throttle is applied by advance_frame, which returns Ok(<empty>)
            // when the window is full), so under correct wiring these always
            // succeed; the `added*` guard is defensive. Only advance when both
            // inputs were accepted this tick.
            let added1 = sess1.add_local_input(PlayerHandle::new(0), input1).is_ok();
            let added2 = sess2.add_local_input(PlayerHandle::new(1), input2).is_ok();
            if !(added1 && added2) {
                continue;
            }

            fold_advance_frame(sess1.advance_frame(), &mut stub1);
            fold_advance_frame(sess2.advance_frame(), &mut stub2);
            frame_input = frame_input.wrapping_add(1);
        }
    }

    // --- Drain any in-flight packets so confirmations settle ----------------
    for _ in 0..200 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        clock.advance(Duration::from_millis(16));
    }

    let confirmed1 = sess1.confirmed_frame().as_i32();
    let confirmed2 = sess2.confirmed_frame().as_i32();

    // --- The core determinism + checksum assertion --------------------------
    // Compare confirmed inputs frame-by-frame over the shared confirmed range.
    let shared = confirmed1.min(confirmed2);
    let (checksum1, checksum2, first_divergence) =
        confirmed_input_checksums(&sess1, &sess2, shared);

    ScenarioRun {
        reached_target: confirmed1 >= s.target_confirmed && confirmed2 >= s.target_confirmed,
        confirmed1,
        confirmed2,
        current1: sess1.current_frame().as_i32(),
        current2: sess2.current_frame().as_i32(),
        stub_frame1: stub1.gs.frame,
        stub_frame2: stub2.gs.frame,
        checksum1,
        checksum2,
        first_divergence,
        state1: sess1.current_state(),
        state2: sess2.current_state(),
    }
}

/// Folds each peer's confirmed inputs over `0..=shared` into an FNV-1a
/// checksum and reports the first frame at which the two streams diverge.
fn confirmed_input_checksums(
    sess1: &P2PSession<StubConfig>,
    sess2: &P2PSession<StubConfig>,
    shared: i32,
) -> (u64, u64, Option<i32>) {
    // StubInput does not implement Hash, so fold its raw `inp` values (which
    // fully define the input) into u32 vectors for the FNV-1a checksum.
    let mut acc1: Vec<u32> = Vec::new();
    let mut acc2: Vec<u32> = Vec::new();
    let mut first_divergence: Option<i32> = None;

    let mut f = 0;
    while f <= shared {
        let frame = Frame::new(f);
        let inputs1 = sess1.confirmed_inputs_for_frame(frame);
        let inputs2 = sess2.confirmed_inputs_for_frame(frame);
        match (inputs1, inputs2) {
            (Ok(i1), Ok(i2)) => {
                if first_divergence.is_none() && i1 != i2 {
                    first_divergence = Some(f);
                }
                acc1.extend(i1.iter().map(|input| input.inp));
                acc2.extend(i2.iter().map(|input| input.inp));
            },
            // If either side cannot produce confirmed inputs for a frame within
            // the shared range, record divergence and stop accumulating.
            _ => {
                if first_divergence.is_none() {
                    first_divergence = Some(f);
                }
                break;
            },
        }
        f += 1;
    }

    (fnv1a_hash(&acc1), fnv1a_hash(&acc2), first_divergence)
}

/// Renders a rich, self-explanatory diagnostic block for a failing scenario.
fn diagnostics(s: &ChaosScenario, run: &ScenarioRun) -> String {
    format!(
        "scenario '{name}' FAILED\n  \
         seeds: peer1={seed1}, peer2={seed2} (must differ)\n  \
         preset: {preset:?}, input_delay={delay}, target_confirmed={target}\n  \
         peer1 profile: {p1:?}\n  \
         peer2 profile: {p2:?}\n  \
         peer1: state={state1:?} current_frame={cur1} confirmed_frame={conf1} stub_frame={stub1} checksum={cs1:#018x}\n  \
         peer2: state={state2:?} current_frame={cur2} confirmed_frame={conf2} stub_frame={stub2} checksum={cs2:#018x}\n  \
         reached_target={reached}\n  \
         first_confirmed_input_divergence={div:?}",
        name = s.name,
        seed1 = s.seed,
        seed2 = s.seed + 1,
        preset = s.preset,
        delay = s.input_delay,
        target = s.target_confirmed,
        p1 = s.peer1,
        p2 = s.peer2,
        state1 = run.state1,
        cur1 = run.current1,
        conf1 = run.confirmed1,
        stub1 = run.stub_frame1,
        cs1 = run.checksum1,
        state2 = run.state2,
        cur2 = run.current2,
        conf2 = run.confirmed2,
        stub2 = run.stub_frame2,
        cs2 = run.checksum2,
        reached = run.reached_target,
        div = run.first_divergence,
    )
}

/// Minimum confirmed frames EVERY scenario must reach, so the cross-peer
/// determinism checksum is never computed over an empty/trivial range (which
/// would make the equality assertion vacuously pass). Even the most hostile
/// profile in this table must confirm at least this many frames within the
/// (generous, virtual-time) budget, or it is treated as a real failure.
const MIN_CONFIRMED_FLOOR: i32 = 10;

/// Checks every per-run invariant for a single completed run.
///
/// Every scenario in the table must satisfy ALL of these on its single fixed
/// seed (there is no seed search and no per-row exemption: a profile that
/// cannot deterministically reach its target on a fixed seed does not belong
/// in this table — catastrophic-loss profiles that merely near-deadlock are
/// covered by the nightly real-UDP suite instead):
///   1. a non-vacuous confirmed range (so the checksum below is meaningful,
///      and `confirmed >= MIN_CONFIRMED_FLOOR > 0` also proves there was no
///      spurious disconnect downgrade back to `Synchronizing`),
///   2. no confirmed-input divergence between peers,
///   3. matching cross-peer determinism checksums,
///   4. both `GameStub`s advanced,
///   5. both peers reached the target confirmed frame (progress/liveness).
fn check_run(s: &ChaosScenario, run: &ScenarioRun) -> Result<(), String> {
    // Non-vacuous floor FIRST, so the checksum equality below is meaningful.
    if run.confirmed1 < MIN_CONFIRMED_FLOOR || run.confirmed2 < MIN_CONFIRMED_FLOOR {
        return Err(format!(
            "confirmed range below floor {MIN_CONFIRMED_FLOOR} (p1={}, p2={}); \
             determinism checksum would be vacuous\n{}",
            run.confirmed1,
            run.confirmed2,
            diagnostics(s, run)
        ));
    }
    if let Some(frame) = run.first_divergence {
        return Err(format!(
            "confirmed inputs diverged at frame {frame}\n{}",
            diagnostics(s, run)
        ));
    }
    if run.checksum1 != run.checksum2 {
        return Err(format!(
            "confirmed-input checksums differ between peers\n{}",
            diagnostics(s, run)
        ));
    }
    if run.stub_frame1 <= 0 || run.stub_frame2 <= 0 {
        return Err(format!(
            "a peer's GameStub did not advance\n{}",
            diagnostics(s, run)
        ));
    }
    if !run.reached_target {
        return Err(format!(
            "did not reach target confirmed frame\n{}",
            diagnostics(s, run)
        ));
    }
    Ok(())
}

/// Runs a single scenario on its FIXED seed and returns `Ok(())` if all
/// invariants hold, or `Err(diagnostic)`.
///
/// There is no seed search: each row uses exactly one fixed `(seed, seed+1)`
/// pairing, so a passing result reflects a genuinely robust run rather than a
/// lucky draw. After the invariants pass, reproducibility is proven by
/// re-running the identical scenario from scratch and asserting bit-identical
/// confirmed frames AND checksums.
fn run_chaos_scenario(s: &ChaosScenario) -> Result<(), String> {
    let run = execute_scenario_once(s);
    check_run(s, &run)?;

    // Reproducibility: the identical scenario + seed must reproduce the exact
    // confirmed frames and checksums, bit-for-bit (full determinism).
    let rerun = execute_scenario_once(s);
    let repro_ok = rerun.first_divergence.is_none()
        && rerun.confirmed1 == run.confirmed1
        && rerun.confirmed2 == run.confirmed2
        && rerun.checksum1 == run.checksum1
        && rerun.checksum2 == run.checksum2;
    if !repro_ok {
        return Err(format!(
            "reproducibility mismatch: \
             run=(c1={},c2={},{:#018x},{:#018x}) rerun=(c1={},c2={},{:#018x},{:#018x})\n{}",
            run.confirmed1,
            run.confirmed2,
            run.checksum1,
            run.checksum2,
            rerun.confirmed1,
            rerun.confirmed2,
            rerun.checksum1,
            rerun.checksum2,
            diagnostics(s, &rerun)
        ));
    }
    Ok(())
}

/// The primary data-driven chaos test: iterates the entire scenario table,
/// aggregates failures into a single clear report, and asserts at the end.
///
/// This is fully deterministic and runs in virtual time, so it completes in a
/// few seconds regardless of the (extreme) loss rates in the table.
#[test]
fn in_process_chaos_table() {
    let scenarios = scenarios();
    let mut failures: Vec<String> = Vec::new();
    let mut passed = 0usize;

    for scenario in &scenarios {
        match run_chaos_scenario(scenario) {
            Ok(()) => passed += 1,
            Err(report) => {
                eprintln!("\n=== CHAOS SCENARIO FAILURE ===\n{report}\n");
                failures.push(scenario.name.to_string());
            },
        }
    }

    eprintln!(
        "in_process_chaos_table: {passed}/{} scenarios passed",
        scenarios.len()
    );

    assert!(
        failures.is_empty(),
        "{} chaos scenario(s) failed: {:?}",
        failures.len(),
        failures
    );
}

/// A focused sanity check on the determinism harness itself using a perfect
/// link: confirmed inputs must match exactly and reproduce bit-for-bit. If
/// this fails while the table passes, the bug is in the harness, not chaos
/// handling.
#[test]
fn in_process_chaos_perfect_link_determinism() {
    let scenario = ChaosScenario {
        name: "perfect_link",
        peer1: ChaosProfile::local(),
        peer2: ChaosProfile::local(),
        seed: 7,
        target_confirmed: 50,
        input_delay: 2,
        preset: SyncPreset::Lan,
    };

    if let Err(report) = run_chaos_scenario(&scenario) {
        panic!("perfect-link determinism failed:\n{report}");
    }
}

// ===========================================================================
// N≥3 deterministic chaos mesh
// ===========================================================================
//
// The 2-peer table above is the original deterministic chaos coverage. This
// section generalizes the exact same methodology — seeded `ChaosSocket`s over
// `ChannelSocket`s on a shared virtual `TestClock`, a cross-peer confirmed-input
// determinism + checksum assertion, and a bit-for-bit reproducibility re-run —
// to a **fully-meshed N-peer session** (N ∈ {3, 4}). It is the deterministic,
// in-process companion to the nightly real-UDP N≥3 multi-process driver
// (`tests/network/multi_process.rs`): same determinism oracle, but every run is
// reproducible and completes in milliseconds of virtual time, so it can run in
// the default (non-nightly) suite and pin regressions exactly.
//
// # Why N≥3 is a distinct test, not just a bigger number
//
// Confirmed-input agreement across **three or more** peers exercises the
// disconnect-gossip relay, per-(local, remote) checksum keying, and the
// freeze-barrier `confirmed_frame()` bound — machinery that is structurally
// inert at N=2 (where every peer is the other's sole remote). A 2-peer chaos
// run cannot diverge the way the N-player audit's findings (F4/F7/F9/F11/F12/
// F17) require, because those mechanisms only have N-1 ≥ 2 remotes to
// reconcile. This harness puts the steady-state (no-disconnect) N≥3 confirmed
// stream under sustained loss/latency/reorder/duplication and asserts it stays
// byte-identical across the whole mesh.
//
// # Seed correlation (N-peer form)
//
// The 2-peer seed-correlation warning generalizes: every peer in the mesh must
// use a **distinct** chaos seed, or correlated drop sequences systematically
// block synchronization. Peer `i` uses `seed + i`.

/// One row of the N-peer data-driven scenario table.
#[derive(Debug, Clone)]
struct NPeerChaosScenario {
    /// Human-readable scenario name (used in diagnostics).
    name: &'static str,
    /// Per-peer chaos profiles; `profiles.len()` is the mesh size `N`.
    profiles: Vec<ChaosProfile>,
    /// Base seed; peer `i` uses `seed + i` (never identical across peers).
    seed: u64,
    /// Target confirmed frame every peer must reach.
    target_confirmed: i32,
    /// Per-player input delay frames.
    input_delay: usize,
    /// Sync configuration preset to use for all peers.
    preset: SyncPreset,
}

impl NPeerChaosScenario {
    /// The mesh size `N`.
    fn num_players(&self) -> usize {
        self.profiles.len()
    }
}

/// The result of running a single N-peer scenario once.
struct NPeerScenarioRun {
    /// Per-peer confirmed frame reached.
    confirmed: Vec<i32>,
    /// Per-peer current frame.
    current: Vec<i32>,
    /// Per-peer final `GameStub` frame.
    stub_frame: Vec<i32>,
    /// Per-peer checksum over the shared confirmed-input range.
    checksum: Vec<u64>,
    /// The shared confirmed frame (the min confirmed frame across the mesh) over
    /// which the checksum was actually computed.
    shared: i32,
    /// Per-peer length of the flattened confirmed-input stream actually folded
    /// into the checksum. Used to assert the checksum is non-vacuous (it covers
    /// the full `[0, shared]` range and was not truncated by an unavailable
    /// confirmed frame) — see `check_npeer_run`.
    stream_len: Vec<usize>,
    /// First `(peer, flattened-input-index)` at which a peer's confirmed-input
    /// stream diverged from peer 0's, if any. The index is into the
    /// all-players-per-frame flattened `u32` stream (index = frame * N + player
    /// for N players), NOT a frame number.
    first_divergence: Option<(usize, i32)>,
    /// Per-peer final session state for diagnostics.
    state: Vec<SessionState>,
    /// Whether every peer reached the target confirmed frame.
    reached_target: bool,
}

/// Deterministic, frame-and-peer-derived local input for peer `peer` on input
/// tick `tick`. Independent of any wall clock. Distinct per peer at the small
/// `(N, tick)` ranges this harness exercises — the negative-control test
/// (`in_process_chaos_npeer_oracle_detects_divergence`) independently proves a
/// cross-peer mix-up is caught, so exact all-tick injectivity is not relied on.
fn npeer_input(tick: u32, peer: usize) -> u32 {
    let peer = u32::try_from(peer).unwrap_or(0);
    tick.wrapping_mul(7u32.wrapping_add(peer.wrapping_mul(4)))
        .wrapping_add(1u32.wrapping_add(peer.wrapping_mul(2)))
}

/// Builds a fully-meshed N-peer session set for the scenario, runs the
/// simulation to the target confirmed frame, and computes the cross-peer
/// determinism checksum. This is the single source of truth used by both the
/// initial run and the reproducibility re-run.
fn execute_npeer_scenario_once(s: &NPeerChaosScenario) -> NPeerScenarioRun {
    let n = s.num_players();
    let clock = TestClock::new();

    // Distinct seed per peer (see the seed-correlation note above).
    let configs: Vec<_> = s
        .profiles
        .iter()
        .enumerate()
        .map(|(i, profile)| profile.to_chaos_config(s.seed + i as u64))
        .collect();

    let (sockets, addrs) = create_chaos_channel_mesh(configs, &clock);

    // As in the 2-peer harness, set a disconnect timeout provably larger than
    // the whole virtual-time budget: there is no peer that died here, only chaos
    // sockets dropping packets, so any spurious disconnect would be a false
    // positive that breaks the confirmed-input invariant. Virtual time is free.
    let disconnect_timeout = Duration::from_secs(100_000);
    let disconnect_notify = Duration::from_secs(50_000);

    let mut sessions: Vec<P2PSession<StubConfig>> = Vec::with_capacity(n);
    for (i, socket) in sockets.into_iter().enumerate() {
        let mut builder = SessionBuilder::<StubConfig>::new()
            .with_num_players(n)
            .expect("valid player count")
            .with_protocol_config(protocol_config(&clock, s.preset))
            .with_sync_config(s.preset.sync_config())
            .with_time_sync_config(time_sync_config(s.preset))
            .with_disconnect_timeout(disconnect_timeout)
            .with_disconnect_notify_delay(disconnect_notify)
            .with_input_delay(s.input_delay)
            .expect("valid input delay");
        for (h, &addr) in addrs.iter().enumerate() {
            let player = if h == i {
                PlayerType::Local
            } else {
                PlayerType::Remote(addr)
            };
            builder = builder
                .add_player(player, PlayerHandle::new(h))
                .expect("add player");
        }
        sessions.push(
            builder
                .start_p2p_session(socket)
                .expect("start p2p session"),
        );
    }

    // --- Synchronize the whole mesh (bounded, virtual-time iteration cap) ----
    // An N-peer mesh must sync every pair, so the cap is more generous than the
    // 2-peer harness. Poll several times per tick to drain chaos buffers.
    for _ in 0..8000 {
        for _ in 0..4 {
            for sess in &mut sessions {
                sess.poll_remote_clients();
            }
        }
        if sessions
            .iter()
            .all(|sess| sess.current_state() == SessionState::Running)
        {
            break;
        }
        clock.advance(Duration::from_millis(20));
    }

    let mut stubs: Vec<GameStub> = (0..n).map(|_| GameStub::new()).collect();

    // --- Advance until every peer reaches the target confirmed frame --------
    if sessions
        .iter()
        .all(|sess| sess.current_state() == SessionState::Running)
    {
        let mut tick: u32 = 0;
        for _ in 0..8000 {
            if sessions
                .iter()
                .all(|sess| sess.confirmed_frame().as_i32() >= s.target_confirmed)
            {
                break;
            }

            for _ in 0..10 {
                for sess in &mut sessions {
                    sess.poll_remote_clients();
                }
            }
            clock.advance(Duration::from_millis(16));

            // Every peer offers its (deterministic, distinct) local input.
            // add_local_input only fails on a wrong handle, so under correct
            // wiring this always succeeds; the prediction-window throttle is
            // applied by advance_frame (handled below), not here.
            let mut all_added = true;
            for (i, sess) in sessions.iter_mut().enumerate() {
                let input = StubInput {
                    inp: npeer_input(tick, i),
                };
                if sess.add_local_input(PlayerHandle::new(i), input).is_err() {
                    all_added = false;
                }
            }
            if !all_added {
                continue;
            }

            for (i, sess) in sessions.iter_mut().enumerate() {
                fold_advance_frame(sess.advance_frame(), &mut stubs[i]);
            }
            tick = tick.wrapping_add(1);
        }
    }

    // --- Drain any in-flight packets so confirmations settle ----------------
    for _ in 0..400 {
        for sess in &mut sessions {
            sess.poll_remote_clients();
        }
        clock.advance(Duration::from_millis(16));
    }

    let confirmed: Vec<i32> = sessions
        .iter()
        .map(|sess| sess.confirmed_frame().as_i32())
        .collect();
    let current: Vec<i32> = sessions
        .iter()
        .map(|sess| sess.current_frame().as_i32())
        .collect();
    let state: Vec<SessionState> = sessions.iter().map(P2PSession::current_state).collect();
    let stub_frame: Vec<i32> = stubs.iter().map(|stub| stub.gs.frame).collect();

    // --- The core cross-peer determinism + checksum assertion ---------------
    // Compare confirmed inputs frame-by-frame over the shared confirmed range
    // (the min confirmed frame across the whole mesh).
    let shared = confirmed.iter().copied().min().unwrap_or(-1);
    let streams = collect_mesh_streams(&sessions, shared);
    let stream_len: Vec<usize> = streams.iter().map(Vec::len).collect();
    let (checksum, first_divergence) = mesh_input_checksums(&streams);

    let reached_target = confirmed.iter().all(|&c| c >= s.target_confirmed);

    NPeerScenarioRun {
        confirmed,
        current,
        stub_frame,
        checksum,
        shared,
        stream_len,
        first_divergence,
        state,
        reached_target,
    }
}

/// Collects each peer's confirmed-input `u32` stream over `0..=shared` (all
/// players' inputs flattened per frame, in handle order). If a peer cannot
/// produce confirmed inputs for some frame in range, that peer's stream is
/// truncated there — `mesh_input_checksums` then surfaces the truncation as a
/// divergence relative to peer 0.
fn collect_mesh_streams(sessions: &[P2PSession<StubConfig>], shared: i32) -> Vec<Vec<u32>> {
    sessions
        .iter()
        .map(|sess| {
            let mut acc: Vec<u32> = Vec::new();
            let mut f = 0;
            while f <= shared {
                match sess.confirmed_inputs_for_frame(Frame::new(f)) {
                    Ok(inputs) => acc.extend(inputs.iter().map(|input| input.inp)),
                    Err(_) => break,
                }
                f += 1;
            }
            acc
        })
        .collect()
}

/// Folds each peer's flattened confirmed-input stream into an FNV-1a checksum
/// and reports the first `(peer, element_index)` at which any peer's stream
/// differs from peer 0's (a shorter stream counts as a divergence at its end).
///
/// Pure and self-contained so the negative-control test
/// (`npeer_oracle_detects_divergence`) can prove it actually catches a
/// divergence — i.e. that the green assertions below are non-vacuous.
fn mesh_input_checksums(streams: &[Vec<u32>]) -> (Vec<u64>, Option<(usize, i32)>) {
    let checksums: Vec<u64> = streams.iter().map(fnv1a_hash).collect();

    let mut first_divergence: Option<(usize, i32)> = None;
    if let Some(reference) = streams.first() {
        for (peer, stream) in streams.iter().enumerate().skip(1) {
            let common = reference.len().min(stream.len());
            let mut diverged = false;
            for idx in 0..common {
                if reference[idx] != stream[idx] {
                    first_divergence = Some((peer, idx as i32));
                    diverged = true;
                    break;
                }
            }
            if !diverged && reference.len() != stream.len() {
                // Streams agree on the common prefix but differ in length: one
                // peer produced fewer confirmed inputs over the shared range.
                first_divergence = Some((peer, common as i32));
                diverged = true;
            }
            if diverged {
                break;
            }
        }
    }

    (checksums, first_divergence)
}

/// Renders a rich, self-explanatory diagnostic block for a failing N-peer
/// scenario.
fn npeer_diagnostics(s: &NPeerChaosScenario, run: &NPeerScenarioRun) -> String {
    use std::fmt::Write as _;

    let mut out = format!(
        "N-peer scenario '{name}' FAILED\n  \
         N={n}, base_seed={seed} (peer i uses seed+i)\n  \
         preset: {preset:?}, input_delay={delay}, target_confirmed={target}\n  \
         reached_target={reached}, first_confirmed_input_divergence={div:?}\n",
        name = s.name,
        n = s.num_players(),
        seed = s.seed,
        preset = s.preset,
        delay = s.input_delay,
        target = s.target_confirmed,
        reached = run.reached_target,
        div = run.first_divergence,
    );
    for i in 0..s.num_players() {
        let _ = write!(
            out,
            "  peer{i}: profile={profile:?}\n         \
             state={state:?} current={cur} confirmed={conf} stub_frame={stub} checksum={cs:#018x}\n",
            i = i,
            profile = s.profiles[i],
            state = run.state[i],
            cur = run.current[i],
            conf = run.confirmed[i],
            stub = run.stub_frame[i],
            cs = run.checksum[i],
        );
    }
    out
}

/// Checks every per-run invariant for a single completed N-peer run:
///   1. a non-vacuous confirmed range AND a non-vacuous, fully-collected
///      checksummed stream on EVERY peer (so the checksum is meaningful, and
///      `confirmed >= MIN_CONFIRMED_FLOOR > 0` also proves no peer suffered a
///      spurious disconnect downgrade to `Synchronizing`),
///   2. no confirmed-input divergence anywhere in the mesh,
///   3. all peers' determinism checksums identical,
///   4. every `GameStub` advanced,
///   5. every peer reached the target confirmed frame (progress/liveness).
fn check_npeer_run(s: &NPeerChaosScenario, run: &NPeerScenarioRun) -> Result<(), String> {
    // Non-vacuous floor FIRST, on every peer.
    if let Some((peer, conf)) = run
        .confirmed
        .iter()
        .copied()
        .enumerate()
        .find(|&(_, c)| c < MIN_CONFIRMED_FLOOR)
    {
        return Err(format!(
            "peer{peer} confirmed range {conf} below floor {MIN_CONFIRMED_FLOOR}; \
             determinism checksum would be vacuous\n{}",
            npeer_diagnostics(s, run)
        ));
    }
    // The checksum is folded over `collect_mesh_streams(.., shared)`, which
    // truncates a peer's stream if any confirmed frame in `[0, shared]` is
    // unavailable (e.g. evicted from the input ring). `confirmed_frame()` being
    // high does NOT, by itself, prove the stream was retrievable — so assert the
    // checksummed streams are themselves non-vacuous and complete: `shared`
    // clears the floor AND every peer folded the full `(shared + 1) * N`
    // elements. This keeps the determinism assertion honest even if a future
    // target/queue-length change pushes frame 0 out of the ring.
    if run.shared < MIN_CONFIRMED_FLOOR {
        return Err(format!(
            "shared confirmed range {} below floor {MIN_CONFIRMED_FLOOR}; \
             determinism checksum would be vacuous\n{}",
            run.shared,
            npeer_diagnostics(s, run)
        ));
    }
    let expected_len = usize::try_from((run.shared + 1).max(0)).unwrap_or(0) * s.num_players();
    if let Some((peer, len)) = run
        .stream_len
        .iter()
        .copied()
        .enumerate()
        .find(|&(_, len)| len != expected_len)
    {
        return Err(format!(
            "peer{peer} folded {len} confirmed-input elements but the full shared range \
             [0, {}] over {} players needs {expected_len} (stream truncated — an in-range \
             confirmed frame was unavailable); determinism checksum would be vacuous\n{}",
            run.shared,
            s.num_players(),
            npeer_diagnostics(s, run)
        ));
    }
    if let Some((peer, index)) = run.first_divergence {
        return Err(format!(
            "confirmed inputs diverged: peer{peer} differs from peer0 at flattened index {index}\n{}",
            npeer_diagnostics(s, run)
        ));
    }
    // All checksums must equal peer 0's.
    if let Some(reference) = run.checksum.first() {
        if let Some((peer, _)) = run
            .checksum
            .iter()
            .enumerate()
            .find(|&(_, cs)| cs != reference)
        {
            return Err(format!(
                "confirmed-input checksums differ: peer{peer} != peer0\n{}",
                npeer_diagnostics(s, run)
            ));
        }
    }
    if let Some((peer, _)) = run.stub_frame.iter().enumerate().find(|&(_, f)| *f <= 0) {
        return Err(format!(
            "peer{peer}'s GameStub did not advance\n{}",
            npeer_diagnostics(s, run)
        ));
    }
    if !run.reached_target {
        return Err(format!(
            "not every peer reached the target confirmed frame\n{}",
            npeer_diagnostics(s, run)
        ));
    }
    Ok(())
}

/// Runs a single N-peer scenario on its FIXED per-peer seeds and returns
/// `Ok(())` if all invariants hold, or `Err(diagnostic)`. After the invariants
/// pass, reproducibility is proven by re-running the identical scenario from
/// scratch and asserting bit-identical confirmed frames AND checksums.
fn run_npeer_chaos_scenario(s: &NPeerChaosScenario) -> Result<(), String> {
    let run = execute_npeer_scenario_once(s);
    check_npeer_run(s, &run)?;

    let rerun = execute_npeer_scenario_once(s);
    let repro_ok = rerun.first_divergence.is_none()
        && rerun.confirmed == run.confirmed
        && rerun.checksum == run.checksum;
    if !repro_ok {
        return Err(format!(
            "reproducibility mismatch:\n  run    confirmed={:?} checksum={:?}\n  \
             rerun  confirmed={:?} checksum={:?}\n{}",
            run.confirmed,
            run.checksum,
            rerun.confirmed,
            rerun.checksum,
            npeer_diagnostics(s, &rerun)
        ));
    }
    Ok(())
}

/// The N-peer data-driven scenario table (N ∈ {3, 4}).
///
/// Profiles are drawn from the same `ChaosProfile` palette as the 2-peer table.
/// Targets are deliberately conservative for the larger meshes: an N-peer mesh
/// must synchronize every pair and reconcile every remote's confirmed stream,
/// so sustained heavy loss converges more slowly than the 2-peer case. Each row
/// is pinned to a single seed family (no seed search), so a pass reflects a
/// genuinely robust deterministic run, and reproducibility is asserted on top.
fn npeer_scenarios() -> Vec<NPeerChaosScenario> {
    use SyncPreset::{Competitive, Default, Lan, Mobile};
    vec![
        // --- N = 3 (symmetric) ----------------------------------------------
        NPeerChaosScenario {
            name: "n3_local",
            profiles: vec![ChaosProfile::local(); 3],
            seed: 2_100,
            target_confirmed: 40,
            input_delay: 2,
            preset: Lan,
        },
        NPeerChaosScenario {
            name: "n3_lan",
            profiles: vec![ChaosProfile::lan(); 3],
            seed: 2_200,
            target_confirmed: 35,
            input_delay: 2,
            preset: Lan,
        },
        NPeerChaosScenario {
            name: "n3_wifi_good",
            profiles: vec![ChaosProfile::wifi_good(); 3],
            seed: 2_300,
            target_confirmed: 35,
            input_delay: 2,
            preset: Competitive,
        },
        NPeerChaosScenario {
            name: "n3_wifi_average",
            profiles: vec![ChaosProfile::wifi_average(); 3],
            seed: 2_400,
            target_confirmed: 30,
            input_delay: 2,
            preset: Default,
        },
        // mobile_4g (~8% loss) is the heaviest symmetric profile that still
        // confirms a meaningful (non-vacuous) range across a 3-peer mesh in the
        // virtual-time budget. The 15%-loss `wifi_congested` / `mobile_3g`
        // profiles — which the 2-peer table tolerates — near-deadlock at N≥3
        // (the per-frame confirmation needs EVERY peer's input, so the combined
        // effective loss compounds with mesh size), so they are deliberately
        // omitted here for the same reason the 2-peer table omits `terrible`:
        // "does it survive catastrophic loss at all" is the nightly real-UDP
        // suite's job, not this determinism table's.
        NPeerChaosScenario {
            name: "n3_mobile_4g",
            profiles: vec![ChaosProfile::mobile_4g(); 3],
            seed: 3_400,
            target_confirmed: 25,
            input_delay: 2,
            preset: Mobile,
        },
        // --- N = 3 (asymmetric: one lossy peer) -----------------------------
        NPeerChaosScenario {
            name: "n3_asymmetric_one_wifi_average",
            profiles: vec![
                ChaosProfile::lan(),
                ChaosProfile::lan(),
                ChaosProfile::wifi_average(),
            ],
            seed: 2_500,
            target_confirmed: 30,
            input_delay: 2,
            preset: Default,
        },
        // --- N = 4 (symmetric) ----------------------------------------------
        NPeerChaosScenario {
            name: "n4_local",
            profiles: vec![ChaosProfile::local(); 4],
            seed: 2_600,
            target_confirmed: 35,
            input_delay: 2,
            preset: Lan,
        },
        NPeerChaosScenario {
            name: "n4_lan",
            profiles: vec![ChaosProfile::lan(); 4],
            seed: 2_700,
            target_confirmed: 30,
            input_delay: 2,
            preset: Lan,
        },
        NPeerChaosScenario {
            name: "n4_wifi_good",
            profiles: vec![ChaosProfile::wifi_good(); 4],
            seed: 2_800,
            target_confirmed: 30,
            input_delay: 2,
            preset: Competitive,
        },
        // --- N = 4 (asymmetric: mixed quality) ------------------------------
        NPeerChaosScenario {
            name: "n4_asymmetric_mixed",
            profiles: vec![
                ChaosProfile::lan(),
                ChaosProfile::wifi_good(),
                ChaosProfile::lan(),
                ChaosProfile::wifi_average(),
            ],
            seed: 2_900,
            target_confirmed: 25,
            input_delay: 2,
            preset: Mobile,
        },
    ]
}

/// The primary N≥3 data-driven chaos test: iterates the whole N-peer scenario
/// table, aggregates failures into a single clear report, and asserts at the
/// end. Fully deterministic and virtual-time, so it completes in a few seconds.
#[test]
fn in_process_chaos_npeer_table() {
    let scenarios = npeer_scenarios();
    let mut failures: Vec<String> = Vec::new();
    let mut passed = 0usize;

    for scenario in &scenarios {
        match run_npeer_chaos_scenario(scenario) {
            Ok(()) => passed += 1,
            Err(report) => {
                eprintln!("\n=== N-PEER CHAOS SCENARIO FAILURE ===\n{report}\n");
                failures.push(scenario.name.to_string());
            },
        }
    }

    eprintln!(
        "in_process_chaos_npeer_table: {passed}/{} scenarios passed",
        scenarios.len()
    );

    assert!(
        failures.is_empty(),
        "{} N-peer chaos scenario(s) failed: {:?}",
        failures.len(),
        failures
    );
}

/// A focused sanity check on the N-peer determinism harness itself using a
/// perfect 3-peer link: confirmed inputs must match exactly across all peers
/// and reproduce bit-for-bit. If this fails while the table passes, the bug is
/// in the harness, not chaos handling.
#[test]
fn in_process_chaos_npeer_perfect_link_determinism() {
    let scenario = NPeerChaosScenario {
        name: "n3_perfect_link",
        profiles: vec![ChaosProfile::local(); 3],
        seed: 11,
        target_confirmed: 50,
        input_delay: 2,
        preset: SyncPreset::Lan,
    };

    if let Err(report) = run_npeer_chaos_scenario(&scenario) {
        panic!("N-peer perfect-link determinism failed:\n{report}");
    }
}

/// Negative control: proves the cross-peer determinism oracle is **non-vacuous**
/// — i.e. it genuinely detects a divergence rather than passing trivially.
///
/// `mesh_input_checksums` is the exact comparator the table test asserts on, so
/// confirming it (a) reports identical checksums and no divergence for agreeing
/// streams and (b) reports unequal checksums and the precise `(peer, index)`
/// for a single corrupted element guarantees that a real N-peer desync would
/// turn the table test RED.
#[test]
fn in_process_chaos_npeer_oracle_detects_divergence() {
    // Agreeing streams: no divergence, all checksums equal.
    let agree = vec![vec![1u32, 2, 3, 4], vec![1, 2, 3, 4], vec![1, 2, 3, 4]];
    let (checksums, divergence) = mesh_input_checksums(&agree);
    assert_eq!(divergence, None, "agreeing streams must not diverge");
    assert!(
        checksums.iter().all(|&c| c == checksums[0]),
        "agreeing streams must share one checksum"
    );

    // One element corrupted on peer 2: divergence at (2, 2), checksum differs.
    let diverge = vec![vec![1u32, 2, 3, 4], vec![1, 2, 3, 4], vec![1, 2, 99, 4]];
    let (checksums, divergence) = mesh_input_checksums(&diverge);
    assert_eq!(
        divergence,
        Some((2, 2)),
        "a corrupted element must be reported at its exact (peer, index)"
    );
    assert_ne!(
        checksums[2], checksums[0],
        "a divergent peer's checksum must differ from peer 0's"
    );

    // Two peers BOTH differing from peer 0: keying off peer 0 still flags the
    // first such peer (lowest index), proving the transitive-equality approach
    // catches multi-peer divergence.
    let multi = vec![vec![1u32, 2, 3, 4], vec![1, 9, 3, 4], vec![1, 2, 9, 4]];
    let (_checksums, divergence) = mesh_input_checksums(&multi);
    assert_eq!(
        divergence,
        Some((1, 1)),
        "when multiple peers differ from peer 0, the lowest-index one is reported first"
    );

    // A short stream (a peer that produced fewer confirmed inputs) also counts
    // as a divergence at the truncation point.
    let truncated = vec![vec![1u32, 2, 3, 4], vec![1, 2], vec![1, 2, 3, 4]];
    let (_checksums, divergence) = mesh_input_checksums(&truncated);
    assert_eq!(
        divergence,
        Some((1, 2)),
        "a truncated stream must be reported as a divergence at its end"
    );

    // The full run-checker rejects a run whose checksums disagree, so the
    // table-level green assertion is itself non-vacuous. The stream lengths are
    // set to the complete `(shared + 1) * N` so the run clears the vacuity gates
    // and actually reaches the checksum-equality check.
    let s = NPeerChaosScenario {
        name: "synthetic_divergent",
        profiles: vec![ChaosProfile::local(); 3],
        seed: 0,
        target_confirmed: 20,
        input_delay: 2,
        preset: SyncPreset::Lan,
    };
    let full_len = (30 + 1) * 3; // shared = 30, N = 3
    let good_lens = vec![full_len; 3];
    let bad_run = NPeerScenarioRun {
        confirmed: vec![30, 30, 30],
        current: vec![32, 32, 32],
        stub_frame: vec![30, 30, 30],
        checksum: vec![0xAAAA, 0xAAAA, 0xBBBB],
        shared: 30,
        stream_len: good_lens.clone(),
        first_divergence: None,
        state: vec![SessionState::Running; 3],
        reached_target: true,
    };
    assert!(
        check_npeer_run(&s, &bad_run).is_err(),
        "check_npeer_run must reject a run whose checksums disagree"
    );

    // And the run-checker rejects a VACUOUS run: a peer whose checksummed stream
    // was truncated (an in-range confirmed frame was unavailable) must fail even
    // though confirmed_frame() and the checksums look fine — this pins the
    // stream-length non-vacuity guard.
    let vacuous_run = NPeerScenarioRun {
        confirmed: vec![30, 30, 30],
        current: vec![32, 32, 32],
        stub_frame: vec![30, 30, 30],
        checksum: vec![0xAAAA; 3],
        shared: 30,
        // Peer 1 folded too few elements (frame 0 evicted from its ring).
        stream_len: vec![full_len, full_len - 3, full_len],
        first_divergence: None,
        state: vec![SessionState::Running; 3],
        reached_target: true,
    };
    assert!(
        check_npeer_run(&s, &vacuous_run).is_err(),
        "check_npeer_run must reject a run whose checksummed stream was truncated"
    );

    // A fully non-vacuous, agreeing run passes the checker (positive control, so
    // the rejections above aren't trivially always-Err).
    let good_run = NPeerScenarioRun {
        confirmed: vec![30, 30, 30],
        current: vec![32, 32, 32],
        stub_frame: vec![30, 30, 30],
        checksum: vec![0xAAAA; 3],
        shared: 30,
        stream_len: good_lens,
        first_divergence: None,
        state: vec![SessionState::Running; 3],
        reached_target: true,
    };
    assert!(
        check_npeer_run(&s, &good_run).is_ok(),
        "check_npeer_run must accept a complete, agreeing, non-vacuous run"
    );
}
