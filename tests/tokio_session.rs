//! End-to-end runtime coverage for `TokioUdpSocket` after session ownership.

#![cfg(feature = "tokio")]
#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]

use fortress_rollback::tokio_socket::TokioUdpSocket;
use fortress_rollback::{
    Config, FortressRequest, Frame, InputVec, P2PSession, PlayerHandle, PlayerType, ProtocolConfig,
    RequestVec, SessionBuilder, SessionState,
};
use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::timeout;
use web_time::Instant;

const TEST_TIMEOUT: Duration = Duration::from_secs(10);
const SYNC_POLL_BUDGET: usize = 2_000;
const INPUT_POLL_BUDGET: usize = 512;
const FRAME_COUNT: u32 = 4;

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct RuntimeInput {
    value: u32,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct RuntimeState {
    frame: i32,
    input_sum: u64,
}

#[derive(Debug)]
struct RuntimeConfig;

impl Config for RuntimeConfig {
    type Input = RuntimeInput;
    type State = RuntimeState;
    type Address = SocketAddr;
}

#[derive(Clone)]
struct TestClock {
    now: Arc<Mutex<Instant>>,
}

impl TestClock {
    fn new() -> Self {
        Self {
            now: Arc::new(Mutex::new(Instant::now())),
        }
    }

    fn protocol_config(&self, seed: u64) -> ProtocolConfig {
        let now = Arc::clone(&self.now);
        ProtocolConfig {
            protocol_rng_seed: Some(seed),
            clock: Some(Arc::new(move || {
                *now.lock().expect("Tokio session test clock poisoned")
            })),
            ..ProtocolConfig::default()
        }
    }

    fn advance(&self) {
        let mut now = self.now.lock().expect("Tokio session test clock poisoned");
        *now += Duration::from_millis(10);
    }
}

fn loopback_addr(socket: &TokioUdpSocket) -> Result<SocketAddr, String> {
    let bound = socket
        .local_addr()
        .map_err(|error| format!("failed to read Tokio UDP address: {error}"))?;
    Ok(SocketAddr::from((Ipv4Addr::LOCALHOST, bound.port())))
}

fn build_session(
    socket: TokioUdpSocket,
    local: PlayerHandle,
    remote: PlayerHandle,
    remote_addr: SocketAddr,
    protocol: ProtocolConfig,
) -> Result<P2PSession<RuntimeConfig>, String> {
    SessionBuilder::<RuntimeConfig>::new()
        .with_protocol_config(protocol)
        .add_player(PlayerType::Local, local)
        .and_then(|builder| builder.add_player(PlayerType::Remote(remote_addr), remote))
        .and_then(|builder| builder.start_p2p_session(socket))
        .map_err(|error| format!("failed to build Tokio-owned session: {error}"))
}

fn apply_requests(state: &mut RuntimeState, requests: RequestVec<RuntimeConfig>) {
    for request in requests {
        match request {
            FortressRequest::SaveGameState { cell, frame } => {
                assert_eq!(state.frame, frame.as_i32());
                let checksum = u128::from(state.input_sum) ^ (state.frame as u128);
                cell.save(frame, Some(*state), Some(checksum));
            },
            FortressRequest::LoadGameState { cell, .. } => {
                *state = cell.load().expect("saved Tokio test state must exist");
            },
            FortressRequest::AdvanceFrame { inputs } => advance_state(state, &inputs),
        }
    }
}

fn advance_state(state: &mut RuntimeState, inputs: &InputVec<RuntimeInput>) {
    state.input_sum = state.input_sum.saturating_add(
        inputs
            .iter()
            .map(|(input, _status)| u64::from(input.value))
            .sum::<u64>(),
    );
    state.frame = state.frame.saturating_add(1);
}

fn record_diagnostics(
    diagnostics: &Arc<Mutex<String>>,
    stage: &str,
    polls: usize,
    first: &P2PSession<RuntimeConfig>,
    second: &P2PSession<RuntimeConfig>,
) {
    let message = format!(
        "stage={stage}, polls={polls}, states=({:?}, {:?}), frames=({}, {}), confirmed=({}, {})",
        first.current_state(),
        second.current_state(),
        first.current_frame(),
        second.current_frame(),
        first.confirmed_frame(),
        second.confirmed_frame()
    );
    *diagnostics
        .lock()
        .expect("Tokio session diagnostics lock poisoned") = message;
}

async fn run_tokio_owned_sessions(diagnostics: Arc<Mutex<String>>) -> Result<(), String> {
    let socket1 = TokioUdpSocket::bind_to_port(0)
        .await
        .map_err(|error| format!("failed to bind first Tokio socket: {error}"))?;
    let socket2 = TokioUdpSocket::bind_to_port(0)
        .await
        .map_err(|error| format!("failed to bind second Tokio socket: {error}"))?;
    let addr1 = loopback_addr(&socket1)?;
    let addr2 = loopback_addr(&socket2)?;
    assert_ne!(addr1, addr2, "ephemeral Tokio sockets must be distinct");

    let clock = TestClock::new();
    let mut session1 = build_session(
        socket1,
        PlayerHandle::new(0),
        PlayerHandle::new(1),
        addr2,
        clock.protocol_config(0xA11C_E001),
    )?;
    let mut session2 = build_session(
        socket2,
        PlayerHandle::new(1),
        PlayerHandle::new(0),
        addr1,
        clock.protocol_config(0xA11C_E002),
    )?;

    let mut sync_polls = 0usize;
    while sync_polls < SYNC_POLL_BUDGET {
        session1.poll_remote_clients();
        session2.poll_remote_clients();
        sync_polls = sync_polls.saturating_add(1);
        record_diagnostics(
            &diagnostics,
            "synchronizing",
            sync_polls,
            &session1,
            &session2,
        );
        if session1.current_state() == SessionState::Running
            && session2.current_state() == SessionState::Running
        {
            break;
        }
        clock.advance();
        tokio::task::yield_now().await;
    }
    if session1.current_state() != SessionState::Running
        || session2.current_state() != SessionState::Running
    {
        return Err(format!(
            "Tokio sessions did not synchronize within {SYNC_POLL_BUDGET} polls: {}",
            diagnostics
                .lock()
                .expect("Tokio session diagnostics lock poisoned")
        ));
    }
    assert!(
        sync_polls > 0,
        "synchronization oracle must poll the sessions"
    );

    let mut state1 = RuntimeState::default();
    let mut state2 = RuntimeState::default();
    for frame_index in 0..FRAME_COUNT {
        let frame = Frame::new(frame_index as i32);
        let input = RuntimeInput { value: 1 };
        session1
            .add_local_input(PlayerHandle::new(0), input)
            .map_err(|error| format!("session 1 rejected frame {frame}: {error}"))?;
        session2
            .add_local_input(PlayerHandle::new(1), input)
            .map_err(|error| format!("session 2 rejected frame {frame}: {error}"))?;

        // `advance_frame` moves each pending local input into the endpoint's
        // outbound protocol window. Symmetric values keep both peers equal
        // during the initial prediction window before packets arrive.
        let requests1 = session1
            .advance_frame()
            .map_err(|error| format!("session 1 could not advance frame {frame}: {error}"))?;
        let requests2 = session2
            .advance_frame()
            .map_err(|error| format!("session 2 could not advance frame {frame}: {error}"))?;
        apply_requests(&mut state1, requests1);
        apply_requests(&mut state2, requests2);
        assert_eq!(state1.frame, frame_index as i32 + 1);
        assert_eq!(state2.frame, frame_index as i32 + 1);
        if frame_index > 0 {
            assert_eq!(
                state1, state2,
                "Tokio peers did not converge after the frame-0 repair opportunity at {frame}"
            );
        }

        let mut input_polls = 0usize;
        while input_polls < INPUT_POLL_BUDGET
            && (session1.confirmed_frame() < frame || session2.confirmed_frame() < frame)
        {
            session1.poll_remote_clients();
            session2.poll_remote_clients();
            input_polls = input_polls.saturating_add(1);
            record_diagnostics(
                &diagnostics,
                "confirming-inputs",
                input_polls,
                &session1,
                &session2,
            );
            clock.advance();
            tokio::task::yield_now().await;
        }
        if session1.confirmed_frame() < frame || session2.confirmed_frame() < frame {
            return Err(format!(
                "frame {frame} was not confirmed within {INPUT_POLL_BUDGET} polls: {}",
                diagnostics
                    .lock()
                    .expect("Tokio session diagnostics lock poisoned")
            ));
        }

        let confirmed1 = session1
            .confirmed_inputs_for_frame(frame)
            .map_err(|error| format!("session 1 could not read frame {frame}: {error}"))?;
        let confirmed2 = session2
            .confirmed_inputs_for_frame(frame)
            .map_err(|error| format!("session 2 could not read frame {frame}: {error}"))?;
        assert_eq!(confirmed1, vec![input, input]);
        assert_eq!(confirmed2, confirmed1);
    }

    let metrics1 = session1
        .peer_metrics(PlayerHandle::new(1))
        .map_err(|error| format!("session 1 metrics unavailable: {error}"))?;
    let metrics2 = session2
        .peer_metrics(PlayerHandle::new(0))
        .map_err(|error| format!("session 2 metrics unavailable: {error}"))?;
    assert!(metrics1.packets_sent > 0 && metrics1.packets_received > 0);
    assert!(metrics2.packets_sent > 0 && metrics2.packets_received > 0);
    assert_eq!(state1.frame, FRAME_COUNT as i32);
    assert_eq!(state1, state2);
    assert_eq!(state1.input_sum, u64::from(FRAME_COUNT).saturating_mul(2));
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn sessions_exchange_confirmed_frames_after_tokio_socket_ownership_move() {
    let diagnostics = Arc::new(Mutex::new(String::from("test setup not started")));
    let timeout_diagnostics = Arc::clone(&diagnostics);
    match timeout(TEST_TIMEOUT, run_tokio_owned_sessions(diagnostics)).await {
        Ok(Ok(())) => {},
        Ok(Err(error)) => panic!("Tokio session runtime oracle failed: {error}"),
        Err(error) => panic!(
            "Tokio session runtime oracle exceeded {TEST_TIMEOUT:?} ({error}): {}",
            timeout_diagnostics
                .lock()
                .expect("Tokio session diagnostics lock poisoned")
        ),
    }
}
