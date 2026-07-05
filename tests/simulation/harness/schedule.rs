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

use crate::common::sim_net::LinkPolicy;
use fortress_rollback::rng::{Pcg32, SeedableRng};
use fortress_rollback::DisconnectBehavior;
use serde::{Deserialize, Serialize};
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
/// - `4`: adds [`ScheduleEvent::PeerKill`] (a peer crash — session dropped and
///   detached from the fabric — modeled distinctly from a network black-hole).
pub const SCHEDULE_SCHEMA_VERSION: u32 = 4;

/// Background link-noise level applied to every directed link at start.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackgroundNoise {
    /// Perfect links.
    Clean,
    /// LAN-to-good-WAN: ≤2% loss, 5–30ms delay, ≤10ms jitter, ≤1% dup.
    Mild,
    /// Bad WAN / mobile: 2–10% loss, 20–80ms delay, ≤30ms jitter, ≤3% dup,
    /// occasional short loss bursts.
    Rough,
}

/// Static configuration of one simulation run — the fleet's axes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    /// Peer-drop policy for every session in the mesh. `#[serde(default)]`
    /// (= `Halt`, the library default) keeps pre-existing corpus artifacts
    /// replayable without a schema bump.
    #[serde(default)]
    pub disconnect_behavior: DropPolicy,
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
            disconnect_behavior: DropPolicy::default(),
        }
    }

    /// Virtual duration of one step.
    #[must_use]
    pub fn step_dt(&self) -> Duration {
        Duration::from_millis(self.step_dt_ms)
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
    /// silence tips the peer into a genuine disconnect. The planted lifecycle
    /// tests keep it well under that bound so the hitch is a recoverable pause,
    /// not a drop; the random generator does not emit this event yet (it lands
    /// with the M3 storyline overhaul, which re-blesses seed baselines).
    PeerStall { peer: usize, steps: u32 },
    /// Set peer `peer`'s local input delay to `delay` frames mid-run
    /// (`P2PSession::set_input_delay`). A mid-session *increase* is the
    /// interesting case: it gap-fills the newly delayed frames with replicated
    /// confirmed inputs and flushes them to every remote — a reconfiguration
    /// path a fixed-delay fleet never exercises. Values are per-peer local, so
    /// the mesh must still agree on every confirmed frame across the change.
    ///
    /// Like `PeerStall`, this is planted by lifecycle tests, not yet emitted by
    /// the random generator (that lands with the §6.1 storyline overhaul).
    SetInputDelay { peer: usize, delay: usize },
    /// Permanently kill peer `peer`: the harness stops driving its session and
    /// detaches it from the fabric (its inbox is discarded; further traffic to
    /// it is dropped). Models a **crash** — the peer is gone for good and, being
    /// no longer observable, is excluded from the oracle's end-of-run checks
    /// (its remaining mesh survives per the configured `DisconnectBehavior`).
    /// Distinct from a network black-hole (`Block`), where the peer keeps
    /// running and observing. `HealAll` does not revive it.
    ///
    /// Planted by lifecycle tests, not yet emitted by the random generator.
    PeerKill { peer: usize },
    /// Reset every link to clean and release all held traffic.
    HealAll,
}

/// A fully materialized simulation plan. Serializable (corpus format).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
        lo + self.unit() * (hi - lo)
    }
}

/// Rolls one background-noise link policy.
fn roll_background_policy(draw: &mut Draw<'_>, noise: BackgroundNoise) -> LinkPolicy {
    match noise {
        BackgroundNoise::Clean => LinkPolicy::clean(),
        BackgroundNoise::Mild => LinkPolicy {
            drop_rate: draw.range_f64(0.0, 0.02),
            dup_rate: draw.range_f64(0.0, 0.01),
            base_delay: Duration::from_millis(draw.range_u64(5, 30)),
            jitter: Duration::from_millis(draw.range_u64(0, 10)),
            burst_rate: 0.0,
            burst_len: 0,
        },
        BackgroundNoise::Rough => LinkPolicy {
            drop_rate: draw.range_f64(0.02, 0.10),
            dup_rate: draw.range_f64(0.0, 0.03),
            base_delay: Duration::from_millis(draw.range_u64(20, 80)),
            jitter: Duration::from_millis(draw.range_u64(0, 30)),
            burst_rate: draw.range_f64(0.0, 0.005),
            burst_len: u32::try_from(draw.range_u64(2, 5)).unwrap_or(3),
        },
    }
}

/// Generates the fully materialized schedule for `(seed, config)`.
///
/// Pure and deterministic: same inputs, same schedule, always.
#[must_use]
pub fn generate(seed: u64, config: SimConfig) -> Schedule {
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
    let n_storylines = if storyline_budget > 100 {
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

    /// The lifecycle-vocabulary events must serialize losslessly — a corpus
    /// artifact carrying them has to replay the same faults. The generator does
    /// not yet emit them, so plant them explicitly.
    #[test]
    fn lifecycle_events_round_trip_through_json() {
        let mut schedule = generate(42, SimConfig::smoke(3));
        schedule
            .events
            .push((200, ScheduleEvent::PeerStall { peer: 1, steps: 40 }));
        schedule
            .events
            .push((250, ScheduleEvent::SetInputDelay { peer: 2, delay: 3 }));
        schedule
            .events
            .push((300, ScheduleEvent::PeerKill { peer: 2 }));
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
            .any(|(_, ev)| matches!(ev, ScheduleEvent::SetInputDelay { peer: 2, delay: 3 })));
        assert!(back
            .events
            .iter()
            .any(|(_, ev)| matches!(ev, ScheduleEvent::PeerKill { peer: 2 })));
    }
}
