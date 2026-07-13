//! Long-running deterministic boundedness soak.

#![allow(clippy::expect_used, clippy::panic)]

use crate::common::sim_net::{LinkPolicy, SimNet};
use crate::common::stubs::{GameStub, StubConfig, StubInput};
use crate::common::test_clock::TestClock;
use fortress_rollback::telemetry::{CollectingObserver, ViolationSeverity};
use fortress_rollback::{
    __internal::p2p_container_lengths, DesyncDetection, DisconnectBehavior, FortressError, Message,
    P2PSession, PeerMetrics, PlayerHandle, PlayerType, ProtocolConfig, SessionBuilder,
    SessionState, SpectatorSession,
};
use std::collections::BTreeSet;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

const TARGET_CONFIRMED_FRAMES: i32 = 4_000_000;
const CHECKPOINT_FRAMES: i32 = 50_000;
const WARMUP_FRAMES: i32 = 1_000_000;
const VIRTUAL_HOUR_FRAMES: i32 = 216_000;
const HOT_JOIN_INTERVAL_FRAMES: i32 = 100_000;
const POLL_INTERVAL: Duration = Duration::from_millis(16);
const MAX_STEPS_PER_TARGET_FRAME: i64 = 8;
const MAX_CHECKSUM_HISTORY: usize = 32;
const PENDING_OUTPUT_LIMIT: u64 = 128;
const RSS_HOURLY_GROWTH_PERCENT: u64 = 5;
const RSS_TOTAL_GROWTH_PERCENT: u64 = 10;

fn target_confirmed_frames() -> i32 {
    std::env::var("FORTRESS_SOAK_TARGET_FRAMES")
        .ok()
        .and_then(|value| value.parse::<i32>().ok())
        .filter(|target| *target >= CHECKPOINT_FRAMES)
        .unwrap_or(TARGET_CONFIRMED_FRAMES)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ContainerHighWater {
    event_queue: usize,
    checksum_history: usize,
    pending_checksums: u64,
    pending_output: u64,
}

impl ContainerHighWater {
    fn observe_session(&mut self, session: &P2PSession<StubConfig>, n_players: usize) {
        let (event_len, event_limit, checksum_len) = p2p_container_lengths(session);
        assert!(
            event_len <= event_limit,
            "event queue exceeded its configured bound: {event_len} > {event_limit}"
        );
        assert!(
            checksum_len <= MAX_CHECKSUM_HISTORY,
            "checksum history exceeded its configured bound: {checksum_len} > {MAX_CHECKSUM_HISTORY}"
        );
        let session_metrics = session.metrics();
        let event_high =
            usize::try_from(session_metrics.event_queue_high_water).unwrap_or(usize::MAX);
        let checksum_high =
            usize::try_from(session_metrics.checksum_history_high_water).unwrap_or(usize::MAX);
        assert!(
            event_high <= event_limit,
            "event-queue high-water mark exceeded its configured bound: {event_high} > {event_limit}"
        );
        assert!(
            checksum_high <= MAX_CHECKSUM_HISTORY,
            "checksum-history high-water mark exceeded its configured bound: {checksum_high} > {MAX_CHECKSUM_HISTORY}"
        );
        self.event_queue = self.event_queue.max(event_high);
        self.checksum_history = self.checksum_history.max(checksum_high);

        for peer in 0..n_players {
            let handle = PlayerHandle::new(peer);
            let Ok(metrics) = session.peer_metrics(handle) else {
                continue;
            };
            self.observe_peer(&metrics);
        }
    }

    fn observe_peer(&mut self, metrics: &PeerMetrics) {
        self.observe_pending(metrics.pending_output_len, metrics.pending_checksums_len);
    }

    fn observe_pending(&mut self, pending_output_len: u64, pending_checksums_len: u64) {
        assert!(
            pending_output_len <= PENDING_OUTPUT_LIMIT,
            "pending output exceeded its configured bound: {} > {PENDING_OUTPUT_LIMIT}",
            pending_output_len
        );
        assert!(
            pending_checksums_len <= u64::try_from(MAX_CHECKSUM_HISTORY).unwrap_or(u64::MAX),
            "pending checksum history exceeded its configured bound: {} > {MAX_CHECKSUM_HISTORY}",
            pending_checksums_len
        );
        self.pending_output = self.pending_output.max(pending_output_len);
        self.pending_checksums = self.pending_checksums.max(pending_checksums_len);
    }
}

struct PeerSlot {
    session: P2PSession<StubConfig>,
    game: GameStub,
    observer: Arc<CollectingObserver>,
}

struct SoakRun {
    clock: TestClock,
    net: SimNet<Message>,
    addrs: Vec<SocketAddr>,
    spectator_addr: SocketAddr,
    peers: Vec<PeerSlot>,
    spectator: SpectatorSession<StubConfig>,
    spectator_observer: Arc<CollectingObserver>,
    next_checkpoint: i32,
    next_hot_join: i32,
    replay_checked: bool,
    hot_joins_completed: u32,
    periodic_hot_join: bool,
    pending_hot_join: Option<(usize, usize, u64)>,
    drop_commit_observers: BTreeSet<usize>,
    high_water: ContainerHighWater,
    last_high_water_change_frame: i32,
    rss_baseline: Option<(i32, u64)>,
    rss_last_hour: Option<(i32, u64)>,
}

fn addr(index: usize) -> SocketAddr {
    let host = u8::try_from(index.saturating_add(1)).unwrap_or(u8::MAX);
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 1, host)), 31_000)
}

fn protocol_config(clock: &TestClock, seed: u64) -> ProtocolConfig {
    ProtocolConfig {
        clock: Some(clock.as_protocol_clock()),
        protocol_rng_seed: Some(seed),
        max_checksum_history: MAX_CHECKSUM_HISTORY,
        pending_output_limit: usize::try_from(PENDING_OUTPUT_LIMIT).unwrap_or(128),
        ..ProtocolConfig::default()
    }
}

fn mild_policy() -> LinkPolicy {
    LinkPolicy {
        drop_rate: 0.01,
        dup_rate: 0.002,
        base_delay: Duration::from_millis(4),
        jitter: Duration::from_millis(4),
        burst_rate: 0.000_2,
        burst_len: 2,
        ..LinkPolicy::clean()
    }
}

impl SoakRun {
    fn new(n_players: usize, seed: u64, periodic_hot_join: bool) -> Result<Self, FortressError> {
        let clock = TestClock::new();
        let net = SimNet::new(seed, clock.as_protocol_clock());
        net.set_default_policy(mild_policy());
        let addrs: Vec<_> = (0..n_players).map(addr).collect();
        let spectator_addr = addr(n_players);

        let mut peers = Vec::with_capacity(n_players);
        for local in 0..n_players {
            let observer = Arc::new(CollectingObserver::new());
            let mut builder = SessionBuilder::<StubConfig>::new()
                .with_num_players(n_players)?
                .with_protocol_config(protocol_config(
                    &clock,
                    seed.saturating_add(u64::try_from(local).unwrap_or(u64::MAX)),
                ))
                .with_desync_detection_mode(DesyncDetection::On { interval: 60 })
                .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
                .with_hot_join(periodic_hot_join && local == n_players.saturating_sub(1))
                .with_recording(local == 0)
                .with_violation_observer(Arc::clone(&observer) as Arc<_>);
            for (peer, &peer_addr) in addrs.iter().enumerate() {
                let player_type = if peer == local {
                    PlayerType::Local
                } else {
                    PlayerType::Remote(peer_addr)
                };
                builder = builder.add_player(player_type, PlayerHandle::new(peer))?;
            }
            builder = builder.add_player(
                PlayerType::Spectator(spectator_addr),
                PlayerHandle::new(n_players),
            )?;
            peers.push(PeerSlot {
                session: builder.start_p2p_session(net.attach(addrs[local]))?,
                game: GameStub::new(),
                observer,
            });
        }

        let spectator_observer = Arc::new(CollectingObserver::new());
        let spectator = SessionBuilder::<StubConfig>::new()
            .with_num_players(n_players)?
            .with_protocol_config(protocol_config(&clock, seed ^ 0x5350_4543_5441_544f))
            .with_violation_observer(Arc::clone(&spectator_observer) as Arc<_>)
            .start_spectator_session_multi(&addrs, net.attach(spectator_addr))
            .expect("validated spectator host set starts");

        Ok(Self {
            clock,
            net,
            addrs,
            spectator_addr,
            peers,
            spectator,
            spectator_observer,
            next_checkpoint: CHECKPOINT_FRAMES,
            next_hot_join: HOT_JOIN_INTERVAL_FRAMES,
            replay_checked: false,
            hot_joins_completed: 0,
            periodic_hot_join,
            pending_hot_join: None,
            drop_commit_observers: BTreeSet::new(),
            high_water: ContainerHighWater::default(),
            last_high_water_change_frame: 0,
            rss_baseline: None,
            rss_last_hour: None,
        })
    }

    fn min_confirmed(&self) -> i32 {
        self.peers
            .iter()
            .map(|peer| peer.session.confirmed_frame().as_i32())
            .min()
            .unwrap_or(-1)
    }

    fn poll_and_advance(&mut self, step: u64) -> Result<(), FortressError> {
        for (local, peer) in self.peers.iter_mut().enumerate() {
            peer.session.poll_remote_clients();
            for event in peer.session.events() {
                if self.pending_hot_join.is_some_and(|(slot, _host, _)| {
                    matches!(event, fortress_rollback::FortressEvent::PeerDropped { handle, .. } if handle == PlayerHandle::new(slot))
                }) {
                    self.drop_commit_observers.insert(local);
                }
            }
        }
        self.spectator.poll_remote_clients();
        let _: Vec<_> = self.spectator.events().collect();

        for (local, peer) in self.peers.iter_mut().enumerate() {
            if peer.session.current_state() != SessionState::Running {
                continue;
            }
            let local_word = u32::try_from(local).unwrap_or(u32::MAX);
            peer.session.add_local_input(
                PlayerHandle::new(local),
                StubInput {
                    inp: u32::try_from(step)
                        .unwrap_or(u32::MAX)
                        .wrapping_mul(31)
                        .wrapping_add(local_word),
                },
            )?;
            match peer.session.advance_frame() {
                Ok(requests) => peer.game.handle_requests(requests),
                Err(FortressError::PredictionThreshold | FortressError::NotSynchronized) => {},
                Err(error) => return Err(error),
            }
        }
        if self.spectator.current_state() == SessionState::Running {
            match self.spectator.advance_frame() {
                Ok(_)
                | Err(FortressError::PredictionThreshold | FortressError::NotSynchronized) => {},
                Err(error) => return Err(error),
            }
        }
        self.clock.advance(POLL_INTERVAL);
        if self.pending_hot_join.is_some_and(|(slot, _, _)| {
            self.drop_commit_observers.len() == self.addrs.len().saturating_sub(1)
                && !self.drop_commit_observers.contains(&slot)
        }) {
            self.start_replacement()?;
        }
        Ok(())
    }

    fn checkpoint(&mut self, confirmed: i32) -> Result<(), FortressError> {
        let previous_high_water = self.high_water;
        for peer in &self.peers {
            self.high_water
                .observe_session(&peer.session, self.addrs.len());
            assert!(
                peer.observer
                    .violations_at_severity(ViolationSeverity::Error)
                    .is_empty(),
                "peer emitted Error+ violations at frame {confirmed}: {:?}",
                peer.observer.violations()
            );
            // The observer is diagnostic instrumentation, not production
            // session state. Keep it from becoming the only unbounded object
            // in the boundedness test when warning-level events are expected.
            peer.observer.clear();
        }
        assert!(
            self.spectator_observer
                .violations_at_severity(ViolationSeverity::Error)
                .is_empty(),
            "spectator emitted Error+ violations at frame {confirmed}: {:?}",
            self.spectator_observer.violations()
        );
        self.spectator_observer.clear();

        if !self.replay_checked {
            let replay = self.peers[0].session.take_replay()?;
            replay.validate()?;
            assert!(
                !replay.frames.is_empty(),
                "soak replay must contain confirmed frames"
            );
            self.replay_checked = true;
        }

        if self.high_water != previous_high_water {
            self.last_high_water_change_frame = confirmed;
        }
        self.check_rss(confirmed);
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn check_rss(&mut self, confirmed: i32) {
        if confirmed < WARMUP_FRAMES {
            return;
        }
        let Some(rss) = read_rss_bytes() else {
            return;
        };
        if self.rss_baseline.is_none() {
            self.rss_baseline = Some((confirmed, rss));
            self.rss_last_hour = Some((confirmed, rss));
            return;
        }
        let Some((_, previous_rss)) = self.rss_last_hour else {
            self.rss_last_hour = Some((confirmed, rss));
            return;
        };
        let Some((previous_frame, _)) = self.rss_last_hour else {
            return;
        };
        if confirmed.saturating_sub(previous_frame) < VIRTUAL_HOUR_FRAMES {
            return;
        }
        assert!(
            rss_growth_within_limit(previous_rss, rss, RSS_HOURLY_GROWTH_PERCENT),
            "RSS grew by at least 5% in one post-warmup virtual hour: {previous_rss} -> {rss} bytes"
        );
        let Some((_, baseline_rss)) = self.rss_baseline else {
            return;
        };
        assert!(
            rss_growth_within_limit(baseline_rss, rss, RSS_TOTAL_GROWTH_PERCENT),
            "RSS grew by at least 10% from the first post-warmup sample: {baseline_rss} -> {rss} bytes"
        );
        self.rss_last_hour = Some((confirmed, rss));
    }

    #[cfg(not(target_os = "linux"))]
    fn check_rss(&mut self, _confirmed: i32) {}

    fn maybe_hot_join(&mut self, confirmed: i32, seed: u64) -> Result<(), FortressError> {
        if !self.periodic_hot_join
            || confirmed < self.next_hot_join
            || self.pending_hot_join.is_some()
        {
            return Ok(());
        }
        let n_players = self.addrs.len();
        // Drive order makes slot 0 the stable lagging edge and the last slot
        // the stable leading edge. Keeping one explicit coordinator avoids
        // teaching every survivor to claim the mutually exclusive serve role.
        let slot = 0;
        let host = n_players.saturating_sub(1);

        self.peers[host]
            .session
            .remove_player(PlayerHandle::new(slot))?;
        self.pending_hot_join = Some((slot, host, seed));
        self.drop_commit_observers.clear();
        self.next_hot_join = self.next_hot_join.saturating_add(HOT_JOIN_INTERVAL_FRAMES);
        Ok(())
    }

    fn start_replacement(&mut self) -> Result<(), FortressError> {
        let Some((slot, host, seed)) = self.pending_hot_join else {
            return Ok(());
        };
        let n_players = self.addrs.len();
        self.net.detach(self.addrs[slot]);
        let observer = Arc::new(CollectingObserver::new());
        let mut builder = SessionBuilder::<StubConfig>::new()
            .with_num_players(n_players)?
            .with_protocol_config(protocol_config(
                &self.clock,
                seed.saturating_add(u64::from(self.hot_joins_completed))
                    .saturating_add(1),
            ))
            .with_desync_detection_mode(DesyncDetection::On { interval: 60 })
            .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
            .with_violation_observer(Arc::clone(&observer) as Arc<_>);
        for (peer, &peer_addr) in self.addrs.iter().enumerate() {
            let player_type = if peer == slot {
                PlayerType::Local
            } else {
                PlayerType::Remote(peer_addr)
            };
            builder = builder.add_player(player_type, PlayerHandle::new(peer))?;
        }
        builder = builder.add_player(
            PlayerType::Spectator(self.spectator_addr),
            PlayerHandle::new(n_players),
        )?;

        let session =
            builder.start_hot_join_session(self.net.attach(self.addrs[slot]), self.addrs[host])?;
        self.peers[slot] = PeerSlot {
            session,
            game: GameStub::new(),
            observer,
        };
        self.hot_joins_completed = self.hot_joins_completed.saturating_add(1);
        self.pending_hot_join = None;
        self.drop_commit_observers.clear();
        Ok(())
    }

    fn run_to_target(mut self, seed: u64) -> Result<(), FortressError> {
        let target = target_confirmed_frames();
        let step_limit = i64::from(target).saturating_mul(MAX_STEPS_PER_TARGET_FRAME);
        for step in 0..u64::try_from(step_limit).unwrap_or(u64::MAX) {
            self.poll_and_advance(step)?;
            let confirmed = self.min_confirmed();
            self.maybe_hot_join(confirmed, seed)?;
            while confirmed >= self.next_checkpoint {
                self.checkpoint(confirmed)?;
                self.next_checkpoint = self.next_checkpoint.saturating_add(CHECKPOINT_FRAMES);
            }
            if confirmed >= target && self.pending_hot_join.is_none() {
                assert!(self.replay_checked, "the replay checkpoint must run");
                if target >= WARMUP_FRAMES.saturating_add(VIRTUAL_HOUR_FRAMES) {
                    assert!(
                        confirmed.saturating_sub(self.last_high_water_change_frame)
                            >= VIRTUAL_HOUR_FRAMES,
                        "bounded-container high-water marks did not plateau for a final virtual hour: last change at {}, final frame {confirmed}, high-water {:?}",
                        self.last_high_water_change_frame,
                        self.high_water
                    );
                }
                if self.periodic_hot_join {
                    let expected = u32::try_from(target / HOT_JOIN_INTERVAL_FRAMES).unwrap_or(0);
                    assert!(
                        self.hot_joins_completed >= expected,
                        "expected at least {expected} periodic hot-joins through the soak, got {}",
                        self.hot_joins_completed
                    );
                }
                assert!(
                    self.spectator.current_frame().as_i32() > 0,
                    "the attached spectator must make progress"
                );
                return Ok(());
            }
        }
        let states: Vec<_> = self
            .peers
            .iter()
            .map(|peer| {
                (
                    peer.session.current_state(),
                    peer.session.current_frame().as_i32(),
                    peer.session.confirmed_frame().as_i32(),
                    peer.session.diagnostic_connect_status(),
                    peer.observer
                        .violations_at_severity(ViolationSeverity::Error),
                )
            })
            .collect();
        panic!(
            "soak failed to reach {target} confirmed frames; stopped at {}; peers={states:#?}",
            self.min_confirmed(),
        );
    }
}

#[cfg(target_os = "linux")]
fn read_rss_bytes() -> Option<u64> {
    let statm = std::fs::read_to_string("/proc/self/statm").ok()?;
    let resident_pages = statm.split_whitespace().nth(1)?.parse::<u64>().ok()?;
    Some(resident_pages.saturating_mul(4096))
}

fn rss_growth_within_limit(previous: u64, current: u64, growth_percent: u64) -> bool {
    u128::from(current).saturating_mul(100)
        < u128::from(previous).saturating_mul(u128::from(100_u64.saturating_add(growth_percent)))
}

#[test]
#[ignore = "4,000,000-frame deterministic release-mode boundedness soak; nightly CI only"]
fn four_million_frame_soak_preserves_bounds_replay_and_lifecycle() -> Result<(), FortressError> {
    SoakRun::new(2, 0x50A4_0002, true)?.run_to_target(0x50A4_0002)?;
    SoakRun::new(4, 0x50A4_0004, false)?.run_to_target(0x50A4_0004)
}

#[test]
fn high_water_tracks_each_bounded_container() {
    let mut high = ContainerHighWater::default();
    high.observe_pending(7, 3);
    assert_eq!(high.pending_output, 7);
    assert_eq!(high.pending_checksums, 3);
}

#[test]
fn rss_growth_gate_rejects_large_cumulative_growth() {
    assert!(rss_growth_within_limit(100, 104, RSS_HOURLY_GROWTH_PERCENT));
    assert!(!rss_growth_within_limit(100, 110, RSS_TOTAL_GROWTH_PERCENT));
}
