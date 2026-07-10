use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use fortress_rollback::{
    ClockFn, Config, Message, NetworkStats, NonBlockingSocket, P2PSession, PlayerHandle,
    PlayerType, ProtocolConfig, SessionBuilder, SessionState,
};
use godot::prelude::*;
use web_time::{Duration, Instant};

const FIRST_ADDRESS: u8 = 1;
const SECOND_ADDRESS: u8 = 2;
const MAX_QUEUED_MESSAGES: usize = 256;
const MAX_SYNC_POLLS: usize = 100;
const REAL_CLOCK_DEADLINE: Duration = Duration::from_millis(100);
const VIRTUAL_POLL_INTERVAL_MS: u64 = 50;
const VIRTUAL_MEASUREMENT_POLLS: usize = 40;
const EXPECTED_VIRTUAL_RTT_MS: u128 = 50;

struct ProbeConfig;

impl Config for ProbeConfig {
    type Input = u8;
    type State = ();
    type Address = u8;
}

#[derive(Default)]
struct SocketHealth {
    failed: Cell<bool>,
    sends: Cell<u64>,
}

struct MemorySocket {
    local_address: u8,
    peer_address: u8,
    inbox: Rc<RefCell<VecDeque<(u8, Message)>>>,
    peer_inbox: Rc<RefCell<VecDeque<(u8, Message)>>>,
    health: Rc<SocketHealth>,
}

impl NonBlockingSocket<u8> for MemorySocket {
    fn send_to(&mut self, message: &Message, address: &u8) {
        if *address != self.peer_address {
            self.health.failed.set(true);
            return;
        }

        let Ok(mut inbox) = self.peer_inbox.try_borrow_mut() else {
            self.health.failed.set(true);
            return;
        };
        if inbox.len() >= MAX_QUEUED_MESSAGES {
            self.health.failed.set(true);
            return;
        }

        inbox.push_back((self.local_address, message.clone()));
        self.health
            .sends
            .set(self.health.sends.get().saturating_add(1));
    }

    fn receive_all_messages(&mut self) -> Vec<(u8, Message)> {
        let Ok(mut inbox) = self.inbox.try_borrow_mut() else {
            self.health.failed.set(true);
            return Vec::new();
        };

        let message_count = inbox.len().min(MAX_QUEUED_MESSAGES);
        let mut messages = Vec::new();
        if messages.try_reserve_exact(message_count).is_err() {
            self.health.failed.set(true);
            return messages;
        }
        for _ in 0..message_count {
            if let Some(message) = inbox.pop_front() {
                messages.push(message);
            }
        }
        messages
    }
}

struct SessionPair {
    first: P2PSession<ProbeConfig>,
    second: P2PSession<ProbeConfig>,
    health: Rc<SocketHealth>,
}

fn create_socket_pair() -> (MemorySocket, MemorySocket, Rc<SocketHealth>) {
    let first_inbox = Rc::new(RefCell::new(VecDeque::new()));
    let second_inbox = Rc::new(RefCell::new(VecDeque::new()));
    let health = Rc::new(SocketHealth::default());

    let first = MemorySocket {
        local_address: FIRST_ADDRESS,
        peer_address: SECOND_ADDRESS,
        inbox: Rc::clone(&first_inbox),
        peer_inbox: Rc::clone(&second_inbox),
        health: Rc::clone(&health),
    };
    let second = MemorySocket {
        local_address: SECOND_ADDRESS,
        peer_address: FIRST_ADDRESS,
        inbox: second_inbox,
        peer_inbox: first_inbox,
        health: Rc::clone(&health),
    };
    (first, second, health)
}

fn create_sessions(
    clock: Option<ClockFn>,
    quality_interval: Duration,
) -> Result<SessionPair, String> {
    let (first_socket, second_socket, health) = create_socket_pair();
    let protocol_config = |seed| ProtocolConfig {
        quality_report_interval: quality_interval,
        protocol_rng_seed: Some(seed),
        clock: clock.clone(),
        ..ProtocolConfig::default()
    };

    let first = SessionBuilder::<ProbeConfig>::new()
        .with_protocol_config(protocol_config(101))
        .add_player(PlayerType::Local, PlayerHandle::new(0))
        .map_err(|error| format!("first local player: {error}"))?
        .add_player(PlayerType::Remote(SECOND_ADDRESS), PlayerHandle::new(1))
        .map_err(|error| format!("first remote player: {error}"))?
        .start_p2p_session(first_socket)
        .map_err(|error| format!("first session: {error}"))?;

    let second = SessionBuilder::<ProbeConfig>::new()
        .with_protocol_config(protocol_config(202))
        .add_player(PlayerType::Remote(FIRST_ADDRESS), PlayerHandle::new(0))
        .map_err(|error| format!("second remote player: {error}"))?
        .add_player(PlayerType::Local, PlayerHandle::new(1))
        .map_err(|error| format!("second local player: {error}"))?
        .start_p2p_session(second_socket)
        .map_err(|error| format!("second session: {error}"))?;

    Ok(SessionPair {
        first,
        second,
        health,
    })
}

fn sessions_are_running(pair: &SessionPair) -> bool {
    pair.first.current_state() == SessionState::Running
        && pair.second.current_state() == SessionState::Running
}

fn synchronize_sessions(
    pair: &mut SessionPair,
    mut after_poll: impl FnMut(),
) -> Result<(), String> {
    for _ in 0..MAX_SYNC_POLLS {
        pair.first.poll_remote_clients();
        pair.second.poll_remote_clients();
        if sessions_are_running(pair) {
            return Ok(());
        }
        after_poll();
    }

    Err("sessions did not synchronize within 100 poll iterations".to_owned())
}

fn run_real_clock_probe() -> Result<u64, String> {
    let mut pair = create_sessions(None, Duration::from_millis(1))?;
    synchronize_sessions(&mut pair, || {})?;

    for _ in 0..4 {
        pair.first.poll_remote_clients();
        pair.second.poll_remote_clients();
    }
    let sends_before = pair.health.sends.get();
    let deadline = Instant::now() + REAL_CLOCK_DEADLINE;

    while Instant::now() < deadline {
        pair.first.poll_remote_clients();
        pair.second.poll_remote_clients();
        pair.first.poll_remote_clients();
        pair.second.poll_remote_clients();

        if pair.health.failed.get() {
            return Err("in-memory socket failed during real-clock probe".to_owned());
        }
        let delta = pair.health.sends.get().saturating_sub(sends_before);
        if delta >= 2 {
            return Ok(delta);
        }
    }

    Err("default-clock quality exchange did not fire within 100ms".to_owned())
}

fn session_ping(
    stats: Result<NetworkStats, fortress_rollback::FortressError>,
) -> Result<u128, String> {
    stats
        .map(|network_stats| network_stats.ping)
        .map_err(|error| format!("network stats: {error}"))
}

fn run_virtual_clock_probe() -> Result<(u128, u128), String> {
    let base = Instant::now();
    let offset_ms = Arc::new(AtomicU64::new(0));
    let clock_offset = Arc::clone(&offset_ms);
    let clock: ClockFn =
        Arc::new(move || base + Duration::from_millis(clock_offset.load(Ordering::Relaxed)));
    let mut pair = create_sessions(
        Some(clock),
        ProtocolConfig::default().quality_report_interval,
    )?;
    synchronize_sessions(&mut pair, || {
        offset_ms.fetch_add(VIRTUAL_POLL_INTERVAL_MS, Ordering::Relaxed);
    })?;

    for _ in 0..VIRTUAL_MEASUREMENT_POLLS {
        pair.first.poll_remote_clients();
        pair.second.poll_remote_clients();
        offset_ms.fetch_add(VIRTUAL_POLL_INTERVAL_MS, Ordering::Relaxed);
    }

    if pair.health.failed.get() {
        return Err("in-memory socket failed during virtual-clock probe".to_owned());
    }
    if !sessions_are_running(&pair) {
        return Err("sessions left Running state during virtual-clock probe".to_owned());
    }

    let first_ping = session_ping(pair.first.network_stats(PlayerHandle::new(1)))?;
    let second_ping = session_ping(pair.second.network_stats(PlayerHandle::new(0)))?;
    if first_ping != EXPECTED_VIRTUAL_RTT_MS || second_ping != EXPECTED_VIRTUAL_RTT_MS {
        return Err(format!(
            "expected 50ms virtual RTT in both directions, got {first_ping}ms and {second_ping}ms"
        ));
    }

    Ok((first_ping, second_ping))
}

fn mode() -> &'static str {
    if cfg!(feature = "nothreads") {
        "nothreads"
    } else {
        "threaded"
    }
}

fn dictionary_int(value: u128) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

#[derive(GodotClass)]
#[class(init, base=RefCounted)]
struct FortressEmscriptenProbe {
    base: Base<RefCounted>,
}

#[godot_api]
impl FortressEmscriptenProbe {
    #[func]
    fn run_probe(&self) -> VarDictionary {
        let mut result = VarDictionary::new();
        result.set("status", "complete");
        result.set("mode", mode());
        result.set("target_os", std::env::consts::OS);

        match run_real_clock_probe().and_then(|real_clock_send_delta| {
            run_virtual_clock_probe().map(|pings| (real_clock_send_delta, pings))
        }) {
            Ok((real_clock_send_delta, (first_ping, second_ping))) => {
                result.set("ok", true);
                result.set("real_clock_smoke", true);
                result.set(
                    "real_clock_send_delta",
                    i64::try_from(real_clock_send_delta).unwrap_or(i64::MAX),
                );
                result.set("ping_a_ms", dictionary_int(first_ping));
                result.set("ping_b_ms", dictionary_int(second_ping));
                result.set("error", "");
            },
            Err(error) => {
                result.set("ok", false);
                result.set("real_clock_smoke", false);
                result.set("real_clock_send_delta", 0);
                result.set("ping_a_ms", -1);
                result.set("ping_b_ms", -1);
                result.set("error", error.as_str());
            },
        }

        result
    }
}

struct FortressProbeExtension;

// SAFETY: godot-rust requires this marker to register generated GDExtension
// callbacks. The implementation supplies no raw pointers or custom lifecycle code.
#[gdextension]
unsafe impl ExtensionLibrary for FortressProbeExtension {}
