//! Schedule generation for the deterministic whole-mesh simulation harness.
//!
//! A [`Schedule`] is the **fully materialized** plan for one simulation run:
//! per-link fault policies, timed fault events, and the heal/drain window.
//! [`generate`] is a pure function of `(seed, SimConfig)` — every random draw
//! comes from one seeded RNG consumed at generation time, and the runtime
//! never touches that RNG — so a schedule reproduces exactly from its seed,
//! and a serialized schedule (the corpus format) reproduces even after the
//! generator itself evolves.
//!
//! Link-level *noise* (per-send drop/dup/jitter rolls) is separately seeded
//! via [`Schedule::link_seed`], so editing a schedule's event list during
//! shrinking does not perturb the background noise stream.

// Test infrastructure: not every test binary uses every helper.
#![allow(dead_code)]

use crate::common::sim_net::{GilbertElliottPolicy, LinkPolicy};
use fortress_rollback::rng::{Pcg32, SeedableRng};
use fortress_rollback::{__internal::MAX_FRAME_DELAY, DisconnectBehavior, SaveMode};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::time::Duration;

/// Serializable mirror of [`DisconnectBehavior`] (the production enum does
/// not derive serde; schedules must round-trip as corpus artifacts).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DropPolicy {
    /// Mirror of [`DisconnectBehavior::Halt`] (the library default).
    #[default]
    Halt,
    /// Mirror of [`DisconnectBehavior::ContinueWithout`].
    ContinueWithout,
}

impl From<DropPolicy> for DisconnectBehavior {
    fn from(policy: DropPolicy) -> Self {
        match policy {
            DropPolicy::Halt => Self::Halt,
            DropPolicy::ContinueWithout => Self::ContinueWithout,
        }
    }
}

/// Serializable mirror of [`SaveMode`] (the production enum does not derive
/// serde; schedules must round-trip as corpus artifacts).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SavePolicy {
    /// Mirror of [`SaveMode::EveryFrame`] (the library default).
    #[default]
    EveryFrame,
    /// Mirror of [`SaveMode::Sparse`].
    Sparse,
}

impl From<SavePolicy> for SaveMode {
    fn from(policy: SavePolicy) -> Self {
        match policy {
            SavePolicy::EveryFrame => Self::EveryFrame,
            SavePolicy::Sparse => Self::Sparse,
        }
    }
}

/// How the harness's app model reacts to `FortressEvent::WaitRecommendation`.
///
/// The real time-sync control loop is only closed if the application actually
/// *obeys* the recommendation (skips the recommended frames to let peers catch
/// up). The reference client and every prior fleet run **ignore** it, leaving
/// the loop open — so oscillation (H-OSC) is unobservable. This makes the app
/// behavior an explicit, per-run axis.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppModel {
    /// Ignore `WaitRecommendation` — the open-loop behavior of the reference
    /// client (and of every fleet run before this axis existed). The default,
    /// so pre-existing schedules replay identically.
    #[default]
    Ignore,
    /// Obey `WaitRecommendation`: skip `skip_frames` advances (poll but do not
    /// advance) so the ahead peer lets the others catch up — closing the
    /// time-sync loop the H-OSC hypothesis probes.
    Obey,
}

/// How an obeying application applies `WaitRecommendation` events.
///
/// The defaults preserve the historical harness behavior: accept every event,
/// apply its full skip count immediately, and consume the debt on consecutive
/// frame opportunities. Non-default policies are schema-v17 experiment axes.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaitRecommendationPolicy {
    /// Distance between accepted application frame opportunities: an event
    /// accepted at `t` permits the next acceptance at `t + cooldown_frames`.
    pub cooldown_frames: u32,
    /// Optional cap applied to an accepted event's `skip_frames` payload.
    pub max_skip_frames: Option<u32>,
    /// Opportunities to wait before beginning to consume accepted skip debt.
    pub response_delay_frames: u32,
    /// Consume one skip every N otherwise-runnable frame opportunities.
    pub smear_interval: u32,
}

impl Default for WaitRecommendationPolicy {
    fn default() -> Self {
        Self {
            cooldown_frames: 0,
            max_skip_frames: None,
            response_delay_frames: 0,
            smear_interval: 1,
        }
    }
}

impl WaitRecommendationPolicy {
    pub(super) fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

/// Deterministic application CPU-time feedback applied after simulation work.
///
/// Each successful session advance charges the number of simulation frames it
/// executed (one visual frame plus any rollback re-simulation) at a fixed cost.
/// If that virtual work extends beyond the next outer step, the peer consumes
/// frame opportunities without polling, draining events, or advancing until
/// the work completes. This closes H-META-RB's work → poll-gap feedback edge
/// without consulting host timing.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CpuFeedbackPolicy {
    /// Virtual CPU microseconds charged per simulated frame.
    pub simulated_frame_cost_us: u32,
    /// Hard cap on consecutive future outer steps one advance may occupy.
    pub max_poll_delay_steps: u32,
}

/// How the harness turns virtual wall-clock time into frame opportunities.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameModel {
    /// Preserve the historical harness cadence: every peer accrues exactly one
    /// opportunity per outer step. Dead, stalled, or synchronizing peers
    /// consume that opportunity without advancing, so they never catch up in a
    /// burst after becoming runnable.
    #[default]
    Lockstep,
    /// Gate each peer at 60 Hz using its own rate-skewed clock. This is the
    /// model required to exercise H-SKEW's frame-production drift. Validation
    /// admits only cadences that yield at most one opportunity per outer step.
    SkewGated60Hz,
}

/// Which storyline vocabulary the deterministic generator may use.
///
/// `NetworkOnly` is the default so every pre-existing generated seed and every
/// serialized schedule that predates this axis keeps the exact same network
/// policies and event stream. `Lifecycle` opts into one substantive peer/app
/// lifecycle operation in addition to the existing network storylines.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ScenarioMix {
    /// Generate only link-policy and link-state storylines.
    #[default]
    NetworkOnly,
    /// Add one valid lifecycle operation selected deterministically by seed.
    Lifecycle,
}

/// Schema version for serialized schedules (bump on breaking layout change).
///
/// - `1`: network-fault vocabulary only (`SetLink`/`Block`/`Hold`/`HealAll`).
/// - `2`: adds the first lifecycle-vocabulary fault
///   [`ScheduleEvent::PeerStall`] (a peer that hangs, not just a degraded
///   link). A v1 reader cannot interpret a v2 schedule carrying a `PeerStall`,
///   so the format capability — not just this generator's output — dictates
///   the bump.
/// - `3`: adds [`ScheduleEvent::SetInputDelay`] (a mid-run input-delay change,
///   exercising the session's gap-fill/reconfiguration path).
/// - `4`: adds [`ScheduleEvent::PeerKill`] (a peer crash — the harness stops
///   driving the peer and detaches it from the fabric — modeled distinctly from
///   a network black-hole).
/// - `5`: adds [`ScheduleEvent::GracefulRemove`], the explicit
///   user-driven graceful-drop entry point (`P2PSession::remove_player`).
/// - `6`: adds [`ScheduleEvent::LegacyDisconnect`], the older
///   user-driven disconnect entry point (`P2PSession::disconnect_player`).
/// - `7`: adds [`ScheduleEvent::SpectatorHostKill`], a spectator-focused host
///   crash/failover probe.
/// - `8`: adds [`ScheduleEvent::HotJoin`], a returning-peer reactivation probe
///   driven through the public hot-join API.
/// - `9`: adds [`BackgroundNoise::ReliableFifo`] and
///   [`LinkPolicy::retransmit_delay`](crate::common::sim_net::LinkPolicy::retransmit_delay)
///   for reliable-ordered transport probes.
/// - `10`: adds [`ScheduleEvent::Rebind`], a peer source-address/NAT mapping
///   change that leaves every other peer's canonical destination unchanged.
/// - `11`: adds [`GilbertElliottPolicy`], a deterministic two-state
///   correlated-loss model on materialized directed links.
/// - `12`: makes protocol RNG seeds target-width-stable and gives each
///   hot-join replacement generation an initial protocol connection ID distinct from
///   the generation it replaces. It bounds new schedules to one replacement
///   generation because fencing older/per-link protocol eras needs a wider
///   model. Schemas <=11 retain their historical seed and validation semantics
///   for corpus replay.
/// - `13`: adds optional fixed-threshold IPv4-style fragmentation loss on
///   materialized directed links.
/// - `14`: adds deterministic token-bucket bandwidth and bounded queueing on
///   materialized directed links.
/// - `15`: adds [`FrameModel::SkewGated60Hz`], which makes per-peer clock skew
///   control frame-production cadence instead of timestamps alone.
/// - `16`: adds bounded directed-endpoint and live bandwidth-queue samples to
///   control-loop experiments. Schema 15 retains its original trace identity.
/// - `17`: adds application-side wait-recommendation cooldown, payload-cap,
///   response-delay, and smearing policies plus exact rolling-average endpoint
///   evidence for fixed-cadence H-OSC experiments.
/// - `18`: adds deterministic bounded CPU-work feedback from simulation frames
///   into future poll cadence for H-META-RB experiments.
pub const SCHEDULE_SCHEMA_VERSION: u32 = 18;
/// Hard execution bound for one materialized harness schedule.
///
/// This admits the H-SKEW experiment's 240,001 sampled steps at 15 ms cadence:
/// step 0 starts at the epoch and the last driven step starts exactly one hour
/// later at 3,600,000 ms. The bound also prevents an untrusted corpus entry
/// from turning the runner's `0..steps` loop into an effectively unbounded job.
pub const MAX_SIMULATION_STEPS: u32 = 250_000;
/// A 60 Hz target sampled at integer milliseconds can advance twice only when
/// a peer's local clock jumps by at least 17 ms between outer steps.
const MAX_SKEW_GATED_LOCAL_MS_PER_STEP: u128 = 16;
/// Maximum virtual time advanced by one harness step.
pub const MAX_SIMULATION_STEP_DT_MS: u64 = 1_000;
/// Maximum virtual CPU cost assigned to one simulated frame.
pub const MAX_CPU_FRAME_COST_US: u32 = 1_000_000;
/// Maximum delay accepted on any materialized simulated link.
pub const MAX_SIMULATION_LINK_DELAY: Duration = Duration::from_secs(60);
/// Maximum number of timed operations in one materialized schedule.
pub const MAX_SIMULATION_EVENTS: usize = 100_000;
/// Maximum burst or queued-byte bound accepted from a corpus schedule.
pub const MAX_SIMULATION_BANDWIDTH_BYTES: u64 = crate::common::sim_net::MAX_BANDWIDTH_BYTES;

/// Background link-noise level applied to every directed link at start.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackgroundNoise {
    /// Perfect links.
    Clean,
    /// Reliable, ordered, loss-free links (TCP/WebRTC-reliable baseline).
    ReliableFifo,
    /// LAN-to-good-WAN: ≤2% loss, 5–30ms delay, ≤10ms jitter, ≤1% dup.
    Mild,
    /// Bad WAN / mobile: 2–10% loss, 20–80ms delay, ≤30ms jitter, ≤3% dup,
    /// occasional short loss bursts.
    Rough,
}

/// Static configuration of one simulation run — the fleet's axes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimConfig {
    /// Number of P2P peers in the mesh (2..=16).
    pub n_players: usize,
    /// Total schedule length in steps (including the drain window).
    pub steps: u32,
    /// Virtual milliseconds advanced per step.
    pub step_dt_ms: u64,
    /// Session input delay (frames).
    pub input_delay: usize,
    /// Session prediction window (frames).
    pub max_prediction: usize,
    /// Desync-detection checksum interval (frames); always on in simulation.
    pub desync_interval: u32,
    /// Background link noise.
    pub noise: BackgroundNoise,
    /// Storyline vocabulary available to the generator. `#[serde(default)]`
    /// preserves the exact pre-lifecycle generator behavior for old corpus
    /// schedules and existing callers.
    #[serde(default)]
    pub scenario_mix: ScenarioMix,
    /// Peer-drop policy for every session in the mesh. `#[serde(default)]`
    /// (= `Halt`, the library default) keeps pre-existing corpus artifacts
    /// replayable without a schema bump.
    #[serde(default)]
    pub disconnect_behavior: DropPolicy,
    /// Game-state save strategy for every P2P session in the mesh.
    /// `#[serde(default)]` (= `EveryFrame`, the library default) keeps
    /// pre-existing corpus artifacts replayable without a schema bump.
    #[serde(default)]
    pub save_mode: SavePolicy,
    /// How every peer's app model reacts to `WaitRecommendation`.
    /// `#[serde(default)]` (= `Ignore`, the open-loop behavior every prior run
    /// used) keeps pre-existing corpus artifacts replayable without a bump.
    #[serde(default)]
    pub app_model: AppModel,
    /// Application-side wait actuation policy. Omitted for the historical
    /// immediate/full-obedience behavior so schema-v16 artifacts remain byte
    /// stable when decoded and re-encoded.
    #[serde(default, skip_serializing_if = "WaitRecommendationPolicy::is_default")]
    pub wait_recommendation_policy: WaitRecommendationPolicy,
    /// Optional deterministic CPU-work feedback. Omitted to preserve the
    /// historical fixed-cost/fixed-poll-cadence runner.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_feedback_policy: Option<CpuFeedbackPolicy>,
    /// Frame-production model. `Lockstep` preserves every schema <=14 run.
    #[serde(default)]
    pub frame_model: FrameModel,
    /// Per-peer clock-rate skew in parts-per-million (peer `i` reads
    /// `clock_skew_ppm[i]`; a missing/short entry means 0 = no skew). The
    /// peer's local clock runs `ppm` faster (positive) or slower (negative)
    /// than the network's real-time base, modeling unsynchronized clocks
    /// (H-SKEW). `#[serde(default)]` (empty = every clock exact) keeps existing
    /// corpus artifacts replayable without a bump.
    #[serde(default)]
    pub clock_skew_ppm: Vec<i32>,
    /// Peers (by index) whose app model **never drains the session event
    /// queue** — modeling an application that stops servicing
    /// `P2PSession::events` (a wedged UI thread, a dropped event consumer). The
    /// session keeps polling and advancing, so its bounded `event_queue` fills;
    /// the session then trims the oldest events, incrementing the D9
    /// `events_discarded_*` telemetry (`§6.6-pre.6h` event-loss oracle). A
    /// starved peer is excluded only from *event-derived* oracle signals (the
    /// secondary `DesyncDetected` observation); the primary state-agreement,
    /// confirmed-prefix, and liveness checks read `confirmed_frame`/recorded
    /// state directly and keep full teeth. `#[serde(default)]` (empty = every
    /// peer drains every step, the behavior every prior run used) keeps existing
    /// corpus artifacts replayable without a schema bump.
    #[serde(default)]
    pub starve_events: Vec<usize>,
    /// Optional override for every session's event-queue capacity
    /// (`SessionBuilder::with_event_queue_size`, min 10). `#[serde(default)]`
    /// (`None` = the library default of 100) keeps existing corpus artifacts
    /// replayable without a bump. A small cap makes event-starvation overflow
    /// (D9) reachable within a short schedule.
    #[serde(default)]
    pub event_queue_size: Option<usize>,
    /// P2P peer indices that should serve one pre-planned redundant spectator.
    /// Empty disables spectator driving. The generator leaves this empty so
    /// existing random schedules stay byte-identical; hand-authored schedules
    /// opt in when they need the §6.2(d) spectator-convergence oracle.
    #[serde(default)]
    pub spectator_hosts: Vec<usize>,
}

impl SimConfig {
    /// The PR-smoke configuration: short schedule, mild noise.
    ///
    /// 600 steps × 16ms ≈ 10 virtual seconds, of which the last 250 steps
    /// (≈4s) are the post-heal drain window — enough to cover the default 2s
    /// disconnect timeout plus sync retries after the harshest storyline.
    #[must_use]
    pub fn smoke(n_players: usize) -> Self {
        Self {
            n_players,
            steps: 600,
            step_dt_ms: 16,
            input_delay: 1,
            max_prediction: 8,
            desync_interval: 30,
            noise: BackgroundNoise::Mild,
            scenario_mix: ScenarioMix::default(),
            disconnect_behavior: DropPolicy::default(),
            save_mode: SavePolicy::default(),
            app_model: AppModel::default(),
            wait_recommendation_policy: WaitRecommendationPolicy::default(),
            cpu_feedback_policy: None,
            frame_model: FrameModel::default(),
            clock_skew_ppm: Vec::new(),
            starve_events: Vec::new(),
            event_queue_size: None,
            spectator_hosts: Vec::new(),
        }
    }

    /// Virtual duration of one step.
    #[must_use]
    pub fn step_dt(&self) -> Duration {
        Duration::from_millis(self.step_dt_ms)
    }

    /// Virtual wall-clock budget for post-heal recovery, in milliseconds — the
    /// bound `B` for the oracle's (c) bounded-liveness check. Folds the maximum
    /// documented compound recovery path (a peer that dropped during a partition
    /// must, after the network heals, still time its endpoints out, re-run the
    /// sync handshake, then re-fold connect-status gossip before it resumes
    /// confirming): `disconnect_timeout` (2000ms), plus a sync-retry burst
    /// (`num_sync_packets` times `sync_retry_interval` = 5 times 200ms = 1000ms),
    /// plus gossip settle (3 times `keepalive_interval` = 600ms), rounded up to
    /// one further keepalive round of slack. Source of the constants:
    /// `src/network/protocol/mod.rs` `poll()` Running arm (:1237-1258, the
    /// disconnect and keepalive cadence) and `SyncConfig::default` (:6431-6435);
    /// `DEFAULT_DISCONNECT_TIMEOUT` and `DEFAULT_DISCONNECT_NOTIFY_START`
    /// (`src/sessions/builder.rs:43-44`). This is the same ~250-step budget the
    /// schedule generator already reserves as its post-heal drain window.
    pub const RECOVERY_WINDOW_MS: u64 = 4000;

    /// The (c) bounded-recovery window `B` in **steps** at this schedule's
    /// `step_dt` — a fixed [`RECOVERY_WINDOW_MS`](Self::RECOVERY_WINDOW_MS)
    /// wall-clock budget converted to the step granularity, so the bound is the
    /// same real duration regardless of `step_dt_ms` (250 steps at 16ms, 125 at
    /// 32ms, 500 at 8ms).
    #[must_use]
    pub fn recovery_window_steps(&self) -> u32 {
        u32::try_from(Self::RECOVERY_WINDOW_MS.div_ceil(self.step_dt_ms.max(1))).unwrap_or(u32::MAX)
    }
}

/// A timed control-plane event. Link endpoints are peer *indices* (resolved
/// to addresses by the harness) so schedules are address-independent.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ScheduleEvent {
    /// Replace the fault policy of the directed link `from → to`.
    SetLink {
        from: usize,
        to: usize,
        policy: LinkPolicy,
    },
    /// Black-hole (or restore) the directed link `from → to`.
    Block {
        from: usize,
        to: usize,
        blocked: bool,
    },
    /// Start (or stop, flushing FIFO) capture-and-hold on `from → to`.
    Hold {
        from: usize,
        to: usize,
        holding: bool,
    },
    /// Freeze peer `peer` for `steps` steps: it stops polling, draining
    /// events, adding input, and advancing — modeling a local hang (GC pause,
    /// frame-time spike, blocked save) rather than a network fault. Unlike a
    /// link fault, the peer emits *nothing* (no inputs, keepalives, or quality
    /// reports) while frozen and resumes exactly where it left off.
    ///
    /// The stall must stay shorter than the mesh's disconnect timeout, or the
    /// silence tips the peer into a genuine disconnect. Planted and generated
    /// lifecycle schedules keep it well under that bound so the hitch is a
    /// recoverable pause, not a drop.
    PeerStall { peer: usize, steps: u32 },
    /// Set peer `peer`'s local input delay to `delay` frames mid-run
    /// (`P2PSession::set_input_delay`). A mid-session *increase* is the
    /// interesting case: it gap-fills the newly delayed frames with replicated
    /// confirmed inputs and flushes them to every remote — a reconfiguration
    /// path a fixed-delay fleet never exercises. Values are per-peer local, so
    /// the mesh must still agree on every confirmed frame across the change.
    ///
    /// Covered by both planted and generated lifecycle schedules.
    SetInputDelay { peer: usize, delay: usize },
    /// A live peer `by` explicitly removes remote player `target` via
    /// `P2PSession::remove_player`, then the target leaves the harness. This is
    /// the clean user-driven graceful-drop path: one survivor applies the API
    /// call and the remaining live mesh must learn the drop through protocol
    /// gossip, freeze the target slot to an agreed value, and keep confirming.
    ///
    /// Covered by both planted and generated lifecycle schedules.
    GracefulRemove { by: usize, target: usize },
    /// A live peer `by` explicitly disconnects remote player `target` via
    /// `P2PSession::disconnect_player`. If the API call succeeds, the target
    /// then leaves the harness; if it errors, the runner records a
    /// `SessionError` and keeps the target live. This is the legacy GGRS-style
    /// user path: unlike
    /// [`ScheduleEvent::GracefulRemove`], it does not freeze the dropped slot or
    /// emit `PeerDropped`; the fail-closed `Halt` path reports non-recovery.
    /// This makes it a Halt terminal-semantics probe rather than a
    /// graceful-convergence contract.
    ///
    /// Planted only; the random generator deliberately excludes this terminal
    /// operation from its green lifecycle mix.
    LegacyDisconnect { by: usize, target: usize },
    /// Permanently kill peer `peer`: the harness stops driving its session and
    /// detaches it from the fabric (its inbox is discarded; further traffic to
    /// it is dropped). Models a **crash** — the peer is gone for good and, being
    /// no longer observable, is excluded from the oracle's *liveness* checks
    /// (its pre-death observations still count for agreement; its remaining mesh
    /// survives per the configured `DisconnectBehavior`).
    /// Distinct from a network black-hole (`Block`), where the peer keeps
    /// running and observing. `HealAll` does not revive it.
    ///
    /// Survivor byte-consistency after a kill is a property of
    /// `DisconnectBehavior::ContinueWithout` over a clean-enough link (the
    /// freeze-convergence path). Under `Halt` the mesh instead freezes each
    /// session at its pre-disconnect confirmation ceiling. The planted crash
    /// tests use `ContinueWithout` because they assert survivor availability.
    ///
    /// Covered by both planted and generated lifecycle schedules.
    PeerKill { peer: usize },
    /// Move peer `peer`'s live simulated socket to a fresh source address
    /// without changing the canonical address stored by any session. This
    /// models a NAT rebinding or mobile-network path change: the rebound peer's
    /// messages arrive from an unknown source, while other peers keep sending
    /// to the abandoned mapping until their disconnect behavior fires.
    ///
    /// Planted only; current protocol behavior is intentionally fail-closed and
    /// therefore is not part of the generated green lifecycle fleet.
    Rebind { peer: usize },
    /// Permanently kill a peer that is configured as one of the redundant
    /// spectator hosts. The runtime effect is the same crash model as
    /// [`ScheduleEvent::PeerKill`] — the host is no longer driven and is detached
    /// from the fabric — but the event has a stricter schedule precondition:
    /// `host` must appear in [`SimConfig::spectator_hosts`]. This pins spectator
    /// failover coverage separately from ordinary mesh crash coverage.
    ///
    /// Covered by both planted and generated lifecycle schedules.
    SpectatorHostKill { host: usize },
    /// Reactivate player slot `slot` through the public hot-join path. The
    /// runner first constructs a fresh hot-joiner for that slot; once that
    /// succeeds, a live survivor gracefully removes the old slot, the fabric
    /// inbox at the old address is reset, and the fresh session takes over that
    /// address. That exercises the returning clean-drop path without permanently
    /// retiring the slot from the oracle.
    ///
    /// Requires the crate's `hot-join` feature at runtime. Hot-join schedules
    /// must use input delay 0 and `max_prediction >= 1`, matching
    /// `SessionBuilder::start_hot_join_session`'s public contract. Covered by
    /// planted schedules and generated two-player lifecycle cells.
    HotJoin { slot: usize },
    /// Reset every link to clean and release all held traffic.
    HealAll,
}

/// Stable classifier for schedule operations that change peer/application
/// lifecycle rather than only link behavior.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LifecycleEventKind {
    PeerStall,
    SetInputDelay,
    GracefulRemove,
    LegacyDisconnect,
    PeerKill,
    Rebind,
    SpectatorHostKill,
    HotJoin,
}

impl ScheduleEvent {
    /// Returns the stable lifecycle class for this event, if any.
    #[must_use]
    pub const fn lifecycle_kind(&self) -> Option<LifecycleEventKind> {
        match self {
            Self::PeerStall { .. } => Some(LifecycleEventKind::PeerStall),
            Self::SetInputDelay { .. } => Some(LifecycleEventKind::SetInputDelay),
            Self::GracefulRemove { .. } => Some(LifecycleEventKind::GracefulRemove),
            Self::LegacyDisconnect { .. } => Some(LifecycleEventKind::LegacyDisconnect),
            Self::PeerKill { .. } => Some(LifecycleEventKind::PeerKill),
            Self::Rebind { .. } => Some(LifecycleEventKind::Rebind),
            Self::SpectatorHostKill { .. } => Some(LifecycleEventKind::SpectatorHostKill),
            Self::HotJoin { .. } => Some(LifecycleEventKind::HotJoin),
            Self::SetLink { .. } | Self::Block { .. } | Self::Hold { .. } | Self::HealAll => None,
        }
    }
}

/// A fully materialized simulation plan. Serializable (corpus format).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Schedule {
    /// Serialized-layout version.
    pub schema_version: u32,
    /// The generator seed this schedule was derived from (provenance).
    pub seed: u64,
    /// Seed for the `SimNet` per-send fault rolls (independent stream).
    pub link_seed: u64,
    /// Run configuration.
    pub config: SimConfig,
    /// Initial policy for **every** directed link `(from, to)`, explicit.
    pub initial_links: Vec<(usize, usize, LinkPolicy)>,
    /// Timed events, sorted ascending by step.
    pub events: Vec<(u32, ScheduleEvent)>,
    /// Step at which [`ScheduleEvent::HealAll`] fires (always present in
    /// `events`); the remaining steps are the clean drain window.
    pub heal_at: u32,
}

impl Schedule {
    /// Counts lifecycle operations after validating that the materialized
    /// schedule is structurally executable.
    ///
    /// This is the fleet's non-vacuity premise: a `Lifecycle` generator cell
    /// must report at least one effective operation rather than silently
    /// falling back to link-only noise.
    pub fn effective_lifecycle_event_count(&self) -> Result<usize, String> {
        validate_schedule(self)?;
        Ok(self
            .events
            .iter()
            .filter(|(_, event)| event.lifecycle_kind().is_some())
            .count())
    }
}

/// Returns the deterministic hot-join coordinator for `slot`.
///
/// Materialized schedules are validated before this is used, so a valid
/// hot-join event always has at least one peer other than `slot` available as
/// an initial coordinator. Runtime lifecycle events may subsequently retire
/// that coordinator; those sequences are validated separately below when the
/// retirement is guaranteed by the harness.
pub(super) fn hot_join_host_for_slot(n_players: usize, slot: usize) -> Option<usize> {
    (0..n_players).find(|&peer| peer != slot)
}

fn link_policy_required_schema(policy: &LinkPolicy) -> u32 {
    if policy.bandwidth.is_some() {
        14
    } else if policy.fragmentation.is_some() {
        13
    } else if policy.gilbert_elliott.is_some() {
        11
    } else if !policy.retransmit_delay.is_zero() {
        9
    } else {
        1
    }
}

fn event_required_schema(event: &ScheduleEvent) -> u32 {
    match event {
        ScheduleEvent::SetLink { policy, .. } => link_policy_required_schema(policy),
        ScheduleEvent::Block { .. } | ScheduleEvent::Hold { .. } | ScheduleEvent::HealAll => 1,
        ScheduleEvent::PeerStall { .. } => 2,
        ScheduleEvent::SetInputDelay { .. } => 3,
        ScheduleEvent::PeerKill { .. } => 4,
        ScheduleEvent::GracefulRemove { .. } => 5,
        ScheduleEvent::LegacyDisconnect { .. } => 6,
        ScheduleEvent::SpectatorHostKill { .. } => 7,
        ScheduleEvent::HotJoin { .. } => 8,
        ScheduleEvent::Rebind { .. } => 10,
    }
}

fn required_schema_version(schedule: &Schedule) -> u32 {
    let mut required = if schedule.config.cpu_feedback_policy.is_some() {
        18
    } else if !schedule.config.wait_recommendation_policy.is_default() {
        17
    } else if schedule.config.frame_model == FrameModel::SkewGated60Hz {
        15
    } else if schedule.config.noise == BackgroundNoise::ReliableFifo {
        9
    } else {
        1
    };
    for (_, _, policy) in &schedule.initial_links {
        required = required.max(link_policy_required_schema(policy));
    }
    for (_, event) in &schedule.events {
        required = required.max(event_required_schema(event));
    }
    required
}

fn validate_probability(value: f64, field: &str, context: &str) -> Result<(), String> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(format!(
            "{context} {field} must be finite and within 0.0..=1.0 (got {value:?})"
        ));
    }
    Ok(())
}

fn validate_link_policy(policy: &LinkPolicy, context: &str) -> Result<(), String> {
    validate_probability(policy.drop_rate, "drop_rate", context)?;
    validate_probability(policy.dup_rate, "dup_rate", context)?;
    validate_probability(policy.burst_rate, "burst_rate", context)?;
    let burst_rate_enabled = policy.burst_rate > 0.0;
    let burst_len_enabled = policy.burst_len > 0;
    if burst_rate_enabled != burst_len_enabled {
        return Err(format!(
            "{context} burst_rate/burst_len must both be zero or both be enabled \
             (got burst_rate={:?}, burst_len={})",
            policy.burst_rate, policy.burst_len
        ));
    }
    if let Some(ge) = &policy.gilbert_elliott {
        for (field, value) in [
            ("good_to_bad", ge.good_to_bad),
            ("bad_to_good", ge.bad_to_good),
            ("good_drop_rate", ge.good_drop_rate),
            ("bad_drop_rate", ge.bad_drop_rate),
        ] {
            validate_probability(value, field, context)?;
        }
        if ge.good_to_bad == 0.0 {
            return Err(format!(
                "{context} Gilbert-Elliott good_to_bad must be > 0 so the bad state is reachable"
            ));
        }
        if ge.bad_drop_rate <= ge.good_drop_rate {
            return Err(format!(
                "{context} Gilbert-Elliott bad_drop_rate must be greater than good_drop_rate \
                 (got good={:?}, bad={:?})",
                ge.good_drop_rate, ge.bad_drop_rate
            ));
        }
        if policy.drop_rate > 0.0 || burst_rate_enabled {
            return Err(format!(
                "{context} Gilbert-Elliott loss cannot be layered with drop_rate or \
                 burst_rate/burst_len"
            ));
        }
    }
    if let Some(fragmentation) = policy.fragmentation {
        validate_probability(
            fragmentation.fragment_drop_rate,
            "fragment_drop_rate",
            context,
        )?;
        if !policy.retransmit_delay.is_zero() {
            return Err(format!(
                "{context} fragmentation cannot be combined with retransmit_delay"
            ));
        }
    }
    if let Some(bandwidth) = policy.bandwidth {
        if bandwidth.rate_bytes_per_second == 0 {
            return Err(format!(
                "{context} bandwidth rate_bytes_per_second must be > 0"
            ));
        }
        if bandwidth.burst_bytes == 0 {
            return Err(format!("{context} bandwidth burst_bytes must be > 0"));
        }
        for (field, value) in [
            ("burst_bytes", bandwidth.burst_bytes),
            ("queue_capacity_bytes", bandwidth.queue_capacity_bytes),
        ] {
            if value > MAX_SIMULATION_BANDWIDTH_BYTES {
                return Err(format!(
                    "{context} bandwidth {field} must be <= \
                     {MAX_SIMULATION_BANDWIDTH_BYTES} (got {value})"
                ));
            }
        }
    }
    for (field, value) in [
        ("base_delay", policy.base_delay),
        ("jitter", policy.jitter),
        ("retransmit_delay", policy.retransmit_delay),
    ] {
        if value > MAX_SIMULATION_LINK_DELAY {
            return Err(format!(
                "{context} {field} must be <= {MAX_SIMULATION_LINK_DELAY:?} (got {value:?})"
            ));
        }
    }
    Ok(())
}

/// Validates a materialized schedule before replay, shrinking, or execution.
///
/// Generated schedules satisfy these invariants by construction. Serialized
/// corpus entries and shrink candidates are untrusted inputs, however, so this
/// function returns a diagnostic instead of relying on indexing panics deep in
/// the runner. [`super::run`] preserves the existing fail-loud test behavior by
/// turning the returned diagnostic into one boundary panic.
pub fn validate_schedule(schedule: &Schedule) -> Result<(), String> {
    if schedule.schema_version == 0 || schedule.schema_version > SCHEDULE_SCHEMA_VERSION {
        return Err(format!(
            "schedule schema_version must be within 1..={SCHEDULE_SCHEMA_VERSION} (got {})",
            schedule.schema_version
        ));
    }
    let required_schema = required_schema_version(schedule);
    if schedule.schema_version < required_schema {
        return Err(format!(
            "schedule schema_version {} under-declares capability requiring schema_version \
             {required_schema}",
            schedule.schema_version
        ));
    }

    let n = schedule.config.n_players;
    if !(2..=16).contains(&n) {
        return Err(format!(
            "materialized simulation schedules must use 2..=16 players (got {n})"
        ));
    }
    if !(2..=MAX_SIMULATION_STEPS).contains(&schedule.config.steps) {
        return Err(format!(
            "materialized simulation schedules need 2..={MAX_SIMULATION_STEPS} steps (got {})",
            schedule.config.steps
        ));
    }
    if schedule.config.input_delay > MAX_FRAME_DELAY {
        return Err(format!(
            "input_delay must be <= {MAX_FRAME_DELAY} (got {})",
            schedule.config.input_delay
        ));
    }
    if schedule.config.max_prediction > MAX_FRAME_DELAY {
        return Err(format!(
            "max_prediction must be <= {MAX_FRAME_DELAY} (got {})",
            schedule.config.max_prediction
        ));
    }
    if schedule.config.desync_interval == 0 {
        return Err("desync_interval must be >= 1 when desync detection is enabled".to_owned());
    }
    let wait_policy = schedule.config.wait_recommendation_policy;
    if wait_policy.smear_interval == 0 {
        return Err("wait_recommendation_policy.smear_interval must be >= 1".to_owned());
    }
    if wait_policy.max_skip_frames == Some(0) {
        return Err("wait_recommendation_policy.max_skip_frames must be >= 1 when set".to_owned());
    }
    if schedule.config.app_model == AppModel::Ignore && !wait_policy.is_default() {
        return Err("non-default wait_recommendation_policy requires app_model Obey".to_owned());
    }
    if let Some(policy) = schedule.config.cpu_feedback_policy {
        if policy.simulated_frame_cost_us == 0 {
            return Err(
                "cpu_feedback_policy.simulated_frame_cost_us must be >= 1 when set".to_owned(),
            );
        }
        if policy.simulated_frame_cost_us > MAX_CPU_FRAME_COST_US {
            return Err(format!(
                "cpu_feedback_policy.simulated_frame_cost_us must be <= \
                 {MAX_CPU_FRAME_COST_US} (got {})",
                policy.simulated_frame_cost_us
            ));
        }
        if policy.max_poll_delay_steps == 0 || policy.max_poll_delay_steps > schedule.config.steps {
            return Err(format!(
                "cpu_feedback_policy.max_poll_delay_steps must be within 1..={} (got {})",
                schedule.config.steps, policy.max_poll_delay_steps
            ));
        }
        if schedule
            .events
            .iter()
            .any(|(_, event)| matches!(event, ScheduleEvent::HotJoin { .. }))
        {
            return Err(
                "cpu_feedback_policy cannot be combined with HotJoin because bridge-frame work \
                 is not represented by SessionMetrics::frames_advanced"
                    .to_owned(),
            );
        }
    }

    if schedule.events.len() > MAX_SIMULATION_EVENTS {
        return Err(format!(
            "schedule has {} events (maximum {MAX_SIMULATION_EVENTS})",
            schedule.events.len()
        ));
    }

    if !schedule
        .events
        .windows(2)
        .all(|pair| pair[0].0 <= pair[1].0)
    {
        return Err("schedule events must be sorted by nondecreasing step".to_owned());
    }
    for (step, _) in &schedule.events {
        if *step >= schedule.config.steps {
            return Err(format!(
                "schedule event at step {step} is outside the run (0..{})",
                schedule.config.steps
            ));
        }
    }
    if let Some(last_heal) = schedule
        .events
        .iter()
        .filter(|(_, event)| matches!(event, ScheduleEvent::HealAll))
        .map(|(step, _)| *step)
        .max()
    {
        for (step, event) in &schedule.events {
            if *step > last_heal && !matches!(event, ScheduleEvent::HealAll) {
                return Err(format!(
                    "non-HealAll event at step {step} occurs after the last HealAll at {last_heal}"
                ));
            }
        }
    }

    let has_hot_join = schedule
        .events
        .iter()
        .any(|(_, event)| matches!(event, ScheduleEvent::HotJoin { .. }));
    let has_rebind = schedule
        .events
        .iter()
        .any(|(_, event)| matches!(event, ScheduleEvent::Rebind { .. }));
    #[cfg(not(feature = "hot-join"))]
    if has_hot_join {
        return Err("ScheduleEvent::HotJoin requires the crate's `hot-join` feature".to_owned());
    }
    if has_rebind && has_hot_join {
        return Err(
            "Rebind and HotJoin cannot share a schedule; replacement binding semantics \
             require a dedicated future capability"
                .to_owned(),
        );
    }
    if has_rebind
        && schedule
            .events
            .iter()
            .any(|(_, event)| matches!(event, ScheduleEvent::Hold { .. }))
    {
        return Err(
            "Rebind cannot share a schedule with Hold; releasing held traffic across \
             source-address generations requires a dedicated future capability"
                .to_owned(),
        );
    }
    if has_hot_join {
        if schedule.config.input_delay != 0 {
            return Err(format!(
                "HotJoin schedules must use input_delay 0 (got {})",
                schedule.config.input_delay
            ));
        }
        if schedule.config.max_prediction < 1 {
            return Err(format!(
                "HotJoin schedules must use max_prediction >= 1 (got {})",
                schedule.config.max_prediction
            ));
        }
        if n >= 3 && schedule.config.save_mode != SavePolicy::EveryFrame {
            return Err(format!(
                "N-peer HotJoin schedules must use save_mode EveryFrame (got {:?})",
                schedule.config.save_mode
            ));
        }
    }

    let expected_links = n * (n - 1);
    if schedule.initial_links.len() != expected_links {
        return Err(
            "initial_links must contain exactly one directed policy for every non-self pair"
                .to_owned(),
        );
    }
    let mut seen_links = BTreeSet::new();
    for (from, to, _) in &schedule.initial_links {
        if *from >= n || *to >= n {
            return Err(format!(
                "initial link ({from} -> {to}) out of range for a {n}-peer mesh"
            ));
        }
        if from == to {
            return Err(format!(
                "initial link ({from} -> {to}) must not target itself"
            ));
        }
        if !seen_links.insert((*from, *to)) {
            return Err(format!("duplicate initial link ({from} -> {to})"));
        }
    }
    for (from, to, policy) in &schedule.initial_links {
        validate_link_policy(policy, &format!("initial link ({from} -> {to})"))?;
    }

    for (_, event) in &schedule.events {
        match event {
            ScheduleEvent::SetLink { from, to, .. }
            | ScheduleEvent::Block { from, to, .. }
            | ScheduleEvent::Hold { from, to, .. } => {
                if *from >= n || *to >= n {
                    return Err(format!(
                        "schedule event link ({from} -> {to}) out of range for a {n}-peer mesh"
                    ));
                }
                if from == to {
                    return Err(format!(
                        "schedule event link ({from} -> {to}) must not target itself"
                    ));
                }
            },
            ScheduleEvent::PeerStall { peer, steps } => {
                if *peer >= n {
                    return Err(format!(
                        "PeerStall peer {peer} out of range for a {n}-peer mesh"
                    ));
                }
                if *steps == 0 {
                    return Err(
                        "PeerStall steps must be > 0 (a 0-step stall freezes nothing)".to_owned(),
                    );
                }
            },
            ScheduleEvent::SetInputDelay { peer, delay } => {
                if *peer >= n {
                    return Err(format!(
                        "SetInputDelay peer {peer} out of range for a {n}-peer mesh"
                    ));
                }
                if *delay > MAX_FRAME_DELAY {
                    return Err(format!(
                        "SetInputDelay delay must be <= {MAX_FRAME_DELAY} (got {delay})"
                    ));
                }
            },
            ScheduleEvent::GracefulRemove { by, target }
            | ScheduleEvent::LegacyDisconnect { by, target } => {
                if *by >= n || *target >= n {
                    return Err(format!(
                        "lifecycle drop ({by} -> {target}) out of range for a {n}-peer mesh"
                    ));
                }
                if by == target {
                    return Err(format!(
                        "lifecycle drop target must be remote (by={by}, target={target})"
                    ));
                }
            },
            ScheduleEvent::PeerKill { peer } => {
                if *peer >= n {
                    return Err(format!(
                        "PeerKill peer {peer} out of range for a {n}-peer mesh"
                    ));
                }
            },
            ScheduleEvent::Rebind { peer } => {
                if *peer >= n {
                    return Err(format!(
                        "Rebind peer {peer} out of range for a {n}-peer mesh"
                    ));
                }
            },
            ScheduleEvent::SpectatorHostKill { host } => {
                if *host >= n {
                    return Err(format!(
                        "SpectatorHostKill host {host} out of range for a {n}-peer mesh"
                    ));
                }
            },
            ScheduleEvent::HotJoin { slot } => {
                if *slot >= n {
                    return Err(format!(
                        "HotJoin slot {slot} out of range for a {n}-peer mesh"
                    ));
                }
            },
            ScheduleEvent::HealAll => {},
        }
    }
    for (_, event) in &schedule.events {
        if let ScheduleEvent::SetLink { from, to, policy } = event {
            validate_link_policy(policy, &format!("SetLink ({from} -> {to})"))?;
        }
    }

    // Events execute in their stable serialized order at step-top. A stalled
    // application does not poll, advance, or invoke local configuration APIs,
    // so SetInputDelay cannot occur inside that peer's half-open stall window.
    // Same-step SetInputDelay-before-PeerStall is valid; the reverse order is
    // rejected, matching the runner's event application order exactly.
    let mut stalled_until = vec![0u32; n];
    for (step, event) in &schedule.events {
        match event {
            ScheduleEvent::PeerStall { peer, steps } => {
                stalled_until[*peer] = stalled_until[*peer].max(step.saturating_add(*steps));
            },
            ScheduleEvent::SetInputDelay { peer, .. } if *step < stalled_until[*peer] => {
                return Err(format!(
                    "SetInputDelay for peer {peer} at step {step} overlaps its PeerStall window \
                     ending at step {} (a stalled application cannot invoke local config APIs)",
                    stalled_until[*peer]
                ));
            },
            ScheduleEvent::SetLink { .. }
            | ScheduleEvent::Block { .. }
            | ScheduleEvent::Hold { .. }
            | ScheduleEvent::SetInputDelay { .. }
            | ScheduleEvent::GracefulRemove { .. }
            | ScheduleEvent::LegacyDisconnect { .. }
            | ScheduleEvent::PeerKill { .. }
            | ScheduleEvent::Rebind { .. }
            | ScheduleEvent::SpectatorHostKill { .. }
            | ScheduleEvent::HotJoin { .. }
            | ScheduleEvent::HealAll => {},
        }
    }

    if schedule.config.clock_skew_ppm.len() > n {
        return Err(format!(
            "clock_skew_ppm has {} entries for a {n}-peer mesh",
            schedule.config.clock_skew_ppm.len()
        ));
    }
    for &ppm in &schedule.config.clock_skew_ppm {
        if ppm < -1_000_000 {
            return Err(format!(
                "clock_skew_ppm must be >= -1_000_000 (-100% = a frozen clock); a \
                 lower value would run time backwards (got {ppm})"
            ));
        }
        if schedule.config.frame_model == FrameModel::SkewGated60Hz && ppm > 1_000_000 {
            return Err(format!(
                "clock_skew_ppm must be <= 1_000_000 (+100%) under SkewGated60Hz; \
                 larger values would exceed the bounded frame-work model (got {ppm})"
            ));
        }
    }
    if schedule.config.frame_model == FrameModel::SkewGated60Hz {
        let fastest_ppm = schedule
            .config
            .clock_skew_ppm
            .iter()
            .copied()
            .chain((schedule.config.clock_skew_ppm.len() < n).then_some(0))
            .max()
            .unwrap_or(0);
        let rate = u128::try_from(i64::from(1_000_000) + i64::from(fastest_ppm)).unwrap_or(0);
        let local_millis_numerator = u128::from(schedule.config.step_dt_ms).saturating_mul(rate);
        let local_millis_denominator = 1_000_000_u128;
        if local_millis_numerator
            > local_millis_denominator.saturating_mul(MAX_SKEW_GATED_LOCAL_MS_PER_STEP)
        {
            return Err(format!(
                "SkewGated60Hz requires at most one frame opportunity per peer per outer step; \
                 step_dt_ms={} and fastest clock skew {fastest_ppm} ppm exceed that bound",
                schedule.config.step_dt_ms
            ));
        }
    }
    let mut starved_peers = BTreeSet::new();
    for &peer in &schedule.config.starve_events {
        if peer >= n {
            return Err(format!(
                "starve_events peer {peer} out of range for a {n}-peer mesh"
            ));
        }
        if !starved_peers.insert(peer) {
            return Err(format!("duplicate starve_events peer {peer}"));
        }
    }

    let mut spectator_host_enabled = vec![false; n];
    for &peer in &schedule.config.spectator_hosts {
        if peer >= n {
            return Err(format!(
                "spectator_hosts peer {peer} out of range for a {n}-peer mesh"
            ));
        }
        if spectator_host_enabled[peer] {
            return Err(format!("duplicate spectator_hosts peer {peer}"));
        }
        spectator_host_enabled[peer] = true;
    }

    // Only guaranteed harness kills can make a later SpectatorHostKill or
    // HotJoin malformed up front. User API drops retire only after the runtime
    // call returns `Ok`, so they do not update this mask.
    let mut retired_by_guaranteed_kill = vec![false; n];
    let mut rebound_peers = BTreeSet::new();
    let mut hot_join_seen = false;
    for (_, event) in &schedule.events {
        match event {
            ScheduleEvent::GracefulRemove { .. } | ScheduleEvent::LegacyDisconnect { .. } => {},
            ScheduleEvent::PeerKill { peer } => {
                retired_by_guaranteed_kill[*peer] = true;
            },
            ScheduleEvent::Rebind { peer } => {
                if retired_by_guaranteed_kill[*peer] {
                    return Err(format!(
                        "Rebind peer {peer} is already retired by an earlier kill event"
                    ));
                }
                if !rebound_peers.insert(*peer) {
                    return Err(format!(
                        "Rebind peer {peer} appears more than once; the Rebind capability supports one \
                         address generation change per peer"
                    ));
                }
            },
            ScheduleEvent::SpectatorHostKill { host } => {
                if !spectator_host_enabled[*host] {
                    return Err(format!(
                        "SpectatorHostKill host {host} is not configured in spectator_hosts"
                    ));
                }
                if retired_by_guaranteed_kill[*host] {
                    return Err(format!(
                        "SpectatorHostKill host {host} is already retired by an earlier kill event"
                    ));
                }
                retired_by_guaranteed_kill[*host] = true;
            },
            ScheduleEvent::HotJoin { slot } => {
                if schedule.schema_version >= 12 && hot_join_seen {
                    return Err(
                        "schema v12 HotJoin appears more than once; the simulation capability supports one replacement generation per schedule"
                            .to_owned(),
                    );
                }
                hot_join_seen = true;
                if retired_by_guaranteed_kill[*slot] {
                    return Err(format!(
                        "HotJoin slot {slot} is already retired by an earlier kill event"
                    ));
                }
                let Some(host) = hot_join_host_for_slot(n, *slot) else {
                    return Err(format!(
                        "HotJoin requires at least two players; got n_players={n}, slot={slot}"
                    ));
                };
                if retired_by_guaranteed_kill[host] {
                    return Err(format!(
                        "HotJoin slot {slot}'s coordinator peer {host} is already retired by an \
                         earlier kill event"
                    ));
                }
            },
            ScheduleEvent::SetLink { .. }
            | ScheduleEvent::Block { .. }
            | ScheduleEvent::Hold { .. }
            | ScheduleEvent::PeerStall { .. }
            | ScheduleEvent::SetInputDelay { .. }
            | ScheduleEvent::HealAll => {},
        }
    }

    if let Some(size) = schedule.config.event_queue_size {
        if size < 10 {
            return Err(format!(
                "event_queue_size must be >= 10 (SessionBuilder rejects smaller); got {size}"
            ));
        }
    }
    if !(1..=MAX_SIMULATION_STEP_DT_MS).contains(&schedule.config.step_dt_ms) {
        return Err(format!(
            "step_dt_ms must be within 1..={MAX_SIMULATION_STEP_DT_MS} (got {})",
            schedule.config.step_dt_ms
        ));
    }

    Ok(())
}

/// Storyline kinds — multi-step fault narratives layered over the background
/// noise, FoundationDB-buggify style. These target the emergent N-peer corner
/// space (windows of heavy asymmetric loss, black-holes, and reordering) that
/// uniform i.i.d. noise rarely reaches.
#[derive(Copy, Clone, Debug)]
enum StorylineKind {
    /// A window of heavy loss on one directed link.
    HeavyLossWindow,
    /// A window where one direction of a pair is fully black-holed
    /// (asymmetric partition).
    AsymmetricBlackhole,
    /// Capture-and-hold one directed link, then release (mass reorder).
    HoldRelease,
}

/// Deterministic uniform helpers over the generator RNG.
struct Draw<'a> {
    rng: &'a mut Pcg32,
}

impl Draw<'_> {
    /// Uniform `f64` in `[0, 1)`.
    fn unit(&mut self) -> f64 {
        f64::from(self.rng.next_u32()) / (f64::from(u32::MAX) + 1.0)
    }

    /// Uniform integer in `[lo, hi]` (inclusive). `hi >= lo` required.
    fn range_u64(&mut self, lo: u64, hi: u64) -> u64 {
        debug_assert!(hi >= lo);
        let span = hi - lo + 1;
        lo + (self.rng.next_u64() % span)
    }

    /// Uniform `f64` in `[lo, hi)`.
    fn range_f64(&mut self, lo: f64, hi: f64) -> f64 {
        self.unit().mul_add(hi - lo, lo)
    }
}

/// Rolls one background-noise link policy.
fn roll_background_policy(draw: &mut Draw<'_>, noise: BackgroundNoise) -> LinkPolicy {
    match noise {
        BackgroundNoise::Clean => LinkPolicy::clean(),
        BackgroundNoise::ReliableFifo => LinkPolicy {
            base_delay: Duration::from_millis(30),
            ..LinkPolicy::clean()
        },
        BackgroundNoise::Mild => LinkPolicy {
            drop_rate: draw.range_f64(0.0, 0.02),
            dup_rate: draw.range_f64(0.0, 0.01),
            base_delay: Duration::from_millis(draw.range_u64(5, 30)),
            jitter: Duration::from_millis(draw.range_u64(0, 10)),
            burst_rate: 0.0,
            burst_len: 0,
            retransmit_delay: Duration::ZERO,
            gilbert_elliott: None,
            fragmentation: None,
            bandwidth: None,
        },
        BackgroundNoise::Rough => LinkPolicy {
            drop_rate: draw.range_f64(0.02, 0.10),
            dup_rate: draw.range_f64(0.0, 0.03),
            base_delay: Duration::from_millis(draw.range_u64(20, 80)),
            jitter: Duration::from_millis(draw.range_u64(0, 30)),
            burst_rate: draw.range_f64(0.0, 0.005),
            burst_len: u32::try_from(draw.range_u64(2, 5)).unwrap_or(3),
            retransmit_delay: Duration::ZERO,
            gilbert_elliott: None,
            fragmentation: None,
            bandwidth: None,
        },
    }
}

const GENERATED_LIFECYCLE_KINDS: [LifecycleEventKind; 5] = [
    LifecycleEventKind::PeerStall,
    LifecycleEventKind::SetInputDelay,
    LifecycleEventKind::PeerKill,
    LifecycleEventKind::GracefulRemove,
    LifecycleEventKind::SpectatorHostKill,
];

fn generated_lifecycle_kind(seed: u64, config: &SimConfig) -> Option<LifecycleEventKind> {
    if config.scenario_mix == ScenarioMix::NetworkOnly {
        return None;
    }

    let mut eligible = GENERATED_LIFECYCLE_KINDS.to_vec();
    if config.input_delay >= MAX_FRAME_DELAY {
        eligible.retain(|kind| *kind != LifecycleEventKind::SetInputDelay);
    }
    #[cfg(feature = "hot-join")]
    if config.n_players == 2 && config.max_prediction >= 1 {
        eligible.push(LifecycleEventKind::HotJoin);
    }

    let index = usize::try_from(seed % u64::try_from(eligible.len()).unwrap_or(1)).unwrap_or(0);
    eligible.get(index).copied()
}

fn prepare_lifecycle_config(seed: u64, config: &mut SimConfig) -> Option<LifecycleEventKind> {
    let kind = generated_lifecycle_kind(seed, config)?;
    let subject = usize::try_from(seed % u64::try_from(config.n_players).unwrap_or(1)).unwrap_or(0);

    match kind {
        LifecycleEventKind::PeerKill | LifecycleEventKind::GracefulRemove => {
            // Retirement stories are graceful-degradation probes; Halt
            // intentionally stops the whole mesh and is covered by the Halt
            // fail-closed production pin.
            config.disconnect_behavior = DropPolicy::ContinueWithout;
        },
        LifecycleEventKind::SpectatorHostKill => {
            config.disconnect_behavior = DropPolicy::ContinueWithout;
            let surviving_host = (subject + 1) % config.n_players;
            for host in [subject, surviving_host] {
                if !config.spectator_hosts.contains(&host) {
                    config.spectator_hosts.push(host);
                }
            }
        },
        LifecycleEventKind::HotJoin => {
            // Public hot-join construction requires zero input delay. N-peer
            // snapshot/ack convergence also needs the established 900-step
            // lifecycle budget (the planted hot-join schedule heals at 650);
            // shorter smoke schedules can end while the joiner is legitimately
            // still HotJoining.
            config.input_delay = 0;
            config.steps = config.steps.max(900);
            config.disconnect_behavior = DropPolicy::ContinueWithout;
        },
        LifecycleEventKind::PeerStall | LifecycleEventKind::SetInputDelay => {},
        LifecycleEventKind::LegacyDisconnect | LifecycleEventKind::Rebind => {
            unreachable!("planted-only lifecycle event cannot be randomly generated")
        },
    }
    Some(kind)
}

fn lifecycle_event_step(seed: u64, heal_at: u32) -> u32 {
    let last_pre_heal = heal_at.saturating_sub(1);
    let first = last_pre_heal.min(100);
    let width = last_pre_heal.saturating_sub(first).saturating_add(1);
    first.saturating_add(u32::try_from(seed % u64::from(width.max(1))).unwrap_or(0))
}

fn materialize_lifecycle_event(
    kind: LifecycleEventKind,
    seed: u64,
    config: &SimConfig,
) -> ScheduleEvent {
    let n = config.n_players;
    let subject = usize::try_from(seed % u64::try_from(n).unwrap_or(1)).unwrap_or(0);
    let other = (subject + 1) % n;
    match kind {
        LifecycleEventKind::PeerStall => ScheduleEvent::PeerStall {
            peer: subject,
            steps: 20 + u32::try_from(seed % 21).unwrap_or(0),
        },
        LifecycleEventKind::SetInputDelay => ScheduleEvent::SetInputDelay {
            peer: subject,
            delay: config.input_delay.saturating_add(1).min(MAX_FRAME_DELAY),
        },
        LifecycleEventKind::PeerKill => ScheduleEvent::PeerKill { peer: subject },
        LifecycleEventKind::GracefulRemove => ScheduleEvent::GracefulRemove {
            by: subject,
            target: other,
        },
        LifecycleEventKind::SpectatorHostKill => ScheduleEvent::SpectatorHostKill { host: subject },
        LifecycleEventKind::HotJoin => ScheduleEvent::HotJoin { slot: subject },
        LifecycleEventKind::LegacyDisconnect | LifecycleEventKind::Rebind => {
            unreachable!("planted-only lifecycle event cannot be randomly generated")
        },
    }
}

/// Generates the fully materialized schedule for `(seed, config)`.
///
/// Pure and deterministic: same inputs, same schedule, always.
#[must_use]
pub fn generate(seed: u64, mut config: SimConfig) -> Schedule {
    assert!(
        (2..=16).contains(&config.n_players),
        "simulation supports 2..=16 players (got {})",
        config.n_players
    );
    // The schedule always appends a `HealAll` at `heal_at` followed by a drain
    // window, and the run loop is `0..steps`. With fewer than 2 steps `heal_at`
    // saturates to 1 (or the loop is empty), stranding the heal at a step that
    // never runs and violating the heal-before-end invariant. Guard it like the
    // player bound; real configs use hundreds of steps (smoke default 600).
    assert!(
        config.steps >= 2,
        "simulation needs at least 2 steps so the appended HealAll lands inside \
         the run (got {})",
        config.steps
    );
    let lifecycle_kind = prepare_lifecycle_config(seed, &mut config);
    // Domain-separated generator stream; link noise gets its own stream so
    // shrinking events never perturbs background rolls.
    let mut rng = Pcg32::seed_from_u64(seed ^ 0x5EED_5EED_0000_0001);
    let link_seed = seed ^ 0x1111_2222_3333_4444;
    let mut draw = Draw { rng: &mut rng };

    // Drain window: cover the default 2s disconnect timeout + sync retries
    // with margin at 16ms steps, but never consume the whole schedule.
    let drain_steps = (config.steps / 2).min(250);
    let heal_at = config.steps.saturating_sub(drain_steps).max(1);

    // Explicit initial policy for every directed link.
    let n = config.n_players;
    let mut initial_links = Vec::new();
    for from in 0..n {
        for to in 0..n {
            if from != to {
                let policy = roll_background_policy(&mut draw, config.noise);
                initial_links.push((from, to, policy));
            }
        }
    }

    // 0..=3 storyline segments, each entirely inside (10%..90%) of the
    // pre-heal window and short enough (≤60 steps ≈ 1s virtual) not to trip
    // the 2s disconnect timeout on its own — full disconnect lifecycles are
    // introduced deliberately in the lifecycle vocabulary, not by accident.
    let mut events: Vec<(u32, ScheduleEvent)> = Vec::new();
    let storyline_budget = heal_at.saturating_sub(20);
    let n_storylines = if matches!(config.noise, BackgroundNoise::ReliableFifo) {
        0
    } else if storyline_budget > 100 {
        draw.range_u64(0, 3)
    } else {
        0
    };
    for _ in 0..n_storylines {
        let kind = match draw.range_u64(0, 2) {
            0 => StorylineKind::HeavyLossWindow,
            1 => StorylineKind::AsymmetricBlackhole,
            _ => StorylineKind::HoldRelease,
        };
        let from = usize::try_from(draw.range_u64(0, (n - 1) as u64)).unwrap_or(0);
        let mut to = usize::try_from(draw.range_u64(0, (n - 1) as u64)).unwrap_or(0);
        if to == from {
            to = (to + 1) % n;
        }
        let lo = storyline_budget / 10;
        let hi = storyline_budget.saturating_sub(70).max(lo + 1);
        let start = u32::try_from(draw.range_u64(u64::from(lo), u64::from(hi))).unwrap_or(lo);
        let duration = u32::try_from(draw.range_u64(10, 60)).unwrap_or(30);
        let end = (start + duration).min(heal_at.saturating_sub(1));

        match kind {
            StorylineKind::HeavyLossWindow => {
                let heavy = LinkPolicy {
                    drop_rate: draw.range_f64(0.25, 0.45),
                    ..roll_background_policy(&mut draw, config.noise)
                };
                let restore = roll_background_policy(&mut draw, config.noise);
                events.push((
                    start,
                    ScheduleEvent::SetLink {
                        from,
                        to,
                        policy: heavy,
                    },
                ));
                events.push((
                    end,
                    ScheduleEvent::SetLink {
                        from,
                        to,
                        policy: restore,
                    },
                ));
            },
            StorylineKind::AsymmetricBlackhole => {
                events.push((
                    start,
                    ScheduleEvent::Block {
                        from,
                        to,
                        blocked: true,
                    },
                ));
                events.push((
                    end,
                    ScheduleEvent::Block {
                        from,
                        to,
                        blocked: false,
                    },
                ));
            },
            StorylineKind::HoldRelease => {
                events.push((
                    start,
                    ScheduleEvent::Hold {
                        from,
                        to,
                        holding: true,
                    },
                ));
                events.push((
                    end,
                    ScheduleEvent::Hold {
                        from,
                        to,
                        holding: false,
                    },
                ));
            },
        }
    }

    if let Some(kind) = lifecycle_kind {
        events.push((
            lifecycle_event_step(seed, heal_at),
            materialize_lifecycle_event(kind, seed, &config),
        ));
    }
    events.push((heal_at, ScheduleEvent::HealAll));
    // Stable sort: same-step events keep their push order.
    events.sort_by_key(|(step, _)| *step);

    Schedule {
        schema_version: SCHEDULE_SCHEMA_VERSION,
        seed,
        link_seed,
        config,
        initial_links,
        events,
        heal_at,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn valid_gilbert_elliott() -> GilbertElliottPolicy {
        GilbertElliottPolicy {
            good_to_bad: 0.05,
            bad_to_good: 0.20,
            good_drop_rate: 0.01,
            bad_drop_rate: 0.80,
        }
    }

    #[test]
    fn generate_is_pure_same_seed_same_schedule() {
        let a = generate(42, SimConfig::smoke(4));
        let b = generate(42, SimConfig::smoke(4));
        assert_eq!(a, b, "generation must be a pure function of (seed, config)");
    }

    #[test]
    fn generate_differs_across_seeds() {
        let a = generate(1, SimConfig::smoke(4));
        let b = generate(2, SimConfig::smoke(4));
        assert_ne!(a, b, "different seeds should differ (links or events)");
    }

    #[test]
    fn reliable_fifo_profile_is_lossless_ordered_and_storyline_free() {
        let schedule = generate(
            42,
            SimConfig {
                noise: BackgroundNoise::ReliableFifo,
                ..SimConfig::smoke(4)
            },
        );
        for (from, to, policy) in &schedule.initial_links {
            assert_ne!(from, to);
            assert!(policy.drop_rate.abs() < f64::EPSILON);
            assert!(policy.dup_rate.abs() < f64::EPSILON);
            assert_eq!(policy.base_delay, Duration::from_millis(30));
            assert_eq!(policy.jitter, Duration::ZERO);
            assert!(policy.burst_rate.abs() < f64::EPSILON);
            assert_eq!(policy.burst_len, 0);
            assert_eq!(policy.retransmit_delay, Duration::ZERO);
            assert_eq!(policy.gilbert_elliott, None);
        }
        assert_eq!(
            schedule
                .events
                .iter()
                .filter(|(_, event)| !matches!(event, ScheduleEvent::HealAll))
                .count(),
            0,
            "ReliableFifo must not add random loss, partition, or reorder storylines"
        );
    }

    #[test]
    fn generate_covers_every_directed_link() {
        for n in 2..=16 {
            let schedule = generate(7, SimConfig::smoke(n));
            assert_eq!(
                schedule.initial_links.len(),
                n * (n - 1),
                "every directed pair needs an explicit policy (n={n})"
            );
        }
    }

    #[test]
    fn generate_always_heals_before_end() {
        for seed in 0..50 {
            let schedule = generate(seed, SimConfig::smoke(3));
            assert!(schedule.heal_at < schedule.config.steps);
            assert!(
                schedule
                    .events
                    .iter()
                    .any(|(step, ev)| *step == schedule.heal_at
                        && matches!(ev, ScheduleEvent::HealAll)),
                "HealAll must be scheduled (seed={seed})"
            );
            // All events happen at or before heal.
            assert!(
                schedule
                    .events
                    .iter()
                    .all(|(step, _)| *step <= schedule.heal_at),
                "no fault events may fire inside the drain window (seed={seed})"
            );
        }
    }

    /// The heal-before-end invariant must hold at the minimum allowed step
    /// count, not just for the smoke default — the boundary the `steps >= 2`
    /// guard protects.
    #[test]
    fn generate_heals_before_end_at_minimum_steps() {
        let schedule = generate(
            7,
            SimConfig {
                steps: 2,
                ..SimConfig::smoke(2)
            },
        );
        assert!(schedule.heal_at < schedule.config.steps);
        assert!(schedule
            .events
            .iter()
            .any(|(step, ev)| *step == schedule.heal_at && matches!(ev, ScheduleEvent::HealAll)));
    }

    /// Too few steps would strand `HealAll` outside the `0..steps` run — the
    /// guard rejects it rather than emitting an inconsistent schedule.
    #[test]
    #[should_panic(expected = "at least 2 steps")]
    fn generate_rejects_degenerate_step_count() {
        let _ = generate(
            0,
            SimConfig {
                steps: 1,
                ..SimConfig::smoke(2)
            },
        );
    }

    #[test]
    fn events_are_sorted_by_step() {
        for seed in 0..50 {
            let schedule = generate(seed, SimConfig::smoke(5));
            let steps: Vec<u32> = schedule.events.iter().map(|(s, _)| *s).collect();
            let mut sorted = steps.clone();
            sorted.sort_unstable();
            assert_eq!(steps, sorted, "events must be sorted (seed={seed})");
        }
    }

    #[test]
    fn schedule_round_trips_through_json() {
        let schedule = generate(42, SimConfig::smoke(4));
        let json = serde_json::to_string(&schedule).unwrap();
        let back: Schedule = serde_json::from_str(&json).unwrap();
        assert_eq!(
            schedule, back,
            "corpus artifacts must round-trip losslessly"
        );
    }

    #[test]
    fn validator_accepts_generated_and_valid_hand_authored_schedules() {
        for n_players in 2..=16 {
            let schedule = generate(42, SimConfig::smoke(n_players));
            assert_eq!(
                validate_schedule(&schedule),
                Ok(()),
                "generated n={n_players} schedule must validate"
            );
        }

        let mut hand_authored = generate(7, SimConfig::smoke(3));
        hand_authored.config.spectator_hosts = vec![0, 1];
        hand_authored
            .events
            .push((100, ScheduleEvent::PeerStall { peer: 2, steps: 20 }));
        hand_authored
            .events
            .push((150, ScheduleEvent::SetInputDelay { peer: 1, delay: 2 }));
        hand_authored
            .events
            .push((200, ScheduleEvent::SpectatorHostKill { host: 0 }));
        hand_authored.events.sort_by_key(|(step, _)| *step);
        assert_eq!(
            validate_schedule(&hand_authored),
            Ok(()),
            "a structurally valid hand-authored lifecycle schedule must validate"
        );
    }

    #[test]
    fn network_only_default_preserves_representative_generated_schedules() {
        for (seed, n_players) in [(1, 2), (7, 4), (34, 8), (99, 16)] {
            let default_config = SimConfig::smoke(n_players);
            let explicit_config = SimConfig {
                scenario_mix: ScenarioMix::NetworkOnly,
                ..default_config.clone()
            };
            assert_eq!(
                generate(seed, default_config.clone()),
                generate(seed, explicit_config),
                "the default scenario axis must not perturb seed {seed} n={n_players}"
            );

            let mut old_value = serde_json::to_value(&default_config).unwrap();
            assert!(
                old_value
                    .as_object_mut()
                    .and_then(|config| config.remove("scenario_mix"))
                    .is_some(),
                "serialized SimConfig must contain scenario_mix before migration removal"
            );
            let defaulted: SimConfig = serde_json::from_value(old_value).unwrap();
            assert_eq!(defaulted.scenario_mix, ScenarioMix::NetworkOnly);
            assert_eq!(
                generate(seed, default_config),
                generate(seed, defaulted),
                "an old/defaulted config must preserve seed {seed} n={n_players} exactly"
            );
        }
    }

    #[test]
    fn validator_accepts_supported_legacy_schema_versions_when_capabilities_fit() {
        let mut v1_value = serde_json::to_value(generate(7, SimConfig::smoke(3))).unwrap();
        *v1_value
            .get_mut("schema_version")
            .expect("schedule serializes schema_version") = serde_json::json!(1);
        let v1_config = v1_value
            .get_mut("config")
            .and_then(serde_json::Value::as_object_mut)
            .expect("schedule serializes config");
        assert!(v1_config.remove("scenario_mix").is_some());
        let v1: Schedule = serde_json::from_value(v1_value).unwrap();
        assert_eq!(validate_schedule(&v1), Ok(()));

        let mut v8 = generate(
            8,
            SimConfig {
                input_delay: 0,
                ..SimConfig::smoke(3)
            },
        );
        v8.schema_version = 8;
        #[cfg(feature = "hot-join")]
        {
            v8.events.push((100, ScheduleEvent::HotJoin { slot: 1 }));
            v8.events.sort_by_key(|(step, _)| *step);
        }
        let v8_json = serde_json::to_vec(&v8).unwrap();
        let v8_round_trip: Schedule = serde_json::from_slice(&v8_json).unwrap();
        assert_eq!(validate_schedule(&v8_round_trip), Ok(()));

        let mut under_declared_v8 = v8_round_trip;
        under_declared_v8.config.noise = BackgroundNoise::ReliableFifo;
        assert!(validate_schedule(&under_declared_v8)
            .expect_err("schema v8 cannot claim the v9 reliable-FIFO capability")
            .contains("requiring schema_version 9"));
    }

    #[test]
    fn schema_capability_floor_tracks_every_versioned_event() {
        let cases = [
            (ScheduleEvent::PeerStall { peer: 1, steps: 10 }, 2),
            (ScheduleEvent::SetInputDelay { peer: 1, delay: 2 }, 3),
            (ScheduleEvent::PeerKill { peer: 1 }, 4),
            (ScheduleEvent::GracefulRemove { by: 0, target: 1 }, 5),
            (ScheduleEvent::LegacyDisconnect { by: 0, target: 1 }, 6),
            (ScheduleEvent::SpectatorHostKill { host: 0 }, 7),
            (ScheduleEvent::HotJoin { slot: 1 }, 8),
            (
                ScheduleEvent::SetLink {
                    from: 0,
                    to: 1,
                    policy: LinkPolicy {
                        retransmit_delay: Duration::from_millis(200),
                        ..LinkPolicy::clean()
                    },
                },
                9,
            ),
            (ScheduleEvent::Rebind { peer: 1 }, 10),
            (
                ScheduleEvent::SetLink {
                    from: 0,
                    to: 1,
                    policy: LinkPolicy {
                        gilbert_elliott: Some(valid_gilbert_elliott()),
                        ..LinkPolicy::clean()
                    },
                },
                11,
            ),
            (
                ScheduleEvent::SetLink {
                    from: 0,
                    to: 1,
                    policy: LinkPolicy {
                        fragmentation: Some(crate::common::sim_net::FragmentationPolicy {
                            fragment_drop_rate: 0.2,
                        }),
                        ..LinkPolicy::clean()
                    },
                },
                13,
            ),
            (
                ScheduleEvent::SetLink {
                    from: 0,
                    to: 1,
                    policy: LinkPolicy {
                        bandwidth: Some(crate::common::sim_net::BandwidthPolicy {
                            rate_bytes_per_second: 10_000,
                            burst_bytes: 1_500,
                            queue_capacity_bytes: 3_000,
                        }),
                        ..LinkPolicy::clean()
                    },
                },
                14,
            ),
        ];

        for (event, required) in cases {
            let mut schedule = generate(7, SimConfig::smoke(2));
            schedule
                .events
                .retain(|(_, existing)| matches!(existing, ScheduleEvent::HealAll));
            if matches!(event, ScheduleEvent::SpectatorHostKill { .. }) {
                schedule.config.spectator_hosts = vec![0, 1];
            }
            if matches!(event, ScheduleEvent::HotJoin { .. }) {
                schedule.config.input_delay = 0;
            }
            schedule.events.push((100, event));
            schedule.events.sort_by_key(|(step, _)| *step);

            schedule.schema_version = required - 1;
            let error = validate_schedule(&schedule)
                .expect_err("a schedule cannot under-declare a versioned capability");
            assert!(
                error.contains(&format!("requiring schema_version {required}")),
                "wrong capability floor for schema v{required}: {error}"
            );

            schedule.schema_version = required;
            #[cfg(feature = "hot-join")]
            assert_eq!(
                validate_schedule(&schedule),
                Ok(()),
                "schema v{required} must accept its declared capability"
            );
            #[cfg(not(feature = "hot-join"))]
            if required == 8 {
                assert!(validate_schedule(&schedule)
                    .expect_err("HotJoin still requires the build feature")
                    .contains("requires the crate's `hot-join` feature"));
            } else {
                assert_eq!(
                    validate_schedule(&schedule),
                    Ok(()),
                    "schema v{required} must accept its declared capability"
                );
            }
        }
    }

    #[test]
    fn schema_v11_floor_applies_to_initial_gilbert_elliott_links() {
        let mut schedule = generate(7, SimConfig::smoke(2));
        schedule.initial_links[0].2 = LinkPolicy {
            gilbert_elliott: Some(valid_gilbert_elliott()),
            ..LinkPolicy::clean()
        };
        schedule.schema_version = 10;
        assert!(validate_schedule(&schedule)
            .expect_err("schema v10 cannot claim a GE initial-link capability")
            .contains("requiring schema_version 11"));
        schedule.schema_version = 11;
        assert_eq!(validate_schedule(&schedule), Ok(()));
    }

    #[test]
    fn schema_v13_floor_applies_to_fragmentation_links() {
        let mut schedule = generate(7, SimConfig::smoke(2));
        schedule.initial_links[0].2.fragmentation =
            Some(crate::common::sim_net::FragmentationPolicy {
                fragment_drop_rate: 0.2,
            });
        schedule.schema_version = 12;
        assert!(validate_schedule(&schedule)
            .expect_err("schema v12 cannot claim fragmentation capability")
            .contains("requiring schema_version 13"));
        schedule.schema_version = 13;
        assert_eq!(validate_schedule(&schedule), Ok(()));
    }

    #[test]
    fn schema_v14_floor_applies_to_bandwidth_links() {
        let mut schedule = generate(7, SimConfig::smoke(2));
        schedule.initial_links[0].2.bandwidth = Some(crate::common::sim_net::BandwidthPolicy {
            rate_bytes_per_second: 10_000,
            burst_bytes: 1_500,
            queue_capacity_bytes: 3_000,
        });
        schedule.schema_version = 13;
        assert!(validate_schedule(&schedule)
            .expect_err("schema v13 cannot claim bandwidth capability")
            .contains("requiring schema_version 14"));
        schedule.schema_version = 14;
        assert_eq!(validate_schedule(&schedule), Ok(()));
        let json = serde_json::to_vec(&schedule).expect("bandwidth schedule serializes");
        let round_trip: Schedule =
            serde_json::from_slice(&json).expect("bandwidth schedule deserializes");
        assert_eq!(round_trip, schedule);
        assert_eq!(validate_schedule(&round_trip), Ok(()));
    }

    #[test]
    fn schema_v15_floor_applies_to_skew_gated_frame_model() {
        let mut schedule = generate(7, SimConfig::smoke(2));
        schedule.config.frame_model = FrameModel::SkewGated60Hz;
        schedule.schema_version = 14;
        assert!(validate_schedule(&schedule)
            .expect_err("schema v14 cannot claim skew-gated frame semantics")
            .contains("requiring schema_version 15"));
        schedule.schema_version = 15;
        assert_eq!(validate_schedule(&schedule), Ok(()));
    }

    #[test]
    fn bandwidth_policy_validates_rate_burst_and_memory_bound() {
        let mut schedule = generate(7, SimConfig::smoke(2));
        schedule.initial_links[0].2.bandwidth = Some(crate::common::sim_net::BandwidthPolicy {
            rate_bytes_per_second: 0,
            burst_bytes: 1,
            queue_capacity_bytes: 1,
        });
        assert!(validate_schedule(&schedule)
            .expect_err("zero bandwidth rate must fail")
            .contains("rate_bytes_per_second must be > 0"));

        schedule.initial_links[0].2.bandwidth = Some(crate::common::sim_net::BandwidthPolicy {
            rate_bytes_per_second: 1,
            burst_bytes: 0,
            queue_capacity_bytes: 1,
        });
        assert!(validate_schedule(&schedule)
            .expect_err("zero bandwidth burst must fail")
            .contains("burst_bytes must be > 0"));

        schedule.initial_links[0].2.bandwidth = Some(crate::common::sim_net::BandwidthPolicy {
            rate_bytes_per_second: 1,
            burst_bytes: 1,
            queue_capacity_bytes: MAX_SIMULATION_BANDWIDTH_BYTES + 1,
        });
        assert!(validate_schedule(&schedule)
            .expect_err("unbounded simulated queue must fail")
            .contains("queue_capacity_bytes must be <="));
    }

    #[test]
    fn fragmentation_policy_validates_probability_and_retransmission_exclusion() {
        let mut schedule = generate(7, SimConfig::smoke(2));
        schedule.initial_links[0].2.fragmentation =
            Some(crate::common::sim_net::FragmentationPolicy {
                fragment_drop_rate: f64::NAN,
            });
        assert!(validate_schedule(&schedule)
            .expect_err("NaN fragmentation probability must fail")
            .contains("fragment_drop_rate must be finite"));

        schedule.initial_links[0].2.fragmentation =
            Some(crate::common::sim_net::FragmentationPolicy {
                fragment_drop_rate: 0.2,
            });
        schedule.initial_links[0].2.retransmit_delay = Duration::from_millis(1);
        assert!(validate_schedule(&schedule)
            .expect_err("fragmentation plus retransmission must fail")
            .contains("fragmentation cannot be combined with retransmit_delay"));
    }

    #[test]
    fn persisted_schedule_envelopes_reject_unknown_fields() {
        let schedule = generate(7, SimConfig::smoke(3));

        let mut unknown_schedule = serde_json::to_value(&schedule).unwrap();
        unknown_schedule
            .as_object_mut()
            .expect("Schedule serializes as an object")
            .insert("unknown_schedule_field".to_owned(), serde_json::json!(true));
        let schedule_error = serde_json::from_value::<Schedule>(unknown_schedule)
            .expect_err("unknown Schedule fields must be rejected");
        assert!(schedule_error.to_string().contains("unknown field"));

        let mut unknown_config = serde_json::to_value(&schedule).unwrap();
        unknown_config
            .get_mut("config")
            .and_then(serde_json::Value::as_object_mut)
            .expect("Schedule config serializes as an object")
            .insert("unknown_config_field".to_owned(), serde_json::json!(true));
        let config_error = serde_json::from_value::<Schedule>(unknown_config)
            .expect_err("unknown SimConfig fields must be rejected");
        assert!(config_error.to_string().contains("unknown field"));

        let scenario_error = serde_json::from_str::<ScenarioMix>(r#""FutureMix""#)
            .expect_err("unknown ScenarioMix variants must be rejected");
        assert!(scenario_error.to_string().contains("unknown variant"));
    }

    #[test]
    fn lifecycle_generator_respects_input_delay_and_hot_join_boundaries() {
        for seed in 0..60u64 {
            let max_delay = generate(
                seed,
                SimConfig {
                    input_delay: MAX_FRAME_DELAY,
                    scenario_mix: ScenarioMix::Lifecycle,
                    ..SimConfig::smoke(4)
                },
            );
            assert_eq!(validate_schedule(&max_delay), Ok(()));
            assert!(
                max_delay
                    .events
                    .iter()
                    .all(|(_, event)| !matches!(event, ScheduleEvent::SetInputDelay { .. })),
                "seed {seed} generated a non-substantive increase at the maximum input delay"
            );

            #[cfg(feature = "hot-join")]
            {
                let no_prediction = generate(
                    seed,
                    SimConfig {
                        max_prediction: 0,
                        scenario_mix: ScenarioMix::Lifecycle,
                        ..SimConfig::smoke(2)
                    },
                );
                assert_eq!(validate_schedule(&no_prediction), Ok(()));
                assert!(
                    no_prediction
                        .events
                        .iter()
                        .all(|(_, event)| !matches!(event, ScheduleEvent::HotJoin { .. })),
                    "seed {seed} generated HotJoin with max_prediction=0"
                );
            }
        }
    }

    #[test]
    fn lifecycle_generator_covers_every_eligible_class_with_effective_valid_events() {
        let mut seen = std::collections::BTreeSet::new();

        for n_players in [4usize, 2] {
            for seed in 0..60u64 {
                let schedule = generate(
                    seed,
                    SimConfig {
                        scenario_mix: ScenarioMix::Lifecycle,
                        ..SimConfig::smoke(n_players)
                    },
                );
                assert_eq!(
                    validate_schedule(&schedule),
                    Ok(()),
                    "lifecycle seed {seed} n={n_players} must materialize a valid schedule"
                );
                assert_eq!(
                    schedule.effective_lifecycle_event_count(),
                    Ok(1),
                    "lifecycle seed {seed} n={n_players} must contain one effective operation"
                );

                let (step, event) = schedule
                    .events
                    .iter()
                    .find(|(_, event)| event.lifecycle_kind().is_some())
                    .expect("the effective lifecycle event must be present");
                assert!(
                    *step < schedule.heal_at,
                    "lifecycle event must occur before heal: seed={seed}, step={step}, heal={} ",
                    schedule.heal_at
                );
                let kind = event.lifecycle_kind().expect("event was selected by kind");
                let _ = seen.insert(kind);

                match event {
                    ScheduleEvent::PeerKill { .. } => assert_eq!(
                        schedule.config.disconnect_behavior,
                        DropPolicy::ContinueWithout,
                        "generated crash must leave at least one survivor able to continue"
                    ),
                    ScheduleEvent::SpectatorHostKill { host } => {
                        assert!(schedule.config.spectator_hosts.contains(host));
                        assert!(
                            schedule.config.spectator_hosts.len() >= 2,
                            "generated spectator kill must retain a redundant live host"
                        );
                        assert_eq!(
                            schedule.config.disconnect_behavior,
                            DropPolicy::ContinueWithout
                        );
                    },
                    ScheduleEvent::HotJoin { .. } => {
                        assert_eq!(n_players, 2);
                        assert_eq!(schedule.config.input_delay, 0);
                    },
                    ScheduleEvent::PeerStall { .. }
                    | ScheduleEvent::SetInputDelay { .. }
                    | ScheduleEvent::GracefulRemove { .. } => {},
                    ScheduleEvent::LegacyDisconnect { .. }
                    | ScheduleEvent::Rebind { .. }
                    | ScheduleEvent::SetLink { .. }
                    | ScheduleEvent::Block { .. }
                    | ScheduleEvent::Hold { .. }
                    | ScheduleEvent::HealAll => {
                        panic!("unexpected generated lifecycle event: {event:?}")
                    },
                }
            }
        }

        let expected = std::collections::BTreeSet::from([
            LifecycleEventKind::PeerStall,
            LifecycleEventKind::SetInputDelay,
            LifecycleEventKind::PeerKill,
            LifecycleEventKind::GracefulRemove,
            LifecycleEventKind::SpectatorHostKill,
            #[cfg(feature = "hot-join")]
            LifecycleEventKind::HotJoin,
        ]);
        assert_eq!(
            seen, expected,
            "bounded lifecycle seeds must cover every eligible generated class"
        );
    }

    #[test]
    fn lifecycle_generator_classes_execute_under_full_oracle() {
        let mut executed = std::collections::BTreeSet::new();

        for n_players in [4usize, 2] {
            for seed in 0..60u64 {
                let schedule = generate(
                    seed,
                    SimConfig {
                        noise: BackgroundNoise::Clean,
                        scenario_mix: ScenarioMix::Lifecycle,
                        ..SimConfig::smoke(n_players)
                    },
                );
                let kind = schedule
                    .events
                    .iter()
                    .find_map(|(_, event)| event.lifecycle_kind())
                    .expect("Lifecycle mode materializes one classified event");
                if !executed.insert(kind) {
                    continue;
                }

                let report = super::super::run(&schedule, &super::super::RunOptions::default());
                report.expect_pass(&schedule);
            }
        }

        let expected = std::collections::BTreeSet::from([
            LifecycleEventKind::PeerStall,
            LifecycleEventKind::SetInputDelay,
            LifecycleEventKind::PeerKill,
            LifecycleEventKind::GracefulRemove,
            LifecycleEventKind::SpectatorHostKill,
            #[cfg(feature = "hot-join")]
            LifecycleEventKind::HotJoin,
        ]);
        assert_eq!(
            executed, expected,
            "every eligible lifecycle class must execute through the full oracle"
        );
    }

    #[test]
    fn validator_rejects_set_input_delay_during_peer_stall() {
        let base = generate(7, SimConfig::smoke(3));
        let cases = [(100, 100), (100, 120), (100, 149)];

        for (stall_at, delay_at) in cases {
            let mut schedule = base.clone();
            schedule
                .events
                .retain(|(_, event)| matches!(event, ScheduleEvent::HealAll));
            schedule
                .events
                .push((stall_at, ScheduleEvent::PeerStall { peer: 1, steps: 50 }));
            schedule
                .events
                .push((delay_at, ScheduleEvent::SetInputDelay { peer: 1, delay: 2 }));
            schedule.events.sort_by_key(|(step, _)| *step);
            let error = validate_schedule(&schedule)
                .expect_err("SetInputDelay inside a stall window must be rejected");
            assert!(
                error.contains("overlaps its PeerStall window"),
                "unexpected overlap diagnostic: {error}"
            );
        }

        let mut at_boundary = base.clone();
        at_boundary
            .events
            .retain(|(_, event)| matches!(event, ScheduleEvent::HealAll));
        at_boundary
            .events
            .push((100, ScheduleEvent::PeerStall { peer: 1, steps: 50 }));
        at_boundary
            .events
            .push((150, ScheduleEvent::SetInputDelay { peer: 1, delay: 2 }));
        at_boundary.events.sort_by_key(|(step, _)| *step);
        assert_eq!(
            validate_schedule(&at_boundary),
            Ok(()),
            "the half-open stall window must permit reconfiguration at its end"
        );

        let mut before_same_step_stall = base;
        before_same_step_stall
            .events
            .retain(|(_, event)| matches!(event, ScheduleEvent::HealAll));
        before_same_step_stall
            .events
            .push((100, ScheduleEvent::SetInputDelay { peer: 1, delay: 2 }));
        before_same_step_stall
            .events
            .push((100, ScheduleEvent::PeerStall { peer: 1, steps: 50 }));
        before_same_step_stall.events.sort_by_key(|(step, _)| *step);
        assert_eq!(
            validate_schedule(&before_same_step_stall),
            Ok(()),
            "stable same-step ordering invokes SetInputDelay before the stall begins"
        );
    }

    #[test]
    fn validator_rejects_malformed_materialized_schedules_with_diagnostics() {
        let valid = generate(7, SimConfig::smoke(3));
        let mut cases: Vec<(Schedule, &str)> = Vec::new();

        let mut zero_schema = valid.clone();
        zero_schema.schema_version = 0;
        cases.push((zero_schema, "schema_version must be within"));

        let mut future_schema = valid.clone();
        future_schema.schema_version = SCHEDULE_SCHEMA_VERSION + 1;
        cases.push((future_schema, "schema_version must be within"));

        let mut under_declared = valid.clone();
        under_declared.schema_version = 1;
        under_declared
            .events
            .push((100, ScheduleEvent::PeerStall { peer: 1, steps: 10 }));
        under_declared.events.sort_by_key(|(step, _)| *step);
        cases.push((under_declared, "under-declares capability"));

        let mut too_few_players = valid.clone();
        too_few_players.config.n_players = 1;
        cases.push((too_few_players, "2..=16 players"));

        let mut too_few_steps = valid.clone();
        too_few_steps.config.steps = 1;
        cases.push((too_few_steps, "2..="));

        let mut excessive_input_delay = valid.clone();
        excessive_input_delay.config.input_delay = MAX_FRAME_DELAY + 1;
        cases.push((excessive_input_delay, "input_delay must be <="));

        let mut zero_desync_interval = valid.clone();
        zero_desync_interval.config.desync_interval = 0;
        cases.push((zero_desync_interval, "desync_interval must be >= 1"));

        let mut zero_smear = valid.clone();
        zero_smear.config.app_model = AppModel::Obey;
        zero_smear.config.wait_recommendation_policy.smear_interval = 0;
        cases.push((zero_smear, "smear_interval must be >= 1"));

        let mut zero_skip_cap = valid.clone();
        zero_skip_cap.config.app_model = AppModel::Obey;
        zero_skip_cap
            .config
            .wait_recommendation_policy
            .max_skip_frames = Some(0);
        cases.push((zero_skip_cap, "max_skip_frames must be >= 1"));

        let mut ignored_policy = valid.clone();
        ignored_policy
            .config
            .wait_recommendation_policy
            .cooldown_frames = 60;
        cases.push((ignored_policy, "requires app_model Obey"));

        let mut under_declared_policy = valid.clone();
        under_declared_policy.schema_version = 16;
        under_declared_policy.config.app_model = AppModel::Obey;
        under_declared_policy
            .config
            .wait_recommendation_policy
            .cooldown_frames = 60;
        cases.push((under_declared_policy, "under-declares capability"));

        let mut zero_cpu_cost = valid.clone();
        zero_cpu_cost.config.cpu_feedback_policy = Some(CpuFeedbackPolicy {
            simulated_frame_cost_us: 0,
            max_poll_delay_steps: 1,
        });
        cases.push((zero_cpu_cost, "simulated_frame_cost_us must be >= 1"));

        let mut excessive_cpu_cost = valid.clone();
        excessive_cpu_cost.config.cpu_feedback_policy = Some(CpuFeedbackPolicy {
            simulated_frame_cost_us: MAX_CPU_FRAME_COST_US + 1,
            max_poll_delay_steps: 1,
        });
        cases.push((excessive_cpu_cost, "simulated_frame_cost_us must be <="));

        let mut zero_cpu_cap = valid.clone();
        zero_cpu_cap.config.cpu_feedback_policy = Some(CpuFeedbackPolicy {
            simulated_frame_cost_us: 1,
            max_poll_delay_steps: 0,
        });
        cases.push((zero_cpu_cap, "max_poll_delay_steps must be within"));

        let mut excessive_cpu_cap = valid.clone();
        excessive_cpu_cap.config.cpu_feedback_policy = Some(CpuFeedbackPolicy {
            simulated_frame_cost_us: 1,
            max_poll_delay_steps: valid.config.steps + 1,
        });
        cases.push((excessive_cpu_cap, "max_poll_delay_steps must be within"));

        let mut cpu_hot_join = valid.clone();
        cpu_hot_join.config.cpu_feedback_policy = Some(CpuFeedbackPolicy {
            simulated_frame_cost_us: 1,
            max_poll_delay_steps: 1,
        });
        cpu_hot_join
            .events
            .push((10, ScheduleEvent::HotJoin { slot: 1 }));
        cpu_hot_join.events.sort_by_key(|(step, _)| *step);
        cases.push((cpu_hot_join, "cannot be combined with HotJoin"));

        let mut unsorted = valid.clone();
        unsorted.events = vec![(10, ScheduleEvent::HealAll), (9, ScheduleEvent::HealAll)];
        cases.push((unsorted, "sorted by nondecreasing step"));

        let mut event_outside_run = valid.clone();
        event_outside_run
            .events
            .push((event_outside_run.config.steps, ScheduleEvent::HealAll));
        event_outside_run.events.sort_by_key(|(step, _)| *step);
        cases.push((event_outside_run, "outside the run"));

        let mut post_heal_fault = valid.clone();
        post_heal_fault.events.push((
            post_heal_fault.heal_at + 1,
            ScheduleEvent::Block {
                from: 0,
                to: 1,
                blocked: true,
            },
        ));
        post_heal_fault.events.sort_by_key(|(step, _)| *step);
        cases.push((post_heal_fault, "after the last HealAll"));

        let mut missing_link = valid.clone();
        let _ = missing_link.initial_links.pop();
        cases.push((missing_link, "exactly one directed policy"));

        let mut out_of_range_link = valid.clone();
        out_of_range_link.initial_links[0].0 = 9;
        cases.push((out_of_range_link, "initial link (9 ->"));

        let mut self_link = valid.clone();
        self_link.initial_links[0].1 = self_link.initial_links[0].0;
        cases.push((self_link, "must not target itself"));

        let mut duplicate_link = valid.clone();
        let duplicate_pair = (
            duplicate_link.initial_links[0].0,
            duplicate_link.initial_links[0].1,
        );
        duplicate_link.initial_links[1].0 = duplicate_pair.0;
        duplicate_link.initial_links[1].1 = duplicate_pair.1;
        cases.push((duplicate_link, "duplicate initial link"));

        for (field, value, expected) in [
            ("drop_rate", f64::NAN, "drop_rate must be finite"),
            ("dup_rate", -0.1, "dup_rate must be finite"),
            ("burst_rate", 1.1, "burst_rate must be finite"),
        ] {
            let mut invalid_probability = valid.clone();
            let policy = &mut invalid_probability.initial_links[0].2;
            match field {
                "drop_rate" => policy.drop_rate = value,
                "dup_rate" => policy.dup_rate = value,
                "burst_rate" => policy.burst_rate = value,
                _ => unreachable!("table contains only policy probability fields"),
            }
            cases.push((invalid_probability, expected));
        }

        let mut burst_rate_without_length = valid.clone();
        burst_rate_without_length.initial_links[0].2.burst_rate = 0.1;
        burst_rate_without_length.initial_links[0].2.burst_len = 0;
        cases.push((
            burst_rate_without_length,
            "burst_rate/burst_len must both be zero or both be enabled",
        ));

        let mut burst_length_without_rate = valid.clone();
        burst_length_without_rate.initial_links[0].2.burst_rate = 0.0;
        burst_length_without_rate.initial_links[0].2.burst_len = 3;
        cases.push((
            burst_length_without_rate,
            "burst_rate/burst_len must both be zero or both be enabled",
        ));

        for (field, value, expected) in [
            ("good_to_bad", f64::NAN, "good_to_bad must be finite"),
            ("bad_to_good", -0.1, "bad_to_good must be finite"),
            ("good_drop_rate", 1.1, "good_drop_rate must be finite"),
            (
                "bad_drop_rate",
                f64::INFINITY,
                "bad_drop_rate must be finite",
            ),
        ] {
            let mut invalid_ge_probability = valid.clone();
            let mut ge = valid_gilbert_elliott();
            match field {
                "good_to_bad" => ge.good_to_bad = value,
                "bad_to_good" => ge.bad_to_good = value,
                "good_drop_rate" => ge.good_drop_rate = value,
                "bad_drop_rate" => ge.bad_drop_rate = value,
                _ => unreachable!("table contains only GE probability fields"),
            }
            invalid_ge_probability.initial_links[0].2.gilbert_elliott = Some(ge);
            cases.push((invalid_ge_probability, expected));
        }

        let mut unreachable_bad_state = valid.clone();
        let mut ge = valid_gilbert_elliott();
        ge.good_to_bad = 0.0;
        unreachable_bad_state.initial_links[0].2.gilbert_elliott = Some(ge);
        cases.push((unreachable_bad_state, "good_to_bad must be > 0"));

        let mut non_correlated_drop_rates = valid.clone();
        let mut ge = valid_gilbert_elliott();
        ge.bad_drop_rate = ge.good_drop_rate;
        non_correlated_drop_rates.initial_links[0].2.gilbert_elliott = Some(ge);
        cases.push((
            non_correlated_drop_rates,
            "bad_drop_rate must be greater than good_drop_rate",
        ));

        for legacy_loss in ["drop_rate", "burst"] {
            let mut layered = valid.clone();
            layered.initial_links[0].2.gilbert_elliott = Some(valid_gilbert_elliott());
            if legacy_loss == "drop_rate" {
                layered.initial_links[0].2.drop_rate = 0.1;
            } else {
                layered.initial_links[0].2.burst_rate = 0.1;
                layered.initial_links[0].2.burst_len = 2;
            }
            cases.push((layered, "Gilbert-Elliott loss cannot be layered"));
        }

        let event_cases = [
            (
                ScheduleEvent::SetLink {
                    from: 0,
                    to: 1,
                    policy: LinkPolicy {
                        drop_rate: f64::INFINITY,
                        ..LinkPolicy::clean()
                    },
                },
                "SetLink (0 -> 1) drop_rate must be finite",
            ),
            (
                ScheduleEvent::Block {
                    from: 0,
                    to: 0,
                    blocked: true,
                },
                "schedule event link (0 -> 0) must not target itself",
            ),
            (
                ScheduleEvent::PeerStall { peer: 9, steps: 1 },
                "PeerStall peer 9 out of range",
            ),
            (
                ScheduleEvent::PeerStall { peer: 1, steps: 0 },
                "PeerStall steps must be > 0",
            ),
            (
                ScheduleEvent::SetInputDelay { peer: 9, delay: 1 },
                "SetInputDelay peer 9 out of range",
            ),
            (
                ScheduleEvent::SetInputDelay {
                    peer: 1,
                    delay: MAX_FRAME_DELAY + 1,
                },
                "SetInputDelay delay must be <=",
            ),
            (
                ScheduleEvent::GracefulRemove { by: 1, target: 1 },
                "lifecycle drop target must be remote",
            ),
            (
                ScheduleEvent::LegacyDisconnect { by: 9, target: 1 },
                "lifecycle drop (9 -> 1) out of range",
            ),
            (
                ScheduleEvent::PeerKill { peer: 9 },
                "PeerKill peer 9 out of range",
            ),
            (
                ScheduleEvent::Rebind { peer: 9 },
                "Rebind peer 9 out of range",
            ),
            (
                ScheduleEvent::SpectatorHostKill { host: 9 },
                "SpectatorHostKill host 9 out of range",
            ),
        ];
        for (event, expected) in event_cases {
            let mut malformed = valid.clone();
            malformed.events.push((100, event));
            malformed.events.sort_by_key(|(step, _)| *step);
            cases.push((malformed, expected));
        }

        let mut bad_skew = valid.clone();
        bad_skew.config.clock_skew_ppm = vec![-1_000_001];
        cases.push((bad_skew, "would run time backwards"));

        let mut too_many_skews = valid.clone();
        too_many_skews.config.clock_skew_ppm = vec![0; 4];
        cases.push((too_many_skews, "entries for a 3-peer mesh"));

        let mut too_fast_gated_clock = valid.clone();
        too_fast_gated_clock.config.frame_model = FrameModel::SkewGated60Hz;
        too_fast_gated_clock.config.clock_skew_ppm = vec![1_000_001];
        cases.push((too_fast_gated_clock, "must be <= 1_000_000"));

        let mut coarse_gated_cadence = valid.clone();
        coarse_gated_cadence.config.frame_model = FrameModel::SkewGated60Hz;
        coarse_gated_cadence.config.step_dt_ms = 17;
        cases.push((coarse_gated_cadence, "at most one frame opportunity"));

        let mut rounded_two_opportunity_boundary = valid.clone();
        rounded_two_opportunity_boundary.config.frame_model = FrameModel::SkewGated60Hz;
        rounded_two_opportunity_boundary.config.step_dt_ms = 16;
        rounded_two_opportunity_boundary.config.clock_skew_ppm = vec![1_000];
        cases.push((
            rounded_two_opportunity_boundary,
            "at most one frame opportunity",
        ));

        let mut implicit_exact_peer_is_fastest = valid.clone();
        implicit_exact_peer_is_fastest.config.frame_model = FrameModel::SkewGated60Hz;
        implicit_exact_peer_is_fastest.config.step_dt_ms = 33;
        implicit_exact_peer_is_fastest.config.clock_skew_ppm = vec![-500_000];
        cases.push((
            implicit_exact_peer_is_fastest,
            "at most one frame opportunity",
        ));

        let mut bad_starved_peer = valid.clone();
        bad_starved_peer.config.starve_events = vec![9];
        cases.push((bad_starved_peer, "starve_events peer 9 out of range"));

        let mut duplicate_starved_peer = valid.clone();
        duplicate_starved_peer.config.starve_events = vec![1, 1];
        cases.push((duplicate_starved_peer, "duplicate starve_events peer 1"));

        let mut bad_spectator_peer = valid.clone();
        bad_spectator_peer.config.spectator_hosts = vec![9];
        cases.push((bad_spectator_peer, "spectator_hosts peer 9 out of range"));

        let mut duplicate_spectator = valid.clone();
        duplicate_spectator.config.spectator_hosts = vec![0, 0];
        cases.push((duplicate_spectator, "duplicate spectator_hosts peer 0"));

        let mut unconfigured_spectator_kill = valid.clone();
        unconfigured_spectator_kill
            .events
            .push((100, ScheduleEvent::SpectatorHostKill { host: 0 }));
        unconfigured_spectator_kill
            .events
            .sort_by_key(|(step, _)| *step);
        cases.push((
            unconfigured_spectator_kill,
            "is not configured in spectator_hosts",
        ));

        let mut already_killed_spectator = valid.clone();
        already_killed_spectator.config.spectator_hosts = vec![0];
        already_killed_spectator
            .events
            .push((90, ScheduleEvent::PeerKill { peer: 0 }));
        already_killed_spectator
            .events
            .push((100, ScheduleEvent::SpectatorHostKill { host: 0 }));
        already_killed_spectator
            .events
            .sort_by_key(|(step, _)| *step);
        cases.push((already_killed_spectator, "already retired"));

        let mut duplicate_rebind = valid.clone();
        duplicate_rebind
            .events
            .retain(|(_, event)| matches!(event, ScheduleEvent::HealAll));
        duplicate_rebind.events.extend([
            (90, ScheduleEvent::Rebind { peer: 1 }),
            (100, ScheduleEvent::Rebind { peer: 1 }),
        ]);
        duplicate_rebind.events.sort_by_key(|(step, _)| *step);
        cases.push((duplicate_rebind, "appears more than once"));

        let mut held_rebind = valid.clone();
        held_rebind
            .events
            .retain(|(_, event)| matches!(event, ScheduleEvent::HealAll));
        held_rebind.events.extend([
            (
                90,
                ScheduleEvent::Hold {
                    from: 1,
                    to: 2,
                    holding: true,
                },
            ),
            (100, ScheduleEvent::Rebind { peer: 1 }),
        ]);
        held_rebind.events.sort_by_key(|(step, _)| *step);
        cases.push((held_rebind, "cannot share a schedule with Hold"));

        let mut killed_rebind = valid.clone();
        killed_rebind
            .events
            .retain(|(_, event)| matches!(event, ScheduleEvent::HealAll));
        killed_rebind.events.extend([
            (90, ScheduleEvent::PeerKill { peer: 1 }),
            (100, ScheduleEvent::Rebind { peer: 1 }),
        ]);
        killed_rebind.events.sort_by_key(|(step, _)| *step);
        cases.push((killed_rebind, "already retired by an earlier kill event"));

        #[cfg(feature = "hot-join")]
        {
            let mut hot_join_rebind = valid.clone();
            hot_join_rebind.config.input_delay = 0;
            hot_join_rebind
                .events
                .retain(|(_, event)| matches!(event, ScheduleEvent::HealAll));
            hot_join_rebind.events.extend([
                (90, ScheduleEvent::HotJoin { slot: 1 }),
                (100, ScheduleEvent::Rebind { peer: 1 }),
            ]);
            hot_join_rebind.events.sort_by_key(|(step, _)| *step);
            cases.push((hot_join_rebind, "Rebind and HotJoin cannot share"));
        }

        let mut small_event_queue = valid.clone();
        small_event_queue.config.event_queue_size = Some(9);
        cases.push((small_event_queue, "event_queue_size must be >= 10"));

        let mut zero_step_duration = valid;
        zero_step_duration.config.step_dt_ms = 0;
        cases.push((zero_step_duration, "step_dt_ms must be within"));

        for (schedule, expected) in cases {
            let error = validate_schedule(&schedule)
                .expect_err("a malformed materialized schedule must be rejected");
            assert!(
                error.contains(expected),
                "expected diagnostic containing {expected:?}, got {error:?}"
            );
        }
    }

    #[test]
    fn validator_rejects_out_of_domain_scalar_bounds() {
        type Mutation = fn(&mut Schedule);
        let cases: [(Mutation, &str); 7] = [
            (
                |schedule| schedule.config.steps = MAX_SIMULATION_STEPS + 1,
                "materialized simulation schedules need 2..=",
            ),
            (
                |schedule| schedule.config.max_prediction = MAX_FRAME_DELAY + 1,
                "max_prediction must be <=",
            ),
            (
                |schedule| schedule.config.step_dt_ms = MAX_SIMULATION_STEP_DT_MS + 1,
                "step_dt_ms must be within",
            ),
            (
                |schedule| {
                    schedule.initial_links[0].2.base_delay =
                        MAX_SIMULATION_LINK_DELAY + Duration::from_millis(1);
                },
                "base_delay must be <=",
            ),
            (
                |schedule| {
                    schedule.initial_links[0].2.jitter =
                        MAX_SIMULATION_LINK_DELAY + Duration::from_millis(1);
                },
                "jitter must be <=",
            ),
            (
                |schedule| {
                    schedule.initial_links[0].2.retransmit_delay =
                        MAX_SIMULATION_LINK_DELAY + Duration::from_millis(1);
                },
                "retransmit_delay must be <=",
            ),
            (
                |schedule| {
                    schedule.events = vec![(0, ScheduleEvent::HealAll); MAX_SIMULATION_EVENTS + 1];
                },
                "maximum 100000",
            ),
        ];

        for (mutate, expected) in cases {
            let mut schedule = generate(7, SimConfig::smoke(3));
            mutate(&mut schedule);
            let error = validate_schedule(&schedule)
                .expect_err("an out-of-domain scalar must be rejected before execution");
            assert!(
                error.contains(expected),
                "expected diagnostic containing {expected:?}, got {error:?}"
            );
        }

        let mut nightly_boundary = generate(9, SimConfig::smoke(3));
        nightly_boundary.config.steps = 5_000;
        nightly_boundary.heal_at = 4_999;
        nightly_boundary.events = vec![(4_999, ScheduleEvent::HealAll)];
        assert_eq!(validate_schedule(&nightly_boundary), Ok(()));
    }

    #[test]
    fn runner_boundary_panic_identifies_schedule_seed_and_schema() {
        let mut schedule = generate(0x5EED, SimConfig::smoke(2));
        schedule.schema_version = SCHEDULE_SCHEMA_VERSION + 1;
        let result = std::panic::catch_unwind(|| {
            let _ = super::super::run(&schedule, &super::super::RunOptions::default());
        });
        let payload = result.expect_err("an invalid schedule must fail at the runner boundary");
        let message = payload.downcast_ref::<String>().map_or_else(
            || {
                payload.downcast_ref::<&'static str>().map_or_else(
                    || "<non-string panic>".to_owned(),
                    |text| (*text).to_owned(),
                )
            },
            Clone::clone,
        );
        assert!(message.contains("seed=24301"), "missing seed: {message}");
        assert!(
            message.contains(&format!("schema_version={}", SCHEDULE_SCHEMA_VERSION + 1)),
            "missing schema: {message}"
        );
    }

    #[cfg(feature = "hot-join")]
    #[test]
    fn validator_enforces_hot_join_preconditions_and_kill_ordering() {
        let mut valid = generate(
            7,
            SimConfig {
                input_delay: 0,
                ..SimConfig::smoke(3)
            },
        );
        valid.events.push((100, ScheduleEvent::HotJoin { slot: 1 }));
        valid.events.sort_by_key(|(step, _)| *step);
        assert_eq!(validate_schedule(&valid), Ok(()));

        let mut cases: Vec<(Schedule, &str)> = Vec::new();

        let mut bad_slot = valid.clone();
        bad_slot
            .events
            .retain(|(_, event)| !matches!(event, ScheduleEvent::HotJoin { .. }));
        bad_slot
            .events
            .push((100, ScheduleEvent::HotJoin { slot: 9 }));
        bad_slot.events.sort_by_key(|(step, _)| *step);
        cases.push((bad_slot, "HotJoin slot 9 out of range"));

        let mut input_delay = valid.clone();
        input_delay.config.input_delay = 1;
        cases.push((input_delay, "must use input_delay 0"));

        let mut zero_prediction = valid.clone();
        zero_prediction.config.max_prediction = 0;
        cases.push((zero_prediction, "max_prediction >= 1"));

        let mut sparse = valid.clone();
        sparse.config.save_mode = SavePolicy::Sparse;
        cases.push((sparse, "save_mode EveryFrame"));

        let mut repeated = valid.clone();
        repeated
            .events
            .push((120, ScheduleEvent::HotJoin { slot: 2 }));
        repeated.events.sort_by_key(|(step, _)| *step);
        let mut legacy_repeated = repeated.clone();
        legacy_repeated.schema_version = 11;
        assert_eq!(validate_schedule(&legacy_repeated), Ok(()));
        cases.push((repeated, "schema v12 HotJoin appears more than once"));

        let mut killed_slot = valid.clone();
        killed_slot
            .events
            .push((90, ScheduleEvent::PeerKill { peer: 1 }));
        killed_slot.events.sort_by_key(|(step, _)| *step);
        cases.push((killed_slot, "slot 1 is already retired"));

        let mut killed_coordinator = valid;
        killed_coordinator
            .events
            .push((90, ScheduleEvent::PeerKill { peer: 0 }));
        killed_coordinator.events.sort_by_key(|(step, _)| *step);
        cases.push((killed_coordinator, "coordinator peer 0 is already retired"));

        for (schedule, expected) in cases {
            let error = validate_schedule(&schedule)
                .expect_err("a malformed HotJoin schedule must be rejected");
            assert!(
                error.contains(expected),
                "expected diagnostic containing {expected:?}, got {error:?}"
            );
        }
    }

    #[cfg(not(feature = "hot-join"))]
    #[test]
    fn validator_rejects_hot_join_without_feature() {
        let mut schedule = generate(7, SimConfig::smoke(2));
        schedule
            .events
            .push((100, ScheduleEvent::HotJoin { slot: 1 }));
        schedule.events.sort_by_key(|(step, _)| *step);
        assert_eq!(
            validate_schedule(&schedule),
            Err("ScheduleEvent::HotJoin requires the crate's `hot-join` feature".to_owned())
        );
    }

    #[test]
    fn link_policy_without_retransmit_delay_uses_zero_default() {
        let schedule = generate(42, SimConfig::smoke(2));
        let mut value = serde_json::to_value(&schedule).unwrap();
        let first_policy = value
            .get_mut("initial_links")
            .and_then(|links| links.as_array_mut())
            .and_then(|links| links.first_mut())
            .and_then(|link| link.as_array_mut())
            .and_then(|link| link.get_mut(2))
            .and_then(|policy| policy.as_object_mut())
            .expect("initial link policy must serialize as an object");
        assert!(
            first_policy.remove("retransmit_delay").is_some(),
            "policy must serialize retransmit_delay for this test to remove"
        );

        let back: Schedule = serde_json::from_value(value).unwrap();
        assert_eq!(
            back.initial_links[0].2.retransmit_delay,
            Duration::ZERO,
            "old corpus artifacts without retransmit_delay must keep UDP-like loss semantics"
        );
        assert_eq!(back.initial_links[0].2.gilbert_elliott, None);
        assert_eq!(back.initial_links[0].2.fragmentation, None);
    }

    #[test]
    fn gilbert_elliott_policy_round_trips_and_none_is_omitted() {
        let mut schedule = generate(42, SimConfig::smoke(2));
        let clean_json = serde_json::to_value(&schedule).unwrap();
        let clean_policy = clean_json
            .get("initial_links")
            .and_then(serde_json::Value::as_array)
            .and_then(|links| links.first())
            .and_then(serde_json::Value::as_array)
            .and_then(|link| link.get(2))
            .and_then(serde_json::Value::as_object)
            .expect("initial link policy must serialize as an object");
        assert!(!clean_policy.contains_key("gilbert_elliott"));
        assert!(!clean_policy.contains_key("fragmentation"));

        schedule.initial_links[0].2 = LinkPolicy {
            gilbert_elliott: Some(valid_gilbert_elliott()),
            ..LinkPolicy::clean()
        };
        let json = serde_json::to_vec(&schedule).unwrap();
        let back: Schedule = serde_json::from_slice(&json).unwrap();
        assert_eq!(back, schedule);
        assert_eq!(validate_schedule(&back), Ok(()));
    }

    #[test]
    fn fragmentation_policy_round_trips_and_none_is_omitted() {
        let mut schedule = generate(42, SimConfig::smoke(2));
        let clean_json = serde_json::to_value(&schedule).unwrap();
        let clean_policy = clean_json
            .get("initial_links")
            .and_then(serde_json::Value::as_array)
            .and_then(|links| links.first())
            .and_then(serde_json::Value::as_array)
            .and_then(|link| link.get(2))
            .and_then(serde_json::Value::as_object)
            .expect("initial link policy must serialize as an object");
        assert!(!clean_policy.contains_key("fragmentation"));

        schedule.initial_links[0].2.fragmentation =
            Some(crate::common::sim_net::FragmentationPolicy {
                fragment_drop_rate: 0.2,
            });
        let json = serde_json::to_vec(&schedule).unwrap();
        let back: Schedule = serde_json::from_slice(&json).unwrap();
        assert_eq!(back, schedule);
        assert_eq!(validate_schedule(&back), Ok(()));
    }

    /// The lifecycle-vocabulary events must serialize losslessly — a corpus
    /// artifact carrying them has to replay the same faults. The generator does
    /// not yet emit them, so plant them explicitly.
    #[test]
    fn lifecycle_events_round_trip_through_json() {
        let mut schedule = generate(42, SimConfig::smoke(3));
        schedule.config.spectator_hosts = vec![0, 1];
        schedule
            .events
            .push((200, ScheduleEvent::PeerStall { peer: 1, steps: 40 }));
        schedule
            .events
            .push((250, ScheduleEvent::SetInputDelay { peer: 2, delay: 3 }));
        schedule
            .events
            .push((275, ScheduleEvent::GracefulRemove { by: 0, target: 1 }));
        schedule
            .events
            .push((285, ScheduleEvent::LegacyDisconnect { by: 0, target: 2 }));
        schedule
            .events
            .push((300, ScheduleEvent::PeerKill { peer: 2 }));
        schedule
            .events
            .push((325, ScheduleEvent::SpectatorHostKill { host: 0 }));
        schedule
            .events
            .push((340, ScheduleEvent::Rebind { peer: 1 }));
        schedule
            .events
            .push((350, ScheduleEvent::HotJoin { slot: 1 }));
        schedule.events.sort_by_key(|(step, _)| *step);
        let json = serde_json::to_string(&schedule).unwrap();
        let back: Schedule = serde_json::from_str(&json).unwrap();
        assert_eq!(
            schedule, back,
            "lifecycle events must round-trip losslessly"
        );
        assert!(back
            .events
            .iter()
            .any(|(_, ev)| matches!(ev, ScheduleEvent::PeerStall { peer: 1, steps: 40 })));
        assert!(back
            .events
            .iter()
            .any(|(_, ev)| matches!(ev, ScheduleEvent::Rebind { peer: 1 })));
        assert!(back
            .events
            .iter()
            .any(|(_, ev)| matches!(ev, ScheduleEvent::SetInputDelay { peer: 2, delay: 3 })));
        assert!(back
            .events
            .iter()
            .any(|(_, ev)| matches!(ev, ScheduleEvent::GracefulRemove { by: 0, target: 1 })));
        assert!(back
            .events
            .iter()
            .any(|(_, ev)| matches!(ev, ScheduleEvent::LegacyDisconnect { by: 0, target: 2 })));
        assert!(back
            .events
            .iter()
            .any(|(_, ev)| matches!(ev, ScheduleEvent::PeerKill { peer: 2 })));
        assert!(back
            .events
            .iter()
            .any(|(_, ev)| matches!(ev, ScheduleEvent::SpectatorHostKill { host: 0 })));
        assert!(back
            .events
            .iter()
            .any(|(_, ev)| matches!(ev, ScheduleEvent::HotJoin { slot: 1 })));
    }

    /// A corpus artifact predating the `#[serde(default)]` config axes (its
    /// `config` object lacks those fields) must deserialize to their defaults —
    /// including `save_mode = EveryFrame`, `app_model = Ignore` (open-loop),
    /// `frame_model = Lockstep`, and `clock_skew_ppm = []` (no skew) — so old
    /// schedules replay bit-identically. Each field is asserted
    /// present-and-removed so the test can't pass vacuously (a full config also
    /// deserializes fine).
    #[test]
    fn config_without_serde_default_fields_uses_defaults() {
        let schedule = generate(42, SimConfig::smoke(2));
        let mut value = serde_json::to_value(&schedule).unwrap();
        let config = value
            .get_mut("config")
            .and_then(|c| c.as_object_mut())
            .expect("a serialized schedule must have a `config` object");
        assert!(
            config.remove("save_mode").is_some(),
            "config must serialize a `save_mode` field for this test to remove"
        );
        assert!(
            config.remove("scenario_mix").is_some(),
            "config must serialize a `scenario_mix` field for this test to remove"
        );
        assert!(
            config.remove("app_model").is_some(),
            "config must serialize an `app_model` field for this test to remove"
        );
        assert!(
            !config.contains_key("wait_recommendation_policy"),
            "the default wait policy must stay omitted for legacy byte stability"
        );
        assert!(
            !config.contains_key("cpu_feedback_policy"),
            "disabled CPU feedback must stay omitted for legacy byte stability"
        );
        assert!(
            config.remove("frame_model").is_some(),
            "config must serialize a `frame_model` field for this test to remove"
        );
        assert!(
            config.remove("clock_skew_ppm").is_some(),
            "config must serialize a `clock_skew_ppm` field for this test to remove"
        );
        assert!(
            config.remove("starve_events").is_some(),
            "config must serialize a `starve_events` field for this test to remove"
        );
        assert!(
            config.remove("event_queue_size").is_some(),
            "config must serialize an `event_queue_size` field for this test to remove"
        );
        assert!(
            config.remove("spectator_hosts").is_some(),
            "config must serialize a `spectator_hosts` field for this test to remove"
        );
        let back: Schedule = serde_json::from_value(value).unwrap();
        assert_eq!(
            back.config.save_mode,
            SavePolicy::EveryFrame,
            "a pre-axis config (no save_mode) must default to EveryFrame"
        );
        assert_eq!(
            back.config.scenario_mix,
            ScenarioMix::NetworkOnly,
            "a pre-axis config (no scenario_mix) must default to NetworkOnly"
        );
        assert_eq!(
            back.config.app_model,
            AppModel::Ignore,
            "a pre-axis config (no app_model) must default to Ignore"
        );
        assert_eq!(
            back.config.wait_recommendation_policy,
            WaitRecommendationPolicy::default(),
            "a pre-axis config must default to immediate uncapped obedience"
        );
        assert_eq!(
            back.config.cpu_feedback_policy, None,
            "a pre-axis config must retain fixed poll cadence"
        );
        assert_eq!(
            back.config.frame_model,
            FrameModel::Lockstep,
            "a pre-axis config (no frame_model) must default to Lockstep"
        );
        assert!(
            back.config.clock_skew_ppm.is_empty(),
            "a pre-axis config (no clock_skew_ppm) must default to no skew"
        );
        assert!(
            back.config.starve_events.is_empty(),
            "a pre-axis config (no starve_events) must default to every peer draining"
        );
        assert_eq!(
            back.config.event_queue_size, None,
            "a pre-axis config (no event_queue_size) must default to the library cap"
        );
        assert!(
            back.config.spectator_hosts.is_empty(),
            "a pre-axis config (no spectator_hosts) must default to no spectator"
        );
    }

    #[test]
    fn non_default_wait_recommendation_policy_round_trips_as_schema_v17() {
        let mut schedule = generate(42, SimConfig::smoke(2));
        schedule.config.app_model = AppModel::Obey;
        schedule.config.wait_recommendation_policy = WaitRecommendationPolicy {
            cooldown_frames: 240,
            max_skip_frames: Some(9),
            response_delay_frames: 30,
            smear_interval: 4,
        };
        assert_eq!(validate_schedule(&schedule), Ok(()));
        let value = serde_json::to_value(&schedule).expect("schedule serializes");
        assert!(value["config"].get("wait_recommendation_policy").is_some());
        let decoded: Schedule = serde_json::from_value(value).expect("schedule deserializes");
        assert_eq!(decoded, schedule);
    }

    #[test]
    fn cpu_feedback_policy_round_trips_as_schema_v18() {
        let mut schedule = generate(42, SimConfig::smoke(2));
        schedule.config.cpu_feedback_policy = Some(CpuFeedbackPolicy {
            simulated_frame_cost_us: 8_000,
            max_poll_delay_steps: 16,
        });
        assert_eq!(validate_schedule(&schedule), Ok(()));
        let value = serde_json::to_value(&schedule).expect("schedule serializes");
        assert!(value["config"].get("cpu_feedback_policy").is_some());
        let decoded: Schedule = serde_json::from_value(value).expect("schedule deserializes");
        assert_eq!(decoded, schedule);

        let mut under_declared = schedule;
        under_declared.schema_version = 17;
        assert!(validate_schedule(&under_declared)
            .expect_err("schema 17 cannot express CPU feedback")
            .contains("under-declares capability requiring schema_version 18"));
    }
}
