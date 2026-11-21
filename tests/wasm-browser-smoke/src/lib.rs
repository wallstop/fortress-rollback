#[cfg(all(test, target_arch = "wasm32", target_os = "unknown"))]
mod tests {
    use fortress_replay_fixture::{EXPECTED_DIGEST, EXPECTED_FRAMES, EXPECTED_REQUEST_DIGEST};
    use fortress_rollback::network::codec;
    use fortress_rollback::{
        ChaosConfig, ChaosSocket, Config, DisconnectBehavior, FortressEvent, FortressRequest,
        Frame, InputVec, Message, NonBlockingSocket, P2PSession, PlayerHandle, PlayerType,
        ProtocolConfig, RequestVec, SessionBuilder, SessionState,
    };
    use serde::{Deserialize, Serialize};
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
    use web_time::{Duration, Instant};

    wasm_bindgen_test_configure!(run_in_browser);

    const MAX_RAW_PACKETS_PER_POLL: usize = 8;
    const MAX_QUEUED_RAW_PACKETS: usize = 64;
    const SYNC_POLL_BUDGET: usize = 256;
    const INPUT_POLL_BUDGET: usize = 64;
    const REMOVAL_POLL_BUDGET: usize = 512;
    const HOT_JOIN_POLL_BUDGET: usize = 512;
    const HOT_JOIN_CONFIRMED_FRAMES: u32 = 2;
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
    fn coordinated_drop_wire_codec_matches_native_in_browser() {
        assert!(
            fortress_replay_fixture::check_wire_codec_digest(),
            "the coordinated-drop wire codec digest differs in the browser"
        );
    }

    #[wasm_bindgen_test]
    fn captured_native_replay_replays_identically_in_browser() {
        let evidence = fortress_replay_fixture::replay_fixture()
            .expect("checked-in captured schedule must replay in the browser");
        assert_eq!(
            evidence.frames, EXPECTED_FRAMES,
            "browser replay frame count changed"
        );
        assert_eq!(evidence.desyncs, 0, "browser replay reported a desync");
        assert_eq!(
            evidence.digest, EXPECTED_DIGEST,
            "browser replay state-checksum digest differs from native"
        );
        assert_eq!(
            evidence.request_digest, EXPECTED_REQUEST_DIGEST,
            "browser replay request stream differs from native"
        );
        assert_eq!(evidence.requests, u64::from(EXPECTED_FRAMES) * 2);
    }

    #[wasm_bindgen_test]
    fn machine_width_hashing_matches_native_reference() {
        fortress_replay_fixture::check_machine_width_parity().expect(
            "browser hashing of usize/isize/collections must match the native reference constants",
        );
    }

    #[wasm_bindgen_test]
    fn input_compression_matches_native_reference() {
        fortress_replay_fixture::check_compression_parity().expect(
            "browser input-compression and RLE bytes must match the native reference digest",
        );
    }

    fn virtual_clock() -> (fortress_rollback::ClockFn, Arc<AtomicU64>) {
        let base = Instant::now();
        let offset_ms = Arc::new(AtomicU64::new(0));
        let clock_offset = Arc::clone(&offset_ms);
        let clock: fortress_rollback::ClockFn =
            Arc::new(move || base + Duration::from_millis(clock_offset.load(Ordering::Relaxed)));
        (clock, offset_ms)
    }

    fn build_virtual_clock_sessions(
        clock: fortress_rollback::ClockFn,
    ) -> (P2PSession<BrowserConfig>, P2PSession<BrowserConfig>) {
        build_virtual_clock_sessions_with_behavior(clock, DisconnectBehavior::Halt)
    }

    fn build_virtual_clock_sessions_with_behavior(
        clock: fortress_rollback::ClockFn,
        behavior: DisconnectBehavior,
    ) -> (P2PSession<BrowserConfig>, P2PSession<BrowserConfig>) {
        let (socket1, socket2, _bus) = RawBus::pair();
        let first = SessionBuilder::<BrowserConfig>::new()
            .with_protocol_config(ProtocolConfig {
                protocol_rng_seed: Some(0xB101),
                clock: Some(clock.clone()),
                ..ProtocolConfig::default()
            })
            .with_disconnect_behavior(behavior)
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .and_then(|builder| {
                builder.add_player(PlayerType::Remote(BrowserPeer(1)), PlayerHandle::new(1))
            })
            .and_then(|builder| builder.start_p2p_session(socket1))
            .expect("first browser suspension session must build");
        let second = SessionBuilder::<BrowserConfig>::new()
            .with_protocol_config(ProtocolConfig {
                protocol_rng_seed: Some(0xB102),
                clock: Some(clock),
                ..ProtocolConfig::default()
            })
            .with_disconnect_behavior(behavior)
            .add_player(PlayerType::Remote(BrowserPeer(0)), PlayerHandle::new(0))
            .and_then(|builder| builder.add_player(PlayerType::Local, PlayerHandle::new(1)))
            .and_then(|builder| builder.start_p2p_session(socket2))
            .expect("second browser suspension session must build");
        (first, second)
    }

    fn poll_pair_with_clock(
        session1: &mut P2PSession<BrowserConfig>,
        session2: &mut P2PSession<BrowserConfig>,
        offset_ms: &Arc<AtomicU64>,
    ) {
        session1.poll_remote_clients();
        session2.poll_remote_clients();
        offset_ms.fetch_add(16, Ordering::Relaxed);
    }

    #[derive(Debug, Default, Clone, Copy)]
    struct LinkEvents {
        interrupted: bool,
        resumed: bool,
        disconnected: bool,
    }

    impl LinkEvents {
        fn merge(&mut self, other: Self) {
            self.interrupted |= other.interrupted;
            self.resumed |= other.resumed;
            self.disconnected |= other.disconnected;
        }
    }

    fn drain_link_events(session: &mut P2PSession<BrowserConfig>) -> LinkEvents {
        let mut events = LinkEvents::default();
        for event in session.events() {
            match event {
                fortress_rollback::FortressEvent::NetworkInterrupted { .. } => {
                    events.interrupted = true;
                },
                fortress_rollback::FortressEvent::NetworkResumed { .. } => {
                    events.resumed = true;
                },
                fortress_rollback::FortressEvent::Disconnected { .. } => {
                    events.disconnected = true;
                },
                _ => {},
            }
        }
        events
    }

    #[wasm_bindgen_test]
    fn browser_tab_suspension_within_timeout_resumes_and_advances() {
        let (clock, offset_ms) = virtual_clock();
        let (mut session1, mut session2) = build_virtual_clock_sessions(clock);

        let mut sync_polls = 0usize;
        while sync_polls < SYNC_POLL_BUDGET
            && (session1.current_state() != SessionState::Running
                || session2.current_state() != SessionState::Running)
        {
            poll_pair_with_clock(&mut session1, &mut session2, &offset_ms);
            sync_polls = sync_polls.saturating_add(1);
        }
        assert!(
            session1.current_state() == SessionState::Running
                && session2.current_state() == SessionState::Running,
            "virtual-clock sessions failed to synchronize"
        );
        let _ = session1.events().count();
        let _ = session2.events().count();

        let mut state1 = BrowserState::default();
        let mut state2 = BrowserState::default();
        for frame_index in 0..6u32 {
            session1
                .add_local_input(PlayerHandle::new(0), BrowserInput { value: frame_index })
                .expect("first suspended-tab input must be accepted");
            session2
                .add_local_input(PlayerHandle::new(1), BrowserInput { value: frame_index })
                .expect("second suspended-tab input must be accepted");
            apply_requests(
                &mut state1,
                session1
                    .advance_frame()
                    .expect("first suspended-tab session must advance"),
            );
            apply_requests(
                &mut state2,
                session2
                    .advance_frame()
                    .expect("second suspended-tab session must advance"),
            );
            poll_pair_with_clock(&mut session1, &mut session2, &offset_ms);
        }

        // A throttled background tab performs no polls while wall time passes.
        // Stay above the 500ms interruption notice and below the 2s timeout.
        offset_ms.fetch_add(1_000, Ordering::Relaxed);

        let mut events1 = LinkEvents::default();
        let mut events2 = LinkEvents::default();
        for _ in 0..64 {
            poll_pair_with_clock(&mut session1, &mut session2, &offset_ms);
            events1.merge(drain_link_events(&mut session1));
            events2.merge(drain_link_events(&mut session2));
            if (events1.interrupted && events1.resumed) || (events2.interrupted && events2.resumed)
            {
                break;
            }
        }
        assert!(
            events1.interrupted || events2.interrupted,
            "suspended tabs must observe NetworkInterrupted: {events1:?}/{events2:?}"
        );
        assert!(
            events1.resumed || events2.resumed,
            "suspended tabs must observe NetworkResumed after polling resumes: \
             {events1:?}/{events2:?}"
        );
        assert!(
            !events1.disconnected && !events2.disconnected,
            "a sub-timeout suspension must not disconnect: {events1:?}/{events2:?}"
        );

        let drive_frame = |session1: &mut P2PSession<BrowserConfig>,
                           session2: &mut P2PSession<BrowserConfig>,
                           state1: &mut BrowserState,
                           state2: &mut BrowserState,
                           frame_index: u32,
                           input: BrowserInput| {
            session1
                .add_local_input(PlayerHandle::new(0), input)
                .expect("post-resume first input must be accepted");
            session2
                .add_local_input(PlayerHandle::new(1), input)
                .expect("post-resume second input must be accepted");
            apply_requests(
                state1,
                session1
                    .advance_frame()
                    .expect("post-resume first session must advance"),
            );
            apply_requests(
                state2,
                session2
                    .advance_frame()
                    .expect("post-resume second session must advance"),
            );
            let target = Frame::new(frame_index as i32);
            let mut confirm_polls = 0usize;
            while confirm_polls < INPUT_POLL_BUDGET
                && (session1.confirmed_frame() < target || session2.confirmed_frame() < target)
            {
                poll_pair_with_clock(session1, session2, &offset_ms);
                confirm_polls = confirm_polls.saturating_add(1);
            }
            assert!(
                session1.confirmed_frame() >= target && session2.confirmed_frame() >= target,
                "post-resume frame {frame_index} failed to confirm within \
                 {INPUT_POLL_BUDGET} polls"
            );
        };

        // The same post-resume input on every frame makes repeat-last
        // prediction exact after the first correction, so the final state is a
        // confirmed checkpoint rather than a speculative tail.
        let post_resume_input = BrowserInput { value: 9 };
        for frame_index in 6..9u32 {
            drive_frame(
                &mut session1,
                &mut session2,
                &mut state1,
                &mut state2,
                frame_index,
                post_resume_input,
            );
        }
        assert_eq!(state1, state2, "post-resume states must agree");
        assert_eq!(state1.frame, 9, "post-resume advances must move the frame");
    }

    #[wasm_bindgen_test]
    fn browser_tab_suspension_past_timeout_disconnects_fail_closed() {
        let (clock, offset_ms) = virtual_clock();
        let (mut session1, mut session2) = build_virtual_clock_sessions(clock);

        let mut sync_polls = 0usize;
        while sync_polls < SYNC_POLL_BUDGET
            && (session1.current_state() != SessionState::Running
                || session2.current_state() != SessionState::Running)
        {
            poll_pair_with_clock(&mut session1, &mut session2, &offset_ms);
            sync_polls = sync_polls.saturating_add(1);
        }
        assert!(
            session1.current_state() == SessionState::Running
                && session2.current_state() == SessionState::Running,
            "virtual-clock sessions failed to synchronize"
        );
        let _ = session1.events().count();
        let _ = session2.events().count();

        // One tab keeps polling while the other is throttled past the 2s default
        // disconnect timeout. The live tab must report interruption and then
        // fail closed instead of silently blocking.
        offset_ms.fetch_add(3_000, Ordering::Relaxed);
        let mut live_events = LinkEvents::default();
        for _ in 0..32 {
            session1.poll_remote_clients();
            offset_ms.fetch_add(16, Ordering::Relaxed);
            live_events.merge(drain_link_events(&mut session1));
            if live_events.disconnected {
                break;
            }
        }
        assert!(
            live_events.interrupted,
            "a timed-out suspension must first report interruption: {live_events:?}"
        );
        assert!(
            live_events.disconnected,
            "a suspension past the disconnect timeout must fail closed: {live_events:?}"
        );

        // The throttled tab resumes with the live peer's keepalives still
        // buffered. It must drain them without panicking.
        for _ in 0..32 {
            poll_pair_with_clock(&mut session1, &mut session2, &offset_ms);
            let _ = drain_link_events(&mut session1);
            let _ = drain_link_events(&mut session2);
        }
    }

    /// Explicit graceful removal must complete under the browser's 32-bit
    /// `usize` world, not only in the native simulation harness. The survivor
    /// certifies the drop, the removed target learns its committed certificate
    /// and stops, and the survivor keeps advancing on the frozen slot.
    #[wasm_bindgen_test]
    fn explicit_graceful_removal_certifies_in_browser() {
        let (clock, offset_ms) = virtual_clock();
        let (mut survivor, mut removed) =
            build_virtual_clock_sessions_with_behavior(clock, DisconnectBehavior::ContinueWithout);

        let mut sync_polls = 0usize;
        while sync_polls < SYNC_POLL_BUDGET
            && (survivor.current_state() != SessionState::Running
                || removed.current_state() != SessionState::Running)
        {
            poll_pair_with_clock(&mut survivor, &mut removed, &offset_ms);
            sync_polls = sync_polls.saturating_add(1);
        }
        assert!(
            survivor.current_state() == SessionState::Running
                && removed.current_state() == SessionState::Running,
            "removal sessions failed to synchronize in the browser"
        );
        let _ = survivor.events().count();
        let _ = removed.events().count();

        let mut survivor_state = BrowserState::default();
        let mut removed_state = BrowserState::default();
        for frame_index in 0..4u32 {
            let input = BrowserInput { value: frame_index };
            survivor
                .add_local_input(PlayerHandle::new(0), input)
                .expect("survivor input must be accepted");
            removed
                .add_local_input(PlayerHandle::new(1), input)
                .expect("removed-player input must be accepted");
            apply_requests(
                &mut survivor_state,
                survivor
                    .advance_frame()
                    .expect("survivor must advance before the removal"),
            );
            apply_requests(
                &mut removed_state,
                removed
                    .advance_frame()
                    .expect("removed player must advance before the removal"),
            );
            poll_pair_with_clock(&mut survivor, &mut removed, &offset_ms);
        }

        let cut = survivor.confirmed_frame();
        survivor
            .remove_player(PlayerHandle::new(1))
            .expect("explicit graceful removal must be accepted");

        let mut dropped = false;
        let mut removed_stopped = false;
        for _ in 0..REMOVAL_POLL_BUDGET {
            poll_pair_with_clock(&mut survivor, &mut removed, &offset_ms);
            for event in survivor.events() {
                if let FortressEvent::PeerDropped { handle, .. } = event {
                    assert_eq!(
                        handle,
                        PlayerHandle::new(1),
                        "the certificate must drop exactly the requested handle"
                    );
                    dropped = true;
                }
            }
            let _ = removed.events().count();
            removed_stopped = removed.current_state() == SessionState::Synchronizing;
            if dropped && removed_stopped {
                break;
            }
        }
        assert!(
            dropped,
            "the browser survivor must certify the requested removal"
        );
        assert!(
            removed_stopped,
            "the removed target must fail closed to Synchronizing after applying its committed certificate"
        );
        assert_eq!(
            survivor.current_state(),
            SessionState::Running,
            "the survivor must keep running after the removal"
        );
        assert!(
            survivor.confirmed_frame() >= cut,
            "the survivor must retain its confirmed prefix across the removal"
        );

        survivor
            .add_local_input(PlayerHandle::new(0), BrowserInput { value: 99 })
            .expect("post-removal survivor input must be accepted");
        apply_requests(
            &mut survivor_state,
            survivor
                .advance_frame()
                .expect("the survivor must keep advancing on the frozen slot"),
        );
        poll_pair_with_clock(&mut survivor, &mut removed, &offset_ms);
        assert_eq!(
            survivor.current_state(),
            SessionState::Running,
            "the survivor must stay live after the frozen-slot advance"
        );
    }

    /// Hot join is the most stateful membership transition in the library:
    /// buffer capture, snapshot transfer, snapshot apply, activation-frame
    /// gating, and joined-slot input contribution. This must work under the
    /// browser's 32-bit `usize` world, not only in the native fleet. The host
    /// runs solo with a reserved slot, a joiner loads the served snapshot, and
    /// the joined slot must confirm at least two real inputs after activation.
    #[cfg(feature = "hot-join")]
    #[wasm_bindgen_test]
    fn hot_join_activates_and_confirms_joined_slot_in_browser() {
        let (clock, offset_ms) = virtual_clock();
        let (host_socket, joiner_socket, _bus) = RawBus::pair();
        let protocol_config = |seed| ProtocolConfig {
            protocol_rng_seed: Some(seed),
            clock: Some(clock.clone()),
            ..ProtocolConfig::default()
        };

        let mut host = SessionBuilder::<BrowserConfig>::new()
            .with_protocol_config(protocol_config(0xB103))
            .with_num_players(2)
            .expect("two-player hot-join host must accept its player count")
            .with_hot_join(true)
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .expect("hot-join host local player must be accepted")
            .add_reserved_player(BrowserPeer(1), PlayerHandle::new(1))
            .expect("hot-join host reserved slot must be accepted")
            .start_p2p_session(host_socket)
            .expect("browser hot-join host must build");

        host.poll_remote_clients();
        assert_eq!(
            host.current_state(),
            SessionState::Running,
            "a host with a reserved slot must run solo before the join"
        );
        let _ = host.events().count();

        let mut host_state = BrowserState::default();
        for frame_index in 0..3u32 {
            host.add_local_input(PlayerHandle::new(0), BrowserInput { value: frame_index })
                .expect("pre-join host input must be accepted");
            apply_requests(
                &mut host_state,
                host.advance_frame()
                    .expect("pre-join host must advance solo"),
            );
            host.poll_remote_clients();
            offset_ms.fetch_add(16, Ordering::Relaxed);
        }

        let mut joiner = SessionBuilder::<BrowserConfig>::new()
            .with_protocol_config(protocol_config(0xB104))
            .with_num_players(2)
            .expect("two-player hot-join joiner must accept its player count")
            .add_player(PlayerType::Remote(BrowserPeer(0)), PlayerHandle::new(0))
            .expect("hot-join joiner host slot must be accepted")
            .add_player(PlayerType::Local, PlayerHandle::new(1))
            .expect("hot-join joiner local slot must be accepted")
            .start_hot_join_session(joiner_socket, BrowserPeer(0))
            .expect("browser hot-join joiner must build");
        assert_eq!(
            joiner.current_state(),
            SessionState::HotJoining,
            "a joiner must start in HotJoining"
        );

        // The serving host pauses while the snapshot handshake is in flight, so
        // polling alone completes the join and leaves both peers on the
        // snapshot frame.
        for _ in 0..HOT_JOIN_POLL_BUDGET {
            host.poll_remote_clients();
            joiner.poll_remote_clients();
            offset_ms.fetch_add(16, Ordering::Relaxed);
            if joiner.current_state() == SessionState::Running {
                break;
            }
        }
        assert_eq!(
            joiner.current_state(),
            SessionState::Running,
            "the browser joiner must complete the snapshot handshake within \
             {HOT_JOIN_POLL_BUDGET} polls"
        );

        // The joiner's first advance after the join applies the snapshot.
        let requests = joiner
            .advance_frame()
            .expect("the joiner must advance after applying its snapshot");
        let mut load_count = 0usize;
        let mut other_count = 0usize;
        let mut snapshot_frame = None;
        for request in &*requests {
            if let FortressRequest::LoadGameState { frame, .. } = request {
                load_count = load_count.saturating_add(1);
                snapshot_frame = Some(*frame);
            } else {
                other_count = other_count.saturating_add(1);
            }
        }
        assert_eq!(
            load_count, 1,
            "the joiner's first advance must return exactly one LoadGameState"
        );
        assert_eq!(
            other_count, 0,
            "the joiner's first advance must return only the LoadGameState"
        );
        let snapshot_frame = snapshot_frame.expect("the joiner must report a snapshot frame");
        let mut joiner_state = BrowserState::default();
        apply_requests(&mut joiner_state, requests);
        assert_eq!(
            joiner_state.frame,
            snapshot_frame.as_i32(),
            "the loaded snapshot must position the joiner on the served frame"
        );
        assert_eq!(
            host.current_state(),
            SessionState::Running,
            "the host must resume Running once the join commits"
        );

        // Advance both peers in lockstep. The joiner's own frame cursor starts
        // one frame behind the host's current frame (the host keeps running
        // while the snapshot is served), so drive enough frames for the joined
        // slot's activation frame to clear before checking confirmations.
        for index in 0..6u32 {
            host.add_local_input(
                PlayerHandle::new(0),
                BrowserInput {
                    value: 1_000 + index,
                },
            )
            .expect("post-join host input must be accepted");
            joiner
                .add_local_input(
                    PlayerHandle::new(1),
                    BrowserInput {
                        value: 2_000 + index,
                    },
                )
                .expect("post-join joined-slot input must be accepted");
            apply_requests(
                &mut host_state,
                host.advance_frame().expect("post-join host must advance"),
            );
            apply_requests(
                &mut joiner_state,
                joiner
                    .advance_frame()
                    .expect("post-join joiner must advance"),
            );
            for _ in 0..INPUT_POLL_BUDGET {
                poll_pair_with_clock(&mut host, &mut joiner, &offset_ms);
            }
        }

        let confirm_floor = Frame::new(snapshot_frame.as_i32() + 2);
        for _ in 0..HOT_JOIN_POLL_BUDGET {
            if host.confirmed_frame() >= confirm_floor && joiner.confirmed_frame() >= confirm_floor
            {
                break;
            }
            poll_pair_with_clock(&mut host, &mut joiner, &offset_ms);
        }
        let last_confirmed = Frame::new(
            host.confirmed_frame()
                .as_i32()
                .min(joiner.confirmed_frame().as_i32()),
        );
        assert!(
            last_confirmed >= confirm_floor,
            "post-join confirmation stalled at {last_confirmed:?} \
             (floor {confirm_floor:?}, snapshot {snapshot_frame:?})"
        );

        let mut joined_frames_confirmed = 0u32;
        let mut previous_joined_input = None;
        for offset in 0..HOT_JOIN_CONFIRMED_FRAMES as i32 {
            let frame = Frame::new(last_confirmed.as_i32() - offset);
            let host_inputs = host
                .confirmed_inputs_for_frame(frame)
                .expect("host must expose confirmed post-join inputs");
            let joiner_inputs = joiner
                .confirmed_inputs_for_frame(frame)
                .expect("joiner must expose confirmed post-join inputs");
            assert_eq!(
                host_inputs, joiner_inputs,
                "both peers must confirm identical inputs at post-join frame {frame:?}"
            );
            let joined_input = host_inputs
                .get(1)
                .copied()
                .expect("the joined slot must have a confirmed input");
            assert_ne!(
                joined_input,
                BrowserInput::default(),
                "the joined slot must not confirm the frozen default input at frame {frame:?}"
            );
            if let Some(previous) = previous_joined_input {
                assert_ne!(
                    joined_input, previous,
                    "the joined slot must advance its input stream at frame {frame:?}"
                );
            }
            previous_joined_input = Some(joined_input);
            joined_frames_confirmed = joined_frames_confirmed.saturating_add(1);
        }

        assert_eq!(
            joined_frames_confirmed, HOT_JOIN_CONFIRMED_FRAMES,
            "the joined slot must confirm exactly {HOT_JOIN_CONFIRMED_FRAMES} checked frames"
        );
    }

    /// A fresh joiner must import the snapshot's non-empty `certified_dropped`
    /// list and derive the same membership epoch as the survivors. The host
    /// starts with a live remote and a reserved slot, removes the remote, then
    /// serves the reserved slot. The joiner's epoch must be `[0, 2]`, not
    /// `[0, 1, 2]`; the native suite covers the same gate but not the wasm
    /// snapshot/decode path.
    #[cfg(feature = "hot-join")]
    #[wasm_bindgen_test]
    fn hot_join_imports_certified_dropped_epoch_in_browser() {
        let observer = Arc::new(fortress_rollback::telemetry::CollectingObserver::new());
        let _observer_scope = fortress_rollback::telemetry::push_violation_observer(Arc::clone(
            &observer,
        )
            as Arc<dyn fortress_rollback::telemetry::ViolationObserver>);
        let (clock, offset_ms) = virtual_clock();
        let (sockets, _bus) = RawBus::with_peers(3);
        let mut sockets = sockets.into_iter();
        let host_socket = sockets.next().expect("host socket");
        let peer_socket = sockets.next().expect("peer socket");
        let joiner_socket = sockets.next().expect("joiner socket");
        let protocol_config = |seed| ProtocolConfig {
            protocol_rng_seed: Some(seed),
            clock: Some(clock.clone()),
            ..ProtocolConfig::default()
        };

        let mut host = SessionBuilder::<BrowserConfig>::new()
            .with_protocol_config(protocol_config(0xB105))
            .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
            .with_num_players(3)
            .expect("three-player certified-snapshot host must accept its player count")
            .with_hot_join(true)
            .add_player(PlayerType::Local, PlayerHandle::new(0))
            .expect("certified-snapshot host local player must be accepted")
            .add_player(PlayerType::Remote(BrowserPeer(1)), PlayerHandle::new(1))
            .expect("certified-snapshot host remote player must be accepted")
            .add_reserved_player(BrowserPeer(2), PlayerHandle::new(2))
            .expect("certified-snapshot host reserved slot must be accepted")
            .start_p2p_session(host_socket)
            .expect("certified-snapshot host must build");

        let mut peer = SessionBuilder::<BrowserConfig>::new()
            .with_protocol_config(protocol_config(0xB106))
            .with_disconnect_behavior(DisconnectBehavior::ContinueWithout)
            .with_num_players(3)
            .expect("certified-snapshot peer must accept its player count")
            .with_hot_join(true)
            .add_player(PlayerType::Remote(BrowserPeer(0)), PlayerHandle::new(0))
            .expect("certified-snapshot peer host slot must be accepted")
            .add_player(PlayerType::Local, PlayerHandle::new(1))
            .expect("certified-snapshot peer local slot must be accepted")
            .add_reserved_player(BrowserPeer(2), PlayerHandle::new(2))
            .expect("certified-snapshot peer reserved slot must be accepted")
            .start_p2p_session(peer_socket)
            .expect("certified-snapshot peer must build");

        for _ in 0..SYNC_POLL_BUDGET {
            host.poll_remote_clients();
            peer.poll_remote_clients();
            offset_ms.fetch_add(16, Ordering::Relaxed);
            if host.current_state() == SessionState::Running
                && peer.current_state() == SessionState::Running
            {
                break;
            }
        }
        assert!(
            host.current_state() == SessionState::Running
                && peer.current_state() == SessionState::Running,
            "certified-snapshot host and peer failed to synchronize"
        );
        let _ = host.events().count();
        let _ = peer.events().count();

        let mut host_state = BrowserState::default();
        let mut peer_state = BrowserState::default();
        for frame_index in 0..4u32 {
            host.add_local_input(
                PlayerHandle::new(0),
                BrowserInput {
                    value: 100 + frame_index,
                },
            )
            .expect("pre-removal host input must be accepted");
            peer.add_local_input(
                PlayerHandle::new(1),
                BrowserInput {
                    value: 200 + frame_index,
                },
            )
            .expect("pre-removal peer input must be accepted");
            apply_requests(
                &mut host_state,
                host.advance_frame().expect("pre-removal host must advance"),
            );
            apply_requests(
                &mut peer_state,
                peer.advance_frame().expect("pre-removal peer must advance"),
            );
            host.poll_remote_clients();
            peer.poll_remote_clients();
            offset_ms.fetch_add(16, Ordering::Relaxed);
        }
        for _ in 0..INPUT_POLL_BUDGET {
            if host.confirmed_frame() >= Frame::new(3) {
                break;
            }
            host.poll_remote_clients();
            peer.poll_remote_clients();
            offset_ms.fetch_add(16, Ordering::Relaxed);
        }
        assert!(
            host.confirmed_frame() >= Frame::new(3),
            "the host must confirm a pre-removal snapshot prefix"
        );

        host.remove_player(PlayerHandle::new(1))
            .expect("the certified-snapshot host must accept the removal");
        let mut dropped = false;
        let mut peer_stopped = false;
        for _ in 0..REMOVAL_POLL_BUDGET {
            host.poll_remote_clients();
            peer.poll_remote_clients();
            offset_ms.fetch_add(16, Ordering::Relaxed);
            for event in host.events() {
                if let FortressEvent::PeerDropped { handle, .. } = event {
                    assert_eq!(
                        handle,
                        PlayerHandle::new(1),
                        "the removal must certify exactly the configured remote"
                    );
                    dropped = true;
                }
            }
            let _ = peer.events().count();
            peer_stopped = peer.current_state() == SessionState::Synchronizing;
            if dropped && peer_stopped {
                break;
            }
        }
        assert!(
            dropped,
            "the host must certify the removal before the join; state={:?} confirmed={:?} \
             epoch={:?} host=[{}] peer=[{}] violations={:?}",
            host.current_state(),
            host.confirmed_frame(),
            host.diagnostic_coordinated_drop_membership_epoch(),
            host.diagnostic_connect_status(),
            peer.diagnostic_connect_status(),
            observer
                .violations()
                .iter()
                .map(|violation| violation.message.clone())
                .collect::<Vec<_>>()
        );
        assert!(
            peer_stopped,
            "the removed peer must apply its certificate before the join"
        );
        assert_eq!(
            host.diagnostic_coordinated_drop_membership_epoch(),
            vec![PlayerHandle::new(0)],
            "the survivor epoch must exclude the certified drop and the unjoined reserved slot"
        );

        // Let the survivor drain the removed target's speculative tail before
        // serving the snapshot: the N-peer serve captures `last_saved_frame`
        // and waits for it to confirm while the host is paused.
        for offset in 0..2u32 {
            host.add_local_input(
                PlayerHandle::new(0),
                BrowserInput {
                    value: 300 + offset,
                },
            )
            .expect("post-removal host input must be accepted");
            apply_requests(
                &mut host_state,
                host.advance_frame()
                    .expect("post-removal host must confirm its speculative tail"),
            );
            host.poll_remote_clients();
            offset_ms.fetch_add(16, Ordering::Relaxed);
        }
        assert_eq!(
            host.current_state(),
            SessionState::Running,
            "the survivor must remain Running after draining the removed target"
        );

        let mut joiner = SessionBuilder::<BrowserConfig>::new()
            .with_protocol_config(protocol_config(0xB107))
            .with_num_players(3)
            .expect("certified-snapshot joiner must accept its player count")
            .add_player(PlayerType::Remote(BrowserPeer(0)), PlayerHandle::new(0))
            .expect("certified-snapshot joiner host slot must be accepted")
            .add_player(PlayerType::Remote(BrowserPeer(1)), PlayerHandle::new(1))
            .expect("certified-snapshot joiner dropped slot must be accepted")
            .add_player(PlayerType::Local, PlayerHandle::new(2))
            .expect("certified-snapshot joiner local slot must be accepted")
            .start_hot_join_session(joiner_socket, BrowserPeer(0))
            .expect("certified-snapshot joiner must build");
        assert_eq!(
            joiner.current_state(),
            SessionState::HotJoining,
            "the certified-snapshot joiner must start in HotJoining"
        );

        for _ in 0..HOT_JOIN_POLL_BUDGET {
            host.poll_remote_clients();
            joiner.poll_remote_clients();
            offset_ms.fetch_add(16, Ordering::Relaxed);
            if joiner.current_state() == SessionState::Running {
                break;
            }
        }
        assert!(
            joiner.current_state() == SessionState::Running,
            "the certified-snapshot joiner must complete the snapshot handshake; \
             joiner={:?} host={:?} host_epoch={:?} joiner_epoch={:?} host=[{}] joiner=[{}] \
             violations={:?}",
            joiner.current_state(),
            host.current_state(),
            host.diagnostic_coordinated_drop_membership_epoch(),
            joiner.diagnostic_coordinated_drop_membership_epoch(),
            host.diagnostic_connect_status(),
            joiner.diagnostic_connect_status(),
            observer
                .violations()
                .iter()
                .map(|violation| violation.message.clone())
                .collect::<Vec<_>>()
        );

        // The first advance applies the served snapshot, including its
        // certified-dropped list.
        let mut joiner_state = BrowserState::default();
        apply_requests(
            &mut joiner_state,
            joiner
                .advance_frame()
                .expect("the certified-snapshot joiner must apply its snapshot"),
        );

        assert_eq!(
            joiner.diagnostic_coordinated_drop_membership_epoch(),
            vec![PlayerHandle::new(0), PlayerHandle::new(2)],
            "the joiner must import the snapshot's certified-dropped slot into its epoch"
        );
        assert_eq!(
            host.diagnostic_coordinated_drop_membership_epoch(),
            vec![PlayerHandle::new(0), PlayerHandle::new(2)],
            "the host epoch must include the now-joined reserved slot and exclude the dropped slot"
        );
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

    struct RawBusState {
        queues: Vec<VecDeque<(BrowserPeer, Vec<u8>)>>,
        stats: RawTransportStats,
    }

    #[derive(Clone)]
    struct RawBus {
        state: Rc<RefCell<RawBusState>>,
    }

    impl RawBus {
        fn with_peers(count: usize) -> (Vec<RawChannelSocket>, Self) {
            let mut queues = Vec::with_capacity(count);
            queues.resize_with(count, VecDeque::new);
            let bus = Self {
                state: Rc::new(RefCell::new(RawBusState {
                    queues,
                    stats: RawTransportStats::default(),
                })),
            };
            let sockets = (0..count)
                .map(|index| RawChannelSocket {
                    local: BrowserPeer(u8::try_from(index).unwrap_or(0)),
                    bus: bus.clone(),
                })
                .collect();
            (sockets, bus)
        }

        fn pair() -> (RawChannelSocket, RawChannelSocket, Self) {
            let (sockets, bus) = Self::with_peers(2);
            let mut sockets = sockets.into_iter();
            let first = sockets.next().expect("first browser test socket");
            let second = sockets.next().expect("second browser test socket");
            (first, second, bus)
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

    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
