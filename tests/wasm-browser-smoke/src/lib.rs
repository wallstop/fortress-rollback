#[cfg(all(test, target_arch = "wasm32", target_os = "unknown"))]
mod tests {
    use fortress_rollback::network::codec;
    use fortress_rollback::{
        ChaosConfig, ChaosSocket, Config, FortressRequest, Frame, InputVec, Message,
        NonBlockingSocket, PlayerHandle, PlayerType, ProtocolConfig, RequestVec, SessionBuilder,
        SessionState,
    };
    use serde::{Deserialize, Serialize};
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::rc::Rc;
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    wasm_bindgen_test_configure!(run_in_browser);

    const MAX_RAW_PACKETS_PER_POLL: usize = 8;
    const MAX_QUEUED_RAW_PACKETS: usize = 64;
    const SYNC_POLL_BUDGET: usize = 256;
    const INPUT_POLL_BUDGET: usize = 64;
    const FRAME_COUNT: u32 = 2;
    const MALFORMED_PACKET_COUNT: usize = MAX_RAW_PACKETS_PER_POLL + 1;

    struct EmptySocket;

    impl NonBlockingSocket<u8> for EmptySocket {
        fn send_to(&mut self, _msg: &Message, _addr: &u8) {}

        fn receive_all_messages(&mut self) -> Vec<(u8, Message)> {
            Vec::new()
        }
    }

    #[wasm_bindgen_test]
    fn default_clock_receive_does_not_panic_in_browser() {
        let config = ChaosConfig::builder().seed(42).build();
        let mut socket = ChaosSocket::new(EmptySocket, config);

        // Checking for ready packets reads the default clock even when the queue is empty.
        let messages = socket.receive_all_messages();

        assert!(messages.is_empty(), "empty inner socket returned messages");
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct BrowserPeer(u8);

    #[derive(Debug, Default, Clone, Copy)]
    struct RawTransportStats {
        send_calls: usize,
        poll_calls: usize,
        decode_attempts: usize,
        decoded_packets: usize,
        rejected_packets: usize,
        dropped_full_packets: usize,
        max_attempts_in_one_poll: usize,
    }

    #[derive(Default)]
    struct RawBusState {
        queues: [VecDeque<(BrowserPeer, Vec<u8>)>; 2],
        stats: RawTransportStats,
    }

    #[derive(Clone)]
    struct RawBus {
        state: Rc<RefCell<RawBusState>>,
    }

    impl RawBus {
        fn pair() -> (RawChannelSocket, RawChannelSocket, Self) {
            let bus = Self {
                state: Rc::new(RefCell::new(RawBusState::default())),
            };
            (
                RawChannelSocket {
                    local: BrowserPeer(0),
                    bus: bus.clone(),
                },
                RawChannelSocket {
                    local: BrowserPeer(1),
                    bus: bus.clone(),
                },
                bus,
            )
        }

        fn inject_raw(&self, source: BrowserPeer, destination: BrowserPeer, bytes: Vec<u8>) {
            let mut state = self.state.borrow_mut();
            let queue = state
                .queues
                .get_mut(usize::from(destination.0))
                .expect("browser test destination must exist");
            assert!(
                queue.len() < MAX_QUEUED_RAW_PACKETS,
                "browser raw test queue overflow while injecting malformed packets"
            );
            queue.push_back((source, bytes));
        }

        fn stats(&self) -> RawTransportStats {
            self.state.borrow().stats
        }
    }

    struct RawChannelSocket {
        local: BrowserPeer,
        bus: RawBus,
    }

    impl NonBlockingSocket<BrowserPeer> for RawChannelSocket {
        fn send_to(&mut self, message: &Message, destination: &BrowserPeer) {
            let Ok(bytes) = codec::encode(message) else {
                return;
            };
            let mut state = self.bus.state.borrow_mut();
            state.stats.send_calls = state.stats.send_calls.saturating_add(1);
            let Some(queue) = state.queues.get_mut(usize::from(destination.0)) else {
                return;
            };
            if queue.len() >= MAX_QUEUED_RAW_PACKETS {
                state.stats.dropped_full_packets =
                    state.stats.dropped_full_packets.saturating_add(1);
                return;
            }
            queue.push_back((self.local, bytes));
        }

        fn receive_all_messages(&mut self) -> Vec<(BrowserPeer, Message)> {
            let mut messages = Vec::new();
            if messages
                .try_reserve_exact(MAX_RAW_PACKETS_PER_POLL)
                .is_err()
            {
                return messages;
            }

            let mut state = self.bus.state.borrow_mut();
            state.stats.poll_calls = state.stats.poll_calls.saturating_add(1);
            let mut attempts = 0usize;
            while attempts < MAX_RAW_PACKETS_PER_POLL {
                let packet = state
                    .queues
                    .get_mut(usize::from(self.local.0))
                    .and_then(VecDeque::pop_front);
                let Some((source, bytes)) = packet else {
                    break;
                };
                attempts = attempts.saturating_add(1);
                state.stats.decode_attempts = state.stats.decode_attempts.saturating_add(1);
                match codec::decode_message(&bytes) {
                    Ok((message, consumed)) if consumed == bytes.len() => {
                        messages.push((source, message));
                        state.stats.decoded_packets = state.stats.decoded_packets.saturating_add(1);
                    },
                    Ok(_) | Err(_) => {
                        state.stats.rejected_packets =
                            state.stats.rejected_packets.saturating_add(1);
                    },
                }
            }
            state.stats.max_attempts_in_one_poll =
                state.stats.max_attempts_in_one_poll.max(attempts);
            messages
        }
    }

    #[repr(C)]
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    struct BrowserInput {
        value: u32,
    }

    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
    struct BrowserState {
        frame: i32,
        input_sum: u64,
    }

    #[derive(Debug)]
    struct BrowserConfig;

    impl Config for BrowserConfig {
        type Input = BrowserInput;
        type State = BrowserState;
        type Address = BrowserPeer;
    }

    fn apply_requests(state: &mut BrowserState, requests: RequestVec<BrowserConfig>) {
        for request in requests {
            match request {
                FortressRequest::SaveGameState { cell, frame } => {
                    assert_eq!(state.frame, frame.as_i32());
                    cell.save(frame, Some(*state), Some(u128::from(state.input_sum)));
                },
                FortressRequest::LoadGameState { cell, .. } => {
                    *state = cell.load().expect("browser test state must exist");
                },
                FortressRequest::AdvanceFrame { inputs } => advance_state(state, &inputs),
            }
        }
    }

    fn advance_state(state: &mut BrowserState, inputs: &InputVec<BrowserInput>) {
        state.input_sum = state.input_sum.saturating_add(
            inputs
                .iter()
                .map(|(input, _status)| u64::from(input.value))
                .sum::<u64>(),
        );
        state.frame = state.frame.saturating_add(1);
    }

    #[wasm_bindgen_test]
    fn raw_channel_sessions_reject_malformed_packet_and_exchange_frame() {
        let (socket1, socket2, bus) = RawBus::pair();
        for marker in 0..MALFORMED_PACKET_COUNT {
            bus.inject_raw(BrowserPeer(1), BrowserPeer(0), vec![0xF5, marker as u8]);
        }

        let mut session1 = SessionBuilder::<BrowserConfig>::new()
            .with_protocol_config(ProtocolConfig::deterministic(0xB001))
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .and_then(|builder| {
                builder.add_player(PlayerType::Remote(BrowserPeer(1)), PlayerHandle::new(1))
            })
            .and_then(|builder| builder.start_p2p_session(socket1))
            .expect("first browser raw-channel session must build");
        let mut session2 = SessionBuilder::<BrowserConfig>::new()
            .with_protocol_config(ProtocolConfig::deterministic(0xB002))
            .add_player(PlayerType::Remote(BrowserPeer(0)), PlayerHandle::new(0))
            .and_then(|builder| builder.add_player(PlayerType::Local, PlayerHandle::new(1)))
            .and_then(|builder| builder.start_p2p_session(socket2))
            .expect("second browser raw-channel session must build");

        let mut sync_polls = 0usize;
        while sync_polls < SYNC_POLL_BUDGET
            && (session1.current_state() != SessionState::Running
                || session2.current_state() != SessionState::Running)
        {
            session1.poll_remote_clients();
            session2.poll_remote_clients();
            sync_polls = sync_polls.saturating_add(1);
        }
        assert!(
            session1.current_state() == SessionState::Running
                && session2.current_state() == SessionState::Running,
            "browser sessions failed to synchronize within {SYNC_POLL_BUDGET} polls: polls={sync_polls}, states=({:?}, {:?}), stats={:?}",
            session1.current_state(),
            session2.current_state(),
            bus.stats()
        );
        assert!(sync_polls > 0, "browser synchronization oracle was vacuous");

        let input = BrowserInput { value: 9 };
        let mut state1 = BrowserState::default();
        let mut state2 = BrowserState::default();
        for frame_index in 0..FRAME_COUNT {
            let frame = Frame::new(frame_index as i32);
            session1
                .add_local_input(PlayerHandle::new(0), input)
                .expect("first browser input must be accepted");
            session2
                .add_local_input(PlayerHandle::new(1), input)
                .expect("second browser input must be accepted");

            // Advancing publishes each local input to the raw channel. Frame 0
            // may be predicted asymmetrically; the same confirmed value on
            // frame 1 makes repeat-last prediction exact and repairs frame 0.
            apply_requests(
                &mut state1,
                session1
                    .advance_frame()
                    .expect("first browser session must advance"),
            );
            apply_requests(
                &mut state2,
                session2
                    .advance_frame()
                    .expect("second browser session must advance"),
            );
            assert_eq!(state1.frame, frame_index as i32 + 1);
            assert_eq!(state2.frame, frame_index as i32 + 1);

            let mut input_polls = 0usize;
            while input_polls < INPUT_POLL_BUDGET
                && (session1.confirmed_frame() < frame || session2.confirmed_frame() < frame)
            {
                session1.poll_remote_clients();
                session2.poll_remote_clients();
                input_polls = input_polls.saturating_add(1);
            }
            assert!(
                session1.confirmed_frame() >= frame && session2.confirmed_frame() >= frame,
                "browser frame {frame} failed to confirm within {INPUT_POLL_BUDGET} polls: polls={input_polls}, confirmed=({}, {}), states=({state1:?}, {state2:?}), stats={:?}",
                session1.confirmed_frame(),
                session2.confirmed_frame(),
                bus.stats()
            );
            let confirmed1 = session1
                .confirmed_inputs_for_frame(frame)
                .expect("first browser session must expose confirmed inputs");
            let confirmed2 = session2
                .confirmed_inputs_for_frame(frame)
                .expect("second browser session must expose confirmed inputs");
            assert_eq!(confirmed1, vec![input, input]);
            assert_eq!(confirmed2, confirmed1);
        }
        assert_eq!(
            state1,
            BrowserState {
                frame: FRAME_COUNT as i32,
                input_sum: u64::from(FRAME_COUNT).saturating_mul(18),
            }
        );
        assert_eq!(
            state2, state1,
            "browser peers must converge after frame-0 rollback repair"
        );

        let stats = bus.stats();
        assert_eq!(stats.rejected_packets, MALFORMED_PACKET_COUNT);
        assert_eq!(stats.max_attempts_in_one_poll, MAX_RAW_PACKETS_PER_POLL);
        assert!(stats.send_calls > 0 && stats.decoded_packets > 0);
        assert!(stats.poll_calls > 0 && stats.decode_attempts > MALFORMED_PACKET_COUNT);
        assert_eq!(
            stats.dropped_full_packets, 0,
            "browser test queue overflowed"
        );
    }
}
